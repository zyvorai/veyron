/* ============================================================
   SectionHeading — section title with optional eyebrow + fade line.
   Usage:
     <SectionHeading eyebrow="cluster" title="Fleet Health" />
     <SectionHeading title="Alerts" count={3} plasma />
   ============================================================ */

interface SectionHeadingProps {
  title: string;
  eyebrow?: string;
  count?: number;
  plasma?: boolean;   // accent the title with plasma color
  action?: React.ReactNode;
  className?: string;
}

export function SectionHeading({ title, eyebrow, count, plasma = false, action, className }: SectionHeadingProps) {
  return (
    <div
      className={className}
      style={{
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'space-between',
        gap: 12,
        paddingBottom: 10,
        borderBottom: '1px solid var(--hairline)',
        marginBottom: 16,
      }}
    >
      <div>
        {eyebrow && (
          <div style={{
            fontFamily: 'var(--mono-deck)',
            fontSize: 9,
            fontWeight: 700,
            letterSpacing: '0.12em',
            textTransform: 'uppercase',
            color: 'var(--ink-3)',
            marginBottom: 4,
          }}>
            {eyebrow}
          </div>
        )}
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <h2 style={{
            margin: 0,
            fontFamily: 'var(--sans-deck)',
            fontSize: 17,
            fontWeight: 600,
            letterSpacing: '-0.01em',
            color: plasma ? 'var(--plasma)' : 'var(--ink)',
          }}>
            {title}
          </h2>
          {count !== undefined && (
            <span style={{
              fontFamily: 'var(--mono-deck)',
              fontSize: 11,
              fontWeight: 600,
              color: 'var(--ink-3)',
              fontFeatureSettings: '"tnum"',
            }}>
              {count}
            </span>
          )}
        </div>
      </div>

      {action && (
        <div style={{ flexShrink: 0, paddingBottom: 1 }}>
          {action}
        </div>
      )}
    </div>
  );
}
