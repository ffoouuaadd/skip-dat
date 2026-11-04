// Skip Dat companion, inside the page: reads what is playing and the page's real queue, and does
// what the Skip Dat app asks (play a song from the queue, next, previous, play/pause).
// Runs in the page's own JavaScript world because YouTube's player functions live there.
// It reports only what the page actually shows; it never makes up queue entries.
(() => {
  'use strict';
  if (window.__skipDatCompanion) return;
  window.__skipDatCompanion = true;
  const TAG = '__skipdat';
  const MAX_QUEUE = 400;
  const host = location.hostname;
  const SITE = host === 'music.youtube.com' ? 'ytmusic' : host.endsWith('youtube.com') ? 'youtube' : host.endsWith('soundcloud.com') ? 'soundcloud' : '';
  if (!SITE) return;

  // ---------- small helpers ----------
  const $ = (sel, root) => (root || document).querySelector(sel);
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));
  const text = el => (el ? (el.getAttribute('title') || el.textContent || '').replace(/\s+/g, ' ').trim() : '');
  const runs = r => (r && (r.simpleText || (r.runs || []).map(x => x.text).join(''))) || '';
  const norm = s => (s || '').toLowerCase().replace(/\s+/g, ' ').trim();
  const visible = el => !!(el && el.offsetParent !== null);

  /** A picture URL at a useful size (pages often hand out tiny thumbnails). */
  function bigArt(url) {
    if (!url || typeof url !== 'string') return null;
    url = url.trim();
    if (url.startsWith('//')) url = 'https:' + url;
    if (!url.startsWith('https://')) return null;
    let m = url.match(/^https:\/\/i\d?\.ytimg\.com\/(vi|vi_webp)\/([^/]+)\//);
    if (m) return 'https://i.ytimg.com/vi/' + m[2] + '/mqdefault.jpg';
    if (/googleusercontent\.com|ggpht\.com/.test(url)) {
      if (/=w\d+-h\d+/.test(url)) return url.replace(/=w\d+-h\d+[^?#]*/, '=w320-h320-l90-rj');
      if (/=s\d+/.test(url)) return url.replace(/=s\d+[^?#]*/, '=s320');
      return url;
    }
    if (/sndcdn\.com/.test(url)) return url.replace(/-(t\d+x\d+|large|small|tiny|badge|crop)\./, '-t300x300.');
    return url;
  }
  function bgUrl(el) {
    const m = el && (el.style.backgroundImage || getComputedStyle(el).backgroundImage || '').match(/url\(["']?([^"')]+)["']?\)/);
    return m ? m[1] : null;
  }
  function biggestArtwork(list) {
    let best = null, bestSize = -1;
    for (const a of list || []) {
      // "544x544", or SoundCloud's "t500x500"
      const s = parseInt((String(a.sizes || '').match(/\d+/) || ['0'])[0], 10);
      if (s >= bestSize) { best = a.src; bestSize = s; }
    }
    return best;
  }
  function session() {
    const md = navigator.mediaSession && navigator.mediaSession.metadata;
    if (!md || !md.title) return null;
    return { title: md.title, artist: md.artist || '', album: md.album || '', art: bigArt(biggestArtwork(md.artwork)) };
  }
  const player = () => document.getElementById('movie_player');
  function mainVideo() {
    return $('#movie_player video.html5-main-video') || $('#movie_player video') || $('video.html5-main-video');
  }
  const videoPlaying = v => !!(v && !v.paused && !v.ended && v.readyState > 2);

  // ---------- YouTube Music ----------
  // The queue lives in the page's own store, the one YouTube Music's queue panel draws from: every
  // song of a playlist, album, radio or mix (it grows as a mix plays on), the autoplay songs lined
  // up after it, and which one is playing. The rows on the page are only a fallback.
  const lastThumb = t => { const a = t && t.thumbnails; return a && a.length ? a[a.length - 1].url : null; };
  const firstPart = s => (s || '').split(' \u2022 ')[0];
  function ytmQueueEl() {
    const q = $('ytmusic-player-queue#queue') || $('#queue');
    return q && q.queue && q.queue.store && q.queue.store.store ? q : null;
  }
  function ytmRenderer(it) {
    if (!it) return null;
    if (it.playlistPanelVideoRenderer) return it.playlistPanelVideoRenderer;
    const w = it.playlistPanelVideoWrapperRenderer;   // a song that also has a video version
    return (w && w.primaryRenderer && w.primaryRenderer.playlistPanelVideoRenderer) || null;
  }
  function ytmEntry(v, raw, auto) {
    const title = runs(v.title);
    if (!title) return null;
    return { title, artist: runs(v.shortBylineText) || firstPart(runs(v.longBylineText)), art: bigArt(lastThumb(v.thumbnail)), id: v.videoId || '', raw, auto, selected: !!v.selected };
  }
  function ytmList() {
    const q = ytmQueueEl();
    let st = null;
    try { st = q && q.queue.store.store.getState().queue; } catch (e) { st = null; }
    if (st && Array.isArray(st.items)) {
      const list = [];
      let index = -1;
      st.items.forEach((it, raw) => {
        const e = ytmEntry(ytmRenderer(it) || {}, raw, false);
        if (!e) return;
        if (e.selected && index < 0) index = list.length;
        list.push(e);
      });
      if (index < 0 && typeof st.selectedItemIndex === 'number') index = list.findIndex(e => e.raw === st.selectedItemIndex);
      // with autoplay on, these play once the queue runs out
      if (st.autoplay !== false && Array.isArray(st.automixItems)) {
        st.automixItems.forEach((it, k) => { const e = ytmEntry(ytmRenderer(it) || {}, k, true); if (e) list.push(e); });
      }
      return { list, index, store: q };
    }
    // fallback: the rows of the queue panel
    const root = $('ytmusic-player-queue') || $('#queue');
    const list = [];
    let index = -1;
    if (root) {
      $$('ytmusic-player-queue-item', root).filter(el => !el.closest('#counterpart-renderer')).forEach((el, raw) => {
        const d = el.data || (el.__data && el.__data.data) || null;
        const title = (d && runs(d.title)) || text($('.song-title', el));
        if (!title) return;
        const img = $('img', el);
        const e = { title, artist: (d && runs(d.shortBylineText)) || text($('.byline', el)), art: bigArt(lastThumb(d && d.thumbnail) || (img && img.src)),
          id: (d && d.videoId) || '', raw, auto: !!el.closest('#automix-contents'), el, selected: el.hasAttribute('selected') || !!(d && d.selected) };
        if (e.selected && index < 0) index = list.length;
        list.push(e);
      });
    }
    return { list, index, store: null };
  }
  function ytmState() {
    const p = player();
    const vd = p && p.getVideoData ? p.getVideoData() : null;
    const ms = session();
    const { list, index: sel } = ytmList();
    let index = sel;
    // the playing song by its video id; the highlighted row can lag a moment behind a skip
    if (vd && vd.video_id && (index < 0 || (list[index].id && list[index].id !== vd.video_id))) {
      const j = list.findIndex(e => e.id === vd.video_id);
      if (j >= 0) index = j;
    }
    let title = ms ? ms.title : '', artist = ms ? ms.artist : '', album = ms ? ms.album : '', art = ms ? ms.art : null;
    // the media session can lag behind, and shows the ad during ads; the player itself is never stale
    if (vd && vd.title && norm(vd.title) !== norm(title)) {
      title = vd.title; artist = vd.author || ''; album = '';
      art = index >= 0 ? list[index].art : (vd.video_id ? bigArt('https://i.ytimg.com/vi/' + vd.video_id + '/x.jpg') : null);
    }
    const v = mainVideo();
    return {
      site: SITE, category: '', title, artist, album, art,
      playing: videoPlaying(v),
      hasMedia: !!title,
      queue: index >= 0 ? list.slice(0, MAX_QUEUE).map(e => ({ title: e.title, artist: e.artist, art: e.art })) : [],
      index: index >= 0 && index < MAX_QUEUE ? index : -1,
      canJump: index >= 0,
    };
  }
  function ytmJump(index, want) {
    const { list, store } = ytmList();
    let t = list[index];
    if (!t || (want && norm(t.title) !== want)) t = list.find(e => norm(e.title) === want);
    if (!t) return false;
    if (store && !t.auto) {
      // the queue's own "play this one" (what clicking a row in YouTube Music does)
      store.dispatch({ type: 'SET_INDEX', payload: t.raw });
      return true;
    }
    const el = t.el || $$('#automix-contents ytmusic-player-queue-item')[t.raw];
    if (!el) return false;
    click($('ytmusic-play-button-renderer', el)) || click($('.song-info', el)) || el.click();
    return true;
  }

  // ---------- YouTube ----------
  const watchHistory = [];
  // A playlist, mix ("Mix - …", it grows as it plays) or queue ("Add to queue") shows in the
  // playlist panel; its data holds every entry, also rows the page hasn't drawn yet.
  function ytPanel() {
    const panels = $$('ytd-playlist-panel-renderer#playlist');
    const hasData = p => p.data && Array.isArray(p.data.contents) && p.data.contents.length;
    return panels.find(p => visible(p) && hasData(p)) || panels.find(hasData)
      || panels.find(p => visible(p) && $('ytd-playlist-panel-video-renderer', p)) || panels.find(p => $('ytd-playlist-panel-video-renderer', p)) || null;
  }
  function ytRows(panel) { return panel ? $$('#items > ytd-playlist-panel-video-renderer', panel) : []; }
  function ytList() {
    const panel = ytPanel();
    const list = [];
    let index = -1;
    if (!panel) return { list, index, panel };
    const d = panel.data;
    if (d && Array.isArray(d.contents) && d.contents.length) {
      d.contents.forEach((c, raw) => {
        const v = c && c.playlistPanelVideoRenderer;
        if (!v) return;
        const title = runs(v.title);
        if (!title) return;
        if (v.selected && index < 0) index = list.length;
        list.push({ title, artist: runs(v.shortBylineText) || runs(v.longBylineText), art: bigArt(v.videoId ? 'https://i.ytimg.com/vi/' + v.videoId + '/x.jpg' : lastThumb(v.thumbnail)), id: v.videoId || '', raw });
      });
      if (index < 0 && typeof d.currentIndex === 'number') index = list.findIndex(e => e.raw === d.currentIndex);
      return { list, index, panel };
    }
    ytRows(panel).forEach((el, raw) => {
      const v = el.data || null;
      const title = (v && runs(v.title)) || text($('#video-title', el));
      if (!title) return;
      const img = $('img', el);
      if ((el.hasAttribute('selected') || (v && v.selected)) && index < 0) index = list.length;
      list.push({ title, artist: (v && runs(v.shortBylineText || v.longBylineText)) || text($('#byline', el)), art: bigArt(v && v.videoId ? 'https://i.ytimg.com/vi/' + v.videoId + '/x.jpg' : img && img.src), id: (v && v.videoId) || '', raw, el });
    });
    return { list, index, panel };
  }
  function ytJumpTo(t, panel) {
    const p = player();
    let ids = null;
    try { ids = p && p.getPlaylist && p.getPlaylist(); } catch (e) { ids = null; }
    // the player's own playlist, when it lines up with the panel
    if (ids && t.id && ids[t.raw] === t.id && p.playVideoAt) { p.playVideoAt(t.raw); return true; }
    const el = t.el || ytRows(panel)[t.raw];
    if (!el) return false;
    return click($('a#wc-endpoint', el)) || click($('a', el)) || (el.click(), true);
  }
  function ytNextUp() {
    const b = $('.ytp-next-button');
    // (YouTube hides the button in a small player; its data still says what plays next)
    if (!b || b.getAttribute('aria-disabled') === 'true') return null;
    const title = (b.getAttribute('data-tooltip-text') || '').trim();
    if (!title || /^next/i.test(title)) return null;
    const href = b.getAttribute('href') || '';
    const vid = (href.match(/[?&]v=([\w-]{6,})/) || [])[1];
    return { title, artist: '', art: bigArt(vid ? 'https://i.ytimg.com/vi/' + vid + '/x.jpg' : b.getAttribute('data-preview')) };
  }
  function ytState() {
    const p = player();
    const vd = p && p.getVideoData ? p.getVideoData() : null;
    const app = $('ytd-app');
    const onWatch = location.pathname === '/watch' || !!(app && app.hasAttribute('miniplayer-is-active'));
    if (!vd || !vd.video_id || !onWatch) return { site: SITE, hasMedia: false, index: -1 };
    let category = '';
    try {
      const pr = p.getPlayerResponse && p.getPlayerResponse();
      category = (pr && pr.microformat && pr.microformat.playerMicroformatRenderer && pr.microformat.playerMicroformatRenderer.category) || '';
    } catch (e) { /* the player is between videos */ }
    const url = 'https://www.youtube.com/watch?v=' + vd.video_id;
    if (watchHistory[watchHistory.length - 1] !== url) { watchHistory.push(url); if (watchHistory.length > 20) watchHistory.shift(); }
    const { list: entries, index: sel } = ytList();
    let index = sel;
    // the playing video by its id; the highlighted row can lag a moment behind
    if (index < 0 || (entries[index].id && entries[index].id !== vd.video_id)) {
      const j = entries.findIndex(e => e.id === vd.video_id);
      if (j >= 0) index = j;
    }
    const queue = entries.slice(0, MAX_QUEUE).map(e => ({ title: e.title, artist: e.artist, art: e.art }));
    if (index >= MAX_QUEUE) index = -1;
    let canJump = index >= 0;
    let list = index >= 0 ? queue : [];
    if (index < 0) {
      // not in a playlist: the one song YouTube says plays next (autoplay)
      const n = ytNextUp();
      if (n) { list = [n]; canJump = true; }
    }
    return {
      site: SITE, category,
      title: vd.title || '', artist: vd.author || '', album: '',
      art: bigArt('https://i.ytimg.com/vi/' + vd.video_id + '/x.jpg'),
      playing: videoPlaying(mainVideo()),
      hasMedia: !!vd.title,
      queue: list, index, canJump,
    };
  }

  // ---------- SoundCloud ----------
  function scQueueItems() {
    return $$('.queue .queueItemView, .queue__itemsContainer .queueItemView').filter((el, i, a) => a.indexOf(el) === i);
  }
  function scItem(el) {
    const title = text($('.queueItemView__title a, .queueItemView__title', el));
    const artist = text($('.queueItemView__username, .queueItemView__userName, .queueItemView__user', el));
    const art = bigArt(bgUrl($('.queueItemView__artwork [style*="background-image"], .queueItemView__artwork span', el)));
    const cls = el.className || '';
    // SoundCloud marks the song playing "m-active", what's next "m-upcoming", played ones "m-previous"
    const current = /\bm-active\b/.test(cls) || el.getAttribute('aria-current') === 'true';
    return { title, artist, art, current };
  }
  // SoundCloud builds its "Next up" list the first time the panel opens, then keeps it up to date.
  // Open it once, out of sight, so the list exists without anyone having to open it.
  let scPrimedAt = 0;
  function scPrimeQueue() {
    if (Date.now() - scPrimedAt < 60000) return;
    const toggle = $('.playbackSoundBadge__showQueue');
    const q = $('.queue');
    if (!toggle || !q || q.classList.contains('m-visible')) return;
    scPrimedAt = Date.now();
    const hide = document.createElement('style');
    hide.textContent = '.queue{opacity:0!important;pointer-events:none!important;transition:none!important}';
    document.head.appendChild(hide);
    toggle.click();
    setTimeout(() => { if (q.classList.contains('m-visible')) toggle.click(); }, 700);
    setTimeout(() => { hide.remove(); soon(100); }, 1200);
  }
  function scState() {
    const ms = session();
    const badgeTitle = $('.playbackSoundBadge__titleLink');
    const title = (ms && ms.title) || text(badgeTitle) || '';
    const artist = (ms && ms.artist) || text($('.playbackSoundBadge__lightLink')) || '';
    const art = (ms && ms.art) || bigArt(bgUrl($('.playbackSoundBadge__avatar [style*="background-image"], .playbackSoundBadge .image span')));
    const playBtn = $('.playControls .playControl, .playControl');
    const playing = playBtn ? playBtn.classList.contains('playing') : navigator.mediaSession && navigator.mediaSession.playbackState === 'playing';
    const queue = [];
    let index = -1;
    scQueueItems().slice(0, MAX_QUEUE).forEach(el => {
      const it = scItem(el);
      if (!it.title) return;
      if (it.current && index < 0) index = queue.length;
      queue.push({ title: it.title, artist: it.artist, art: it.art });
    });
    if (index >= 0 && norm(queue[index].title) !== norm(title)) index = queue.findIndex(q => norm(q.title) === norm(title));
    if (title && !queue.length) scPrimeQueue();
    return {
      site: SITE, category: '', title, artist, album: '', art,
      playing: !!playing,
      hasMedia: !!title,
      queue: index >= 0 ? queue : [], index, canJump: index >= 0,
    };
  }

  const state = () => (SITE === 'ytmusic' ? ytmState() : SITE === 'youtube' ? ytState() : scState());

  // ---------- doing what the app asks ----------
  function click(el) { if (el) { el.click(); return true; } return false; }
  function playPause(want) {
    if (SITE === 'soundcloud') {
      const b = $('.playControls .playControl, .playControl');
      const playing = b && b.classList.contains('playing');
      if (want === undefined || want !== playing) click(b);
      return;
    }
    const v = mainVideo();
    const p = player();
    const playing = videoPlaying(v);
    const play = want === undefined ? !playing : want;
    if (play === playing) return;
    if (p && p.playVideo && p.pauseVideo) { play ? p.playVideo() : p.pauseVideo(); return; }
    if (v) { play ? v.play().catch(() => {}) : v.pause(); }
  }
  function next() {
    if (SITE === 'soundcloud') return click($('.playControls__next, .skipControl__next'));
    const p = player();
    // the player's own "next": the next song of the list, or YouTube's autoplay pick
    if (p && p.nextVideo) return p.nextVideo();
    click($('ytmusic-player-bar .next-button, .ytp-next-button'));
  }
  function previous() {
    if (SITE === 'soundcloud') return click($('.playControls__prev, .skipControl__previous'));
    const p = player();
    if (SITE === 'youtube' && !ytList().list.length) {
      // a single video has no "previous" in YouTube itself: go back to the video before it
      if (watchHistory.length >= 2) history.back();
      return;
    }
    if (p && p.previousVideo) return p.previousVideo();
    click($('ytmusic-player-bar .previous-button, .ytp-prev-button'));
  }
  function jump(index, title) {
    const want = norm(title);
    let els = [];
    let info = () => ({});
    if (SITE === 'ytmusic') { ytmJump(index, want); return; }
    if (SITE === 'youtube') {
      const { list, panel } = ytList();
      let t = list[index];
      if (!t || (want && norm(t.title) !== want)) t = list.find(e => norm(e.title) === want);
      if (t) { ytJumpTo(t, panel); return; }
      // YouTube's autoplay "next" song is not in any list: it is the Next button
      if (!list.length) { const n = ytNextUp(); if (n && (!want || norm(n.title) === want)) next(); }
      return;
    }
    els = scQueueItems(); info = scItem;
    const listed = els.map(el => ({ el, it: info(el) })).filter(x => x.it.title);
    let target = listed[index];
    // the list may have changed since the app saw it: go by the song's name, not just its place
    if (!target || (want && norm(target.it.title) !== want)) target = listed.find(x => norm(x.it.title) === want);
    if (!target) return;
    const el = target.el;
    // the row's own play button (its title is a link to the track's page, not "play")
    click($('.queueItemView__playButton .sc-button-play, .sc-button-play, .queueItemView__play', el)) || el.click();
  }
  function run(cmd) {
    try {
      switch (cmd && cmd.type) {
        case 'play': playPause(true); break;
        case 'pause': playPause(false); break;
        case 'toggle': playPause(); break;
        case 'next': next(); break;
        case 'previous': previous(); break;
        case 'jump': jump(cmd.index | 0, cmd.title || ''); break;
        default: return;
      }
    } catch (e) { console.warn('[Skip Dat]', e); }
    // report the result as soon as the page has caught up
    setTimeout(check, 250); setTimeout(check, 900);
  }
  window.addEventListener('message', e => {
    if (e.source !== window || !e.data) return;
    // the bridge started (or reconnected) after our last report: send everything again
    if (e.data[TAG] === 'resend') { check(true); return; }
    if (e.data[TAG] !== 'cmd') return;
    // commands for the same tab arrive in order; small gaps let the page catch up between skips
    let delay = 0;
    for (const c of e.data.cmds || []) { setTimeout(() => run(c), delay); delay += c.type === 'next' || c.type === 'previous' ? 180 : 40; }
  });

  // ---------- reporting changes ----------
  let lastSent = '';
  let timer = 0;
  function check(force) {
    timer = 0;
    let s;
    try { s = state(); } catch (e) { return; }
    const json = JSON.stringify(s);
    if (json === lastSent && !force) return;
    lastSent = json;
    window.postMessage({ [TAG]: 'state', report: s }, location.origin);
  }
  function soon(ms) { if (!timer) timer = setTimeout(check, ms || 200); }
  ['play', 'pause', 'ended', 'loadeddata', 'emptied'].forEach(ev => document.addEventListener(ev, () => soon(120), true));
  ['yt-navigate-finish', 'yt-page-data-updated', 'yt-player-updated'].forEach(ev => document.addEventListener(ev, () => soon(150)));
  window.addEventListener('pageshow', e => { if (e.persisted) check(true); });
  // the queue and the now-playing bar change without media events (reordering, adding songs)
  let watched = null;
  const mo = new MutationObserver(() => soon(300));
  // YouTube Music announces every queue change (a mix growing, a song added, a skip)
  let storeWatched = null;
  function watchStore() {
    const q = SITE === 'ytmusic' ? ytmQueueEl() : null;
    if (q && q !== storeWatched) {
      try { q.queue.store.store.subscribe(() => soon(200)); storeWatched = q; } catch (e) { /* not available */ }
    }
  }
  function watchQueue() {
    watchStore();
    const target = SITE === 'ytmusic' ? $('ytmusic-player-queue') : SITE === 'youtube' ? $('ytd-playlist-panel-renderer#playlist') : $('.playControls');
    if (target && target !== watched) {
      mo.disconnect();
      mo.observe(target, { childList: true, subtree: true, attributes: true, attributeFilter: ['selected', 'class', 'title'] });
      watched = target;
    }
  }
  setInterval(() => { watchQueue(); check(); }, 1000);
  check();
})();
