import { useCallback, useEffect, useRef, useState, Fragment } from 'react';
import { Sparkles, X, Send, Square, Wrench, Check, AlertTriangle, RotateCcw } from 'lucide-react';
import { api, streamAiChat } from './api.js';
import { ProposalCard } from './ProposalCard.jsx';
import { useFocusTrap } from './a11y.js';

const SUGGESTIONS = [
  'What needs my attention right now?',
  'Which VMs are idle or oversized, and what would I save?',
  'Why did my last VM fail to start?',
  'Is any node running out of capacity?',
];

/** Inline formatting for model text: **bold**, `code`. */
function inline(text) {
  const parts = String(text).split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  return parts.map((s, i) => {
    if (s.startsWith('**') && s.endsWith('**') && s.length > 4) return <b key={i}>{s.slice(2, -2)}</b>;
    if (s.startsWith('`') && s.endsWith('`') && s.length > 2) return <code key={i}>{s.slice(1, -1)}</code>;
    return <Fragment key={i}>{s}</Fragment>;
  });
}

/** Small, safe Markdown subset (headings, lists, code fences, paragraphs). No HTML. */
export function Md({ text }) {
  const out = [];
  const lines = String(text || '').split('\n');
  let list = null;
  let code = null;
  const flush = () => {
    if (list) out.push(list.ordered ? <ol key={out.length}>{list.items}</ol> : <ul key={out.length}>{list.items}</ul>);
    list = null;
  };
  lines.forEach((raw, idx) => {
    const line = raw.replace(/\s+$/, '');
    if (line.startsWith('```')) {
      if (code) {
        out.push(<pre key={`c${idx}`}>{code.join('\n')}</pre>);
        code = null;
      } else {
        flush();
        code = [];
      }
      return;
    }
    if (code) {
      code.push(raw);
      return;
    }
    const bullet = line.match(/^\s*[-*•]\s+(.*)$/);
    const num = line.match(/^\s*\d+[.)]\s+(.*)$/);
    if (bullet || num) {
      const ordered = !!num;
      if (!list || list.ordered !== ordered) {
        flush();
        list = { ordered, items: [] };
      }
      list.items.push(<li key={idx}>{inline((bullet || num)[1])}</li>);
      return;
    }
    flush();
    const h = line.match(/^#{1,4}\s+(.*)$/);
    if (h) out.push(<p key={idx} className="md-h">{inline(h[1])}</p>);
    else if (line.trim()) out.push(<p key={idx}>{inline(line)}</p>);
  });
  if (code) out.push(<pre key="cend">{code.join('\n')}</pre>);
  flush();
  return <div className="md">{out}</div>;
}

function ToolTrace({ calls }) {
  if (!calls.length) return null;
  return (
    <details className="ai-trace">
      <summary>
        <Wrench size={12} />
        {calls.length === 1 ? `Used ${calls[0].name}` : `Used ${calls.length} tools`}
        {calls.some((c) => c.ok === false) && <AlertTriangle size={12} className="warn" />}
      </summary>
      <ul>
        {calls.map((c) => (
          <li key={c.id} data-ok={c.ok == null ? undefined : String(c.ok)}>
            {c.ok == null ? <span className="spin-dot" /> : c.ok ? <Check size={11} /> : <AlertTriangle size={11} />}
            <code>{c.name}</code>
            {c.args && Object.keys(c.args).length > 0 && <small>{JSON.stringify(c.args)}</small>}
            {c.preview && <pre>{c.preview}</pre>}
          </li>
        ))}
      </ul>
    </details>
  );
}

function contextLabel(ctx) {
  if (ctx?.vm_name) return `${ctx.namespace || 'default'}/${ctx.vm_name}`;
  if (ctx?.page && ctx.page !== 'mission') return ctx.page;
  return null;
}

/**
 * Veyron AI drawer (⌘J). Streams the agent, shows each tool it used, and renders
 * change proposals inline so the user can approve them without leaving the page.
 */
export function Assistant({ open, onClose, context, user, seed, onSeedUsed }) {
  const [turns, setTurns] = useState([]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState(null);
  const abortRef = useRef(null);
  const scrollRef = useRef(null);
  const inputRef = useRef(null);
  const panelRef = useRef(null);
  useFocusTrap(panelRef, open);

  useEffect(() => {
    if (!open || status) return;
    api.aiStatus().then(setStatus).catch(() => setStatus({ mode: 'unavailable' }));
  }, [open, status]);

  useEffect(() => {
    if (open) setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [turns]);

  const patchLast = useCallback((fn) => {
    setTurns((t) => {
      if (!t.length) return t;
      const copy = t.slice();
      copy[copy.length - 1] = fn({ ...copy[copy.length - 1] });
      return copy;
    });
  }, []);

  const send = useCallback(
    async (text) => {
      const q = (text ?? input).trim();
      if (!q || busy) return;
      setInput('');
      const history = turns
        .filter((t) => t.text)
        .flatMap((t) => [
          { role: 'user', content: t.q },
          { role: 'assistant', content: t.text },
        ])
        .slice(-12);
      setTurns((t) => [...t, { q, text: '', calls: [], proposals: [], error: '', mode: null, done: false }]);
      setBusy(true);
      const ctrl = new AbortController();
      abortRef.current = ctrl;
      try {
        await streamAiChat(
          { messages: [...history, { role: 'user', content: q }], context: context || {} },
          (ev) => {
            switch (ev.type) {
              case 'start':
                patchLast((t) => ({ ...t, mode: ev.mode, model: ev.model }));
                break;
              case 'token':
                patchLast((t) => ({ ...t, text: t.text + ev.text }));
                break;
              case 'tool_call':
                patchLast((t) => ({ ...t, calls: [...t.calls, { id: ev.id, name: ev.name, args: ev.args }] }));
                break;
              case 'tool_result':
                patchLast((t) => ({
                  ...t,
                  calls: t.calls.map((c) => (c.id === ev.id ? { ...c, ok: ev.ok, preview: ev.preview } : c)),
                }));
                break;
              case 'proposal':
                patchLast((t) => ({ ...t, proposals: [...t.proposals, ev.proposal] }));
                break;
              case 'message':
                patchLast((t) => ({ ...t, text: ev.content || t.text }));
                break;
              case 'error':
                patchLast((t) => ({ ...t, error: ev.message }));
                break;
              case 'done':
                patchLast((t) => ({ ...t, done: true }));
                break;
              default:
            }
          },
          ctrl.signal,
        );
      } catch (e) {
        if (e.name !== 'AbortError') patchLast((t) => ({ ...t, error: e.message || String(e) }));
      } finally {
        patchLast((t) => ({ ...t, done: true }));
        setBusy(false);
        abortRef.current = null;
      }
    },
    [input, busy, turns, context, patchLast],
  );

  useEffect(() => {
    if (open && seed && !busy) {
      send(seed);
      onSeedUsed?.();
    }
  }, [open, seed, busy, send, onSeedUsed]);

  const stop = () => abortRef.current?.abort();
  const ctxLabel = contextLabel(context);
  const mode = status?.mode;

  return (
    <>
      <div className={`ai-scrim${open ? ' on' : ''}`} onClick={onClose} />
      <aside
        ref={panelRef}
        className={`ai-drawer${open ? ' open' : ''}`}
        role="dialog"
        aria-modal={open ? 'true' : undefined}
        aria-hidden={open ? undefined : 'true'}
        aria-label="Veyron AI"
        {...(open ? {} : { inert: '' })}
      >
        <header className="ai-head">
          <span className="ai-mark">
            <Sparkles size={15} />
          </span>
          <div>
            <b>Veyron AI</b>
            <small>
              {mode === 'agent'
                ? `Agent · ${status.model || 'model'}`
                : mode === 'advisor'
                  ? 'Built-in advisors · no model configured'
                  : mode === 'unavailable'
                    ? 'Unavailable'
                    : 'Connecting…'}
            </small>
          </div>
          {turns.length > 0 && (
            <button className="tb" onClick={() => setTurns([])} title="New conversation" aria-label="New conversation">
              <RotateCcw size={14} />
            </button>
          )}
          <button className="tb" onClick={onClose} aria-label="Close assistant">
            <X size={15} />
          </button>
        </header>

        <div className="ai-body" ref={scrollRef}>
          {turns.length === 0 ? (
            <div className="ai-empty">
              <p>
                Ask about your fleet. Veyron AI reads live state with your permissions. Anything it wants to change
                becomes a proposal you approve.
              </p>
              <div className="ai-suggest">
                {(ctxLabel && context?.vm_name
                  ? [`What's wrong with ${context.vm_name}?`, `Right-size ${context.vm_name}`, ...SUGGESTIONS.slice(0, 2)]
                  : SUGGESTIONS
                ).map((s) => (
                  <button key={s} onClick={() => send(s)}>
                    {s}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            turns.map((t, i) => (
              <div key={i} className="ai-turn">
                <div className="ai-q">{t.q}</div>
                <ToolTrace calls={t.calls} />
                {t.text ? (
                  <div className="ai-a">
                    <Md text={t.text} />
                  </div>
                ) : (
                  !t.done && !t.error && <div className="ai-thinking"><span /><span /><span /></div>
                )}
                {t.proposals.map((p) => (
                  <ProposalCard key={p.id} proposal={p} user={user} />
                ))}
                {t.error && <div className="ops-err">{t.error}</div>}
              </div>
            ))
          )}
        </div>

        <form
          className="ai-input"
          onSubmit={(e) => {
            e.preventDefault();
            send();
          }}
        >
          {ctxLabel && <span className="ai-ctx">Looking at {ctxLabel}</span>}
          <div className="ai-input-row">
            <textarea
              ref={inputRef}
              rows={1}
              value={input}
              placeholder="Ask Veyron AI…"
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                  e.preventDefault();
                  send();
                }
              }}
            />
            {busy ? (
              <button type="button" className="tb" onClick={stop} aria-label="Stop">
                <Square size={14} />
              </button>
            ) : (
              <button type="submit" className="tb primary" disabled={!input.trim()} aria-label="Send">
                <Send size={14} />
              </button>
            )}
          </div>
        </form>
      </aside>
    </>
  );
}
