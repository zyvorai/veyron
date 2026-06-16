// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"testing"

	vmroguev1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
)

func TestResolveWindows2022WithProdProfile(t *testing.T) {
	r := &Resolver{}
	ctx := t.Context()

	base := vmroguev1alpha1.VMRogueVMSpec{Running: boolPtr(true)}
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
	out, err := r.ResolveSpec(ctx, "ubuntu-22.04", "", vmroguev1alpha1.VMRogueVMSpec{}, BlueprintOverrides{CPU: &cpu})
	if err != nil {
		t.Fatal(err)
	}
	if out.CPU.Cores != 8 {
		t.Fatalf("expected override 8 cores, got %d", out.CPU.Cores)
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
