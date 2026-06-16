// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package eventbus

import (
	"encoding/json"
	"time"

	"github.com/google/uuid"
)

// Event is the shared envelope for all Veyron events.
// Both the Go operator and Rust API use this same JSON structure.
type Event struct {
	// Unique event identifier.
	ID string `json:"id"`

	// Event type (e.g. "vm.created", "blueprint.ready").
	Type string `json:"type"`

	// Source of the event ("operator" or "api").
	Source string `json:"source"`

	// RFC3339 timestamp.
	Timestamp string `json:"timestamp"`

	// NATS subject this event was published on.
	Subject string `json:"subject"`

	// Event-specific payload.
	Data json.RawMessage `json:"data"`
}

// Event type constants (NATS subjects).
const (
	SubjectVMCreated  = "veyron.vm.created"
	SubjectVMUpdated  = "veyron.vm.updated"
	SubjectVMDeleted  = "veyron.vm.deleted"
	SubjectVMStarted  = "veyron.vm.started"
	SubjectVMStopped  = "veyron.vm.stopped"

	SubjectBlueprintDeploying = "veyron.blueprint.deploying"
	SubjectBlueprintReady     = "veyron.blueprint.ready"
	SubjectBlueprintFailed    = "veyron.blueprint.failed"

	SubjectPolicyViolation = "veyron.policy.violation"

	SubjectInsightCreated = "veyron.insight.created"

	SubjectActionProposed = "veyron.action.proposed"
	SubjectActionExecuted = "veyron.action.executed"
	SubjectActionFailed   = "veyron.action.failed"
)

// VMEventData is the payload for VM lifecycle events.
type VMEventData struct {
	Name      string `json:"name"`
	Namespace string `json:"namespace"`
	Phase     string `json:"phase"`
	NodeName  string `json:"nodeName,omitempty"`
	IPAddress string `json:"ipAddress,omitempty"`
}

// BlueprintEventData is the payload for blueprint events.
type BlueprintEventData struct {
	Name      string `json:"name"`
	Namespace string `json:"namespace"`
	Phase     string `json:"phase"`
	TotalVMs  int    `json:"totalVMs"`
	ReadyVMs  int    `json:"readyVMs"`
}

// PolicyViolationData is the payload for policy violation events.
type PolicyViolationData struct {
	PolicyName string `json:"policyName"`
	VMName     string `json:"vmName"`
	Namespace  string `json:"namespace"`
	RuleName   string `json:"ruleName"`
	Message    string `json:"message"`
}

// ActionEventData is the payload for action events.
type ActionEventData struct {
	Name       string `json:"name"`
	Namespace  string `json:"namespace"`
	ActionType string `json:"actionType"`
	VMRef      string `json:"vmRef,omitempty"`
	Phase      string `json:"phase"`
	Message    string `json:"message,omitempty"`
}

// NewEvent creates a new Event with a generated ID and current timestamp.
func NewEvent(eventType, source, subject string, data interface{}) (*Event, error) {
	dataBytes, err := json.Marshal(data)
	if err != nil {
		return nil, err
	}

	return &Event{
		ID:        uuid.New().String(),
		Type:      eventType,
		Source:    source,
		Timestamp: time.Now().UTC().Format(time.RFC3339),
		Subject:   subject,
		Data:      dataBytes,
	}, nil
}
