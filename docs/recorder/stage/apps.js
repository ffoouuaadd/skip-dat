// Pretend apps for the install and companion GIFs, on the stage's desktop and clock:
// - a setup file on the desktop and the Skip Dat installer: the pages of installer/skipdat.nsi as
//   Modern UI 2 draws them on Windows 11 at 125 % (its header.bmp and side.bmp, BrandingText, the
//   buttons and when they're greyed), with the glass animation of installer/anim/src/main.rs
//   redrawn on a canvas, frame by frame by the virtual clock
// - a plain dark browser (no brand marks) on its extensions page, and Windows' Select Folder dialog,
//   both at 100 %
// Their buttons take the recorder's real mouse clicks.
(() => {
  const SD = window.__SD, desk = document.getElementById('sd-desk');
  const VER = '1.5.2', DIR = 'C:\\Users\\you\\AppData\\Local\\Programs\\Skip Dat';
  const Z = 1.25;                                    // the installer and the dialog at 125 % (Windows' display scale)
  const css = `
  .ap-pos { position: absolute; pointer-events: auto; opacity: 0; }
  .ap-win { position: relative; overflow: hidden; border-radius: 8px; box-shadow: 0 0 0 1px rgba(0,0,0,.4), 0 20px 56px rgba(0,0,0,.6); }
  .ap-tb { height: 31px; display: flex; align-items: center; font: 12px "Segoe UI"; }
  .ap-tb img { width: 16px; height: 16px; margin: 0 9px 0 10px; }
  .ap-tb .ap-tt { flex: 1; white-space: nowrap; }
  .ap-tb .cb { width: 46px; height: 31px; display: grid; place-items: center; }
  .ap-tb .cb svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 1; }
  .ap-tb .cb.off { opacity: .32; }
  /* the installer: Modern UI 2 in the dialog font (MS Shell Dlg 2, which is Tahoma) */
  .ns { width: 497px; background: #F0F0F0; color: #000; font: 11px Tahoma, "Segoe UI", sans-serif; zoom: ${Z}; }
  .ns .ap-tb { background: #fff; color: #1b1b1b; }
  .ns-c { position: relative; width: 497px; height: 358px; }
  .ns-c > * { position: absolute; }
  .ns-page { inset: 0 0 44px 0; display: none; }
  .ns-page.on { display: block; }
  .ns-page > * { position: absolute; }
  .ns-side { left: 0; top: 0; width: 164px; height: 314px; }
  .ns-wr { left: 164px; top: 0; width: 333px; height: 314px; background: #fff; }
  .ns-wt { left: 176px; top: 12px; width: 310px; font: bold 16px Tahoma, sans-serif; line-height: 1.2; }
  .ns-wx { left: 176px; top: 58px; width: 306px; line-height: 1.32; }
  .ns-head { left: 0; top: 0; width: 497px; height: 57px; background: #fff; border-bottom: 1px solid #A0A0A0; box-shadow: 0 1px 0 #fff; }
  .ns-head b { position: absolute; left: 10px; top: 10px; }
  .ns-head span { position: absolute; left: 16px; top: 27px; }
  .ns-head img { position: absolute; right: 0; top: 0; width: 150px; height: 57px; }
  .ns-tx { left: 24px; top: 70px; width: 450px; line-height: 1.32; }
  .ns-line { left: 0; right: 0; top: 314px; height: 0; border-top: 1px solid #A0A0A0; border-bottom: 1px solid #fff; }
  .ns-brand { left: 8px; top: 307px; padding-right: 3px; background: #F0F0F0; color: #8C8C8C; }
  .ns-btn { top: 325px; width: 75px; height: 23px; box-sizing: border-box; display: grid; place-items: center; border-radius: 4px;
    background: #FDFDFD; border: 1px solid #D0D0D0; border-bottom-color: #BABABA; }
  .ns-btn.def { border-color: #005FB8; box-shadow: inset 0 0 0 1px #005FB8; }
  .ns-btn:hover:not(.off) { background: #E8F1FB; border-color: #3D8EE0; }
  .ns-btn.off { color: #A3A3A3; background: #F6F6F6; border-color: #E1E1E1; box-shadow: none; }
  .ns-btn.gone { display: none; }
  .ns-chk { display: inline-block; width: 13px; height: 13px; box-sizing: border-box; border: 1px solid #333; border-radius: 3px; background: #fff; vertical-align: -3px; position: relative; }
  .ns-chk.on { background: #005FB8; border-color: #005FB8; }
  .ns-chk.on::after { content: ""; position: absolute; left: 3.5px; top: .5px; width: 3px; height: 7px; border: solid #fff; border-width: 0 1.6px 1.6px 0; transform: rotate(40deg); }
  .ns-chk.ro { background: #CCCCCC; border-color: #CCCCCC; }
  .ns-tree { left: 176px; top: 104px; width: 298px; height: 112px; background: #fff; border: 1px solid #828790; box-sizing: border-box; padding: 3px 0; }
  .ns-tree div { height: 18px; display: flex; align-items: center; gap: 5px; padding-left: 6px; margin: 0 2px; }
  .ns-tree div.hl { background: #CCE8FF; }
  .ns-grp { border: 1px solid #DCDCDC; border-radius: 4px; box-sizing: border-box; }
  .ns-grp > i { position: absolute; left: 7px; top: -8px; padding: 0 3px; background: #F0F0F0; font-style: normal; }
  .ns-desc { left: 176px; top: 226px; width: 298px; height: 66px; }
  .ns-desc p { margin: 0; position: absolute; left: 9px; top: 12px; right: 9px; line-height: 1.3; }
  .ns-desc p.idle { color: #8C8C8C; }
  .ns-dir { left: 24px; top: 148px; width: 450px; height: 54px; }
  .ns-edit { position: absolute; left: 14px; top: 18px; width: 320px; height: 21px; box-sizing: border-box; background: #fff; border: 1px solid #C6C6C6; border-bottom-color: #868686; border-radius: 3px; padding: 3px 5px; white-space: nowrap; overflow: hidden; }
  .ns-dir .ns-btn { top: 17px; left: 350px; width: 85px; }
  .ns-status { left: 24px; top: 70px; width: 450px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ns-prog { left: 24px; top: 87px; width: 450px; height: 17px; box-sizing: border-box; border: 1px solid #BCBCBC; background: #E6E6E6; }
  .ns-prog i { position: absolute; left: 0; top: 0; bottom: 0; width: 0; background: #06B025; }
  .ns-anim { left: 24px; top: 137px; width: 450px; height: 163px; }
  /* the desktop's setup file */
  .ap-icon { position: absolute; left: 34px; top: 96px; width: 92px; padding: 4px 2px 5px; border-radius: 3px; text-align: center; pointer-events: auto;
    font: 12px "Segoe UI"; color: #fff; text-shadow: 0 1px 2px #000, 0 0 2px #000; line-height: 1.2; border: 1px solid transparent; }
  .ap-icon img { width: 48px; height: 48px; display: block; margin: 0 auto 3px; }
  .ap-icon.sel { background: rgba(0, 120, 215, .38); border-color: rgba(102, 178, 255, .75); }
  /* the browser: dark, generic */
  .br { width: 600px; height: 470px; background: #202124; color: #E8EAED; font: 13px "Segoe UI", sans-serif; }
  .br-top { height: 38px; background: #1B1B1C; display: flex; align-items: flex-end; padding-left: 8px; color: #E8EAED; }
  .br-tab { width: 190px; height: 32px; box-sizing: border-box; display: flex; align-items: center; gap: 8px; padding: 0 10px; border-radius: 8px 8px 0 0; font-size: 12px; color: #C4C7C5; }
  .br-tab.on { background: #35363A; color: #E8EAED; }
  .br-tab svg { width: 16px; height: 16px; flex: none; }
  .br-tab span { flex: 1; white-space: nowrap; overflow: hidden; }
  .br-tab i { font-style: normal; opacity: .7; }
  .br-plus { width: 28px; height: 32px; display: grid; place-items: center; font-size: 18px; color: #C4C7C5; }
  .br-top .sp { flex: 1; }
  .br-top .ap-tb { height: 38px; align-self: flex-start; color: #E8EAED; }
  .br-bar { height: 40px; background: #35363A; display: flex; align-items: center; gap: 4px; padding: 0 8px; border-bottom: 1px solid #000; }
  .br-bar .ic { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 50%; }
  .br-bar .ic svg { width: 16px; height: 16px; fill: none; stroke: #C4C7C5; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .br-bar .ic.off svg { stroke: #6E7175; }
  .br-url { flex: 1; height: 28px; border-radius: 14px; background: #202124; display: flex; align-items: center; padding: 0 14px; font-size: 13px; color: #E8EAED; margin-left: 4px; }
  .br-url.focus { box-shadow: 0 0 0 2px #8AB4F8; }
  .br-url .ph { color: #9AA0A6; }
  .br-url .caret { display: inline-block; width: 1px; height: 15px; background: #E8EAED; margin-left: 1px; vertical-align: -3px; }
  .br-page { position: absolute; left: 0; right: 0; top: 79px; bottom: 0; display: none; }
  .br-page.on { display: block; }
  .bx-head { height: 56px; display: flex; align-items: center; padding: 0 20px; border-bottom: 1px solid #3C4043; }
  .bx-head h1 { margin: 0; font: 400 22px "Segoe UI", sans-serif; flex: 1; }
  .bx-dev { display: flex; align-items: center; gap: 10px; font-size: 13px; color: #C4C7C5; }
  .bx-tg { width: 34px; height: 14px; border-radius: 7px; background: #5F6368; position: relative; }
  .bx-tg b { position: absolute; top: -3px; left: -2px; width: 20px; height: 20px; border-radius: 50%; background: #BDC1C6; box-shadow: 0 1px 3px rgba(0,0,0,.5); }
  .bx-tg.on { background: #4D6A93; }
  .bx-tg.on b { background: #8AB4F8; }
  .bx-tools { height: 0; overflow: hidden; display: flex; align-items: center; gap: 10px; padding: 0 20px; border-bottom: 1px solid transparent; }
  .bx-tools.on { height: 54px; border-bottom-color: #3C4043; }
  .bx-btn { height: 32px; padding: 0 15px; box-sizing: border-box; display: grid; place-items: center; border-radius: 4px; border: 1px solid #5F6368; color: #8AB4F8; font-weight: 600; font-size: 13px; }
  .bx-btn:hover { background: rgba(138, 180, 248, .08); }
  .bx-empty { margin: 26px 20px; color: #9AA0A6; }
  .bx-card { margin: 18px 20px; width: 400px; box-sizing: border-box; border-radius: 8px; background: #292A2D; box-shadow: 0 1px 3px rgba(0,0,0,.4); opacity: 0; }
  .bx-card .top { display: flex; gap: 16px; padding: 16px 16px 10px; }
  .bx-card .top img { width: 40px; height: 40px; flex: none; }
  .bx-card .nm { font-size: 14px; color: #E8EAED; }
  .bx-card .nm small { color: #9AA0A6; font-size: 13px; margin-left: 8px; }
  .bx-card .ds { color: #9AA0A6; font-size: 12.5px; line-height: 1.4; margin-top: 4px; }
  .bx-card .id { color: #9AA0A6; font-size: 12px; margin-top: 6px; }
  .bx-card .row { display: flex; align-items: center; gap: 6px; padding: 6px 10px 12px 10px; }
  .bx-card .row span { color: #8AB4F8; font-weight: 600; padding: 6px; }
  .bx-card .row .sp { flex: 1; }
  .bx-card .row svg { width: 18px; height: 18px; fill: none; stroke: #C4C7C5; stroke-width: 1.8; stroke-linecap: round; margin-right: 10px; }
  .by-page { position: absolute; inset: 0; background: #0F0F0F; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; }
  .by-page img { width: 200px; height: 200px; border-radius: 6px; box-shadow: 0 10px 30px rgba(0,0,0,.6); }
  .by-page b { font-size: 18px; font-weight: 600; }
  .by-page span { color: #AAAAAA; }
  /* Windows 11's Select Folder dialog */
  .fd { width: 560px; background: #F3F3F3; color: #1A1A1A; font: 12px "Segoe UI", sans-serif; }
  .fd .ap-tb { background: #F3F3F3; }
  .fd-nav { height: 40px; display: flex; align-items: center; gap: 6px; padding: 0 8px; }
  .fd-nav .ar { width: 22px; text-align: center; color: #9A9A9A; font-size: 14px; }
  .fd-nav .ar.on { color: #1A1A1A; }
  .fd-crumb { flex: 1; height: 28px; border-radius: 4px; background: #fff; border: 1px solid #E5E5E5; display: flex; align-items: center; gap: 7px; padding: 0 8px; white-space: nowrap; overflow: hidden; }
  .fd-crumb i { font-style: normal; color: #7A7A7A; font-size: 11px; }
  .fd-search { width: 150px; height: 28px; border-radius: 4px; background: #fff; border: 1px solid #E5E5E5; display: flex; align-items: center; padding: 0 9px; color: #6E6E6E; box-sizing: border-box; }
  .fd-cmd { height: 34px; display: flex; align-items: center; gap: 18px; padding: 0 14px; border-bottom: 1px solid #E5E5E5; background: #F9F9F9; }
  .fd-body { display: flex; height: 200px; background: #fff; overflow: hidden; }
  .fd-left { width: 150px; border-right: 1px solid #EBEBEB; padding: 6px 0; box-sizing: border-box; }
  .fd-left div { height: 21px; display: flex; align-items: center; gap: 8px; padding-left: 14px; }
  .fd-left hr { border: 0; border-top: 1px solid #EBEBEB; margin: 4px 10px; }
  .fd-left svg, .fd-row svg { width: 16px; height: 16px; flex: none; }
  .fd-list { flex: 1; padding: 0 6px; }
  .fd-cols, .fd-row { display: grid; grid-template-columns: 1fr 120px 80px; align-items: center; height: 26px; padding: 0 8px; }
  .fd-cols { color: #5A5A5A; border-bottom: 1px solid #EBEBEB; margin-bottom: 2px; }
  .fd-row { border: 1px solid transparent; border-radius: 3px; }
  .fd-row > span:first-child { display: flex; align-items: center; gap: 8px; }
  .fd-row .dim { color: #6E6E6E; }
  .fd-row:hover { background: #E5F3FF; }
  .fd-row.sel { background: #CCE8FF; border-color: #99D1FF; }
  .fd-foot { height: 78px; display: grid; grid-template-columns: auto 1fr; grid-template-rows: 1fr 1fr; align-items: center; gap: 0 10px; padding: 4px 14px 8px; box-sizing: border-box; }
  .fd-foot .ed { height: 24px; background: #fff; border: 1px solid #C6C6C6; border-bottom-color: #868686; border-radius: 3px; box-sizing: border-box; padding: 3px 7px; }
  .fd-foot .bts { grid-column: 2; display: flex; justify-content: flex-end; gap: 8px; }
  .fd-b { width: 96px; height: 26px; box-sizing: border-box; border-radius: 4px; display: grid; place-items: center; background: #FDFDFD; border: 1px solid #D0D0D0; border-bottom-color: #BABABA; }
  .fd-b.def { border-color: #005FB8; box-shadow: inset 0 0 0 1px #005FB8; }
  .fd-b:hover { background: #E8F1FB; border-color: #3D8EE0; }`;
  const st = document.createElement('style'); st.textContent = css; document.head.append(st);
  const el = (tag, cls, html, parent) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; if (parent) parent.append(e); return e; };
  const ease = x => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
  const icons = {
    min: '<svg viewBox="0 0 10 10"><path d="M0 5.5h10"/></svg>',
    max: '<svg viewBox="0 0 10 10"><rect x=".5" y=".5" width="9" height="9" rx="1"/></svg>',
    close: '<svg viewBox="0 0 10 10"><path d="M.5.5l9 9M9.5.5l-9 9"/></svg>',
    folder: '<svg viewBox="0 0 16 16"><path d="M1 3.5h5l1.5 1.5H15v8.5H1z" fill="#F6C343"/><path d="M1 6h14v7.5H1z" fill="#FFD764"/></svg>',
    puzzle: '<svg viewBox="0 0 16 16"><path d="M6 2.5a1.5 1.5 0 0 1 3 0V4h3v3h-1.5a1.5 1.5 0 0 0 0 3H12v3.5H9V12a1.5 1.5 0 0 0-3 0v1.5H2.5V10H4a1.5 1.5 0 0 0 0-3H2.5V4H6z" fill="#9AA0A6"/></svg>',
    note: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="7.5" fill="#F27DB2"/><path d="M6.5 11.2V5.2l4-1v5.4" stroke="#2A0F1E" stroke-width="1.3" fill="none"/><circle cx="5.5" cy="11.2" r="1.3" fill="#2A0F1E"/><circle cx="9.5" cy="9.7" r="1.3" fill="#2A0F1E"/></svg>',
    blank: '<svg viewBox="0 0 16 16"><rect x="2.5" y="1.5" width="11" height="13" rx="1.5" fill="none" stroke="#9AA0A6"/></svg>',
  };
  const titleBar = (title, icon, maxOff) => `<div class="ap-tb">${icon ? `<img src="${icon}" alt="">` : '<span style="width:12px"></span>'}<span class="ap-tt">${title}</span>` +
    `<span class="cb">${icons.min}</span><span class="cb${maxOff ? ' off' : ''}">${icons.max}</span><span class="cb">${icons.close}</span></div>`;
  // a window fades and grows in (Windows' own open animation), and fades out
  function show(pos, on, dur = 0.2) {
    SD.anim(dur, k => { const e = ease(k); pos.style.opacity = on ? e : 1 - e; pos.style.transform = `scale(${on ? 0.96 + 0.04 * e : 1 - 0.03 * e})`; if (!on && k >= 1) pos.style.visibility = 'hidden'; });
    if (on) pos.style.visibility = '';
  }
  const A = SD.apps = {};

  // ---- the tray icon appears once Skip Dat runs ----
  const trayIcon = document.getElementById('sd-trayicon');
  A.tray = on => {
    if (!on) { trayIcon.style.visibility = 'hidden'; return; }
    trayIcon.style.visibility = '';
    SD.anim(0.4, k => { const s = k < 0.6 ? 0.3 + 0.95 * Math.sin(k / 0.6 * Math.PI / 2) : 1.25 - 0.25 * ease((k - 0.6) / 0.4); trayIcon.style.transform = `scale(${s})`; });
  };

  // ---- the setup file on the desktop ----
  const icon = el('div', 'ap-icon', `<img src="/branding/128x128.png" alt=""><span>SkipDat-${VER}-setup.exe</span>`, desk);
  icon.style.display = 'none';
  A.desktopIcon = on => { icon.style.display = on ? '' : 'none'; };
  A.selectIcon = on => icon.classList.toggle('sel', on);

  // ---- the installer ----
  const nsPos = el('div', 'ap-pos', null, desk);
  nsPos.style.visibility = 'hidden';
  const wel = (title, text, extra = '') => `<img class="ns-side" src="/installer/side.bmp" alt=""><div class="ns-wr"></div><div class="ns-wt">${title}</div><div class="ns-wx">${text}</div>${extra}`;
  const head = (t, s) => `<div class="ns-head"><b>${t}</b><span>${s}</span><img src="/installer/header.bmp" alt=""></div>`;
  const comps = [['app', 'Skip Dat', 'ro', 'The app, and the browser companion folder for YouTube Music, YouTube and SoundCloud.'], ['start', 'Start menu shortcut', 1, 'Skip Dat in the Start menu.'],
    ['desk', 'Desktop shortcut', 0, 'Skip Dat on the desktop.'], ['auto', 'Start with Windows', 1, 'Skip Dat starts in the tray when you sign in, ready for the shortcut. You can change this later in Settings.']];
  const IDLE = 'Position your mouse over a component to see its description.';
  const SPACE = 'Space required: 12.0MB';
  nsPos.innerHTML = `<div class="ap-win ns">${titleBar('Skip Dat Setup', '/branding/32x32.png', true)}<div class="ns-c">
    <div class="ns-page" data-p="welcome">${wel('Welcome to Skip Dat', `Every music app's queue, one Alt + scroll away.<br><br>This installs Skip Dat ${VER} for your Windows account. No administrator rights are needed.<br><br>If Skip Dat is running, it will be closed and updated.`)}</div>
    <div class="ns-page" data-p="components">${head('Choose Components', 'Choose which features of Skip Dat you want to install.')}
      <div class="ns-tx">Check the components you want to install and uncheck the components you don't want to install. Click Next to continue.</div>
      <div style="left:24px;top:106px">Select components to install:</div>
      <div class="ns-tree">${comps.map(([id, n, on]) => `<div data-c="${id}"><span class="ns-chk${on === 'ro' ? ' on ro' : on ? ' on' : ''}"></span>${n}</div>`).join('')}</div>
      <div style="left:24px;top:278px">${SPACE}</div>
      <div class="ns-grp ns-desc"><i>Description</i><p class="idle">${IDLE}</p></div></div>
    <div class="ns-page" data-p="directory">${head('Choose Install Location', 'Choose the folder in which to install Skip Dat.')}
      <div class="ns-tx">Setup will install Skip Dat in the following folder. To install in a different folder, click Browse and select another folder. Click Install to start the installation.</div>
      <div class="ns-grp ns-dir"><i>Destination Folder</i><div class="ns-edit">${DIR}</div><div class="ns-btn" style="position:absolute">Browse...</div></div>
      <div style="left:24px;top:256px">${SPACE}</div><div style="left:24px;top:272px">Space available: 212.4GB</div></div>
    <div class="ns-page" data-p="instfiles">${head('Installing', 'Please wait while Skip Dat is being installed.')}
      <div class="ns-status"></div><div class="ns-prog"><i></i></div><canvas class="ns-anim"></canvas></div>
    <div class="ns-page" data-p="finish">${wel('Skip Dat is ready', 'Skip Dat waits in the tray, near the clock. Hold Alt and scroll anywhere to open it.<br><br>To see the queues of YouTube Music, YouTube and SoundCloud, add the browser companion. Settings shows how.',
      '<div style="left:176px;top:176px"><span class="ns-chk on"></span>&nbsp; Start Skip Dat now</div>')}</div>
    <div class="ns-line"></div><span class="ns-brand">Skip Dat ${VER}</span>
    <div class="ns-btn" id="ns-back" style="left:245px">&lt; Back</div><div class="ns-btn" id="ns-next" style="left:320px">Next &gt;</div><div class="ns-btn" id="ns-cancel" style="left:410px">Cancel</div>
  </div></div>`;
  const $n = s => nsPos.querySelector(s);
  const ORDER = ['welcome', 'components', 'directory', 'instfiles', 'finish'];
  // the buttons on each page: [Back, Next, Cancel], each 'on', 'off' (greyed) or 'gone'; and Next's label
  const BTNS = { welcome: ['gone', 'on', 'on', 'Next >'], components: ['on', 'on', 'on', 'Next >'], directory: ['on', 'on', 'on', 'Install'],
    instfiles: ['off', 'off', 'off', 'Next >'], finish: ['off', 'on', 'off', 'Finish'] };
  let nsPage = null, instT0 = null;
  function page(p) {
    nsPage = p;
    nsPos.querySelectorAll('.ns-page').forEach(e => e.classList.toggle('on', e.dataset.p === p));
    const b = BTNS[p];
    ['#ns-back', '#ns-next', '#ns-cancel'].forEach((s, i) => { const e = $n(s); e.className = 'ns-btn' + (b[i] === 'on' ? '' : ' ' + b[i]) + (i === 1 ? ' def' : ''); });
    $n('#ns-next').textContent = b[3];
    if (p === 'instfiles') { instT0 = SD.T() / 1000; drawAnim(0); }
  }
  A.installer = {
    open() { page('welcome'); show(nsPos, true); },
    close() { show(nsPos, false, 0.18); },
    page: () => nsPage,
    at(x, y) { nsPos.style.left = x + 'px'; nsPos.style.top = y + 'px'; },
    onFinish: null,
  };
  $n('#ns-next').addEventListener('click', () => {
    if (BTNS[nsPage][1] !== 'on') return;
    if (nsPage === 'finish') { A.installer.close(); if (A.installer.onFinish) A.installer.onFinish(); return; }
    page(ORDER[ORDER.indexOf(nsPage) + 1]);
  });
  // pointing at a component shows its description; a click ticks it (Skip Dat itself is always in)
  nsPos.querySelectorAll('.ns-tree div').forEach(row => {
    const c = comps.find(x => x[0] === row.dataset.c), p = $n('.ns-desc p');
    row.addEventListener('mouseenter', () => { p.textContent = c[3]; p.className = ''; });
    row.addEventListener('mouseleave', () => { p.textContent = IDLE; p.className = 'idle'; });
    row.addEventListener('click', () => { const k = row.querySelector('.ns-chk'); if (!k.classList.contains('ro')) k.classList.toggle('on'); nsPos.querySelectorAll('.ns-tree div').forEach(r => r.classList.toggle('hl', r === row)); });
  });
  // what the progress page says, second by second: setup-anim.exe comes out as the page opens, then the
  // section runs (closing a running copy takes about half a second), then Modern UI moves on by itself
  const sub = p => DIR + (p ? '\\' + p : '');
  const LINES = [[0, 'Extract: setup-anim.exe', 0.01], [0.62, 'Output folder: ' + sub(), 0.02], [0.72, 'Extract: Skip Dat.exe', 0.02, 0.78, 1.55],
    [1.62, 'Extract: WebView2Loader.dll', 0.8], [1.72, 'Extract: Read me.txt', 0.81], [1.8, 'Output folder: ' + sub('browser-companion'), 0.82], [1.86, 'Extract: background.js', 0.83],
    [1.92, 'Extract: bridge.js', 0.84], [1.98, 'Extract: manifest.json', 0.85], [2.04, 'Extract: page.js', 0.87], [2.1, 'Output folder: ' + sub('browser-companion\\icons'), 0.88],
    [2.16, 'Extract: 128.png', 0.89], [2.22, 'Extract: 16.png', 0.9], [2.28, 'Extract: 48.png', 0.91], [2.36, 'Output folder: ' + sub(), 0.92],
    [2.44, 'Created uninstaller: ' + sub('Uninstall Skip Dat.exe'), 0.97], [2.62, 'Completed', 1]];
  const DONE = 2.95;
  const status = $n('.ns-status'), bar = $n('.ns-prog i');
  SD.onStep.push(t => {
    if (nsPage !== 'instfiles' || instT0 == null) return;
    const e = t - instT0;
    let line = LINES[0];
    for (const l of LINES) if (e >= l[0]) line = l;
    let txt = line[1], p = line[2];
    if (line[3]) { const k = Math.min(1, (e - line[0]) / (line[4] - line[0])); p = line[2] + (line[3] - line[2]) * k; txt += '... ' + Math.floor(k * 100) + '%'; }
    else if (/^Extract/.test(txt)) txt += '... 100%';
    if (status.textContent !== txt) status.textContent = txt;
    bar.style.width = (p * 100).toFixed(2) + '%';
    drawAnim(e);
    if (e >= DONE) page('finish');
  });

  // ---- the glass animation (installer/anim/src/main.rs, drawn the same way) ----
  // It fills the details list's place in physical pixels at 125 %, on the dialog's face colour.
  const cv = $n('.ns-anim'), dpr = window.devicePixelRatio || 1, W = Math.round(450 * Z), H = Math.round(163 * Z);
  cv.width = W * dpr; cv.height = H * dpr;
  const g = cv.getContext('2d');
  const barImg = new Image(); barImg.src = '/branding/icon.png';
  const TAU = Math.PI * 2, LOOP = 6;
  const rgba = (r, gg, b, a) => `rgba(${r},${gg},${b},${a})`;
  const surface = (x, t, w, base, amp) => base + amp * Math.sin(TAU * (x / (w * 0.55) - t / 3)) + amp * 0.45 * Math.sin(TAU * (x / (w * 0.31) + t / 2));
  const rrect = (x, y, w, h, r) => { g.beginPath(); g.roundRect(x, y, w, h, r); };
  const lin = (x0, y0, x1, y1, stops) => { const l = g.createLinearGradient(x0, y0, x1, y1); stops.forEach(([p, c]) => l.addColorStop(p, c)); return l; };
  // an elliptical radial gradient, drawn in a stretched space
  function ellipse(cx, cy, rx, ry, stops) {
    g.save(); g.translate(cx, cy); g.scale(1, ry / rx);
    const r = g.createRadialGradient(0, 0, 0, 0, 0, rx); stops.forEach(([p, c]) => r.addColorStop(p, c));
    g.fillStyle = r; g.beginPath(); g.arc(0, 0, rx, 0, TAU); g.fill();
    g.restore();
  }
  function drawAnim(tt) {
    const t = tt % LOOP, w = W, h = H, m = 2;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = '#F0F0F0'; g.fillRect(0, 0, w, h);
    // the dark glass card, with a pink glow
    rrect(m, m, w - 2 * m, h - 2 * m, 14); g.fillStyle = lin(0, 0, 0, h, [[0, rgba(26, 35, 54, 1)], [1, rgba(11, 15, 23, 1)]]); g.fill();
    g.save(); rrect(m, m, w - 2 * m, h - 2 * m, 14); g.clip();
    ellipse(w * 0.5, h * 0.62, w * 0.42, h * 0.8, [[0, rgba(246, 120, 180, 0.3)], [1, rgba(246, 120, 180, 0)]]);
    g.restore();
    const base = h * 0.7, amp = h * 0.035, cx = w * 0.5;
    const sy = surface(cx, t, w, base, amp), slope = (surface(cx + 8, t, w, base, amp) - surface(cx - 8, t, w, base, amp)) / 16;
    // the soap bar rides the surface, tilts with it and breathes
    const size = h * 0.78, bob = sy - size * 0.24;
    const ang = Math.atan(slope) * 180 / Math.PI * 0.6 + 1.5 * Math.sin(TAU * t / 3), breathe = 1 + 0.015 * Math.sin(TAU * t / 3 + 1);
    ellipse(cx, sy + h * 0.06, size * 0.42, h * 0.07, [[0, rgba(60, 8, 40, 0.45)], [1, rgba(60, 8, 40, 0)]]);
    g.save(); g.translate(cx, bob); g.rotate(ang * Math.PI / 180); g.scale(breathe, breathe);
    if (barImg.complete && barImg.naturalWidth) g.drawImage(barImg, -size / 2, -size / 2, size, size);
    // light sweeping over the bar's face
    const fw = size * 0.8, fh = size * 0.52, ph = (t % 3) / 3, sx = -fw * 0.9 + ph * fw * 2.4;
    g.rotate(9 * Math.PI / 180); g.translate(0, -size * 0.03);
    rrect(-fw / 2, -fh / 2, fw, fh, fh * 0.22);
    g.fillStyle = lin(sx - fw * 0.18, -fh * 0.5, sx + fw * 0.18, fh * 0.5, [[0, rgba(255, 255, 255, 0)], [0.5, rgba(255, 255, 255, 0.32)], [1, rgba(255, 255, 255, 0)]]); g.fill();
    g.restore();
    // the see-through liquid in front, its lit surface, and glass bubbles rising through it
    g.save(); g.beginPath(); g.rect(m, m, w - 2 * m, h - 2 * m); g.clip();
    const steps = 48, xs = i => m + (w - 2 * m) * i / steps;
    g.beginPath(); g.moveTo(m, h - m); for (let i = 0; i <= steps; i++) g.lineTo(xs(i), surface(xs(i), t, w, base, amp)); g.lineTo(w - m, h - m); g.closePath();
    g.fillStyle = lin(0, base - amp * 2, 0, h, [[0, rgba(246, 140, 190, 0.42)], [1, rgba(120, 30, 84, 0.78)]]); g.fill();
    g.beginPath(); for (let i = 0; i <= steps; i++) g[i ? 'lineTo' : 'moveTo'](xs(i), surface(xs(i), t, w, base, amp));
    g.strokeStyle = rgba(255, 220, 238, 0.55); g.lineWidth = 1.5; g.stroke();
    for (const [fx, r, phase] of [[0.16, 7, 0], [0.31, 4.5, 0.35], [0.69, 6, 0.62], [0.84, 9, 0.18], [0.58, 3.5, 0.8]]) {
      const k = ((t / LOOP) * 2 + phase) % 1, x = w * fx + 4 * Math.sin(TAU * (k * 2 + phase));
      const top = surface(x, t, w, base, amp) + r + 2, y = (h - m - r) - k * ((h - m - r) - top);
      const a = Math.min(k * 6, 1) * Math.min((1 - k) * 6, 1), rr = r * Math.max(h / 150, 0.8);
      const b = g.createRadialGradient(x, y, 0, x, y, rr);
      b.addColorStop(0, rgba(255, 255, 255, 0.05 * a)); b.addColorStop(0.7, rgba(255, 220, 238, 0.12 * a)); b.addColorStop(1, rgba(255, 255, 255, 0.55 * a));
      g.fillStyle = b; g.beginPath(); g.arc(x, y, rr, 0, TAU); g.fill();
      g.fillStyle = rgba(255, 255, 255, 0.7 * a); g.beginPath(); g.arc(x - rr * 0.35, y - rr * 0.35, rr * 0.22, 0, TAU); g.fill();
    }
    g.restore();
    // the card's glass rim
    rrect(m, m, w - 2 * m, h - 2 * m, 14); g.lineWidth = 1;
    g.strokeStyle = lin(0, 0, 0, h, [[0, rgba(255, 255, 255, 0.28)], [0.5, rgba(255, 255, 255, 0.06)], [1, rgba(255, 255, 255, 0.12)]]); g.stroke();
  }

  // ---- the browser, on a new tab ----
  const brPos = el('div', 'ap-pos', null, desk);
  brPos.style.visibility = 'hidden';
  brPos.innerHTML = `<div class="ap-win br">
    <div class="br-top"><div class="br-tab" id="br-tab-ytm">${icons.note}<span>YouTube Music</span><i>&#x2715;</i></div><div class="br-tab on" id="br-tab-ext">${icons.blank}<span>New Tab</span><i>&#x2715;</i></div>
      <span class="br-plus">+</span><span class="sp"></span>${titleBar('', null, false).replace('<span style="width:12px"></span><span class="ap-tt"></span>', '')}</div>
    <div class="br-bar"><span class="ic off"><svg viewBox="0 0 16 16"><path d="M13 8H3M7 4L3 8l4 4"/></svg></span><span class="ic off"><svg viewBox="0 0 16 16"><path d="M3 8h10M9 4l4 4-4 4"/></svg></span>
      <span class="ic" id="br-reload"><svg viewBox="0 0 16 16"><path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5v3h-3"/></svg></span><div class="br-url" id="br-url"><span class="ph">Search or type a web address</span></div></div>
    <div class="br-page on" id="br-new"></div>
    <div class="br-page" id="br-ext"><div class="bx-head"><h1>Extensions</h1><div class="bx-dev">Developer mode <div class="bx-tg" id="br-dev"><b></b></div></div></div>
      <div class="bx-tools" id="br-tools"><div class="bx-btn" id="br-load">Load unpacked</div><div class="bx-btn">Pack extension</div><div class="bx-btn">Update</div></div>
      <div class="bx-empty" id="br-empty">Your extensions show up here.</div>
      <div class="bx-card" id="br-card" hidden><div class="top"><img src="/companion/icons/128.png" alt=""><div><div class="nm">Skip Dat companion<small>1.5.2</small></div>
        <div class="ds">Lets the Skip Dat app see the real queue of YouTube Music, YouTube and SoundCloud tabs and play any song in it.</div></div></div>
        <div class="row"><span>Details</span><span>Remove</span><span class="sp"></span><svg viewBox="0 0 16 16"><path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5v3h-3"/></svg><div class="bx-tg on"><b style="left:16px"></b></div></div></div></div>
    <div class="br-page" id="br-ytm"><div class="by-page"><img src="/art/cc-memotone.jpg" alt=""><b>Ochre Against Blue</b><span>Memotone</span></div></div>
  </div>`;
  const $b = s => brPos.querySelector(s);
  const url = $b('#br-url');
  const tabs = { ext: $b('#br-tab-ext'), ytm: $b('#br-tab-ytm') };
  let tab = 'ext', extUrl = '', ytmUrl = 'music.youtube.com';
  const setUrl = (text, caret) => { url.innerHTML = text ? text.replace(/&/g, '&amp;').replace(/</g, '&lt;') + (caret ? '<span class="caret"></span>' : '') : '<span class="ph">Search or type a web address</span>' + (caret ? '<span class="caret"></span>' : ''); };
  A.browser = {
    open(x, y) { brPos.style.left = x + 'px'; brPos.style.top = y + 'px'; show(brPos, true); },
    focusUrl() { url.classList.add('focus'); setUrl('', true); },
    // types into the address bar over `dur` seconds, then goes there
    type(text, dur) {
      SD.anim(dur, k => setUrl(text.slice(0, Math.round(text.length * k)), true));
    },
    enter() {
      url.classList.remove('focus'); extUrl = 'chrome://extensions'; setUrl(extUrl);
      $b('#br-new').classList.remove('on'); $b('#br-ext').classList.add('on');
      tabs.ext.innerHTML = `${icons.puzzle}<span>Extensions</span><i>&#x2715;</i>`;
    },
    tab(which) {
      tab = which;
      Object.entries(tabs).forEach(([k, e]) => e.classList.toggle('on', k === which));
      brPos.querySelectorAll('.br-page').forEach(p => p.classList.remove('on'));
      $b(which === 'ytm' ? '#br-ytm' : '#br-ext').classList.add('on');
      setUrl(which === 'ytm' ? ytmUrl : extUrl);
    },
  };
  url.addEventListener('click', () => A.browser.focusUrl());
  $b('#br-dev').addEventListener('click', () => {
    const tg = $b('#br-dev'), on = !tg.classList.contains('on');
    tg.classList.toggle('on', on); $b('#br-tools').classList.toggle('on', on);
    SD.anim(0.15, k => { tg.querySelector('b').style.left = (on ? -2 + 18 * ease(k) : 16 - 18 * ease(k)) + 'px'; });
  });
  $b('#br-load').addEventListener('click', () => A.folder.open());
  tabs.ytm.addEventListener('click', () => A.browser.tab('ytm'));
  tabs.ext.addEventListener('click', () => A.browser.tab('ext'));
  // reload: the arrow turns once and the page comes back
  $b('#br-reload').addEventListener('click', () => {
    const s = $b('#br-reload svg'), pg = $b(tab === 'ytm' ? '#br-ytm' : '#br-ext');
    SD.anim(0.6, k => { s.style.transform = `rotate(${360 * ease(k)}deg)`; pg.style.opacity = k < 0.25 ? 1 - k * 4 : k < 0.5 ? 0 : (k - 0.5) * 2; });
  });

  // ---- Windows' Select Folder dialog, in Skip Dat's folder (it lists folders only) ----
  const fdPos = el('div', 'ap-pos', null, desk);
  fdPos.style.visibility = 'hidden';
  const nav = [['Home', '#5E9CF0'], ['Gallery', '#E8A33D'], 0, ['Desktop', '#4A8FE0'], ['Downloads', '#2FA36B'], ['Documents', '#7A8CA3'], ['Pictures', '#3C9BD6'], ['Music', '#E25C8E'], 0, ['This PC', '#5E9CF0']];
  fdPos.innerHTML = `<div class="ap-win fd">${titleBar('Select the extension directory.', null, true).replace(/<span class="cb[^"]*">.*?<\/span><span class="cb[^"]*">.*?<\/span>/, '')}
    <div class="fd-nav"><span class="ar">&#x2190;</span><span class="ar">&#x2192;</span><span class="ar on">&#x2191;</span>
      <div class="fd-crumb">${icons.folder}<i>&#xAB;</i>AppData<i>&#x203A;</i>Local<i>&#x203A;</i>Programs<i>&#x203A;</i>Skip Dat<i>&#x203A;</i></div><div class="fd-search">Search Skip Dat</div></div>
    <div class="fd-cmd"><span>Organize &#x25BE;</span><span>New folder</span></div>
    <div class="fd-body"><div class="fd-left">${nav.map(n => n ? `<div><svg viewBox="0 0 16 16"><rect x="2" y="2.5" width="12" height="11" rx="2" fill="${n[1]}"/></svg>${n[0]}</div>` : '<hr>').join('')}</div>
      <div class="fd-list"><div class="fd-cols"><span>Name</span><span>Date modified</span><span>Type</span></div>
        <div class="fd-row" id="fd-row"><span>${icons.folder}browser-companion</span><span class="dim">11/5/2026 9:40 PM</span><span class="dim">File folder</span></div></div></div>
    <div class="fd-foot"><span>Folder:</span><div class="ed" id="fd-name"></div><div class="bts"><div class="fd-b def" id="fd-ok">Select Folder</div><div class="fd-b" id="fd-cancel">Cancel</div></div></div></div>`;
  const $f = s => fdPos.querySelector(s);
  A.folder = {
    open() { const r = brPos.getBoundingClientRect(); fdPos.style.left = Math.round(r.left + (r.width - 560) / 2) + 'px'; fdPos.style.top = Math.round(r.top + 44) + 'px'; show(fdPos, true); },
  };
  $f('#fd-row').addEventListener('click', () => { $f('#fd-row').classList.add('sel'); $f('#fd-name').textContent = 'browser-companion'; });
  $f('#fd-ok').addEventListener('click', () => {
    show(fdPos, false, 0.15);
    // the browser loads the folder and shows the companion
    SD.later(350, () => { const c = $b('#br-card'); $b('#br-empty').hidden = true; c.hidden = false; SD.anim(0.3, k => { c.style.opacity = ease(k); c.style.transform = `translateY(${(1 - ease(k)) * 6}px)`; }); });
  });
})();
