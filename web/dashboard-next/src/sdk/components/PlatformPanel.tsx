// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  createTenant,
  fetchCustomResources,
  fetchGitOpsStatus,
  fetchImageCatalog,
  fetchNetworkPolicies,
  fetchTenants,
  importDataVolume,
  triggerGitOpsSync,
  type CustomResourceRecord,
  type GitOpsStatusRecord,
  type ImageCatalogResponse,
  type NetworkPolicyRecord,
  type TenantRecord,
} from "../../lib/api";
import { requestVmrogueNav } from "../../lib/nav";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

type Props = {
  scopeNamespace?: string;
};

export function PlatformPanel({ scopeNamespace = "all" }: Props) {
  const [gitops, setGitops] = useState<GitOpsStatusRecord | null>(null);
  const [crds, setCrds] = useState<CustomResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [gitopsError, setGitopsError] = useState<string | null>(null);
  const [crdsError, setCrdsError] = useState<string | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const [syncMessage, setSyncMessage] = useState<string | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [selectedArgoApp, setSelectedArgoApp] = useState("");
  const [selectedFluxKust, setSelectedFluxKust] = useState("");
  const [tenants, setTenants] = useState<TenantRecord[]>([]);
  const [images, setImages] = useState<ImageCatalogResponse | null>(null);
  const [netpols, setNetpols] = useState<NetworkPolicyRecord[]>([]);
  const [tenantsError, setTenantsError] = useState<string | null>(null);
  const [imagesError, setImagesError] = useState<string | null>(null);
  const [netpolError, setNetpolError] = useState<string | null>(null);
  const [tenantBusy, setTenantBusy] = useState(false);
  const [importBusy, setImportBusy] = useState(false);
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const [tenantForm, setTenantForm] = useState({ id: "", display_name: "", owner_email: "" });
  const [importForm, setImportForm] = useState({
    name: "",
    url: "",
    registry: "",
    size: "20Gi",
  });

  const load = useCallback(async () => {
    setLoading(true);
    setGitopsError(null);
    setCrdsError(null);
    setTenantsError(null);
    setImagesError(null);
    setNetpolError(null);
    const [statusResult, resourcesResult, tenantsResult, imagesResult, netpolResult] =
      await Promise.allSettled([
      fetchGitOpsStatus(scopeNamespace),
      fetchCustomResources(),
      fetchTenants(),
      fetchImageCatalog(scopeNamespace),
      fetchNetworkPolicies(scopeNamespace),
    ]);

    if (statusResult.status === "fulfilled") {
      setGitops(statusResult.value);
    } else {
      setGitops(null);
      setGitopsError(
        statusResult.reason instanceof Error
          ? statusResult.reason.message
          : "Failed to load GitOps status",
      );
    }

    if (resourcesResult.status === "fulfilled") {
      setCrds(resourcesResult.value);
    } else {
      setCrds([]);
      setCrdsError(
        resourcesResult.reason instanceof Error
          ? resourcesResult.reason.message
          : "Failed to load CRD inventory",
      );
    }

    if (tenantsResult.status === "fulfilled") {
      setTenants(tenantsResult.value);
    } else {
      setTenants([]);
      setTenantsError(
        tenantsResult.reason instanceof Error
          ? tenantsResult.reason.message
          : "Failed to load tenants",
      );
    }

    if (imagesResult.status === "fulfilled") {
      setImages(imagesResult.value);
    } else {
      setImages(null);
      setImagesError(
        imagesResult.reason instanceof Error
          ? imagesResult.reason.message
          : "Failed to load image catalog",
      );
    }

    if (netpolResult.status === "fulfilled") {
      setNetpols(netpolResult.value);
    } else {
      setNetpols([]);
      setNetpolError(
        netpolResult.reason instanceof Error
          ? netpolResult.reason.message
          : "Failed to load network policies",
      );
    }

    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const apps = gitops?.argo_applications ?? [];
    const kusts = gitops?.flux_kustomizations ?? [];
    setSelectedArgoApp((prev) => (prev && apps.includes(prev) ? prev : apps[0] ?? ""));
    setSelectedFluxKust((prev) => (prev && kusts.includes(prev) ? prev : kusts[0] ?? ""));
  }, [gitops?.argo_applications, gitops?.flux_kustomizations]);

  const runSync = async () => {
    setSyncBusy(true);
    setSyncError(null);
    setSyncMessage(null);
    try {
      const result = await triggerGitOpsSync(
        {
          force: false,
          dry_run: false,
          argo_app: selectedArgoApp || undefined,
          flux_kustomization: selectedFluxKust || undefined,
        },
        scopeNamespace,
      );
      const parts = ["GitOps sync requested"];
      if (result.argo_sync_triggered) parts.push("Argo CD refresh sent");
      if (result.flux_reconcile_triggered) parts.push("Flux reconcile annotated");
      setSyncMessage(result.message ?? parts.join(" · "));
      await load();
    } catch (e) {
      setSyncError(e instanceof Error ? e.message : "GitOps sync failed");
    } finally {
      setSyncBusy(false);
    }
  };

  const runImageImport = async () => {
    const name = importForm.name.trim();
    if (!name) return;
    const ns = scopeNamespace === "all" ? "default" : scopeNamespace;
    const url = importForm.url.trim();
    const registry = importForm.registry.trim();
    if (!url && !registry) {
      setImagesError("Provide HTTP(S) URL or container registry image");
      return;
    }
    setImportBusy(true);
    setImportMessage(null);
    setImagesError(null);
    try {
      const result = await importDataVolume({
        name,
        namespace: ns,
        url: url || undefined,
        registry: registry || undefined,
        size: importForm.size.trim() || "20Gi",
      });
      setImportMessage(result.message ?? `Import started: ${name}`);
      setImportForm({ name: "", url: "", registry: "", size: "20Gi" });
      await load();
    } catch (e) {
      setImagesError(e instanceof Error ? e.message : "Image import failed");
    } finally {
      setImportBusy(false);
    }
  };

  const provisionTenant = async () => {
    if (!tenantForm.id.trim() || !tenantForm.display_name.trim()) return;
    setTenantBusy(true);
    setTenantsError(null);
    try {
      await createTenant({
        id: tenantForm.id.trim(),
        display_name: tenantForm.display_name.trim(),
        owner_email: tenantForm.owner_email.trim() || "ops@example.com",
        bootstrap_namespace: true,
      });
      setTenantForm({ id: "", display_name: "", owner_email: "" });
      await load();
    } catch (e) {
      setTenantsError(e instanceof Error ? e.message : "Tenant create failed");
    } finally {
      setTenantBusy(false);
    }
  };

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Platform</h2>
          <p style={subtitle}>
            {loading
              ? "Loading GitOps and CRD inventory…"
              : `GitOps: ${gitops?.sync_status ?? "unknown"} · ${crds.length} CRD(s) · namespace scope: ${scopeNamespace}`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          {(gitops?.argo_applications?.length ?? 0) > 0 ? (
            <label style={inlineLabel}>
              Argo app
              <select
                value={selectedArgoApp}
                onChange={(e) => setSelectedArgoApp(e.target.value)}
                style={inlineSelect}
              >
                {gitops!.argo_applications!.map((app) => (
                  <option key={app} value={app}>
                    {app}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          {(gitops?.flux_kustomizations?.length ?? 0) > 0 ? (
            <label style={inlineLabel}>
              Flux kustomization
              <select
                value={selectedFluxKust}
                onChange={(e) => setSelectedFluxKust(e.target.value)}
                style={inlineSelect}
              >
                {gitops!.flux_kustomizations!.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
          <button type="button" style={syncBtn} onClick={() => void runSync()} disabled={syncBusy || loading}>
            {syncBusy ? "Syncing…" : "Sync now"}
          </button>
        </div>
      </div>

      {syncError ? <ErrorBanner message={syncError} /> : null}
      {syncMessage && !syncError ? (
        <p style={successNote}>{syncMessage}</p>
      ) : null}

      <section style={section}>
        <h3 style={sectionTitle}>GitOps status</h3>
        {gitopsError ? <ErrorBanner message={gitopsError} /> : null}
        {gitops ? (
          <>
            <CapabilityBanner context={gitops.vmrogue_context} />
            <div style={statGrid}>
              <div style={statCard}>
                <div style={statLabel}>Repo</div>
                <div style={statValue}>{gitops.repo_url || "Not configured"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Branch</div>
                <div style={statValue}>{gitops.branch || "—"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Sync</div>
                <div style={{ ...statValue, ...syncStyle(gitops.sync_status) }}>{gitops.sync_status}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Drift</div>
                <div style={statValue}>{gitops.drift_detected ? "Detected" : "Clear"}</div>
              </div>
            </div>
            {(gitops.argo_applications?.length ?? 0) > 0 ? (
              <p style={note}>
                <strong>Argo CD applications:</strong> {gitops.argo_applications!.join(", ")}
              </p>
            ) : null}
            {(gitops.flux_kustomizations?.length ?? 0) > 0 ? (
              <p style={note}>
                <strong>Flux kustomizations:</strong> {gitops.flux_kustomizations!.join(", ")}
              </p>
            ) : null}
            {gitops.note ? <p style={note}>{gitops.note}</p> : null}
          </>
        ) : !gitopsError ? (
          <p style={muted}>{loading ? "Loading…" : "No GitOps status returned."}</p>
        ) : null}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Custom resource definitions</h3>
        {crdsError ? <ErrorBanner message={crdsError} /> : null}
        {crds.length === 0 && !loading && !crdsError ? (
          <p style={muted}>No CRDs returned (check API RBAC for apiextensions.k8s.io).</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Group</th>
                  <th style={th}>Version</th>
                  <th style={th}>Kind</th>
                  <th style={th}>Scope</th>
                  <th style={th}>Instances</th>
                </tr>
              </thead>
              <tbody>
                {crds.map((cr) => (
                  <tr key={cr.name} style={tr}>
                    <td style={td}><strong>{cr.name}</strong></td>
                    <td style={tdMono}>{cr.group}</td>
                    <td style={td}>{cr.version}</td>
                    <td style={td}>{cr.kind}</td>
                    <td style={td}>{cr.scope}</td>
                    <td style={td}>{cr.instance_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Tenants</h3>
        {tenantsError ? <ErrorBanner message={tenantsError} /> : null}
        <div style={statGrid}>
          {tenants.map((t) => (
            <div key={t.id} style={statCard}>
              <div style={statLabel}>{t.display_name}</div>
              <div style={statValue}>{t.id}</div>
              <p style={note}>
                {t.namespaces.join(", ") || "no namespaces"} · quota {t.cpu_quota} CPU / {t.memory_quota}
              </p>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 12 }}>
          <input
            placeholder="tenant-id"
            value={tenantForm.id}
            onChange={(e) => setTenantForm((f) => ({ ...f, id: e.target.value }))}
            style={refreshBtn}
          />
          <input
            placeholder="Display name"
            value={tenantForm.display_name}
            onChange={(e) => setTenantForm((f) => ({ ...f, display_name: e.target.value }))}
            style={refreshBtn}
          />
          <input
            placeholder="owner@email"
            value={tenantForm.owner_email}
            onChange={(e) => setTenantForm((f) => ({ ...f, owner_email: e.target.value }))}
            style={refreshBtn}
          />
          <button type="button" style={syncBtn} disabled={tenantBusy} onClick={() => void provisionTenant()}>
            {tenantBusy ? "Creating…" : "Bootstrap tenant"}
          </button>
        </div>
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Golden image catalog</h3>
        {imagesError ? <ErrorBanner message={imagesError} /> : null}
        {images ? (
          <>
            <CapabilityBanner context={images.vmrogue_context} />
            {images.images.length === 0 ? (
              <p style={muted}>No DataVolumes or golden-image PVCs in scope.</p>
            ) : (
              <div style={tableWrap}>
                <table style={table}>
                  <thead>
                    <tr>
                      <th style={th}>Name</th>
                      <th style={th}>NS</th>
                      <th style={th}>Kind</th>
                      <th style={th}>Source</th>
                      <th style={th}>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {images.images.map((img) => (
                      <tr key={`${img.namespace}-${img.name}`} style={tr}>
                        <td style={td}><strong>{img.name}</strong></td>
                        <td style={tdMono}>{img.namespace}</td>
                        <td style={td}>{img.kind}</td>
                        <td style={td}>{img.source_type}</td>
                        <td style={td}>{img.status}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        ) : (
          <p style={muted}>{loading ? "Loading…" : "No image catalog data"}</p>
        )}
        <div style={{ marginTop: 16 }}>
          <h4 style={{ margin: "0 0 8px", fontSize: 14, color: "#374151" }}>Import CDI DataVolume</h4>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            <input
              placeholder="volume name"
              value={importForm.name}
              onChange={(e) => setImportForm((f) => ({ ...f, name: e.target.value }))}
              style={refreshBtn}
            />
            <input
              placeholder="https://…/image.qcow2"
              value={importForm.url}
              onChange={(e) => setImportForm((f) => ({ ...f, url: e.target.value }))}
              style={{ ...refreshBtn, minWidth: 220 }}
            />
            <input
              placeholder="registry image (if no URL)"
              value={importForm.registry}
              onChange={(e) => setImportForm((f) => ({ ...f, registry: e.target.value }))}
              style={{ ...refreshBtn, minWidth: 200 }}
            />
            <input
              placeholder="20Gi"
              value={importForm.size}
              onChange={(e) => setImportForm((f) => ({ ...f, size: e.target.value }))}
              style={{ ...refreshBtn, width: 72 }}
            />
            <button type="button" style={syncBtn} disabled={importBusy} onClick={() => void runImageImport()}>
              {importBusy ? "Importing…" : "Start import"}
            </button>
          </div>
          {importMessage ? <p style={successNote}>{importMessage}</p> : null}
        </div>
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Network policies</h3>
        {netpolError ? <ErrorBanner message={netpolError} /> : null}
        {netpols.length === 0 && !loading && !netpolError ? (
          <p style={muted}>No NetworkPolicies in scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Namespace</th>
                  <th style={th}>Types</th>
                  <th style={th}>Ingress</th>
                  <th style={th}>Egress</th>
                  <th style={th}>Created</th>
                </tr>
              </thead>
              <tbody>
                {netpols.map((p) => (
                  <tr key={`${p.namespace}-${p.name}`} style={tr}>
                    <td style={td}><strong>{p.name}</strong></td>
                    <td style={tdMono}>{p.namespace}</td>
                    <td style={td}>{(p.policy_types || []).join(", ") || "—"}</td>
                    <td style={td}>{p.ingress_rules}</td>
                    <td style={td}>{p.egress_rules}</td>
                    <td style={tdMono}>{p.created_at ? new Date(p.created_at).toLocaleString() : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <p style={note}>
        Velero cluster backups and KubeVirt snapshot schedules: open the{" "}
        <button type="button" style={linkBtn} onClick={() => requestVmrogueNav({ view: "backups" })}>
          Backups
        </button>{" "}
        sidebar.
      </p>

      <p style={footerNote}>
        Advanced VMRogue CRD YAML editors remain in the{" "}
        <a href="/dashboard" style={link}>classic dashboard</a>.
      </p>
    </div>
  );
}

function syncStyle(status: string): CSSProperties {
  if (status === "synced") return { color: "#15803d" };
  if (status === "out_of_sync") return { color: "#b45309" };
  if (status === "not_configured") return { color: "#6b7280" };
  return { color: "#1d4ed8" };
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 20, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const syncBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #f0583a", background: "#f0583a", color: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const inlineLabel: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 4,
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
};
const inlineSelect: CSSProperties = {
  padding: "6px 8px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  fontSize: 13,
  minWidth: 140,
};
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const muted: CSSProperties = { color: "#6b7280", fontSize: 14 };
const note: CSSProperties = { margin: "12px 0 0", fontSize: 12, color: "#6b7280", lineHeight: 1.45 };
const successNote: CSSProperties = { margin: "0 0 16px", fontSize: 13, color: "#15803d", fontWeight: 600 };
const statGrid: CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12,
};
const statCard: CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: "12px 14px",
};
const statLabel: CSSProperties = { fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase", letterSpacing: "0.04em" };
const statValue: CSSProperties = { marginTop: 6, fontSize: 14, fontWeight: 600, color: "#222324", wordBreak: "break-word" };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600, color: "#374151", whiteSpace: "nowrap",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const footerNote: CSSProperties = { marginTop: 8, fontSize: 12, color: "#9ca3af" };
const link: CSSProperties = { color: "#f0583a", fontWeight: 600 };
const linkBtn: CSSProperties = {
  border: "none", background: "none", color: "#c2410c", fontWeight: 600, cursor: "pointer", padding: 0, fontSize: 12,
};

export default PlatformPanel;
