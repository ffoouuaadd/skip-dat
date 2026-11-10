// Records Skip Dat's real overlay (ui/overlay.html + overlay.js) with a fake Tauri, frame by frame
// on a virtual clock, then encodes each scene to a GIF.   node rec.mjs [scene ...]
import { launch } from './pp.mjs';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '..', '..');
const FRAMES = path.join(HERE, 'frames');                 // scratch frames (git-ignored)
const OUT = path.join(REPO, 'docs', 'media');             // the GIFs go straight onto the README
const FFMPEG = process.env.FFMPEG || 'ffmpeg';            // ffmpeg on the PATH, or the one named here
const FPS = 50, W = 1000, H = 625, DPR = 2;          // virtual steps per second

// ---- static files ----
const ROUTES = { '/ui/': path.join(REPO, 'ui'), '/art/': path.join(HERE, 'art'), '/stage/': path.join(HERE, 'stage'), '/media/': path.join(REPO, 'docs', 'media'),
  // the installer's own art and the companion's icon, for the install and companion scenes
  '/installer/': path.join(REPO, 'installer'), '/branding/': path.join(REPO, 'branding', 'out'), '/companion/': path.join(REPO, 'browser-companion') };
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.png': 'image/png', '.jpg': 'image/jpeg', '.webp': 'image/webp', '.gif': 'image/gif', '.json': 'application/json', '.woff2': 'font/woff2', '.css': 'text/css', '.bmp': 'image/bmp' };
const server = http.createServer((req, res) => {
  const u = decodeURIComponent(req.url.split('?')[0]);
  const pre = Object.keys(ROUTES).find(p => u.startsWith(p));
  const file = pre && path.join(ROUTES[pre], u.slice(pre.length));
  if (!file || !file.startsWith(ROUTES[pre]) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream', 'Access-Control-Allow-Origin': '*' });
  fs.createReadStream(file).pipe(res);
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const ORIGIN = 'http://127.0.0.1:' + server.address().port;

// ---- demo music: real albums released under Creative Commons, with their real covers (credits in README) ----
// acts: the song buttons that work for each (view.rs acts_for: a tab with Spotify connected, Spotify itself)
const S = (key, title, artist, art) => ({ key, title, artist, art });
const YTM = {
  key: 'ytm', name: 'YouTube Music', via: 'Chrome', playing: true, acts: ['like', 'link', 'spotify'], liked: false,
  hist: [S('y1', 'Express Cause', 'Kodak To Graph', 'cc-kodak.jpg'), S('y2', 'Easy', 'Ben Mason', 'cc-ben-mason.jpg')],
  now: S('y3', 'Ochre Against Blue', 'Memotone', 'cc-memotone.jpg'),
  up: [S('y4', 'Plumy Tale', 'Dumbo Gets Mad', 'cc-dumbo-prawn.jpg'), S('y5', 'It’s Okay, Roseanne', 'The Parish of Little Clifton', 'cc-parish.jpg'),
    S('y6', 'Been A While Since You Spoke', 'Handbook', 'cc-handbook.jpg'), S('y7', 'Usual Pop Line (Populous Remix)', 'Iròi', 'cc-iroi.jpg'),
    S('y8', 'Balancing Act', 'The Underscore Orkestra', 'cc-underscore.jpg'), S('y9', 'Ode To A Baby Snowstorm', 'Ghostly Dust Machine', 'cc-ghostly-dust.jpg'),
    S('y10', 'You Make You Feel (Diva Remix)', 'Dumbo Gets Mad', 'cc-dumbo-feel.jpg'), S('y11', 'My Darling Grace', 'Berlinist', 'cc-berlinist.jpg')],
};
const slip = (k, t) => S(k, t, 'Nine Inch Nails', 'nin-slip.jpg');
const SPOT = {
  key: 'spotify', name: 'Spotify', via: '', playing: false, acts: ['like', 'link', 'next'], likes: [],
  hist: [slip('s1', '999,999'), slip('s2', '1,000,000')], now: slip('s3', 'Letting You'),
  up: [slip('s4', 'Discipline'), slip('s5', 'Echoplex'), slip('s6', 'Head Down'), slip('s7', 'Lights in the Sky'), slip('s8', 'Corona Radiata'), slip('s9', 'The Four of Us Are Dying'), slip('s10', 'Demon Seed')],
};
const ghost = n => S('g' + n, `${n} Ghosts ${['I', 'II', 'III', 'IV'][Math.floor((n - 1) / 9)]}`, 'Nine Inch Nails', 'nin-ghosts.jpg');
const SC = { key: 'soundcloud', name: 'SoundCloud', via: 'Chrome', playing: false, acts: ['like', 'link', 'spotify'], hist: [ghost(4), ghost(5)], now: ghost(6), up: [7, 8, 9, 10, 11, 12, 13].map(ghost) };

// the built-in skins, in the app's order (skins.rs)
const SKINS = ['liquid-glass', 'lime-pod', 'lcd-night', 'brushed-chrome', 'pixel-party', 'quicksilver', 'bare-knuckle', 'bubble-gum', 'claude-ish'].map(id => {
  const data = JSON.parse(fs.readFileSync(path.join(REPO, 'ui', 'skins', id, 'skin.json'), 'utf8'));
  return { id, name: data.name || id, author: data.author || 'Skip Dat', builtin: true, folder: `skins/${id}/`, rev: 0, data, error: null };
});
// every skin picture, so animated ones are decoded before recording (stage.js plays them on the virtual clock)
const PICTURES = SKINS.flatMap(s => [].concat(s.data.decoration || []).filter(d => d && d.image).map(d => s.folder + d.image));
const LOOK = { style: 'wheel', layout: 'vertical', size: 'medium', depth_blur: 0.55, depth_fade: 0.45, perf: false, glass_hue: null, skin: SKINS[0] };

// In the windows (Settings, history): the floating flag when asked for, and timers and CSS animations on the
// virtual clock, so a panel grows at its real speed however long a frame takes to record.
const FRAME_JS = `(() => {
  if (window === window.top) return;
  const SD = window.top.__SD; if (!SD) return;
  if (SD.floatNext) window.__SKIPDAT_FLOAT = true;
  const q = new Map(); let seq = 0, last = SD.T();
  window.setTimeout = (fn, ms, ...a) => { q.set(++seq, [SD.T() + (+ms || 0), fn, a, 0]); return seq; };
  window.setInterval = (fn, ms, ...a) => { q.set(++seq, [SD.T() + (+ms || 0), fn, a, Math.max(+ms || 0, 10)]); return seq; };
  window.clearTimeout = window.clearInterval = id => { q.delete(id); };
  SD.onStep.push(() => {
    if (!window.frameElement) return;               // the window was closed
    const now = SD.T(), dt = now - last; last = now;
    for (const [id, t] of [...q]) if (t[0] <= now) { if (t[3]) t[0] += t[3]; else q.delete(id); try { if (typeof t[1] === 'function') t[1](...t[2]); } catch (e) { console.error(e); } }
    for (const a of document.getAnimations()) {
      if (!a.__sd) { a.__sd = 1; a.pause(); a.currentTime = 0; continue; }
      const end = a.effect ? a.effect.getComputedTiming().endTime : 0, ct = (a.currentTime || 0) + dt;
      if (end !== Infinity && ct >= end) { if (a.playState !== 'finished') a.finish(); } else a.currentTime = ct;
    }
  });
})();`;

// ---- recorder ----
const browser = await launch();
async function record(name, script) {
  const dir = path.join(FRAMES, name);
  fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const page = await browser.newPage();
  page.on('console', m => { if (m.type() === 'error' || m.type() === 'warn') console.log(`  [${name}] ${m.type()}: ${m.text()} ${m.location().url || ''}`); });
  page.on('pageerror', e => console.log(`  [${name}] pageerror: ${e.message}`));
  await page.setViewport({ width: W, height: H, deviceScaleFactor: DPR });
  await page.evaluateOnNewDocument(fs.readFileSync(path.join(HERE, 'mock.js'), 'utf8'));
  await page.evaluateOnNewDocument(FRAME_JS);
  await page.goto(ORIGIN + '/ui/overlay.html', { waitUntil: 'load' });
  await page.addScriptTag({ url: ORIGIN + '/stage/stage.js' });
  await page.addScriptTag({ url: ORIGIN + '/stage/apps.js' });
  await page.evaluate(urls => window.__SD.preloadPictures(urls), PICTURES);
  const cdp = await page.createCDPSession();
  let n = 0, gen = 0, mouse = [-1, -1], moved = false, busyUntil = 0;
  const pending = [], kept = [];
  const ev = (fn, ...a) => page.evaluate(fn, ...a);
  // Something is moving for a while after every action (and while the pointer moves): those frames are
  // kept densely, resting ones (only the wheel's slow float) sparsely; see encode()
  const busy = (s = 1.2) => { busyUntil = Math.max(busyUntil, n / FPS + s); };
  const emit = (e, p) => { busy(); return ev((e, p) => window.__SD.emit(e, p), e, p); };
  // one step of virtual time; the real mouse follows the drawn pointer (hover and clicks in the windows)
  const tick = async () => {
    const [x, y] = await ev(async ms => { const SD = window.__SD; SD.step(ms); await SD.settle(); return [SD.ptr.x, SD.ptr.y]; }, 1000 / FPS);
    moved = x !== mouse[0] || y !== mouse[1];
    if (moved) { mouse = [x, y]; await page.mouse.move(x, y); }
  };
  const step = async () => {
    await tick();
    const i = n++, on = moved || i / FPS <= busyUntil;
    if (on ? i % 2 && i % 3 : i % 5) return;
    const { data } = await cdp.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
    kept.push([i, on ? 1 : 0]);
    pending.push(fs.promises.writeFile(path.join(dir, 'f' + String(i).padStart(5, '0') + '.png'), Buffer.from(data, 'base64')));
  };
  const frameOf = kind => page.frames().find(f => f.url().includes('/ui/' + kind + '.html'));
  const ctl = {
    async wait(s) { const k = Math.round(s * FPS); for (let i = 0; i < k; i++) await step(); },
    async quiet(s) { const k = Math.round(s * FPS); for (let i = 0; i < k; i++) await tick(); },
    emit,
    async music(srcs, active, look = LOOK) {
      await ev((srcs, active, look, skins) => { const SD = window.__SD; SD.setMusic(srcs, active); SD.look = look; SD.skins = skins; SD.emit('lg-look', look); SD.emit('lg-media', SD.view()); }, srcs, active, look, SKINS);
    },
    setLook: patch => ev(p => { const SD = window.__SD; if (p.skinId) { p.skin = SD.skins.find(s => s.id === p.skinId); delete p.skinId; } SD.look = { ...SD.look, ...p }; SD.emit('lg-look', SD.look); }, patch),
    caption: c => ev(c => window.__SD.caption(c), c),
    noCaption: () => ev(() => window.__SD.noCaption()),
    keys: down => ev(d => window.__SD.keys(d), down),
    ptr: (x, y) => ev((x, y) => { const p = window.__SD.ptr; p.x = x; p.y = y; p.to = null; }, x, y),
    moveTo: (x, y, dur = 0.6) => ev((x, y, d) => window.__SD.moveTo(x, y, d), x, y, dur),
    async glide(x, y, dur = 0.6) { await ctl.moveTo(x, y, dur); await ctl.wait(dur + 0.08); },
    flyTag: tag => ev(t => { window.__SD.flyTag = t; }, tag),
    showNow: () => ev(() => window.__SD.nowChanged()),
    seed: n => ev(n => window.__skipdat.seed(n), n),
    async open() {
      // like the app: photograph the desktop for the glass, then show the wheel at the pointer
      await ev(() => window.__SD.showChrome(false));
      const { data } = await cdp.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
      await ev(() => window.__SD.showChrome(true));
      await ev(async b64 => {
        const img = new Image(); img.src = 'data:image/png;base64,' + b64; await img.decode();
        const c = document.createElement('canvas'); c.width = img.width; c.height = img.height;
        const x = c.getContext('2d'); x.drawImage(img, 0, 0);
        const d = x.getImageData(0, 0, c.width, c.height).data;
        const buf = new ArrayBuffer(8 + d.length), dv = new DataView(buf);
        dv.setUint32(0, c.width, true); dv.setUint32(4, c.height, true); new Uint8Array(buf, 8).set(d);
        window.__SD.snap = buf;
      }, data);
      gen++;
      await ev(g => { const SD = window.__SD, d = window.devicePixelRatio; SD.emit('lg-snapshot', null); SD.emit('lg-show', { x: SD.ptr.x * d, y: SD.ptr.y * d, gen: g, layout: SD.look.layout }); }, gen);
    },
    async scroll(steps, gap = 0.3) {
      const dir = Math.sign(steps);
      for (let i = 0; i < Math.abs(steps); i++) { await emit('lg-wheel', dir); await ev(() => window.__SD.wheelPulse()); await ctl.wait(gap); }
    },
    app: steps => emit('lg-app', steps),
    commit: () => emit('lg-commit', null),
    cancel: () => emit('lg-cancel', null),
    hit: pred => ev(p => { const h = window.__SD.hits.find(r => new RegExp(p).test(r.id)); return h ? [h.x + h.w / 2, h.y + h.h / 2, h.id] : null; }, pred.source),
    async clickHit(pred, dur = 0.55, settle = 0.25) {
      let h = await ctl.hit(pred);
      if (!h) throw new Error(name + ': no hit ' + pred);
      await ctl.glide(h[0], h[1], dur);
      h = (await ctl.hit(pred)) || h;
      await ctl.wait(settle);
      await ev(() => window.__SD.clickFx());
      await emit('lg-click', h[2]);
    },
    // glide onto a part of the wheel and stay there (hover). The wheel leans while the pointer moves,
    // so its place is looked up again once it has settled.
    async hoverHit(pred, dur = 0.55) {
      let h = await ctl.hit(pred);
      if (!h) throw new Error(name + ': no hit ' + pred);
      await ctl.glide(h[0], h[1], dur); await ctl.wait(0.15);
      h = (await ctl.hit(pred)) || h;
      await ctl.glide(h[0], h[1], 0.15);
      return h;
    },
    calls: cmd => ev(c => window.__SD.calls.filter(x => x[0] === c).length, cmd),
    // the pretend apps (stage/apps.js): run something on the page, find a part, click it for real
    exec: (fn, ...a) => ev(fn, ...a),
    async at(sel, fx = 0.5, fy = 0.5) {
      const p = await ev((s, fx, fy) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return [b.left + b.width * fx, b.top + b.height * fy]; }, sel, fx, fy);
      if (!p) throw new Error(name + ': nothing at ' + sel);
      return p;
    },
    async hoverAt(sel, dur = 0.6) { const [x, y] = await ctl.at(sel); await ctl.glide(x, y, dur); },
    async clickAt(sel, dur = 0.6, settle = 0.15) {
      await ctl.hoverAt(sel, dur); await ctl.wait(settle);
      await ctl.clickFx(); await page.mouse.down(); await ctl.wait(0.06); await page.mouse.up();
    },
    clickFx: () => ev(() => window.__SD.clickFx()),
    meme: (t, b, dur, y) => ev((t, b, d, y) => window.__SD.meme(t, b, d, y), t || '', b || '', dur || 1.6, y || [70, 470]),
    sticker: (e, x, y, dur, rot) => ev((e, x, y, d, r) => window.__SD.sticker(e, x, y, d, r), e, x, y, dur || 1.4, rot ?? -12),
    chip: (t, x, y, dur, rot) => ev((t, x, y, d, r) => window.__SD.chip(t, x, y, d, r), t, x, y, dur || 1.8, rot ?? 4),
    punch: (a, d) => ev((a, d) => window.__SD.punch(a, d), a || 0.045, d || 0.42),
    // ---- windows (Settings, history) ----
    async openWindow(kind, x, y, w, h, opt = {}) {
      await ev((k, x, y, w, h, o) => window.__SD.openWindow(k, x, y, w, h, o), kind, x, y, w, h, opt);
      // the page loads in real time: wait for it (and its fonts) without moving the clock much
      for (let i = 0; i < 200 && !(frameOf(kind) && await frameOf(kind).evaluate(() => document.readyState === 'complete').catch(() => false)); i++) await new Promise(r => setTimeout(r, 20));
      await frameOf(kind).evaluate(() => document.fonts.ready.then(() => new Promise(r => requestAnimationFrame(() => r()))));
    },
    closeWindow: kind => ev(k => window.__SD.closeWindow(k), kind),
    // an element's middle on the desktop, or a point inside it (fx, fy: 0..1 across it)
    async inWin(kind, sel, fx = 0.5, fy = 0.5) {
      const fr = frameOf(kind);
      const r = await fr.$eval(sel, (e, fx, fy) => { const b = e.getBoundingClientRect(); return [b.left + b.width * fx, b.top + b.height * fy]; }, fx, fy);
      const o = await ev(k => { const f = [...document.querySelectorAll('.sd-win iframe')].find(f => f.src.includes(k)).getBoundingClientRect(); return [f.left, f.top]; }, kind);
      return { fr, x: r[0] + o[0], y: r[1] + o[1] };
    },
    async hoverIn(kind, sel, dur = 0.6) { const { x, y } = await ctl.inWin(kind, sel); await ctl.glide(x, y, dur); },
    // a real click (the page sees the pointer press and the hover before it)
    async clickIn(kind, sel, dur = 0.6) {
      const { x, y } = await ctl.inWin(kind, sel);
      await ctl.glide(x, y, dur); await ctl.wait(0.2);
      await ctl.clickFx(); await page.mouse.down(); await ctl.wait(0.06); await page.mouse.up();
    },
    // drag a slider's thumb from where it is to `to` (0..1 along it), eased, over `dur` seconds
    async dragSlider(kind, sel, to, dur = 1.6) {
      const fr = frameOf(kind);
      const info = await fr.$eval(sel, e => { const b = e.getBoundingClientRect(); return { l: b.left, w: b.width, t: b.top + b.height / 2, v: (+e.value - +e.min) / (+e.max - +e.min) }; });
      const o = await ev(k => { const f = [...document.querySelectorAll('.sd-win iframe')].find(f => f.src.includes(k)).getBoundingClientRect(); return [f.left, f.top]; }, kind);
      const at = v => o[0] + info.l + 11 + (info.w - 22) * v, y = o[1] + info.t;
      await ctl.glide(at(info.v), y, 0.6); await ctl.wait(0.15);
      await ctl.clickFx(); await page.mouse.down(); await ctl.wait(0.1);
      await ctl.moveTo(at(to), y, dur); await ctl.wait(dur + 0.08);
      await page.mouse.up();
    },
    key: async (k, mods = []) => { for (const m of mods) await page.keyboard.down(m); await page.keyboard.press(k); for (const m of mods.reverse()) await page.keyboard.up(m); },
    // smooth scroll inside a window to an element (Settings floating scrolls its body, not the page)
    async scrollWin(kind, sel, dur = 0.7, pad = 10) {
      await ev((k, s, d, pad) => {
        const f = [...document.querySelectorAll('.sd-win iframe')].find(f => f.src.includes(k)), doc = f.contentDocument;
        const sc = doc.documentElement.classList.contains('float') ? doc.body : doc.scrollingElement;
        const to = s ? doc.querySelector(s).getBoundingClientRect().top + sc.scrollTop - pad : 0, from = sc.scrollTop;
        window.__SD.anim(d, q => { const e = q < 0.5 ? 4 * q * q * q : 1 - Math.pow(-2 * q + 2, 3) / 2; sc.scrollTop = from + (to - from) * e; });
      }, kind, sel, dur, pad);
      await ctl.wait(dur + 0.1);
    },
    busy,
    page,
  };
  // every action marks the next moments as moving (waiting and looking things up don't)
  for (const k of Object.keys(ctl)) {
    const f = ctl[k];
    if (typeof f === 'function' && !['wait', 'quiet', 'hit', 'inWin', 'busy', 'calls', 'at'].includes(k)) ctl[k] = (...a) => { busy(); return f(...a); };
  }
  await ctl.quiet(0.2);
  await script(ctl);
  await Promise.all(pending);
  fs.writeFileSync(path.join(dir, 'frames.json'), JSON.stringify(kept));
  await page.close();
  console.log(`${name}: ${(n / FPS).toFixed(1)} s, ${kept.length} frames kept`);
  return dir;
}

// GIF settings, best first. The open wheel floats, so every frame changes where it is; a scene takes the
// first setting that keeps it at about 9 MB or less.
const LADDER = [{ move: 2, rest: 5, scale: 'lanczos', bayer: 3 }, { move: 3, rest: 5, scale: 'lanczos', bayer: 3 }, { move: 3, rest: 10, scale: 'area', bayer: 4 },
  { move: 3, rest: 10, scale: 'bicubic', bayer: 5 }, { move: 4, rest: 10, scale: 'area', bayer: 4 }];
const MAX_MB = 9.5;
function encode(name, dir) {
  fs.mkdirSync(OUT, { recursive: true });
  const out = path.join(OUT, name + '.gif'), list = path.join(dir, 'gif.txt');
  const kept = JSON.parse(fs.readFileSync(path.join(dir, 'frames.json'), 'utf8'));
  for (const { move, rest, scale, bayer } of LADDER) {
    // while something moves, every `move`th step (25, 16.7 or 12.5 fps); at rest every `rest`th (10 or 5 fps).
    // Steps are 20 ms, so every delay is whole hundredths, as a GIF stores them.
    const pick = kept.filter(([i, on]) => i % (on ? move : rest) === 0).map(([i]) => i);
    const f = i => `file 'f${String(i).padStart(5, '0')}.png'`;
    const lines = pick.map((i, k) => `${f(i)}\nduration ${(((pick[k + 1] ?? i + move) - i) / FPS).toFixed(2)}`);
    // the last picture is named once more, or the concat reader drops its duration
    fs.writeFileSync(list, ['ffconcat version 1.0', ...lines, f(pick[pick.length - 1]), ''].join('\n'));
    const vf = `scale=960:-2:flags=${scale},split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=bayer:bayer_scale=${bayer}:diff_mode=rectangle`;
    execFileSync(FFMPEG, ['-v', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', list, '-vf', vf, '-fps_mode', 'vfr', '-loop', '0', out]);
    const mb = fs.statSync(out).size / 1e6;
    console.log(`${name}.gif: ${mb.toFixed(2)} MB (${pick.length} frames: ${(FPS / move).toFixed(1)} fps moving, ${FPS / rest} at rest, ${scale}, bayer ${bayer})`);
    if (mb <= MAX_MB) return;
  }
}

// ---- scenes ----
const SCENES = (await import('./scenes.mjs')).default({ YTM, SPOT, SC, LOOK, SKINS });
// --no-gif: record only; --gif-only: encode the frames recorded last time
const want = process.argv.slice(2), flag = f => want.includes(f);
const only = want.filter(w => !w.startsWith('--'));
for (const [name, script] of Object.entries(SCENES)) {
  if (only.length && !only.includes(name)) continue;
  const dir = flag('--gif-only') ? path.join(FRAMES, name) : await record(name, script);
  if (!flag('--no-gif')) encode(name, dir);
}
await browser.close();
server.close();
