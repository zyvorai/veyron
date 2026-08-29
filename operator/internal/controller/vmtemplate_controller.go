// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

// VMTemplateReconciler validates cluster-scoped VMTemplate catalog entries.
type VMTemplateReconciler struct {
	client.Client
}

// +kubebuilder:rbac:groups=veyron.io,resources=vmtemplates,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=vmtemplates/status,verbs=get;update;patch

func (r *VMTemplateReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var tpl veyronv1alpha1.VMTemplate
	if err := r.Get(ctx, req.NamespacedName, &tpl); err != nil {
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	tpl.Status.ObservedGeneration = tpl.Generation
	now := metav1.Now()
	setCondition(&tpl.Status.Conditions, metav1.Condition{
		Type:               "Ready",
		Status:             metav1.ConditionTrue,
		Reason:             "Validated",
		Message:            "Template catalog entry is valid",
		LastTransitionTime: now,
	})

	if err := r.Status().Update(ctx, &tpl); err != nil {
		logger.Error(err, "failed to update VMTemplate status")
		return ctrl.Result{}, err
	}
	return ctrl.Result{RequeueAfter: 0}, nil
}

func (r *VMTemplateReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VMTemplate{}).
		Complete(r)
}

// VMProfileReconciler validates cluster-scoped VMProfile catalog entries.
type VMProfileReconciler struct {
	client.Client
}

// +kubebuilder:rbac:groups=veyron.io,resources=vmprofiles,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=vmprofiles/status,verbs=get;update;patch

func (r *VMProfileReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var prof veyronv1alpha1.VMProfile
	if err := r.Get(ctx, req.NamespacedName, &prof); err != nil {
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	prof.Status.ObservedGeneration = prof.Generation
	now := metav1.Now()
	setCondition(&prof.Status.Conditions, metav1.Condition{
		Type:               "Ready",
		Status:             metav1.ConditionTrue,
		Reason:             "Validated",
		Message:            "Profile catalog entry is valid",
		LastTransitionTime: now,
	})

	if err := r.Status().Update(ctx, &prof); err != nil {
		logger.Error(err, "failed to update VMProfile status")
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

func (r *VMProfileReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VMProfile{}).
		Complete(r)
}
