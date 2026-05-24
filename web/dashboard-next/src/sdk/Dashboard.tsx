import React, { useEffect, useMemo } from "react";
import { requestVmrogueNav } from "../lib/nav";
import { useVmrogueMetricsFeed } from "../hooks/useVmrogueMetricsFeed";
import { AlertsList } from "./components/AlertsList";
import { ChartContainer } from "./components/ChartContainer";
import { HyperPageTabs } from "./components/HyperPageTabs";
import { Hero } from "./components/Hero";
import { JobsTable } from "./components/JobsTable";
import {
  VmrogueInventoryPanel,
  VmroguePlatformPanel,
} from "./components/PlaceholderPanels";
import { QuickLinks } from "./components/QuickLinks";
import { StatCard } from "./components/StatCard";
import { useMetricsHistory } from "./hooks/useMetricsHistory";
import { cancelJob } from "./utils/api";
import { formatBytes, formatDuration, getStatusColor } from "./utils/formatters";

export const Dashboard: React.FC = () => {
  const { data: metrics, connected, reconnecting, error, transport } = useVmrogueMetricsFeed(4000);
  const { history, addMetrics } = useMetricsHistory(60);

  useEffect(() => {
    if (metrics) {
      addMetrics(metrics);
    }
  }, [metrics, addMetrics]);

  const openInventory = () => requestVmrogueNav({ view: "inventory" });

  const handleCancelJob = async (jobId: string) => {
    try {
      await cancelJob(jobId);
    } catch (err) {
      console.error("Failed to stop VM:", err);
      alert("Failed to stop VM");
    }
  };

  const jobsChartData = useMemo(() => {
    return history.map((m) => ({
      timestamp: m.timestamp,
      Running: m.jobs_active,
      Stopped: m.jobs_completed,
      Failed: m.jobs_failed,
      Pending: m.jobs_pending,
    }));
  }, [history]);

  const resourceChartData = useMemo(() => {
    return history.map((m) => ({
      timestamp: m.timestamp,
      "Memory (MB)": Math.round(m.memory_usage / 1024 / 1024),
      "CPU (%)": m.cpu_usage,
      Routines: m.goroutines,
    }));
  }, [history]);

  const providerChartData = useMemo(() => {
    if (!metrics?.provider_stats) return [];

    return Object.entries(metrics.provider_stats).map(([name, stats]) => ({
      name,
      value: stats.jobs_total,
      completed: stats.jobs_completed,
      failed: stats.jobs_failed,
    }));
  }, [metrics?.provider_stats]);

  // Show dashboard even if WebSocket fails - just display a warning banner
  const hasConnectionIssue = error || (!metrics && !connected && !reconnecting);

  const quickLinks = [
    {
      title: "VM inventory",
      description: "Browse KubeVirt VirtualMachines and run lifecycle actions",
      icon: "●",
      href: "#inventory",
      onClick: openInventory,
    },
    {
      title: "Stop a VM",
      description: "Open inventory — Stop maps to POST /api/v1/vms/{ns}/{name}/stop",
      icon: "◈",
      href: "#jobs",
      onClick: openInventory,
    },
    {
      title: "Alerts & events",
      description: "Warning events from the cluster surfaced as alerts",
      icon: "◷",
      href: "#alerts",
      onClick: () => requestVmrogueNav({ view: "dashboard", scrollTo: "alerts" }),
    },
    {
      title: "Cluster nodes",
      description: "Read-only node capacity, roles, and readiness from GET /api/v1/nodes",
      icon: "⬡",
      href: "#nodes",
      onClick: () => requestVmrogueNav({ view: "nodes" }),
    },
    {
      title: "Storage",
      description: "PVCs and storage classes from GET /api/v1/storage/*",
      icon: "▣",
      href: "#storage",
      onClick: () => requestVmrogueNav({ view: "storage" }),
    },
    {
      title: "Platform (GitOps + CRDs)",
      description: "GitOps sync status and installed CRD inventory",
      icon: "◫",
      href: "#platform",
      onClick: () => requestVmrogueNav({ view: "platform" }),
    },
    {
      title: "Workloads (pods)",
      description: "Pod phase, node placement, and restart counts",
      icon: "◎",
      href: "#workloads",
      onClick: () => requestVmrogueNav({ view: "workloads" }),
    },
    {
      title: "Insights",
      description: "Monitoring stack, security posture, costs, and recent events",
      icon: "◷",
      href: "#insights",
      onClick: () => requestVmrogueNav({ view: "insights" }),
    },
    {
      title: "Full dashboard",
      description: "VNC, snapshots, nodes, GitOps, and platform pages",
      icon: "▣",
      href: "/dashboard",
      onClick: () => {
        window.location.href = "/dashboard";
      },
    },
  ];

  return (
    <div style={{ backgroundColor: '#f0f2f7', minHeight: '100vh' }}>
      <HyperPageTabs />

      <div id="dashboard">
      <Hero
        title="KubeVirt fleet operations"
        subtitle="Live inventory, health, and cluster signals from the VMRogue API."
        primaryActionLabel="Open Clusters & VMs"
        onPrimaryAction={openInventory}
      />
      </div>

      {/* Connection Status Bar */}
      {hasConnectionIssue && (
        <div style={{
          backgroundColor: '#fee2e2',
          borderBottom: '2px solid #ef4444',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
            padding: '16px 24px',
            textAlign: 'center',
          }}>
            <div style={{
              fontSize: '14px',
              fontWeight: '600',
              color: '#991b1b',
            }}>
              ⚠️ Metrics unavailable (REST fallback failing). Check API reachability and credentials.
            </div>
            {error && (
              <div style={{
                fontSize: '12px',
                color: '#991b1b',
                marginTop: '4px',
                opacity: 0.8,
              }}>
                Error: {error.message}
              </div>
            )}
          </div>
        </div>
      )}

      {connected && transport === "poll" && metrics && (
        <div
          style={{
            backgroundColor: "#fffbeb",
            borderBottom: "2px solid #f59e0b",
          }}
        >
          <div
            style={{
              maxWidth: "1400px",
              margin: "0 auto",
              padding: "12px 24px",
              fontSize: "13px",
              color: "#92400e",
              lineHeight: 1.45,
            }}
          >
            REST poll mode: CPU and memory charts use estimated values derived from VM counts, not live Prometheus metrics.
            Open <strong>Insights</strong> for monitoring stack status or use the classic dashboard for full platform analytics.
          </div>
        </div>
      )}

      {connected && (
        <div style={{
          backgroundColor: '#10b98120',
          borderBottom: '2px solid #10b981',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
            padding: '12px 24px',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span
                style={{
                  width: '10px',
                  height: '10px',
                  borderRadius: '50%',
                  backgroundColor: '#10b981',
                }}
              />
              <span style={{
                fontSize: '14px',
                fontWeight: '600',
                color: '#10b981',
              }}>
                Connected to VMRogue
              </span>
            </div>
            {metrics && (
              <div style={{ fontSize: '14px', color: '#6b7280', fontWeight: '500' }}>
                Transport: {transport === "ws" ? "WebSocket" : transport === "poll" ? "REST poll" : "—"} · Uptime:{" "}
                {formatDuration(metrics.uptime_seconds)}
              </div>
            )}
          </div>
        </div>
      )}

      <QuickLinks links={quickLinks} />

      {/* Alerts */}
      <div
        id="alerts"
        style={{
          backgroundColor: '#f0f2f7',
          padding: '24px',
        }}
      >
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
        }}>
          {metrics?.alerts && metrics.alerts.length > 0 ? (
            <AlertsList alerts={metrics.alerts} />
          ) : (
            <p style={{ margin: 0, fontSize: '13px', color: '#6b7280' }}>
              No warning alerts in the current namespace scope.
            </p>
          )}
        </div>
      </div>

      {/* Stats Grid - Show demo data if no metrics */}
      {(metrics || hasConnectionIssue) && (
        <div style={{
          backgroundColor: '#f0f2f7',
          padding: '24px 16px',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <h2 style={{
              margin: '0 0 10px 0',
              fontSize: '18px',
              fontWeight: '600',
              color: '#000',
            }}>
              Cluster overview
            </h2>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))',
                gap: '8px',
                marginBottom: '24px',
              }}
            >
          <StatCard
            title="Running VMs"
            value={metrics?.jobs_active ?? 0}
            icon="●"
            color="#f0583a"
          />
          <StatCard
            title="Stopped / completed"
            value={metrics?.jobs_completed ?? 0}
            icon="✓"
            color="#10b981"
          />
          <StatCard
            title="Failed or error VMs"
            value={metrics?.jobs_failed ?? 0}
            icon="✕"
            color="#ef4444"
          />
          <StatCard
            title="Total VMs"
            value={metrics?.queue_length ?? 0}
            subtitle={`${metrics?.jobs_pending ?? 0} other / unknown`}
            icon="◷"
            color="#222324"
          />
          <StatCard
            title="Gateway memory (RSS proxy)"
            value={metrics ? formatBytes(metrics.memory_usage) : '0 B'}
            icon="▣"
            color="#222324"
          />
          <StatCard
            title="Active metric subscribers"
            value={metrics?.websocket_clients ?? 0}
            icon="◈"
            color="#f0583a"
          />
            </div>
          </div>
        </div>
      )}

      {/* Charts Row 1 */}
      <div style={{
        backgroundColor: '#f0f2f7',
        padding: '48px 24px',
      }}>
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
        }}>
          <h2 style={{
            margin: '0 0 20px 0',
            fontSize: '18px',
            fontWeight: '600',
            color: '#000',
          }}>
            Inventory & gateway signals
          </h2>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))',
              gap: '16px',
              marginBottom: '24px',
            }}
          >
        <ChartContainer
          title="VM states over time"
          type="line"
          data={jobsChartData}
          dataKeys={['Running', 'Stopped', 'Failed', 'Pending']}
          colors={['#3b82f6', '#10b981', '#ef4444', '#f59e0b']}
        />
        <ChartContainer
          title="Gateway footprint"
          type="line"
          data={resourceChartData}
          dataKeys={['Memory (MB)', 'CPU (%)', 'Routines']}
          colors={['#8b5cf6', '#ec4899', '#14b8a6']}
        />
          </div>
        </div>
      </div>

      {/* Charts Row 2 */}
      {providerChartData.length > 0 && (
        <div style={{
          backgroundColor: '#f0f2f7',
          padding: '24px 16px',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <ChartContainer
              title="VMs by namespace"
              type="pie"
              data={providerChartData}
            />
          </div>
        </div>
      )}

      {/* Jobs Table */}
      {metrics?.recent_jobs && (
        <div
          id="jobs"
          style={{
          backgroundColor: '#f0f2f7',
          padding: '24px 16px',
        }}
        >
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <h2 style={{
              margin: '0 0 10px 0',
              fontSize: '20px',
              fontWeight: '600',
              color: '#000',
            }}>
              Virtual machines
            </h2>
            <JobsTable jobs={metrics.recent_jobs} onCancelJob={handleCancelJob} />
          </div>
        </div>
      )}

      {/* Inventory CTA */}
      <div
        id="manage"
        style={{
        backgroundColor: '#f0f2f7',
        padding: '24px 16px',
      }}
      >
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
        }}>
          <VmrogueInventoryPanel onOpenInventory={openInventory} />
        </div>
      </div>

      {/* Platform links */}
      <div style={{
        backgroundColor: '#f0f2f7',
        padding: '24px 16px',
      }}>
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
        }}>
          <VmroguePlatformPanel />
        </div>
      </div>

      {/* System Info */}
      {metrics && (
        <div style={{
          backgroundColor: '#f0f2f7',
          padding: '24px 16px',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <h2 style={{
              margin: '0 0 10px 0',
              fontSize: '18px',
              fontWeight: '600',
              color: '#000',
            }}>
              System health
            </h2>
            <div
              style={{
                backgroundColor: '#fff',
                borderRadius: '3px',
                padding: '12px',
                border: '1px solid #e0e0e0',
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))',
                gap: '12px',
              }}
            >
          <div>
            <div style={{ fontSize: '9px', color: '#6b7280', marginBottom: '3px' }}>
              system health
            </div>
            <div style={{ fontSize: '12px', fontWeight: '600', color: getStatusColor(metrics.system_health) }}>
              {metrics.system_health}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '9px', color: '#6b7280', marginBottom: '3px' }}>
              HTTP Requests
            </div>
            <div style={{ fontSize: '12px', fontWeight: '600' }}>
              {metrics.http_requests.toLocaleString()}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '9px', color: '#6b7280', marginBottom: '3px' }}>
              HTTP Errors
            </div>
            <div style={{ fontSize: '12px', fontWeight: '600', color: metrics.http_errors > 0 ? '#ef4444' : '#10b981' }}>
              {metrics.http_errors.toLocaleString()}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '9px', color: '#6b7280', marginBottom: '3px' }}>
              Avg Response Time
            </div>
            <div style={{ fontSize: '12px', fontWeight: '600' }}>
              {metrics.avg_response_time.toFixed(2)}ms
            </div>
          </div>
            </div>
          </div>
        </div>
      )}

    </div>
  );
};

export default Dashboard;
