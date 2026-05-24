import { useState } from "react";
import {
  deleteVirtualMachine,
  restartVirtualMachine,
  startVirtualMachine,
  stopVirtualMachine,
} from "../../lib/api";

type VmLifecycleBarProps = {
  namespace: string;
  vmName: string;
  status: string;
  onChanged?: () => void;
  onDeleted?: () => void;
  onOpenConsole?: () => void;
};

export function VmLifecycleBar({
  namespace,
  vmName,
  status,
  onChanged,
  onDeleted,
  onOpenConsole,
}: VmLifecycleBarProps) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const normalized = status.toLowerCase();
  const running = normalized === "running";
  const stopped = normalized === "stopped" || normalized === "paused";

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

  return (
    <div style={{ marginTop: "16px", marginBottom: "8px" }}>
      <div style={{ fontSize: "13px", fontWeight: 600, color: "#374151", marginBottom: "8px" }}>
        Lifecycle &amp; console
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
        {btn("Start", "start", running)}
        {btn("Stop", "stop", stopped)}
        {btn("Restart", "restart", !running)}
        {running && onOpenConsole ? (
          <button
            type="button"
            disabled={busy !== null}
            onClick={onOpenConsole}
            style={{
              padding: "8px 14px",
              borderRadius: "6px",
              border: "1px solid rgba(0,180,255,.35)",
              background: "rgba(0,180,255,.08)",
              color: "#0369a1",
              fontWeight: 600,
              fontSize: "13px",
              cursor: "pointer",
            }}
          >
            VNC console
          </button>
        ) : null}
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
  );
}
