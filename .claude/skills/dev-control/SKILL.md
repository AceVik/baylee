---
name: dev-control
description: Drive and photograph the Bevy client without its window — the loopback HTTP harness in devctl.rs, how to play a duel through it, and the failure modes that look exactly like "the key did nothing". Use when you need to see a client change actually working, take a screenshot, reproduce an interaction bug, or assert on what the client believes rather than on what a test constructed.
---

# Driving the client without its window

`crates/baylee-client/src/devctl.rs` is a loopback HTTP server that presses
keys, moves and clicks the pointer, types text, dumps what the client believes
and saves a screenshot — while the window sits behind everything else on the
desktop. `docs/client.md` §"Driving the client without its window" is
normative; this is how to use it.

Reach for it when a change is only really verified by the running app: the
board drawn, a key reaching the right handler, a panel that appears. A unit
test proves the model; this proves the client.

## Starting it

Two locks before the socket exists — the feature and the variable:

```bash
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
```

That is `LocalHost`: a duel in process against the house AI, from
`data/acceptance-decks.txt`, needing no gateway, no account and no agent. It is
the fastest way to a board. For a *networked* seat add `BAYLEE_GATEWAY`,
`BAYLEE_GAME` and `BAYLEE_SEAT_TOKEN` (a `SeatTicket`, most easily made with
`cargo run -p xtask -- dev-table --seats 2 --ai sharp`).

The binary is slow to link. Build it first, wait for the window, then poll
until it answers rather than assuming:

```bash
until curl -sf localhost:28770/health >/dev/null; do sleep 1; done
curl -s localhost:28770/health
# {"ok":true,"frame":1183,"width":1728,"height":1052,"scale":2}
```

If `/health` never answers: the feature was left off (the routes are not in the
binary), the variable was unset (the socket is never opened), or the app
panicked at startup — check its stderr before blaming the harness.

## The protocol

```text
GET  /health                     → {"ok":true,"frame":…,"width":…,"height":…,"scale":…}
GET  /state                      → the dump below
POST /key        {"name":"Space","shift":false,"hold":false,"release":false}
POST /text       {"text":"dev@baylee.local"}
POST /pointer    {"x":100,"y":200,"button":"left","press":true}
POST /scroll     {"y":-3}
POST /screenshot {"path":"/tmp/table.png"}
```

Bodies are read by a hand-rolled field scanner, not a JSON parser: send flat
objects with plain string and number values. Nesting, arrays and escapes are
not understood. Every route answers `{"ok":true,…}` or
`{"error":"<reason>"}` — **read the answer**; a refused request is a 200 with
an error in it, so `curl -f` will not catch it.

`name` for `/key` is whatever `crates/baylee-client/src/keys.rs` accepts, plus
the modifiers. Keys go into `ButtonInput<KeyCode>`, so they travel through the
account's `Keymap` exactly as a real press does — which is the part most worth
exercising, and why `docs/keyboard-map.md` is the list of what to send.

## Playing a duel through it

Never send a fixed script of keys. Read the question, answer it, read again:

```bash
q() { curl -s localhost:28770/state | python3 -m json.tool | head -40; }
k() { curl -s -XPOST localhost:28770/key -d "{\"name\":\"$1\"}"; }
```

`/state.interaction.pending` is the engine's own question — the same
`Pending` the taxonomy defines — and `/state.view` is the seat's whole
`PlayerView`. Together they say what is legal right now. Pass priority with
the `Space` action until `pending` becomes the choice you are after, then
answer it. A duel reaches combat in a handful of passes.

To act on a card, find its screen position rather than guessing: `/state.view`
gives you the object, the board gives you the lane, and a screenshot gives you
the pixels. Clicking blind wastes more turns than measuring once.

## The five things that go wrong

**A click is three frames.** `/pointer` writes a `CursorMoved`, then the press,
then the release, mirrored into `WindowEvent` the way `bevy_winit` does,
because Bevy's picking backend reads window events and turns a press into a
`Pointer<Click>` only once press and release land on the same hovered entity.
The route answers *after* the release, so a click is several frames slower than
a key. The first version set the cursor and pressed in one frame, answered
`{"ok":true}`, and clicked nothing at all — the screenshot before and after
were byte-identical.

**Coordinates are logical, screenshots are physical.** `/health` reports
`width`, `height` and `scale` so the ratio is read rather than guessed. On a
Retina display a guess is wrong by a factor of two, and a click lands in the
wrong quarter of the window.

**A wheel lands where the pointer is.** `/scroll` sends both a `MouseWheel`
message and the `WindowEvent` beside it, because picking is what turns a wheel
into the `Pointer<Scroll>` a list listens for. Put the pointer over the list
with `/pointer` first, or the scroll goes to whatever was under it last.

**Three states answer silently, and all three look like "the key did
nothing".** `/state` reports them for exactly that reason:

- `outbox` — an action was built and queued but never sent.
- `mana_run` — the client is tapping lands on the player's behalf and owns the
  next few keys.
- `ability_menu` — an ability chooser is open and is swallowing the keyboard.

Check those three before concluding that a handler is unwired. `last_error`
carries the engine's refusal of the last action, which is the fourth thing that
looks like silence.

**A held key is not a tapped one.** `{"hold":true}` presses and leaves the key
down; `{"release":true}` lifts it. Part of the client is about a key *being*
held — shift turns a double-faced card over for as long as it is down — and a
harness that could only tap cannot reach it.

## Screenshots, and proving a render claim

`/screenshot` replies only once the file is written, so the reply is the
signal — there is no need to sleep after it.

Then **measure; do not theorise**. `docs/client.md` and the
`measure-before-theorising-on-render-bugs` lesson exist because a table shipped
as a black screen and the cause was none of the plausible ones: an opaque
overlay that defaulted to open. Two rules earned the hard way:

- Compare against a shader-free reference. A clear colour touches no material,
  texture or shader, so rendering a known colour and reading the pixel back
  separates "this shader is wrong" from "nothing is drawing at all".
- Bound an assertion on **both** sides. A one-sided "dark enough" check is what
  let a felt authored four times too dark through.

To prove an *animation*, take two frames and diff them, peak per channel, and
always run the counter-test — a diff that is non-zero for the wrong reason
proves nothing.

## What this is not for

It is not a substitute for a test. Anything that can be asserted in
`baylee-client-core` belongs there, where it runs in milliseconds and in CI;
the harness reaches the things that only exist once there is a window. And it
is not the way to drive an *opponent*: that is `baylee-engine-server`'s
listening harness, where a socket names its own seat and taking over an AI
chair gets you a scripted opponent seeing exactly what a networked seat sees.
