(() => {
  'use strict';

  const CFG = {
    maxDpr: 1.5,
    maxFps: 60,
    storageKey: 'hills-t',
    pointerSmoothing: 0.04,
  };

  const root = document.documentElement;
  const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const darkScheme = matchMedia('(prefers-color-scheme: dark)');

  const FALLBACK = {
    light: {
      top: [0.65, 0.89, 0.72, 0.44, 0.81, 0.59, 0.31, 0.75, 0.53, 0.20, 0.66, 0.46],
      bot: [0.49, 0.82, 0.63, 0.25, 0.68, 0.48, 0.18, 0.58, 0.41, 0.10, 0.47, 0.34],
    },
    dark: {
      top: [0.114, 0.353, 0.263, 0.137, 0.478, 0.333, 0.184, 0.627, 0.424, 0.120, 0.500, 0.360],
      bot: [0.078, 0.255, 0.184, 0.090, 0.360, 0.247, 0.102, 0.420, 0.270, 0.060, 0.330, 0.240],
    },
  };

  const TRAIL = 24;
  const WAKE_LIFE = 2.8;

  const LAYERS = [
    { base: 0.60, amp: 0.115, wave: 1150.0, speed: 12.0 },
    { base: 0.46, amp: 0.105, wave: 860.0,  speed: 18.0 },
    { base: 0.33, amp: 0.095, wave: 660.0,  speed: 26.0 },
    { base: 0.21, amp: 0.080, wave: 500.0,  speed: 34.0 },
  ];

  const VERT = `#version 300 es
void main() {
  vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

  const FRAG = `#version 300 es
precision highp float;

uniform vec2  uSize;
uniform float uScale;
uniform float uTime;
uniform float uDark;
uniform vec3  uTop[4];
uniform vec3  uBot[4];
uniform vec4  uTrail[${TRAIL}];
uniform vec4  uTrailMask[${TRAIL}];
uniform vec4  uTrailAux[${TRAIL}];
uniform float uWake;
uniform vec3  uCursor;
out vec4 fragColor;

const float TAU = 6.2831853;

const float BASE[4]   = float[4](0.60, 0.46, 0.33, 0.21);
const float AMP[4]    = float[4](0.115, 0.105, 0.095, 0.08);
const float WAVE[4]   = float[4](1150.0, 860.0, 660.0, 500.0);
const float SPEED[4]  = float[4](12.0, 18.0, 26.0, 34.0);
const float OPAC_L[4] = float[4](0.28, 0.30, 0.33, 0.36);
const float OPAC_D[4] = float[4](0.30, 0.32, 0.35, 0.38);
const vec3 CAUSC = vec3(1.15, 1.0, 1.15);

const vec3 LIGHT = normalize(vec3(-0.50, 0.55, 0.67));
const vec3 HALFV = normalize(LIGHT + vec3(0.0, 0.0, 1.0));

float hillHeight(int i, float x, float wsc, float H, out float slope) {
  float fi = float(i);
  float s  = SPEED[i] * wsc * uTime;
  float k1 = TAU / (WAVE[i] * wsc);
  float ph = fi * 1.9 + 0.7;
  float a1 = k1 * (x + s) + ph;
  float a2 = k1 * 2.17 * (x - 0.6 * s) + ph * 1.7;
  float a3 = k1 * 0.41 * (x + 1.4 * s) + ph * 0.5;
  float h  = 0.50 * sin(a1) + 0.22 * sin(a2) + 0.36 * sin(a3);
  slope = AMP[i] * H * (0.50 * k1 * cos(a1) + 0.22 * k1 * 2.17 * cos(a2) + 0.36 * k1 * 0.41 * cos(a3));
  return BASE[i] * H + AMP[i] * H * h + 5.0 * sin(uTime * 0.4 + fi * 0.8);
}

float lightField(vec2 q) {
  float a = sin(q.x * 0.0125 + 2.0 * sin(q.y * 0.021 + uTime * 0.18) + uTime * 0.11);
  float b = sin(q.x * 0.0049 - q.y * 0.0071 + 1.3 - uTime * 0.05);
  return 0.5 + 0.28 * a + 0.22 * b;
}

void wake(int k, vec2 p, float S, float wsc, out float hh, out vec2 gg, out vec3 hs) {
  hh = 0.0;
  gg = vec2(0.0);
  hs = vec3(0.0);
  for (int i = 0; i < ${TRAIL}; i++) {
    vec4  w   = uTrail[i];
    float age = uTime - w.z;
    if (w.w <= 0.0 || age < 0.0 || age > ${WAKE_LIFE.toFixed(2)}) continue;

    float mask = uTrailMask[i][k];
    if (mask < 0.015) continue;

    float spd  = uTrailAux[i].x;
    float str  = w.w * mask;
    float fade = 1.0 - smoothstep(${WAKE_LIFE.toFixed(2)} - 0.6, ${WAKE_LIFE.toFixed(2)}, age);

    float drift  = SPEED[k] * wsc * age;
    vec2  center = vec2(w.x - drift, w.y);

    float front = 185.0 * S * (0.80 + 0.50 * spd) * age;
    float wid   = (44.0 + 26.0 * age) * S * (0.85 + 0.45 * spd);
    float sd    = 42.0 * S * (0.8 + 0.2 * str);
    float rmax  = front + 2.6 * wid + 2.0 * sd;
    float kk    = TAU / (76.0 * S * (0.75 + 0.80 * spd));
    float rl0   = str * exp(-age * 0.75) * smoothstep(0.0, 0.14, age) * fade * 2.7 * S;

    vec2  dv = p - center;
    float r2 = dot(dv, dv);
    if (r2 <= rmax * rmax) {
      float r   = sqrt(r2) + 1e-3;
      vec2  dir = dv / r;
      float D  = exp(-r2 / (sd * sd));
      float dl = str * exp(-age / 0.70) * fade * 12.0 * S;
      hh -= D * dl;
      gg += dir * (2.0 * r / (sd * sd)) * D * dl;
      float ft1  = dl * (2.0 * r / (sd * sd)) * D;
      float ft2  = -dl * (4.0 * r2 / (sd * sd * sd * sd) - 2.0 / (sd * sd)) * D;
      float ft1r = ft1 / r;
      hs += vec3(ft2 * dir.x * dir.x + ft1r * (1.0 - dir.x * dir.x),
                 ft2 * dir.y * dir.y + ft1r * (1.0 - dir.y * dir.y),
                 (ft2 - ft1r) * dir.x * dir.y);
    }

    for (int m = 0; m < 3; m++) {
      vec2 cm = center;
      if (m == 1) cm.x = -center.x;
      if (m == 2) cm.x = 2.0 * uSize.x - center.x;
      vec2  dvm = p - cm;
      float r2m = dot(dvm, dvm);
      if (r2m > rmax * rmax) continue;

      float rr   = sqrt(r2m + 90.0 * S * S);
      vec2  dirr = dvm / rr;
      float x   = rr - front;
      float env = exp(-(x * x) / (wid * wid));
      float ph  = kk * x;
      float rl  = rl0 * ((m == 0) ? 1.0 : 0.85);
      hh += rl * env * cos(ph);
      float fw1 = rl * env * (-kk * sin(ph) - 2.0 * x / (wid * wid) * cos(ph));
      float fw2 = rl * env * ((4.0 * x * x / (wid * wid * wid * wid) - 2.0 / (wid * wid) - kk * kk) * cos(ph)
                              + 4.0 * x * kk / (wid * wid) * sin(ph));
      gg += dirr * fw1;
      float fw1r = fw1 / rr;
      hs += vec3(fw2 * dirr.x * dirr.x + fw1r * (1.0 - dirr.x * dirr.x),
                 fw2 * dirr.y * dirr.y + fw1r * (1.0 - dirr.y * dirr.y),
                 (fw2 - fw1r) * dirr.x * dirr.y);
    }
  }
  gg = clamp(gg, vec2(-0.75), vec2(0.75));
}

const vec2  WDIR[8] = vec2[8](vec2(0.98007, 0.19867), vec2(0.58168, 0.81342), vec2(-0.12884, 0.99166), vec2(-0.70271, 0.71147), vec2(-0.99581, 0.09146), vec2(-0.79097, -0.61186), vec2(-0.21080, -0.97753), vec2(0.63469, -0.77276));
const float WLEN[8] = float[8](230.0, 170.0, 125.0, 200.0, 95.0, 150.0, 110.0, 140.0);
const float WSPD[8] = float[8](0.30, -0.24, 0.20, -0.33, 0.27, -0.21, 0.31, -0.18);
const float WAMP[8] = float[8](0.50, 0.45, 0.40, 0.48, 0.33, 0.41, 0.35, 0.38);

const vec3  SPC[5] = vec3[5](vec3(1.00, 0.28, 0.18), vec3(1.00, 0.85, 0.25), vec3(0.45, 1.00, 0.55),
                             vec3(0.20, 0.85, 1.00), vec3(0.35, 0.35, 1.00));
const float SP_LO = 0.80;
const float SP_HI = 1.20;

vec3 spectrumColor(float u) {
  float f = clamp(u, 0.0, 1.0) * 4.0;
  int   i = int(min(f, 3.0));
  return mix(SPC[i], SPC[i + 1], f - float(i));
}

vec3 causticSpectrum(vec2 u, float seed, vec3 hw, float jit) {
  float cs = cos(seed * 0.7);
  float sn = sin(seed * 0.7);
  float hxx = 0.0, hyy = 0.0, hxy = 0.0;
  for (int i = 0; i < 8; i++) {
    vec2  d0  = WDIR[i];
    vec2  dir = vec2(d0.x * cs - d0.y * sn, d0.x * sn + d0.y * cs);
    float ph  = TAU * dot(dir, u) / WLEN[i] + uTime * WSPD[i] + seed * 1.3 + float(i) * 1.1;
    float sv  = -WAMP[i] * sin(ph);
    hxx += sv * dir.x * dir.x;
    hyy += sv * dir.y * dir.y;
    hxy += sv * dir.x * dir.y;
  }
  hxx += hw.x; hyy += hw.y; hxy += hw.z;

  const int NS = 9;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < NS; i++) {
    float us  = clamp((float(i) + jit - 0.5) / float(NS - 1), 0.0, 1.0);
    float Ds  = mix(SP_LO, SP_HI, us);
    float a   = 1.0 + hxx * Ds;
    float b   = 1.0 + hyy * Ds;
    float det = a * b - hxy * hxy * Ds * Ds;
    float y   = 0.10 / (abs(det) + 0.10);
    float line = y * sqrt(y);
    float fold = smoothstep(0.05, -0.20, det);
    acc += spectrumColor(us) * (line + 0.20 * fold);
  }
  return acc / (float(NS) * vec3(0.581, 0.754, 0.598));
}

float cursorLight(vec2 q, float S) {
  vec2  dv = (q - uCursor.xy) / (170.0 * S);
  float r2 = dot(dv, dv);
  return uCursor.z * (exp(-r2 * 3.0) + 0.35 * exp(-r2 * 0.8));
}

void main() {
  vec2  p  = gl_FragCoord.xy / uScale;
  float H  = uSize.y;

  if (p.y > 0.76 * H + 30.0) { fragColor = vec4(0.0); return; }

  float S   = H / 300.0;
  float wsc = clamp(uSize.x / 1400.0, 0.45, 1.0);
  float px  = 1.0 / uScale;
  const vec3 CH = vec3(0.0, 0.33, 0.67);
  float grain = fract(sin(dot(floor(gl_FragCoord.xy), vec2(12.9898, 78.233))) * 43758.5453);

  vec3 col = vec3(0.0);
  vec3 T   = vec3(1.0);

  float cwh[4] = float[4](0.0, 0.0, 0.0, 0.0);
  vec2  cwg[4] = vec2[4](vec2(0.0), vec2(0.0), vec2(0.0), vec2(0.0));
  vec3  cwH[4] = vec3[4](vec3(0.0), vec3(0.0), vec3(0.0), vec3(0.0));

  for (int ci = 0; ci < 3; ci++) {
    int   c    = (ci == 0) ? 1 : ((ci == 1) ? 0 : 2);
    float disp = (c == 0) ? 0.92 : ((c == 1) ? 1.0 : 1.08);
    vec2  q    = p;
    float t    = 1.0;
    float acc  = 0.0;

    for (int k = 3; k >= 0; k--) {
      float slope;
      float h  = hillHeight(k, q.x, wsc, H, slope);
      float d0 = h - q.y;

      if (ci == 0 && uWake > 0.5 && d0 > -(30.0 * S) - px) {
        float wh;
        vec2  wg;
        vec3  wH;
        wake(k, q, S, wsc, wh, wg, wH);
        cwh[k] = wh;
        cwg[k] = wg;
        cwH[k] = wH;
      }

      float wl  = cwh[k];
      float d   = d0 + wl;
      float cov = clamp(d / px + 0.5, 0.0, 1.0);
      if (cov <= 0.0) continue;

      // Bevel
      float e    = exp(-max(d, 0.0) / (30.0 * S));
      vec2 tilt  = 1.2 * e * vec2(-slope, 1.0);

      // Interior ripples
      float sp = SPEED[k] * wsc * uTime;
      float r1 = 0.012 * (q.x + 1.5 * sp) + 0.020 * q.y + float(k) * 2.1;
      float r2 = 0.021 * (q.x - 0.8 * sp) - 0.014 * q.y + float(k) * 0.9 + 1.0;
      vec2  g  = vec2(0.012 * cos(r1) * 8.0 + 0.021 * cos(r2) * 4.5,
                      0.020 * cos(r1) * 8.0 - 0.014 * cos(r2) * 4.5);
      g *= smoothstep(0.0, 16.0 * S, d);

      vec2 nxy = tilt + g + 1.0 * cwg[k];
      vec3 n   = normalize(vec3(nxy, 1.0));

      // Frosted refraction
      vec2 off = -nxy * (56.0 * S) * (0.4 + (1.0 - e));
      float v  = 0.5 * (lightField(q + off * disp) + lightField(q + off * disp * 1.7 + vec2(90.0, 40.0)));
      float vs = smoothstep(0.15, 0.95, v);
      float B  = mix(mix(0.45, 1.0, vs), mix(0.20, 0.95, vs), uDark);

      // Lighting
      float fr   = pow(1.0 - n.z, 1.6);
      float F    = clamp(0.03 + 0.70 * fr, 0.0, 1.0);
      float lit  = clamp(dot(normalize(vec2(-slope, 1.0)), vec2(-0.6, 0.8)), 0.0, 1.0);
      float edge = exp(-max(d, 0.0) / 2.2) * (0.5 + 0.5 * lit);
      float glow = exp(-max(d, 0.0) / (14.0 * S)) * (0.4 + 0.6 * lit);

      // Environment reflection (soft, tinted by the seasonal palette)
      float env  = pow(lightField(q * 0.7 + nxy * (200.0 * S) * disp + vec2(137.0, 61.0)), 2.0);

      vec3  top   = uTop[k];
      vec3  bot   = uBot[k];
      float tint  = mix(top[c], bot[c], smoothstep(0.0, 100.0 * S, d));

      // Faint chromatic fringe along the rim only
      float edgeBand = exp(-max(d, 0.0) / (24.0 * S));
      float ph    = nxy.x * 2.0 + d * 0.03 + float(k) * 0.23 + CH[c];
      float irid  = 0.5 + 0.5 * cos(TAU * ph);
      float diffr = irid * irid * (0.2 * fr + 0.3 * edgeBand);

      // Caustics
      vec3  wakeH = clamp(cwH[k] * (45.0 * S), vec3(-1.2), vec3(1.2));
      float cl = cursorLight(q + off * disp, S);

      vec2  causOffset  = off * (disp - 1.0) * (1.8 + 2.7 * cl);
      vec2  lensDistort = (q - uCursor.xy) * (0.20 * cl);

      float causK = causticSpectrum((q + causOffset - lensDistort + vec2(sp, 0.0)) / S, float(k), wakeH, grain)[c];
      float causMod = smoothstep(0.0, 10.0 * S, d) * exp(-max(d, 0.0) / (130.0 * S))
                    * (0.55 + 0.15 * float(k)) * (1.0 + 1.2 * cl);
      float caus = causK * causMod;
      float hot  = smoothstep(0.65, 1.15, causK) * causMod;

      float thick = 1.0 - exp(-max(d, 0.0) / (120.0 * S));
      float body  = tint * (0.55 + 0.65 * B) * (1.0 - 0.30 * thick) * (1.0 + 0.10 * (grain - 0.5)) * (1.0 + 0.25 * cl);

      float add = F * env * (0.15 + 0.70 * top[c]) * mix(0.7, 1.0, uDark)
          + 0.28 * edge * (1.0 + 0.8 * cl)
          + 0.10 * cl * (0.20 + 0.80 * top[c])
          + 0.14 * glow * (0.20 + 0.80 * top[c])
          + 0.05 * diffr
          + 0.55 * caus * CAUSC[c]
          + 0.40 * hot;

      float op  = mix(OPAC_L[k], OPAC_D[k], uDark);
      float opb = op * (0.55 + 0.45 * thick);
      float a   = cov * clamp(opb + 0.40 * F + 0.10 * edge, 0.0, 0.80);

      acc += t * (a * body + cov * add);
      t   *= 1.0 - a;
      q   += cov * off * disp;
    }

    col[c] = acc;
    T[c]   = t;
  }

  float A = 1.0 - (T.x + T.y + T.z) / 3.0;
  float n1  = fract(sin(dot(gl_FragCoord.xy, vec2(12.9898, 78.233))) * 43758.5453);
  float n2  = fract(sin(dot(gl_FragCoord.xy + 19.19, vec2(39.3468, 11.135))) * 24634.6345);
  float tri = (n1 + n2 - 1.0) / 255.0 * smoothstep(0.0, 0.03, A);
  fragColor = vec4(max(col + tri, 0.0), clamp(A + tri, 0.0, 1.0));
}`;

  let canvas = null;
  let gl = null;
  let uSize, uScale, uTime, uDark, uTrail, uTrailMask, uWake, uCursor, uTrailAux, uTop, uBot;
  let hillTop = new Float32Array(FALLBACK.light.top);
  let hillBot = new Float32Array(FALLBACK.light.bot);
  let cssW = 0;
  let cssH = 0;
  let scale = 1;
  let raf = 0;
  let lastFrame = 0;
  let lightbox = null;

  const trail = new Float32Array(TRAIL * 4);
  const trailMask = new Float32Array(TRAIL * 4);
  const trailAux = new Float32Array(TRAIL * 4);
  const tempMask = new Float32Array(4);
  let trailI = 0;
  let wakeUntil = 0;
  let lastSpawnT = -1e9;
  let lastSpawnX = -1e9;
  let lastSpawnY = -1e9;

  let targetX = 0;
  let targetY = 0;
  let currX = 0;
  let currY = 0;
  let isPointerActive = false;
  let hasPos = false;
  let cursorI = 0;
  let vSm = 0;

  let t = 0;
  try {
    t = parseFloat(sessionStorage.getItem(CFG.storageKey)) || 0;
  } catch (_) {}

  const TAU = Math.PI * 2;
  const clamp = (x, a, b) => Math.min(b, Math.max(a, x));
  const smoothstep = (a, b, x) => {
    const u = clamp((x - a) / (b - a), 0, 1);
    return u * u * (3 - 2 * u);
  };

  function crestHeight(k, x) {
    const L = LAYERS[k];
    const wsc = clamp(cssW / 1400, 0.45, 1);
    const s = L.speed * wsc * t;
    const k1 = TAU / (L.wave * wsc);
    const ph = k * 1.9 + 0.7;
    const a1 = k1 * (x + s) + ph;
    const a2 = k1 * 2.17 * (x - 0.6 * s) + ph * 1.7;
    const a3 = k1 * 0.41 * (x + 1.4 * s) + ph * 0.5;
    const h = 0.5 * Math.sin(a1) + 0.22 * Math.sin(a2) + 0.36 * Math.sin(a3);
    return L.base * cssH + L.amp * cssH * h + 5 * Math.sin(t * 0.4 + k * 0.8);
  }

  function computeLayerMask(x, y, out) {
    const S = cssH / 300;
    let reach = 1.0;
    for (let k = 3; k >= 0; k--) {
      const ch = crestHeight(k, x);
      const dy = y - ch;
      const span = dy > 0 ? 34 * S : 50 * S;
      const prox = Math.exp(-(dy * dy) / (span * span));

      out[k] = reach * prox;

      if (dy < 6 * S) {
        const penetration = Math.max(0, -dy);
        reach *= Math.max(0.12, 1.0 - smoothstep(0, 32 * S, penetration) * 0.85);
      }
    }
  }

  function compile(type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      console.warn('hills: shader compile failed\n' + gl.getShaderInfoLog(s));
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
      console.warn('hills: program link failed\n' + gl.getProgramInfoLog(prog));
      return false;
    }
    gl.useProgram(prog);
    uSize = gl.getUniformLocation(prog, 'uSize');
    uScale = gl.getUniformLocation(prog, 'uScale');
    uTime = gl.getUniformLocation(prog, 'uTime');
    uDark = gl.getUniformLocation(prog, 'uDark');
    uTrail = gl.getUniformLocation(prog, 'uTrail');
    uTrailMask = gl.getUniformLocation(prog, 'uTrailMask');
    uTrailAux = gl.getUniformLocation(prog, 'uTrailAux');
    uWake = gl.getUniformLocation(prog, 'uWake');
    uCursor = gl.getUniformLocation(prog, 'uCursor');
    uTop = gl.getUniformLocation(prog, 'uTop');
    uBot = gl.getUniformLocation(prog, 'uBot');
    gl.clearColor(0, 0, 0, 0);
    resize();
    return true;
  }

  function refreshPalette() {
    let c = null;
    try {
      c = window.siteSeason && window.siteSeason.hills();
    } catch (_) {}
    if (!c) c = darkScheme.matches ? FALLBACK.dark : FALLBACK.light;
    hillTop = new Float32Array(c.top);
    hillBot = new Float32Array(c.bot);
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
    gl.viewport(0, 0, canvas.width, canvas.height);
    draw();
  }

  function draw() {
    if (!gl || gl.isContextLost() || !cssW) return;
    gl.uniform2f(uSize, cssW, cssH);
    gl.uniform1f(uScale, scale);
    gl.uniform1f(uTime, t);
    gl.uniform1f(uDark, darkScheme.matches ? 1 : 0);
    gl.uniform3fv(uTop, hillTop);
    gl.uniform3fv(uBot, hillBot);
    gl.uniform4fv(uTrail, trail);
    gl.uniform4fv(uTrailMask, trailMask);
    gl.uniform4fv(uTrailAux, trailAux);
    gl.uniform1f(uWake, t < wakeUntil ? 1 : 0);
    gl.uniform3f(uCursor, currX, currY, cursorI);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  function spawn(x, y, strength, speedN = 0.3) {
    const o = trailI * 4;
    trailAux[o] = speedN;
    trail[o] = x;
    trail[o + 1] = y;
    trail[o + 2] = t;
    trail[o + 3] = strength;

    computeLayerMask(x, y, tempMask);
    trailMask[o] = tempMask[0];
    trailMask[o + 1] = tempMask[1];
    trailMask[o + 2] = tempMask[2];
    trailMask[o + 3] = tempMask[3];

    trailI = (trailI + 1) % TRAIL;
    wakeUntil = t + WAKE_LIFE + 0.1;
    lastSpawnT = t;
    lastSpawnX = x;
    lastSpawnY = y;
  }

  function toCanvas(e) {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, r.bottom - e.clientY];
  }

  function nearHills(y) {
    return y > -20 && y < cssH * 0.82 + 50;
  }

  function onPointerMove(e) {
    if (!gl || reduceMotion.matches || paused()) return;
    const [x, y] = toCanvas(e);
    targetX = x;
    targetY = y;
    if (!hasPos) {
      currX = x;
      currY = y;
      hasPos = true;
    }
    isPointerActive = nearHills(y);
  }

  function onPointerDown(e) {
    if (!gl || reduceMotion.matches || paused()) return;
    const [x, y] = toCanvas(e);
    if (nearHills(y)) spawn(x, y, 1.0);
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

    cursorI += ((isPointerActive ? 1 : 0) - cursorI) * (1 - Math.exp(-dt / 0.18));

    if (isPointerActive) {
      const dx = targetX - currX;
      const dy = targetY - currY;
      const dist = Math.hypot(dx, dy);

      const follow = 1 - Math.exp(-dt / CFG.pointerSmoothing);
      const prevX = currX;
      const prevY = currY;
      currX += dx * follow;
      currY += dy * follow;

      const inst = Math.hypot(currX - prevX, currY - prevY) / Math.max(dt, 1e-3);
      vSm += (inst - vSm) * (1 - Math.exp(-dt / 0.08));

      const moveDist = Math.hypot(currX - lastSpawnX, currY - lastSpawnY);
      const timeSince = t - lastSpawnT;

      if ((moveDist > 22 * (cssH / 300) || (dist > 6 && timeSince > 0.075)) && nearHills(currY)) {
        const sN = Math.sqrt(clamp(vSm / (1400 * (cssH / 300)), 0, 1));
        spawn(currX, currY, 0.30 + 0.70 * sN, sN);
      }
    } else {
      vSm *= Math.exp(-dt / 0.1);
    }

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
    canvas = document.createElement('canvas');
    canvas.className = 'hills';
    canvas.setAttribute('aria-hidden', 'true');

    document.body.prepend(canvas);
    if (!setup()) {
      canvas.remove();
      return;
    }
    root.classList.add('hills-gl');

    new ResizeObserver(resize).observe(canvas);
    addEventListener('pointermove', onPointerMove, { passive: true });
    addEventListener('pointerdown', onPointerDown, { passive: true });
    root.addEventListener('pointerleave', () => (isPointerActive = false));
    refreshPalette();
    const repaint = () => {
      refreshPalette();
      draw();
    };
    darkScheme.addEventListener('change', repaint);
    addEventListener('seasonchange', repaint);
    document.addEventListener('visibilitychange', () => (document.hidden ? stop() : start()));
    addEventListener('pagehide', () => {
      try {
        sessionStorage.setItem(CFG.storageKey, String(t));
      } catch (_) {}
    });

    canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      stop();
      root.classList.remove('hills-gl');
    });
    canvas.addEventListener('webglcontextrestored', () => {
      if (setup()) {
        root.classList.add('hills-gl');
        start();
      }
    });

    draw();
    start();
  }

  init();
})();