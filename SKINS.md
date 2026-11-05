# Making a Skip Dat skin

A skin changes how the carousel looks: colours, font, corners, glass or solid panels, pictures. It's a folder with a `skin.json` file in it, plus any pictures and fonts it uses.

## Where skins go

Open **Settings → Skin → Open skins folder**. That's `%APPDATA%\app.skipdat.desktop\skins`. Each folder in it is one skin, and `my-first-skin` is there to start from.

Skip Dat watches this folder. Save a file and the carousel updates straight away, so you can keep the carousel open (hold Alt and scroll) while you edit. Pick your skin under **Settings → Skin**.

## skin.json

Everything is optional; anything you leave out keeps the default. Colours are CSS colours (`"#c6ff4d"`, `"rgba(0,0,0,.5)"`). Sizes are in pixels at the Medium carousel size.

```json
{
  "name": "My Skin",
  "author": "you",
  "base": "panel",

  "text":   { "font": "myfont.woff2", "family": "Segoe UI", "color": "#ffffff", "soft": "#b0b0b0",
              "title": 21, "artist": 15, "weight": 650, "glow": "rgba(0,0,0,.3)", "upper": false,
              "shadow": "2px 2px 0 #000" },

  "card":   { "radius": 24, "scale": 1, "top": "#2a2f3a", "bottom": "#14171d",
              "edge": "#000000", "edgeWidth": 1, "bevel": 4, "gloss": 0.3, "scanlines": 0,
              "highlight": "#ffffff", "dark": "#000000", "texture": "panel.png", "textureMix": 0.5,
              "textureMode": "normal", "texturePixels": false },

  "cover":  { "radius": 12, "frame": 4, "frameColor": "#888888", "frameStyle": "solid",
              "tint": "#5cff86", "tintAmount": 0, "pixelate": 0 },

  "button": { "top": "#3a3f4a", "bottom": "#1a1d23", "edge": "#000000", "icon": "#ffffff",
              "gloss": 0.3, "bevel": 3, "scanlines": 0 },

  "chip":   { "top": "#2a2f3a", "bottom": "#14171d", "text": "#dddddd",
              "activeTop": "#5b8cff", "activeBottom": "#2a55c8", "activeText": "#ffffff" },

  "glass":  { "frost": 1, "tint": "#ffffff", "tintAmount": 0.06 },
  "decoration": [ { "image": "hat.png", "width": 132, "x": 0, "y": -30 },
                  { "image": "shoes.png", "width": 128, "y": 60, "anchor": "bottom" } ],
  "shadow": 1
}
```

| Key | What it does |
|---|---|
| `base` | `"glass"`: liquid glass that shows your desktop through the cards (uses `glass`). `"panel"`: solid cards drawn from `card` (lighter on the graphics card). |
| `text.font` | A `.woff2`, `.woff`, `.ttf` or `.otf` file in the skin's folder. `family` is a font installed on Windows, used when there's no file. |
| `text.title` / `text.artist` | Font sizes. `weight` 100–900, `upper` makes everything capitals, `glow` is a soft shadow behind the text. `shadow` replaces it with any CSS text-shadow (`"2px 2px 0 #000"` for a hard pixel shadow). |
| `card.top` / `card.bottom` | The panel's colour, as a gradient from top to bottom. |
| `card.edge` / `edgeWidth` | An outline. |
| `card.bevel` | A chunky 3D edge this many pixels wide, lit with `highlight` and shaded with `dark`. |
| `card.gloss` | A shiny plastic highlight across the top, 0–1. |
| `card.scanlines` | LCD-style lines, 0–1. |
| `card.texture` | A picture stretched over the card, mixed in by `textureMix` (0–1). PNG, JPEG or WebP, up to 16 MB. By default it shades the card's colours (good for grey textures like brushed metal); `"textureMode": "normal"` shows its own colours. `"texturePixels": true` keeps pixel art sharp. |
| `card.scale` | Card size, 0.6–1.4. |
| `cover.frame` | A frame around the album art, `frameStyle` `"solid"`, `"chrome"` or `"inset"`. |
| `cover.tint` / `tintAmount` | Colours the album art (for an LCD look). |
| `cover.pixelate` | Turns the album art into big pixels: how many across (for example `28`). `0` is off. |
| `decoration` | Pictures the card in front wears (a hat, shoes, a sticker): one `{ … }` or a list of up to four. `width` in pixels; `x` and `y` move it from the middle of the card's top edge, or its bottom edge with `"anchor": "bottom"` (negative `y` is up). |
| `button` / `chip` | The play buttons and the app names beside the carousel. |
| `glass` | For glass skins: `frost` (how blurry the desktop behind looks, 0–2), a `tint` colour and how much. |
| `shadow` | Shadow strength, 0–2. |

Performance mode (Settings → Options) overrides a skin: no textures, gloss, blur or shadows.

## Tips

- If `skin.json` has a mistake, Settings shows the error next to the skin, and the carousel keeps the default look until it's fixed.
- Fonts: only use fonts whose licence allows sharing them (for example from Google Fonts, under the SIL Open Font License), and put the licence file in the folder.
- Pictures: use your own, or ones you're allowed to share.
- To share a skin, zip its folder. Others unzip it into their skins folder.
