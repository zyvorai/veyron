import React, { useCallback, useEffect, useState } from "react";
import {
  addDataDisk,
  fetchDataDiskDefaults,
  type AddDataDiskResponse,
  type DataDiskDefaults,
} from "../../lib/api";

interface Props {
  namespace: string;
  vmName: string;
  onAttached?: () => void;
}

function title(isWindows: boolean, isLinux: boolean, mountPath: string): string {
  if (isWindows) return "+ Add data disk (D:/E:)";
  if (isLinux) return `+ Add data disk (${mountPath})`;
  return "+ Add data disk";
}

const AddDataDiskPanel: React.FC<Props> = ({ namespace, vmName, onAttached }) => {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [defaultsLoading, setDefaultsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AddDataDiskResponse | null>(null);

  const [diskName, setDiskName] = useState("data-1");
  const [sizeGi, setSizeGi] = useState(50);
  const [storageClass, setStorageClass] = useState("");
  const [bus, setBus] = useState("virtio");
  const [driveLetter, setDriveLetter] = useState("D");
  const [mountPath, setMountPath] = useState("/mnt/data");
  const [filesystem, setFilesystem] = useState("ext4");
  const [isWindows, setIsWindows] = useState(false);
  const [isLinux, setIsLinux] = useState(true);
  const [storageClasses, setStorageClasses] = useState<string[]>([]);

  const loadDefaults = useCallback(async () => {
    setDefaultsLoading(true);
    setError(null);
    try {
      const data: DataDiskDefaults = await fetchDataDiskDefaults(namespace, vmName);
      setDiskName(data.suggested_disk_name);
      setBus(data.suggested_bus);
      setIsWindows(data.is_windows);
      setIsLinux(data.is_linux);
      setSizeGi(data.default_size_gi || (data.is_windows ? 100 : 50));
      setStorageClasses(data.storage_classes);
      setStorageClass(data.storage_class || data.storage_classes[0] || "");
      if (data.suggested_mount_path) setMountPath(data.suggested_mount_path);
      if (data.suggested_filesystem) setFilesystem(data.suggested_filesystem);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load defaults");
    } finally {
      setDefaultsLoading(false);
    }
  }, [namespace, vmName]);

  useEffect(() => {
    if (open) void loadDefaults();
  }, [open, loadDefaults]);

  const handleAdd = async () => {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const data = await addDataDisk(namespace, vmName, {
        disk_name: diskName.trim() || undefined,
        size_gi: sizeGi,
        storage_class: storageClass.trim() || undefined,
        bus: bus.trim() || undefined,
        drive_letter: isWindows ? driveLetter.trim().toUpperCase() : undefined,
        mount_path: isLinux ? mountPath.trim() : undefined,
        filesystem: isLinux ? filesystem : undefined,
        wait_bound: true,
      });
      setResult(data);
      onAttached?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to add data disk");
    } finally {
      setLoading(false);
    }
  };

  const script = result?.guest_init.shell_script || result?.guest_init.powershell;

  return (
    <div style={{ marginTop: 12 }}>
      <button
        type="button"
        onClick={() => {
          setOpen(!open);
          setResult(null);
          setError(null);
        }}
        style={{
          padding: "8px 14px",
          backgroundColor: "#2e7d32",
          color: "#fff",
          border: "none",
          borderRadius: 6,
          fontWeight: 600,
          cursor: "pointer",
        }}
      >
        {title(isWindows, isLinux, mountPath)}
      </button>

      {open && (
        <div style={{ marginTop: 12, padding: 16, border: "1px solid #c8e6c9", borderRadius: 8, backgroundColor: "#f1f8e9" }}>
          <p style={{ margin: "0 0 12px", fontSize: 14, color: "#33691e" }}>
            Creates a PVC, waits until bound, and hot-plugs to <strong>{namespace}/{vmName}</strong>.
            {isWindows && " Finish in Windows Disk Management or PowerShell."}
            {isLinux && !isWindows && " Run the bash script over SSH to format and mount."}
          </p>

          {defaultsLoading ? (
            <p style={{ fontSize: 13, color: "#666" }}>Loading defaults…</p>
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
              <label style={{ fontSize: 13 }}>
                Volume name
                <input value={diskName} onChange={(e) => setDiskName(e.target.value)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8 }} />
              </label>
              <label style={{ fontSize: 13 }}>
                Size (GiB)
                <input type="number" min={1} value={sizeGi} onChange={(e) => setSizeGi(Number(e.target.value) || 1)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8 }} />
              </label>
              <label style={{ fontSize: 13 }}>
                Storage class
                <select value={storageClass} onChange={(e) => setStorageClass(e.target.value)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8 }}>
                  {storageClasses.map((sc) => (
                    <option key={sc} value={sc}>{sc}</option>
                  ))}
                </select>
              </label>
              <label style={{ fontSize: 13 }}>
                Bus
                <select value={bus} onChange={(e) => setBus(e.target.value)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8 }}>
                  <option value="virtio">virtio</option>
                  <option value="scsi">scsi</option>
                  <option value="sata">sata</option>
                </select>
              </label>
              {isWindows && (
                <label style={{ fontSize: 13, gridColumn: "1 / -1" }}>
                  Drive letter
                  <input value={driveLetter} maxLength={1} onChange={(e) => setDriveLetter(e.target.value.toUpperCase().slice(0, 1))} style={{ display: "block", width: 64, marginTop: 4, padding: 8 }} />
                </label>
              )}
              {isLinux && !isWindows && (
                <>
                  <label style={{ fontSize: 13 }}>
                    Mount path
                    <input value={mountPath} onChange={(e) => setMountPath(e.target.value)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8, fontFamily: "monospace" }} />
                  </label>
                  <label style={{ fontSize: 13 }}>
                    Filesystem
                    <select value={filesystem} onChange={(e) => setFilesystem(e.target.value)} style={{ display: "block", width: "100%", marginTop: 4, padding: 8 }}>
                      <option value="ext4">ext4</option>
                      <option value="xfs">xfs</option>
                    </select>
                  </label>
                </>
              )}
            </div>
          )}

          <button
            type="button"
            disabled={loading || defaultsLoading}
            onClick={() => void handleAdd()}
            style={{
              padding: "8px 16px",
              backgroundColor: "#1565c0",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              fontWeight: 600,
              cursor: loading ? "wait" : "pointer",
              opacity: loading || defaultsLoading ? 0.6 : 1,
            }}
          >
            {loading ? "Working…" : "Create PVC & attach"}
          </button>

          {error && <p style={{ color: "#c62828", fontSize: 13, marginTop: 10 }}>{error}</p>}

          {result && (
            <div style={{ marginTop: 12, padding: 12, backgroundColor: "#e8f5e9", borderRadius: 6, fontSize: 13 }}>
              <strong style={{ color: "#2e7d32" }}>{result.message}</strong>
              <p style={{ margin: "8px 0", color: "#555" }}>{result.guest_init.summary}</p>
              <ol style={{ margin: 0, paddingLeft: 20 }}>
                {result.guest_init.steps.map((s, i) => (
                  <li key={i}>{s}</li>
                ))}
              </ol>
              {script && (
                <pre style={{ marginTop: 8, padding: 8, backgroundColor: "#263238", color: "#eceff1", fontSize: 11, overflow: "auto", maxHeight: 160 }}>
                  {script}
                </pre>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};


export default AddDataDiskPanel;
