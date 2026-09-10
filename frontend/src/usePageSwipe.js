import { useRef, useCallback, useEffect } from 'react';
import { PAGE_ORDER } from './resources.js';

/**
 * Horizontal page swipe with 1:1 finger tracking on the main pane.
 * Prev/next pages peek under the drag; release snaps by distance or velocity.
 */
export function usePageSwipe({ page, onPage, enabled = true, disabled = false }) {
  const trackRef = useRef(null);
  const drag = useRef(null);

  const applyX = useCallback((x, anim) => {
    const el = trackRef.current;
    if (!el) return;
    el.style.transition = anim ? 'transform 0.28s cubic-bezier(.32,.72,0,1)' : 'none';
    el.style.transform = `translate3d(${x}px,0,0)`;
  }, []);

  const reset = useCallback(() => {
    applyX(0, true);
    const el = trackRef.current;
    if (!el) return;
    const done = () => {
      el.style.transition = 'none';
      el.removeEventListener('transitionend', done);
    };
    el.addEventListener('transitionend', done);
  }, [applyX]);

  useEffect(() => {
    applyX(0, false);
  }, [page, applyX]);

  useEffect(() => {
    if (!enabled || disabled) return undefined;
    const root = trackRef.current?.parentElement;
    if (!root) return undefined;

    const onDown = (e) => {
      if (e.button != null && e.button !== 0) return;
      if (e.target.closest?.('input, textarea, select, .menu, .scrim, .insp, .bulk, .source')) return;
      // Don't steal vertical scroll / table text selection until horizontal intent is clear.
      const pt = e.touches ? e.touches[0] : e;
      drag.current = {
        id: e.pointerId,
        x0: pt.clientX,
        y0: pt.clientY,
        t0: performance.now(),
        dx: 0,
        locked: null, // null | 'h' | 'v'
        width: root.clientWidth || 1,
      };
      // Pointer capture is deferred to onMove, once horizontal drag intent is
      // confirmed — capturing here on every pointerdown redirects the plain
      // click/tap that follows (mouseup + click are retargeted to whichever
      // element holds pointer capture) away from the row/button actually
      // pressed, to this swipe root, silently swallowing ordinary clicks.
    };

    const onMove = (e) => {
      const d = drag.current;
      if (!d || (e.pointerId != null && e.pointerId !== d.id)) return;
      const pt = e.touches ? e.touches[0] : e;
      const dx = pt.clientX - d.x0;
      const dy = pt.clientY - d.y0;
      if (d.locked == null) {
        if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return;
        d.locked = Math.abs(dx) > Math.abs(dy) * 1.15 ? 'h' : 'v';
        if (d.locked === 'v') return;
        try {
          root.setPointerCapture?.(d.id);
        } catch {
          /* ignore */
        }
      }
      if (d.locked !== 'h') return;
      e.preventDefault();
      d.dx = dx;
      const idx = PAGE_ORDER.indexOf(page);
      const atStart = idx <= 0;
      const atEnd = idx >= PAGE_ORDER.length - 1;
      let x = dx;
      // Rubber-band at ends (still ~1:1 until edge, then damp).
      if ((atStart && dx > 0) || (atEnd && dx < 0)) x = dx * 0.28;
      applyX(x, false);
    };

    const onUp = (e) => {
      const d = drag.current;
      if (!d || (e.pointerId != null && e.pointerId !== d.id)) return;
      drag.current = null;
      if (d.locked !== 'h') {
        applyX(0, false);
        return;
      }
      const dt = Math.max(1, performance.now() - d.t0);
      const v = d.dx / dt; // px/ms
      const idx = PAGE_ORDER.indexOf(page);
      const thresh = Math.min(96, d.width * 0.22);
      let next = page;
      if ((d.dx < -thresh || v < -0.55) && idx < PAGE_ORDER.length - 1) next = PAGE_ORDER[idx + 1];
      else if ((d.dx > thresh || v > 0.55) && idx > 0) next = PAGE_ORDER[idx - 1];

      if (next !== page) {
        applyX(d.dx < 0 ? -d.width : d.width, true);
        window.setTimeout(() => {
          onPage(next);
          applyX(0, false);
        }, 200);
      } else {
        reset();
      }
    };

    root.addEventListener('pointerdown', onDown);
    root.addEventListener('pointermove', onMove, { passive: false });
    root.addEventListener('pointerup', onUp);
    root.addEventListener('pointercancel', onUp);
    return () => {
      root.removeEventListener('pointerdown', onDown);
      root.removeEventListener('pointermove', onMove);
      root.removeEventListener('pointerup', onUp);
      root.removeEventListener('pointercancel', onUp);
    };
  }, [page, onPage, enabled, disabled, applyX, reset]);

  return { trackRef, pageIndex: PAGE_ORDER.indexOf(page), pageCount: PAGE_ORDER.length };
}
