# GIF recorder

Makes the GIFs in `docs/media` from Skip Dat's **real** carousel code (`ui/overlay.html`, `ui/overlay.js`, `ui/settings.html`, `ui/history.html`). A fake Tauri layer feeds it example songs, and a virtual clock steps it one frame at a time, so every GIF is smooth and the same each run.

## Run it

You need Node 18+, `ffmpeg` on your PATH, and a Chromium browser. Microsoft Edge works, and so does Playwright's Chromium or any `chrome.exe` you name with the `CHROME` environment variable.

```
cd docs/recorder
npm install
node render_assets.mjs     # draws the wallpaper (wall.html into stage/wall.png)
node rec.mjs               # records every scene into docs/media
node rec.mjs hero skins    # or just some of them
```

The scene names are `hero`, `hold-scroll-release`, `click-and-switch`, `spotify-jump`, `back-to`, `styles`, `skins`, `history` and `shortcut`.

## What's where

| File | What it does |
|---|---|
| `rec.mjs` | Serves the app's UI, records each scene with a 1000 × 625 page at 2× pixels, and encodes 960 px GIFs (Bayer dither) |
| `scenes.mjs` | What each GIF shows: pointer moves, scrolls, clicks, captions, memes |
| `mock.js` | The fake Tauri, the virtual clock and a small music model (play, jump, Back to, history, Settings) |
| `stage/stage.js` | The pretend desktop: taskbar and tray, now-playing card, windows, captions, pointer, meme text and stickers |
| `wall.html` | The desktop background (plain black), in HTML; `render_assets.mjs` turns it into `stage/wall.png`, the desktop every scene uses |
| `art/` | The covers the songs use. The song list (titles, artists and which cover each uses) is at the top of `rec.mjs` |

## The covers

All covers are the albums' real artwork, from releases their artists put out under Creative Commons. Each one is listed with its link in the main README's Credits.

- `nin-slip.jpg` and `nin-ghosts.jpg`: Nine Inch Nails, *The Slip* and *Ghosts I–IV*, [CC BY-NC-SA 3.0 US](https://creativecommons.org/licenses/by-nc-sa/3.0/us/).
- `cc-*.jpg`: albums released by Bad Panda Records under [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/), cropped square to 600 px.
- Anything made with them must credit the artists, stay non-commercial and use the same licence.
- To use different art, put the file in `art/` and point the song at it in `rec.mjs`. Only use art you have the rights to.
