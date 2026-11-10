// Renders a skin on the real carousel (ui/overlay.js) and checks its text contrast.
//   node skin_preview.mjs <skin id in ui/skins> <out.png>
// The picture has three views side by side: the vertical wheel, the same a moment later (to see
// animations), and the Stack style with a song button's name showing. Contrast is checked for
// the card text, the app names and the button icons (WCAG: 4.5 or more is easy to read).
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch } from './pp.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '..', '..');
const [id, out] = process.argv.slice(2);
if (!id || !out) { console.error('usage: node skin_preview.mjs <skin id> <out.png>'); process.exit(2); }
const data = JSON.parse(fs.readFileSync(path.join(REPO, 'ui', 'skins', id, 'skin.json'), 'utf8'));

// ---- contrast ----
function rgb(c) {
  if (typeof c !== 'string') return null;
  let m = c.match(/^#([0-9a-f]{6})$/i);
  if (m) return [0, 2, 4].map(i => parseInt(m[1].slice(i, i + 2), 16));
  m = c.match(/^#([0-9a-f]{3})$/i);
  if (m) return [...m[1]].map(h => parseInt(h + h, 16));
  m = c.match(/^rgba?\(([^)]+)\)/i);
  if (m) return m[1].split(',').slice(0, 3).map(v => parseFloat(v));
  return null;
}
const lum = c => { const [r, g, b] = c.map(v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
const ratio = (a, b) => { const A = rgb(a), B = rgb(b); if (!A || !B) return null; const [x, y] = [lum(A), lum(B)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
const lines = [];
const pair = (what, fg, bgs) => {
  const rs = bgs.filter(Boolean).map(bg => ratio(fg, bg)).filter(v => v !== null);
  if (!rs.length) return;
  const worst = Math.min(...rs);
  lines.push(`${worst >= 4.5 ? 'OK  ' : worst >= 3 ? 'LOW ' : 'BAD '} ${what}: ${worst.toFixed(1)}:1`);
};
if (data.base === 'panel') {
  const c = data.card || {}, t = data.text || {};
  pair('title on card', t.color || '#FAFBFD', [c.top || '#2a2f3a', c.bottom || '#14171d']);
  pair('artist on card', t.soft || 'rgba(250,251,253,.76)', [c.top || '#2a2f3a', c.bottom || '#14171d']);
  const ch = data.chip || {};
  pair('app name on chip', ch.text || t.color || '#FAFBFD', [ch.top || c.top, ch.bottom || c.bottom]);
  pair('active app name', ch.activeText || t.color || '#FAFBFD', [ch.activeTop, ch.activeBottom]);
  const b = data.button || {};
  pair('button icons', b.icon || t.color || '#FAFBFD', [b.top || c.top, b.bottom || c.bottom]);
} else lines.push('glass skin: text sits on your desktop through the glass; check the picture');

// ---- render ----
const ROUTES = { '/ui/': path.join(REPO, 'ui'), '/art/': path.join(HERE, 'art') };
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.png': 'image/png', '.jpg': 'image/jpeg', '.gif': 'image/gif', '.webp': 'image/webp', '.woff2': 'font/woff2', '.ttf': 'font/ttf' };
const server = http.createServer((req, res) => {
  const u = decodeURIComponent(req.url.split('?')[0]);
  const pre = Object.keys(ROUTES).find(p => u.startsWith(p));
  const f = pre && path.join(ROUTES[pre], u.slice(pre.length));
  if (!f || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'Content-Type': TYPES[path.extname(f)] || 'application/octet-stream' });
  fs.createReadStream(f).pipe(res);
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const b = await launch();
const p = await b.newPage();
const errors = [];
p.on('pageerror', e => errors.push(e.message));
p.on('console', m => { if (m.type() === 'warning' || m.type() === 'error') errors.push(m.text()); });
await p.setViewport({ width: 900, height: 640, deviceScaleFactor: 1 });
await p.evaluateOnNewDocument(fs.readFileSync(path.join(HERE, 'mock.js'), 'utf8'));
await p.goto(`http://127.0.0.1:${server.address().port}/ui/overlay.html`, { waitUntil: 'load' });
// a desktop with light and dark areas, to judge readability over both
await p.evaluate(() => { document.documentElement.style.background = 'linear-gradient(135deg, #2e5f9e 0%, #9ec5e8 38%, #e9d8a6 55%, #6b4f2a 78%, #1d1d24 100%)'; });
const S = (key, title, artist, art) => ({ key, title, artist, art });
const src = { key: 'spotify', name: 'Spotify', via: '', playing: true, acts: ['like', 'link', 'next'], liked: true,
  hist: [S('a', 'Easy', 'Ben Mason', 'cc-ben-mason.jpg')], now: S('c', 'Ochre Against Blue', 'Memotone', 'cc-memotone.jpg'),
  up: [S('d', 'Plumy Tale', 'Dumbo Gets Mad', 'cc-dumbo-prawn.jpg'), S('e', 'Balancing Act', 'The Underscore Orkestra', 'cc-underscore.jpg'), S('f', 'Septemberish', 'The Parish', 'cc-parish.jpg')] };
const yt = { key: 'ytm', name: 'YouTube Music', via: 'Chrome', playing: false, hist: [], now: S('y', 'Been A While', 'Handbook', 'cc-handbook.jpg'), up: [] };
const skin = { id, name: data.name || id, author: data.author || '', builtin: true, folder: `skins/${id}/`, rev: 0, data, error: null };
await p.evaluate((s, y, skin) => {
  const SD = window.__SD; SD.setMusic([s, y], 'spotify');
  SD.look = { style: 'wheel', layout: 'vertical', size: 'medium', depth_blur: 0.5, depth_fade: 0.4, perf: false, skin };
  SD.emit('lg-look', SD.look); SD.emit('lg-media', SD.view());
  SD.emit('lg-show', { x: 300, y: 380, gen: 1 });
}, src, yt, skin);
const frames = n => p.evaluate(n => { for (let i = 0; i < n; i++) window.__SD.step(40); }, n);
const shots = [];
await frames(60); await new Promise(r => setTimeout(r, 400)); await frames(2);
shots.push(await p.screenshot({ encoding: 'base64' }));
await frames(18);
shots.push(await p.screenshot({ encoding: 'base64' }));
await p.evaluate(skin => { const SD = window.__SD; SD.look = { ...SD.look, style: 'stack' }; SD.emit('lg-look', SD.look); SD.emit('lg-toast', 'Added Ochre Against Blue to your Liked Songs'); }, skin);
await frames(40);
shots.push(await p.screenshot({ encoding: 'base64' }));
// side by side
const page2 = await b.newPage();
await page2.setViewport({ width: 900 * 3 + 20, height: 640 });
await page2.setContent(`<body style="margin:0;display:flex;gap:10px;background:#111">${shots.map(s => `<img src="data:image/png;base64,${s}" width="900" height="640">`).join('')}</body>`);
await page2.screenshot({ path: out });
await b.close(); server.close();
console.log(`${data.name || id}\n${lines.join('\n')}${errors.length ? '\nERRORS: ' + [...new Set(errors)].join(' | ') : '\nno page errors'}`);
