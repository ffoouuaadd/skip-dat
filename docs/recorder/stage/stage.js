// The pretend desktop around the real overlay: wallpaper, taskbar and tray, a now-playing toast,
// windows (Settings, history), the tutorial caption and the mouse pointer. Driven by virtual time.
(() => {
  const SD = window.__SD;
  const css = `
  html { background: #1a1020 url(/stage/wall.png) center / cover no-repeat; }
  #sd-desk { position: fixed; inset: 0; z-index: -1; pointer-events: none; font-family: "Segoe UI Variable Text", "Segoe UI", sans-serif; color: #f5f7fb; }
  #sd-top { position: fixed; inset: 0; z-index: 50; pointer-events: none; font-family: "Segoe UI Variable Text", "Segoe UI", sans-serif; color: #f5f7fb; }
  .sd-bar { position: absolute; left: 0; right: 0; bottom: 0; height: 44px; background: rgba(22, 16, 30, .78); backdrop-filter: blur(24px) saturate(1.4); border-top: 1px solid rgba(255,255,255,.08); }
  .sd-apps { position: absolute; left: 50%; top: 6px; transform: translateX(-50%); display: flex; gap: 8px; }
  .sd-apps i { width: 32px; height: 32px; border-radius: 8px; background: rgba(255,255,255,.07); display: grid; place-items: center; }
  .sd-apps i b { width: 16px; height: 16px; border-radius: 5px; display: block; }
  .sd-tray { position: absolute; right: 10px; top: 0; height: 44px; display: flex; align-items: center; gap: 4px; font-size: 12px; }
  .sd-tray .ic { width: 30px; height: 30px; border-radius: 6px; display: grid; place-items: center; }
  .sd-tray .ic.on { background: rgba(255,255,255,.12); }
  .sd-tray svg { width: 15px; height: 15px; fill: none; stroke: #f5f7fb; stroke-width: 1.6; }
  .sd-tray img { width: 18px; height: 18px; }
  .sd-clock { text-align: right; line-height: 1.25; padding: 0 6px 0 8px; font-size: 11.5px; }
  .sd-fly { position: absolute; left: 16px; bottom: 58px; width: 300px; height: 82px; border-radius: 14px; display: flex; gap: 12px; align-items: center; padding: 0 12px;
    background: linear-gradient(180deg, rgba(40, 30, 54, .82), rgba(24, 18, 34, .86)); backdrop-filter: blur(22px); box-shadow: 0 10px 30px rgba(0,0,0,.35), inset 0 0 0 1px rgba(255,255,255,.1); box-sizing: border-box; }
  .sd-fly img { width: 58px; height: 58px; border-radius: 8px; object-fit: cover; flex: none; box-shadow: 0 2px 8px rgba(0,0,0,.4); }
  .sd-fly .tx { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .sd-fly .app { font-size: 11px; opacity: .7; display: flex; align-items: center; gap: 6px; }
  .sd-fly .app i { width: 6px; height: 6px; border-radius: 50%; background: #5be37d; display: block; }
  .sd-fly b { font-size: 15px; font-weight: 650; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sd-fly .ar { font-size: 12.5px; opacity: .75; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sd-fly .tag { position: absolute; right: 10px; top: -12px; font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 999px; background: #F27DB2; color: #2A0F1E; letter-spacing: .02em; }
  .sd-menu { position: absolute; width: 210px; padding: 5px; border-radius: 10px; background: rgba(36, 28, 46, .94); backdrop-filter: blur(20px); box-shadow: 0 12px 34px rgba(0,0,0,.45), inset 0 0 0 1px rgba(255,255,255,.1); font-size: 13px; box-sizing: border-box; }
  .sd-menu div { padding: 7px 10px; border-radius: 6px; }
  .sd-menu div.hi { background: rgba(255,255,255,.1); }
  .sd-menu hr { border: 0; border-top: 1px solid rgba(255,255,255,.1); margin: 4px 6px; }
  .sd-win { position: absolute; border-radius: 10px; overflow: hidden; background: #0B1018; box-shadow: 0 24px 60px rgba(0,0,0,.55), 0 0 0 1px rgba(255,255,255,.12); transform-origin: 50% 60%; }
  .sd-win .tb { height: 32px; display: flex; align-items: center; gap: 8px; padding-left: 12px; font-size: 12px; background: #121826; color: rgba(245,247,251,.85); }
  .sd-win .tb img { width: 16px; height: 16px; }
  .sd-win .tb .sp { flex: 1; }
  .sd-win .tb span.b { width: 44px; height: 32px; display: grid; place-items: center; font-size: 13px; opacity: .8; }
  .sd-win iframe { border: 0; display: block; width: 100%; height: calc(100% - 32px); background: #0B1018; }
  /* the real pointer (driven by the recorder) reaches the windows through the overlay, which takes no clicks itself */
  html, body, #gl { pointer-events: none; }
  .sd-win { pointer-events: auto; }
  /* Settings on Windows floats: no frame or background, only its glass panels (the page draws its own buttons) */
  .sd-win.float { background: transparent; box-shadow: none; border-radius: 0; overflow: visible; }
  .sd-win.float iframe { height: 100%; background: transparent; color-scheme: dark; }
  .sd-cap { position: absolute; left: 16px; top: 14px; display: flex; align-items: center; gap: 12px; padding: 10px 14px 10px 12px; border-radius: 16px;
    background: linear-gradient(180deg, rgba(30, 24, 48, .78), rgba(18, 14, 30, .82)); backdrop-filter: blur(18px); box-shadow: 0 8px 26px rgba(0,0,0,.35), inset 0 0 0 1px rgba(255,255,255,.12); }
  .sd-cap .badge { min-width: 26px; height: 26px; padding: 0 7px; box-sizing: border-box; border-radius: 13px; background: #F27DB2; color: #2A0F1E; font: 800 13px/26px "Segoe UI"; text-align: center; flex: none; }
  .sd-cap .badge.new { background: linear-gradient(135deg, #8EF5C8, #4FD1F5); color: #052a2a; font-size: 11px; letter-spacing: .06em; }
  .sd-cap .ct b { display: block; font: 650 17px/1.2 "Segoe UI Variable Display", "Segoe UI"; letter-spacing: -.005em; }
  .sd-cap .ct span { display: block; font-size: 12.5px; opacity: .74; margin-top: 1px; }
  .sd-cap .keys { display: flex; align-items: center; gap: 6px; margin-left: 6px; }
  .sd-cap kbd { font: 650 13px "Segoe UI"; padding: 6px 9px; border-radius: 8px; background: linear-gradient(180deg, #3a3350, #262036); box-shadow: inset 0 -2px 0 rgba(0,0,0,.35), inset 0 0 0 1px rgba(255,255,255,.14); color: #f5f7fb; }
  .sd-cap kbd.down { background: linear-gradient(180deg, #F6A9CB, #E86BA6); color: #2A0F1E; box-shadow: inset 0 1px 0 rgba(0,0,0,.15), 0 0 14px rgba(242,125,178,.55); transform: translateY(1px); }
  .sd-cap .plus { opacity: .6; font-size: 13px; }
  .sd-mouse { width: 20px; height: 30px; border-radius: 10px; box-shadow: inset 0 0 0 1.6px rgba(245,247,251,.85); position: relative; }
  .sd-mouse i { position: absolute; left: 8.2px; top: 6px; width: 3.6px; height: 8px; border-radius: 2px; background: rgba(245,247,251,.85); }
  .sd-mouse i.go { background: #F27DB2; box-shadow: 0 0 8px #F27DB2; }
  .sd-cur { position: absolute; left: 0; top: 0; width: 22px; height: 32px; filter: drop-shadow(0 1px 1.5px rgba(0,0,0,.45)); }
  .sd-ring { position: absolute; width: 34px; height: 34px; margin: -17px 0 0 -17px; border-radius: 50%; border: 2.5px solid rgba(255,255,255,.95); box-shadow: 0 0 10px rgba(242,125,178,.9); opacity: 0; }
  `;
  const st = document.createElement('style'); st.textContent = css; document.head.append(st);
  const el = (tag, cls, html, parent) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; if (parent) parent.append(e); return e; };
  const desk = el('div', null, null, document.body); desk.id = 'sd-desk';
  const top = el('div', null, null, document.body); top.id = 'sd-top';
  const ease = x => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
  const clamp01 = x => Math.max(0, Math.min(1, x));
  const anims = [];        // [t0, dur, fn(k)]
  const anim = SD.anim = (dur, fn) => { anims.push([SD.T() / 1000, dur, fn]); fn(0); };

  // ---- taskbar and tray ----
  const bar = el('div', 'sd-bar', null, desk);
  el('div', 'sd-apps', ['#4FD1F5', '#F6A9CB', '#FFD166', '#8EF5C8', '#B39DFF'].map(c => `<i><b style="background:${c}"></b></i>`).join(''), bar);
  const tray = el('div', 'sd-tray', `
    <span class="ic"><svg viewBox="0 0 16 16"><path d="M4 10l4-4 4 4"/></svg></span>
    <span class="ic" id="sd-trayicon"><img src="/media/icon.png" alt=""></span>
    <span class="ic"><svg viewBox="0 0 16 16"><path d="M1.5 6.2a9.5 9.5 0 0 1 13 0M3.8 8.6a6.2 6.2 0 0 1 8.4 0M6.1 11a3 3 0 0 1 3.8 0"/></svg></span>
    <span class="ic"><svg viewBox="0 0 16 16"><path d="M2 6h3l4-3v10l-4-3H2zM11.5 5.5a3.5 3.5 0 0 1 0 5"/></svg></span>
    <span class="sd-clock">9:41 PM<br>11/5/2026</span>`, bar);
  const trayIcon = tray.querySelector('#sd-trayicon');
  SD.trayAt = () => { const r = trayIcon.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };

  // ---- now-playing toast ----
  const fly = el('div', 'sd-fly', '<img alt=""><div class="tx"><span class="app"><i></i><span></span></span><b></b><span class="ar"></span></div><span class="tag" hidden></span>', desk);
  fly.style.opacity = 0;
  let flyKey = null;
  SD.flyout = (song, tag) => {
    const key = song.title + '|' + song.artist + '|' + (song.name || '');
    if (key === flyKey && !tag) return;
    flyKey = key;
    const shown = fly.style.opacity !== '0';
    const fill = () => {
      fly.querySelector('img').src = '/art/' + song.art;
      fly.querySelector('.app span').textContent = song.name || '';
      fly.querySelector('b').textContent = song.title;
      fly.querySelector('.ar').textContent = song.artist;
      const t = fly.querySelector('.tag'); t.hidden = !tag; t.textContent = tag || '';
    };
    if (!shown) { fill(); anim(0.35, k => { const e = ease(k); fly.style.opacity = e; fly.style.transform = `translateY(${(1 - e) * 16}px)`; }); return; }
    anim(0.5, k => {
      if (k < 0.5) { const e = ease(k * 2); fly.style.opacity = 1 - e; fly.style.transform = `translateX(${-e * 18}px)`; }
      else { if (!fly._mid) { fly._mid = true; fill(); } const e = ease((k - 0.5) * 2); fly.style.opacity = e; fly.style.transform = `translateX(${(1 - e) * 18}px)`; }
      if (k >= 1) fly._mid = false;
    });
  };
  SD.nowChanged = () => { const s = SD.src(SD.M.active); if (s) SD.flyout({ ...s.now, name: s.name }, SD.flyTag || null); };

  // ---- tray menu ----
  const menu = el('div', 'sd-menu', '<div data-id="settings">Settings and media…</div><div data-id="history">What was I playing?</div><hr><div data-id="quit">Quit Skip Dat</div>', desk);
  menu.style.opacity = 0;
  SD.menu = open => {
    const [x] = SD.trayAt(); menu.style.left = (x - 190) + 'px'; menu.style.bottom = '50px';
    anim(0.18, k => { const e = ease(k); menu.style.opacity = open ? e : 1 - e; menu.style.transform = `translateY(${(open ? 1 - e : e) * 8}px)`; });
  };
  SD.menuItem = id => { const r = menu.querySelector(`[data-id="${id}"]`).getBoundingClientRect(); return [r.left + 40, r.top + r.height / 2]; };

  // ---- windows ----
  const wins = {};
  // opt.float: a frameless, see-through window like Settings on Windows (the page gets __SKIPDAT_FLOAT)
  SD.openWindow = (kind, x, y, w, h, opt = {}) => {
    if (wins[kind]) return wins[kind];
    const win = el('div', 'sd-win' + (opt.float ? ' float' : ''), opt.float ? '' : `<div class="tb"><img src="/media/icon.png" alt=""><span>${kind === 'history' ? 'What was I playing?' : 'Skip Dat'}</span><span class="sp"></span><span class="b">&#8211;</span><span class="b">&#9633;</span><span class="b">&#10005;</span></div>`, desk);
    Object.assign(win.style, { left: x + 'px', top: y + 'px', width: w + 'px', height: h + 'px', opacity: 0 });
    SD.floatNext = !!opt.float;
    const fr = el('iframe', null, null, win); fr.src = '/ui/' + kind + '.html';
    wins[kind] = { win, fr };
    anim(0.28, k => { const e = ease(k); win.style.opacity = e; win.style.transform = `scale(${0.94 + 0.06 * e})`; });
    return wins[kind];
  };
  SD.closeWindow = kind => { const w = wins[kind]; if (!w) return; delete wins[kind]; anim(0.2, k => { w.win.style.opacity = 1 - ease(k); w.win.style.transform = `scale(${1 - 0.04 * ease(k)})`; if (k >= 1) w.win.remove(); }); };
  SD.openHistory = () => SD.openWindow('history', 250, 46, 470, 500);
  // the floating Settings' own minimize and close buttons
  SD.cmd.settings_window = a => { if (a.what === 'close' || a.what === 'minimize') SD.closeWindow('settings'); };

  // ---- animated pictures (a skin's torch, drip, bubble) on the virtual clock ----
  // The browser plays an animated PNG or WebP in real time, which the recorder doesn't keep. So each one is
  // decoded once into its frames, and a decoration shows them one by one by the recorder's clock.
  const SRC = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src');
  const plain = u => String(u).replace(/[?&]play=\d+$/, '');
  const ready = new Map(), loading = new Map();       // url -> { frames, ends, total, plays } or null (a still picture)
  function decode(url) {
    if (!loading.has(url)) loading.set(url, (async () => {
      try {
        if (!window.ImageDecoder) return null;
        const r = await fetch(url), type = (r.headers.get('content-type') || '').split(';')[0];
        if (!/^image\/(png|webp|gif)$/.test(type)) return null;
        const dec = new ImageDecoder({ data: await r.arrayBuffer(), type });
        await dec.tracks.ready; await dec.completed;
        const tr = dec.tracks.selectedTrack;
        if (!tr || tr.frameCount < 2) return null;
        const frames = [], ends = []; let t = 0;
        for (let i = 0; i < tr.frameCount; i++) {
          const { image } = await dec.decode({ frameIndex: i });
          const c = new OffscreenCanvas(image.displayWidth, image.displayHeight);
          c.getContext('2d').drawImage(image, 0, 0);
          t += (image.duration || 100000) / 1000; ends.push(t); image.close();
          frames.push(URL.createObjectURL(await c.convertToBlob()));
        }
        return { frames, ends, total: t, plays: tr.repetitionCount === Infinity ? Infinity : tr.repetitionCount + 1 };
      } catch (e) { console.warn('[stage] animated picture', url, String(e)); return null; }
    })().then(a => { ready.set(url, a); return a; }));
    return loading.get(url);
  }
  SD.preloadPictures = urls => Promise.all(urls.map(decode));
  // where a looping picture starts when a skin is chosen, by file name (ms into its loop), as if it had
  // been on screen a while; a replay on a song change ("restart") always starts at the top
  SD.phase = {};
  const decoImgs = [...document.querySelectorAll('#ui img.deco, #under img.deco')];
  decoImgs.forEach(img => Object.defineProperty(img, 'src', {
    configurable: true,
    get() { return img.__want != null ? img.__want : SRC.get.call(img); },
    set(v) {
      const ph = /[?&]play=/.test(v) ? 0 : (Object.entries(SD.phase).find(([k]) => String(v).includes(k)) || [0, 0])[1];
      img.__want = String(v); img.__t0 = SD.T() - ph; img.__k = -1;
      const a = ready.get(plain(v));
      if (a === undefined) decode(plain(v));
      if (!a) SRC.set.call(img, v);
    },
  }));
  const decoding = [];
  SD.onStep.push(() => {
    for (const img of decoImgs) {
      if (img.__want == null) continue;
      if (!img.hasAttribute('src')) { img.__want = null; continue; }   // the overlay took it away
      const a = ready.get(plain(img.__want));
      if (!a) continue;
      const e = SD.T() - img.__t0, done = e >= a.total * a.plays;
      const k = done ? a.frames.length - 1 : a.ends.findIndex(x => e % a.total < x);
      if (k !== img.__k) { img.__k = k; SRC.set.call(img, a.frames[k]); decoding.push(img.decode().catch(() => {})); }
    }
  });
  // the recorder waits for the frames just shown before it takes a picture
  SD.settle = () => (decoding.length ? Promise.all(decoding.splice(0)) : null);

  // ---- caption ----
  let cap = null;
  SD.caption = c => {
    const old = cap;
    const n = el('div', 'sd-cap', `${c.badge ? `<span class="badge${c.badge === 'NEW' ? ' new' : ''}">${c.badge}</span>` : ''}<div class="ct"><b>${c.title}</b>${c.sub ? `<span>${c.sub}</span>` : ''}</div>` +
      (c.keys ? `<div class="keys">${c.keys.map(k => `<kbd>${k}</kbd>`).join('<span class="plus">+</span>')}${c.mouse !== false ? '<span class="plus">+</span><span class="sd-mouse"><i></i></span>' : ''}</div>` : ''), top);
    n.style.opacity = 0; cap = n;
    anim(0.3, k => { const e = ease(k); n.style.opacity = e; n.style.transform = `translateY(${(1 - e) * -8}px)`; if (old) { old.style.opacity = 1 - e; if (k >= 1) old.remove(); } });
    if (SD.keysDown) SD.keys(true);
  };
  SD.noCaption = () => { const old = cap; cap = null; if (old) anim(0.3, k => { old.style.opacity = 1 - ease(k); if (k >= 1) old.remove(); }); };
  SD.keys = down => { SD.keysDown = down; if (cap) cap.querySelectorAll('kbd').forEach(k => k.classList.toggle('down', down)); };
  let wheelAt = -9;
  SD.wheelPulse = () => { wheelAt = SD.T() / 1000; };

  // ---- pointer ----
  const cur = el('div', 'sd-cur', '<svg viewBox="0 0 22 32" width="22" height="32"><path d="M2 2 L2 25 L7.6 19.6 L11.4 28.4 L15 26.8 L11.3 18.3 L19 18.3 Z" fill="#fff" stroke="#111" stroke-width="1.6" stroke-linejoin="round"/></svg>', top);
  const ring = el('div', 'sd-ring', null, top);
  SD.ptr = { x: 500, y: 330, from: null, to: null, t0: 0, dur: 0, hidden: false };
  SD.moveTo = (x, y, dur) => { const p = SD.ptr; p.from = [p.x, p.y]; p.to = [x, y]; p.t0 = SD.T() / 1000; p.dur = Math.max(0.01, dur); };
  SD.clickFx = () => { ring.style.left = SD.ptr.x + 'px'; ring.style.top = SD.ptr.y + 'px'; anim(0.45, k => { ring.style.opacity = (1 - k) * 0.95; ring.style.transform = `scale(${0.5 + k * 0.9})`; }); anim(0.16, k => { cur.style.scale = 1 - 0.12 * Math.sin(Math.PI * k); }); };
  SD.showChrome = on => { top.style.visibility = on ? '' : 'hidden'; };

  SD.onStep.push(t => {
    for (let i = anims.length - 1; i >= 0; i--) { const [t0, d, fn] = anims[i]; const k = clamp01((t - t0) / d); fn(k); if (k >= 1) anims.splice(i, 1); }
    const p = SD.ptr;
    if (p.to) { const k = clamp01((t - p.t0) / p.dur), e = ease(k); p.x = p.from[0] + (p.to[0] - p.from[0]) * e; p.y = p.from[1] + (p.to[1] - p.from[1]) * e; if (k >= 1) p.to = null; }
    cur.style.transform = `translate(${p.x - 2}px, ${p.y - 2}px)`;
    const d = window.devicePixelRatio || 1;
    SD.emit('lg-cursor', { x: p.x * d, y: p.y * d });
    const mi = cap && cap.querySelector('.sd-mouse i'); if (mi) { const a = t - wheelAt; mi.classList.toggle('go', a < 0.18); mi.style.transform = `translateY(${a < 0.18 ? Math.sin(a / 0.18 * Math.PI) * 4 : 0}px)`; }
    // hover in the tray menu
    if (menu.style.opacity > 0.5) menu.querySelectorAll('div').forEach(dv => { const r = dv.getBoundingClientRect(); dv.classList.toggle('hi', p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom); });
    trayIcon.classList.toggle('on', (() => { const r = trayIcon.getBoundingClientRect(); return p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom; })());
  });

  // ---- meme layer: Impact captions, emoji stickers, a zoom punch ----
  const mcss = document.createElement('style');
  mcss.textContent = `
  .sd-meme { position: absolute; left: 0; right: 0; text-align: center; font: 900 46px/1.02 Impact, "Arial Black", sans-serif; color: #fff; letter-spacing: 1px; text-transform: uppercase;
    -webkit-text-stroke: 2.2px #000; paint-order: stroke fill; text-shadow: 0 3px 0 #000, 0 0 14px rgba(0,0,0,.35); }
  .sd-stk { position: absolute; font: 54px/1 "Segoe UI Emoji", sans-serif; filter: drop-shadow(0 4px 6px rgba(0,0,0,.35)); }
  .sd-chip { position: absolute; padding: 6px 12px; border-radius: 999px; font: 800 15px "Segoe UI"; color: #2A0F1E; background: #FFE14D; box-shadow: 0 4px 0 #b38f00, 0 8px 18px rgba(0,0,0,.3); white-space: nowrap; }`;
  document.head.append(mcss);
  SD.meme = (topText, bottomText, dur = 1.6, y = [70, 470]) => {
    const els = [[topText, y[0]], [bottomText, y[1]]].filter(x => x[0]).map(([s, yy]) => { const e = el('div', 'sd-meme', null, top); e.textContent = s; e.style.top = yy + 'px'; return e; });
    anim(dur, k => {
      const inK = clamp01(k * dur / 0.18), outK = clamp01((k * dur - (dur - 0.25)) / 0.25);
      els.forEach(e => { e.style.opacity = inK * (1 - outK); e.style.transform = `scale(${1.35 - 0.35 * ease(inK)})`; if (k >= 1) e.remove(); });
    });
  };
  SD.sticker = (emoji, x, y, dur = 1.4, rot = -12) => {
    const e = el('div', 'sd-stk', null, top); e.textContent = emoji; e.style.left = x + 'px'; e.style.top = y + 'px';
    anim(dur, k => {
      const inK = clamp01(k * dur / 0.22), outK = clamp01((k * dur - (dur - 0.25)) / 0.25);
      const pop = inK < 1 ? 0.2 + 1.1 * Math.sin(inK * Math.PI * 0.6) / Math.sin(Math.PI * 0.6) : 1;
      e.style.opacity = 1 - outK; e.style.transform = `rotate(${rot}deg) scale(${pop * (1 - 0.3 * outK)})`; if (k >= 1) e.remove();
    });
  };
  SD.chip = (text, x, y, dur = 1.8, rot = 4) => {
    const e = el('div', 'sd-chip', null, top); e.textContent = text; e.style.left = x + 'px'; e.style.top = y + 'px';
    anim(dur, k => {
      const inK = clamp01(k * dur / 0.2), outK = clamp01((k * dur - (dur - 0.25)) / 0.25);
      e.style.opacity = inK * (1 - outK); e.style.transform = `rotate(${rot}deg) scale(${0.6 + 0.4 * ease(inK)})`; if (k >= 1) e.remove();
    });
  };
  // a quick "vine boom" zoom on the whole desktop
  SD.punch = (amt = 0.045, dur = 0.42) => {
    const root = document.documentElement; root.style.transformOrigin = `${SD.ptr.x}px ${SD.ptr.y}px`;
    anim(dur, k => { const s = 1 + amt * Math.sin(Math.min(1, k * 2.2) * Math.PI) * (1 - k * 0.4); root.style.transform = k >= 1 ? '' : `scale(${s})`; });
  };
})();
