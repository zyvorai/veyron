// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"k8s.io/apimachinery/pkg/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
)

// newScheme returns a runtime.Scheme with the core Kubernetes types and the
// Veyron CRD types registered, matching main.go's manager scheme setup.
// KubeVirt types (VirtualMachine, VirtualMachineInstance) are handled as
// unstructured.Unstructured, which the fake client registers on demand.
func newScheme() *runtime.Scheme {
	scheme := runtime.NewScheme()
	_ = clientgoscheme.AddToScheme(scheme)
	_ = veyronv1alpha1.AddToScheme(scheme)
	return scheme
}

// newFakeClientBuilder returns a fake client builder pre-wired with the
// Veyron scheme and status-subresource registration for every Veyron CRD.
// Without WithStatusSubresource, the fake client's Status().Update() always
// returns NotFound for a type it doesn't know has a status subresource —
// even when the object genuinely exists — so every reconciler test that
// calls r.Status().Update() (all of them) needs this.
func newFakeClientBuilder() *fake.ClientBuilder {
	return fake.NewClientBuilder().
		WithScheme(newScheme()).
		WithStatusSubresource(
			&veyronv1alpha1.VeyronVM{},
			&veyronv1alpha1.VeyronBlueprint{},
			&veyronv1alpha1.VeyronPolicy{},
			&veyronv1alpha1.VeyronAction{},
			&veyronv1alpha1.VeyronInsight{},
			&veyronv1alpha1.VMTemplate{},
			&veyronv1alpha1.VMProfile{},
		)
}

// minimalVMSpec returns a VeyronVMSpec with every field the converter needs
// to build a KubeVirt VirtualMachine, and no template/profile reference so
// catalog.Resolver.ResolveSpec is a pure passthrough (no cluster fetch).
func minimalVMSpec() veyronv1alpha1.VeyronVMSpec {
	return veyronv1alpha1.VeyronVMSpec{
		CPU:    veyronv1alpha1.CPUSpec{Cores: 2, Sockets: 1, Threads: 1},
		Memory: veyronv1alpha1.MemorySpec{Size: "4Gi"},
		Disks: []veyronv1alpha1.DiskSpec{
			{
				Name:      "rootdisk",
				Size:      "10Gi",
				BootOrder: 1,
				Source:    veyronv1alpha1.DiskSource{Type: "blank"},
			},
		},
		Interfaces: []veyronv1alpha1.InterfaceSpec{
			{
				Name:        "default",
				Network:     "default",
				NetworkType: veyronv1alpha1.NetworkType{Type: "pod"},
			},
		},
	}
}
