// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// SecretKeyRef references a key in a Kubernetes Secret.
type SecretKeyRef struct {
	// Secret name.
	Name string `json:"name"`

	// Key within the Secret.
	Key string `json:"key"`

	// Secret namespace (defaults to VM namespace).
	// +optional
	Namespace string `json:"namespace,omitempty"`
}

// WindowsSpec holds Windows-specific bootstrap secret references.
type WindowsSpec struct {
	// Sysprep unattend.xml or answer file in a Secret.
	// +optional
	SysprepSecretRef *SecretKeyRef `json:"sysprepSecretRef,omitempty"`

	// Domain join credentials JSON in a Secret (domain, ou, user, password).
	// +optional
	DomainJoinSecretRef *SecretKeyRef `json:"domainJoinSecretRef,omitempty"`
}

// VeyronVMSpec defines the desired state of a virtual machine.
// Fields mirror the Rust VMConfig from src/config/types.rs.
type VeyronVMSpec struct {
	// Template name from Veyron template library (e.g. "ubuntu-22.04", "fedora-41").
	// +optional
	Template string `json:"template,omitempty"`

	// Profile name for resource presets (e.g. "minimal", "dev", "prod", "database").
	// +optional
	Profile string `json:"profile,omitempty"`

	// CPU configuration.
	CPU CPUSpec `json:"cpu"`

	// Memory configuration.
	Memory MemorySpec `json:"memory"`

	// Disk definitions.
	// +optional
	Disks []DiskSpec `json:"disks,omitempty"`

	// Network interfaces.
	// +optional
	Interfaces []InterfaceSpec `json:"interfaces,omitempty"`

	// Cloud-init configuration.
	// +optional
	CloudInit *CloudInitSpec `json:"cloudInit,omitempty"`

	// VM features (ACPI, HyperV, SMM).
	// +optional
	Features *FeaturesSpec `json:"features,omitempty"`

	// Firmware/bootloader configuration.
	// +optional
	Firmware *FirmwareSpec `json:"firmware,omitempty"`

	// Clock configuration.
	// +optional
	Clock *ClockSpec `json:"clock,omitempty"`

	// Eviction strategy (e.g. "LiveMigrate", "LiveMigrateIfPossible").
	// +optional
	EvictionStrategy *string `json:"evictionStrategy,omitempty"`

	// Termination grace period in seconds.
	// +optional
	TerminationGracePeriod *int64 `json:"terminationGracePeriod,omitempty"`

	// Enable TPM 2.0 device.
	// +optional
	EnableTPM bool `json:"enableTpm,omitempty"`

	// Enable virtio-rng device.
	// +optional
	EnableRNG bool `json:"enableRng,omitempty"`

	// Machine type (e.g. "q35").
	// +optional
	MachineType *string `json:"machineType,omitempty"`

	// Running state: true to start, false to stop.
	// +optional
	Running *bool `json:"running,omitempty"`

	// AllowInternet applies a CiliumNetworkPolicy or NetworkPolicy so virt-launcher pods
	// for this VM can reach the internet (default true).
	// +optional
	// +kubebuilder:default=true
	AllowInternet *bool `json:"allowInternet,omitempty"`

	// Labels to apply to the KubeVirt VM.
	// +optional
	Labels map[string]string `json:"labels,omitempty"`

	// Annotations to apply to the KubeVirt VM.
	// +optional
	Annotations map[string]string `json:"annotations,omitempty"`

	// Windows-specific bootstrap (sysprep, domain join).
	// +optional
	Windows *WindowsSpec `json:"windows,omitempty"`

	// GPUs bound via KubeVirt domain.devices.gpus (whole-GPU passthrough or
	// vGPU mediated devices). A VM with a passthrough GPU can never
	// live-migrate.
	// +optional
	GPUs []GPUSpec `json:"gpus,omitempty"`

	// Generic passthrough host devices (domain.devices.hostDevices).
	// +optional
	HostDevices []HostDeviceSpec `json:"hostDevices,omitempty"`
}

// GPUSpec mirrors KubeVirt's domain.devices.gpus entry.
type GPUSpec struct {
	// Device alias inside the VM spec (e.g. "gpu0").
	Name string `json:"name"`
	// Device-plugin resource, e.g. "nvidia.com/gpu" or "nvidia.com/GRID_T4-2Q".
	DeviceName string `json:"deviceName"`
	// vGPU display options (mediated devices only).
	// +optional
	VirtualGPUOptions *VGPUOptionsSpec `json:"virtualGPUOptions,omitempty"`
}

// VGPUOptionsSpec mirrors KubeVirt's virtualGPUOptions.display.
type VGPUOptionsSpec struct {
	// Enable the vGPU display head.
	// +optional
	Display *bool `json:"display,omitempty"`
	// Enable the RAM framebuffer.
	// +optional
	RAMFB *bool `json:"ramFB,omitempty"`
}

// HostDeviceSpec mirrors KubeVirt's domain.devices.hostDevices entry.
type HostDeviceSpec struct {
	// Device alias inside the VM spec.
	Name string `json:"name"`
	// Device-plugin resource name permitted via permittedHostDevices.
	DeviceName string `json:"deviceName"`
}

// CPUSpec mirrors Rust CPUConfig.
type CPUSpec struct {
	// Number of CPU cores.
	Cores uint32 `json:"cores"`

	// Number of CPU sockets.
	// +optional
	// +kubebuilder:default=1
	Sockets uint32 `json:"sockets,omitempty"`

	// Number of threads per core.
	// +optional
	// +kubebuilder:default=1
	Threads uint32 `json:"threads,omitempty"`

	// CPU model (e.g. "host-passthrough", "Haswell").
	// +optional
	Model *string `json:"model,omitempty"`

	// Pin vCPUs to physical CPUs for latency-sensitive workloads.
	// +optional
	DedicatedCPUPlacement *bool `json:"dedicatedCpuPlacement,omitempty"`

	// Isolate QEMU emulator thread from vCPU threads.
	// +optional
	IsolateEmulatorThread *bool `json:"isolateEmulatorThread,omitempty"`
}

// MemorySpec mirrors Rust MemoryConfig.
type MemorySpec struct {
	// Memory size (e.g. "4Gi").
	Size string `json:"size"`

	// Hugepages page size (e.g. "2Mi", "1Gi").
	// +optional
	HugepagesPageSize *string `json:"hugepagesPageSize,omitempty"`

	// Maximum guest memory for hotplug (e.g. "16Gi").
	// +optional
	MaxGuest *string `json:"maxGuest,omitempty"`
}

// DiskSpec mirrors Rust DiskConfig.
type DiskSpec struct {
	// Disk name.
	Name string `json:"name"`

	// Disk size (e.g. "20Gi").
	Size string `json:"size"`

	// Storage class for PVC-backed disks.
	// +optional
	StorageClass *string `json:"storageClass,omitempty"`

	// Boot order (1 = first boot device).
	BootOrder uint32 `json:"bootOrder"`

	// Disk source (blank, pvc, containerDisk, dataVolume).
	Source DiskSource `json:"source"`

	// Device type: "disk" (default), "cdrom", or "lun".
	// +optional
	// +kubebuilder:default="disk"
	// +kubebuilder:validation:Enum=disk;cdrom;lun
	DeviceType string `json:"deviceType,omitempty"`

	// Bus type: "virtio" (default), "sata", "scsi".
	// +optional
	Bus *string `json:"bus,omitempty"`

	// Cache mode: "none", "writethrough", "writeback".
	// +optional
	Cache *string `json:"cache,omitempty"`

	// I/O mode: "native", "threads", "default".
	// +optional
	IO *string `json:"io,omitempty"`
}

// DiskSource mirrors Rust DiskSource enum.
type DiskSource struct {
	// Source type. "dataSource" clones a per-VM disk from a golden image at create
	// time (emitted as a dataVolumeTemplate); "pvc"/"dataVolume" only *reference* an
	// existing volume, which two VMs must never share.
	// +kubebuilder:validation:Enum=blank;pvc;containerDisk;dataVolume;dataSource
	Type string `json:"type"`

	// Name of the PVC, DataVolume, or golden-image DataSource.
	// +optional
	Name string `json:"name,omitempty"`

	// Container disk image (e.g. "quay.io/containerdisks/ubuntu:22.04").
	// +optional
	Image string `json:"image,omitempty"`

	// Namespace holding the golden-image DataSource / source PVC ("dataSource" type).
	// +optional
	Namespace string `json:"namespace,omitempty"`

	// Clone straight from a PVC instead of resolving a DataSource ("dataSource" type).
	// +optional
	FromPvc bool `json:"fromPvc,omitempty"`
}

// InterfaceSpec mirrors Rust InterfaceConfig.
type InterfaceSpec struct {
	// Interface name.
	Name string `json:"name"`

	// Network name.
	Network string `json:"network"`

	// NIC model (e.g. "virtio", "e1000").
	// +optional
	// +kubebuilder:default="virtio"
	Model string `json:"model,omitempty"`

	// Network type configuration.
	NetworkType NetworkType `json:"networkType"`

	// MAC address (e.g. "52:54:00:12:34:56").
	// +optional
	MacAddress *string `json:"macAddress,omitempty"`
}

// NetworkType mirrors Rust NetworkType enum.
type NetworkType struct {
	// Network type.
	// +kubebuilder:validation:Enum=pod;bridge;multus;sriov
	Type string `json:"type"`

	// Network attachment name (for multus/sriov).
	// +optional
	Name string `json:"name,omitempty"`
}

// CloudInitSpec mirrors Rust CloudInitConfig.
type CloudInitSpec struct {
	// Cloud-init user data (inline; prefer userDataSecretRef in GitOps).
	// +optional
	UserData string `json:"userData,omitempty"`

	// Reference to userData in a Secret.
	// +optional
	UserDataSecretRef *SecretKeyRef `json:"userDataSecretRef,omitempty"`

	// Cloud-init network data.
	// +optional
	NetworkData *string `json:"networkData,omitempty"`

	// Delivery mechanism: empty or "nocloud" (default), or "configdrive" for Cloudbase-Init / Windows.
	// +optional
	Delivery string `json:"delivery,omitempty"`
}

// FeaturesSpec mirrors Rust FeaturesConfig.
type FeaturesSpec struct {
	// Enable ACPI (default: true for most guests).
	// +optional
	// +kubebuilder:default=true
	ACPI bool `json:"acpi,omitempty"`

	// Enable APIC.
	// +optional
	APIC bool `json:"apic,omitempty"`

	// HyperV enlightenments (critical for Windows performance).
	// +optional
	HyperV *HyperVSpec `json:"hyperv,omitempty"`

	// Hide KVM from guest (e.g. for GPU passthrough).
	// +optional
	KVMHidden *bool `json:"kvmHidden,omitempty"`

	// System Management Mode (required for UEFI Secure Boot).
	// +optional
	SMM *bool `json:"smm,omitempty"`
}

// HyperVSpec mirrors Rust HyperVConfig.
type HyperVSpec struct {
	Relaxed         bool    `json:"relaxed,omitempty"`
	VAPIC           bool    `json:"vapic,omitempty"`
	Spinlocks       *uint32 `json:"spinlocks,omitempty"`
	VPIndex         bool    `json:"vpindex,omitempty"`
	Runtime         bool    `json:"runtime,omitempty"`
	SyNIC           bool    `json:"synic,omitempty"`
	STimer          bool    `json:"stimer,omitempty"`
	Reset           bool    `json:"reset,omitempty"`
	Frequencies     bool    `json:"frequencies,omitempty"`
	Reenlightenment bool    `json:"reenlightenment,omitempty"`
	TLBFlush        bool    `json:"tlbflush,omitempty"`
	IPI             bool    `json:"ipi,omitempty"`
}

// FirmwareSpec mirrors Rust FirmwareConfig.
type FirmwareSpec struct {
	// Bootloader type.
	// +kubebuilder:validation:Enum=bios;efi
	Bootloader string `json:"bootloader"`

	// Enable UEFI Secure Boot (EFI only).
	// +optional
	SecureBoot bool `json:"secureBoot,omitempty"`

	// Persist NVRAM across reboots (EFI only).
	// +optional
	Persistent bool `json:"persistent,omitempty"`
}

// ClockSpec mirrors Rust ClockConfig.
type ClockSpec struct {
	// Use UTC offset.
	// +optional
	// +kubebuilder:default=true
	UTC bool `json:"utc,omitempty"`

	// Timezone (e.g. "America/New_York").
	// +optional
	Timezone *string `json:"timezone,omitempty"`

	// Timer configuration.
	// +optional
	Timers *TimersSpec `json:"timers,omitempty"`
}

// TimersSpec mirrors Rust TimersConfig.
type TimersSpec struct {
	// HPET timer present.
	// +optional
	HPETPresent *bool `json:"hpetPresent,omitempty"`

	// PIT tick policy ("delay", "catchup", "merge", "discard").
	// +optional
	PITTickPolicy *string `json:"pitTickPolicy,omitempty"`

	// RTC tick policy ("delay", "catchup", "merge", "discard").
	// +optional
	RTCTickPolicy *string `json:"rtcTickPolicy,omitempty"`

	// Enable HyperV timer (for Windows guests).
	// +optional
	HyperVPresent *bool `json:"hypervPresent,omitempty"`
}

// VeyronVMPhase describes the lifecycle phase of a VeyronVM.
// +kubebuilder:validation:Enum=Pending;Creating;Running;Stopped;Failed;Deleting;Unknown
type VeyronVMPhase string

const (
	VMPhasePending  VeyronVMPhase = "Pending"
	VMPhaseCreating VeyronVMPhase = "Creating"
	VMPhaseRunning  VeyronVMPhase = "Running"
	VMPhaseStopped  VeyronVMPhase = "Stopped"
	VMPhaseFailed   VeyronVMPhase = "Failed"
	VMPhaseDeleting VeyronVMPhase = "Deleting"
	VMPhaseUnknown  VeyronVMPhase = "Unknown"
)

// VeyronVMStatus defines the observed state of VeyronVM.
type VeyronVMStatus struct {
	// Current lifecycle phase.
	// +optional
	Phase VeyronVMPhase `json:"phase,omitempty"`

	// Name of the owned KubeVirt VirtualMachine.
	// +optional
	KubevirtVMName string `json:"kubevirtVMName,omitempty"`

	// Node where the VM is running.
	// +optional
	NodeName string `json:"nodeName,omitempty"`

	// IP address of the running VM.
	// +optional
	IPAddress string `json:"ipAddress,omitempty"`

	// Guest OS detected by the VMI agent.
	// +optional
	GuestOS string `json:"guestOS,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`

	// Timestamp of the last reconciliation.
	// +optional
	LastReconciled *metav1.Time `json:"lastReconciled,omitempty"`

	// Generation observed by the controller.
	// +optional
	ObservedGeneration int64 `json:"observedGeneration,omitempty"`

	// SHA256 hash of the resolved spec (template + profile + overrides).
	// +optional
	ResolvedSpecHash string `json:"resolvedSpecHash,omitempty"`

	// True when KubeVirt VM spec differs from resolved VeyronVM spec.
	// +optional
	DriftDetected bool `json:"driftDetected,omitempty"`

	// Human-readable drift summary.
	// +optional
	DriftMessage string `json:"driftMessage,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Phase",type=string,JSONPath=`.status.phase`
// +kubebuilder:printcolumn:name="VM",type=string,JSONPath=`.status.kubevirtVMName`
// +kubebuilder:printcolumn:name="Node",type=string,JSONPath=`.status.nodeName`
// +kubebuilder:printcolumn:name="IP",type=string,JSONPath=`.status.ipAddress`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:shortName=vrvm

// VeyronVM is the Schema for the veyronvms API.
type VeyronVM struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VeyronVMSpec   `json:"spec,omitempty"`
	Status VeyronVMStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VeyronVMList contains a list of VeyronVM.
type VeyronVMList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VeyronVM `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VeyronVM{}, &VeyronVMList{})
}
