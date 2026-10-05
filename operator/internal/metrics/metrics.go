// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package metrics

import (
	"github.com/prometheus/client_golang/prometheus"
	"sigs.k8s.io/controller-runtime/pkg/metrics"
)

var (
	// ReconcileTotal counts total reconciliation attempts by controller and result.
	ReconcileTotal = prometheus.NewCounterVec(
		prometheus.CounterOpts{
			Name: "veyron_reconcile_total",
			Help: "Total number of reconciliation attempts",
		},
		[]string{"controller", "result"},
	)

	// ReconcileDuration tracks reconciliation duration in seconds.
	ReconcileDuration = prometheus.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "veyron_reconcile_duration_seconds",
			Help:    "Duration of reconciliation in seconds",
			Buckets: prometheus.DefBuckets,
		},
		[]string{"controller"},
	)

	// VMCount tracks the current number of VeyronVM resources by phase.
	VMCount = prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "veyron_vm_count",
			Help: "Current number of VeyronVM resources by phase",
		},
		[]string{"phase"},
	)

	// BlueprintCount tracks the current number of VeyronBlueprint resources by phase.
	BlueprintCount = prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "veyron_blueprint_count",
			Help: "Current number of VeyronBlueprint resources by phase",
		},
		[]string{"phase"},
	)

	// PolicyViolations tracks the number of active policy violations.
	PolicyViolations = prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "veyron_policy_violations",
			Help: "Current number of policy violations",
		},
		[]string{"policy"},
	)

	// ActionExecutions counts action executions by type and result.
	ActionExecutions = prometheus.NewCounterVec(
		prometheus.CounterOpts{
			Name: "veyron_action_executions_total",
			Help: "Total number of action executions",
		},
		[]string{"action_type", "result"},
	)

	// InsightCount tracks the number of insights by type and severity.
	InsightCount = prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "veyron_insight_count",
			Help: "Current number of insights by type and severity",
		},
		[]string{"type", "severity"},
	)
)

func init() {
	metrics.Registry.MustRegister(
		ReconcileTotal,
		ReconcileDuration,
		VMCount,
		BlueprintCount,
		PolicyViolations,
		ActionExecutions,
		InsightCount,
	)
}
