// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package network

import (
	"context"
	"fmt"
	"strings"
	"testing"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
	"sigs.k8s.io/controller-runtime/pkg/client/interceptor"
)

func testScheme() *runtime.Scheme {
	scheme := runtime.NewScheme()
	_ = clientgoscheme.AddToScheme(scheme)
	return scheme
}

// Regression test for commit 42dd9bdf: both deletes returning NotFound
// (the common case — no policy was ever created, or a previous reconcile
// already removed it) must be treated as success, not an error.
func TestRemoveVmInternetEgressBothNotFoundIsNil(t *testing.T) {
	fakeClient := fake.NewClientBuilder().WithScheme(testScheme()).Build()

	if err := RemoveVmInternetEgress(context.Background(), fakeClient, "default", "vm1"); err != nil {
		t.Fatalf("expected nil error when both policies are already absent, got: %v", err)
	}
}

// Regression test for commit 42dd9bdf: a real (non-NotFound) delete error
// must be propagated to the caller instead of being silently discarded.
func TestRemoveVmInternetEgressPropagatesRealDeleteError(t *testing.T) {
	injectedErr := apierrors.NewForbidden(
		schema.GroupResource{Group: "cilium.io", Resource: "ciliumnetworkpolicies"},
		InternetPolicyName("vm1"),
		fmt.Errorf("RBAC: delete forbidden"),
	)

	fakeClient := fake.NewClientBuilder().
		WithScheme(testScheme()).
		WithInterceptorFuncs(interceptor.Funcs{
			Delete: func(ctx context.Context, c client.WithWatch, obj client.Object, opts ...client.DeleteOption) error {
				if u, ok := obj.(*unstructured.Unstructured); ok && u.GetKind() == "CiliumNetworkPolicy" {
					return injectedErr
				}
				return c.Delete(ctx, obj, opts...)
			},
		}).
		Build()

	err := RemoveVmInternetEgress(context.Background(), fakeClient, "default", "vm1")
	if err == nil {
		t.Fatal("expected the real Cilium delete error to be propagated, got nil")
	}
	if !strings.Contains(err.Error(), "delete cilium network policy") {
		t.Fatalf("expected error to be attributed to the cilium policy delete, got: %v", err)
	}
	if !strings.Contains(err.Error(), "RBAC: delete forbidden") {
		t.Fatalf("expected the real underlying error message to be preserved, got: %v", err)
	}
}

// Both deletes failing must report both underlying errors, not just one.
func TestRemoveVmInternetEgressBothDeletesFailReportsBoth(t *testing.T) {
	cnpErr := fmt.Errorf("cilium: forbidden")
	npErr := fmt.Errorf("networkpolicy: forbidden")

	fakeClient := fake.NewClientBuilder().
		WithScheme(testScheme()).
		WithInterceptorFuncs(interceptor.Funcs{
			Delete: func(ctx context.Context, c client.WithWatch, obj client.Object, opts ...client.DeleteOption) error {
				u, ok := obj.(*unstructured.Unstructured)
				if !ok {
					return c.Delete(ctx, obj, opts...)
				}
				switch u.GetKind() {
				case "CiliumNetworkPolicy":
					return cnpErr
				case "NetworkPolicy":
					return npErr
				default:
					return c.Delete(ctx, obj, opts...)
				}
			},
		}).
		Build()

	err := RemoveVmInternetEgress(context.Background(), fakeClient, "default", "vm1")
	if err == nil {
		t.Fatal("expected an error when both deletes fail")
	}
	if !strings.Contains(err.Error(), "cilium: forbidden") || !strings.Contains(err.Error(), "networkpolicy: forbidden") {
		t.Fatalf("expected combined error to mention both failures, got: %v", err)
	}
}
