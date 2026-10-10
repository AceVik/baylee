# Baylee · the sanctuary

Baylee is the owner's cat and the project's namesake. This identity is a
celebration of her: green eyes, tabby forehead, white muzzle and chest. The
guardian is alive, curious and welcoming. The owner's permission to use her
as a mascot was given on 26 September 2026.

## Assets

| Runtime asset, under `crates/baylee-client/assets/` | Source and use |
| --- | --- |
| `brand/baylee-logo.png` | Transparent 1942 × 809 title lockup; generated from the owner's cat reference |
| `brand/baylee-logo-{48,96,192}.png` | Lanczos downsamples of the lockup (trimmed) at 48, 96 and 192 px high: the shell header's brand mark picks the one nearest its drawn height in physical pixels, so it stays crisp at every scale |
| `brand/baylee-icon.png` | 1254 × 1254 icon master; generated using the logo as identity reference |
| `brand/favicon-32.png` | Mechanical downsample for browser tabs |
| `brand/apple-touch-icon.png` | 180 × 180 browser home-screen icon |
| `brand/baylee.icns` | Native macOS sizes, the master cut to a rounded square on Apple's grid by `scripts/installers/make-art.py` (through `iconutil`); release packaging installs it |
| `brand/baylee-window.png` | 128 × 128 rounded window icon for Linux X11, from the same script |
| `scenes/sanctuary-world.png` | Distant landscape with near architecture removed for independent parallax |
| `scenes/sanctuary-frame.png` | Transparent near arch, lamps, ivy and steps |
| `scenes/baylee-guardian.png` | Transparent 1086 × 1448 hand-painted petite Baylee, fitted crimson body and slender tapered tail |
| `scenes/wayfinder-lantern.png` | Transparent 600 × 800 Cycles render of the original Blender lantern |

The original composition is retained as `art/baylee/sanctuary-master.png`;
only its separated runtime layers are shipped.

The generated masters and their refinements use the built-in image generation tool. Exact
prompts are in [prompts.md](prompts.md) and [revisions.md](revisions.md). The supplied photographs remain in
the owner's Pictures directory; none are copied into the project. No external
game screenshot, model, texture, logo or font was used as an image input.

The composition borrows the *idea* of an inviting threshold and cinematic
depth from the requested Midnight reference. The limestone botanical arch,
conservatory, streams and Baylee identity are original. The previous geode
and inward-pointing teeth are completely replaced.

## Editable Blender source

[wayfinder-lantern.blend](wayfinder-lantern.blend) contains the dedicated
`Baylee • sanctuary lantern` scene, materials, lights and portrait camera.
[lantern.py](lantern.py) constructs it in Blender 5.2.2 LTS.
[wayfinder-lantern.glb](wayfinder-lantern.glb) exports only the active lantern
scene's selected meshes, with applied scale and glTF Y-up conversion. It is
an editing/interchange asset; the runtime uses the transparent render, avoiding
50 mesh draws for a small foreground prop. The unrelated original Blender
scene was preserved. The script creates a new scene each run.

## Runtime art direction

Champagne gold and warm amber guide the eye; blue-green shadow and silver
moonlight hold the world. A dark opaque form protects input readability.
Artwork preserves its aspect ratio. Baylee and the lantern stand on separate
foreground planes with contact shadows and pointer parallax. Mist, water,
moonbeams, lamp flicker, fireflies and subtle breathing share virtual time.
Reduced motion produces a stable still. Narrow screens omit foreground props
that would collide with controls; the logo retains Baylee's identity.

The arrival curtain retains and waits for all login art before revealing the
screen. The legal footer blends into the scene through translucent blue mist and remains visible while the form body scrolls. See `docs/legal.md` §10 for the
source-checked asset audit. Original project materials are distributed under
AGPL-3.0-only, to the extent rights apply; existing font notices remain intact.

## Inspecting in the client

```sh
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT="$PWD" \
  cargo run -p baylee-client --features dev-control,dev-reload
```

Add `BAYLEE_DEV_WINDOW=1280x768` or `390x844` for repeatable logical viewports.
This override exists only in dev-control builds. `/pause`, `/step`, `/pointer`
and `/screenshot` support actual renderer checks; do not judge motion solely
from a still image.

The owner's follow-up replaces the photographic mascot with grouped painted
fur and moon/lantern lighting. Logo and icon now share a seated Baylee, crescent
moon and wisteria; the earlier frontal cat-head crest is retired. Exact revised
prompts are recorded in `painterly-revision.md`.

## Spatial garden follow-up

Login, lobby and deckbuilder now share a hybrid garden: painted architecture,
ray-intersected 3D water waves and a wind-driven cloud volume rendered in WGSL.
The moon's light is occluded by that cloud volume. This is not a fully modelled
3D landscape. No additional third-party assets are introduced. Footer sentences
are centred symmetrically and its source QR no longer offsets the text block.
