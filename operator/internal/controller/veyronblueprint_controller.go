// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	"github.com/ssahani/Veyron/operator/internal/catalog"
	"github.com/ssahani/Veyron/operator/internal/eventbus"
)

const (
	blueprintFinalizer = "veyron.io/blueprint-finalizer"
)

// VeyronBlueprintReconciler reconciles a VeyronBlueprint object.
type VeyronBlueprintReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=veyron.io,resources=veyronblueprints,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=veyronblueprints/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=veyron.io,resources=veyronblueprints/finalizers,verbs=update

func (r *VeyronBlueprintReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var bp veyronv1alpha1.VeyronBlueprint
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
		r.updateBlueprintStatus(ctx, &bp, veyronv1alpha1.BlueprintPhaseFailed, err.Error())
		r.Recorder.Eventf(&bp, "Warning", "InvalidDependencies", "Dependency resolution failed: %v", err)
		return ctrl.Result{}, nil
	}

	bp.Status.DeploymentOrder = order
	bp.Status.TotalVMs = len(bp.Spec.VMs)

	// Build VM spec lookup
	vmSpecs := make(map[string]veyronv1alpha1.BlueprintVMSpec)
	for _, vm := range bp.Spec.VMs {
		vmSpecs[vm.Name] = vm
	}

	// Track statuses
	readyCount := 0
	allCreated := true
	anyFailed := false
	vmStatuses := make([]veyronv1alpha1.VMDeploymentStatus, 0, len(order))

	for _, vmName := range order {
		vmSpec := vmSpecs[vmName]
		crName := fmt.Sprintf("%s-%s", bp.Name, vmName)

		// Check if VeyronVM CR exists
		var existingVM veyronv1alpha1.VeyronVM
		err := r.Get(ctx, types.NamespacedName{
			Name:      crName,
			Namespace: bp.Namespace,
		}, &existingVM)

		if errors.IsNotFound(err) {
			// Check if dependencies are met
			depsMet := true
			for _, dep := range vmSpec.DependsOn {
				depCRName := fmt.Sprintf("%s-%s", bp.Name, dep)
				var depVM veyronv1alpha1.VeyronVM
				if err := r.Get(ctx, types.NamespacedName{
					Name:      depCRName,
					Namespace: bp.Namespace,
				}, &depVM); err != nil || depVM.Status.Phase != veyronv1alpha1.VMPhaseRunning {
					depsMet = false
					break
				}
			}

			if !depsMet {
				allCreated = false
				vmStatuses = append(vmStatuses, veyronv1alpha1.VMDeploymentStatus{
					Name:    vmName,
					Phase:   veyronv1alpha1.VMPhasePending,
					Message: "Waiting for dependencies",
				})
				continue
			}

			// Create the VeyronVM CR
			newVM, err := r.buildVMFromBlueprint(ctx, &bp, &vmSpec, crName)
			if err != nil {
				logger.Error(err, "failed to build VeyronVM spec", "vm", crName)
				anyFailed = true
				vmStatuses = append(vmStatuses, veyronv1alpha1.VMDeploymentStatus{
					Name:    vmName,
					Phase:   veyronv1alpha1.VMPhaseFailed,
					VMRef:   crName,
					Message: fmt.Sprintf("Build failed: %v", err),
				})
				continue
			}
			if err := r.Create(ctx, newVM); err != nil {
				logger.Error(err, "failed to create VeyronVM for blueprint", "vm", crName)
				anyFailed = true
				vmStatuses = append(vmStatuses, veyronv1alpha1.VMDeploymentStatus{
					Name:    vmName,
					Phase:   veyronv1alpha1.VMPhaseFailed,
					VMRef:   crName,
					Message: fmt.Sprintf("Create failed: %v", err),
				})
				continue
			}

			r.Recorder.Eventf(&bp, "Normal", "VMCreated", "Created VeyronVM %s", crName)
			vmStatuses = append(vmStatuses, veyronv1alpha1.VMDeploymentStatus{
				Name:  vmName,
				Phase: veyronv1alpha1.VMPhaseCreating,
				VMRef: crName,
			})
			allCreated = false

		} else if err != nil {
			return ctrl.Result{}, err
		} else {
			// VM exists, track its status
			status := veyronv1alpha1.VMDeploymentStatus{
				Name:  vmName,
				Phase: existingVM.Status.Phase,
				VMRef: crName,
			}
			if existingVM.Status.Phase == veyronv1alpha1.VMPhaseRunning {
				readyCount++
			}
			if existingVM.Status.Phase == veyronv1alpha1.VMPhaseFailed {
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

	var bpPhase veyronv1alpha1.BlueprintPhase
	switch {
	case anyFailed:
		bpPhase = veyronv1alpha1.BlueprintPhaseFailed
	case readyCount == len(order):
		bpPhase = veyronv1alpha1.BlueprintPhaseReady
		r.publishBlueprintEvent(eventbus.SubjectBlueprintReady, &bp)
	case !allCreated || readyCount < len(order):
		bpPhase = veyronv1alpha1.BlueprintPhaseDeploying
	default:
		bpPhase = veyronv1alpha1.BlueprintPhasePending
	}

	bp.Status.Phase = bpPhase

	now := metav1.Now()
	setCondition(&bp.Status.Conditions, metav1.Condition{
		Type:               "Ready",
		Status:             conditionBool(bpPhase == veyronv1alpha1.BlueprintPhaseReady),
		Reason:             string(bpPhase),
		Message:            fmt.Sprintf("%d/%d VMs ready", readyCount, len(order)),
		LastTransitionTime: now,
	})

	if err := r.Status().Update(ctx, &bp); err != nil {
		return ctrl.Result{}, err
	}

	// Requeue if not fully ready
	if bpPhase != veyronv1alpha1.BlueprintPhaseReady && bpPhase != veyronv1alpha1.BlueprintPhaseFailed {
		return ctrl.Result{RequeueAfter: 15 * time.Second}, nil
	}

	return ctrl.Result{RequeueAfter: 60 * time.Second}, nil
}

func (r *VeyronBlueprintReconciler) handleDeletion(ctx context.Context, bp *veyronv1alpha1.VeyronBlueprint) (ctrl.Result, error) {
	logger := log.FromContext(ctx)
	if controllerutil.ContainsFinalizer(bp, blueprintFinalizer) {
		// Delete all owned VeyronVM CRs. A real delete error (RBAC-forbidden,
		// admission-webhook denial, etc.) must not be silently discarded —
		// mirrors VeyronVMReconciler.handleDeletion's pattern of only
		// removing the finalizer once every owned resource is actually gone.
		for _, vmSpec := range bp.Spec.VMs {
			crName := fmt.Sprintf("%s-%s", bp.Name, vmSpec.Name)
			var vm veyronv1alpha1.VeyronVM
			if err := r.Get(ctx, types.NamespacedName{
				Name:      crName,
				Namespace: bp.Namespace,
			}, &vm); err == nil {
				if err := r.Delete(ctx, &vm); err != nil && !errors.IsNotFound(err) {
					logger.Error(err, "failed to delete child VeyronVM", "name", crName)
					return ctrl.Result{}, err
				}
			} else if !errors.IsNotFound(err) {
				logger.Error(err, "failed to get child VeyronVM", "name", crName)
				return ctrl.Result{}, err
			}
		}

		controllerutil.RemoveFinalizer(bp, blueprintFinalizer)
		if err := r.Update(ctx, bp); err != nil {
			return ctrl.Result{}, err
		}
	}
	return ctrl.Result{}, nil
}

func (r *VeyronBlueprintReconciler) buildVMFromBlueprint(ctx context.Context, bp *veyronv1alpha1.VeyronBlueprint, vmSpec *veyronv1alpha1.BlueprintVMSpec, crName string) (*veyronv1alpha1.VeyronVM, error) {
	resolver := catalog.Resolver{Client: r.Client}
	profile := derefStringOr(vmSpec.Profile, "")

	base := veyronv1alpha1.VeyronVMSpec{
		Running: boolPtr(true),
	}
	resolved, err := resolver.ResolveSpec(ctx, vmSpec.Template, profile, base, catalog.BlueprintOverrides{
		CPU:      vmSpec.CPU,
		Memory:   vmSpec.Memory,
		DiskSize: vmSpec.DiskSize,
		Override: vmSpec.Override,
	})
	if err != nil {
		return nil, err
	}

	vm := &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{
			Name:      crName,
			Namespace: bp.Namespace,
			Labels: map[string]string{
				"veyron.io/blueprint": bp.Name,
				"veyron.io/vm-name":   vmSpec.Name,
			},
			OwnerReferences: []metav1.OwnerReference{
				{
					APIVersion:         veyronv1alpha1.GroupVersion.String(),
					Kind:               "VeyronBlueprint",
					Name:               bp.Name,
					UID:                bp.UID,
					Controller:         boolPtr(true),
					BlockOwnerDeletion: boolPtr(true),
				},
			},
		},
		Spec: resolved,
	}

	for k, v := range vmSpec.Labels {
		vm.Labels[k] = v
	}

	return vm, nil
}

func (r *VeyronBlueprintReconciler) updateBlueprintStatus(ctx context.Context, bp *veyronv1alpha1.VeyronBlueprint, phase veyronv1alpha1.BlueprintPhase, message string) {
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

func (r *VeyronBlueprintReconciler) publishBlueprintEvent(subject string, bp *veyronv1alpha1.VeyronBlueprint) {
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
func (r *VeyronBlueprintReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VeyronBlueprint{}).
		Owns(&veyronv1alpha1.VeyronVM{}).
		Complete(r)
}

// resolveDeploymentOrder performs a topological sort (Kahn's algorithm) on blueprint VMs.
// Ported from the Rust implementation in src/blueprints/validator.rs.
func resolveDeploymentOrder(vms []veyronv1alpha1.BlueprintVMSpec) ([]string, error) {
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
