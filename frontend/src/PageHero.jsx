import { RefreshCw } from 'lucide-react';

export function PageHero({ kicker, title, lede, dense, onRefresh, busy }) {
  return (
    <div className={`apple-chapter${dense ? ' dense' : ''}`}>
      <div className="kicker">{kicker}</div>
      <h2>{title}</h2>
      <p>{lede}</p>
      {onRefresh && (
        <div className="acts" style={{ justifyContent: 'flex-start', marginTop: 16 }}>
          <button className="btn secondary" disabled={busy} onClick={onRefresh}>
            <RefreshCw size={13} className={busy ? 'spin' : ''} />
            Refresh
          </button>
        </div>
      )}
    </div>
  );
}
