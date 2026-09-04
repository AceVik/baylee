# Where the client is going

This is the design programme for the duel client: what it should look like,
how it should be operated, and what is still missing before it is a complete
Magic client. `docs/client.md` remains normative for what the client *is*;
this file is normative for what it is becoming, and the two are meant to be
read in that order.

It was written from three parallel read-only audits of the tree at `f0b0e81`
— visual language, interaction model, completeness — plus verification of the
claims that carry the most weight. Where a finding is quoted below it names the
file it rests on. Where a number was checked against the code rather than
against a document, it says so, because two of the numbers in `CLAUDE.md` were
wrong and one of them sent an audit down the wrong path.

Two facts bound every proposal here and were verified rather than assumed:
the workspace is **Bevy 0.19** (`Cargo.toml:97`), and the wasm build renders
through **WebGL2**, not WebGPU (`Cargo.toml:107` lists `webgl2`; there is no
`webgpu` feature and `index.html` overrides nothing). So every shader below
obeys what `card_common.wgsl` already obeys: uniforms only, no storage
buffers, no compute, compile-time loop bounds.

---

## 0. Three locks, before any of the design

A design programme that begins with palettes while the table freezes is a
programme that will not be believed. Three faults lock a real game today. Two
were found by audit and confirmed by reading the code; the third is the one I
had been chasing by hand.

### Lock A — a refused action removes the question

`flush_outbox` clears `duel.interaction` the moment it sends
(`crates/baylee-client/src/lib.rs:630`). If the engine then refuses the
action, nothing replaces it:

```rust
// crates/baylee-engine-server/src/lib.rs:241
// An illegal action is the seat's problem, not the game's: `act`
// already refused it and nothing moved.
let Ok(routed) = session.act(player, action) else {
    return Vec::new();
};
```

The reasoning in that comment is sound about the *game* and wrong about the
*client*: nothing moved on the table, but the client had already thrown the
question away. The seat is left with no prompt and no way to answer.

Locally that is a freeze. Over the network it is worse than a freeze: the
decision clock keeps running in the engine process and on expiry the house AI
answers for the seat (`crates/baylee-gamehost/src/session.rs:313-331`). A
refused click does not stall your seat — **it spends your decision.**

It is reachable from an ordinary click. `is_selectable` accepts any `ObjectId`
when `options` is empty (`interaction.rs:566`, the discard and bottom-of-library
case), and `activate_card` falls through to `toggle` for a permanent with no
abilities (`input.rs:79-90`). So tapping a land during a cleanup discard
selects it, Confirm sends it, the engine refuses it, and that seat is done for
the rest of the game. Offer-a-Draw is a second door to the same room: the
button is enabled unconditionally (`hud/overlay.rs:235`) while the engine
requires the offerer to hold priority (`engine/actions.rs:15-20`).

The existing test asserts that a rejection *surfaces* (`host.rs:380-392`). It
does not assert that the player can still answer.

**This is consistent with the fault I measured live yesterday, and not yet
confirmed as its cause** — a legal land under the cursor that no key and no
click would play, with `last_error` null and the phase advancing on its own.
The advancing phase fits: it is the house AI taking the seat. One thing does
not. That `/state` dump showed `interaction` *present*, with `lands`
containing the card; Lock A predicts `interaction: null`. Both can hold only
if the AI answered and a fresh `Pending` arrived between the key and the read,
which is plausible and unmeasured. The `outbox` / `mana_run` / `ability_menu`
fields added to `/state` today are what settle it: a queued-but-unsent action
or a mana run owning the keys are the two other shapes that look identical
from outside. Fix Lock A regardless — it is a real defect on its own evidence,
which is the `Vec::new()` at `engine-server/src/lib.rs:248`.

**Fixed.** The repair turned out to be smaller and better placed than the plan
above, which wanted the client to remember `last_pending` and rebuild the
`Interaction` itself. It does not have to: `Session::snapshot` already returns
the outstanding choice for a seat, already sends it *only* to the seat being
awaited, and is documented read-only precisely so a reconnect cannot pump an AI
seat's turn. So both hosts just ask it again —

- `LocalHost::submit` pushes the `Failed`, then feeds `snapshot(seat)` through
  the same `host_message` decoder its normal traffic uses.
- `EngineRunner::refused` sends an `Error` frame plus that snapshot, through
  the existing `route`, so an unattached seat is still filtered out.

Fixing it in the *host* rather than in `Duel` means the client keeps no new
state and the rule holds for anything else that ever speaks the protocol.
A `Phrase::WhyEngineRefused` on the prompt's second line is still worth having,
but it is now cosmetic rather than load-bearing.

### Lock B — questions whose options are drawn nowhere

`PlayerView::looking_at` is filled by the gamehost for library searches, scry,
dig, wish and reorder (`gamehost/view.rs:253-269`). I checked what reads it in
the client:

```
grep -rn 'looking_at' crates/baylee-client/src crates/baylee-client-core/src
→ table.rs:240    ….looking_at(look, Vec3::Y)      Transform, the camera
→ table.rs:623    ….looking_at(Vec3::ZERO, Vec3::Y) Transform, the camera
→ test_support.rs:97   looking_at: Vec::new(),      test scaffolding
```

Three hits, unfiltered: two are Bevy's `Transform::looking_at` and have
nothing to do with the field, and the third initialises it empty in a test.
**Nothing has ever rendered it.** So a
`ChooseCards` from a library search, and *every* `OrderObjects` (both are
library reorders, `resolve/mod.rs:512,904`), draws a headline and
`HintClickBoard` over a board containing nothing to click. The prompt bar shows
its OK button only when `can_confirm()` is true, so what a player gets is a
dead bar. The gamehost even names the missing consumer in a comment at
`view.rs:814`: "without `looking_at` the dialog is a row of blanks."

The same hole covers every target in a graveyard, in exile, or on the stack —
counterspells and reanimation cannot be aimed at all, because the stack panel
is `Pickable::IGNORE` throughout (`hud/stack.rs`) and graveyards reach the
board model only as counts (`board.rs:320-323`).

Fetchlands landed in `f0b0e81`. This is now in most decks.

### Why CI is green over both

`crates/baylee-client/tests/duel_flow.rs` answers `ChooseCards`,
`ChooseTargets` and `OrderObjects` by hand-building the `PlayerAction`. Those
are exactly the variants whose options can live off the board. A test that
constructs the action proves the engine accepts it, not that a player can
produce it — which is the reason `docs/client.md` already gives for the colour
case, applied there and nowhere else.

The repair that closes the class is an invariant, not a fix: **every option in
`Interaction::selectable()` must be drawn**, computed from the board model,
the zone-browser model and the chooser rows, never from the renderer. That test
fails on both locks and on every future one of the same shape.

### Two corrections to `CLAUDE.md`

- `VIEW_VERSION` is **9** (`crates/baylee-view/src/lib.rs:41`); the file said 7,
  then 8. Bumped to 9 by `PlayerView::priority_held`. One audit worked from
  the stale number, which is what a stale contract file costs.
- The copy limit was enforced **per row** in the gateway (`main.rs`) and **per
  card** in the client (`deckbuilder/builder.rs`). Two printings of one card
  therefore saved with eight copies — through the side that is supposed to be
  doing the enforcing, so it was a way to cheat rather than a display bug.
  **Fixed:** `parse_deck_lines` now accumulates per `CardIndex`.

  This one is not a `CLAUDE.md` correction, and calling it one was my own
  error. The file says "the copy limit stays on the card, as the gateway
  enforces it" — which was a true statement of the *intent* and a false
  statement about the code, because the gateway did not do it. The file is
  correct as of the fix; nothing in it needs changing. Worth keeping as a
  reminder that a doc which disagrees with an enforcing code path is a bug
  report about the code, not a typo in the prose.

  One question that fix deliberately does **not** answer: both sides count per
  *list*, so four in the deck and four in the sideboard is eight legal copies.
  Real tournament rules count the two together. Changing that means changing
  `DeckBuilder::add_print` in the same breath, because `CLAUDE.md`'s rule for
  the builder is that a live save button always saves — so it is its own
  change, with its own test, and not a thing to slip into a bug fix. Noted
  here so it is not lost.

---

## 1. The design language

### 1.1 The table, read numerically

The hearth ring is `HEARTH_SIZE` 34 with its band at 0.46–0.60 of the
half-size: a ring roughly 20 table units across on a 44-unit felt, with 24
ticks and two hairlines, and the medallion's two rings inside it. It is the
largest, brightest, most detailed object on screen, and the eye reads a
roulette wheel. Nothing in the frame says *cards are played here*.

The felt is not the problem — `CLOTH` (0.098, 0.165, 0.122) is a decent
billiard green — the `WARM` candle pool at α 0.12 sitting on top of it is,
which is why the middle reads olive and the corners read green.

First moves: `HEARTH_SIZE` 34 → 18, 24 ticks → 8 (a compass, not a clock), one
hairline, `WARM` α 0.12 → 0.06, medallion glows α 0.55 → 0.35. The measurable
bound: in a full-table shot the ring must be smaller than the nearest seat's
mat.

And the prerequisite that makes all of it moot until it is done: the local
seat's battlefield is underneath the hand bar.

### 1.2 Tokens

One palette source. "Blue" is currently defined three times — `tabletop::PIE`,
`manaui::disc_color`, `face::frame_color`. `PIE` is already renderer-free and
becomes the single source; disc and frame derive from it and the derivation is
pinned by a test.

| token | hex | purpose |
|---|---|---|
| `felt.deep / cloth / worn` | `#090F0B` `#192A1F` `#243729` | the world's neutral |
| `pie.W U B R G` | `#F0E8CC #4A8CD4 #4D4557 #D45C47 #5CA86B` | colour identity — **no other token may sit on these hues** |
| `gilt` | `#C7A154` | *metal*: mine, my seat, the active rim. Never pulses |
| `candle` | `#FFDB9E` | *light*: the game is asking you. May breathe |
| `ember` / `cool` / `dusk` | `#E65738` `#6B94DB` `#8566B8` | combat / untap-draw / end washes |
| `steel` | `#5C6B80` | indestructible band |
| `jade` / `frost` | `#99E6BD` `#CCDBE6` | hexproof / shroud films, deliberately off-pie |
| `slate` | `#0F1214` α .80 | chrome; blue-black so it reads as *not cloth* |

Two decisions inside that table. **`palette::ACCENT` teal goes** — it is the
one hue on screen that comes from nowhere in this world, and "the game is
asking you" becomes `candle` while "mine" stays `gilt`; the two are told apart
by metaphor (light versus metal) and by behaviour (candle may breathe, gilt
never does). And **offers become one hue at two energies**: today playable is
gold and reachable is indigo, which spends a second hue on a distinction that
is really about certainty. Both become `candle` — the engine's yes at full
strength, the client's "I could tap for that" at half with a dashed perimeter.
`dusk` is then free to mean only end-of-turn.

Type: three faces are shipped and there is no fourth. A serif display face is
the cliché to avoid and fails at the 8–10 px this game sets card text at;
Beleren is not ours to imitate. Inter carries every role, separated by size,
weight and tracking rather than by family — and every number that changes gets
`tnum` (`TextFont::font_features`, `FontFeatureTag::TABULAR_FIGURES`,
verified in `bevy_text-0.19.1`), which nothing currently sets. Life totals
that jitter their neighbours are a legibility bug, not a nicety.

### 1.3 The keyword algebra

This is the question that was asked directly — how do several animated
keyword borders compose without becoming noise — and the answer is that they
must not compose at all. Colours that mix are the failure mode: six glows
summing to a grey halo, on a board of thirty such halos strobing at unrelated
rates, next to a card whose colour identity is now unreadable.

So instead: **eight channels, one keyword per channel, and hue is not one of
them.**

| | channel | capacity | nature |
|---|---|---|---|
| C1 | band **material** (inner 60% of the border) | one | continuous |
| C2 | band **film** (outer haze) | one | continuous |
| C3 | **perimeter light** (travelling) | one | transient — an offer |
| C4 | **face treatment** | one | this-turn state |
| C5 | **elevation** | one | geometric |
| C6 | **rail** marks | ordered list | discrete, countable |
| C7 | **corner plate** | numbers | P/T, damage, counters |
| C8 | **shadow tint** | one | selection, targeting |

Five rules hold it together:

1. One keyword, one channel — never two.
2. A continuous channel holds at most one claim; two claims are resolved by a
   **dominance table**, engine-truthfully, and the loser is not drawn.
3. A discrete channel shows *all* claims, shrinking slot width, never hiding —
   a tail nobody can see is a lie by omission.
4. **Hue is not a free channel.** The five pie hues are colour identity;
   films live off-pie (`jade`, `frost`, `steel`).
5. **Motion is not a free channel either.** Everything that says "is" breathes
   on one shared `BEAT` with per-effect phase offsets; only the *offer*
   travels, so a player can find it with peripheral vision because it is the
   only thing moving along an edge.

The ceiling is **three continuous claims legible at once** — one band, one face
treatment, one elevation — plus the transient offer. There is deliberately
nothing left to assign a fourth to.

**Dominance.** `glow_bits` already drops hexproof under shroud, which is
correct: a shrouded object cannot be targeted by anyone, so hexproof adds
nothing. Two more collapses of exactly the same kind are currently drawn as two
marks where one is a fact — **double strike ⊐ first strike** (CR 702.4: the
first-strike mark carries no information the double-strike mark does not) and
**unblockable ⊐ menace** (menace's only effect is a restriction on being
blocked). Flying does *not* collapse under either, because it also decides what
the creature can block. No other collapse: hexproof and indestructible are
different facts and both draw — which is what having C1 and C2 as separate
channels is *for*.

**Shroud + indestructible, the case that had to work.** Today `film_amount`
0.88 over a base buries the steel and a player sees a lavender rim. The fix is
sub-lanes: the base occupies the inner 60% of the band at full strength with
its specular line intact, the film becomes a soft outer haze peaking at the
card's edge, and the film's alpha is **capped at 0.55 whenever a base is
present**. Shroud's haze is already noise-modulated, and that noise is what
lets the specular through — mist over metal is the right picture. The
measurement: at 90 px card width the steel specular must be visible along the
top and bottom edges through the frost. Below α 0.40 the film stops reading as
a film; that is the floor.

**Flying is elevation, not colour.** It is the keyword a player scans a board
for, and its picture is geometric: the card floats, the contact shadow stays on
the table, and the *gap* is the cue. `FLYING_LIFT` 0.05 goes into the base lift
in `card_transform`, never into the hover/selected `+=` lines, or a hovered
flier would move 0.036 and read as nothing. The rail mark stays too — rule 3 —
exactly as a real player both sets the card on a die and reads the word.

All 20 keywords in `keyword_tests::ENFORCED` get an assignment. Three are not
battlefield facts and get nothing on a permanent (flash, uncounterable,
rebound); changeling is a type-line fact and belongs in the type line, not on
the border. Prowess and unblockable become new rail marks 12 and 13.

**Past six marks the rail wraps to two rows** rather than compressing below
7 px, because seven keywords on one creature is rare and the picture should
look rare. And `cardrail::badge_at` — written, and called by nothing — becomes
the hover and long-press tooltip, which is the honest fallback for a pip that
small: the pip says a keyword is there, the word says which.

**The prerequisite nobody has wired.** `Preferences::reduce_motion` stops
`glide` and the camera and is ignored by every shader animation. A `motion: f32`
in `CardParams` multiplied into `t` fixes all of them at once (std140 32 → 48
bytes). At `motion = 0` the offer light must degrade to a *steady* perimeter at
mean brightness rather than vanishing. This lands before any new animation in
this document — it is the price of admission for all of them.

### 1.4 Counters, and the dice question

The dice idea was evaluated properly and rejected, and I agree with the
reasoning. A die on a slab at `CAMERA_LEAN` is about 6 px tall; nobody reads a
d8 from a d10 at 6 px, and nobody reads it at 60 px either without counting
faces. **Shape is not a number.** What a player actually reads off a real die
is the numeral on top — so a polyhedron costs a mesh, a material, a
light-dependent silhouette and a text label in order to deliver what the label
delivers alone.

But dice do carry one thing worth keeping: **pips**. One to six as dot patterns
are read pre-attentively, faster than numerals and with no glyph at all.

So a **chip, not a die**: flat, round, stamped, die pips up to 6, numerals from
7, in the reserved bottom-right fifth. A chip is also what a counter looks like
when it is *paper*, which is what Magic actually sells, and on a 2.5D table a
flat token sits honestly where a polyhedron does not. Up to three chips are
visible and a fourth kind collapses to "+N" with the tooltip listing them.
+1/+1 and −1/−1 annihilate in the engine (CR 704.5q), so only one of those ever
shows.

Lore counters are the exception that proves the rule: a saga chapter is a page,
not a token, so it gets a *square* parchment plate with a roman numeral.

**Loyalty is not a counter.** It is `PublicObject.loyalty`, its own field, and
it is a planeswalker's life total. It takes the P/T plate's place — same
corner, same plate, same numeral role — with a gilt rim. Explicitly **not a
shield**: a shield-shaped loyalty box in that corner is the WotC planeswalker
frame's own element, and "a plain shield nobody owns" is the argument every
borrowed frame element makes.

And the plainest gap of all, found by grep rather than by eye: **there is no
P/T, no damage and no counter drawn anywhere on a card showing art.** They exist
only on the constructed text face. Three rules facts, invisible on the normal
view of the board. That goes first.

### 1.5 States that are not keywords

They take channels no keyword uses, so they cannot collide by construction.
Tapped is rotation; summoning sickness is the face treatment (retimed onto the
shared `BEAT`); targeted and selected are the shadow tint; phased-out is face
alpha.

Combat is **position**. An attacker slides toward the defender's side of its
lane; a blocker slides to meet it; a thin hairline pairs them when there are two
or more defenders. Not a glow, because combat is already a colour on the table
— the ember phase wash — and a red border on an attacker would sit inside a red
room. Position is the one channel nothing else uses during combat, and it is
what a player does with a physical card: slides it forward. `CombatView`
already carries everything needed; no view change.

### 1.6 Motion

Everything moves through `Motion` and `glide`'s `1 − e^(−rate·dt)`, so the
table below is rates, never fixed clocks — 95% settle is `3/rate`.

| event | rate | settle |
|---|---|---|
| enters battlefield, lane repack, tap | 16 | 190 ms |
| hover lift | 22 | 135 ms — a hover is a question, not an event |
| attack advance / block meet | 12 | 250 ms — deliberate; the two arrive together |
| **leaves the board** | 12 | 250 ms — new; today the card despawns on the frame it vanishes |
| camera | 24 | 125 ms |

Shader tempo is the same discipline: one `BEAT` for every "is" breath, half of
it for steel (metal is slow), double for the lethal flash. Six unrelated rates
is the fairground this section exists to prevent.

### 1.7 Day/night, sleeves, playmats

Day/night does not exist in the engine, the view, core or the DSL — verified by
grep. As a rules fact (CR 726) it is engine work, and it is listed in §4. What
belongs here is why it will not fight the phase tint when it arrives: different
*extent* (the phase wash is a lamp over the pool; day/night is a single scalar
multiplied into the felt everywhere) and different *timescale* (phases change
every few seconds; day/night changes once a turn at most and glides three times
slower). Night darkens felt, never cards.

The sleeve seam is three local facts and no protocol: a `SeatStyle` per seat
resolved from a resource that returns the default for everyone today; an
`edge_tint` on `CardParams` so the card's wall can be the sleeve's colour
rather than the scan's edge; and a **generated** back — a guilloché of summed
sines in polar coordinates, seeded, no `rand` — because ornament is the easiest
thing to borrow by accident and arithmetic borrows nothing. What a *remote*
seat's sleeve looks like has to travel in `GameStatic`, which is §4.

---

## 2. The interaction model

### 2.1 One pipeline

`input.rs` is 900 lines of Bevy events, resource reads and early returns in a
fixed system order, and it is where truth gets lost between `Interaction` and
the wire. Yesterday's land-drop fault is the evidence: I proved with a test
driving the real Bevy system that the chain works —

```
test input::tests::the_primary_key_plays_the_land_under_the_cursor ... ok
```

— which means the fault was never in the decision, only in the state the
decision reads. That is a structural problem and it gets a structural answer:

```
Bevy events ──► Gesture ──► resolve(&Gesture, &Snapshot) -> Vec<Intent> ──► apply
  input.rs                    core/gesture.rs (pure, headless)                input.rs
```

A keyboard cursor over a card and a pointer over the same card produce the same
`Hit`. Every HUD button dispatches into the same `resolve`, so a test that
fires `Act(Confirm)` and a test that taps the OK button become the same test —
and `Fired::of_actions`, which exists precisely to let buttons and touch enter
the key pipeline and is called only by tests, finally has a caller.

`Why` is an enum rather than a string and reaches three places: the prompt
bar's second line, `devctl /state`, and the tests. A refusal the engine still
makes after the client has checked is then a bug report with the gesture
attached.

I have already made `/state` report `outbox`, `mana_run` and `ability_menu`,
because all three answer silently and all three look exactly like "the key did
nothing".

### 2.2 The seventeen questions

`Interaction::mode_for` matches all 17 `Pending` variants with no wildcard arm
and `confirm()` builds an answer for every mode. **The model layer is
complete; the renderer is not.** The client makes exactly four things
clickable: battlefield cards, the own-board overlay lanes, the own hand and
command zone, and the player tabs.

Status by variant: eight are fine (`Mulligan`, `LegendChoice`, `ChooseColor`,
`ChoosePlayer`, `YesNo`, `ChooseSubtype` — the last only since yesterday —
`GameOver`, `Priority` in the ordinary case). Four are operable but **blind**:
`MulliganBottom` and `DiscardChoice` show no tally of what has been chosen,
`ChooseCastMode` renders "Mode 1 / Mode 2" with no text, and `ChooseNumber`
never prints its value anywhere while its arrow keys also pan the camera. Two
are **unreadable**: both combat prompts are fully operable and draw nothing,
because the renderer reads `selected()` — which is empty in combat modes —
while `is_selected()`, the method that knows about pairs, has no caller, and
`assignment()` and `focus_position()` have none either. Two **lock**:
`ChooseCards` with off-board options, and `OrderObjects` always.

The fix that covers the locked pair and most of the rest is one thing: a **zone
browser**. A model in `client-core` over `looking_at ∪ graveyards ∪ exile ∪
command ∪ stack`, and a tray panel that reuses `spawn_hand_card`'s node path —
**not a second card renderer**. It opens automatically when a pending's
options include anything outside battlefield and hand, and on demand from a
key or a tap on a pile chip at the mat's corner. It carries the filter box the
subtype picker already proved, and in order mode it answers `OrderObjects` by
click-click swap. One panel closes tutoring, scry, dig, wish, delve, reorder,
graveyard targets and counterspell targets.

### 2.3 The problems of scale

**A stack of thirty.** Consecutive entries with the same source, controller and
targets collapse into runs — Grapeshot's twelve copies become one row — decided
in the model, not the renderer. The top entry keeps its card because it is the
one being answered; everything else is a 24 px line in a scrollable column.
Two Bolts at two faces stay two rows, because the identity test is on the
targets.

**Forty tokens.** `CardGroup` groups by a key that includes damage and status,
which is correct for *drawing* — a damaged Soldier is no longer interchangeable
— and wrong for *declaring*, because one damaged token forks a group into forty
mid-declaration. A coarser `combat_key` for declaration, plus
`toggle_group(members, want)` and a count stepper, turns "attack with 30 of 40"
into one gesture. Which thirty is the engine's problem and no player cares.
`Lane.overflowing` is already computed and ignored; honouring it with a
collapse chip is the rule that a card never leaves the mat.

**A hand of eighteen.** The hand currently re-sorts by playability on every
priority change, which moves cards under the pointer. Draw order is the stable
answer, with playability expressed as a glow rather than a position, and an
explicit sort key for players who want one. A card a player is aiming at must
not move.

### 2.4 Stops, and the pass

The engine already has `PriorityHold::{Always, PassWhenNothingToDo,
UntilStackEmpty, UntilTopOfStack, UntilEndOfTurn}`, and `SetPriorityHold` is
journaled and legal at any time. The only sender today is the gateway replaying
standing answers. That is the whole F6 apparatus, unused.

Three layers: ~~**engine holds from the client**~~ (a key that sends
`UntilStackEmpty{depth}` — cancelled the moment anyone responds, which is
exactly F6 semantics), **the rail as the stop table** with an opt-in
"competitive stops" preset, and ~~**hold-this-turn** as a toggle rather than a
held key, because a held key cannot be a touch gesture~~.

**Done — the first and third, as `F6` and `F7`.** Two actions rather than one,
because "let this stack resolve" and "nothing more this turn" are genuinely
different orders and the engine has both. Three things the implementation
settled that the sketch above did not:

- *Both keys cancel.* Either key, pressed while any hold is running, sends
  `Always` rather than replacing the hold. A player who has stopped being
  asked should not have to remember which key did it.
- *A hold has no other symptom.* The prompt bar is empty because the seat is
  not being asked — which is exactly what an idle turn looks like. So the
  state is drawn (an accent chip beside the concede button) with the way out
  next to it. Without that, a hold set two turns ago and forgotten is a game
  playing itself with nothing on screen to blame.
- *The view carries it, as a bool.* `PlayerView::priority_held`,
  `VIEW_VERSION` 8 → 9, the viewing seat's own hold only — a hold is a
  statement about what its owner will respond to, and the whole table knowing
  it is a read a player is entitled to keep. `PassWhenNothingToDo` reports
  false through `PriorityHold::suppresses`, which `auto_answer` now shares, so
  the indicator cannot disagree with the engine.

**Done — the second, as `RailPreset`.** Two presets rather than one, because a
preset with no way back is a trap: competitive stops turn seventeen of the
twenty-four buttons red and clicking them green again one at a time is not an
undo. `Stop everywhere` is the default said out loud, and `Competitive stops`
keeps seven windows: both of your own main phases, the whole of combat on both
turns, and the end step of an opponent's.

The thing that decided which seven is not taste. **A red row is not a pass in a
declaration step** — `auto_answer` turns it into `DeclareNoAttackers` /
`DeclareNoBlockers`, which is a decision and not a skipped window. So the row
on whichever side actually asks this seat to declare has to stay green:
attackers on its own turn, blockers on an opponent's. A preset that got that
wrong would not stop asking, it would decline every block for the rest of the
game — and the test that holds it is written through `auto_answer` rather than
against the table, because the table is not the claim.

A preset is written as a button, not held as a mode. The chip is nonetheless
lit while the rail still matches, which is the only honest answer to "am I on
competitive stops right now"; the first hand correction puts both chips out.

One bug in that area, found in the audit: `Situation` carries no stack depth,
so a red rail row passes priority *with a spell on the stack*. Red should mean
"nothing to do here when nothing is happening", never "let their sorcery
resolve unanswered".

**The keymap.** This answers the open question I had been carrying. Today
`Primary` is Space and means "the hovered card if any, else pass" — a pass key
whose meaning depends on where the mouse happens to be resting. That is a
misclick generator, and it is the same precedence chain that made my land
mysteriously not play. **Space becomes `Confirm` (pass / OK) and Enter becomes
`Primary` (act on the cursor card)**, which is also the MTGO and Arena
convention. It is a swap of two rows in `Keymap::standard()` and no new action.

### 2.5 Undo, and its absence

There is no undo in the engine and there should not be one: a journaled,
deterministic kernel does not roll back. So the client's job is to make the
irreversible **two-stage**. ~~A tap on a spell *arms* it — the card lifts, its
mana plan is drawn as rings on the lands it would tap, and a second tap or
Enter sends it; Escape disarms with nothing on the wire.~~

~~**Mana abilities are the exception and stay one tap**, because floating mana
is the one cheap mistake: it empties at end of step and a wrong colour is fixed
by tapping another land. Exactly the entries in `legal.mana_abilities` — today
any lone ability goes through on the click that found it, including "Sacrifice
this:".~~ ~~Concede has no confirmation at all today; one misclick ends a
ranked game.~~

**Done.** `Duel::armed` holds an [`Armed`] — an
`ObjectId` and a `Deed` (`Play`, `Ability`, or a mana `Run`) — and the second
tap on the same card, the confirm keys, or the button in the prompt bar send
it. Everything is resolved against the *current* `LegalActions` at every one
of those points rather than trusted from the tap that armed it, which is the
rule `ManaRun` already followed step by step; a deed the engine has withdrawn
disarms instead of guessing. Cancel is first in the Escape order, because an
armed deed is the only state in the client with nothing on the wire behind it.

Two decisions worth writing down. The exemption is **not** `legal.mana_abilities`
after all: that list carries the CR 305.6 shortcut and granted abilities but
not a printed `{T}: Add {G}`, so a mana dork would have asked for a
confirmation a basic land does not. It is `AbilityOption::mana`, read off the
card's own `mana_ability` flag (CR 605.1) — and not off the source list either,
which reduces a permanent to the *one* tap it usually has: Yavimaya Coast
prints two mana abilities, and the second would have asked. And picking from the
**ability chooser arms** rather than sends — the chooser disambiguates, it does
not confirm, and a lone "Sacrifice this:" arming while the same ability among
three did not would be exactly the inconsistency a player trips on.

The *drawing* is two more bits in the `glow` word — `ARMED` and `WILL_TAP`,
bits 5 and 6, below `MARK_SHIFT` where three were free — and it is the same
sentence in two halves: the armed card says what will happen, the lands the
plan would spend say what it will cost. Drawn rather than written, because
"Tap 3, then cast" does not say *which* three and which three is a plan the
player never made.

Both ride in the border's outer register beside `ACTIVATABLE`, and the
grammar there is now three words rather than one. `ACTIVATABLE` travels — it
is an invitation, and a travelling light is what the eye finds across a whole
board. `ARMED` holds still and pulls in tight against the printed edge: the
invitation has been accepted, and a light that still moved would say it was
still a suggestion. `WILL_TAP` is cool where those two are warm, and a beat
behind the armed card's breath, because the price follows the verb. An armed
card is **not** also drawn activatable — `glow_of` drops the offer the arming
accepted, or the same border would carry a chase and a ring saying the same
thing twice.

The lift is the other half and costs no bit: an armed permanent takes the
`SELECTED_LIFT`, not the hover's, and keeps it when the pointer leaves,
because arming is a commitment the player has already made and being selected
is the same claim. In the hand the card stands eight pixels out of the row,
which is headroom the bar already had.

Two things about the group case. `Offer::on` takes a `CardGroup`'s **members**,
not its representative, because a plan taps one particular Forest and the card
drawn for it may be standing for four. And both bits are **any**, where
`CardGroup::activatable` is deliberately *all* — the reason for *all* is that
an offer invites a click and must not invite one that gets refused, and these
two invite nothing. A stack of three Forests two of which are about to tap is
better drawn lit than dark.

### 2.6 Touch

Touch never hovers, because hover is gated on `CursorMoved`. So the preview
becomes a long press, and the "hovered card" path — which is the primary
click's first precedence — is simply dead on a phone until gestures are unified
(§2.1). `Metrics::of`, which the lobby already has, moves into the duel HUD:
the rail becomes a horizontal strip because twelve 44 px rows do not fit a
phone's height, and `softkeys.rs` gains a filter field kind so the subtype and
browser boxes can raise a keyboard at all.

No drag-to-target. Tap-tap is the same gesture as the keyboard path, both are
tested by the same resolver, and a drag across a pinch-zoomed 3D table lands on
the wrong card. The one drag is the camera.

A four-player board does not fit a phone, so on a phone the 3D table shows only
the local mat and the seat whose turn it is; the others are their tabs, and the
text board is the real board view.

---

## 3. What a complete client still needs

Ordered by damage, not by area. Sizes are rough.

| | Item | Scope | Size |
|---|---|---|---|
| 1 | Lock A: refusal keeps the question (client + one engine-server function) | client + engine-server | 1–2 d |
| 2 | Lock B: the zone browser, and the "every offered option is drawn" invariant | client | 3–5 d |
| 3 | Stack and graveyard entries clickable as targets | client | 1–2 d |
| 4 | Reconnect — `NetworkHost::reconnect()` is called only from a test and `DuelReport::Failed` has no reader at all | client | 1 d |
| 5 | Disconnected-seat policy: an unattached seat stops the clock and stalls the table forever; `Playing` games are never reaped; engine death is silent to seats | engine-server + gateway | 3–4 d |
| 6 | Legal strings — the fan-content disclaimer and Scryfall attribution are required by `docs/legal.md` §2–§3 and appear nowhere in the client | client + gateway | 1 d |
| 7 | Combat drawn at all; P/T, damage and counters on cards showing art | client | 3–4 d |
| 8 | Commander end to end | core→engine→gateway→client, **bump** | 2–3 w |
| 9 | Opponents' command zones, graveyard/exile browsers, monarch badge, saga/level counters — all already in the view | client | 2–3 d |
| 10 | `ChooseNumber` visible; concede confirmation; Offer-a-Draw only with priority | client | 1 d |
| 11 | `layout` carried through `CardDef` — fixes adventure faces requesting a nonexistent back image, and the MDFC back that cannot be previewed from hand | dsl + codegen + gateway + client | 2–3 d |
| 12 | Decision clock on screen; game log; results and replays | view **bump** | 1–2 w |
| 13 | Accessibility: colourblind-safe seat and team palettes, UI scale, an aria-live headline on wasm | client | 3–4 d |

**Commander deserves its own note**, because the gap is much deeper than the
missing display. No commander ever enters `Zone::Command`: the gateway drops
`Deck.commander`, `SeatSpec` has no field for it, `compute_legal` never walks
the command zone, the tax counter is dead code, and every lobby game is
`Freeform` at 20 life — `FormatId` does not appear in the gateway's source at
all. The local command zone *is* drawn; there is simply never anything in it.

Two more that are cheap and embarrassing: **the deckbuilder cannot import or
export a decklist**, and there is **no sound and no "your turn" notification**,
which a slow multiplayer game needs more than it needs shaders.

---

## 4. The protocol batch

Everything here is outside the client and costs more. Grouped so it can land in
one `VIEW_VERSION` bump where possible — 9 has since been spent on
`priority_held` (§2.4), so this batch is a bump to **10**.

- **A refused action answered on the wire** — the other half of Lock A, and the
  only item here that blocks something in §0.
- **A per-seat event log.** No `GameEvent` leaves the engine process and
  `Session` exposes no `journal()`, so there is no in-game log, no post-game
  log, and no answer to "why did that die". Filtered per seat by the same rules
  as objects, because hidden information applies to a log too. Until then the
  client can diff successive views and be honest about what that can and cannot
  attribute.
- **Reveals reach other seats.** `GameEvent::Revealed` is journal-only, so a
  revealed card — public by CR 701.20a — is seen only by the asked seat.
- **The decision clock on the wire** as remaining milliseconds on
  `ChoiceRequest`, not an absolute time; clients share no clock.
- **`LossReason` per seat**, so a concession is not drawn as a death, and so a
  future rating can tell concede from timeout from disconnect.
- **Combat damage division.** The engine divides automatically in blocker
  order; under post-Foundations CR 510.1a that is a player's decision being made
  for them. A 6/6 blocked by a 2/2 and a 4/4 should be able to put all six into
  the 4/4.
- **Trigger ordering**, **split piles**, **modal min/max** (choose two, choose
  one or both), **`Pending: PartialEq`** so a re-offered question does not wipe
  a half-made selection, and **`Cancel` before costs are paid**.
- **Day/night**, and **remote seats' cosmetics** in `GameStatic`.

---

## 5. Sequencing

**First, because nothing else matters while a fetchland freezes the table.**
~~Lock A~~ **— done.** Both hosts now answer a refusal with the error *and* the
question again: `LocalHost::submit` re-reads `Session::snapshot`, and
`EngineRunner::refused` does the same over the wire. `snapshot` is read-only,
so re-asking cannot pump an AI seat's turn, and it sends the choice only to the
seat actually being awaited. Two tests hold it — one per host — and each ends
by playing the refused seat's *next* action, because asserting that a
`ChoiceRequest` came back does not prove the seat can still act.

~~Then: the zone browser and the pile chips; the "every offered option is
drawn" invariant; and `duel_flow.rs` rewritten to answer through the browser
model rather than by hand-building actions.~~ **Lock B — done.**
`baylee-client-core/src/browser.rs` is the model, and it has no `Hand` and no
`Battlefield` zone on purpose: `BoardModel` and `Browser` cover disjoint halves
of the table, which is what lets
`every_offered_object_is_drawn_somewhere` assert that each offered id is drawn
*exactly once* rather than at least once. The panel is `hud/tray.rs`, opened by
the pile chips or by `G`. The chips sit in the HUD rather than at the mat
corners as planned above, because the zone counts they replace are `TextSpan`s
inside one text entity, and a span has no layout node to click.

**Second, the common turn made fast and safe.** ~~The keymap swap~~; ~~the
`ChooseNumber` stepper and digit picks~~; ~~arm-then-act with mana abilities
exempt~~; ~~concede confirmation~~; ~~engine holds~~ and the stop preset; the
stack panel with runs.

**Third, the board made legible.** The hearth shrunk and the own battlefield
out from under the hand bar; P/T, damage and counter chips on art faces; combat
drawn — `is_selected`, assignment lines, the focus pulse and the per-defender
summary; `motion` wired to `reduce_motion`; the palette unified; the keyword
dominance table and the off-pie films; flying as elevation.

**Fourth, the rest of the client.** Text board, detail panel, badge tooltips,
the diff log; touch; the constructed face with inline symbols; departures.

**Then the protocol batch**, in one bump.

---

## 6. Refused, deliberately

- **Polyhedral dice**, growing or otherwise. Shape is not a number.
- **Scene lights, PBR, tonemapping changes.** The stage is unlit on purpose so
  that a blue card at the far end of the table is the same blue as one near the
  camera. Gloss lives in the shader, not in the scene.
- **A fourth typeface**, and a serif display face in particular.
- **Hue-mixing two keyword films into one border**, or per-keyword colours for
  flying, reach and trample. The rail is the countable answer, and the careful
  composition thinking that was asked for concludes by refusing to compose
  colours at all.
- **Independent animation rates per keyword.**
- **Drag-and-drop targeting or ordering.**
- **A second card renderer** for the browser, tray or stack.
- **Client-side rules inference** beyond arithmetic on projected public
  numbers. The layer system stays in the engine.
- **Undo.** Two stages before the wire, and a `Cancel` from the engine where
  the rules allow one.
