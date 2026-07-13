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
