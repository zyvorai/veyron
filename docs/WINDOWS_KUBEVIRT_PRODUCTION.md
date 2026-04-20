# Production-ready Windows VMs on KubeVirt

This guide consolidates **golden image** preparation (Audit Mode, Sysprep), **VirtIO** and **Cloudbase-Init** setup for KubeVirt’s config drive, **YAML** deployment patterns, and day‑two operations (licensing, drivers, tuning). It complements VMRogue’s built‑in Windows templates in `src/templates/mod.rs`—validate every field against your **KubeVirt** and **Kubernetes** versions before production use.

### What is already in VMRogue’s Rust templates

The `windows`, `windows-2022`, `windows-2019`, `windows-11`, and `windows-10` entries in **`src/templates/mod.rs`** (via `VMConfigBuilder` and **`src/kube/converter.rs`**) encode:

| Area | In code? | Where |
|------|-----------|--------|
| Hyper‑V enlightenments (relaxed, vAPIC, spinlocks, synic, timers, …) | Yes | `windows_features()` |
| SMM enabled alongside firmware choices | Yes | `windows_features()` + `FirmwareConfig` |
| Hyper‑V **timer** / clock policy | Yes | `windows_clock()` |
| UEFI (Secure Boot on for Win11 template) | Yes | `uefi_firmware()` / `uefi_secure_boot_firmware()` |
| `q35` machine type | Yes | `.machine_type("q35")` |
| VirtIO **driver** CDROM (`virtio-container-disk`) | Yes | `.add_cdrom("virtio-drivers", …)` |
| VirtIO **NIC** (Windows path) | Yes | `.add_windows_network(...)` |
| virtio‑rng | Yes | `.enable_rng()` |
| USB tablet (VNC pointer) | Yes | `.usb_tablet()` |
| virtio balloon disabled | Yes | `.disable_balloon()` |
| IOThreads policy | Yes | `.io_threads_policy("shared")` |
| TPM (selected SKUs) | Yes | `.enable_tpm()` on 2022 and 11 |
| Blank install disk (SATA) | Yes | `.add_blank_disk_sata(...)` — **not** the same as a generalized golden PVC; that remains your image pipeline |

**Not in VMRogue source** (by design — guest or pipeline work): Audit Mode / **Sysprep**, **Cloudbase‑Init** `cloudbase-init.conf`, **`cloudInitConfigDrive`** `userData` snippets, **KMS/slmgr** automation, registry driver‑block lists, **`host-passthrough`** / **`dedicatedCpuPlacement`** defaults for Windows (you can still set CPU model / sockets in `VMConfig` / YAML when you generate manifests). Use this document and your GitOps for those layers.

---

## Part 1: Prerequisites and planning

- **KubeVirt**: Healthy cluster on a supported release (check your vendor matrix; treat `v1.0+` as a planning baseline, not a substitute for reading release notes).
- **virtctl**: Install a build compatible with your KubeVirt version ([KubeVirt user guide — accessing VMs](https://kubevirt.io/user-guide/user_workloads/accessing_virtual_machines/)).
- **Media**: Windows Server (or desktop) ISO, **VirtIO drivers** ISO (`virtio-win-…`), and the **Cloudbase-Init** installer (run inside the guest during image prep).
- **VM sizing (minimums for install + Sysprep)**:
  - **vCPU**: at least **4** for a smooth experience.
  - **Memory**: at least **8 GiB** guest RAM.
  - **Disk**: at least **50 GiB** for OS, updates, and modest applications.
- **Host**: Hardware virtualization (Intel VT‑x / AMD‑V) enabled; node hypervisor stack (e.g. KVM/QEMU) healthy per your platform docs.

---

## Part 2: Preparing the Windows golden image

Goal: a **generalized** disk image you can clone into new `VirtualMachine` definitions.

### Step 1: Initial install and Audit Mode

1. Create a VM that boots from the Windows ISO (CDROM first in boot order during install).
2. Complete Windows setup until the **OOBE welcome** screen.
3. Press **`Ctrl+Shift+F3`** to enter **Audit Mode** (skips full OOBE so you can customize the image).
4. When the **Sysprep** dialog appears, **leave it open** in the background while you perform the next steps.

### Step 2: VirtIO drivers and Cloudbase-Init

In Audit Mode:

1. **VirtIO drivers**  
   Attach the VirtIO ISO and install drivers so the generalized image boots on virtio devices:
   - **Storage**: install **VirtIO SCSI** (`vioscsi`) **or** the **VirtIO Block** (`viostor`) driver set, depending on whether your final VM disk is presented as SCSI or virtio-blk. Mismatch here causes **INACCESSIBLE_BOOT_DEVICE** or “no bootable device” after clone.
   - **Network**: install **NetKVM** so the guest has networking after first boot.

2. **Cloudbase-Init**  
   Run the installer with typical choices:
   - Admin user (often `Administrator`).
   - **Run Cloudbase-Init as Local System** (when offered).
   - **Configuration drive**: **NoCloud** / config‑drive style options as appropriate for your installer version.

### Step 3: Configure Cloudbase-Init for KubeVirt

Edit:

`C:\Program Files\Cloudbase Solutions\Cloudbase-Init\conf\cloudbase-init.conf`

Align metadata discovery with the **config drive** KubeVirt attaches. A common template:

```ini
[DEFAULT]
username=Administrator
groups=Administrators
inject_user_password=true
config_drive_path=C:\OpenStack\configdrive\
config_drive_raw_hdd=true
log_dir=C:\Program Files\Cloudbase Solutions\Cloudbase-Init\log\
log_file=cloudbase-init.log
default_log_levels=comtypes=INFO,suds=INFO,iso8601=WARN,requests=WARN
logging_serial_port_settings=
mtu_use_dhcp_config=true
netbios_host_name_conf=True
bsdtar_path=C:\Program Files\Cloudbase Solutions\Cloudbase-Init\bin\bsdtar.exe
mtools_path=C:\Program Files\Cloudbase Solutions\Cloudbase-Init\bin\
metadata_services=cloudbaseinit.metadata.services.nocloudservice.NoCloudConfigDriveService
plugins=cloudbaseinit.plugins.common.sethostname.SetHostNamePlugin,cloudbaseinit.plugins.common.userdata.UserDataPlugin,cloudbaseinit.plugins.windows.extendvolumes.ExtendVolumesPlugin
allow_reboot=false
first_logon_behaviour=no
```

Notes:

- **`config_drive_path`**: Points Cloudbase-Init at the mounted config‑drive content. `C:\OpenStack\configdrive\` is widely used with **`cloudInitConfigDrive`** style volumes in KubeVirt examples—confirm on a test VM if hostname or user-data is not applied.
- **`config_drive_raw_hdd`**: Use the spelling **`config_drive_raw_hdd`** (some older snippets typo this as `hhd`).
- **`metadata_services`**: `NoCloudConfigDriveService` matches NoCloud/config‑drive style metadata.
- **`plugins`**: Ensures hostname, user-data scripts, and **volume extend** run so the disk grows to the PVC size on first boot.

### Step 4: Generalize with Sysprep

When drivers, updates, and Cloudbase-Init are complete:

1. Close unnecessary applications.
2. In the Sysprep UI:
   - **System cleanup action**: **Enter System Out-of-Box Experience (OOBE)**.
   - **Shutdown options**: **Shutdown**.
   - Enable **Generalize**.
3. Confirm and let Sysprep finish; the VM powers off.

Capture or retain the root disk as your **golden** PVC or image (snapshot workflow depends on your storage / CSI stack).

---

## Part 3: Deploying on KubeVirt (example manifest)

Below is an illustrative **`VirtualMachine`** using a pre-populated PVC for the golden image and a **`cloudInitConfigDrive`** for first-boot customization. Replace names, sizes, and feature flags with values validated in your environment.

```yaml
apiVersion: kubevirt.io/v1
kind: VirtualMachine
metadata:
  name: windows-server-2022
  namespace: default
  labels:
    app: windows-server
    version: v1
spec:
  runStrategy: Always
  template:
    metadata:
      labels:
        kubevirt.io/domain: windows-server-2022
    spec:
      domain:
        cpu:
          cores: 4
          model: host-passthrough
          dedicatedCpuPlacement: true
        memory:
          guest: 8Gi
        machine:
          type: q35
        firmware:
          bootloader:
            efi:
              secureBoot: false
        features:
          acpi: {}
          apic: {}
          hyperv:
            relaxed: {}
            vapic: {}
            spinlocks:
              spinlocks: 8191
          smm:
            enabled: true
        devices:
          rng: {}
          inputs:
            - type: tablet
              bus: usb
          disks:
            - name: rootdisk
              disk:
                bus: virtio
              bootOrder: 2
            - name: cloudinitdisk
              disk:
                bus: virtio
          interfaces:
            - name: default
              model: virtio
              masquerade: {}
      networks:
        - name: default
          pod: {}
      volumes:
        - name: rootdisk
          persistentVolumeClaim:
            claimName: windows-golden-image-pvc
        - name: cloudinitdisk
          cloudInitConfigDrive:
            userData: |
              #ps1_sysnative
              net user Administrator "REPLACE_WITH_SECURE_PASSWORD"
              Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" -Name "AutoAdminLogon" -Value 0
              Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" -Name "DefaultUserName" -Value ""
              Write-Host "Initial setup complete."
```

**Field notes**

| Area | Guidance |
|------|------------|
| **CPU** | `host-passthrough` exposes host CPU features to the guest; best when nodes are homogeneous. Prefer **`host-model`** (or a fixed model) if you need **live migration** across mixed CPU generations. |
| **`dedicatedCpuPlacement`** | Reserves physical CPUs; requires sufficient allocatable cores and compatible CPU manager policy on nodes. |
| **UEFI / SMM** | EFI is typical for modern Windows. `secureBoot: false` avoids SB enrollment in this example; enabling Secure Boot needs matching firmware/keys. **`smm`** is often required alongside EFI—confirm with your KubeVirt version. |
| **I/O and UX** | **`rng`** improves entropy. **USB tablet** improves pointer behavior in **VNC**. **VirtIO** disk and NIC reduce overhead vs emulated devices. |
| **`cloudInitConfigDrive`** | Supplies `userData` on a config drive—pair with the Cloudbase-Init settings above. **Do not** commit real passwords; inject via secrets/ExternalSecrets in real pipelines. |

---

## Part 4: Post-installation and operations

### License activation

Sysprep affects activation state. Example `userData` fragment (replace key material and validate legally):

```yaml
userData: |
  #ps1_sysnative
  slmgr.vbs /ipk YOUR-PRODUCT-KEY
  slmgr.vbs /ato
```

Enterprise deployments often use KMS, AD-based activation, or `unattend.xml` during image build instead of clear-text keys in manifests.

### Driver persistence (optional hardening)

To reduce the chance of unwanted driver swaps from Windows Update, some teams apply policy/registry controls. Example pattern (test before wide rollout):

```powershell
$registryPath = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeviceInstall\Restrictions"
New-Item -Path $registryPath -Force | Out-Null
Set-ItemProperty -Path $registryPath -Name "DenyDeviceIDs" -Value 1 -Type DWord
Set-ItemProperty -Path $registryPath -Name "DenyDeviceIDsRetroactive" -Value 1 -Type DWord

$denyPath = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeviceInstall\Restrictions\DenyDeviceIDs"
New-Item -Path $denyPath -Force | Out-Null
Set-ItemProperty -Path $denyPath -Name "1" -Value "PCI\VEN_1AF4&DEV_1000" -Type String
Set-ItemProperty -Path $denyPath -Name "2" -Value "PCI\VEN_1AF4&DEV_1001" -Type String
Set-ItemProperty -Path $denyPath -Name "3" -Value "PCI\VEN_1AF4&DEV_1042" -Type String
Set-ItemProperty -Path $denyPath -Name "4" -Value "PCI\VEN_1AF4&DEV_1041" -Type String
```

Adjust PCI IDs to match the VirtIO devices you actually expose.

---

## Part 5: Performance and troubleshooting

### Performance tuning

- **CPU pinning**: `dedicatedCpuPlacement: true` when latency predictability matters.
- **IOThreads** (storage-heavy guests), example fragment:

```yaml
domain:
  ioThreadsPolicy: auto
  devices:
    disks:
      - name: rootdisk
        disk:
          bus: virtio
        dedicatedIOThread: true
```

- **VirtIO RNG**: Low cost; helps avoid entropy-related stalls.
- **Clock / timers**: VMRogue’s Windows templates set Hyper-V-related timer defaults in code—mirror or extend in your manifest if guests show time drift.

### Common issues

| Symptom | Likely cause |
|---------|----------------|
| **No bootable device** / **INACCESSIBLE_BOOT_DEVICE** | Wrong or missing **VirtIO storage** driver for the bus your disk uses (`viostor` vs `vioscsi`). During setup, use **Load driver** from the VirtIO ISO. |
| **No network** | **NetKVM** not installed or disabled. |
| **High idle CPU / sluggish guest** | Missing virtio drivers (balloon, rng, etc.) or suboptimal timer/Hyper-V feature set. |
| **Cloudbase-Init ignores hostname / user-data** | Wrong **`config_drive_path`** or metadata service selection; verify the mounted path inside the running VM. |

---

## Checklist summary

1. Build and patch Windows in **Audit Mode**; install **VirtIO** + **Cloudbase-Init**.
2. Fix **`cloudbase-init.conf`** for **NoCloud config drive** and desired **plugins**.
3. **Sysprep (generalize + shutdown)** and publish the golden disk to PVC/image your platform supports.
4. Deploy **`VirtualMachine`** with **UEFI/virtio/hyperv** settings appropriate for your cluster.
5. Automate **secrets**, **activation**, and **driver policy** outside of hand-edited YAML where possible.

For **developer access** (VNC, RDP, or SSH where applicable), see [DEVELOPER_VM_ACCESS.md](./DEVELOPER_VM_ACCESS.md).
