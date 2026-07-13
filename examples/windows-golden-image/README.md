<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
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
pre-built image (the Packer path in `docs/WINDOWS_PACKER_GITOPS_PIPELINE.md`) avoids.
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

5. **Windows 11 24H2 Setup + custom autounattend** can fail early with
   `0xD000A000 - 0x40031` (before setupact.log is written) — a 24H2 Setup-engine
   answer-file compatibility issue that needs iterative tuning. This is the strongest
   argument for the **Packer pre-built-image** pipeline over interactive ISO installs
   for production: build and validate the image once, then CDI-import a known-good
   qcow2 and skip Setup entirely.

## Notes

- The answer file bypasses Win11's Setup hardware checks with `LabConfig` registry
  keys. This is Microsoft's documented Setup escape hatch, not a licensing bypass —
  KubeVirt provides a genuine emulated TPM 2.0 and Secure Boot.
- The `ProductKey` is the public KMS client setup key; it only lets Setup proceed
  without prompting and activates nothing.
- The local administrator password in the answer file (`V3yron-Build!`) is for the
  builder only. Real tenant VMs get their own per-VM sysprep Secret (see
  `docs/WINDOWS_GOLDEN_IMAGES.md`).
- Windows 11 needs 4 GiB RAM minimum; this builder requests 6 GiB. A single-node lab
  cluster must have the headroom free before you start.
