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
a width other than this desktop's — and the shelf's three columns collide or do
not at a width, so there is no reading this one off a 1728-pixel photograph.
There is no route for it; macOS will do it by PID:

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
POST /pointer    {"x":100,"y":200,"button":"left","press":true,"hold":false,"release":false}
POST /scroll     {"y":-3}
POST /screenshot {"path":"/tmp/table.png"}
POST /timescale  {"speed":0.1}          → {"ok":true,"speed":0.1,"paused":false}
POST /pause      {}  |  {"paused":false} → the same two numbers
POST /step       {"frames":6}           → answers after the sixth frame, paused
```

Bodies are read by a hand-rolled field scanner, not a JSON parser: send flat
objects with plain string and number values. Nesting, arrays and escapes are
not understood. Every route answers `{"ok":true,…}` or
`{"error":"<reason>"}` — **read the answer**; a refused request is a 200 with
an error in it, so `curl -f` will not catch it.

`name` for `/key` is whatever `crates/baylee-client/src/keys.rs` accepts, plus
the modifiers — those are **physical** key names, `KeyK` rather than the `K`
`docs/keyboard-map.md` prints — plus, for a letter or a digit, the bare
character: `Y` and `1` resolve to `KeyY` and `Digit1`. That alias exists
because this paragraph was here and a whole morning was still lost to
`{"name":"Y"}` being refused and read as a key that did nothing; see
`docs/observed-faults.md` 52, which is the withdrawn entry that came out of
it. Keys go into `ButtonInput<KeyCode>`, so they travel through the account's
`Keymap` exactly as a real press does, which is the part most worth exercising
and why the keyboard map is the list of what to send.

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

To act on a card, read `/state.cards` — every card drawn on the table, by
`object` and by `name`, with `at_x`/`at_y` already in the logical pixels
`/pointer` takes and `w`/`h` for the box it covers:

```bash
card() { curl -s localhost:28770/state \
  | python3 -c 'import json,sys
for c in json.load(sys.stdin)["cards"]: print(c["name"], c["at_x"], c["at_y"])'; }
```

The numbers are this frame's, measured from where `glide` has each card right
now, so a card still travelling reports where it is rather than where it is
going — read them again after the board moves rather than keeping them. Do not
go back to finding a card by eye on a screenshot: that is three lookups, the
last of them on a half-size image, and it has to be redone every time a lane
repacks.

`/state.buttons` is the same answer for the HUD: the prompt bar's answers by
name (`Yes`, `No`, `Keep`, `Mulligan`, `Confirm`, `DeclareNothing`) and the
ability and choice rows by index. Press one by name rather than by eye:

```bash
btn() { curl -s localhost:28770/state \
  | python3 -c "import json,sys
for b in json.load(sys.stdin)['buttons']:
    if b['label'] == '$1': print(b['at_x'], b['at_y']); break"; }
```

Use it when a pointer is what you want. Every answer in that bar also has a
key: a shockland's "pay 2 life?" is `Y` or `N`, a mulligan is `K` or `B`,
`Confirm` is `Space`. The entry that used to say otherwise
(`docs/observed-faults.md` 52) was withdrawn — the keys were being sent under
names `/key` refuses.

Two fields beside those answer a "nothing happened" that is really "something
happened quietly". `armed` is the tap that has been made and not sent — the
first tap on anything irreversible only arms it, and the second fires it
(`{"object":58,"deed":"Play"}`, then null). And in combat `selected` is
**empty** by design: an attack and a block are *pairs*, and they are in
`interaction.assignments`, beside the `focus` they are aimed at. Watching
`selected` through a declaration shows nothing moving while the whole thing is
being built.

## The ten things that go wrong

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

**A key that was refused looks exactly like a key that did nothing.** Both
leave the game where it was, `last_error` null and `pending` unchanged, and
the difference is only in the answer `/key` already gave you:
`{"ok":true,"pressed":1}` or `{"error":"unknown key: …"}`. Before writing down
that a handler is unwired, check that the press was accepted — a helper that
throws the body away is a helper that will sooner or later hand you a fault
that is not there. It has: `docs/observed-faults.md` 52.

**A held key is not a tapped one.** `{"hold":true}` presses and leaves the key
down; `{"release":true}` lifts it. Part of the client is about a key *being*
held — shift turns a double-faced card over for as long as it is down — and a
harness that could only tap cannot reach it.

**The pointer takes the same pair, and a press is not a click.**
`{"hold":true}` presses and stops there — `press` need not be said as well,
and until `100c211` it did, with a body that only said `hold` falling through
to a bare cursor move and answering `{"ok":true,"clicked":false}`. A later
`{"release":true}` lets go; the answer says `"clicked"`, `"held"` or
`"released"`, so a script cannot confuse them. Everything that exists only
*between* the two needs it — a drag, and a card giving way under the finger —
because a screenshot cannot be asked for in the middle of one call.
Photograph the rest state, hold, photograph, release, photograph: the third
should come back to the first, and "byte-identical" is a real answer.

**A move is sometimes lost outright, and only re-*sending* it helps.**
Measured: the cursor put on a permanent at (600, 311), then `/state.hovered`
read **fifteen times** in a row — `None` every time; the same move sent a
second time answered with the object on the first read. Reading again never
repairs it, sending again always does. So aim in a loop until the client says
what is under the pointer, and only then press:

```bash
until curl -s localhost:28770/state | grep -q '"hovered":{"object":52'; do
  curl -s -XPOST localhost:28770/pointer -d '{"x":600,"y":311}'
done
curl -s -XPOST localhost:28770/pointer -d '{"x":600,"y":311,"press":true}'
```

That is the whole reason "hover first, then click" works: not the extra frame,
the second send. Two repairs are owed in `devctl.rs` — get the move through a
full frame before the press *and* assure it arrived, and answer only once the
frame that **read** the click is done.

**And what a click did is visible a frame later** — the one place where
reading again *is* the repair. A click that arms a card answers, and
`/state.armed` is still `null`; a moment later it stands there complete with
nothing sent in between. Poll after a click instead of reading once, or a
click that worked is written down as a dead handler.

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

**A misplaced thing wants the arithmetic beside the picture.** `/state.shelves`
reports where each seat's bar was told to go — `mid_x`/`mid_y`, the projected
`along` and `depth` of its mat's ledge, the chosen `density`, the box that came
out of it and the `ink` that box has to hold — in the same logical pixels
`/pointer` takes. Read those against the pixels in a screenshot and "it looks
displaced" splits into two different bugs that need different fixes: ink that
does not sit where the model says (the drawing is wrong) versus a model whose
ledge is not the ledge on screen (the projection is wrong). It was added for
that split and settled it in one reading.

## Working on a look, rather than photographing one

Two things turn "rebuild, launch, blink, miss it" into an actual loop, and
they are meant to be used together.

**Stop the clock.** `/timescale` scales `Time<Virtual>`, `/pause` stops it,
`/step` advances a counted number of frames and pauses again — answering only
once the last of them has been drawn, so the reply is the signal and there is
nothing to sleep on. Everything the table draws with reads through that clock
(`table::glide`, `table::retire`, the sheen clock, every shader's
`globals.time`), so a tenth speed slows the whole picture together instead of
pulling one movement's parts apart. Zero is refused by `/timescale` on
purpose: a pause and a stopped speed are different states, and a caller who
could stop the clock two ways would have to remember which one to undo.

**Edit the shader under the running client.** Launch it with both features and
both variables — each of the four is load-bearing, and the one most easily
dropped is the port: `--features dev-control` without `BAYLEE_DEV_CONTROL`
opens no socket, so there is nothing to pause the picture with while the
shader is being edited.

```bash
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD \
    cargo run -p baylee-client --features dev-control,dev-reload
```

Then an edit to `crates/baylee-client/src/shaders/felt.wgsl` repaints the felt
in the client already on screen — no rebuild, no restart, and the camera,
the board and the game keep their state. `BEVY_ASSET_ROOT` is not optional and
is not decoration: bevy's watcher strips its own base path off every changed
file before looking it up, and with the wrong base every lookup misses in
silence. It used to look exactly like a watcher that worked; the binary now
refuses to start instead.

The proof shape for a look, and it is the same shape as any animation claim:
`/pause`, screenshot, screenshot again — byte-identical, which is the
counter-test for the harness. Edit the colour, screenshot — different, which
is the claim. Put the colour back, screenshot — byte-identical to the first,
which is the counter-test for the claim.

## What this is not for

It is not a substitute for a test. Anything that can be asserted in
`baylee-client-core` belongs there, where it runs in milliseconds and in CI;
the harness reaches the things that only exist once there is a window. And it
is not the way to drive an *opponent*: that is `baylee-engine-server`'s
listening harness, where a socket names its own seat and taking over an AI
chair gets you a scripted opponent seeing exactly what a networked seat sees.
