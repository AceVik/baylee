#!/usr/bin/env python3
"""Draws the installers' pictures and every app icon from our own brand art.

    uv run --with pillow==11.3.0 scripts/installers/make-art.py [--review DIR]

Run by hand when the icon or the layout changes; the outputs are committed,
so CI needs no image library. Everything here is ours: the brand painting
(`crates/baylee-client/assets/brand/baylee-icon.png`, the owner's cat), a
gradient, an arrow, a geometric mask and Alegreya Sans (OFL, which allows
text set in it inside a picture; `assets/fonts/licenses`). No third-party
art (docs/legal.md).

The icon is the square painting cut to a rounded square whose corners are
quarter superellipses (a continuous curve, no visible join where the edge
turns), transparent outside. One shape at every size; only the inset
differs: macOS gets Apple's grid (824 of 1024, with a soft shadow below),
Windows and Linux a near full-bleed body, and the 16-32 px entries a
slightly closer crop and an unsharp mask so they still read as a cat on a
moon.

Writes:
- macos/background.png, macos/background@2x.png (beside this file): the
  disk image window (dmgbuild finds the @2x itself). The icon positions are
  dmg-settings.py's.
- crates/baylee-client/assets/brand/baylee.icns: the bundle's icon, and the
  Dock icon a `cargo run` sets (`app_icon.rs`). Through `iconutil` where it
  exists (macOS), so the 16 and 32 pt entries are there too.
- crates/baylee-client/assets/brand/baylee-window.png: 128 px, the window
  icon a Linux X11 client sets on itself (`window_icon.rs`).
- windows/baylee.ico: embedded as the icon resource of both packaged
  programs, the launcher (baylee-client.exe, built as baylee-launch) and
  the client (baylee-runtime.exe), by build.rs of baylee-update and
  baylee-client; and the setup's, the shortcuts' and Apps & features' icon.
- linux/baylee.png (256 px, the AppImage's) and
  linux/hicolor/<n>x<n>/apps/baylee.png for every size in LINUX_SIZES (the
  .deb's).

--review DIR also writes DIR/icon-old-vs-new.png: the previous square icon
beside the new one, on a dark and a light ground, at several sizes.
"""

import argparse
import math
import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BRAND = ROOT / "crates/baylee-client/assets/brand"
ICON = BRAND / "baylee-icon.png"
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


# The rounded square. Each corner is a quarter superellipse of this exponent
# spanning CORNER of the side: it leaves the straight edge with no jump in
# curvature, which is what makes Apple's corners read as soft rather than as
# a circle stuck on. At exponent 4 a corner spanning 0.40 of the side meets
# the diagonal where a circular corner of 0.22 would (0.159 x 0.40 = 0.293 x
# 0.22), the proportion of the macOS 11 grid, and leaves the edge earlier
# and more gently than that circle does.
EXPONENT = 4.0
CORNER = 0.40
# Supersampling of the mask's edge.
SUPER = 4

# macOS: Apple's grid, a 824 px body on a 1024 px canvas, shadow below.
MAC_BODY = 824 / 1024
# Windows and Linux: almost the whole canvas; neither platform adds a margin
# of its own, and a smaller body only makes the icon look smaller than its
# neighbours.
FLAT_BODY = 0.94
# At these sizes and below, every pixel goes to the picture.
SMALL = 32

WINDOWS_SIZES = [16, 24, 32, 48, 64, 128, 256]
LINUX_SIZES = [16, 24, 32, 48, 64, 128, 256, 512]
WINDOW_ICON = 128
ICONSET = [  # iconutil's names: (points, scale)
    (16, 1), (16, 2), (32, 1), (32, 2), (128, 1), (128, 2),
    (256, 1), (256, 2), (512, 1), (512, 2),
]


def outline(side, steps=64):
    """The rounded square's outline, clockwise, in a side x side box."""
    c = CORNER * side
    corners = [  # centre of each corner's quarter, and its outward signs
        (side - c, c, 1, -1),
        (side - c, side - c, 1, 1),
        (c, side - c, -1, 1),
        (c, c, -1, -1),
    ]
    points = []
    for k, (cx, cy, sx, sy) in enumerate(corners):
        for j in range(steps + 1):
            # Each quarter runs from one edge to the next: clockwise.
            t = math.radians(k * 90 + j * 90 / steps - 90)
            x = abs(math.cos(t)) ** (2 / EXPONENT)
            y = abs(math.sin(t)) ** (2 / EXPONENT)
            points.append((cx + x * c * sx, cy + y * c * sy))
    return points


def mask(side):
    """The rounded square as an anti-aliased L mask of side x side."""
    big = side * SUPER
    m = Image.new("L", (big, big), 0)
    ImageDraw.Draw(m).polygon(outline(big), fill=255)
    return m.resize((side, side), Image.BOX)


def painting(side, small):
    """The brand painting at side px; a closer crop for the smallest."""
    source = Image.open(ICON).convert("RGB")
    if small:
        # The cat and the moon's lit edge; the clouds and the flowers at the
        # border are what a 16 px picture can least afford.
        w = source.width
        source = source.crop((round(w * 0.08), round(w * 0.04), round(w * 0.92), round(w * 0.88)))
    out = source.resize((side, side), Image.LANCZOS)
    if small:
        out = out.filter(ImageFilter.UnsharpMask(radius=1, percent=80, threshold=1))
    return out


def body(side, small=False):
    """The rounded painting, side x side, transparent outside its corners."""
    image = painting(side, small).convert("RGBA")
    image.putalpha(mask(side))
    return image


def flat_icon(size):
    """Windows and Linux: near full bleed, centred, transparent around."""
    if size <= SMALL:
        return body(size, small=True)
    inner = round(size * FLAT_BODY)
    inner -= (size - inner) % 2
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.alpha_composite(body(inner), ((size - inner) // 2, (size - inner) // 2))
    return canvas


def mac_icon(size):
    """macOS: Apple's grid, with the soft shadow its own icons carry."""
    if size <= SMALL:
        return body(size, small=True)
    inner = round(size * MAC_BODY)
    inner -= (size - inner) % 2
    at = (size - inner) // 2
    shadow = Image.new("L", (size, size), 0)
    shadow.paste(mask(inner), (at, at + round(size * 0.012)))
    shadow = shadow.filter(ImageFilter.GaussianBlur(size * 0.014))
    shadow = shadow.point(lambda v: round(v * 0.45))
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.putalpha(shadow)
    canvas.alpha_composite(body(inner), (at, at))
    return canvas


def save_icns(path):
    """The bundle icon; iconutil where there is one, else Pillow's writer."""
    if shutil.which("iconutil"):
        with tempfile.TemporaryDirectory() as tmp:
            iconset = Path(tmp) / "baylee.iconset"
            iconset.mkdir()
            for points, scale in ICONSET:
                suffix = "" if scale == 1 else "@2x"
                mac_icon(points * scale).save(iconset / f"icon_{points}x{points}{suffix}.png")
            subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(path)], check=True)
    else:
        # Pillow writes no 16 and 32 pt entries; Finder scales the next ones.
        print("no iconutil: baylee.icns without its 16 and 32 pt entries")
        sizes = sorted({p * s for p, s in ICONSET})
        images = [mac_icon(n) for n in sizes]
        images[-1].save(path, append_images=images[:-1])


def review(directory):
    """The old square icon and the new rounded one, side by side."""
    old = Image.open(ICON).convert("RGBA")
    sizes = [256, 128, 64, 32, 16]
    pad = 24
    label = ImageFont.truetype(str(FONTS / "AlegreyaSans-Medium.ttf"), 22)
    small = ImageFont.truetype(str(FONTS / "AlegreyaSans-Regular.ttf"), 16)
    rows = [
        ("before (square)", lambda n: old.resize((n, n), Image.LANCZOS)),
        ("after: Windows / Linux", flat_icon),
        ("after: macOS (Apple grid)", mac_icon),
    ]
    grounds = [(18, 22, 40), (236, 238, 242)]
    width = pad + sum(n + pad for n in sizes)
    row_h = 256 + pad * 2 + 30
    sheet = Image.new("RGBA", (width * 2, row_h * len(rows) + 50), (255, 255, 255, 255))
    draw = ImageDraw.Draw(sheet)
    for g, ground in enumerate(grounds):
        draw.rectangle([g * width, 0, (g + 1) * width, sheet.height], fill=ground)
        ink = (240, 240, 240) if g == 0 else (20, 20, 30)
        draw.text((g * width + pad, 12), "dark ground" if g == 0 else "light ground", font=label, fill=ink)
        for r, (name, make) in enumerate(rows):
            y = 50 + r * row_h
            draw.text((g * width + pad, y), name, font=label, fill=ink)
            x = g * width + pad
            for n in sizes:
                sheet.alpha_composite(make(n), (x, y + 34 + (256 - n)))
                draw.text((x, y + 34 + 256 + 4), f"{n}px", font=small, fill=ink)
                x += n + pad
    directory.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(directory / "icon-old-vs-new.png", optimize=True)
    # The new icons at full size, to look at closely.
    mac_icon(1024).save(directory / "new-macos-1024.png", optimize=True)
    flat_icon(512).save(directory / "new-flat-512.png", optimize=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--review", type=Path, help="also write the before/after sheet here")
    args = parser.parse_args()

    background(1).save(HERE / "macos/background.png", optimize=True)
    background(2).save(HERE / "macos/background@2x.png", optimize=True)

    save_icns(BRAND / "baylee.icns")
    flat_icon(WINDOW_ICON).save(BRAND / "baylee-window.png", optimize=True)

    flat_icon(256).save(HERE / "linux/baylee.png", optimize=True)
    for n in LINUX_SIZES:
        target = HERE / f"linux/hicolor/{n}x{n}/apps/baylee.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        flat_icon(n).save(target, optimize=True)

    # Plain bitmaps rather than PNG entries: every Windows resource reader
    # takes those, older Inno Setup compilers included. Each size is drawn
    # for itself (append_images) rather than scaled down from the largest.
    images = [flat_icon(n) for n in WINDOWS_SIZES]
    images[-1].save(
        HERE / "windows/baylee.ico",
        bitmap_format="bmp",
        sizes=[(n, n) for n in WINDOWS_SIZES],
        append_images=images[:-1],
    )

    if args.review:
        review(args.review)


if __name__ == "__main__":
    main()
