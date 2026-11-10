// What each GIF shows. Coordinates are CSS px on a 1000 x 625 desktop.
// Settings floats on Windows (no frame, only its glass panels): it opens on the right, clear of the captions.
const SET = [442, 6, 548, 570];

export default ({ YTM, SPOT, SC, LOOK, SKINS }) => {
  const spotOn = () => [{ ...YTM, playing: false }, { ...SPOT, playing: true }, SC];
  // double-click Skip Dat in the tray: Settings opens
  async function trayToSettings(c, dur = 0.9) {
    const [tx, ty] = await c.page.evaluate(() => window.__SD.trayAt());
    await c.glide(tx, ty, dur); await c.wait(0.2);
    await c.clickFx(); await c.wait(0.12); await c.clickFx();
    await c.openWindow('settings', ...SET, { float: true });
  }
  // click a song button, then step off the wheel's side so the message under the buttons shows
  async function press(c, what, dur = 0.5) {
    const re = new RegExp('^act:' + what + '$');
    await c.clickHit(re, dur, 0.15);
    const [x, y] = await c.hit(re);
    await c.wait(0.1); await c.glide(x + 96, y + 6, 0.3);
  }
  return {

  // the showcase at the top of the page
  async hero(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(250, 470);
    await c.caption({ title: 'Hold Alt + scroll', sub: 'every music app’s queue, one wheel', keys: ['Alt'] });
    await c.moveTo(380, 410, 0.8); await c.wait(0.9);
    await c.keys(true); await c.wait(0.25);
    await c.open(); await c.wait(0.45);
    await c.scroll(3, 0.3); await c.wait(0.45);
    await c.flyTag('jumped straight there');
    await c.keys(false); await c.commit(); await c.wait(0.45);
    await c.punch(); await c.meme('skipped 2 songs', 'heard none of them', 1.7, [92, 420]);
    await c.wait(1.7);
    await c.flyTag(null);
    await c.caption({ badge: 'NEW', title: 'Song buttons', sub: 'like it, copy the link, take it to Spotify', keys: ['Alt'] });
    await c.keys(true); await c.wait(0.2); await c.open(); await c.wait(0.6);
    await press(c, 'like', 0.6);
    await c.chip('certified banger', 640, 500, 1.5, -3); await c.wait(1.3);
    await c.caption({ title: 'Every app in one wheel', sub: 'YouTube Music, Spotify, SoundCloud and more', keys: ['Alt'] });
    await c.app(1); await c.wait(1.2);
    await c.app(1); await c.wait(1.2);
    await c.keys(false); await c.cancel(); await c.wait(0.9);
  },

  // 2. hold, scroll, let go
  async 'hold-scroll-release'(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(300, 480);
    await c.caption({ badge: '2', title: 'Hold Alt, then scroll', sub: 'it pops out of your cursor. no cap', keys: ['Alt'] });
    await c.moveTo(400, 410, 0.7); await c.wait(0.9);
    await c.keys(true); await c.wait(0.4);
    await c.open(); await c.wait(0.6);
    await c.scroll(4, 0.36); await c.wait(0.3);
    await c.caption({ badge: '2', title: 'Scroll up for what already played', sub: 'one song per notch', keys: ['Alt'] });
    await c.scroll(-6, 0.3); await c.wait(0.3);
    await c.scroll(4, 0.3); await c.wait(0.5);
    await c.caption({ badge: '2', title: 'Let go to play it', sub: 'and the wheel yeets itself back into your cursor', keys: ['Alt'] });
    await c.wait(0.6);
    await c.keys(false); await c.commit(); await c.wait(2.4);
  },

  // 3. click a song, the buttons, switch apps
  async 'click-and-switch'(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(330, 470);
    await c.caption({ badge: '3', title: 'Click a song', sub: 'no thoughts, just bangers', keys: ['Alt'], mouse: false });
    await c.moveTo(380, 420, 0.5); await c.wait(0.6);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.8);
    await c.clickHit(/^card:ytm\|y4$/, 0.7); await c.wait(1.0);
    await c.caption({ badge: '3', title: 'Play, pause and skip', sub: 'the buttons beside the wheel work too', keys: ['Alt'], mouse: false });
    await c.clickHit(/^ctl:toggle$/, 0.6); await c.wait(0.65);
    await c.clickHit(/^ctl:toggle$/, 0.2); await c.wait(0.6);
    await c.clickHit(/^ctl:next$/, 0.35); await c.wait(0.95);
    await c.caption({ badge: '3', title: 'Click an app to switch', sub: 'its own queue comes in. zero alt-tabbing', keys: ['Alt'], mouse: false });
    await c.clickHit(/^src:spotify$/, 0.5); await c.wait(1.1);
    await c.caption({ badge: '3', title: 'Esc or a click outside closes it', sub: 'nothing changes. we were never here', keys: ['Esc'], mouse: false });
    await c.wait(0.45); await c.keys(true); await c.cancel(); await c.wait(0.3); await c.keys(false); await c.wait(0.9);
  },

  // new in 1.5: the wheel turns volume over play/pause, and jumps within the song over its card
  async 'volume-seek'(c) {
    await c.music(spotOn(), 'spotify'); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Scroll on play/pause: volume', sub: 'just that app. your game stays loud', keys: ['Alt'] });
    await c.moveTo(390, 420, 0.6); await c.wait(0.7);
    await c.keys(true); await c.wait(0.2); await c.open(); await c.wait(0.6);
    await c.hoverHit(/^ctl:toggle$/, 0.5); await c.wait(0.2);
    await c.scroll(-4, 0.3); await c.wait(0.3);
    await c.scroll(3, 0.3); await c.wait(0.7);
    await c.caption({ badge: 'NEW', title: 'Scroll on the song: jump 10 s', sub: 'skip the 40 second intro. finally', keys: ['Alt'] });
    await c.hoverHit(/^card:spotify\|s3$/, 0.6); await c.wait(0.2);
    await c.scroll(3, 0.45); await c.wait(0.3);
    await c.scroll(-2, 0.45); await c.wait(0.9);
    // every notch must have gone to the volume or the song, none to the wheel
    if (await c.calls('set_volume') !== 7 || await c.calls('seek') !== 5) throw new Error('volume-seek: a notch missed its button');
    await c.keys(false); await c.commit(); await c.wait(0.8);
  },

  // new in 1.5: the song buttons under play/pause
  async 'song-buttons'(c) {
    await c.music(spotOn(), 'spotify'); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Like', sub: 'the heart fills. into your Liked Songs', keys: ['Alt'] });
    await c.moveTo(390, 420, 0.5); await c.wait(0.55);
    await c.keys(true); await c.wait(0.15); await c.open(); await c.wait(0.5);
    await press(c, 'like', 0.5); await c.wait(0.9);
    await c.caption({ badge: 'NEW', title: 'Like it again', sub: 'it never unlikes. no accidents', keys: ['Alt'] });
    await press(c, 'like', 0.3); await c.wait(0.9);
    await c.caption({ badge: 'NEW', title: 'Copy link', sub: 'for the group chat', keys: ['Alt'] });
    await press(c, 'link', 0.35); await c.wait(0.8);
    await c.caption({ badge: 'NEW', title: 'Play next', sub: 'scroll to any song, it cuts the line', keys: ['Alt'] });
    await c.scroll(3, 0.28); await c.wait(0.1);
    await press(c, 'next', 0.35); await c.wait(1.1);
    await c.keys(false); await c.cancel(); await c.wait(0.8);
  },

  // new in 1.5: take the song playing to Spotify, at the same moment
  async 'continue-spotify'(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Continue on Spotify', sub: 'same song, same second. from YouTube Music', keys: ['Alt'] });
    await c.moveTo(390, 420, 0.6); await c.wait(0.8);
    await c.keys(true); await c.wait(0.25); await c.open(); await c.wait(0.7);
    await c.hoverHit(/^act:spotify$/, 0.8); await c.wait(0.5);
    await c.clickFx(); await c.emit('lg-click', 'act:spotify'); await c.busy(2.6);
    await c.wait(0.12); await c.glide(...(await c.hit(/^act:spotify$/)).slice(0, 2).map((v, k) => v + [70, 6][k]), 0.35); await c.wait(1.2);
    await c.punch(0.03); await c.chip('zero seconds lost', 600, 520, 1.6, -3); await c.wait(1.8);
    await c.caption({ badge: 'NEW', title: 'YouTube Music pauses itself', sub: 'Spotify takes it from there', keys: ['Alt'] });
    await c.clickHit(/^src:spotify$/, 0.6); await c.wait(1.8);
    await c.keys(false); await c.cancel(); await c.wait(1.0);
  },

  // new in 1.4.1: Spotify goes straight to the song
  async 'spotify-jump'(c) {
    await c.music(spotOn(), 'spotify'); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Spotify, but it teleports', sub: 'straight to your song, nothing in between', keys: ['Alt'] });
    await c.moveTo(390, 420, 0.7); await c.wait(1.0);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.6);
    await c.scroll(4, 0.36); await c.wait(0.7);
    await c.flyTag('straight there');
    await c.keys(false); await c.commit(); await c.wait(0.45);
    await c.punch(); await c.meme('teleported', '0 songs in between', 1.9, [92, 420]);
    await c.wait(2.1);
    await c.flyTag(null);
    await c.caption({ badge: 'NEW', title: 'Your album keeps going', sub: 'inside the playlist or album you were playing', keys: ['Alt'] });
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.6);
    await c.scroll(2, 0.4); await c.wait(0.9);
    await c.keys(false); await c.cancel(); await c.wait(1.0);
  },

  // new in 1.4.1: Back to …
  async 'back-to'(c) {
    await c.music(spotOn(), 'spotify');
    await c.page.evaluate(() => { const SD = window.__SD; SD.cmd.choose({ source: 'spotify', item: 's7' }); });
    await c.quiet(0.5); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Back to …', sub: 'under play/pause: the song you were on before', keys: ['Alt'] });
    await c.moveTo(380, 420, 0.6); await c.wait(0.9);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(1.1);
    await c.clickHit(/^ctl:back$/, 0.8, 0.35); await c.wait(0.3);
    await c.punch(0.035); await c.chip('ctrl + z, but for music', 110, 300, 1.7, -4); await c.wait(1.5);
    await c.caption({ badge: 'NEW', title: 'Click again to return', sub: 'indecisive? same', keys: ['Alt'] });
    await c.wait(0.4);
    await c.clickHit(/^ctl:back$/, 0.25, 0.3); await c.wait(1.6);
    await c.keys(false); await c.cancel(); await c.wait(1.0);
  },

  // new in 1.5: Freaky Random
  async 'freaky-random'(c) {
    await c.music([YTM, SPOT, SC], 'ytm', { ...LOOK, style: 'random' }); await c.showNow();
    await c.ptr(330, 500); await c.seed(7);
    await c.caption({ badge: 'NEW', title: 'Freaky Random', sub: 'one big card, the queue curls round it', keys: ['Alt'] });
    await c.moveTo(420, 440, 0.7); await c.wait(0.8);
    await c.keys(true); await c.wait(0.25); await c.open(); await c.wait(1.4);
    await c.scroll(1, 0.7); await c.scroll(1, 0.7); await c.wait(0.3);
    await c.caption({ badge: 'NEW', title: 'Scroll fast', sub: 'it swirls, and still keeps the order', keys: ['Alt'] });
    await c.scroll(5, 0.08); await c.wait(0.9);
    await c.scroll(-4, 0.08); await c.wait(0.9);
    // the biggest card out in the cloud (not the one in front)
    const pick = await c.page.evaluate(() => window.__SD.hits.filter(h => h.id.startsWith('card:')).sort((a, b) => b.w * b.h - a.w * a.h)[2].id);
    await c.caption({ badge: 'NEW', title: 'Point at one', sub: 'it lifts up and says its name', keys: ['Alt'] });
    await c.hoverHit(new RegExp('^' + pick.replace(/[|]/g, '\\|') + '$'), 0.8); await c.wait(1.2);
    await c.caption({ badge: 'NEW', title: 'Click it', sub: 'it flies to the front and plays', keys: ['Alt'] });
    await c.clickHit(new RegExp('^' + pick.replace(/[|]/g, '\\|') + '$'), 0.15, 0.1); await c.wait(1.8);
    await c.keys(false); await c.commit(); await c.wait(1.1);
  },

  // styles
  async styles(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(400, 420); await c.seed(7);
    const list = [['wheel', 'Wheel', 'the classic drum'], ['stack', 'Stack', 'a tidy little pile of cards'], ['coverflow', 'Cover flow', 'very 2008, still hard'],
      ['orbit', 'Orbit', 'your songs orbit your cursor'], ['compact', 'Compact', 'just the titles. minimalist king'], ['random', 'Freaky Random', 'one big card, the rest just vibe']];
    await c.caption({ title: 'Style: ' + list[0][1], sub: list[0][2] });
    await c.wait(0.4); await c.open(); await c.wait(0.6);
    for (let i = 0; i < list.length; i++) {
      if (i) { await c.setLook({ style: list[i][0] }); await c.caption({ badge: list[i][0] === 'random' ? 'NEW' : undefined, title: 'Style: ' + list[i][1], sub: list[i][2] }); await c.wait(0.4); }
      await c.scroll(2, 0.25); await c.scroll(-2, 0.25); await c.wait(0.2);
    }
    await c.cancel(); await c.wait(1.0);
  },

  // skins: each with its living details; a skip on Bubble Gum pops its bubble
  async skins(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(400, 420);
    const about = { 'liquid-glass': 'frosted glass, the default drip', 'lime-pod': 'glossy lime plastic. 2004 called', 'lcd-night': 'pocket-player green, scanlines included',
      'brushed-chrome': 'brushed metal and a pixel font', 'pixel-party': '8-bit cave. the torch flickers', quicksilver: 'liquid mercury drips over the top',
      'bare-knuckle': 'a bare bulb swings. first rule: skip', 'bubble-gum': '8-bit gum. every skip pops it', 'claude-ish': 'cream paper, a serif, a spark that keeps thinking' };
    const fresh = ['quicksilver', 'bare-knuckle', 'bubble-gum', 'claude-ish'];
    const cap = s => c.caption({ badge: fresh.includes(s.id) ? 'NEW' : undefined, title: 'Skin: ' + s.name, sub: about[s.id] });
    // the mercury is about to drip and the bubble is blown up when they come on (the skip then pops it)
    await c.page.evaluate(() => { window.__SD.phase = { 'mercury.webp': 3900, 'gum.png': 5300 }; });
    await cap(SKINS[0]);
    await c.wait(0.4); await c.open(); await c.wait(0.6);
    for (let i = 0; i < SKINS.length; i++) {
      if (i) { await c.setLook({ skinId: SKINS[i].id }); await cap(SKINS[i]); await c.wait(0.45); }
      // a skip replays Bare Knuckle's film splice and pops Bubble Gum's bubble
      if (SKINS[i].id === 'bubble-gum' || SKINS[i].id === 'bare-knuckle') { await c.clickHit(/^ctl:next$/, 0.4, 0.1); await c.busy(1.8); await c.wait(1.25); }
      else await c.scroll(1, SKINS[i].id === 'quicksilver' ? 1.1 : 0.7);
    }
    await c.cancel(); await c.wait(1.0);
  },

  // new in 1.5: Liquid Glass in your own colour
  async 'glass-colour'(c) {
    await c.music([YTM, SPOT, SC], 'ytm');
    await c.ptr(300, 330);
    await c.caption({ badge: 'NEW', title: 'Liquid Glass, any colour', sub: 'Settings, Skin, Change' });
    await c.wait(0.3); await c.openWindow('settings', ...SET, { float: true }); await c.wait(0.6);
    await c.scrollWin('settings', '.pickrow', 0.7);
    await c.clickIn('settings', '#skin-open', 0.8); await c.wait(0.8);
    await c.caption({ badge: 'NEW', title: 'Drag the colour', sub: 'the whole rainbow. pick your fighter' });
    await c.dragSlider('settings', '#glass-hue', 0.8, 2.2); await c.wait(0.5);
    await c.caption({ title: 'Your wheel, your colour', sub: 'it matches your setup now', keys: ['Alt'] });
    await c.glide(720, 440, 0.9); await c.wait(0.2);
    await c.keys(true); await c.wait(0.2); await c.open(); await c.wait(0.6);
    await c.scroll(2, 0.4); await c.wait(1.2);
    await c.keys(false); await c.cancel(); await c.wait(0.4);
    await c.caption({ title: 'Original', sub: 'back to clear glass in one click' });
    await c.clickIn('settings', '#hue-reset', 0.9); await c.wait(0.6);
    await c.glide(720, 440, 0.8); await c.keys(true); await c.open(); await c.wait(1.2);
    await c.keys(false); await c.cancel(); await c.wait(0.9);
  },

  // new in 1.5: Settings floats, Look and Skin open in panels
  async settings(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(640, 300);
    await c.caption({ badge: 'NEW', title: 'Settings floats now', sub: 'no window, just glass panels' });
    await trayToSettings(c); await c.wait(1.0);
    await c.scrollWin('settings', '.pickrow', 0.8); await c.wait(0.2);
    await c.caption({ badge: 'NEW', title: 'Look: Change', sub: 'style, size and depth, in their own panel', keys: ['Ctrl', 'L'], mouse: false });
    await c.clickIn('settings', '#look-open', 0.8); await c.wait(1.0);
    await c.clickIn('settings', '[data-style="orbit"]', 0.8); await c.wait(1.2);
    await c.caption({ badge: 'NEW', title: 'Ctrl + K: the Skin panel', sub: 'one panel at a time. no clutter', keys: ['Ctrl', 'K'], mouse: false });
    await c.wait(0.4); await c.keys(true); await c.key('k', ['Control']); await c.wait(0.25); await c.keys(false); await c.wait(1.4);
    await c.caption({ badge: 'NEW', title: 'Esc closes it', sub: 'and the glass steps forward again', keys: ['Esc'], mouse: false });
    await c.wait(0.4); await c.keys(true); await c.key('Escape'); await c.wait(0.25); await c.keys(false); await c.wait(1.2);
    await c.scrollWin('settings', null, 0.7);
    await c.clickIn('settings', '#win-close', 0.8); await c.wait(1.0);
  },

  // What was I playing? (tray menu)
  async history(c) {
    const ytm = { ...YTM, playing: true };
    await c.music([ytm, SPOT, SC], 'ytm'); await c.showNow();
    await c.page.evaluate(() => {
      const now = Math.floor(Date.now() / 1000), H = window.__SD.M.history;
      let id = 0; const row = (t, a, art, app, via, ago) => ({ id: 'p' + id++, title: t, artist: a, art, app, via, time: now - ago });
      H.push(row('Ochre Against Blue', 'Memotone', 'cc-memotone.jpg', 'YouTube Music', 'Chrome', 20), row('Easy', 'Ben Mason', 'cc-ben-mason.jpg', 'YouTube Music', 'Chrome', 260),
        row('Lights in the Sky', 'Nine Inch Nails', 'nin-slip.jpg', 'Spotify', '', 900), row('Plumy Tale', 'Dumbo Gets Mad', 'cc-dumbo-prawn.jpg', 'YouTube Music', 'Chrome', 1900),
        row('6 Ghosts I', 'Nine Inch Nails', 'nin-ghosts.jpg', 'SoundCloud', 'Chrome', 4300), row('Been A While Since You Spoke', 'Handbook', 'cc-handbook.jpg', 'YouTube Music', 'Chrome', 9000),
        row('Discipline', 'Nine Inch Nails', 'nin-slip.jpg', 'Spotify', '', 90000), row('Balancing Act', 'The Underscore Orkestra', 'cc-underscore.jpg', 'YouTube Music', 'Chrome', 95000));
    });
    await c.ptr(600, 300);
    await c.caption({ title: 'What was I playing?', sub: 'right-click Skip Dat in the tray' });
    const [tx, ty] = await c.page.evaluate(() => window.__SD.trayAt());
    await c.glide(tx, ty, 0.9); await c.wait(0.3);
    await c.clickFx(); await c.page.evaluate(() => window.__SD.menu(true)); await c.wait(0.7);
    const [mx, my] = await c.page.evaluate(() => window.__SD.menuItem('history'));
    await c.glide(mx, my, 0.5); await c.wait(0.3);
    await c.clickFx(); await c.page.evaluate(() => window.__SD.menu(false)); await c.wait(0.15);
    await c.openWindow('history', 350, 46, 470, 500); await c.wait(1.4);
    await c.caption({ title: 'Click one to play it again', sub: '3 AM you, meet 3 PM you. it stays on your PC' });
    await c.clickIn('history', '.song:nth-of-type(4)', 0.9); await c.wait(2.6);
  },

  // 4. your own shortcut
  async shortcut(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(640, 300);
    await c.caption({ badge: '5', title: 'Pick your own shortcut', sub: 'double-click Skip Dat in the tray for Settings' });
    await trayToSettings(c); await c.wait(1.0);
    await c.scrollWin('settings', '#h-key', 0.8); await c.wait(0.3);
    await c.clickIn('settings', '[data-trigger="ctrl+alt"]', 0.9); await c.wait(1.2);
    await c.scrollWin('settings', null, 0.6);
    await c.clickIn('settings', '#win-close', 0.7); await c.wait(0.4);
    await c.caption({ badge: '5', title: 'Now hold Ctrl + Alt and scroll', sub: 'works right away. no restart', keys: ['Ctrl', 'Alt'] });
    await c.glide(400, 420, 0.8); await c.wait(0.3);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.5);
    await c.scroll(3, 0.34); await c.wait(0.5);
    await c.keys(false); await c.commit(); await c.wait(0.4); await c.punch(0.03); await c.wait(2.0);
  },

  // 1. install it: the setup file, the installer's pages (installer/skipdat.nsi), then the tray and Settings
  async install(c) {
    await c.music([YTM, SPOT, SC], 'ytm');
    await c.exec(() => { const A = window.__SD.apps; A.tray(false); A.desktopIcon(true); A.installer.at(360, 66); });
    await c.ptr(300, 360);
    await c.caption({ badge: '1', title: 'Run the installer', sub: 'installs for your account, no admin needed' });
    const [ix, iy] = await c.at('.ap-icon img');
    await c.glide(ix + 4, iy + 6, 0.7); await c.wait(0.1);
    await c.clickFx(); await c.exec(() => window.__SD.apps.selectIcon(true)); await c.wait(0.12); await c.clickFx(); await c.wait(0.25);
    await c.exec(() => window.__SD.apps.installer.open()); await c.wait(0.7);
    await c.clickAt('#ns-next', 0.6, 0.1); await c.wait(0.2);
    await c.caption({ badge: '1', title: 'Pick your extras', sub: 'Start menu, desktop, start with Windows' });
    await c.hoverAt('.ns-tree [data-c="auto"]', 0.5); await c.wait(0.8);
    await c.clickAt('#ns-next', 0.45, 0.1); await c.wait(0.4);
    await c.clickAt('#ns-next', 0.3, 0.1); await c.busy(3.4);
    await c.caption({ badge: '1', title: 'Installing', sub: 'a few seconds. the soap bar vibes meanwhile' });
    await c.glide(250, 440, 0.6); await c.wait(2.45);
    await c.caption({ badge: '1', title: 'Finish', sub: 'Skip Dat starts in the tray, near the clock' });
    await c.wait(0.2); await c.clickAt('#ns-next', 0.55, 0.1); await c.wait(0.3);
    await c.glide(820, 520, 0.5); await c.exec(() => window.__SD.apps.tray(true)); await c.wait(0.45);
    await c.caption({ badge: '1', title: 'It lives in the tray', sub: 'the first time, Settings opens by itself' });
    await c.openWindow('settings', ...SET, { float: true }); await c.wait(1.6);
    await c.clickIn('settings', '#win-close', 0.7); await c.wait(0.8);
  },

  // 8. the browser companion: the extensions page, Developer mode, Load unpacked, then Connected
  async companion(c) {
    await c.music([YTM, SPOT, SC], 'ytm');
    await c.exec(() => window.__SD.apps.browser.open(24, 78));
    await c.ptr(420, 330);
    await c.caption({ badge: '7', title: 'Open chrome://extensions', sub: 'in Edge it’s edge://extensions' });
    await c.wait(0.2); await c.clickAt('#br-url', 0.55, 0.1);
    await c.exec(() => window.__SD.apps.browser.type('chrome://extensions', 0.9)); await c.wait(1.0);
    await c.exec(() => window.__SD.apps.browser.enter()); await c.wait(0.6);
    await c.caption({ badge: '7', title: 'Turn on Developer mode', sub: 'top right. you’re a developer now' });
    await c.clickAt('#br-dev', 0.7); await c.wait(0.6);
    await c.caption({ badge: '7', title: 'Load unpacked', sub: 'pick browser-companion in Skip Dat’s folder' });
    await c.clickAt('#br-load', 0.6); await c.wait(0.7);
    await c.clickAt('#fd-row', 0.7); await c.wait(0.4);
    await c.clickAt('#fd-ok', 0.6); await c.wait(1.1);
    await c.caption({ badge: '7', title: 'Reload your music tabs', sub: 'then Settings says Connected' });
    await c.clickAt('#br-tab-ytm', 0.7); await c.wait(0.25);
    await c.clickAt('#br-reload', 0.45); await c.wait(0.6);
    await trayToSettings(c, 0.7);
    await c.scrollWin('settings', '#h-ext', 0.7, 40); await c.wait(1.3);
  },
  };
};
