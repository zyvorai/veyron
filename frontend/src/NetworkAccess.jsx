import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react';
import {
  Check,
  Eye,
  Globe,
  Lock,
  Plus,
  Radar,
  RefreshCw,
  Search,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Timer,
  Unlock,
  Wand2,
  X,
  Zap,
} from 'lucide-react';
import { api } from './api.js';
import { PageHero } from './PageHero.jsx';
import {
  DURATIONS,
  MODE_ORDER,
  diagramNodes,
  easeCount,
  filterTiles,
  flowLabel,
  formatRemaining,
  heatLevel,
  lane,
  matchPreset,
  modeInfo,
  parseTarget,
  putBody,
  remainingFraction,
  ruleKey,
  ruleLabel,
  summarizeFlows,
  tallies,
  toggleRule,
} from './networkAccess.js';
import './network-access.css';

const errText = (e) => (e?.status === 501 ? 'Network access policy needs the Kairon backend.' : e?.message || String(e));

function useReducedMotion() {
  const [reduced, setReduced] = useState(() => (typeof window !== 'undefined' && window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)').matches : false));
  useEffect(() => {
    if (!window.matchMedia) return undefined;
    const q = window.matchMedia('(prefers-reduced-motion: reduce)');
    const on = () => setReduced(q.matches);
    q.addEventListener?.('change', on);
    return () => q.removeEventListener?.('change', on);
  }, []);
  return reduced;
}

/** Animated count-up; jumps straight to the value under reduced motion. */
function useCountUp(target, reduced, ms = 700) {
  const [shown, setShown] = useState(reduced ? target : 0);
  const from = useRef(0);
  useEffect(() => {
    if (reduced) {
      setShown(target);
      return undefined;
    }
    const start = performance.now();
    const base = from.current;
    let raf = requestAnimationFrame(function tick(now) {
      const t = (now - start) / ms;
      setShown(easeCount(base, target, t));
      if (t < 1) raf = requestAnimationFrame(tick);
      else from.current = target;
    });
    return () => cancelAnimationFrame(raf);
  }, [target, reduced, ms]);
  return shown;
}

function ModePill({ mode, small }) {
  const m = modeInfo(mode);
  return (
    <span className={`na-pill tone-${m.tone}${small ? ' sm' : ''}`}>
      <i aria-hidden="true" />
      {m.label}
    </span>
  );
}

/** Segmented control with a thumb that slides to the selected option. */
function Segmented({ value, options, onChange, disabled, label }) {
  const idx = Math.max(0, options.findIndex((o) => o.id === value));
  return (
    <div className="na-seg" role="radiogroup" aria-label={label} style={{ '--n': options.length, '--i': idx }}>
      <span className={`na-seg-thumb tone-${options[idx]?.tone || 'gray'}`} aria-hidden="true" />
      {options.map((o) => (
        <button key={o.id} type="button" role="radio" aria-checked={o.id === value} disabled={disabled} className={o.id === value ? 'on' : ''} onClick={() => onChange(o.id)}>
          {o.icon}
          {o.label}
        </button>
      ))}
    </div>
  );
}

/* ——— the live flow diagram ——————————————————————————————————————————— */

function FlowDiagram({ policy, flows, running, reduced }) {
  const uid = useId().replace(/:/g, '');
  const mode = policy?.mode || 'loopback';
  const m = modeInfo(mode);
  const nodes = useMemo(() => diagramNodes(policy, flows), [policy, flows]);
  const VM = { x: 78, y: 120 };
  const GATE = { x: 318, y: 120 };
  const DEST_X = 500;
  return (
    <figure className={`na-flow tone-${m.tone}`} aria-label={`Traffic from the VM through its ${m.label} policy`}>
      <svg viewBox="0 0 640 240" role="img" aria-hidden="false">
        <defs>
          <linearGradient id={`${uid}-g`} x1="0" x2="1">
            <stop offset="0" stopColor="var(--apple-blue)" stopOpacity="0.9" />
            <stop offset="1" stopColor="var(--na-tone)" stopOpacity="0.9" />
          </linearGradient>
          <radialGradient id={`${uid}-glow`}>
            <stop offset="0" stopColor="var(--na-tone)" stopOpacity="0.55" />
            <stop offset="1" stopColor="var(--na-tone)" stopOpacity="0" />
          </radialGradient>
        </defs>

        {/* VM → gate */}
        <path d={`M${VM.x + 40} ${VM.y} L${GATE.x - 34} ${GATE.y}`} className="na-wire" />
        {nodes.map((n, i) => {
          const y = lane(i, nodes.length);
          const d = `M${GATE.x + 34} ${GATE.y} C ${GATE.x + 130} ${GATE.y}, ${DEST_X - 130} ${y}, ${DEST_X - 46} ${y}`;
          return <path key={`w${i}`} d={d} className={`na-wire ${n.kind}`} />;
        })}

        {/* gate */}
        <g transform={`translate(${GATE.x} ${GATE.y})`}>
          <circle r="64" fill={`url(#${uid}-glow)`} className={reduced ? '' : 'na-breathe'} />
          <path d="M0 -38 L30 -26 V4 C30 22 14 34 0 40 C-14 34 -30 22 -30 4 V-26 Z" className="na-gate" />
          <path
            d={{ allowlist: 'M-11 2 L-3 10 L12 -8', loopback: 'M-9 -9 L9 9 M9 -9 L-9 9', audit: 'M-13 0 C-7 -10 7 -10 13 0 C7 10 -7 10 -13 0 M0 -3 A3 3 0 1 0 0.01 -3', open: 'M0 -12 V3 M0 10 V11' }[mode] || 'M-9 0 H9'}
            className="na-gate-glyph"
          />
          <text y="62" textAnchor="middle" className="na-gate-label">
            {m.label}
          </text>
        </g>

        {/* VM */}
        <g transform={`translate(${VM.x} ${VM.y})`}>
          <rect x="-40" y="-30" width="80" height="60" rx="14" className={`na-vm${running ? '' : ' off'}`} />
          <rect x="-26" y="-18" width="52" height="30" rx="5" className="na-vm-screen" />
          <circle cx="0" cy="22" r="2.5" className="na-vm-led" />
          <text y="52" textAnchor="middle" className="na-cap">
            {running ? 'guest' : 'stopped'}
          </text>
        </g>

        {/* destinations */}
        {nodes.map((n, i) => {
          const y = lane(i, nodes.length);
          return (
            <g key={`n${i}`} transform={`translate(${DEST_X} ${y})`} className={`na-dest ${n.kind}`} style={reduced ? undefined : { animationDelay: `${i * 70}ms` }}>
              <rect x="-46" y="-17" width="148" height="34" rx="17" />
              <circle cx="-28" cy="0" r="5" className="na-dest-dot" />
              <text x="-16" y="-2" className="na-dest-label">
                {n.label.length > 17 ? `${n.label.slice(0, 16)}…` : n.label}
              </text>
              <text x="-16" y="10" className="na-dest-sub">
                {n.sub.length > 20 ? `${n.sub.slice(0, 19)}…` : n.sub}
              </text>
            </g>
          );
        })}

        {/* packets */}
        {!reduced && running && (
          <g aria-hidden="true">
            {nodes.map((n, i) => {
              const y = lane(i, nodes.length);
              const toGate = `M${VM.x + 40} ${VM.y} L${GATE.x - 34} ${GATE.y}`;
              const through = `M${VM.x + 40} ${VM.y} L${GATE.x - 34} ${GATE.y} L${GATE.x + 34} ${GATE.y} C ${GATE.x + 130} ${GATE.y}, ${DEST_X - 130} ${y}, ${DEST_X - 46} ${y}`;
              const delay = `${(i * 0.55) % 2.4}s`;
              if (n.kind === 'blocked') {
                return (
                  <g key={`p${i}`}>
                    <circle r="4" className="na-packet blocked">
                      <animateMotion dur="1.8s" begin={delay} repeatCount="indefinite" path={toGate} keyPoints="0;1;1" keyTimes="0;0.7;1" calcMode="linear" />
                      <animate attributeName="opacity" values="1;1;0" keyTimes="0;0.7;1" dur="1.8s" begin={delay} repeatCount="indefinite" />
                    </circle>
                    <circle cx={GATE.x - 34} cy={GATE.y} r="4" className="na-burst">
                      <animate attributeName="r" values="4;22" keyTimes="0;1" dur="1.8s" begin={`${parseFloat(delay) + 1.26}s`} repeatCount="indefinite" />
                      <animate attributeName="opacity" values="0.7;0" dur="1.8s" begin={`${parseFloat(delay) + 1.26}s`} repeatCount="indefinite" />
                    </circle>
                  </g>
                );
              }
              return (
                <circle key={`p${i}`} r="4" className={`na-packet ${n.kind}`}>
                  <animateMotion dur="2.6s" begin={delay} repeatCount="indefinite" path={through} />
                </circle>
              );
            })}
          </g>
        )}
      </svg>
      <figcaption>
        {nodes.length === 0 ? (mode === 'loopback' ? 'Nothing can leave this VM.' : 'No destinations yet.') : `${nodes.filter((n) => n.kind === 'allowed').length} allowed · ${nodes.filter((n) => n.kind === 'blocked').length} refused · ${nodes.filter((n) => n.kind === 'learning').length} learning`}
      </figcaption>
    </figure>
  );
}

/* ——— rules ————————————————————————————————————————————————————————————— */

function RuleChips({ rules, onRemove, reduced }) {
  const [leaving, setLeaving] = useState({});
  if (!rules.length) return <p className="na-hint">No rules. Pick a preset above or add one below.</p>;
  const remove = (r) => {
    if (reduced) {
      onRemove(r);
      return;
    }
    const k = ruleKey(r);
    setLeaving((l) => ({ ...l, [k]: true }));
    setTimeout(() => {
      onRemove(r);
      setLeaving((l) => {
        const next = { ...l };
        delete next[k];
        return next;
      });
    }, 180);
  };
  return (
    <ul className="na-chips">
      {rules.map((r) => (
        <li key={ruleKey(r)} className={leaving[ruleKey(r)] ? 'out' : ''} title={r.note || ''}>
          <span className="na-chip-main">{ruleLabel(r)}</span>
          {r.note && <small>{r.note}</small>}
          <button type="button" aria-label={`Remove ${ruleLabel(r)}`} onClick={() => remove(r)}>
            <X size={12} />
          </button>
        </li>
      ))}
    </ul>
  );
}

const CIDR_RE = /^\d{1,3}(\.\d{1,3}){3}(\/\d{1,2})?$/;

function AddRule({ onAdd, disabled }) {
  const [cidr, setCidr] = useState('');
  const [proto, setProto] = useState('tcp');
  const [port, setPort] = useState('443');
  const portNum = port === '' ? null : Number(port);
  const valid = CIDR_RE.test(cidr.trim()) && (proto === 'any' ? port === '' : portNum >= 1 && portNum <= 65535);
  return (
    <form
      className="na-add"
      onSubmit={(e) => {
        e.preventDefault();
        if (!valid) return;
        onAdd({ cidr: cidr.trim(), proto, ...(proto !== 'any' && portNum ? { port: portNum } : {}) });
        setCidr('');
      }}
    >
      <input aria-label="Network (CIDR)" placeholder="10.0.0.0/8 or 1.2.3.4" value={cidr} onChange={(e) => setCidr(e.target.value)} disabled={disabled} spellCheck={false} />
      <select aria-label="Protocol" value={proto} onChange={(e) => { setProto(e.target.value); if (e.target.value === 'any') setPort(''); else if (!port) setPort('443'); }} disabled={disabled}>
        <option value="tcp">tcp</option>
        <option value="udp">udp</option>
        <option value="any">any</option>
      </select>
      <input aria-label="Port" className="port" inputMode="numeric" placeholder="port" value={port} onChange={(e) => setPort(e.target.value.replace(/\D/g, '').slice(0, 5))} disabled={disabled || proto === 'any'} />
      <button type="submit" className="na-btn" disabled={disabled || !valid}>
        <Plus size={14} /> Add
      </button>
    </form>
  );
}

/* ——— explain ————————————————————————————————————————————————————————— */

function ExplainBox({ ns, name, onAllow, disabled }) {
  const [text, setText] = useState('');
  const [res, setRes] = useState(null);
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const seq = useRef(0);
  useEffect(() => {
    const target = parseTarget(text);
    if (!target) {
      setRes(null);
      setErr('');
      return undefined;
    }
    const mine = ++seq.current;
    const t = setTimeout(async () => {
      setBusy(true);
      try {
        const r = await api.vmEgressExplain(ns, name, target);
        if (mine === seq.current) {
          setRes(r);
          setErr('');
        }
      } catch (e) {
        if (mine === seq.current) {
          setRes(null);
          setErr(errText(e));
        }
      } finally {
        if (mine === seq.current) setBusy(false);
      }
    }, 450);
    return () => clearTimeout(t);
  }, [text, ns, name]);
  const first = res?.verdicts?.find((v) => !v.allowed) || res?.verdicts?.[0];
  return (
    <div className="na-explain">
      <label className="na-search">
        <Search size={15} aria-hidden="true" />
        <input placeholder="Can it reach…  archive.ubuntu.com:443" value={text} onChange={(e) => setText(e.target.value)} spellCheck={false} aria-label="Check a destination" />
        {busy && <RefreshCw size={13} className="spin" aria-label="Checking" />}
      </label>
      {err && <p className="na-err sm">{err}</p>}
      {res && first && (
        <div className={`na-verdict ${res.allowed ? 'yes' : 'no'}`} role="status">
          <span className="na-verdict-icon">{res.allowed ? <Check size={16} /> : <X size={16} />}</span>
          <div>
            <b>{res.allowed ? 'Allowed' : 'Blocked'}</b> <span className="mono">{res.host}:{res.port}/{res.proto}</span>
            <p>{first.reason}</p>
            {res.verdicts.length > 1 && <small>{res.verdicts.length} addresses checked</small>}
          </div>
          {!res.allowed && first.fix && (
            <button type="button" className="na-btn primary sm" disabled={disabled} onClick={() => onAllow(first.fix)}>
              <Plus size={13} /> Allow
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/* ——— timed change ring ———————————————————————————————————————————————— */

function GrantRing({ grant, onEnd, busy }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  const frac = remainingFraction(grant.created_at, grant.expires_at, now);
  const left = Date.parse(grant.expires_at) - now;
  const R = 22;
  const C = 2 * Math.PI * R;
  return (
    <div className="na-grant" role="timer" aria-label={`Reverts in ${formatRemaining(left)}`}>
      <svg viewBox="0 0 56 56" width="56" height="56" aria-hidden="true">
        <circle cx="28" cy="28" r={R} className="track" />
        <circle cx="28" cy="28" r={R} className="arc" strokeDasharray={C} strokeDashoffset={C * (1 - frac)} transform="rotate(-90 28 28)" />
        <Timer x="18" y="18" size={20} />
      </svg>
      <div>
        <b>{formatRemaining(left)}</b>
        <small>then it goes back to what it was</small>
      </div>
      <button type="button" className="na-btn sm" disabled={busy} onClick={onEnd}>
        End now
      </button>
    </div>
  );
}

/* ——— learn → review → lock down wizard ——————————————————————————————— */

const STEPS = ['Observe', 'Review', 'Apply'];

function LearnWizard({ ns, name, platformRules, onDone, setErr, current }) {
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [flows, setFlows] = useState([]);
  const [sugg, setSugg] = useState(null);
  const [picked, setPicked] = useState({});
  const learning = current?.mode === 'audit';

  // While observing, poll what the guest reaches.
  useEffect(() => {
    if (step !== 0 || !learning) return undefined;
    let live = true;
    const pull = () =>
      api
        .vmEgressObserved(ns, name)
        .then((r) => live && setFlows(r?.flows || []))
        .catch(() => {});
    pull();
    const t = setInterval(pull, 4000);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [step, learning, ns, name]);

  const seen = summarizeFlows(flows);
  const start = async () => {
    setBusy(true);
    try {
      await api.setVmEgress(ns, name, putBody({ preset: 'learn' }));
      await onDone(false);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const review = async () => {
    setBusy(true);
    try {
      const s = await api.vmEgressSuggest(ns, name);
      setSugg(s);
      setPicked(Object.fromEntries((s.rules || []).map((r) => [ruleKey(r), true])));
      setStep(1);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const apply = async () => {
    setBusy(true);
    try {
      const chosen = (sugg?.rules || []).filter((r) => picked[ruleKey(r)]);
      const merged = [...platformRules];
      for (const r of chosen) if (!merged.some((x) => ruleKey(x) === ruleKey(r))) merged.push(r);
      await api.setVmEgress(ns, name, putBody({ mode: 'allowlist', rules: merged }));
      setStep(2);
      await onDone(false);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const chosenCount = Object.values(picked).filter(Boolean).length;

  return (
    <section className="na-wizard" aria-label="Learn and lock down">
      <ol className="na-steps" style={{ '--p': step / (STEPS.length - 1) }}>
        <span className="na-steps-bar" aria-hidden="true"><i /></span>
        {STEPS.map((s, i) => (
          <li key={s} className={i < step ? 'done' : i === step ? 'now' : ''}>
            <b>{i < step ? <Check size={13} /> : i + 1}</b>
            {s}
          </li>
        ))}
      </ol>

      {step === 0 && (
        <div className="na-step">
          <p>
            Let the VM run its workload with <b>learning</b> on: nothing is blocked, and every destination it reaches is recorded. Then lock it down to exactly that.
          </p>
          {!learning ? (
            <button type="button" className="na-btn primary" disabled={busy} onClick={start}>
              <Eye size={15} /> Start learning
            </button>
          ) : (
            <>
              <div className="na-live"><i aria-hidden="true" /> Watching · {seen.wouldBlock.length} destination{seen.wouldBlock.length === 1 ? '' : 's'} so far</div>
              <ul className="na-seen">
                {seen.wouldBlock.slice(0, 6).map((f) => (
                  <li key={flowLabel(f)}><span className="mono">{flowLabel(f)}</span><small>{f.packets} pkt</small></li>
                ))}
              </ul>
              <button type="button" className="na-btn primary" disabled={busy} onClick={review}>
                <Wand2 size={15} /> Review what it reached
              </button>
            </>
          )}
        </div>
      )}

      {step === 1 && (
        <div className="na-step">
          {(sugg?.rules || []).length === 0 ? (
            <p className="na-hint">Nothing was reached yet. Run the workload a little longer, then review again.</p>
          ) : (
            <>
              <p>Keep the destinations you expect. The platform rules (Veyron, DNS) are always included.</p>
              <ul className="na-pick">
                {sugg.rules.map((r) => (
                  <li key={ruleKey(r)}>
                    <label>
                      <input type="checkbox" checked={!!picked[ruleKey(r)]} onChange={() => setPicked((p) => ({ ...p, [ruleKey(r)]: !p[ruleKey(r)] }))} />
                      <span className="mono">{ruleLabel(r)}</span>
                      {r.note && <small>{r.note}</small>}
                    </label>
                  </li>
                ))}
              </ul>
            </>
          )}
          {sugg && sugg.valid === false && <p className="na-err sm">{sugg.problem}</p>}
          <div className="na-row">
            <button type="button" className="na-btn" onClick={() => setStep(0)}>Back</button>
            <button type="button" className="na-btn primary" disabled={busy} onClick={apply}>
              <Lock size={15} /> Lock down{chosenCount ? ` to ${chosenCount} destination${chosenCount === 1 ? '' : 's'}` : ''}
            </button>
          </div>
        </div>
      )}

      {step === 2 && (
        <div className="na-step na-done">
          <svg viewBox="0 0 52 52" width="64" height="64" aria-hidden="true"><circle cx="26" cy="26" r="24" className="ring" /><path d="M15 27l8 8 15-17" className="tick" /></svg>
          <p><b>Locked down.</b> The VM can now reach only what you kept.</p>
          <button type="button" className="na-btn" onClick={() => setStep(0)}>Learn again</button>
        </div>
      )}
    </section>
  );
}

/* ——— one VM's panel ———————————————————————————————————————————————— */

export function NetworkAccessPanel({ ns, name, running = true, template = '', compact = false, onChanged }) {
  const reduced = useReducedMotion();
  const [data, setData] = useState(null);
  const [presets, setPresets] = useState([]);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const [msg, setMsg] = useState('');
  const [openFor, setOpenFor] = useState('1h');
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [showWizard, setShowWizard] = useState(false);

  const load = useCallback(async () => {
    try {
      const d = await api.vmEgress(ns, name);
      setData(d);
      setErr('');
      return d;
    } catch (e) {
      setErr(errText(e));
      return null;
    }
  }, [ns, name]);

  useEffect(() => {
    setData(null);
    setMsg('');
    setErr('');
    setShowWizard(false);
    setConfirmOpen(false);
    load();
    api.egressPresets(template).then((p) => setPresets(p?.presets || [])).catch(() => {});
    const t = setInterval(load, 8000);
    return () => clearInterval(t);
  }, [ns, name, template, load]);

  const policy = data?.policy || { mode: 'loopback', rules: [] };
  const flows = data?.observed?.flows || [];
  const grant = data?.grant;
  const active = matchPreset(policy, presets);
  const platform = presets.find((p) => p.id === 'platform');

  const save = async (label, body, ok) => {
    setBusy(label);
    setErr('');
    setMsg('');
    try {
      await api.setVmEgress(ns, name, body);
      await load();
      setMsg(ok || 'Saved. kairon-node applies it to the running VM on its next reconcile.');
      onChanged?.();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy('');
    }
  };

  const pickMode = (id) => {
    if (id === 'open') {
      setConfirmOpen(true);
      return;
    }
    setConfirmOpen(false);
    if (id === 'loopback') save('mode', putBody({ preset: 'isolated' }));
    else if (id === 'audit') save('mode', putBody({ preset: 'learn' }));
    else if (policy.rules.length) save('mode', putBody({ mode: 'allowlist', rules: policy.rules }));
    else save('mode', putBody({ preset: 'platform-packages' }));
  };

  const fixProblem = (p) => {
    const f = p.fix || {};
    if (f.preset) save('fix', putBody({ preset: f.preset }), 'Fixed. The guest agent can reach Veyron.');
    else save('fix', putBody({ mode: f.mode, rules: f.rules || [] }), 'Re-applied.');
  };

  const addRule = (rule) => save('rule', putBody({ mode: 'allowlist', rules: toggleRule(policy.rules, rule) }), `Allowed ${ruleLabel(rule)}.`);
  const removeRule = (rule) => save('rule', putBody({ mode: 'allowlist', rules: policy.rules.filter((r) => ruleKey(r) !== ruleKey(rule)) }), 'Rule removed.');

  const reset = async () => {
    setBusy('reset');
    try {
      await api.clearVmEgress(ns, name);
      await load();
      setMsg('This VM now follows the node default.');
      onChanged?.();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy('');
    }
  };

  const endGrant = async () => {
    setBusy('grant');
    try {
      await api.endVmEgressGrant(ns, name);
      await load();
      setMsg('Timed change ended. Previous policy restored.');
      onChanged?.();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy('');
    }
  };

  if (!data && !err) {
    return (
      <div className="na na-loading" aria-busy="true">
        <div className="na-skel tall" />
        <div className="na-skel" />
        <div className="na-skel" />
      </div>
    );
  }
  if (!data) {
    return (
      <div className="na">
        <p className="na-err">{err}</p>
      </div>
    );
  }

  const modeOptions = [
    { id: 'loopback', label: 'Isolated', tone: 'rose', icon: <Lock size={13} /> },
    { id: 'allowlist', label: 'Allow-list', tone: 'sky', icon: <ShieldCheck size={13} /> },
    { id: 'audit', label: 'Learning', tone: 'amber', icon: <Eye size={13} /> },
    { id: 'open', label: 'Open', tone: 'danger', icon: <Unlock size={13} /> },
  ];
  const sorted = MODE_ORDER.includes(policy.mode) ? policy.mode : 'loopback';

  return (
    <div className={`na${compact ? ' compact' : ''}`}>
      <header className="na-head">
        <div>
          <ModePill mode={sorted} />
          <small>{data.configured ? 'Set on this VM' : 'Following the node default'}{active ? ` · ${active.title}` : ''}</small>
        </div>
        <button type="button" className="na-icon" aria-label="Refresh" onClick={load}><RefreshCw size={14} /></button>
      </header>

      {(data.problems || []).map((p) => (
        <div key={p.code} className={`na-problem ${p.severity}`} role="alert">
          <ShieldAlert size={18} aria-hidden="true" />
          <p>{p.message}</p>
          {p.fix && p.code !== 'open' && (
            <button type="button" className="na-btn primary shimmer" disabled={!!busy} onClick={() => fixProblem(p)}>
              <Zap size={14} /> {p.code === 'agent_blocked' ? 'Allow platform services' : 'Fix it'}
            </button>
          )}
        </div>
      ))}

      {err && <p className="na-err">{err}</p>}
      {msg && <p className="na-ok" role="status"><Check size={14} /> {msg}</p>}
      {data.live_error && <p className="na-hint">{data.live_error}</p>}

      <FlowDiagram policy={policy} flows={flows} running={running && data.running !== false} reduced={reduced} />

      {grant && <GrantRing grant={grant} onEnd={endGrant} busy={busy === 'grant'} />}

      <Segmented label="Network access mode" value={sorted} options={modeOptions} onChange={pickMode} disabled={!!busy} />

      {confirmOpen && (
        <div className="na-confirm" role="alertdialog" aria-label="Open this VM">
          <p><b>Open</b> lets this VM reach anything, including your private networks. Open it only for as long as you need.</p>
          <div className="na-row wrap">
            <select aria-label="Open for" value={openFor} onChange={(e) => setOpenFor(e.target.value)}>
              {DURATIONS.map((d) => <option key={d.id} value={d.id}>for {d.label}</option>)}
              <option value="">until I close it</option>
            </select>
            <button type="button" className="na-btn danger" disabled={!!busy} onClick={() => { setConfirmOpen(false); save('open', putBody({ preset: 'open', expiresIn: openFor || undefined }), openFor ? `Open for ${openFor}, then it reverts.` : 'Open.'); }}>
              <Unlock size={14} /> Open it
            </button>
            <button type="button" className="na-btn" onClick={() => setConfirmOpen(false)}>Cancel</button>
          </div>
        </div>
      )}

      <h3 className="na-h">Presets</h3>
      <div className="na-presets">
        {presets.map((p) => (
          <button key={p.id} type="button" className={`na-preset risk-${p.risk}${active?.id === p.id ? ' on' : ''}`} disabled={!!busy} onClick={() => save('preset', putBody({ preset: p.id }), `${p.title} applied.`)}>
            <span className="na-preset-top">
              <b>{p.title}</b>
              <em>{p.risk === 'none' ? 'safest' : `${p.risk} risk`}</em>
            </span>
            <span className="na-preset-desc">{p.description}</span>
            {p.needs?.length > 0 && (
              <span className="na-needs">{p.needs.map((n) => <i key={n}>{n}</i>)}</span>
            )}
          </button>
        ))}
      </div>

      <h3 className="na-h">Allowed destinations <small>{policy.rules.length}</small></h3>
      <RuleChips rules={policy.rules} onRemove={removeRule} reduced={reduced} />
      <AddRule onAdd={addRule} disabled={!!busy} />

      <h3 className="na-h">Can it reach…?</h3>
      <ExplainBox ns={ns} name={name} onAllow={addRule} disabled={!!busy} />

      {summarizeFlows(flows).blocked.length > 0 && (
        <div className="na-refused">
          <b><ShieldAlert size={14} /> Refused recently</b>
          <ul>
            {summarizeFlows(flows).blocked.slice(0, 5).map((f) => (
              <li key={flowLabel(f)}>
                <span className="mono">{flowLabel(f)}</span>
                <small>{f.packets} pkt</small>
                <button type="button" className="na-btn sm" disabled={!!busy} onClick={() => addRule({ cidr: `${f.dst}/32`, proto: f.proto === 'udp' ? 'udp' : 'tcp', port: f.port })}>Allow</button>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="na-foot">
        <button type="button" className="na-btn" onClick={() => setShowWizard((v) => !v)}>
          <Sparkles size={14} /> {showWizard ? 'Hide' : 'Learn & lock down'}
        </button>
        {data.configured && (
          <button type="button" className="na-link" disabled={!!busy} onClick={reset}>Use the node default</button>
        )}
      </div>
      {showWizard && <LearnWizard ns={ns} name={name} platformRules={platform?.rules || []} current={policy} setErr={setErr} onDone={load} />}
    </div>
  );
}

/** The inspector tab. */
export function VmNetworkAccess({ row }) {
  const ns = row.ns || 'default';
  return <NetworkAccessPanel ns={ns} name={row.name} running={row.status === 'Running'} compact />;
}

/* ——— fleet page ——————————————————————————————————————————————————————— */

function Stat({ label, value, tone, reduced, onClick, active }) {
  const shown = useCountUp(value, reduced);
  const Tag = onClick ? 'button' : 'div';
  return (
    <Tag type={onClick ? 'button' : undefined} className={`na-stat tone-${tone}${active ? ' on' : ''}`} onClick={onClick} aria-pressed={onClick ? !!active : undefined}>
      <b>{shown}</b>
      <span>{label}</span>
    </Tag>
  );
}

function BulkSheet({ tiles, presets, onClose, onDone }) {
  const [preset, setPreset] = useState('platform-packages');
  const [dur, setDur] = useState('');
  const [plan, setPlan] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const groups = useMemo(() => {
    const by = {};
    for (const t of tiles) (by[t.namespace] ||= []).push(t.name);
    return by;
  }, [tiles]);

  const run = async (dry) => {
    setBusy(true);
    setErr('');
    try {
      const results = [];
      for (const [namespace, names] of Object.entries(groups)) {
        const r = await api.egressApply({ selector: { namespaces: [namespace], names }, preset, ...(dur ? { expires_in: dur } : {}), dry_run: dry });
        results.push(...(r.results || []));
      }
      setPlan({ dry, results });
      if (!dry) onDone();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const failed = (plan?.results || []).filter((r) => r.ok === false || r.error).length;
  const changes = (plan?.results || []).filter((r) => !r.unchanged && !r.error).length;
  return (
    <div className="na-sheet-back" role="presentation" onClick={onClose}>
      <aside className="na-sheet" role="dialog" aria-modal="true" aria-label="Change network access" onClick={(e) => e.stopPropagation()}>
        <header>
          <h3>Change {tiles.length} VM{tiles.length === 1 ? '' : 's'}</h3>
          <button type="button" className="na-icon" aria-label="Close" onClick={onClose}><X size={16} /></button>
        </header>
        <label className="na-field">Preset
          <select value={preset} onChange={(e) => { setPreset(e.target.value); setPlan(null); }}>
            {presets.map((p) => <option key={p.id} value={p.id}>{p.title} · {p.risk === 'none' ? 'safest' : `${p.risk} risk`}</option>)}
          </select>
        </label>
        <label className="na-field">Duration
          <select value={dur} onChange={(e) => { setDur(e.target.value); setPlan(null); }}>
            <option value="">Permanent</option>
            {DURATIONS.map((d) => <option key={d.id} value={d.id}>{d.label}, then revert</option>)}
          </select>
        </label>
        {err && <p className="na-err">{err}</p>}
        {plan && (
          <div className="na-plan" role="status">
            <b>{plan.dry ? 'Preview' : 'Done'}: {changes} change{changes === 1 ? '' : 's'}{failed ? `, ${failed} failed` : ''}</b>
            <ul>
              {plan.results.slice(0, 40).map((r) => (
                <li key={`${r.namespace}/${r.name}`} className={r.error ? 'bad' : r.unchanged ? 'same' : ''}>
                  <span className="mono">{r.namespace}/{r.name}</span>
                  <small>{r.error ? r.error : r.unchanged ? 'no change' : `${r.from} → ${r.to}`}</small>
                </li>
              ))}
            </ul>
          </div>
        )}
        <div className="na-row">
          <button type="button" className="na-btn" disabled={busy} onClick={() => run(true)}>Preview</button>
          <button type="button" className="na-btn primary" disabled={busy || !plan || !plan.dry || changes === 0} onClick={() => run(false)}>
            {busy ? <RefreshCw size={14} className="spin" /> : <Check size={14} />} Apply to {changes || tiles.length}
          </button>
        </div>
        <p className="na-hint">Preview first: nothing changes until you apply.</p>
      </aside>
    </div>
  );
}

export function NetworkAccessPage() {
  const reduced = useReducedMotion();
  const [data, setData] = useState(null);
  const [presets, setPresets] = useState([]);
  const [live, setLive] = useState(true);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const [mode, setMode] = useState('all');
  const [only, setOnly] = useState('');
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(null);
  const [bulk, setBulk] = useState(false);

  const load = useCallback(async () => {
    setBusy(true);
    setErr('');
    try {
      setData(await api.egressOverview({ live }));
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }, [live]);

  useEffect(() => {
    load();
  }, [load]);
  useEffect(() => {
    api.egressPresets().then((p) => setPresets(p?.presets || [])).catch(() => {});
  }, []);

  const items = data?.items || [];
  const t = tallies(items);
  const tiles = filterTiles(items, { mode, only, query });
  const selected = sel && items.find((i) => `${i.namespace}/${i.name}` === sel);

  return (
    <div className="chapter-page na-page">
      <PageHero kicker="Network" title="Network access" lede="What each VM may reach, in one place: who is isolated, who is learning, who is open, and who is being refused. Change one VM or hundreds, with a preview." onRefresh={load} busy={busy} />
      {err && <div className="ops-err page-err">{err}</div>}
      <section className="apple-band na-band">
        <div className="na-inner">
        <div className="na-stats">
          <Stat label="VMs" value={t.total} tone="gray" reduced={reduced} onClick={() => { setMode('all'); setOnly(''); }} active={mode === 'all' && !only} />
          <Stat label="Allow-list" value={t.byMode.allowlist || 0} tone="sky" reduced={reduced} onClick={() => setMode(mode === 'allowlist' ? 'all' : 'allowlist')} active={mode === 'allowlist'} />
          <Stat label="Isolated" value={t.byMode.loopback || 0} tone="rose" reduced={reduced} onClick={() => setMode(mode === 'loopback' ? 'all' : 'loopback')} active={mode === 'loopback'} />
          <Stat label="Learning" value={t.byMode.audit || 0} tone="amber" reduced={reduced} onClick={() => setMode(mode === 'audit' ? 'all' : 'audit')} active={mode === 'audit'} />
          <Stat label="Open" value={t.byMode.open || 0} tone="danger" reduced={reduced} onClick={() => setMode(mode === 'open' ? 'all' : 'open')} active={mode === 'open'} />
          <Stat label="Node default" value={t.byMode.inherit || 0} tone="gray" reduced={reduced} onClick={() => setMode(mode === 'inherit' ? 'all' : 'inherit')} active={mode === 'inherit'} />
        </div>

        <div className="na-toolbar">
          <label className="na-search">
            <Search size={15} aria-hidden="true" />
            <input placeholder="Filter VMs" value={query} onChange={(e) => setQuery(e.target.value)} aria-label="Filter VMs" />
          </label>
          <button type="button" className={`na-chipbtn${only === 'blocked' ? ' on' : ''}`} onClick={() => setOnly(only === 'blocked' ? '' : 'blocked')}><ShieldAlert size={13} /> Refused flows · {t.blocked}</button>
          <button type="button" className={`na-chipbtn${only === 'noagent' ? ' on' : ''}`} onClick={() => setOnly(only === 'noagent' ? '' : 'noagent')}><Radar size={13} /> No agent · {t.noAgent}</button>
          <button type="button" className={`na-chipbtn${only === 'running' ? ' on' : ''}`} onClick={() => setOnly(only === 'running' ? '' : 'running')}><Globe size={13} /> Running · {t.running}</button>
          <label className="na-toggle">
            <input type="checkbox" checked={live} onChange={(e) => setLive(e.target.checked)} />
            <span aria-hidden="true" />
            Live probes
          </label>
          <button type="button" className="na-btn primary" disabled={!tiles.length} onClick={() => setBulk(true)}>
            <Wand2 size={14} /> Change {tiles.length}
          </button>
        </div>

        {data && tiles.length === 0 ? (
          <p className="na-hint na-empty">No VMs match.</p>
        ) : (
          <div className="na-heat" role="list" aria-label="VMs by network access">
            {tiles.map((i, idx) => {
              const key = `${i.namespace}/${i.name}`;
              const heat = heatLevel(i.blocked);
              return (
                <button
                  key={key}
                  type="button"
                  role="listitem"
                  className={`na-tile tone-${modeInfo(i.mode).tone} heat-${heat}${sel === key ? ' on' : ''}${i.running ? '' : ' off'}`}
                  style={reduced ? undefined : { animationDelay: `${Math.min(idx, 40) * 18}ms` }}
                  onClick={() => setSel(sel === key ? null : key)}
                  title={`${key} · ${modeInfo(i.mode).label}${i.blocked ? ` · ${i.blocked} refused` : ''}`}
                >
                  <b>{i.name}</b>
                  <small>{i.namespace}</small>
                  <span className="na-tile-foot">
                    <ModePill mode={i.mode} small />
                    {i.blocked > 0 && <em>{i.blocked}</em>}
                    {i.running && i.agent_connected === false && <ShieldAlert size={12} aria-label="No guest agent" />}
                  </span>
                </button>
              );
            })}
          </div>
        )}
        </div>
      </section>

      {selected && (
        <section className="na-detail">
          <div className="na-inner">
            <div className="na-detail-head">
              <h3>{selected.namespace}/{selected.name}</h3>
              <button type="button" className="na-icon" aria-label="Close" onClick={() => setSel(null)}><X size={16} /></button>
            </div>
            <NetworkAccessPanel key={sel} ns={selected.namespace} name={selected.name} running={selected.running} template={selected.template || ''} onChanged={load} />
          </div>
        </section>
      )}

      {bulk && <BulkSheet tiles={tiles} presets={presets} onClose={() => setBulk(false)} onDone={load} />}
    </div>
  );
}
