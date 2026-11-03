import { useMemo, useState } from 'react';
import { Plus, X, Search, Check, ChevronRight, ChevronLeft } from 'lucide-react';

const DNS1123 = /^[a-z0-9]([-a-z0-9]*[a-z0-9])?$/;
const FAMILIES = [
  ['all', 'All'], ['ubuntu', 'Ubuntu'], ['windows', 'Windows'], ['rhel', 'RHEL family'],
  ['debian', 'Debian'], ['fedora', 'Fedora'], ['suse', 'SUSE'], ['other', 'Other'],
];
const PRESETS = [
  { id: 'tpl', label: 'Template default', cpus: null, mem: null },
  { id: 'xs', label: 'Tiny', cpus: 1, mem: '512Mi', gib: 0.5 },
  { id: 's', label: 'Small', cpus: 1, mem: '2Gi', gib: 2 },
  { id: 'm', label: 'Medium', cpus: 2, mem: '4Gi', gib: 4 },
  { id: 'l', label: 'Large', cpus: 4, mem: '8Gi', gib: 8 },
  { id: 'xl', label: 'X-Large', cpus: 8, mem: '16Gi', gib: 16 },
];

function family(name = '') {
  const n = name.toLowerCase();
  if (n.includes('windows')) return 'windows';
  if (n.includes('ubuntu')) return 'ubuntu';
  if (/(rhel|centos|rocky|alma|oracle)/.test(n)) return 'rhel';
  if (n.includes('debian')) return 'debian';
  if (n.includes('fedora')) return 'fedora';
  if (/(suse|sles|opensuse)/.test(n)) return 'suse';
  return 'other';
}
// Memory strings from the API look like "32178144Ki", "4Gi" or a bare number of GiB.
const memGiB = (v) => {
  const m = String(v ?? '').match(/([\d.]+)\s*(Ki|Mi|Gi|Ti)?/i);
  if (!m) return 0;
  const n = parseFloat(m[1]);
  const u = (m[2] || 'Gi').toLowerCase();
  return u === 'ki' ? n / 1024 / 1024 : u === 'mi' ? n / 1024 : u === 'ti' ? n * 1024 : n;
};
const num = (v) => {
  const x = parseFloat(String(v ?? '').replace(/[^\d.]/g, ''));
  return Number.isFinite(x) ? x : 0;
};

/** Three-step guided VM creation: pick a template, size it, review against capacity, create. */
export function VmWizard({ templates, hosts, vms, existingNames, onClose, onCreate }) {
  const [step, setStep] = useState(0);
  const [q, setQ] = useState('');
  const [fam, setFam] = useState('all');
  const [template, setTemplate] = useState(templates?.[0]?.name || '');
  const [preset, setPreset] = useState('tpl');
  const [cpus, setCpus] = useState(2);
  const [mem, setMem] = useState(4);
  const [custom, setCustom] = useState(false);
  const [disk, setDisk] = useState('10Gi');
  const [name, setName] = useState('');
  const [start, setStart] = useState(true);
  const [internet, setInternet] = useState(true);
  const [busy, setBusy] = useState(false);

  const tpl = (templates || []).find((t) => t.name === template);
  const shown = useMemo(
    () => (templates || []).filter((t) => (fam === 'all' || family(t.name) === fam) && (!q || t.name.toLowerCase().includes(q.toLowerCase()))),
    [templates, fam, q],
  );

  const p = PRESETS.find((x) => x.id === preset);
  const effCpu = custom ? Number(cpus) : p.cpus ?? num(tpl?.cpu) ?? 1;
  const effMemGiB = custom ? Number(mem) : p.gib ?? num(tpl?.ram);

  const nameErr = !name
    ? ''
    : !DNS1123.test(name) || name.length > 63
      ? 'Lowercase letters, digits and “-”, starting and ending with a letter or digit (max 63).'
      : (existingNames || []).includes(name)
        ? 'A VM with this name already exists in this namespace.'
        : '';

  // Rough capacity picture: total cores/memory across hosts vs what running VMs already use.
  const totalCores = (hosts || []).reduce((a, h) => a + num(h.cores), 0);
  const totalMem = (hosts || []).reduce((a, h) => a + memGiB(h.memTotal), 0);
  const usedCpu = (vms || []).filter((v) => v.status === 'Running').reduce((a, v) => a + num(v.cpu), 0);
  const usedMem = (vms || []).filter((v) => v.status === 'Running').reduce((a, v) => a + num(v.ram), 0);
  const freeMem = totalMem ? totalMem - usedMem : null;
  const overMem = start && freeMem != null && effMemGiB > freeMem;
  const overCpu = start && totalCores && effCpu > totalCores - usedCpu;

  const submit = async () => {
    setBusy(true);
    try {
      const body = { name, template, start, allow_internet: internet };
      if (custom || preset !== 'tpl') {
        body.cpus = effCpu;
        body.memory = custom ? `${mem}Gi` : p.mem;
      }
      if (disk && disk !== '10Gi') body.disk_size = disk.replace(/\s+/g, '');
      await onCreate(body);
    } finally {
      setBusy(false);
    }
  };

  const steps = ['Template', 'Size', 'Review'];
  const canNext = step === 0 ? !!template : step === 1 ? (!custom && preset === 'tpl') || (effCpu > 0 && effMemGiB > 0) : !!name && !nameErr;

  return (
    <div className="scrim" onClick={onClose}>
      <div className="sheet wiz" onClick={(e) => e.stopPropagation()}>
        <div className="sheet-h">
          <Plus size={15} />
          New virtual machine
          <button className="tb x" onClick={onClose}><X size={15} /></button>
        </div>
        <ol className="wiz-steps">
          {steps.map((s, i) => (
            <li key={s} className={i === step ? 'on' : i < step ? 'done' : ''}>
              <span>{i < step ? <Check size={12} /> : i + 1}</span>{s}
            </li>
          ))}
        </ol>

        {step === 0 && (
          <div className="wiz-body">
            <label className="wiz-search"><Search size={14} />
              <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search templates" autoFocus />
            </label>
            <div className="wiz-chips">
              {FAMILIES.map(([id, l]) => (
                <button key={id} className={fam === id ? 'on' : ''} onClick={() => setFam(id)}>{l}</button>
              ))}
            </div>
            <div className="wiz-tpls">
              {shown.length === 0 && <div className="wiz-empty">No templates match.</div>}
              {shown.map((t) => (
                <button key={t.id || t.name} className={`wiz-tpl ${template === t.name ? 'on' : ''}`} onClick={() => setTemplate(t.name)}>
                  <b>{t.name}</b>
                  <small>{[num(t.cpu) > 0 && `${t.cpu} vCPU`, num(t.ram) > 0 && `${t.ram} GiB`].filter(Boolean).join(' · ') || family(t.name)}</small>
                </button>
              ))}
            </div>
          </div>
        )}

        {step === 1 && (
          <div className="wiz-body">
            <div className="wiz-presets">
              {PRESETS.map((x) => (
                <button key={x.id} className={!custom && preset === x.id ? 'on' : ''} onClick={() => { setPreset(x.id); setCustom(false); }}>
                  <b>{x.label}</b>
                  <small>{x.cpus ? `${x.cpus} vCPU · ${x.gib} GiB` : num(tpl?.cpu) > 0 ? `${tpl.cpu} vCPU · ${tpl.ram} GiB` : 'Whatever the template sets'}</small>
                </button>
              ))}
              <button className={custom ? 'on' : ''} onClick={() => setCustom(true)}><b>Custom</b><small>Pick exact values</small></button>
            </div>
            {custom && (
              <div className="wiz-row">
                <div className="field"><label>vCPU</label><input type="number" min="1" max="128" value={cpus} onChange={(e) => setCpus(e.target.value)} /></div>
                <div className="field"><label>Memory (GiB)</label><input type="number" min="0.5" step="0.5" value={mem} onChange={(e) => setMem(e.target.value)} /></div>
              </div>
            )}
            <div className="field"><label>Disk size</label><input value={disk} onChange={(e) => setDisk(e.target.value)} placeholder="10Gi" /></div>
          </div>
        )}

        {step === 2 && (
          <div className="wiz-body">
            <div className="field">
              <label>Name</label>
              <input value={name} onChange={(e) => setName(e.target.value.toLowerCase())} placeholder="e.g. web-01" autoFocus />
              {nameErr && <small className="wiz-err">{nameErr}</small>}
            </div>
            <dl className="wiz-sum">
              <div><dt>Template</dt><dd>{template}</dd></div>
              <div><dt>Compute</dt><dd>{effCpu || '?'} vCPU · {effMemGiB || '?'} GiB</dd></div>
              <div><dt>Disk</dt><dd>{disk}</dd></div>
              <div><dt>Namespace</dt><dd>default</dd></div>
            </dl>
            <div className="wiz-cap">
              {totalCores ? (
                <>
                  <div>CPU: {usedCpu} in use + {effCpu || 0} new of {totalCores} cores</div>
                  <div>Memory: {usedMem.toFixed(1)} in use + {effMemGiB || 0} new of {totalMem ? totalMem.toFixed(1) : '?'} GiB</div>
                </>
              ) : <div>Capacity unknown.</div>}
              {(overMem || overCpu) && <div className="wiz-err">This may not fit on the cluster right now{overMem ? ' (memory)' : ''}{overCpu ? ' (CPU)' : ''}. It can still be created, but may stay Pending.</div>}
            </div>
            <label className="wiz-check"><input type="checkbox" checked={start} onChange={(e) => setStart(e.target.checked)} /> Start after creating</label>
            <label className="wiz-check"><input type="checkbox" checked={internet} onChange={(e) => setInternet(e.target.checked)} /> Allow outbound internet</label>
          </div>
        )}

        <div className="sheet-f">
          {step > 0 ? <button className="btn" onClick={() => setStep(step - 1)}><ChevronLeft size={13} /> Back</button> : <button className="btn" onClick={onClose}>Cancel</button>}
          {step < 2 ? (
            <button className="btn primary" disabled={!canNext} onClick={() => setStep(step + 1)}>Next <ChevronRight size={13} /></button>
          ) : (
            <button className="btn primary" disabled={!canNext || busy} onClick={submit}>{busy ? 'Creating…' : 'Create VM'}</button>
          )}
        </div>
      </div>
    </div>
  );
}
