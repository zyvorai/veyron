// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"
	"fmt"
	"strings"
	"testing"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/interceptor"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

func kubevirtVMGVK() schema.GroupVersionKind {
	return schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachine"}
}

func testVeyronVM(name string) *veyronv1alpha1.VeyronVM {
	return &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{
			Name:       name,
			Namespace:  "default",
			Finalizers: []string{vmFinalizer},
		},
		Spec: minimalVMSpec(),
	}
}

func TestVeyronVMReconcileCreatesKubeVirtVM(t *testing.T) {
	ctx := context.Background()
	vm := testVeyronVM("test-vm")

	fakeClient := newFakeClientBuilder().
		WithObjects(vm).
		Build()

	r := &VeyronVMReconciler{
		Client:   fakeClient,
		Recorder: record.NewFakeRecorder(10),
	}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: vm.Name, Namespace: vm.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	kvVM := &unstructured.Unstructured{}
	kvVM.SetGroupVersionKind(kubevirtVMGVK())
	if err := fakeClient.Get(ctx, req.NamespacedName, kvVM); err != nil {
		t.Fatalf("expected KubeVirt VirtualMachine to be created, get failed: %v", err)
	}

	var got veyronv1alpha1.VeyronVM
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronVM: %v", err)
	}
	if got.Status.Phase != veyronv1alpha1.VMPhaseStopped {
		t.Fatalf("expected phase Stopped (no VMI present), got %s", got.Status.Phase)
	}
}

// Regression test for commit 42dd9bdf: a KubeVirt Create failure (quota,
// admission webhook, etc.) must surface as a Ready=False condition with the
// real error, not just a log line and a silent requeue.
func TestVeyronVMReconcileCreateFailureSetsReadyFalseCondition(t *testing.T) {
	ctx := context.Background()
	vm := testVeyronVM("test-vm")

	injectedErr := apierrors.NewForbidden(
		schema.GroupResource{Group: "kubevirt.io", Resource: "virtualmachines"},
		vm.Name,
		fmt.Errorf("exceeded quota: requests.memory"),
	)

	fakeClient := newFakeClientBuilder().
		WithObjects(vm).
		WithInterceptorFuncs(interceptor.Funcs{
			Create: func(ctx context.Context, c client.WithWatch, obj client.Object, opts ...client.CreateOption) error {
				if u, ok := obj.(*unstructured.Unstructured); ok && u.GetKind() == "VirtualMachine" {
					return injectedErr
				}
				return c.Create(ctx, obj, opts...)
			},
		}).
		Build()

	r := &VeyronVMReconciler{
		Client:   fakeClient,
		Recorder: record.NewFakeRecorder(10),
	}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: vm.Name, Namespace: vm.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error (failure path returns nil to requeue): %v", err)
	}

	var got veyronv1alpha1.VeyronVM
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronVM: %v", err)
	}

	var readyCond *metav1.Condition
	for i := range got.Status.Conditions {
		if got.Status.Conditions[i].Type == "Ready" {
			readyCond = &got.Status.Conditions[i]
			break
		}
	}
	if readyCond == nil {
		t.Fatal("expected a Ready condition to be set after Create failure")
	}
	if readyCond.Status != metav1.ConditionFalse {
		t.Fatalf("expected Ready=False, got %s", readyCond.Status)
	}
	if !strings.Contains(readyCond.Message, "exceeded quota") {
		t.Fatalf("expected condition message to contain the real error, got: %q", readyCond.Message)
	}
	if got.Status.Phase != veyronv1alpha1.VMPhaseFailed {
		t.Fatalf("expected status.phase Failed, got %s", got.Status.Phase)
	}
}
