// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
)

func strPtr(s string) *string { return &s }
func u32Ptr(v uint32) *uint32 { return &v }
func boolPtr(b bool) *bool      { return &b }

var windowsCloudInitUserData = `#ps1_sysnative
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name 'fDenyTSConnections' -Value 0
Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'
Write-Host 'Cloudbase-Init config-drive applied by VMRogue template.'
`

var windowsFeatures = &vmroguev1alpha1.FeaturesSpec{
	ACPI: true,
	APIC: true,
	HyperV: &vmroguev1alpha1.HyperVSpec{
		Relaxed:         true,
		VAPIC:           true,
		Spinlocks:       u32Ptr(8191),
		VPIndex:         true,
		Runtime:         true,
		SyNIC:           true,
		STimer:          true,
		Reset:           true,
		Frequencies:     true,
		Reenlightenment: true,
		TLBFlush:        true,
		IPI:             true,
	},
	SMM: boolPtr(true),
}

var windowsClock = &vmroguev1alpha1.ClockSpec{
	UTC: true,
	Timers: &vmroguev1alpha1.TimersSpec{
		HyperVPresent: boolPtr(true),
		PITTickPolicy: strPtr("delay"),
		RTCTickPolicy: strPtr("catchup"),
	},
}

func windows2022Default() vmroguev1alpha1.VMRogueVMSpec {
	sata := "sata"
	cacheNone := "none"
	return vmroguev1alpha1.VMRogueVMSpec{
		CPU:    vmroguev1alpha1.CPUSpec{Cores: 4, Sockets: 1, Threads: 1},
		Memory: vmroguev1alpha1.MemorySpec{Size: "8Gi"},
		Disks: []vmroguev1alpha1.DiskSpec{
			{
				Name: "rootdisk", Size: "60Gi", BootOrder: 1,
				Source: vmroguev1alpha1.DiskSource{Type: "blank"},
				DeviceType: "disk", Bus: &sata, Cache: &cacheNone,
			},
			{
				Name: "virtio-drivers", Size: "1Gi", BootOrder: 2,
				Source: vmroguev1alpha1.DiskSource{
					Type:  "containerDisk",
					Image: "quay.io/kubevirt/virtio-container-disk:v1.8.1",
				},
				DeviceType: "cdrom", Bus: &sata,
			},
		},
		Interfaces: []vmroguev1alpha1.InterfaceSpec{
			{
				Name: "default", Network: "default", Model: "virtio",
				NetworkType: vmroguev1alpha1.NetworkType{Type: "pod"},
			},
		},
		CloudInit: &vmroguev1alpha1.CloudInitSpec{
			UserData: windowsCloudInitUserData,
			Delivery: "configdrive",
		},
		Features:               windowsFeatures,
		Firmware:               &vmroguev1alpha1.FirmwareSpec{Bootloader: "efi"},
		Clock:                  windowsClock,
		EnableTPM:              true,
		EnableRNG:              true,
		MachineType:            strPtr("q35"),
		TerminationGracePeriod: int64Ptr(120),
	}
}

func ubuntu2204Default() vmroguev1alpha1.VMRogueVMSpec {
	virtio := "virtio"
	return vmroguev1alpha1.VMRogueVMSpec{
		CPU:    vmroguev1alpha1.CPUSpec{Cores: 2, Sockets: 1, Threads: 1},
		Memory: vmroguev1alpha1.MemorySpec{Size: "4Gi"},
		Disks: []vmroguev1alpha1.DiskSpec{
			{
				Name: "rootdisk", Size: "20Gi", BootOrder: 1,
				Source: vmroguev1alpha1.DiskSource{
					Type:  "containerDisk",
					Image: "quay.io/containerdisks/ubuntu:22.04",
				},
				DeviceType: "disk", Bus: &virtio,
			},
		},
		Interfaces: []vmroguev1alpha1.InterfaceSpec{
			{
				Name: "default", Network: "default", Model: "virtio",
				NetworkType: vmroguev1alpha1.NetworkType{Type: "pod"},
			},
		},
		EnableRNG:   true,
		MachineType: strPtr("q35"),
	}
}

func int64Ptr(v int64) *int64 { return &v }

// embeddedTemplates is the operator fallback when cluster VMTemplate CRs are absent.
// Regenerate via scripts/generate-catalog-crds.sh from Rust templates.
var embeddedTemplates = map[string]vmroguev1alpha1.VMTemplateSpec{
	"windows": {
		Description:         "Windows Server 2022 (alias)",
		Tags:                []string{"windows", "server"},
		Family:              "windows",
		RecommendedProfiles: []string{"prod"},
		Default:             windows2022Default(),
	},
	"windows-2022": {
		Description:         "Windows Server 2022 with Hyper-V enlightenments, UEFI, TPM, config-drive",
		Tags:                []string{"windows", "server", "2022"},
		Family:              "windows",
		RecommendedProfiles: []string{"prod"},
		Default:             windows2022Default(),
	},
	"ubuntu-22.04": {
		Description:         "Ubuntu 22.04 container disk",
		Tags:                []string{"linux", "ubuntu"},
		Family:              "linux",
		RecommendedProfiles: []string{"dev", "prod"},
		Default:             ubuntu2204Default(),
	},
	"ubuntu": {
		Description:         "Ubuntu 22.04 (alias)",
		Tags:                []string{"linux", "ubuntu"},
		Family:              "linux",
		RecommendedProfiles: []string{"dev"},
		Default:             ubuntu2204Default(),
	},
}

var embeddedProfiles = map[string]vmroguev1alpha1.VMProfileSpec{
	"dev": {
		Description: "Development - minimal resources",
		Cores:       1, Sockets: 1, Threads: 1,
		Memory: "2Gi", DiskSize: "10Gi",
		UseCases: []string{"testing", "learning"},
	},
	"prod": {
		Description: "Production - balanced resources",
		Cores:       4, Sockets: 1, Threads: 1,
		Memory: "8Gi", DiskSize: "40Gi",
		UseCases: []string{"production workloads"},
	},
	"database": {
		Description: "Database - I/O optimized",
		Cores:       6, Sockets: 1, Threads: 1,
		Memory: "16Gi", DiskSize: "200Gi",
		UseCases: []string{"PostgreSQL", "MySQL", "SQL Server"},
	},
	"web": {
		Description: "Web server",
		Cores:       4, Sockets: 1, Threads: 1,
		Memory: "8Gi", DiskSize: "40Gi",
		UseCases: []string{"IIS", "nginx", "Apache"},
	},
}
