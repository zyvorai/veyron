package eventbus

import (
	"encoding/json"
	"time"

	"github.com/google/uuid"
)

// Event is the shared envelope for all VMRogue events.
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
	SubjectVMCreated  = "vmrogue.vm.created"
	SubjectVMUpdated  = "vmrogue.vm.updated"
	SubjectVMDeleted  = "vmrogue.vm.deleted"
	SubjectVMStarted  = "vmrogue.vm.started"
	SubjectVMStopped  = "vmrogue.vm.stopped"

	SubjectBlueprintDeploying = "vmrogue.blueprint.deploying"
	SubjectBlueprintReady     = "vmrogue.blueprint.ready"
	SubjectBlueprintFailed    = "vmrogue.blueprint.failed"

	SubjectPolicyViolation = "vmrogue.policy.violation"

	SubjectInsightCreated = "vmrogue.insight.created"

	SubjectActionProposed = "vmrogue.action.proposed"
	SubjectActionExecuted = "vmrogue.action.executed"
	SubjectActionFailed   = "vmrogue.action.failed"
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
