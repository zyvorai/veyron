// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package network

import (
	"context"
	"fmt"
	"strings"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/api/meta"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

const (
	policyPrefix   = "veyron-net-"
	labelManaged   = "veyron.io/managed-by"
	labelVMName    = "veyron.io/vm-name"
	labelEgress    = "veyron.io/vm-egress"
	managedByValue = "veyron"
)

var (
	ciliumCNPGVK = schema.GroupVersionKind{
		Group:   "cilium.io",
		Version: "v2",
		Kind:    "CiliumNetworkPolicy",
	}
	networkPolicyGVK = schema.GroupVersionKind{
		Group:   "networking.k8s.io",
		Version: "v1",
		Kind:    "NetworkPolicy",
	}
)

// InternetPolicyName returns the stable policy object name for a VM.
func InternetPolicyName(vmName string) string {
	var slug strings.Builder
	for _, r := range vmName {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '-' {
			slug.WriteRune(r)
		} else {
			slug.WriteRune('-')
		}
	}
	s := strings.Trim(slug.String(), "-")
	if s == "" {
		s = "vm"
	}
	maxBody := 63 - len(policyPrefix)
	if len(s) > maxBody {
		s = strings.TrimRight(s[:maxBody], "-")
		if s == "" {
			s = "vm"
		}
	}
	return policyPrefix + s
}

// AllowInternetEnabled returns whether the spec requests internet egress (default true).
func AllowInternetEnabled(spec *veyronv1alpha1.VeyronVMSpec) bool {
	if spec == nil || spec.AllowInternet == nil {
		return true
	}
	return *spec.AllowInternet
}

func policyLabels(vmName string) map[string]string {
	return map[string]string{
		labelManaged: managedByValue,
		labelVMName:  vmName,
		labelEgress:  "internet",
	}
}

func ciliumPolicy(namespace, vmName, polName string) *unstructured.Unstructured {
	obj := &unstructured.Unstructured{}
	obj.SetGroupVersionKind(ciliumCNPGVK)
	obj.SetName(polName)
	obj.SetNamespace(namespace)
	obj.SetLabels(policyLabels(vmName))
	_ = unstructured.SetNestedMap(obj.Object, map[string]interface{}{
		"matchLabels": map[string]interface{}{
			"kubevirt.io/vm": vmName,
		},
	}, "spec", "endpointSelector")
	_ = unstructured.SetNestedSlice(obj.Object, []interface{}{
		map[string]interface{}{
			"toEntities": []interface{}{"all"},
		},
	}, "spec", "egress")
	return obj
}

func kubernetesNetworkPolicy(namespace, vmName, polName string) *unstructured.Unstructured {
	obj := &unstructured.Unstructured{}
	obj.SetGroupVersionKind(networkPolicyGVK)
	obj.SetName(polName)
	obj.SetNamespace(namespace)
	obj.SetLabels(policyLabels(vmName))
	_ = unstructured.SetNestedMap(obj.Object, map[string]interface{}{
		"matchLabels": map[string]interface{}{
			"kubevirt.io/vm": vmName,
		},
	}, "spec", "podSelector")
	_ = unstructured.SetNestedStringSlice(obj.Object, []string{"Egress"}, "spec", "policyTypes")
	_ = unstructured.SetNestedSlice(obj.Object, []interface{}{
		map[string]interface{}{},
	}, "spec", "egress")
	return obj
}

// EnsureVmInternetEgress creates a CiliumNetworkPolicy when possible, else a NetworkPolicy.
func EnsureVmInternetEgress(ctx context.Context, c client.Client, namespace, vmName string) (backend string, err error) {
	polName := InternetPolicyName(vmName)
	cnp := ciliumPolicy(namespace, vmName, polName)
	ciliumErr := createOrUpdate(ctx, c, cnp)
	if ciliumErr == nil {
		return "cilium", nil
	}
	// Only fall back to a Kubernetes NetworkPolicy when the CiliumNetworkPolicy CRD
	// is genuinely absent. Falling back on ANY error (conflict, timeout, RBAC) would
	// leave BOTH a CNP and an NP for the same VM once Cilium recovers.
	if !meta.IsNoMatchError(ciliumErr) {
		return "", fmt.Errorf("internet egress policy (cilium): %w", ciliumErr)
	}
	np := kubernetesNetworkPolicy(namespace, vmName, polName)
	if err := createOrUpdate(ctx, c, np); err != nil {
		return "", fmt.Errorf("internet egress policy: cilium: %v; kubernetes: %w", ciliumErr, err)
	}
	return "kubernetes", nil
}

func createOrUpdate(ctx context.Context, c client.Client, desired *unstructured.Unstructured) error {
	existing := &unstructured.Unstructured{}
	existing.SetGroupVersionKind(desired.GroupVersionKind())
	key := client.ObjectKeyFromObject(desired)
	err := c.Get(ctx, key, existing)
	if apierrors.IsNotFound(err) {
		return c.Create(ctx, desired)
	}
	if err != nil {
		return err
	}
	desired.SetResourceVersion(existing.GetResourceVersion())
	return c.Update(ctx, desired)
}

// RemoveVmInternetEgress deletes Veyron-managed internet policies for the VM.
// Both deletes are attempted (best effort) even if one fails, but a real
// error (RBAC-forbidden, admission-webhook denial, etc. — anything other
// than NotFound) is propagated instead of being silently discarded, so
// callers that check the returned error can actually observe and act on it.
func RemoveVmInternetEgress(ctx context.Context, c client.Client, namespace, vmName string) error {
	polName := InternetPolicyName(vmName)
	cnp := &unstructured.Unstructured{}
	cnp.SetGroupVersionKind(ciliumCNPGVK)
	cnp.SetName(polName)
	cnp.SetNamespace(namespace)
	cnpErr := client.IgnoreNotFound(c.Delete(ctx, cnp))

	np := &unstructured.Unstructured{}
	np.SetGroupVersionKind(networkPolicyGVK)
	np.SetName(polName)
	np.SetNamespace(namespace)
	npErr := client.IgnoreNotFound(c.Delete(ctx, np))

	if cnpErr != nil && npErr != nil {
		return fmt.Errorf("delete cilium network policy: %v; delete network policy: %w", cnpErr, npErr)
	}
	if cnpErr != nil {
		return fmt.Errorf("delete cilium network policy: %w", cnpErr)
	}
	if npErr != nil {
		return fmt.Errorf("delete network policy: %w", npErr)
	}
	return nil
}
