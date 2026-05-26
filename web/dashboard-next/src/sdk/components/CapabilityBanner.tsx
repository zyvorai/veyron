// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { CSSProperties } from 'react';
import type { VmrogueFeatureContext } from '../../lib/api';

type Props = {
  context?: VmrogueFeatureContext | null;
  style?: CSSProperties;
};

export function CapabilityBanner({ context, style }: Props) {
  if (!context?.limitations) return null;
  return (
    <div style={{ ...capabilityBox, ...style }}>
      {context.scope ? (
        <p style={{ margin: '0 0 6px', fontWeight: 600, fontSize: 12, color: '#92400e' }}>
          {context.scope}
        </p>
      ) : null}
      <p style={{ margin: 0, fontSize: 12, color: '#78350f', lineHeight: 1.45 }}>
        {context.limitations}
      </p>
      {context.data_source ? (
        <p style={{ margin: '6px 0 0', fontSize: 11, color: '#a16207' }}>
          Source: {context.data_source}
        </p>
      ) : null}
    </div>
  );
}

export const capabilityBox: CSSProperties = {
  marginBottom: 16,
  padding: '12px 14px',
  borderRadius: 8,
  background: '#fffbeb',
  border: '1px solid #fde68a',
};

export default CapabilityBanner;
