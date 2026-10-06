import { useEffect, useState } from 'react';
import { Sparkles, ArrowRight } from 'lucide-react';
import { api, relTime } from './api.js';

/** Latest AI investigations, shown above the incident timeline. */
export function InvestigationStrip({ onOpen }) {
  const [items, setItems] = useState(null);
  useEffect(() => {
    let live = true;
    api
      .listInvestigations()
      .then((r) => live && setItems(r))
      .catch(() => live && setItems([]));
    return () => {
      live = false;
    };
  }, []);
  if (!items?.length) return null;
  return (
    <section className="ai-strip" aria-label="AI investigations">
      <div className="ai-strip-h">
        <Sparkles size={14} />
        <b>Investigation</b>
        <span className="ai-muted">Veyron AI looked into the latest incidents</span>
        <button type="button" className="btn sm secondary" onClick={onOpen}>
          All investigations <ArrowRight size={12} />
        </button>
      </div>
      {items.slice(0, 3).map((inv) => (
        <button type="button" key={inv.id} className="ai-strip-item" onClick={onOpen}>
          <span className="ai-sev" data-sev={inv.severity}>
            {inv.severity}
          </span>
          <span className="ai-strip-text">
            <b>{inv.title}</b>
            <small>{inv.root_cause || inv.recommendations?.[0] || inv.trigger}</small>
          </span>
          <small className="ai-muted">{relTime(inv.created_at)}</small>
        </button>
      ))}
    </section>
  );
}
