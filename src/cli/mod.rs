// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "veyron", alias = "veyron")]
#[command(about = "Veyron — Kubernetes-native VM command center (KubeVirt)")]
#[command(long_about = "\
Veyron — Kubernetes-native VM command center.

Forge, operate, secure, and observe KubeVirt virtual machines from one CLI.

Quick start:
  veyron doctor                     Check cluster connectivity and KubeVirt
  veyron create my-vm -t ubuntu     Create a VM from a template
  veyron ls                         List all VMs
  veyron status my-vm               Show VM status and resources
  veyron tui                        Launch interactive terminal UI

  veyron commands                   List all commands grouped by category
  veyron <command> --help           Detailed help for any command

Configuration:
  veyron config-init                Create default config file
  veyron completions bash           Generate shell completions

Environment variables:
  KUBECONFIG                         Path to kubeconfig file
  VEYRON_NAMESPACE                   Default namespace (VEYRON_NAMESPACE alias)
  VEYRON_NAMESPACE                  Default namespace (legacy alias)
  NO_COLOR                           Disable colored output")]
#[command(version)]
pub struct Cli {
    /// Kubernetes namespace
    #[arg(long, default_value = "default", env = "VEYRON_NAMESPACE")]
    pub namespace: String,

    /// Path to kubeconfig file
    #[arg(long, env = "KUBECONFIG")]
    pub kubeconfig: Option<String>,

    /// Path to config file (default: ~/.config/veyron/config.toml)
    #[arg(long, env = "VEYRON_CONFIG")]
    pub config: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable colored output (also respects NO_COLOR env var)
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Box<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum CatalogAction {
    /// Export built-in templates and profiles to operator/config/catalog YAML
    Export {
        /// Output directory
        #[arg(long)]
        output: Option<String>,
    },
    /// Apply exported catalog YAML to the cluster (cluster-scoped CRDs)
    Sync {
        /// Namespace context label only (CRDs are cluster-scoped)
        #[arg(long, default_value = "default")]
        namespace: String,
    },
    /// List VMTemplate and VMProfile CRDs from the connected cluster
    List {
        /// Show templates only
        #[arg(long)]
        templates: bool,
        /// Show profiles only
        #[arg(long)]
        profiles: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AiCommands {
    /// VM health report (Veyron Doctor)
    Doctor {
        /// VM name
        name: String,
    },
    /// Why is this VM pending / unschedulable?
    Scheduling {
        /// VM name
        name: String,
    },
    /// Translate a Kubernetes / KubeVirt error
    Explain {
        /// Raw error message
        message: String,
    },
    /// Generate VirtualMachine YAML with cluster validation
    Yaml {
        /// VM name
        #[arg(long)]
        name: Option<String>,
        /// Template (e.g. windows-2022, ubuntu-22.04)
        #[arg(long)]
        template: Option<String>,
        /// CPU cores
        #[arg(long)]
        cpus: Option<u32>,
        /// Memory (e.g. 32Gi)
        #[arg(long)]
        memory: Option<String>,
        /// Root disk size
        #[arg(long)]
        disk: Option<String>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Recommend a VM profile from workload description
    Recommend {
        /// Workload description
        description: String,
    },
    /// Fleet backup coverage (snapshots, schedules, Velero)
    Backup,
    /// Fleet or per-VM cost analysis (OpenCost or reference rates)
    Cost {
        /// VM name (optional — fleet sweep when omitted)
        #[arg(long)]
        name: Option<String>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Per-VM network posture
    Network {
        /// VM name
        name: String,
    },
    /// QEMU guest-agent and in-guest signals
    Guest {
        /// VM name
        name: String,
    },
    /// In-guest filesystem usage (guest-exec df / Get-PSDrive)
    Filesystem {
        /// VM name
        name: String,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Fleet PVC pressure and snapshot sprawl
    Storage,
    /// Security exposure, drift, and policies
    Security {
        /// VM name (optional — fleet sweep when omitted)
        #[arg(long)]
        name: Option<String>,
    },
    /// Fleet CPU/memory hotspots
    Performance {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// GitOps repos, drift, and Argo/Flux presence
    Gitops {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Heuristic 30-day capacity forecast
    Forecast {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Optional observability / FinOps backend wiring
    Integrations {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Fleet scheduling pressure (pending VMs)
    Pending {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// VM hardening and compliance gaps
    Compliance {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Metrics/logs/traces stack discovery
    Observability {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Cilium agents and network policies
    Cilium {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Node capacity and pressure
    Nodes {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// VeyronVM operator drift
    Drift {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Warning events narrative
    Alerts {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Fleet availability SLO
    Slo {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Live migration status
    Migrations {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// ResourceQuota pressure
    Quotas {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Template catalog sync health
    Catalog {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Velero backup/restore DR readiness
    VeleroDr {
        #[arg(short, long, default_value = "text")]
        output: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new VM
    Create {
        /// VM name
        name: String,

        /// Use a template (ubuntu, centos, fedora, debian, rhel, windows)
        #[arg(short, long)]
        template: Option<String>,

        /// Load configuration from file
        #[arg(short, long)]
        from_file: Option<String>,

        /// Number of CPU cores
        #[arg(long)]
        cpus: Option<u32>,

        /// Memory size (e.g., 4Gi, 8Gi)
        #[arg(long)]
        memory: Option<String>,

        /// Disk size (e.g., 20Gi, 40Gi)
        #[arg(long)]
        disk_size: Option<String>,

        /// Storage class for disks
        #[arg(long)]
        storage_class: Option<String>,

        /// Container disk image
        #[arg(long)]
        container_disk: Option<String>,

        /// Cloud-init user data file
        #[arg(long)]
        cloud_init: Option<String>,

        /// Do not apply Veyron internet egress policy (CiliumNetworkPolicy / NetworkPolicy)
        #[arg(long)]
        no_internet: bool,

        /// Dry run (don't create, just show manifest)
        #[arg(long)]
        dry_run: bool,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// List VMs in the namespace
    #[command(visible_alias = "ls")]
    List {
        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Get details of a VM
    #[command(visible_alias = "describe")]
    Get {
        /// VM name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Delete a VM
    #[command(visible_alias = "rm")]
    Delete {
        /// VM name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Start a VM
    Start {
        /// VM name
        name: String,
    },

    /// Stop a VM
    Stop {
        /// VM name
        name: String,
    },

    /// Restart a VM
    Restart {
        /// VM name
        name: String,
    },

    /// Pause a running VM
    Pause {
        /// VM name
        name: String,
    },

    /// Unpause a paused VM
    Unpause {
        /// VM name
        name: String,
    },

    /// Freeze guest filesystems via QEMU guest agent (for consistent backups)
    GuestFreeze {
        /// VM / VMI name
        name: String,
    },
    /// Unfreeze guest filesystems
    GuestUnfreeze {
        /// VM / VMI name
        name: String,
    },
    /// ACPI soft reboot via guest agent
    GuestSoftreboot {
        /// VM / VMI name
        name: String,
    },
    /// Hotplug a PVC onto a running VM (`virtctl addvolume`)
    VolumeAdd {
        /// VM name
        name: String,
        /// Volume name to attach in the VM spec
        #[arg(long)]
        volume_name: String,
        /// Existing PVC name
        #[arg(long)]
        pvc: String,
    },
    /// Remove a hotplug volume (`virtctl removevolume`)
    VolumeRemove {
        /// VM name
        name: String,
        #[arg(long)]
        volume_name: String,
    },

    /// Resize VM CPU and/or memory (requires restart to take effect)
    Resize {
        /// VM name
        name: String,

        /// Number of CPU cores
        #[arg(long)]
        cpus: Option<u32>,

        /// Memory size (e.g., 4Gi, 8Gi)
        #[arg(long)]
        memory: Option<String>,
    },

    /// Attach to a VM's serial console
    Console {
        /// VM name
        name: String,
    },

    /// SSH into a running VM
    Ssh {
        /// VM name
        name: String,
        /// SSH user (default: root)
        #[arg(long, default_value = "root")]
        user: String,
    },

    /// Inject an SSH public key into a VM (KubeVirt accessCredentials + Secret)
    #[command(name = "ssh-key-inject")]
    SshKeyInject {
        /// VM name
        name: String,
        /// Path to public key file (e.g. ~/.ssh/id_ed25519.pub)
        #[arg(long, conflicts_with = "public_key")]
        public_key_file: Option<std::path::PathBuf>,
        /// Inline public key (ssh-ed25519 / ssh-rsa …)
        #[arg(long, conflicts_with = "public_key_file")]
        public_key: Option<String>,
        /// Linux guest user for qemuGuestAgent propagation
        #[arg(long, default_value = "clouduser")]
        guest_user: String,
        /// Kubernetes Secret name (default: {vm}-ssh-key)
        #[arg(long)]
        secret_name: Option<String>,
        /// Use configDrive propagation instead of qemuGuestAgent
        #[arg(long)]
        config_drive: bool,
    },

    /// Open VNC console for a VM
    Vnc {
        /// VM name
        name: String,
    },

    /// Stream logs from a VM's virt-launcher pod
    #[command(visible_alias = "log")]
    Logs {
        /// VM name
        name: String,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(long, default_value = "100")]
        tail: u32,
    },

    /// Generate a VM manifest without creating it
    #[command(visible_alias = "gen")]
    Generate {
        /// VM name
        name: String,

        /// Use a template
        #[arg(short, long)]
        template: Option<String>,

        /// Load configuration from file
        #[arg(short, long)]
        from_file: Option<String>,

        /// Number of CPU cores
        #[arg(long)]
        cpus: Option<u32>,

        /// Memory size
        #[arg(long)]
        memory: Option<String>,

        /// Disk size
        #[arg(long)]
        disk_size: Option<String>,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format (yaml, json)
        #[arg(long, default_value = "yaml")]
        format: String,

        /// Generate KubeVirt VirtualMachine CRD instead of VMConfig
        #[arg(long)]
        kubevirt: bool,
    },

    /// List available templates
    Templates {
        /// Group templates by OS family
        #[arg(long)]
        by_family: bool,

        /// List templates from cluster VMTemplate CRDs (requires operator CRDs)
        #[arg(long)]
        source: Option<String>,
    },

    /// Template catalog export and cluster sync (VMTemplate / VMProfile CRDs)
    Catalog {
        #[command(subcommand)]
        action: CatalogAction,
    },

    /// Show template details
    Template {
        /// Template name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Validate a VM configuration file
    Validate {
        /// Path to configuration file
        file: String,
    },

    /// Show detailed VM status with resource information
    #[command(visible_alias = "stat")]
    Status {
        /// VM name
        name: String,

        /// Watch mode - continuously update status
        #[arg(short, long)]
        watch: bool,

        /// Update interval in seconds (for watch mode)
        #[arg(long, default_value = "3")]
        interval: u64,
    },

    /// Clone an existing VM
    Clone {
        /// Source VM name
        source: String,

        /// New VM name
        target: String,

        /// Start the cloned VM immediately
        #[arg(long)]
        start: bool,
    },

    /// Show resource usage summary
    #[command(visible_alias = "top")]
    Resources {
        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Sort by (name, cpu, memory)
        #[arg(long, default_value = "name")]
        sort_by: String,
    },

    /// Export VM configuration
    Export {
        /// VM name
        name: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Export as KubeVirt manifest
        #[arg(long)]
        kubevirt: bool,
    },

    /// Interactive VM creation wizard
    #[command(visible_alias = "wiz")]
    Wizard {
        /// VM name (optional, will prompt if not provided)
        name: Option<String>,
    },

    /// Create multiple VMs from a batch configuration file
    Batch {
        /// Path to batch configuration file (YAML/JSON)
        file: String,

        /// Namespace override for all VMs
        #[arg(short, long)]
        namespace: Option<String>,

        /// Dry run - show what would be created
        #[arg(long)]
        dry_run: bool,

        /// Continue on errors instead of stopping
        #[arg(long)]
        continue_on_error: bool,
    },

    // ========== INNOVATIVE FEATURES ==========
    /// List VM resource profiles (dev, prod, high-perf, etc.)
    Profiles {
        /// Show detailed information
        #[arg(short, long)]
        details: bool,
    },

    /// Show specific profile details
    Profile {
        /// Profile name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Create a custom profile
    ProfileCreate {
        /// Profile name (lowercase alphanumeric with hyphens)
        name: String,

        /// CPU cores
        #[arg(long)]
        cpus: u32,

        /// CPU sockets
        #[arg(long, default_value = "1")]
        sockets: u32,

        /// CPU threads per core
        #[arg(long, default_value = "1")]
        threads: u32,

        /// Memory (e.g., 4Gi, 512Mi, 16G)
        #[arg(long)]
        memory: String,

        /// Disk size (e.g., 10Gi, 500G)
        #[arg(long)]
        disk_size: String,

        /// Profile description
        #[arg(long)]
        description: Option<String>,

        /// Use cases (comma-separated)
        #[arg(long)]
        use_cases: Option<String>,

        /// Recommended OS templates (comma-separated)
        #[arg(long)]
        recommended_os: Option<String>,

        /// Load profile from YAML file
        #[arg(long, conflicts_with_all = &["cpus", "memory", "disk_size"])]
        from_file: Option<String>,
    },

    /// Edit a custom profile
    ProfileEdit {
        /// Profile name
        name: String,

        /// CPU cores
        #[arg(long)]
        cpus: Option<u32>,

        /// CPU sockets
        #[arg(long)]
        sockets: Option<u32>,

        /// CPU threads per core
        #[arg(long)]
        threads: Option<u32>,

        /// Memory (e.g., 4Gi, 512Mi, 16G)
        #[arg(long)]
        memory: Option<String>,

        /// Disk size (e.g., 10Gi, 500G)
        #[arg(long)]
        disk_size: Option<String>,

        /// Profile description
        #[arg(long)]
        description: Option<String>,

        /// Use cases (comma-separated)
        #[arg(long)]
        use_cases: Option<String>,

        /// Recommended OS templates (comma-separated)
        #[arg(long)]
        recommended_os: Option<String>,
    },

    /// Delete a custom profile
    ProfileDelete {
        /// Profile name
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// List multi-VM blueprints (LAMP, Kubernetes, 3-tier, etc.)
    Blueprints {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        details: bool,
    },

    /// Show specific blueprint details
    Blueprint {
        /// Blueprint name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Create a custom blueprint
    BlueprintCreate {
        /// Blueprint name (lowercase alphanumeric with hyphens)
        name: String,

        /// Load blueprint from YAML file
        #[arg(long)]
        from_file: String,

        /// Blueprint description
        #[arg(long)]
        description: Option<String>,
    },

    /// Edit a custom blueprint
    BlueprintEdit {
        /// Blueprint name
        name: String,

        /// Blueprint description
        #[arg(long)]
        description: Option<String>,
    },

    /// Delete a custom blueprint
    BlueprintDelete {
        /// Blueprint name
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Validate a blueprint file
    BlueprintValidate {
        /// Path to blueprint YAML file
        file: String,

        /// Show detailed validation report
        #[arg(short, long)]
        detailed: bool,
    },

    /// Deploy a multi-VM blueprint
    Deploy {
        /// Blueprint name
        blueprint: String,

        /// Name prefix for VMs (default: blueprint name)
        #[arg(short, long)]
        prefix: Option<String>,

        /// Start VMs after creation
        #[arg(long)]
        start: bool,

        /// Dry run - show what would be created
        #[arg(long)]
        dry_run: bool,
    },

    /// Run health check on a VM configuration or running VM
    Health {
        /// VM name (for running VM) or config file path
        target: String,

        /// Show detailed checks
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get resource recommendations for a workload
    Recommend {
        /// Workload type (web, database, cache, ci, etc.)
        workload: String,

        /// Show alternative profiles
        #[arg(short, long)]
        alternatives: bool,
    },

    // ========== VM SNAPSHOTS & BACKUP ==========
    /// Create a VM snapshot
    SnapshotCreate {
        /// VM name
        vm: String,

        /// Snapshot name (optional, auto-generated if not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Description of the snapshot
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List snapshots
    SnapshotList {
        /// VM name (optional, shows all snapshots if not provided)
        vm: Option<String>,

        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show snapshot details
    SnapshotGet {
        /// Snapshot name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Delete a snapshot
    SnapshotDelete {
        /// Snapshot name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Restore VM from snapshot
    SnapshotRestore {
        /// Snapshot name
        snapshot: String,

        /// Target VM name (if different from original)
        #[arg(short, long)]
        target: Option<String>,

        /// Restore in-place (overwrite existing VM)
        #[arg(long)]
        in_place: bool,

        /// Start VM after restore
        #[arg(long)]
        start: bool,
    },

    // ========== PERFORMANCE MONITORING ==========
    /// Show live performance monitoring for a VM
    MonitorLive {
        /// VM name
        vm: String,

        /// Update interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Get performance statistics for a VM
    MonitorStats {
        /// VM name
        vm: String,

        /// Time period (5m, 15m, 1h, 6h, 24h, 7d)
        #[arg(short, long, default_value = "1h")]
        period: String,

        /// Output format (table, json, yaml, summary)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Compare performance of multiple VMs
    MonitorCompare {
        /// VM names to compare
        vms: Vec<String>,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show top VMs by resource usage
    MonitorTop {
        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Sort by (cpu, memory, disk, score)
        #[arg(long, default_value = "score")]
        sort_by: String,

        /// Number of VMs to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    // ========== DISK MANAGEMENT ==========
    /// Expand VM disk size
    DiskExpand {
        /// VM name
        vm: String,

        /// Disk name
        disk: String,

        /// New size (e.g., 100Gi)
        size: String,

        /// PVC name (if different from disk name)
        #[arg(long)]
        pvc: Option<String>,

        /// Show expansion plan without executing
        #[arg(long)]
        plan: bool,
    },

    /// Check disk health for a VM
    DiskHealth {
        /// VM name
        vm: String,

        /// Show detailed disk information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Generate filesystem expansion script
    DiskScript {
        /// Filesystem type (ext4, xfs, lvm, lvm-xfs, btrfs)
        #[arg(short, long, default_value = "lvm")]
        filesystem: String,

        /// Device path (e.g., /dev/vda)
        #[arg(short, long, default_value = "/dev/vda")]
        device: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Generate dry-run script
        #[arg(long)]
        dry_run: bool,
    },

    /// Get disk usage statistics for VMs
    DiskUsage {
        /// VM name (optional, shows all VMs if not provided)
        vm: Option<String>,

        /// Sort by (name, usage, size, available)
        #[arg(long, default_value = "usage")]
        sort_by: String,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    // ========== NETWORK MANAGEMENT ==========
    /// List network interfaces for a VM
    NetworkList {
        /// VM name
        vm: String,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show network interface details
    NetworkGet {
        /// VM name
        vm: String,

        /// Interface name
        interface: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Monitor network bandwidth for a VM
    NetworkBandwidth {
        /// VM name
        vm: String,

        /// Interface name (optional, shows all if not provided)
        #[arg(short, long)]
        interface: Option<String>,

        /// Watch mode - continuously update
        #[arg(short, long)]
        watch: bool,

        /// Update interval in seconds (for watch mode)
        #[arg(long, default_value = "5")]
        interval: u64,
    },

    /// Show network traffic analysis
    NetworkTraffic {
        /// VM name
        vm: String,

        /// Interface name
        #[arg(short, long)]
        interface: Option<String>,

        /// Time period (5m, 15m, 1h, 6h)
        #[arg(short, long, default_value = "15m")]
        period: String,

        /// Show top N talkers
        #[arg(long, default_value = "10")]
        top: usize,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// List network policies
    NetworkPolicies {
        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show network policy details
    NetworkPolicy {
        /// Policy name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    // ========== VM MIGRATION & HIGH AVAILABILITY ==========
    /// Migrate a VM to another node
    Migrate {
        /// VM name
        vm: String,

        /// Target node (auto-select if not specified)
        #[arg(short, long)]
        target_node: Option<String>,

        /// Migration type (live, offline, post-copy)
        #[arg(long, default_value = "live")]
        migration_type: String,

        /// Show migration plan without executing
        #[arg(long)]
        plan: bool,
    },

    /// Show migration status
    MigrationStatus {
        /// VM name
        vm: String,

        /// Watch mode - continuously update status
        #[arg(short, long)]
        watch: bool,

        /// Update interval in seconds (for watch mode)
        #[arg(long, default_value = "5")]
        interval: u64,
    },

    /// List migrations
    MigrationList {
        /// Show all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Filter by state (running, succeeded, failed)
        #[arg(long)]
        state: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Configure VM high availability
    HAConfig {
        /// VM name
        vm: String,

        /// Enable HA
        #[arg(long)]
        enable: bool,

        /// Disable HA
        #[arg(long)]
        disable: bool,

        /// HA priority (critical, high, normal, low)
        #[arg(long)]
        priority: Option<String>,

        /// Eviction strategy (live-migrate, shutdown, none)
        #[arg(long)]
        eviction_strategy: Option<String>,
    },

    /// Show VM HA status
    HAStatus {
        /// VM name
        vm: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Evacuate/drain a node
    EvacuateNode {
        /// Node name
        node: String,

        /// Reason for evacuation
        #[arg(short, long)]
        reason: Option<String>,

        /// Max parallel migrations
        #[arg(long, default_value = "2")]
        max_parallel: u32,

        /// Timeout in seconds
        #[arg(long, default_value = "3600")]
        timeout: u64,

        /// Force evacuation
        #[arg(long)]
        force: bool,

        /// Show evacuation plan without executing
        #[arg(long)]
        plan: bool,
    },

    /// Show node evacuation status
    EvacuationStatus {
        /// Node name
        node: String,

        /// Watch mode - continuously update status
        #[arg(short, long)]
        watch: bool,
    },

    // ========== BACKUP & DISASTER RECOVERY ==========
    /// Create a VM backup
    BackupCreate {
        /// VM name
        vm: String,

        /// Backup name (auto-generated if not specified)
        #[arg(short, long)]
        name: Option<String>,

        /// Backup type (full, incremental, differential)
        #[arg(long, default_value = "full")]
        backup_type: String,

        /// Compression type (gzip, zstd, lz4, none)
        #[arg(long, default_value = "gzip")]
        compression: String,

        /// Disable encryption
        #[arg(long)]
        no_encryption: bool,
    },

    /// List backups
    BackupList {
        /// VM name (optional, shows all if not specified)
        vm: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show backup details
    BackupGet {
        /// Backup name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Delete a backup
    BackupDelete {
        /// Backup name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Restore VM from backup
    BackupRestore {
        /// Backup name
        backup: String,

        /// Target VM name (defaults to original)
        #[arg(short, long)]
        target: Option<String>,

        /// Start VM after restore
        #[arg(long)]
        start: bool,
    },

    /// Verify backup integrity
    BackupVerify {
        /// Backup name
        name: String,

        /// Verification type (quick, standard, full)
        #[arg(long, default_value = "standard")]
        verification_type: String,
    },

    /// List backup schedules
    BackupSchedules {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create backup schedule
    BackupScheduleCreate {
        /// Schedule name
        name: String,

        /// Schedule type (hourly, daily, weekly, monthly)
        #[arg(long)]
        schedule: String,

        /// VM selector (all, or specific VM name)
        #[arg(long)]
        vm: Option<String>,
    },

    /// Show disaster recovery plan
    RecoveryPlan {
        /// Plan name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Execute disaster recovery
    RecoveryExecute {
        /// Plan name
        plan: String,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    // ========== SECURITY & COMPLIANCE ==========
    /// Scan VM for security vulnerabilities
    SecurityScan {
        /// VM name
        vm: String,

        /// Scan type (quick, standard, deep, compliance)
        #[arg(long, default_value = "standard")]
        scan_type: String,

        /// Include container scanning
        #[arg(long)]
        containers: bool,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show security assessment for a VM
    SecurityAssess {
        /// VM name
        vm: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Apply security hardening profile
    SecurityHarden {
        /// VM name
        vm: String,

        /// Hardening profile (cis, stig, pci-dss, nist, custom)
        #[arg(short, long, default_value = "cis")]
        profile: String,

        /// Verify only, don't apply changes
        #[arg(long)]
        verify_only: bool,
    },

    /// List hardening profiles
    SecurityProfiles {
        /// Show detailed information
        #[arg(short, long)]
        details: bool,
    },

    /// Run compliance check
    ComplianceCheck {
        /// VM name
        vm: String,

        /// Compliance framework (pci-dss, hipaa, soc2, iso27001, gdpr, nist, cis)
        #[arg(short, long, default_value = "pci-dss")]
        framework: String,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show compliance report
    ComplianceReport {
        /// VM name
        vm: String,

        /// Report ID (optional, shows latest if not provided)
        #[arg(short, long)]
        report_id: Option<String>,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// List audit events
    AuditList {
        /// VM name (optional, shows all if not provided)
        vm: Option<String>,

        /// Event type filter
        #[arg(long)]
        event_type: Option<String>,

        /// Severity filter (critical, high, medium, low, info)
        #[arg(long)]
        severity: Option<String>,

        /// Show only security events
        #[arg(long)]
        security_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show audit log details
    AuditGet {
        /// Log ID
        log_id: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Show audit statistics
    AuditStats {
        /// VM name (optional, shows all if not provided)
        vm: Option<String>,

        /// Time period (24h, 7d, 30d, 90d)
        #[arg(short, long, default_value = "7d")]
        period: String,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    // ========== COST MANAGEMENT & OPTIMIZATION ==========
    /// Show VM cost analysis
    CostAnalyze {
        /// VM name (optional, shows all if not provided)
        vm: Option<String>,

        /// Time period (7d, 30d, 90d, 180d, 365d)
        #[arg(short, long, default_value = "30d")]
        period: String,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show cost summary
    CostSummary {
        /// Namespace filter
        #[arg(short, long)]
        namespace: Option<String>,

        /// Time period (7d, 30d, 90d, 180d, 365d)
        #[arg(short, long, default_value = "30d")]
        period: String,

        /// Group by (namespace, team, project)
        #[arg(long)]
        group_by: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Generate cost report
    CostReport {
        /// Report type (daily, weekly, monthly, quarterly, yearly)
        #[arg(short, long, default_value = "monthly")]
        report_type: String,

        /// Export format (json, csv, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Manage budgets
    BudgetList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create a budget
    BudgetCreate {
        /// Budget name
        name: String,

        /// Budget amount
        #[arg(short, long)]
        amount: f64,

        /// Budget period (daily, weekly, monthly, quarterly, yearly)
        #[arg(short, long, default_value = "monthly")]
        period: String,

        /// Scope (global, namespace:NAME, team:NAME, project:NAME)
        #[arg(short, long, default_value = "global")]
        scope: String,

        /// Alert threshold (e.g., 80 for 80%)
        #[arg(long)]
        alert_threshold: Option<f64>,
    },

    /// Show budget status
    BudgetStatus {
        /// Budget name
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Get cost optimization recommendations
    CostOptimize {
        /// VM name (optional, shows all if not provided)
        vm: Option<String>,

        /// Show only high priority recommendations
        #[arg(long)]
        high_priority_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show cost waste report
    CostWaste {
        /// Waste type filter (idle, oversized, storage, snapshots)
        #[arg(long)]
        waste_type: Option<String>,

        /// Minimum monthly waste to show
        #[arg(long, default_value = "10")]
        min_waste: f64,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Forecast costs
    CostForecast {
        /// Budget to compare against
        #[arg(short, long)]
        budget: Option<f64>,

        /// Forecast period (7d, 30d, 90d)
        #[arg(short, long, default_value = "30d")]
        period: String,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    // ========== AUTOMATION & ORCHESTRATION ==========
    /// List automation rules
    AutomationList {
        /// Show only enabled rules
        #[arg(long)]
        enabled_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create automation rule
    AutomationCreate {
        /// Rule name
        name: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Trigger type (manual, schedule, event, metric)
        #[arg(short, long, default_value = "manual")]
        trigger: String,

        /// Enable immediately
        #[arg(long)]
        enable: bool,
    },

    /// Show automation rule details
    AutomationGet {
        /// Rule ID or name
        rule: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Execute automation rule
    AutomationRun {
        /// Rule ID or name
        rule: String,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// List workflows
    WorkflowList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create workflow
    WorkflowCreate {
        /// Workflow name
        name: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Template (provisioning, disaster-recovery, maintenance)
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Show workflow details
    WorkflowGet {
        /// Workflow ID or name
        workflow: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Execute workflow
    WorkflowRun {
        /// Workflow ID or name
        workflow: String,

        /// Show execution progress
        #[arg(short, long)]
        watch: bool,
    },

    /// List workflow executions
    WorkflowExecutions {
        /// Workflow ID or name (optional, shows all if not provided)
        workflow: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// List scheduled tasks
    ScheduleList {
        /// Show only enabled tasks
        #[arg(long)]
        enabled_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create scheduled task
    ScheduleCreate {
        /// Task name
        name: String,

        /// Rule ID to execute
        #[arg(short, long)]
        rule: String,

        /// Schedule (hourly, daily, weekly, interval:3600)
        #[arg(short, long)]
        schedule: String,

        /// Enable immediately
        #[arg(long)]
        enable: bool,
    },

    // ========== OBSERVABILITY & ANALYTICS ==========
    /// Query logs
    LogsQuery {
        /// Start time (RFC3339 format)
        #[arg(long)]
        start: Option<String>,

        /// End time (RFC3339 format)
        #[arg(long)]
        end: Option<String>,

        /// Log level filter (debug, info, warning, error, critical)
        #[arg(short, long)]
        level: Option<String>,

        /// Source filter
        #[arg(short, long)]
        source: Option<String>,

        /// Search text
        #[arg(long)]
        search: Option<String>,

        /// Limit results
        #[arg(long, default_value = "100")]
        limit: usize,
    },

    /// Show log statistics
    LogsStats {
        /// Group by (level, source)
        #[arg(long, default_value = "level")]
        group_by: String,
    },

    /// Analyze log patterns
    LogsPatterns {
        /// Minimum pattern count
        #[arg(long, default_value = "2")]
        min_count: usize,
    },

    /// Collect VM metrics
    MetricsCollect {
        /// VM name
        vm: String,
    },

    /// Query metrics
    MetricsQuery {
        /// Metric name
        name: String,

        /// Start time (RFC3339 format)
        #[arg(long)]
        start: Option<String>,

        /// End time (RFC3339 format)
        #[arg(long)]
        end: Option<String>,

        /// Aggregation (avg, sum, max, min, p50, p95, p99)
        #[arg(long, default_value = "avg")]
        aggregation: String,
    },

    /// Show metrics snapshot
    MetricsSnapshot {
        /// Filter by VM
        #[arg(long)]
        vm: Option<String>,

        /// CPU threshold for highlighting
        #[arg(long, default_value = "80")]
        cpu_threshold: f64,

        /// Memory threshold for highlighting
        #[arg(long, default_value = "80")]
        memory_threshold: f64,
    },

    /// List alert rules
    AlertsList {
        /// Show only enabled rules
        #[arg(long)]
        enabled_only: bool,

        /// Filter by severity (info, warning, critical)
        #[arg(long)]
        severity: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create alert rule
    AlertsCreate {
        /// Rule name
        name: String,

        /// Alert severity (info, warning, critical)
        #[arg(long)]
        severity: String,

        /// Metric name
        #[arg(long)]
        metric: String,

        /// Threshold operator (gt, lt, eq, gte, lte)
        #[arg(long)]
        operator: String,

        /// Threshold value
        #[arg(long)]
        threshold: f64,

        /// Duration in minutes
        #[arg(long, default_value = "5")]
        duration: i64,
    },

    /// Show active alerts
    AlertsActive {
        /// Filter by severity
        #[arg(long)]
        severity: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Resolve alert
    AlertsResolve {
        /// Alert ID
        alert_id: String,
    },

    /// Generate insights
    InsightsGenerate {
        /// VM name (optional, analyzes all VMs if not provided)
        vm: Option<String>,

        /// Insight type (performance, cost, security, availability, capacity)
        #[arg(long)]
        insight_type: Option<String>,

        /// Minimum severity (low, medium, high)
        #[arg(long, default_value = "low")]
        min_severity: String,
    },

    /// Show recommendations
    Recommendations {
        /// Category (cost, performance, security, reliability, sustainability)
        #[arg(long)]
        category: Option<String>,

        /// Minimum priority (low, medium, high)
        #[arg(long, default_value = "low")]
        min_priority: String,

        /// Show estimated savings
        #[arg(long)]
        with_savings: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Analyze trends
    TrendsAnalyze {
        /// Metric name
        metric: String,

        /// Time window in hours
        #[arg(long, default_value = "24")]
        window: i64,

        /// Significance threshold percentage
        #[arg(long, default_value = "10")]
        threshold: f64,
    },

    /// Check system health
    HealthCheck {
        /// Component filter (optional)
        component: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    // ========== MULTI-TENANCY & RBAC ==========
    /// List tenants
    TenantsList {
        /// Show only active tenants
        #[arg(long)]
        active_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create tenant
    TenantsCreate {
        /// Tenant name
        name: String,

        /// Owner user ID
        #[arg(short, long)]
        owner: String,

        /// Contact email
        #[arg(short, long)]
        email: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Default namespace
        #[arg(long)]
        namespace: Option<String>,
    },

    /// Show tenant details
    TenantsShow {
        /// Tenant ID or name
        tenant: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Delete tenant
    TenantsDelete {
        /// Tenant ID
        tenant: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// List users
    UsersList {
        /// Show only active users
        #[arg(long)]
        active_only: bool,

        /// Filter by group
        #[arg(short, long)]
        group: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create user
    UsersCreate {
        /// Username
        username: String,

        /// Email address
        #[arg(short, long)]
        email: String,

        /// Assign role
        #[arg(short, long)]
        role: Option<String>,

        /// Add to group
        #[arg(short, long)]
        group: Option<String>,
    },

    /// Assign role to user
    UsersAssignRole {
        /// User ID or username
        user: String,

        /// Role to assign
        role: String,

        /// Scope (cluster or namespace:NAME)
        #[arg(short, long, default_value = "cluster")]
        scope: String,
    },

    /// List roles
    RolesList {
        /// Show only built-in roles
        #[arg(long)]
        builtin: bool,

        /// Show only custom roles
        #[arg(long)]
        custom: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Show role details
    RolesShow {
        /// Role name
        role: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// Create custom role
    RolesCreate {
        /// Role name
        name: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Permissions (comma-separated, e.g. vm:create,vm:view)
        #[arg(short, long)]
        permissions: String,
    },

    /// List resource quotas
    QuotasList {
        /// Filter by namespace
        #[arg(short, long)]
        namespace: Option<String>,

        /// Show only exceeded quotas
        #[arg(long)]
        exceeded: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create resource quota
    QuotasCreate {
        /// Quota name
        name: String,

        /// Namespace
        #[arg(short, long)]
        namespace: String,

        /// Preset (small, medium, large, unlimited)
        #[arg(short, long, default_value = "medium")]
        preset: String,
    },

    /// Show quota details
    QuotasShow {
        /// Quota ID or name
        quota: String,

        /// Show utilization
        #[arg(long)]
        utilization: bool,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        output: String,
    },

    /// List groups
    GroupsList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create group
    GroupsCreate {
        /// Group name
        name: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Assign role to group
        #[arg(short, long)]
        role: Option<String>,
    },

    /// Add user to group
    GroupsAddUser {
        /// Group ID or name
        group: String,

        /// User ID or username
        user: String,
    },

    // ========== DEVELOPER EXPERIENCE & TOOLING ==========
    /// Generate shell completions
    Completions {
        /// Shell type (bash, zsh, fish, powershell, elvish)
        shell: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Show install instructions
        #[arg(long)]
        install: bool,
    },

    /// Save a VM configuration as a reusable template
    ConfigSave {
        /// Template name
        name: String,

        /// Path to configuration file
        #[arg(short, long)]
        file: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Category (dev, test, staging, prod, db, web, cicd, ml)
        #[arg(short, long, default_value = "dev")]
        category: String,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Load a saved configuration template
    ConfigLoad {
        /// Template name
        name: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },

    /// List saved configuration templates
    ConfigList {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Sort by (name, usage, created)
        #[arg(long, default_value = "name")]
        sort_by: String,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Delete a saved configuration template
    ConfigDelete {
        /// Template name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Compare two VM configurations
    Diff {
        /// First configuration file
        source: String,

        /// Second configuration file
        target: String,

        /// Show unchanged fields
        #[arg(long)]
        show_unchanged: bool,

        /// Output format (text, yaml, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Initialize a new veyron project
    Init {
        /// Project name
        name: String,

        /// Project type (basic, dev, prod, microservices, data-pipeline)
        #[arg(short, long, default_value = "basic")]
        project_type: String,

        /// Target directory
        #[arg(short, long)]
        directory: Option<String>,

        /// Default namespace
        #[arg(long)]
        namespace: Option<String>,

        /// Skip example files
        #[arg(long)]
        no_examples: bool,

        /// Include CI/CD configuration
        #[arg(long)]
        ci: bool,

        /// Skip git initialization
        #[arg(long)]
        no_git: bool,
    },

    /// Show environment and version information
    Info {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Run diagnostics
        #[arg(long)]
        diagnostics: bool,

        /// Output format (text, yaml, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    // ========== API & REST INTERFACE ==========
    /// Start the REST API server
    ApiServe {
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to
        #[arg(long)]
        host: Option<String>,

        /// Enable TLS
        #[arg(long)]
        tls: bool,

        /// TLS certificate path
        #[arg(long)]
        tls_cert: Option<String>,

        /// TLS key path
        #[arg(long)]
        tls_key: Option<String>,

        /// Authentication method (none, api-key, bearer, basic, oauth2, mtls)
        #[arg(long)]
        auth: Option<String>,

        /// Rate limit (requests per minute, 0 to disable)
        #[arg(long)]
        rate_limit: Option<u32>,
    },

    /// Show API server status
    ApiStatus {
        /// Output format (text, yaml, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// List API routes
    ApiRoutes {
        /// Filter by method (GET, POST, PUT, DELETE)
        #[arg(short, long)]
        method: Option<String>,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Generate OpenAPI specification
    ApiSpec {
        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Manage API keys
    ApiKeyList {
        /// Show only active keys
        #[arg(long)]
        active_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Create an API key
    ApiKeyCreate {
        /// Key name
        name: String,

        /// Permissions (comma-separated: read, write, admin)
        #[arg(short, long, default_value = "read")]
        permissions: String,

        /// Rate limit for this key (requests per minute)
        #[arg(long)]
        rate_limit: Option<u32>,
    },

    /// Delete an API key
    ApiKeyDelete {
        /// Key ID or name
        key: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// List webhook registrations
    WebhookList {
        /// Show only active webhooks
        #[arg(long)]
        active_only: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Register a webhook
    WebhookCreate {
        /// Webhook name
        name: String,

        /// Webhook URL
        #[arg(short, long)]
        url: String,

        /// Events to subscribe to (comma-separated)
        #[arg(short, long)]
        events: String,

        /// Webhook secret for signing
        #[arg(short, long)]
        secret: Option<String>,
    },

    /// Delete a webhook
    WebhookDelete {
        /// Webhook ID or name
        webhook: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// List activity events
    EventList {
        /// Filter by VM name
        #[arg(long)]
        vm: Option<String>,

        /// Maximum number of events to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output format (text, yaml, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Show recent activity events
    EventRecent {
        /// Maximum number of events to show
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format (text, yaml, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Launch interactive TUI
    Tui {
        /// Disable splash screen
        #[arg(long)]
        no_splash: bool,

        /// Theme (light, dark)
        #[arg(long)]
        theme: Option<String>,

        /// Use basic TUI mode (without dialogs, menus, notifications)
        #[arg(long)]
        basic: bool,
    },

    /// Show current configuration
    #[command(name = "config-show", visible_alias = "config")]
    ConfigShow {
        /// Show config file path only
        #[arg(long)]
        path: bool,
    },

    /// Initialize default configuration file
    #[command(name = "config-init")]
    ConfigInit {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
    },

    /// List all commands grouped by category
    #[command(name = "commands")]
    CommandList,

    /// Diagnose environment and connectivity
    #[command(name = "doctor", visible_alias = "doc")]
    Doctor,

    /// AI copilot — VM doctor, scheduling, YAML, error explain (v1)
    #[command(name = "ai", visible_alias = "copilot")]
    Ai {
        #[command(subcommand)]
        action: Option<AiCommands>,
        /// Natural-language question when no subcommand is used
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        query: Vec<String>,
    },

    // ========== GITOPS ==========
    /// Export all VMs as VeyronVM CRD manifests to a directory
    #[command(name = "gitops-export")]
    GitopsExport {
        /// Output directory for YAML manifests
        #[arg(short, long, default_value = ".")]
        directory: String,
    },

    // ========== OPERATOR CRD MANAGEMENT (veyron.io/v1alpha1) ==========
    /// List VeyronVM custom resources
    #[command(name = "vrvm-list")]
    VrvmList,

    /// Get a VeyronVM custom resource
    #[command(name = "vrvm-get")]
    VrvmGet {
        /// VM name
        name: String,
    },

    /// Create a VeyronVM custom resource with full hardware configuration
    #[command(name = "vrvm-create")]
    VrvmCreate {
        /// VM name
        name: String,

        /// OS template (e.g. ubuntu-22.04, fedora-41, windows-11)
        #[arg(short, long)]
        template: Option<String>,

        /// CPU cores
        #[arg(long, default_value = "2")]
        cpus: u32,

        /// Memory (e.g. 4Gi, 8Gi, 16Gi)
        #[arg(long, default_value = "4Gi")]
        memory: String,

        /// Disk specification (repeatable). Format: name=X,size=Y,source=Z[,image=I][,boot=N][,bus=B]
        /// Sources: blank, containerDisk, pvc, dataVolume
        /// Examples:
        ///   --disk name=root,size=20Gi,source=containerDisk,image=quay.io/containerdisks/ubuntu:22.04,boot=1
        ///   --disk name=data,size=50Gi,source=blank
        ///   --disk name=existing,source=pvc,pvc=my-pvc
        #[arg(long, num_args = 1)]
        disk: Vec<String>,

        /// Attach CDROM from container disk image
        #[arg(long)]
        cdrom: Option<String>,

        /// Network interface (repeatable). Format: type=X[,name=Y]
        /// Types: pod (default), multus, bridge, sriov
        /// Examples:
        ///   --network type=pod
        ///   --network type=multus,name=br-net
        ///   --network type=sriov,name=sriov-net1
        #[arg(long, num_args = 1)]
        network: Vec<String>,

        /// Path to cloud-init user-data file
        #[arg(long)]
        cloud_init: Option<String>,

        /// Firmware type: bios or efi
        #[arg(long)]
        firmware: Option<String>,

        /// Enable UEFI Secure Boot (implies --firmware efi)
        #[arg(long)]
        secure_boot: bool,

        /// Enable TPM 2.0
        #[arg(long)]
        tpm: bool,

        /// Disable virtio-rng (enabled by default)
        #[arg(long)]
        no_rng: bool,

        /// Do not apply internet egress policy (operator reconciles when enabled)
        #[arg(long)]
        no_internet: bool,

        /// Machine type (e.g. q35)
        #[arg(long)]
        machine_type: Option<String>,

        /// Eviction strategy (LiveMigrate, LiveMigrateIfPossible)
        #[arg(long)]
        eviction_strategy: Option<String>,

        /// Labels (repeatable, format: key=value)
        #[arg(short, long, num_args = 1)]
        label: Vec<String>,

        /// Load full VeyronVMSpec from YAML file
        #[arg(short, long)]
        from_file: Option<String>,

        /// Print YAML instead of creating
        #[arg(long)]
        dry_run: bool,

        /// Start VM after creation
        #[arg(long)]
        start: bool,
    },

    /// Apply a VeyronVM from a YAML manifest file
    #[command(name = "vrvm-apply")]
    VrvmApply {
        /// Path to YAML manifest file
        #[arg(short, long)]
        file: String,

        /// Print diff instead of applying
        #[arg(long)]
        dry_run: bool,
    },

    /// Delete a VeyronVM custom resource
    #[command(name = "vrvm-delete")]
    VrvmDelete {
        /// VM name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// List VeyronBlueprint custom resources
    #[command(name = "vrbp-list")]
    VrbpList,

    /// Get a VeyronBlueprint custom resource
    #[command(name = "vrbp-get")]
    VrbpGet {
        /// Blueprint name
        name: String,
    },

    /// Delete a VeyronBlueprint custom resource
    #[command(name = "vrbp-delete")]
    VrbpDelete {
        /// Blueprint name
        name: String,
    },

    /// List VeyronPolicy custom resources
    #[command(name = "vrpol-list")]
    VrpolList,

    /// Get a VeyronPolicy custom resource
    #[command(name = "vrpol-get")]
    VrpolGet {
        /// Policy name
        name: String,
    },

    /// Delete a VeyronPolicy custom resource
    #[command(name = "vrpol-delete")]
    VrpolDelete {
        /// Policy name
        name: String,
    },

    /// List VeyronInsight custom resources
    #[command(name = "vrin-list")]
    VrinList,

    /// List VeyronAction custom resources
    #[command(name = "vract-list")]
    VractList,

    /// Approve a VeyronAction
    #[command(name = "vract-approve")]
    VractApprove {
        /// Action name
        name: String,
    },

    // ========== GITOPS (EXTENDED) ==========
    /// Show GitOps sync diff between local and cluster state
    #[command(name = "gitops-diff")]
    GitopsDiff {
        /// Directory containing manifests
        #[arg(short, long, default_value = ".")]
        directory: String,
    },

    /// Show GitOps sync status
    #[command(name = "gitops-status")]
    GitopsStatus,

    // ========== MULTI-CLUSTER ==========
    /// List all known Kubernetes clusters
    #[command(name = "clusters-list")]
    ClustersList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Discover clusters from kubeconfig
    #[command(name = "clusters-discover")]
    ClustersDiscover,

    // ========== IMPORT ==========
    /// Import a KubeVirt VirtualMachine YAML (not hypervisor migration — see HyperSDK)
    #[command(
        name = "import",
        after_help = "Hypervisor migration (VMware/Hyper-V) is provided by HyperSDK: https://zyvor.dev/hypersdk"
    )]
    Import {
        /// Path to YAML/JSON file
        file: String,

        /// Start the VM after import
        #[arg(long)]
        start: bool,

        /// Dry run (don't create, just validate)
        #[arg(long)]
        dry_run: bool,
    },

    // ========== EVENTS & NODES (TOP-LEVEL) ==========
    /// List Kubernetes events
    #[command(name = "events")]
    EventsList {
        /// Limit number of events
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// List Kubernetes nodes
    #[command(name = "nodes")]
    NodesList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// List pods in the namespace
    #[command(name = "pods")]
    PodsList {
        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    // ========== NATURAL LANGUAGE SEARCH ==========
    /// Search VMs using natural language queries
    #[command(name = "search")]
    Search {
        /// Natural language query (e.g., "running vms using more than 4 cpu")
        query: Vec<String>,
    },

    // ========== AI TROUBLESHOOT ==========
    /// Run AI-assisted troubleshooting for a VM
    #[command(name = "troubleshoot")]
    Troubleshoot {
        /// VM name to troubleshoot
        name: String,
    },

    // ========== CAPACITY & PLACEMENT ==========
    /// Analyze cluster capacity and VM resource allocation
    #[command(name = "capacity")]
    Capacity {
        /// Show detailed per-node breakdown
        #[arg(long)]
        detailed: bool,

        /// Output format (table, yaml, json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Recommend optimal node placement for a VM
    #[command(name = "placement")]
    Placement {
        /// VM name to find placement for
        name: String,

        /// Placement strategy (spread, binpack, leastloaded)
        #[arg(long, default_value = "leastloaded")]
        strategy: String,
    },

    /// List VMs in JSON Lines format (for scripting)
    #[command(name = "list-json")]
    ListJson,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        // The Commands enum has 146 variants, requiring a larger stack in debug mode.
        // Run parsing in a thread with 16MB stack to avoid stack overflow.
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || Cli::try_parse_from(args_owned))
            .expect("Failed to spawn parse thread")
            .join()
            .expect("Parse thread panicked")
    }

    #[test]
    fn test_create_command() {
        let cli = parse(&["veyron", "create", "my-vm", "--template", "ubuntu"]).unwrap();
        match *cli.command {
            Commands::Create { name, template, .. } => {
                assert_eq!(name, "my-vm");
                assert_eq!(template, Some("ubuntu".to_string()));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_with_resources() {
        let cli = parse(&[
            "veyron",
            "create",
            "test-vm",
            "--template",
            "fedora",
            "--cpus",
            "4",
            "--memory",
            "8Gi",
            "--disk-size",
            "100Gi",
        ])
        .unwrap();
        match *cli.command {
            Commands::Create {
                name,
                cpus,
                memory,
                disk_size,
                ..
            } => {
                assert_eq!(name, "test-vm");
                assert_eq!(cpus, Some(4));
                assert_eq!(memory, Some("8Gi".to_string()));
                assert_eq!(disk_size, Some("100Gi".to_string()));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_dry_run() {
        let cli = parse(&[
            "veyron",
            "create",
            "my-vm",
            "--template",
            "ubuntu",
            "--dry-run",
        ])
        .unwrap();
        match *cli.command {
            Commands::Create { dry_run, .. } => assert!(dry_run),
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_list_command() {
        let cli = parse(&["veyron", "list"]).unwrap();
        match *cli.command {
            Commands::List {
                all_namespaces,
                output,
            } => {
                assert!(!all_namespaces);
                assert_eq!(output, "table");
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_list_all_namespaces() {
        let cli = parse(&["veyron", "list", "-A"]).unwrap();
        match *cli.command {
            Commands::List { all_namespaces, .. } => assert!(all_namespaces),
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_get_command() {
        let cli = parse(&["veyron", "get", "my-vm"]).unwrap();
        match *cli.command {
            Commands::Get { name, .. } => assert_eq!(name, "my-vm"),
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_delete_command() {
        let cli = parse(&["veyron", "delete", "my-vm", "--yes"]).unwrap();
        match *cli.command {
            Commands::Delete { name, yes } => {
                assert_eq!(name, "my-vm");
                assert!(yes);
            }
            _ => panic!("Expected Delete command"),
        }
    }

    #[test]
    fn test_start_stop_restart() {
        let cli = parse(&["veyron", "start", "vm1"]).unwrap();
        assert!(matches!(*cli.command, Commands::Start { name } if name == "vm1"));

        let cli = parse(&["veyron", "stop", "vm1"]).unwrap();
        assert!(matches!(*cli.command, Commands::Stop { name } if name == "vm1"));

        let cli = parse(&["veyron", "restart", "vm1"]).unwrap();
        assert!(matches!(*cli.command, Commands::Restart { name } if name == "vm1"));
    }

    #[test]
    fn test_pause_unpause() {
        let cli = parse(&["veyron", "pause", "vm1"]).unwrap();
        assert!(matches!(*cli.command, Commands::Pause { name } if name == "vm1"));

        let cli = parse(&["veyron", "unpause", "vm1"]).unwrap();
        assert!(matches!(*cli.command, Commands::Unpause { name } if name == "vm1"));
    }

    #[test]
    fn test_resize_command() {
        let cli = parse(&["veyron", "resize", "vm1", "--cpus", "4", "--memory", "8Gi"]).unwrap();
        match *cli.command {
            Commands::Resize { name, cpus, memory } => {
                assert_eq!(name, "vm1");
                assert_eq!(cpus, Some(4));
                assert_eq!(memory, Some("8Gi".to_string()));
            }
            _ => panic!("Expected Resize command"),
        }

        // Test with only cpus
        let cli = parse(&["veyron", "resize", "vm1", "--cpus", "2"]).unwrap();
        match *cli.command {
            Commands::Resize { name, cpus, memory } => {
                assert_eq!(name, "vm1");
                assert_eq!(cpus, Some(2));
                assert!(memory.is_none());
            }
            _ => panic!("Expected Resize command"),
        }
    }

    #[test]
    fn test_namespace_default() {
        let cli = parse(&["veyron", "list"]).unwrap();
        assert_eq!(cli.namespace, "default");
    }

    #[test]
    fn test_namespace_override() {
        let cli = parse(&["veyron", "--namespace", "prod", "list"]).unwrap();
        assert_eq!(cli.namespace, "prod");
    }

    #[test]
    fn test_verbose_flag() {
        let cli = parse(&["veyron", "-v", "list"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_generate_command() {
        let cli = parse(&[
            "veyron",
            "generate",
            "test-vm",
            "--template",
            "ubuntu",
            "--format",
            "json",
        ])
        .unwrap();
        match *cli.command {
            Commands::Generate {
                name,
                template,
                format,
                ..
            } => {
                assert_eq!(name, "test-vm");
                assert_eq!(template, Some("ubuntu".to_string()));
                assert_eq!(format, "json");
            }
            _ => panic!("Expected Generate command"),
        }
    }

    #[test]
    fn test_templates_command() {
        let cli = parse(&["veyron", "templates"]).unwrap();
        assert!(matches!(
            *cli.command,
            Commands::Templates {
                by_family: false,
                source: None,
            }
        ));
    }

    #[test]
    fn test_validate_command() {
        let cli = parse(&["veyron", "validate", "config.yaml"]).unwrap();
        match *cli.command {
            Commands::Validate { file } => assert_eq!(file, "config.yaml"),
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_profiles_command() {
        let cli = parse(&["veyron", "profiles", "--details"]).unwrap();
        match *cli.command {
            Commands::Profiles { details } => assert!(details),
            _ => panic!("Expected Profiles command"),
        }
    }

    #[test]
    fn test_health_command() {
        let cli = parse(&["veyron", "health", "my-vm", "--detailed"]).unwrap();
        match *cli.command {
            Commands::Health { target, detailed } => {
                assert_eq!(target, "my-vm");
                assert!(detailed);
            }
            _ => panic!("Expected Health command"),
        }
    }

    #[test]
    fn test_cost_analyze() {
        let cli = parse(&["veyron", "cost-analyze", "db-vm", "--period", "weekly"]).unwrap();
        match *cli.command {
            Commands::CostAnalyze { vm, period, .. } => {
                assert_eq!(vm, Some("db-vm".to_string()));
                assert_eq!(period, "weekly");
            }
            _ => panic!("Expected CostAnalyze command"),
        }
    }

    #[test]
    fn test_security_scan() {
        let cli = parse(&[
            "veyron",
            "security-scan",
            "web-vm",
            "--scan-type",
            "deep",
            "--containers",
        ])
        .unwrap();
        match *cli.command {
            Commands::SecurityScan {
                vm,
                scan_type,
                containers,
                ..
            } => {
                assert_eq!(vm, "web-vm");
                assert_eq!(scan_type, "deep");
                assert!(containers);
            }
            _ => panic!("Expected SecurityScan command"),
        }
    }

    #[test]
    fn test_tui_command() {
        let cli = parse(&["veyron", "tui", "--basic"]).unwrap();
        match *cli.command {
            Commands::Tui { basic, .. } => assert!(basic),
            _ => panic!("Expected Tui command"),
        }
    }

    #[test]
    fn test_unknown_command_fails() {
        assert!(parse(&["veyron", "nonexistent"]).is_err());
    }

    #[test]
    fn test_missing_required_arg_fails() {
        assert!(parse(&["veyron", "create"]).is_err()); // name is required
    }

    #[test]
    fn test_clone_command() {
        let cli = parse(&["veyron", "clone", "source-vm", "clone-vm"]).unwrap();
        match *cli.command {
            Commands::Clone { source, target, .. } => {
                assert_eq!(source, "source-vm");
                assert_eq!(target, "clone-vm");
            }
            _ => panic!("Expected Clone command"),
        }
    }

    #[test]
    fn test_snapshot_create() {
        let cli = parse(&["veyron", "snapshot-create", "my-vm", "--name", "snap1"]).unwrap();
        match *cli.command {
            Commands::SnapshotCreate { vm, name, .. } => {
                assert_eq!(vm, "my-vm");
                assert_eq!(name, Some("snap1".to_string()));
            }
            _ => panic!("Expected SnapshotCreate command"),
        }
    }

    #[test]
    fn test_backup_create() {
        let cli = parse(&[
            "veyron",
            "backup-create",
            "db-vm",
            "--backup-type",
            "incremental",
        ])
        .unwrap();
        match *cli.command {
            Commands::BackupCreate {
                vm, backup_type, ..
            } => {
                assert_eq!(vm, "db-vm");
                assert_eq!(backup_type, "incremental");
            }
            _ => panic!("Expected BackupCreate command"),
        }
    }

    #[test]
    fn test_migrate_command() {
        let cli = parse(&[
            "veyron",
            "migrate",
            "vm1",
            "--target-node",
            "node2",
            "--plan",
        ])
        .unwrap();
        match *cli.command {
            Commands::Migrate {
                vm,
                target_node,
                plan,
                ..
            } => {
                assert_eq!(vm, "vm1");
                assert_eq!(target_node, Some("node2".to_string()));
                assert!(plan);
            }
            _ => panic!("Expected Migrate command"),
        }
    }

    #[test]
    fn test_events_command() {
        let cli = parse(&["veyron", "events", "--limit", "25"]).unwrap();
        match *cli.command {
            Commands::EventsList { limit, .. } => assert_eq!(limit, 25),
            _ => panic!("Expected EventsList"),
        }
    }

    #[test]
    fn test_nodes_command() {
        let cli = parse(&["veyron", "nodes"]).unwrap();
        assert!(matches!(*cli.command, Commands::NodesList { .. }));
    }

    #[test]
    fn test_pods_command() {
        let cli = parse(&["veyron", "pods", "--output", "json"]).unwrap();
        match *cli.command {
            Commands::PodsList { output } => assert_eq!(output, "json"),
            _ => panic!("Expected PodsList"),
        }
    }

    #[test]
    fn test_import_command() {
        let cli = parse(&["veyron", "import", "vm.yaml", "--start", "--dry-run"]).unwrap();
        match *cli.command {
            Commands::Import {
                ref file,
                start,
                dry_run,
            } => {
                assert_eq!(file, "vm.yaml");
                assert!(start);
                assert!(dry_run);
            }
            _ => panic!("Expected Import"),
        }
    }

    #[test]
    fn test_search_command() {
        let cli = parse(&["veyron", "search", "running", "vms", "in", "production"]).unwrap();
        match *cli.command {
            Commands::Search { ref query } => {
                assert_eq!(query, &["running", "vms", "in", "production"]);
            }
            _ => panic!("Expected Search"),
        }
    }

    #[test]
    fn test_troubleshoot_command() {
        let cli = parse(&["veyron", "troubleshoot", "my-vm"]).unwrap();
        match *cli.command {
            Commands::Troubleshoot { ref name } => assert_eq!(name, "my-vm"),
            _ => panic!("Expected Troubleshoot"),
        }
    }

    #[test]
    fn test_capacity_command() {
        let cli = parse(&["veyron", "capacity", "--detailed"]).unwrap();
        match *cli.command {
            Commands::Capacity { detailed, .. } => assert!(detailed),
            _ => panic!("Expected Capacity"),
        }
    }

    #[test]
    fn test_placement_command() {
        let cli = parse(&["veyron", "placement", "my-vm", "--strategy", "binpack"]).unwrap();
        match *cli.command {
            Commands::Placement {
                ref name,
                ref strategy,
            } => {
                assert_eq!(name, "my-vm");
                assert_eq!(strategy, "binpack");
            }
            _ => panic!("Expected Placement"),
        }
    }

    #[test]
    fn test_clusters_list_command() {
        let cli = parse(&["veyron", "clusters-list"]).unwrap();
        assert!(matches!(*cli.command, Commands::ClustersList { .. }));
    }

    #[test]
    fn test_clusters_discover_command() {
        let cli = parse(&["veyron", "clusters-discover"]).unwrap();
        assert!(matches!(*cli.command, Commands::ClustersDiscover));
    }

    #[test]
    fn test_gitops_diff_command() {
        let cli = parse(&["veyron", "gitops-diff", "--directory", "manifests"]).unwrap();
        match *cli.command {
            Commands::GitopsDiff { ref directory } => assert_eq!(directory, "manifests"),
            _ => panic!("Expected GitopsDiff"),
        }
    }

    #[test]
    fn test_gitops_status_command() {
        let cli = parse(&["veyron", "gitops-status"]).unwrap();
        assert!(matches!(*cli.command, Commands::GitopsStatus));
    }

    #[test]
    fn test_list_json_command() {
        let cli = parse(&["veyron", "list-json"]).unwrap();
        assert!(matches!(*cli.command, Commands::ListJson));
    }
}
