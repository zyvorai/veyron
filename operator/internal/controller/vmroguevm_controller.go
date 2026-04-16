package controller

import (
	"context"
	"fmt"
	"time"

	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
	"sigs.k8s.io/controller-runtime/pkg/log"

	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
	"github.com/ssahani/vmrogue/operator/internal/converter"
	"github.com/ssahani/vmrogue/operator/internal/eventbus"
	vmmetrics "github.com/ssahani/vmrogue/operator/internal/metrics"
)

const (
	vmFinalizer = "vmrogue.io/vm-finalizer"
)

// VMRogueVMReconciler reconciles a VMRogueVM object.
type VMRogueVMReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguevms,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguevms/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguevms/finalizers,verbs=update
// +kubebuilder:rbac:groups=kubevirt.io,resources=virtualmachines,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=kubevirt.io,resources=virtualmachineinstances,verbs=get;list;watch
// +kubebuilder:rbac:groups="",resources=events,verbs=create;patch

func (r *VMRogueVMReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)
	reconcileStart := time.Now()
	defer func() {
		vmmetrics.ReconcileDuration.WithLabelValues("vmroguevm").Observe(time.Since(reconcileStart).Seconds())
	}()

	// Fetch the VMRogueVM
	var vm vmroguev1alpha1.VMRogueVM
	if err := r.Get(ctx, req.NamespacedName, &vm); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		vmmetrics.ReconcileTotal.WithLabelValues("vmroguevm", "error").Inc()
		return ctrl.Result{}, err
	}

	// Handle deletion
	if !vm.DeletionTimestamp.IsZero() {
		return r.handleDeletion(ctx, &vm)
	}

	// Add finalizer if not present
	if !controllerutil.ContainsFinalizer(&vm, vmFinalizer) {
		controllerutil.AddFinalizer(&vm, vmFinalizer)
		if err := r.Update(ctx, &vm); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{Requeue: true}, nil
	}

	// Convert VMRogueVM spec to KubeVirt VirtualMachine
	desired, err := converter.VMRogueVMToKubeVirt(&vm)
	if err != nil {
		logger.Error(err, "failed to convert VMRogueVM to KubeVirt VM")
		r.updateStatus(ctx, &vm, vmroguev1alpha1.VMPhaseFailed, fmt.Sprintf("conversion error: %v", err))
		return ctrl.Result{}, err
	}

	// Create or update the KubeVirt VirtualMachine
	existing := &unstructured.Unstructured{}
	existing.SetGroupVersionKind(schema.GroupVersionKind{
		Group:   "kubevirt.io",
		Version: "v1",
		Kind:    "VirtualMachine",
	})

	err = r.Get(ctx, types.NamespacedName{
		Name:      vm.Name,
		Namespace: vm.Namespace,
	}, existing)

	if errors.IsNotFound(err) {
		// Create new KubeVirt VM
		logger.Info("creating KubeVirt VirtualMachine", "name", vm.Name)

		if err := r.Create(ctx, desired); err != nil {
			logger.Error(err, "failed to create KubeVirt VM")
			r.Recorder.Eventf(&vm, "Warning", "CreateFailed", "Failed to create KubeVirt VM: %v", err)
			return ctrl.Result{RequeueAfter: 10 * time.Second}, nil
		}

		r.Recorder.Event(&vm, "Normal", "Created", "Created KubeVirt VirtualMachine")
		r.publishEvent(eventbus.SubjectVMCreated, &vm, "Created")

	} else if err != nil {
		return ctrl.Result{}, err
	} else {
		// Update existing KubeVirt VM spec
		existingSpec, _, _ := unstructured.NestedMap(existing.Object, "spec")
		desiredSpec, _, _ := unstructured.NestedMap(desired.Object, "spec")

		if existingSpec != nil && desiredSpec != nil {
			existing.Object["spec"] = desiredSpec
			if err := r.Update(ctx, existing); err != nil {
				logger.Error(err, "failed to update KubeVirt VM")
				return ctrl.Result{}, err
			}
			r.publishEvent(eventbus.SubjectVMUpdated, &vm, "Updated")
		}
	}

	// Re-fetch the VMRogueVM to get the latest ResourceVersion before status update
	if err := r.Get(ctx, req.NamespacedName, &vm); err != nil {
		return ctrl.Result{}, err
	}

	// Read VMI status for runtime info
	phase, nodeName, ipAddress := r.readVMIStatus(ctx, vm.Namespace, vm.Name)

	// Update VMRogueVM status (single status update with latest ResourceVersion)
	now := metav1.Now()
	vm.Status.KubevirtVMName = vm.Name
	vm.Status.Phase = phase
	vm.Status.NodeName = nodeName
	vm.Status.IPAddress = ipAddress
	vm.Status.LastReconciled = &now
	vm.Status.ObservedGeneration = vm.Generation

	condition := metav1.Condition{
		Type:               "Ready",
		Status:             metav1.ConditionFalse,
		Reason:             string(phase),
		Message:            fmt.Sprintf("VM is in %s phase", phase),
		LastTransitionTime: now,
	}
	if phase == vmroguev1alpha1.VMPhaseRunning {
		condition.Status = metav1.ConditionTrue
		condition.Message = "VM is running"
	}
	setCondition(&vm.Status.Conditions, condition)

	if err := r.Status().Update(ctx, &vm); err != nil {
		logger.Error(err, "failed to update VMRogueVM status")
		return ctrl.Result{}, err
	}

	// Requeue to poll VMI status until stable
	if phase != vmroguev1alpha1.VMPhaseRunning && phase != vmroguev1alpha1.VMPhaseStopped {
		return ctrl.Result{RequeueAfter: 15 * time.Second}, nil
	}

	return ctrl.Result{RequeueAfter: 60 * time.Second}, nil
}

func (r *VMRogueVMReconciler) handleDeletion(ctx context.Context, vm *vmroguev1alpha1.VMRogueVM) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	if controllerutil.ContainsFinalizer(vm, vmFinalizer) {
		// Delete the owned KubeVirt VM
		kvVM := &unstructured.Unstructured{}
		kvVM.SetGroupVersionKind(schema.GroupVersionKind{
			Group:   "kubevirt.io",
			Version: "v1",
			Kind:    "VirtualMachine",
		})

		err := r.Get(ctx, types.NamespacedName{
			Name:      vm.Name,
			Namespace: vm.Namespace,
		}, kvVM)

		if err == nil {
			logger.Info("deleting KubeVirt VirtualMachine", "name", vm.Name)
			if err := r.Delete(ctx, kvVM); err != nil && !errors.IsNotFound(err) {
				return ctrl.Result{}, err
			}
		}

		r.publishEvent(eventbus.SubjectVMDeleted, vm, "Deleted")

		// Remove finalizer
		controllerutil.RemoveFinalizer(vm, vmFinalizer)
		if err := r.Update(ctx, vm); err != nil {
			return ctrl.Result{}, err
		}
	}

	return ctrl.Result{}, nil
}

func (r *VMRogueVMReconciler) readVMIStatus(ctx context.Context, namespace, name string) (vmroguev1alpha1.VMRogueVMPhase, string, string) {
	vmi := &unstructured.Unstructured{}
	vmi.SetGroupVersionKind(schema.GroupVersionKind{
		Group:   "kubevirt.io",
		Version: "v1",
		Kind:    "VirtualMachineInstance",
	})

	err := r.Get(ctx, types.NamespacedName{
		Name:      name,
		Namespace: namespace,
	}, vmi)

	if errors.IsNotFound(err) {
		return vmroguev1alpha1.VMPhaseStopped, "", ""
	}
	if err != nil {
		return vmroguev1alpha1.VMPhaseUnknown, "", ""
	}

	phase, _, _ := unstructured.NestedString(vmi.Object, "status", "phase")
	nodeName, _, _ := unstructured.NestedString(vmi.Object, "status", "nodeName")

	// Try to get IP from interfaces
	ipAddress := ""
	interfaces, found, _ := unstructured.NestedSlice(vmi.Object, "status", "interfaces")
	if found && len(interfaces) > 0 {
		if ifaceMap, ok := interfaces[0].(map[string]interface{}); ok {
			if ip, ok := ifaceMap["ipAddress"].(string); ok {
				ipAddress = ip
			}
		}
	}

	var vmPhase vmroguev1alpha1.VMRogueVMPhase
	switch phase {
	case "Running":
		vmPhase = vmroguev1alpha1.VMPhaseRunning
	case "Succeeded":
		vmPhase = vmroguev1alpha1.VMPhaseStopped
	case "Failed":
		vmPhase = vmroguev1alpha1.VMPhaseFailed
	case "Pending", "Scheduling", "Scheduled":
		vmPhase = vmroguev1alpha1.VMPhaseCreating
	default:
		vmPhase = vmroguev1alpha1.VMPhasePending
	}

	return vmPhase, nodeName, ipAddress
}

func (r *VMRogueVMReconciler) updateStatus(ctx context.Context, vm *vmroguev1alpha1.VMRogueVM, phase vmroguev1alpha1.VMRogueVMPhase, message string) {
	vm.Status.Phase = phase
	now := metav1.Now()
	vm.Status.LastReconciled = &now
	if message != "" {
		setCondition(&vm.Status.Conditions, metav1.Condition{
			Type:               "Ready",
			Status:             metav1.ConditionFalse,
			Reason:             string(phase),
			Message:            message,
			LastTransitionTime: now,
		})
	}
	_ = r.Status().Update(ctx, vm)
}

func (r *VMRogueVMReconciler) publishEvent(subject string, vm *vmroguev1alpha1.VMRogueVM, phase string) {
	if r.EventBus == nil {
		return
	}
	event, err := eventbus.NewEvent(subject, "operator", subject, eventbus.VMEventData{
		Name:      vm.Name,
		Namespace: vm.Namespace,
		Phase:     phase,
		NodeName:  vm.Status.NodeName,
		IPAddress: vm.Status.IPAddress,
	})
	if err == nil {
		_ = r.EventBus.Publish(event)
	}
}

// SetupWithManager sets up the controller with the Manager.
func (r *VMRogueVMReconciler) SetupWithManager(mgr ctrl.Manager) error {
	// Register the KubeVirt GVKs with the scheme so we can watch them
	kvVMGVK := schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachine"}
	kvVMIGVK := schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachineInstance"}

	kvVM := &unstructured.Unstructured{}
	kvVM.SetGroupVersionKind(kvVMGVK)

	kvVMI := &unstructured.Unstructured{}
	kvVMI.SetGroupVersionKind(kvVMIGVK)

	return ctrl.NewControllerManagedBy(mgr).
		For(&vmroguev1alpha1.VMRogueVM{}).
		Owns(kvVM).
		Watches(kvVMI, &EnqueueForOwner{OwnerKind: "VMRogueVM"}).
		Complete(r)
}

// setCondition updates or adds a condition in the conditions slice.
func setCondition(conditions *[]metav1.Condition, condition metav1.Condition) {
	if conditions == nil {
		return
	}
	for i, existing := range *conditions {
		if existing.Type == condition.Type {
			(*conditions)[i] = condition
			return
		}
	}
	*conditions = append(*conditions, condition)
}
