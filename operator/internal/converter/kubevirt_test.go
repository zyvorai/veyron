// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package converter

import (
	"testing"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

func baseVM() *veyronv1alpha1.VeyronVM {
	return &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{Name: "test-vm", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronVMSpec{
			CPU:    veyronv1alpha1.CPUSpec{Cores: 2, Sockets: 1, Threads: 1},
			Memory: veyronv1alpha1.MemorySpec{Size: "4Gi"},
			Disks: []veyronv1alpha1.DiskSpec{
				{
					Name: "rootdisk",
					Size: "10Gi",
					Source: veyronv1alpha1.DiskSource{
						Type: "blank",
					},
				},
			},
			Interfaces: []veyronv1alpha1.InterfaceSpec{
				{
					Name:        "default",
					Network:     "default",
					NetworkType: veyronv1alpha1.NetworkType{Type: "pod"},
				},
			},
		},
	}
}

func devicesOf(t *testing.T, obj *unstructured.Unstructured) map[string]interface{} {
	t.Helper()
	devices, found, err := unstructured.NestedMap(obj.Object,
		"spec", "template", "spec", "domain", "devices")
	if err != nil || !found {
		t.Fatalf("devices not found: %v", err)
	}
	return devices
}

func TestBuildDevicesEmitsGpus(t *testing.T) {
	vm := baseVM()
	display := true
	vm.Spec.GPUs = []veyronv1alpha1.GPUSpec{
		{Name: "gpu0", DeviceName: "nvidia.com/gpu"},
		{
			Name:       "gpu1",
			DeviceName: "nvidia.com/GRID_T4-2Q",
			VirtualGPUOptions: &veyronv1alpha1.VGPUOptionsSpec{
				Display: &display,
			},
		},
	}

	obj, err := VeyronVMToKubeVirt(vm)
	if err != nil {
		t.Fatalf("convert: %v", err)
	}
	devices := devicesOf(t, obj)

	gpus, ok := devices["gpus"].([]interface{})
	if !ok || len(gpus) != 2 {
		t.Fatalf("expected 2 gpus, got %#v", devices["gpus"])
	}
	first := gpus[0].(map[string]interface{})
	if first["name"] != "gpu0" || first["deviceName"] != "nvidia.com/gpu" {
		t.Fatalf("unexpected gpu[0]: %#v", first)
	}
	if _, has := first["virtualGPUOptions"]; has {
		t.Fatalf("gpu[0] must not carry virtualGPUOptions: %#v", first)
	}
	second := gpus[1].(map[string]interface{})
	vgpuOpts, ok := second["virtualGPUOptions"].(map[string]interface{})
	if !ok {
		t.Fatalf("gpu[1] missing virtualGPUOptions: %#v", second)
	}
	displayOpts := vgpuOpts["display"].(map[string]interface{})
	if displayOpts["enabled"] != true {
		t.Fatalf("unexpected display opts: %#v", displayOpts)
	}
}

func TestBuildDevicesEmitsHostDevices(t *testing.T) {
	vm := baseVM()
	vm.Spec.HostDevices = []veyronv1alpha1.HostDeviceSpec{
		{Name: "audio0", DeviceName: "nvidia.com/hda-audio"},
	}

	obj, err := VeyronVMToKubeVirt(vm)
	if err != nil {
		t.Fatalf("convert: %v", err)
	}
	devices := devicesOf(t, obj)

	hds, ok := devices["hostDevices"].([]interface{})
	if !ok || len(hds) != 1 {
		t.Fatalf("expected 1 hostDevice, got %#v", devices["hostDevices"])
	}
	hd := hds[0].(map[string]interface{})
	// KubeVirt's schema field is deviceName — resourceName would be pruned.
	if hd["deviceName"] != "nvidia.com/hda-audio" {
		t.Fatalf("unexpected hostDevice: %#v", hd)
	}
}

func TestBuildDevicesOmitsGpuKeysWhenUnset(t *testing.T) {
	obj, err := VeyronVMToKubeVirt(baseVM())
	if err != nil {
		t.Fatalf("convert: %v", err)
	}
	devices := devicesOf(t, obj)
	if _, has := devices["gpus"]; has {
		t.Fatal("gpus must be omitted when no GPU is requested")
	}
	if _, has := devices["hostDevices"]; has {
		t.Fatal("hostDevices must be omitted when none are requested")
	}
}
