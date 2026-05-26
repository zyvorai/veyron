// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, type CSSProperties } from "react";
import { resizeVirtualMachine } from "../../lib/api";

type Props = {
  namespace: string;
  vmName: string;
  initialCpu?: string;
  initialMemory?: string;
  onClose: () => void;
  onResized?: () => void;
};

export function VmResizeModal({
  namespace,
  vmName,
  initialCpu,
  initialMemory,
  onClose,
  onResized,
}: Props) {
  const [cpus, setCpus] = useState(1);
  const [memory, setMemory] = useState("2Gi");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const parsed = parseInt(String(initialCpu ?? "").replace(/\D/g, ""), 10);
    setCpus(Number.isFinite(parsed) && parsed > 0 ? parsed : 1);
    setMemory(initialMemory?.trim() || "2Gi");
  }, [initialCpu, initialMemory, namespace, vmName]);

  const submit = async () => {
    if (cpus < 1) {
      setError("CPU cores must be at least 1");
      return;
    }
    const mem = memory.trim();
    if (!mem) {
      setError("Memory is required (e.g. 4Gi)");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await resizeVirtualMachine(namespace, vmName, { cpus, memory: mem });
      onResized?.();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Resize failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div role="presentation" style={overlay} onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={`Resize ${vmName}`}
        style={modal}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={header}>
          <h3 style={{ margin: 0, fontSize: 16 }}>Resize VM</h3>
          <button type="button" onClick={onClose} style={btnSecondary}>
            Close
          </button>
        </div>
        <p style={{ margin: "8px 16px 0", fontSize: 13, color: "#666" }}>
          {namespace}/{vmName} — restart the VM to apply CPU/memory changes.
        </p>
        <div style={form}>
          <label style={label}>
            CPU cores
            <input
              type="number"
              min={1}
              max={64}
              value={cpus}
              onChange={(e) => setCpus(parseInt(e.target.value, 10) || 1)}
              style={input}
            />
          </label>
          <label style={label}>
            Memory
            <input
              value={memory}
              onChange={(e) => setMemory(e.target.value)}
              placeholder="4Gi"
              style={input}
            />
          </label>
        </div>
        {error ? (
          <p style={{ margin: "0 16px", fontSize: 13, color: "#dc2626" }} role="alert">
            {error}
          </p>
        ) : null}
        <div style={footer}>
          <button type="button" onClick={onClose} disabled={busy} style={btnSecondary}>
            Cancel
          </button>
          <button type="button" onClick={() => void submit()} disabled={busy} style={btnPrimary}>
            {busy ? "Saving…" : "Apply resize"}
          </button>
        </div>
      </div>
    </div>
  );
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(15,23,42,.55)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 1000,
  padding: 16,
};

const modal: CSSProperties = {
  background: "#fff",
  borderRadius: 10,
  width: "100%",
  maxWidth: 420,
  boxShadow: "0 20px 50px rgba(0,0,0,.2)",
};

const header: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "14px 16px",
  borderBottom: "1px solid #e5e7eb",
};

const form: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 12,
  padding: 16,
};

const label: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  fontSize: 13,
  fontWeight: 600,
  color: "#374151",
};

const input: CSSProperties = {
  padding: "8px 10px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  fontSize: 14,
};

const footer: CSSProperties = {
  display: "flex",
  justifyContent: "flex-end",
  gap: 8,
  padding: "12px 16px 16px",
};

const btnSecondary: CSSProperties = {
  padding: "8px 14px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  background: "#fff",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
};

const btnPrimary: CSSProperties = {
  padding: "8px 14px",
  borderRadius: 6,
  border: "none",
  background: "#f0583a",
  color: "#fff",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
};
