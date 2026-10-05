// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VeyronBlueprintSpec defines a multi-VM deployment with dependency ordering.
// Mirrors the Rust Blueprint from src/blueprints/mod.rs.
type VeyronBlueprintSpec struct {
	// Human-readable description of this blueprint.
	// +optional
	Description string `json:"description,omitempty"`

	// VM definitions with dependency ordering.
	VMs []BlueprintVMSpec `json:"vms"`

	// Tags for categorization and search.
	// +optional
	Tags []string `json:"tags,omitempty"`
}

// BlueprintVMSpec defines a single VM within a blueprint.
// Mirrors the Rust VMSpec from src/blueprints/mod.rs.
type BlueprintVMSpec struct {
	// VM name within the blueprint (used as suffix for the VeyronVM CR name).
	Name string `json:"name"`

	// OS template name (e.g. "ubuntu-22.04").
	Template string `json:"template"`

	// Resource profile name (e.g. "minimal", "dev", "prod").
	// +optional
	Profile *string `json:"profile,omitempty"`

	// CPU cores override.
	// +optional
	CPU *uint32 `json:"cpu,omitempty"`

	// Memory size override (e.g. "4Gi").
	// +optional
	Memory *string `json:"memory,omitempty"`

	// Root disk size override (e.g. "20Gi").
	// +optional
	DiskSize *string `json:"diskSize,omitempty"`

	// Names of VMs in this blueprint that must be Running before this VM starts.
	// +optional
	DependsOn []string `json:"dependsOn,omitempty"`

	// Labels to apply to the created VeyronVM.
	// +optional
	Labels map[string]string `json:"labels,omitempty"`

	// Full VeyronVM spec override for advanced configuration.
	// +optional
	Override *VeyronVMSpec `json:"override,omitempty"`
}

// BlueprintPhase describes the deployment phase of a blueprint.
// +kubebuilder:validation:Enum=Pending;Deploying;Ready;Failed;Deleting
type BlueprintPhase string

const (
	BlueprintPhasePending   BlueprintPhase = "Pending"
	BlueprintPhaseDeploying BlueprintPhase = "Deploying"
	BlueprintPhaseReady     BlueprintPhase = "Ready"
	BlueprintPhaseFailed    BlueprintPhase = "Failed"
	BlueprintPhaseDeleting  BlueprintPhase = "Deleting"
)

// VMDeploymentStatus tracks the status of a single VM within a blueprint.
type VMDeploymentStatus struct {
	// VM name within the blueprint.
	Name string `json:"name"`

	// Current phase of the VM.
	Phase VeyronVMPhase `json:"phase"`

	// Name of the created VeyronVM CR.
	// +optional
	VMRef string `json:"vmRef,omitempty"`

	// Human-readable status message.
	// +optional
	Message string `json:"message,omitempty"`
}

// VeyronBlueprintStatus defines the observed state of VeyronBlueprint.
type VeyronBlueprintStatus struct {
	// Overall deployment phase.
	// +optional
	Phase BlueprintPhase `json:"phase,omitempty"`

	// Resolved deployment order (topological sort of VM names).
	// +optional
	DeploymentOrder []string `json:"deploymentOrder,omitempty"`

	// Per-VM deployment status.
	// +optional
	VMStatuses []VMDeploymentStatus `json:"vmStatuses,omitempty"`

	// Total number of VMs in the blueprint.
	// +optional
	TotalVMs int `json:"totalVMs,omitempty"`

	// Number of VMs in Running phase.
	// +optional
	ReadyVMs int `json:"readyVMs,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`

	// Generation observed by the controller.
	// +optional
	ObservedGeneration int64 `json:"observedGeneration,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Phase",type=string,JSONPath=`.status.phase`
// +kubebuilder:printcolumn:name="VMs",type=integer,JSONPath=`.status.totalVMs`
// +kubebuilder:printcolumn:name="Ready",type=integer,JSONPath=`.status.readyVMs`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:shortName=vrbp

// VeyronBlueprint is the Schema for the veyronblueprints API.
type VeyronBlueprint struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VeyronBlueprintSpec   `json:"spec,omitempty"`
	Status VeyronBlueprintStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VeyronBlueprintList contains a list of VeyronBlueprint.
type VeyronBlueprintList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VeyronBlueprint `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VeyronBlueprint{}, &VeyronBlueprintList{})
}
