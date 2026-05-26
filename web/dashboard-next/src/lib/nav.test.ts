// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  VMROGUE_NAV_EVENT,
  requestVmrogueNav,
  scrollDashboardSection,
  type VmrogueNavDetail,
} from './nav';

describe('nav helpers', () => {
  afterEach(() => {
    document.body.innerHTML = '';
    vi.restoreAllMocks();
  });

  it('dispatches vmrogue:nav with detail', () => {
    const detail: VmrogueNavDetail = { view: 'inventory', scrollTo: 'alerts' };
    const handler = vi.fn();
    window.addEventListener(VMROGUE_NAV_EVENT, handler);
    requestVmrogueNav(detail);
    expect(handler).toHaveBeenCalledTimes(1);
    expect((handler.mock.calls[0][0] as CustomEvent<VmrogueNavDetail>).detail).toEqual(detail);
  });

  it('scrolls to dashboard section when element exists', () => {
    const target = document.createElement('section');
    target.id = 'alerts';
    const scrollIntoView = vi.fn();
    target.scrollIntoView = scrollIntoView;
    document.body.appendChild(target);
    scrollDashboardSection('alerts');
    expect(scrollIntoView).toHaveBeenCalledWith({ behavior: 'smooth', block: 'start' });
  });
});
