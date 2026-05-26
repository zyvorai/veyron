// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import React, { useCallback, useEffect, useState } from "react";
import {
  disableRdpViaGuestAgent,
  enableRdpViaGuestAgent,
  fetchRdpExpose,
  fetchVmDetail,
  putRdpExpose,
  type RdpExposeStatus,
  type RdpGuestAgentResult,
} from "../../lib/api";

interface Props {
  namespace: string;
  vmName: string;
  vmRunning?: boolean;
}

function guestAgentConnected(
  conditions?: Array<{ type?: string; type_?: string; status?: string }>
): boolean {
  return (
    conditions?.some(
      (c) =>
        (c.type === "AgentConnected" || c.type_ === "AgentConnected") && c.status === "True"
    ) ?? false
  );
}

const VmRdpPanel: React.FC<Props> = ({ namespace, vmName, vmRunning }) => {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<RdpExposeStatus | null>(null);
  const [agentUp, setAgentUp] = useState(false);
  const [running, setRunning] = useState(Boolean(vmRunning));
  const [nodePort, setNodePort] = useState(30100);
  const [guestMsg, setGuestMsg] = useState<string | null>(null);
  const [guestBusy, setGuestBusy] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [rdp, vm] = await Promise.all([
        fetchRdpExpose(namespace, vmName),
        fetchVmDetail(namespace, vmName),
      ]);
      setStatus(rdp);
      setNodePort(rdp.suggested_node_port ?? 30100);
      setAgentUp(guestAgentConnected(vm.vmi_status?.conditions));
      const phase = vm.vmi_status?.phase ?? vm.status;
      setRunning(
        vmRunning ??
          (phase?.toLowerCase() === "running" || vm.status?.toLowerCase() === "running")
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load RDP status");
    } finally {
      setLoading(false);
    }
  }, [namespace, vmName, vmRunning]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const canGuestAgent = running && agentUp;

  const runGuest = async (enable: boolean) => {
    setGuestBusy(true);
    setGuestMsg(null);
    setError(null);
    try {
      const out: RdpGuestAgentResult = enable
        ? await enableRdpViaGuestAgent(namespace, vmName)
        : await disableRdpViaGuestAgent(namespace, vmName);
      setGuestMsg(out.message);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Guest-agent RDP action failed");
    } finally {
      setGuestBusy(false);
    }
  };

  const expose = async () => {
    if (nodePort < 30000 || nodePort > 32767) {
      setError("NodePort must be between 30000 and 32767");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const rdp = await putRdpExpose(namespace, vmName, {
        enabled: true,
        service_type: "NodePort",
        node_port: nodePort,
      });
      setStatus(rdp);
      setGuestMsg("RDP NodePort applied");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to expose RDP");
    } finally {
      setLoading(false);
    }
  };

  const removeExpose = async () => {
    setLoading(true);
    setError(null);
    try {
      const rdp = await putRdpExpose(namespace, vmName, { enabled: false });
      setStatus(rdp);
      setGuestMsg("RDP exposure removed");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to remove RDP expose");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={styles.wrap}>
      <button type="button" style={styles.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} Remote access (RDP)
      </button>
      {open && (
        <div style={styles.panel}>
          {loading && <p style={styles.muted}>Loading…</p>}
          {error && <p style={styles.err}>{error}</p>}
          {status && (
            <>
              {status.is_windows_vm === false && (
                <p style={styles.note}>
                  VM is not detected as Windows; RDP still works if port 3389 listens in the guest.
                </p>
              )}
              <p>
                <strong>Guest agent:</strong> {agentUp ? "Connected" : "Not connected"}
              </p>
              {status.exposed ? (
                <>
                  <p>
                    <strong>NodePort:</strong> {status.node_port ?? "—"}
                  </p>
                  {status.rdp_via_nodeport_example && (
                    <p style={styles.mono}>{status.rdp_via_nodeport_example}</p>
                  )}
                  <p>
                    <strong>Service:</strong> {status.service_name}
                  </p>
                </>
              ) : (
                <p>No RDP NodePort Service (virt-launcher selector, guest TCP 3389).</p>
              )}
              {status.suggested_node_port != null && (
                <p style={styles.muted}>
                  Suggested NodePort: {status.suggested_node_port} (range 30100–30199)
                </p>
              )}
            </>
          )}
          <div style={styles.row}>
            <label style={styles.label}>
              NodePort
              <input
                type="number"
                min={30000}
                max={32767}
                value={nodePort}
                onChange={(e) => setNodePort(parseInt(e.target.value, 10) || 30100)}
                style={styles.input}
              />
            </label>
            <button type="button" style={styles.btnPrimary} disabled={loading} onClick={() => void expose()}>
              Expose RDP
            </button>
            <button type="button" style={styles.btnDanger} disabled={loading} onClick={() => void removeExpose()}>
              Remove
            </button>
          </div>
          <div style={styles.row}>
            <button
              type="button"
              style={styles.btnSuccess}
              disabled={!canGuestAgent || guestBusy}
              title={
                canGuestAgent
                  ? undefined
                  : "VM must be Running with guest agent connected"
              }
              onClick={() => void runGuest(true)}
            >
              Enable RDP in guest (agent)
            </button>
            <button
              type="button"
              style={styles.btnWarn}
              disabled={!canGuestAgent || guestBusy}
              title={
                canGuestAgent
                  ? undefined
                  : "VM must be Running with guest agent connected"
              }
              onClick={() => void runGuest(false)}
            >
              Disable RDP in guest (agent)
            </button>
          </div>
          {guestMsg && <p style={styles.ok}>{guestMsg}</p>}
          <p style={styles.hint}>
            Enable RDP inside Windows first, then expose NodePort. Requires Windows
            Pro/Enterprise/Server and cluster guest-exec support.
          </p>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  wrap: { marginTop: 12, borderTop: "1px solid #eee", paddingTop: 12 },
  toggle: {
    background: "none",
    border: "none",
    cursor: "pointer",
    fontWeight: 600,
    fontSize: 14,
    padding: 0,
    color: "#222",
  },
  panel: { marginTop: 10, fontSize: 13, color: "#333" },
  row: { display: "flex", flexWrap: "wrap", gap: 8, alignItems: "flex-end", marginTop: 10 },
  label: { display: "flex", flexDirection: "column", gap: 4, fontSize: 12 },
  input: { width: 100, padding: 6, border: "1px solid #ddd", borderRadius: 4 },
  btnPrimary: {
    padding: "8px 12px",
    background: "#f0583a",
    color: "#fff",
    border: "none",
    borderRadius: 4,
    cursor: "pointer",
  },
  btnDanger: {
    padding: "8px 12px",
    background: "#f44336",
    color: "#fff",
    border: "none",
    borderRadius: 4,
    cursor: "pointer",
  },
  btnSuccess: {
    padding: "8px 12px",
    background: "rgba(34,197,94,.15)",
    color: "#15803d",
    border: "1px solid rgba(34,197,94,.35)",
    borderRadius: 4,
    cursor: "pointer",
  },
  btnWarn: {
    padding: "8px 12px",
    background: "rgba(244,63,94,.12)",
    color: "#be123c",
    border: "1px solid rgba(244,63,94,.28)",
    borderRadius: 4,
    cursor: "pointer",
  },
  muted: { color: "#888", margin: "4px 0" },
  note: { color: "#666", fontSize: 12, margin: "4px 0" },
  mono: { fontFamily: "monospace", fontSize: 12, wordBreak: "break-all" },
  hint: { fontSize: 12, color: "#666", marginTop: 8 },
  err: { color: "#c62828", margin: "4px 0" },
  ok: { color: "#15803d", margin: "4px 0" },
};

export default VmRdpPanel;
