// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"
	"testing"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
)

func TestVeyronActionReconcileStartVMSucceeds(t *testing.T) {
	ctx := context.Background()

	target := &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{Name: "vm1", Namespace: "default"},
		Spec:       minimalVMSpec(),
	}
	stopped := false
	target.Spec.Running = &stopped

	action := &veyronv1alpha1.VeyronAction{
		ObjectMeta: metav1.ObjectMeta{Name: "start-vm1", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronActionSpec{
			ActionType: "StartVM",
			VMRef:      "vm1",
			Approved:   true,
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(target, action).Build()
	r := &VeyronActionReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: action.Name, Namespace: action.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var gotAction veyronv1alpha1.VeyronAction
	if err := fakeClient.Get(ctx, req.NamespacedName, &gotAction); err != nil {
		t.Fatalf("get VeyronAction: %v", err)
	}
	if gotAction.Status.Phase != veyronv1alpha1.ActionPhaseCompleted {
		t.Fatalf("expected phase Completed, got %s (message: %s)", gotAction.Status.Phase, gotAction.Status.Message)
	}

	var gotVM veyronv1alpha1.VeyronVM
	if err := fakeClient.Get(ctx, types.NamespacedName{Name: "vm1", Namespace: "default"}, &gotVM); err != nil {
		t.Fatalf("get target VeyronVM: %v", err)
	}
	if gotVM.Spec.Running == nil || !*gotVM.Spec.Running {
		t.Fatal("expected target VM's spec.running to be set to true")
	}
}

func TestVeyronActionReconcileFailsWhenTargetVMMissing(t *testing.T) {
	ctx := context.Background()

	action := &veyronv1alpha1.VeyronAction{
		ObjectMeta: metav1.ObjectMeta{Name: "start-missing", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronActionSpec{
			ActionType: "StartVM",
			VMRef:      "does-not-exist",
			Approved:   true,
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(action).Build()
	r := &VeyronActionReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: action.Name, Namespace: action.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronAction
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronAction: %v", err)
	}
	if got.Status.Phase != veyronv1alpha1.ActionPhaseFailed {
		t.Fatalf("expected phase Failed, got %s", got.Status.Phase)
	}
}
