// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"
	"fmt"
	"testing"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/interceptor"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

func testBlueprintWithChild(bpName string) (*veyronv1alpha1.VeyronBlueprint, *veyronv1alpha1.VeyronVM) {
	bp := &veyronv1alpha1.VeyronBlueprint{
		ObjectMeta: metav1.ObjectMeta{
			Name:       bpName,
			Namespace:  "default",
			Finalizers: []string{blueprintFinalizer},
		},
		Spec: veyronv1alpha1.VeyronBlueprintSpec{
			VMs: []veyronv1alpha1.BlueprintVMSpec{{Name: "web"}},
		},
	}
	crName := fmt.Sprintf("%s-%s", bpName, "web")
	vm := &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{Name: crName, Namespace: "default"},
		Spec:       minimalVMSpec(),
	}
	return bp, vm
}

// Regression test for commit 42dd9bdf: a real (non-NotFound) delete error on
// a child VeyronVM during blueprint teardown must NOT be swallowed — the
// finalizer must stay in place so the blueprint requeues instead of
// orphaning the child VM.
func TestVeyronBlueprintHandleDeletionKeepsFinalizerOnChildDeleteFailure(t *testing.T) {
	ctx := context.Background()
	bp, vm := testBlueprintWithChild("web-stack")

	injectedErr := apierrors.NewForbidden(
		schema.GroupResource{Group: "veyron.io", Resource: "veyronvms"},
		vm.Name,
		fmt.Errorf("RBAC: delete forbidden"),
	)

	fakeClient := newFakeClientBuilder().
		WithObjects(bp, vm).
		WithInterceptorFuncs(interceptor.Funcs{
			Delete: func(ctx context.Context, c client.WithWatch, obj client.Object, opts ...client.DeleteOption) error {
				if childVM, ok := obj.(*veyronv1alpha1.VeyronVM); ok && childVM.Name == vm.Name {
					return injectedErr
				}
				return c.Delete(ctx, obj, opts...)
			},
		}).
		Build()

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: bp.Name, Namespace: bp.Namespace}}

	// Trigger deletion: finalizer is present, so this sets DeletionTimestamp
	// instead of actually removing the object.
	if err := fakeClient.Delete(ctx, bp); err != nil {
		t.Fatalf("failed to mark blueprint for deletion: %v", err)
	}

	r := &VeyronBlueprintReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}
	if _, err := r.Reconcile(ctx, req); err == nil {
		t.Fatal("expected Reconcile to return the child delete error")
	}

	var got veyronv1alpha1.VeyronBlueprint
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("expected blueprint to still exist (finalizer kept it around): %v", err)
	}
	found := false
	for _, f := range got.Finalizers {
		if f == blueprintFinalizer {
			found = true
		}
	}
	if !found {
		t.Fatal("expected blueprint finalizer to remain after a failed child delete")
	}

	var childStillExists veyronv1alpha1.VeyronVM
	if err := fakeClient.Get(ctx, types.NamespacedName{Name: vm.Name, Namespace: vm.Namespace}, &childStillExists); err != nil {
		t.Fatalf("expected child VeyronVM to still exist (delete was rejected): %v", err)
	}
}

// Once all children are confirmed gone, the finalizer must be removed and
// the blueprint allowed to actually delete.
func TestVeyronBlueprintHandleDeletionRemovesFinalizerWhenChildrenGone(t *testing.T) {
	ctx := context.Background()
	bp, _ := testBlueprintWithChild("web-stack")
	// Note: the child VeyronVM is intentionally NOT added to the fake client,
	// simulating it already having been deleted.

	fakeClient := newFakeClientBuilder().
		WithObjects(bp).
		Build()

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: bp.Name, Namespace: bp.Namespace}}

	if err := fakeClient.Delete(ctx, bp); err != nil {
		t.Fatalf("failed to mark blueprint for deletion: %v", err)
	}

	r := &VeyronBlueprintReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronBlueprint
	err := fakeClient.Get(ctx, req.NamespacedName, &got)
	if !apierrors.IsNotFound(err) {
		t.Fatalf("expected blueprint to be fully deleted once finalizer was removed, got err=%v obj=%+v", err, got)
	}
}
