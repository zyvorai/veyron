import { describe, expect, it } from 'vitest';
import {
  DURATIONS,
  diagramNodes,
  easeCount,
  filterTiles,
  flowLabel,
  formatRemaining,
  heatLevel,
  lane,
  matchPreset,
  modeInfo,
  parseTarget,
  putBody,
  remainingFraction,
  ruleLabel,
  summarizeFlows,
  tallies,
  toggleRule,
} from '../networkAccess.js';

const r = (cidr, proto = 'tcp', port) => ({ cidr, proto, port });

describe('rules', () => {
  it('labels rules for chips', () => {
    expect(ruleLabel(r('10.0.0.0/8', 'tcp', 5151))).toBe('10.0.0.0/8 · tcp/5151');
    expect(ruleLabel(r('192.168.0.0/16', 'any'))).toBe('192.168.0.0/16 · all ports');
    expect(ruleLabel(null)).toBe('');
  });

  it('toggles a rule on and off without duplicating', () => {
    const a = r('1.2.3.4/32', 'tcp', 443);
    const once = toggleRule([], a);
    expect(once).toHaveLength(1);
    expect(toggleRule(once, { ...a })).toHaveLength(0);
    expect(toggleRule(once, r('1.2.3.4/32', 'tcp', 80))).toHaveLength(2);
  });

  it('builds PUT bodies: a preset alone, or a full replacement', () => {
    expect(putBody({ preset: 'web' })).toEqual({ preset: 'web' });
    expect(putBody({ preset: 'web', expiresIn: '1h' })).toEqual({ preset: 'web', expires_in: '1h' });
    const full = putBody({ mode: 'allowlist', rules: [r('10.0.0.0/8', 'tcp', 5151), r('1.1.1.1/32', 'any')] });
    expect(full).toEqual({ mode: 'allowlist', rules: [{ cidr: '10.0.0.0/8', proto: 'tcp', port: 5151 }, { cidr: '1.1.1.1/32', proto: 'any' }] });
    // A preset with extra rules sends both.
    expect(putBody({ preset: 'platform', rules: [r('9.9.9.9/32', 'tcp', 443)] }).rules).toHaveLength(1);
  });
});

describe('preset matching', () => {
  const presets = [
    { id: 'platform', mode: 'allowlist', rules: [r('10.0.0.0/16', 'tcp', 5151)] },
    { id: 'learn', mode: 'audit', rules: [] },
  ];
  it('matches by mode and rule set, ignoring order and notes', () => {
    expect(matchPreset({ mode: 'audit', rules: [] }, presets).id).toBe('learn');
    expect(matchPreset({ mode: 'allowlist', rules: [{ ...r('10.0.0.0/16', 'tcp', 5151), note: 'x' }] }, presets).id).toBe('platform');
    expect(matchPreset({ mode: 'allowlist', rules: [r('10.0.0.0/16', 'tcp', 5151), r('1.1.1.1/32', 'tcp', 443)] }, presets)).toBeNull();
    expect(matchPreset({ mode: 'open', rules: [] }, presets)).toBeNull();
    expect(matchPreset(null, presets)).toBeNull();
  });
});

describe('explain input', () => {
  it('parses host, port and protocol with sensible defaults', () => {
    expect(parseTarget('archive.ubuntu.com')).toEqual({ host: 'archive.ubuntu.com', port: 443, proto: 'tcp' });
    expect(parseTarget('10.0.0.5:5151')).toEqual({ host: '10.0.0.5', port: 5151, proto: 'tcp' });
    expect(parseTarget('dns.google:53/udp')).toEqual({ host: 'dns.google', port: 53, proto: 'udp' });
    expect(parseTarget('a b')).toBeNull();
    expect(parseTarget('x:0')).toBeNull();
    expect(parseTarget('x:70000')).toBeNull();
    expect(parseTarget('')).toBeNull();
  });
});

describe('observed flows', () => {
  const flows = [
    { dst: '1.1.1.1', port: 443, proto: 'tcp', packets: 5, audit: false },
    { dst: '2.2.2.2', port: 80, proto: 'tcp', packets: 3, audit: true },
    { dst: '3.3.3.3', port: 53, proto: 'udp', packets: 2 },
  ];
  it('splits refused from would-be-refused and totals packets', () => {
    const s = summarizeFlows(flows);
    expect(s.blocked).toHaveLength(2);
    expect(s.wouldBlock).toHaveLength(1);
    expect(s.blockedPackets).toBe(7);
    expect(summarizeFlows(null).blocked).toEqual([]);
  });
  it('labels a flow', () => {
    expect(flowLabel(flows[0])).toBe('1.1.1.1:443/tcp');
  });
  it('draws allowed rules first and caps the diagram', () => {
    const policy = { mode: 'allowlist', rules: [r('10.0.0.0/8', 'tcp', 5151), r('1.2.3.0/24', 'tcp', 80), r('4.4.4.0/24', 'tcp', 80), r('5.5.5.0/24', 'tcp', 80), r('6.6.6.0/24', 'tcp', 80)] };
    const nodes = diagramNodes(policy, flows, 6);
    expect(nodes).toHaveLength(6);
    expect(nodes[0].kind).toBe('allowed');
    expect(nodes.some((n) => n.kind === 'blocked')).toBe(true);
    expect(diagramNodes({ mode: 'open', rules: [] }, [])[0].label).toBe('anywhere');
    expect(diagramNodes({ mode: 'loopback', rules: [] }, [])).toEqual([]);
  });
  it('spreads lanes evenly', () => {
    expect(lane(0, 1)).toBe(120);
    expect(lane(0, 3)).toBe(36);
    expect(lane(2, 3)).toBe(204);
  });
});

describe('timers', () => {
  it('formats remaining time', () => {
    expect(formatRemaining(59 * 60_000 + 12_000)).toBe('59m 12s');
    expect(formatRemaining(2 * 3600_000 + 5 * 60_000)).toBe('2h 05m');
    expect(formatRemaining(0)).toBe('ending…');
    expect(formatRemaining(NaN)).toBe('ending…');
  });
  it('computes the ring fraction and clamps it', () => {
    const start = '2030-01-01T00:00:00Z';
    const end = '2030-01-01T01:00:00Z';
    expect(remainingFraction(start, end, Date.parse('2030-01-01T00:30:00Z'))).toBeCloseTo(0.5);
    expect(remainingFraction(start, end, Date.parse('2029-12-31T00:00:00Z'))).toBe(1);
    expect(remainingFraction(start, end, Date.parse('2030-02-01T00:00:00Z'))).toBe(0);
    expect(remainingFraction('x', end)).toBe(0);
  });
  it('offers sensible durations', () => {
    expect(DURATIONS.map((d) => d.id)).toEqual(['30m', '1h', '4h', '1d']);
  });
});

describe('fleet', () => {
  const items = [
    { namespace: 'a', name: 'web', mode: 'allowlist', running: true, blocked: 4, agent_connected: true },
    { namespace: 'a', name: 'db', mode: 'loopback', running: true, blocked: 0, agent_connected: false },
    { namespace: 'b', name: 'lab', mode: 'inherit', running: false, blocked: null, agent_connected: null },
  ];
  it('filters by mode, problem chip and query', () => {
    expect(filterTiles(items, { mode: 'allowlist' })).toHaveLength(1);
    expect(filterTiles(items, { only: 'blocked' }).map((i) => i.name)).toEqual(['web']);
    expect(filterTiles(items, { only: 'noagent' }).map((i) => i.name)).toEqual(['db']);
    expect(filterTiles(items, { only: 'running' })).toHaveLength(2);
    expect(filterTiles(items, { query: 'B/LAB' })).toHaveLength(1);
    expect(filterTiles(null)).toEqual([]);
  });
  it('tallies', () => {
    const t = tallies(items);
    expect(t).toMatchObject({ total: 3, blocked: 1, noAgent: 1, running: 2 });
    expect(t.byMode).toEqual({ allowlist: 1, loopback: 1, inherit: 1 });
  });
  it('maps blocked counts to heat', () => {
    expect([0, 1, 2, 3, 9, 10, 50].map(heatLevel)).toEqual([0, 1, 1, 2, 2, 3, 3]);
    expect(heatLevel(null)).toBe(0);
  });
  it('knows every mode and falls back to the node default', () => {
    expect(modeInfo('audit').label).toBe('Learning');
    expect(modeInfo('weird').id).toBe('inherit');
  });
});

describe('count-up', () => {
  it('eases monotonically from start to target', () => {
    expect(easeCount(0, 100, 0)).toBe(0);
    expect(easeCount(0, 100, 1)).toBe(100);
    expect(easeCount(0, 100, 2)).toBe(100);
    const mid = easeCount(0, 100, 0.3);
    expect(mid).toBeGreaterThan(30);
    expect(mid).toBeLessThan(100);
    expect(easeCount(10, 0, 1)).toBe(0);
  });
});
