// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"testing"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

func TestResolveWindows2022WithProdProfile(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()

	base := veyronv1alpha1.VeyronVMSpec{Running: boolPtr(true)}
	out, err := r.ResolveSpec(ctx, "windows-2022", "prod", base, BlueprintOverrides{})
	if err != nil {
		t.Fatal(err)
	}
	if out.CPU.Cores != 4 {
		t.Fatalf("expected 4 cores from prod profile, got %d", out.CPU.Cores)
	}
	if out.Memory.Size != "8Gi" {
		t.Fatalf("expected 8Gi memory, got %s", out.Memory.Size)
	}
	if !out.EnableTPM {
		t.Fatal("expected TPM enabled for windows-2022")
	}
	if out.CloudInit == nil || out.CloudInit.Delivery != "configdrive" {
		t.Fatal("expected configdrive cloud-init")
	}
	if len(out.Disks) < 2 {
		t.Fatal("expected root disk + virtio drivers cdrom")
	}
}

func TestBlueprintOverrideCPU(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()
	cpu := uint32(8)
	out, err := r.ResolveSpec(ctx, "ubuntu-22.04", "", veyronv1alpha1.VeyronVMSpec{}, BlueprintOverrides{CPU: &cpu})
	if err != nil {
		t.Fatal(err)
	}
	if out.CPU.Cores != 8 {
		t.Fatalf("expected override 8 cores, got %d", out.CPU.Cores)
	}
}

func TestProfileGrantsGpusButVmWins(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()
	embeddedProfiles["gpu-test"] = veyronv1alpha1.VMProfileSpec{
		Cores:  8,
		Memory: "16Gi",
		GPUs: []veyronv1alpha1.GPUSpec{
			{Name: "gpu0", DeviceName: "nvidia.com/gpu"},
		},
	}
	defer delete(embeddedProfiles, "gpu-test")

	out, err := r.ResolveSpec(ctx, "", "gpu-test", veyronv1alpha1.VeyronVMSpec{}, BlueprintOverrides{})
	if err != nil {
		t.Fatal(err)
	}
	if len(out.GPUs) != 1 || out.GPUs[0].DeviceName != "nvidia.com/gpu" {
		t.Fatalf("expected profile GPU to be granted, got %#v", out.GPUs)
	}

	// A VM that already declares its own GPUs keeps them.
	base := veyronv1alpha1.VeyronVMSpec{
		GPUs: []veyronv1alpha1.GPUSpec{{Name: "mine", DeviceName: "nvidia.com/GRID_T4-2Q"}},
	}
	out, err = r.ResolveSpec(ctx, "", "gpu-test", base, BlueprintOverrides{})
	if err != nil {
		t.Fatal(err)
	}
	if len(out.GPUs) != 1 || out.GPUs[0].Name != "mine" {
		t.Fatalf("expected explicit VM GPUs to win over profile, got %#v", out.GPUs)
	}
}

// A VM's explicit GPUs must survive the template-default merge in
// ResolveSpec (mergeVMSpec previously had no case for GPUs/HostDevices, so
// they silently reverted to the template's, which is nil for virtually
// every template).
func TestTemplateMergePreservesGpus(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()
	base := veyronv1alpha1.VeyronVMSpec{
		GPUs: []veyronv1alpha1.GPUSpec{{Name: "gpu0", DeviceName: "nvidia.com/gpu"}},
	}
	out, err := r.ResolveSpec(ctx, "ubuntu-22.04", "", base, BlueprintOverrides{})
	if err != nil {
		t.Fatal(err)
	}
	if len(out.GPUs) != 1 || out.GPUs[0].DeviceName != "nvidia.com/gpu" {
		t.Fatalf("expected VM's explicit GPUs to survive template merge, got %#v", out.GPUs)
	}
}

// A VeyronBlueprint per-VM override's GPUs/HostDevices must win last, per
// the documented "override wins last" contract.
func TestBlueprintOverrideGpus(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()
	out, err := r.ResolveSpec(ctx, "ubuntu-22.04", "", veyronv1alpha1.VeyronVMSpec{}, BlueprintOverrides{
		Override: &veyronv1alpha1.VeyronVMSpec{
			GPUs: []veyronv1alpha1.GPUSpec{{Name: "gpu0", DeviceName: "nvidia.com/gpu"}},
		},
	})
	if err != nil {
		t.Fatal(err)
	}
	if len(out.GPUs) != 1 || out.GPUs[0].DeviceName != "nvidia.com/gpu" {
		t.Fatalf("expected blueprint override GPUs to be applied, got %#v", out.GPUs)
	}
}

func TestSpecHashStable(t *testing.T) {
	spec := ubuntu2204Default()
	h1, err := SpecHash(spec)
	if err != nil {
		t.Fatal(err)
	}
	h2, err := SpecHash(spec)
	if err != nil {
		t.Fatal(err)
	}
	if h1 != h2 || len(h1) != 64 {
		t.Fatalf("unexpected hash: %s", h1)
	}
}
