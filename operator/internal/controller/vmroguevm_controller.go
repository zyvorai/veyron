// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package controller

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	corev1 "k8s.io/api/core/v1"
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

	vmroguev1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	"github.com/ssahani/Veyron/operator/internal/catalog"
	"github.com/ssahani/Veyron/operator/internal/converter"
	"github.com/ssahani/Veyron/operator/internal/eventbus"
	"github.com/ssahani/Veyron/operator/internal/network"
	vmmetrics "github.com/ssahani/Veyron/operator/internal/metrics"
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
// +kubebuilder:rbac:groups="",resources=secrets,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=cilium.io,resources=ciliumnetworkpolicies,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=networking.k8s.io,resources=networkpolicies,verbs=get;list;watch;create;update;patch;delete

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

	resolver := catalog.Resolver{Client: r.Client}
	resolvedSpec, err := resolver.ResolveSpec(ctx, vm.Spec.Template, vm.Spec.Profile, vm.Spec, catalog.BlueprintOverrides{})
	if err != nil {
		logger.Error(err, "failed to resolve template/profile")
		r.updateStatus(ctx, &vm, vmroguev1alpha1.VMPhaseFailed, fmt.Sprintf("catalog resolve: %v", err))
		return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
	}

	workVM := vm.DeepCopy()
	workVM.Spec = resolvedSpec
	if workVM.Spec.CloudInit != nil {
		userData, uErr := catalog.ResolveCloudInitUserData(ctx, r.Client, vm.Namespace, &workVM.Spec)
		if uErr != nil {
			logger.Error(uErr, "failed to resolve cloud-init userData")
			r.updateStatus(ctx, &vm, vmroguev1alpha1.VMPhaseFailed, fmt.Sprintf("cloud-init: %v", uErr))
			return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
		}
		if userData != "" {
			workVM.Spec.CloudInit.UserData = userData
		}
	}

	if workVM.Spec.CloudInit != nil {
		d := strings.ToLower(strings.TrimSpace(workVM.Spec.CloudInit.Delivery))
		if d == "configdrive" || d == "config_drive" {
			if err := r.ensureConfigDriveSecret(ctx, workVM); err != nil {
				logger.Error(err, "failed to reconcile config-drive Secret")
				r.updateStatus(ctx, &vm, vmroguev1alpha1.VMPhaseFailed, fmt.Sprintf("config-drive secret: %v", err))
				return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
			}
		}
	}

	specHash, err := catalog.SpecHash(resolvedSpec)
	if err != nil {
		logger.Error(err, "failed to hash resolved spec")
	}

	// Convert VMRogueVM spec to KubeVirt VirtualMachine
	desired, err := converter.VMRogueVMToKubeVirt(workVM)
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
		r.applyKubeVirtAnnotations(desired, specHash)

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
		// Update existing KubeVirt VM spec only when it changed
		existingSpec, _, _ := unstructured.NestedMap(existing.Object, "spec")
		desiredSpec, _, _ := unstructured.NestedMap(desired.Object, "spec")

		specChanged := false
		if existingSpec != nil && desiredSpec != nil {
			existingBytes, errExisting := json.Marshal(existingSpec)
			desiredBytes, errDesired := json.Marshal(desiredSpec)
			if errExisting == nil && errDesired == nil && !bytes.Equal(existingBytes, desiredBytes) {
				specChanged = true
				existing.Object["spec"] = desiredSpec
			}
		}
		annChanged := r.applyKubeVirtAnnotations(existing, specHash)
		if specChanged || annChanged {
			if err := r.Update(ctx, existing); err != nil {
				logger.Error(err, "failed to update KubeVirt VM")
				return ctrl.Result{}, err
			}
			r.publishEvent(eventbus.SubjectVMUpdated, &vm, "Updated")
		}

		drift, driftMsg := catalog.CompareKubeVirtSpec(specHash, existing)
		if drift {
			vm.Status.DriftDetected = true
			vm.Status.DriftMessage = driftMsg
			_ = r.ensureDriftInsight(ctx, &vm, driftMsg)
		} else {
			vm.Status.DriftDetected = false
			vm.Status.DriftMessage = ""
		}
	}

	if err := r.reconcileInternetEgress(ctx, &vm); err != nil {
		logger.Error(err, "failed to reconcile internet egress policy")
		r.Recorder.Eventf(&vm, "Warning", "InternetPolicyFailed", "Internet egress policy: %v", err)
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
	if specHash != "" {
		vm.Status.ResolvedSpecHash = specHash
	}

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

func (r *VMRogueVMReconciler) applyKubeVirtAnnotations(kvVM *unstructured.Unstructured, specHash string) bool {
	ann := kvVM.GetAnnotations()
	if ann == nil {
		ann = map[string]string{}
	}
	changed := false
	if specHash != "" && ann["vmrogue.io/resolved-spec-hash"] != specHash {
		ann["vmrogue.io/resolved-spec-hash"] = specHash
		changed = true
	}
	if domainHash, err := catalog.KubeVirtDomainHash(kvVM); err == nil && domainHash != "" {
		if ann["vmrogue.io/kubevirt-domain-hash"] != domainHash {
			ann["vmrogue.io/kubevirt-domain-hash"] = domainHash
			changed = true
		}
	}
	if changed || kvVM.GetAnnotations() == nil {
		kvVM.SetAnnotations(ann)
	}
	return changed
}

func (r *VMRogueVMReconciler) reconcileInternetEgress(ctx context.Context, vm *vmroguev1alpha1.VMRogueVM) error {
	if network.AllowInternetEnabled(&vm.Spec) {
		_, err := network.EnsureVmInternetEgress(ctx, r.Client, vm.Namespace, vm.Name)
		return err
	}
	return network.RemoveVmInternetEgress(ctx, r.Client, vm.Namespace, vm.Name)
}

func (r *VMRogueVMReconciler) ensureConfigDriveSecret(ctx context.Context, vm *vmroguev1alpha1.VMRogueVM) error {
	name := converter.ConfigDriveSecretName(vm.Name)
	sec := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: vm.Namespace,
		},
	}
	_, err := controllerutil.CreateOrUpdate(ctx, r.Client, sec, func() error {
		if sec.StringData == nil {
			sec.StringData = make(map[string]string)
		}
		sec.StringData["userdata"] = vm.Spec.CloudInit.UserData
		if vm.Spec.CloudInit.UserData == "" && vm.Spec.CloudInit.UserDataSecretRef != nil {
			data, err := catalog.ResolveCloudInitUserData(ctx, r.Client, vm.Namespace, &vm.Spec)
			if err != nil {
				return err
			}
			sec.StringData["userdata"] = data
		}
		sec.Type = corev1.SecretTypeOpaque
		if sec.Labels == nil {
			sec.Labels = map[string]string{}
		}
		// Distinct from API-created Secrets (`vmrogue.io/managed-by=vmrogue`) so the HTTP API’s
		// VM delete path never removes operator-owned objects by mistake.
		sec.Labels["vmrogue.io/managed-by"] = "veyron-operator"
		sec.Labels["vmrogue.io/configdrive-userdata"] = "true"
		return controllerutil.SetControllerReference(vm, sec, r.Scheme)
	})
	return err
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

		if vm.Spec.CloudInit != nil {
			d := strings.ToLower(strings.TrimSpace(vm.Spec.CloudInit.Delivery))
			if d == "configdrive" || d == "config_drive" {
				sec := &corev1.Secret{
					ObjectMeta: metav1.ObjectMeta{
						Name:      converter.ConfigDriveSecretName(vm.Name),
						Namespace: vm.Namespace,
					},
				}
				if err := r.Delete(ctx, sec); err != nil && !errors.IsNotFound(err) {
					logger.Error(err, "failed to delete config-drive Secret")
					return ctrl.Result{}, err
				}
			}
		}

		if err := network.RemoveVmInternetEgress(ctx, r.Client, vm.Namespace, vm.Name); err != nil {
			logger.Error(err, "failed to remove internet egress policy")
			return ctrl.Result{}, err
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

func (r *VMRogueVMReconciler) ensureDriftInsight(ctx context.Context, vm *vmroguev1alpha1.VMRogueVM, message string) error {
	name := fmt.Sprintf("drift-%s", vm.Name)
	insight := &vmroguev1alpha1.VMRogueInsight{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: vm.Namespace,
			Labels: map[string]string{
				"vmrogue.io/vm":    vm.Name,
				"vmrogue.io/type":  "drift",
				"vmrogue.io/managed-by": "veyron-operator",
			},
		},
		Spec: vmroguev1alpha1.VMRogueInsightSpec{
			InsightType: "Drift",
			Severity:    "High",
			VMRef:       vm.Name,
			Namespace:   vm.Namespace,
			Title:       fmt.Sprintf("Configuration drift: %s", vm.Name),
			Description: message,
			Source:      "operator",
			Data: map[string]string{
				"resolvedSpecHash": vm.Status.ResolvedSpecHash,
			},
		},
	}
	existing := &vmroguev1alpha1.VMRogueInsight{}
	err := r.Get(ctx, types.NamespacedName{Name: name, Namespace: vm.Namespace}, existing)
	if errors.IsNotFound(err) {
		return r.Create(ctx, insight)
	}
	if err != nil {
		return err
	}
	existing.Spec.Description = message
	existing.Spec.Data["resolvedSpecHash"] = vm.Status.ResolvedSpecHash
	return r.Update(ctx, existing)
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
		Watches(kvVMI, &EnqueueVmiForVMRogueVM{}).
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
