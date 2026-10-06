// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use crate::config::*;
use std::collections::HashMap;
use std::sync::LazyLock;

pub mod registry;

// ============================================================================
// Shared feature/firmware/clock presets
// ============================================================================

/// Windows HyperV enlightenments for optimal performance
fn windows_features() -> FeaturesConfig {
    FeaturesConfig {
        acpi: true,
        apic: true,
        hyperv: Some(HyperVConfig {
            relaxed: true,
            vapic: true,
            spinlocks: Some(8191),
            vpindex: true,
            runtime: true,
            synic: true,
            stimer: true,
            reset: true,
            frequencies: true,
            reenlightenment: true,
            tlbflush: true,
            ipi: true,
        }),
        kvm_hidden: None,
        smm: Some(true),
    }
}

/// UEFI firmware with Secure Boot (required for Windows 11)
fn uefi_secure_boot_firmware() -> FirmwareConfig {
    FirmwareConfig {
        bootloader: BootloaderType::EFI {
            secure_boot: true,
            persistent: true,
        },
    }
}

/// UEFI firmware without Secure Boot
fn uefi_firmware() -> FirmwareConfig {
    FirmwareConfig {
        bootloader: BootloaderType::EFI {
            secure_boot: false,
            persistent: true,
        },
    }
}

/// Windows clock configuration with HyperV timer
fn windows_clock() -> ClockConfig {
    ClockConfig {
        utc: true,
        timezone: None,
        timers: Some(TimersConfig {
            hpet_present: Some(false),
            pit_tick_policy: Some("delay".to_string()),
            rtc_tick_policy: Some("catchup".to_string()),
            hyperv_present: Some(true),
        }),
    }
}

/// Linux clock configuration
fn linux_clock() -> ClockConfig {
    ClockConfig {
        utc: true,
        timezone: None,
        timers: Some(TimersConfig {
            hpet_present: Some(false),
            pit_tick_policy: Some("delay".to_string()),
            rtc_tick_policy: Some("catchup".to_string()),
            hyperv_present: None,
        }),
    }
}

pub static TEMPLATES: LazyLock<TemplateManager> = LazyLock::new(TemplateManager::new);

/// Manages VM templates
pub struct TemplateManager {
    templates: HashMap<String, VMConfig>,
}

/// Built-in catalog, aligned with the Kryton image catalog (`../kryton/internal/catalog`).
/// Only releases still in vendor support ship; a bare family name aliases the newest one.
const LINUX_TEMPLATES: &[LinuxTemplate] = &[
    LinuxTemplate::container(
        "ubuntu-26.04",
        "ubuntu",
        "26.04",
        "quay.io/containerdisks/ubuntu:26.04",
    ),
    LinuxTemplate::container(
        "ubuntu-24.04",
        "ubuntu",
        "24.04",
        "quay.io/containerdisks/ubuntu:24.04",
    ),
    LinuxTemplate::container(
        "ubuntu-22.04",
        "ubuntu",
        "22.04",
        "quay.io/containerdisks/ubuntu:22.04",
    ),
    LinuxTemplate::container(
        "debian-13",
        "debian",
        "13",
        "quay.io/containerdisks/debian:13",
    ),
    LinuxTemplate::container(
        "debian-12",
        "debian",
        "12",
        "quay.io/containerdisks/debian:12",
    ),
    LinuxTemplate::container(
        "fedora-44",
        "fedora",
        "44",
        "quay.io/containerdisks/fedora:44",
    ),
    LinuxTemplate::container(
        "centos-stream-10",
        "centos",
        "stream10",
        "quay.io/containerdisks/centos-stream:10",
    ),
    LinuxTemplate::container(
        "centos-stream-9",
        "centos",
        "stream9",
        "quay.io/containerdisks/centos-stream:9",
    ),
    LinuxTemplate::container(
        "almalinux-10",
        "almalinux",
        "10",
        "quay.io/containerdisks/almalinux:10",
    ),
    LinuxTemplate::container(
        "almalinux-9",
        "almalinux",
        "9",
        "quay.io/containerdisks/almalinux:9",
    ),
    // quay.io/containerdisks/rockylinux is auth-gated; Rocky boots from the Kryton golden
    // image (`POST /api/v1/kryton/golden/:id/bootstrap` publishes the DataSource).
    LinuxTemplate::kryton("rocky-10", "rocky", "10"),
    LinuxTemplate::kryton("rocky-9", "rocky", "9"),
    LinuxTemplate::container(
        "opensuse-leap-16",
        "opensuse",
        "leap16",
        "quay.io/containerdisks/opensuse-leap:16.0",
    ),
];

const LINUX_ALIASES: &[(&str, &str)] = &[
    ("ubuntu", "ubuntu-26.04"),
    ("debian", "debian-13"),
    ("fedora", "fedora-44"),
    ("centos", "centos-stream-10"),
    ("almalinux", "almalinux-10"),
    ("rocky", "rocky-10"),
    ("opensuse", "opensuse-leap-16"),
];

struct LinuxTemplate {
    name: &'static str,
    os: &'static str,
    version: &'static str,
    /// `Some(image)` boots a containerdisk; `None` clones `kryton-images/<name>`.
    container_image: Option<&'static str>,
}

impl LinuxTemplate {
    const fn container(
        name: &'static str,
        os: &'static str,
        version: &'static str,
        image: &'static str,
    ) -> Self {
        Self {
            name,
            os,
            version,
            container_image: Some(image),
        }
    }

    const fn kryton(name: &'static str, os: &'static str, version: &'static str) -> Self {
        Self {
            name,
            os,
            version,
            container_image: None,
        }
    }

    fn build(&self) -> VMConfig {
        let builder = VMConfigBuilder::new(format!("{}-vm", self.os))
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi");
        let builder = match self.container_image {
            Some(image) => builder
                .add_container_disk("rootdisk", image, 1)
                .add_blank_disk("datadisk", "20Gi", 2),
            None => builder.add_golden_image_disk(
                "rootdisk",
                self.name,
                crate::api::kryton::KRYTON_IMAGE_NAMESPACE,
                "20Gi",
                1,
            ),
        };
        let cloud_init = if self.os == "ubuntu" {
            default_ubuntu_cloud_init()
        } else {
            default_cloud_init()
        };
        builder
            .add_pod_network("default")
            .label("os", self.os)
            .label("os.version", self.version)
            .cloud_init(cloud_init)
            .enable_rng()
            .clock(linux_clock())
            .interactive_console_defaults()
            .build()
    }
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        for t in LINUX_TEMPLATES {
            templates.insert(t.name.to_string(), t.build());
        }
        for (alias, target) in LINUX_ALIASES {
            let cfg = templates[*target].clone();
            templates.insert(alias.to_string(), cfg);
        }

        templates.insert("windows".to_string(), windows_server_template("2025"));
        templates.insert("windows-2025".to_string(), windows_server_template("2025"));
        templates.insert("windows-2022".to_string(), windows_server_template("2022"));
        templates.insert("windows-2019".to_string(), windows_server_template("2019"));
        templates.insert("windows-11".to_string(), windows_11_template());

        Self { templates }
    }

    pub fn get(&self, name: &str) -> Option<VMConfig> {
        if let Ok(Some(reg)) = registry::TemplateRegistry::load() {
            if let Some(path) = registry::TemplateRegistry::registry_path() {
                if let Ok(Some(cfg)) = reg.resolve_config(name, &path) {
                    return Some(cfg);
                }
            }
        }
        self.templates.get(name).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<_> = self.templates.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn exists(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get templates grouped by OS family
    pub fn list_by_family(&self) -> HashMap<String, Vec<String>> {
        let mut families: HashMap<String, Vec<String>> = HashMap::new();

        for name in self.templates.keys() {
            let family = if name.starts_with("ubuntu") {
                "Ubuntu"
            } else if name.starts_with("fedora") {
                "Fedora"
            } else if name.starts_with("centos") {
                "CentOS Stream"
            } else if name.starts_with("debian") {
                "Debian"
            } else if name.starts_with("almalinux") {
                "AlmaLinux"
            } else if name.starts_with("rocky") {
                "Rocky Linux"
            } else if name.starts_with("opensuse") {
                "openSUSE"
            } else if name.starts_with("windows") {
                "Windows"
            } else {
                "Other"
            };

            families
                .entry(family.to_string())
                .or_default()
                .push(name.clone());
        }

        for templates in families.values_mut() {
            templates.sort();
        }

        families
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Windows Templates
// ============================================================================

/// Minimal Cloudbase-Init config-drive userData for first-boot setup.
/// Replace credentials via GitOps secrets before production use.
fn windows_cloud_init_userdata() -> &'static str {
    r#"#ps1_sysnative
# Veyron Windows template — enable RDP; inject passwords via GitOps/Secrets in production.
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name 'fDenyTSConnections' -Value 0
Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'
Write-Host 'Cloudbase-Init config-drive applied by Veyron template.'
"#
}

fn windows_base(version: &str, root_size: &str) -> VMConfigBuilder {
    VMConfigBuilder::new("windows-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk_sata("rootdisk", root_size, 1)
        .set_disk_cache("rootdisk", "none")
        .add_cdrom(
            "virtio-drivers",
            "quay.io/kubevirt/virtio-container-disk:v1.8.1",
            2,
        )
        .add_windows_network("default")
        .label("os", "windows")
        .label("os.version", version)
        .features(windows_features())
        .clock(windows_clock())
        .enable_tpm()
        .enable_rng()
        .usb_tablet()
        .disable_balloon()
        .io_threads_policy("shared")
        .termination_grace_period(120)
        .machine_type("q35")
}

fn windows_server_template(version: &str) -> VMConfig {
    windows_base(version, "80Gi")
        .firmware(uefi_firmware())
        .cloud_init_config_drive(windows_cloud_init_userdata())
        .build()
}

fn windows_11_template() -> VMConfig {
    windows_base("11", "80Gi")
        .firmware(uefi_secure_boot_firmware())
        .cloud_init_config_drive(windows_cloud_init_userdata())
        .build()
}

// ============================================================================
// Cloud-init configurations
// ============================================================================

fn generate_random_password() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%"
        .chars()
        .collect();
    (0..16)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// Shared GuestKit agent systemd unit (Linux cloud-init).
pub fn guestkit_agent_systemd_unit() -> &'static str {
    r#"[Unit]
Description=GuestKit Agent (Zyvor)
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/bin/sh -c 'until [ -e /dev/virtio-ports/org.qemu.guest_agent.0 ]; do sleep 2; done; exec /usr/local/bin/guestkit agent --channel virtio'
Restart=always
RestartSec=5
User=root
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
"#
}

/// Default URL cloud-init uses to fetch the GuestKit binary (override via VEYRON_GUESTKIT_BINARY_URL).
pub fn default_guestkit_binary_url() -> String {
    guestkit_binary_urls()
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            "https://veyron-api.veyron-system.svc.cluster.local/api/v1/platform/guestkit/binary"
                .into()
        })
}

/// Ordered GuestKit binary download URLs (tried in sequence during cloud-init).
pub fn guestkit_binary_urls() -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(u) = std::env::var("VEYRON_GUESTKIT_BINARY_URL") {
        if !u.is_empty() {
            urls.push(u);
        }
    }
    if let Ok(host) = std::env::var("VEYRON_API_NODE_HOST") {
        if !host.is_empty() {
            let port = std::env::var("VEYRON_API_NODE_PORT").unwrap_or_else(|_| "30151".into());
            urls.push(format!(
                "https://{host}:{port}/api/v1/platform/guestkit/binary"
            ));
        }
    }
    if let Ok(ip) = std::env::var("VEYRON_API_CLUSTER_IP") {
        if !ip.is_empty() {
            urls.push(format!("https://{ip}/api/v1/platform/guestkit/binary"));
        }
    }
    urls.push(
        "https://veyron-api.veyron-system.svc.cluster.local/api/v1/platform/guestkit/binary".into(),
    );
    // Public GuestKit release (github.com/zyvorai/guestkit) — authoritative
    // last-resort source of the static `guestkitd` agent when the in-cluster
    // endpoint is unavailable. Also the default for `veyron agent deploy`.
    urls.push(GUESTKIT_RELEASE_LINUX_BINARY_URL.into());
    urls.dedup();
    urls
}

/// Published GuestKit release tag (github.com/zyvorai/guestkit/releases).
pub const GUESTKIT_RELEASE_TAG: &str = "guestkit-agent-v0.3.14";
/// Static Linux agent binary (`guestkitd`) in the published GuestKit release.
pub const GUESTKIT_RELEASE_LINUX_BINARY_URL: &str =
    "https://github.com/zyvorai/guestkit/releases/download/guestkit-agent-v0.3.14/guestkitd";
/// Windows agent bundle ISO in the published GuestKit release.
pub const GUESTKIT_RELEASE_WINDOWS_ISO_URL: &str = "https://github.com/zyvorai/guestkit/releases/download/guestkit-agent-v0.3.14/guestkit-agent-0.3.14.iso";

fn guestkit_resolv_conf_yaml() -> String {
    let dns = std::env::var("VEYRON_CLUSTER_DNS").unwrap_or_else(|_| "10.43.0.10".into());
    format!(
        r#"manage_resolv_conf: true
resolv_conf:
  nameservers: ['{dns}']
  searchdomains:
    - veyron-system.svc.cluster.local
    - svc.cluster.local
    - cluster.local
"#
    )
}

fn guestkit_install_script() -> String {
    let urls = guestkit_binary_urls();
    let url_lines = urls
        .iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"#!/bin/sh
set -eu
URLS="
{url_lines}
"
mkdir -p /usr/local/bin
for attempt in $(seq 1 30); do
  for url in $URLS; do
    [ -z "$url" ] && continue
    if curl -fkSL -o /usr/local/bin/guestkit "$url" 2>/dev/null && [ -s /usr/local/bin/guestkit ]; then
      chmod 0755 /usr/local/bin/guestkit
      exit 0
    fi
  done
  sleep 10
done
echo "GuestKit binary download failed after 30 attempts" >&2
exit 1
"#
    )
}

fn guestkit_write_files_block() -> String {
    format!(
        r#"  - path: /usr/local/sbin/veyron-install-guestkit.sh
    permissions: '0755'
    content: |
{}
  - path: /etc/systemd/system/guestkit-agent.service
    content: |
{}"#,
        guestkit_install_script()
            .lines()
            .map(|l| format!("      {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
        guestkit_agent_systemd_unit()
            .lines()
            .map(|l| format!("      {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// Refresh GuestKit install script URLs in an existing cloud-init payload (call at VM create time).
pub fn refresh_guestkit_cloud_init(user_data: &str) -> String {
    if !user_data.contains("guestkit-agent") {
        return user_data.to_string();
    }
    if user_data.contains("/usr/local/sbin/veyron-install-guestkit.sh") {
        let script = guestkit_install_script();
        let indented = script
            .lines()
            .map(|l| format!("      {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(start) = user_data.find("  - path: /usr/local/sbin/veyron-install-guestkit.sh")
        {
            if let Some(content_start) = user_data[start..].find("content: |\n") {
                let abs_content = start + content_start + "content: |\n".len();
                if let Some(end) = user_data[abs_content..]
                    .find("\n  - path: /etc/systemd/system/guestkit-agent.service")
                {
                    let mut out = String::new();
                    out.push_str(&user_data[..abs_content]);
                    out.push_str(&indented);
                    out.push('\n');
                    out.push_str(&user_data[abs_content + end..]);
                    return out;
                }
            }
        }
    }
    user_data.replace(
        &default_guestkit_binary_url(),
        &guestkit_binary_urls().first().cloned().unwrap_or_default(),
    )
}

fn linux_guestkit_cloud_init(user: &str, password: &str, package_update: bool) -> String {
    let package_update_line = if package_update {
        "package_update: true\n"
    } else {
        ""
    };
    format!(
        r#"#cloud-config
user: {user}
password: {password}
lock_passwd: false
chpasswd: {{ expire: True }}
ssh_pwauth: False
{package_update_line}packages:
  - curl
{resolv_conf}
write_files:
{write_files}
runcmd:
  - /usr/local/sbin/veyron-install-guestkit.sh
  - [ systemctl, daemon-reload ]
  - [ systemctl, enable, guestkit-agent ]
  - [ systemctl, start, guestkit-agent ]
"#,
        resolv_conf = guestkit_resolv_conf_yaml(),
        write_files = guestkit_write_files_block(),
    )
}

fn default_cloud_init() -> String {
    linux_guestkit_cloud_init("veyron", &generate_random_password(), true)
}

fn default_ubuntu_cloud_init() -> String {
    linux_guestkit_cloud_init("ubuntu", &generate_random_password(), true)
}

/// Cloud-config keys whose lists are concatenated (template first) instead of replaced.
const CLOUD_INIT_APPEND_KEYS: &[&str] = &[
    "packages",
    "write_files",
    "runcmd",
    "bootcmd",
    "ssh_authorized_keys",
    "users",
    "mounts",
];

/// Merge caller user-data into a template's cloud-config so the GuestKit bootstrap
/// survives. A `#!` script runs after the template's commands; anything that is not
/// a cloud-config mapping replaces the template payload as before.
pub fn merge_cloud_init(base: &str, custom: &str) -> String {
    use serde_yml::{Mapping, Value};
    let custom_map: Mapping = if custom.trim_start().starts_with("#!") {
        let mut m = Mapping::new();
        let mut file = Mapping::new();
        file.insert("path".into(), "/usr/local/sbin/veyron-user-data.sh".into());
        file.insert("permissions".into(), "0755".into());
        file.insert("content".into(), custom.into());
        m.insert(
            "write_files".into(),
            Value::Sequence(vec![Value::Mapping(file)]),
        );
        m.insert(
            "runcmd".into(),
            Value::Sequence(vec!["/usr/local/sbin/veyron-user-data.sh".into()]),
        );
        m
    } else {
        match serde_yml::from_str::<Value>(custom) {
            Ok(Value::Mapping(m)) => m,
            _ => return custom.to_string(),
        }
    };
    let Ok(Value::Mapping(mut merged)) = serde_yml::from_str::<Value>(base) else {
        return custom.to_string();
    };
    for (k, v) in custom_map {
        let append = k
            .as_str()
            .is_some_and(|k| CLOUD_INIT_APPEND_KEYS.contains(&k));
        match (merged.get_mut(&k), v) {
            (Some(Value::Sequence(existing)), Value::Sequence(extra)) if append => {
                existing.extend(extra)
            }
            (_, v) => {
                merged.insert(k, v);
            }
        }
    }
    match serde_yml::to_string(&Value::Mapping(merged)) {
        Ok(body) => format!("#cloud-config\n{body}"),
        Err(_) => custom.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager() {
        let manager = TemplateManager::new();

        assert!(manager.exists("ubuntu"));
        assert!(manager.exists("ubuntu-22.04"));
        assert!(manager.exists("fedora"));
        assert!(manager.exists("almalinux"));
        assert!(manager.exists("rocky"));
        assert!(!manager.exists("nonexistent"));

        let ubuntu = manager.get("ubuntu").unwrap();
        assert_eq!(ubuntu.cpu.cores, 2);
        assert_eq!(ubuntu.memory.size, "4Gi");
    }

    #[test]
    fn merge_cloud_init_keeps_guestkit_bootstrap() {
        let base = default_ubuntu_cloud_init();
        let merged = merge_cloud_init(
            &base,
            "#cloud-config\npackages:\n  - jq\nruncmd:\n  - echo hi\nhostname: llm\n",
        );
        let v: serde_yml::Value = serde_yml::from_str(&merged).unwrap();
        let b: serde_yml::Value = serde_yml::from_str(&base).unwrap();
        let strs = |v: &serde_yml::Value| -> Vec<String> {
            v.as_sequence()
                .map(|s| {
                    s.iter()
                        .filter_map(|p| p.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut want = strs(&b["packages"]);
        want.push("jq".into());
        assert_eq!(strs(&v["packages"]), want);
        let run = v["runcmd"].as_sequence().unwrap();
        assert_eq!(
            run.first(),
            b["runcmd"].as_sequence().and_then(|r| r.first())
        );
        assert_eq!(run.last().and_then(|r| r.as_str()), Some("echo hi"));
        assert_eq!(v["hostname"].as_str(), Some("llm"));
        assert!(merged.starts_with("#cloud-config\n") && merged.contains("guestkit-agent"));

        let script = merge_cloud_init(&base, "#!/bin/sh\necho x\n");
        assert!(script.contains("veyron-user-data.sh") && script.contains("guestkit-agent"));

        assert_eq!(merge_cloud_init(&base, "just text"), "just text");
    }

    #[test]
    fn test_list_templates() {
        let manager = TemplateManager::new();
        let templates = manager.list();

        for name in [
            "ubuntu-26.04",
            "debian-13",
            "fedora-44",
            "centos-stream-10",
            "almalinux-10",
            "rocky-10",
            "windows-2025",
            "windows-11",
        ] {
            assert!(templates.contains(&name.to_string()), "missing {name}");
        }
        for retired in [
            "ubuntu-18.04",
            "ubuntu-20.04",
            "centos-stream-8",
            "debian-11",
            "almalinux-8",
            "rocky-8",
            "windows-10",
            "alpine",
            "rhel",
            "freebsd",
        ] {
            assert!(
                !templates.contains(&retired.to_string()),
                "{retired} should be retired"
            );
        }
    }

    #[test]
    fn test_family_aliases_point_at_newest_release() {
        let manager = TemplateManager::new();
        let version = |n: &str| manager.get(n).unwrap().labels.get("os.version").cloned();
        assert_eq!(version("ubuntu"), Some("26.04".into()));
        assert_eq!(version("debian"), Some("13".into()));
        assert_eq!(version("windows"), Some("2025".into()));
    }

    #[test]
    fn test_rocky_clones_kryton_golden_image() {
        let cfg = TemplateManager::new().get("rocky-10").unwrap();
        match &cfg.disks[0].source {
            DiskSource::GoldenImage {
                name, namespace, ..
            } => {
                assert_eq!(name, "rocky-10");
                assert_eq!(namespace, crate::api::kryton::KRYTON_IMAGE_NAMESPACE);
            }
            other => panic!("expected golden image root disk, got {other:?}"),
        }
    }

    #[test]
    fn test_list_by_family() {
        let manager = TemplateManager::new();
        let families = manager.list_by_family();

        assert!(families.contains_key("Ubuntu"));
        assert!(families.contains_key("Fedora"));
        assert!(families.contains_key("AlmaLinux"));
        assert!(families.contains_key("Rocky Linux"));
        assert!(!families.contains_key("Other"));

        let ubuntu_templates = families.get("Ubuntu").unwrap();
        assert_eq!(ubuntu_templates.len(), 4);
    }

    #[test]
    fn test_guestkit_binary_urls_always_include_cluster_service() {
        let urls = guestkit_binary_urls();
        assert!(
            urls.iter()
                .any(|u| u.contains("veyron-api.veyron-system.svc"))
        );
    }

    #[test]
    fn test_refresh_guestkit_cloud_init_noop_without_marker() {
        let plain = "#cloud-config\nuser: ubuntu\n";
        assert_eq!(refresh_guestkit_cloud_init(plain), plain);
    }

    #[test]
    fn test_ubuntu_template_includes_guestkit_bootstrap() {
        let manager = TemplateManager::new();
        let cfg = manager.get("ubuntu-22.04").unwrap();
        let cloud_init = cfg
            .cloud_init
            .as_ref()
            .map(|c| c.user_data.as_str())
            .unwrap_or("");
        assert!(cloud_init.contains("guestkit-agent"));
        assert!(cloud_init.contains("veyron-install-guestkit.sh"));
    }
}
