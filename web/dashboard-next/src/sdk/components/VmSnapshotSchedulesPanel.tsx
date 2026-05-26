// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from "react";
import {
  createSnapshotSchedule,
  deleteSnapshotSchedule,
  fetchSnapshotSchedules,
  type SnapshotScheduleRecord,
} from "../../lib/api";
import ErrorBanner from "./ErrorBanner";
import { panelStyles as s } from "./vmPanelStyles";

type Props = {
  namespace: string;
  vmName: string;
};

export function VmSnapshotSchedulesPanel({ namespace, vmName }: Props) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [schedules, setSchedules] = useState<SnapshotScheduleRecord[]>([]);
  const [cron, setCron] = useState("0 2 * * *");
  const [prefix, setPrefix] = useState("sched");
  const [maxSnapshots, setMaxSnapshots] = useState(0);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await fetchSnapshotSchedules(namespace);
      setSchedules(rows.filter((row) => row.vm_name === vmName));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load schedules");
      setSchedules([]);
    } finally {
      setLoading(false);
    }
  }, [namespace, vmName]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const create = async () => {
    if (!cron.trim()) {
      setError("Cron expression is required");
      return;
    }
    setBusy("create");
    setError(null);
    try {
      await createSnapshotSchedule({
        namespace,
        vm_name: vmName,
        cron: cron.trim(),
        snapshot_prefix: prefix.trim() || "sched",
        max_snapshots: maxSnapshots,
      });
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Create schedule failed");
    } finally {
      setBusy(null);
    }
  };

  const remove = async (row: SnapshotScheduleRecord) => {
    if (!confirm(`Delete schedule "${row.name}"?`)) return;
    setBusy(row.name);
    setError(null);
    try {
      await deleteSnapshotSchedule(row.namespace, row.name);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Delete schedule failed");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div style={s.wrap}>
      <button type="button" style={s.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} Snapshot schedules
      </button>
      {open && (
        <div style={s.panel}>
          {loading && <p style={s.muted}>Loading…</p>}
          {error ? <ErrorBanner message={error} style={{ marginBottom: 10 }} /> : null}
          <div style={s.row}>
            <label style={s.label}>
              Cron (UTC)
              <input value={cron} onChange={(e) => setCron(e.target.value)} style={s.inputWide} />
            </label>
            <label style={s.label}>
              Prefix
              <input value={prefix} onChange={(e) => setPrefix(e.target.value)} style={s.input} />
            </label>
            <label style={s.label}>
              Max snapshots
              <input
                type="number"
                min={0}
                value={maxSnapshots}
                onChange={(e) => setMaxSnapshots(parseInt(e.target.value, 10) || 0)}
                style={s.input}
              />
            </label>
            <button type="button" style={s.btnPrimary} disabled={busy !== null} onClick={() => void create()}>
              {busy === "create" ? "Creating…" : "Add schedule"}
            </button>
          </div>
          {schedules.length === 0 && !loading ? (
            <p style={s.muted}>No cron schedules for this VM.</p>
          ) : (
            <ul style={{ listStyle: "none", padding: 0, margin: "12px 0 0" }}>
              {schedules.map((row) => (
                <li
                  key={`${row.namespace}/${row.name}`}
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: 8,
                    alignItems: "center",
                    padding: "8px 0",
                    borderBottom: "1px solid #eee",
                  }}
                >
                  <strong>{row.name}</strong>
                  <span style={s.mono}>{row.cron}</span>
                  <span style={s.muted}>
                    prefix {row.snapshot_prefix}
                    {row.max_snapshots > 0 ? ` · keep ${row.max_snapshots}` : ""}
                    {row.enabled ? "" : " · disabled"}
                  </span>
                  {row.last_run ? (
                    <span style={s.muted}>last {row.last_run.slice(0, 19)}</span>
                  ) : null}
                  <button
                    type="button"
                    style={s.btnDanger}
                    disabled={busy !== null}
                    onClick={() => void remove(row)}
                  >
                    Delete
                  </button>
                </li>
              ))}
            </ul>
          )}
          <p style={s.hint}>
            Schedules run in the API pod (Lease leader). Cron is UTC.
          </p>
        </div>
      )}
    </div>
  );
}

export default VmSnapshotSchedulesPanel;
