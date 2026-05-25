import { useCallback, useEffect, useState } from "react";
import {
  disableVmInternet,
  enableVmInternet,
  fetchNetworkAttachmentDefinitions,
  fetchVmInternet,
  fetchVmExpose,
  putVmExpose,
  type VmExposeStatus,
  type VmInternetStatus,
} from "../../lib/api";
import { panelStyles as s } from "./vmPanelStyles";

type Props = {
  namespace: string;
  vmName: string;
};

export function VmSshExposePanel({ namespace, vmName }: Props) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<VmExposeStatus | null>(null);
  const [serviceType, setServiceType] = useState("NodePort");
  const [port, setPort] = useState(22);
  const [targetPort, setTargetPort] = useState(22);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const ex = await fetchVmExpose(namespace, vmName);
      setStatus(ex);
      if (ex.service_type) setServiceType(ex.service_type);
      const p0 = ex.ports?.[0];
      if (p0?.port) setPort(p0.port);
      if (p0?.target_port) setTargetPort(p0.target_port);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load SSH expose");
    } finally {
      setLoading(false);
    }
  }, [namespace, vmName]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const apply = async () => {
    setLoading(true);
    setError(null);
    try {
      const ex = await putVmExpose(namespace, vmName, {
        enabled: true,
        service_type: serviceType,
        ports: [{ name: "ssh", port, target_port: targetPort, protocol: "TCP" }],
      });
      setStatus(ex);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to apply expose");
    } finally {
      setLoading(false);
    }
  };

  const remove = async () => {
    setLoading(true);
    setError(null);
    try {
      const ex = await putVmExpose(namespace, vmName, { enabled: false });
      setStatus(ex);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to remove expose");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={s.wrap}>
      <button type="button" style={s.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} SSH / service expose
      </button>
      {open && (
        <div style={s.panel}>
          {loading && <p style={s.muted}>Loading…</p>}
          {error && <p style={s.err}>{error}</p>}
          {status?.enabled ? (
            <>
              <p>
                <strong>Service:</strong> {status.service_name} ({status.service_type})
              </p>
              {status.ports?.map((p) => (
                <p key={`${p.port}-${p.target_port}`} style={s.mono}>
                  port {p.port} → {p.target_port}
                  {p.node_port != null ? ` · NodePort ${p.node_port}` : ""}
                </p>
              ))}
            </>
          ) : (
            <p style={s.muted}>No expose Service — forward port 22 (or another) to the guest via virt-launcher.</p>
          )}
          <div style={s.row}>
            <label style={s.label}>
              Type
              <select value={serviceType} onChange={(e) => setServiceType(e.target.value)} style={s.inputWide}>
                <option value="ClusterIP">ClusterIP</option>
                <option value="NodePort">NodePort</option>
                <option value="LoadBalancer">LoadBalancer</option>
              </select>
            </label>
            <label style={s.label}>
              Service port
              <input type="number" min={1} max={65535} value={port} onChange={(e) => setPort(parseInt(e.target.value, 10) || 22)} style={s.input} />
            </label>
            <label style={s.label}>
              Target port
              <input
                type="number"
                min={1}
                max={65535}
                value={targetPort}
                onChange={(e) => setTargetPort(parseInt(e.target.value, 10) || 22)}
                style={s.input}
              />
            </label>
            <button type="button" style={s.btnPrimary} disabled={loading} onClick={() => void apply()}>
              Apply
            </button>
            <button type="button" style={s.btnDanger} disabled={loading} onClick={() => void remove()}>
              Remove
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export function VmInternetPanel({ namespace, vmName }: Props) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<VmInternetStatus | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setStatus(await fetchVmInternet(namespace, vmName));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load internet policy");
    } finally {
      setLoading(false);
    }
  }, [namespace, vmName]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const enable = async () => {
    setLoading(true);
    setError(null);
    try {
      setStatus(await enableVmInternet(namespace, vmName));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Enable failed");
    } finally {
      setLoading(false);
    }
  };

  const disable = async () => {
    setLoading(true);
    setError(null);
    try {
      setStatus(await disableVmInternet(namespace, vmName));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Remove failed");
    } finally {
      setLoading(false);
    }
  };

  const backendLabel =
    status?.enabled && status.backend
      ? status.backend === "cilium"
        ? "CiliumNetworkPolicy"
        : status.backend === "kubernetes"
          ? "Kubernetes NetworkPolicy"
          : status.backend
      : null;

  return (
    <div style={s.wrap}>
      <button type="button" style={s.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} Internet egress
      </button>
      {open && (
        <div style={s.panel}>
          {loading && <p style={s.muted}>Loading…</p>}
          {error && <p style={s.err}>{error}</p>}
          {status && (
            <p>
              <strong>Status:</strong>{" "}
              {status.enabled ? `On (${backendLabel})` : "Off — enable on default-deny clusters for guest NAT"}
            </p>
          )}
          {status?.policy_name ? <p style={s.mono}>Policy: {status.policy_name}</p> : null}
          <div style={s.row}>
            <button type="button" style={s.btnSuccess} disabled={loading} onClick={() => void enable()}>
              Enable internet
            </button>
            <button type="button" style={s.btnDanger} disabled={loading} onClick={() => void disable()}>
              Remove policy
            </button>
          </div>
          <p style={s.hint}>Targets virt-launcher pods labeled kubevirt.io/vm for this VM.</p>
        </div>
      )}
    </div>
  );
}

export function VmMultusPanel({ namespace }: { namespace: string }) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nads, setNads] = useState<Array<{ name: string; namespace: string; config: string }>>([]);
  const [selected, setSelected] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await fetchNetworkAttachmentDefinitions(namespace);
      setNads(rows);
      if (rows.length && !selected) setSelected(`${rows[0].namespace}/${rows[0].name}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load NADs");
    } finally {
      setLoading(false);
    }
  }, [namespace, selected]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  return (
    <div style={s.wrap}>
      <button type="button" style={s.toggle} onClick={() => setOpen((o) => !o)}>
        {open ? "▼" : "▶"} Multus network attachments
      </button>
      {open && (
        <div style={s.panel}>
          {loading && <p style={s.muted}>Loading NAD inventory…</p>}
          {error && <p style={s.err}>{error}</p>}
          {nads.length === 0 && !loading ? (
            <p style={s.muted}>No NetworkAttachmentDefinitions in scope. Create NADs in your CNI namespace first.</p>
          ) : (
            <>
              <label style={s.label}>
                Attach NAD (reference for VM spec)
                <select value={selected} onChange={(e) => setSelected(e.target.value)} style={s.inputWide}>
                  {nads.map((nad) => (
                    <option key={`${nad.namespace}/${nad.name}`} value={`${nad.namespace}/${nad.name}`}>
                      {nad.namespace}/{nad.name}
                    </option>
                  ))}
                </select>
              </label>
              {selected ? (
                <p style={s.mono}>
                  multus networkName: {selected.split("/")[1]} · namespace: {selected.split("/")[0]}
                </p>
              ) : null}
              <p style={s.hint}>
                Add a secondary interface in the VM YAML with networkName matching the NAD. Full attach editor ships with VM update API.
              </p>
            </>
          )}
        </div>
      )}
    </div>
  );
}

export default VmSshExposePanel;
