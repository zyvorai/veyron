// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package controller

import (
	"context"
	"strings"
	"testing"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

// Regression test for commit 42dd9bdf: an invalid spec.selector must set an
// Evaluated=False condition with the real selector error instead of being
// logged and reconciled away with no visible trace.
func TestVeyronPolicyReconcileInvalidSelectorSetsCondition(t *testing.T) {
	ctx := context.Background()

	policy := &veyronv1alpha1.VeyronPolicy{
		ObjectMeta: metav1.ObjectMeta{Name: "no-tpm", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronPolicySpec{
			Enabled: true,
			Selector: &metav1.LabelSelector{
				MatchExpressions: []metav1.LabelSelectorRequirement{
					{Key: "tier", Operator: "NotAValidOperator", Values: []string{"x"}},
				},
			},
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(policy).Build()
	r := &VeyronPolicyReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: policy.Name, Namespace: policy.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronPolicy
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronPolicy: %v", err)
	}

	var evaluated *metav1.Condition
	for i := range got.Status.Conditions {
		if got.Status.Conditions[i].Type == "Evaluated" {
			evaluated = &got.Status.Conditions[i]
		}
	}
	if evaluated == nil {
		t.Fatal("expected an Evaluated condition to be set for an invalid selector")
	}
	if evaluated.Status != metav1.ConditionFalse {
		t.Fatalf("expected Evaluated=False, got %s", evaluated.Status)
	}
	if evaluated.Reason != "InvalidSelector" {
		t.Fatalf("expected reason InvalidSelector, got %s", evaluated.Reason)
	}
	if !strings.Contains(evaluated.Message, "invalid spec.selector") {
		t.Fatalf("expected message to explain the selector error, got: %q", evaluated.Message)
	}
	if got.Status.LastEvaluated == nil {
		t.Fatal("expected LastEvaluated to be set even on the invalid-selector path")
	}
}

func TestVeyronPolicyReconcileEvaluatesMatchingVMs(t *testing.T) {
	ctx := context.Background()

	compliantVM := &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{Name: "vm-tpm", Namespace: "default", Labels: map[string]string{"tier": "secure"}},
		Spec:       minimalVMSpec(),
	}
	compliantVM.Spec.EnableTPM = true

	policy := &veyronv1alpha1.VeyronPolicy{
		ObjectMeta: metav1.ObjectMeta{Name: "require-tpm", Namespace: "default"},
		Spec: veyronv1alpha1.VeyronPolicySpec{
			Enabled:  true,
			Selector: &metav1.LabelSelector{MatchLabels: map[string]string{"tier": "secure"}},
			Rules: []veyronv1alpha1.PolicyRuleSpec{
				{Name: "tpm-required", Condition: "spec.enableTpm == true", Message: "TPM must be enabled"},
			},
		},
	}

	fakeClient := newFakeClientBuilder().WithObjects(policy, compliantVM).Build()
	r := &VeyronPolicyReconciler{Client: fakeClient, Recorder: record.NewFakeRecorder(10)}

	req := ctrl.Request{NamespacedName: types.NamespacedName{Name: policy.Name, Namespace: policy.Namespace}}
	if _, err := r.Reconcile(ctx, req); err != nil {
		t.Fatalf("Reconcile returned unexpected error: %v", err)
	}

	var got veyronv1alpha1.VeyronPolicy
	if err := fakeClient.Get(ctx, req.NamespacedName, &got); err != nil {
		t.Fatalf("get VeyronPolicy: %v", err)
	}
	if got.Status.MatchingVMs != 1 || got.Status.CompliantVMs != 1 || got.Status.ViolatingVMs != 0 {
		t.Fatalf("expected 1 matching/compliant VM and 0 violations, got matching=%d compliant=%d violating=%d",
			got.Status.MatchingVMs, got.Status.CompliantVMs, got.Status.ViolatingVMs)
	}
}
