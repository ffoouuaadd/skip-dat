// renders the wallpaper (wall.html) to stage/wall.png at 2x pixels
import { launch } from './pp.mjs';
import { pathToFileURL, fileURLToPath } from 'node:url';
import path from 'node:path';
const here = path.dirname(fileURLToPath(import.meta.url));
const b = await launch();
const p = await b.newPage();
await p.setViewport({ width: 1000, height: 625, deviceScaleFactor: 2 });
await p.goto(pathToFileURL(path.join(here, 'wall.html')).href, { waitUntil: 'load' });
await p.evaluate(() => document.fonts.ready);
await p.screenshot({ path: path.join(here, 'stage', 'wall.png') });
await b.close();
console.log('ok');
