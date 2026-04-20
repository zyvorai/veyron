# Windows golden image (Packer) → QCOW2 → CDI → GitOps / VMRogue

This document is the **operational companion** to [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md). It describes a **production-style** pipeline: **HashiCorp Packer** + **QEMU/KVM**, Windows **Audit Mode** / **Sysprep**, artifact **QCOW2**, **CDI** import, and **Kubernetes** manifests — including **Kustomize** and a **VMRogue** `generate` + patch workflow.

**Repo examples:** `examples/windows-kubevirt-gitops/` (DataVolume + VM YAML + `patch_kubevirt_configdrive.py`).

---

## End-to-end flow

```
Windows ISO + VirtIO ISO
    → Packer QEMU VM (WinRM)
    → Audit Mode / drivers / Cloudbase-Init (in guest)
    → Sysprep / shutdown
    → QCOW2 artifact
    → qemu-img convert / virt-sparsify (optional)
    → HTTP(S) or S3 + CDI DataVolume
    → VirtualMachine (cloudInitConfigDrive)
    → virtctl start / GitOps sync
```

---

## 1. Host dependencies (image builder)

```bash
# Fedora / RHEL family
sudo dnf install -y qemu-kvm libvirt virt-install qemu-img guestfs-tools

# Packer (pin a current release from https://developer.hashicorp.com/packer/downloads)
curl -LO "https://releases.hashicorp.com/packer/1.10.0/packer_1.10.0_linux_amd64.zip"
unzip packer_*.zip && sudo mv packer /usr/local/bin/
```

`virt-sparsify` lives in **libguestfs-tools** on many distributions.

---

## 2. Suggested repository layout (image factory)

```text
windows-image/
├── packer.pkr.hcl
├── autounattend.xml
├── scripts/
│   ├── setup.ps1
│   └── cleanup.ps1
└── iso/
    └── Win11.iso          # license-required; not redistributed by VMRogue
```

Mount the **VirtIO** ISO in Packer (secondary CDROM or attach path) so `setup.ps1` can run `virtio-win-gt-x64.exe` (or equivalent) — **without VirtIO storage/network drivers the KubeVirt VM will not boot cleanly.**

---

## 3. Packer QEMU builder (starter)

`packer.pkr.hcl` (illustrative — validate plugin versions and WinRM settings in your environment):

```hcl
packer {
  required_plugins {
    qemu = {
      version = ">= 1.0.9"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

variable "iso_path" {
  type    = string
  default = "iso/Win11.iso"
}

source "qemu" "windows" {
  iso_url      = var.iso_path
  iso_checksum = "none" # Prefer sha256 from Microsoft / your mirror

  output_directory = "output-windows"
  vm_name          = "win11-golden.qcow2"
  accelerator      = "kvm"
  format           = "qcow2"
  disk_size        = "65536" # MiB ≈ 64 GiB (adjust)

  communicator   = "winrm"
  winrm_username = "Administrator"
  winrm_password = "P@ssw0rd!" # rotate; use vault in real CI
  winrm_timeout    = "8h"

  boot_wait = "5s"

  # Autounattend delivery varies by Windows version:
  # secondary ISO, floppy, or http_directory — pick one supported by your Packer + QEMU combo.
  # floppy_files = ["autounattend.xml"]

  shutdown_command = "shutdown /s /t 10 /f"
}

build {
  sources = ["source.qemu.windows"]

  provisioner "powershell" {
    scripts = ["scripts/setup.ps1"]
  }

  provisioner "powershell" {
    scripts = ["scripts/cleanup.ps1"]
  }
}
```

**WinRM** must be enabled early (typically via `autounattend.xml` **FirstLogonCommands** or `setup.ps1`) or Packer will hang.

---

## 4. `autounattend.xml` (boot into Audit Mode)

A **minimal** pattern is: **Reseal → Audit** in `auditSystem`, plus **AutoLogon** for the Administrator used by Packer. Real files need correct `processorArchitecture`, `publicKeyToken`, and component names for your ISO — generate once with Windows SIM or a known-good baseline, then store in **Git** (no secrets) or a **private** artifact bucket.

```xml
<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend">
  <settings pass="oobeSystem">
    <component name="Microsoft-Windows-Shell-Setup" processorArchitecture="amd64"
               publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS">
      <AutoLogon>
        <Username>Administrator</Username>
        <Enabled>true</Enabled>
        <LogonCount>5</LogonCount>
      </AutoLogon>
    </component>
  </settings>
  <settings pass="auditSystem">
    <component name="Microsoft-Windows-Deployment" processorArchitecture="amd64"
               publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS">
      <Reseal>
        <Mode>Audit</Mode>
      </Reseal>
    </component>
  </settings>
</unattend>
```

---

## 5. Guest scripts

### `scripts/setup.ps1`

- Enable **WinRM** for Packer.
- Install **VirtIO guest tools** from the mounted VirtIO ISO (path depends on QEMU block device letters — often `D:\` or `E:\`).
- Optionally install **Cloudbase-Init** MSI and lay down `cloudbase-init.conf` as described in [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md).

### `scripts/cleanup.ps1`

- Remove build caches if desired.
- Run **`sysprep /generalize /oobe /shutdown`** with an `unattend.xml` appropriate for your org. Packer should detect shutdown and finalize the QCOW2.

---

## 6. Build and shrink

```bash
packer init .
packer build .
qemu-img convert -O qcow2 -c output-windows/win11-golden.qcow2 win11-golden.sparse.qcow2
virt-sparsify win11-golden.sparse.qcow2 win11-golden.final.qcow2   # needs libguestfs
```

Publish `win11-golden.final.qcow2` to **HTTPS**, **S3**, or an **internal artifact registry** your CDI installation can reach.

---

## 7. CDI `DataVolume` + `VirtualMachine`

See **`examples/windows-kubevirt-gitops/kustomize/`**:

- `kustomize/example/datavolume.yaml` — HTTP import template.
- `kustomize/example/vm.yaml` — **`cloudInitConfigDrive`** with `#ps1_sysnative` userData (Cloudbase-Init–friendly).

**Important:** `userData` for Windows + Cloudbase-Init is usually **PowerShell** with a `#ps1_sysnative` shebang, not Linux **`#cloud-config`**. If you use Cloudbase’s multi-part MIME features, generate that payload in CI.

**KubeVirt `sysprep` volume** (inject `Autounattend.xml` from a Secret) exists in some versions — validate against your KubeVirt CRD schema before relying on it in automation.

---

## 8. VMRogue: `generate` + config-drive patch

VMRogue’s **`cloud_init`** field maps to **`cloudInitNoCloud`** in `src/kube/converter.rs` (Linux-oriented). **Windows** first boot with Cloudbase-Init on KubeVirt typically uses **`cloudInitConfigDrive`**.

Windows **`windows-*`** templates do **not** set `cloud_init`, so:

```bash
vmrogue generate win11-vm --template windows-11 --kubevirt --memory 8Gi -o vm.yaml
```

produces a VM **without** a `cloudinitdisk` volume. Use the example script to **add** `cloudInitConfigDrive`:

```bash
pip install pyyaml
python3 examples/windows-kubevirt-gitops/scripts/patch_kubevirt_configdrive.py \
  vm.yaml win11-vm win11-golden-dv --userdata-file ./first-boot.ps1
```

Or the wrapper:

```bash
bash examples/windows-kubevirt-gitops/scripts/generate-and-patch.sh ./vm.yaml win11-vm win11-golden-dv windows-11
```

Then **`kubectl apply -f vm.yaml`** (set `metadata.namespace` or use `-n`).

---

## 9. GitOps (Argo CD / Flux)

- Keep **`DataVolume` + `VirtualMachine`** (and **Secrets** for sysprep/userData) in Git.
- Prefer **External Secrets** / **Sealed Secrets** over committing passwords or KMS keys.
- Optional **`Application`** manifest pointing at the overlay path — same pattern as any other Kustomize app.

---

## 10. Common failures

| Issue | Mitigation |
|-------|------------|
| VM does not boot on KubeVirt | Install correct **VirtIO block + NetKVM** in the golden image for the bus you use (`virtio` vs `sata` during install). |
| Packer hangs | **WinRM** firewall / credentials; confirm `autounattend` actually enables remoting. |
| Cloudbase does nothing | Wrong **`cloudbase-init.conf`** metadata path; use **`cloudInitConfigDrive`**, not `cloudInitNoCloud`, for this pattern. |
| Clones collide | Run **Sysprep generalize** before publishing the golden disk. |

---

## 11. Architecture sketch

```
CI (GitHub Actions / Tekton)
  → Packer + QEMU
  → QCOW2 artifact to object storage
  → (optional) cosign / checksum attestation
  → kubectl / Argo CD applies DataVolume + VM
  → virtctl start
  → VMRogue dashboard / API for lifecycle + VNC
```

For **developer access** after boot, see [DEVELOPER_VM_ACCESS.md](./DEVELOPER_VM_ACCESS.md).
