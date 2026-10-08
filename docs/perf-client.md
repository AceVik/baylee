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
| Lobby, settled | after | 30 | — | — | 17 % | 16 | 450 MB |
| Lobby, resting | after | 1 | — | — | 4 % | 4 | 367 MB |
| Four seats (two busy boards, 1,244 entities) | after | 53–60 (cap) | 6.1 | 42 % | 57 % | 54 | 555 MB |

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
| Frame rate at rest ("Bildrate in Ruhe", 08.10.) | 30, 60; Low/Medium 30, High/Ultra 60; at or above the frame limit the table does not ease | six seats at rest 60 → 30: CPU 57 → 34 %, energy 54 → 33 (`docs/perf-baseline.md`) |
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

- The browser build's size and frame time: not measured this round; nor a
  four-seat table on the old build (only the new one above).
- The audio thread was 3.8 % of a core at the resting front door before the
  block renderer; not re-measured in the app after it (the second round below
  measured it at a table).
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

## The second round (8 October 2026)

Numbers: `docs/perf-baseline.md` §"The client at rest, second round"; raw rows
and every A/B: `.claude/ux-table/mocks/real/perf2-measures.md`. What the
method gained:

- **Energy beside CPU.** `top -l 2 -s 8 -pid <pid> -stats pid,power`, the
  second sample, over the same window as the CPU reading. GPU ms per second
  stays in the table but reads longer on a GPU that has clocked down, so a
  rate change is judged on CPU and energy, and a shader change on GPU ms/s at
  one fixed rate in one binary.
- **Where allocations come from.** `/allocs {"every":N}` keeps every N-th
  allocation's stack on each thread and `/allocs {}` ranks the sites; build
  with `RUSTFLAGS="-Cforce-unwind-tables=yes -Cforce-frame-pointers=yes"`
  (a separate `CARGO_TARGET_DIR`), or the release profile's stacks stop after
  a few frames. `/perf` reports `main_allocs_per_frame` beside the process's.
- **What marks a resource changed every frame.** A build with
  `--features bevy/track_location` and a system in `Last` printing
  `changed_by()` for the duel, the settings, the texts and every changed
  `Node`/`Text`/`UiTransform`/`Transform`; only the last writer is recorded,
  so fix it and run again until nothing moves at rest.
- **Interleaved rounds.** Before and after alternate on fresh tables, twice,
  so neither has the quiet half of a shared machine.

The audio thread at a table with the music muted was 11 % of the process's
samples before the orchestra learnt to pause (1.5 % after). The browser build
is measured in the baseline (62.9 MB, 15.6 MB brotli); its frame time is not.

## Music v2 (8 October 2026)

The score was rewritten (`docs/client.md` §"One orchestra follows the
player"): 99 recordings at 44,100 Hz instead of 30 at 22,050, looped
sustains, a synthesised lute, more voices at the climax. What it costs, on
the M1 Max:

| Where | Before (`47ea193d4`) | After (`c41/music-v2`) |
|---|---|---|
| criterion, one second of music, stream (the audio thread's path) | 8.0 ms (sanctuary) | front door 4.5–6.6, lobby 8.2, calm table 8.7, tension 13.5, climax 11.5 ms (after the owner's livelier pass; 6.9, 10.3 and 9.7 before it) |
| the app at the resting front door (30 s untouched, unmuted, a fresh settings file): `Tune::next` in `sample <pid> 10` | 3.8 / 3.7 % of a core | 0.6 / 0.8 % of a core |
| the same, process CPU (`top`, 10 s) | 4.7 / 4.7 % | 1.6 / 1.6 % |

Two interleaved passes each (before, after, before, after; the app measured before the livelier pass), release
builds with `dev-control`, other sessions' gates running on the machine.
The old score cost more in the app than its bench (3.8 % against 0.8 %), the
new one the same as its bench; why the old differed was not chased. A table
at tension or climax was benched, not sampled in the app. The bank is
15.6 MB in every build (8.9 MB before); the browser build, not released now,
carries it too.

Round 2 (9 October 2026: the orchestral body, four themes, de-shrilled)
costs more at the loud end: one second of music as the audio thread pulls
it is lobby 6.5, calm table 10.7, tension 15.9, climax 20.5 ms (criterion,
M1 Max): at most 2 % of one core at the climax, about 1 % at rest. Not
sampled in the app this round. The bank is 19.6 MB (133 recordings).
