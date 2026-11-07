#version 300 es
precision highp float;
precision highp int;

uniform vec2  uSize;
uniform float uScale;
uniform float uTime;
uniform float uDark;
uniform float uUnit;
uniform vec3  uTop[4];
uniform vec3  uBot[4];
uniform vec3  uAccent;
out vec4 fragColor;

const float TAU = 6.2831853;

#pragma leaves_config

const vec3  SUN = normalize(vec3(-0.45, -0.55, 0.70));

const float PRESENCE = 0.90;

uint pcg(uint v) {
  uint s = v * 747796405u + 2891336453u;
  uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

uint cellHash(ivec2 c, int layer) {
  uint x = uint(c.x + 1048576);
  uint y = uint(c.y + 1048576);
  return pcg(x + pcg(y + pcg(uint(layer) + 101u)));
}

vec4 rnd4(inout uint h) {
  h = pcg(h); uint a = h;
  h = pcg(h); uint b = h;
  h = pcg(h); uint c = h;
  h = pcg(h); uint d = h;
  return vec4(float(a), float(b), float(c), float(d)) * (1.0 / 4294967296.0);
}

vec2 driftAt(float t, float spd) {
  vec2 g = vec2(64.0 * sin(0.19 * t) + 26.0 * sin(0.47 * t + 1.3),
                34.0 * sin(0.15 * t + 0.6) + 12.0 * sin(0.41 * t + 2.1));
  return spd * (WIND * WIND_SPEED * t + g);
}

float gustAt(float t) {
  float v = 12.2 * cos(0.19 * t) + 12.2 * cos(0.47 * t + 1.3);
  return smoothstep(-6.0, 20.0, v);
}

float gain(float x, float k) {
  float a = 0.5 * pow(2.0 * ((x < 0.5) ? x : 1.0 - x), k);
  return (x < 0.5) ? a : 1.0 - a;
}

float sdHalf(vec2 p, float b, float w, out vec2 g) {
  float r = (b * b + w * w) / (2.0 * w);
  float d = r - w;
  vec2  v;
  float l;
  if ((p.y - b) * d > p.x * b) {
    v = p - vec2(0.0, b);
    l = length(v);
    g = v / max(l, 1e-5);
    return l;
  }
  v = p - vec2(-d, 0.0);
  l = length(v);
  g = v / max(l, 1e-5);
  return l - r;
}

void main() {
  vec2  p = vec2(gl_FragCoord.x, uSize.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  float U = uUnit;
  vec4  acc = vec4(0.0);

  for (int L = 0; L < LAYERS; L++) {
    float cell  = CELL[L] * U;
    float size  = SIZE[L] * U;
    float blur  = BLUR[L] * U;
    vec2  drift = driftAt(t, SPD[L]);

    vec2  w  = (p - drift) / cell;
    ivec2 ic = ivec2(floor(w));
    vec2  fr = w - vec2(ic);
    ivec2 o0 = ic - ivec2(fr.x < 0.5 ? 1 : 0, fr.y < 0.5 ? 1 : 0);

    for (int j = 0; j < 2; j++) {
      for (int i = 0; i < 2; i++) {
        ivec2 c = o0 + ivec2(i, j);
        uint  h = cellHash(c, L);
        h = pcg(h);
        if (float(h) * (1.0 / 4294967296.0) > DENS[L]) continue;

        vec4 r0 = rnd4(h);
        vec4 r1 = rnd4(h);

        float tl  = t - dot(vec2(c), vec2(2.1, 0.7));
        float gst = gustAt(tl);
        vec2 base = vec2(c) + 0.5 + (r0.xy - 0.5) * 0.56;
        vec2 wob  = 0.20 * vec2(sin(mix(0.25, 0.60, r1.x) * t + r0.z * TAU),
                                sin(mix(0.30, 0.75, r1.y) * t + r0.w * TAU));
        vec2 eddy = (0.10 + 0.10 * gst) *
                    vec2(sin(0.8 * tl + dot(base, vec2(0.6, 0.9))),
                         cos(0.65 * tl + dot(base, vec2(-0.7, 0.5)) + 1.0));
        vec2 lp = base + wob + eddy;

        vec2 d = (w - lp) * cell;
        if (dot(d, d) > 0.64 * size * size) continue;

        vec4 r2 = rnd4(h);
        vec4 r3 = rnd4(h);
        vec4 r4 = rnd4(h);
        vec4 r5 = rnd4(h);

        float sz = size * (0.8 + 0.4 * r4.w);

        float flap  = 0.08 + 0.30 * gst;
        float spin  = (r2.z - 0.5) * 0.5 * (r3.y > 0.5 ? 1.0 : 0.3);
        float theta = -1.5708 + (r2.x - 0.5) * 2.2
                    + 0.45 * sin(0.5 * t + r2.y * TAU)
                    + spin * t
                    + flap * sin(mix(2.0, 4.0, r2.w) * t + r3.x * TAU);

        // Flutter
        float phi = mix(0.5, 1.2, r3.z) * t + r3.w * TAU;
        float kk  = clamp(0.65 * (1.0 - cos(phi)) - 0.12, 0.0, 1.0);
        float al  = mix(2.1, 2.7, r5.y) * gain(kk, 1.4);
        al *= (sin(0.5 * phi) < 0.0) ? -1.0 : 1.0;
        float wc = cos(al);
        float sp = sin(al);
        float ls = 0.90 + 0.10 * cos(0.8 * phi + 1.7);

        float cr = cos(theta), sr = sin(theta);
        vec2 q = vec2(cr * d.x + sr * d.y, -sr * d.x + cr * d.y) / sz;
        q.y /= ls;
        float bend = (r1.z - 0.5) * 0.5 + 0.10 * sin(0.9 * t + r3.x * TAU) * (0.3 + gst);
        q.x -= bend * q.y * q.y;
        float lx = q.x;

        // Geometry
        float W  = mix(0.15, 0.26, r1.w);
        float B1 = mix(0.52, 0.62, r4.z);
        float B2 = 0.98 - B1;
        float tf = mix(0.18, 0.55, r5.x);
        float aaw = 0.6 + blur;

        // Palette
        int   pal  = int(min(floor(r4.x * 4.0), 3.0));
        int   pal2 = (pal + 1) & 3;
        float tone = r4.y;
        float dk0  = mix(0.70, 0.90, uDark);
        float dk1  = mix(0.95, 1.30, uDark);
        vec3  shA  = mix(uBot[pal],  uTop[pal],  0.10 + 0.25 * tone) * dk0;
        vec3  liA  = mix(uBot[pal],  uTop[pal],  0.55 + 0.35 * tone) * dk1;
        vec3  shB  = mix(uBot[pal2], uTop[pal2], 0.10 + 0.25 * tone) * dk0;
        vec3  liB  = mix(uBot[pal2], uTop[pal2], 0.55 + 0.35 * tone) * dk1;
        float grad = 0.40 * r5.z;

        vec3  Ll = vec3(cr * SUN.x + sr * SUN.y, -sr * SUN.x + cr * SUN.y, SUN.z);
        vec3  Hv = normalize(Ll + vec3(0.0, 0.0, 1.0));
        float veinVis = smoothstep(18.0, 30.0, sz) * (1.0 - smoothstep(1.2, 3.2, blur));

        vec2  sq = vec2(lx, (q.y - clamp(q.y, -B2 - 0.14, -B2 + 0.03)) * ls);
        float cS = 1.0 - smoothstep(-aaw, aaw, (length(sq) - 0.018) * sz);

        vec3  lc = vec3(0.0);
        float la = 0.0;
        float lm = 0.0;
        if (cS > 0.004) {
          lc = shA * 0.85 * cS;
          la = cS;
          lm = cS;
        }

        for (int s = 0; s < 2; s++) {
          float hs = ((s == 0) == (sp > 0.0)) ? 1.0 : -1.0;   // +1 right wing, -1 left
          float c0 = wc + hs * sp * tf;
          float ac = max(abs(c0), 0.02);
          float sg = (c0 < 0.0) ? -1.0 : 1.0;
          float u  = lx / (sg * ac);

          vec2  gr = vec2(0.0);
          vec2  aq = vec2(abs(u), abs(q.y));
          float sdL = sdHalf(aq, (q.y >= 0.0) ? B1 : B2, W, gr);

          float bT  = (q.y >= 0.0) ? B1 : B2;
          vec2  g2  = vec2(0.0);
          // Coverage in screen space: an exact lens of half-width W*ac, tips at B*ls.
          // Don't scale sdL by min(ac, ls) here, it streaks the tips when a wing goes edge-on.
          float dpx = sdHalf(vec2(abs(lx), abs(q.y) * ls), bT * ls, W * ac, g2) * sz;

          float cut = -hs * sg * lx * sz - ((s == 1) ? aaw : 0.0);
          float cov = 1.0 - smoothstep(-aaw, aaw, max(dpx, cut));
          if (cov < 0.004) continue;

          // Interior distance
          float dep   = max(-sdL, 0.0);
          float edge  = 1.0 - smoothstep(0.0, 0.08, dep);
          float crown = smoothstep(0.0, 0.16, abs(u));
          vec2  gv    = gr * vec2((u < 0.0) ? -1.0 : 1.0, (q.y < 0.0) ? -1.0 : 1.0);
          vec3  nl    = normalize(vec3(-hs * tf * crown + 0.8 * edge * gv.x,
                                       0.8 * edge * gv.y, 1.0));
          vec3  nr    = vec3(nl.x * wc + nl.z * sp, nl.y, -nl.x * sp + nl.z * wc);
          float fs    = nr.z * inversesqrt(nr.z * nr.z + 0.02);
          vec3  n     = nr * fs;
          float under = 0.5 - 0.5 * fs;

          float lit  = clamp(0.55 + 0.55 * dot(n, Ll), 0.0, 1.0);
          float spec = pow(clamp(dot(n, Hv), 0.0, 1.0), 30.0);

          float gm = grad * (1.0 - smoothstep(-B2, B1 * 0.7, q.y));
          vec3  sC = mix(shA, shB, gm);
          vec3  lC = mix(liA, liB, gm);
          vec3  col = mix(sC, lC, lit);
          col = mix(col, lC, 0.35 * under);

          if (veinVis > 0.01) {
            float band  = smoothstep(2.5, 6.0, sz * ac / 3.2);
            float depth = clamp(dep / 0.05, 0.0, 1.0);
            float sv    = abs(fract(q.y * 7.0 - abs(u) * 3.2) - 0.5);
            float sideV = 1.0 - smoothstep(0.07, 0.17, sv);
            float mw    = mix(0.014, 0.005, smoothstep(-B2, B1, q.y));
            float rib   = 1.0 - smoothstep(mw, mw + 0.012, abs(lx));
            rib *= smoothstep(-B2 - 0.02, -B2 + 0.05, q.y) *
                   (1.0 - smoothstep(B1 * 0.62, B1 * 0.95, q.y));
            float vein  = max(0.6 * sideV * band, rib) * depth;
            vec3  veinC = mix(col * 0.62, mix(col, uAccent, 0.55), uDark);
            col = mix(col, veinC, vein * veinVis * (0.40 + 0.25 * under));
          }

          // Light through the blade (cheap subsurface) when it faces away from the sun
          float tr = clamp(-dot(n, Ll), 0.0, 1.0);
          col += mix(lC, uAccent, 0.5) * (0.12 * tr * tr);

          // Fresnel rim
          float fre = pow(1.0 - min(abs(nr.z), 1.0), 2.5);
          col += uAccent * fre * mix(0.03, 0.09, uDark);

          col += uAccent * spec * mix(0.04, 0.20, uDark) * (1.0 - 0.5 * under);
          col = clamp(col, 0.0, 1.0);

          lc = col * cov + lc * (1.0 - cov);
          la = cov + la * (1.0 - cov);
          lm = max(lm, cov);
        }
        if (lm < 0.004) continue;

        vec3  col = lc / max(la, 1e-4);
        float a = min(lm * PRESENCE * OPAC[L] * OPACITY * mix(1.0, 1.15, uDark), 1.0);
        acc.rgb = col * a + acc.rgb * (1.0 - a);
        acc.a   = a + acc.a * (1.0 - a);
      }
    }
  }

  fragColor = acc;
}