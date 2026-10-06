import { useEffect, useState } from 'react';
import { Plug, Trash2, Loader2 } from 'lucide-react';
import { api } from './api.js';
import { Status } from './status.jsx';

const FIELDS = ['name', 'url', 'enabled', 'allow_write', 'read_only_tools', 'tls_verify', 'description'];
const strip = (s) => Object.fromEntries(FIELDS.filter((k) => s[k] !== undefined).map((k) => [k, s[k]]));

/** External MCP servers whose tools Veyron AI may use (Settings → AI). */
export function McpServersBox() {
  const [servers, setServers] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const [draft, setDraft] = useState({ name: '', url: '', token: '', allow_write: false });

  useEffect(() => {
    let live = true;
    api
      .listMcpServers()
      .then((r) => live && setServers(r))
      .catch((e) => {
        if (!live) return;
        setServers([]);
        if (!/403|admin/i.test(e.message || '')) setErr(e.message || String(e));
      });
    return () => {
      live = false;
    };
  }, []);

  const save = async (next) => {
    setBusy(true);
    setErr('');
    try {
      const r = await api.putMcpServers(next);
      setServers(Array.isArray(r) ? r : next);
      return true;
    } catch (e) {
      setErr(e.message || String(e));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const update = (name, patch) => save(servers.map((s) => (s.name === name ? { ...strip(s), ...patch } : strip(s))));

  const add = async (e) => {
    e.preventDefault();
    const name = draft.name.trim().toLowerCase();
    const entry = { name, url: draft.url.trim(), enabled: true, allow_write: draft.allow_write };
    if (draft.token.trim()) entry.token = draft.token.trim();
    if (await save([...(servers || []).map(strip), entry])) setDraft({ name: '', url: '', token: '', allow_write: false });
  };

  const summary = (s) => {
    if (!s.enabled) return 'Disabled';
    if (!s.connected) return s.error || 'Not connected';
    const tools = s.tools || [];
    const offered = tools.filter((t) => t.offered).length;
    const writes = tools.filter((t) => !t.read_only).length;
    return `${offered} of ${tools.length} tools offered${writes ? ` · ${writes} can change things${s.allow_write ? ' (need Admin approval)' : ' (hidden)'}` : ''}`;
  };

  return (
    <div className="sbox">
      {err && <div className="srow2"><small className="ai-bad">{err}</small></div>}
      {servers === null ? (
        <div className="srow2"><small className="dim">Checking…</small></div>
      ) : (
        servers.map((s) => (
          <div className="srow2" key={s.name}>
            <Plug size={16} style={{ color: 'var(--accent)' }} />
            <div style={{ minWidth: 0 }}>
              <b>{s.name}</b>
              <small title={s.url}>{summary(s)}</small>
            </div>
            <span className="r" style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              {s.enabled && <Status s={s.connected ? 'Healthy' : 'Failed'} />}
              <button
                className="sw"
                role="switch"
                aria-label={`Use ${s.name}`}
                aria-checked={!!s.enabled}
                disabled={busy}
                onClick={() => update(s.name, { enabled: !s.enabled })}
              >
                <i />
              </button>
              <button
                className="btn sm secondary"
                aria-label={`Remove ${s.name}`}
                disabled={busy}
                onClick={() => window.confirm(`Remove MCP server ${s.name}?`) && save(servers.filter((x) => x.name !== s.name).map(strip))}
              >
                <Trash2 size={13} />
              </button>
            </span>
          </div>
        ))
      )}
      <form className="srow2 ai-mcp-add" onSubmit={add}>
        <input aria-label="Server name" placeholder="name (e.g. github)" value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} required pattern="[a-z0-9-]{1,32}" />
        <input aria-label="Server URL" placeholder="https://host/mcp" type="url" value={draft.url} onChange={(e) => setDraft({ ...draft, url: e.target.value })} required />
        <input aria-label="Bearer token" placeholder="token (optional)" type="password" autoComplete="off" value={draft.token} onChange={(e) => setDraft({ ...draft, token: e.target.value })} />
        <label className="ai-muted" title="Tools not marked read-only become proposals an Admin approves">
          <input type="checkbox" checked={draft.allow_write} onChange={(e) => setDraft({ ...draft, allow_write: e.target.checked })} /> write tools
        </label>
        <button className="btn sm" disabled={busy}>
          {busy ? <Loader2 size={13} className="spin" /> : 'Add'}
        </button>
      </form>
    </div>
  );
}
