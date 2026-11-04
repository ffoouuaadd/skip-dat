"""Skip Dat icon: the pink soap bar from the reference, tilted, with a thick dark outline, made of
liquid glass like the app: an inner rim where the glass is thick, light gathering at its lower edge,
a broad curved reflection and a crisp highlight on the upper edge, and a raised glass skip symbol.
Rendered large and downsampled per size; the small sizes get a simplified drawing (no side face, a
bigger solid symbol, relatively thicker outline) so they stay readable in the taskbar and tray."""
import math
import sys
from PIL import Image, ImageDraw, ImageFilter, ImageChops

OUTLINE = (34, 10, 26, 255)      # deep plum, reads as black at small sizes
SIDE = (190, 34, 104, 255)       # bar thickness
TOP_A = (255, 132, 190, 255)     # top face, upper
TOP_B = (241, 74, 153, 255)      # top face, lower
GLYPH = (255, 192, 224, 255)     # embossed symbol
GLYPH_SHADE = (176, 24, 92, 255)


def rounded(size, box, r):
    m = Image.new('L', size, 0)
    ImageDraw.Draw(m).rounded_rectangle(box, radius=r, fill=255)
    return m


def gradient(size, a, b):
    # a vertical blend built from PIL's 256-step ramp, stretched to size (fast at any size)
    ramp = Image.linear_gradient('L').resize(size, Image.BILINEAR)
    return Image.composite(Image.new('RGBA', size, b), Image.new('RGBA', size, a), ramp)


def glyph_mask(size, cx, cy, s):
    """skip-forward: two triangles and a bar, centred on (cx, cy), height s"""
    m = Image.new('L', size, 0)
    d = ImageDraw.Draw(m)
    h = s / 2
    w = s * 0.62
    x0 = cx - w * 1.18
    d.polygon([(x0, cy - h), (x0 + w, cy), (x0, cy + h)], fill=255)
    d.polygon([(x0 + w * 0.92, cy - h), (x0 + w * 1.92, cy), (x0 + w * 0.92, cy + h)], fill=255)
    bw = s * 0.17
    bx = x0 + w * 1.92 + s * 0.04
    d.rounded_rectangle((bx, cy - h, bx + bw, cy + h), radius=bw * 0.3, fill=255)
    # round the sharp corners a touch so it reads as moulded soap
    return m.filter(ImageFilter.GaussianBlur(s * 0.02)).point(lambda v: 255 if v > 110 else 0)


def dilate(m, r):
    """grow a mask by about r pixels"""
    if r < 1:
        return m
    return m.filter(ImageFilter.GaussianBlur(r * 0.55)).point(lambda v: 255 if v > 20 else v * 12)


def erode(m, r):
    """shrink a mask by about r pixels"""
    return ImageChops.invert(dilate(ImageChops.invert(m), r))


def solid(size, color, mask):
    im = Image.new('RGBA', size, color)
    im.putalpha(mask)
    return im


def render(S, simple):
    W = S * (8 if S <= 32 else 4 if S <= 128 else 2)   # supersample, more for the tiny sizes
    size = (W, W)
    tilt = -9 if not simple else -6
    bw, bh = (0.80 * W, 0.52 * W) if not simple else (0.86 * W, 0.60 * W)
    r = bh * 0.22
    cx, cy = W / 2, W / 2 - (0.03 * W if not simple else 0)
    depth = 0 if simple else 0.07 * W
    ol = (0.055 if not simple else 0.085) * W

    layer = Image.new('RGBA', size, (0, 0, 0, 0))
    box = (cx - bw / 2, cy - bh / 2, cx + bw / 2, cy + bh / 2)
    top_m = rounded(size, box, r)
    side_m = rounded(size, (box[0], box[1] + depth, box[2], box[3] + depth), r) if depth else top_m
    body_m = ImageChops.lighter(top_m, side_m)

    # thick outline around the whole bar
    grow = dilate(body_m, ol)
    layer = Image.alpha_composite(layer, solid(size, OUTLINE, grow))
    if depth:
        layer = Image.alpha_composite(layer, solid(size, SIDE, side_m))
    face = gradient(size, TOP_A, TOP_B)
    face.putalpha(top_m)
    layer = Image.alpha_composite(layer, face)

    if not simple:
        # ---- liquid glass ----
        # the glass is thicker at its edges: a darker inner rim
        rim = ImageChops.subtract(top_m, erode(top_m, bh * 0.10)).filter(ImageFilter.GaussianBlur(bh * 0.05))
        layer = Image.alpha_composite(layer, solid(size, (176, 30, 98, 255), ImageChops.multiply(rim.point(lambda v: v * 0.55), top_m)))
        # light passing through gathers along the lower edge
        glow = Image.new('L', size, 0)
        ImageDraw.Draw(glow).ellipse((box[0] + bw * 0.12, box[3] - bh * 0.30, box[2] - bw * 0.12, box[3] + bh * 0.12), fill=170)
        glow = ImageChops.multiply(glow.filter(ImageFilter.GaussianBlur(bh * 0.09)), top_m)
        layer = Image.alpha_composite(layer, solid(size, (255, 196, 224, 255), glow))
        # a broad curved reflection over the upper part, with a soft lower edge
        refl = Image.new('L', size, 0)
        ImageDraw.Draw(refl).ellipse((box[0] - bw * 0.25, box[1] - bh * 0.95, box[2] - bw * 0.05, box[1] + bh * 0.46), fill=92)
        refl = ImageChops.multiply(refl.filter(ImageFilter.GaussianBlur(bh * 0.025)), erode(top_m, bh * 0.035))
        layer = Image.alpha_composite(layer, solid(size, (255, 255, 255, 255), refl))
        # a diagonal streak, the way light runs across a sheet of glass
        streak = Image.new('L', size, 0)
        sd = ImageDraw.Draw(streak)
        x0 = box[0] + bw * 0.10
        sd.polygon([(x0, box[3]), (x0 + bw * 0.16, box[3]), (x0 + bw * 0.42, box[1]), (x0 + bw * 0.26, box[1])], fill=46)
        streak = ImageChops.multiply(streak.filter(ImageFilter.GaussianBlur(bh * 0.02)), erode(top_m, bh * 0.05))
        layer = Image.alpha_composite(layer, solid(size, (255, 255, 255, 255), streak))
        # a crisp highlight where the light catches the upper-left edge
        k = max(2, int(bh * 0.035))
        edge = ImageChops.subtract(top_m, ImageChops.offset(top_m, k, k)).filter(ImageFilter.GaussianBlur(k * 0.35))
        fade = gradient(size, (255, 255, 255, 255), (255, 255, 255, 0)).getchannel('A')   # strongest at the top
        layer = Image.alpha_composite(layer, solid(size, (255, 255, 255, 255), ImageChops.multiply(edge, fade)))
        # the lower edge of the side face catches light too
        if depth:
            low = ImageChops.subtract(side_m, ImageChops.offset(side_m, 0, -k)).filter(ImageFilter.GaussianBlur(k * 0.4))
            layer = Image.alpha_composite(layer, solid(size, (255, 170, 210, 255), ImageChops.multiply(low.point(lambda v: v * 0.7), ImageChops.subtract(side_m, top_m))))
    else:
        # small sizes: one simple sheen
        sheen = Image.new('L', size, 0)
        ImageDraw.Draw(sheen).rounded_rectangle((box[0] + bh * 0.10, box[1] + bh * 0.07, box[2] - bh * 0.10, box[1] + bh * 0.36), radius=r * 0.8, fill=120)
        sheen = ImageChops.multiply(sheen.filter(ImageFilter.GaussianBlur(bh * 0.05)), top_m)
        layer = Image.alpha_composite(layer, solid(size, (255, 255, 255, 255), sheen))

    # the skip symbol: raised clear glass (large sizes), solid white with an outline (small sizes)
    gs = bh * (0.50 if not simple else 0.60)
    g = glyph_mask(size, cx + bw * 0.02, cy + bh * 0.02, gs)
    off = max(2, int(gs * 0.07))
    if not simple:
        # the glass symbol bends light: a deeper shadow down-right, a bright lip up-left
        shade = ImageChops.offset(g, int(off * 1.3), int(off * 1.3)).filter(ImageFilter.GaussianBlur(off * 0.55))
        layer = Image.alpha_composite(layer, solid(size, (128, 8, 66, 255), ImageChops.multiply(shade.point(lambda v: v * 0.9), top_m)))
        gface = gradient(size, (255, 255, 255, 255), (255, 206, 230, 255))
        gface.putalpha(g.point(lambda v: v * 0.97))
        layer = Image.alpha_composite(layer, gface)
        inner = ImageChops.subtract(g, erode(g, off * 0.9)).filter(ImageFilter.GaussianBlur(off * 0.3))
        layer = Image.alpha_composite(layer, solid(size, (226, 92, 152, 255), ImageChops.multiply(inner.point(lambda v: v * 0.45), ImageChops.offset(g, off // 2, off // 2))))
        lip = ImageChops.subtract(g, ImageChops.offset(g, max(1, off // 2), max(1, off // 2))).filter(ImageFilter.GaussianBlur(off * 0.25))
        layer = Image.alpha_composite(layer, solid(size, (255, 255, 255, 255), lip))
    else:
        gol = dilate(g, ol * 0.55)
        layer = Image.alpha_composite(layer, solid(size, OUTLINE, gol))
        layer = Image.alpha_composite(layer, solid(size, (255, 236, 246, 255), g))

    layer = layer.rotate(tilt, resample=Image.BICUBIC, center=(W / 2, W / 2))
    return layer.resize((S, S), Image.LANCZOS)


def main(out):
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024]
    imgs = {s: render(s, simple=s <= 64) for s in sizes}
    imgs[1024].save(f'{out}/icon-1024.png')
    imgs[512].save(f'{out}/icon.png')
    imgs[256].save(f'{out}/128x128@2x.png')
    imgs[128].save(f'{out}/128x128.png')
    imgs[32].save(f'{out}/32x32.png')
    for s in (16, 48, 128):
        imgs[s].save(f'{out}/ext-{s}.png')
    ico = [imgs[s] for s in (16, 20, 24, 32, 40, 48, 64, 128, 256)]
    ico[-1].save(f'{out}/icon.ico', sizes=[(i.width, i.height) for i in ico], append_images=ico[:-1])
    # preview sheet on light and dark taskbar colours
    sheet = Image.new('RGBA', (900, 300), (0, 0, 0, 0))
    for row, bg in enumerate([(243, 243, 243, 255), (32, 32, 32, 255)]):
        band = Image.new('RGBA', (900, 150), bg)
        x = 12
        for s in (16, 24, 32, 48, 64, 128):
            band.alpha_composite(imgs[s], (x, (150 - s) // 2))
            x += s + 24
        sheet.alpha_composite(band, (0, row * 150))
    big = imgs[256]
    sheet.alpha_composite(Image.new('RGBA', (300, 300), (0, 0, 0, 0)), (600, 0))
    sheet.alpha_composite(big.resize((280, 280), Image.LANCZOS), (610, 10))
    sheet.save(f'{out}/preview.png')


if __name__ == '__main__':
    main(sys.argv[1])
