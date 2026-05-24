import { useEffect, useState, type CSSProperties } from "react";
import { createVirtualMachine, fetchTemplates, type VmTemplate } from "../../lib/api";

type Props = {
  defaultNamespace: string;
  onCreated?: () => void;
};

export function VmCreateModal({ defaultNamespace, onCreated }: Props) {
  const [open, setOpen] = useState(false);
  const [templates, setTemplates] = useState<VmTemplate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [name, setName] = useState("");
  const [namespace, setNamespace] = useState(defaultNamespace === "all" ? "default" : defaultNamespace);
  const [template, setTemplate] = useState("ubuntu");
  const [cpus, setCpus] = useState(2);
  const [memory, setMemory] = useState("4Gi");
  const [diskSize, setDiskSize] = useState("20Gi");
  const [start, setStart] = useState(true);
  const [allowInternet, setAllowInternet] = useState(true);

  useEffect(() => {
    setNamespace(defaultNamespace === "all" ? "default" : defaultNamespace);
  }, [defaultNamespace]);

  useEffect(() => {
    if (!open) return;
    void fetchTemplates()
      .then((rows) => {
        setTemplates(rows);
        if (rows.length && !rows.some((t) => t.name === template)) {
          setTemplate(rows[0].name);
        }
      })
      .catch(() => setTemplates([]));
  }, [open, template]);

  const onTemplateChange = (value: string) => {
    setTemplate(value);
    const t = templates.find((row) => row.name === value);
    if (t) {
      setCpus(t.default_cpus);
      setMemory(t.default_memory);
      setDiskSize(t.default_disk_size);
    }
  };

  const submit = async () => {
    const vmName = name.trim();
    if (!vmName) {
      setError("VM name is required");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await createVirtualMachine({
        name: vmName,
        namespace: namespace.trim() || "default",
        template,
        cpus,
        memory,
        disk_size: diskSize,
        start,
        allow_internet: allowInternet,
      });
      setOpen(false);
      setName("");
      onCreated?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Create failed");
    } finally {
      setLoading(false);
    }
  };

  if (!open) {
    return (
      <button type="button" onClick={() => setOpen(true)} style={createBtn}>
        + Create VM
      </button>
    );
  }

  return (
    <div
      role="presentation"
      style={overlay}
      onClick={() => setOpen(false)}
    >
      <div role="dialog" aria-modal="true" aria-labelledby="create-vm-title" style={dialog} onClick={(e) => e.stopPropagation()}>
        <h3 id="create-vm-title" style={{ margin: "0 0 16px", fontSize: 18 }}>
          Create virtual machine
        </h3>
        {error ? (
          <p style={{ color: "#dc2626", fontSize: 13, margin: "0 0 12px" }} role="alert">
            {error}
          </p>
        ) : null}
        <div style={{ display: "grid", gap: 10 }}>
          <label style={label}>
            Name
            <input value={name} onChange={(e) => setName(e.target.value)} style={field} required />
          </label>
          <label style={label}>
            Namespace
            <input value={namespace} onChange={(e) => setNamespace(e.target.value)} style={field} />
          </label>
          <label style={label}>
            Template
            <select value={template} onChange={(e) => onTemplateChange(e.target.value)} style={field}>
              {templates.length === 0 ? <option value={template}>{template}</option> : null}
              {templates.map((t) => (
                <option key={t.name} value={t.name}>
                  {t.name} — {t.description}
                </option>
              ))}
            </select>
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
            <label style={label}>
              CPUs
              <input type="number" min={1} value={cpus} onChange={(e) => setCpus(parseInt(e.target.value, 10) || 1)} style={field} />
            </label>
            <label style={label}>
              Memory
              <input value={memory} onChange={(e) => setMemory(e.target.value)} style={field} />
            </label>
            <label style={label}>
              Disk
              <input value={diskSize} onChange={(e) => setDiskSize(e.target.value)} style={field} />
            </label>
          </div>
          <label style={checkLabel}>
            <input type="checkbox" checked={start} onChange={(e) => setStart(e.target.checked)} />
            Start after create
          </label>
          <label style={checkLabel}>
            <input type="checkbox" checked={allowInternet} onChange={(e) => setAllowInternet(e.target.checked)} />
            Allow internet egress policy
          </label>
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 16, justifyContent: "flex-end" }}>
          <button type="button" onClick={() => setOpen(false)} style={btnSecondary} disabled={loading}>
            Cancel
          </button>
          <button type="button" onClick={() => void submit()} style={btnPrimary} disabled={loading}>
            {loading ? "Creating…" : "Create VM"}
          </button>
        </div>
      </div>
    </div>
  );
}

const overlay: CSSProperties = {
  position: "fixed",
  inset: 0,
  zIndex: 1500,
  background: "rgba(0,0,0,0.45)",
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "center",
  padding: "8vh 16px 16px",
};

const dialog: CSSProperties = {
  width: "min(480px, 96vw)",
  background: "#fff",
  borderRadius: 10,
  padding: 20,
  boxShadow: "0 16px 40px rgba(0,0,0,0.2)",
};

const field: CSSProperties = {
  display: "block",
  width: "100%",
  marginTop: 4,
  padding: 8,
  border: "1px solid #ddd",
  borderRadius: 4,
  boxSizing: "border-box",
};

const label: CSSProperties = { fontSize: 13 };

const checkLabel: CSSProperties = {
  fontSize: 13,
  display: "flex",
  alignItems: "center",
  gap: 8,
};

const createBtn: CSSProperties = {
  padding: "10px 18px",
  backgroundColor: "#222324",
  color: "#fff",
  border: "none",
  borderRadius: 6,
  fontWeight: 600,
  cursor: "pointer",
  fontSize: 14,
};

const btnPrimary: CSSProperties = {
  padding: "8px 16px",
  background: "#f0583a",
  color: "#fff",
  border: "none",
  borderRadius: 6,
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: CSSProperties = {
  padding: "8px 16px",
  background: "#fff",
  color: "#333",
  border: "1px solid #ddd",
  borderRadius: 6,
  fontWeight: 600,
  cursor: "pointer",
};

export default VmCreateModal;
