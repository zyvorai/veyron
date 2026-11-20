// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
(() => {
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Scroll reveal
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.classList.add('in');
        io.unobserve(e.target);
      }
    },
    { threshold: 0.18 }
  );
  document.querySelectorAll('.reveal, .stack').forEach((el) => io.observe(el));
  document.querySelectorAll('.stack > li').forEach((li, i) => li.style.setProperty('--i', i));

  // Count-up stats
  const countUp = (el) => {
    const end = parseFloat(el.dataset.count);
    const dec = Number(el.dataset.dec || 0);
    const suffix = el.dataset.suffix || '';
    if (reduced || end === 0) {
      el.textContent = end.toFixed(dec) + suffix;
      return;
    }
    const t0 = performance.now();
    const dur = 1400;
    const tick = (t) => {
      const p = Math.min(1, (t - t0) / dur);
      const v = end * (1 - Math.pow(1 - p, 3));
      el.textContent = v.toFixed(dec) + suffix;
      if (p < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  };
  const statIo = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      countUp(e.target);
      statIo.unobserve(e.target);
    }
  });
  document.querySelectorAll('[data-count]').forEach((el) => statIo.observe(el));

  // Stack detail
  const sdK = document.getElementById('sd-k');
  const sdD = document.getElementById('sd-d');
  const layers = [...document.querySelectorAll('.stack > li')];
  const pick = (li) => {
    layers.forEach((l) => l.classList.toggle('on', l === li));
    const name = li.querySelector('.ln')?.textContent || 'Hypervisors';
    sdK.textContent = name;
    sdD.innerHTML = li.dataset.d;
  };
  layers.forEach((li) => {
    li.tabIndex = 0;
    li.addEventListener('mouseenter', () => pick(li));
    li.addEventListener('focus', () => pick(li));
  });
  if (layers[0]) pick(layers[0]);

  // Race: p50 seconds from the 2026-10-04 benchmark (create to Running, create to SSH banner)
  const RUNS = {
    1: { kairon: { run: 10.3, ssh: 23.7 }, kubevirt: { run: 23.4, ssh: 67.6 } },
    5: { kairon: { run: 3.9, ssh: 24.8 }, kubevirt: { run: 78.6, ssh: 184.7 } },
  };
  const SPEED = 10;
  let n = 1;
  let raf = 0;
  const clock = document.getElementById('race-clock');
  const lanes = {
    kairon: document.querySelector('[data-lane="kairon"]'),
    kubevirt: document.querySelector('[data-lane="kubevirt"]'),
  };
  const layout = () => {
    const r = RUNS[n];
    const max = Math.max(r.kairon.ssh, r.kubevirt.ssh);
    for (const [k, lane] of Object.entries(lanes)) {
      lane.querySelector('.mark.run').style.setProperty('--x', `${(r[k].run / max) * 100}%`);
      lane.querySelector('.mark.ssh').style.setProperty('--x', `calc(${(r[k].ssh / max) * 100}% - 4px)`);
    }
    return max;
  };
  const paint = (t, max) => {
    const r = RUNS[n];
    clock.textContent = t.toFixed(1);
    for (const [k, lane] of Object.entries(lanes)) {
      const s = r[k];
      lane.querySelector('.fill').style.width = `${(Math.min(t, s.ssh) / max) * 100}%`;
      lane.querySelector('.mark.run').classList.toggle('on', t >= s.run);
      lane.querySelector('.mark.ssh').classList.toggle('on', t >= s.ssh);
      const state = lane.querySelector('.lane-s');
      const tl = lane.querySelector('.lane-t');
      if (t >= s.ssh) {
        state.innerHTML = `<b>SSH ready</b> at ${s.ssh} s${n > 1 ? ' (p50 of 5)' : ''}`;
        tl.textContent = `${s.ssh} s`;
      } else if (t >= s.run) {
        state.innerHTML = `<b>Running</b> at ${s.run} s, guest booting&hellip;`;
        tl.textContent = `${t.toFixed(1)} s`;
      } else if (t > 0) {
        state.textContent = k === 'kubevirt' ? 'Scheduling pod, starting virt-launcher\u2026' : 'Placing Machine, starting VMM\u2026';
        tl.textContent = `${t.toFixed(1)} s`;
      } else {
        state.textContent = 'Waiting';
        tl.textContent = '\u2014';
      }
    }
  };
  const reset = () => {
    cancelAnimationFrame(raf);
    paint(0, layout());
  };
  const go = () => {
    cancelAnimationFrame(raf);
    const max = layout();
    if (reduced) {
      paint(max, max);
      return;
    }
    const t0 = performance.now();
    const step = (now) => {
      const t = Math.min(max, ((now - t0) / 1000) * SPEED);
      paint(t, max);
      if (t < max) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
  };
  document.getElementById('race-go').addEventListener('click', go);
  document.querySelectorAll('.race .seg button').forEach((b) =>
    b.addEventListener('click', () => {
      document.querySelectorAll('.race .seg button').forEach((x) => x.setAttribute('aria-selected', String(x === b)));
      n = Number(b.dataset.n);
      reset();
    })
  );
  reset();
  const raceIo = new IntersectionObserver((entries) => {
    if (entries.some((e) => e.isIntersecting)) {
      go();
      raceIo.disconnect();
    }
  }, { threshold: 0.5 });
  raceIo.observe(document.querySelector('.race'));

  // Gallery tabs
  const shot = document.getElementById('shot');
  document.querySelectorAll('.gallery .seg button').forEach((b) =>
    b.addEventListener('click', () => {
      document.querySelectorAll('.gallery .seg button').forEach((x) => x.setAttribute('aria-selected', String(x === b)));
      shot.style.opacity = '0';
      setTimeout(() => {
        shot.src = `assets/${b.dataset.shot}`;
        shot.alt = `Veyron console: ${b.textContent}`;
        shot.onload = () => (shot.style.opacity = '1');
      }, reduced ? 0 : 200);
    })
  );

  // Copy quickstart
  const copy = document.querySelector('.copy');
  copy?.addEventListener('click', async () => {
    const text = document.querySelector('.code code').innerText;
    try {
      await navigator.clipboard.writeText(text);
      copy.textContent = 'Copied';
    } catch {
      copy.textContent = 'Select and copy';
    }
    setTimeout(() => (copy.textContent = 'Copy'), 1600);
  });
})();
