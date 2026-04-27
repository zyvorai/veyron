import React, { useState, useEffect, useCallback } from "react";
import { fetchNamespaces } from "../../lib/api";

/** One row per Kubernetes namespace visible to VMRogue (KubeVirt inventory scope). */
interface NamespaceRow {
  id: string;
  name: string;
  vmCount: number;
  clusterStatus: string;
  /** Last manual verify against `/api/v1/namespaces`. */
  apiReachable: boolean;
  lastChecked?: string;
  verifyError?: string;
}

interface ProviderManagerProps {
  /** Called when the user picks a namespace to scope the VM list (e.g. from “Browse VMs”). */
  onProviderSelect?: (namespace: string) => void;
}

const ProviderManager: React.FC<ProviderManagerProps> = ({ onProviderSelect }) => {
  const [rows, setRows] = useState<NamespaceRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);

  const loadNamespaces = useCallback(async () => {
    setLoading(true);
    try {
      const namespaces = await fetchNamespaces();
      setRows(
        namespaces.map((ns) => ({
          id: ns.name,
          name: ns.name,
          vmCount: ns.vmCount,
          clusterStatus: ns.status,
          apiReachable: true,
          lastChecked: new Date().toISOString(),
        })),
      );
    } catch (e) {
      console.error("Failed to load namespaces:", e);
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadNamespaces();
  }, [loadNamespaces]);

  const verifyNamespace = async (namespaceId: string) => {
    setTesting(namespaceId);
    try {
      const namespaces = await fetchNamespaces();
      const found = namespaces.find((n) => n.name === namespaceId);
      const ok = Boolean(found);
      setRows((prev) =>
        prev.map((r) =>
          r.id === namespaceId
            ? {
                ...r,
                apiReachable: ok,
                vmCount: found?.vmCount ?? r.vmCount,
                clusterStatus: found?.status ?? r.clusterStatus,
                verifyError: ok ? undefined : "Namespace not returned by VMRogue API",
                lastChecked: new Date().toISOString(),
              }
            : r,
        ),
      );
      return ok;
    } catch {
      setRows((prev) =>
        prev.map((r) =>
          r.id === namespaceId
            ? {
                ...r,
                apiReachable: false,
                verifyError: "Could not reach VMRogue API",
                lastChecked: new Date().toISOString(),
              }
            : r,
        ),
      );
      return false;
    } finally {
      setTesting(null);
    }
  };

  const handleSelectNamespace = (namespaceId: string) => {
    const row = rows.find((r) => r.id === namespaceId);
    if (row?.apiReachable && onProviderSelect) {
      onProviderSelect(namespaceId);
    }
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <div>
          <h2 style={styles.title}>Namespaces</h2>
          <p style={styles.subtitle}>
            Kubernetes namespaces from your cluster API. Use “Verify” to re-check access; “Browse VMs” scopes the VM
            list below to that namespace.
          </p>
        </div>
        <button type="button" onClick={() => void loadNamespaces()} disabled={loading} style={styles.refreshButton}>
          {loading ? "Loading…" : "↻ Refresh list"}
        </button>
      </div>

      <div style={styles.providerGrid}>
        {rows.map((row) => (
          <div
            key={row.id}
            style={{
              ...styles.providerCard,
              ...(row.apiReachable ? styles.providerCardConnected : styles.providerCardDisconnected),
            }}
            onClick={() => handleSelectNamespace(row.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectNamespace(row.id);
              }
            }}
            role="button"
            tabIndex={0}
          >
            <div style={styles.providerCardHeader}>
              <span style={styles.providerIcon}>🖥️</span>
              <div style={styles.providerCardInfo}>
                <h3 style={styles.providerName}>{row.name}</h3>
                <span
                  style={{
                    ...styles.statusBadge,
                    backgroundColor: row.apiReachable ? "#4caf50" : "#f44336",
                  }}
                >
                  {row.apiReachable ? "✓ API" : "✗ Unreachable"}
                </span>
              </div>
            </div>

            <p style={styles.metaLine}>
              {row.vmCount} VM{row.vmCount === 1 ? "" : "s"} · Phase {row.clusterStatus}
            </p>

            {row.verifyError && (
              <div style={styles.providerError}>
                ⚠️ {row.verifyError}
              </div>
            )}

            {row.lastChecked && (
              <div style={styles.providerLastChecked}>
                Last checked: {new Date(row.lastChecked).toLocaleString()}
              </div>
            )}

            <div style={styles.providerActions}>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  void verifyNamespace(row.id);
                }}
                disabled={testing === row.id}
                style={{
                  ...styles.testButton,
                  ...(testing === row.id ? styles.testButtonDisabled : {}),
                }}
              >
                {testing === row.id ? "…" : "🔌 Verify"}
              </button>
              {row.apiReachable && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleSelectNamespace(row.id);
                  }}
                  style={styles.browseButton}
                >
                  📂 Browse VMs
                </button>
              )}
            </div>
          </div>
        ))}

        {rows.length === 0 && !loading && (
          <div style={styles.emptyState}>
            <p style={styles.emptyIcon}>📭</p>
            <p style={styles.emptyText}>No namespaces returned</p>
            <p style={styles.emptySubtext}>Check RBAC, VMRogue API connectivity, and namespace list permissions.</p>
          </div>
        )}
      </div>
    </div>
  );
};

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    padding: "20px",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "flex-start",
    gap: "16px",
    marginBottom: "20px",
  },
  title: {
    margin: 0,
    fontSize: "24px",
    color: "#222324",
  },
  subtitle: {
    margin: "8px 0 0 0",
    fontSize: "13px",
    color: "#666",
    maxWidth: "720px",
    lineHeight: 1.45,
  },
  refreshButton: {
    padding: "10px 18px",
    backgroundColor: "#f0583a",
    color: "#fff",
    border: "none",
    borderRadius: "5px",
    cursor: "pointer",
    fontSize: "14px",
    fontWeight: "bold",
    flexShrink: 0,
  },
  providerGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
    gap: "20px",
  },
  providerCard: {
    padding: "20px",
    border: "2px solid",
    borderRadius: "8px",
    cursor: "pointer",
    transition: "all 0.3s",
    backgroundColor: "#fff",
  },
  providerCardConnected: {
    borderColor: "#4caf50",
  },
  providerCardDisconnected: {
    borderColor: "#f44336",
  },
  providerCardHeader: {
    display: "flex",
    alignItems: "center",
    marginBottom: "12px",
  },
  providerIcon: {
    fontSize: "36px",
    marginRight: "12px",
  },
  providerCardInfo: {
    flex: 1,
  },
  providerName: {
    margin: "0 0 5px 0",
    fontSize: "18px",
    color: "#222324",
  },
  metaLine: {
    margin: "0 0 10px 0",
    fontSize: "13px",
    color: "#555",
  },
  statusBadge: {
    display: "inline-block",
    padding: "4px 8px",
    borderRadius: "12px",
    color: "#fff",
    fontSize: "12px",
    fontWeight: "bold",
  },
  providerError: {
    padding: "10px",
    backgroundColor: "#ffebee",
    borderRadius: "4px",
    color: "#c62828",
    fontSize: "12px",
    marginBottom: "10px",
  },
  providerLastChecked: {
    fontSize: "12px",
    color: "#999",
    marginBottom: "10px",
  },
  providerActions: {
    display: "flex",
    gap: "10px",
  },
  testButton: {
    flex: 1,
    padding: "8px",
    backgroundColor: "#2196f3",
    color: "#fff",
    border: "none",
    borderRadius: "4px",
    cursor: "pointer",
    fontSize: "12px",
    fontWeight: "bold",
  },
  testButtonDisabled: {
    backgroundColor: "#ccc",
    cursor: "not-allowed",
  },
  browseButton: {
    flex: 1,
    padding: "8px",
    backgroundColor: "#4caf50",
    color: "#fff",
    border: "none",
    borderRadius: "4px",
    cursor: "pointer",
    fontSize: "12px",
    fontWeight: "bold",
  },
  emptyState: {
    gridColumn: "1 / -1",
    textAlign: "center",
    padding: "48px 20px",
    color: "#999",
  },
  emptyIcon: {
    fontSize: "48px",
    margin: "0 0 10px 0",
  },
  emptyText: {
    fontSize: "18px",
    margin: "0 0 5px 0",
  },
  emptySubtext: {
    fontSize: "14px",
    margin: 0,
  },
};

export default ProviderManager;
