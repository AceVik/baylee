# Redesign proposal — a table you want to sit at

Status: proposal, to be merged into `docs/design.md` by its owner. This
document is opinionated on purpose. Every claim that rests on code names the
file and line it was read from; anything not read or run is marked
**unverified**. Nothing here is Rust.

The owner's complaints, verbatim and unsoftened, are the brief:

> Kartenbilder werden nicht geladen/angezeigt. Der Fetchland-Effekt fügt
> irgendwas in den Manapool statt ein Land aus der Bibliothek auswählen zu
> lassen. Login/Create-account Eingabefelder sollen sich wie im Browser
> anfühlen, Passwortfeld braucht visible toggle. Ausspielbare Länder beim
> Klick ausspielen. […] Der Client/Das Spiel soll sich schön anfühlen und auch
> schön aussehen.

Plus, later: the stack should preview on hover, and the preview should behave
like a speech bubble at the card it belongs to, not in the middle of the
screen.

Two of the four complaints are defects, not design, and are being diagnosed
separately. They are handed over in §0 and appear nowhere in the sequencing.

---

## 0. What is handed over, not designed

**Card images.** Verified by the coordinator: the gateway route answers 200
and images *do* load in the offline duel (seven real card faces in the hand
at 190×230 physical px). So this document designs with art present. It does
not design a "no art" mode beyond what `docs/client.md` §Images already has
(`Failure::Unresolved/Load`, retries, the constructed text face).

**The fetchland.** Flooded Strand is authored correctly
(`crates/baylee-cards/src/cards/flooded_strand.rs:28-37` — `activated!`, index
0, `mana_ability: false`). The client turns its one ability into
`ActivateManaAbility` whenever the land is *also* in `legal.mana_abilities`:

```rust
// crates/baylee-client-core/src/interaction.rs:1104-1106
if legal.mana_abilities.contains(&source) && ability_index == 0 {
    return Some(PlayerAction::ActivateManaAbility { source });
}
```

A fetchland is in `mana_abilities` exactly when something grants it one —
Chromatic Lantern (`chromatic_lantern.rs:19-27`, `GrantActivated {
mana_ability: true }` on every land you control), Urborg or Yavimaya. The
starter deck the lobby posts (`data/acceptance-decks.txt:5-30`, `Allytifact`)
holds Chromatic Lantern **and** two fetchlands, which is why the owner meets
it on turn three of the first game. The chain is
`engine/abilities.rs:242-243` (granted mana ability offered) →
`client/abilities.rs:105-110` (printed ability, index 0, calls
`interaction.activate(object, 0)`) → the guard above →
`engine/actions.rs:868-897` (`ActivateManaAbility` takes the granted slot and
adds a colour). **Not verified live.** The regression test shape: a Flooded
Strand on the battlefield, `LegalActions { mana_abilities: vec![id],
abilities: vec![(id, 0)], .. }`, `Interaction::activate(id, 0)` must answer
`ActivateAbility { ability_index: 0 }`. `abilities.rs` tests never cover index
0 together with `mana_abilities`. The fix is to route by the *offer* (is
`(id, 0)` in `abilities`?) and never by the index's value; the design in §7
assumes that fix.

---

## 1. Design intent

The client should feel like sitting down at a good table: the felt is calm,
the cards are the brightest thing on it, and the one question you must answer
is where your eyes already are. Everything the interface says is one of three
claims, and each has one look:

- **What the game says** — engine offers (`LegalActions`, a `Pending`). Candle
  light, bright.
- **What this client offers to do for you** — tapping lands, floating mana,
  sorting a search. Candle light, dim.
- **What a thing is** — keywords, counters, P/T. Steady sheaths, plates,
  chips (`docs/design.md` §1.3-1.4, unchanged).

`docs/design.md` §1.2 already decided "ACCENT teal goes; offers become candle
at two energies". This document takes that decision as settled and applies it
everywhere it was not yet applied. Two drifts show why it matters: the hand's
offered glow is documented as gold and drawn teal
(`crates/baylee-client/src/hud/hand.rs:88-97` vs `hud.rs:426`), the tray does
the same (`hud/tray.rs:220-225`), and `docs/keyboard-map.md` §Arming says an
armed hand card lifts to the height a chosen card sits at while the code
raises it a flat 8 px (`hand.rs:167`, `ARMED_RAISE`) and a selected hand
card does not lift at all. Two hues and two lift grammars for one claim is
how a player learns to distrust the display.

Principles that every section below obeys:

1. **The engine decides, the client shows.** "Playable" is `LegalActions`,
   never a client rule (`CLAUDE.md`, "The engine advances only through
   choices"). Where a wanted feature needs a fact the engine does not
   project, the section names the protocol item rather than inferring it.
2. **Decisions in `baylee-client-core`, drawing in `baylee-client`.** Each
   section says which side each piece lands on and what its headless test
   looks like.
3. **One hue per claim, one lift per state, one material per surface.**
   Candle for offers, parchment for a sheet you read, panel for a dialog you
   work in, felt for the game.
4. **A must-answer question is the most prominent thing on screen.** Today it
   is the least (§2).
5. **No new dependency on the player's memory.** A number key, a colour, a
   position — each is reinforced by text or shape.

---

## 2. The frame: table, HUD, question, rail, empty board

Coordinator's frame (offline duel, 1728×1052 logical): a full-height column
of 24 three-letter pills down the right edge; two seat plaques top-left; draw
and concede top-right; two empty mats and felt covering ~75% of the frame on
turn 1; zone piles clipped by the window edge; a "Mana pool —" label bottom
left; the mulligan question as small text low right; and a hand of seven
real cards. Verified with it: all four automation rules are off by default
(`prefs.rs:489-501`, `AutoRules` derives `Default`) and the client stops at
every priority grant including ones with empty `legal` — fourteen stops in a
row, most offering only "pass".

### 2.1 The question belongs on the table, not in a corner

Today the prompt bar sits bottom-right above the hand bar, right of the rail
(`hud/overlay.rs:365-803`; headline 18 px in ACCENT or MUTED). Its rows are a
link note, a headline, an error, a hint, combat lines, a number stepper,
answer buttons, a subtype box, an indexed chooser, the armed row and the
ability chooser — eleven concerns in one column that is placed where nothing
else is happening.

**Proposal: a prompt slip.** The `Pending` the player must answer becomes a
single panel centred horizontally over the near edge of the local mat, just
above the hand bar: headline 22 px INK on `PANEL_LIT`, answer buttons as real
44-px buttons in one row, the min/max tally when there is one. It is the only
thing in the frame drawn at that weight, and it is where the eye already sits
(between the hand and the mat). It appears with the §1.6 arrival motion and
does not blink; urgency is carried by position and weight, not animation.
When no question is pending the slip is not drawn — a priority window with
nothing to do shows only the pass affordance (§2.3).

What moves *out* of the prompt column: the link note and automation state go
to the top strip beside draw/concede; the hint line goes under the slip in
MUTED at 13 px and only when there is one; the ability chooser becomes the
parchment sheet at the permanent (§7); the armed row becomes the ledger
(§5). The `ChooseNumber` stepper stays on the slip.

Model: nothing new in core — the slip is the existing `Prompt`
(`interaction.rs:73-167`) drawn once, in one place. Test (renderer): the
overlay test asserts the slip node is a child of the root when `Prompt` is
anything but priority, and absent otherwise.

### 2.2 The rail earns its space as a clock, or not at all

`RAIL_W 56` (`hud/rail.rs:27`) is subtracted from the felt permanently
(`table.rs:345-365`, `Canvas::hud`), so every screen gives up a 56-px column
for a control panel whose 24 rows carry a state that is off by default. It
reads as bolted-on because it *is* a settings panel drawn in the game frame.

**Proposal: the turn track.** The rail becomes a 24-px timeline on the right
edge of the felt, over the felt (still subtracted from `Canvas` so no card
hides under it, but half the width). Each step is a dot; the current step is
a candle-bright dot with its three-letter label beside it; the active
player's twelve steps are drawn at full size and the other player's as a
compact band with one label (OPPONENT / YOU), because a player needs to know
where the turn is, not read a table. The turn number sits at the top of the
track. Stops and holds are *pins* on a dot (a small gilt mark), set from the
same place they are today (⇧W/⇧S, F6/F7, `settingsui.rs`) and from a hover
that widens the track to today's 56 px with all labels for as long as the
pointer is on it. `RailPreset` stays. Nothing about `PhaseOrders` changes;
this is `hud/rail.rs::row_visual` and `spawn_phase_rail` only.

Why it earns the space: a track whose one lit dot moves each step tells the
player *where* they are at a glance; a column of 24 equal pills does not.

### 2.3 Stop only where there is something to do

Fourteen stops with nothing to press is the frame's real defect. The prefs
comment (`prefs.rs:496-500`) already calls this "the common case of a player
pressing pass forty times a turn". **No reason for off-by-default is
documented** — not in `prefs.rs`, not in `docs/client.md` or
`docs/keyboard-map.md` §Automation (grepped for the field and the phrase);
`docs/design.md:877` frames the rule as "nothing to do here when nothing is
happening", which is the argument *for* on. For a first game the least
surprising thing is a client that does not ask a question with one answer.

**Proposal:** `pass_when_nothing_to_do` defaults **on**. `auto_answer`
(`automation.rs:409`) already passes only when `legal` is empty and never
over an opposing stack, so the safety argument holds. Consequences to state
plainly: `AutoRules` derives `Default` and is `#[serde(default)]`, so a stored
blob without the field flips to on for existing accounts (acceptable — no
existing account has it stored *off* on purpose, unverified); and every
`duel_flow` test that counts priority stops changes count and must be
updated in the same change. The other three rules stay off; skipping
opponent turns is a preference, not a fix.

For the stops that remain, the pass affordance is a single candle-dim button
"Pass" at the slip's position — the slip's smallest form — so that "pass" and
"answer" are the same place.

### 2.4 The empty board should look like a game about to start

On turn 1 the frame is 75% felt because the mats are empty and the plaques,
piles and pool are scattered to the edges. Changes, all renderer-side:

- **Seat plaques onto the mats.** Name, life, hand and library counts sit on
  the near rim of each seat's own mat (`tabletop.rs` already colours the rim
  by seat and mood). Mats do not move, so their screen position is projected
  when the camera changes, not per frame — this is not the per-card UI
  projection `docs/design.md` §1.4 refuses. The top-left plaques go.
- **Piles inside the frame.** The photographed piles are clipped by the
  window edge. `CameraRig::home` fits `TableLayout::extent`
  (`layout.rs`, `extent`), and piles stand `PILE_REACH` beyond the mat
  (`layout.rs:145-157`). **Unverified** whether `extent` includes the pile
  strip; if it does not, that is the whole bug, and `camera_tests` gets a case
  that projects every pile centre and asserts it inside the canvas.
- **The pool bar only when it holds something.** "Mana pool —" as a permanent
  label is a form with nothing in it. The pool row (`overlay.rs:805-905`)
  appears when `Floating` is non-empty or the ledger (§5) is open.
- **The mat's lanes as faint bands** (already generated) and the hearth glow
  under the local mat carry the "your side" reading; the medallion stays. No
  decorative filler — an empty table at a card game is empty, and that is
  fine once the question, the track and the plaques are where they belong.

---

## 3. The hand and what "playable" looks like

The engine already answers timing, mana and the land drop:
`compute_legal` (`crates/baylee-engine/src/engine/abilities.rs:62-…`) sets
`sorcery_timing = main phase && active == player && stack empty`, pushes a
land only if `sorcery_timing && lands_played_this_turn == 0`, and lists
`castable` only for cards whose cost the *floating* pool can pay
(`choice.rs`, "timing + mana verified"). The owner's "by timing / mana / land
slot" is therefore a display of the offer, not a client rule.

Three states, as `Openings { playable, reachable, activatable }`
(`crates/baylee-client-core/src/board.rs:579-604`):

| State | Source | Look |
|---|---|---|
| `playable` | `legal.lands ∪ legal.castable` | candle bright, lifted 6 px, border still |
| `reachable` | `manaplan::plan` over engine-listed sources (`lib.rs:1045-1078`) | candle dim, no lift |
| `activatable` | `legal.mana_abilities ∪ legal.abilities` (permanents) | the travelling warm light, unchanged |

Everything else in hand is drawn plain — not dimmed, not greyed. A card that
is not playable is still a card you are reading.

**Hue.** REACHABLE indigo (`hud.rs:437`) and ACCENT teal (`hud.rs:426`) leave
the hand; `WILL_TAP` on lands (§5) also becomes candle dim, because it is the
same claim ("the client will do this") on a different object. The gold ring
for `ARMED` (`card_ui.wgsl:229-234`) stays as the "accepted" state where
arming survives (§4).

**Order.** `board.rs:705-711` sorts the hand playable → reachable → mana value
→ name, so the hand re-sorts at every priority. `docs/design.md` §2.3 keeps
the re-sort and only pins the card being aimed at; **this proposal reverses
§2.3**: the hand keeps the order `view.hand` arrives in (**unverified**
whether that is draw order — `gamehost/view.rs` was not read for it; if it
is not, the client sorts once by arrival and appends) and shows playability
by lift and light only. Sorting was carrying information the glow now
carries, and a stable hand is one the player can learn. Test in `board.rs`:
two views differing only in `castable` produce the same hand order.

**The timing hole in `reachable`.** `reachable` (`lib.rs:1045-1078`) filters
hand cards not in `castable`/`lands` and keeps any whose printed cost the
planner can cover; nothing there knows whether it is your main phase. A
sorcery in hand lights dim on the opponent's turn, and a run then aborts with
`"the mana is up but the spell is not castable"` — an English literal
(`lib.rs:831`), not a `Phrase`, which is a second finding. The client cannot
gate this without rules inference (`docs/design.md` §6 refuses it). The
honest fix is a list from the engine, the shape every other field of
`LegalActions` has: `castable_if_paid: Vec<ObjectId>` — the cards that pass
timing (sorcery speed, instant, flash, whatever the card's own rules say)
and fail only `can_pay`. `compute_legal` computes exactly that set on the
way to `castable` and throws the mana-short half away. A single
`sorcery_timing: bool` would be the wrong shape: it leaves the client
deciding "is this an instant, does it have flash" from types and keywords,
which is the inference §12 refuses. With the list, `reachable` becomes the
planner over `castable_if_paid \ castable`. Until it lands, `reachable` is
what it is, the abort becomes a phrase (appendix), and §5's ledger opens
only for `castable`/`reachable` and re-checks `castable` after every tap,
which the run already does.

**The MDFC exception.** `Interaction::play_card` (`interaction.rs:1088-1098`)
checks `lands` before `castable`. A modal double-faced card with a spell
front and a land back is in *both* lists in your main phase; a one-click
(§4) would resolve it to `PlayLand` unconditionally and the front would be
unreachable by mouse. Rule: a card in both lists is **not** one-click; it
opens a two-row sheet (§7's parchment, "Play as a land" / "Cast {name}")
and nothing fires until a row is chosen. Test: `lands = castable = [id]`,
`play_card(id)` returns `None` and a new `Interaction::play_choices(id)`
returns both.

---

## 4. Land play as one click, and where the line is

Today a click on a land arms `Deed::Play` (`lib.rs:154-165`), a second click
sends, `Esc` disarms (`docs/keyboard-map.md` §Arming). The owner wants the
land to play on the click. That is right, and the reason it is right has to
be written down so the next exemption is argued the same way.

**The line.** An action is one-click when *all* of these hold:

1. It has no cost — nothing is tapped, sacrificed, paid or discarded.
2. It uses no stack, so there is no window in which the player would have
   wanted to respond first.
3. It changes no other object's state (nothing else leaves a zone, no
   counter moves, no life changes).
4. Its worst case is bounded by one turn: the wrong land in the one land drop.

Playing a land is a special action with no cost and no stack
(`casting.rs:254`, "Plays a land (special action, no stack)"). A mana ability
fails (1) but is already exempt on the different ground that floating mana
is the cheap mistake (`docs/client.md`, `AbilityOption::mana`); that exemption
stays and is not widened. A sacrifice fails (1) and (3). A spell fails (1)
and (2) — and with §5 casting is a multi-step flow ending in its own Send,
so `Deed::Play` for spells is *replaced* by the ledger rather than kept as a
second arming layer. `Deed::Ability` and `Deed::Run` keep two-stage.

**Mitigating the one real risk** — the wrong land — without a confirmation:
the click must be on a lit card (the 6-px lift and candle border, §3), the
hover grace (`input.rs:1268-1287`) already prevents a click on a card that
just slid under the pointer, and the hand no longer re-sorts (§3), so the
card you aimed at is the card that plays. A land not in `legal.lands` does
nothing on click and the slip's hint says why: "Land drop used this turn"
when `lands_played_this_turn > 0` is not projected — **unverified** whether
the view carries it; if not, the hint is the generic "Not playable now".

**Touch.** Every mitigation above is a hover fact, and a phone has no hover
(`docs/design.md` §2.6). On phone metrics the first tap on a land does what
the hover does on a desktop — lifts and lights it — and the second tap plays
it. That is not a confirmation dialog; it is the pointing step touch lacks,
and it is the same two-tap grammar the hand's preview already needs there.
On desktop and tablet with a pointer the land plays on the click.

**Model change.** `Interaction::play_card` stays; `Duel::submit` for a land
sends `PlayLand` directly instead of arming (`lib.rs:325`, `input.rs:97-140`
`activate_card`). Test in `duel_flow`: click a lit land → outbox holds
`PlayLand`, `armed` is `None`. Click a land not offered → outbox empty, no
error. Keyboard: `Primary` (Enter) on a cursor-selected land does the same.

---

## 5. Manual mana payment — the biggest new surface

### 5.1 What the engine does today, and the conflict

The engine pays from the pool itself: `mana_pay::pay` (`mana_pay.rs:105`)
spends coloured pips, then hybrid, then generic via `pay_any(pool,
&ManaColor::ALL, n)` (`:177`) in a fixed colour order. `CastSpell` carries
only `{ card }`; there is no `PayMana`, no `Cancel`, and the header of
`mana_pay.rs` says the "full payment-plan solver … `ChoiceRequest::PayMana`
is M2" — it does not exist. The cast wizard (`cast_wizard.rs:330-420`) goes
`CastSpell → ChooseCastMode → ChooseNumber (X, 0..50) → targets`, and a
payment failure mid-wizard fizzles cleanly.

So the owner's "choose how to pay from the pool" is **not expressible on the
wire**. The client can choose which *sources* to tap — each an engine-offered
`ActivateManaAbility`/`ActivateAbility`, one per round trip, as `ManaRun`
already does (`lib.rs:753-856`) — but once mana floats, the engine's greedy
order decides which pip it pays. With `{R}{R}` floating and a `{1}{R}` spell,
the player cannot keep one `{R}` for the next spell by choice; `pay_any`
takes whatever comes first in `ManaColor::ALL`.

**Closest buildable thing:** the client makes sure the pool holds *exactly*
the cost before sending, by tapping what the player points at. Then greedy
and manual agree, because there is nothing to disagree about. Where the pool
already holds more than the cost (mana left over from earlier), the ledger
shows the engine's assignment honestly instead of pretending it can steer.
The engine item that lifts this is small and named in §11: `CastSpell {
card, pay: Option<PoolPlan> }`, a per-pip assignment from the pool the engine
validates like any other answer.

### 5.2 The model, in core

`crates/baylee-client-core/src/payment.rs`, renderer-free, tested headless:

```
Ledger {
  card: ObjectId,
  cost: ManaCost,                  // from the registry via hand_cost (manasources.rs:162)
  pips: Vec<Pip { need: PipKind, covered_by: Option<Cover> }>,
  auto: bool,                      // the Auto button; default off
}
Cover = Pool(ManaColor) | Tap { source: ObjectId, tap: Tap, color: ManaColor }
```

- `Ledger::open(card, cost, pool, sources)` lays the pips out in printed
  order and pre-covers from the pool what the engine would take, by running
  the engine's own `baylee_engine::mana_pay::pay(&mut pool_copy, &cost)`
  (`mana_pay.rs:104`, mutates the pool and answers whether it paid) on a
  copy and diffing the pool before and after. `mana_pay` is `pub mod`
  (`engine/lib.rs:40`) and `baylee-client-core` already links
  `baylee-engine` (`client-core/Cargo.toml:14`). One function, two callers:
  what the ledger shows as spent is what the engine will spend. The diff is
  per *colour*, not per pip — `pay` reports nothing finer — so the ledger
  shows "from the pool: {R}{R}" as a row, not a pip-by-pip assignment; the
  per-pip display is gated on the engine item in 5.1. No relocation to
  `baylee-core` needed.
- `Ledger::tap(source)` adds a `Tap` cover to the first pip the source's
  colours can pay (coloured pips first, generic last — the planner's rule,
  `manaplan.rs`), and answers the engine action to send. It is refused when
  the source is not in the current `LegalActions` (the `ManaRun` rule).
- `Ledger::auto(pool, sources)` runs `manaplan::plan` for the uncovered pips
  and queues the steps; it is a button, off by default, as the owner asked.
- `Ledger::ready(legal)` is true iff `legal.castable` contains the card —
  the engine's word, never the ledger's arithmetic. The Send button reads
  this and nothing else.
- `Ledger::x` for `{X}` costs: a stepper on the ledger sets the intended X
  *before* tapping, so the pips expand to `X` generic; the engine then asks
  `ChooseNumber` after `CastSpell`, bounded by the pool, and the client
  answers with the ledger's X if it is still within bounds. `manaplan`
  refuses `{X}` today, so manual tapping is the only route for X spells —
  which is exactly the case the owner wants to control by hand.
- Hybrid pips take a click on the pip to switch the half; Phyrexian pips are
  never paid with life by the client (the planner's rule); whether the
  engine's `pay` accepts life for Phyrexian is **unverified** and the ledger
  draws the pip as a mana pip only.
- Cancel: closes the ledger. Mana already floated stays in the pool — there
  is no untap — and the ledger says so once ("Floating mana stays in your
  pool"). This is the same consequence a `ManaRun` abort has today
  (`lib.rs:781-787`), now stated instead of silent.

### 5.3 The surface

The ledger is a bubble (§8's placement rule) anchored to the card being
cast, on the hand side. It shows the cost as pips (`manaui.rs`, the OFL Mana
font — pips are drawn, never typed), each pip filled when covered, the source
it is covered by named under it in 12 px, a **read-only** row of what the
pool will pay (the `pay` diff from 5.2), then `Auto` · `Cast` · `Cancel`.
The pool chips are not clickable until `CastSpell.pay` exists: a click that
changed nothing the engine does would be the false affordance the deck
builder's "if the button is live, the action succeeds" rule forbids. When
the engine item lands, the same chips become the per-pip picker and nothing
else on the ledger moves. While the ledger is open:

- Lands and sources on the table that can still pay an open pip carry the
  candle-dim border (today's `WILL_TAP`); a tapped-for-this-ledger source
  shows the pip colour it paid as a small chip at its plate corner for the
  ledger's lifetime.
- A click on such a source sends its mana ability at once (one tap, the
  existing exemption) and the ledger fills the pip when the next view shows
  the mana floating. The round trip is the existing `ManaRun` machinery, one
  step per click.
- The hand card being cast is lifted to the chosen height, and nothing else
  in the hand is lit — one question at a time.
- `Esc` cancels; `Enter` sends when `ready`; digits are the sheet's (§7) and
  do nothing here.

Test in `payment.rs`: Forest + Mountain, cost `{R}{G}`; `tap(forest)` covers
`{G}`; `ready` is false until a `LegalActions` with the card in `castable`
arrives; `tap(mountain)` then covers `{R}`; with both floating `ready` is
true. A second test: pool `{R}{R}`, cost `{1}{R}` — the ledger shows both pips
covered from the pool and *no* tap targets, because the pool already pays.

---

## 6. Search and browse — the dialog

`Browser` (`crates/baylee-client-core/src/browser.rs`) already has the model
half: `BrowseZone`, `BrowseRow { selectable, selected, place }`, `follow`
(`:187`) opens it on `ChooseCards` with `ChoicePrompt::SearchLibrary`,
`rows` (`:251`) filters by name-contains. `Interaction::toggle`
(`interaction.rs:636-716`) already has checkbox semantics — `Added`, `Removed`,
`Rejected`, `Full` at `max` — and `can_confirm` (`:981`) needs `min`. What is
missing is sort, a wired filter (`set_filter`, `:176`, has no non-test
caller), scrolling and the tally.

**Model changes (`browser.rs`).**
- `SortKey { Name, ManaValue, Type, Place }` + `SortDir`; `rows` sorts after
  filtering. Keys read `PublicObject` fields the view already projects
  (`gamehost/view.rs:158`, `looking_at` at `:339`): name, mana value, types.
  No rules — arithmetic on projected numbers, which §6 allows.
- `filter` is fed from a text field that goes through `softkeys.rs` like the
  lobby's (the tray's filter line today is read-only and bypasses softkeys,
  `tray.rs:136-155`, so on wasm it cannot be typed at all). The field is a
  `TextBuffer` (§9).
- `tally()` = (selected, min, max) for the header.

**Panel changes (`hud/tray.rs`).**
- The panel scrolls. `tray.rs:45-64` sets `max_height 70%` and no overflow;
  `Overflow::scroll_y()` + `ScrollPosition` with the wheel driver that
  already exists for the lobby's lists (`lobby/systems.rs:647-732`,
  `Scrollable`) is the whole change.
- Rows get a checkbox at the left (FA square / check-square in the icon font),
  the card art at `TRAY_CARD_W`, name, mana pips, type line, and the `place`
  badge. Selected rows: candle-bright checkbox and border, no teal.
- Header: zone tabs, then the sort control (one button cycling
  Name/Cost/Type, one direction toggle), then the search field with
  placeholder; the tally "3 of up to 4 chosen" in the same row, using the
  existing `ChooseUpTo`/`ChooseExactly`/`ChooseBetween` phrases plus the
  appendix's `SelectedOf`.
- Footer: `Confirm` (lit only when `can_confirm`), and `Cancel` only when
  `min == 0` — there is no `Cancel` action in the engine, so a `min > 0`
  search has no way out and the dialog must not pretend it has.
- Opens automatically on `SearchLibrary` (already), on a graveyard/exile pile
  click (`input.rs:1016-1030`) for browsing, and stays a panel (dark
  `PANEL_LIT`), not parchment: parchment is for the sheet you read and pick
  from (§7), a dialog is a place you work. One material per surface.

Keyboard: Tab moves through rows, Space toggles, Enter confirms when
`can_confirm`, typing goes to the search field while it has focus.

Test in `browser.rs`: fifteen rows, filter "isl", sort ManaValue desc →
row order asserted; `toggle` past `max` answers `Full` and the row stays
unselected.

---

## 7. The ability sheet, in parchment

Today several abilities open a chooser row on the prompt bar
(`overlay.rs:751-801`), keyboard-owned, no digits, no cost drawn. The owner
wants: click a permanent, see its activatable effects in a parchment design,
pick by mouse or number key.

**Material.** `PARCHMENT (0.88, 0.83, 0.69)` exists in
`card_common.wgsl:569` and is reused: `palette::PARCHMENT` in `hud.rs` takes
the same value, `INK` (`card_common.wgsl:91`) for text and `SEPIA` (`:570`)
for the hairline rule, with a test that reads the WGSL and fails when the
palette drifts — the same guard the rail and plate already use. Parchment is
**opaque**; a translucent parchment over dark felt is grey. Text is Inter
(§6 refuses a serif); a 0.5 px `LetterSpacing` on the title row gives the
small-caps feel without a new face. Contrast: INK on PARCHMENT is well above
4.5:1 (unverified to the digit; the test asserts it both ways).

**Placement.** The sheet is anchored to the permanent and follows it: its
node's `left/top` are reprojected each frame from the card's live
`Transform`, exactly as `combatlines.rs` does for lines, because a card
glides (`Motion`) and a sheet anchored to `Motion::target` arrives before the
card does. This is stated as a decision: `docs/design.md` §1.4 refuses
projecting UI onto every card per frame; one sheet is not that. The sheet
stands above the card (toward the table centre for the local seat) and flips
below when there is no room above; horizontally it is clamped to the canvas
(§8's `bubble::place`). A tail points at the card.

**Content.** Rows are `AbilityOption`s (`client/abilities.rs:27-41`) built
from the *current* `LegalActions` on every press (the existing rebuild rule,
`docs/client.md`), in the order the engine offers them: mana abilities first,
each with its pips drawn ("Tap for {G}" is a pip, not a string), then
printed abilities with their cost pips and label, then granted ones. Each row
carries its digit at the left, 1–9, in a small gilt roundel.

**Digits.** 1–9 are unbound in `Keymap::standard` (`prefs.rs:333-371`;
`number_keys` in `input.rs:399` reads raw digits only under `ChooseNumber`).
A new action group `Pick1..Pick9` binds them; the sheet only opens under
`Priority`, `ChooseNumber` never coincides with it, so there is no clash.
Digits go to the sheet when it is open and to nothing when it is not. More
than nine rows is real only for a granted-mana land with printed abilities
(five colours plus two or three printed); the sheet shows nine and a tenth
row "More…" on `0` that pages. Picking a mana row sends at once (the
exemption); picking a printed row *arms* it (today's rule, `Deed::Ability`),
the row turns gold, and a second press of the same digit or a click on the
row sends. `Esc`, a click outside, or the engine withdrawing the offer
dismisses the sheet; an armed row that the next `LegalActions` no longer
offers disarms with the existing STALE note (`input.rs:199-252`).

**Single ability.** One option still activates on the click that found it —
unless it is not a mana ability, in which case the click arms and the sheet
shows the one row so the player sees what the second click will do.

**Touch.** Rows are 44 px on phone metrics; the digit roundel is decoration
there.

Test (core): `abilities::options` ordering and the `(digit → option)` map;
(renderer) `ability_menu_keys` pressing `Pick3` sends the third option by
position after a rebuild.

---

## 8. The preview as a bubble, and the stack you can hover

### 8.1 Two requests, one fix

The stack panel spawns `Pickable::IGNORE` at six places
(`hud/stack.rs:61,150,178,247,322,393`), so no stack entry can be hovered or
clicked. `docs/design.md` §2.2 names the same fact as why counterspells and
reanimation "cannot be aimed at all". Making stack entries pickable is one
change that serves both: hover raises the preview, click is a target under
`ChooseTargets`. They are the same fix seen from two sides and are scheduled
as one. A stack entry becomes `HoverSource::Stack` (`input.rs:1301`, beside
`Hand`/`Table`), reports the spell's `ObjectId` (which `PlayerView::object`
resolves — the hand-first comment in `hand.rs` says the stack is "one
lookup"), and `the_click` (`input.rs:634`) treats it as any other object.

### 8.2 Finishing the bubble the code already names

`preview_anchor` (`hand.rs:324-378`) is documented as returning "where its
anchor (the bubble's tail target) sits horizontally": a hand card answers
`Some(x)`, everything else — lanes, piles, command zone — answers `None`,
which means screen centre. The tail exists (`overlay.rs:1148-1164`): a
`CARET_DOWN` glyph at `anchor − 9`, bottom `HAND_BAR_H + 2` — it always
points *down at the hand bar*, even for a permanent on the table. The panel
is always at the bottom (`overlay.rs:1061`). So a table hover shows a bubble
in the middle of the screen with a tail pointing at nothing.

**The anchor becomes a point.** `PreviewAnchor { at: Vec2, from: Side }`
where `Side` is which edge of the source the bubble grows away from. Every
source supplies one:

| Source | Anchor |
|---|---|
| hand card | top centre of its strip slot (x as today, y = hand bar top) |
| permanent | the card's live world centre projected through the camera, each frame |
| pile top card | same projection |
| command zone card | its node's top centre |
| stack entry | the entry node's right centre (the bubble opens toward the felt) |
| browser row | the row's right centre |

**Placement rule in core.** `crates/baylee-client-core/src/bubble.rs`:
`place(anchor, size, canvas) -> Placed { rect, tail: (Side, f32) }`. Prefer
the side the anchor names; if the rect would cross `canvas.right`, mirror to
the left; if it would cross `canvas.top`, flip below; clamp the remainder;
the tail slides along the facing edge to stay on the anchor and never nearer
than one radius to a corner. The bubble never covers its own anchor and never
covers the prompt slip (§2.1) — the slip's rect is passed as an exclusion.
Pure arithmetic, six tests (four edges, both flips).

**Look.** `PANEL_LIT` body, the existing shadow, a 1-px gilt hairline, radius
from `preview_radius`. The tail is a 12-px rotated square in the same body
colour with the same hairline, drawn as a node rather than a glyph so it
works on all four sides. Over felt the hairline separates it; over panels the
shadow does. The preview is a description of the card and is the *only* place
rules text is read at size, so its scale control (`PreviewResize`) stays.

**Surviving the rebuild.** In the duel the whole overlay respawns whenever
`hovered` changes (`overlay.rs:166,191`) — this is not the deck builder's
epoch entity (`lobby/preview.rs:1-105`). A bubble that must follow a gliding
card cannot be rebuilt per frame, so: the bubble is spawned by the overlay
rebuild as today, tagged `PreviewBubble { anchor_source }`, and a small
`Present`-stage system updates only its `left/top` from the live anchor —
the same shape as `combatlines`. Nothing else about it changes between
hovers, so the rebuild stays where it is.

**Not doing:** a preview that follows the pointer (it would hide what you
are reading), a delay before showing (the hover grace already exists), or a
second card renderer for the preview (`docs/design.md` §6).

---

## 9. Lobby forms that feel like a browser

What exists: append-only text (`lobby.rs:837` `type_char`, `:846`
`backspace`), a caret drawn as the glyph `▏` appended to the value
(`lobby/ui.rs:1526-1530`), masking done by the caller with `"•".repeat`
(`:398,:786`), single focus + `focus_epoch`, forward-only Tab
(`cycle_focus`, `:782-799`), a border in ACCENT when focused, no selection,
no arrows, no paste on native, no error state, `text_field` and `text_box`
as duplicates (`ui.rs:1345-1409, 1496-1563`). On wasm, `softkeys.rs` keeps an
invisible focusable `<input>` (`:168-179` open, `:182-194` close) and diffs
its value per frame (`:196-217`), with `type=password` and
`autocomplete=current-password` for every password field (`:163-167`).

**Principle:** the browser's `<input>` is the authority on wasm and must stay
so — it is what buys autofill, IME, the phone keyboard and paste. Native gets
a real text buffer. The *same* model drives both, so the lobby's headless
tests cover both.

**Model (`crates/baylee-client-core/src/textbuf.rs`).** `TextBuffer { text,
cursor, anchor: Option<usize> }` with `insert(&str)`, `delete_back`,
`delete_forward`, `move(Char|Word|Line, Left|Right, select: bool)`,
`select_all`, `selection() -> Option<Range>`, `replace_selection`. `Lobby`
and `DeckBuilder` fields (`lobby.rs:44-59`, `deckbuilder.rs:564-578`) hold a
`TextBuffer` instead of a `String`, and `set_field` (`:650`) becomes "the
`<input>` said the value is now X, cursor at Y" — softkeys reads
`selectionStart`/`selectionEnd` too, so the caret drawn on the canvas is the
browser's caret. One catch: the HTML spec defines `selectionStart` only for
`text`, `search`, `url`, `tel` and `password`; reading it on `type=email`
throws (**unverified in this repo**, spec-known), and `softkeys.rs:163` uses
`type=email`. The email field therefore becomes `type=text` with
`inputmode=email` and `autocomplete=username`, which keeps the phone
keyboard and autofill and gains the caret. `cycle_focus` takes a direction
(⇧Tab). `submit` unchanged.

**Native input.** `lobby/systems.rs::keyboard` (`:198-313`) grows arrows,
Home/End, Shift-selection, ⌘/Ctrl+A/C/V/X and Delete. Clipboard on native
needs a dependency or Bevy's own text entry: Bevy 0.19 ships `EditableText`
with IME, selection and clipboard (per the pinned skill notes; whether it is
reachable under this workspace's `default-features = false` set with
`bevy_ui_widgets` on — `Cargo.toml:101` — is **unverified**). Decision: try
`EditableText` first behind the same `TextBuffer` interface; if it does not
compile under the feature set, `arboard` on native only (never on wasm) for
the clipboard. Either way the model is `TextBuffer` and the tests do not
change.

**Look and feel.**
- Caret: a 1-px INK bar at the cursor, blinking at 530 ms, steady under
  `reduce_motion`. Never a glyph in the string.
- Selection: a candle-dim fill behind the selected run.
- Focus: a 2-px candle ring, not the teal border. Click sets focus (exists)
  and the caret at the nearest glyph boundary (`TextLayoutInfo` glyph
  positions — **unverified** that Bevy 0.19 exposes per-glyph rects in UI
  text; if not, click-to-caret is end-of-text and arrows do the rest).
- Placeholder in MUTED when empty; a label above; an error line below in
  DANGER with the ring in DANGER — the gateway's `{"error":…}` stays in the
  gateway's words (`docs/client.md`).
- Password: `FieldKind::Password { visible: bool }`. An eye button at the
  field's right toggles it (FA `eye`/`eye-slash`, with the phrase as its
  tooltip). On wasm the toggle flips `type=password` ↔ `type=text` on the same
  `<input>`; `autocomplete` becomes `new-password` on the create-account
  form and `current-password` on sign-in (today it is always
  `current-password`, `softkeys.rs:165-167`). Masking moves from the caller
  into `text_field`.
- Tab order: Email → Display name (create only) → Password → primary button;
  ⇧Tab reverses; Enter submits from any field (exists, `keydown` listener
  `softkeys.rs:139-150`).
- Paste: free on wasm through the `<input>`; native per above.
- One `text_field`; `text_box` goes.
- The tray's search field (§6) and the subtype box (`overlay.rs:606-638`)
  open a softkeys field too, or they cannot be typed into on wasm.

Test in `textbuf.rs`: insert/move/select/replace; in `lobby.rs`: ⇧Tab
order, and that a value pushed by `set_field` with a cursor lands the caret
there.

---

## 10. Choosing a target

This closes `docs/observed-faults.md` entry 20 — "target selection needs a
real design" — for declaring attackers and blockers and for every question
the engine answers with a list of things to click on
(`Pending::ChooseTargets`, `ChooseObjects`, convoke and delve). A mode is not
one of those: it is a list on the slip, not a pointing, and it belongs to the
ability sheet's `Pick1..9` in §7. It is written against the prompt slip of
§2.1; every step lands in today's prompt bar.

### 10.1 One model: point this at that

A pick is a pair — a thing, and what it is pointed at. An attacker is pointed
at a defender, a blocker at an attacker, a target at the spell or ability that
wants it. One state (`Interaction`), one drawing (`combat::Line`), one pair of
keys for walking the second half (`CombatFocusNext`/`Prev`, whose names stay
because a keymap is stored per account). The three differ only in **who
supplies the second half**, and each difference is a fact the engine gives us.

Attacking, the player supplies both halves: the engine lists the defenders
(CR 508.1a) and the client carries a focus so the next creature tapped goes
against the one aimed at. Blocking, the second half is somebody else's
standing declaration, already on the felt from `view.combat`, and it is the
one case where aiming changes what is offered: `BlockOption` is per blocker,
so while the focus is on a flier the ground is not lit. That is what "never
click and get refused" costs here, and it is cheap — the offer is re-read from
the focus. Casting, the second half is fixed — the spell — so a pick collapses
to a set, and the list can hold seats (`player_options`), as an attack
does through `Defender::Player`. Nothing else differs.

### 10.2 What is legal is lit; what is not is plain

The perimeter light (channel C3, `glow::ACTIVATABLE`) means "the engine offers
a click here". It is the only thing on the table that travels, and the border
register has no spare bit under `MARK_SHIFT`. It needs none.
`Interaction::legal_actions()` is `None` outside `Mode::Priority`, so the
moment a question stands nothing is activatable and the light goes dark —
exactly when the offer needs it. Same claim, same light: while a question
stands, `Offer::on` reads `is_selectable` over the group's `members` instead
of `LegalActions`, and the warm chase runs round every offered card in hand,
on the felt and in the tray. Nothing can be confused, because the two readers
are never lit at once. A counted stack lights when **any** member is offered,
where `CardGroup::activatable` demands all — the click rule in §10.3 is what
makes "any" honest. A stack stays a stack while it is picked from;
`Individual::Targeted` is for spells already sent, and a group forking under
the pointer mid-question is the fault `docs/design.md` §2.3 names.

A seat is offered in both its bodies: its tab wears the hand bar's offer
treatment, and on the felt a still candle ring stands at its `seat_anchor` —
the focus ring's geometry, held still and dim, so aiming at a seat brightens
something already there. It cannot be read as the focus ring: the focus is
gold, breathes, and there is exactly one.

What is not offered is drawn as it always is — not dimmed, not greyed: the
lit set already says it, and a felt that goes dark under every spell makes
the whole table flinch twice a turn. A click on it changes nothing and puts
one line on the slip, `NotOffered`; hover, preview and an unoffered pile top
work as they always did. A pile chip wears the offer when any card inside it
is offered — delve and a reanimation target live nowhere else on the felt —
and the browser it opens lights them one by one.
`every_offered_object_is_drawn_somewhere` grows one clause — drawn *as
offered* — and reads `is_selectable`, never the board model, or this ships
the way Lock B did.

One thing this does not solve and must not hide: `Interaction` is rebuilt on
every `HostMessage::Choice`, so a question re-sent unchanged wipes the picks
so far. `Pending: PartialEq` from `docs/design.md` §4 is the fix, and it is a
protocol item, not something to route around here.

### 10.3 Aiming: pointer, keys, and forty tokens

A pointer aims by clicking. The keyboard aims two ways, both of which exist
today for combat. The cursor (`W A S D`) walks the hand row and every lane's
representative, and `E` picks what it stands on. `C`/`⇧C` — the aim keys —
walk **what the question is pointed at**: the defenders when attacking, the
attackers when blocking, and under a target prompt the offer itself, objects
then seats, in the engine's order. Walking moves the cursor, so one highlight
stands where the answer will land; a seat is a cursor position too
(`HoverSource::Seat`), and the click key on it is `toggle_player`, which
today has one caller and no key. `focus_position` answers for a target prompt
as for combat, so the slip says `AimedAt Grizzly Bears (2 of 5)`; with one
candidate the focus starts on it and the slip says nothing about aiming.

A counted stack takes clicks like a card takes one. A click picks the next
unchosen offered member; another picks another, up to `max`; when no unchosen
member is left the click takes the last one back — on a single card, the
toggle it always was. Which four of the forty is the engine's problem
(`docs/design.md` §2.3). The badge over the stack, the one place UI text
already stands over the felt, reads `StackPicked` — "2 of 4" — instead of
"×4" while the stack holds picks. There is no count stepper: one click per
pick is the currency everywhere.

### 10.4 A pick drawn: proposed, then standing

A picked card lifts (`SELECTED_LIFT`, channel C5) and stops wearing the offer
light, as an armed card does — one border never says "you could" and "you
did" at once. A convoke pick wears `WILL_TAP` on top of the lift, because
that is what the bit means — this will be spent — and a creature about to be
tapped for a spell is the same claim as a land the plan would tap. And a line
ties the pick to what it is pointed at. `Line.from` becomes a `LineEnd` and
`LineKind` gains `Target`. The source is **the client's own record**, not
`view.stack`: the engine puts nothing on the stack until every cost is paid
(`cast_wizard.rs`, "pays everything and puts the spell on the stack"), so
while the question stands the deed just sent has to be kept — an ability's
line starts at the permanent it was armed on, a spell from hand has no place
on the felt and starts at the caster's `seat_anchor`; a seat target ends at
that seat's anchor, as an attack at a face does. The drawer does not change
— the same quads at `PROPOSED` weight, recomputed from live transforms.
Attack lines stay `DANGER`; block and target lines are the seat's candle,
and cannot be mistaken for each other because the engine asks one question
at a time.

Target lines exist **only while proposed**. Once sent, the stack panel names
each target as its own small card and the permanent wears the shadow tint
(C8, `Individual::Targeted`) that `docs/design.md` §1.5 reserves for it; a
third copy is the fault `docs/design.md` §1.3 forbids. Combat is the
exception for a reason on the felt: a standing attack keeps deciding who
takes damage until the step ends, so its line stays at `STANDING` weight. The
focus ring stands at whatever is aimed at — a card, or a seat's anchor.

### 10.5 Counting to done

The slip shows three lines and a button. The headline is today's
(`ChooseExactly`, `ChooseUpTo`, `ChooseBetween`, `ConvokeToHelpPay`). Under
it the tally, `SelectedOf` — "2 of 3 chosen" — which Appendix A already
lists and this section shares rather than twins. Under that the aim line,
only with more than one candidate. The button says what it will send:
`SendTargets` when something is picked, `NoTargets` when `min` is zero and
nothing is, `ConvokeNone` for a convoke paid with mana — inert until
`can_confirm`. `Space` is the button.

"Any number" and "up to N" end when the player says so, with the button. So
does exactly one. The house's one-click line is "the whole cost comes out of
the card, and the next untap undoes it"; a target fails both halves, and Lock
A says a mis-sent answer spends the decision. So a single target is picked
and then confirmed, and the payoff of two stages here is that **confirm is the
one send for every count** — there is no count at which the last click
becomes the only click. Arming each pick would be a third stage, and noise:
the line and the lift already say "proposed", and Lock A is not at risk until
`Space`. A pick past `max` — `toggle` answers `Full` silently today — draws
`AtMost` on the slip and moves nothing.

`Esc` takes back the **last** pick, one at a time; today's `cancel()` wipes
the whole answer in `Mode::Objects`, and the keyboard map's "half-built
answer" becomes "the last pick". `O` stays combat's wholesale word and is a
real answer, not a clearing: `DeclareNone` sends. Cancelling the whole
question is the honest gap. There is no `PlayerAction::Cancel`; the reversal
fault 24 describes happens in the engine on a *refused* answer, and
`Interaction` cannot build one — the door is closed by construction. For
`min == 0` the answer is `NoTargets`, and the button says so. For `min ≥ 1`
the way out is `docs/design.md` §4's `Cancel` before costs are paid, and
until it exists the slip carries no button that lies. The blocker's decision
clock (`docs/design.md` §3 item 12) is still not on the slip; that is the
next fault, not this one.

### 10.6 Refused here

**Sending the moment the count is full.** "Up to three" is not finished at
three; a player who wanted two and slipped on a third has just sent it. For
"exactly one" it makes the last click the only click — Lock A with a
friendlier name. `Space` sends, at every count.

**Dimming everything else.** The offer is one claim drawn once. A felt that
darkens under every spell says it a second time in the one channel — the
whole table — the design keeps quiet.

**Client-side legality.** Nothing in the client says "probably illegal" or
greys a card on its own reading of hexproof. The offer is the engine's
enumeration and nothing else; the day the client disagrees with the engine it
should be visibly wrong, so the engine gets fixed. Drag to target stays
refused for the reasons already given.

### 10.7 The six steps

Six commits, each with its tests, in dependency order. §11 carries them as
one entry; this is what that entry unfolds into.

1. **The model.** `crates/baylee-client-core/src/interaction.rs`: focus and
   `cycle_focus`/`focus_position` over `options ++ player_options` in
   `Mode::Objects`; a members-aware `toggle` that picks the next unchosen
   offered member and takes the last one back when none is left; `take_back()`
   replacing wholesale `cancel()` for picks; `assignments()` yielding
   `Target` pairs with a `LineEnd` source kept from the deed that was sent;
   `is_selectable` in `Mode::Blockers` answers for the focus, not the whole
   candidate list. Tests: a stack of four yields two distinct members, `Full`
   past `max`, a seat reachable by cycling, a ground creature not selectable
   while the focus is on a flier.
2. **The offer on the table.** `crates/baylee-client/src/cardmat.rs` and
   `table.rs`: `Offer::on` reads `is_selectable` over `members` while a
   question stands and lights `ACTIVATABLE`; a picked card drops it and lifts;
   `stack_badge` reads `StackPicked`. Tests: placement of an offered stack, a
   picked card, and `every_offered_object_is_drawn_as_offered`.
3. **The lines.** `crates/baylee-client-core/src/combat.rs` and
   `crates/baylee-client/src/combatlines.rs`: `Line.from: LineEnd`,
   `LineKind::Target`, proposed lines from a source permanent or seat anchor,
   the still candle ring at an offered seat, the focus ring at a seat anchor.
   Tests in `running`: a target line exists while proposed and is gone once
   sent; a ring at a named seat point.
4. **Input.** `crates/baylee-client/src/input.rs`: aim keys walk the offer and
   move the cursor; `HoverSource::Seat` and the click on it; an unoffered click
   is inert and says why; `Esc` takes back one. Test in
   `crates/baylee-client/tests/duel_flow.rs`: a spell's target chosen through
   the aim keys and `Space`, never a hand-built `PlayerAction`.
5. **The words.** `crates/baylee-client-core/src/i18n.rs` and
   `crates/baylee-client/src/hud/overlay.rs`: the seven arms; the tally, the
   stateful button, the `NotOffered` and `AtMost` lines; the seat tab's offer
   treatment. Overlay tests per button state; `docs/keyboard-map.md` gains the
   aim keys' second job.
6. **The stack panel.** `crates/baylee-client/src/hud/stack.rs`: entries
   become pickable, on top of §11 step 4, so a counterspell's target can
   be clicked. Test: a `ChooseTargets` whose only option is on the stack is
   answerable by click.

---

## 11. Sequencing

Defects are handed over (§0) and are prerequisites, not steps. Each step is
one review; each ends with the tests it names.

1. **Tokens and hue.** ACCENT out of the hand and tray; candle at two
   energies; palette test against the WGSL constants; `tnum` for the
   changing numbers (nowhere set yet). Small, unblocks every later look.
2. **Hand states and order.** Stable hand order; lift for `playable`; the
   abort literal (`lib.rs:831`) becomes a phrase. Protocol item filed:
   `LegalActions::castable_if_paid`.
3. **One-click land + MDFC sheet + the arming line** (§4). `Deed::Play` for
   lands removed; `duel_flow` test.
4. **Preview bubble + pickable stack** (§8). Highest value per line changed.
5. **Ability sheet in parchment with digits** (§7). Depends on 4's
   `bubble::place`; `Pick1..9` in `Keymap::standard` and the keymap doc.
6. **Search dialog** (§6). Sort/filter in the model, scroll, checkboxes,
   tally; the search field through softkeys (needs 9's `TextBuffer` only for
   the caret — a plain buffer suffices to ship this earlier).
7. **The frame** (§2): prompt slip, turn track, plaques on mats, pool bar
   only when non-empty, `pass_when_nothing_to_do` on with the `duel_flow`
   counts updated, `extent` covering the piles.
8. **Choosing a target** (§10). Six commits of its own — the model, the
   offer on the table, the lines, input, the words, the stack panel — and
   the last of them wants step 4's pickable stack. Here because it is what
   entry 20 asks for and every question the engine puts a list in front of
   already goes through it.
9. **Lobby text** (§9): `TextBuffer`, native keys, password toggle,
   `autocomplete` per form, one `text_field`.
10. **Payment ledger** (§5). Last because it is largest and because its full
    form needs the engine item. Ships in the "pool holds exactly the cost"
    form first.

**Protocol batch additions** (to `docs/design.md` §4):
- `LegalActions::castable_if_paid: Vec<ObjectId>` (§3).
- `PlayerAction::CastSpell { card, pay: Option<PoolPlan> }` — per-pip
  assignment from the pool, validated by the engine; the M2 `PayMana` in
  `mana_pay.rs`'s header, in its smallest form.
- `lands_played_this_turn` projected per seat (for the §4 hint) —
  **unverified** whether it already is.

---

## 12. Refused, in the spirit of `docs/design.md` §6

- **Client-side legality of any kind** — "playable" is the offer, and where
  the offer lacks a bit (timing for `reachable`) the answer is a protocol
  item, not a heuristic.
- **Faking pool-directed payment.** The client will not pretend to steer
  which floating mana pays which pip while the engine pays greedily; it
  floats exactly the cost or shows the engine's assignment.
- **A confirmation dialog for the land drop**, and equally **widening the
  one-click line** to anything with a cost or a stack. The line in §4 is the
  whole rule.
- **Hover-to-open ability sheets or previews that follow the pointer.**
- **Per-frame UI projection onto every card.** One sheet and one bubble
  follow a card; the rest of the HUD does not.
- **Translucent parchment, or a serif for it.** Parchment is opaque; Inter
  with letter-spacing is the whole typographic budget.
- **The rail as a settings panel in the game frame.** It becomes a clock;
  the switches live in settings and in the hold gestures.
- **Re-sorting the hand by playability.** The glow says it; the order stays.
- **A second preview renderer**, drag-to-pay, drag-to-target, lights,
  tonemapping, undo — all already refused and still refused.

---

## Appendix A — new phrases (both languages, `messages!` in `i18n.rs`)

None of these names exists in `i18n.rs` as of this reading (each grepped as
a variant). A phrase for passing priority very likely exists under another
name for the current pass button — reuse it and drop `Pass` below rather
than add a second.

| Phrase | en | de |
|---|---|---|
| `PlayLandHint` | Click a lit land to play it | Ein leuchtendes Land anklicken, um es zu spielen |
| `LandDropUsed` | Land drop used this turn | Das Land für diesen Zug ist bereits gespielt |
| `NotPlayableNow` | Not playable right now | Gerade nicht spielbar |
| `PlayAsLand` | Play as a land | Als Land spielen |
| `CastFace` | Cast {0} | {0} wirken |
| `PayTitle` | Pay {0} | {0} bezahlen |
| `PayRemaining` | {0} still to pay | Noch {0} zu bezahlen |
| `PayFromPool` | From the pool | Aus dem Manavorrat |
| `PayTapHint` | Click a land or source to tap it | Ein Land oder eine Quelle anklicken, um sie zu tappen |
| `PayAuto` | Auto | Auto |
| `PaySend` | Cast | Wirken |
| `PayStop` | Stop paying | Bezahlen abbrechen |
| `PayFloatsRemain` | Floating mana stays in your pool | Schwebendes Mana bleibt im Vorrat |
| `ManaUpNotCastable` | The mana is there, the timing is not | Das Mana ist da, der Zeitpunkt nicht |
| `SortByName` | Name | Name |
| `SortByCost` | Cost | Kosten |
| `SortByType` | Type | Typ |
| `SortByPlace` | Place | Ort |
| `SelectedOf` | {0} of {1} chosen | {0} von {1} gewählt |
| `SendTargets` | Confirm targets | Ziele bestätigen |
| `NoTargets` | No targets | Keine Ziele |
| `ConvokeNone` | Pay with mana | Mit Mana bezahlen |
| `NotOffered` | Not on offer here | Steht hier nicht zur Wahl |
| `AtMost` | No more than {0} | Höchstens {0} |
| `StackPicked` | {0} of {1} | {0} von {1} |
| `SearchPlaceholder` | Search by name… | Nach Namen suchen… |
| `NoMatches` | Nothing matches | Kein Treffer |
| `SheetMore` | More… | Mehr… |
| `Pass` | Pass | Passen |
| `ShowPassword` | Show password | Passwort anzeigen |
| `HidePassword` | Hide password | Passwort verbergen |
| `FieldRequired` | Required | Pflichtfeld |
| `StopHere` | Stop here | Hier anhalten |

## Appendix B — drifts found while reading

- `hand.rs:88-97` and `tray.rs:220-225`: comment says gold, code uses teal
  `ACCENT` (`hud.rs:426`).
- `docs/keyboard-map.md` §Arming: "lifts to the height a chosen card sits
  at" vs `ARMED_RAISE 8` (`hand.rs:167`) and no lift on selection.
- `lib.rs:831`: an English literal shown to the player, outside `Phrase`.
- `overlay.rs:1148-1164`: a bubble tail that always points at the hand bar.
- `tray.rs:136-155` and `overlay.rs:606-638`: text fields that bypass
  `softkeys.rs` and cannot be typed into on wasm (**unverified** on a
  device; follows from `owns_typing()`, `softkeys.rs:41-43`).
- `softkeys.rs:165-167`: `autocomplete=current-password` on the
  create-account form.
- `browser.rs:176`: `set_filter` has no non-test caller.
- The brief placed `Openings` in `interaction.rs`; it is `board.rs:579`.
