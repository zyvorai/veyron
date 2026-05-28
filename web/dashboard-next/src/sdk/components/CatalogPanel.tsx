// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import {
  createCatalogProfile,
  createCatalogTemplate,
  createVmrogueVm,
  fetchCatalogProfiles,
  fetchCatalogTemplates,
  fetchProfiles,
  fetchTemplates,
  type CatalogProfileRecord,
  type CatalogTemplateRecord,
  type VmProfile,
  type VmTemplate,
} from "../../lib/api";
import { requestVmrogueNav } from "../../lib/nav";
import ErrorBanner from "./ErrorBanner";

type CatalogTab = "browse" | "create";

export function CatalogPanel() {
  const [tab, setTab] = useState<CatalogTab>("browse");
  const [templates, setTemplates] = useState<CatalogTemplateRecord[]>([]);
  const [profiles, setProfiles] = useState<CatalogProfileRecord[]>([]);
  const [embedded, setEmbedded] = useState<VmTemplate[]>([]);
  const [embeddedProfiles, setEmbeddedProfiles] = useState<VmProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [catalogMessage, setCatalogMessage] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState(false);

  const [tplForm, setTplForm] = useState({ name: "", from_embedded: "ubuntu", description: "" });
  const [profForm, setProfForm] = useState({
    name: "",
    cores: 2,
    memory: "4Gi",
    disk_size: "20Gi",
    description: "",
    from_embedded: "",
  });
  const [deployForm, setDeployForm] = useState({
    namespace: "default",
    name: "",
    template: "",
    profile: "",
  });

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    setCatalogMessage(null);
    const results = await Promise.allSettled([
      fetchCatalogTemplates(),
      fetchCatalogProfiles(),
      fetchTemplates(),
      fetchProfiles(),
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
      setDeployForm((f) => ({
        ...f,
        template: f.template || results[2].value[0]?.name || "ubuntu",
      }));
      setTplForm((f) => ({ ...f, from_embedded: f.from_embedded || results[2].value[0]?.name || "ubuntu" }));
    } else {
      setEmbedded([]);
    }
    if (results[3].status === "fulfilled") {
      setEmbeddedProfiles(results[3].value);
      setDeployForm((f) => ({
        ...f,
        profile: f.profile || results[3].value[0]?.name || "",
      }));
    } else {
      setEmbeddedProfiles([]);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const templateOptions = useMemo(() => {
    const crd = templates.map((t) => t.name);
    const emb = embedded.map((t) => t.name);
    return [...new Set([...crd, ...emb])].sort();
  }, [templates, embedded]);

  const profileOptions = useMemo(() => {
    const crd = profiles.map((p) => p.name);
    const emb = embeddedProfiles.map((p) => p.name);
    return [...new Set([...crd, ...emb])].sort();
  }, [profiles, embeddedProfiles]);

  const selectedProfile = useMemo(
    () => profiles.find((p) => p.name === deployForm.profile)
      ?? embeddedProfiles.find((p) => p.name === deployForm.profile),
    [profiles, embeddedProfiles, deployForm.profile],
  );

  const publishTemplate = async () => {
    const name = tplForm.name.trim();
    if (!name) {
      setActionMsg("Template CRD name required");
      return;
    }
    setActionBusy(true);
    setActionMsg(null);
    try {
      await createCatalogTemplate({
        name,
        from_embedded: tplForm.from_embedded,
        description: tplForm.description.trim() || undefined,
      });
      setActionMsg(`Published VMTemplate '${name}'`);
      setTplForm((f) => ({ ...f, name: "", description: "" }));
      await load();
    } catch (e) {
      setActionMsg(e instanceof Error ? e.message : "Publish failed");
    } finally {
      setActionBusy(false);
    }
  };

  const publishProfile = async () => {
    const name = profForm.name.trim();
    if (!name) {
      setActionMsg("Profile CRD name required");
      return;
    }
    setActionBusy(true);
    setActionMsg(null);
    try {
      await createCatalogProfile(
        profForm.from_embedded
          ? {
              name,
              from_embedded: profForm.from_embedded,
              cores: profForm.cores,
              memory: profForm.memory,
              disk_size: profForm.disk_size,
              description: profForm.description.trim() || undefined,
            }
          : {
              name,
              cores: profForm.cores,
              memory: profForm.memory,
              disk_size: profForm.disk_size,
              description: profForm.description.trim() || undefined,
            },
      );
      setActionMsg(`Created VMProfile '${name}'`);
      setProfForm((f) => ({ ...f, name: "", description: "" }));
      await load();
    } catch (e) {
      setActionMsg(e instanceof Error ? e.message : "Create profile failed");
    } finally {
      setActionBusy(false);
    }
  };

  const deployVmrogueVm = async () => {
    const vmName = deployForm.name.trim();
    const ns = deployForm.namespace.trim() || "default";
    if (!vmName) {
      setActionMsg("VM name required");
      return;
    }
    if (!deployForm.template) {
      setActionMsg("Template required");
      return;
    }
    const cores = selectedProfile
      ? ("cores" in selectedProfile ? selectedProfile.cores : selectedProfile.cpu_cores)
      : 2;
    const memory = selectedProfile?.memory ?? "4Gi";
    setActionBusy(true);
    setActionMsg(null);
    try {
      const created = await createVmrogueVm({
        namespace: ns,
        name: vmName,
        template: deployForm.template,
        profile: deployForm.profile.trim() || undefined,
        cpu: { cores, sockets: 1, threads: 1 },
        memory: { size: memory },
        running: true,
        allowInternet: true,
      });
      setActionMsg(`VMRogueVM ${created.namespace}/${created.name} created — operator will reconcile KubeVirt VM`);
      setDeployForm((f) => ({ ...f, name: "" }));
    } catch (e) {
      setActionMsg(e instanceof Error ? e.message : "Deploy failed");
    } finally {
      setActionBusy(false);
    }
  };

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
          <button type="button" style={secondaryBtn} onClick={() => requestVmrogueNav({ view: "platform" })}>
            Golden images
          </button>
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      <div style={tabRow}>
        {(
          [
            ["browse", "Browse"],
            ["create", "Create & deploy"],
          ] as const
        ).map(([t, label]) => (
          <button
            key={t}
            type="button"
            style={tab === t ? tabBtnActive : tabBtn}
            onClick={() => setTab(t)}
          >
            {label}
          </button>
        ))}
      </div>

      {error ? <ErrorBanner message={error} /> : null}
      {catalogMessage ? <p style={note}>{catalogMessage}</p> : null}
      {actionMsg ? (
        <p style={actionMsg.startsWith("VMRogueVM") || actionMsg.startsWith("Published") || actionMsg.startsWith("Created") ? successNote : note}>
          {actionMsg}
        </p>
      ) : null}

      {tab === "browse" ? (
        <>
          <p style={hint}>
            Operator resolves <code style={code}>template</code> + <code style={code}>profile</code> on{" "}
            <strong>VMRogueVM</strong>. Bulk sync: <code style={code}>vmrogue catalog sync</code> · YAML:{" "}
            <a href="/dashboard" style={link}>classic dashboard</a>.
          </p>

          <section style={section}>
            <h3 style={sectionTitle}>VMTemplate CRDs</h3>
            {templates.length === 0 && !loading ? (
              <p style={muted}>No VMTemplate resources. Use <strong>Create & deploy</strong> to publish from embedded templates.</p>
            ) : (
              <CatalogTable
                headers={["Name", "Family", "Tags", "Description"]}
                rows={templates.map((t) => [t.name, t.family || "—", t.tags?.join(", ") || "—", t.description || "—"])}
              />
            )}
          </section>

          <section style={section}>
            <h3 style={sectionTitle}>VMProfile CRDs</h3>
            {profiles.length === 0 && !loading ? (
              <p style={muted}>No VMProfile resources yet.</p>
            ) : (
              <CatalogTable
                headers={["Name", "CPU", "Memory", "Disk", "Description"]}
                rows={profiles.map((p) => [p.name, String(p.cores), p.memory, p.disk_size, p.description || "—"])}
                monoCols={[2, 3]}
              />
            )}
          </section>

          <section style={section}>
            <h3 style={sectionTitle}>Embedded OS templates</h3>
            <p style={hint}>
              <code style={code}>GET /api/v1/templates</code>
              {crdCount > 0 ? " — fallback names when CRD catalog is partial." : " — used until CRDs are published."}
            </p>
            {embedded.length === 0 && !loading ? (
              <p style={muted}>No embedded templates returned.</p>
            ) : (
              <CatalogTable
                headers={["Name", "OS", "Defaults", "Description"]}
                rows={embedded.slice(0, 30).map((t) => [
                  t.name,
                  t.os_type,
                  `${t.default_cpus} CPU · ${t.default_memory} · ${t.default_disk_size}`,
                  t.description || "—",
                ])}
                monoCols={[2]}
              />
            )}
          </section>
        </>
      ) : (
        <>
          <section style={section}>
            <h3 style={sectionTitle}>Publish VMTemplate CRD</h3>
            <p style={hint}>Copies a built-in OS template spec into cluster-scoped <strong>VMTemplate</strong>.</p>
            <div style={formRow}>
              <label style={label}>
                CRD name
                <input style={input} value={tplForm.name} onChange={(e) => setTplForm((f) => ({ ...f, name: e.target.value }))} placeholder="ubuntu-22-04" />
              </label>
              <label style={label}>
                From embedded template
                <select style={input} value={tplForm.from_embedded} onChange={(e) => setTplForm((f) => ({ ...f, from_embedded: e.target.value }))}>
                  {embedded.map((t) => (
                    <option key={t.name} value={t.name}>{t.name}</option>
                  ))}
                </select>
              </label>
              <label style={{ ...label, flex: 1, minWidth: 200 }}>
                Description
                <input style={input} value={tplForm.description} onChange={(e) => setTplForm((f) => ({ ...f, description: e.target.value }))} />
              </label>
              <button type="button" style={primaryBtn} disabled={actionBusy} onClick={() => void publishTemplate()}>
                {actionBusy ? "Publishing…" : "Publish template"}
              </button>
            </div>
          </section>

          <section style={section}>
            <h3 style={sectionTitle}>Create VMProfile CRD</h3>
            <div style={formRow}>
              <label style={label}>
                CRD name
                <input style={input} value={profForm.name} onChange={(e) => setProfForm((f) => ({ ...f, name: e.target.value }))} placeholder="small" />
              </label>
              <label style={label}>
                Copy from embedded (optional)
                <select
                  style={input}
                  value={profForm.from_embedded}
                  onChange={(e) => {
                    const v = e.target.value;
                    const p = embeddedProfiles.find((row) => row.name === v);
                    setProfForm((f) => ({
                      ...f,
                      from_embedded: v,
                      cores: p?.cpu_cores ?? f.cores,
                      memory: p?.memory ?? f.memory,
                      disk_size: p?.disk_size ?? f.disk_size,
                    }));
                  }}
                >
                  <option value="">— custom —</option>
                  {embeddedProfiles.map((p) => (
                    <option key={p.name} value={p.name}>{p.name}</option>
                  ))}
                </select>
              </label>
              <label style={label}>
                CPU cores
                <input type="number" min={1} style={input} value={profForm.cores} onChange={(e) => setProfForm((f) => ({ ...f, cores: Number(e.target.value) || 1 }))} />
              </label>
              <label style={label}>
                Memory
                <input style={input} value={profForm.memory} onChange={(e) => setProfForm((f) => ({ ...f, memory: e.target.value }))} />
              </label>
              <label style={label}>
                Disk
                <input style={input} value={profForm.disk_size} onChange={(e) => setProfForm((f) => ({ ...f, disk_size: e.target.value }))} />
              </label>
              <button type="button" style={primaryBtn} disabled={actionBusy} onClick={() => void publishProfile()}>
                {actionBusy ? "Creating…" : "Create profile"}
              </button>
            </div>
          </section>

          <section style={section}>
            <h3 style={sectionTitle}>Deploy VMRogueVM from catalog</h3>
            <p style={hint}>
              Creates a <strong>VMRogueVM</strong> CR; the operator reconciles a KubeVirt <code style={code}>VirtualMachine</code>.
            </p>
            <div style={formRow}>
              <label style={label}>
                Namespace
                <input style={input} value={deployForm.namespace} onChange={(e) => setDeployForm((f) => ({ ...f, namespace: e.target.value }))} />
              </label>
              <label style={label}>
                VM name
                <input style={input} value={deployForm.name} onChange={(e) => setDeployForm((f) => ({ ...f, name: e.target.value }))} placeholder="my-vm" />
              </label>
              <label style={label}>
                Template
                <select style={input} value={deployForm.template} onChange={(e) => setDeployForm((f) => ({ ...f, template: e.target.value }))}>
                  {templateOptions.map((n) => (
                    <option key={n} value={n}>{n}</option>
                  ))}
                </select>
              </label>
              <label style={label}>
                Profile (optional)
                <select style={input} value={deployForm.profile} onChange={(e) => setDeployForm((f) => ({ ...f, profile: e.target.value }))}>
                  <option value="">— default sizing —</option>
                  {profileOptions.map((n) => (
                    <option key={n} value={n}>{n}</option>
                  ))}
                </select>
              </label>
              <button type="button" style={primaryBtn} disabled={actionBusy} onClick={() => void deployVmrogueVm()}>
                {actionBusy ? "Deploying…" : "Deploy VMRogueVM"}
              </button>
            </div>
            {selectedProfile ? (
              <p style={hint}>
                Profile sizing:{" "}
                {"cores" in selectedProfile ? selectedProfile.cores : selectedProfile.cpu_cores} CPU ·{" "}
                {selectedProfile.memory}
                {"disk_size" in selectedProfile ? ` · ${selectedProfile.disk_size}` : ""}
              </p>
            ) : null}
          </section>
        </>
      )}
    </div>
  );
}

function CatalogTable({
  headers,
  rows,
  monoCols = [],
}: {
  headers: string[];
  rows: string[][];
  monoCols?: number[];
}) {
  return (
    <div style={tableWrap}>
      <table style={table}>
        <thead>
          <tr>
            {headers.map((h) => (
              <th key={h} style={th}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row[0]} style={tr}>
              {row.map((cell, i) => (
                <td key={i} style={i === 0 ? tdStrong : monoCols.includes(i) ? tdMono : td}>{cell}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default CatalogPanel;

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1200, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start",
  gap: 16, marginBottom: 16, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, fontWeight: 700, color: "#111827" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const tabRow: CSSProperties = { display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 16 };
const tabBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const tabBtnActive: CSSProperties = { ...tabBtn, borderColor: "#f0583a", background: "rgba(240,88,58,0.08)", color: "#c2410c" };
const section: CSSProperties = {
  background: "#fff", borderRadius: 10, border: "1px solid #e5e7eb",
  padding: "16px 18px", marginBottom: 16,
};
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 700, color: "#374151" };
const formRow: CSSProperties = { display: "flex", flexWrap: "wrap", gap: 12, alignItems: "flex-end" };
const label: CSSProperties = { display: "flex", flexDirection: "column", gap: 4, fontSize: 12, fontWeight: 600, color: "#374151" };
const input: CSSProperties = {
  padding: "8px 10px", borderRadius: 6, border: "1px solid #d1d5db", fontSize: 13, minWidth: 140,
};
const tableWrap: CSSProperties = { overflowX: "auto" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "8px 10px", borderBottom: "2px solid #e5e7eb",
  color: "#6b7280", fontWeight: 600, fontSize: 11, textTransform: "uppercase",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "10px", verticalAlign: "top", color: "#374151" };
const tdStrong: CSSProperties = { ...td, fontWeight: 700 };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const muted: CSSProperties = { margin: 0, fontSize: 13, color: "#9ca3af" };
const hint: CSSProperties = { margin: "0 0 12px", fontSize: 12, color: "#6b7280", lineHeight: 1.5 };
const note: CSSProperties = { margin: "0 0 12px", fontSize: 12, color: "#92400e", background: "#fffbeb", padding: "8px 12px", borderRadius: 6 };
const successNote: CSSProperties = { margin: "0 0 12px", fontSize: 12, color: "#166534", background: "#dcfce7", padding: "8px 12px", borderRadius: 6 };
const code: CSSProperties = { fontFamily: "ui-monospace, monospace", fontSize: 11, background: "#f3f4f6", padding: "1px 4px", borderRadius: 3 };
const link: CSSProperties = { color: "#c2410c", fontWeight: 600 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db",
  background: "#fff", fontSize: 13, fontWeight: 600, cursor: "pointer",
};
const secondaryBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#f0583a", color: "#c2410c", background: "#fff7ed",
};
const primaryBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#f0583a", background: "#f0583a", color: "#fff",
};
