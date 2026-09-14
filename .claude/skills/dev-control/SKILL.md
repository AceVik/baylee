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

That reaches `LocalHost` — a duel in process against the house AI, from
`data/acceptance-decks.txt`, needing no gateway, no account and no agent — but
not on its own: without a `SeatTicket` the binary adds `LobbyPlugin` and opens
the **lobby**, so `/state` answers `"view":null` until two clicks have been
made. "Play offline", then "Play the house". For a *networked* seat add
`BAYLEE_GATEWAY`, `BAYLEE_GAME` and `BAYLEE_SEAT_TOKEN` (a `SeatTicket`, most
easily made with `cargo run -p xtask -- dev-table --seats 2 --ai sharp`), and
the lobby is skipped.

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

**The window can be resized from outside**, which is how a layout is checked at
a width other than this desktop's. There is no route for it; macOS will do it
by PID:

```bash
osascript -e 'tell application "System Events" to tell (first process whose unix id is '"$PID"') to set size of front window to {1280, 800}'
curl -s localhost:28770/health    # {"…","width":1280,"height":768,"scale":2}
```

`/health` is the acceptance, not the number asked for: the frame keeps its
title bar, so 800 comes back as 768 of canvas, and the layout that matters is
the one `/health` reports.

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
the modifiers — and those are **physical** key names, `KeyK` and not the `K`
`docs/keyboard-map.md` prints. Keys go into `ButtonInput<KeyCode>`, so they
travel through the account's `Keymap` exactly as a real press does, which is
the part most worth exercising and why the keyboard map is the list of what to
send.

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

`legal.castable` and `legal.abilities` are what the seat can pay for **now**,
out of mana already floating. A land in play is not mana, so an ability costing
{5} is offered by neither list until five lands have been tapped by hand — walk
the cursor onto each and press the take key. An empty `abilities` on a
permanent that obviously has one is nearly always this and not a client fault.

To act on a card, find its screen position rather than guessing: `/state.view`
gives you the object, the board gives you the lane, and a screenshot gives you
the pixels. Clicking blind wastes more turns than measuring once.

## The eight things that go wrong

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

**Two questions answer with keys that are not the card cursor's.** An open
ability chooser owns the keyboard: `KeyA`/`KeyD` step `ability_pick` and not
the card cursor, `KeyE` or Enter takes the entry, `Escape` closes it. `/state`
says only that the menu is open and which object it belongs to — there is no
field for the pick — so count presses from zero and read the entries off
`legal.abilities` in the order they appear there. And `ChooseTargets` is *two*
keys: `KeyE` on the hovered object toggles it into `interaction.selected`, and
`Space` sends whatever is selected. `Space` with `selected: 0` and a `min` of 0
answers "no targets", silently and legally — which is how a Spark Double
resolves as a 0/0 and dies to a state-based action with nothing in
`last_error` to say why.

**A held key is not a tapped one.** `{"hold":true}` presses and leaves the key
down; `{"release":true}` lifts it. Part of the client is about a key *being*
held — shift turns a double-faced card over for as long as it is down — and a
harness that could only tap cannot reach it.

**A `/pointer` move is sometimes lost outright, and only re-*sending* it
helps.** Measured: the cursor put on a permanent at (600, 311), then
`/state.hovered` read **fifteen times** in a row — `None` every time. The same
move sent a second time answered with the object on the first read. Reading
again never repairs it; sending again always does. So aim in a loop until the
client says what is under the pointer, and only then press:

```bash
until curl -s localhost:28770/state | grep -q '"hovered":{"object":52'; do
  curl -s -XPOST localhost:28770/pointer -d '{"x":600,"y":311}'
done
curl -s -XPOST localhost:28770/pointer -d '{"x":600,"y":311,"press":true}'
```

That is also the whole reason "hover first, then click" works: not the extra
frame, the second send.

**And what a click did is visible a frame later** — the one place where
reading again *is* the repair. A click that arms a card answers, and
`/state.armed` is still `null`; a moment later it is complete with nothing
sent in between. Poll the state after a click instead of reading it once, or
a click that worked is written down as a dead handler.

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
