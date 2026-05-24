import React from 'react';
import { requestVmrogueNav } from '../../lib/nav';
import { ZYVOR_DOCS_URL } from './ZyvorBrand';

const card: React.CSSProperties = {
  backgroundColor: '#fff',
  borderRadius: '4px',
  border: '2px solid #e0e0e0',
  padding: '16px',
};

const bar: React.CSSProperties = {
  width: '2px',
  height: '12px',
  backgroundColor: '#f0583a',
  marginRight: '6px',
};

const linkBtn: React.CSSProperties = {
  display: 'inline-block',
  marginTop: '10px',
  marginRight: '8px',
  padding: '6px 12px',
  fontSize: '11px',
  fontWeight: 600,
  border: '1px solid #222324',
  borderRadius: '4px',
  background: '#fff',
  color: '#222324',
  textDecoration: 'none',
  cursor: 'pointer',
};

type PanelProps = {
  onOpenInventory?: () => void;
};

export const VmrogueInventoryPanel: React.FC<PanelProps> = ({ onOpenInventory }) => (
  <div style={card}>
    <div style={{ display: 'flex', alignItems: 'center', marginBottom: '8px' }}>
      <div style={bar} />
      <h3 style={{ margin: 0, fontSize: '12px', fontWeight: 600 }}>VM inventory</h3>
    </div>
    <p style={{ margin: 0, fontSize: '11px', color: '#6b7280', lineHeight: 1.5 }}>
      Browse KubeVirt VirtualMachines by namespace, run start/stop/restart, and manage RDP exposure from the
      Clusters &amp; VMs view.
    </p>
    {onOpenInventory ? (
      <button type="button" onClick={onOpenInventory} style={linkBtn}>
        Open Clusters &amp; VMs
      </button>
    ) : null}
  </div>
);

export const VmroguePlatformPanel: React.FC = () => (
  <div style={card}>
    <div style={{ display: 'flex', alignItems: 'center', marginBottom: '8px' }}>
      <div style={bar} />
      <h3 style={{ margin: 0, fontSize: '12px', fontWeight: 600 }}>Platform &amp; docs</h3>
    </div>
    <p style={{ margin: 0, fontSize: '11px', color: '#6b7280', lineHeight: 1.5 }}>
      Nodes, storage, GitOps, and CRD inventory live in the operator sidebar. VMRogue CRD editors and advanced
      GitOps actions remain in the full dashboard.
    </p>
    <button type="button" onClick={() => requestVmrogueNav({ view: 'platform' })} style={linkBtn}>
      Open platform
    </button>
    <a href="/dashboard" style={linkBtn}>
      Full dashboard
    </a>
    <a href={ZYVOR_DOCS_URL} target="_blank" rel="noopener noreferrer" style={linkBtn}>
      Zyvor docs
    </a>
  </div>
);
