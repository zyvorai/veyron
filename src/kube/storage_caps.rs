// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! StorageClass provisioner classification shared by the platform-capabilities
//! probe (`handlers/operators.rs`) and the migration eligibility guard
//! (`kube/migration_guard.rs`).

/// Provisioners whose volumes are pinned to a single node: a VM on one of these
/// cannot live-migrate and loses its disk if the node dies.
pub fn is_node_local_provisioner(provisioner: &str) -> bool {
    matches!(
        provisioner,
        "rancher.io/local-path"
            | "kubernetes.io/no-provisioner"
            | "openebs.io/local"
            | "topolvm.io"
    )
}

/// Provisioners that can back a `ReadWriteMany` volume — the KubeVirt
/// prerequisite for live migration of a PVC-backed VM.
pub fn is_rwx_capable_provisioner(provisioner: &str) -> bool {
    provisioner.contains("cephfs")
        || provisioner.contains("nfs")
        || provisioner.contains("glusterfs")
        || provisioner.contains("azurefile")
        || provisioner.contains("efs")
}
