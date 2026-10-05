<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Windows golden-image E2E

Reference assets for `scripts/test-windows-golden-image-remote.sh`, which drives the
full workflow through the Veyron API: import a Windows ISO, install + sysprep-seal it
unattended, publish it as a versioned golden image, then clone a tenant VM from it.

| File | Role |
|---|---|
| `autounattend-builder.xml` | Fully unattended Setup: injects the `viostor` VirtIO driver in WinPE, partitions UEFI (ESP+MSR+OS), bypasses the Win11 TPM/SecureBoot/CPU Setup checks (the guest *has* a real vTPM 2.0 + Secure Boot — these only stop Setup nagging), installs VirtIO + QEMU guest agent on first logon, then runs `sysprep /generalize /shutdown`. |
| `win-builder.yaml` | Raw KubeVirt builder VM: blank root disk + ISO CD + pinned VirtIO CD + the sysprep answer-file CD. `runStrategy: RerunOnFailure` so a clean sysprep shutdown is not restarted. |

## Run

```bash
VEYRON_API_KEY='...' \
WIN_ISO_URL='https://.../Win11_EnterpriseLTSC_x64.iso' \
  ./scripts/test-windows-golden-image-remote.sh <host> <ssh-user>
```

The ISO URL must be reachable **from the cluster node** — CDI pulls it directly, not
via your workstation.

## Headless ISO-install gotchas (learned the hard way on a live cluster)

Installing Windows from an ISO on headless KubeVirt has friction points that a
pre-built image (the Packer path in `docs/windows.md`) avoids.
For a lab/one-off ISO build, know these:

1. **"Press any key to boot from CD or DVD….."** — the Microsoft ISO shows this for
   ~5 s and, with no key pressed, times out and Setup never starts (the VM then halts
   at OVMF "No bootable option or device was found"). Headless, you must inject a
   keypress during that window over VNC, or the disk stays empty. The scratch tooling
   used here connects to the KubeVirt VNC and sends Enter; production should prefer a
   pre-built image so this never arises.

2. **UEFI fresh NVRAM on clones** — see the EFI fallback bootloader step in the answer
   file. A cloned VM has empty EFI NVRAM and halts at "no bootable device" unless the
   golden image carries `\EFI\Boot\bootx64.efi`.

3. **Clone volumeMode** — clone Filesystem→Filesystem (matching the golden image).
   A cross-mode clone falls back to a slow host-assisted copy that also fails on some
   Ceph RBD with `/dev/cdi-block-volume: Permission denied`. Veyron's converter now
   defaults golden-image clones to Filesystem.

4. **Cross-namespace clone RBAC** binds the TENANT namespace's `default` SA, not the
   Veyron API SA — see `deploy/k8s/bootstrap/cdi-golden-image-cloner.yaml`.

5. **`0xD000A000 - 0x40031` = the windowsPE driver-injection block.** Confirmed from
   `X:\$WINDOWS.~BT\Sources\Panther\setuperr.log`: it fails in
   `ExecuteUnattendDriverInstall`. A `PnpCustomizationsWinPE` `DriverPaths` block
   pointing at CD-ROM roots makes 24H2 Setup abort. The answer file here has NO
   windowsPE driver injection — the builder root disk is **SATA** (WinPE sees it
   natively), and VirtIO drivers are installed at first logon instead.

6. **Win11 24H2's redesigned Setup shows the first two locale screens (language,
   keyboard) interactively even with a full answer file** — a known 24H2 regression.
   Drive past them (2× Enter, Next is the focused default); the answer file then
   auto-handles product key/edition, disk partitioning, image install, OOBE, and
   sysprep with no further input. For fully hands-off production, prefer the Packer
   pre-built-image pipeline (`docs/windows.md`).

## Notes

- The answer file bypasses Win11's Setup hardware checks with `LabConfig` registry
  keys. This is Microsoft's documented Setup escape hatch, not a licensing bypass —
  KubeVirt provides a genuine emulated TPM 2.0 and Secure Boot.
- The `ProductKey` is the public KMS client setup key; it only lets Setup proceed
  without prompting and activates nothing.
- The local administrator password in the answer file (`V3yron-Build!`) is for the
  builder only. Real tenant VMs get their own per-VM sysprep Secret (see
  `docs/windows.md`).
- Windows 11 needs 4 GiB RAM minimum; this builder requests 6 GiB. A single-node lab
  cluster must have the headroom free before you start.

## Generalizing on Windows 11 24H2 — two hard walls (measured on the cluster)

Producing a **sysprep-generalized** golden image from an ISO install on headless
24H2 hits two Windows-side blockers that no Veyron change can remove:

1. **Interactive OOBE overrides the answer file.** 24H2's redesigned OOBE shows the
   locale/account screens even with a complete `oobeSystem` pass, so the auto-sysprep
   `FirstLogonCommands` never run, and OOBE creates an account with a password that is
   NOT the one in the answer file — leaving the built image un-loginable headlessly.

2. **BitLocker device encryption is auto-enabled.** With a vTPM present (which KubeVirt
   provides), 24H2 encrypts the OS partition on install (`blkid` reports
   `TYPE="BitLocker"`). That blocks the offline fallback (mount the disk, clear the
   password / inject a RunOnce sysprep) — the partition can't be mounted without the
   recovery key.

Net: a *force-stopped* 24H2 ISO-install image **boots and clones fine** (proven), but
cannot be generalized after the fact. For a generalized image:

- **Recommended — Packer pre-built image** (`docs/windows.md`):
  build + sysprep once in a controlled VM, export a qcow2, and `POST /images/import`
  it. Sidesteps OOBE, BitLocker, and credentials entirely.
- **Or, for a headless ISO build**, the answer file must (a) set
  `HKLM\SYSTEM\CurrentControlSet\Control\BitLocker\PreventDeviceEncryption=1`
  before first boot so the disk stays offline-editable, and (b) reliably run sysprep
  in the build — which on 24H2 means driving OOBE, since its interactive setup does
  not honor the `oobeSystem` auto-seal.
