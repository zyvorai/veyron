import React from "react";

const card: React.CSSProperties = {
  backgroundColor: "#fff",
  borderRadius: "4px",
  border: "2px solid #e0e0e0",
  padding: "16px",
};

const bar: React.CSSProperties = {
  width: "2px",
  height: "12px",
  backgroundColor: "#f0583a",
  marginRight: "6px",
};

export const PlaceholderExportForm: React.FC = () => (
  <div style={card}>
    <div style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
      <div style={bar} />
      <h3 style={{ margin: 0, fontSize: "12px", fontWeight: 600 }}>VM operations</h3>
    </div>
    <p style={{ margin: 0, fontSize: "11px", color: "#6b7280", lineHeight: 1.5 }}>
      The HyperSDK vSphere export wizard is not bundled here. Use the VMRogue API, kubectl, or your GitOps pipeline
      for lifecycle changes.
    </p>
  </div>
);

export const WorkflowPlaceholder: React.FC = () => (
  <div style={card}>
    <div style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
      <div style={bar} />
      <h3 style={{ margin: 0, fontSize: "12px", fontWeight: 600 }}>Workflow daemon</h3>
    </div>
    <p style={{ margin: 0, fontSize: "11px", color: "#6b7280", lineHeight: 1.5 }}>
      HyperSDK workflow HTTP endpoints are not part of VMRogue. Inventory above is live from your cluster via the
      VMRogue API.
    </p>
  </div>
);

type ManifestPlaceholderProps = {
  onSubmitSuccess?: (jobId: string) => void;
};

export const ManifestPlaceholder: React.FC<ManifestPlaceholderProps> = ({ onSubmitSuccess }) => (
  <div style={card}>
    <div style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
      <div style={bar} />
      <h3 style={{ margin: 0, fontSize: "12px", fontWeight: 600 }}>Manifest builder</h3>
    </div>
    <p style={{ margin: "0 0 12px", fontSize: "11px", color: "#6b7280", lineHeight: 1.5 }}>
      HyperSDK manifest submission UI is not wired. Use CRDs and the operator API instead.
    </p>
    <button
      type="button"
      onClick={() => onSubmitSuccess?.("demo")}
      style={{
        padding: "6px 12px",
        fontSize: "11px",
        fontWeight: 600,
        border: "1px solid #222324",
        borderRadius: "4px",
        background: "#fff",
        cursor: "pointer",
      }}
    >
      Demo callback
    </button>
  </div>
);
