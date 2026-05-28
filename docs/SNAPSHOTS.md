# 📸 VM Snapshots & Backup System

**See also:** [Documentation index](README.md), [DISK_MANAGEMENT.md](DISK_MANAGEMENT.md) for storage workflows.

VMRogue's VM Snapshots & Backup System provides production-grade snapshot management for disaster recovery and VM lifecycle management.

---

## 🎯 Features

✅ **Create Snapshots** - Capture VM state at any point in time
✅ **List & Browse** - View all snapshots with detailed status
✅ **Restore Operations** - Restore to new VM or in-place
✅ **Retention Policies** - Automatic snapshot cleanup
✅ **Health Monitoring** - Track snapshot status and readiness
✅ **Themed CLI** - Beautiful colored output with status symbols

---

## 🚀 Quick Start

###  Create a Snapshot

```bash
# Create snapshot with auto-generated name
vmrogue snapshot-create my-vm

# Create snapshot with custom name
vmrogue snapshot-create my-vm --name pre-upgrade-backup

# Create snapshot with description
vmrogue snapshot-create my-vm \
  --name before-update \
  --description "Before OS upgrade to Ubuntu 24.04"
```

### 📋 List Snapshots

```bash
# List all snapshots in namespace
vmrogue snapshot-list

# List snapshots for specific VM
vmrogue snapshot-list my-vm

# List in different formats
vmrogue snapshot-list --output yaml
vmrogue snapshot-list --output json
```

### 🔍 Get Snapshot Details

```bash
# Get snapshot details (YAML format)
vmrogue snapshot-get my-snapshot

# Get details in JSON format
vmrogue snapshot-get my-snapshot --output json
```

### 🔄 Restore from Snapshot

```bash
# Restore to new VM
vmrogue snapshot-restore my-snapshot --target restored-vm

# Restore and start immediately
vmrogue snapshot-restore my-snapshot --target restored-vm --start

# Restore in-place (overwrite existing VM)
vmrogue snapshot-restore my-snapshot --in-place

# Restore in-place with custom target
vmrogue snapshot-restore my-snapshot --target my-vm --in-place
```

### 🗑️ Delete Snapshot

```bash
# Delete snapshot (with confirmation)
vmrogue snapshot-delete old-snapshot

# Delete snapshot (skip confirmation)
vmrogue snapshot-delete old-snapshot --yes
```

---

## 📊 Command Reference

### `vmrogue snapshot-create`

Create a VM snapshot.

**Options:**
- `<VM>` - VM name (required)
- `--name, -n <NAME>` - Snapshot name (optional, auto-generated if not provided)
- `--description, -d <DESC>` - Description of the snapshot

**Examples:**
```bash
vmrogue snapshot-create prod-db --name daily-backup
vmrogue snapshot-create web-server --description "Before deployment"
```

### `vmrogue snapshot-list`

List snapshots.

**Options:**
- `[VM]` - VM name (optional, shows all snapshots if not provided)
- `--all-namespaces, -A` - Show snapshots from all namespaces
- `--output, -o <FORMAT>` - Output format (table, yaml, json)

**Examples:**
```bash
vmrogue snapshot-list                    # All snapshots
vmrogue snapshot-list prod-db            # Snapshots for prod-db
vmrogue snapshot-list -A                 # All namespaces
vmrogue snapshot-list --output json      # JSON format
```

### `vmrogue snapshot-get`

Show detailed snapshot information.

**Options:**
- `<NAME>` - Snapshot name (required)
- `--output, -o <FORMAT>` - Output format (yaml, json)

**Examples:**
```bash
vmrogue snapshot-get my-snapshot
vmrogue snapshot-get my-snapshot --output json
```

### `vmrogue snapshot-delete`

Delete a snapshot.

**Options:**
- `<NAME>` - Snapshot name (required)
- `--yes, -y` - Skip confirmation prompt

**Examples:**
```bash
vmrogue snapshot-delete old-snapshot
vmrogue snapshot-delete old-snapshot --yes
```

### `vmrogue snapshot-restore`

Restore VM from snapshot.

**Options:**
- `<SNAPSHOT>` - Snapshot name (required)
- `--target, -t <VM>` - Target VM name (optional, defaults to `<snapshot>-restored`)
- `--in-place` - Restore in-place (overwrite existing VM)
- `--start` - Start VM after restore

**Examples:**
```bash
vmrogue snapshot-restore backup-20260205 --target restored-vm
vmrogue snapshot-restore backup-20260205 --in-place
vmrogue snapshot-restore backup-20260205 --target new-vm --start
```

---

## 💡 Usage Examples

### Example 1: Daily Backup Workflow

```bash
# Create daily snapshot
vmrogue snapshot-create prod-db \
  --name "prod-db-daily-$(date +%Y%m%d)" \
  --description "Daily backup"

# List recent snapshots
vmrogue snapshot-list prod-db

# Verify snapshot is ready
vmrogue snapshot-get prod-db-daily-20260205
```

### Example 2: Pre-Deployment Backup

```bash
# Create snapshot before deployment
vmrogue snapshot-create web-server \
  --name pre-deploy-v2.0 \
  --description "Before v2.0 deployment"

# Deploy new version
# ... deploy your application ...

# If deployment fails, restore
vmrogue snapshot-restore pre-deploy-v2.0 \
  --target web-server \
  --in-place \
  --start
```

### Example 3: Test Environment Cloning

```bash
# Create snapshot of production
vmrogue snapshot-create prod-db --name prod-snapshot

# Restore to new VM for testing
vmrogue snapshot-restore prod-snapshot \
  --target test-db \
  --start

# Verify test VM
vmrogue status test-db
```

### Example 4: Disaster Recovery

```bash
# List available snapshots
vmrogue snapshot-list prod-db

# Check snapshot details
vmrogue snapshot-get prod-db-daily-20260204

# Restore to recover
vmrogue snapshot-restore prod-db-daily-20260204 \
  --target prod-db-recovered \
  --start

# Verify recovered VM
vmrogue health prod-db-recovered
```

---

## 📋 Output Examples

### Snapshot List (Table Format)

```
All Snapshots in namespace: default

NAME                           VM                   STATUS       SIZE       AGE
----------------------------------------------------------------------------------
prod-db-daily-20260205         prod-db              Running      15Gi       2h0m
prod-db-daily-20260204         prod-db              Running      15Gi       1d2h
web-server-pre-deploy          web-server           Pending      -          30m
test-db-backup                 test-db              Running      8Gi        5d
```

### Snapshot Details (Formatted)

```
Snapshot: prod-db-daily-20260205

  VM:          prod-db
  Namespace:   default
  Status:      ✓ READY
  Description: Daily backup
  Size:        15Gi
  Age:         2h30m
  Duration:    3m45s
  Ready:       Yes
```

### Snapshot Creation

```
Creating snapshot for VM: prod-db
  Snapshot name: prod-db-snapshot-20260205-140530
  Description:   Daily backup

✓ Snapshot creation started
  Status:    InProgress

ℹ Check snapshot status with:
  vmrogue snapshot-get prod-db-snapshot-20260205-140530
```

### Snapshot Restore

```
Restoring snapshot to new VM: prod-db-recovered
  Snapshot:  prod-db-daily-20260204
  Target VM: prod-db-recovered
  Start:     Yes

✓ Restore started
  Restore name: prod-db-recovered-restore
  Status:       InProgress

ℹ VM will be started after restore completes
```

---

## 🔧 Advanced Features

### Retention Policies

Snapshots support retention policies to automatically manage snapshot lifecycle:

```rust
use vmrogue::snapshots::{RetentionPolicy, SnapshotConfig};

let mut config = SnapshotConfig::new("my-vm", "snapshot-name");
config.retention = RetentionPolicy {
    max_snapshots: Some(10),     // Keep max 10 snapshots
    max_age_days: Some(30),      // Delete snapshots older than 30 days
    keep_last_n: Some(5),        // Always keep last 5 snapshots
};
```

### Snapshot Labels

Add custom labels to snapshots for organization:

```rust
use vmrogue::snapshots::SnapshotConfig;

let config = SnapshotConfig::new("my-vm", "snapshot-name")
    .with_label("env", "production")
    .with_label("backup-type", "daily")
    .with_label("app", "database");
```

### Programmatic Usage

Use snapshots in your Rust applications:

```rust
use vmrogue::snapshots::{SnapshotManager, SnapshotConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = SnapshotManager::new("default");

    // Create snapshot
    let config = SnapshotConfig::new("my-vm", "backup-20260205")
        .with_description("Automated backup");

    let snapshot = manager.create_snapshot(&config).await?;
    println!("Snapshot created: {}", snapshot.name);

    // List snapshots
    let snapshots = manager.list_snapshots_for_vm("my-vm").await?;
    for snap in snapshots {
        println!("  - {} ({}) - {}", snap.name, snap.status, snap.age());
    }

    Ok(())
}
```

---

## 📈 Integration with Other Features

### With Health Checks

```bash
# Create snapshot
vmrogue snapshot-create my-vm --name pre-update

# Check VM health before restore
vmrogue health my-vm

# Restore if needed
vmrogue snapshot-restore pre-update --in-place
```

### With Profiles

```bash
# Create VM with profile
vmrogue create my-vm --template ubuntu-22.04 --profile database

# Create snapshot
vmrogue snapshot-create my-vm --name initial-state

# Restore maintains the same resource configuration
vmrogue snapshot-restore initial-state --target my-vm-copy
```

### With Blueprints

```bash
# Deploy blueprint
vmrogue deploy lamp --prefix prod

# Snapshot all VMs in the stack
vmrogue snapshot-create prod-mysql-db --name lamp-backup-db
vmrogue snapshot-create prod-web-server --name lamp-backup-web

# Restore entire stack if needed
vmrogue snapshot-restore lamp-backup-db --in-place
vmrogue snapshot-restore lamp-backup-web --in-place
```

---

## 🎨 Status Symbols

Snapshots use color-coded status symbols for quick visual feedback:

- **●** Green = Ready/Succeeded
- **◐** Yellow = In Progress
- **✗** Red = Failed
- **?** Gray = Unknown

---

## 🧪 Testing

All snapshot features include comprehensive tests:

```bash
# Test snapshot module
cargo test --lib snapshots

# Test specific functionality
cargo test --lib snapshots::manager::tests
cargo test --lib snapshots::restore::tests
cargo test --lib snapshots::types::tests
```

Test Results:
```
running 16 tests
test snapshots::manager::tests::test_create_snapshot ... ok
test snapshots::manager::tests::test_list_snapshots_for_vm ... ok
test snapshots::manager::tests::test_list_all_snapshots ... ok
test snapshots::manager::tests::test_get_snapshot ... ok
test snapshots::manager::tests::test_is_snapshot_ready ... ok
test snapshots::restore::tests::test_restore_to_new_vm ... ok
test snapshots::restore::tests::test_restore_in_place ... ok
test snapshots::restore::tests::test_get_restore_status ... ok
test snapshots::restore::tests::test_list_restores ... ok
test snapshots::restore::tests::test_validate_snapshot ... ok
test snapshots::restore::tests::test_estimate_restore_time ... ok
test snapshots::types::tests::test_snapshot_info_creation ... ok
test snapshots::types::tests::test_restore_info_creation ... ok
test snapshots::types::tests::test_snapshot_status_display ... ok
test snapshots::tests::test_snapshot_config ... ok
test snapshots::tests::test_default_retention_policy ... ok

test result: ok. 16 passed; 0 failed
```

---

## 🔮 Future Enhancements

Planned features for future releases:

- [x] Scheduled snapshots (cron-like) — `GET/POST /api/v1/snapshot-schedules`, Lease leader in API pod; UI on Snapshots page and operator console
- [x] Automated retention policy enforcement — `RetentionEnforcer` + schedule `max_snapshots`
- [ ] Snapshot chains visualization
- [ ] Incremental snapshot support
- [ ] Snapshot encryption
- [ ] Cross-namespace snapshots
- [ ] Snapshot import/export
- [ ] Automated retention policy enforcement
- [ ] Snapshot size prediction
- [ ] Multi-VM snapshot coordination

---

## 📚 Related Documentation

- [INNOVATIVE_FEATURES.md](INNOVATIVE_FEATURES.md) - All innovative features
- [OS_TEMPLATES.md](OS_TEMPLATES.md) - OS template catalog
- [../QUICK_REFERENCE.md](../QUICK_REFERENCE.md) - Quick reference card
- [../README.md](../README.md) - Main documentation

---

## REST API

The snapshot system is also available via the REST API. All endpoints require API key authentication.

### List Snapshots

```
GET /snapshots
```

Returns all VirtualMachineSnapshot CRDs in the configured namespace.

### Create Snapshot

```
POST /snapshots
Content-Type: application/json

{
  "vm_name": "my-vm",
  "name": "pre-upgrade-backup",
  "description": "Before OS upgrade"
}
```

If `name` is omitted, one is auto-generated as `snap-{vm_name}-{timestamp}`.

### Delete Snapshot

```
DELETE /snapshots/{snapshot-name}
```

### Restore Snapshot

```
POST /snapshots/{snapshot-name}/restore
Content-Type: application/json

{"target_vm": "restored-vm"}
```

> **Note:** Restore via API is implemented at `POST /api/v1/snapshots/{id}/restore` (in-place when `target_vm` is omitted). Export snapshot metadata via `GET /api/v1/snapshots/export`.

---

## 🎉 Summary

VMRogue's VM Snapshots & Backup System provides:

✅ **5 CLI commands** for comprehensive snapshot management
✅ **REST API** with real VirtualMachineSnapshot CRD integration
✅ **Production-ready** disaster recovery capabilities
✅ **Beautiful CLI** with themed colored output
✅ **Flexible restore** options (new VM or in-place)
✅ **Automatic** snapshot naming and metadata
✅ **Integration** with existing VMRogue features

**Critical for production environments - No other KubeVirt CLI offers this level of snapshot management!**
