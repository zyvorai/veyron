// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import React, { useCallback, useEffect, useState } from "react";
import type { VmrogueView } from "../../lib/nav";
import { ZyvorAboutModal } from "../components/ZyvorAboutModal";
import { ZyvorFooter } from "../components/ZyvorBrand";

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
  activeView: VmrogueView;
  onViewChange: (view: VmrogueView) => void;
  onLogout: () => void;
  displayUser?: string;
  children: React.ReactNode;
}

/**
 * HyperSDK dashboard-react–style chrome: persistent left application rail + main workspace.
 */
export const HyperShell: React.FC<HyperShellProps> = ({
  activeView,
  onViewChange,
  onLogout,
  displayUser,
  children,
}) => {
  const [aboutOpen, setAboutOpen] = useState(false);
  const [apiVersion, setApiVersion] = useState<string | undefined>();

  const openAbout = useCallback(() => {
    setAboutOpen(true);
    void fetch("/api/v1/health", { headers: { Accept: "application/json" } })
      .then((r) => (r.ok ? r.json() : null))
      .then((body: { version?: string; data?: { version?: string } } | null) => {
        const v = body?.version ?? body?.data?.version;
        if (v) setApiVersion(v);
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!aboutOpen) setApiVersion(undefined);
  }, [aboutOpen]);

  const navActive = (view: VmrogueView): React.CSSProperties =>
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
            KubeVirt console
          </div>
          <h1 style={{ margin: 0, fontSize: "22px", fontWeight: 700, color: "#fff" }}>VMRogue</h1>
          <p style={{ margin: "6px 0 0 0", fontSize: "12px", color: "#9ca3af", lineHeight: 1.4 }}>
            Fleet operations dashboard
          </p>
        </div>

        <nav style={{ flex: 1 }} aria-label="Primary">
          <button
            type="button"
            style={navActive("dashboard")}
            onClick={() => onViewChange("dashboard")}
          >
            <span aria-hidden>▤</span>
            Dashboard
          </button>
          <button
            type="button"
            style={navActive("fleet")}
            onClick={() => onViewChange("fleet")}
          >
            <span aria-hidden>◉</span>
            Fleet
          </button>
          <button
            type="button"
            style={navActive("inventory")}
            onClick={() => onViewChange("inventory")}
          >
            <span aria-hidden>◇</span>
            Clusters{" & "}VMs
          </button>
          <button
            type="button"
            style={navActive("nodes")}
            onClick={() => onViewChange("nodes")}
          >
            <span aria-hidden>⬡</span>
            Nodes
          </button>
          <button
            type="button"
            style={navActive("storage")}
            onClick={() => onViewChange("storage")}
          >
            <span aria-hidden>▣</span>
            Storage
          </button>
          <button
            type="button"
            style={navActive("backups")}
            onClick={() => onViewChange("backups")}
          >
            <span aria-hidden>💾</span>
            Backups
          </button>
          <button
            type="button"
            style={navActive("catalog")}
            onClick={() => onViewChange("catalog")}
          >
            <span aria-hidden>📋</span>
            Catalog
          </button>
          <button
            type="button"
            style={navActive("platform")}
            onClick={() => onViewChange("platform")}
          >
            <span aria-hidden>◫</span>
            Platform
          </button>
          <button
            type="button"
            style={navActive("workloads")}
            onClick={() => onViewChange("workloads")}
          >
            <span aria-hidden>◎</span>
            Workloads
          </button>
          <button
            type="button"
            style={navActive("insights")}
            onClick={() => onViewChange("insights")}
          >
            <span aria-hidden>◷</span>
            Insights
          </button>
          <button
            type="button"
            style={navActive("integrations")}
            onClick={() => onViewChange("integrations")}
          >
            <span aria-hidden>⚡</span>
            Integrations
          </button>
          <button
            type="button"
            style={navActive("compliance")}
            onClick={() => onViewChange("compliance")}
          >
            <span aria-hidden>✓</span>
            Compliance
          </button>
          <button
            type="button"
            style={navActive("operations")}
            onClick={() => onViewChange("operations")}
          >
            <span aria-hidden>↻</span>
            Operations
          </button>
        </nav>

        <div style={{ borderTop: "1px solid #2d2f32", paddingTop: "16px", marginTop: "8px" }}>
          <p
            style={{
              fontSize: "10px",
              fontWeight: 700,
              letterSpacing: "0.1em",
              textTransform: "uppercase",
              color: "#6b7280",
              margin: "0 0 8px 8px",
            }}
          >
            Help
          </p>
          <button
            type="button"
            onClick={openAbout}
            style={{
              ...navItemBase,
              marginBottom: "12px",
              fontSize: "13px",
              fontWeight: 500,
            }}
          >
            <span aria-hidden>?</span>
            About VMRogue
          </button>
          {displayUser ? (
            <p style={{ fontSize: "11px", color: "#9ca3af", margin: "0 0 10px 8px" }}>
              Signed in as <strong style={{ color: "#e5e7eb" }}>{displayUser}</strong>
            </p>
          ) : null}
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
        <ZyvorFooter className="border-t border-slate-200 bg-white/80" />
      </div>

      {aboutOpen ? (
        <ZyvorAboutModal
          product="VMRogue"
          productTagline="KubeVirt fleet operations console for Kubernetes clusters."
          version={apiVersion}
          onClose={() => setAboutOpen(false)}
          extraLinks={[
            {
              label: "Full dashboard",
              href: "/dashboard",
              description: "Classic SPA with VNC, snapshots, and platform pages",
            },
          ]}
        />
      ) : null}
    </div>
  );
};
