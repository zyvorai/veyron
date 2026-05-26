// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package eventbus

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
)

// EventBus wraps a NATS connection for publishing and subscribing to VMRogue events.
type EventBus struct {
	conn *nats.Conn
	js   nats.JetStreamContext
}

// NewEventBus connects to NATS and initializes JetStream.
func NewEventBus(url string) (*EventBus, error) {
	opts := []nats.Option{
		nats.Name("vmrogue-operator"),
		nats.ReconnectWait(2 * time.Second),
		nats.MaxReconnects(60),
	}

	conn, err := nats.Connect(url, opts...)
	if err != nil {
		return nil, fmt.Errorf("connecting to NATS at %s: %w", url, err)
	}

	js, err := conn.JetStream()
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("initializing JetStream: %w", err)
	}

	// Ensure the VMROGUE stream exists.
	_, err = js.StreamInfo("VMROGUE")
	if err != nil {
		_, err = js.AddStream(&nats.StreamConfig{
			Name:     "VMROGUE",
			Subjects: []string{"vmrogue.>"},
			Storage:  nats.FileStorage,
			MaxAge:   7 * 24 * time.Hour, // 7-day retention
		})
		if err != nil {
			conn.Close()
			return nil, fmt.Errorf("creating VMROGUE stream: %w", err)
		}
	}

	return &EventBus{conn: conn, js: js}, nil
}

// Publish sends an event to the NATS event bus.
func (eb *EventBus) Publish(event *Event) error {
	if eb == nil || eb.conn == nil {
		return nil // Event bus not configured, silently skip.
	}

	data, err := json.Marshal(event)
	if err != nil {
		return fmt.Errorf("marshaling event: %w", err)
	}

	_, err = eb.js.Publish(event.Subject, data)
	if err != nil {
		return fmt.Errorf("publishing to %s: %w", event.Subject, err)
	}

	return nil
}

// Subscribe registers a handler for events matching the given subject pattern.
func (eb *EventBus) Subscribe(subject string, handler func(Event)) error {
	if eb == nil || eb.conn == nil {
		return nil
	}

	_, err := eb.js.Subscribe(subject, func(msg *nats.Msg) {
		var event Event
		if err := json.Unmarshal(msg.Data, &event); err != nil {
			return // Skip malformed events.
		}
		handler(event)
		msg.Ack()
	}, nats.Durable("vmrogue-operator"))
	if err != nil {
		return fmt.Errorf("subscribing to %s: %w", subject, err)
	}

	return nil
}

// Close shuts down the NATS connection.
func (eb *EventBus) Close() {
	if eb != nil && eb.conn != nil {
		eb.conn.Drain()
	}
}
