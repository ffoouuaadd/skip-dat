# GIF recorder

Makes the GIFs in `docs/media` from Skip Dat's **real** carousel code (`ui/overlay.html`, `ui/overlay.js`, `ui/settings.html`, `ui/history.html`). A fake Tauri layer feeds it example songs, and a virtual clock steps it one frame at a time, so every GIF is smooth and the same each run.

## Run it

You need Node 18+, `ffmpeg` on your PATH (or named with the `FFMPEG` environment variable), and a Chromium browser. Microsoft Edge works, and so does Playwright's Chromium or any `chrome.exe` you name with the `CHROME` environment variable.

```
cd docs/recorder
npm install
node render_assets.mjs              # draws the wallpaper (wall.html into stage/wall.png)
node rec.mjs                        # records every scene into docs/media
node rec.mjs hero skins             # or just some of them
node rec.mjs skins --no-gif         # record only (frames in frames/)
node rec.mjs skins --gif-only       # encode the frames recorded last time again
```

The scenes are `hero`, `install`, `hold-scroll-release`, `click-and-switch`, `volume-seek`, `song-buttons`, `continue-spotify`, `spotify-jump`, `back-to`, `freaky-random`, `styles`, `skins`, `glass-colour`, `settings`, `history`, `shortcut` and `companion`.

## What's where

| File | What it does |
|---|---|
| `rec.mjs` | Serves the app's UI, records each scene with a 1000 × 625 page at 2× pixels, and encodes 960 px GIFs |
| `scenes.mjs` | What each GIF shows: pointer moves, scrolls, clicks, captions, memes |
| `mock.js` | The fake Tauri, the virtual clock and a small music model (play, jump, Back to, history, Settings, volume, jumping within a song, the song buttons with the app's own messages) |
| `stage/stage.js` | The pretend desktop: taskbar and tray, now-playing card, windows (framed, or floating like Settings), captions, pointer, meme text and stickers |
| `stage/apps.js` | Pretend apps for `install` and `companion`: the setup file, a copy of the installer's pages (`installer/skipdat.nsi`, its own `header.bmp` and `side.bmp`) with its glass animation redrawn from `installer/anim`, a plain browser on its extensions page, and Windows' Select Folder dialog |
| `wall.html` | The desktop background (plain black), in HTML; `render_assets.mjs` turns it into `stage/wall.png`, the desktop every scene uses |
| `art/` | The covers the songs use. The song list (titles, artists and which cover each uses) is at the top of `rec.mjs` |

## How it stays smooth

- The clock moves 20 ms a step, and only when the recorder says so. The overlay, the stage and the mock all run on it.
- Inside the windows (Settings, history), timers and CSS animations run on the same clock, so a panel grows at its real speed.
- A skin's animated pictures (the torch, the mercury, the bubble) are decoded into their frames once and shown by the same clock.
- The real mouse follows the drawn pointer, so the windows show their hover states, and clicks and slider drags in them are real.
- Frames are kept densely while something moves and sparsely at rest. The GIF stores each frame's own delay. Each scene takes the smoothest setting that keeps it at about 9 MB or less (25 or 16.7 frames a second while something moves).

## The covers

All covers are the albums' real artwork, from releases their artists put out under Creative Commons. Each one is listed with its link in the main README's Credits.

- `nin-slip.jpg` and `nin-ghosts.jpg`: Nine Inch Nails, *The Slip* and *Ghosts I–IV*, [CC BY-NC-SA 3.0 US](https://creativecommons.org/licenses/by-nc-sa/3.0/us/).
- `cc-*.jpg`: albums released by Bad Panda Records under [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/), cropped square to 600 px.
- Anything made with them must credit the artists, stay non-commercial and use the same licence.
- To use different art, put the file in `art/` and point the song at it in `rec.mjs`. Only use art you have the rights to.
