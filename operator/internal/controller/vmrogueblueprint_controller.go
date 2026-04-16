package controller

import (
	"context"
	"fmt"
	"time"

	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
	"sigs.k8s.io/controller-runtime/pkg/log"

	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
	"github.com/ssahani/vmrogue/operator/internal/eventbus"
)

const (
	blueprintFinalizer = "vmrogue.io/blueprint-finalizer"
)

// VMRogueBlueprintReconciler reconciles a VMRogueBlueprint object.
type VMRogueBlueprintReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueblueprints,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueblueprints/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmrogueblueprints/finalizers,verbs=update

func (r *VMRogueBlueprintReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var bp vmroguev1alpha1.VMRogueBlueprint
	if err := r.Get(ctx, req.NamespacedName, &bp); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	// Handle deletion
	if !bp.DeletionTimestamp.IsZero() {
		return r.handleDeletion(ctx, &bp)
	}

	// Add finalizer
	if !controllerutil.ContainsFinalizer(&bp, blueprintFinalizer) {
		controllerutil.AddFinalizer(&bp, blueprintFinalizer)
		if err := r.Update(ctx, &bp); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{Requeue: true}, nil
	}

	// Resolve deployment order via topological sort
	order, err := resolveDeploymentOrder(bp.Spec.VMs)
	if err != nil {
		logger.Error(err, "failed to resolve deployment order")
		r.updateBlueprintStatus(ctx, &bp, vmroguev1alpha1.BlueprintPhaseFailed, err.Error())
		r.Recorder.Eventf(&bp, "Warning", "InvalidDependencies", "Dependency resolution failed: %v", err)
		return ctrl.Result{}, nil
	}

	bp.Status.DeploymentOrder = order
	bp.Status.TotalVMs = len(bp.Spec.VMs)

	// Build VM spec lookup
	vmSpecs := make(map[string]vmroguev1alpha1.BlueprintVMSpec)
	for _, vm := range bp.Spec.VMs {
		vmSpecs[vm.Name] = vm
	}

	// Track statuses
	readyCount := 0
	allCreated := true
	anyFailed := false
	vmStatuses := make([]vmroguev1alpha1.VMDeploymentStatus, 0, len(order))

	for _, vmName := range order {
		vmSpec := vmSpecs[vmName]
		crName := fmt.Sprintf("%s-%s", bp.Name, vmName)

		// Check if VMRogueVM CR exists
		var existingVM vmroguev1alpha1.VMRogueVM
		err := r.Get(ctx, types.NamespacedName{
			Name:      crName,
			Namespace: bp.Namespace,
		}, &existingVM)

		if errors.IsNotFound(err) {
			// Check if dependencies are met
			depsMet := true
			for _, dep := range vmSpec.DependsOn {
				depCRName := fmt.Sprintf("%s-%s", bp.Name, dep)
				var depVM vmroguev1alpha1.VMRogueVM
				if err := r.Get(ctx, types.NamespacedName{
					Name:      depCRName,
					Namespace: bp.Namespace,
				}, &depVM); err != nil || depVM.Status.Phase != vmroguev1alpha1.VMPhaseRunning {
					depsMet = false
					break
				}
			}

			if !depsMet {
				allCreated = false
				vmStatuses = append(vmStatuses, vmroguev1alpha1.VMDeploymentStatus{
					Name:    vmName,
					Phase:   vmroguev1alpha1.VMPhasePending,
					Message: "Waiting for dependencies",
				})
				continue
			}

			// Create the VMRogueVM CR
			newVM := r.buildVMFromBlueprint(&bp, &vmSpec, crName)
			if err := r.Create(ctx, newVM); err != nil {
				logger.Error(err, "failed to create VMRogueVM for blueprint", "vm", crName)
				anyFailed = true
				vmStatuses = append(vmStatuses, vmroguev1alpha1.VMDeploymentStatus{
					Name:    vmName,
					Phase:   vmroguev1alpha1.VMPhaseFailed,
					VMRef:   crName,
					Message: fmt.Sprintf("Create failed: %v", err),
				})
				continue
			}

			r.Recorder.Eventf(&bp, "Normal", "VMCreated", "Created VMRogueVM %s", crName)
			vmStatuses = append(vmStatuses, vmroguev1alpha1.VMDeploymentStatus{
				Name:  vmName,
				Phase: vmroguev1alpha1.VMPhaseCreating,
				VMRef: crName,
			})
			allCreated = false

		} else if err != nil {
			return ctrl.Result{}, err
		} else {
			// VM exists, track its status
			status := vmroguev1alpha1.VMDeploymentStatus{
				Name:  vmName,
				Phase: existingVM.Status.Phase,
				VMRef: crName,
			}
			if existingVM.Status.Phase == vmroguev1alpha1.VMPhaseRunning {
				readyCount++
			}
			if existingVM.Status.Phase == vmroguev1alpha1.VMPhaseFailed {
				anyFailed = true
				status.Message = "VM failed"
			}
			vmStatuses = append(vmStatuses, status)
		}
	}

	// Update status
	bp.Status.VMStatuses = vmStatuses
	bp.Status.ReadyVMs = readyCount
	bp.Status.ObservedGeneration = bp.Generation

	var bpPhase vmroguev1alpha1.BlueprintPhase
	switch {
	case anyFailed:
		bpPhase = vmroguev1alpha1.BlueprintPhaseFailed
	case readyCount == len(order):
		bpPhase = vmroguev1alpha1.BlueprintPhaseReady
		r.publishBlueprintEvent(eventbus.SubjectBlueprintReady, &bp)
	case !allCreated || readyCount < len(order):
		bpPhase = vmroguev1alpha1.BlueprintPhaseDeploying
	default:
		bpPhase = vmroguev1alpha1.BlueprintPhasePending
	}

	bp.Status.Phase = bpPhase

	now := metav1.Now()
	setCondition(&bp.Status.Conditions, metav1.Condition{
		Type:               "Ready",
		Status:             conditionBool(bpPhase == vmroguev1alpha1.BlueprintPhaseReady),
		Reason:             string(bpPhase),
		Message:            fmt.Sprintf("%d/%d VMs ready", readyCount, len(order)),
		LastTransitionTime: now,
	})

	if err := r.Status().Update(ctx, &bp); err != nil {
		return ctrl.Result{}, err
	}

	// Requeue if not fully ready
	if bpPhase != vmroguev1alpha1.BlueprintPhaseReady && bpPhase != vmroguev1alpha1.BlueprintPhaseFailed {
		return ctrl.Result{RequeueAfter: 15 * time.Second}, nil
	}

	return ctrl.Result{RequeueAfter: 60 * time.Second}, nil
}

func (r *VMRogueBlueprintReconciler) handleDeletion(ctx context.Context, bp *vmroguev1alpha1.VMRogueBlueprint) (ctrl.Result, error) {
	if controllerutil.ContainsFinalizer(bp, blueprintFinalizer) {
		// Delete all owned VMRogueVM CRs
		for _, vmSpec := range bp.Spec.VMs {
			crName := fmt.Sprintf("%s-%s", bp.Name, vmSpec.Name)
			var vm vmroguev1alpha1.VMRogueVM
			if err := r.Get(ctx, types.NamespacedName{
				Name:      crName,
				Namespace: bp.Namespace,
			}, &vm); err == nil {
				_ = r.Delete(ctx, &vm)
			}
		}

		controllerutil.RemoveFinalizer(bp, blueprintFinalizer)
		if err := r.Update(ctx, bp); err != nil {
			return ctrl.Result{}, err
		}
	}
	return ctrl.Result{}, nil
}

func (r *VMRogueBlueprintReconciler) buildVMFromBlueprint(bp *vmroguev1alpha1.VMRogueBlueprint, vmSpec *vmroguev1alpha1.BlueprintVMSpec, crName string) *vmroguev1alpha1.VMRogueVM {
	vm := &vmroguev1alpha1.VMRogueVM{
		ObjectMeta: metav1.ObjectMeta{
			Name:      crName,
			Namespace: bp.Namespace,
			Labels: map[string]string{
				"vmrogue.io/blueprint": bp.Name,
				"vmrogue.io/vm-name":   vmSpec.Name,
			},
			OwnerReferences: []metav1.OwnerReference{
				{
					APIVersion:         vmroguev1alpha1.GroupVersion.String(),
					Kind:               "VMRogueBlueprint",
					Name:               bp.Name,
					UID:                bp.UID,
					Controller:         boolPtr(true),
					BlockOwnerDeletion: boolPtr(true),
				},
			},
		},
		Spec: vmroguev1alpha1.VMRogueVMSpec{
			Template: vmSpec.Template,
			Profile:  derefStringOr(vmSpec.Profile, ""),
			Running:  boolPtr(true),
		},
	}

	// Apply overrides
	if vmSpec.CPU != nil {
		vm.Spec.CPU.Cores = *vmSpec.CPU
	} else {
		vm.Spec.CPU.Cores = 2
	}
	vm.Spec.CPU.Sockets = 1
	vm.Spec.CPU.Threads = 1

	if vmSpec.Memory != nil {
		vm.Spec.Memory.Size = *vmSpec.Memory
	} else {
		vm.Spec.Memory.Size = "4Gi"
	}

	// Apply labels
	for k, v := range vmSpec.Labels {
		vm.Labels[k] = v
	}

	// Apply full override — merge onto existing spec, preserving base fields
	if vmSpec.Override != nil {
		override := vmSpec.Override
		if override.CPU.Cores > 0 {
			vm.Spec.CPU = override.CPU
		}
		if override.Memory.Size != "" {
			vm.Spec.Memory = override.Memory
		}
		if len(override.Disks) > 0 {
			vm.Spec.Disks = override.Disks
		}
		if len(override.Interfaces) > 0 {
			vm.Spec.Interfaces = override.Interfaces
		}
		if override.CloudInit != nil {
			vm.Spec.CloudInit = override.CloudInit
		}
		if override.Features != nil {
			vm.Spec.Features = override.Features
		}
		if override.Firmware != nil {
			vm.Spec.Firmware = override.Firmware
		}
		if override.Clock != nil {
			vm.Spec.Clock = override.Clock
		}
		vm.Spec.EnableTPM = override.EnableTPM
		vm.Spec.EnableRNG = override.EnableRNG
		if override.MachineType != nil {
			vm.Spec.MachineType = override.MachineType
		}
		if override.EvictionStrategy != nil {
			vm.Spec.EvictionStrategy = override.EvictionStrategy
		}
	}

	return vm
}

func (r *VMRogueBlueprintReconciler) updateBlueprintStatus(ctx context.Context, bp *vmroguev1alpha1.VMRogueBlueprint, phase vmroguev1alpha1.BlueprintPhase, message string) {
	bp.Status.Phase = phase
	now := metav1.Now()
	setCondition(&bp.Status.Conditions, metav1.Condition{
		Type:               "Ready",
		Status:             metav1.ConditionFalse,
		Reason:             string(phase),
		Message:            message,
		LastTransitionTime: now,
	})
	_ = r.Status().Update(ctx, bp)
}

func (r *VMRogueBlueprintReconciler) publishBlueprintEvent(subject string, bp *vmroguev1alpha1.VMRogueBlueprint) {
	if r.EventBus == nil {
		return
	}
	event, err := eventbus.NewEvent(subject, "operator", subject, eventbus.BlueprintEventData{
		Name:      bp.Name,
		Namespace: bp.Namespace,
		Phase:     string(bp.Status.Phase),
		TotalVMs:  bp.Status.TotalVMs,
		ReadyVMs:  bp.Status.ReadyVMs,
	})
	if err == nil {
		_ = r.EventBus.Publish(event)
	}
}

// SetupWithManager sets up the controller with the Manager.
func (r *VMRogueBlueprintReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&vmroguev1alpha1.VMRogueBlueprint{}).
		Owns(&vmroguev1alpha1.VMRogueVM{}).
		Complete(r)
}

// resolveDeploymentOrder performs a topological sort (Kahn's algorithm) on blueprint VMs.
// Ported from the Rust implementation in src/blueprints/validator.rs.
func resolveDeploymentOrder(vms []vmroguev1alpha1.BlueprintVMSpec) ([]string, error) {
	// Build adjacency list and in-degree count
	inDegree := make(map[string]int)
	dependents := make(map[string][]string)
	vmNames := make(map[string]bool)

	for _, vm := range vms {
		vmNames[vm.Name] = true
		if _, exists := inDegree[vm.Name]; !exists {
			inDegree[vm.Name] = 0
		}
	}

	for _, vm := range vms {
		for _, dep := range vm.DependsOn {
			if !vmNames[dep] {
				return nil, fmt.Errorf("VM %q depends on unknown VM %q", vm.Name, dep)
			}
			inDegree[vm.Name]++
			dependents[dep] = append(dependents[dep], vm.Name)
		}
	}

	// Start with nodes that have no dependencies
	var queue []string
	for name, degree := range inDegree {
		if degree == 0 {
			queue = append(queue, name)
		}
	}

	var order []string
	for len(queue) > 0 {
		// Dequeue
		node := queue[0]
		queue = queue[1:]
		order = append(order, node)

		for _, dep := range dependents[node] {
			inDegree[dep]--
			if inDegree[dep] == 0 {
				queue = append(queue, dep)
			}
		}
	}

	if len(order) != len(vms) {
		return nil, fmt.Errorf("circular dependency detected in blueprint VMs")
	}

	return order, nil
}

func conditionBool(b bool) metav1.ConditionStatus {
	if b {
		return metav1.ConditionTrue
	}
	return metav1.ConditionFalse
}

func derefStringOr(s *string, def string) string {
	if s != nil {
		return *s
	}
	return def
}

func boolPtr(b bool) *bool {
	return &b
}
