// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VMProfileSpec defines a resource preset (CPU, memory, disk).
type VMProfileSpec struct {
	// Human-readable description.
	// +optional
	Description string `json:"description,omitempty"`

	// CPU cores.
	Cores uint32 `json:"cores"`

	// CPU sockets.
	// +optional
	// +kubebuilder:default=1
	Sockets uint32 `json:"sockets,omitempty"`

	// Threads per core.
	// +optional
	// +kubebuilder:default=1
	Threads uint32 `json:"threads,omitempty"`

	// Memory size (e.g. "8Gi").
	Memory string `json:"memory"`

	// Root disk size (e.g. "40Gi").
	DiskSize string `json:"diskSize"`

	// GPUs granted by this profile (KubeVirt domain.devices.gpus). A VM using
	// a GPU profile can never live-migrate while it holds a passthrough GPU.
	// The VM's own explicit gpus win over the profile's.
	// +optional
	GPUs []GPUSpec `json:"gpus,omitempty"`

	// Use cases for this profile.
	// +optional
	UseCases []string `json:"useCases,omitempty"`

	// Recommended template names.
	// +optional
	RecommendedTemplates []string `json:"recommendedTemplates,omitempty"`
}

// VMProfileStatus defines the observed state of VMProfile.
type VMProfileStatus struct {
	// +optional
	ObservedGeneration int64 `json:"observedGeneration,omitempty"`
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="CPU",type=integer,JSONPath=`.spec.cores`
// +kubebuilder:printcolumn:name="Memory",type=string,JSONPath=`.spec.memory`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:scope=Cluster,shortName=vmprof

// VMProfile is the Schema for the vmprofiles API.
type VMProfile struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VMProfileSpec   `json:"spec,omitempty"`
	Status VMProfileStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VMProfileList contains a list of VMProfile.
type VMProfileList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VMProfile `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VMProfile{}, &VMProfileList{})
}
