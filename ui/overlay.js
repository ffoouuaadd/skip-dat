// Skip Dat overlay: a wheel of liquid glass cards drawn over the real desktop.
// Everything visible is driven by a few smoothed values (wheel position, presence, pill morph),
// so any input at any moment just retargets them: nothing queues up, nothing snaps.
//
// Shows only real songs: each source's own list as the app reports it. Cards are keyed by song,
// so a list that updates (a skip, a new song added) keeps the wheel on the song you were on.
// The control rail and source switcher beside the wheel take clicks while the shortcut is held.
(() => {
'use strict';
const TAURI = window.__TAURI__;
const invoke = (cmd, args) => (TAURI ? Promise.resolve(TAURI.core.invoke(cmd, args)).catch(err => { console.warn('[Skip Dat]', cmd, err); return null; }) : Promise.resolve(null));
const listen = (ev, fn) => (TAURI ? TAURI.event.listen(ev, e => { try { fn(e.payload); } catch (err) { console.error('[Skip Dat]', ev, err); } }) : null);
const artUrl = id => (TAURI && TAURI.core.convertFileSrc && !/^(data|https?):/.test(id) ? TAURI.core.convertFileSrc(id, 'skipart') : id);

// ---------- look and feel (card-local px; U scales the whole overlay) ----------
const U = 0.82;
const CARD = [125, 155], PILL = [132, 34], COVER = 109, COVER_Y = -30;
const PD = 1200, WR = 620, WSTEP = Math.asin(222 / 620), DEG = Math.PI / 180;
const FEEL = {
  wheelOmega: 19,     // 3-stage chain: one notch settles in about a third of a second
  topSpeed: 16,       // cards per second, so long jumps cruise instead of whipping
  float: 1, trail: 0.06, fan: 34,
  pillHold: 1.25,     // seconds the "now playing" pill stays before going into the cursor
  absorb: 0.42,       // seconds to be drawn into the cursor
};
// the rail beside the wheel: source switcher and transport buttons (card-local px)
const RAIL = { gap: 44, w: 150, chipH: 15, chipStep: 38, btnR: 20, btnStep: 50, maxChips: 6 };
const RAIL_X = CARD[0] + RAIL.gap + RAIL.w / 2;
// horizontal layout: a drum turning around a vertical axis; sources stacked top to bottom; the
// source names in a row above the wheel and the transport buttons centred under the front card
const WRH = 700, WSTEP_H = Math.asin(196 / 700), COLH = 400;
const HRAIL = { chipW: 150, chipStep: 160, chipY: -(CARD[1] + 44), btnY: CARD[1] + 46, btnStep: 56 };
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
if (reduceMotion) { FEEL.float = 0; FEEL.trail = 0; }

// ---------- math ----------
const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
const clamp01 = x => clamp(x, 0, 1);
const lerp = (a, b, x) => a + (b - a) * x;
const smoothstep = (a, b, x) => { const t = clamp01((x - a) / (b - a)); return t * t * (3 - 2 * t); };
const norm = v => { const l = Math.hypot(v[0], v[1]) || 1; return [v[0] / l, v[1] / l]; };
function mul(a, b) { const o = new Array(16); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) { let s = 0; for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k]; o[c * 4 + r] = s; } return o; }
const chain = (...ms) => ms.reduce(mul);
const Tm = (x, y, z) => [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, x, y, z, 1];
const Sm = (x, y, z) => [x, 0, 0, 0, 0, y, 0, 0, 0, 0, z, 0, 0, 0, 0, 1];
function RX(a) { const c = Math.cos(a), s = Math.sin(a); return [1, 0, 0, 0, 0, c, s, 0, 0, -s, c, 0, 0, 0, 0, 1]; }
function RY(a) { const c = Math.cos(a), s = Math.sin(a); return [c, 0, -s, 0, 0, 1, 0, 0, s, 0, c, 0, 0, 0, 0, 1]; }
function RZ(a) { const c = Math.cos(a), s = Math.sin(a); return [c, s, 0, 0, -s, c, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]; }
let OX = 0, OY = 0;
function proj(M, x, y) { const wx = M[0] * x + M[4] * y + M[12], wy = M[1] * x + M[5] * y + M[13], wz = M[2] * x + M[6] * y + M[14]; const s = PD / (PD - wz); return [OX + (wx - OX) * s, OY + (wy - OY) * s]; }
function aff(M, x, y, k) { const a = proj(M, x, y), b = proj(M, x + 100, y), c = proj(M, x, y + 100); return [(b[0] - a[0]) / 100 * k, (b[1] - a[1]) / 100 * k, (c[0] - a[0]) / 100 * k, (c[1] - a[1]) / 100 * k, a[0], a[1]]; }

// chains of first-order smoothers: retargetable at any moment, speed carries over, no overshoot
const mk = (n, v) => ({ x: new Array(n).fill(v || 0) });
const val = c => c.x[c.x.length - 1];
const setC = (c, v) => c.x.fill(v);
function stepC(c, target, w, h, vmax) {
  const k = 1 - Math.exp(-w * h);
  let d = (target - c.x[0]) * k;
  if (vmax) d = clamp(d, -vmax * h, vmax * h);
  c.x[0] += d;
  for (let i = 1; i < c.x.length; i++) c.x[i] += (c.x[i - 1] - c.x[i]) * k;
}

// ---------- WebGL ----------
const cv = document.getElementById('gl');
const ui = document.getElementById('ui');
const gl = cv.getContext('webgl2', { antialias: false, alpha: true, premultipliedAlpha: true, preserveDrawingBuffer: false });
if (!gl) { console.error('Skip Dat needs WebGL 2'); return; }

const VS_FULL = `#version 300 es
in vec2 aP; void main(){ gl_Position = vec4(aP, 0.0, 1.0); }`;
// the desktop snapshot goes in with alpha 0, so alpha ends up holding only what we drew on top
const FS_BG = `#version 300 es
precision highp float; out vec4 o; uniform sampler2D uSnap; uniform vec2 uFb;
void main(){ vec2 uv = vec2(gl_FragCoord.x / uFb.x, 1.0 - gl_FragCoord.y / uFb.y); o = vec4(texture(uSnap, uv).rgb, 0.0); }`;
// take the desktop back out: what remains is exactly our cards (premultiplied) over the real screen
const FS_PRESENT = `#version 300 es
precision highp float; out vec4 o; uniform sampler2D uComp, uSnap; uniform vec2 uFb;
void main(){
  vec4 c = texelFetch(uComp, ivec2(gl_FragCoord.xy), 0);
  vec2 uv = vec2(gl_FragCoord.x / uFb.x, 1.0 - gl_FragCoord.y / uFb.y);
  vec3 s = texture(uSnap, uv).rgb;
  vec3 rgb = min(clamp(c.rgb - s * (1.0 - c.a), 0.0, 1.0), vec3(c.a));
  o = vec4(rgb, c.a);
}`;
const VS_Q = `#version 300 es
in vec2 aP;
uniform mat4 uM; uniform vec2 uSize, uOff, uO, uRes; uniform float uPd;
out vec2 vL;
void main(){
  vec2 l = uOff + aP * uSize; vL = l;
  vec4 w = uM * vec4(l, 0.0, 1.0);
  float s = uPd / (uPd - w.z);
  vec2 sc = uO + (w.xy - uO) * s;
  vec2 n = vec2(sc.x / uRes.x * 2.0 - 1.0, 1.0 - sc.y / uRes.y * 2.0);
  float q = 1.0 / s;
  gl_Position = vec4(n * q, 0.0, q);
}`;
const FS_Q = `#version 300 es
precision highp float;
in vec2 vL; out vec4 o;
uniform int uMode; uniform vec2 uHalf, uFb, uSOff, uLight; uniform float uRad, uA, uLod, uDim, uSBlur, uSA, uSheen, uSpec, uFrost, uDpr, uSdiv;
uniform sampler2D uTex; uniform mat2 uAx; uniform vec4 uBand; uniform vec2 uCrop;
float sd(vec2 p){ float r = min(uRad, min(uHalf.x, uHalf.y)); vec2 q = abs(p) - uHalf + r; return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r; }
vec2 nrm(vec2 p){ vec2 e = vec2(0.75, 0.0); return normalize(vec2(sd(p + e.xy) - sd(p - e.xy), sd(p + e.yx) - sd(p - e.yx)) + 1e-6); }
vec3 frost(vec2 uv, float L){ vec2 t = exp2(L) * uSdiv * 0.5 / uFb;
  return 0.25 * (textureLod(uTex, uv + vec2(t.x, t.y), L).rgb + textureLod(uTex, uv + vec2(-t.x, t.y), L).rgb
               + textureLod(uTex, uv + vec2(t.x, -t.y), L).rgb + textureLod(uTex, uv - t, L).rgb); }
void main(){
  if (uMode == 0) {   // soft shadow
    float d = sd(vL - uSOff);
    o = vec4(0.0, 0.0, 0.0, uSA * uA * (1.0 - smoothstep(-uSBlur * 0.6, uSBlur, d))); return;
  }
  float d = sd(vL);
  float cov = clamp(0.5 - d / max(fwidth(d), 0.5), 0.0, 1.0);
  if (cov <= 0.0) discard;
  vec2 n = nrm(vL);
  float facing = dot(n, uLight), e = -d;
  float g = dot(vL / length(uHalf), vec2(0.7071));
  if (uMode == 1) {   // album art
    vec3 c = texture(uTex, vL / (2.0 * uHalf) * uCrop + 0.5, uLod).rgb * (1.0 - uDim);   // uCrop: centre square of wide pictures
    float rim = 1.0 - smoothstep(0.0, 1.6, e);
    c += rim * (0.08 + 0.32 * max(facing, 0.0));
    c += exp(-pow((g - uSheen) * 3.0, 2.0)) * 0.045;
    o = vec4(c, cov * uA); return;
  }
  // liquid glass: lensing bevel, slight magnification, chromatic edge, adaptive frost behind text
  float B = 26.0, k = clamp(1.0 - e / B, 0.0, 1.0), bend = k * k * k;
  vec2 s = uAx * (n * bend * 34.0 - vL * 0.035);
  vec2 off = vec2(s.x, -s.y) * uDpr;
  vec2 uv = (gl_FragCoord.xy + off) / uFb;
  float band = smoothstep(uBand.x - uBand.z, uBand.x + uBand.z, vL.y) * (1.0 - smoothstep(uBand.y - uBand.z, uBand.y + uBand.z, vL.y));
  float L = max(mix(uFrost, uBand.w, band) + bend * 1.2 + log2(uDpr) - log2(uSdiv), 0.0);
  vec2 ca = off / uFb * 0.2 * bend;
  vec3 c = vec3(frost(uv + ca, L).r, frost(uv, L).g, frost(uv - ca, L).b);
  float lum = dot(c, vec3(0.2126, 0.7152, 0.0722));
  c *= 1.0 - band * mix(0.14, 0.46, smoothstep(0.3, 0.85, lum));
  c *= 1.0 - 0.10 * smoothstep(0.55, 0.95, lum);           // very bright desktops: keep white text readable
  c = mix(c, vec3(1.0), 0.06) * 1.02;
  float rim = 1.0 - smoothstep(0.0, 2.2, e);
  float spec = rim * (0.2 + 0.75 * pow(max(facing, 0.0), 2.0) + 0.3 * pow(max(-facing, 0.0), 2.0));
  float bev = pow(k, 4.0) * pow(max(facing, 0.0), 2.0) * 0.22;
  float inner = (1.0 - smoothstep(0.0, B, e)) * pow(max(-facing, 0.0), 1.5) * 0.1;
  float sheen = exp(-pow((g - uSheen) * 3.2, 2.0)) * 0.07;
  c = (c * (1.0 - inner) + vec3((spec + bev) * uSpec + sheen)) * (1.0 - uDim);
  o = vec4(c, cov * uA);
}`;
function shader(type, src) { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; }
function program(vs, fs) {
  const pr = gl.createProgram(); gl.attachShader(pr, shader(gl.VERTEX_SHADER, vs)); gl.attachShader(pr, shader(gl.FRAGMENT_SHADER, fs));
  gl.bindAttribLocation(pr, 0, 'aP'); gl.linkProgram(pr);
  if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(pr));
  const u = {}; const n = gl.getProgramParameter(pr, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const nm = gl.getActiveUniform(pr, i).name; u[nm] = gl.getUniformLocation(pr, nm); }
  return { pr, u: new Proxy(u, { get: (o, k) => o[k] || null }) };
}
function vao(data) { const v = gl.createVertexArray(); gl.bindVertexArray(v); const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(data), gl.STATIC_DRAW); gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0); return v; }
function texParams(mip) { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, mip ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); }
const PBG = program(VS_FULL, FS_BG), PPR = program(VS_FULL, FS_PRESENT), PQ = program(VS_Q, FS_Q);
const vFull = vao([-1, -1, 3, -1, -1, 3]), vQ = vao([-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5]);

function solidTex(rgba) { const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(rgba)); texParams(false); return t; }
const tSnapFallback = solidTex([38, 44, 56, 255]);
let tSnap = tSnapFallback;

// songs with no picture at all get a plain tile (never another song's art)
const tNoArt = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 256; const x = c.getContext('2d');
  const g = x.createLinearGradient(0, 0, 256, 256); g.addColorStop(0, '#3b4a66'); g.addColorStop(1, '#1d2433');
  x.fillStyle = g; x.fillRect(0, 0, 256, 256);
  x.fillStyle = 'rgba(250,251,253,.55)'; x.beginPath(); x.arc(112, 168, 26, 0, Math.PI * 2); x.fill();
  x.fillRect(130, 70, 10, 100); x.fillRect(130, 70, 52, 12);
  return texFromSource(c);
})();
function texFromSource(src) { const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, src); texParams(true); gl.generateMipmap(gl.TEXTURE_2D); return t; }
// album art by id: loaded once, kept for the most recent 160 pictures
const artCache = new Map();
function artEntry(id) {
  if (!id) return null;
  let e = artCache.get(id);
  if (e) { artCache.delete(id); artCache.set(id, e); return e; }   // most recently used last
  e = { tex: null, crop: [1, 1], t0: 0, failed: false };
  artCache.set(id, e);
  const im = new Image();
  im.crossOrigin = 'anonymous';
  im.onload = () => {
    if (!artCache.has(id)) return;
    try {
      e.tex = texFromSource(im);
      const a = (im.naturalWidth || 1) / (im.naturalHeight || 1);
      e.crop = a > 1 ? [1 / a, 1] : [1, a];                              // wide video stills: the middle square
      e.t0 = performance.now() / 1000;
    } catch (err) { e.failed = true; }
  };
  im.onerror = () => { e.failed = true; };
  im.src = artUrl(id);
  while (artCache.size > 160) {
    const [k, old] = artCache.entries().next().value;
    if (old && old.tex) gl.deleteTexture(old.tex);
    artCache.delete(k);
  }
  return e;
}

let W = 0, H = 0, dpr = 1, sdiv = 2, sw = 1, sh = 1;
let tComp = null, tScene = null;
const fbComp = gl.createFramebuffer(), fbScene = gl.createFramebuffer();
function resize() {
  dpr = window.devicePixelRatio || 1; W = innerWidth; H = innerHeight;
  cv.width = Math.max(2, Math.round(W * dpr)); cv.height = Math.max(2, Math.round(H * dpr));
  if (tComp) gl.deleteTexture(tComp);
  tComp = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tComp); gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, cv.width, cv.height); texParams(false);
  gl.bindFramebuffer(gl.FRAMEBUFFER, fbComp); gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tComp, 0);
  sdiv = dpr >= 1.5 ? 4 : 2;
  sw = Math.max(1, Math.round(cv.width / sdiv)); sh = Math.max(1, Math.round(cv.height / sdiv));
  if (tScene) gl.deleteTexture(tScene);
  tScene = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tScene);
  gl.texStorage2D(gl.TEXTURE_2D, Math.floor(Math.log2(Math.max(sw, sh))) + 1, gl.RGBA8, sw, sh); texParams(true);
  gl.bindFramebuffer(gl.FRAMEBUFFER, fbScene); gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tScene, 0);
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
}

function draw(F) {
  const fw = cv.width, fh = cv.height;
  gl.viewport(0, 0, fw, fh); gl.disable(gl.DEPTH_TEST); gl.disable(gl.BLEND);
  gl.bindFramebuffer(gl.FRAMEBUFFER, fbComp);
  gl.useProgram(PBG.pr); gl.bindVertexArray(vFull);
  gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tSnap);
  gl.uniform1i(PBG.u.uSnap, 0); gl.uniform2f(PBG.u.uFb, fw, fh);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  gl.enable(gl.BLEND); gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  const q = PQ.u;
  gl.useProgram(PQ.pr); gl.bindVertexArray(vQ);
  gl.uniform1f(q.uPd, PD); gl.uniform2f(q.uO, OX, OY); gl.uniform2f(q.uRes, W, H); gl.uniform2f(q.uFb, fw, fh);
  gl.uniform1f(q.uDpr, dpr); gl.uniform1f(q.uSdiv, sdiv); gl.uniform1i(q.uTex, 0);
  for (const it of F.items) {
    if (it.k === 'copy') {
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, fbComp); gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, fbScene);
      gl.blitFramebuffer(0, 0, fw, fh, 0, 0, sw, sh, gl.COLOR_BUFFER_BIT, gl.LINEAR);
      gl.bindFramebuffer(gl.FRAMEBUFFER, fbComp);
      gl.bindTexture(gl.TEXTURE_2D, tScene); gl.generateMipmap(gl.TEXTURE_2D);
      continue;
    }
    const mode = it.k === 'shadow' ? 0 : it.k === 'cover' ? 1 : 2;
    const pad = mode === 0 ? it.blur * 1.3 + 4 : 2;
    gl.uniformMatrix4fv(q.uM, false, it.M);
    gl.uniform2f(q.uSize, 2 * (it.half[0] + pad), 2 * (it.half[1] + pad));
    gl.uniform2f(q.uOff, mode === 0 ? it.off[0] : 0, mode === 0 ? it.off[1] : 0);
    gl.uniform1i(q.uMode, mode); gl.uniform2f(q.uHalf, it.half[0], it.half[1]);
    gl.uniform1f(q.uRad, it.rad); gl.uniform1f(q.uA, it.a);
    if (mode === 0) { gl.uniform2f(q.uSOff, it.off[0], it.off[1]); gl.uniform1f(q.uSBlur, it.blur); gl.uniform1f(q.uSA, 1); }
    else {
      gl.uniform2f(q.uLight, it.light[0], it.light[1]); gl.uniform1f(q.uSheen, it.sheen); gl.uniform1f(q.uDim, it.dim);
      gl.bindTexture(gl.TEXTURE_2D, mode === 1 ? it.tex : tScene);
      if (mode === 1) { gl.uniform1f(q.uLod, it.lod); gl.uniform2f(q.uCrop, it.crop ? it.crop[0] : 1, it.crop ? it.crop[1] : 1); }
      else {
        gl.uniformMatrix2fv(q.uAx, false, it.ax.slice(0, 4)); gl.uniform4f(q.uBand, it.band[0], it.band[1], it.band[2], it.band[3]);
        gl.uniform1f(q.uFrost, it.frost); gl.uniform1f(q.uSpec, it.spec);
      }
    }
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }
  gl.disable(gl.BLEND);
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  gl.useProgram(PPR.pr); gl.bindVertexArray(vFull);
  gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tComp);
  gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, tSnap);
  gl.uniform1i(PPR.u.uComp, 0); gl.uniform1i(PPR.u.uSnap, 1); gl.uniform2f(PPR.u.uFb, fw, fh);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  gl.activeTexture(gl.TEXTURE0);
}
function clearScreen() { gl.bindFramebuffer(gl.FRAMEBUFFER, null); gl.viewport(0, 0, cv.width, cv.height); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); }

// ---------- sources: each a column of its own songs ----------
// The source you are browsing stands in front with its whole wheel. Switching (rail, Tab, side
// buttons, tilting the wheel) slides the row sideways.
const COLW = 300;
const NONE_SRC = { key: 'none', name: '', via: '', playing: false, queue: 'none', can_jump: false,
  items: [{ key: 'none', rel: 0, title: 'Nothing playing', artist: 'Start a song in any music app', art: null }] };
let V = { active: 'none', sources: [] };
let pendingV = null;
let cols = [];
let dirty = false;          // the person scrolled to a song (releasing plays it)
let userPickedApp = false;  // the person switched source with the keyboard or wheel tilt
let gen = 0;                // which opening of the window this is (from the app)
const eqEl = document.createElement('div'); eqEl.className = 'eq'; eqEl.innerHTML = '<i></i><i></i><i></i><i></i>';
const bars = [...eqEl.children].map(e => e.style);

function lineFor(item, src) {
  const title = item.title || (item.rel === 0 ? (src.name || 'Playing') : 'Untitled');
  let artist = item.artist || (item.rel === 0 && src.via ? 'in ' + src.via : '');
  if (item.rel === 0 && src.key !== 'none' && !src.playing) artist = artist ? artist + ' · Paused' : 'Paused';
  return [title, artist];
}
function setWords(w, item, src) {
  const [title, artist] = lineFor(item, src);
  if (w.title !== title) { w.title = title; w.tEl.textContent = title; }
  if (w.artist !== artist) { w.artist = artist; w.aEl.textContent = artist; }
}
function wordsFor(col, item) {
  let w = col.words.get(item.key);
  if (!w) {
    const t = document.createElement('p'); t.className = 't t-title';
    const a = document.createElement('p'); a.className = 't t-artist';
    ui.append(t, a);
    w = { tEl: t, aEl: a, t: t.style, a: a.style, shown: false, title: null, artist: null };
    col.words.set(item.key, w);
  }
  setWords(w, item, col.src);
  return w;
}
function hideWords(w) { if (w.shown) { w.t.visibility = w.a.visibility = 'hidden'; w.shown = false; } }
function dropCol(c) { c.words.forEach(w => { w.tEl.remove(); w.aEl.remove(); }); c.words.clear(); }

function buildCol(src, old) {
  const items = (src.items || []).filter(i => i && typeof i.rel === 'number').slice().sort((a, b) => a.rel - b.rel);
  if (!items.length) items.push({ key: 'now', rel: 0, title: '', artist: '', art: null });
  const c = old || { key: src.key, tgt: 0, wheel: mk(3, 0), hist: [], p: 0, vel: 0, words: new Map(), items: [] };
  if (old && old.items.length && state !== 'hidden') {
    // the list moved (a skip, songs added before this one): stay on the same song
    const anchor = old.byRel.get(clamp(Math.round(c.tgt), c.relMin, c.relMax));
    const now = anchor && items.find(i => i.key === anchor.key);
    const shift = now ? now.rel - anchor.rel : 0;
    if (shift) { c.tgt += shift; c.wheel.x = c.wheel.x.map(x => x + shift); c.hist.forEach(h => { h[1] += shift; }); c.p += shift; }
  }
  c.src = src; c.items = items;
  c.byRel = new Map(items.map(i => [i.rel, i]));
  c.relByKey = new Map(items.map(i => [i.key, i.rel]));
  c.relMin = items[0].rel; c.relMax = items[items.length - 1].rel;
  if (!dirty || state === 'hidden') c.tgt = 0;                       // follow the player
  c.tgt = clamp(c.tgt, c.relMin, c.relMax);
  const keys = new Set(items.map(i => i.key));
  for (const [k, w] of c.words) if (!keys.has(k)) { w.tEl.remove(); w.aEl.remove(); c.words.delete(k); }
  for (const it of items) { const w = c.words.get(it.key); if (w) setWords(w, it, src); }
  return c;
}
function applyView(v) {
  if (!v || !Array.isArray(v.sources)) return;
  const prevKey = cols.length ? curCol().key : null;
  const old = new Map(cols.map(c => [c.key, c]));
  V = v;
  const list = v.sources.length ? v.sources : [NONE_SRC];
  const next = list.map(s => buildCol(s, old.get(s.key)));
  for (const [k, c] of old) if (!next.some(n => n.key === k)) dropCol(c);
  cols = next;
  if (!eqEl.parentNode) ui.append(eqEl);
  const activeI = Math.max(0, cols.findIndex(c => c.key === v.active));
  // while open, the source you are looking at stays in front (unless it went away)
  let idx = state === 'hidden' || !prevKey ? -1 : cols.findIndex(c => c.key === prevKey);
  if (idx < 0) idx = activeI;
  const shift = idx - Math.round(ai);
  if (state === 'hidden' || presence < 0.2) { ai = idx; setC(hx, idx); }
  else if (shift) { ai += shift; hx.x = hx.x.map(x => x + shift); }
  ai = clamp(Math.round(ai), 0, cols.length - 1);
}

// ---------- the rail (DOM parts; the glass is drawn with the cards) ----------
const ICON = {
  previous: '<svg viewBox="0 0 24 24"><path d="M6 5h2.4v14H6zM18.6 5.7v12.6c0 .7-.8 1.1-1.3.7l-8.6-6.3a.9.9 0 0 1 0-1.4l8.6-6.3c.5-.4 1.3 0 1.3.7z"/></svg>',
  next: '<svg viewBox="0 0 24 24"><path d="M15.6 5H18v14h-2.4zM5.4 5.7v12.6c0 .7.8 1.1 1.3.7l8.6-6.3a.9.9 0 0 0 0-1.4L6.7 5c-.5-.4-1.3 0-1.3.7z"/></svg>',
  play: '<svg viewBox="0 0 24 24"><path d="M7 4.9v14.2c0 .8.9 1.3 1.6.8l11-7.1a1 1 0 0 0 0-1.6l-11-7.1C7.9 3.6 7 4.1 7 4.9z"/></svg>',
  pause: '<svg viewBox="0 0 24 24"><path d="M6.5 4.5h3.6v15H6.5zM13.9 4.5h3.6v15h-3.6z"/></svg>',
};
const chipEls = Array.from({ length: RAIL.maxChips }, () => {
  const el = document.createElement('div'); el.className = 'chip';
  el.innerHTML = '<span class="mark"><i></i><i></i><i></i></span><span class="name"></span>';
  ui.append(el);
  return { el, s: el.style, name: el.querySelector('.name'), mark: el.querySelector('.mark'), bars: [...el.querySelectorAll('.mark i')].map(i => i.style), text: null, on: null, shown: false };
});
const BTNS = ['previous', 'toggle', 'next'];
const btnEls = BTNS.map(id => {
  const el = document.createElement('div'); el.className = 'ctl'; ui.append(el);
  return { id, el, s: el.style, icon: null, shown: false };
});
const noteEl = document.createElement('p'); noteEl.className = 'note'; ui.append(noteEl);
let noteText = null, noteShown = false;
const hov = new Map(), pressAt = new Map();
function hoverOf(id, on, h) { const v = hov.get(id) || 0; const n = v + ((on ? 1 : 0) - v) * (1 - Math.exp(-h * 18)); hov.set(id, n); return n; }
function noteFor(src) {
  if (!src || src.key === 'none' || src.queue !== 'none') return '';
  if (/spotify/i.test(src.key)) return 'Connect Spotify in Settings to see what plays next';
  return (src.name || 'This app') + ' doesn’t share what plays next';
}
function rectOf(M, hw, hh) {
  const pts = [proj(M, -hw, -hh), proj(M, hw, -hh), proj(M, hw, hh), proj(M, -hw, hh)];
  const xs = pts.map(p => p[0]), ys = pts.map(p => p[1]);
  const x0 = Math.min(...xs), y0 = Math.min(...ys);
  return [x0, y0, Math.max(...xs) - x0, Math.max(...ys) - y0];
}
const inRect = (r, p) => p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3];

// clickable areas go to the app, which takes those clicks while the overlay is open
let hitsSent = '', hitsAt = 0;
function sendHits(list, t) {
  const rects = list.map(h => ({ id: h.id, x: Math.round(h.r[0] * dpr), y: Math.round(h.r[1] * dpr), w: Math.round(h.r[2] * dpr), h: Math.round(h.r[3] * dpr) }));
  const key = JSON.stringify(rects);
  if (key === hitsSent) return;
  if (rects.length && t - hitsAt < 0.05) return;                    // at most 20 updates a second
  hitsSent = key; hitsAt = t;
  invoke('set_hit_rects', { gen, rects });
}

// ---------- state ----------
let state = 'hidden', running = false, last = 0, frameErr = 0, lastPrune = 0;
let ai = 0, lastStep = -10, lastAppStep = -10, sel = 0, selCol = 0, pillAt = -10, toggleAt = -10, absorb = null;
const hx = mk(3, 0), pres = mk(3, 0), pill = mk(3, 0), cm = mk(2, 0), sideC = mk(2, 1);
// 0 = vertical (default), 1 = horizontal; eased, so a change of layout while open morphs smoothly
let oriT = 0;
const oriC = mk(2, 0);
const setLayout = (name, snap) => { oriT = name === 'horizontal' ? 1 : 0; if (snap) setC(oriC, oriT); };
const lerpM = (A, B, k) => A.map((v, i) => v + (B[i] - v) * k);
const anc = { sx: mk(2, 0), sy: mk(2, 0), px: mk(2, 0), py: mk(2, 0) };
let presT = 0, pillT = 0, cmT = 0, presence = 0;
let m = [0, 0], lpSlow = [0, 0], home = [0, 0], side = 1, bounds = null;
const nowS = () => performance.now() / 1000;
const curCol = () => cols[clamp(Math.round(ai), 0, cols.length - 1)];
function pAt(col, tt) {
  const h = col.hist; if (!h.length) return col.p;
  if (tt >= h[h.length - 1][0]) return h[h.length - 1][1];
  for (let i = h.length - 1; i > 0; i--) if (h[i - 1][0] <= tt) { const a = h[i - 1], b = h[i]; return a[1] + (b[1] - a[1]) * ((tt - a[0]) / ((b[0] - a[0]) || 1)); }
  return h[0][1];
}
function rubber(x, lo, hi) { const k = 0.3; if (x < lo) return lo - k * (1 - Math.exp((x - lo) / k)); if (x > hi) return hi + k * (1 - Math.exp(-(x - hi) / k)); return x; }
// The wheel settles beside where it opened and stays there, so the mouse can reach its buttons;
// moving well away from it brings it along.
function targets() {
  const reach = (205 + RAIL_X + RAIL.w / 2 + 24) * U;
  side = home[0] + reach < W ? 1 : -1;
  const pill = [clamp(home[0] + side * 158 * U, 140 * U, W - 140 * U), clamp(home[1] - 56 * U, 50 * U, H - 50 * U)];
  const vert = [home[0] + side * 205 * U, clamp(home[1] - 190 * U, 260 * U, H - 260 * U)];
  // horizontal: centred over the cursor, above it (below it when there is no room above)
  const above = home[1] - (CARD[1] + 46 + 20 + 58) * U;
  const roomAbove = above - (CARD[1] + 44 + 40) * U > 8;
  const hy = roomAbove ? above : home[1] + (CARD[1] + 44 + 40 + 22) * U;
  const hor = [clamp(home[0], 330 * U, W - 330 * U), clamp(hy, (CARD[1] + 84) * U, H - (CARD[1] + 120) * U)];
  return [lerp(vert[0], hor[0], oriT), lerp(vert[1], hor[1], oriT), pill[0], pill[1]];
}
function snapAnchors() { const [sx, sy, px, py] = targets(); setC(anc.sx, sx); setC(anc.sy, sy); setC(anc.px, px); setC(anc.py, py); setC(sideC, side); setC(oriC, oriT); }
function startLoop() { if (!running) { running = true; last = performance.now(); requestAnimationFrame(frame); } }
function resetWheels() { cols.forEach(c => { c.tgt = 0; setC(c.wheel, 0); c.hist.length = 0; c.p = 0; c.vel = 0; }); }

function resumeBrowse() {
  if (state === 'browse') return;
  if (pendingV) { const v = pendingV; pendingV = null; state = 'browse'; applyView(v); }
  if (absorb) { setC(pres, presence); absorb = null; }
  state = 'browse'; presT = 1; pillT = 0; cmT = 0;
}
function startAbsorb() {
  if (state === 'absorb' || state === 'hidden') return;
  absorb = { t0: nowS(), from: presence };
  state = 'absorb'; cmT = 1;
}
function resetHidden() {
  state = 'hidden'; absorb = null; presence = 0; dirty = false; userPickedApp = false;
  clearScreen();
  cols.forEach(c => c.words.forEach(hideWords));
  chipEls.forEach(c => { c.s.visibility = 'hidden'; c.shown = false; });
  btnEls.forEach(b => { b.s.visibility = 'hidden'; b.shown = false; });
  noteEl.style.visibility = 'hidden'; noteShown = false;
  eqEl.style.opacity = 0;
  tSnap = tSnapFallback;
  hitsSent = '';
  if (pendingV) { const v = pendingV; pendingV = null; applyView(v); }
}
function finishHide() { resetHidden(); invoke('overlay_done', { gen }); }

function curSrc() { const c = cols.length ? curCol() : null; return c && c.key !== 'none' ? c : null; }
function control(action) {
  const c = curSrc(); if (!c) return;
  invoke('media_control', { source: c.key, action });
  toggleAt = nowS(); pressAt.set(action, nowS());
  if (action !== 'toggle') { dirty = false; c.tgt = 0; }
}
function playItem(c, item) {
  if (item.rel === 0) { control('toggle'); return; }
  invoke('choose', { source: c.key, item: item.key, rel: item.rel });
  c.tgt = item.rel; dirty = false; toggleAt = nowS();
}

// ---------- events from the app ----------
listen('lg-show', d => {
  gen = d.gen || gen;
  if (d.layout) setLayout(d.layout, state === 'hidden');
  if (innerWidth !== W || innerHeight !== H || (window.devicePixelRatio || 1) !== dpr) resize();
  m = [d.x / dpr, d.y / dpr];
  if (state === 'hidden') {
    lpSlow = m.slice(); home = m.slice();
    dirty = false; userPickedApp = false; frameErr = 0;
    resetWheels();
    const activeI = Math.max(0, cols.findIndex(c => c.key === V.active));
    ai = activeI; setC(hx, activeI);
    setC(pres, 0); setC(pill, 0); setC(cm, 0); presT = 1; pillT = 0; cmT = 0; absorb = null;
    snapAnchors();
    state = 'browse';
    startLoop();
  } else resumeBrowse();
});
listen('lg-cursor', d => { m = [d.x / dpr, d.y / dpr]; });
function wheelStep(steps) {
  if (state === 'hidden' || !cols.length) return;
  resumeBrowse();
  const c = curCol();
  c.tgt = clamp(Math.round(c.tgt) + steps, c.relMin - 1, c.relMax + 1);   // exactly one song per notch
  lastStep = nowS(); dirty = true;
}
listen('lg-wheel', wheelStep);
function switchTo(i, viaKeys) {
  const before = curCol();
  ai = i;
  lastAppStep = nowS();
  if (curCol() !== before) { before.tgt = 0; curCol().tgt = 0; }        // the source you left rolls back to "now"
  if (viaKeys) { userPickedApp = true; dirty = true; } else dirty = false;
}
function appStep(steps) {
  if (state === 'hidden' || !cols.length) return;
  resumeBrowse();
  switchTo(clamp(Math.round(ai) + steps, -1, cols.length), true);       // one source per step, a soft bump at the ends
}
listen('lg-app', appStep);
// arrow keys follow the layout: along the wheel moves between songs, across it between sources
listen('lg-arrow', dir => {
  const along = oriT > 0.5 ? { left: -1, right: 1 } : { up: -1, down: 1 };
  const across = oriT > 0.5 ? { up: -1, down: 1 } : { left: -1, right: 1 };
  if (along[dir]) wheelStep(along[dir]); else if (across[dir]) appStep(across[dir]);
});
listen('lg-layout', name => setLayout(name, state === 'hidden'));
listen('lg-click', id => {
  if (state !== 'browse' || typeof id !== 'string') return;
  if (id.startsWith('ctl:')) control(id.slice(4));
  else if (id.startsWith('src:')) { const i = cols.findIndex(c => c.key === id.slice(4)); if (i >= 0) switchTo(i, false); }
  else if (id.startsWith('card:')) {
    const k = id.slice(5), cut = k.lastIndexOf('|');
    const c = cols.find(x => x.key === k.slice(0, cut)), item = c && c.items.find(i => i.key === k.slice(cut + 1));
    if (c && item && c.key !== 'none') playItem(c, item);
  }
});
listen('lg-commit', () => {
  if (state !== 'browse' || !cols.length) return;
  ai = clamp(Math.round(ai), 0, cols.length - 1);
  selCol = ai;
  const c = cols[selCol];
  sel = clamp(Math.round(c.tgt), c.relMin, c.relMax); c.tgt = sel;
  const item = c.byRel.get(sel);
  const othersPlaying = cols.some(o => o !== c && o.src.playing);
  // only an actual choice plays something; just looking (or using the buttons) closes quietly
  if (c.key === 'none' || !item || !(dirty || userPickedApp) || (sel === 0 && c.src.playing && !othersPlaying)) { startAbsorb(); return; }
  invoke('choose', { source: c.key, item: item.key, rel: sel });
  state = 'commit';
});
listen('lg-cancel', () => startAbsorb());
listen('lg-force-hide', () => { if (state !== 'hidden') resetHidden(); });
listen('lg-toggle', () => { if (state !== 'hidden') control('toggle'); });
listen('lg-media', v => { if (state === 'browse' || state === 'hidden') applyView(v); else pendingV = v; });
listen('lg-snapshot', async () => {
  try {
    const buf = await invoke('get_snapshot');
    if (!buf || buf.byteLength < 9) return;
    const dv = new DataView(buf), w = dv.getUint32(0, true), h = dv.getUint32(4, true);
    const px = new Uint8Array(buf, 8, w * h * 4);
    const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, px); texParams(false);
    if (tSnap !== tSnapFallback) gl.deleteTexture(tSnap);
    tSnap = t;
  } catch (err) { console.warn('snapshot', err); }
});
invoke('get_view').then(v => { if (v) applyView(v); });
applyView({ active: 'none', sources: [] });
addEventListener('resize', () => { resize(); });

// ---------- frame ----------
function frame(nowMs) {
  if (state === 'hidden') { running = false; return; }
  requestAnimationFrame(frame);
  try { render(nowMs); }
  catch (err) { if (frameErr++ < 3) console.error('[Skip Dat] frame', err); }   // keep animating; the app closes us if it must
}

function coverItems(M, half, rad, a, art, lod, dim, light, sheen, t) {
  const e = artEntry(art);
  const base = { k: 'cover', M, half, rad, a, lod, dim, light, sheen };
  if (!e || !e.tex) return [{ ...base, tex: tNoArt }];
  const fade = smoothstep(0, 0.22, t - e.t0);                         // a picture that just arrived fades in
  const out = [];
  if (fade < 1) out.push({ ...base, tex: tNoArt });
  out.push({ ...base, a: a * fade, tex: e.tex, crop: e.crop });
  return out;
}

function render(nowMs) {
  if (innerWidth !== W || innerHeight !== H) resize();
  const dt = Math.min(0.1, Math.max(0, (nowMs - last) / 1000)); last = nowMs;   // a slow frame still moves (in small steps)
  const t = nowMs / 1000;
  const n = cols.length;
  if (state === 'browse' && t - lastAppStep > 0.16) ai = clamp(Math.round(ai), 0, n - 1);   // ease back from the ends
  const cI = clamp(Math.round(ai), 0, n - 1);
  cols.forEach((c, j) => {
    if (j !== cI && state === 'browse') c.tgt = 0;
    if (state === 'browse' && t - lastStep > 0.16) c.tgt = clamp(c.tgt, c.relMin, c.relMax);
  });
  // stay put while the mouse heads for the wheel; follow when it goes well away
  if (state === 'browse' && bounds && !inRect([bounds[0] - 150, bounds[1] - 150, bounds[2] + 300, bounds[3] + 300], m)) home = m.slice();
  const [sx, sy, px, py] = targets();
  let rem = dt;
  while (rem > 1e-6) {
    const h = Math.min(rem, 1 / 240); rem -= h;
    cols.forEach(c => stepC(c.wheel, rubber(c.tgt, c.relMin, c.relMax), FEEL.wheelOmega, h, FEEL.topSpeed));
    stepC(hx, rubber(ai, 0, n - 1), 16, h, 12);
    stepC(pres, presT, 12, h); stepC(pill, pillT, 9, h); stepC(cm, cmT, 14, h); stepC(sideC, side, 10, h); stepC(oriC, oriT, 7, h);
    stepC(anc.sx, sx, 9, h); stepC(anc.sy, sy, 9, h); stepC(anc.px, px, 9, h); stepC(anc.py, py, 9, h);
    const a2 = 1 - Math.exp(-h / 0.45);
    lpSlow[0] += (m[0] - lpSlow[0]) * a2; lpSlow[1] += (m[1] - lpSlow[1]) * a2;
  }
  cols.forEach(c => {
    const prev = c.p; c.p = val(c.wheel);
    if (dt > 0) c.vel += ((c.p - prev) / dt - c.vel) * (1 - Math.exp(-dt / 0.05));
    c.hist.push([t, c.p]); while (c.hist.length && c.hist[0][0] < t - 0.5) c.hist.shift();
  });
  const hxv = val(hx);
  const mp = clamp01(val(pill)), cc = clamp01(val(cm));
  if (state === 'commit') {
    const c = cols[selCol];
    if (!c || (Math.abs(c.p - sel) < 0.06 && Math.abs(c.vel) < 1 && Math.abs(hxv - selCol) < 0.05)) { state = 'pill'; pillT = 1; cmT = 1; pillAt = t; }
  }
  if (state === 'pill' && t - pillAt > FEEL.pillHold) startAbsorb();
  if (absorb) {
    const qa = clamp01((t - absorb.t0) / FEEL.absorb);
    presence = absorb.from * (1 - qa * qa * qa);   // accelerates into the cursor
    if (qa >= 1) { finishHide(); return; }
  } else presence = clamp01(val(pres));

  const fc = cols[cI] || null;
  const speed = fc ? Math.abs(fc.vel) : 0, fast = smoothstep(4, 10, speed);
  const pulse = smoothstep(0, 0.1, t - toggleAt) * (1 - smoothstep(0.15, 0.5, t - toggleAt))
              + smoothstep(0, 0.1, t - pillAt) * (1 - smoothstep(0.15, 0.5, t - pillAt));
  const sweep = clamp01((t - pillAt) / 0.55);
  // emerge from / disappear into the cursor tip, stretching slightly toward it on the way
  const baseX = lerp(val(anc.sx), val(anc.px), mp), baseY = lerp(val(anc.sy), val(anc.py), mp);
  const ax = lerp(m[0], baseX, presence) + 5 * Math.sin(t * 0.85 + 1) * (1 - mp) * FEEL.float * presence;
  const ay = lerp(m[1], baseY, presence) + 7 * Math.sin(t * 1.2) * (1 - mp) * FEEL.float * presence;
  OX = ax; OY = ay;
  const th = Math.atan2(m[1] - ay, m[0] - ax), kS = 4 * presence * (1 - presence) * 0.14;
  const tf = 1 - 0.6 * mp;
  const ry = clamp((m[0] - lpSlow[0]) * 0.04, -3, 3) * tf * DEG;
  const rx = clamp(-(m[1] - lpSlow[1]) * 0.04, -2.5, 2.5) * tf * DEG;
  const gs = lerp(0.03, 1, presence) * U;
  const R = chain(Tm(ax, ay, 0), RZ(th), Sm(1 + kS, 1 - kS * 0.6, 1), RZ(-th), RY(ry), RX(rx), Sm(gs, gs, gs));
  const alphaP = smoothstep(0, 0.3, presence);
  const Lgt = norm([-0.5 + ry * 0.9, -0.86 - rx * 0.9]);
  const sheen0 = -0.85 + (m[0] - lpSlow[0]) * 0.004 + ry * 1.2;
  const browsing = state === 'browse';
  const oriV = clamp01(val(oriC)), eo = oriV * oriV * (3 - 2 * oriV);   // eased layout blend
  const onlyV = eo < 0.001, onlyH = eo > 0.999;

  const drawn = [];
  cols.forEach((col, j) => {
    const dj = j - hxv, adj = Math.abs(dj);
    if (adj > 0.9) return;
    const chosen = j === selCol && (state === 'commit' || state === 'pill' || state === 'absorb');
    const colW = 1 - smoothstep(0.15, 0.6, adj);                       // 1 for the source you are on
    const colS = lerp(1, 0.8, Math.min(adj, 1)), colZ = -150 * Math.min(adj, 1.4);
    const colA = (1 - smoothstep(0.25, 0.85, adj)) * (1 - cc * (chosen ? 0 : 1));   // other sources show only while sliding past
    if (colA < 0.003) return;
    const Mcol = chain(R, Tm(dj * COLW * (1 - eo), dj * COLH * eo, colZ), Sm(colS, colS, colS));
    // only the songs near the wheel's position are drawn, however long the queue is
    const lo = Math.max(col.relMin, Math.floor(col.p - 3.4)), hi = Math.min(col.relMax, Math.ceil(col.p + 3.4));
    if (j === cI) for (let r = Math.round(col.p) - 4; r <= Math.round(col.p) + 6; r++) { const it = col.byRel.get(r); if (it && it.art) artEntry(it.art); }
    for (let rel = lo; rel <= hi; rel++) {
      const item = col.byRel.get(rel);
      if (!item) continue;
      const d0 = rel - col.p;
      if (Math.abs(d0) > 3.4) continue;
      const lag = FEEL.trail * Math.min(Math.abs(d0), 2) * (1 - 0.7 * fast);
      const d = rel - pAt(col, t - lag), ad = Math.abs(d);
      const nb = smoothstep(0.3, 0.6, ad), hm = chosen ? mp * (1 - nb) : 0;
      const a = clamp01((2.6 - ad) / 0.9) * alphaP * colA * (rel === 0 ? 1 : colW) * (1 - cc * nb);
      if (a < 0.003) continue;
      const thW = clamp(d, -3.3, 3.3) * WSTEP * (1 - 0.35 * cc * nb);
      const push = 220 * cc * nb;
      const fl = (1 - hm) * FEEL.float * (1 - 0.6 * fast);
      const fx = -Math.sin(d * 0.9) * FEEL.fan * (1 - hm);
      const fy = 6 * Math.sin(t * 1.1 - d * 0.6 - j * 0.8) * fl, fz = 8 * Math.sin(t * 0.9 - d * 0.6 - j * 0.8) * fl;
      let M, z;
      const Mv = onlyH ? null : chain(Mcol, Tm(fx, fy, fz - WR - push), RX(-thW), Tm(0, 0, WR));
      const zv = WR * (Math.cos(thW) - 1) - push + fz + colZ;
      if (onlyV) { M = Mv; z = zv; }
      else {
        // horizontal: cards turn around a vertical axis, the ones beside sitting a little lower
        const thH = clamp(d, -3.3, 3.3) * WSTEP_H * (1 - 0.35 * cc * nb);
        const fyh = 16 * (1 - Math.cos(clamp(d, -3, 3) * 0.5)) * (1 - hm) + fy;
        const Mh = chain(Mcol, Tm(0, fyh, fz - WRH - push), RY(thH), Tm(0, 0, WRH));
        const zh = WRH * (Math.cos(thH) - 1) - push + fz + colZ;
        M = onlyH ? Mh : lerpM(Mv, Mh, eo); z = lerp(zv, zh, eo);
      }
      const half = [lerp(CARD[0], PILL[0], hm), lerp(CARD[1], PILL[1], hm)], rad = lerp(30, 34, hm);
      const ch = lerp(COVER, 24, hm), crad = lerp(14, 11, hm);
      const Mc = chain(M, Tm(lerp(0, -98, hm), lerp(COVER_Y, 0, hm), 6));
      const dim = Math.min(ad * 0.24, 0.5) + 0.3 * (1 - colW) + (col.src.playing || rel !== 0 ? 0 : 0.08);
      const sheen = sheen0 + (hm > 0 ? sweep * 2.6 : 0);
      const cardId = 'card:' + col.key + '|' + item.key;
      const hv = browsing && j === cI ? (hov.get(cardId) || 0) : 0;
      drawn.push({ z, w: wordsFor(col, item), d, ad, a, hm, M, colW, rel, chosen, col, item, cardId, items: [
        { k: 'shadow', M, half, rad, off: [0, 14], blur: 30, a: 0.2 * a },
        { k: 'copy' },
        { k: 'glass', M, half, rad, a, ax: aff(M, 0, 0, 1), light: Lgt, sheen, dim: dim * (1 - 0.5 * hv), spec: 1 + 0.7 * pulse * colW * (1 - nb) + 0.35 * hv,
          band: [lerp(80, -50, hm), lerp(200, 50, hm), 8, lerp(3.8, 3.2, hm)], frost: lerp(1.4, 3.2, hm) + ad * 0.6 + fast * 0.5 + (1 - colW) * 0.6 },
        { k: 'shadow', M: Mc, half: [ch, ch], rad: crad, off: [0, 6], blur: 10, a: 0.28 * a },
        ...coverItems(Mc, [ch, ch], crad, a, item.art, Math.min(ad * 1.1, 2.6) + fast * 1.1 + (1 - colW) * 0.8, dim, Lgt, sheen, t),
      ] });
    }
  });
  drawn.sort((a1, a2) => a1.z - a2.z);
  const items = [];
  if (drawn.length) items.push({ k: 'shadow', M: R, half: [lerp(150, 380, eo), lerp(220, 150, eo)], rad: 80, off: [0, 0], blur: 170, a: 0.10 * alphaP * (1 - mp) });
  drawn.forEach(dd => items.push(...dd.items));

  // ---- the rail: sources and transport, beside the wheel on the side away from the cursor ----
  const hits = [];
  const srcs = V.sources.slice(0, RAIL.maxChips);
  const railA = alphaP * (1 - mp) * (1 - cc) * (srcs.length ? 1 : 0);
  const cur = cols[cI] && cols[cI].key !== 'none' ? cols[cI] : null;
  const note = cur ? noteFor(cur.src) : '';
  const chipsH = srcs.length * RAIL.chipStep - 8, railH = chipsH + 18 + RAIL.btnR * 2 + (note ? 52 : 0);
  const top = -railH / 2;
  const rx0 = val(sideC) * RAIL_X;
  // where each part of the rail sits, in both layouts
  const chipAt = i => [lerp(rx0, (i - (srcs.length - 1) / 2) * HRAIL.chipStep, eo), lerp(top + RAIL.chipH + i * RAIL.chipStep, HRAIL.chipY, eo)];
  const chipHalf = lerp(RAIL.w / 2, HRAIL.chipW / 2, eo);
  const by = lerp(top + chipsH + 18 + RAIL.btnR, HRAIL.btnY, eo);
  const btnAt = k => [lerp(rx0 + (k - 1) * RAIL.btnStep, (k - 1) * HRAIL.btnStep, eo), by];
  const noteAt = [lerp(rx0, 0, eo), by + RAIL.btnR + 30];
  const railItems = [];
  let railDrawn = false;
  if (railA > 0.003) {
    railDrawn = true;
    railItems.push({ k: 'copy' });
    srcs.forEach((s, i) => {
      const [cx, cy] = chipAt(i);
      const id = 'src:' + s.key, M = chain(R, Tm(cx, cy, 0));
      const r = rectOf(M, chipHalf, RAIL.chipH), hv = hoverOf(id, browsing && inRect(r, m), dt);
      const on = cur && s.key === cur.key;
      if (browsing) hits.push({ id, r });
      railItems.push({ k: 'shadow', M, half: [chipHalf, RAIL.chipH], rad: RAIL.chipH, off: [0, 5], blur: 12, a: 0.16 * railA });
      railItems.push({ k: 'glass', M, half: [chipHalf, RAIL.chipH], rad: RAIL.chipH, a: railA, ax: aff(M, 0, 0, 1), light: Lgt, sheen: sheen0,
        dim: (on ? 0 : 0.34) * (1 - hv), spec: (on ? 1.45 : 0.8) + 0.4 * hv, band: [-RAIL.chipH, RAIL.chipH, 4, 3.4], frost: 1.8 });
      const ce = chipEls[i];
      if (ce.text !== s.name) { ce.text = s.name; ce.name.textContent = s.name; }
      const mode = on ? (s.playing ? 'eq' : 'dot') : (s.playing ? 'dim' : 'off');
      if (ce.on !== mode) { ce.on = mode; ce.el.className = 'chip ' + mode; }
      if (!ce.shown) { ce.s.visibility = 'visible'; ce.shown = true; }
      ce.s.opacity = railA * (on ? 1 : lerp(0.6, 0.9, hv));
      const tw = Math.round(chipHalf * 2 - 22);
      if (ce.w !== tw) { ce.w = tw; ce.s.width = tw + 'px'; }
      ce.s.transform = 'matrix(' + aff(M, -chipHalf + 12, -10, 1).join(',') + ')';
      if (mode === 'eq') ce.bars.forEach((b, k) => { b.height = (3 + 7 * (0.5 + 0.5 * Math.sin(t * 7 + k * 2.1))) + 'px'; });
    });
    BTNS.forEach((id, k) => {
      const [bx, byy] = btnAt(k);
      const M = chain(R, Tm(bx, byy, 0)), hid = 'ctl:' + id;
      const r = rectOf(M, RAIL.btnR, RAIL.btnR), hv = hoverOf(hid, browsing && inRect(r, m), dt);
      const pr = pressAt.get(id) || -10, flash = smoothstep(0, 0.06, t - pr) * (1 - smoothstep(0.1, 0.45, t - pr));
      if (browsing && cur) hits.push({ id: hid, r });
      const big = id === 'toggle' ? 1.08 : 1;
      railItems.push({ k: 'shadow', M, half: [RAIL.btnR * big, RAIL.btnR * big], rad: RAIL.btnR * big, off: [0, 5], blur: 12, a: 0.16 * railA });
      railItems.push({ k: 'glass', M, half: [RAIL.btnR * big, RAIL.btnR * big], rad: RAIL.btnR * big, a: railA * (cur ? 1 : 0.5), ax: aff(M, 0, 0, 1), light: Lgt, sheen: sheen0,
        dim: 0.08 * (1 - hv), spec: 1 + 0.45 * hv + 0.9 * flash, band: [1e4, 1e4, 1, 0], frost: 1.6 });
      const b = btnEls[k];
      const icon = id === 'toggle' ? (cur && cur.src.playing ? 'pause' : 'play') : id;
      if (b.icon !== icon) { b.icon = icon; b.el.innerHTML = ICON[icon]; }
      if (!b.shown) { b.s.visibility = 'visible'; b.shown = true; }
      b.s.opacity = railA * (cur ? lerp(0.86, 1, hv) : 0.4);
      b.s.transform = 'matrix(' + aff(M, -9 * big, -9 * big, big).join(',') + ')';
    });
    if (note) {
      // on its own frosted backing so it reads on any wallpaper
      const Mn = chain(R, Tm(noteAt[0], noteAt[1], 0));
      railItems.push({ k: 'glass', M: Mn, half: [RAIL.w / 2, 19], rad: 12, a: railA, ax: aff(Mn, 0, 0, 1), light: Lgt, sheen: sheen0,
        dim: 0.3, spec: 0.6, band: [-19, 19, 4, 3.8], frost: 2.4 });
      if (noteText !== note) { noteText = note; noteEl.textContent = note; }
      if (!noteShown) { noteEl.style.visibility = 'visible'; noteShown = true; }
      noteEl.style.opacity = railA * 0.92;
      noteEl.style.transform = 'matrix(' + aff(Mn, -RAIL.w / 2 + 7, -15, 1).join(',') + ')';
    }
  }
  if (!railDrawn || !note) { if (noteShown) { noteEl.style.visibility = 'hidden'; noteShown = false; } }
  chipEls.forEach((c, i) => { if (c.shown && (!railDrawn || i >= srcs.length)) { c.s.visibility = 'hidden'; c.shown = false; } });
  if (!railDrawn) btnEls.forEach(b => { if (b.shown) { b.s.visibility = 'hidden'; b.shown = false; } });
  items.push(...railItems);
  draw({ items });

  // words ride on each card's glass; an upper card's words would sit behind the card in front
  const shownNow = new Set();
  const cardHits = [];
  let bx0 = Infinity, by0 = Infinity, bx1 = -Infinity, by1 = -Infinity;
  drawn.forEach(dd => {
    const w = dd.w;
    if (browsing && dd.col === cols[cI] && dd.ad < 1.25 && dd.a > 0.35) {
      const r = rectOf(dd.M, CARD[0], CARD[1]);
      cardHits.push({ id: dd.cardId, r, z: dd.z });
      bx0 = Math.min(bx0, r[0]); by0 = Math.min(by0, r[1]); bx1 = Math.max(bx1, r[0] + r[2]); by1 = Math.max(by1, r[1] + r[3]);
    }
    // vertical: lower cards keep their words (upper ones are covered); horizontal: the front card's only
    const ov = (dd.d >= 0 ? clamp01((2.3 - dd.ad) / 0.6) : 1 - smoothstep(0, 0.3, dd.ad)) * (1 - 0.25 * Math.min(dd.ad, 2));
    const oh = 1 - smoothstep(0.3, 0.75, dd.ad);
    let o = lerp(ov, oh, eo) * dd.a * (1 - 0.85 * fast * (1 - dd.hm));
    o *= dd.rel === 0 ? lerp(0.85, 1, dd.colW) : dd.colW;
    if (o < 0.003) return;
    shownNow.add(w);
    if (!w.shown) { w.t.visibility = w.a.visibility = 'visible'; w.shown = true; }
    w.t.opacity = w.a.opacity = o;
    const x = lerp(-107, -62, dd.hm);
    w.t.transform = 'matrix(' + aff(dd.M, x, lerp(93, -19, dd.hm), lerp(1, 0.86, dd.hm)).join(',') + ')';
    w.a.transform = 'matrix(' + aff(dd.M, x, lerp(120, 2, dd.hm), lerp(1, 0.87, dd.hm)).join(',') + ')';
  });
  cols.forEach(c => c.words.forEach(w => { if (w.shown && !shownNow.has(w)) hideWords(w); }));
  // long queues: text for songs far from the wheel is dropped and made again if you come back
  if (t - lastPrune > 1) {
    lastPrune = t;
    cols.forEach(c => c.words.forEach((w, k) => {
      const r = c.relByKey.get(k);
      if (!w.shown && (r === undefined || Math.abs(r - c.p) > 12)) { w.tEl.remove(); w.aEl.remove(); c.words.delete(k); }
    }));
  }
  const hero = drawn.find(dd => dd.chosen && dd.rel === sel);
  eqEl.style.opacity = hero ? smoothstep(0.55, 0.9, mp) * hero.a : 0;
  if (hero) eqEl.style.transform = 'matrix(' + aff(hero.M, 86, -11, 1).join(',') + ')';
  bars.forEach((b, k) => { b.height = (6 + 14 * (0.5 + 0.5 * Math.sin(t * 6.3 + k * 1.9))) + 'px'; });

  // the front card is on top of the cards behind it; the rail is never covered
  cardHits.sort((a1, a2) => a2.z - a1.z);
  const hovered = cardHits.find(h => inRect(h.r, m));
  cardHits.forEach(h => hoverOf(h.id, hovered === h, dt));
  hits.push(...cardHits);
  hits.forEach(h => { if (h.id.startsWith('src:') || h.id.startsWith('ctl:')) { bx0 = Math.min(bx0, h.r[0]); by0 = Math.min(by0, h.r[1]); bx1 = Math.max(bx1, h.r[0] + h.r[2]); by1 = Math.max(by1, h.r[1] + h.r[3]); } });
  bounds = isFinite(bx0) ? [bx0, by0, bx1 - bx0, by1 - by0] : null;
  sendHits(browsing && presence > 0.5 ? hits : [], t);
}

resize();
clearScreen();
// Opened in an ordinary browser (no app around it): a small hook so the page can be checked.
if (!TAURI) {
  window.__preview = {
    view: v => applyView(v),
    snapshot: src => new Promise(res => { const im = new Image(); im.onload = () => { tSnap = texFromSource(im); res(); }; im.src = src; }),
    state: () => state,
  };
}
window.__skipdat = { layout: () => oriT, ori: () => val(oriC), state: () => state, dirty: () => dirty, cols: () => cols.map(c => ({ key: c.key, tgt: c.tgt, p: c.p, relMin: c.relMin, relMax: c.relMax, words: c.words.size })), ai: () => Math.round(ai), home: () => home.slice(), bounds: () => bounds };
})();
