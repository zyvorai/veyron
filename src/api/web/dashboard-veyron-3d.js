/**
 * Veyron WebGL bootstrap helper for the IronWolf signature 3D surfaces
 * (Reactor Core, Topology, Fleet Constellation, Signal Desk hero orb).
 *
 * Lazy-loaded via dynamic import() only on pages that actually render a
 * WebGL scene — never bundled into the main dashboard payload. Pairs with
 * /assets/three.module.min.js (vendored three.js r160, ESM build).
 *
 * Every caller MUST call the returned dispose() when the page is left — it
 * frees every geometry/material/texture in the scene, which is real GPU
 * memory that otherwise accumulates on every revisit.
 *
 * Context reuse: each signature page has one static, never-removed <canvas>,
 * and createScene() runs again on every visit. dispose() never force-loses
 * the WebGL context, and createScene() stashes the live context on the
 * canvas (`canvas._veyronGL`) and passes it back into the next
 * THREE.WebGLRenderer via the `context` constructor option instead of asking
 * the canvas for a new one. This is load-bearing, not defensive: verified
 * empirically that letting WebGLRenderer request a fresh context on every
 * revisit exhausts the browser's WebGL context budget after roughly a dozen
 * navigations, after which getContext() returns null forever on that page.
 */
import * as THREE from '/assets/three.module.min.js';

export { THREE };

function readToken(name, fallback) {
  var v = (getComputedStyle(document.documentElement).getPropertyValue(name) || '').trim();
  return v || fallback;
}

/** IronWolf design tokens, read live so scenes re-theme with tahoe/light without a reload. */
export function veyronTokenColors() {
  return {
    plasma: readToken('--plasma', '#64a0dc'),
    plasmaDeep: readToken('--plasma-deep', '#3b82b8'),
    nominal: readToken('--nominal', '#1D9E75'),
    caution: readToken('--caution', '#EF9F27'),
    critical: readToken('--critical', '#E24B4A'),
    inert: readToken('--inert', '#5F5E5A'),
    ink: readToken('--ink', '#e8e9ec'),
    void: readToken('--void', '#080c16'),
  };
}

function disposeObject3D(root) {
  root.traverse(function (child) {
    if (child.geometry) child.geometry.dispose();
    if (child.material) {
      var mats = Array.isArray(child.material) ? child.material : [child.material];
      mats.forEach(function (m) {
        if (m.map) m.map.dispose();
        m.dispose();
      });
    }
  });
}

/**
 * Bootstrap a WebGL scene into `canvas`, sized to its parent container.
 * Returns { THREE, scene, camera, renderer, colors, reduceMotion, onFrame, dispose }.
 */
export function createScene(canvas, opts) {
  opts = opts || {};
  var colors = veyronTokenColors();
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  var rendererParams = {
    canvas: canvas,
    antialias: true,
    alpha: true,
    powerPreference: 'high-performance',
  };
  // Reuse a WebGL context already created on this canvas, if one exists and
  // is still live, instead of letting WebGLRenderer ask the canvas for a new
  // one. A <canvas> can only ever hold a single context, and this SPA calls
  // createScene() again every time a signature 3D page is revisited — without
  // explicit reuse (verified empirically: repeated create/dispose cycles on
  // the same canvas start returning a null context after roughly a dozen
  // revisits), that exhausts the browser's WebGL context budget even though
  // dispose() below never force-loses the context itself.
  if (canvas._veyronGL && !canvas._veyronGL.isContextLost()) {
    rendererParams.context = canvas._veyronGL;
  }
  var renderer = new THREE.WebGLRenderer(rendererParams);
  canvas._veyronGL = renderer.getContext();
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));

  var scene = new THREE.Scene();
  var camera = new THREE.PerspectiveCamera(opts.fov || 50, 1, 0.1, 2000);
  camera.position.set(0, 0, opts.cameraDistance || 10);

  var container = canvas.parentElement || canvas;
  var disposed = false;
  var frameHandle = null;
  var onFrameCb = null;

  function resize() {
    var w = container.clientWidth || 1;
    var h = container.clientHeight || 1;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
  resize();

  var ro = null;
  if (window.ResizeObserver) {
    ro = new ResizeObserver(resize);
    ro.observe(container);
  } else {
    window.addEventListener('resize', resize);
  }

  var clock = new THREE.Clock();
  function tick() {
    if (disposed) return;
    frameHandle = requestAnimationFrame(tick);
    var dt = clock.getDelta();
    if (onFrameCb) onFrameCb(dt, reduceMotion);
    renderer.render(scene, camera);
  }
  frameHandle = requestAnimationFrame(tick);

  return {
    THREE: THREE,
    scene: scene,
    camera: camera,
    renderer: renderer,
    colors: colors,
    reduceMotion: reduceMotion,
    onFrame: function (cb) { onFrameCb = cb; },
    dispose: function () {
      if (disposed) return;
      disposed = true;
      if (frameHandle) cancelAnimationFrame(frameHandle);
      if (ro) ro.disconnect(); else window.removeEventListener('resize', resize);
      disposeObject3D(scene);
      renderer.dispose();
      // Deliberately NOT calling renderer.forceContextLoss(): these are static,
      // reused <canvas> elements (one per signature page, never removed from the
      // DOM) — a canvas can only ever hold one WebGL context, and force-losing it
      // makes getContext() return null forever after, breaking every future visit
      // to the page. renderer.dispose() already frees the GPU-heavy resources
      // (geometries/materials/textures via disposeObject3D above, plus internal
      // programs/render lists); leaving the context itself alive and reusable by
      // the next createScene() call on the same canvas is correct here.
    },
  };
}

/**
 * Attach drag-to-look-around + optional idle auto-rotate to `camera`, orbiting
 * around `target` at a fixed radius (spherical coordinates). No external
 * OrbitControls dependency — this is deliberately minimal so we don't need a
 * second vendored addon file. Call the returned tick(dt, reduceMotion) once
 * per frame (wire it into createScene()'s onFrame), and dispose() on teardown.
 */
export function attachOrbitDrag(camera, target, canvas, opts) {
  opts = opts || {};
  var radius = opts.radius != null ? opts.radius : camera.position.distanceTo(target);
  var theta = opts.theta != null ? opts.theta : Math.PI / 2;
  var phi = opts.phi != null ? opts.phi : Math.PI / 3;
  var minPhi = opts.minPhi != null ? opts.minPhi : 0.15;
  var maxPhi = opts.maxPhi != null ? opts.maxPhi : Math.PI - 0.15;
  var autoRotateSpeed = opts.autoRotateSpeed || 0;
  var dragging = false;
  var lastX = 0;
  var lastY = 0;
  var moved = false;

  function apply() {
    var sinPhi = Math.sin(phi);
    camera.position.set(
      target.x + radius * sinPhi * Math.cos(theta),
      target.y + radius * Math.cos(phi),
      target.z + radius * sinPhi * Math.sin(theta)
    );
    camera.lookAt(target);
  }
  apply();

  function pointOf(e) {
    if (e.touches && e.touches[0]) return { x: e.touches[0].clientX, y: e.touches[0].clientY };
    return { x: e.clientX, y: e.clientY };
  }
  function onDown(e) {
    dragging = true;
    moved = false;
    var p = pointOf(e);
    lastX = p.x; lastY = p.y;
    canvas.style.cursor = 'grabbing';
  }
  function onMove(e) {
    if (!dragging) return;
    var p = pointOf(e);
    var dx = p.x - lastX, dy = p.y - lastY;
    if (Math.abs(dx) > 2 || Math.abs(dy) > 2) moved = true;
    lastX = p.x; lastY = p.y;
    theta -= dx * 0.006;
    phi = Math.max(minPhi, Math.min(maxPhi, phi - dy * 0.006));
    apply();
  }
  function onUp() {
    dragging = false;
    canvas.style.cursor = 'grab';
  }

  canvas.style.cursor = 'grab';
  canvas.addEventListener('pointerdown', onDown);
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);

  return {
    tick: function (dt, reduceMotion) {
      if (!dragging && autoRotateSpeed && !reduceMotion) {
        theta += autoRotateSpeed * dt;
        apply();
      }
    },
    wasDragged: function () { return moved; },
    setAutoRotate: function (v) { autoRotateSpeed = v; },
    dispose: function () {
      canvas.removeEventListener('pointerdown', onDown);
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    },
  };
}
