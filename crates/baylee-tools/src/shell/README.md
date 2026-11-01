# Native shell acceptance tools

All former `scripts/shell/*.py` entry points are implemented in this module.
Run `cargo run --profile tools -p baylee-tools -- shell <command> ...` (or the built binary).
The caller must supply an **already running, isolated dev-control client**.
These tools do not build or launch a client, gateway, server or model.

## Command mapping

| Former script | Native command after `shell` |
| --- | --- |
| `check.py` | `check gallery OUTDIR` / `check centred` |
| `devctl.py` (shared library) | shared `devctl.rs`; CLI `devctl /ROUTE [--body-file FILE_OR_-]` |
| `front.py` | `front OUTDIR "Saved local gateway name"` |
| `header.py` | `header OUTDIR` |
| `keys.py` | `keys OUTDIR` |
| `lane.py login` | `lane login …`: refused since the cutover (no gateway signs anybody in with a password); its rewrite to Central's sign-in is [#406](https://github.com/AceVik/baylee/issues/406) |
| `look.py` | `look OUTDIR play,decks 960x700,844x390 [--touch] [--lang de]` |
| `measure.py` | `measure hover` / `measure presses` / `measure quit PID --session-env VARIABLE [--gateway URL]` |
| `probe.py` | `probe SCREEN 844x390 de touch xl [SHOT.png]` |
| `report.py` | `report walk` / `report table` / `report dump OUTDIR` |
| `screens.py` | `screens OUTDIR [lobby\|room\|all]` |
| `settings.py` | `settings OUTDIR` |
| `walks.py` | `walks [w2 w3 w6 w7 ...]` (default `w6 w7 w3`) |

`--port N` is global. Without it, `BAYLEE_DEVCTL_PORT` or 28770 is used.
`front` and `settings` retain `SHELL_SIZES=960x700,844x390` and
`SHELL_STEPS=xs,m,xl` overrides. Other acceptance matrices retain their fixed
sizes/steps. `screens all` retains the former alias for the lobby matrix;
run `screens room` separately after seating the client.

For generic devctl POSTs, put the JSON body in a file or pipe it to
`devctl /key --body-file -`. Omitting `--body-file` sends GET. This covers
`/state`, `/health`, `/pointer`, `/text`, `/key`, `/shell`, `/window`, `/hide`,
`/pause`, `/step` and `/screenshot` without another process or language.
All answers must be successful HTTP and valid JSON without `error`/`ok:false`.
There is a 60-second request timeout, 120 seconds for screenshots. Raw output
redacts password/token/secret/API-key fields recursively; error bodies are
never printed. Requests do not follow redirects or inherit HTTP proxies.

## Fixtures and side effects

- Use a private client config/cache and an OS network sandbox restricting the
  **client as well as this driver** to loopback. The driver contacts only a
  literal loopback IP (localhost is normalized to 127.0.0.1), but dev-control
  does not expose enough selected-gateway state to prove the client's own
  network destinations. This is a manual preparation requirement, not a
  claim that the HTTP driver sandboxes the client.
- Use `BAYLEE_DEV_SCALE=1` for ring and art-credit pixel checks; these checks
  explicitly reject a different scale. Black screenshots fail measurement.
- `front` needs a saved local gateway. `lane login` refuses: it typed a
  password into the client's gateway sign-in, which no gateway has since the
  cutover; sign a lane's client in through Baylee Central or as a guest by
  hand until #406 brings it back.
- `header`, `screens lobby`, `look`, `probe`, `measure hover` and the default
  walks expect the lane signed in. Walks require multiple decks and an open
  table named "Thursday …". `screens room` requires an already seated client.
- Complete modal prerequisites before a sweep: accept the test gateway's
  terms and acknowledge the first-device E2E notice (`GotIt`) when offered.
  Those sheets intentionally block the header; the helpers do not dismiss
  consent or safety notices automatically. The settings sweep supports the
  signed-in desktop's `Header(Nav(3))` and the front door's
  `Settings(OpenSettings)` entry, or an already open settings screen.
- Front navigation and settings cleanup scroll a clipped control into its measured scroll
  viewport before pressing it, with at most 12 wheel requests and fresh
  geometry reads. This does not relax any layout/contrast assertion.
- `keys` uses the source tree's named `GALLERY_ORDER`. W2 ends in a room, as
  the old script did; `measure presses` separately proves Start reaches an
  actual game view. W7 deletes one deck and undoes it.
- `report walk`/`dump` need no login. `report table` starts a house game through
  the existing offline controls only if no game view is present. It checks all
  a–z suggestion prefixes against the seat's own/public view and checks exact
  reference spans, objects, zones and player references.
- `settings` opens and discards an unsaved model profile; it invokes no model.
- `measure quit` is macOS-only and closes the **supplied client's actual window**
  through System Events. It duplicates/deletes a deck, verifies the pending
  Undo still leaves that copy on the local gateway, then checks process exit
  and exact original deck identity after the flush. It never kills the client.
  Only its owned osascript helper is terminated/reaped on timeout/error.
  The gateway defaults to `BAYLEE_GATEWAY_URL` or `http://127.0.0.1:28931`.
  It reads the decks with the client's own session token, since a gateway
  signs nobody in with a password any more (`docs/central-login.md`):
  `--session-env NAME`, `--session-file FILE`, `--session-stdin`, or
  `BAYLEE_SHELL_SESSION`.
- Session tokens are never positional arguments.
  Request bodies, bearer tokens and returned error bodies are not logged.

Clicks reread semantic controls and positions, wait for stable geometry, send a
separate prehover request, reread geometry, then press. Retried transitions are
bounded and stop if the screen changes without the intended postcondition.
Keyboard-only walks have no pointer calls. Chords release their modifier even
when the main key fails. The old 24/32 backspaces, 20 name-edit backspaces,
physical key/character distinction, frame-step calls and settling delays remain.
All layout assertions retain their tolerances; a navigation failure or timeout
now fails rather than silently accepting a different screen. `probe` and the
three `measure` scenarios return failures for violated assertions rather than
merely printing suspicious results.

## Screenshot budgets and verification

Default image counts are unchanged: gallery 14, front 45, header 27,
settings at most 104 (90 section shots + 14 profile sheets), keys 6,
screens lobby 88 / room 22, report dump 4. Quick looks/probes take only requested
shots. All faults are preserved in scenario logs, with bounded terminal output.

`tests.rs` has pure geometry, Unicode-budget, ancestry/clipping, contrast,
credit/ring, phone geometry, report privacy, CLI/matrix and bounded-wait tests,
plus local TCP fixtures proving refusal/redirect handling, modifier release on
failure and fresh-position prehover/click ordering. It launches no application.
The client role test compiles the tool's actual `role_contract.rs` and compares
all five roles against both kit budgets and frozen English/German values.

Parent validation commands (not run by this porting worker):

```sh
cargo test -p baylee-tools shell::
cargo clippy -p baylee-tools --all-targets -- -D warnings
cargo test -p baylee-client native_shell_role_budgets_match_the_kit_and_frozen_contract
```

Runtime visual acceptance still requires the prepared private client and the
relevant commands above. Static format checks do not establish a live render pass.

`parity-fixtures.json` freezes 12 concrete cases evaluated against the original
`check.py` (SHA-256 included): all ten layout checks and every node's label. The
native test checks those results without Python. The reference-generation pass
performed no driver/network calls; the native tests still require the parent's
Cargo run.
