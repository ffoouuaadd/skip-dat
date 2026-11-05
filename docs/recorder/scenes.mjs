// What each GIF shows. Coordinates are CSS px on a 1000 x 625 desktop.
export default ({ YTM, SPOT, SC, LOOK, SKINS }) => ({

  // the showcase at the top of the page
  async hero(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(250, 470);
    await c.caption({ title: 'Hold Alt + scroll', sub: 'every music app’s queue, one wheel', keys: ['Alt'] });
    await c.moveTo(380, 410, 0.8); await c.wait(1.0);
    await c.keys(true); await c.wait(0.3);
    await c.open(); await c.wait(0.5);
    await c.scroll(3, 0.32); await c.wait(0.6);
    await c.flyTag('jumped straight there');
    await c.keys(false); await c.commit(); await c.wait(0.45);
    await c.punch(); await c.meme('skipped 2 songs', 'heard none of them', 1.9, [92, 420]);
    await c.wait(1.9);
    await c.flyTag(null);
    await c.caption({ badge: 'NEW', title: 'Back to …', sub: 'skipped a banger by accident? one click', keys: ['Alt'] });
    await c.keys(true); await c.wait(0.25); await c.open(); await c.wait(0.9);
    await c.clickHit(/^ctl:back$/, 0.6); await c.chip('ctrl + z, but for music', 690, 470, 1.6, -3); await c.wait(1.4);
    await c.caption({ title: 'Every app in one wheel', sub: 'YouTube Music, Spotify, SoundCloud and more', keys: ['Alt'] });
    await c.app(1); await c.wait(1.3);
    await c.app(1); await c.wait(1.3);
    await c.keys(false); await c.cancel(); await c.wait(1.0);
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
    await c.keys(false); await c.commit(); await c.wait(0.5); await c.wait(2.1);
  },

  // 3. click a song, the buttons, switch apps
  async 'click-and-switch'(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(330, 470);
    await c.caption({ badge: '3', title: 'Click a song', sub: 'no thoughts, just bangers', keys: ['Alt'], mouse: false });
    await c.moveTo(380, 420, 0.5); await c.wait(0.6);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.9);
    await c.clickHit(/^card:ytm\|y4$/, 0.7); await c.wait(1.4);
    await c.caption({ badge: '3', title: 'Play, pause and skip', sub: 'the buttons beside the wheel work too', keys: ['Alt'], mouse: false });
    await c.clickHit(/^ctl:toggle$/, 0.7); await c.wait(0.9);
    await c.clickHit(/^ctl:toggle$/, 0.2); await c.wait(0.8);
    await c.clickHit(/^ctl:next$/, 0.4); await c.wait(1.2);
    await c.caption({ badge: '3', title: 'Click an app to switch', sub: 'its own queue comes in. zero alt-tabbing', keys: ['Alt'], mouse: false });
    await c.clickHit(/^src:spotify$/, 0.6); await c.wait(1.3);
    await c.clickHit(/^src:soundcloud$/, 0.4); await c.wait(1.3);
    await c.caption({ badge: '3', title: 'Esc or a click outside closes it', sub: 'nothing changes. we were never here', keys: ['Esc'], mouse: false });
    await c.wait(0.6); await c.keys(true); await c.cancel(); await c.wait(0.3); await c.keys(false); await c.wait(1.3);
  },

  // new in 1.4.1: Spotify goes straight to the song
  async 'spotify-jump'(c) {
    const spot = { ...SPOT, playing: true }, ytm = { ...YTM, playing: false };
    await c.music([ytm, spot, SC], 'spotify'); await c.showNow();
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
    const spot = { ...SPOT, playing: true }, ytm = { ...YTM, playing: false };
    await c.music([ytm, spot, SC], 'spotify');
    await c.page.evaluate(() => { const SD = window.__SD; SD.cmd.choose({ source: 'spotify', item: 's7' }); });
    await c.quiet(0.5); await c.showNow();
    await c.ptr(300, 470);
    await c.caption({ badge: 'NEW', title: 'Back to …', sub: 'under play/pause: the song you were on before', keys: ['Alt'] });
    await c.moveTo(380, 420, 0.6); await c.wait(0.9);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(1.1);
    await c.clickHit(/^ctl:back$/, 0.8, 0.35); await c.wait(0.3);
    await c.punch(0.035); await c.meme('ctrl + z', 'but for music', 1.7, [92, 420]); await c.wait(1.5);
    await c.caption({ badge: 'NEW', title: 'Click again to return', sub: 'indecisive? same', keys: ['Alt'] });
    await c.wait(0.4);
    await c.clickHit(/^ctl:back$/, 0.25, 0.3); await c.wait(1.6);
    await c.keys(false); await c.cancel(); await c.wait(1.0);
  },

  // styles
  async styles(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(400, 420);
    const list = [['wheel', 'Wheel', 'the classic drum'], ['stack', 'Stack', 'a tidy little pile of cards'], ['coverflow', 'Cover flow', 'very 2008, still hard'],
      ['orbit', 'Orbit', 'your songs orbit your cursor'], ['compact', 'Compact', 'just the titles. minimalist king']];
    await c.caption({ title: 'Style: ' + list[0][1], sub: list[0][2] });
    await c.wait(0.4); await c.open(); await c.wait(0.6);
    for (let i = 0; i < list.length; i++) {
      if (i) { await c.setLook({ style: list[i][0] }); await c.caption({ title: 'Style: ' + list[i][1], sub: list[i][2] }); await c.wait(0.5); }
      await c.scroll(2, 0.3); await c.scroll(-2, 0.3); await c.wait(0.4);
    }
    await c.cancel(); await c.wait(1.0);
  },

  // skins
  async skins(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(400, 420);
    const about = { 'liquid-glass': 'frosted glass, the default drip', 'lime-pod': 'glossy lime plastic. 2004 called', 'lcd-night': 'pocket-player green, scanlines included',
      'brushed-chrome': 'brushed metal and a pixel font', 'pixel-party': '8-bit cave. the torch flickers' };
    await c.caption({ title: 'Skin: ' + SKINS[0].name, sub: about[SKINS[0].id] });
    await c.wait(0.4); await c.open(); await c.wait(0.6);
    for (let i = 0; i < SKINS.length; i++) {
      if (i) { await c.setLook({ skinId: SKINS[i].id }); await c.caption({ title: 'Skin: ' + SKINS[i].name, sub: about[SKINS[i].id] }); await c.wait(0.6); }
      await c.scroll(2, 0.3); await c.scroll(-1, 0.3); await c.wait(0.5);
    }
    await c.cancel(); await c.wait(1.0);
  },

  // What was I playing? (tray menu)
  async history(c) {
    const ytm = { ...YTM, playing: true };
    await c.music([ytm, SPOT, SC], 'ytm'); await c.showNow();
    await c.page.evaluate(() => {
      const now = Math.floor(Date.now() / 1000), H = window.__SD.M.history;
      const row = (t, a, art, app, via, ago) => ({ id: 'p' + H.length, title: t, artist: a, art, app, via, time: now - ago });
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
    await c.page.evaluate(() => window.__SD.openHistory()); await c.wait(1.0); await c.wait(0.4);
    await c.caption({ title: 'Click one to play it again', sub: '3 AM you, meet 3 PM you. it stays on your PC' });
    await c.clickIn('history', '.song:nth-of-type(4)', 0.9); await c.wait(2.2);
  },

  // 4. your own shortcut
  async shortcut(c) {
    await c.music([YTM, SPOT, SC], 'ytm'); await c.showNow();
    await c.ptr(640, 300);
    await c.caption({ badge: '4', title: 'Pick your own shortcut', sub: 'double-click Skip Dat in the tray for Settings' });
    const [tx, ty] = await c.page.evaluate(() => window.__SD.trayAt());
    await c.glide(tx, ty, 0.9); await c.wait(0.2);
    await c.clickFx(); await c.wait(0.12); await c.clickFx();
    await c.page.evaluate(() => window.__SD.openWindow('settings', 40, 22, 500, 540)); await c.wait(1.2);
    await c.scrollWin('settings', '#h-key', 0.8); await c.wait(0.3);
    await c.clickIn('settings', '[data-trigger="ctrl+alt"]', 0.9); await c.wait(1.2);
    await c.page.evaluate(() => window.__SD.closeWindow('settings')); await c.wait(0.4);
    await c.caption({ badge: '4', title: 'Now hold Ctrl + Alt and scroll', sub: 'works right away. no restart', keys: ['Ctrl', 'Alt'] });
    await c.glide(400, 420, 0.8); await c.wait(0.3);
    await c.keys(true); await c.wait(0.3); await c.open(); await c.wait(0.5);
    await c.scroll(3, 0.34); await c.wait(0.5);
    await c.keys(false); await c.commit(); await c.wait(0.4); await c.punch(0.03); await c.wait(2.0);
  },
});
