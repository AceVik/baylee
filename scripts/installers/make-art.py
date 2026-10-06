#!/usr/bin/env python3
"""Draws the installers' pictures from our own brand icon and fonts.

    uv run --with pillow==11.3.0 scripts/installers/make-art.py

Run by hand when the icon or the layout changes; the outputs are committed,
so CI needs no image library. Everything here is ours: the brand icon
(`crates/baylee-client/assets/brand/baylee-icon.png`), a gradient, an arrow
and Alegreya Sans (OFL, which allows text set in it inside a picture;
`assets/fonts/licenses`). No third-party art (docs/legal.md).

Writes, beside this file:
- macos/background.png, macos/background@2x.png: the disk image window
  (dmgbuild finds the @2x itself). The icon positions are dmg-settings.py's.
- windows/baylee.ico: the setup's, the shortcuts' and Apps & features' icon
  (the executables carry no icon resource).
- linux/baylee.png: 256 px, the .desktop and AppImage icon.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ICON = ROOT / "crates/baylee-client/assets/brand/baylee-icon.png"
FONTS = ROOT / "crates/baylee-client/assets/fonts"

# The window, in points; dmg-settings.py places the icons on this grid.
WIDTH, HEIGHT = 660, 420
APP_X, APPS_X, ICON_Y = 170, 490, 205

# Blue hour: deep at the top, lighter where the icon names sit. Finder
# writes those names black in light mode and white in dark mode, so the band
# under the icons is a middle tone both read against (about 4:1 either way).
TOP = (20, 27, 52)
BAND = (92, 108, 156)
BOTTOM = (128, 140, 182)
GOLD = (243, 213, 138)
INK = (22, 28, 50)


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def background(scale):
    w, h = WIDTH * scale, HEIGHT * scale
    image = Image.new("RGB", (w, h))
    draw = ImageDraw.Draw(image)
    for y in range(h):
        t = y / (h - 1)
        colour = (
            mix(TOP, BAND, t / 0.62)
            if t < 0.62
            else mix(BAND, BOTTOM, (t - 0.62) / 0.38)
        )
        draw.line([(0, y), (w, y)], fill=colour)

    # A few fixed stars in the dark part; no randomness, so a rerun is identical.
    for x, y, r in [
        (58, 34, 1.4),
        (132, 70, 1.0),
        (240, 26, 1.2),
        (402, 58, 1.0),
        (520, 30, 1.5),
        (604, 82, 1.1),
        (318, 96, 0.9),
        (88, 118, 0.9),
    ]:
        r *= scale
        draw.ellipse(
            [x * scale - r, y * scale - r, x * scale + r, y * scale + r], fill=GOLD
        )

    title = ImageFont.truetype(str(FONTS / "AlegreyaSans-Medium.ttf"), 26 * scale)
    small = ImageFont.truetype(str(FONTS / "AlegreyaSans-Regular.ttf"), 12 * scale)
    draw.text(
        (w // 2, 52 * scale),
        "Drag Baylee to Applications",
        font=title,
        fill=GOLD,
        anchor="mm",
    )

    # The arrow between the two icons: a shaft and a head, gold.
    y = ICON_Y * scale
    x0, x1 = (APP_X + 74) * scale, (APPS_X - 74) * scale
    stroke = 5 * scale
    head = 16 * scale
    draw.line([(x0, y), (x1 - head, y)], fill=GOLD, width=stroke)
    draw.polygon(
        [
            (x1, y),
            (x1 - head - 2 * scale, y - head * 0.75),
            (x1 - head - 2 * scale, y + head * 0.75),
        ],
        fill=GOLD,
    )

    draw.text(
        (w // 2, (HEIGHT - 22) * scale),
        "Unofficial fan content, not affiliated with Wizards of the Coast.",
        font=small,
        fill=INK,
        anchor="mm",
    )
    return image


def main():
    background(1).save(HERE / "macos/background.png", optimize=True)
    background(2).save(HERE / "macos/background@2x.png", optimize=True)
    icon = Image.open(ICON).convert("RGBA")
    icon.resize((256, 256), Image.LANCZOS).save(
        HERE / "linux/baylee.png", optimize=True
    )
    # Plain bitmaps rather than PNG entries: every Windows resource reader
    # takes those, older Inno Setup compilers included.
    icon.save(
        HERE / "windows/baylee.ico",
        bitmap_format="bmp",
        sizes=[
            (16, 16),
            (24, 24),
            (32, 32),
            (48, 48),
            (64, 64),
            (128, 128),
            (256, 256),
        ],
    )


if __name__ == "__main__":
    main()
