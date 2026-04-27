package api

import (
	"context"
	"log"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
)

var metricsUpgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(_ *http.Request) bool {
		return true
	},
}

// metricsWebSocketHandler streams HyperSDK-shaped metrics JSON (wrapped) every few seconds.
func metricsWebSocketHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		conn, err := metricsUpgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Printf("metrics ws upgrade: %v", err)
			return
		}
		defer conn.Close()

		ctx := r.Context()
		send := func() error {
			ctx2, cancel := context.WithTimeout(ctx, 25*time.Second)
			defer cancel()
			raw := buildVmrogueMetricsSnapshot(ctx2)
			env := map[string]any{
				"type": "metrics",
				"data": map[string]any{"raw": raw},
			}
			_ = conn.SetWriteDeadline(time.Now().Add(30 * time.Second))
			return conn.WriteJSON(env)
		}

		if err := send(); err != nil {
			return
		}

		ticker := time.NewTicker(4 * time.Second)
		defer ticker.Stop()

		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				if err := send(); err != nil {
					return
				}
			}
		}
	}
}
