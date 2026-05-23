import React from "react";

export type HyperShellView = "dashboard" | "inventory";

const SIDEBAR_W = 260;

const navItemBase: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  width: "100%",
  padding: "12px 16px",
  marginBottom: "4px",
  border: "none",
  borderRadius: "6px",
  background: "transparent",
  color: "#e8e8ea",
  fontSize: "14px",
  fontWeight: 600,
  textAlign: "left",
  cursor: "pointer",
  fontFamily: "inherit",
  transition: "background 0.2s, color 0.2s",
};

interface HyperShellProps {
  activeView: HyperShellView;
  onViewChange: (view: HyperShellView) => void;
  onLogout: () => void;
  children: React.ReactNode;
}

/**
 * HyperSDK dashboard-react–style chrome: persistent left application rail + main workspace.
 */
export const HyperShell: React.FC<HyperShellProps> = ({ activeView, onViewChange, onLogout, children }) => {
  const navActive = (view: HyperShellView): React.CSSProperties =>
    activeView === view
      ? {
          ...navItemBase,
          backgroundColor: "rgba(240, 88, 58, 0.18)",
          color: "#fff",
          boxShadow: "inset 3px 0 0 #f0583a",
        }
      : navItemBase;

  return (
    <div style={{ display: "flex", minHeight: "100vh", backgroundColor: "#f0f2f7" }}>
      <aside
        style={{
          width: SIDEBAR_W,
          flexShrink: 0,
          backgroundColor: "#1a1b1c",
          color: "#fff",
          display: "flex",
          flexDirection: "column",
          padding: "20px 14px",
          borderRight: "1px solid #2d2f32",
        }}
        aria-label="Application"
      >
        <div style={{ marginBottom: "28px" }}>
          <div
            style={{
              fontSize: "11px",
              fontWeight: 700,
              letterSpacing: "0.12em",
              color: "#f0583a",
              textTransform: "uppercase",
              marginBottom: "6px",
            }}
          >
            HyperSDK layout
          </div>
          <h1 style={{ margin: 0, fontSize: "22px", fontWeight: 700, color: "#fff" }}>VMRogue</h1>
          <p style={{ margin: "6px 0 0 0", fontSize: "12px", color: "#9ca3af", lineHeight: 1.4 }}>
            KubeVirt operator console
          </p>
        </div>

        <nav style={{ flex: 1 }} aria-label="Primary">
          <button
            type="button"
            style={navActive("dashboard")}
            onClick={() => {
              onViewChange("dashboard");
              window.dispatchEvent(new CustomEvent("vmrogue:nav", { detail: { view: "dashboard" } }));
            }}
          >
            <span aria-hidden>▤</span>
            Dashboard
          </button>
          <button
            type="button"
            style={navActive("inventory")}
            onClick={() => onViewChange("inventory")}
          >
            <span aria-hidden>◇</span>
            Clusters{" & "}VMs
          </button>
        </nav>

        <div style={{ borderTop: "1px solid #2d2f32", paddingTop: "16px", marginTop: "8px" }}>
          <p style={{ fontSize: "10px", color: "#6b7280", margin: "0 0 12px 8px", lineHeight: 1.45 }}>
            UI clone of HyperSDK <code style={{ color: "#9ca3af" }}>dashboard-react</code> rail + workspace. Data from
            VMRogue API.
          </p>
          <button
            type="button"
            onClick={onLogout}
            style={{
              width: "100%",
              padding: "10px 14px",
              borderRadius: "6px",
              border: "1px solid #4b5563",
              background: "transparent",
              color: "#fca5a5",
              fontWeight: 600,
              fontSize: "13px",
              cursor: "pointer",
            }}
          >
            Log out
          </button>
        </div>
      </aside>

      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
          minHeight: "100vh",
        }}
      >
        {children}
        <footer className="zyvor-footer" style={{ marginTop: "auto", padding: "12px", textAlign: "center", fontSize: "12px", color: "#6b7280", borderTop: "1px solid #e5e7eb" }} role="contentinfo">
          <a href="https://zyvor.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#f0583a", fontWeight: 600, textDecoration: "none" }}>zyvor.dev</a>
          {" · HyperSDK · © 2026"}
        </footer>
      </div>
    </div>
  );
};
