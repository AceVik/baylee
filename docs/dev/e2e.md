# The client's end-to-end tests

`crates/baylee-client-e2e` tests the desktop client the way a player uses
it: a real `baylee-client` built with `dev-control`, one per test, driven
through its loopback harness (pointer, wheel, keys, typed text, window size)
and read back through `/state`. A test checks that input does what it should
and that the screen shows what it should, at the window sizes and text steps
a player has. The harness itself is `docs/client.md` §"Driving the client
without its window"; this document is about the tests.

```bash
cargo dev e2e                          # build the client once, run every test, one client at a time
cargo dev e2e house_game               # only tests whose names contain a filter
cargo dev e2e --no-build --jobs 2      # the client the last build left, two clients at once
cargo dev e2e --monitor 1              # another screen (default on macOS: the built-in display)
cargo test -p baylee-client-e2e        # the framework's own unit tests; no window (the gates run these)
```

The tests open windows, so they are `#[ignore]`d: the gates compile them and
run only the framework's unit tests. `cargo dev e2e` is local, like the
development features (`docs/dev/commands.md`); CI does not run it. Run it
after a change to anything a player sees or presses.

## Writing a test

```rust
use baylee_client_e2e::prelude::*;

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_key_on_the_keep_button_keeps_the_hand() {
    e2e::run(|client| {
        client.play_the_house()?;
        client.press("K")?;
        let kept = client.wait_until("the hand kept", |s| s.control("Keep").is_none())?;
        kept.expect_not_shown("Keep this hand?")
    });
}
```

- **`e2e::run(|client| …)`** starts a client with the defaults, runs the
  test and closes the client whatever happened; `e2e::run_with(Launch, …)`
  starts it otherwise: `Launch::new().viewport(Viewport::PHONE)`,
  `.lang("de")`, `.text_step(TextStep::Xl)`, `.tours(true)`,
  `.setting("key", json)` (written into `client-settings.json` before the
  start), `.env(…)`, `.arg(…)` (an argument, as a launcher passes a
  `baylee:` link on a cold start).
- **Acting**: `click`, `right_click`, `hover`, `drag`, `scroll` (lines
  down, negative up, as the harness's `/scroll` counts them), `press`
  (`"K"`, `Key::named("KeyP").shift()`), `type_text`, `resize`, and the
  clock: `pause`, `timescale`, `step`. A click aims at a **target** where it
  is drawn now:
  - `press("Front(PlayOffline)")`: a control by its `Press` spelling, from
    `lobby_controls`, `desk_controls`, `update_buttons`, the chat's
    controls, and at the table `buttons` (`press("Keep")`,
    `press("SortHand(Mana)")`); `press_prefix("Front(SelectGateway(")`.
  - `label("Play offline")`: a pressable node by the words on it.
  - `text("…")`: a visible text node; `in_hand("Forest")`, `card("…")`: a
    card the table draws; `at(x, y)`: a point.

  A target that matches nothing or more than one thing is refused, naming
  what is there instead: a click is never a guess.
- **Reading**: `client.screen()` is one reading; `settled()` waits until two
  readings draw the same lobby tree (`shell_nodes`) and the same controls
  and the front door is still. At the table that means the buttons hold
  still; the HUD's own nodes are not compared, because the seat chips and
  clocks breathe, so a check of a HUD animation waits for what it means;
  `wait_until("what", |screen| …)` polls until a condition holds (20 s,
  `wait_for` with a deadline of its own). A `Screen` answers `shows`,
  `expect_shows`, `expect_not_shown`, `locate`, `control`, `cards_in`,
  `hovered`, `in_duel`, `flag("/account/offline")`, and `at("/json/pointer")`
  for anything else in `/state`.
- **Ways through**: `client.play_offline()`, `client.play_the_house()`
  (`src/flows.rs`). A way a second test needs belongs there.
- **Window sizes**: `client.across(Viewport::ALL, |client, viewport| …)` runs
  a check at every size, resizing one client, and reports every size it
  failed at with a screenshot each. The presets sit on the shell's
  breakpoints: `PHONE` 844 × 390, `COMPACT` 720 × 900, `NARROW` 960 × 700,
  `DESKTOP_FLOOR` 1280 × 720, `LAPTOP` 1440 × 900 (the default), `FULL_HD`,
  `QHD` 2560 × 1440 (Vast), `UHD`; sets `ALL`, `SMALL`, `DESKTOP`; or
  `Viewport::new("name", w, h)`. Sizes are logical pixels at scale 1, so a
  QHD layout fits a Retina screen; a size the screen cannot hold fails
  naming the size it got.
- **Layout**: `screen.expect_layout_ok()` finds what a size breaks (`src/layout.rs`):
  a pressable node cut off by a box that clips without scrolling (the
  lobby's root clips at the window's edge), one out of the window, a table
  button out of the window, a text whose glyphs measure larger than its
  box, two pressable nodes on the same pixels. A row a list has scrolled
  away is not misplaced and is skipped. `Layout::touch()` adds 44-px
  targets, for the phone builds to come: the desktop front door's text
  links are 23 px high by design. `Layout::desktop().may_overflow("…")` and
  `.ignoring("…")` excuse one text or node, by name, in the test that needs
  it.

## What the harness reports for it

- `shell_nodes`: the lobby's tree (and the gallery's and the report
  sheet's); `ui_nodes`: every other UI tree in the same rows (the table's
  HUD, the chat, tours, the updater's card), each root `"r":"ui"`, roots top
  to bottom then left to right. Both are byte-stable for one screen.
- Each row: `d` depth, `x y w h`, `t` text, `tw th ln` what the glyphs
  measure, `hid` hidden (inherited), `i` carries a `Press`, `k` the kit's
  role, `cx`/`cy` how the node clips its children across and down
  (`"scroll"` or `"cut"`; absent: it does not).
- The one driver of the harness is `Devctl` (`src/devctl.rs`), which `cargo dev
  central`, `cargo tools shell` and `cargo tools recording` use too: loopback
  by port, no proxy or redirect, a deadline for the whole reply, and any
  `error` or `"ok": false` a `Refused` that names the harness's reason, which
  can repeat a request field (`cargo tools shell` withholds it).
- At the table only the lobby's nodes say they are pressable; what can be
  pressed there is `buttons` and `cards`.

## Safety, and what a run leaves

Every client gets configuration, cache and data directories of its own
under `target/e2e/<run>/<test>/`, a free loopback port, a passive and silent
window at scale 1 (`BAYLEE_DEV_PASSIVE`, `BAYLEE_DEV_SOUND` empty,
`BAYLEE_DEV_SCALE=1`), no tours, a two-minute idle exit as the backstop, and
dead loopback addresses for the gateway and the updater: without
`BAYLEE_GATEWAY` a client takes the production gateway even when it only
plays offline. The front door still lists the built-in production gateway,
and like every client asks each listed address once at startup what it is
(`GET /info`, anonymous: `lobby::http::ask_about_saved_gateways`), so each
test's client sends that one request to production and nothing else. The
framework refuses any click on a gateway row that has no bin
(`Screen::refuse_a_real_gateway`), so no test signs in there. The card
images come from Scryfall, as in every client.

A test's client is closed when the test ends, also on failure, and its port
proven closed; `cargo dev e2e` then checks that no process of the run's
client is left and kills and reports one that is. A test process that
aborts (not panics) leaves its client to the idle exit.

A failed test leaves `failure.state.json` (the reading, as a recording),
a screenshot, `client.stderr.log` and the client's configuration in its
directory; `across` leaves one of each per size that failed. A recording
reads back with `Screen::from_recording`, which is how the unit tests run
the layout checks on real screens (`tests/fixtures/`), each check shown to
fire on a fault injected into a recording. When `/state` changes shape,
`cargo run -p baylee-client-e2e --example record_fixtures` records them
again from a dev-control build. Recordings hold no secret; keep it so (no
signed-in client's reading goes into a fixture).

One client at a time is the default (`--jobs`, `BAYLEE_E2E_JOBS`): every
client is a window and a GPU context, and a test timed out under load
proves nothing. The window opens on the built-in display on macOS
(`BAYLEE_E2E_MONITOR`; the launch checks `/monitors` agrees) behind the
owner's windows, and comes up only while a screenshot is taken.
