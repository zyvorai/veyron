// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"

	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
	"github.com/zyvorai/veyron/operator/internal/eventbus"
)

// VeyronInsightReconciler reconciles a VeyronInsight object.
type VeyronInsightReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=veyron.io,resources=veyroninsights,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=veyroninsights/status,verbs=get;update;patch

func (r *VeyronInsightReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var insight veyronv1alpha1.VeyronInsight
	if err := r.Get(ctx, req.NamespacedName, &insight); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	// Set initial state to "New" if not set
	if insight.Status.State == "" {
		insight.Status.State = veyronv1alpha1.InsightStateNew

		now := metav1.Now()
		setCondition(&insight.Status.Conditions, metav1.Condition{
			Type:               "Recorded",
			Status:             metav1.ConditionTrue,
			Reason:             "InsightCreated",
			Message:            "Insight has been recorded",
			LastTransitionTime: now,
		})

		if err := r.Status().Update(ctx, &insight); err != nil {
			logger.Error(err, "failed to update insight status")
			return ctrl.Result{}, err
		}

		// Publish event
		if r.EventBus != nil {
			event, err := eventbus.NewEvent(
				eventbus.SubjectInsightCreated,
				"operator",
				eventbus.SubjectInsightCreated,
				map[string]string{
					"name":        insight.Name,
					"namespace":   insight.Namespace,
					"insightType": insight.Spec.InsightType,
					"severity":    insight.Spec.Severity,
					"vmRef":       insight.Spec.VMRef,
					"title":       insight.Spec.Title,
				},
			)
			if err == nil {
				_ = r.EventBus.Publish(event)
			}
		}
	}

	// Check linked action status
	if insight.Status.ActionRef != "" {
		var action veyronv1alpha1.VeyronAction
		if err := r.Get(ctx, client.ObjectKey{
			Namespace: insight.Namespace,
			Name:      insight.Status.ActionRef,
		}, &action); err == nil {
			if action.Status.Phase == veyronv1alpha1.ActionPhaseCompleted {
				insight.Status.State = veyronv1alpha1.InsightStateResolved
				_ = r.Status().Update(ctx, &insight)
			}
		}
	}

	return ctrl.Result{}, nil
}

func (r *VeyronInsightReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VeyronInsight{}).
		Complete(r)
}
