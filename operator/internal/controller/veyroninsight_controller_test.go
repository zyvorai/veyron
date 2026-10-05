// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package controller

import (
	"context"
	"testing"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

func TestVeyronInsightReconcileSetsInitialStateAndCondition(t *testing.T) {
	ctx := context.Background()

	insight := &veyronv1alpha1.VeyronInsight{
		ObjectMeta: metav1.ObjectMeta{Name: "drift-vm1", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronInsightSpec{
			InsightType: "Drift",
			Severity:    "High",
			VMRef:       "vm1",
			Title:       "Configuration drift",
			Description: "spec differs from live object",
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(insight).Build()
	r := &VeyronInsightReconciler{Client: fakeClient}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: insight.Name, Namespace: insight.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronInsight
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronInsight: %v", err)
	}
	if got.Status.State != veyronv1alpha1.InsightStateNew {
		t.Fatalf("expected initial state New, got %s", got.Status.State)
	}

	var recorded *metav1.Condition
	for i := range got.Status.Conditions {
		if got.Status.Conditions[i].Type == "Recorded" {
			recorded = &got.Status.Conditions[i]
		}
	}
	if recorded == nil || recorded.Status != metav1.ConditionTrue {
		t.Fatalf("expected a Recorded=True condition, got %+v", got.Status.Conditions)
	}
}

func TestVeyronInsightReconcileResolvesWhenLinkedActionCompletes(t *testing.T) {
	ctx := context.Background()

	action := &veyronv1alpha1.VeyronAction{
		ObjectMeta: metav1.ObjectMeta{Name: "remediate-1", Namespace: "default"},
		Spec:       veyronv1alpha1.VeyronActionSpec{ActionType: "StartVM", VMRef: "vm1"},
		Status:     veyronv1alpha1.VeyronActionStatus{Phase: veyronv1alpha1.ActionPhaseCompleted},
	}
	insight := &veyronv1alpha1.VeyronInsight{
		ObjectMeta: metav1.ObjectMeta{Name: "drift-vm1", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronInsightSpec{
			InsightType: "Drift",
			Severity:    "High",
			VMRef:       "vm1",
			Title:       "Configuration drift",
			Description: "spec differs from live object",
		},
		Status: veyronv1alpha1.VeyronInsightStatus{
			State:     veyronv1alpha1.InsightStateAcknowledged,
			ActionRef: action.Name,
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(insight, action).Build()
	r := &VeyronInsightReconciler{Client: fakeClient}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: insight.Name, Namespace: insight.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronInsight
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronInsight: %v", err)
	}
	if got.Status.State != veyronv1alpha1.InsightStateResolved {
		t.Fatalf("expected state Resolved once linked action completed, got %s", got.Status.State)
	}
}
