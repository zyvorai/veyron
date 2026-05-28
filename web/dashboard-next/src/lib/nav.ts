// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** Cross-view navigation for dashboard-next (shell + in-page sections). */

export type VmrogueView =
  | 'dashboard'
  | 'inventory'
  | 'nodes'
  | 'storage'
  | 'platform'
  | 'workloads'
  | 'insights'
  | 'compliance'
  | 'operations'
  | 'fleet'
  | 'integrations';

export const VMROGUE_NAV_EVENT = 'vmrogue:nav';

export type VmrogueNavDetail = {
  view: VmrogueView;
  scrollTo?: string;
};

export function requestVmrogueNav(detail: VmrogueNavDetail): void {
  window.dispatchEvent(new CustomEvent(VMROGUE_NAV_EVENT, { detail }));
}

export function scrollDashboardSection(id: string): void {
  const el = document.getElementById(id);
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

/** Scroll a section inside the active dashboard-next view (e.g. Insights panels). */
export function scrollViewSection(id: string): void {
  scrollDashboardSection(id);
}
