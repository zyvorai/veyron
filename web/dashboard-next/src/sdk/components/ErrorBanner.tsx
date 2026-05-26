// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { CSSProperties } from 'react';

type Props = {
  message: string;
  style?: CSSProperties;
};

export function ErrorBanner({ message, style }: Props) {
  if (!message) return null;
  return (
    <div role="alert" style={{ ...errorBox, ...style }}>
      {message}
    </div>
  );
}

export const errorBox: CSSProperties = {
  marginBottom: 16,
  padding: '12px 14px',
  borderRadius: 8,
  background: '#fef2f2',
  border: '1px solid #fecaca',
  color: '#b91c1c',
  fontSize: 14,
  lineHeight: 1.45,
};

export default ErrorBanner;
