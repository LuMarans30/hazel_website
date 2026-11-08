(() => {
  'use strict';

  const CFG = __CONFIG__;
  const STORAGE_KEY = 'leaves-t';

  const UNIFORMS = __UNIFORMS__;

  const VERT = __VERT__;
  const FRAG = __FRAG__;

  const root = document.documentElement;
  const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const darkScheme = matchMedia('(prefers-color-scheme: dark)');

  let canvas = null;
  let gl = null;
  const loc = {};
  let palTop = new Float32Array(12);
  let palBot = new Float32Array(12);
  let palAccent = new Float32Array(3);
  let cssW = 0;
  let cssH = 0;
  let scale = 1;
  let unit = 1;
  let raf = 0;
  let lastFrame = 0;
  let lightbox = null;

  let t = 0;
  try {
    t = parseFloat(sessionStorage.getItem(STORAGE_KEY)) || 0;
  } catch (_) {}

  const clamp = (x, a, b) => Math.min(b, Math.max(a, x));

  function compile(type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      console.warn('leaves: shader compile failed\n' + gl.getShaderInfoLog(s));
      return null;
    }
    return s;
  }

  function setup() {
    gl = canvas.getContext('webgl2', {
      alpha: true,
      premultipliedAlpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: 'low-power',
    });
    if (!gl) return false;

    const vs = compile(gl.VERTEX_SHADER, VERT);
    const fs = compile(gl.FRAGMENT_SHADER, FRAG);
    if (!vs || !fs) return false;

    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.warn('leaves: program link failed\n' + gl.getProgramInfoLog(prog));
      return false;
    }
    gl.useProgram(prog);
    for (const name of UNIFORMS) loc[name] = gl.getUniformLocation(prog, name);
    gl.clearColor(0, 0, 0, 0);
    resize();
    return true;
  }

  function parseHex(raw) {
    const hex = raw.trim();
    if (!/^#[0-9a-f]{6}$/i.test(hex)) return null;
    return [0, 1, 2].map((j) => parseInt(hex.slice(1 + j * 2, 3 + j * 2), 16) / 255);
  }

  function refreshPalette() {
    const style = getComputedStyle(root);
    const top = new Float32Array(12);
    const bot = new Float32Array(12);
    for (let i = 0; i < 4; i++) {
      const a = parseHex(style.getPropertyValue(`--leaf-top-${i + 1}`));
      const b = parseHex(style.getPropertyValue(`--leaf-bot-${i + 1}`));
      if (!a || !b) return false;
      top.set(a, i * 3);
      bot.set(b, i * 3);
    }
    const accent = parseHex(style.getPropertyValue('--accent'));
    if (!accent) return false;
    palTop = top;
    palBot = bot;
    palAccent = Float32Array.from(accent);
    return true;
  }

  function resize() {
    if (!gl) return;
    cssW = canvas.clientWidth;
    cssH = canvas.clientHeight;
    if (!cssW || !cssH) return;
    const dpr = Math.min(window.devicePixelRatio || 1, CFG.maxDpr);
    canvas.width = Math.max(1, Math.round(cssW * dpr));
    canvas.height = Math.max(1, Math.round(cssH * dpr));
    scale = canvas.width / cssW;
    unit = clamp(Math.sqrt(cssW * cssH) / 1100, 0.7, 1.4);
    gl.viewport(0, 0, canvas.width, canvas.height);
    draw();
  }

  function draw() {
    if (!gl || gl.isContextLost() || !cssW) return;
    gl.uniform2f(loc.uSize, cssW, cssH);
    gl.uniform1f(loc.uScale, scale);
    gl.uniform1f(loc.uTime, t);
    gl.uniform1f(loc.uDark, darkScheme.matches ? 1 : 0);
    gl.uniform1f(loc.uUnit, unit);
    gl.uniform3fv(loc.uTop, palTop);
    gl.uniform3fv(loc.uBot, palBot);
    gl.uniform3fv(loc.uAccent, palAccent);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  function paused() {
    if (document.hidden) return true;
    if (!lightbox) lightbox = document.getElementById('lightbox');
    return !!(lightbox && lightbox.open);
  }

  function frame(now) {
    raf = requestAnimationFrame(frame);
    const minDt = CFG.maxFps ? 1000 / CFG.maxFps - 1 : 0;
    const elapsed = now - lastFrame;
    if (elapsed < minDt) return;
    lastFrame = now;
    if (paused()) return;

    const dt = Math.min(elapsed / 1000, 0.1);
    t += dt;
    draw();
  }

  function start() {
    if (raf || reduceMotion.matches) return;
    lastFrame = performance.now();
    raf = requestAnimationFrame(frame);
  }

  function stop() {
    cancelAnimationFrame(raf);
    raf = 0;
  }

  function init() {
    if (!refreshPalette()) {
      console.warn('leaves: --leaf-* / --accent colors not found; is the theme <style> in <head>?');
      return;
    }

    canvas = document.createElement('canvas');
    canvas.className = 'leaves';
    canvas.setAttribute('aria-hidden', 'true');

    document.body.prepend(canvas);
    if (!setup()) {
      canvas.remove();
      return;
    }

    new ResizeObserver(resize).observe(canvas);
    darkScheme.addEventListener('change', () => {
      refreshPalette();
      draw();
    });
    reduceMotion.addEventListener('change', () => (reduceMotion.matches ? stop() : start()));
    document.addEventListener('visibilitychange', () => (document.hidden ? stop() : start()));
    addEventListener('pagehide', () => {
      try {
        sessionStorage.setItem(STORAGE_KEY, String(t));
      } catch (_) {}
    });

    canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      stop();
    });
    canvas.addEventListener('webglcontextrestored', () => {
      if (setup()) {
        start();
      }
    });

    start();
  }

  init();
})();