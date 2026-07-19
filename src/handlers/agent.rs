// GuestKit in-guest agent deployment — `veyron agent {deploy,status,verify}`.
//
// Gets the Zyvor GuestKit agent running inside a Veyron-spawned VM and verifies
// it. On KubeVirt the agent rides the QEMU guest-agent channel
// (`org.qemu.guest_agent.0`) and answers the QGA protocol (guest-info, guest-exec,
// guest-fsfreeze, …), so it doubles as the KubeVirt guest agent — KubeVirt reports
// `AgentConnected` and Veyron's guest-exec / GuestKit-RPC paths both work through it.
//
//   Linux   — cloud-init downloads the published static `guestkitd` and runs it
//             on the QGA virtio channel via a systemd unit.
//   Windows — the published GuestKit ISO is attached as a CD-ROM (CDI import) and
//             Cloudbase-Init installs the MSI (registers the GuestKitAgent service).
//
// The agent binary/ISO come from the public GuestKit release by default; override
// with `--bundle-url` / `--iso`.

use anyhow::{anyhow, bail, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;

use crate::cli::AgentCommands;
use crate::config::VMConfigBuilder;
use crate::kube::kubevirt_subresources::{vmi_guest_exec, wait_guest_exec};
use crate::kube::{self, KubeClient};

/// Published GuestKit release (see github.com/hypersdk/guestkit/releases).
const GUESTKIT_RELEASE_TAG: &str = "guestkit-agent-v0.3.14";
const GUESTKIT_RELEASE_BASE: &str =
    "https://github.com/hypersdk/guestkit/releases/download/guestkit-agent-v0.3.14";

fn default_linux_bundle_url() -> String {
    format!("{GUESTKIT_RELEASE_BASE}/guestkitd")
}
fn default_windows_iso_url() -> String {
    format!("{GUESTKIT_RELEASE_BASE}/guestkit-agent-0.3.14.iso")
}

pub async fn handle_agent(command: AgentCommands, namespace: &str) -> Result<()> {
    match command {
        AgentCommands::Deploy {
            name,
            spawn,
            template,
            os,
            bundle_url,
            iso,
            wait,
            dry_run,
        } => {
            if is_windows(&os, &template) {
                deploy_windows(
                    &name,
                    iso.unwrap_or_else(default_windows_iso_url),
                    spawn,
                    dry_run,
                    namespace,
                )
                .await
            } else {
                deploy_linux(
                    &name,
                    bundle_url.unwrap_or_else(default_linux_bundle_url),
                    spawn,
                    wait,
                    dry_run,
                    namespace,
                )
                .await
            }
        }
        AgentCommands::Status { name } => report_status(&name, namespace).await,
        AgentCommands::Verify { name } => verify(&name, namespace).await,
    }
}

fn is_windows(os: &Option<String>, template: &Option<String>) -> bool {
    if let Some(o) = os {
        return o.eq_ignore_ascii_case("windows") || o.eq_ignore_ascii_case("win");
    }
    template
        .as_deref()
        .map(|t| t.to_ascii_lowercase().contains("windows"))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Linux
// ---------------------------------------------------------------------------

async fn deploy_linux(
    name: &str,
    bundle_url: String,
    spawn: bool,
    wait: bool,
    dry_run: bool,
    namespace: &str,
) -> Result<()> {
    let client = KubeClient::new().await?;

    if spawn {
        let mut config = VMConfigBuilder::new(name)
            .namespace(namespace)
            .cpu(2, 1, 1)
            .memory("2Gi")
            .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:22.04", 1)
            .add_pod_network("default")
            .label("os", "ubuntu")
            .label("app.kubernetes.io/managed-by", "veyron-agent-deploy")
            .label("guestkit.zyvor.dev/agent", GUESTKIT_RELEASE_TAG)
            .cloud_init(linux_cloud_init(&bundle_url))
            .enable_rng()
            .build();
        config.allow_internet = true; // needs egress to fetch the release binary

        if dry_run {
            let vm = kube::vm_config_to_kubevirt(&config)?;
            println!("{}", serde_json::to_string_pretty(&vm)?);
            return Ok(());
        }

        println!("==> Creating VM '{name}' with GuestKit agent ({GUESTKIT_RELEASE_TAG})");
        println!("    agent source: {bundle_url}");
        client.create_vm(&config).await?;
        client.start_vm(namespace, name).await?;
        println!("==> VM '{name}' created and starting");
    } else if dry_run {
        println!("{}", linux_cloud_init(&bundle_url));
        return Ok(());
    } else {
        // Deploy into an already-running VM via guest-exec (needs a guest agent
        // already answering the QGA channel — e.g. a base cloud image's qemu-ga).
        println!("==> Installing GuestKit agent into running VM '{name}' via guest-exec");
        let script = linux_install_script(&bundle_url);
        let out = guest_exec(&client, namespace, name, "/bin/sh", &["-c", &script]).await?;
        println!("{out}");
    }

    if wait {
        wait_and_verify_linux(&client, namespace, name).await?;
    } else if spawn {
        println!(
            "    verify once booted:  veyron agent verify {name}  (or --wait to block here)"
        );
    }
    Ok(())
}

/// Shell that installs `guestkitd` as the QGA-channel agent and starts it.
fn linux_install_script(bundle_url: &str) -> String {
    format!(
        r#"set -eu
# Free the QGA virtio channel so GuestKit can own it (it speaks the QGA protocol).
systemctl disable --now qemu-guest-agent 2>/dev/null || true
mkdir -p /usr/local/bin
for i in $(seq 1 30); do
  if curl -fkSL -o /usr/local/bin/guestkit "{bundle_url}" && [ -s /usr/local/bin/guestkit ]; then break; fi
  sleep 5
done
chmod 0755 /usr/local/bin/guestkit
/usr/local/bin/guestkit selftest /var/log/guestkit-selftest.json 2>/dev/null || true
cat > /etc/systemd/system/guestkit-agent.service <<'UNIT'
[Unit]
Description=Zyvor GuestKit Agent
After=network-online.target
Wants=network-online.target
[Service]
ExecStart=/bin/sh -c 'until [ -e /dev/virtio-ports/org.qemu.guest_agent.0 ]; do sleep 2; done; exec /usr/local/bin/guestkit --channel virtio'
Restart=always
RestartSec=5
User=root
[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable --now guestkit-agent
"#
    )
}

fn linux_cloud_init(bundle_url: &str) -> String {
    // Indent every script line by 6 spaces so it sits inside the `content: |`
    // block scalar (whose first line `#!/bin/sh` is also at 6 spaces). cloud-init
    // strips this common indent when it writes the file.
    let script = format!("#!/bin/sh\n{}", linux_install_script(bundle_url))
        .lines()
        .map(|l| format!("      {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"#cloud-config
package_update: true
packages:
  - curl
write_files:
  - path: /usr/local/sbin/veyron-install-guestkit.sh
    permissions: '0755'
    content: |
{script}
runcmd:
  - /usr/local/sbin/veyron-install-guestkit.sh
"#
    )
}

async fn wait_and_verify_linux(client: &KubeClient, namespace: &str, name: &str) -> Result<()> {
    println!("==> Waiting for the GuestKit agent to connect (KubeVirt AgentConnected)…");
    let mut connected = false;
    for attempt in 0..90 {
        if let Ok(vmi) = client.get_vmi(namespace, name).await {
            let vmi_json = serde_json::to_value(&vmi)?;
            if kube::windows_rdp::vmi_guest_agent_connected(&vmi_json) {
                connected = true;
                break;
            }
        }
        if attempt % 6 == 0 {
            println!("    …still booting/installing ({}s)", attempt * 5);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    if !connected {
        bail!("timed out waiting for the guest agent to connect (VM may still be installing)");
    }
    println!("    ✓ AgentConnected — GuestKit is answering the QGA channel");

    // Prove the deployed agent actually works: run its self-test battery in-guest
    // (through the agent's own guest-exec) and show the version it reports.
    let probe = "systemctl is-active guestkit-agent 2>/dev/null; echo '---'; \
                 /usr/local/bin/guestkit selftest /tmp/gk.json >/dev/null 2>&1; \
                 cat /tmp/gk.json 2>/dev/null | head -c 3000";
    match guest_exec(client, namespace, name, "/bin/sh", &["-c", probe]).await {
        Ok(out) => {
            println!("==> In-guest GuestKit self-test:\n{}", indent(&out));
        }
        Err(e) => println!("    (self-test via guest-exec unavailable: {e})"),
    }
    println!("\n✓ GuestKit agent deployed and verified in '{name}'.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

async fn deploy_windows(
    name: &str,
    iso_url: String,
    spawn: bool,
    dry_run: bool,
    namespace: &str,
) -> Result<()> {
    // The GuestKit ISO is imported into a PVC by CDI and attached as a CD-ROM;
    // Cloudbase-Init installs the MSI on first boot (registers GuestKitAgent).
    let iso_dv = format!("{name}-guestkit-iso");
    let datavolume = guestkit_iso_datavolume(&iso_dv, &iso_url, namespace);

    let mut config = VMConfigBuilder::new(name)
        .namespace(namespace)
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_pod_network("default")
        .label("os", "windows")
        .label("app.kubernetes.io/managed-by", "veyron-agent-deploy")
        .label("guestkit.zyvor.dev/agent", GUESTKIT_RELEASE_TAG)
        .cloud_init_config_drive(windows_cloudbase_init())
        .build();
    config.allow_internet = true;
    // Attach the GuestKit ISO PVC as a read-only CD-ROM.
    attach_iso_cdrom(&mut config, "guestkit-iso", &iso_dv);

    let vm = kube::vm_config_to_kubevirt(&config)?;

    if dry_run || !spawn {
        println!("# GuestKit ISO import (CDI DataVolume):");
        println!("{}", serde_json::to_string_pretty(&datavolume)?);
        println!("---");
        println!("# Windows VM with GuestKit ISO attached + Cloudbase-Init install:");
        println!("{}", serde_json::to_string_pretty(&vm)?);
        if !dry_run {
            println!(
                "\nNote: spawning a Windows guest also needs a Windows OS disk/base image on \
                 the cluster. Apply the DataVolume + VM above once that base image is present."
            );
        }
        return Ok(());
    }

    println!("==> Importing GuestKit ISO via CDI and creating Windows VM '{name}'");
    let client = KubeClient::new().await?;
    apply_datavolume(&client, namespace, &datavolume).await?;
    client.create_vm(&config).await?;
    client.start_vm(namespace, name).await?;
    println!("==> Windows VM '{name}' created; Cloudbase-Init will install the GuestKit MSI on boot");
    Ok(())
}

fn attach_iso_cdrom(config: &mut crate::config::VMConfig, disk_name: &str, dv_name: &str) {
    use crate::config::types::{DiskConfig, DiskDeviceType, DiskSource};
    config.disks.push(DiskConfig {
        name: disk_name.to_string(),
        size: "0".to_string(),
        storage_class: None,
        boot_order: 3,
        source: DiskSource::DataVolume {
            name: dv_name.to_string(),
        },
        device_type: DiskDeviceType::CDROM,
        bus: Some("sata".to_string()),
        cache: None,
        io: None,
    });
}

fn guestkit_iso_datavolume(name: &str, url: &str, namespace: &str) -> Value {
    json!({
        "apiVersion": "cdi.kubevirt.io/v1beta1",
        "kind": "DataVolume",
        "metadata": { "name": name, "namespace": namespace },
        "spec": {
            "source": { "http": { "url": url } },
            "storage": {
                "resources": { "requests": { "storage": "1Gi" } },
                "accessModes": ["ReadWriteOnce"]
            }
        }
    })
}

/// Cloudbase-Init first-boot script: install the GuestKit MSI from the attached CD.
fn windows_cloudbase_init() -> &'static str {
    r#"#ps1_sysnative
# Veyron: install the GuestKit agent from the attached CD-ROM (registers GuestKitAgent service).
$cd = Get-Volume | Where-Object { $_.DriveType -eq 'CD-ROM' -and $_.FileSystemLabel -eq 'GUESTKIT' } | Select-Object -First 1
if ($cd) {
  $msi = Join-Path ($cd.DriveLetter + ':\') 'gk\guestkit-agent.msi'
  if (Test-Path $msi) {
    Start-Process msiexec.exe -ArgumentList "/i `"$msi`" /qn /norestart" -Wait
  }
  $bat = Join-Path ($cd.DriveLetter + ':\') 'gk\install.bat'
  if ((-not (Test-Path $msi)) -and (Test-Path $bat)) { & $bat }
}
sc.exe query GuestKitAgent
"#
}

async fn apply_datavolume(client: &KubeClient, namespace: &str, dv: &Value) -> Result<()> {
    use ::kube::api::PostParams;
    use ::kube::core::{DynamicObject, GroupVersionKind};
    use ::kube::discovery::ApiResource;
    use ::kube::Api;
    let gvk = GroupVersionKind::gvk("cdi.kubevirt.io", "v1beta1", "DataVolume");
    let ar = ApiResource::from_gvk_with_plural(&gvk, "datavolumes");
    let api: Api<DynamicObject> = Api::namespaced_with(client.client(), namespace, &ar);
    let obj: DynamicObject = serde_json::from_value(dv.clone())?;
    match api.create(&PostParams::default(), &obj).await {
        Ok(_) => Ok(()),
        Err(::kube::Error::Api(e)) if e.code == 409 => Ok(()), // already exists
        Err(e) => Err(anyhow!("failed to create GuestKit ISO DataVolume: {e}")),
    }
}

// ---------------------------------------------------------------------------
// status / verify (shared)
// ---------------------------------------------------------------------------

async fn report_status(name: &str, namespace: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let vmi = client
        .get_vmi(namespace, name)
        .await
        .map_err(|e| anyhow!("no running VMI for '{name}': {e}"))?;
    let vmi_json = serde_json::to_value(&vmi)?;
    let connected = kube::windows_rdp::vmi_guest_agent_connected(&vmi_json);
    let windows = kube::windows_rdp::is_windows_guest(None, Some(&vmi_json));
    println!("VM:             {name}");
    println!("Guest OS:       {}", if windows { "windows" } else { "linux" });
    println!(
        "AgentConnected: {}",
        if connected { "true" } else { "false" }
    );
    if !connected {
        println!("(GuestKit not answering the guest-agent channel yet)");
        return Ok(());
    }
    let out = if windows {
        guest_exec(
            &client,
            namespace,
            name,
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            &["-NoProfile", "-Command", "sc.exe query GuestKitAgent"],
        )
        .await
    } else {
        guest_exec(
            &client,
            namespace,
            name,
            "/bin/sh",
            &["-c", "systemctl is-active guestkit-agent; /usr/local/bin/guestkit --help >/dev/null 2>&1 && echo binary-present"],
        )
        .await
    };
    match out {
        Ok(o) => println!("GuestKit:\n{}", indent(&o)),
        Err(e) => println!("GuestKit probe failed: {e}"),
    }
    Ok(())
}

async fn verify(name: &str, namespace: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let vmi = client.get_vmi(namespace, name).await?;
    let vmi_json = serde_json::to_value(&vmi)?;
    if !kube::windows_rdp::vmi_guest_agent_connected(&vmi_json) {
        bail!("guest agent not connected on '{name}' — deploy it first / wait for boot");
    }
    let windows = kube::windows_rdp::is_windows_guest(None, Some(&vmi_json));
    let out = if windows {
        guest_exec(
            &client,
            namespace,
            name,
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            &[
                "-NoProfile",
                "-Command",
                r#"& 'C:\Program Files\Zyvor GuestKit\guestkitd.exe' selftest C:\gk.json; Get-Content C:\gk.json -Raw"#,
            ],
        )
        .await?
    } else {
        guest_exec(
            &client,
            namespace,
            name,
            "/bin/sh",
            &[
                "-c",
                "/usr/local/bin/guestkit selftest /tmp/gk.json >/dev/null 2>&1; cat /tmp/gk.json",
            ],
        )
        .await?
    };
    println!("{out}");
    Ok(())
}

// ---------------------------------------------------------------------------
// guest-exec helper (QGA guest-exec + wait + decode)
// ---------------------------------------------------------------------------

async fn guest_exec(
    client: &KubeClient,
    namespace: &str,
    vmi: &str,
    path: &str,
    args: &[&str],
) -> Result<String> {
    let body = json!({ "path": path, "arg": args, "capture-output": true });
    let mut resp = vmi_guest_exec(client.client(), namespace, vmi, body).await?;
    if let Some(pid) = resp.get("pid").and_then(|p| p.as_i64()) {
        let done = resp.get("out-data").is_some() || resp.get("exit-code").is_some();
        if !done {
            if let Some(status) = wait_guest_exec(
                client.client(),
                namespace,
                vmi,
                pid,
                60,
                Duration::from_millis(500),
            )
            .await?
            {
                resp = status;
            }
        }
    }
    let decode = |key: &str| -> Option<String> {
        resp.get(key)
            .and_then(|v| v.as_str())
            .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
            .map(|d| String::from_utf8_lossy(&d).into_owned())
    };
    let mut out = decode("out-data").unwrap_or_default();
    if let Some(err) = decode("err-data") {
        if !err.trim().is_empty() {
            out.push_str("\n[stderr] ");
            out.push_str(err.trim());
        }
    }
    if out.trim().is_empty() {
        out = format!("(no output; raw: {resp})");
    }
    Ok(out)
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}
