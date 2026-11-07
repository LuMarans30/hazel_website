(() => {
  'use strict';

  const SPEC = __SPEC__;

  const smooth = (u) => u * u * (3 - 2 * u);

  function blendForDay(day) {
    const P = SPEC.peaks;
    let n = (((day - 1) % 365) + 365) % 365 + 1;
    if (n < P[0]) n += 365;
    let i = 0;
    while (i < 3 && n >= P[i + 1]) i++;
    const end = i < 3 ? P[i + 1] : P[0] + 365;
    return [i, (i + 1) % 4, smooth((n - P[i]) / (end - P[i]))];
  }

  function dayOfYear(d) {
    const y = d.getFullYear();
    return Math.round((Date.UTC(y, d.getMonth(), d.getDate()) - Date.UTC(y, 0, 1)) / 864e5) + 1;
  }

  function unpack(packed) {
    const out = [];
    for (let i = 0; i < packed.length; i += 6) {
      out.push([0, 2, 4].map((j) => parseInt(packed.slice(i + j, i + j + 2), 16)));
    }
    return out;
  }

  const channels = (c) => c.map((v) => Math.round(v));
  const hex = (c) => '#' + channels(c).map((v) => v.toString(16).padStart(2, '0')).join('');
  const triplet = (c) => channels(c).join(' ');

  function resolve(palette, from, to, w) {
    const a = unpack(palette.seasons[from]);
    const b = unpack(palette.seasons[to]);
    const colors = a.map((c, i) => c.map((v, k) => v + (b[i][k] - v) * w));
    const vars = {};
    let css = '';
    for (const [name, x, fmt, alpha] of palette.decls) {
      const value =
        fmt === undefined ? x
        : fmt === 'h' ? hex(colors[x])
        : fmt === 't' ? triplet(colors[x])
        : `rgb(${triplet(colors[x])} / ${alpha})`;
      vars[name] = value;
      css += `--${name}:${value};`;
    }
    return { css, vars };
  }

  const style = document.getElementById('theme');
  if (!style) return;

  const [from, to, w] = blendForDay(dayOfYear(new Date()));
  const light = resolve(SPEC.light, from, to, w);
  const dark = resolve(SPEC.dark, from, to, w);

  const css = `:root{${light.css}}@media (prefers-color-scheme:dark){:root{${dark.css}}}`;
  if (style.textContent !== css) style.textContent = css;

  for (const m of document.querySelectorAll('meta[name="theme-color"]')) {
    const isDark = (m.getAttribute('media') || '').includes('dark');
    m.setAttribute('content', (isDark ? dark : light).vars.bg);
  }

  const icon = document.querySelector('link[rel="icon"]');
  if (icon) {
    icon.setAttribute(
      'href',
      SPEC.favicon
        .replace('{light}', light.vars.accent.slice(1))
        .replace('{dark}', dark.vars.accent.slice(1)),
    );
  }
})();