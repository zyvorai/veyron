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

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
	"github.com/ssahani/Veyron/operator/internal/catalog"
	"github.com/ssahani/Veyron/operator/internal/converter"
	"github.com/ssahani/Veyron/operator/internal/eventbus"
	vmmetrics "github.com/ssahani/Veyron/operator/internal/metrics"
	"github.com/ssahani/Veyron/operator/internal/network"
)

const (
	vmFinalizer = "veyron.io/vm-finalizer"
)

// VeyronVMReconciler reconciles a VeyronVM object.
type VeyronVMReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	EventBus *eventbus.EventBus
}

// +kubebuilder:rbac:groups=veyron.io,resources=veyronvms,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=veyron.io,resources=veyronvms/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=veyron.io,resources=veyronvms/finalizers,verbs=update
// +kubebuilder:rbac:groups=kubevirt.io,resources=virtualmachines,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=kubevirt.io,resources=virtualmachineinstances,verbs=get;list;watch
// +kubebuilder:rbac:groups="",resources=events,verbs=create;patch
// +kubebuilder:rbac:groups="",resources=secrets,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=cilium.io,resources=ciliumnetworkpolicies,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=networking.k8s.io,resources=networkpolicies,verbs=get;list;watch;create;update;patch;delete

func (r *VeyronVMReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)
	reconcileStart := time.Now()
	defer func() {
		vmmetrics.ReconcileDuration.WithLabelValues("veyronvm").Observe(time.Since(reconcileStart).Seconds())
	}()

	// Fetch the VeyronVM
	var vm veyronv1alpha1.VeyronVM
	if err := r.Get(ctx, req.NamespacedName, &vm); err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		vmmetrics.ReconcileTotal.WithLabelValues("veyronvm", "error").Inc()
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
		r.updateStatus(ctx, &vm, veyronv1alpha1.VMPhaseFailed, fmt.Sprintf("catalog resolve: %v", err))
		return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
	}

	workVM := vm.DeepCopy()
	workVM.Spec = resolvedSpec
	if workVM.Spec.CloudInit != nil {
		userData, uErr := catalog.ResolveCloudInitUserData(ctx, r.Client, vm.Namespace, &workVM.Spec)
		if uErr != nil {
			logger.Error(uErr, "failed to resolve cloud-init userData")
			r.updateStatus(ctx, &vm, veyronv1alpha1.VMPhaseFailed, fmt.Sprintf("cloud-init: %v", uErr))
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
				r.updateStatus(ctx, &vm, veyronv1alpha1.VMPhaseFailed, fmt.Sprintf("config-drive secret: %v", err))
				return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
			}
		}
	}

	// Windows domain join -> sysprep answer file. Must run before the VM is created:
	// the converter mounts this Secret, and a missing one strands Setup at a prompt.
	if err := r.ensureSysprepSecret(ctx, workVM); err != nil {
		logger.Error(err, "failed to reconcile sysprep Secret")
		r.updateStatus(ctx, &vm, veyronv1alpha1.VMPhaseFailed, fmt.Sprintf("sysprep secret: %v", err))
		return ctrl.Result{RequeueAfter: 30 * time.Second}, nil
	}

	specHash, err := catalog.SpecHash(resolvedSpec)
	if err != nil {
		logger.Error(err, "failed to hash resolved spec")
	}

	// Convert VeyronVM spec to KubeVirt VirtualMachine
	desired, err := converter.VeyronVMToKubeVirt(workVM)
	if err != nil {
		logger.Error(err, "failed to convert VeyronVM to KubeVirt VM")
		r.updateStatus(ctx, &vm, veyronv1alpha1.VMPhaseFailed, fmt.Sprintf("conversion error: %v", err))
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

	// Drift is computed against the existing KubeVirt VM below, but the status
	// re-fetch before the single Status().Update() overwrites vm.Status — so
	// capture the drift result here and re-apply it after the re-fetch.
	driftDetected := false
	driftMessage := ""

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
			driftDetected = true
			driftMessage = driftMsg
			_ = r.ensureDriftInsight(ctx, &vm, driftMsg)
		}
	}

	if err := r.reconcileInternetEgress(ctx, &vm); err != nil {
		logger.Error(err, "failed to reconcile internet egress policy")
		r.Recorder.Eventf(&vm, "Warning", "InternetPolicyFailed", "Internet egress policy: %v", err)
	}

	// Re-fetch the VeyronVM to get the latest ResourceVersion before status update
	if err := r.Get(ctx, req.NamespacedName, &vm); err != nil {
		return ctrl.Result{}, err
	}

	// Read VMI status for runtime info
	phase, nodeName, ipAddress := r.readVMIStatus(ctx, vm.Namespace, vm.Name)

	// Update VeyronVM status (single status update with latest ResourceVersion)
	now := metav1.Now()
	vm.Status.KubevirtVMName = vm.Name
	vm.Status.Phase = phase
	vm.Status.NodeName = nodeName
	vm.Status.IPAddress = ipAddress
	vm.Status.LastReconciled = &now
	vm.Status.ObservedGeneration = vm.Generation
	vm.Status.DriftDetected = driftDetected
	vm.Status.DriftMessage = driftMessage
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
	if phase == veyronv1alpha1.VMPhaseRunning {
		condition.Status = metav1.ConditionTrue
		condition.Message = "VM is running"
	}
	setCondition(&vm.Status.Conditions, condition)

	if err := r.Status().Update(ctx, &vm); err != nil {
		logger.Error(err, "failed to update VeyronVM status")
		return ctrl.Result{}, err
	}

	// Requeue to poll VMI status until stable
	if phase != veyronv1alpha1.VMPhaseRunning && phase != veyronv1alpha1.VMPhaseStopped {
		return ctrl.Result{RequeueAfter: 15 * time.Second}, nil
	}

	return ctrl.Result{RequeueAfter: 60 * time.Second}, nil
}

func (r *VeyronVMReconciler) applyKubeVirtAnnotations(kvVM *unstructured.Unstructured, specHash string) bool {
	ann := kvVM.GetAnnotations()
	if ann == nil {
		ann = map[string]string{}
	}
	changed := false
	if specHash != "" && ann["veyron.io/resolved-spec-hash"] != specHash {
		ann["veyron.io/resolved-spec-hash"] = specHash
		changed = true
	}
	if domainHash, err := catalog.KubeVirtDomainHash(kvVM); err == nil && domainHash != "" {
		if ann["veyron.io/kubevirt-domain-hash"] != domainHash {
			ann["veyron.io/kubevirt-domain-hash"] = domainHash
			changed = true
		}
	}
	if changed || kvVM.GetAnnotations() == nil {
		kvVM.SetAnnotations(ann)
	}
	return changed
}

func (r *VeyronVMReconciler) reconcileInternetEgress(ctx context.Context, vm *veyronv1alpha1.VeyronVM) error {
	if network.AllowInternetEnabled(&vm.Spec) {
		_, err := network.EnsureVmInternetEgress(ctx, r.Client, vm.Namespace, vm.Name)
		return err
	}
	return network.RemoveVmInternetEgress(ctx, r.Client, vm.Namespace, vm.Name)
}

func (r *VeyronVMReconciler) ensureConfigDriveSecret(ctx context.Context, vm *veyronv1alpha1.VeyronVM) error {
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
		// Distinct from API-created Secrets (`veyron.io/managed-by=veyron`) so the HTTP API’s
		// VM delete path never removes operator-owned objects by mistake.
		sec.Labels["veyron.io/managed-by"] = "veyron-operator"
		sec.Labels["veyron.io/configdrive-userdata"] = "true"
		return controllerutil.SetControllerReference(vm, sec, r.Scheme)
	})
	return err
}

// ensureSysprepSecret materializes the answer file mounted as sysprep media when a
// domain join is requested.
//
// The join credential goes into a Windows unattend <Identification> element rather
// than a PowerShell block in cloud-init: userdata is echoed into the Cloudbase-Init
// log inside the guest, so the old path left domain credentials in plaintext on disk.
//
// When the caller also supplied their own sysprepSecretRef, their answer file is read
// and the join is injected into it — so both survive.
func (r *VeyronVMReconciler) ensureSysprepSecret(ctx context.Context, vm *veyronv1alpha1.VeyronVM) error {
	if vm.Spec.Windows == nil || vm.Spec.Windows.DomainJoinSecretRef == nil {
		return nil
	}

	creds, err := catalog.ResolveDomainJoinCredentials(
		ctx, r.Client, vm.Namespace, vm.Spec.Windows.DomainJoinSecretRef)
	if err != nil {
		return fmt.Errorf("windows domainJoinSecretRef: %w", err)
	}

	answerFile := catalog.BuildDomainJoinUnattend(creds)

	// Caller brought their own answer file — merge, never discard.
	if ref := vm.Spec.Windows.SysprepSecretRef; ref != nil {
		userXML, err := catalog.ReadSysprepAnswerFile(ctx, r.Client, vm.Namespace, ref)
		if err != nil {
			return fmt.Errorf("windows sysprepSecretRef: %w", err)
		}
		merged, err := catalog.MergeDomainJoinIntoUnattend(userXML, creds)
		if err != nil {
			return fmt.Errorf("merging domain join into sysprep answer file: %w", err)
		}
		answerFile = merged
	}

	sec := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name:      converter.SysprepSecretName(vm.Name),
			Namespace: vm.Namespace,
		},
	}
	_, err = controllerutil.CreateOrUpdate(ctx, r.Client, sec, func() error {
		if sec.StringData == nil {
			sec.StringData = make(map[string]string)
		}
		// KubeVirt mounts this key as the sysprep answer file.
		sec.StringData[catalog.SysprepAnswerFileKey] = answerFile
		sec.Type = corev1.SecretTypeOpaque
		if sec.Labels == nil {
			sec.Labels = map[string]string{}
		}
		sec.Labels["veyron.io/managed-by"] = "veyron-operator"
		sec.Labels["veyron.io/sysprep-unattend"] = "true"
		return controllerutil.SetControllerReference(vm, sec, r.Scheme)
	})
	return err
}

func (r *VeyronVMReconciler) handleDeletion(ctx context.Context, vm *veyronv1alpha1.VeyronVM) (ctrl.Result, error) {
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

func (r *VeyronVMReconciler) readVMIStatus(ctx context.Context, namespace, name string) (veyronv1alpha1.VeyronVMPhase, string, string) {
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
		return veyronv1alpha1.VMPhaseStopped, "", ""
	}
	if err != nil {
		return veyronv1alpha1.VMPhaseUnknown, "", ""
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

	var vmPhase veyronv1alpha1.VeyronVMPhase
	switch phase {
	case "Running":
		vmPhase = veyronv1alpha1.VMPhaseRunning
	case "Succeeded":
		vmPhase = veyronv1alpha1.VMPhaseStopped
	case "Failed":
		vmPhase = veyronv1alpha1.VMPhaseFailed
	case "Pending", "Scheduling", "Scheduled":
		vmPhase = veyronv1alpha1.VMPhaseCreating
	default:
		vmPhase = veyronv1alpha1.VMPhasePending
	}

	return vmPhase, nodeName, ipAddress
}

func (r *VeyronVMReconciler) ensureDriftInsight(ctx context.Context, vm *veyronv1alpha1.VeyronVM, message string) error {
	name := fmt.Sprintf("drift-%s", vm.Name)
	insight := &veyronv1alpha1.VeyronInsight{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: vm.Namespace,
			Labels: map[string]string{
				"veyron.io/vm":         vm.Name,
				"veyron.io/type":       "drift",
				"veyron.io/managed-by": "veyron-operator",
			},
		},
		Spec: veyronv1alpha1.VeyronInsightSpec{
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
	existing := &veyronv1alpha1.VeyronInsight{}
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

func (r *VeyronVMReconciler) updateStatus(ctx context.Context, vm *veyronv1alpha1.VeyronVM, phase veyronv1alpha1.VeyronVMPhase, message string) {
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

func (r *VeyronVMReconciler) publishEvent(subject string, vm *veyronv1alpha1.VeyronVM, phase string) {
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
func (r *VeyronVMReconciler) SetupWithManager(mgr ctrl.Manager) error {
	// Register the KubeVirt GVKs with the scheme so we can watch them
	kvVMGVK := schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachine"}
	kvVMIGVK := schema.GroupVersionKind{Group: "kubevirt.io", Version: "v1", Kind: "VirtualMachineInstance"}

	kvVM := &unstructured.Unstructured{}
	kvVM.SetGroupVersionKind(kvVMGVK)

	kvVMI := &unstructured.Unstructured{}
	kvVMI.SetGroupVersionKind(kvVMIGVK)

	return ctrl.NewControllerManagedBy(mgr).
		For(&veyronv1alpha1.VeyronVM{}).
		Owns(kvVM).
		Watches(kvVMI, &EnqueueVmiForVeyronVM{}).
		Complete(r)
}

// setCondition updates or adds a condition in the conditions slice.
func setCondition(conditions *[]metav1.Condition, condition metav1.Condition) {
	if conditions == nil {
		return
	}
	for i, existing := range *conditions {
		if existing.Type == condition.Type {
			// Per Kubernetes API conventions, LastTransitionTime must only change
			// when the Status transitions. Reconcile runs every 15-60s, so blindly
			// overwriting it would flap the timestamp on every tick. Preserve the
			// existing timestamp unless the Status actually changed.
			if existing.Status == condition.Status {
				condition.LastTransitionTime = existing.LastTransitionTime
			}
			(*conditions)[i] = condition
			return
		}
	}
	*conditions = append(*conditions, condition)
}
