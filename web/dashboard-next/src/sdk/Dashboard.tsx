import React, { useEffect, useMemo, useState } from "react";
import { useVmrogueMetricsFeed } from "../hooks/useVmrogueMetricsFeed";
import { AlertsList } from "./components/AlertsList";
import { ChartContainer } from "./components/ChartContainer";
import { Footer } from "./components/Footer";
import { HyperPageTabs } from "./components/HyperPageTabs";
import { Hero } from "./components/Hero";
import { JobsTable } from "./components/JobsTable";
import {
  ManifestPlaceholder,
  PlaceholderExportForm,
  WorkflowPlaceholder,
} from "./components/PlaceholderPanels";
import { QuickLinks } from "./components/QuickLinks";
import { StatCard } from "./components/StatCard";
import { useMetricsHistory } from "./hooks/useMetricsHistory";
import { cancelJob } from "./utils/api";
import { formatBytes, formatDuration, getStatusColor } from "./utils/formatters";

export const Dashboard: React.FC = () => {
  const { data: metrics, connected, reconnecting, error, transport } = useVmrogueMetricsFeed(4000);
  const { history, addMetrics } = useMetricsHistory(60); // Keep last 60 data points
  const [showJobForm, setShowJobForm] = useState(false);

  useEffect(() => {
    if (metrics) {
      addMetrics(metrics);
    }
  }, [metrics, addMetrics]);

  const handleCancelJob = async (jobId: string) => {
    try {
      await cancelJob(jobId);
    } catch (err) {
      console.error('Failed to cancel job:', err);
      alert('Failed to cancel job');
    }
  };

  // Job submission is now handled by VSphereExportWorkflow component

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
      description: "Open the Clusters & VMs view to browse KubeVirt VirtualMachines",
      icon: "●",
      href: "#inventory",
      onClick: () => {
        window.dispatchEvent(new CustomEvent("vmrogue:nav", { detail: { view: "inventory" } }));
      },
    },
    {
      title: "Request stop",
      description: "Use the table below — Stop maps to POST /api/v1/vms/{ns}/{name}/stop",
      icon: "◈",
      href: "#vms",
      onClick: () => {},
    },
    {
      title: "Alerts & events",
      description: "Warning events from the cluster are surfaced as alerts",
      icon: "◷",
      href: "#alerts",
      onClick: () => {},
    },
    {
      title: "Namespaces",
      description: "Treat each namespace as a provider scope in the Clusters & VMs view",
      icon: "▣",
      href: "#namespaces",
      onClick: () => {
        window.dispatchEvent(new CustomEvent("vmrogue:nav", { detail: { view: "inventory" } }));
      },
    },
  ];

  return (
    <div style={{ backgroundColor: '#f0f2f7', minHeight: '100vh' }}>
      <HyperPageTabs />

      <div id="dashboard">
      <Hero
        title="KubeVirt fleet operations"
        subtitle="Live inventory, health, and signals from the VMRogue API — same dashboard layout as HyperSDK dashboard-react."
        onNewJob={() => setShowJobForm(true)}
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

      {/* Job Submission Form */}
      {showJobForm && (
        <div style={{
          backgroundColor: '#f0f2f7',
          padding: '24px 16px',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <div style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: '24px',
            }}>
              <h2 style={{
                margin: 0,
                fontSize: '18px',
                fontWeight: '600',
                color: '#000',
              }}>
                New export job
              </h2>
              <button
                onClick={() => setShowJobForm(false)}
                style={{
                  padding: '8px 16px',
                  backgroundColor: 'transparent',
                  color: '#222324',
                  border: '2px solid #222324',
                  borderRadius: '4px',
                  fontSize: '12px',
                  fontWeight: '600',
                  cursor: 'pointer',
                  transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = '#f0583a';
                  e.currentTarget.style.color = '#f0583a';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = '#222324';
                  e.currentTarget.style.color = '#222324';
                }}
              >
                Close
              </button>
            </div>
            <PlaceholderExportForm />
          </div>
        </div>
      )}

      {/* Alerts */}
      {metrics?.alerts && metrics.alerts.length > 0 && (
        <div style={{
          backgroundColor: '#f0f2f7',
          padding: '24px',
        }}>
          <div style={{
            maxWidth: '1400px',
            margin: '0 auto',
          }}>
            <AlertsList alerts={metrics.alerts} />
          </div>
        </div>
      )}

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

      {/* Workflow Daemon Integration */}
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
          <WorkflowPlaceholder />
        </div>
      </div>

      {/* Manifest Builder */}
      <div style={{
        backgroundColor: '#f0f2f7',
        padding: '24px 16px',
      }}>
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
        }}>
          <ManifestPlaceholder
            onSubmitSuccess={(jobId) => {
              console.log("Manifest demo callback:", jobId);
            }}
          />
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

      <Footer />
    </div>
  );
};

export default Dashboard;
