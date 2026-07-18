// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Package catalog resolves VMTemplate and VMProfile into VeyronVMSpec.
package catalog

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
)

// BlueprintOverrides carries per-VM fields from a VeyronBlueprint entry.
type BlueprintOverrides struct {
	CPU      *uint32
	Memory   *string
	DiskSize *string
	Override *veyronv1alpha1.VeyronVMSpec
}

// Resolver loads templates and profiles from cluster CRDs with embedded fallback.
type Resolver struct {
	Client client.Client
}

// ResolveSpec merges template, profile, blueprint overrides, and explicit override.
func (r *Resolver) ResolveSpec(
	ctx context.Context,
	templateName, profileName string,
	base veyronv1alpha1.VeyronVMSpec,
	bp BlueprintOverrides,
) (veyronv1alpha1.VeyronVMSpec, error) {
	out := base

	if templateName != "" {
		tpl, err := r.GetTemplate(ctx, templateName)
		if err != nil {
			return out, err
		}
		out = mergeVMSpec(tpl.Default, out)
		out.Template = templateName
	}

	if profileName != "" {
		prof, err := r.GetProfile(ctx, profileName)
		if err != nil {
			return out, err
		}
		out.CPU.Cores = prof.Cores
		if prof.Sockets > 0 {
			out.CPU.Sockets = prof.Sockets
		}
		if prof.Threads > 0 {
			out.CPU.Threads = prof.Threads
		}
		out.Memory.Size = prof.Memory
		if prof.DiskSize != "" && len(out.Disks) > 0 {
			out.Disks[0].Size = prof.DiskSize
		}
		// Profile-granted GPUs; the VM's own explicit gpus win.
		if len(prof.GPUs) > 0 && len(out.GPUs) == 0 {
			out.GPUs = append([]veyronv1alpha1.GPUSpec(nil), prof.GPUs...)
		}
		out.Profile = profileName
	}

	if bp.CPU != nil {
		out.CPU.Cores = *bp.CPU
	}
	if bp.Memory != nil {
		out.Memory.Size = *bp.Memory
	}
	if bp.DiskSize != nil && len(out.Disks) > 0 {
		out.Disks[0].Size = *bp.DiskSize
	}

	if bp.Override != nil {
		out = mergeVMSpec(out, *bp.Override)
	}

	// Preserve instance fields from base
	if base.Running != nil {
		out.Running = base.Running
	}
	if base.Labels != nil {
		if out.Labels == nil {
			out.Labels = map[string]string{}
		}
		for k, v := range base.Labels {
			out.Labels[k] = v
		}
	}
	if base.Annotations != nil {
		if out.Annotations == nil {
			out.Annotations = map[string]string{}
		}
		for k, v := range base.Annotations {
			out.Annotations[k] = v
		}
	}
	if base.Windows != nil {
		out.Windows = base.Windows
	}
	if base.AllowInternet != nil {
		out.AllowInternet = base.AllowInternet
	}

	return out, nil
}

// GetTemplate returns a VMTemplate spec by name (cluster CRD or embedded).
func (r *Resolver) GetTemplate(ctx context.Context, name string) (*veyronv1alpha1.VMTemplateSpec, error) {
	if r.Client != nil {
		var tpl veyronv1alpha1.VMTemplate
		err := r.Client.Get(ctx, types.NamespacedName{Name: name}, &tpl)
		if err == nil {
			spec := tpl.Spec
			return &spec, nil
		}
		if !apierrors.IsNotFound(err) {
			return nil, err
		}
	}
	if embedded, ok := embeddedTemplates[name]; ok {
		return &embedded, nil
	}
	return nil, fmt.Errorf("template not found: %s", name)
}

// GetProfile returns a VMProfile spec by name (cluster CRD or embedded).
func (r *Resolver) GetProfile(ctx context.Context, name string) (*veyronv1alpha1.VMProfileSpec, error) {
	if r.Client != nil {
		var prof veyronv1alpha1.VMProfile
		err := r.Client.Get(ctx, types.NamespacedName{Name: name}, &prof)
		if err == nil {
			spec := prof.Spec
			return &spec, nil
		}
		if !apierrors.IsNotFound(err) {
			return nil, err
		}
	}
	if embedded, ok := embeddedProfiles[name]; ok {
		return &embedded, nil
	}
	return nil, fmt.Errorf("profile not found: %s", name)
}

// SpecHash returns a stable hash of the resolved VM spec for drift detection.
func SpecHash(spec veyronv1alpha1.VeyronVMSpec) (string, error) {
	b, err := json.Marshal(spec)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:]), nil
}

// mergeVMSpec merges overlay onto base; overlay wins for set fields.
func mergeVMSpec(base, overlay veyronv1alpha1.VeyronVMSpec) veyronv1alpha1.VeyronVMSpec {
	out := base

	if overlay.Template != "" {
		out.Template = overlay.Template
	}
	if overlay.Profile != "" {
		out.Profile = overlay.Profile
	}
	if overlay.CPU.Cores > 0 {
		out.CPU = overlay.CPU
	}
	if overlay.Memory.Size != "" {
		out.Memory = overlay.Memory
	}
	if len(overlay.Disks) > 0 {
		out.Disks = overlay.Disks
	}
	if len(overlay.Interfaces) > 0 {
		out.Interfaces = overlay.Interfaces
	}
	if overlay.CloudInit != nil {
		out.CloudInit = overlay.CloudInit
	}
	if overlay.Features != nil {
		out.Features = overlay.Features
	}
	if overlay.Firmware != nil {
		out.Firmware = overlay.Firmware
	}
	if overlay.Clock != nil {
		out.Clock = overlay.Clock
	}
	if overlay.EvictionStrategy != nil {
		out.EvictionStrategy = overlay.EvictionStrategy
	}
	if overlay.TerminationGracePeriod != nil {
		out.TerminationGracePeriod = overlay.TerminationGracePeriod
	}
	if overlay.EnableTPM {
		out.EnableTPM = true
	}
	if overlay.EnableRNG {
		out.EnableRNG = true
	}
	if overlay.MachineType != nil {
		out.MachineType = overlay.MachineType
	}
	if overlay.Windows != nil {
		out.Windows = overlay.Windows
	}
	return out
}
