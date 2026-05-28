// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchCatalogProfiles,
  fetchCatalogTemplates,
  fetchTemplates,
  type CatalogProfileRecord,
  type CatalogTemplateRecord,
  type VmTemplate,
} from "../../lib/api";
import { requestVmrogueNav } from "../../lib/nav";
import ErrorBanner from "./ErrorBanner";

export function CatalogPanel() {
  const [templates, setTemplates] = useState<CatalogTemplateRecord[]>([]);
  const [profiles, setProfiles] = useState<CatalogProfileRecord[]>([]);
  const [embedded, setEmbedded] = useState<VmTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [catalogMessage, setCatalogMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    setCatalogMessage(null);
    const results = await Promise.allSettled([
      fetchCatalogTemplates(),
      fetchCatalogProfiles(),
      fetchTemplates(),
    ]);
    if (results[0].status === "fulfilled") {
      setTemplates(results[0].value.items);
      setCatalogMessage(results[0].value.message ?? null);
    } else {
      setTemplates([]);
      setError(
        results[0].reason instanceof Error
          ? results[0].reason.message
          : "Failed to load VMTemplate CRDs",
      );
    }
    if (results[1].status === "fulfilled") {
      setProfiles(results[1].value.items);
      if (results[1].value.message) {
        setCatalogMessage((prev) => prev ?? results[1].value.message ?? null);
      }
    } else {
      setProfiles([]);
    }
    if (results[2].status === "fulfilled") {
      setEmbedded(results[2].value);
    } else {
      setEmbedded([]);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const crdCount = templates.length + profiles.length;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Template catalog</h2>
          <p style={subtitle}>
            {loading
              ? "Loading cluster VMTemplate and VMProfile CRDs…"
              : `${templates.length} template(s) · ${profiles.length} profile(s) · ${embedded.length} embedded OS template(s)`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button type="button" style={secondaryBtn} onClick={() => requestVmrogueNav({ view: "inventory" })}>
            Create VM
          </button>
          <button type="button" style={secondaryBtn} onClick={() => requestVmrogueNav({ view: "platform" })}>
            Golden images
          </button>
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      {error ? <ErrorBanner message={error} /> : null}
      {catalogMessage ? <p style={note}>{catalogMessage}</p> : null}

      <p style={hint}>
        Operator resolves <code style={code}>template</code> + <code style={code}>profile</code> on{" "}
        <strong>VMRogueVM</strong> from cluster CRDs. Sync from the repo with{" "}
        <code style={code}>vmrogue catalog sync</code> or edit YAML in the{" "}
        <a href="/dashboard" style={link}>classic dashboard</a>.
      </p>

      <section style={section}>
        <h3 style={sectionTitle}>VMTemplate CRDs</h3>
        {templates.length === 0 && !loading ? (
          <p style={muted}>
            No VMTemplate resources in the cluster. Run{" "}
            <code style={code}>./scripts/generate-catalog-crds.sh</code> then deploy, or use embedded
            templates below for quick VM create.
          </p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Family</th>
                  <th style={th}>Tags</th>
                  <th style={th}>Description</th>
                </tr>
              </thead>
              <tbody>
                {templates.map((t) => (
                  <tr key={t.name} style={tr}>
                    <td style={td}><strong>{t.name}</strong></td>
                    <td style={td}>{t.family || "—"}</td>
                    <td style={td}>{t.tags?.length ? t.tags.join(", ") : "—"}</td>
                    <td style={td}>{t.description || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>VMProfile CRDs</h3>
        {profiles.length === 0 && !loading ? (
          <p style={muted}>No VMProfile resources. Profiles define CPU, memory, and disk defaults.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>CPU</th>
                  <th style={th}>Memory</th>
                  <th style={th}>Disk</th>
                  <th style={th}>Description</th>
                </tr>
              </thead>
              <tbody>
                {profiles.map((p) => (
                  <tr key={p.name} style={tr}>
                    <td style={td}><strong>{p.name}</strong></td>
                    <td style={td}>{p.cores}</td>
                    <td style={tdMono}>{p.memory}</td>
                    <td style={tdMono}>{p.disk_size}</td>
                    <td style={td}>{p.description || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Embedded OS templates (API)</h3>
        <p style={hint}>
          <code style={code}>GET /api/v1/templates</code> — used by quick create when CRD catalog is empty
          {crdCount > 0 ? " or as fallback names" : ""}.
        </p>
        {embedded.length === 0 && !loading ? (
          <p style={muted}>No embedded templates returned.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>OS</th>
                  <th style={th}>Defaults</th>
                  <th style={th}>Description</th>
                </tr>
              </thead>
              <tbody>
                {embedded.slice(0, 30).map((t) => (
                  <tr key={t.name} style={tr}>
                    <td style={td}><strong>{t.name}</strong></td>
                    <td style={td}>{t.os_type}</td>
                    <td style={tdMono}>
                      {t.default_cpus} CPU · {t.default_memory} · {t.default_disk_size}
                    </td>
                    <td style={td}>{t.description || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {embedded.length > 30 ? (
          <p style={hint}>Showing 30 of {embedded.length} embedded templates.</p>
        ) : null}
      </section>
    </div>
  );
}

export default CatalogPanel;

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1200, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start",
  gap: 16, marginBottom: 20, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, fontWeight: 700, color: "#111827" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const section: CSSProperties = {
  background: "#fff", borderRadius: 10, border: "1px solid #e5e7eb",
  padding: "16px 18px", marginBottom: 16,
};
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 700, color: "#374151" };
const tableWrap: CSSProperties = { overflowX: "auto" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "8px 10px", borderBottom: "2px solid #e5e7eb",
  color: "#6b7280", fontWeight: 600, fontSize: 11, textTransform: "uppercase",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "10px", verticalAlign: "top", color: "#374151" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const muted: CSSProperties = { margin: 0, fontSize: 13, color: "#9ca3af" };
const hint: CSSProperties = { margin: "0 0 12px", fontSize: 12, color: "#6b7280", lineHeight: 1.5 };
const note: CSSProperties = { margin: "0 0 12px", fontSize: 12, color: "#92400e", background: "#fffbeb", padding: "8px 12px", borderRadius: 6 };
const code: CSSProperties = { fontFamily: "ui-monospace, monospace", fontSize: 11, background: "#f3f4f6", padding: "1px 4px", borderRadius: 3 };
const link: CSSProperties = { color: "#c2410c", fontWeight: 600 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db",
  background: "#fff", fontSize: 13, fontWeight: 600, cursor: "pointer",
};
const secondaryBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#f0583a", color: "#c2410c", background: "#fff7ed",
};
