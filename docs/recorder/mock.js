// Injected before any page script (puppeteer evaluateOnNewDocument), in every frame.
// Top frame (ui/overlay.html): a virtual clock, a fake Tauri, a small music model, and the
// pretend desktop around the real overlay. Frames (Settings, history) borrow the top's fake Tauri.
(() => {
  if (window !== window.top) {
    const SD = window.top.__SD;
    window.__TAURI__ = { core: { invoke: (c, a) => SD.invoke(c, a), convertFileSrc: p => '/art/' + p }, event: { listen: (e, f) => SD.listen(e, f) } };
    return;
  }
  // ---- virtual time: the overlay animates only when the recorder steps it ----
  let T = 0;
  const rafQ = [], timers = [];
  performance.now = () => T;
  window.requestAnimationFrame = cb => { rafQ.push(cb); return rafQ.length; };
  window.cancelAnimationFrame = () => {};
  const handlers = {};
  const clone = o => JSON.parse(JSON.stringify(o));
  const SD = window.__SD = {
    T: () => T, hits: [], calls: [], snap: null, look: null, cmd: {}, onStep: [],
    later(ms, fn) { timers.push([T + ms, fn]); },
    step(ms) {
      T += ms;
      for (let i = timers.length - 1; i >= 0; i--) if (timers[i][0] <= T) { const f = timers[i][1]; timers.splice(i, 1); f(); }
      const q = rafQ.splice(0); for (const cb of q) cb(T);
      SD.onStep.forEach(f => f(T / 1000));
    },
    emit(ev, payload) { (handlers[ev] || []).forEach(fn => { try { fn({ event: ev, payload }); } catch (e) { console.error(e); } }); },
    listen(ev, fn) { (handlers[ev] = handlers[ev] || []).push(fn); return Promise.resolve(() => {}); },
    invoke(cmd, args) {
      SD.calls.push([cmd, args]);
      const h = SD.cmd[cmd];
      try { return Promise.resolve(h ? h(args || {}) : null); } catch (e) { return Promise.reject(String(e)); }
    },
  };
  window.__TAURI__ = { core: { invoke: SD.invoke, convertFileSrc: p => '/art/' + p }, event: { listen: SD.listen } };

  // ---- the music model: each app's played songs, the song on, and what's next ----
  const M = SD.M = { active: null, srcs: [], back: null, prev: null, history: [] };
  const songOf = s => ({ title: s.title, artist: s.artist, art: s.art });
  SD.setMusic = (srcs, active) => { M.srcs = clone(srcs); M.active = active; M.back = null; M.prev = null; };
  SD.src = key => M.srcs.find(s => s.key === key);
  SD.view = () => ({
    active: M.active, hidden: [], back: M.back,
    sources: M.srcs.map(s => ({
      key: s.key, name: s.name, via: s.via || '', playing: s.playing, queue: 'full', can_jump: true, can_prev: true, can_next: true,
      shuffle: false, site: '', target: null,
      // the song buttons that work for it, and whether the song playing is liked (view.rs)
      acts: s.acts || ['like', 'link', 'next', 'spotify'], liked: s.liked === undefined ? null : s.liked,
      items: [...s.hist.slice(-4).map((x, i, a) => ({ ...x, rel: i - a.length })), { ...s.now, rel: 0 }, ...s.up.map((x, i) => ({ ...x, rel: i + 1 }))],
    })),
  });
  const push = () => { const v = SD.view(); SD.emit('lg-media', v); SD.nowChanged && SD.nowChanged(); };
  const snapshotModel = () => clone({ srcs: M.srcs, active: M.active, back: M.back });
  function play(key, itemKey) {
    const s = SD.src(key); if (!s) return;
    const before = snapshotModel();
    const old = s.now;
    const i = s.up.findIndex(x => x.key === itemKey), j = s.hist.findIndex(x => x.key === itemKey);
    if (i >= 0) { s.hist.push(s.now); s.now = s.up[i]; s.up = s.up.slice(i + 1); }   // straight there: the songs between were never played
    else if (j >= 0) { s.up = [...s.hist.slice(j + 1), s.now, ...s.up]; s.now = s.hist[j]; s.hist = s.hist.slice(0, j); }
    else return;
    M.srcs.forEach(o => { o.playing = o === s; });
    M.active = key;
    M.prev = before;
    M.back = { source: key, ...songOf(old) };
    // the heart follows the new song: Spotify knows each song's like, a tab's like button resets
    if (s.likes) s.liked = s.likes.includes(s.now.key);
    else if (s.via && s.liked === true) s.liked = false;
    s.pos = 0;
    M.history.unshift({ id: 'h' + M.history.length + 1, ...songOf(s.now), app: s.name, via: s.via || '', time: Math.floor(Date.now() / 1000) });
  }
  SD.cmd.get_view = () => SD.view();
  SD.cmd.get_look = () => SD.look;
  SD.cmd.get_snapshot = () => SD.snap;
  SD.cmd.set_hit_rects = a => { const d = window.devicePixelRatio || 1; SD.hits = (a.rects || []).map(r => ({ id: r.id, x: r.x / d, y: r.y / d, w: r.w / d, h: r.h / d })); };
  SD.cmd.overlay_done = () => { SD.hits = []; };
  SD.cmd.choose = a => { SD.later(220, () => { play(a.source, a.item); push(); }); };
  SD.cmd.media_control = a => {
    const s = SD.src(a.source); if (!s) return;
    SD.later(160, () => {
      if (a.action === 'toggle') s.playing = !s.playing;
      else if (a.action === 'next' && s.up.length) play(s.key, s.up[0].key);
      else if (a.action === 'previous' && s.hist.length) play(s.key, s.hist[s.hist.length - 1].key);
      push();
    });
  };
  SD.cmd.go_back = () => {
    if (!M.prev) return;
    SD.later(220, () => {
      const cur = snapshotModel(), was = SD.src(M.active).now;
      M.srcs = M.prev.srcs; M.active = M.prev.active;
      M.srcs.forEach(o => { o.playing = o.key === M.active; });
      M.back = { source: M.active, ...songOf(was) };
      M.prev = cur;
      push();
    });
  };
  // ---- volume, jumping within a song and the song buttons, with the app's own messages (extras.rs) ----
  const toast = t => SD.emit('lg-toast', t);
  const clock = s => Math.floor(Math.max(0, s) / 60) + ':' + String(Math.floor(Math.max(0, s) % 60)).padStart(2, '0');
  const isTab = s => !!s.via;                       // a browser tab, through the companion
  const isSpot = s => /spotify/i.test(s.key) && !isTab(s);
  const site = s => s.site || (/soundcloud/i.test(s.key) ? 'soundcloud' : /ytm|music/i.test(s.key) ? 'music.youtube.com' : 'youtube.com');
  // where the song playing is (seconds), moving on while it plays
  let posT = 0;
  SD.onStep.push(t => { const dt = t - posT; posT = t; M.srcs.forEach(s => { if (s.playing && s.pos != null) s.pos = Math.min(s.pos + dt, (s.len || 225) - 1); }); });
  const posOf = s => { if (s.pos == null) s.pos = 72; if (!s.len) s.len = 225; return s.pos; };
  SD.cmd.set_volume = a => {
    const s = SD.src(a.source); if (!s || !a.steps) return;
    SD.later(80, () => {
      if (isTab(s) && site(s) === 'soundcloud') return toast('SoundCloud’s volume only moves with its own slider');
      s.vol = Math.min(1, Math.max(0, (s.vol == null ? 0.5 : s.vol) + a.steps * 0.05));
      toast(`${s.name} volume ${Math.round(s.vol * 100)}%`);
    });
  };
  SD.cmd.seek = a => {
    const s = SD.src(a.source); if (!s || !a.secs) return;
    SD.later(80, () => {
      const dir = a.secs > 0 ? 'Forward' : 'Back';
      if (isTab(s) && site(s) === 'soundcloud') return toast('SoundCloud jumps within a song only with its own bar');
      s.pos = Math.min(Math.max(0, posOf(s) + a.secs), s.len - 1);
      toast(isTab(s) ? `${dir} ${Math.abs(a.secs)} s` : `${dir} 10 s · ${clock(s.pos)} / ${clock(s.len)}`);
    });
  };
  const unavailable = (s, what) => isSpot(s) && what !== 'spotify' ? 'Connect Spotify in Settings to use this'
    : what === 'spotify' ? (isSpot(s) ? 'This is Spotify already' : 'Connect Spotify in Settings to continue there')
    : what === 'next' ? 'Play next works for Spotify songs' : `${s.name} doesn’t share that with other apps`;
  SD.cmd.song_action = a => {
    const s = SD.src(a.source); if (!s) return;
    SD.later(120, () => {
      if (!(s.acts || ['like', 'link', 'next', 'spotify']).includes(a.what)) return toast(unavailable(s, a.what));
      const all = [...s.hist.map((x, i) => [x, i - s.hist.length]), [s.now, 0], ...s.up.map((x, i) => [x, i + 1])];
      const [it, rel] = all.find(([x]) => x.key === a.item) || [s.now, 0];
      if (a.what === 'like') {
        if (isTab(s)) {
          if (rel !== 0) return toast('Only the song playing can be liked here');
          if (s.liked === true) return toast(`${it.title} is already liked`);
          if (s.liked == null) return toast(`${s.name} shows no like button for this`);
          SD.later(500, () => { s.liked = true; push(); toast(`Liked on ${s.name}`); });
          return;
        }
        s.likes = s.likes || (s.liked ? [s.now.key] : []);
        if (s.likes.includes(it.key)) return toast(`${it.title} is already in your Liked Songs`);
        s.likes.push(it.key); s.liked = s.likes.includes(s.now.key); push();
        return toast(`Added ${it.title} to your Liked Songs`);
      }
      if (a.what === 'link') {
        if (isTab(s) && rel !== 0) return toast('Only the song playing has a link here');
        return toast('Link copied · ' + (isTab(s) ? site(s) : 'open.spotify.com'));
      }
      if (a.what === 'next') {
        if (rel === 0) return toast('That’s the song playing');
        if (rel === 1) return toast(`${it.title} is already up next`);
        if (rel > 1) { s.up.splice(rel - 1, 1); s.up.unshift(it); push(); }
        return toast(`${it.title} plays next`);
      }
      if (a.what === 'spotify') {
        // finds the song on Spotify, starts it there at the same moment, then pauses this app
        const now = s.now, from = posOf(s), sp = M.srcs.find(isSpot);
        toast(`Looking for ${now.title} on Spotify…`);
        SD.later(1400, () => {
          if (!sp) return toast(`Couldn’t find ${now.title} on Spotify`);
          if (sp.now.key !== 'sp:' + now.key) { sp.hist.push(sp.now); sp.now = { ...now, key: 'sp:' + now.key }; }
          sp.pos = from; sp.len = s.len; sp.playing = true; s.playing = false; M.active = sp.key;
          if (sp.likes) sp.liked = sp.likes.includes(sp.now.key);
          push();
          toast(`Playing on Spotify from ${clock(from)}`);
        });
      }
    });
  };
  SD.cmd.history_list = () => M.history;
  SD.cmd.history_play = a => { const h = M.history.find(x => x.id === a.id); if (h) SD.later(200, () => { SD.flyout && SD.flyout({ ...h, name: h.app }); }); };
  SD.cmd.history_clear = () => { M.history = []; };
  // Settings
  SD.settings = { trigger: 'alt', layout: 'vertical', pause_others: true, autostart: true, client_id: '3f6c0e2a9b7d4c1e8a5f2b6d9c0e7a14', spotify_connected: true };
  SD.skins = [];
  SD.cmd.settings_get = () => ({
    ...SD.settings, look: SD.look, skins: SD.skins, redirect_uri: 'http://127.0.0.1:47231/spotify', version: '1.5.2',
    companion: { last_seen: 3, folder: 'C:\\Users\\you\\AppData\\Local\\Programs\\Skip Dat\\browser-companion', error: null, folder_exists: true },
    config_file: 'C:\\Users\\you\\AppData\\Roaming\\app.skipdat.desktop\\config.json',
  });
  SD.cmd.settings_set_trigger = a => { SD.settings.trigger = a.trigger; SD.onTrigger && SD.onTrigger(a.trigger); };
  SD.cmd.settings_set_layout = a => { SD.settings.layout = a.layout; SD.look = { ...SD.look, layout: a.layout }; SD.emit('lg-look', SD.look); };
  SD.cmd.settings_set_look = a => {
    const p = { ...a.patch };
    if (p.skin) p.skin = SD.skins.find(s => s.id === p.skin) || SD.look.skin;
    SD.look = { ...SD.look, ...p }; SD.emit('lg-look', SD.look);
  };
  SD.cmd.open_history = () => { SD.openHistory && SD.openHistory(); };
})();
