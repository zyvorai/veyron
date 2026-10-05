// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VeyronActionSpec defines an autopilot action proposal or execution.
// Actions can be proposed by the system (insights, automation rules) and
// require approval before execution unless auto-approve is enabled.
type VeyronActionSpec struct {
	// Type of action to perform.
	// +kubebuilder:validation:Enum=StartVM;StopVM;RestartVM;ScaleResources;CreateSnapshot;DeleteSnapshot;CreateBackup;DeleteVM;Migrate;SendNotification;RunScript;Webhook
	ActionType string `json:"actionType"`

	// Reference to the target VeyronVM.
	// +optional
	VMRef string `json:"vmRef,omitempty"`

	// Namespace of the target resource.
	// +optional
	Namespace string `json:"namespace,omitempty"`

	// Reference to the VeyronInsight that triggered this action.
	// +optional
	InsightRef string `json:"insightRef,omitempty"`

	// Human-readable description of why this action is proposed.
	// +optional
	Description string `json:"description,omitempty"`

	// Whether this action is approved for execution (autopilot gate).
	Approved bool `json:"approved"`

	// Skip the approval gate and execute immediately.
	// +optional
	AutoApprove bool `json:"autoApprove,omitempty"`

	// Type-specific parameters.
	// For ScaleResources: {"cpu": "4", "memory": "8Gi"}
	// For Migrate: {"targetNode": "node-2"}
	// For Webhook: {"url": "https://...", "payload": "..."}
	// +optional
	Parameters map[string]string `json:"parameters,omitempty"`
}

// ActionPhase describes the execution phase of an action.
// +kubebuilder:validation:Enum=Pending;Approved;Executing;Completed;Failed;Rejected
type ActionPhase string

const (
	ActionPhasePending   ActionPhase = "Pending"
	ActionPhaseApproved  ActionPhase = "Approved"
	ActionPhaseExecuting ActionPhase = "Executing"
	ActionPhaseCompleted ActionPhase = "Completed"
	ActionPhaseFailed    ActionPhase = "Failed"
	ActionPhaseRejected  ActionPhase = "Rejected"
)

// VeyronActionStatus defines the observed state of VeyronAction.
type VeyronActionStatus struct {
	// Current execution phase.
	// +optional
	Phase ActionPhase `json:"phase,omitempty"`

	// When execution started.
	// +optional
	StartedAt *metav1.Time `json:"startedAt,omitempty"`

	// When execution completed (success or failure).
	// +optional
	CompletedAt *metav1.Time `json:"completedAt,omitempty"`

	// Human-readable result or error message.
	// +optional
	Message string `json:"message,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Action",type=string,JSONPath=`.spec.actionType`
// +kubebuilder:printcolumn:name="VM",type=string,JSONPath=`.spec.vmRef`
// +kubebuilder:printcolumn:name="Phase",type=string,JSONPath=`.status.phase`
// +kubebuilder:printcolumn:name="Approved",type=boolean,JSONPath=`.spec.approved`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:shortName=vract

// VeyronAction is the Schema for the veyronactions API.
type VeyronAction struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VeyronActionSpec   `json:"spec,omitempty"`
	Status VeyronActionStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VeyronActionList contains a list of VeyronAction.
type VeyronActionList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VeyronAction `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VeyronAction{}, &VeyronActionList{})
}
