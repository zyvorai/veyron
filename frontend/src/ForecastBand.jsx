import { useEffect, useState } from 'react';
import { Sparkles, ChevronRight } from 'lucide-react';
import { api } from './api.js';
import { Reveal } from './story.jsx';

function eta(h) {
  if (h == null) return 'now';
  if (h < 1) return `${Math.max(1, Math.round(h * 60))} min`;
  if (h < 48) return `${Math.round(h)} h`;
  return `${Math.round(h / 24)} d`;
}

/** Predictive ops: what Veyron AI expects to break next, and the proposed fix. */
export function ForecastBand({ go }) {
  const [f, setF] = useState(null);
  useEffect(() => {
    let live = true;
    api
      .aiForecast()
      .then((r) => live && setF(r))
      .catch(() => live && setF({}));
    return () => {
      live = false;
    };
  }, []);
  if (!f) return null;
  const items = f.items || [];
  return (
    <section className="story-band">
      <Reveal className="apple-chapter">
        <div className="kicker">
          <Sparkles size={12} /> Forecast
        </div>
        <h2>{items.length ? 'Coming up.' : 'Nothing on the horizon.'}</h2>
        <p>
          {f.note ||
            `Trends from the last ${f.history_hours ?? 48} hours of disk, memory, restart and node samples.`}
        </p>
        <div className="ai-forecast">
          {items.slice(0, 6).map((it) => (
            <button
              type="button"
              key={it.id}
              className="ai-forecast-item"
              onClick={() => go(it.proposal_id ? 'proposals' : 'vms')}
              title={it.detail}
            >
              <span className="ai-sev" data-sev={it.severity}>
                {it.severity}
              </span>
              <b>{eta(it.eta_hours)}</b>
              <span>
                {it.title}
                {it.fix ? <small className="ai-muted"> · Fix: {it.fix}</small> : null}
              </span>
              {it.proposal_id ? <small className="ai-muted">Review proposal</small> : null}
              <ChevronRight size={14} />
            </button>
          ))}
        </div>
      </Reveal>
    </section>
  );
}
