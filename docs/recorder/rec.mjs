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
const FPS = 25, W = 1000, H = 625, DPR = 2;

// ---- static files ----
const ROUTES = { '/ui/': path.join(REPO, 'ui'), '/art/': path.join(HERE, 'art'), '/stage/': path.join(HERE, 'stage'), '/media/': path.join(REPO, 'docs', 'media') };
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.png': 'image/png', '.jpg': 'image/jpeg', '.json': 'application/json', '.woff2': 'font/woff2', '.css': 'text/css' };
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
const S = (key, title, artist, art) => ({ key, title, artist, art });
const YTM = {
  key: 'ytm', name: 'YouTube Music', via: 'Chrome', playing: true,
  hist: [S('y1', 'Express Cause', 'Kodak To Graph', 'cc-kodak.jpg'), S('y2', 'Easy', 'Ben Mason', 'cc-ben-mason.jpg')],
  now: S('y3', 'Ochre Against Blue', 'Memotone', 'cc-memotone.jpg'),
  up: [S('y4', 'Plumy Tale', 'Dumbo Gets Mad', 'cc-dumbo-prawn.jpg'), S('y5', 'It’s Okay, Roseanne', 'The Parish of Little Clifton', 'cc-parish.jpg'),
    S('y6', 'Been A While Since You Spoke', 'Handbook', 'cc-handbook.jpg'), S('y7', 'Usual Pop Line (Populous Remix)', 'Iròi', 'cc-iroi.jpg'),
    S('y8', 'Balancing Act', 'The Underscore Orkestra', 'cc-underscore.jpg'), S('y9', 'Ode To A Baby Snowstorm', 'Ghostly Dust Machine', 'cc-ghostly-dust.jpg'),
    S('y10', 'You Make You Feel (Diva Remix)', 'Dumbo Gets Mad', 'cc-dumbo-feel.jpg'), S('y11', 'My Darling Grace', 'Berlinist', 'cc-berlinist.jpg')],
};
const slip = (k, t) => S(k, t, 'Nine Inch Nails', 'nin-slip.jpg');
const SPOT = {
  key: 'spotify', name: 'Spotify', via: '', playing: false,
  hist: [slip('s1', '999,999'), slip('s2', '1,000,000')], now: slip('s3', 'Letting You'),
  up: [slip('s4', 'Discipline'), slip('s5', 'Echoplex'), slip('s6', 'Head Down'), slip('s7', 'Lights in the Sky'), slip('s8', 'Corona Radiata'), slip('s9', 'The Four of Us Are Dying'), slip('s10', 'Demon Seed')],
};
const ghost = n => S('g' + n, `${n} Ghosts ${['I', 'II', 'III', 'IV'][Math.floor((n - 1) / 9)]}`, 'Nine Inch Nails', 'nin-ghosts.jpg');
const SC = { key: 'soundcloud', name: 'SoundCloud', via: 'Chrome', playing: false, hist: [ghost(4), ghost(5)], now: ghost(6), up: [7, 8, 9, 10, 11, 12, 13].map(ghost) };

const SKINS = ['liquid-glass', 'lime-pod', 'lcd-night', 'brushed-chrome', 'pixel-party'].map(id => {
  const data = JSON.parse(fs.readFileSync(path.join(REPO, 'ui', 'skins', id, 'skin.json'), 'utf8'));
  return { id, name: data.name || id, author: data.author || 'Skip Dat', builtin: true, folder: `skins/${id}/`, rev: 0, data, error: null };
});
const LOOK = { style: 'wheel', layout: 'vertical', size: 'medium', depth_blur: 0.55, depth_fade: 0.45, perf: false, skin: SKINS[0] };

// ---- recorder ----
const browser = await launch();
async function record(name, script) {
  const dir = path.join(FRAMES, name);
  fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const page = await browser.newPage();
  page.on('console', m => { if (m.type() === 'error' || m.type() === 'warn') console.log(`  [${name}] ${m.type()}: ${m.text()}`); });
  page.on('pageerror', e => console.log(`  [${name}] pageerror: ${e.message}`));
  await page.setViewport({ width: W, height: H, deviceScaleFactor: DPR });
  await page.evaluateOnNewDocument(fs.readFileSync(path.join(HERE, 'mock.js'), 'utf8'));
  await page.goto(ORIGIN + '/ui/overlay.html', { waitUntil: 'load' });
  await page.addScriptTag({ url: ORIGIN + '/stage/stage.js' });
  const cdp = await page.createCDPSession();
  let n = 0, gen = 0;
  const pending = [];
  const ev = (fn, ...a) => page.evaluate(fn, ...a);
  const emit = (e, p) => ev((e, p) => window.__SD.emit(e, p), e, p);
  const step = async () => {
    await ev(ms => window.__SD.step(ms), 1000 / FPS);
    const { data } = await cdp.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
    const f = path.join(dir, 'f' + String(n++).padStart(5, '0') + '.png');
    pending.push(fs.promises.writeFile(f, Buffer.from(data, 'base64')));
  };
  const ctl = {
    async wait(s) { const k = Math.round(s * FPS); for (let i = 0; i < k; i++) await step(); },
    async quiet(s) { const k = Math.round(s * FPS); for (let i = 0; i < k; i++) await ev(ms => window.__SD.step(ms), 1000 / FPS); },
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
    clickFx: () => ev(() => window.__SD.clickFx()),
    meme: (t, b, dur, y) => ev((t, b, d, y) => window.__SD.meme(t, b, d, y), t || '', b || '', dur || 1.6, y || [70, 470]),
    sticker: (e, x, y, dur, rot) => ev((e, x, y, d, r) => window.__SD.sticker(e, x, y, d, r), e, x, y, dur || 1.4, rot ?? -12),
    chip: (t, x, y, dur, rot) => ev((t, x, y, d, r) => window.__SD.chip(t, x, y, d, r), t, x, y, dur || 1.8, rot ?? 4),
    punch: (a, d) => ev((a, d) => window.__SD.punch(a, d), a || 0.045, d || 0.42),
    // elements inside a window (Settings, history)
    async inWin(kind, sel) {
      const fr = page.frames().find(f => f.url().includes('/ui/' + kind + '.html'));
      const r = await fr.$eval(sel, e => { const b = e.getBoundingClientRect(); return [b.left + b.width / 2, b.top + b.height / 2]; });
      const o = await ev(k => { const f = [...document.querySelectorAll('.sd-win iframe')].find(f => f.src.includes(k)).getBoundingClientRect(); return [f.left, f.top]; }, kind);
      return { fr, x: r[0] + o[0], y: r[1] + o[1] };
    },
    async clickIn(kind, sel, dur = 0.6) {
      const { fr, x, y } = await ctl.inWin(kind, sel);
      await ctl.glide(x, y, dur); await ctl.wait(0.2);
      await ctl.clickFx(); await fr.$eval(sel, e => e.click());
    },
    async scrollWin(kind, sel, dur = 0.7) {
      await ev((k, s, d) => {
        const f = [...document.querySelectorAll('.sd-win iframe')].find(f => f.src.includes(k)), w = f.contentWindow;
        const to = w.document.querySelector(s).getBoundingClientRect().top + w.scrollY - 10, from = w.scrollY;
        window.__SD.anim(d, q => { const e = q < 0.5 ? 4 * q * q * q : 1 - Math.pow(-2 * q + 2, 3) / 2; w.scrollTo(0, from + (to - from) * e); });
      }, kind, sel, dur);
      await ctl.wait(dur + 0.1);
    },
    page,
  };
  await ctl.quiet(0.2);
  await script(ctl);
  await Promise.all(pending);
  await page.close();
  console.log(`${name}: ${n} frames`);
  return dir;
}

function encode(name, dir) {
  fs.mkdirSync(OUT, { recursive: true });
  const out = path.join(OUT, name + '.gif');
  // the busiest scenes get fewer frames, coarser dither (and click-and-switch 900 px) to stay under 10 MB
  const fps = { styles: 16, skins: 16, 'click-and-switch': 14 }[name] || 20;
  const bayer = name === 'styles' || name === 'click-and-switch' ? 4 : 3;
  const width = name === 'click-and-switch' ? 900 : 960;
  const vf = `fps=${fps},scale=${width}:-2:flags=lanczos,split[a][b];[a]palettegen=max_colors=256:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=${bayer}:diff_mode=rectangle`;
  execFileSync('ffmpeg', ['-v', 'error', '-y', '-framerate', String(FPS), '-i', path.join(dir, 'f%05d.png'), '-vf', vf, '-loop', '0', out]);
  console.log(`${name}.gif: ${(fs.statSync(out).size / 1e6).toFixed(2)} MB`);
}

// ---- scenes ----
const SCENES = (await import('./scenes.mjs')).default({ YTM, SPOT, SC, LOOK, SKINS });
const want = process.argv.slice(2);
for (const [name, script] of Object.entries(SCENES)) {
  if (want.length && !want.includes(name)) continue;
  const dir = await record(name, script);
  encode(name, dir);
}
await browser.close();
server.close();
