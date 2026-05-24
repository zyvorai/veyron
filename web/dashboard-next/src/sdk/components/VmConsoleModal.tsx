import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { getApiKey } from "../../lib/auth";

type RfbInstance = {
  disconnect: () => void;
  scaleViewport: boolean;
  resizeSession: boolean;
  sendCredentials: (c: { password: string }) => void;
  addEventListener: (event: string, fn: (e: CustomEvent) => void) => void;
};

declare global {
  interface Window {
    RFB?: new (
      target: HTMLElement,
      url: string,
      options?: Record<string, unknown>
    ) => RfbInstance;
  }
}

const VNC_PRESET_KEY = "vmrogue_vnc_preset";

const VNC_PRESETS: Record<string, { qualityLevel: number; compressionLevel: number; clipViewport: boolean }> = {
  lan: { qualityLevel: 9, compressionLevel: 2, clipViewport: true },
  balanced: { qualityLevel: 6, compressionLevel: 5, clipViewport: true },
  low: { qualityLevel: 3, compressionLevel: 8, clipViewport: true },
};

/** Runtime-only import so Vite does not try to bundle the API-served noVNC asset. */
async function loadNovncRfb(): Promise<NonNullable<typeof window.RFB>> {
  const dynamicImport = new Function(
    "url",
    "return import(url)"
  ) as (url: string) => Promise<{ default?: NonNullable<typeof window.RFB> }>;
  const module = await dynamicImport("/assets/novnc.min.js");
  const RFB = module.default;
  if (!RFB) throw new Error("RFB not found in noVNC module");
  return RFB;
}

type Props = {
  namespace: string;
  vmName: string;
  onClose: () => void;
};

export function VmConsoleModal({ namespace, vmName, onClose }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<RfbInstance | null>(null);
  const [status, setStatus] = useState("Connecting…");
  const [preset, setPreset] = useState(() => localStorage.getItem(VNC_PRESET_KEY) || "balanced");

  const connect = useCallback(async () => {
    if (rfbRef.current) {
      rfbRef.current.disconnect();
      rfbRef.current = null;
    }
    const container = containerRef.current;
    if (!container) return;

    setStatus("Connecting…");
    container.querySelectorAll("canvas").forEach((el) => el.remove());

    if (!window.RFB) {
      try {
        window.RFB = await loadNovncRfb();
      } catch (e) {
        setStatus(`Failed to load VNC client: ${e instanceof Error ? e.message : String(e)}`);
        return;
      }
    }

    const apiKey = getApiKey();
    if (!apiKey) {
      setStatus("API key missing — sign in again");
      return;
    }

    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/api/v1/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(vmName)}/vnc?token=${encodeURIComponent(apiKey)}`;
    const enc = VNC_PRESETS[preset] ?? VNC_PRESETS.balanced;

    try {
      const RFB = window.RFB;
      if (!RFB) throw new Error("RFB unavailable");
      const rfb = new RFB(container, url, {
        wsProtocols: ["binary"],
        qualityLevel: enc.qualityLevel,
        compressionLevel: enc.compressionLevel,
        clipViewport: enc.clipViewport,
      });
      rfb.scaleViewport = true;
      rfb.resizeSession = false;
      rfb.addEventListener("connect", () => setStatus("Connected"));
      rfb.addEventListener("disconnect", (e) => {
        const detail = (e as CustomEvent).detail as { reason?: string; clean?: boolean };
        setStatus(detail?.reason || (detail?.clean ? "Disconnected" : "Connection lost"));
      });
      rfb.addEventListener("securityfailure", (e) => {
        const detail = (e as CustomEvent).detail as { reason?: string };
        setStatus(`Auth failed: ${detail?.reason || "unknown"}`);
      });
      rfb.addEventListener("credentialsrequired", () => {
        rfb.sendCredentials({ password: "" });
      });
      rfbRef.current = rfb;
    } catch (e) {
      setStatus(`VNC error: ${e instanceof Error ? e.message : String(e)}`);
    }
  }, [namespace, vmName, preset]);

  useEffect(() => {
    void connect();
    return () => {
      rfbRef.current?.disconnect();
      rfbRef.current = null;
    };
  }, [connect]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      role="presentation"
      style={overlay}
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={`VNC console ${vmName}`}
        style={modal}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={header}>
          <h3 style={{ margin: 0, fontSize: 16 }}>Console: {namespace}/{vmName}</h3>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <label style={{ fontSize: 12 }}>
              Link quality{" "}
              <select
                value={preset}
                onChange={(e) => {
                  const v = e.target.value;
                  setPreset(v);
                  localStorage.setItem(VNC_PRESET_KEY, v);
                }}
                style={{ marginLeft: 4, padding: 4 }}
              >
                <option value="lan">LAN</option>
                <option value="balanced">Balanced</option>
                <option value="low">Low bandwidth</option>
              </select>
            </label>
            <button type="button" onClick={() => void connect()} style={btnSecondary}>
              Reconnect
            </button>
            <button type="button" onClick={onClose} style={btnSecondary} aria-label="Close console">
              Close
            </button>
          </div>
        </div>
        <p style={{ margin: "8px 16px", fontSize: 12, color: status === "Connected" ? "#15803d" : "#666" }}>
          {status}
        </p>
        <div ref={containerRef} style={vncHost} />
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
  width: "min(960px, 96vw)",
  maxHeight: "92vh",
  background: "#1a1b1c",
  borderRadius: 10,
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  boxShadow: "0 20px 50px rgba(0,0,0,0.4)",
};

const header: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "12px 16px",
  borderBottom: "1px solid #333",
  color: "#fff",
  flexWrap: "wrap",
  gap: 8,
};

const vncHost: CSSProperties = {
  flex: 1,
  minHeight: 420,
  background: "#000",
  margin: "0 16px 16px",
  borderRadius: 6,
  overflow: "hidden",
};

const btnSecondary: CSSProperties = {
  padding: "6px 12px",
  borderRadius: 4,
  border: "1px solid #555",
  background: "transparent",
  color: "#e5e7eb",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 600,
};

export default VmConsoleModal;
