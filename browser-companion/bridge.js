// Skip Dat companion, the tab's side: passes what the page reports to the extension, and the
// app's commands back to the page. (page.js runs inside the page itself; this script runs in the
// extension's own space, which is the only one allowed to reach the app.)
(() => {
  'use strict';
  const TAG = '__skipdat';
  const nonce = Math.floor(Math.random() * 2 ** 31);
  let alive = true;
  let backoff = 0;

  function toPage(cmds) {
    if (cmds && cmds.length) window.postMessage({ [TAG]: 'cmd', cmds }, location.origin);
  }

  function ask(msg) {
    return new Promise(resolve => {
      if (!alive) return resolve({ ok: false });
      try {
        chrome.runtime.sendMessage(msg, r => {
          if (chrome.runtime.lastError || !r) return resolve({ ok: false });
          resolve(r);
        });
      } catch (e) {
        // the extension was reloaded or removed: this page's copy stops quietly
        alive = false;
        resolve({ ok: false });
      }
    });
  }

  // the page script may have reported before this script was listening; ask it again
  function askPage() { window.postMessage({ [TAG]: 'resend' }, location.origin); }

  async function sendState(report) {
    const r = await ask({ kind: 'state', report });
    if (r.ok) toPage(r.commands);
  }

  // Commands arrive through a long-poll that is always open, so they reach this tab at once even
  // when the browser slows down timers in background tabs.
  async function waitLoop() {
    while (alive) {
      const r = await ask({ kind: 'wait', nonce });
      if (r.ok) {
        if (backoff) askPage(); // the app came (back): tell it everything
        backoff = 0;
        toPage(r.commands);
      } else {
        backoff = Math.min(30000, backoff ? backoff * 2 : 2000);
        await new Promise(res => setTimeout(res, backoff));
      }
    }
  }

  window.addEventListener('message', e => {
    if (e.source !== window || !e.data || e.data[TAG] !== 'state') return;
    sendState(e.data.report);
  });
  window.addEventListener('pagehide', () => { ask({ kind: 'closed' }); });
  askPage();
  waitLoop();
})();
