#!/usr/bin/env python3
"""
Patch a KubeVirt VirtualMachine YAML to use cloudInitConfigDrive for cloudinitdisk.

Typical flow:
  vmrogue generate my-win --template windows-11 --kubevirt --memory 8Gi -o vm.yaml
  ./patch_kubevirt_configdrive.py vm.yaml my-win win11-golden-dv

Requires: pip install pyyaml

The windows-* templates do not set cloud_init, so generated VMs have no cloudinit
volume — this script adds volumes[].cloudInitConfigDrive and the matching disk.
"""
from __future__ import annotations

import argparse
import sys

try:
    import yaml
except ImportError:
    print("Install PyYAML: pip install pyyaml", file=sys.stderr)
    sys.exit(1)

DEFAULT_USERDATA = """#ps1_sysnative
New-Item -ItemType Directory -Force C:\\bootstrap | Out-Null
Set-Content -Path C:\\bootstrap\\vmrogue-gitops.txt -Value 'Patched cloudInitConfigDrive'
"""


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("path", help="Path to VirtualMachine YAML (single document)")
    p.add_argument("vm_name", help="metadata.name for the VM")
    p.add_argument("golden_dv", help="DataVolume name for rootdisk")
    p.add_argument(
        "--userdata-file",
        help="Path to file whose contents become userData (UTF-8)",
        default=None,
    )
    args = p.parse_args()

    userdata = DEFAULT_USERDATA
    if args.userdata_file:
        with open(args.userdata_file, encoding="utf-8") as f:
            userdata = f.read()

    with open(args.path, encoding="utf-8") as f:
        doc = yaml.safe_load(f)

    if not isinstance(doc, dict) or doc.get("kind") != "VirtualMachine":
        sys.exit("YAML root must be a single VirtualMachine document")

    doc.setdefault("metadata", {})["name"] = args.vm_name

    spec = doc.setdefault("spec", {}).setdefault("template", {}).setdefault("spec", {})
    domain = spec.setdefault("domain", {})
    devices = domain.setdefault("devices", {})
    disks = devices.setdefault("disks", [])

    # Remove any existing cloudinitdisk device entries
    disks[:] = [d for d in disks if d.get("name") != "cloudinitdisk"]
    disks.append(
        {
            "name": "cloudinitdisk",
            "disk": {"bus": "virtio"},
        }
    )

    volumes = spec.setdefault("volumes", [])
    volumes[:] = [v for v in volumes if v.get("name") != "cloudinitdisk"]

    root_set = False
    for v in volumes:
        if v.get("name") == "rootdisk":
            v.clear()
            v["name"] = "rootdisk"
            v["dataVolume"] = {"name": args.golden_dv}
            root_set = True
            break
    if not root_set:
        volumes.insert(
            0,
            {"name": "rootdisk", "dataVolume": {"name": args.golden_dv}},
        )

    volumes.append(
        {
            "name": "cloudinitdisk",
            "cloudInitConfigDrive": {"userData": userdata},
        }
    )

    with open(args.path, "w", encoding="utf-8") as f:
        yaml.dump(
            doc,
            f,
            sort_keys=False,
            default_flow_style=False,
            allow_unicode=True,
        )

    print(f"Patched {args.path}", file=sys.stderr)


if __name__ == "__main__":
    main()
