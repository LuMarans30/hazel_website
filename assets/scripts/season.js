(() => {
  'use strict';

  const root = document.documentElement;
  const darkScheme = matchMedia('(prefers-color-scheme: dark)');

  const NAMES = ['spring', 'summer', 'autumn', 'winter'];

  const PEAKS = [115, 206, 298, 385];

  const BLOB_ALPHA = {
    light: [0.75, 0.6, 0.55, 0.65],
    dark: [0.5, 0.42, 0.2, 0.38],
  };

  const PALETTE = {
    spring: {
      light: {
        bg: '#f4f9f2', text: '#12241a', muted: '#54695c', accent: '#1f6b45',
        btn: '#173d2a', btnFg: '#ffffff', h1a: '#0f2b1d', h1b: '#1f8057',
        ink: '#123c26', placeholder: '#c8e6d4',
        blobs: ['#a8deb4', '#96e0cd', '#e4eea0', '#c4e8d6'],
        hills: {
          top: ['#a6e3b8', '#70cf96', '#4fbf87', '#33a875'],
          bot: ['#7dd1a1', '#40ad7a', '#2e9469', '#1a7857'],
        },
      },
      dark: {
        bg: '#0a130f', text: '#e8f2ec', muted: '#98b2a3', accent: '#7fdc9f',
        btn: '#7fdc9f', btnFg: '#0a2417', h1a: '#eaf6ee', h1b: '#7fdc9f',
        ink: '#0f3c23', placeholder: '#ffffff',
        blobs: ['#226e48', '#146058', '#6e8228', '#285a46'],
        hills: {
          top: ['#1d5a43', '#237a55', '#2fa06c', '#1f805c'],
          bot: ['#14412f', '#175c3f', '#1a6b45', '#0f543d'],
        },
      },
      lightbox: ['#1e6e3c', '#146446', '#04100a'],
    },

    summer: {
      light: {
        bg: '#f3f8ec', text: '#11261a', muted: '#566a54', accent: '#1e6f3a',
        btn: '#1a3d25', btnFg: '#ffffff', h1a: '#102b18', h1b: '#2a8a3e',
        ink: '#123f20', placeholder: '#cfe8c8',
        blobs: ['#9fd9a0', '#8fdcc0', '#f0e890', '#bfe5b8'],
        hills: {
          top: ['#a4dc98', '#78cb78', '#4fb862', '#2fa04c'],
          bot: ['#80cc80', '#46ae5a', '#2f9450', '#1c7a40'],
        },
      },
      dark: {
        bg: '#0b140c', text: '#ebf3e6', muted: '#9ab49a', accent: '#8fe08a',
        btn: '#8fe08a', btnFg: '#0b2410', h1a: '#eef7e8', h1b: '#8fe08a',
        ink: '#12401f', placeholder: '#ffffff',
        blobs: ['#2a7a3a', '#1a6a50', '#8c8a22', '#2f6a3a'],
        hills: {
          top: ['#235f35', '#2a7f43', '#38a355', '#2a8a45'],
          bot: ['#16401f', '#1c5b2d', '#217038', '#16602c'],
        },
      },
      lightbox: ['#2b7a36', '#1a6a4a', '#050f08'],
    },

    autumn: {
      light: {
        bg: '#faf3ea', text: '#2a1710', muted: '#7a5a48', accent: '#9a3a16',
        btn: '#4a2113', btnFg: '#ffffff', h1a: '#2c140c', h1b: '#b5441c',
        ink: '#3c1c0e', placeholder: '#ecd5bf',
        blobs: ['#f0b98c', '#e59a7e', '#f0d98a', '#dcb89a'],
        hills: {
          top: ['#f4c898', '#f0aa69', '#ee8e50', '#de6c44'],
          bot: ['#e8b078', '#d6804e', '#be603a', '#a8422a'],
        },
      },
      dark: {
        bg: '#150d08', text: '#f3e6da', muted: '#b79f8e', accent: '#f0a15c',
        btn: '#f0a15c', btnFg: '#2a1205', h1a: '#fbeee2', h1b: '#f0a15c',
        ink: '#3a1a0c', placeholder: '#ffffff',
        blobs: ['#7a3a1a', '#6a2418', '#8a6a1a', '#5a3820'],
        hills: {
          top: ['#783c1e', '#aa5528', '#e16e34', '#be3e2a'],
          bot: ['#552a16', '#7d3c1e', '#964622', '#822c1e'],
        },
      },
      lightbox: ['#8a3c1a', '#7a2a20', '#140804'],
    },

    winter: {
      light: {
        bg: '#f1f5f8', text: '#14202b', muted: '#566574', accent: '#2e5f8a',
        btn: '#1c3550', btnFg: '#ffffff', h1a: '#10202f', h1b: '#2f6fa6',
        ink: '#12304c', placeholder: '#cddbe8',
        blobs: ['#b5cfe6', '#a8d8de', '#e0e6ee', '#c9d6e6'],
        hills: {
          top: ['#c8dcef', '#a2c4e4', '#7fb0d8', '#5f98c6'],
          bot: ['#aac8e6', '#74a4d2', '#5486b8', '#386a9c'],
        },
      },
      dark: {
        bg: '#0a1017', text: '#e8eff5', muted: '#9aacbb', accent: '#8fc0ec',
        btn: '#8fc0ec', btnFg: '#0a1c2e', h1a: '#eef5fb', h1b: '#8fc0ec',
        ink: '#0f2a45', placeholder: '#ffffff',
        blobs: ['#1f4a75', '#17566a', '#4a5a7a', '#2a4a6a'],
        hills: {
          top: ['#225480', '#2d73aa', '#3c96d7', '#3278b9'],
          bot: ['#1a3a5e', '#225684', '#286496', '#1c4e7d'],
        },
      },
      lightbox: ['#1e4a7a', '#1a5a78', '#040a12'],
    },
  };

  const parseHex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));

  const parse = (v) =>
    typeof v === 'string'
      ? parseHex(v)
      : Array.isArray(v)
        ? v.map(parse)
        : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, parse(x)]));

  const blend = (a, b, w) =>
    typeof a[0] === 'number'
      ? a.map((x, i) => x + (b[i] - x) * w)
      : Array.isArray(a)
        ? a.map((x, i) => blend(x, b[i], w))
        : Object.fromEntries(Object.keys(a).map((k) => [k, blend(a[k], b[k], w)]));

  const P = Object.fromEntries(NAMES.map((n) => [n, parse(PALETTE[n])]));

  const hex = (c) => '#' + c.map((v) => Math.round(v).toString(16).padStart(2, '0')).join('');
  const triplet = (c) => c.map((v) => Math.round(v)).join(' ');
  const rgba = (c, a) => `rgb(${triplet(c)} / ${a})`;

  function dayOfYear(d) {
    const ms = Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) - Date.UTC(d.getFullYear(), 0, 0);
    return Math.round(ms / 864e5);
  }

  function blendForDay(n) {
    n = ((n - 1) % 365 + 365) % 365 + 1;
    if (n < PEAKS[0]) n += 365;
    let i = 0;
    while (i < 3 && n >= PEAKS[i + 1]) i++;
    const start = PEAKS[i];
    const end = i < 3 ? PEAKS[i + 1] : PEAKS[0] + 365;
    const u = (n - start) / (end - start);
    return { from: NAMES[i], to: NAMES[(i + 1) % 4], w: u * u * (3 - 2 * u) };
  }

  function currentDay() {
    const q = new URLSearchParams(location.search);
    const season = q.get('season');
    if (season && NAMES.includes(season)) return PEAKS[NAMES.indexOf(season)];
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(q.get('date') || '');
    if (m) return dayOfYear(new Date(+m[1], +m[2] - 1, +m[3]));
    return dayOfYear(new Date());
  }

  let current = null;

  function compute(day = currentDay()) {
    const { from, to, w } = blendForDay(day);
    return {
      light: blend(P[from].light, P[to].light, w),
      dark: blend(P[from].dark, P[to].dark, w),
      lightbox: blend(P[from].lightbox, P[to].lightbox, w),
    };
  }

  function apply(notify, day = currentDay()) {
    current = compute(day);
    const key = darkScheme.matches ? 'dark' : 'light';
    const p = current[key];
    const set = (name, value) => root.style.setProperty(name, value);

    set('--bg', hex(p.bg));
    set('--text', hex(p.text));
    set('--muted', hex(p.muted));
    set('--accent', hex(p.accent));
    set('--btn-bg', hex(p.btn));
    set('--btn-fg', hex(p.btnFg));
    set('--h1-a', hex(p.h1a));
    set('--h1-b', hex(p.h1b));
    set('--ink', triplet(p.ink));
    set('--pill', rgba(p.accent, key === 'dark' ? 0.14 : 0.1));
    set('--placeholder', rgba(p.placeholder, key === 'dark' ? 0.06 : 0.55));
    p.blobs.forEach((c, i) => set(`--blob-${i + 1}`, rgba(c, BLOB_ALPHA[key][i])));

    set('--lb-1', triplet(current.lightbox[0]));
    set('--lb-2', triplet(current.lightbox[1]));
    set('--lb-base', triplet(current.lightbox[2]));

    for (const m of document.querySelectorAll('meta[name="theme-color"]')) {
      m.content = hex(current[/dark/.test(m.media) ? 'dark' : 'light'].bg);
    }

    if (notify) window.dispatchEvent(new Event('seasonchange'));
  }

  const flat = (layers) => Float32Array.from(layers.flat().map((v) => v / 255));

  window.siteSeason = {
    hills() {
      const h = current[darkScheme.matches ? 'dark' : 'light'].hills;
      return { top: flat(h.top), bot: flat(h.bot) };
    },
  };

  apply(false);
  darkScheme.addEventListener('change', () => apply(true));
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) apply(true);
  });
})();
