// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import React from 'react';

export const Footer: React.FC = () => {
  return (
    <footer
      style={{
        backgroundColor: '#222324',
        color: '#fff',
        marginTop: '64px',
      }}
    >
      <div
        style={{
          maxWidth: '1400px',
          margin: '0 auto',
          padding: '0 24px 24px',
        }}
      >
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
            gap: '48px',
            marginBottom: '48px',
            paddingTop: '32px',
          }}
        >
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: '600', marginBottom: '16px' }}>about VMRogue</h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {['Dashboard', 'REST API /api/v1', 'KubeVirt CRDs', 'Build from source'].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a href="#" style={{ color: '#fff', fontSize: '14px', textDecoration: 'none', opacity: 0.8 }}>
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: '600', marginBottom: '16px' }}>kubevirt</h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {['VirtualMachines', 'VMI', 'Migrations', 'Snapshots'].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a href="#" style={{ color: '#fff', fontSize: '14px', textDecoration: 'none', opacity: 0.8 }}>
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: '600', marginBottom: '16px' }}>cluster ops</h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {['Namespaces', 'Nodes', 'Storage classes', 'Network policies'].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a href="#" style={{ color: '#fff', fontSize: '14px', textDecoration: 'none', opacity: 0.8 }}>
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: '600', marginBottom: '16px' }}>quick links</h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {['Health', 'VM list', 'Alerts', 'Metrics WS'].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a href="#" style={{ color: '#fff', fontSize: '14px', textDecoration: 'none', opacity: 0.8 }}>
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </footer>
  );
};
