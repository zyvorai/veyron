import React from "react";

/**
 * HyperSDK-style sticky sub-header: hash anchors for in-page sections (matches legacy dashboard-react header).
 * Sign out lives on the HyperShell sidebar.
 */
export const HyperPageTabs: React.FC = () => {
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
          <a
            href="#dashboard"
            style={{
              color: "#222324",
              fontSize: "14px",
              fontWeight: 600,
              textDecoration: "none",
            }}
          >
            Dashboard
          </a>
          <a
            href="#jobs"
            style={{
              color: "#222324",
              fontSize: "14px",
              fontWeight: 600,
              textDecoration: "none",
            }}
          >
            VM jobs
          </a>
          <a
            href="#manage"
            style={{
              color: "#222324",
              fontSize: "14px",
              fontWeight: 600,
              textDecoration: "none",
            }}
          >
            Manage
          </a>
          <a
            href="#clusters-vms"
            style={{
              color: "#222324",
              fontSize: "14px",
              fontWeight: 600,
              textDecoration: "none",
            }}
            onClick={(e) => {
              e.preventDefault();
              window.dispatchEvent(new CustomEvent("vmrogue:nav", { detail: { view: "inventory" } }));
            }}
          >
            Namespaces{" & "}VMs
          </a>
        </nav></div>
    </header>
  );
};
