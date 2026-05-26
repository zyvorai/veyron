// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { getApiKey } from "../../lib/auth";

const OUT_MAX = 256_000;

type Props = {
  namespace: string;
  vmName: string;
  onClose: () => void;
};

function bufToText(buf: ArrayBuffer): string {
  try {
    return new TextDecoder("utf-8", { fatal: false }).decode(buf);
  } catch {
    return "";
  }
}

export function VmSerialModal({ namespace, vmName, onClose }: Props) {
  const wsRef = useRef<WebSocket | null>(null);
  const outRef = useRef<HTMLPreElement>(null);
  const [status, setStatus] = useState("Connecting…");
  const [input, setInput] = useState("");

  const append = useCallback((chunk: string) => {
    const el = outRef.current;
    if (!el || !chunk) return;
    let text = el.textContent ?? "";
    text += chunk;
    if (text.length > OUT_MAX) text = text.slice(-OUT_MAX);
    el.textContent = text;
    el.scrollTop = el.scrollHeight;
  }, []);

  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.onopen = null;
      wsRef.current.onclose = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      try {
        wsRef.current.close();
      } catch {
        /* ignore */
      }
      wsRef.current = null;
    }
    if (outRef.current) outRef.current.textContent = "";

    const apiKey = getApiKey();
    if (!apiKey) {
      setStatus("API key missing — sign in again");
      return;
    }

    setStatus("Connecting…");
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/api/v1/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(vmName)}/serial?token=${encodeURIComponent(apiKey)}`;

    try {
      const ws = new WebSocket(url, ["binary"]);
      ws.binaryType = "arraybuffer";
      ws.onopen = () => setStatus("Connected");
      ws.onclose = () => setStatus("Disconnected");
      ws.onerror = () => setStatus("Connection error");
      ws.onmessage = (ev) => {
        if (typeof ev.data === "string") append(ev.data);
        else if (ev.data instanceof ArrayBuffer) append(bufToText(ev.data));
      };
      wsRef.current = ws;
    } catch (e) {
      setStatus(`Failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }, [append, namespace, vmName]);

  useEffect(() => {
    connect();
    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [connect]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const sendLine = () => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    const line = input.endsWith("\n") ? input : `${input}\n`;
    ws.send(line);
    setInput("");
  };

  return (
    <div role="presentation" style={overlay} onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={`Serial console ${vmName}`}
        style={modal}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={header}>
          <h3 style={{ margin: 0, fontSize: 16 }}>
            Serial: {namespace}/{vmName}
          </h3>
          <div style={{ display: "flex", gap: 8 }}>
            <button type="button" onClick={connect} style={btnSecondary}>
              Reconnect
            </button>
            <button type="button" onClick={onClose} style={btnSecondary}>
              Close
            </button>
          </div>
        </div>
        <p style={{ margin: "8px 16px", fontSize: 12, color: status === "Connected" ? "#15803d" : "#666" }}>
          {status}
        </p>
        <pre ref={outRef} style={output} />
        <div style={footer}>
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                sendLine();
              }
            }}
            placeholder="Type and press Enter to send…"
            style={inputStyle}
            aria-label="Serial input"
          />
          <button type="button" onClick={sendLine} style={btnPrimary}>
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  zIndex: 2000,
  background: "rgba(0,0,0,0.55)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  padding: 16,
};

const modal: CSSProperties = {
  width: "min(920px, 96vw)",
  maxHeight: "92vh",
  background: "#0f172a",
  borderRadius: 10,
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  boxShadow: "0 20px 50px rgba(0,0,0,0.4)",
  color: "#e2e8f0",
};

const header: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "12px 16px",
  borderBottom: "1px solid #334155",
  flexWrap: "wrap",
  gap: 8,
};

const output: CSSProperties = {
  flex: 1,
  minHeight: 320,
  margin: "0 16px",
  padding: 12,
  background: "#020617",
  borderRadius: 6,
  overflow: "auto",
  fontFamily: "ui-monospace, monospace",
  fontSize: 12,
  lineHeight: 1.4,
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
};

const footer: CSSProperties = {
  display: "flex",
  gap: 8,
  padding: 16,
  borderTop: "1px solid #334155",
};

const inputStyle: CSSProperties = {
  flex: 1,
  padding: "8px 10px",
  borderRadius: 4,
  border: "1px solid #475569",
  background: "#1e293b",
  color: "#f8fafc",
  fontFamily: "ui-monospace, monospace",
  fontSize: 13,
};

const btnPrimary: CSSProperties = {
  padding: "8px 14px",
  borderRadius: 4,
  border: "none",
  background: "#f0583a",
  color: "#fff",
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: CSSProperties = {
  padding: "6px 12px",
  borderRadius: 4,
  border: "1px solid #64748b",
  background: "transparent",
  color: "#e2e8f0",
  fontWeight: 600,
  cursor: "pointer",
  fontSize: 12,
};

export default VmSerialModal;
