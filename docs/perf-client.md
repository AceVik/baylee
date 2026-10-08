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

## Windows (8 October 2026)

The first round on Windows, on the owner's desktop, branch
`c41/windows-opt`.

**Machine.** Ryzen 9 3900X (12 cores), 64 GB, RTX 4070 Ti (driver 617.42,
32.0.16.1742), Windows 11 Pro 26200 in German. Two monitors, both at 150 %:
3840×2160 at 240 Hz (primary; the window maximised is 2560×1369 logical) and
2560×1600 at 120 Hz. With no preference saved the RTX picks **High** (cap
120, rest 60, background 30, MSAA 4×).

**Method.** `release` builds with `dev-control`, started by a PowerShell
script with a scratch `XDG_CONFIG_HOME` (music muted, nothing else set) and
the window in front, untouched unless a row says so. "Before" is
`47ea193d` (main before the icon branch), "after" this branch.

- **Frames** and the frame-time spread from `/perf`.
- **CPU** is the process's CPU time over wall time (all threads; 100 % is
  one core).
- **GPU ms/s** is the process's `\GPU Engine(pid_*)\Running Time` summed
  over its engines, per wall second. It is busy time, not energy: a GPU that
  clocks down at a low rate takes longer per frame, so this column *rises*
  from 30 to 15 frames on the same screen. Compare power instead.
- **Board power** is `nvidia-smi --query-gpu=power.draw` every 250 ms over
  the same interval, given as the rise over the desktop with no Baylee
  running, sampled just before (20.3 W, other programs as they were).
- **RAM** is private bytes, **VRAM** `\GPU Process Memory\Dedicated Usage`.
- A **busy** row moves the pointer every 200 ms; a duel's row is an
  offline game against the house (Play offline → Play the house, keep).
- Startup and the first frame: bevy's `trace_chrome` on a `ci-release`
  build, and full-screen captures every few hundred milliseconds.

### What was found, and what changed

| | Before | After |
|---|---|---|
| Startup: dev harness answers | 4.8–5.1 s (3 launches) | **1.1 s** |
| Startup: what the player sees | a white 1280×720 window from 0.9 s, black and maximised at 4.7 s, the front door at 5.3 s | nothing, then the front door maximised at about 1.5–1.9 s |
| The first frame | 3.7 s, of which `sound::voice_the_cues` 3,707 ms | the cues are synthesised on the compute pool |
| Minimised | 30 frames, 21 % CPU (background limit) | **1** frame, 0–3 % CPU, +0.7 W |
| Menu untouched 30 s, High | 30 frames, +13.6 W, 20–22 % CPU | **15** frames, +5.3–5.5 W, 8–10 % CPU |
| Console window beside the game (release) | yes (a console program) | none; `--console` opens one, the log in it |

The "after" column is the branch's final build, two launches each; its
baseline that hour was 21.1 W.

- **The first frame.** `voice_the_cues` rendered all 37 cue buffers in
  `Startup`, on the main thread. Bevy creates the window visible at its
  default size and maximises it on the first frame, so those seconds stood on
  screen as a white box. The buffers are now one task each on the compute
  pool (`sound::collect_the_voices` fills `Voices`), and the desktop window
  is created hidden and shown on its third frame
  (`standalone::show_the_window`). What is left of the first second:
  `RenderPlugin`'s build 0.54 s, the Vulkan surface 0.30 s and the mark
  atlas's bake 0.27 s on the main thread when the font arrives.
- **The console.** A release runtime was a console program, so starting it
  directly opened a console window beside the game (the launcher hid it with
  `CREATE_NO_WINDOW`). It is a windows-subsystem program now, a debug build
  still a console one; `--console` (passed on by the launcher) attaches to
  the parent's console or opens one and points the standard handles at it
  (`baylee_client::console`).
- **Minimised.** winit sends no `Occluded` on Windows (only macOS, iOS, the
  web and Wayland do), so a minimised client was paced as one behind other
  windows. Windows resizes a minimised window to 0×0 (`/health` reads width
  and height 0), and `quality::minimised` reads that as hidden.
- **An idle menu at High** kept its world moving at the background limit.
  `graphics::IDLE_MOVING_FPS` holds it to 15. The same front door untouched
  for 35 s, two interleaved passes:

  | Background pace | Frames | CPU | GPU ms/s (busy) | Board power over idle | GPU clock |
  |---|---|---|---|---|---|
  | 30 (High before) | 30 | 20–22 % | 199–202 | +13.6 W | 1,670–1,720 MHz |
  | 15 (High now) | 15 | 11–15 % | 242–249 | +6.5 W | 427–436 MHz |
  | 5 | 5 | 3–6 % | 145 | +2.2 W | 218–221 MHz |

  The motion at 15 frames was not judged by eye. Low and Medium (an M1, the
  owner's 7840U laptop) already rest at one frame a second and are unchanged.
- **A game's record was not locked on Windows** (not a frame cost, found by
  running the tests here). `records::LiveRecord` opened its file append-only,
  Windows locks only through a handle with read or write-data access, and
  `try_lock` failed in silence, so a second client's `recover` could cut a
  running game's record back. The file is opened for reading as well.
- **The tests on a Windows host.** CI runs them on Linux only. On Windows,
  besides the record, the updater's macOS-bundle fixtures need symlinks
  (skipped there now, which also lets `end_to_end` reach its Windows case),
  two tests split source on `\n` against a `\r\n` checkout, and `userdirs`
  judged a Linux path's absoluteness by the host's rules. All of
  `baylee-client`, `-client-core` and `-update` pass on Windows now.

### Measured and left alone

- **Backend.** Bevy allows every backend and wgpu prefers Vulkan over DX12
  for the same adapter, falling back to DX12 where Vulkan is missing. Forcing
  DX12 (`WGPU_BACKEND=dx12`) against the default, two interleaved passes:

  | | Vulkan | DX12 |
  |---|---|---|
  | Front door at 30 frames: CPU, GPU ms/s | 18–21 %, 258–276 | 19–26 %, 225–235 |
  | Duel busy (≈118 frames): CPU | 93–105 % | 125–147 % |
  | Duel busy: frame time p95 | 9.0–9.3 ms | 14.8–15.3 ms |
  | Duel at rest (60): CPU | 54–56 % | 65 % |
  | Duel: GPU ms/s | 302–313 / 328–330 | 302–305 / 334–336 |
  | Duel: private RAM / VRAM | 1,216 / 906 MB | 1,100–1,146 / 786 MB |

  DX12 costs more CPU at the table and paces worse; the default stays.
  `WGPU_BACKEND` still chooses (the launcher passes the environment on).
- **Frame pacing.** winit 0.30 waits on a high-resolution waitable timer on
  Windows, and the caps hold: 30 frames at p50 33.3 ms, p99 34.1–34.7 ms;
  60 at p50 16.6, p99 17.6–18.0; 120 at p50 8.4 ms.
- **The table's CPU.** The main schedules take 1.15 ms a frame (7 % of a core
  at 60 frames); the rest of a duel's 54 % is rendering and the driver, as
  on the Mac (57 % at 60 frames, §"The client at rest, second round").
- **Memory.** Private bytes 730–860 MB at the front door and about 1.2 GB at
  a duel (working set 340–600 MB); dedicated VRAM 530–640 MB and 790–910 MB.
  Not looked into further this round.
- **"FPS N/A · GPU · CPU · LAT N/A"** in the top right of a screenshot is
  NVIDIA's statistics overlay, not Baylee's: its frame counter does not
  hook this program.
- **"Winit Thread Event Target"**, a 22×22 window at 0,0 that `EnumWindows`
  lists as visible, is winit's message window: `WS_EX_TOOLWINDOW |
  NOACTIVATE | LAYERED | TRANSPARENT`, no taskbar button, no input.
- **AltGr** on the German layout: `@ € { [ ] } \ ~ |` type into a field
  through real key events, and none fires a shortcut (winit drops the left
  Ctrl Windows fakes for AltGr).
- **DPI.** Both monitors are at 150 % here, so a move between different
  scales was not tried; moving the window between the two relays it out
  (2560 → 1707 logical wide) and back.
