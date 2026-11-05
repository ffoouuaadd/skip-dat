// shared: launch a headless Chromium for the recorder.
// puppeteer-core comes from `npm install` here, or from the folder in PUPPETEER_CORE.
// The browser is CHROME if set, else Playwright's Chromium if installed, else Microsoft Edge.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);

let puppeteer;
try { puppeteer = require('puppeteer-core'); }
catch {
  if (!process.env.PUPPETEER_CORE) throw new Error('puppeteer-core not found: run `npm install` in docs/recorder, or set PUPPETEER_CORE to its folder');
  puppeteer = require(process.env.PUPPETEER_CORE);
}

function findBrowser() {
  if (process.env.CHROME) return process.env.CHROME;
  const pw = path.join(process.env.LOCALAPPDATA || '', 'ms-playwright');
  const hit = fs.existsSync(pw) && fs.readdirSync(pw).filter(d => /^chromium-\d+$/.test(d)).sort().reverse()
    .map(d => path.join(pw, d, 'chrome-win64', 'chrome.exe')).find(f => fs.existsSync(f));
  return hit || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
}

export function launch() {
  return puppeteer.launch({
    executablePath: findBrowser(), headless: true,
    args: ['--use-angle=d3d11', '--ignore-gpu-blocklist', '--enable-gpu-rasterization', '--force-color-profile=srgb', '--hide-scrollbars', '--autoplay-policy=no-user-gesture-required'],
  });
}
