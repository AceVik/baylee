# Baylee · in sunlight

The project is dedicated to Viktor's much-loved cat Baylee, who died on
26 October 2026. The memorial shows her small, slender and curled up asleep
on a sunlit cloud, with the garden birds and pigeons she loved watching.
The world below suggests a future **Baylees World**: an ivory garden city,
bridges, a turquoise river, coast and mountains beneath a broad cloud horizon.
This is atmosphere for the existing login/lobby, not a playable open world.

The cloud portrait is the owner's explicitly approved fourth iteration. Its
PNG is unchanged: subsequent light, breathing and hover decoration are shader
layers. Both new logos include sleeping Baylee. No external game art was used.

| Runtime asset under `crates/baylee-client/assets/` | Master / preparation |
| --- | --- |
| `scenes/baylee-cloud.png` | Approved V4, 1536 × 1024 RGBA, SHA-256 in manifest |
| `scenes/baylees-world.png` | Generated gate/world, 3840 × 2160 RGB24: frame 0 of the 18.11.2026 loop (the 60 fps intermediate `world.avif` was encoded from), converted with `scale=in_color_matrix=bt709:in_range=tv:flags=accurate_rnd+full_chroma_int+lanczos,format=rgb24`, so the startup still and `vista.wesl`'s `(still - first)` residual match the loop's first frame |
| `scenes/tavern-lobby.png` | The tavern after sign-in, 3840 × 2160 RGB24: the 4K painting the loop was generated from (its start and end frame), with everything below a 16-pixel Gaussian replaced by frame 0 of the loop (`still - gblur(still) + gblur(frame 0)`, `sigma=16`, in 16-bit planar RGB), so it keeps the painting's detail and takes the loop's tones (18.11.2026) |
| `art/baylee/loops/world-source.mp4`, `tavern-source.mp4` | The untouched AI-generated clips (18.11.2026; `docs/legal.md` §"New 4K vista loops"), both 3840 × 2160 at 24 fps, H.264, about 10 s, made with the owner's Artlist AI toolkit account from the owner's two paintings (`login_img.jpeg`, `lobby_img.jpeg`), which were first enlarged to 4K there with an image model (shown as "Nano Banana Pro"): the gate got glowing runes and wind-borne leaves; the tavern's playing cards got frames and backs of their own, so none shows a Wizards card frame or a real card. `world-source.mp4` (image-to-video, shown as "Omni 1.2", start frame only; SHA-256 `f6a79755…ab1ef0`) is tagged SMPTE 170M but its samples are BT.709. `tavern-source.mp4` (shown as "Kling 3.0", the painting as both start and end frame; SHA-256 `3a610f14…7228a7`) is untagged and BT.601, limited range. Not packaged |
| `art/baylee/loops/world-loop.mp4`, `tavern-loop.mp4` (the masters) | Seamless loops at `60/1` fps, BT.709 `tv`, no audio, made from the sources with `ffmpeg 9.0.2` and Topaz Video AI's `tvai_fi` (`apo-8`). **World** (`510` frames / `8.50 s`): source frames `36..=239` (`204` frames, read as BT.709) closed by a smoothstep cross-fade of the last `36` frames into frames `0..=35` (`blend=all_expr='A*(1-w)+B*w'`, `w = (N/36)^2*(3-2*N/36)`), so the wrap is one frame step like any other; padded with `4` frames of itself at either end, converted to RGB48 by the system `ffmpeg` (`scale=…:flags=accurate_rnd+full_chroma_int+bicubic`; Topaz's own conversion shifts colour), interpolated `tvai_fi=model=apo-8:slowmo=1:rdt=-0.01:fps=60`, trimmed to frames `10..520`, converted back to BT.709 `tv`; the committed master is `libx264 -preset slow -crf 17` of that. **Tavern** (`750` frames / `12.50 s`): source frames `1..=240` (frame `0` repeats frame `1`; the clip ends on its start pose), padded with `8` frames at either end, read as BT.601, slowed by a fifth and interpolated in one pass (`tvai_fi=model=apo-8:slowmo=1.25:rdt=-0.01:fps=60`), trimmed to frames `25..775`, sharpened `unsharp=5:5:0.6:5:5:0` towards the painting (`36.2 dB` → `41.9 dB` against the still), `libx264 -preset slow -crf 14`. Not packaged |
| `crates/baylee-client/src/vista/loops/world.avif`, `tavern.avif` | What the client plays (18.11.2026): `3840 × 2160`, `60/1` fps, `510` and `750` frames, `29.44 MiB` and `13.00 MiB`, AVIF image sequences embedded in the binary (`2 × 4` tiles, flat low-delay 1-frame-per-sample without compound altref super-packets so every 4K sample decodes in real time). `libsvtav1 -preset 6 -crf 26` (world, from the 60 fps intermediate before the master's `-crf 17`; `40.4 dB` against it) and `-crf 14` (tavern, from its master; `46.1 dB`), `-loop 0`, with `pred-struct=1:hierarchical-levels=0:tile-rows=1:tile-columns=2:aq-mode=2:sharpness=2:enable-tf=0:enable-restoration=0:enable-cdef=1:film-grain=0:film-grain-denoise=0:use-fixed-qindex-offsets=1:key-frame-qindex-offset=0` (SVT-AV1 4.2.0). `vista::loops::tests::the_shipped_loops_are_what_the_shader_converts` holds a re-encode to what the shader reads: 3840 × 2160 (16:9), 60/1 fps, 8-bit limited-range BT.709, opaque |
| `brand/baylee-logo.png` | Generated transparent wordmark with Baylee, 1942 × 809 |
| `brand/baylee-logo-{48,96,192}.png` | Mechanical downsamples at the named heights |
| `brand/baylee-icon.png` | Seated Baylee crescent-moon icon master, 1254 × 1254 (`painterly-revision.md`) |
| Native/browser icons | `baylee-tools installers art`; browser downsamples 32/180 px |
| `memorial/baylee-{1..5}.jpg` | Owner-supplied photographs, max 1400 px; location/date metadata removed |

[Exact image generation calls](memorial/prompts.md) preserve the iteration
history; [manifest](memorial/assets.json) identifies the selected outputs and
photo sources. The original photographs remain untouched in Pictures. Their
use in this local album was explicitly requested by the owner on 26 October.
The complete owner-approved remembrance is in the client’s German and English
Fluent files: eight chapters and 37 paragraphs, matching Baylee Central. The
“Bei uns / With us” date range is the time with the family, not an invented lifespan.

The interactive cloud has warm contour light, engraved botanical arcs, a small
sun ornament at the pointer, keyboard focus and an accessible button label.
The book supports independent photo/chapter navigation, a scrollable reading
column, Escape, a focus trap and return to its opener. Its header reuses the
approved cloud material; gold ornaments are geometry, independent of fonts. Reduced motion and low ambient quality freeze all idle movement.
The full-screen world uses one texture with restrained parallax, moving light
and cloud haze; it is a layered image with shader effects, not a 3D city.

[Verification](memorial/verification.md) records the live layout/input iterations,
the motion/still pixel comparison, audio measurements and automated checks.

The [previous sanctuary sources](sanctuary.md) remain as historical authoring
material. Current musical provenance is in [the memorial score](../music/memorial/README.md).
