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

float sdHalf(vec2 p, float b, float w) {
  float r = (b * b + w * w) / (2.0 * w);
  float d = r - w;
  return ((p.y - b) * d > p.x * b) ? length(p - vec2(0.0, b))
                                   : length(p - vec2(-d, 0.0)) - r;
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

        float sz = size * (0.8 + 0.4 * r4.w);

        float flap  = 0.08 + 0.30 * gst;
        float spin  = (r2.z - 0.5) * 0.5 * (r3.y > 0.5 ? 1.0 : 0.3);
        float theta = -1.5708 + (r2.x - 0.5) * 2.2
                    + 0.45 * sin(0.5 * t + r2.y * TAU)
                    + spin * t
                    + flap * sin(mix(2.0, 4.0, r2.w) * t + r3.x * TAU);

        float phi = mix(0.5, 1.2, r3.z) * t + r3.w * TAU;
        float wc  = clamp(0.30 + 1.10 * cos(phi), -1.0, 1.0);
        float ws  = max(abs(wc), 0.16);
        float sps = sin(phi) < 0.0 ? -1.0 : 1.0;
        float ls  = 0.90 + 0.10 * cos(0.8 * phi + 1.7);

        float cr = cos(theta), sr = sin(theta);
        vec2 q = vec2(cr * d.x + sr * d.y, -sr * d.x + cr * d.y) / sz;
        q.y /= ls;
        float bend = (r1.z - 0.5) * 0.5 + 0.10 * sin(0.9 * t + r3.x * TAU) * (0.3 + gst);
        q.x -= bend * q.y * q.y;
        float lx = q.x;
        q.x /= ws;

        // Geometry
        float W  = mix(0.15, 0.26, r1.w);
        float B1 = mix(0.52, 0.62, r4.z);
        float B2 = 0.98 - B1;
        vec2  aq = abs(q);
        float sdL = (q.y >= 0.0) ? sdHalf(aq, B1, W) : sdHalf(aq, B2, W);
        vec2  sq  = vec2(q.x, q.y - clamp(q.y, -B2 - 0.14, -B2 + 0.03));
        float sdS = length(sq) - 0.018;
        float sd  = min(sdL, sdS);

        float dpx = sd * sz * min(ws, ls);
        float aaw = 0.6 + blur;
        float cov = 1.0 - smoothstep(-aaw, aaw, dpx);
        if (cov < 0.004) continue;

        float vein = 0.0;
        float veinVis = smoothstep(18.0, 30.0, sz) * (1.0 - smoothstep(1.2, 3.2, blur));
        if (veinVis > 0.01) {
          float depth = clamp(-sdL / 0.05, 0.0, 1.0);
          float sv    = abs(fract(q.y * 7.0 - abs(lx) * 3.2) - 0.5);
          float sideV = 1.0 - smoothstep(0.07, 0.17, sv);
          float mw    = mix(0.014, 0.005, smoothstep(-B2, B1, q.y));
          float rib   = 1.0 - smoothstep(mw, mw + 0.012, abs(lx));
          rib *= smoothstep(-B2 - 0.02, -B2 + 0.05, q.y) *
                 (1.0 - smoothstep(B1 * 0.62, B1 * 0.95, q.y));
          vein = max(0.6 * sideV, rib) * depth;
        }

        // Lighting
        float cfold = clamp(lx / 0.14, -1.0, 1.0) * 0.55;
        vec3  nl = normalize(vec3(-cfold, 0.0, 1.0));
        float sp = sps * sqrt(max(0.0, 1.0 - wc * wc));
        vec3  n  = vec3(nl.x * wc + nl.z * sp, 0.0, -nl.x * sp + nl.z * wc);
        if (n.z < 0.0) n = -n;
        vec3  Ll = vec3(cr * SUN.x + sr * SUN.y, -sr * SUN.x + cr * SUN.y, SUN.z);
        float lit  = clamp(0.55 + 0.55 * dot(n, Ll), 0.0, 1.0);
        float spec = pow(clamp(dot(n, normalize(Ll + vec3(0.0, 0.0, 1.0))), 0.0, 1.0), 30.0);

        // Colour from the seasonal palette
        int  pal  = int(min(floor(r4.x * 4.0), 3.0));
        vec3 Tc   = uTop[pal];
        vec3 Bc   = uBot[pal];
        float tone = r4.y;
        vec3 shadowC = mix(Bc, Tc, 0.10 + 0.25 * tone) * mix(0.70, 0.90, uDark);
        vec3 lightC  = mix(Bc, Tc, 0.55 + 0.35 * tone) * mix(0.95, 1.30, uDark);

        float under = (wc < 0.0) ? 1.0 : 0.0;
        vec3 col = mix(shadowC, lightC, lit);
        col = mix(col, lightC, 0.35 * under);
        vec3 veinC = mix(col * 0.62, mix(col, uAccent, 0.55), uDark);
        col = mix(col, veinC, vein * veinVis * (0.40 + 0.25 * under));
        col += uAccent * spec * mix(0.04, 0.20, uDark) * (1.0 - 0.5 * under);
        col = clamp(col, 0.0, 1.0);

        float a = min(cov * OPAC[L] * OPACITY * mix(1.0, 1.15, uDark), 1.0);
        acc.rgb = col * a + acc.rgb * (1.0 - a);
        acc.a   = a + acc.a * (1.0 - a);
      }
    }
  }

  fragColor = acc;
}