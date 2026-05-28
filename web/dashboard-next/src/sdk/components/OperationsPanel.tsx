// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import {
  createAutoscalerPolicy,
  drApply,
  drExportManifests,
  drFailover,
  fetchAutoscalerPolicies,
  fetchCiliumPolicies,
  fetchCiliumStatus,
  fetchCustomDashboards,
  fetchResourceHeatmap,
  fetchVmInventory,
  type AutoscalerPolicyRecord,
  type CiliumPolicyRecord,
  type CiliumStatusRecord,
  type CustomDashboardRecord,
  type DrExportPayload,
  type NodeHeatmapEntry,
  type ResourceHeatmapResponse,
  type VmRecord,
} from "../../lib/api";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

type Props = { scopeNamespace?: string };

type OpsTab = "dr" | "heatmap" | "dashboards" | "network" | "scaling";
type DrStep = "select" | "exporting" | "review" | "acting";
type DrView = "summary" | "json";

export function OperationsPanel({ scopeNamespace = "all" }: Props) {
  const [tab, setTab] = useState<OpsTab>("dr");
  const [heatmap, setHeatmap] = useState<ResourceHeatmapResponse | null>(null);
  const [dashboards, setDashboards] = useState<CustomDashboardRecord[]>([]);
  const [ciliumStatus, setCiliumStatus] = useState<CiliumStatusRecord | null>(null);
  const [ciliumPolicies, setCiliumPolicies] = useState<CiliumPolicyRecord[]>([]);
  const [autoscaler, setAutoscaler] = useState<AutoscalerPolicyRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [tabError, setTabError] = useState<string | null>(null);
  const [scaleBusy, setScaleBusy] = useState(false);
  const [scaleForm, setScaleForm] = useState({
    name: "",
    target_name: "",
    min_replicas: 1,
    max_replicas: 3,
    cpu_threshold: 80,
  });

  const [drNs, setDrNs] = useState(scopeNamespace === "all" ? "default" : scopeNamespace);
  const [drVm, setDrVm] = useState("");
  const [drTarget, setDrTarget] = useState("");
  const [drRestoreSnap, setDrRestoreSnap] = useState(false);
  const [drOutput, setDrOutput] = useState("");
  const [drBusy, setDrBusy] = useState(false);
  const [drExport, setDrExport] = useState<DrExportPayload | null>(null);
  const [drStep, setDrStep] = useState<DrStep>("select");
  const [drView, setDrView] = useState<DrView>("summary");
  const [drProgress, setDrProgress] = useState<string | null>(null);
  const [drVmList, setDrVmList] = useState<VmRecord[]>([]);
  const [drVmLoading, setDrVmLoading] = useState(false);

  useEffect(() => {
    setDrNs(scopeNamespace === "all" ? "default" : scopeNamespace);
  }, [scopeNamespace]);

  useEffect(() => {
    if (tab !== "dr") return;
    const ns = drNs.trim() || "default";
    setDrVmLoading(true);
    void fetchVmInventory(ns)
      .then((rows) => setDrVmList(rows))
      .catch(() => setDrVmList([]))
      .finally(() => setDrVmLoading(false));
  }, [tab, drNs]);

  const loadTab = useCallback(async () => {
    setLoading(true);
    setTabError(null);
    try {
      if (tab === "heatmap") {
        setHeatmap(await fetchResourceHeatmap());
      } else if (tab === "dashboards") {
        setDashboards(await fetchCustomDashboards(scopeNamespace));
      } else if (tab === "network") {
        const [st, pol] = await Promise.all([
          fetchCiliumStatus(),
          fetchCiliumPolicies(scopeNamespace),
        ]);
        setCiliumStatus(st);
        setCiliumPolicies(pol);
      } else if (tab === "scaling") {
        setAutoscaler(await fetchAutoscalerPolicies(scopeNamespace));
      }
    } catch (e) {
      setTabError(e instanceof Error ? e.message : "Failed to load");
      if (tab === "heatmap") setHeatmap(null);
      if (tab === "dashboards") setDashboards([]);
      if (tab === "network") {
        setCiliumStatus(null);
        setCiliumPolicies([]);
      }
      if (tab === "scaling") setAutoscaler([]);
    } finally {
      setLoading(false);
    }
  }, [tab, scopeNamespace]);

  useEffect(() => {
    if (tab === "dr") return;
    void loadTab();
  }, [tab, loadTab]);

  const sortedNodes = useMemo(() => {
    const nodes = heatmap?.nodes ?? [];
    return [...nodes].sort((a, b) => (b.heat_score ?? 0) - (a.heat_score ?? 0));
  }, [heatmap]);

  const runExport = async () => {
    const ns = drNs.trim() || "default";
    const vm = drVm.trim();
    if (!vm) {
      setDrOutput("VM name required");
      setDrStep("select");
      return;
    }
    setDrBusy(true);
    setDrStep("exporting");
    setDrProgress("Fetching VirtualMachine manifest from Kubernetes…");
    setDrOutput("");
    setDrExport(null);
    try {
      setDrProgress("Listing KubeVirt snapshots for VM…");
      const data = await drExportManifests(ns, vm);
      setDrExport(data);
      setDrOutput(JSON.stringify(data, null, 2));
      setDrStep("review");
      setDrProgress(null);
    } catch (e) {
      setDrExport(null);
      setDrStep("select");
      setDrProgress(null);
      setDrOutput(e instanceof Error ? e.message : "Export failed");
    } finally {
      setDrBusy(false);
    }
  };

  const copyExportJson = async () => {
    if (!drOutput) return;
    try {
      await navigator.clipboard.writeText(drOutput);
      setDrProgress("Copied export JSON to clipboard");
      window.setTimeout(() => setDrProgress(null), 2500);
    } catch {
      setDrProgress("Could not copy to clipboard");
    }
  };

  const drSnapshotCount = Array.isArray(drExport?.snapshots) ? drExport.snapshots.length : 0;
  const drManifestKind =
    drExport?.virtual_machine && typeof drExport.virtual_machine === "object"
      ? String((drExport.virtual_machine as { kind?: string }).kind ?? "VirtualMachine")
      : null;

  const downloadExport = () => {
    if (!drOutput) return;
    const blob = new Blob([drOutput], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `dr-${drNs}-${drVm || "vm"}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const runFailover = async (dryRun: boolean) => {
    const ns = drNs.trim() || "default";
    const vm = drVm.trim();
    if (!vm) {
      setDrOutput("VM name required");
      return;
    }
    if (!dryRun && !window.confirm(`Run DR failover for ${ns}/${vm}?`)) return;
    setDrBusy(true);
    setDrStep("acting");
    setDrProgress(dryRun ? "Running failover dry-run…" : "Running DR failover…");
    try {
      const data = await drFailover({
        namespace: ns,
        vm_name: vm,
        dry_run: dryRun,
        target_kubeconfig_context: drTarget.trim() || null,
      });
      setDrOutput(JSON.stringify(data, null, 2));
      setDrStep("review");
    } catch (e) {
      setDrOutput(e instanceof Error ? e.message : "Failover failed");
      setDrStep("review");
    } finally {
      setDrBusy(false);
      setDrProgress(null);
    }
  };

  const runApply = async (dryRun: boolean) => {
    const ns = drNs.trim() || "default";
    const vm = drVm.trim();
    if (!vm) {
      setDrOutput("VM name required");
      setDrStep("select");
      return;
    }
    if (!dryRun && !window.confirm(`Apply DR manifest for ${ns}/${vm} on this cluster?`)) return;
    setDrBusy(true);
    setDrStep("acting");
    setDrProgress(dryRun ? "Running apply dry-run…" : "Applying VirtualMachine on cluster…");
    try {
      const data = await drApply({
        namespace: ns,
        vm_name: vm,
        dry_run: dryRun,
        target_name: drTarget.trim() || undefined,
        restore_latest_snapshot: drRestoreSnap,
        virtual_machine: drExport?.virtual_machine,
      });
      setDrOutput(JSON.stringify(data, null, 2));
      setDrStep("review");
    } catch (e) {
      setDrOutput(e instanceof Error ? e.message : "Apply failed");
      setDrStep("review");
    } finally {
      setDrBusy(false);
      setDrProgress(null);
    }
  };

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Operations</h2>
          <p style={subtitle}>Disaster recovery, capacity heatmap, and saved dashboards</p>
        </div>
      </div>

      <div style={tabRow}>
        {(
          [
            ["dr", "Disaster recovery"],
            ["heatmap", "Heatmap"],
            ["dashboards", "Dashboards"],
            ["network", "Cilium / network"],
            ["scaling", "Autoscaler"],
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

      {tab === "dr" ? (
        <section style={section}>
          <p style={note}>
            Export a KubeVirt VM manifest (+ snapshots metadata), then apply or failover on this or another cluster.
          </p>

          <DrStepBar step={drStep} />

          {drProgress ? (
            <p style={progressNote} role="status">
              {drBusy ? "⏳ " : ""}{drProgress}
            </p>
          ) : null}

          <div style={formRow}>
            <label style={label}>
              Namespace
              <input style={input} value={drNs} onChange={(e) => { setDrNs(e.target.value); setDrStep("select"); }} />
            </label>
            <label style={label}>
              VM name
              <input
                style={input}
                list="dr-vm-options"
                value={drVm}
                onChange={(e) => { setDrVm(e.target.value); setDrStep("select"); }}
                placeholder="my-vm"
              />
              <datalist id="dr-vm-options">
                {drVmList.map((v) => (
                  <option key={`${v.namespace}/${v.name}`} value={v.name}>
                    {v.status}
                  </option>
                ))}
              </datalist>
            </label>
            <label style={label}>
              Target name / context (optional)
              <input style={input} value={drTarget} onChange={(e) => setDrTarget(e.target.value)} />
            </label>
          </div>
          {drVmLoading ? <p style={hint}>Loading VMs in {drNs}…</p> : null}
          {!drVmLoading && drVmList.length > 0 ? (
            <p style={hint}>{drVmList.length} VM(s) in namespace — pick from the VM name suggestions.</p>
          ) : null}

          <label style={{ ...label, display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <input
              type="checkbox"
              checked={drRestoreSnap}
              onChange={(e) => setDrRestoreSnap(e.target.checked)}
            />
            Restore latest snapshot on apply
          </label>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginBottom: 12 }}>
            <button type="button" style={primaryBtn} disabled={drBusy} onClick={() => void runExport()}>
              {drBusy && drStep === "exporting" ? "Exporting…" : "1. Export manifest"}
            </button>
            <button type="button" style={refreshBtn} disabled={drBusy || !drOutput} onClick={downloadExport}>
              Download JSON
            </button>
            <button type="button" style={refreshBtn} disabled={!drOutput} onClick={() => void copyExportJson()}>
              Copy JSON
            </button>
            <button type="button" style={refreshBtn} disabled={drBusy || !drExport} onClick={() => void runApply(true)}>
              2. Apply (dry-run)
            </button>
            <button type="button" style={primaryBtn} disabled={drBusy || !drExport} onClick={() => void runApply(false)}>
              Apply
            </button>
            <button type="button" style={refreshBtn} disabled={drBusy} onClick={() => void runFailover(true)}>
              Failover (dry-run)
            </button>
            <button type="button" style={warnBtn} disabled={drBusy} onClick={() => void runFailover(false)}>
              Failover
            </button>
          </div>

          {drExport && drView === "summary" ? (
            <div style={summaryCard}>
              <div style={summaryGrid}>
                <div><span style={summaryLabel}>Namespace</span><div style={summaryValue}>{drExport.namespace}</div></div>
                <div><span style={summaryLabel}>VM</span><div style={summaryValue}>{drExport.vm_name}</div></div>
                <div><span style={summaryLabel}>Manifest</span><div style={summaryValue}>{drManifestKind ?? "VirtualMachine"}</div></div>
                <div><span style={summaryLabel}>Snapshots</span><div style={summaryValue}>{drSnapshotCount}</div></div>
              </div>
              {drExport.note ? <p style={hint}>{drExport.note}</p> : null}
            </div>
          ) : null}

          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <button type="button" style={drView === "summary" ? tabBtnActive : tabBtn} onClick={() => setDrView("summary")}>
              Summary
            </button>
            <button type="button" style={drView === "json" ? tabBtnActive : tabBtn} onClick={() => setDrView("json")}>
              Raw JSON
            </button>
          </div>
          {drView === "json" ? (
            <pre style={pre}>{drOutput || "Run export to fetch GET /api/v1/dr/export payload…"}</pre>
          ) : !drExport ? (
            <p style={muted}>Select a VM and export to review manifest metadata before apply or failover.</p>
          ) : null}
        </section>
      ) : null}

      {tab === "heatmap" ? (
        <section style={section}>
          {tabError ? <ErrorBanner message={tabError} /> : null}
          {heatmap ? <CapabilityBanner context={heatmap.vmrogue_context} /> : null}
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Node</th>
                  <th style={th}>CPU %</th>
                  <th style={th}>Mem %</th>
                  <th style={th}>Disk %</th>
                  <th style={th}>Net %</th>
                  <th style={th}>VMs</th>
                  <th style={th}>Heat</th>
                </tr>
              </thead>
              <tbody>
                {sortedNodes.length === 0 ? (
                  <tr>
                    <td colSpan={7} style={emptyTd}>
                      {loading ? "Loading…" : "No nodes"}
                    </td>
                  </tr>
                ) : (
                  sortedNodes.map((n: NodeHeatmapEntry) => (
                    <tr key={n.node_name} style={tr}>
                      <td style={td}><strong>{n.node_name}</strong></td>
                      <td style={td}>{n.cpu_utilization.toFixed(1)}</td>
                      <td style={td}>{n.memory_utilization.toFixed(1)}</td>
                      <td style={td}>{n.disk_utilization.toFixed(1)}</td>
                      <td style={td}>{n.network_utilization.toFixed(1)}</td>
                      <td style={td}>{n.vm_count}</td>
                      <td style={td}>{n.heat_score.toFixed(2)}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}

      {tab === "network" ? (
        <section style={section}>
          {tabError ? <ErrorBanner message={tabError} /> : null}
          {ciliumStatus ? (
            <div style={statGrid}>
              <div style={statCard}>
                <div style={statLabel}>Cilium</div>
                <div style={statValue}>{ciliumStatus.version || "detected"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Agents</div>
                <div style={statValue}>
                  {ciliumStatus.healthy_agents}/{ciliumStatus.agent_count}
                </div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Hubble</div>
                <div style={statValue}>{ciliumStatus.hubble_enabled ? "on" : "off"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Policies</div>
                <div style={statValue}>{ciliumPolicies.length}</div>
              </div>
            </div>
          ) : (
            <p style={note}>{loading ? "Loading…" : "Cilium not detected on this cluster"}</p>
          )}
          <div style={{ ...tableWrap, marginTop: 16 }}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>NS</th>
                  <th style={th}>Kind</th>
                  <th style={th}>In</th>
                  <th style={th}>Out</th>
                </tr>
              </thead>
              <tbody>
                {ciliumPolicies.length === 0 ? (
                  <tr>
                    <td colSpan={5} style={emptyTd}>
                      {loading ? "Loading…" : "No Cilium or NetworkPolicy rows"}
                    </td>
                  </tr>
                ) : (
                  ciliumPolicies.map((p) => (
                    <tr key={`${p.policy_kind}-${p.namespace}-${p.name}`} style={tr}>
                      <td style={td}><strong>{p.name}</strong></td>
                      <td style={tdMono}>{p.namespace}</td>
                      <td style={td}>{p.policy_kind}</td>
                      <td style={td}>{p.ingress_rules}</td>
                      <td style={td}>{p.egress_rules}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}

      {tab === "scaling" ? (
        <section style={section}>
          {tabError ? <ErrorBanner message={tabError} /> : null}
          <p style={note}>HPA targets and KubeVirt VM autoscaler ConfigMaps (vmrogue.io/type=vm-autoscaler).</p>
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Target</th>
                  <th style={th}>Min</th>
                  <th style={th}>Current</th>
                  <th style={th}>Max</th>
                  <th style={th}>CPU %</th>
                </tr>
              </thead>
              <tbody>
                {autoscaler.length === 0 ? (
                  <tr>
                    <td colSpan={6} style={emptyTd}>
                      {loading ? "Loading…" : "No autoscaler policies"}
                    </td>
                  </tr>
                ) : (
                  autoscaler.map((p) => (
                    <tr key={`${p.namespace}-${p.name}`} style={tr}>
                      <td style={td}><strong>{p.name}</strong></td>
                      <td style={tdMono}>{p.target_kind}/{p.target_name}</td>
                      <td style={td}>{p.min_replicas}</td>
                      <td style={td}>{p.current_replicas}</td>
                      <td style={td}>{p.max_replicas}</td>
                      <td style={td}>{p.cpu_threshold ?? "—"}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
          <div style={{ marginTop: 16 }}>
            <h4 style={{ margin: "0 0 8px", fontSize: 14 }}>Create HPA policy</h4>
            <div style={formRow}>
              <input
                placeholder="policy name"
                value={scaleForm.name}
                onChange={(e) => setScaleForm((f) => ({ ...f, name: e.target.value }))}
                style={input}
              />
              <input
                placeholder="deployment name"
                value={scaleForm.target_name}
                onChange={(e) => setScaleForm((f) => ({ ...f, target_name: e.target.value }))}
                style={input}
              />
              <input
                type="number"
                placeholder="min"
                value={scaleForm.min_replicas}
                onChange={(e) =>
                  setScaleForm((f) => ({ ...f, min_replicas: Number(e.target.value) || 1 }))
                }
                style={{ ...input, width: 72 }}
              />
              <input
                type="number"
                placeholder="max"
                value={scaleForm.max_replicas}
                onChange={(e) =>
                  setScaleForm((f) => ({ ...f, max_replicas: Number(e.target.value) || 1 }))
                }
                style={{ ...input, width: 72 }}
              />
              <button
                type="button"
                style={primaryBtn}
                disabled={scaleBusy}
                onClick={() => {
                  const ns = scopeNamespace === "all" ? "default" : scopeNamespace;
                  if (!scaleForm.name.trim() || !scaleForm.target_name.trim()) return;
                  setScaleBusy(true);
                  void createAutoscalerPolicy(ns, {
                    name: scaleForm.name.trim(),
                    target_name: scaleForm.target_name.trim(),
                    min_replicas: scaleForm.min_replicas,
                    max_replicas: scaleForm.max_replicas,
                    cpu_threshold: scaleForm.cpu_threshold,
                  })
                    .then(() => loadTab())
                    .catch((e) => setTabError(e instanceof Error ? e.message : "Create failed"))
                    .finally(() => setScaleBusy(false));
                }}
              >
                {scaleBusy ? "Creating…" : "Create HPA"}
              </button>
            </div>
          </div>
        </section>
      ) : null}

      {tab === "dashboards" ? (
        <section style={section}>
          {tabError ? <ErrorBanner message={tabError} /> : null}
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Description</th>
                  <th style={th}>Panels</th>
                  <th style={th}>Author</th>
                  <th style={th}>Updated</th>
                </tr>
              </thead>
              <tbody>
                {dashboards.length === 0 ? (
                  <tr>
                    <td colSpan={5} style={emptyTd}>
                      {loading ? "Loading…" : "No custom dashboards in scope"}
                    </td>
                  </tr>
                ) : (
                  dashboards.map((d) => (
                    <tr key={d.id} style={tr}>
                      <td style={td}><strong>{d.name}</strong></td>
                      <td style={td}>{d.description || "—"}</td>
                      <td style={td}>{d.panels?.length ?? 0}</td>
                      <td style={td}>{d.created_by || "—"}</td>
                      <td style={tdMono}>{formatTime(d.updated_at)}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}
    </div>
  );
}

function formatTime(iso: string): string {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function DrStepBar({ step }: { step: DrStep }) {
  const steps: { id: DrStep; label: string }[] = [
    { id: "select", label: "Select VM" },
    { id: "exporting", label: "Export" },
    { id: "review", label: "Review" },
    { id: "acting", label: "Apply / failover" },
  ];
  const activeIdx = steps.findIndex((s) => s.id === step);
  return (
    <div style={stepBar}>
      {steps.map((s, i) => (
        <div key={s.id} style={{ display: "flex", alignItems: "center", gap: 8, flex: 1 }}>
          <div
            style={{
              ...stepDot,
              background: i <= activeIdx ? "#f0583a" : "#e5e7eb",
              color: i <= activeIdx ? "#fff" : "#6b7280",
            }}
          >
            {i + 1}
          </div>
          <span style={{ fontSize: 12, fontWeight: i === activeIdx ? 700 : 500, color: i <= activeIdx ? "#374151" : "#9ca3af" }}>
            {s.label}
          </span>
          {i < steps.length - 1 ? <div style={stepLine} /> : null}
        </div>
      ))}
    </div>
  );
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = { marginBottom: 16 };
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const tabRow: CSSProperties = { display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 20 };
const tabBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const tabBtnActive: CSSProperties = { ...tabBtn, borderColor: "#f0583a", background: "rgba(240,88,58,0.08)", color: "#c2410c" };
const section: CSSProperties = { marginBottom: 24 };
const note: CSSProperties = { fontSize: 13, color: "#6b7280", lineHeight: 1.5, marginBottom: 16 };
const hint: CSSProperties = { fontSize: 12, color: "#6b7280", margin: "0 0 12px" };
const muted: CSSProperties = { fontSize: 13, color: "#9ca3af", margin: 0 };
const progressNote: CSSProperties = {
  fontSize: 13, color: "#1d4ed8", background: "#eff6ff", padding: "8px 12px", borderRadius: 6, marginBottom: 12,
};
const stepBar: CSSProperties = { display: "flex", gap: 4, marginBottom: 16, flexWrap: "wrap" };
const stepDot: CSSProperties = {
  width: 24, height: 24, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 11, fontWeight: 700,
};
const stepLine: CSSProperties = { flex: 1, height: 2, background: "#e5e7eb", minWidth: 12 };
const summaryCard: CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: 14, marginBottom: 12,
};
const summaryGrid: CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 12,
};
const summaryLabel: CSSProperties = { fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "#6b7280" };
const summaryValue: CSSProperties = { marginTop: 4, fontSize: 14, fontWeight: 700, color: "#111827" };
const formRow: CSSProperties = { display: "flex", flexWrap: "wrap", gap: 12, marginBottom: 12 };
const label: CSSProperties = { display: "flex", flexDirection: "column", gap: 4, fontSize: 12, fontWeight: 600, color: "#374151" };
const input: CSSProperties = {
  padding: "8px 10px", borderRadius: 6, border: "1px solid #d1d5db", fontSize: 13, minWidth: 160,
};
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const primaryBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#f0583a", background: "#f0583a", color: "#fff",
};
const warnBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#b45309", background: "#fff7ed", color: "#b45309",
};
const pre: CSSProperties = {
  background: "#1e1e1e", color: "#e5e7eb", padding: 14, borderRadius: 8, fontSize: 12, overflow: "auto", maxHeight: 320, margin: 0,
};
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const emptyTd: CSSProperties = { ...td, textAlign: "center", color: "#9ca3af" };
const statGrid: CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 12,
};
const statCard: CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: "12px 14px",
};
const statLabel: CSSProperties = { fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase" };
const statValue: CSSProperties = { marginTop: 6, fontSize: 16, fontWeight: 700, color: "#222324" };

export default OperationsPanel;
