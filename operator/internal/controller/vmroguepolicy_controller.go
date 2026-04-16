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

	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
	"github.com/ssahani/vmrogue/operator/internal/eventbus"
)

// VMRoguePolicyReconciler reconciles a VMRoguePolicy object.
type VMRoguePolicyReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguepolicies,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguepolicies/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=vmrogue.io,resources=vmroguevms,verbs=get;list;watch

func (r *VMRoguePolicyReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var policy vmroguev1alpha1.VMRoguePolicy
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

	// List all VMRogueVMs in the namespace
	var vmList vmroguev1alpha1.VMRogueVMList
	listOpts := []client.ListOption{
		client.InNamespace(req.Namespace),
	}
	if err := r.List(ctx, &vmList, listOpts...); err != nil {
		return ctrl.Result{}, err
	}

	// Filter by selector
	var matchingVMs []vmroguev1alpha1.VMRogueVM
	if policy.Spec.Selector != nil {
		selector, err := metav1.LabelSelectorAsSelector(policy.Spec.Selector)
		if err != nil {
			logger.Error(err, "invalid label selector")
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
	var violations []vmroguev1alpha1.PolicyViolation
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
func (r *VMRoguePolicyReconciler) evaluateRules(policy *vmroguev1alpha1.VMRoguePolicy, vm *vmroguev1alpha1.VMRogueVM) []vmroguev1alpha1.PolicyViolation {
	var violations []vmroguev1alpha1.PolicyViolation

	for _, rule := range policy.Spec.Rules {
		compliant := evaluateCondition(rule.Condition, vm)
		if !compliant {
			violations = append(violations, vmroguev1alpha1.PolicyViolation{
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
// Supports built-in conditions. Full CEL support via cel-go can be added later.
func evaluateCondition(condition string, vm *vmroguev1alpha1.VMRogueVM) bool {
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
	default:
		// Unknown condition — treat as NON-COMPLIANT and log a warning.
		// This prevents typos from silently bypassing policies.
		log.Log.Info("unknown policy condition, treating as non-compliant", "condition", condition, "vm", vm.Name)
		return false
	}
}

func (r *VMRoguePolicyReconciler) publishViolationEvent(policy *vmroguev1alpha1.VMRoguePolicy, v *vmroguev1alpha1.PolicyViolation) {
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

func (r *VMRoguePolicyReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&vmroguev1alpha1.VMRoguePolicy{}).
		Complete(r)
}
