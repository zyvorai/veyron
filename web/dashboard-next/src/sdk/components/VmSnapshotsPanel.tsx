import { useCallback, useEffect, useState } from "react";
import {
  createVmSnapshot,
  deleteVmSnapshot,
  fetchVmSnapshots,
  restoreVmSnapshot,
  type VmSnapshot,
} from "../../lib/api";
import { panelStyles as s } from "./vmPanelStyles";

type Props = {
  namespace: string;
  vmName: string;
};

export function VmSnapshotsPanel({ namespace, vmName }: Props) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [snapshots, setSnapshots] = useState<VmSnapshot[]>([]);
  const [snapName, setSnapName] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSnapshots(await fetchVmSnapshots(namespace, vmName));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load snapshots");
    } finally {
      setLoading(false);
    }
  }, [namespace, vmName]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const create = async () => {
    setBusy("create");
    setError(null);
    try {
      await createVmSnapshot(namespace, vmName, snapName.trim() || undefined);
      setSnapName("");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Create failed");
    } finally {
      setBusy(null);
    }
  };

  const remove = async (name: string) => {
    if (!confirm(`Delete snapshot "${name}"? This cannot be undone.`)) return;
    setBusy(name);
    setError(null);
    try {
      await deleteVmSnapshot(namespace, name);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setBusy(null);
    }
  };

  const restore = async (name: string) => {
    if (!confirm(`Restore snapshot "${name}"? The VM should be stopped first.`)) return;
    setBusy(`restore-${name}`);
    setError(null);
    try {
      await restoreVmSnapshot(namespace, name);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Restore failed");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div style={s.wrap}>
      <button type="button" style={s.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} Snapshots
      </button>
      {open && (
        <div style={s.panel}>
          {loading && <p style={s.muted}>Loading…</p>}
          {error && <p style={s.err}>{error}</p>}
          <div style={s.row}>
            <label style={s.label}>
              Snapshot name (optional)
              <input
                value={snapName}
                onChange={(e) => setSnapName(e.target.value)}
                placeholder="auto-generated if empty"
                style={s.inputWide}
              />
            </label>
            <button type="button" style={s.btnPrimary} disabled={busy !== null} onClick={() => void create()}>
              {busy === "create" ? "Creating…" : "Create snapshot"}
            </button>
          </div>
          {snapshots.length === 0 && !loading ? (
            <p style={s.muted}>No snapshots for this VM.</p>
          ) : (
            <ul style={{ listStyle: "none", padding: 0, margin: "12px 0 0" }}>
              {snapshots.map((snap) => (
                <li
                  key={snap.name}
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: 8,
                    alignItems: "center",
                    padding: "8px 0",
                    borderBottom: "1px solid #eee",
                  }}
                >
                  <strong>{snap.name}</strong>
                  <span style={s.muted}>{snap.status}</span>
                  <span style={s.muted}>{snap.age}</span>
                  <button
                    type="button"
                    style={s.btnSuccess}
                    disabled={busy !== null}
                    onClick={() => void restore(snap.name)}
                  >
                    Restore
                  </button>
                  <button
                    type="button"
                    style={s.btnDanger}
                    disabled={busy !== null}
                    onClick={() => void remove(snap.name)}
                  >
                    Delete
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

export default VmSnapshotsPanel;
