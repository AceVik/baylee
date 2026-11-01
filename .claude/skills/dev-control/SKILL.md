---
name: dev-control
description: Drive, photograph and record the Bevy desktop client through its loopback HTTP harness; inspect real input, rendered frames, animations and flicker without OS input.
---

# Driving and recording the client

Read `docs/client.md` §"Driving the client without its window" for the full
protocol and §"Recording real render frames" for capture limits. The harness
is in `crates/baylee-client/src/devctl.rs`, compiled only with `dev-control`.
The native Rust helper is `cargo tools …` (the alias for
`cargo run --locked --profile tools -p baylee-tools -- …`); Python and Pillow are not needed. The PyTorch
trainer under `tools/trainer` is a separate, intentionally Python workflow.

## Start an isolated, silent client

Every agent-started test client must be closed after the test, including on
failure or interruption (owner, 19.10.2026; again 29.10.2026: after **each**
test run, not left open between iterations). Record its exact PID/session at
launch and use finally/trap cleanup; verify the process exited and its harness
port stopped listening before finishing. Close helper services started for
the same test too. Never stop unrelated processes or leave cleanup to the
owner: passive windows cannot normally be closed by clicking. Leave a client
running only at the owner's explicit request.

Never test against the owner's account, production gateway, feedback or crash
reporting. Set separate `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` and
`XDG_DATA_HOME` directories for each client; keep the real HOME. Bind only
loopback, use a free port, and set `BAYLEE_DEV_PASSIVE=1` so OS input passes
through. A dev-control run is silent by default; `BAYLEE_DEV_SOUND=1` is an
explicit opt-in, not appropriate for normal agent tests.

Choose a dedicated screen with `BAYLEE_DEV_MONITOR=1` (zero-based native
index) or `BAYLEE_DEV_MONITOR=primary` before launch. `GET /monitors` lists
indices, names, physical desktop rectangles and scale factors; these indices
are not Windows Settings display numbers and can change after reconnecting a
screen. An unavailable index closes the harness client before it is shown.
Keep `BAYLEE_DEV_PASSIVE=1`: screen selection does not itself prevent focus.
On the owner's Windows workstation, use the additional screen **left of the
primary display** (owner preference, 19.10.2026). On that date `/monitors`
reported index **0**, `\\.\DISPLAY2`, at physical `(-2560, 864)`, 2560 × 1600.
Use `BAYLEE_DEV_MONITOR=0` there; reconfirm after display topology changes.
Do not substitute the primary screen if the left screen is absent.
On the owner's MacBook, use the **built-in display** (owner, 22.10.2026: he
works on the external screens). The main display there is external, so
`primary` is wrong, and the built-in display's index changes with the screens
attached (it was 1 with two external screens, 0 with one, the same day).
`/monitors` names every screen `Monitor #…`, and its pixel size follows the
owner's resolution setting (3456×2234 and 4112×2658 were both seen), so find
it by asking macOS: `CGGetActiveDisplayList` with `CGDisplayIsBuiltin` (a short
Swift script, no client needed) gives its index and bounds, which match one
`/monitors` rectangle. Confirm right after launch that the window is on it,
and close the client at once if it opened anywhere else.

```bash
XDG_CONFIG_HOME=/tmp/baylee-qa/config XDG_CACHE_HOME=/tmp/baylee-qa/cache \
XDG_DATA_HOME=/tmp/baylee-qa/data BAYLEE_DEV_PASSIVE=1 \
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
```

An empty configuration opens the lobby. "Play offline", then "Play the
house" reaches `LocalHost`, an in-process duel requiring no gateway or
account. `/state.view` is null before the duel. A networked test uses only an
explicitly authorised staging gateway and isolated test accounts. Keep
secrets out of command transcripts, dumps and screenshots.

A networked test always sets `BAYLEE_GATEWAY` to the QA gateway: without it
the client takes the production gateway. The front door also lists the
built-in production gateway (Baylee Sanctuary) beside the saved ones, and the
rows' order can change between runs, so never pick a row by index alone:
strip every non-loopback address from the profile's `gateways`, choose the
row that has a bin (`Front(ForgetGateway(k))`; the built-in row has none),
and type nothing until the gateway's own username and password fields are on
screen and no Central control is. If the QA gateway is not answering, the
test stops instead of signing in anywhere (29.10.2026: a run picked the
production row by index after a QA restart and tried throwaway test
credentials there).

Build once under the shared Cargo lock, then run the binary for repeated
checks. If worktrees share a target directory, copy the finished client and
helper into a directory for this run; later builds can replace the shared
executables. Pass the matching checkout as the helper's `--root`.
Record its commit, features and settings. Wait for `/health` with a
bounded deadline and inspect stderr if it fails; never loop forever. The
passive macOS window stands behind the owner's windows and comes above them
only while a screenshot or recording captures (`/health` says `"on_top"`),
without focusing or taking OS pointer input. A passive client closes itself
after `BAYLEE_DEV_IDLE_EXIT` seconds without a request (600 by default; set a
longer one only for a long recording run, never `0` for an agent run); that
is the backstop, not the way to close it. A black screenshot is not proof
of a shader defect: inspect the actual surface and window state first.

## Protocol and coordinates

```text
GET  /health                    frame, logical width/height, scale, clock
GET  /monitors                  native display indices and physical geometry
GET  /state                     live view, interaction, controls and geometry
POST /window       {"width":1280,"height":800}
POST /key          {"name":"Space","shift":false,"hold":false,"release":false}
POST /text         {"text":"a search"}
POST /pointer      {"x":100,"y":200,"button":"left","press":true}
POST /pointer      {"x":100,"y":200,"hold":true}
POST /pointer      {"release":true}
POST /pointer/native {}         return to physical mouse input after release
POST /scroll       {"y":-3}
POST /screenshot   {"path":"/tmp/table.png"}
POST /timescale    {"speed":0.05}
POST /pause        {"paused":true}
POST /step         {"frames":6}
POST /recording/start {"duration_ms":3000,"fps":15}
GET  /recording/status
POST /recording/stop {"cancel":true}
```

Always read the JSON response, including `error`: older routes may report a
refusal with HTTP 200. Recording routes also enforce method, request size,
loopback Host, and reject browser Origin/fetch headers. Do not send browser
credentials or expose this socket remotely.

Below the desktop layout floor (1280 × 720), the UI shrinks uniformly.
`/state.presentation.ui_scale` reports that fit (and the table’s large-screen
scale). Control rectangles are already converted to window coordinates: do
not apply the UI scale to `/pointer` coordinates a second time.

Coordinates are **logical**; PNG dimensions are physical. Read `/health.scale`
and divide measured image coordinates accordingly. Use `/window`, then wait
for the reported size (the window manager can round by a pixel); resizing
through OS scripting is unnecessary. Find controls in `/state.lobby_controls`,
`desk_controls` or `buttons` and read `at_x`/`at_y`, not an old screenshot.

A pointer click already spans move, repeated aim, press, release and settle.
Its reply follows the consuming frame. Do not work around the old missing
move/click bug with unbounded resend loops. If a requested target differs,
re-read its live rectangle and the returned error before trying again. A
bare move also waits for aim/settle. The wheel lands at the current pointer:
aim at the intended scroll area first. A hold and a release are separate
requests, suitable for drags and Shift face previews.

`/pointer` uses a client-only virtual cursor in both passive and interactive
runs. It never moves the desktop cursor, including when `/window` resizes the
client. The position persists between requests. Use `/pointer/native` after
all requests settle and held buttons are released to return to physical mouse
movement (move the real mouse once). Passive mode still passes physical input
through the window; returning to native input does not disable passive mode.
Do not substitute OS mouse automation: it moves the owner's cursor.

Keys are physical names (`KeyK`, `Digit1`, `ArrowLeft`, `Space`, `Enter`);
letters/digits also have aliases. Read the result rather than assuming a key
exists. WASD moves the card cursor. Enter acts on that card; Space confirms a
choice. An ability chooser owns its own keys: A/D choose, E/Enter accepts,
Escape closes. For targets, E toggles and Space submits; min=0 with no
selection is a valid empty choice. Combat uses `interaction.assignments`
and `focus`, not `selected`. An irreversible action may first appear in
`armed`; the second action commits it. Inspect `outbox`, `mana_run`,
`ability_menu`, `interaction.pending` and `last_error` before diagnosing a
key as dead. `hovered` is a Debug string such as `"ObjectId(52#0)"`, not a
nested JSON object.

`/key` and `/text` bypass the native OS keyboard path. A Windows system-sound
report needs native/physical input plus output-device loopback audio capture;
validate the capture with an announced control tone. See `docs/client.md`
for the Windows comparison. For tour checks, leave `BAYLEE_NO_TOURS` unset:
`/state.tour` reports `scrims`, `bubbles` (layout and visibility), and each
control's `visible` flag. A run's mode alone does not prove its bubble is
visible or that its scrim was removed after Skip.

## Animation and flicker evidence

```bash
cargo run --profile tools -p baylee-tools -- recording record --duration-ms 3000 --fps 15 --format all --diff
```

For an interaction, start via `/recording/start` (optionally with `"video": true` so the client worker writes `recording.mp4` and `recording-step.mp4` directly), perform input through the harness, poll status to completion, then encode the returned directory:

```bash
cargo run --profile tools -p baylee-tools -- recording encode /tmp/baylee-recording-UUID --format all --diff
```

The helper writes MP4 (`preview-CADENCE.mp4` and 1 FPS `preview-step.mp4` so 1 Hz video viewers inspect every captured frame), GIF/WebP previews, a contact sheet and pixel-difference
CSV, preserving the original PNGs, `frames.jsonl` and `recording.json`.
MP4/GIF/WebP are previews, not authoritative pixel evidence. Inspect selected raw
frames/contact sheets with an image viewer and correlate timestamps with the
input. Do not claim to perceive the full animation from a single image.
Adjacent differences are a heuristic; text carets, hover changes and normal
animation produce differences too. Check a paused, identical-frame control,
reproduce the suspected change, then restore it. A missing intermediate frame
or a dropped sample cannot establish that flicker did not happen.

Sampling is bounded: at most 30 seconds, 60 FPS, 900 requested samples, two
outstanding readbacks/writes, 32 MiB per raw frame and 2 GiB raw in total.
Status and ledger report dropped samples, request/receive times, virtual time,
speed and pause state. The request time is not a presentation timestamp and
the samples are not every displayed frame. Normal screenshots can occupy the
same target. Never hide drops or silently interpolate evidence. An
incomplete recording is refused unless recovery is explicitly requested;
keep its incomplete status visible.

`/timescale` slows `Time<Virtual>`; zero is refused (use `/pause`). `/step`
counts rendered frames but its first delta can hit Bevy's 0.25 s clamp, so it
can jump over a short transition. Slow a short transition instead. A paused
real-time recording is useful for checking stable pixels and recorder cadence.

## Live shader editing and render claims

```bash
BAYLEE_DEV_CONTROL=28770 BAYLEE_DEV_PASSIVE=1 BEVY_ASSET_ROOT=$PWD \
    cargo run -p baylee-client --features dev-control,dev-reload
```

Also keep the isolated XDG configuration above. Edit the current `.wesl`
source, for example `crates/baylee-client/src/shaders/felt.wesl`.
`BEVY_ASSET_ROOT` must be the workspace root; startup refuses the wrong setup.
Pause, photograph twice (identical), change a known colour, photograph
(different), restore it, photograph (identical). Compare with a shader-free
clear-colour reference for black/incorrect surfaces. Bound pixel assertions
on both sides and inspect the actual render. `/state.shelves` explains
projected ledge geometry; compare the model's coordinates with measured ink.
Use `/perf` and the process/frame measurements in `docs/perf-client.md` for
performance claims; screenshot differences do not measure rendering cost.

The harness supplements model tests; logic assertable in client-core belongs
there. It never impersonates an opponent. Use the engine-server's seat-scoped
harness for an explicitly authorised scripted opponent.
