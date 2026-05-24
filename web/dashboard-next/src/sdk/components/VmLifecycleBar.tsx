import { useState, type CSSProperties } from "react";
import {
  cloneVirtualMachine,
  deleteVirtualMachine,
  migrateVirtualMachine,
  pauseVirtualMachine,
  restartVirtualMachine,
  startVirtualMachine,
  stopVirtualMachine,
  unpauseVirtualMachine,
} from "../../lib/api";
import { VmResizeModal } from "./VmResizeModal";

type VmLifecycleBarProps = {
  namespace: string;
  vmName: string;
  status: string;
  cpu?: string;
  memory?: string;
  onChanged?: () => void;
  onDeleted?: () => void;
  onOpenConsole?: () => void;
  onOpenSerial?: () => void;
};

export function VmLifecycleBar({
  namespace,
  vmName,
  status,
  cpu,
  memory,
  onChanged,
  onDeleted,
  onOpenConsole,
  onOpenSerial,
}: VmLifecycleBarProps) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resizeOpen, setResizeOpen] = useState(false);

  const normalized = status.toLowerCase();
  const running = normalized === "running";
  const paused = normalized === "paused";
  const stopped = normalized === "stopped" || paused;

  const run = async (action: "start" | "stop" | "restart", label: string) => {
    setBusy(label);
    setError(null);
    try {
      if (action === "start") await startVirtualMachine(namespace, vmName);
      else if (action === "stop") await stopVirtualMachine(namespace, vmName);
      else await restartVirtualMachine(namespace, vmName);
      onChanged?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "VM action failed");
    } finally {
      setBusy(null);
    }
  };

  const runSimple = async (label: string, fn: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await fn();
      onChanged?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : `${label} failed`);
    } finally {
      setBusy(null);
    }
  };

  const remove = async () => {
    if (!confirm(`Delete VM "${vmName}" in ${namespace}? This cannot be undone.`)) return;
    setBusy("Delete");
    setError(null);
    try {
      await deleteVirtualMachine(namespace, vmName);
      onDeleted?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Delete failed");
    } finally {
      setBusy(null);
    }
  };

  const clone = async () => {
    const newName = prompt(`Clone "${vmName}" — enter new VM name:`)?.trim();
    if (!newName) return;
    await runSimple("Clone", () => cloneVirtualMachine(namespace, vmName, newName));
  };

  const migrate = async () => {
    if (!confirm(`Migrate "${vmName}" to another node?`)) return;
    await runSimple("Migrate", () => migrateVirtualMachine(namespace, vmName));
  };

  const btn = (label: string, action: "start" | "stop" | "restart", disabled: boolean) => (
    <button
      type="button"
      disabled={disabled || busy !== null}
      onClick={() => void run(action, label)}
      style={{
        padding: "8px 14px",
        borderRadius: "6px",
        border: "1px solid #ddd",
        background: disabled ? "#f3f4f6" : "#fff",
        color: disabled ? "#9ca3af" : "#222324",
        fontWeight: 600,
        fontSize: "13px",
        cursor: disabled || busy ? "not-allowed" : "pointer",
      }}
    >
      {busy === label ? `${label}…` : label}
    </button>
  );

  const actionBtn = (
    label: string,
    onClick: () => void,
    style: CSSProperties,
    disabled = false,
  ) => (
    <button
      type="button"
      disabled={disabled || busy !== null}
      onClick={onClick}
      style={{
        padding: "8px 14px",
        borderRadius: "6px",
        fontWeight: 600,
        fontSize: "13px",
        cursor: disabled || busy ? "not-allowed" : "pointer",
        ...style,
      }}
    >
      {busy === label ? `${label}…` : label}
    </button>
  );

  return (
    <>
      <div style={{ marginTop: "16px", marginBottom: "8px" }}>
        <div style={{ fontSize: "13px", fontWeight: 600, color: "#374151", marginBottom: "8px" }}>
          Lifecycle &amp; console
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
          {btn("Start", "start", running || paused)}
          {btn("Stop", "stop", stopped)}
          {btn("Restart", "restart", !running)}
          {paused
            ? actionBtn(
                "Unpause",
                () => void runSimple("Unpause", () => unpauseVirtualMachine(namespace, vmName)),
                {
                  border: "1px solid rgba(255,170,0,.35)",
                  background: "rgba(255,170,0,.1)",
                  color: "#b45309",
                },
              )
            : null}
          {running
            ? actionBtn(
                "Pause",
                () => void runSimple("Pause", () => pauseVirtualMachine(namespace, vmName)),
                {
                  border: "1px solid rgba(255,170,0,.35)",
                  background: "rgba(255,170,0,.1)",
                  color: "#b45309",
                },
              )
            : null}
          {running
            ? actionBtn("Migrate", () => void migrate(), {
                border: "1px solid rgba(123,104,238,.25)",
                background: "rgba(123,104,238,.08)",
                color: "#6d28d9",
              })
            : null}
          {running && onOpenConsole
            ? actionBtn("VNC console", onOpenConsole, {
                border: "1px solid rgba(0,180,255,.35)",
                background: "rgba(0,180,255,.08)",
                color: "#0369a1",
              })
            : null}
          {running && onOpenSerial
            ? actionBtn("Serial console", onOpenSerial, {
                border: "1px solid rgba(34,197,94,.35)",
                background: "rgba(34,197,94,.08)",
                color: "#15803d",
              })
            : null}
          {actionBtn("Clone", () => void clone(), {
            border: "1px solid rgba(0,180,255,.25)",
            background: "rgba(0,180,255,.06)",
            color: "#0369a1",
          })}
          {actionBtn("Resize", () => setResizeOpen(true), {
            border: "1px solid rgba(59,130,246,.25)",
            background: "rgba(59,130,246,.08)",
            color: "#1d4ed8",
          })}
          <button
            type="button"
            disabled={busy !== null}
            onClick={() => void remove()}
            style={{
              padding: "8px 14px",
              borderRadius: "6px",
              border: "1px solid rgba(244,67,54,.35)",
              background: "rgba(244,67,54,.08)",
              color: "#b91c1c",
              fontWeight: 600,
              fontSize: "13px",
              cursor: busy ? "not-allowed" : "pointer",
            }}
          >
            {busy === "Delete" ? "Deleting…" : "Delete VM"}
          </button>
        </div>
        {error ? (
          <p style={{ marginTop: "8px", fontSize: "13px", color: "#dc2626" }} role="alert">
            {error}
          </p>
        ) : null}
      </div>
      {resizeOpen ? (
        <VmResizeModal
          namespace={namespace}
          vmName={vmName}
          initialCpu={cpu}
          initialMemory={memory}
          onClose={() => setResizeOpen(false)}
          onResized={onChanged}
        />
      ) : null}
    </>
  );
}
