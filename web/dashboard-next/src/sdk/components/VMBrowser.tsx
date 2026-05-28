// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  deleteVirtualMachine,
  fetchVmInventory,
  startVirtualMachine,
  stopVirtualMachine,
} from "../../lib/api";
import AddDataDiskPanel from "./AddDataDiskPanel";
import VmConsoleModal from "./VmConsoleModal";
import { VmCreateModal } from "./VmCreateModal";
import VmSerialModal from "./VmSerialModal";
import { VmInternetPanel, VmMultusPanel, VmSshExposePanel } from "./VmNetworkPanels";
import VmRdpPanel from "./VmRdpPanel";
import { VmLifecycleBar } from "./VmLifecycleBar";
import { VmSnapshotsPanel } from "./VmSnapshotsPanel";
import { VmSnapshotSchedulesPanel } from "./VmSnapshotSchedulesPanel";
import VmDriftPanel from "./VmDriftPanel";

interface VM {
  id: string;
  name: string;
  provider: string;
  status: string;
  power_state?: string;
  cpu?: string;
  memory?: string;
  cpu_count?: number;
  memory_mb?: number;
  os?: string;
  disk_gb?: number;
  ip_address?: string;
  datacenter?: string;
  cluster?: string;
  tags?: string[];
  vmrogue_managed?: boolean | null;
  drift_detected?: boolean | null;
  drift_message?: string | null;
}

interface VMBrowserProps {
  provider: string;
  /** Scope VM inventory to one namespace, or `"all"`. */
  inventoryNamespace?: string;
  onVMSelect?: (vm: VM) => void;
  autoDiscoverOnMount?: boolean;
}

const VMBrowser: React.FC<VMBrowserProps> = ({
  provider,
  inventoryNamespace = "all",
  onVMSelect,
  autoDiscoverOnMount = true
}) => {
  const [vms, setVms] = useState<VM[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [filterDrift, setFilterDrift] = useState<'all' | 'drift' | 'clear' | 'unmanaged'>('all');
  const [selectedVM, setSelectedVM] = useState<VM | null>(null);
  const [sortField, setSortField] = useState<keyof VM>('name');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');
  const [consoleVm, setConsoleVm] = useState<{ ns: string; name: string } | null>(null);
  const [serialVm, setSerialVm] = useState<{ ns: string; name: string } | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [bulkBusy, setBulkBusy] = useState<string | null>(null);
  const [bulkError, setBulkError] = useState<string | null>(null);

  const splitVmId = useCallback((id: string): { ns: string; name: string } | null => {
    const slash = id.indexOf("/");
    if (slash < 1) return null;
    return { ns: id.slice(0, slash), name: id.slice(slash + 1) };
  }, []);

  const discoverVMs = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const rows = await fetchVmInventory(inventoryNamespace);
      setVms(
        rows.map((vm) => ({
          id: `${vm.namespace}/${vm.name}`,
          name: `${vm.namespace}/${vm.name}`,
          provider: provider || "kubevirt",
          status: vm.status,
          power_state: vm.status,
          cpu: vm.cpu,
          memory: vm.memory,
          ip_address: vm.ip,
          tags: [vm.node, vm.cpu, vm.memory].filter(Boolean) as string[],
          vmrogue_managed: vm.vmrogue_managed ?? null,
          drift_detected: vm.drift_detected ?? null,
          drift_message: vm.drift_message ?? null,
        })),
      );
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error occurred");
      setVms([]);
    } finally {
      setLoading(false);
    }
  }, [provider, inventoryNamespace]);

  useEffect(() => {
    if (autoDiscoverOnMount && provider) {
      void discoverVMs();
    }
  }, [provider, autoDiscoverOnMount, discoverVMs, inventoryNamespace]);

  const handleVMClick = (vm: VM) => {
    setSelectedVM(vm);
    if (onVMSelect) {
      onVMSelect(vm);
    }
  };

  const handleSort = (field: keyof VM) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };

  // Filter and sort VMs
  const filteredVMs = vms
    .filter(vm => {
      const matchesSearch = vm.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                          vm.id.toLowerCase().includes(searchTerm.toLowerCase());
      const matchesStatus = filterStatus === 'all' ||
                          vm.power_state?.toLowerCase() === filterStatus.toLowerCase() ||
                          vm.status?.toLowerCase() === filterStatus.toLowerCase();
      const matchesDrift =
        filterDrift === 'all' ||
        (filterDrift === 'drift' && vm.drift_detected === true) ||
        (filterDrift === 'clear' && vm.vmrogue_managed === true && vm.drift_detected === false) ||
        (filterDrift === 'unmanaged' && vm.vmrogue_managed !== true);
      return matchesSearch && matchesStatus && matchesDrift;
    })
    .sort((a, b) => {
      const aVal = a[sortField];
      const bVal = b[sortField];

      if (aVal === undefined) return 1;
      if (bVal === undefined) return -1;

      const comparison = aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
      return sortDirection === 'asc' ? comparison : -comparison;
    });

  const driftCount = useMemo(
    () => vms.filter((vm) => vm.drift_detected === true).length,
    [vms],
  );

  const getDriftLabel = (vm: VM): string => {
    if (vm.vmrogue_managed !== true) return 'N/A';
    return vm.drift_detected ? 'Drift' : 'In sync';
  };

  const getDriftColor = (vm: VM): string => {
    if (vm.vmrogue_managed !== true) return '#9ca3af';
    return vm.drift_detected ? '#b45309' : '#15803d';
  };

  const getStatusColor = (status: string | undefined): string => {
    if (!status) return '#999';

    const s = status.toLowerCase();
    if (s.includes('running') || s.includes('poweredon')) return '#4caf50';
    if (s.includes('stopped') || s.includes('poweredoff')) return '#f44336';
    if (s.includes('suspended')) return '#ff9800';
    return '#2196f3';
  };

  const visibleIds = useMemo(() => filteredVMs.map((vm) => vm.id), [filteredVMs]);
  const allVisibleSelected =
    visibleIds.length > 0 && visibleIds.every((id) => selectedIds.has(id));

  const toggleSelect = (id: string, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
  };

  const toggleSelectAllVisible = (checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      for (const id of visibleIds) {
        if (checked) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  };

  const runBulkAction = async (action: 'start' | 'stop' | 'delete') => {
    if (selectedIds.size === 0) return;
    if (action === 'delete' && !confirm(`Delete ${selectedIds.size} VM(s)? This cannot be undone.`)) {
      return;
    }
    setBulkBusy(action);
    setBulkError(null);
    const errors: string[] = [];
    for (const id of selectedIds) {
      const parts = splitVmId(id);
      if (!parts) continue;
      try {
        if (action === 'delete') {
          await deleteVirtualMachine(parts.ns, parts.name);
          if (selectedVM?.id === id) setSelectedVM(null);
        } else if (action === 'start') {
          await startVirtualMachine(parts.ns, parts.name);
        } else {
          await stopVirtualMachine(parts.ns, parts.name);
        }
      } catch (err) {
        errors.push(`${id}: ${err instanceof Error ? err.message : 'failed'}`);
      }
    }
    setSelectedIds(new Set());
    setBulkBusy(null);
    if (errors.length) {
      setBulkError(errors.slice(0, 3).join(' · ') + (errors.length > 3 ? ' …' : ''));
    }
    void discoverVMs();
  };

  return (
    <div style={styles.container}>
      {/* Header */}
      <div style={styles.header}>
        <div style={styles.headerLeft}>
          <h2 style={styles.title}>KubeVirt virtual machines</h2>
          <p style={styles.subtitle}>
            {loading
              ? "Loading inventory…"
              : `${filteredVMs.length} of ${vms.length} VMs · namespace: ${
                  inventoryNamespace === "all" ? "all" : inventoryNamespace
                }${driftCount > 0 ? ` · ${driftCount} with drift` : ""}`}
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
          <button
            onClick={discoverVMs}
            disabled={loading}
            style={{
              ...styles.refreshButton,
              ...(loading ? styles.refreshButtonDisabled : {}),
            }}
          >
            {loading ? "🔄 Loading…" : "🔄 Refresh"}
          </button>
          <VmCreateModal defaultNamespace={inventoryNamespace} onCreated={() => void discoverVMs()} />
        </div>
      </div>

      {/* Filters */}
      <div style={styles.filters}>
        <input
          type="text"
          placeholder="Search VMs by name or ID..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          style={styles.searchInput}
        />
        <select
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value)}
          style={styles.filterSelect}
        >
          <option value="all">All Status</option>
          <option value="running">Running</option>
          <option value="poweredon">Powered On</option>
          <option value="stopped">Stopped</option>
          <option value="poweredoff">Powered Off</option>
          <option value="suspended">Suspended</option>
        </select>
        <select
          value={filterDrift}
          onChange={(e) => setFilterDrift(e.target.value as typeof filterDrift)}
          style={styles.filterSelect}
          aria-label="Filter by operator drift"
        >
          <option value="all">All drift states</option>
          <option value="drift">Drift detected</option>
          <option value="clear">Operator in sync</option>
          <option value="unmanaged">Not operator-managed</option>
        </select>
      </div>

      {/* Error Message */}
      {error && (
        <div style={styles.error}>
          <strong>⚠️ Error:</strong> {error}
        </div>
      )}
      {bulkError ? (
        <div style={styles.bulkError} role="alert">
          {bulkError}
        </div>
      ) : null}
      {selectedIds.size > 0 ? (
        <div style={styles.bulkBar}>
          <label style={styles.bulkSelectAll}>
            <input
              type="checkbox"
              checked={allVisibleSelected}
              onChange={(e) => toggleSelectAllVisible(e.target.checked)}
            />
            Select visible
          </label>
          <span style={styles.bulkCount}>{selectedIds.size} selected</span>
          <button
            type="button"
            disabled={bulkBusy !== null}
            style={styles.bulkBtnStart}
            onClick={() => void runBulkAction('start')}
          >
            {bulkBusy === 'start' ? 'Starting…' : 'Start'}
          </button>
          <button
            type="button"
            disabled={bulkBusy !== null}
            style={styles.bulkBtnStop}
            onClick={() => void runBulkAction('stop')}
          >
            {bulkBusy === 'stop' ? 'Stopping…' : 'Stop'}
          </button>
          <button
            type="button"
            disabled={bulkBusy !== null}
            style={styles.bulkBtnDelete}
            onClick={() => void runBulkAction('delete')}
          >
            {bulkBusy === 'delete' ? 'Deleting…' : 'Delete'}
          </button>
        </div>
      ) : null}

      {/* VM Grid */}
      {loading ? (
        <div style={styles.loading}>
          <div style={styles.spinner}></div>
          <p>Loading virtual machines from the cluster…</p>
        </div>
      ) : filteredVMs.length === 0 ? (
        <div style={styles.empty}>
          <p style={styles.emptyIcon}>📭</p>
          <p style={styles.emptyText}>
            {vms.length === 0
              ? 'No virtual machines found. Click "Refresh VMs" to discover.'
              : 'No VMs match your search criteria.'}
          </p>
        </div>
      ) : (
        <>
          {/* Table View */}
          <div style={styles.tableContainer}>
            <table style={styles.table}>
              <thead>
                <tr style={styles.tableHeaderRow}>
                  <th style={{ ...styles.tableHeader, width: 40, cursor: 'default' }}>
                    <input
                      type="checkbox"
                      aria-label="Select all visible VMs"
                      checked={allVisibleSelected}
                      onChange={(e) => toggleSelectAllVisible(e.target.checked)}
                    />
                  </th>
                  <th style={styles.tableHeader} onClick={() => handleSort('name')}>
                    Name {sortField === 'name' && (sortDirection === 'asc' ? '↑' : '↓')}
                  </th>
                  <th style={styles.tableHeader} onClick={() => handleSort('status')}>
                    Status {sortField === 'status' && (sortDirection === 'asc' ? '↑' : '↓')}
                  </th>
                  <th style={styles.tableHeader} onClick={() => handleSort('cpu_count')}>
                    CPU {sortField === 'cpu_count' && (sortDirection === 'asc' ? '↑' : '↓')}
                  </th>
                  <th style={styles.tableHeader} onClick={() => handleSort('memory_mb')}>
                    Memory {sortField === 'memory_mb' && (sortDirection === 'asc' ? '↑' : '↓')}
                  </th>
                  <th style={styles.tableHeader}>OS</th>
                  <th style={styles.tableHeader}>IP Address</th>
                  <th style={styles.tableHeader}>Drift</th>
                  <th style={styles.tableHeader}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredVMs.map((vm) => (
                  <tr
                    key={vm.id}
                    style={{
                      ...styles.tableRow,
                      ...(selectedVM?.id === vm.id ? styles.tableRowSelected : {})
                    }}
                    onClick={() => handleVMClick(vm)}
                  >
                    <td style={styles.tableCell} onClick={(e) => e.stopPropagation()}>
                      <input
                        type="checkbox"
                        aria-label={`Select ${vm.name}`}
                        checked={selectedIds.has(vm.id)}
                        onChange={(e) => toggleSelect(vm.id, e.target.checked)}
                      />
                    </td>
                    <td style={styles.tableCell}>
                      <div style={styles.vmNameCell}>
                        <strong>{vm.name}</strong>
                        <span style={styles.vmId}>{vm.id}</span>
                      </div>
                    </td>
                    <td style={styles.tableCell}>
                      <span style={{
                        ...styles.statusBadge,
                        backgroundColor: getStatusColor(vm.power_state || vm.status)
                      }}>
                        {vm.power_state || vm.status || 'Unknown'}
                      </span>
                    </td>
                    <td style={styles.tableCell}>
                      {vm.cpu_count ? `${vm.cpu_count} vCPU` : '-'}
                    </td>
                    <td style={styles.tableCell}>
                      {vm.memory_mb ? `${(vm.memory_mb / 1024).toFixed(1)} GB` : '-'}
                    </td>
                    <td style={styles.tableCell}>{vm.os || '-'}</td>
                    <td style={styles.tableCell}>{vm.ip_address || '-'}</td>
                    <td style={styles.tableCell}>
                      <span style={{
                        ...styles.statusBadge,
                        backgroundColor: getDriftColor(vm),
                        color: '#fff',
                      }}>
                        {getDriftLabel(vm)}
                      </span>
                    </td>
                    <td style={styles.tableCell}>
                      <button
                        style={styles.selectButton}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleVMClick(vm);
                        }}
                      >
                        Manage
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Grid View Alternative */}
          <div style={styles.gridContainer}>
            {filteredVMs.map((vm) => (
              <div
                key={vm.id}
                style={{
                  ...styles.vmCard,
                  ...(selectedVM?.id === vm.id ? styles.vmCardSelected : {})
                }}
                onClick={() => handleVMClick(vm)}
              >
                <div style={styles.vmCardHeader}>
                  <h3 style={styles.vmCardTitle}>{vm.name}</h3>
                  <span style={{
                    ...styles.statusBadge,
                    backgroundColor: getStatusColor(vm.power_state || vm.status)
                  }}>
                    {vm.power_state || vm.status}
                  </span>
                </div>
                <div style={styles.vmCardBody}>
                  <div style={styles.vmCardRow}>
                    <span style={styles.vmCardLabel}>ID:</span>
                    <span style={styles.vmCardValue}>{vm.id}</span>
                  </div>
                  {vm.cpu_count && (
                    <div style={styles.vmCardRow}>
                      <span style={styles.vmCardLabel}>CPU:</span>
                      <span style={styles.vmCardValue}>{vm.cpu_count} vCPU</span>
                    </div>
                  )}
                  {vm.memory_mb && (
                    <div style={styles.vmCardRow}>
                      <span style={styles.vmCardLabel}>Memory:</span>
                      <span style={styles.vmCardValue}>{(vm.memory_mb / 1024).toFixed(1)} GB</span>
                    </div>
                  )}
                  {vm.os && (
                    <div style={styles.vmCardRow}>
                      <span style={styles.vmCardLabel}>OS:</span>
                      <span style={styles.vmCardValue}>{vm.os}</span>
                    </div>
                  )}
                  {vm.ip_address && (
                    <div style={styles.vmCardRow}>
                      <span style={styles.vmCardLabel}>IP:</span>
                      <span style={styles.vmCardValue}>{vm.ip_address}</span>
                    </div>
                  )}
                </div>
                <button
                  style={styles.selectButton}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleVMClick(vm);
                  }}
                >
          Manage
                </button>
              </div>
            ))}
          </div>
        </>
      )}

      {/* Selected VM Info */}
      {selectedVM && (
        <div style={styles.selectedVMInfo}>
          <h3 style={styles.selectedVMTitle}>✓ Selected VM: {selectedVM.name}</h3>
          <div style={styles.selectedVMDetails}>
            <div><strong>ID:</strong> {selectedVM.id}</div>
            <div><strong>Provider:</strong> {selectedVM.provider}</div>
            <div><strong>Status:</strong> {selectedVM.power_state || selectedVM.status}</div>
            {selectedVM.datacenter && <div><strong>Datacenter:</strong> {selectedVM.datacenter}</div>}
            {selectedVM.cluster && <div><strong>Cluster:</strong> {selectedVM.cluster}</div>}
          </div>
          {(() => {
            const slash = selectedVM.id.indexOf("/");
            if (slash < 1) return null;
            const ns = selectedVM.id.slice(0, slash);
            const vmName = selectedVM.id.slice(slash + 1);
            const running =
              (selectedVM.power_state || selectedVM.status || "").toLowerCase() === "running";
            return (
              <>
                <VmLifecycleBar
                  namespace={ns}
                  vmName={vmName}
                  status={selectedVM.power_state || selectedVM.status}
                  cpu={selectedVM.cpu}
                  memory={selectedVM.memory}
                  onChanged={() => void discoverVMs()}
                  onDeleted={() => {
                    setSelectedVM(null);
                    void discoverVMs();
                  }}
                  onOpenConsole={() => setConsoleVm({ ns, name: vmName })}
                  onOpenSerial={() => setSerialVm({ ns, name: vmName })}
                />
                <VmDriftPanel
                  namespace={ns}
                  vmName={vmName}
                  initialDrift={selectedVM.drift_detected}
                  initialMessage={selectedVM.drift_message}
                />
                <VmSnapshotsPanel namespace={ns} vmName={vmName} />
                <VmSnapshotSchedulesPanel namespace={ns} vmName={vmName} />
                <VmSshExposePanel namespace={ns} vmName={vmName} />
                <VmInternetPanel namespace={ns} vmName={vmName} />
                <VmMultusPanel namespace={ns} vmName={vmName} />
                <VmRdpPanel namespace={ns} vmName={vmName} vmRunning={running} />
                <AddDataDiskPanel
                  namespace={ns}
                  vmName={vmName}
                  onAttached={() => void discoverVMs()}
                />
              </>
            );
          })()}
        </div>
      )}

      {consoleVm ? (
        <VmConsoleModal
          namespace={consoleVm.ns}
          vmName={consoleVm.name}
          onClose={() => setConsoleVm(null)}
        />
      ) : null}
      {serialVm ? (
        <VmSerialModal
          namespace={serialVm.ns}
          vmName={serialVm.name}
          onClose={() => setSerialVm(null)}
        />
      ) : null}
    </div>
  );
};

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    padding: '20px',
    backgroundColor: '#fff',
    borderRadius: '8px',
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '20px',
    borderBottom: '2px solid #f0f2f7',
    paddingBottom: '15px',
    flexWrap: 'wrap',
    gap: '12px',
  },
  headerLeft: {
    flex: 1,
  },
  title: {
    margin: 0,
    fontSize: '24px',
    color: '#222324',
  },
  subtitle: {
    margin: '5px 0 0 0',
    fontSize: '14px',
    color: '#666',
  },
  refreshButton: {
    padding: '10px 20px',
    backgroundColor: '#f0583a',
    color: '#fff',
    border: 'none',
    borderRadius: '5px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 'bold',
    transition: 'background-color 0.3s',
  },
  refreshButtonDisabled: {
    backgroundColor: '#ccc',
    cursor: 'not-allowed',
  },
  filters: {
    display: 'flex',
    gap: '10px',
    marginBottom: '20px',
  },
  searchInput: {
    flex: 1,
    padding: '10px',
    border: '1px solid #ddd',
    borderRadius: '5px',
    fontSize: '14px',
  },
  filterSelect: {
    padding: '10px',
    border: '1px solid #ddd',
    borderRadius: '5px',
    fontSize: '14px',
    minWidth: '150px',
  },
  error: {
    padding: '15px',
    backgroundColor: '#ffebee',
    border: '1px solid #f44336',
    borderRadius: '5px',
    color: '#c62828',
    marginBottom: '20px',
  },
  bulkError: {
    padding: '10px 14px',
    backgroundColor: '#fff7ed',
    border: '1px solid #fed7aa',
    borderRadius: '8px',
    color: '#c2410c',
    marginBottom: '12px',
    fontSize: '13px',
  },
  bulkBar: {
    display: 'flex',
    flexWrap: 'wrap' as const,
    alignItems: 'center',
    gap: '10px',
    padding: '10px 14px',
    marginBottom: '16px',
    backgroundColor: '#fff7ed',
    border: '1px solid rgba(240, 88, 58, 0.25)',
    borderRadius: '8px',
  },
  bulkSelectAll: {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    fontSize: '13px',
    color: '#374151',
  },
  bulkCount: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#222324',
    marginRight: '8px',
  },
  bulkBtnStart: {
    padding: '6px 12px',
    borderRadius: '6px',
    border: '1px solid rgba(34,197,94,.35)',
    background: 'rgba(34,197,94,.12)',
    color: '#15803d',
    fontWeight: 600,
    fontSize: '12px',
    cursor: 'pointer',
  },
  bulkBtnStop: {
    padding: '6px 12px',
    borderRadius: '6px',
    border: '1px solid rgba(244,67,54,.35)',
    background: 'rgba(244,67,54,.1)',
    color: '#b91c1c',
    fontWeight: 600,
    fontSize: '12px',
    cursor: 'pointer',
  },
  bulkBtnDelete: {
    padding: '6px 12px',
    borderRadius: '6px',
    border: '1px solid rgba(244,67,54,.45)',
    background: '#fef2f2',
    color: '#991b1b',
    fontWeight: 600,
    fontSize: '12px',
    cursor: 'pointer',
  },
  loading: {
    textAlign: 'center',
    padding: '40px',
    color: '#666',
  },
  spinner: {
    width: '40px',
    height: '40px',
    margin: '0 auto 20px',
    border: '4px solid #f0f2f7',
    borderTop: '4px solid #f0583a',
    borderRadius: '50%',
    animation: 'spin 1s linear infinite',
  },
  empty: {
    textAlign: 'center',
    padding: '40px',
    color: '#999',
  },
  emptyIcon: {
    fontSize: '48px',
    margin: '0 0 10px 0',
  },
  emptyText: {
    fontSize: '16px',
    margin: 0,
  },
  tableContainer: {
    overflowX: 'auto',
    marginBottom: '20px',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
  },
  tableHeaderRow: {
    backgroundColor: '#f0f2f7',
  },
  tableHeader: {
    padding: '12px',
    textAlign: 'left',
    fontWeight: 'bold',
    color: '#222324',
    cursor: 'pointer',
    userSelect: 'none',
  },
  tableRow: {
    borderBottom: '1px solid #eee',
    cursor: 'pointer',
    transition: 'background-color 0.2s',
  },
  tableRowSelected: {
    backgroundColor: '#fff3e0',
  },
  tableCell: {
    padding: '12px',
    color: '#333',
  },
  vmNameCell: {
    display: 'flex',
    flexDirection: 'column',
    gap: '4px',
  },
  vmId: {
    fontSize: '12px',
    color: '#999',
  },
  statusBadge: {
    display: 'inline-block',
    padding: '4px 8px',
    borderRadius: '12px',
    color: '#fff',
    fontSize: '12px',
    fontWeight: 'bold',
  },
  selectButton: {
    padding: '6px 12px',
    backgroundColor: '#f0583a',
    color: '#fff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
    fontWeight: 'bold',
    transition: 'background-color 0.3s',
  },
  gridContainer: {
    display: 'none', // Hidden by default, can toggle with table view
    gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))',
    gap: '20px',
  },
  vmCard: {
    padding: '15px',
    border: '1px solid #ddd',
    borderRadius: '8px',
    cursor: 'pointer',
    transition: 'all 0.3s',
    backgroundColor: '#fff',
  },
  vmCardSelected: {
    borderColor: '#f0583a',
    boxShadow: '0 0 0 2px rgba(240, 88, 58, 0.2)',
  },
  vmCardHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '15px',
  },
  vmCardTitle: {
    margin: 0,
    fontSize: '18px',
    color: '#222324',
  },
  vmCardBody: {
    marginBottom: '15px',
  },
  vmCardRow: {
    display: 'flex',
    justifyContent: 'space-between',
    padding: '5px 0',
    borderBottom: '1px solid #f0f2f7',
  },
  vmCardLabel: {
    fontWeight: 'bold',
    color: '#666',
    fontSize: '14px',
  },
  vmCardValue: {
    color: '#333',
    fontSize: '14px',
  },
  selectedVMInfo: {
    marginTop: '20px',
    padding: '15px',
    backgroundColor: '#e8f5e9',
    border: '1px solid #4caf50',
    borderRadius: '5px',
  },
  selectedVMTitle: {
    margin: '0 0 10px 0',
    color: '#2e7d32',
    fontSize: '16px',
  },
  selectedVMDetails: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
    gap: '10px',
    fontSize: '14px',
    color: '#333',
  },
};

export default VMBrowser;
