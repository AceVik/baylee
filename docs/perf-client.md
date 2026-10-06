# Client performance (6 October 2026)

What a frame of the Bevy client costs, where it went, and what was changed.
The owner's report was a MacBook (M1) whose fans spun up with a client that
draws little; the measurements below say why: every screen was bound by the
GPU's fragment work at native Retina resolution, and bevy's default drew as
many frames as the display offered.

## How it was measured

- **Machine.** M1 Max, 64 GB, macOS. Two displays were used on the day and the
  rows say which: a 6K external at 120 Hz (window 1920×1080 logical =
  3840×2160 pixels) and the built-in Liquid Retina at 120 Hz (the window
  clamps to 1708×1032 logical = 3416×2064 pixels). Builds are `--release
  --features dev-control`; the "before" binary is a27c5139.
- **GPU time** comes from Xcode's `xctrace record --template 'Metal System
  Trace' --attach <pid>`, exported as `metal-gpu-intervals` and summed per
  process: *ms/s busy* is GPU time per wall second (what costs power), *ms per
  frame* the same over frames. A GPU that is barely used clocks down, so ms per
  frame rises at low rates; compare ms/s for power. Bevy's
  `RenderDiagnosticsPlugin` records only CPU time on Metal (its own doc), and
  `powermetrics` needs `sudo`, which is password-gated here: neither was used.
- **CPU** is the process's CPU time over wall time (`ps -o cputime`), all
  threads; **energy** is `top`'s energy-impact column, averaged; **RSS** from
  `ps`.
- **Frame time, main-schedule time, entities, systems, allocations per
  frame** from the dev harness's `/perf` (`crates/baylee-client/src/devctl/
  perf.rs`; allocations through a counting global allocator in `dev-control`
  builds only).
- **What a drawing costs and what it shows**: `/hide {"what":"<material>"}`
  takes one kind of drawing away; GPU time is traced with and without it, and
  two screenshots of the same paused frame (`/pause`) are diffed: RMSE in
  8-bit levels and the share of pixels changed by more than 2 and 8 levels. A
  shader variant is compared the same way through `dev-reload` (hot WGSL
  reload): paused, screenshot, swap the source, screenshot, restore,
  screenshot — the restored frame is checked identical to the first, which
  proves the pause holds the picture.
- **Load.** Other sessions compiled on the machine during most runs (load
  average 11–60 on 10 cores). GPU numbers are barely touched by that; CPU
  percentages are, so the A/B pairs that decide something were taken in one
  process, seconds apart.

## Before and after

6K external display, 3840×2160 pixels:

| Screen | Build | Frames/s | GPU ms/frame | GPU busy | CPU | Energy |
|---|---|---|---|---|---|---|
| Front door | before | 32 (GPU-bound) | 30.0 | 111 % | 28 % | 27 |
| Front door | after, active (Medium cap) | 60 | 7.6 | 38 % | 36 % | 35 |
| Lobby | before | 31 (GPU-bound) | 31.0 | 103 % | 28 % | 27 |
| Table, empty board | before | 57 (GPU-bound) | 14.9 | 100 % | 64 % | 62 |
| Table, empty board | after (cap 60) | 60 | 6.2 | 45 % | 56 % | 54 |

Built-in display, 3416×2064 pixels, final build:

| Screen | Build | Frames/s | GPU ms/frame | GPU busy | CPU | Energy | RSS |
|---|---|---|---|---|---|---|---|
| Busy duel (46 permanents) | before | 77 | 10.8 | 84 % | 67 % | 65 | 571 MB |
| Busy duel | after | 56–60 (cap) | 6.4 | 45 % | 50 % | 49 | 490 MB |
| Front door, settled (2 s untouched) | after | 30 | — | — | 18 % | 17 | 357 MB |
| Front door, resting (30 s untouched) | after | 1 | 8 (clocked down) | 1.4 % | 4 % | 4 | 357 MB |

What is left of the resting front door's 4 % is the score, which plays on.
Startup to the harness answering is 1.2–2.4 s for both builds, and RSS six
seconds after launch about 405 MB for both: neither moved.

Per frame, final build: the front door has 607 entities and 542 systems, the
busy duel 1,041 entities; the main schedules take 0.5 ms (front door) and
1.2 ms (busy duel). Allocations are 1,650 a frame (340 KB) at the front door
and 4,100 (1.9 MB) at the busy duel, mostly bevy's own extraction; at about
50 ns each that is under 1 % of a core, and it was left alone.

## Where the GPU time went (before)

| Drawing | Front door | Table | Taken away, the picture changes |
|---|---|---|---|
| `vista.wgsl` (the painted world) | 28 of 30 ms | — | everything (it is the backdrop) |
| `felt.wgsl` (mineral cloth) | — | 8.6 of 14.9 ms | the table |
| MSAA 4× → none | — | −0.2 ms (noise) | RMSE 8.6, 6.8 % of pixels > 8 levels |
| sky, cards, card UI, floor light, mats | — | each below noise | — |

## What changed, and what it costs in the picture

| Change | Saved | Look | Pair |
|---|---|---|---|
| vista: terms masked to zero are not computed (clouds above the sky line, lighting only in cloud, mist in its bell, water terms on water, cascades in the falls, fireflies only outside the form and in live cells) | 30.0 → 10.9 ms | RMSE 0.01, max 2 levels: the same | `images/perf-2026-10-06/v-skips.jpg` |
| noise: `hash_cell`, the lattice case of `hash2`, in every value noise | (in the rows above) | bit for bit the same | — |
| felt: rivers and lava drawn only on their seams | 14.9 → 10.1 ms | exact (the mixes keep `colour` where they are zero) | — |
| felt: every vein cell's point from a table built once per cut (`client-core::feltveins`, the shader's own hash and noise) | 10.0 → 5.6 ms | RMSE 0.31, 0.5 % of pixels > 2 levels, none > 8 | `images/perf-2026-10-06/f-lut.jpg` |
| ambient effects Medium (default on an M1): three cloud layers instead of six, one plane of fireflies instead of two — what phones always drew | GPU busy 27 → 23 % at 30 frames | RMSE 1.7, 1.3 % of pixels > 8 levels | `images/perf-2026-10-06/v-medium-high.jpg` |
| frame pacing (`quality.rs`): the table capped at the preset's limit, a menu at 30 after 2 s untouched, resting at 1 frame after 30 s, background windows at the background limit | front door 60 → 30 → 1 frames; see the tables above | a resting menu's world holds still until touched | — |
| main-world schedules on one thread | busy duel CPU 66 → 56 %, 1.9 → 1.3 ms; front door 22 → 18 % | — | — |
| the score renders in blocks of 256 frames (bit-identical, tested) | one second of music 10.9 → 7.4 ms (criterion) | the same samples | — |
| a view object writes only fields that are not their default (VIEW_VERSION 54) | busy four-seat view 67.4 → 35.6 KB, encode 67.8 → 41.6 µs, decode 122.4 → 80.4 µs | — | — |

For reference, what the two costliest ambient pieces show when taken away
entirely (paused, same frame): the vista's clouds cost 6 ms at full detail and
change 12.7 % of the pixels by more than 8 levels
(`images/perf-2026-10-06/v-clouds.jpg`) — kept; its fireflies cost 2.4 ms and
change 0.5 % (`images/perf-2026-10-06/v-flies.jpg`) — the second plane is
High only. Nothing was removed outright.

## The knobs (`client-core::graphics`, `audiomix`)

Per device, in `ClientSettings`; the settings screen has them as rows of the
existing widgets (`quality::controls`), and the screen's redesign binds to
`Knob`, `turn` and `reading`. A device that never chose gets a preset from
its GPU's name, quiet side first (M1 of any size → Medium; a later Max/Ultra
or a discrete card → High; a software rasteriser → Low).

| Knob | Values | Measured cost |
|---|---|---|
| Preset | Low, Medium, High, Ultra, Custom | the rows below together |
| Frame limit | 30, 60, 120, unlimited | the biggest: every frame pays every shader. Front door with its backdrop hidden at 118 frames: CPU 78 %, energy 74; with the backdrop at the 60 cap: CPU 36 %, energy 35 (not one clean A/B) |
| Background limit | 5, 15, 30, 60 | front door at 15 frames: CPU 15 %, energy 14 |
| Ambient effects | Low (still), Medium, High | Medium vs High above; Low not measured separately |
| Edge smoothing | off, FXAA, MSAA 2×, 4× | MSAA 4× vs none: 0.2 ms, within noise (Apple's tile memory resolves it); FXAA not measured |
| VSync | on, adaptive, off | not measured |
| Overall / game sounds volume, silent in background | 0–100 %, on/off | — |

Deliberately not built: **render scale** (the 3D table would have to be drawn
into a texture and the pointer mapped back into it for picking; a scale that
only blurred the interface would be the wrong half), **texture quality** (card
art sizes were not examined), **SMAA** (needs the `smaa_luts` feature and its
size, and MSAA is nearly free here), **shadows or lighting** (the table has no
lights at all). Reduced motion stays the account's.

## Not measured, or left alone

- The browser build's size and frame time, and a four-seat table: not
  measured this round.
- The audio thread was 3.8 % of a core at the resting front door before the
  block renderer; not re-measured in the app after it.
- The house AI (`baylee-ai`) never showed in a client profile; only its file
  was split.
- The native seat socket uses tungstenite 0.24, which reads into one fixed
  4 KiB chunk allocated once: the per-read zeroing of a 128 KiB buffer that
  the gateway's newer tungstenite had does not happen here.

## Known risks

- The focused frame cap does not wake on window events (else every pointer
  move would raise the frame rate past it). bevy_winit then redraws a macOS
  live resize only when the cap's timer fires; this was not tried live.
- VIEW_VERSION 54: gateway, engine and clients ship as one build, as always.
