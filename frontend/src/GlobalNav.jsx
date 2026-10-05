import { useEffect, useRef, useState } from 'react';
import { Search, Bell, Sun, Moon, Plus, Menu, X, LogOut } from 'lucide-react';
import { NAV, RES_META, pageLabel } from './resources.js';
import zyvorLogomark from './assets/zyvor-logomark.svg';

const OPEN_DELAY_MS = 120;
const CLOSE_DELAY_MS = 450;

export function ZyvorMark({ size = 22 }) {
  return <img src={zyvorLogomark} width={size} height={size} alt="" aria-hidden className="gn-mark" />;
}

/** apple.com-style global navigation: frosted bar, hover mega-menus, mobile sheet. */
export function GlobalNav({
  page, go, data, theme, onToggleTheme, onSearch, onBell, alerts, healthOk, loading, canCreate, onCreate, user, onLogout,
}) {
  const [openGroup, setOpenGroup] = useState(null);
  const [mobile, setMobile] = useState(false);
  const openTimer = useRef(null);
  const closeTimer = useRef(null);
  const navRef = useRef(null);
  const triggerRefs = useRef({});

  const clearTimers = () => {
    clearTimeout(openTimer.current);
    clearTimeout(closeTimer.current);
  };
  const scheduleOpen = (g) => {
    clearTimers();
    openTimer.current = setTimeout(() => setOpenGroup(g), OPEN_DELAY_MS);
  };
  const scheduleClose = () => {
    clearTimers();
    closeTimer.current = setTimeout(() => setOpenGroup(null), CLOSE_DELAY_MS);
  };
  useEffect(() => () => clearTimers(), []);

  useEffect(() => {
    if (!openGroup && !mobile) return undefined;
    const onKey = (e) => {
      if (e.key !== 'Escape') return;
      const g = openGroup;
      setOpenGroup(null);
      setMobile(false);
      if (g) triggerRefs.current[g]?.focus();
    };
    const onDown = (e) => {
      if (navRef.current && !navRef.current.contains(e.target)) setOpenGroup(null);
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onDown);
    };
  }, [openGroup, mobile]);

  const pick = (id) => {
    clearTimers();
    setOpenGroup(null);
    setMobile(false);
    go(id);
  };

  const initial = (user?.display_name || user?.username || 'A').charAt(0).toUpperCase();

  return (
    <nav className={`gn${openGroup ? ' gn--open' : ''}`} aria-label="Global" ref={navRef}>
      <div className="gn-inner">
        <button type="button" className="gn-brand" onClick={() => pick('mission')} aria-label="Veyron home">
          <ZyvorMark />
          <span>Veyron</span>
        </button>

        <div className="gn-links">
          {NAV.map((g) =>
            g.page ? (
              <button
                key={g.g}
                type="button"
                className={page === g.page ? 'active' : ''}
                aria-current={page === g.page ? 'page' : undefined}
                onClick={() => pick(g.page)}
              >
                {g.g}
              </button>
            ) : (
              <div key={g.g} className="gn-group" onMouseEnter={() => scheduleOpen(g.g)} onMouseLeave={scheduleClose}>
                <button
                  type="button"
                  ref={(el) => {
                    triggerRefs.current[g.g] = el;
                  }}
                  className={g.items.some(([id]) => id === page) ? 'active' : ''}
                  aria-haspopup="true"
                  aria-expanded={openGroup === g.g}
                  onClick={() => setOpenGroup((cur) => (cur === g.g ? null : g.g))}
                >
                  {g.g}
                </button>
              </div>
            ),
          )}
        </div>

        <div className="gn-actions">
          <button type="button" className="gn-search" onClick={onSearch} title="Search (⌘K)" aria-label="Search">
            <Search size={14} />
            <span>Search</span>
            <kbd>⌘K</kbd>
          </button>
          <span className={`gn-live${!healthOk || alerts ? ' warn' : ''}`} title={healthOk ? 'API healthy' : 'API degraded'}>
            <i />
            {loading ? 'Syncing' : healthOk ? 'Live' : 'Degraded'}
          </span>
          <button
            type="button"
            className="gn-icon"
            onClick={(e) => {
              e.stopPropagation();
              onBell();
            }}
            title="Notifications"
            aria-label="Notifications"
          >
            <Bell size={15} />
            {alerts > 0 && <span className="gn-badge">{alerts > 99 ? '99+' : alerts}</span>}
          </button>
          <button
            type="button"
            className="gn-icon"
            onClick={onToggleTheme}
            aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
          >
            {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
          </button>
          {canCreate && (
            <button type="button" className="gn-new" onClick={onCreate}>
              <Plus size={14} />
              New
            </button>
          )}
          <button
            type="button"
            className="gn-avatar"
            onClick={onLogout}
            title={`Sign out${user?.username ? ` (${user.username})` : ''}`}
          >
            {initial}
          </button>
          <button
            type="button"
            className="gn-icon gn-burger"
            onClick={() => setMobile((m) => !m)}
            aria-label="Menu"
            aria-expanded={mobile}
          >
            {mobile ? <X size={17} /> : <Menu size={17} />}
          </button>
        </div>
      </div>

      {NAV.filter((g) => !g.page).map((g) => (
        <div
          key={g.g}
          className={`gn-mega${openGroup === g.g ? ' open' : ''}`}
          data-tone={g.tone}
          role="region"
          aria-label={g.g}
          onMouseEnter={() => scheduleOpen(g.g)}
          onMouseLeave={scheduleClose}
        >
          <div className="gn-mega-inner">
            <p className="gn-mega-eyebrow">{g.g}</p>
            <div className="gn-mega-grid">
              {g.items.map(([id, , blurb], i) => {
                const I = RES_META[id]?.I;
                const n = data[id]?.rows?.length;
                return (
                  <button
                    key={id}
                    type="button"
                    style={{ '--i': i }}
                    className={page === id ? 'active' : ''}
                    aria-current={page === id ? 'page' : undefined}
                    onClick={() => pick(id)}
                  >
                    <span className="gn-mega-label">
                      {I && <I size={16} strokeWidth={1.9} />}
                      {pageLabel(id)}
                      {n > 0 && <small>{n}</small>}
                    </span>
                    <span className="gn-mega-blurb">{blurb}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      ))}
      <div className={`gn-scrim${openGroup ? ' on' : ''}`} aria-hidden />

      {mobile && (
        <div className="gn-sheet">
          {NAV.map((g) => (
            <section key={g.g}>
              <p className="gn-mega-eyebrow">{g.g}</p>
              {g.items.map(([id]) => (
                <button key={id} type="button" className={page === id ? 'active' : ''} onClick={() => pick(id)}>
                  {pageLabel(id)}
                </button>
              ))}
            </section>
          ))}
          <section>
            <button type="button" onClick={onLogout}>
              <LogOut size={16} /> Sign out
            </button>
          </section>
        </div>
      )}
    </nav>
  );
}
