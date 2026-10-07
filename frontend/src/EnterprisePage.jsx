/* Copyright 2026 Zyvor AI Labs · https://zyvor.dev
 * SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 */
import { useEffect, useState } from 'react';
import { enterpriseCapabilities, enterpriseAssessment, enterpriseOperations, enterpriseSubmit, enterpriseCancel } from './api.js';
import { ASSESSMENT_KINDS, SAMPLES, parseAssessment } from './enterprise.js';
import './enterprise.css';
export default function EnterprisePage({ user }) {
  const [caps,setCaps]=useState(null);
  const [error,setError]=useState('');
  const [namespace,setNamespace]=useState('default');
  const [vm,setVm]=useState('');
  const [action,setAction]=useState('start');
  const [key,setKey]=useState(()=>crypto.randomUUID());
  const [busy,setBusy]=useState(false);
  const [rows,setRows]=useState([]);
  const [kind,setKind]=useState('blueprint');
  const [text,setText]=useState(()=>JSON.stringify(SAMPLES.blueprint,null,2));
  const [result,setResult]=useState(null);
  const writable=['admin','write'].includes(String(user?.role || '').toLowerCase());
  useEffect(()=>{let active=true;enterpriseCapabilities().then(v=>{if(active)setCaps(v);}).catch(e=>{if(active)setError(e.message);});return()=>{active=false;};},[]);
  useEffect(()=>{setRows([]);},[namespace]);
  async function run(fn) {setBusy(true);setError('');try{await fn();}catch(e){setError(e.message);}finally{setBusy(false);}}
  function change(setter,value){setter(value);setKey(crypto.randomUUID());}
  async function refresh(){setRows(await enterpriseOperations(namespace));}
  return <div className="enterprise-page">
    <header><p className="enterprise-eyebrow">Enterprise operations</p><h1>Workflow center</h1><p>Plan changes with explicit evidence. Track power requests through the controller’s observed state.</p></header>
    {error && <p role="alert" className="enterprise-error">{error}</p>}
    <section className="enterprise-card"><h2>Backend capabilities</h2>{caps ? <><p>Active backend: <strong>{caps.backend}</strong></p><div className="enterprise-capabilities">{Object.entries(caps.execution).map(([name,ok])=><span key={name}>{name}: {ok?'Executable':'Not executable here'}</span>)}</div><p>Power completion confirms the VM state. Application health needs a separate check.</p></> : <p>Loading capabilities…</p>}</section>
    <section className="enterprise-card"><h2>Power workflow</h2><form onSubmit={e=>{e.preventDefault();run(async()=>{await enterpriseSubmit({namespace,vm,action,idempotency_key:key});await refresh();});}}>
      <div className="enterprise-fields"><label>Namespace<input disabled={busy} required value={namespace} onChange={e=>change(setNamespace,e.target.value)} /></label><label>Virtual machine<input disabled={busy} required value={vm} onChange={e=>change(setVm,e.target.value)} /></label><label>Action<select disabled={busy} value={action} onChange={e=>change(setAction,e.target.value)}><option value="start">Start</option><option value="stop">Stop</option></select></label></div>
      <p>Retrying the same request uses the same key. Changing the fields starts a new request.</p>
      <div className="enterprise-actions"><button disabled={busy || !writable} type="submit">Submit power request</button><button disabled={busy} type="button" onClick={()=>setKey(crypto.randomUUID())}>New request key</button><button disabled={busy} type="button" onClick={()=>run(refresh)}>Refresh history</button></div>
    </form><div className="enterprise-table"><table><thead><tr><th>VM</th><th>Action</th><th>State</th><th>Result</th><th>Controls</th></tr></thead><tbody>{rows.map(op=><tr key={op.id}><td>{op.vm}</td><td>{op.action}</td><td>{op.phase}</td><td>{op.message}</td><td>{op.phase==='queued' && <button disabled={busy || !writable} onClick={()=>run(async()=>{await enterpriseCancel(namespace,op.id);await refresh();})}>Cancel</button>}</td></tr>)}</tbody></table>{!rows.length && <p>Refresh history to load requests for this namespace.</p>}</div></section>
    <section className="enterprise-card"><h2>Readiness assessment</h2><p>Assessments report blockers. They do not execute migrations, recovery, maintenance or application deployment. Placement reads live KubeVirt inventory; other checks use the evidence you supply.</p><form onSubmit={e=>{e.preventDefault();run(async()=>setResult(await enterpriseAssessment(kind,parseAssessment(text))));}}><label>Assessment<select value={kind} onChange={e=>{setKind(e.target.value);setText(JSON.stringify(SAMPLES[e.target.value],null,2));setResult(null);}}>{ASSESSMENT_KINDS.map(k=><option key={k} value={k}>{k}</option>)}</select></label><label>Evidence JSON<textarea spellCheck="false" rows={16} value={text} onChange={e=>setText(e.target.value)} /></label><button disabled={busy} type="submit">Assess readiness</button></form>{result && <pre aria-live="polite">{JSON.stringify(result,null,2)}</pre>}</section>
  </div>;
}
