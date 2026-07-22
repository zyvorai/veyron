// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strings"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// vgpuProfileSuffix mirrors classify_gpu_resource in src/kube/gpu_inventory.rs:
// an NVIDIA vGPU mdev profile name ends in a size + profile-class-letter
// suffix (-2Q, -4C, -1-5C), which is what actually distinguishes it from a
// whole-GPU passthrough resource that merely has an uppercase model name
// (nvidia.com/A100, H100, T4). Keep this in sync with the Rust regex.
var vgpuProfileSuffix = regexp.MustCompile(`-\d+(-\d+)?[QCBA]$`)

func isVgpuDeviceName(name string) bool {
	suffix := name
	if idx := strings.LastIndex(name, "/"); idx >= 0 {
		suffix = name[idx+1:]
	}
	return strings.HasPrefix(suffix, "GRID_") || vgpuProfileSuffix.MatchString(suffix)
}

func vgpuLiveMigrationEnabled() bool {
	v := strings.ToLower(strings.TrimSpace(os.Getenv("VEYRON_VGPU_LIVE_MIGRATION")))
	return v == "1" || v == "true" || v == "yes"
}

// evaluateMigrationEligibility is the operator-side counterpart to
// evaluate_eligibility in src/kube/migration_guard.rs — CLAUDE.md documents
// "every migrate entry point consults this guard first" for the Rust API,
// but VeyronAction{actionType: Migrate} is a fourth entry point, reconciled
// entirely in Go with no access to that crate, and previously created a
// VirtualMachineInstanceMigration unconditionally. A VM holding a passthrough
// GPU or host device can never live-migrate: the device state lives on the
// physical card. Pure function over the fetched VMI object — easy to unit
// test without a fake Kubernetes client.
func evaluateMigrationEligibility(vmi *unstructured.Unstructured) error {
	hostDevices, _, _ := unstructured.NestedSlice(vmi.Object, "spec", "domain", "devices", "hostDevices")
	if len(hostDevices) > 0 {
		return fmt.Errorf(
			"VM holds %d passthrough host device(s); live migration is impossible — "+
				"stop the VM, then start it with node placement (a passthrough device can only move with a stop/start 'cold move')",
			len(hostDevices),
		)
	}

	gpus, _, _ := unstructured.NestedSlice(vmi.Object, "spec", "domain", "devices", "gpus")
	if len(gpus) > 0 {
		allVgpu := true
		for _, g := range gpus {
			gm, ok := g.(map[string]interface{})
			if !ok {
				allVgpu = false
				continue
			}
			deviceName, _, _ := unstructured.NestedString(gm, "deviceName")
			if !isVgpuDeviceName(deviceName) {
				allVgpu = false
			}
		}
		if !allVgpu || !vgpuLiveMigrationEnabled() {
			return fmt.Errorf(
				"VM holds %d GPU device(s); a passthrough GPU's state lives on the physical card and can never live-migrate — "+
					"stop the VM, then start it with node placement (a passthrough device can only move with a stop/start 'cold move')",
				len(gpus),
			)
		}
	}

	phase, _, _ := unstructured.NestedString(vmi.Object, "status", "phase")
	if phase != "Running" {
		return fmt.Errorf("VM instance is in phase %q — only Running VMIs migrate", phase)
	}

	conditions, _, _ := unstructured.NestedSlice(vmi.Object, "status", "conditions")
	for _, c := range conditions {
		cm, ok := c.(map[string]interface{})
		if !ok {
			continue
		}
		t, _, _ := unstructured.NestedString(cm, "type")
		s, _, _ := unstructured.NestedString(cm, "status")
		if t == "LiveMigratable" && s == "False" {
			reason, _, _ := unstructured.NestedString(cm, "reason")
			message, _, _ := unstructured.NestedString(cm, "message")
			if message == "" {
				message = "KubeVirt reports this VMI is not live-migratable"
			}
			return fmt.Errorf("KubeVirt blocks migration (%s): %s", reason, message)
		}
	}

	return nil
}

// checkMigrationEligible fetches the target VMI and applies
// evaluateMigrationEligibility. A missing VMI (VM not running) is itself a
// blocker, not an operator error.
func (r *VeyronActionReconciler) checkMigrationEligible(ctx context.Context, namespace, vmName string) error {
	vmi := &unstructured.Unstructured{}
	vmi.SetGroupVersionKind(schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachineInstance"})
	if err := r.Get(ctx, client.ObjectKey{Namespace: namespace, Name: vmName}, vmi); err != nil {
		return fmt.Errorf("VM has no running instance to migrate: %w", err)
	}
	return evaluateMigrationEligibility(vmi)
}
