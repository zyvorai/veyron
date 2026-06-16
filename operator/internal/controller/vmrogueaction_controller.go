// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"context"
	"fmt"
	"strconv"
	"time"

	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	vmroguev1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	"github.com/ssahani/Veyron/operator/internal/eventbus"
	vmmetrics "github.com/ssahani/Veyron/operator/internal/metrics"
)

const restartPhaseAnnotation = "vmrogue.io/restart-phase"

// VMRogueActionReconciler reconciles a VMRogueAction object.
type VMRogueActionReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueactions,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueactions/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueactions/finalizers,verbs=update

func (r *VMRogueActionReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var action vmroguev1alpha1.VMRogueAction
	if err := r.Get(ctx, req.NamespacedName, &action); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	// Skip if already completed or failed
	if action.Status.Phase == vmroguev1alpha1.ActionPhaseCompleted ||
		action.Status.Phase == vmroguev1alpha1.ActionPhaseFailed ||
		action.Status.Phase == vmroguev1alpha1.ActionPhaseRejected {
		return ctrl.Result{}, nil
	}

	// Auto-approve via status (do not mutate spec.Approved)
	if action.Spec.AutoApprove && action.Status.Phase == vmroguev1alpha1.ActionPhasePending {
		now := metav1.Now()
		action.Status.Phase = vmroguev1alpha1.ActionPhaseApproved
		setCondition(&action.Status.Conditions, metav1.Condition{
			Type:               "Approved",
			Status:             metav1.ConditionTrue,
			Reason:             "AutoApproved",
			Message:            "Action auto-approved",
			LastTransitionTime: now,
		})
		if err := r.Status().Update(ctx, &action); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{Requeue: true}, nil
	}

	approved := action.Spec.Approved ||
		action.Status.Phase == vmroguev1alpha1.ActionPhaseApproved ||
		action.Status.Phase == vmroguev1alpha1.ActionPhaseExecuting

	// If not approved, set status to Pending
	if !approved {
		if action.Status.Phase != vmroguev1alpha1.ActionPhasePending {
			action.Status.Phase = vmroguev1alpha1.ActionPhasePending
			now := metav1.Now()
			setCondition(&action.Status.Conditions, metav1.Condition{
				Type:               "Approved",
				Status:             metav1.ConditionFalse,
				Reason:             "AwaitingApproval",
				Message:            "Action requires approval before execution",
				LastTransitionTime: now,
			})
			_ = r.Status().Update(ctx, &action)
		}
		return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
	}

	if action.Spec.ActionType == "RestartVM" {
		return r.reconcileRestartVM(ctx, &action)
	}

	// Execute the action
	logger.Info("executing action", "type", action.Spec.ActionType, "vm", action.Spec.VMRef)

	if action.Status.Phase != vmroguev1alpha1.ActionPhaseExecuting {
		now := metav1.Now()
		action.Status.Phase = vmroguev1alpha1.ActionPhaseExecuting
		action.Status.StartedAt = &now
		setCondition(&action.Status.Conditions, metav1.Condition{
			Type:               "Approved",
			Status:             metav1.ConditionTrue,
			Reason:             "Approved",
			Message:            "Action has been approved",
			LastTransitionTime: now,
		})
		if err := r.Status().Update(ctx, &action); err != nil {
			return ctrl.Result{}, err
		}
	}

	err := r.executeAction(ctx, &action)

	completedAt := metav1.Now()
	action.Status.CompletedAt = &completedAt

	if err != nil {
		logger.Error(err, "action execution failed")
		action.Status.Phase = vmroguev1alpha1.ActionPhaseFailed
		action.Status.Message = fmt.Sprintf("Execution failed: %v", err)
		r.Recorder.Eventf(&action, "Warning", "ExecutionFailed", "Action %s failed: %v", action.Spec.ActionType, err)
		r.publishActionEvent(eventbus.SubjectActionFailed, &action)
		vmmetrics.ActionExecutions.WithLabelValues(action.Spec.ActionType, "failed").Inc()
	} else {
		action.Status.Phase = vmroguev1alpha1.ActionPhaseCompleted
		action.Status.Message = "Action completed successfully"
		r.Recorder.Eventf(&action, "Normal", "Executed", "Action %s completed for VM %s", action.Spec.ActionType, action.Spec.VMRef)
		r.publishActionEvent(eventbus.SubjectActionExecuted, &action)
		vmmetrics.ActionExecutions.WithLabelValues(action.Spec.ActionType, "success").Inc()
	}

	setCondition(&action.Status.Conditions, metav1.Condition{
		Type:               "Executed",
		Status:             conditionBool(err == nil),
		Reason:             string(action.Status.Phase),
		Message:            action.Status.Message,
		LastTransitionTime: completedAt,
	})

	if updateErr := r.Status().Update(ctx, &action); updateErr != nil {
		return ctrl.Result{}, updateErr
	}

	return ctrl.Result{}, nil
}

func (r *VMRogueActionReconciler) reconcileRestartVM(ctx context.Context, action *vmroguev1alpha1.VMRogueAction) (ctrl.Result, error) {
	logger := log.FromContext(ctx)
	namespace := action.Spec.Namespace
	if namespace == "" {
		namespace = action.Namespace
	}
	if action.Spec.VMRef == "" {
		return r.failRestart(ctx, action, fmt.Errorf("RestartVM requires vmRef"))
	}

	ann := action.GetAnnotations()
	if ann == nil {
		ann = map[string]string{}
	}
	phase := ann[restartPhaseAnnotation]

	switch phase {
	case "":
		now := metav1.Now()
		if action.Status.Phase != vmroguev1alpha1.ActionPhaseExecuting {
			action.Status.Phase = vmroguev1alpha1.ActionPhaseExecuting
			action.Status.StartedAt = &now
			action.Status.Message = "Restart: stopping VM"
			if err := r.Status().Update(ctx, action); err != nil {
				return ctrl.Result{}, err
			}
		}
		if err := r.patchVMRunning(ctx, namespace, action.Spec.VMRef, false); err != nil {
			return r.failRestart(ctx, action, fmt.Errorf("stop phase: %w", err))
		}
		ann[restartPhaseAnnotation] = "wait-stop"
		action.SetAnnotations(ann)
		if err := r.Update(ctx, action); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{RequeueAfter: 5 * time.Second}, nil

	case "wait-stop":
		vmi := &unstructured.Unstructured{}
		vmi.SetGroupVersionKind(schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachineInstance"})
		err := r.Get(ctx, client.ObjectKey{Namespace: namespace, Name: action.Spec.VMRef}, vmi)
		if err == nil {
			action.Status.Message = "Restart: waiting for VM to stop"
			_ = r.Status().Update(ctx, action)
			return ctrl.Result{RequeueAfter: 5 * time.Second}, nil
		}
		if !errors.IsNotFound(err) {
			return r.failRestart(ctx, action, err)
		}
		if err := r.patchVMRunning(ctx, namespace, action.Spec.VMRef, true); err != nil {
			return r.failRestart(ctx, action, fmt.Errorf("start phase: %w", err))
		}
		delete(ann, restartPhaseAnnotation)
		action.SetAnnotations(ann)
		if err := r.Update(ctx, action); err != nil {
			return ctrl.Result{}, err
		}

		completedAt := metav1.Now()
		action.Status.Phase = vmroguev1alpha1.ActionPhaseCompleted
		action.Status.CompletedAt = &completedAt
		action.Status.Message = "Restart completed successfully"
		setCondition(&action.Status.Conditions, metav1.Condition{
			Type:               "Executed",
			Status:             metav1.ConditionTrue,
			Reason:             string(vmroguev1alpha1.ActionPhaseCompleted),
			Message:            action.Status.Message,
			LastTransitionTime: completedAt,
		})
		if err := r.Status().Update(ctx, action); err != nil {
			return ctrl.Result{}, err
		}
		r.Recorder.Eventf(action, "Normal", "Executed", "RestartVM completed for VM %s", action.Spec.VMRef)
		r.publishActionEvent(eventbus.SubjectActionExecuted, action)
		vmmetrics.ActionExecutions.WithLabelValues("RestartVM", "success").Inc()
		logger.Info("RestartVM completed", "vm", action.Spec.VMRef)
		return ctrl.Result{}, nil

	default:
		return r.failRestart(ctx, action, fmt.Errorf("unknown restart phase %q", phase))
	}
}

func (r *VMRogueActionReconciler) failRestart(ctx context.Context, action *vmroguev1alpha1.VMRogueAction, err error) (ctrl.Result, error) {
	completedAt := metav1.Now()
	action.Status.Phase = vmroguev1alpha1.ActionPhaseFailed
	action.Status.CompletedAt = &completedAt
	action.Status.Message = fmt.Sprintf("Execution failed: %v", err)
	setCondition(&action.Status.Conditions, metav1.Condition{
		Type:               "Executed",
		Status:             metav1.ConditionFalse,
		Reason:             string(vmroguev1alpha1.ActionPhaseFailed),
		Message:            action.Status.Message,
		LastTransitionTime: completedAt,
	})
	_ = r.Status().Update(ctx, action)
	r.Recorder.Eventf(action, "Warning", "ExecutionFailed", "RestartVM failed: %v", err)
	r.publishActionEvent(eventbus.SubjectActionFailed, action)
	vmmetrics.ActionExecutions.WithLabelValues("RestartVM", "failed").Inc()
	return ctrl.Result{}, err
}

func (r *VMRogueActionReconciler) executeAction(ctx context.Context, action *vmroguev1alpha1.VMRogueAction) error {
	if action.Spec.VMRef == "" && requiresVMRef(action.Spec.ActionType) {
		return fmt.Errorf("action type %s requires a vmRef", action.Spec.ActionType)
	}

	namespace := action.Spec.Namespace
	if namespace == "" {
		namespace = action.Namespace
	}

	switch action.Spec.ActionType {
	case "StartVM":
		return r.patchVMRunning(ctx, namespace, action.Spec.VMRef, true)
	case "StopVM":
		return r.patchVMRunning(ctx, namespace, action.Spec.VMRef, false)
	case "ScaleResources":
		return r.scaleVM(ctx, namespace, action.Spec.VMRef, action.Spec.Parameters)
	case "CreateSnapshot":
		return r.createSnapshot(ctx, namespace, action.Spec.VMRef, action.Spec.Parameters)
	case "DeleteVM":
		return r.deleteVM(ctx, namespace, action.Spec.VMRef)
	case "Migrate":
		return r.migrateVM(ctx, namespace, action.Spec.VMRef)
	case "SendNotification":
		msg := action.Spec.Parameters["message"]
		if msg == "" {
			msg = fmt.Sprintf("Action notification for VM %s", action.Spec.VMRef)
		}
		log.FromContext(ctx).Info("Notification", "message", msg, "vm", action.Spec.VMRef)
		return nil
	default:
		return fmt.Errorf("unsupported action type: %s", action.Spec.ActionType)
	}
}

func (r *VMRogueActionReconciler) patchVMRunning(ctx context.Context, namespace, name string, running bool) error {
	var vm vmroguev1alpha1.VMRogueVM
	if err := r.Get(ctx, client.ObjectKey{Namespace: namespace, Name: name}, &vm); err != nil {
		return fmt.Errorf("getting VM %s/%s: %w", namespace, name, err)
	}

	vm.Spec.Running = &running
	return r.Update(ctx, &vm)
}

func (r *VMRogueActionReconciler) scaleVM(ctx context.Context, namespace, name string, params map[string]string) error {
	var vm vmroguev1alpha1.VMRogueVM
	if err := r.Get(ctx, client.ObjectKey{Namespace: namespace, Name: name}, &vm); err != nil {
		return fmt.Errorf("getting VM %s/%s: %w", namespace, name, err)
	}

	if cpuStr, ok := params["cpu"]; ok {
		cpu, err := strconv.ParseUint(cpuStr, 10, 32)
		if err != nil {
			return fmt.Errorf("invalid cpu value %q: %w", cpuStr, err)
		}
		vm.Spec.CPU.Cores = uint32(cpu)
	}

	if memory, ok := params["memory"]; ok {
		vm.Spec.Memory.Size = memory
	}

	return r.Update(ctx, &vm)
}

func (r *VMRogueActionReconciler) createSnapshot(ctx context.Context, namespace, vmName string, params map[string]string) error {
	snapshotName := params["name"]
	if snapshotName == "" {
		snapshotName = fmt.Sprintf("snap-%s-%d", vmName, time.Now().Unix())
	}

	snapshot := &unstructured.Unstructured{}
	snapshot.SetGroupVersionKind(schema.GroupVersionKind{
		Group:   "snapshot.kubevirt.io",
		Version: "v1alpha1",
		Kind:    "VirtualMachineSnapshot",
	})
	snapshot.SetNamespace(namespace)
	snapshot.SetName(snapshotName)
	snapshot.Object["spec"] = map[string]interface{}{
		"source": map[string]interface{}{
			"apiGroup": "kubevirt.io",
			"kind":     "VirtualMachine",
			"name":     vmName,
		},
	}

	return r.Create(ctx, snapshot)
}

func (r *VMRogueActionReconciler) deleteVM(ctx context.Context, namespace, name string) error {
	var vm vmroguev1alpha1.VMRogueVM
	if err := r.Get(ctx, client.ObjectKey{Namespace: namespace, Name: name}, &vm); err != nil {
		return fmt.Errorf("getting VM %s/%s: %w", namespace, name, err)
	}
	return r.Delete(ctx, &vm)
}

func (r *VMRogueActionReconciler) migrateVM(ctx context.Context, namespace, vmName string) error {
	migration := &unstructured.Unstructured{}
	migration.SetGroupVersionKind(schema.GroupVersionKind{
		Group:   "kubevirt.io",
		Version: "v1",
		Kind:    "VirtualMachineInstanceMigration",
	})
	migration.SetNamespace(namespace)
	migration.SetGenerateName(fmt.Sprintf("migrate-%s-", vmName))
	migration.Object["spec"] = map[string]interface{}{
		"vmiName": vmName,
	}

	return r.Create(ctx, migration)
}

func (r *VMRogueActionReconciler) publishActionEvent(subject string, action *vmroguev1alpha1.VMRogueAction) {
	if r.EventBus == nil {
		return
	}
	event, err := eventbus.NewEvent(subject, "operator", subject, eventbus.ActionEventData{
		Name:       action.Name,
		Namespace:  action.Namespace,
		ActionType: action.Spec.ActionType,
		VMRef:      action.Spec.VMRef,
		Phase:      string(action.Status.Phase),
		Message:    action.Status.Message,
	})
	if err == nil {
		_ = r.EventBus.Publish(event)
	}
}

func requiresVMRef(actionType string) bool {
	switch actionType {
	case "StartVM", "StopVM", "RestartVM", "ScaleResources", "CreateSnapshot", "DeleteVM", "Migrate":
		return true
	default:
		return false
	}
}

func (r *VMRogueActionReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&vmroguev1alpha1.VMRogueAction{}).
		Complete(r)
}
