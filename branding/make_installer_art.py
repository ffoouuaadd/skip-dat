"""Artwork for the Windows installer: the welcome/finish side panel and the page header.
Drawn at twice the size NSIS expects, so it stays sharp on high-DPI screens."""
from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'installer')
ICON = Image.open(os.path.join(HERE, 'out', 'icon-1024.png')).convert('RGBA')
FONT = '/usr/share/fonts/truetype/inter/InterDisplay-Bold.ttf'
for cand in [FONT, '/usr/share/fonts/opentype/inter/InterDisplay-Bold.otf']:
    if os.path.exists(cand):
        FONT = cand
        break
else:
    import subprocess
    FONT = subprocess.run(['fc-match', '-f', '%{file}', 'Inter Display:bold'], capture_output=True, text=True).stdout

def vgrad(w, h, top, bottom):
    g = Image.linear_gradient('L').resize((w, h))
    return Image.composite(Image.new('RGB', (w, h), bottom), Image.new('RGB', (w, h), top), g)

# side panel: 164x314 at 2x
W, H = 328, 628
im = vgrad(W, H, (21, 30, 45), (11, 15, 23)).convert('RGBA')
glow = Image.new('RGBA', (W, H), (0, 0, 0, 0))
ImageDraw.Draw(glow).ellipse([W // 2 - 150, 120, W // 2 + 150, 420], fill=(246, 120, 180, 70))
im = Image.alpha_composite(im, glow.filter(ImageFilter.GaussianBlur(60)))
ic = ICON.resize((220, 220), Image.LANCZOS)
im.alpha_composite(ic, ((W - 220) // 2, 150))
d = ImageDraw.Draw(im)
f = ImageFont.truetype(FONT, 46)
d.text((W // 2, 420), 'Skip Dat', font=f, fill=(250, 251, 253), anchor='mm')
f2 = ImageFont.truetype(FONT.replace('Bold', 'Medium') if os.path.exists(FONT.replace('Bold', 'Medium')) else FONT, 19)
d.text((W // 2, 462), 'hold Alt + scroll', font=f2, fill=(169, 182, 202), anchor='mm')
im.convert('RGB').save(os.path.join(OUT, 'side.bmp'))

# header: 150x57 at 2x, on the white MUI header
W, H = 300, 114
hd = Image.new('RGBA', (W, H), (255, 255, 255, 255))
ic = ICON.resize((96, 96), Image.LANCZOS)
hd.alpha_composite(ic, (W - 96 - 12, (H - 96) // 2))
hd.convert('RGB').save(os.path.join(OUT, 'header.bmp'))
print('ok', FONT)
