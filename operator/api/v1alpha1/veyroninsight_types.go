// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VeyronInsightSpec defines observability data for a VM or cluster resource.
// Designed as the output target for future eBPF agents and metrics analysis.
type VeyronInsightSpec struct {
	// Type of insight.
	// +kubebuilder:validation:Enum=Performance;Security;Cost;Anomaly;Capacity;Network;Drift
	InsightType string `json:"insightType"`

	// Severity level.
	// +kubebuilder:validation:Enum=Critical;High;Medium;Low;Info
	Severity string `json:"severity"`

	// Reference to the VeyronVM this insight relates to.
	// +optional
	VMRef string `json:"vmRef,omitempty"`

	// Namespace of the related resource.
	// +optional
	Namespace string `json:"namespace,omitempty"`

	// Short title summarizing the insight.
	Title string `json:"title"`

	// Detailed description of the observation.
	Description string `json:"description"`

	// Source of this insight (e.g. "ebpf", "metrics", "api", "operator").
	// +optional
	Source string `json:"source,omitempty"`

	// Free-form key-value observation data.
	// +optional
	Data map[string]string `json:"data,omitempty"`

	// Metric measurements associated with this insight.
	// +optional
	Metrics []InsightMetric `json:"metrics,omitempty"`
}

// InsightMetric is a single metric measurement within an insight.
type InsightMetric struct {
	// Metric name (e.g. "cpu_usage_percent").
	Name string `json:"name"`

	// Metric value.
	Value float64 `json:"value"`

	// Unit of measurement (e.g. "percent", "bytes", "ms").
	// +optional
	Unit string `json:"unit,omitempty"`

	// When this metric was sampled.
	// +optional
	Timestamp metav1.Time `json:"timestamp,omitempty"`
}

// InsightState describes the lifecycle state of an insight.
// +kubebuilder:validation:Enum=New;Acknowledged;Resolved;Dismissed
type InsightState string

const (
	InsightStateNew          InsightState = "New"
	InsightStateAcknowledged InsightState = "Acknowledged"
	InsightStateResolved     InsightState = "Resolved"
	InsightStateDismissed    InsightState = "Dismissed"
)

// VeyronInsightStatus defines the observed state of VeyronInsight.
type VeyronInsightStatus struct {
	// Current lifecycle state of this insight.
	// +optional
	State InsightState `json:"state,omitempty"`

	// Reference to a VeyronAction that was proposed in response to this insight.
	// +optional
	ActionRef string `json:"actionRef,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Type",type=string,JSONPath=`.spec.insightType`
// +kubebuilder:printcolumn:name="Severity",type=string,JSONPath=`.spec.severity`
// +kubebuilder:printcolumn:name="VM",type=string,JSONPath=`.spec.vmRef`
// +kubebuilder:printcolumn:name="State",type=string,JSONPath=`.status.state`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:shortName=vrin

// VeyronInsight is the Schema for the veyroninsights API.
type VeyronInsight struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VeyronInsightSpec   `json:"spec,omitempty"`
	Status VeyronInsightStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VeyronInsightList contains a list of VeyronInsight.
type VeyronInsightList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VeyronInsight `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VeyronInsight{}, &VeyronInsightList{})
}
