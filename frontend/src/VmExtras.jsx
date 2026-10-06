import { useEffect, useState } from 'react';
import { HardDrive, ArrowLeftRight, Stethoscope, Wrench } from 'lucide-react';
import { api, relTime } from './api.js';
import { Meter } from './status.jsx';

const label = (k) => String(k).replace(/[_-]+/g, ' ').replace(/^\w/, (c) => c.toUpperCase());

/** Flat key/value rows for an arbitrary guest report; nested lists show their length. */
function Report({ data }) {
  if (data == null) return null;
  if (typeof data !== 'object') return <div className="ops-row"><span>{String(data)}</span></div>;
  const entries = Object.entries(data).filter(([, v]) => v != null && v !== '');
  if (!entries.length) return <small className="ops-hint">Nothing reported.</small>;
  return entries.slice(0, 16).map(([k, v]) => {
    let shown;
    if (Array.isArray(v)) {
      shown = v.length && typeof v[0] !== 'object' ? v.slice(0, 6).join(', ') : `${v.length} item${v.length === 1 ? '' : 's'}`;
    } else if (typeof v === 'object') {
      shown = Object.entries(v)
        .filter(([, x]) => x == null || typeof x !== 'object')
        .slice(0, 4)
        .map(([a, b]) => `${label(a)}: ${b ?? '—'}`)
        .join(' · ') || '…';
    } else if (typeof v === 'boolean') {
      shown = v ? 'Yes' : 'No';
    } else {
      shown = String(v);
    }
    return (
      <div className="ops-row" key={k}>
        <span>{label(k)}</span>
        <span className="mono dim ops-val">{shown}</span>
      </div>
    );
  });
}

/** Issues/checks lists from doctor or fix-plan, whichever key the guest runtime used. */
function issuesOf(d) {
  if (!d || typeof d !== 'object') return [];
  const list = d.findings || d.issues || d.checks || d.steps || d.actions || [];
  return Array.isArray(list) ? list : [];
}

function IssueList({ items }) {
  return items.slice(0, 12).map((it, i) => {
    const text = typeof it === 'string' ? it : it.title || it.name || it.message || it.description || it.summary || JSON.stringify(it);
    const sev = typeof it === 'object' ? it.severity || it.status || (it.pass === false ? 'fail' : it.pass ? 'pass' : '') : '';
    return (
      <div className="ops-row" key={i}>
        <span>{text}</span>
        {sev && <small className="mono">{String(sev)}</small>}
      </div>
    );
  });
}

export function VmGuest({ row }) {
  const ns = row.ns || 'default';
  const name = row.name;
  const [status, setStatus] = useState(null);
  const [fs, setFs] = useState(null);
  const [reports, setReports] = useState({});
  const [plan, setPlan] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  useEffect(() => {
    let cancelled = false;
    setStatus(null);
    setFs(null);
    setReports({});
    setPlan(null);
    setErr('');
    (async () => {
      const [st, f] = await Promise.all([
        api.getGuestStatus(ns, name).catch(() => null),
        api.guestFilesystem(ns, name).catch(() => null),
      ]);
      if (cancelled) return;
      setStatus(st || {});
      setFs(f);
      if (!(st?.connected || st?.guest_agent_connected)) return;
      const keys = ['doctor', 'metrics', 'migrateScore', 'evidence'];
      const fns = [api.guestDoctor, api.guestMetrics, api.guestMigrateScore, api.guestEvidence];
      await Promise.all(
        keys.map(async (k, i) => {
          try {
            const v = await fns[i](ns, name);
            if (!cancelled) setReports((r) => ({ ...r, [k]: { v } }));
          } catch (e) {
            if (!cancelled) setReports((r) => ({ ...r, [k]: { err: e.message || String(e) } }));
          }
        }),
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [ns, name]);

  if (!status) return <div className="ops"><small className="ops-hint">Checking the guest runtime…</small></div>;
  const connected = !!(status.connected || status.guest_agent_connected);
  const block = (k) => reports[k];

  return (
    <div className="ops">
      {err && <div className="ops-err">{err}</div>}
      <div className="ops-block">
        <h4>Runtime</h4>
        <div className="ops-row">
          <span>Agent</span>
          <span className={connected ? '' : 'dim'}>{connected ? 'Connected' : 'Not connected'}</span>
        </div>
        <div className="ops-row">
          <span>Runtime</span>
          <span className="mono dim">{status.guest_runtime && status.guest_runtime !== 'unknown' ? status.guest_runtime : '—'}</span>
        </div>
        {status.version && (
          <div className="ops-row">
            <span>Version</span>
            <span className="mono dim">{status.version}</span>
          </div>
        )}
        {!connected && (
          <small className="ops-hint">
            {row.status === 'Running'
              ? 'Doctor, evidence and guest metrics need GuestKit (Linux) or the QEMU guest agent (Windows) running inside the VM.'
              : 'Start the VM to read anything from inside the guest.'}
          </small>
        )}
      </div>

      <div className="ops-block">
        <h4>
          <HardDrive size={13} /> Filesystems
        </h4>
        {fs?.available && fs.filesystems?.length ? (
          fs.filesystems.map((f, i) => {
            const pct = f.used_percent ?? f.usage_percent ?? (f.total_bytes ? Math.round((f.used_bytes / f.total_bytes) * 100) : null);
            return (
              <div className="ops-row" key={f.mountpoint || f.mount_point || i}>
                <span>
                  {f.mountpoint || f.mount_point || f.name || `fs-${i}`}
                  <small>{[f.type || f.fstype, f.device].filter(Boolean).join(' · ')}</small>
                </span>
                <span>{pct != null ? <Meter v={Math.round(pct)} /> : '—'}</span>
              </div>
            );
          })
        ) : (
          <small className="ops-hint">{fs?.reason || 'No filesystem data.'}</small>
        )}
      </div>

      {connected && (
        <>
          <div className="ops-block">
            <h4>
              <Stethoscope size={13} /> Doctor
            </h4>
            {block('doctor')?.err ? (
              <div className="ops-err">{block('doctor').err}</div>
            ) : block('doctor') ? (
              issuesOf(block('doctor').v).length ? <IssueList items={issuesOf(block('doctor').v)} /> : <Report data={block('doctor').v} />
            ) : (
              <small className="ops-hint">Running checks…</small>
            )}
            <div className="ops-acts" style={{ marginTop: 8 }}>
              <button
                className="btn sm secondary"
                disabled={busy}
                onClick={async () => {
                  setBusy(true);
                  setErr('');
                  try {
                    setPlan(await api.guestFixPlan(ns, name));
                  } catch (e) {
                    setErr(e.message || String(e));
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                <Wrench size={12} /> {busy ? 'Planning…' : 'Build fix plan'}
              </button>
            </div>
            {plan && (issuesOf(plan).length ? <IssueList items={issuesOf(plan)} /> : <Report data={plan} />)}
          </div>
          {[
            ['metrics', 'Guest metrics'],
            ['migrateScore', 'Migration readiness'],
            ['evidence', 'Evidence'],
          ].map(([k, title]) => (
            <div className="ops-block" key={k}>
              <h4>{title}</h4>
              {block(k)?.err ? (
                <div className="ops-err">{block(k).err}</div>
              ) : block(k) ? (
                <Report data={block(k).v} />
              ) : (
                <small className="ops-hint">Loading…</small>
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

/** Disks and migrations blocks appended to the VM Ops tab. */
export function VmDisks({ row, onDone }) {
  const ns = row.ns || 'default';
  const name = row.name;
  const [volumes, setVolumes] = useState(null);
  const [migrations, setMigrations] = useState(null);
  const [defaults, setDefaults] = useState(null);
  const [disk, setDisk] = useState({ size: '', sc: '', bus: '' });
  const [attach, setAttach] = useState({ volume: '', pvc: '' });
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const [note, setNote] = useState('');

  const refresh = async () => {
    const [v, m] = await Promise.all([
      api.vmVolumes(ns, name).catch(() => []),
      api.vmMigrations(ns, name).catch(() => []),
    ]);
    setVolumes(v);
    setMigrations(m);
  };

  useEffect(() => {
    let cancelled = false;
    setVolumes(null);
    setMigrations(null);
    setNote('');
    setErr('');
    (async () => {
      await refresh();
      const d = await api.dataDiskDefaults(ns, name).catch(() => null);
      if (cancelled || !d) return;
      setDefaults(d);
      setDisk({ size: String(d.default_size_gi || 10), sc: d.storage_class || d.storage_classes?.[0] || '', bus: d.suggested_bus || 'virtio' });
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ns, name]);

  const run = async (lbl, fn) => {
    setBusy(lbl);
    setErr('');
    try {
      await fn();
      await refresh();
      onDone?.(lbl);
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy('');
    }
  };

  return (
    <>
      {err && <div className="ops-err">{err}</div>}
      <div className="ops-block">
        <h4>
          <HardDrive size={13} /> Disks
        </h4>
        {volumes == null ? (
          <small className="ops-hint">Loading…</small>
        ) : volumes.length ? (
          volumes.map((v) => (
            <div className="ops-row" key={v.name}>
              <span>
                {v.name}
                <small>
                  {[v.target, v.hotplugVolume ? 'hotplugged' : null, v.phase].filter(Boolean).join(' · ') || '—'}
                </small>
              </span>
              {v.hotplugVolume && (
                <button
                  className="btn sm secondary"
                  disabled={!!busy}
                  onClick={() => {
                    if (!window.confirm(`Detach ${v.name} from ${name}? The volume itself is kept.`)) return;
                    run('Detach volume', () => api.hotremoveVolume(ns, name, v.name));
                  }}
                >
                  Detach
                </button>
              )}
            </div>
          ))
        ) : (
          <small className="ops-hint">{row.status === 'Running' ? 'No volume status reported.' : 'Disk status appears once the VM runs.'}</small>
        )}
        {defaults && (
          <>
            <small className="ops-hint" style={{ marginTop: 12 }}>
              Add a new blank data disk ({defaults.suggested_disk_name}, formatted {defaults.suggested_filesystem || 'by you'}
              {defaults.suggested_mount_path ? ` at ${defaults.suggested_mount_path}` : ''}).
            </small>
            <div className="ops-fields">
              <label>
                Size (GiB)
                <input value={disk.size} onChange={(e) => setDisk({ ...disk, size: e.target.value })} inputMode="numeric" />
              </label>
              <label>
                Storage class
                <select value={disk.sc} onChange={(e) => setDisk({ ...disk, sc: e.target.value })}>
                  {(defaults.storage_classes || []).map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <button
              className="btn sm primary"
              disabled={!!busy}
              onClick={() => {
                const size = parseInt(disk.size, 10);
                if (!(size > 0)) {
                  setErr('Disk size must be a whole number of GiB');
                  return;
                }
                if (!window.confirm(`Create a ${size} GiB ${disk.sc} disk and attach it to ${name}?`)) return;
                run('Add data disk', async () => {
                  const r = await api.addDataDisk(ns, name, {
                    size_gi: size,
                    storage_class: disk.sc || undefined,
                    bus: disk.bus || undefined,
                    disk_name: defaults.suggested_disk_name,
                    pvc_name: defaults.suggested_pvc_name,
                    mount_path: defaults.suggested_mount_path || undefined,
                    filesystem: defaults.suggested_filesystem || undefined,
                  });
                  setNote(r?.guest_init?.summary || r?.message || 'Disk added.');
                });
              }}
            >
              {busy === 'Add data disk' ? 'Adding…' : 'Add data disk'}
            </button>
            {note && <small className="ops-hint" style={{ marginTop: 8 }}>{note}</small>}
          </>
        )}
        <small className="ops-hint" style={{ marginTop: 12 }}>Hot-plug an existing claim into the running VM.</small>
        <div className="ops-fields">
          <label>
            Volume name
            <input value={attach.volume} onChange={(e) => setAttach({ ...attach, volume: e.target.value })} placeholder="data-2" />
          </label>
          <label>
            Claim (PVC)
            <input value={attach.pvc} onChange={(e) => setAttach({ ...attach, pvc: e.target.value })} placeholder="my-claim" />
          </label>
        </div>
        <button
          className="btn sm secondary"
          disabled={!!busy || row.status !== 'Running' || !attach.volume.trim() || !attach.pvc.trim()}
          title={row.status !== 'Running' ? 'The VM must be running to hot-plug' : undefined}
          onClick={() =>
            run('Hot-plug volume', async () => {
              await api.hotplugVolume(ns, name, attach.volume.trim(), attach.pvc.trim());
              setAttach({ volume: '', pvc: '' });
            })
          }
        >
          {busy === 'Hot-plug volume' ? 'Attaching…' : 'Hot-plug'}
        </button>
      </div>

      <div className="ops-block">
        <h4>
          <ArrowLeftRight size={13} /> Migrations
        </h4>
        {migrations == null ? (
          <small className="ops-hint">Loading…</small>
        ) : migrations.length ? (
          migrations.slice(0, 8).map((m, i) => {
            const id = m.metadata?.name || m.name || `mig-${i}`;
            const phase = m.status?.phase || 'Pending';
            const st = m.status?.migrationState || {};
            const done = /succeeded|failed/i.test(phase);
            return (
              <div className="ops-row" key={id}>
                <span>
                  {id}
                  <small>
                    {[phase, st.sourceNode && st.targetNode ? `${st.sourceNode} → ${st.targetNode}` : null, relTime(m.metadata?.creationTimestamp)]
                      .filter((x) => x && x !== '—')
                      .join(' · ')}
                  </small>
                </span>
                {!done && (
                  <button className="btn sm secondary" disabled={!!busy} onClick={() => run('Cancel migration', () => api.cancelVmMigration(ns, name, id))}>
                    Cancel
                  </button>
                )}
              </div>
            );
          })
        ) : (
          <small className="ops-hint">No migrations for this VM.</small>
        )}
      </div>
    </>
  );
}
