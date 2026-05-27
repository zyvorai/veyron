// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VMTemplateSpec defines a canonical OS/platform template for VM creation.
// The Default field holds device, firmware, and cloud-init defaults (no instance state).
type VMTemplateSpec struct {
	// Human-readable description.
	// +optional
	Description string `json:"description,omitempty"`

	// Tags for search and categorization.
	// +optional
	Tags []string `json:"tags,omitempty"`

	// OS family: windows, linux, or other.
	// +optional
	// +kubebuilder:validation:Enum=windows;linux;other
	Family string `json:"family,omitempty"`

	// Recommended profile names for this template.
	// +optional
	RecommendedProfiles []string `json:"recommendedProfiles,omitempty"`

	// Minimum supported KubeVirt version (semver string).
	// +optional
	MinKubeVirtVersion string `json:"minKubeVirtVersion,omitempty"`

	// Default VM spec fields applied when template name is referenced.
	Default VMRogueVMSpec `json:"default"`
}

// VMTemplateStatus defines the observed state of VMTemplate.
type VMTemplateStatus struct {
	// Generation observed by the controller.
	// +optional
	ObservedGeneration int64 `json:"observedGeneration,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Family",type=string,JSONPath=`.spec.family`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:scope=Cluster,shortName=vmtpl

// VMTemplate is the Schema for the vmtemplates API.
type VMTemplate struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VMTemplateSpec   `json:"spec,omitempty"`
	Status VMTemplateStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VMTemplateList contains a list of VMTemplate.
type VMTemplateList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VMTemplate `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VMTemplate{}, &VMTemplateList{})
}
