// Package converter translates VMRogueVM specs into KubeVirt VirtualMachine resources.
// This is the Go equivalent of the Rust function vm_config_to_kubevirt in src/kube/converter.rs.
package converter

import (
	"strings"

	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

// VMRogueVMToKubeVirt converts a VMRogueVM CR into a KubeVirt VirtualMachine unstructured object.
// Uses unstructured to avoid importing the full KubeVirt Go module.
func VMRogueVMToKubeVirt(vm *vmroguev1alpha1.VMRogueVM) (*unstructured.Unstructured, error) {
	spec := &vm.Spec

	// Build labels
	labels := map[string]interface{}{
		"kubevirt.io/vm":          vm.Name,
		"app.kubernetes.io/name":  vm.Name,
		"vmrogue.io/managed-by":   "vmrogue-operator",
		"vmrogue.io/vmrogue-vm":   vm.Name,
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

	// Build disks
	disks := buildDisks(spec)

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
			APIVersion:         vmroguev1alpha1.GroupVersion.String(),
			Kind:               "VMRogueVM",
			Name:               vm.Name,
			UID:                vm.UID,
			Controller:         &controller,
			BlockOwnerDeletion: &blockDeletion,
		},
	})

	return kvVM, nil
}

func buildVolumes(vmName string, spec *vmroguev1alpha1.VMRogueVMSpec) []interface{} {
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
		}

		volumes = append(volumes, vol)
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
	s := vmName + "-vmrogue-cfgdrv"
	runes := []rune(s)
	const maxKubeName = 253
	if len(runes) <= maxKubeName {
		return s
	}
	return string(runes[:maxKubeName])
}

func buildDisks(spec *vmroguev1alpha1.VMRogueVMSpec) []interface{} {
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

	return disks
}

func buildInterfaces(spec *vmroguev1alpha1.VMRogueVMSpec) []interface{} {
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

func buildNetworks(spec *vmroguev1alpha1.VMRogueVMSpec) []interface{} {
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

func buildDevices(disks, interfaces []interface{}, spec *vmroguev1alpha1.VMRogueVMSpec) map[string]interface{} {
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

	return devices
}

func buildFeatures(f *vmroguev1alpha1.FeaturesSpec) map[string]interface{} {
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

func buildClock(c *vmroguev1alpha1.ClockSpec) map[string]interface{} {
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

func buildFirmware(f *vmroguev1alpha1.FirmwareSpec) map[string]interface{} {
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

// GetKubeVirtVMName returns the name of the KubeVirt VM for a VMRogueVM.
func GetKubeVirtVMName(vmName string) string {
	return vmName
}
