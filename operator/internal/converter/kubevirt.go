// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Package converter translates VeyronVM specs into KubeVirt VirtualMachine resources.
// This is the Go equivalent of the Rust function vm_config_to_kubevirt in src/kube/converter.rs.
package converter

import (
	"os"
	"strings"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

// VeyronVMToKubeVirt converts a VeyronVM CR into a KubeVirt VirtualMachine unstructured object.
// Uses unstructured to avoid importing the full KubeVirt Go module.
func VeyronVMToKubeVirt(vm *veyronv1alpha1.VeyronVM) (*unstructured.Unstructured, error) {
	spec := &vm.Spec

	// Build labels
	labels := map[string]interface{}{
		"kubevirt.io/vm":         vm.Name,
		"app.kubernetes.io/name": vm.Name,
		"veyron.io/managed-by":   "veyron-operator",
		"veyron.io/veyron-vm":    vm.Name,
	}
	for k, v := range spec.Labels {
		labels[k] = v
	}

	// Build annotations
	annotations := map[string]interface{}{}
	for k, v := range spec.Annotations {
		annotations[k] = v
	}

	// Build volumes
	volumes := buildVolumes(vm.Name, spec)

	// Golden-image disks additionally own a cloned DataVolume.
	dataVolumeTemplates := buildDataVolumeTemplates(vm.Name, spec)

	// Build disks
	disks := buildDisks(vm.Name, spec)

	// Build interfaces
	interfaces := buildInterfaces(spec)

	// Build networks
	networks := buildNetworks(spec)

	// Build resource requests
	requests := map[string]interface{}{
		"memory": spec.Memory.Size,
	}

	// Build domain spec
	domain := map[string]interface{}{
		"cpu": map[string]interface{}{
			"cores":   int64(spec.CPU.Cores),
			"sockets": int64(defaultUint32(spec.CPU.Sockets, 1)),
			"threads": int64(defaultUint32(spec.CPU.Threads, 1)),
		},
		"resources": map[string]interface{}{
			"requests": requests,
		},
		"devices": buildDevices(disks, interfaces, spec),
	}

	// CPU model
	if spec.CPU.Model != nil {
		domain["cpu"].(map[string]interface{})["model"] = *spec.CPU.Model
	}

	// Dedicated CPU placement
	if spec.CPU.DedicatedCPUPlacement != nil && *spec.CPU.DedicatedCPUPlacement {
		domain["cpu"].(map[string]interface{})["dedicatedCpuPlacement"] = true
	}
	if spec.CPU.IsolateEmulatorThread != nil && *spec.CPU.IsolateEmulatorThread {
		domain["cpu"].(map[string]interface{})["isolateEmulatorThread"] = true
	}

	// Memory hugepages
	if spec.Memory.HugepagesPageSize != nil {
		domain["memory"] = map[string]interface{}{
			"hugepages": map[string]interface{}{
				"pageSize": *spec.Memory.HugepagesPageSize,
			},
		}
	}

	// Features
	if spec.Features != nil {
		domain["features"] = buildFeatures(spec.Features)
	}

	// Clock
	if spec.Clock != nil {
		domain["clock"] = buildClock(spec.Clock)
	}

	// Firmware
	if spec.Firmware != nil {
		domain["firmware"] = buildFirmware(spec.Firmware)
	}

	// Machine type
	if spec.MachineType != nil {
		domain["machine"] = map[string]interface{}{
			"type": *spec.MachineType,
		}
	}

	// Running state
	running := false
	if spec.Running != nil {
		running = *spec.Running
	}

	// Termination grace period
	terminationGracePeriod := int64(30)
	if spec.TerminationGracePeriod != nil {
		terminationGracePeriod = *spec.TerminationGracePeriod
	}

	// Build the VirtualMachine
	kvVM := &unstructured.Unstructured{
		Object: map[string]interface{}{
			"apiVersion": "kubevirt.io/v1",
			"kind":       "VirtualMachine",
			"metadata": map[string]interface{}{
				"name":      vm.Name,
				"namespace": vm.Namespace,
				"labels":    labels,
			},
			"spec": map[string]interface{}{
				"running": running,
				"template": map[string]interface{}{
					"metadata": map[string]interface{}{
						"labels": labels,
					},
					"spec": map[string]interface{}{
						"domain":                        domain,
						"volumes":                       volumes,
						"networks":                      networks,
						"terminationGracePeriodSeconds": terminationGracePeriod,
					},
				},
			},
		},
	}

	// Per-VM cloned disks (golden images).
	if len(dataVolumeTemplates) > 0 {
		vmSpec := kvVM.Object["spec"].(map[string]interface{})
		vmSpec["dataVolumeTemplates"] = dataVolumeTemplates
	}

	// Add annotations if non-empty
	if len(annotations) > 0 {
		metadata := kvVM.Object["metadata"].(map[string]interface{})
		metadata["annotations"] = annotations
	}

	// Eviction strategy
	if spec.EvictionStrategy != nil {
		templateSpec := kvVM.Object["spec"].(map[string]interface{})["template"].(map[string]interface{})["spec"].(map[string]interface{})
		templateSpec["evictionStrategy"] = *spec.EvictionStrategy
	}

	// Set owner reference with Controller=true for proper GC and watch behavior
	controller := true
	blockDeletion := true
	kvVM.SetOwnerReferences([]metav1.OwnerReference{
		{
			APIVersion:         veyronv1alpha1.GroupVersion.String(),
			Kind:               "VeyronVM",
			Name:               vm.Name,
			UID:                vm.UID,
			Controller:         &controller,
			BlockOwnerDeletion: &blockDeletion,
		},
	})

	return kvVM, nil
}

// SysprepVolumeName is the volume/disk name for the Windows unattend CD-ROM.
// Must match the Rust converter (src/kube/converter.rs).
const SysprepVolumeName = "sysprep"

// SysprepSecretName is the operator-managed answer-file Secret, created when a
// domain join is requested (and merged with the caller's answer file if they
// supplied one). Kept distinct from the user's own sysprepSecretRef so the operator
// never writes into a Secret it does not own.
func SysprepSecretName(vmName string) string {
	return vmName + "-veyron-sysprep"
}

// EffectiveSysprepSecret is the Secret actually mounted as sysprep media, and "" when
// the VM needs none.
//
// A domain join wins the mount, because the operator-managed Secret *contains* the
// caller's answer file with the join injected into it — mounting the caller's
// original instead would silently drop the join.
func EffectiveSysprepSecret(vmName string, spec *veyronv1alpha1.VeyronVMSpec) string {
	if spec == nil || spec.Windows == nil {
		return ""
	}
	if spec.Windows.DomainJoinSecretRef != nil {
		return SysprepSecretName(vmName)
	}
	if spec.Windows.SysprepSecretRef != nil {
		return spec.Windows.SysprepSecretRef.Name
	}
	return ""
}

// goldenDataVolumeName names the per-VM clone of a golden image. Keyed by VM +
// disk so two VMs built from the same image never target the same DataVolume.
func goldenDataVolumeName(vmName, diskName string) string {
	return vmName + "-" + diskName
}

// buildDataVolumeTemplates emits a VM-owned CDI DataVolume for each golden-image
// disk, so the VM boots a private clone that is garbage-collected with it.
//
// sourceRef -> DataSource is preferred over an inline PVC source: the DataSource is
// the stable catalog handle, so publishing a new image version repoints every future
// clone without editing a single VM.
func buildDataVolumeTemplates(vmName string, spec *veyronv1alpha1.VeyronVMSpec) []interface{} {
	var templates []interface{}

	for _, disk := range spec.Disks {
		if disk.Source.Type != "dataSource" {
			continue
		}

		storage := map[string]interface{}{
			"resources": map[string]interface{}{
				"requests": map[string]interface{}{
					"storage": disk.Size,
				},
			},
		}
		if disk.StorageClass != nil && *disk.StorageClass != "" {
			storage["storageClassName"] = *disk.StorageClass
		}

		dvSpec := map[string]interface{}{
			"storage": storage,
		}
		if disk.Source.FromPvc {
			dvSpec["source"] = map[string]interface{}{
				"pvc": map[string]interface{}{
					"namespace": disk.Source.Namespace,
					"name":      disk.Source.Name,
				},
			}
		} else {
			sourceRef := map[string]interface{}{
				"kind": "DataSource",
				"name": disk.Source.Name,
			}
			if disk.Source.Namespace != "" {
				sourceRef["namespace"] = disk.Source.Namespace
			}
			dvSpec["sourceRef"] = sourceRef
		}

		templates = append(templates, map[string]interface{}{
			"metadata": map[string]interface{}{
				"name": goldenDataVolumeName(vmName, disk.Name),
			},
			"spec": dvSpec,
		})
	}

	return templates
}

func buildVolumes(vmName string, spec *veyronv1alpha1.VeyronVMSpec) []interface{} {
	var volumes []interface{}

	for _, disk := range spec.Disks {
		vol := map[string]interface{}{
			"name": disk.Name,
		}

		switch disk.Source.Type {
		case "blank":
			vol["emptyDisk"] = map[string]interface{}{
				"capacity": disk.Size,
			}
		case "pvc":
			vol["persistentVolumeClaim"] = map[string]interface{}{
				"claimName": disk.Source.Name,
			}
		case "containerDisk":
			vol["containerDisk"] = map[string]interface{}{
				"image":           disk.Source.Image,
				"imagePullPolicy": "IfNotPresent",
			}
		case "dataVolume":
			vol["dataVolume"] = map[string]interface{}{
				"name": disk.Source.Name,
			}
		case "dataSource":
			// The VM boots its own clone; buildDataVolumeTemplates creates it.
			vol["dataVolume"] = map[string]interface{}{
				"name": goldenDataVolumeName(vmName, disk.Name),
			}
		}

		volumes = append(volumes, vol)
	}

	// Windows unattended setup: autounattend.xml as sysprep CD-ROM media.
	if sysprepSecret := EffectiveSysprepSecret(vmName, spec); sysprepSecret != "" {
		volumes = append(volumes, map[string]interface{}{
			"name": SysprepVolumeName,
			"sysprep": map[string]interface{}{
				"secret": map[string]interface{}{
					"name": sysprepSecret,
				},
			},
		})
	}

	// Cloud-init volume (NoCloud vs config-drive / Cloudbase-Init)
	if spec.CloudInit != nil {
		d := strings.ToLower(strings.TrimSpace(spec.CloudInit.Delivery))
		if d == "configdrive" || d == "config_drive" {
			secretName := ConfigDriveSecretName(vmName)
			volumes = append(volumes, map[string]interface{}{
				"name": "cloudinitdisk",
				"cloudInitConfigDrive": map[string]interface{}{
					"userDataSecretRef": map[string]interface{}{
						"name": secretName,
					},
				},
			})
		} else {
			ciVol := map[string]interface{}{
				"name": "cloudinitdisk",
				"cloudInitNoCloud": map[string]interface{}{
					"userData": spec.CloudInit.UserData,
				},
			}
			if spec.CloudInit.NetworkData != nil {
				ciVol["cloudInitNoCloud"].(map[string]interface{})["networkData"] = *spec.CloudInit.NetworkData
			}
			volumes = append(volumes, ciVol)
		}
	}

	return volumes
}

// ConfigDriveSecretName matches Rust kube::cloudinit_configdrive_secret_name.
func ConfigDriveSecretName(vmName string) string {
	s := vmName + "-veyron-cfgdrv"
	runes := []rune(s)
	const maxKubeName = 253
	if len(runes) <= maxKubeName {
		return s
	}
	return string(runes[:maxKubeName])
}

func buildDisks(vmName string, spec *veyronv1alpha1.VeyronVMSpec) []interface{} {
	var disks []interface{}

	for _, d := range spec.Disks {
		disk := map[string]interface{}{
			"name": d.Name,
		}

		// Boot order
		if d.BootOrder > 0 {
			disk["bootOrder"] = int64(d.BootOrder)
		}

		// Cache and IO
		if d.Cache != nil {
			disk["cache"] = *d.Cache
		}
		if d.IO != nil {
			disk["io"] = *d.IO
		}

		deviceType := d.DeviceType
		if deviceType == "" {
			deviceType = "disk"
		}

		bus := "virtio"
		if d.Bus != nil {
			bus = *d.Bus
		}

		switch deviceType {
		case "cdrom":
			cdromBus := "sata"
			if d.Bus != nil {
				cdromBus = *d.Bus
			}
			disk["cdrom"] = map[string]interface{}{
				"bus":      cdromBus,
				"readonly": true,
			}
		case "lun":
			lunBus := "scsi"
			if d.Bus != nil {
				lunBus = *d.Bus
			}
			disk["lun"] = map[string]interface{}{
				"bus": lunBus,
			}
		default:
			disk["disk"] = map[string]interface{}{
				"bus": bus,
			}
		}

		disks = append(disks, disk)
	}

	// Cloud-init disk
	if spec.CloudInit != nil {
		disks = append(disks, map[string]interface{}{
			"name": "cloudinitdisk",
			"disk": map[string]interface{}{
				"bus": "virtio",
			},
		})
	}

	// Sysprep media must be a CD-ROM: Windows Setup only reads autounattend.xml
	// from removable/optical media.
	if EffectiveSysprepSecret(vmName, spec) != "" {
		disks = append(disks, map[string]interface{}{
			"name": SysprepVolumeName,
			"cdrom": map[string]interface{}{
				"bus":      "sata",
				"readonly": true,
			},
		})
	}

	return disks
}

func buildInterfaces(spec *veyronv1alpha1.VeyronVMSpec) []interface{} {
	var interfaces []interface{}

	for _, iface := range spec.Interfaces {
		ifaceMap := map[string]interface{}{
			"name":  iface.Name,
			"model": iface.Model,
		}

		if iface.MacAddress != nil {
			ifaceMap["macAddress"] = *iface.MacAddress
		}

		switch iface.NetworkType.Type {
		case "pod":
			ifaceMap["masquerade"] = map[string]interface{}{}
		case "bridge":
			ifaceMap["bridge"] = map[string]interface{}{}
		case "multus":
			ifaceMap["bridge"] = map[string]interface{}{}
		case "sriov":
			ifaceMap["sriov"] = map[string]interface{}{}
		}

		interfaces = append(interfaces, ifaceMap)
	}

	return interfaces
}

func buildNetworks(spec *veyronv1alpha1.VeyronVMSpec) []interface{} {
	var networks []interface{}

	for _, iface := range spec.Interfaces {
		network := map[string]interface{}{
			"name": iface.Name,
		}

		switch iface.NetworkType.Type {
		case "pod", "bridge":
			network["pod"] = map[string]interface{}{}
		case "multus":
			network["multus"] = map[string]interface{}{
				"networkName": iface.NetworkType.Name,
			}
		case "sriov":
			network["multus"] = map[string]interface{}{
				"networkName": iface.NetworkType.Name,
			}
		}

		networks = append(networks, network)
	}

	return networks
}

func buildDevices(disks, interfaces []interface{}, spec *veyronv1alpha1.VeyronVMSpec) map[string]interface{} {
	devices := map[string]interface{}{
		"disks":      disks,
		"interfaces": interfaces,
	}

	// TPM
	if spec.EnableTPM {
		devices["tpm"] = map[string]interface{}{}
	}

	// RNG
	if spec.EnableRNG {
		devices["rng"] = map[string]interface{}{}
	}

	// Tablet input for better mouse support
	devices["inputs"] = []interface{}{
		map[string]interface{}{
			"type": "tablet",
			"name": "tablet0",
			"bus":  "usb",
		},
	}

	// GPUs (domain.devices.gpus): whole-GPU passthrough or vGPU mediated devices.
	if len(spec.GPUs) > 0 {
		gpus := make([]interface{}, 0, len(spec.GPUs))
		for _, g := range spec.GPUs {
			gpu := map[string]interface{}{
				"name":       g.Name,
				"deviceName": g.DeviceName,
			}
			if g.VirtualGPUOptions != nil {
				display := map[string]interface{}{}
				if g.VirtualGPUOptions.Display != nil {
					display["enabled"] = *g.VirtualGPUOptions.Display
				}
				if g.VirtualGPUOptions.RAMFB != nil {
					display["ramFB"] = map[string]interface{}{"enabled": *g.VirtualGPUOptions.RAMFB}
				}
				gpu["virtualGPUOptions"] = map[string]interface{}{"display": display}
			}
			gpus = append(gpus, gpu)
		}
		devices["gpus"] = gpus
	}

	// Generic passthrough host devices (domain.devices.hostDevices).
	if len(spec.HostDevices) > 0 {
		hostDevices := make([]interface{}, 0, len(spec.HostDevices))
		for _, h := range spec.HostDevices {
			hostDevices = append(hostDevices, map[string]interface{}{
				"name":       h.Name,
				"deviceName": h.DeviceName,
			})
		}
		devices["hostDevices"] = hostDevices
	}

	// KubeVirt 1.8+ auto-injects the guest-agent channel; explicit channels fail strict validation.
	if emitGuestAgentChannels() {
		devices["channels"] = []interface{}{
			map[string]interface{}{
				"name": "qemu",
				"target": map[string]interface{}{
					"type": "virtio",
					"name": "org.qemu.guest_agent.0",
				},
			},
		}
	}

	return devices
}

func emitGuestAgentChannels() bool {
	switch strings.ToLower(strings.TrimSpace(os.Getenv("VMROGUE_EMIT_GUEST_AGENT_CHANNELS"))) {
	case "1", "true", "yes":
		return true
	default:
		return false
	}
}

func buildFeatures(f *veyronv1alpha1.FeaturesSpec) map[string]interface{} {
	features := map[string]interface{}{}

	if f.ACPI {
		features["acpi"] = map[string]interface{}{}
	}
	if f.APIC {
		features["apic"] = map[string]interface{}{}
	}
	if f.SMM != nil && *f.SMM {
		features["smm"] = map[string]interface{}{
			"enabled": true,
		}
	}
	if f.KVMHidden != nil && *f.KVMHidden {
		features["kvm"] = map[string]interface{}{
			"hidden": true,
		}
	}

	if f.HyperV != nil {
		hyperv := map[string]interface{}{}
		if f.HyperV.Relaxed {
			hyperv["relaxed"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.VAPIC {
			hyperv["vapic"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.Spinlocks != nil {
			hyperv["spinlocks"] = map[string]interface{}{
				"enabled": true,
				"retries": int64(*f.HyperV.Spinlocks),
			}
		}
		if f.HyperV.VPIndex {
			hyperv["vpindex"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.Runtime {
			hyperv["runtime"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.SyNIC {
			hyperv["synic"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.STimer {
			hyperv["stimer"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.Reset {
			hyperv["reset"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.Frequencies {
			hyperv["frequencies"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.Reenlightenment {
			hyperv["reenlightenment"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.TLBFlush {
			hyperv["tlbflush"] = map[string]interface{}{"enabled": true}
		}
		if f.HyperV.IPI {
			hyperv["ipi"] = map[string]interface{}{"enabled": true}
		}
		features["hyperv"] = hyperv
	}

	return features
}

func buildClock(c *veyronv1alpha1.ClockSpec) map[string]interface{} {
	clock := map[string]interface{}{}

	if c.UTC {
		clock["utc"] = map[string]interface{}{}
	}
	if c.Timezone != nil {
		clock["timezone"] = *c.Timezone
	}

	if c.Timers != nil {
		// KubeVirt expects flat timer fields under clock.timer, not an array
		timer := map[string]interface{}{}

		if c.Timers.HPETPresent != nil {
			timer["hpet"] = map[string]interface{}{
				"present": *c.Timers.HPETPresent,
			}
		}
		if c.Timers.PITTickPolicy != nil {
			timer["pit"] = map[string]interface{}{
				"tickPolicy": *c.Timers.PITTickPolicy,
			}
		}
		if c.Timers.RTCTickPolicy != nil {
			timer["rtc"] = map[string]interface{}{
				"tickPolicy": *c.Timers.RTCTickPolicy,
			}
		}
		if c.Timers.HyperVPresent != nil && *c.Timers.HyperVPresent {
			timer["hyperv"] = map[string]interface{}{}
		}

		if len(timer) > 0 {
			clock["timer"] = timer
		}
	}

	return clock
}

func buildFirmware(f *veyronv1alpha1.FirmwareSpec) map[string]interface{} {
	firmware := map[string]interface{}{}

	switch f.Bootloader {
	case "bios":
		firmware["bootloader"] = map[string]interface{}{
			"bios": map[string]interface{}{},
		}
	case "efi":
		efi := map[string]interface{}{}
		if f.SecureBoot {
			efi["secureBoot"] = true
		}
		if f.Persistent {
			efi["persistent"] = true
		}
		firmware["bootloader"] = map[string]interface{}{
			"efi": efi,
		}
	}

	return firmware
}

func defaultUint32(v uint32, def uint32) uint32 {
	if v == 0 {
		return def
	}
	return v
}

// GetKubeVirtVMName returns the name of the KubeVirt VM for a VeyronVM.
func GetKubeVirtVMName(vmName string) string {
	return vmName
}
