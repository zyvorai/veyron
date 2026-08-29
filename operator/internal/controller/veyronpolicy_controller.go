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
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/client-go/tools/record"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	"github.com/google/cel-go/cel"
	"github.com/google/cel-go/common/types"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
	"github.com/zyvorai/veyron/operator/internal/eventbus"
)

// VeyronPolicyReconciler reconciles a VeyronPolicy object.
type VeyronPolicyReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=veyron.io,resources=veyronpolicies,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=veyronpolicies/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=veyron.io,resources=veyronvms,verbs=get;list;watch

func (r *VeyronPolicyReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var policy veyronv1alpha1.VeyronPolicy
	if err := r.Get(ctx, req.NamespacedName, &policy); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	// If disabled, clear violations and return
	if !policy.Spec.Enabled {
		policy.Status.MatchingVMs = 0
		policy.Status.CompliantVMs = 0
		policy.Status.ViolatingVMs = 0
		policy.Status.Violations = nil
		now := metav1.Now()
		policy.Status.LastEvaluated = &now
		_ = r.Status().Update(ctx, &policy)
		return ctrl.Result{RequeueAfter: 5 * time.Minute}, nil
	}

	// List all VeyronVMs in the namespace
	var vmList veyronv1alpha1.VeyronVMList
	listOpts := []client.ListOption{
		client.InNamespace(req.Namespace),
	}
	if err := r.List(ctx, &vmList, listOpts...); err != nil {
		return ctrl.Result{}, err
	}

	// Filter by selector
	var matchingVMs []veyronv1alpha1.VeyronVM
	if policy.Spec.Selector != nil {
		selector, err := metav1.LabelSelectorAsSelector(policy.Spec.Selector)
		if err != nil {
			logger.Error(err, "invalid label selector")
			now := metav1.Now()
			policy.Status.LastEvaluated = &now
			setCondition(&policy.Status.Conditions, metav1.Condition{
				Type:               "Evaluated",
				Status:             metav1.ConditionFalse,
				Reason:             "InvalidSelector",
				Message:            fmt.Sprintf("invalid spec.selector: %v", err),
				LastTransitionTime: now,
			})
			_ = r.Status().Update(ctx, &policy)
			return ctrl.Result{}, nil
		}
		for _, vm := range vmList.Items {
			if selector.Matches(labels.Set(vm.Labels)) {
				matchingVMs = append(matchingVMs, vm)
			}
		}
	} else {
		matchingVMs = vmList.Items
	}

	// Evaluate rules against each matching VM
	var violations []veyronv1alpha1.PolicyViolation
	compliantCount := 0

	for _, vm := range matchingVMs {
		vmViolations := r.evaluateRules(&policy, &vm)
		if len(vmViolations) == 0 {
			compliantCount++
		} else {
			violations = append(violations, vmViolations...)
		}
	}

	// Publish events for new violations
	for _, v := range violations {
		r.publishViolationEvent(&policy, &v)
	}

	// Update status
	now := metav1.Now()
	policy.Status.MatchingVMs = len(matchingVMs)
	policy.Status.CompliantVMs = compliantCount
	policy.Status.ViolatingVMs = len(matchingVMs) - compliantCount
	policy.Status.Violations = violations
	policy.Status.LastEvaluated = &now

	setCondition(&policy.Status.Conditions, metav1.Condition{
		Type:               "Evaluated",
		Status:             metav1.ConditionTrue,
		Reason:             "EvaluationComplete",
		Message:            fmt.Sprintf("Evaluated %d VMs: %d compliant, %d violations", len(matchingVMs), compliantCount, len(violations)),
		LastTransitionTime: now,
	})

	if err := r.Status().Update(ctx, &policy); err != nil {
		return ctrl.Result{}, err
	}

	if len(violations) > 0 {
		r.Recorder.Eventf(&policy, "Warning", "PolicyViolations",
			"%d VMs violating policy with %d total violations", len(matchingVMs)-compliantCount, len(violations))
	}

	return ctrl.Result{RequeueAfter: 2 * time.Minute}, nil
}

// evaluateRules checks a VM against all policy rules.
// Uses simple field-based evaluation. CEL evaluation can be added when cel-go is integrated.
func (r *VeyronPolicyReconciler) evaluateRules(policy *veyronv1alpha1.VeyronPolicy, vm *veyronv1alpha1.VeyronVM) []veyronv1alpha1.PolicyViolation {
	var violations []veyronv1alpha1.PolicyViolation

	for _, rule := range policy.Spec.Rules {
		compliant := evaluateCondition(rule.Condition, vm)
		if !compliant {
			violations = append(violations, veyronv1alpha1.PolicyViolation{
				VMName:    vm.Name,
				Namespace: vm.Namespace,
				RuleName:  rule.Name,
				Message:   rule.Message,
				Timestamp: metav1.Now(),
			})
		}
	}

	return violations
}

// evaluateCondition evaluates a policy rule condition against a VM.
// First checks built-in conditions for fast path, then falls back to CEL evaluation.
func evaluateCondition(condition string, vm *veyronv1alpha1.VeyronVM) bool {
	// Fast path: built-in conditions
	switch condition {
	case "spec.enableTpm == true":
		return vm.Spec.EnableTPM
	case "spec.enableRng == true":
		return vm.Spec.EnableRNG
	case "spec.cpu.cores >= 2":
		return vm.Spec.CPU.Cores >= 2
	case "spec.cpu.cores >= 4":
		return vm.Spec.CPU.Cores >= 4
	case "spec.features.acpi == true":
		return vm.Spec.Features != nil && vm.Spec.Features.ACPI
	case "spec.firmware.bootloader == 'efi'":
		return vm.Spec.Firmware != nil && vm.Spec.Firmware.Bootloader == "efi"
	case "spec.firmware.secureBoot == true":
		return vm.Spec.Firmware != nil && vm.Spec.Firmware.SecureBoot
	case "spec.evictionStrategy != nil":
		return vm.Spec.EvictionStrategy != nil
	case "has(spec.cloudInit)":
		return vm.Spec.CloudInit != nil
	}

	// Slow path: CEL expression evaluation
	result, err := evaluateCEL(condition, vm)
	if err != nil {
		log.Log.Info("CEL evaluation failed, treating as non-compliant",
			"condition", condition, "vm", vm.Name, "error", err.Error())
		return false
	}
	return result
}

// evaluateCEL evaluates a CEL expression against a VM specification.
// The VM fields are exposed as CEL variables: cpu_cores, memory_size,
// enable_tpm, enable_rng, has_cloud_init, has_eviction_strategy,
// num_disks, num_interfaces, template, name, namespace.
func evaluateCEL(expression string, vm *veyronv1alpha1.VeyronVM) (bool, error) {
	env, err := cel.NewEnv(
		cel.Variable("cpu_cores", cel.UintType),
		cel.Variable("cpu_sockets", cel.UintType),
		cel.Variable("cpu_threads", cel.UintType),
		cel.Variable("memory_size", cel.StringType),
		cel.Variable("enable_tpm", cel.BoolType),
		cel.Variable("enable_rng", cel.BoolType),
		cel.Variable("has_cloud_init", cel.BoolType),
		cel.Variable("has_eviction_strategy", cel.BoolType),
		cel.Variable("has_firmware", cel.BoolType),
		cel.Variable("has_features", cel.BoolType),
		cel.Variable("num_disks", cel.UintType),
		cel.Variable("num_interfaces", cel.UintType),
		cel.Variable("template", cel.StringType),
		cel.Variable("name", cel.StringType),
		cel.Variable("namespace", cel.StringType),
		cel.Variable("running", cel.BoolType),
	)
	if err != nil {
		return false, fmt.Errorf("creating CEL environment: %w", err)
	}

	ast, issues := env.Compile(expression)
	if issues != nil && issues.Err() != nil {
		return false, fmt.Errorf("compiling CEL expression: %w", issues.Err())
	}

	// Ensure output is boolean
	if ast.OutputType() != cel.BoolType {
		return false, fmt.Errorf("CEL expression must return bool, got %s", ast.OutputType())
	}

	prg, err := env.Program(ast)
	if err != nil {
		return false, fmt.Errorf("creating CEL program: %w", err)
	}

	// Build variable map from VM spec
	running := false
	if vm.Spec.Running != nil {
		running = *vm.Spec.Running
	}
	vars := map[string]interface{}{
		"cpu_cores":              uint64(vm.Spec.CPU.Cores),
		"cpu_sockets":            uint64(vm.Spec.CPU.Sockets),
		"cpu_threads":            uint64(vm.Spec.CPU.Threads),
		"memory_size":            vm.Spec.Memory.Size,
		"enable_tpm":             vm.Spec.EnableTPM,
		"enable_rng":             vm.Spec.EnableRNG,
		"has_cloud_init":         vm.Spec.CloudInit != nil,
		"has_eviction_strategy":  vm.Spec.EvictionStrategy != nil,
		"has_firmware":           vm.Spec.Firmware != nil,
		"has_features":           vm.Spec.Features != nil,
		"num_disks":              uint64(len(vm.Spec.Disks)),
		"num_interfaces":         uint64(len(vm.Spec.Interfaces)),
		"template":               vm.Spec.Template,
		"name":                   vm.Name,
		"namespace":              vm.Namespace,
		"running":                running,
	}

	out, _, err := prg.Eval(vars)
	if err != nil {
		return false, fmt.Errorf("evaluating CEL expression: %w", err)
	}

	if out.Type() != types.BoolType {
		return false, fmt.Errorf("expected bool result, got %s", out.Type())
	}

	return out.Value().(bool), nil
}

func (r *VeyronPolicyReconciler) publishViolationEvent(policy *veyronv1alpha1.VeyronPolicy, v *veyronv1alpha1.PolicyViolation) {
	if r.EventBus == nil {
		return
	}
	event, err := eventbus.NewEvent(eventbus.SubjectPolicyViolation, "operator", eventbus.SubjectPolicyViolation, eventbus.PolicyViolationData{
		PolicyName: policy.Name,
		VMName:     v.VMName,
		Namespace:  v.Namespace,
		RuleName:   v.RuleName,
		Message:    v.Message,
	})
	if err == nil {
		_ = r.EventBus.Publish(event)
	}
}

func (r *VeyronPolicyReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VeyronPolicy{}).
		Complete(r)
}
