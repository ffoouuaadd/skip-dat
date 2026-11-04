// Skip Dat companion: carries messages between the music tabs and the Skip Dat app on this PC.
// It only ever talks to 127.0.0.1 (this computer); nothing leaves your machine.
'use strict';
const APP = 'http://127.0.0.1:47231';

const BROWSER = (() => {
  const brands = ((navigator.userAgentData && navigator.userAgentData.brands) || []).map(b => b.brand);
  const has = re => brands.some(b => re.test(b));
  if (has(/edge/i)) return 'Edge';
  if (has(/opera/i)) return 'Opera';
  if (has(/brave/i)) return 'Brave';
  if (has(/vivaldi/i)) return 'Vivaldi';
  return 'Chrome';
})();

async function post(path, body, ms) {
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), ms);
  try {
    const r = await fetch(APP + path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: ctl.signal,
    });
    if (!r.ok) throw new Error('Skip Dat answered ' + r.status);
    return await r.json();
  } finally {
    clearTimeout(timer);
  }
}

chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  const tabId = sender.tab && sender.tab.id;
  if (tabId == null || !msg) return false;
  const answer = p => p.then(r => reply({ ok: true, commands: (r && r.commands) || [] }), () => reply({ ok: false }));
  if (msg.kind === 'state') {
    answer(post('/ext/state', Object.assign({}, msg.report, { browser: BROWSER, tabId }), 5000));
    return true;
  }
  if (msg.kind === 'wait') {
    // a long-poll: Skip Dat answers as soon as it has something for this tab (or after 20 s)
    answer(post('/ext/wait', { browser: BROWSER, tabId, nonce: msg.nonce }, 25000));
    return true;
  }
  if (msg.kind === 'closed') {
    post('/ext/closed', { browser: BROWSER, tabId }, 3000).catch(() => {});
  }
  return false;
});

chrome.tabs.onRemoved.addListener(tabId => {
  post('/ext/closed', { browser: BROWSER, tabId }, 3000).catch(() => {});
});
