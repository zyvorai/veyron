import React from "react";
import { requestVmrogueNav } from "../../lib/nav";

/**
 * Sticky sub-header: in-page anchors on the dashboard and cross-view links to inventory.
 */
export const HyperPageTabs: React.FC = () => {
  const tabStyle: React.CSSProperties = {
    color: "#222324",
    fontSize: "14px",
    fontWeight: 600,
    textDecoration: "none",
    background: "none",
    border: "none",
    padding: 0,
    cursor: "pointer",
    fontFamily: "inherit",
  };

  return (
    <header
      style={{
        backgroundColor: "#fff",
        borderBottom: "1px solid #e0e0e0",
        position: "sticky",
        top: 0,
        zIndex: 50,
      }}
    >
      <div
        style={{
          maxWidth: "1400px",
          margin: "0 auto",
          padding: "12px 24px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "16px",
          flexWrap: "wrap",
        }}
      >
        <nav style={{ display: "flex", gap: "20px", alignItems: "center", flexWrap: "wrap" }} aria-label="Section">
          <a href="#dashboard" style={tabStyle}>
            Overview
          </a>
          <a href="#jobs" style={tabStyle}>
            VM list
          </a>
          <button
            type="button"
            style={tabStyle}
            onClick={() => requestVmrogueNav({ view: "dashboard", scrollTo: "alerts" })}
          >
            Alerts
          </button>
          <button
            type="button"
            style={tabStyle}
            onClick={() => requestVmrogueNav({ view: "inventory" })}
          >
            Clusters &amp; VMs
          </button>
          <button
            type="button"
            style={tabStyle}
            onClick={() => requestVmrogueNav({ view: "nodes" })}
          >
            Nodes
          </button>
          <button
            type="button"
            style={tabStyle}
            onClick={() => requestVmrogueNav({ view: "storage" })}
          >
            Storage
          </button>
          <a href="/dashboard" style={tabStyle}>
            Full dashboard
          </a>
        </nav>
      </div>
    </header>
  );
};
