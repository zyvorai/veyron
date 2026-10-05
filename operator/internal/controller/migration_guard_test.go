// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package controller

import (
	"os"
	"testing"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

func runningVMI(extra map[string]interface{}) *unstructured.Unstructured {
	spec := map[string]interface{}{
		"domain": map[string]interface{}{
			"devices": map[string]interface{}{},
		},
	}
	for k, v := range extra {
		spec["domain"].(map[string]interface{})["devices"].(map[string]interface{})[k] = v
	}
	return &unstructured.Unstructured{Object: map[string]interface{}{
		"spec": spec,
		"status": map[string]interface{}{
			"phase": "Running",
		},
	}}
}

func TestEvaluateMigrationEligibilityCleanRunningVM(t *testing.T) {
	vmi := runningVMI(nil)
	if err := evaluateMigrationEligibility(vmi); err != nil {
		t.Fatalf("expected clean running VM to be eligible, got: %v", err)
	}
}

func TestEvaluateMigrationEligibilityBlocksHostDevices(t *testing.T) {
	vmi := runningVMI(map[string]interface{}{
		"hostDevices": []interface{}{
			map[string]interface{}{"name": "hd0", "deviceName": "example.com/dev"},
		},
	})
	err := evaluateMigrationEligibility(vmi)
	if err == nil {
		t.Fatal("expected host devices to block migration")
	}
}

func TestEvaluateMigrationEligibilityBlocksPassthroughGpu(t *testing.T) {
	vmi := runningVMI(map[string]interface{}{
		"gpus": []interface{}{
			map[string]interface{}{"name": "gpu0", "deviceName": "nvidia.com/A100"},
		},
	})
	err := evaluateMigrationEligibility(vmi)
	if err == nil {
		t.Fatal("expected passthrough GPU (uppercase model name, not a vGPU profile) to block migration")
	}
}

func TestEvaluateMigrationEligibilityVgpuBlockedWithoutClusterAttestation(t *testing.T) {
	os.Unsetenv("VEYRON_VGPU_LIVE_MIGRATION")
	vmi := runningVMI(map[string]interface{}{
		"gpus": []interface{}{
			map[string]interface{}{"name": "gpu0", "deviceName": "nvidia.com/GRID_T4-2Q"},
		},
	})
	err := evaluateMigrationEligibility(vmi)
	if err == nil {
		t.Fatal("expected vGPU migration to be blocked without VEYRON_VGPU_LIVE_MIGRATION")
	}
}

func TestEvaluateMigrationEligibilityVgpuAllowedWithClusterAttestation(t *testing.T) {
	t.Setenv("VEYRON_VGPU_LIVE_MIGRATION", "1")
	vmi := runningVMI(map[string]interface{}{
		"gpus": []interface{}{
			map[string]interface{}{"name": "gpu0", "deviceName": "nvidia.com/GRID_T4-2Q"},
		},
	})
	if err := evaluateMigrationEligibility(vmi); err != nil {
		t.Fatalf("expected vGPU migration to be allowed with attestation, got: %v", err)
	}
}

func TestEvaluateMigrationEligibilityBlocksNonRunningVMI(t *testing.T) {
	vmi := &unstructured.Unstructured{Object: map[string]interface{}{
		"spec":   map[string]interface{}{},
		"status": map[string]interface{}{"phase": "Scheduling"},
	}}
	err := evaluateMigrationEligibility(vmi)
	if err == nil {
		t.Fatal("expected non-Running VMI to block migration")
	}
}

func TestEvaluateMigrationEligibilitySurfacesKubeVirtCondition(t *testing.T) {
	vmi := &unstructured.Unstructured{Object: map[string]interface{}{
		"spec": map[string]interface{}{},
		"status": map[string]interface{}{
			"phase": "Running",
			"conditions": []interface{}{
				map[string]interface{}{
					"type":    "LiveMigratable",
					"status":  "False",
					"reason":  "DisksNotLiveMigratable",
					"message": "cannot migrate VMI: PVC rootdisk is not shared",
				},
			},
		},
	}}
	err := evaluateMigrationEligibility(vmi)
	if err == nil {
		t.Fatal("expected KubeVirt LiveMigratable=False condition to block migration")
	}
}

func TestIsVgpuDeviceName(t *testing.T) {
	cases := map[string]bool{
		"nvidia.com/gpu":         false,
		"nvidia.com/A100":        false,
		"nvidia.com/H100":        false,
		"nvidia.com/GRID_T4-2Q":  true,
		"nvidia.com/A100D-40C":   true,
		"nvidia.com/mig-3g.20gb": false,
	}
	for name, want := range cases {
		if got := isVgpuDeviceName(name); got != want {
			t.Errorf("isVgpuDeviceName(%q) = %v, want %v", name, got, want)
		}
	}
}
