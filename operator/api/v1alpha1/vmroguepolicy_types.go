// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VMRoguePolicySpec defines declarative policy rules for VMRogueVMs.
type VMRoguePolicySpec struct {
	// Human-readable description.
	// +optional
	Description string `json:"description,omitempty"`

	// Whether this policy is actively enforced.
	Enabled bool `json:"enabled"`

	// Enforcement action when a rule is violated.
	// +kubebuilder:validation:Enum=Allow;Deny;Warn;Audit
	EnforcementAction string `json:"enforcementAction"`

	// Severity level of violations from this policy.
	// +kubebuilder:validation:Enum=Critical;High;Medium;Low;Info
	Severity string `json:"severity"`

	// Compliance framework this policy supports.
	// +optional
	// +kubebuilder:validation:Enum=SOC2;HIPAA;PCI-DSS;GDPR;ISO27001;NIST;CIS;FedRAMP;Custom
	Framework string `json:"framework,omitempty"`

	// Label selector to match VMRogueVMs this policy applies to.
	// If empty, applies to all VMs in the namespace.
	// +optional
	Selector *metav1.LabelSelector `json:"selector,omitempty"`

	// Policy rules to evaluate.
	Rules []PolicyRuleSpec `json:"rules"`
}

// PolicyRuleSpec defines a single policy rule evaluated against VMRogueVM specs.
type PolicyRuleSpec struct {
	// Rule name.
	Name string `json:"name"`

	// CEL expression evaluated against the VMRogueVM spec.
	// The expression has access to `spec` (VMRogueVMSpec) and `metadata` (ObjectMeta).
	// Must evaluate to a boolean. True means compliant.
	Condition string `json:"condition"`

	// Message shown when the rule is violated.
	Message string `json:"message"`
}

// PolicyViolation records a specific policy violation.
type PolicyViolation struct {
	// Name of the violating VM.
	VMName string `json:"vmName"`

	// Namespace of the violating VM.
	Namespace string `json:"namespace"`

	// Name of the violated rule.
	RuleName string `json:"ruleName"`

	// Human-readable violation message.
	Message string `json:"message"`

	// When the violation was detected.
	Timestamp metav1.Time `json:"timestamp"`
}

// VMRoguePolicyStatus defines the observed state of VMRoguePolicy.
type VMRoguePolicyStatus struct {
	// Number of VMRogueVMs matching this policy's selector.
	// +optional
	MatchingVMs int `json:"matchingVMs,omitempty"`

	// Number of matching VMs that are compliant.
	// +optional
	CompliantVMs int `json:"compliantVMs,omitempty"`

	// Number of matching VMs with violations.
	// +optional
	ViolatingVMs int `json:"violatingVMs,omitempty"`

	// List of current violations.
	// +optional
	Violations []PolicyViolation `json:"violations,omitempty"`

	// Standard Kubernetes conditions.
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`

	// Last time policies were evaluated.
	// +optional
	LastEvaluated *metav1.Time `json:"lastEvaluated,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Enforcement",type=string,JSONPath=`.spec.enforcementAction`
// +kubebuilder:printcolumn:name="Severity",type=string,JSONPath=`.spec.severity`
// +kubebuilder:printcolumn:name="Enabled",type=boolean,JSONPath=`.spec.enabled`
// +kubebuilder:printcolumn:name="Violations",type=integer,JSONPath=`.status.violatingVMs`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
// +kubebuilder:resource:shortName=vrpol

// VMRoguePolicy is the Schema for the vmroguepolicies API.
type VMRoguePolicy struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VMRoguePolicySpec   `json:"spec,omitempty"`
	Status VMRoguePolicyStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// VMRoguePolicyList contains a list of VMRoguePolicy.
type VMRoguePolicyList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VMRoguePolicy `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VMRoguePolicy{}, &VMRoguePolicyList{})
}
