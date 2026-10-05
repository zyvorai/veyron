export function LogsPanel({ text, loading, error }) {
  return (
    <div className="console">
      <div className="console-logs">
        {loading && 'Loading logs…'}
        {error && <span style={{ color: '#ff7b72' }}>{error}</span>}
        {!loading && !error && (text || '(empty)')}
      </div>
    </div>
  );
}
