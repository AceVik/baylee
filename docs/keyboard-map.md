# Keyboard Map (spec)

The game client is fully playable keyboard-only (and equally mouse-only).

Every binding below is a **default**. The keymap is
`baylee_client_core::prefs::Keymap`, it lives in the account's
[`Preferences`](protocol.md), and the gateway stores it under
`GET`/`PUT /settings` — so a player who rebinds *confirm* at home finds it
rebound at a friend's table. `crates/baylee-client/src/keys.rs` is the only
place that knows a stored key name (`"KeyW"`, `"ArrowUp"`) is a Bevy
`KeyCode`; the keymap itself has no renderer type in it and is tested without
a window.

Two consequences worth knowing before changing anything here:

- Names are **physical**, not typed characters. A player who binds the key
  right of `A` finds it right of `A` on a German keyboard too.
- A modifier makes a *different* chord, not an extra one. `W` and `⇧W` are two
  bindings, and the keymap tells them apart — which is why the input handler
  has no `if !shift` guards in it any more.

| Action | Default | Status |
|---|---|---|
| The click (a sheet holding the question → card under cursor → phase toggle → pass) | `Enter` | implemented |
| Confirm / pass (ticks a row on a sheet holding the question, and toggles nothing else) | `Space` | implemented |
| Cancel: armed deed, then preview, then the zone browser, then phase selection, then half-built answer | `Esc` | implemented |
| Send what is armed (a second tap on the card does the same) | `Enter` / `Space` / `E` | implemented |
| Move the card cursor (hand → own board → opponents) | `W A S D` | implemented |
| Activate the card under the cursor (play / select) | `E` | implemented |
| Look at the next opponent's board (wraps home) | `F` | implemented |
| Look at your own board | `H` | implemented |
| Aim the next attack (or block) at the next defender | `C` / `⇧C` | implemented |
| Declare nothing — no attackers, or no blockers | `O` | implemented |
| Show a card's text instead of its art (while held) | `Cmd` / `Alt` | implemented |
| Keep the card text on (latch, persisted) | `T` | implemented |
| Open the zone browser (graveyards, exile, the stack) | `G`, or a tap on the top card of a pile | implemented |
| Move the zone browser / resize it (remembered per client; a sheet a *question* opened is centred, stays put and draws no corner) | drag its title row / its bottom-right corner | implemented |
| Battlefield camera: pan / zoom / rotate / tilt | — (deliberately none) | removed |
| Select a step tile (the seat bars' keyboard cursor) | `⇧W` / `⇧S` | implemented |
| Fast-forward to next phase (decisions still yours) | `Tab` | implemented |
| Fast-forward to the next turn | `⇧Tab` | implemented |
| Number choices (X) | arrows, digits, `⌫` (or the `−`/`+` buttons) | implemented |
| Pick a row of the ability sheet (the digit drawn on it) | `1`–`9` | implemented |
| Walk the ability sheet: its column / its mana pips | `W` `S` / `A` `D` | implemented |
| Turn the ability sheet's page | `0` | implemented |
| Mulligan keep / bottom | `K` / `B` | implemented |
| Yes / no | `Y` / `N` | implemented |
| Let the stack resolve (stop asking me) | `F6` | implemented |
| Nothing more this turn (stop asking me) | `F7` | implemented |
| Ask me again (cancel a hold) | `F6` / `F7`, or the way out on the shelf | implemented |
| Game log | `L` | planned |
| Automation menu for selection | `M` | planned |

There are **no camera controls left**, and the rows above are the whole of the
keyboard. The camera has one job — framing the table against the part of the
window the table is seen through — and `table::frame_table` does it on every
seat count, focus and resize. The two viewpoints that remain are in the table
above and in the keymap like everything else: `F` walks to the next
opponent's board and `H` comes home.

That is also why the arrows have no exception written into them any more. They
are `NumberUp`/`NumberDown` and nothing else, and while a text box holds the
keyboard they are the box's, because every key on this page goes through
`Fired::of` and stops at `browser_keys`. The camera was the one route that did
not: it read `KeyCode` directly, so the arrows drove the table *through* a
focused filter box, which is the owner's report of 14.09.2026 and the reason
the last of the camera went with it.

Removing the camera made the arrows reach the box; it did not make the box
*show* it. `hud::BrowserGate` compared the filter's **string**, so the caret
and the selection — which is all an arrow moves — were state the dialog's
revision could not see, and the box redrew only when the text changed. It
carries the whole `TextBuffer` now. Measured in the running client the same
day: five `ArrowLeft`s over `abcdef` moved the box by zero pixels before, and
`⇧←` three times paints `def` and stands the caret on its near edge after.

A number is typed as well as stepped: a digit appends to what stands (`1`
then `2` reads 12), falls back to the digit alone when appending would leave
the offered range, and `⌫` takes one off. Twelve presses of `↑` is not a way
to say "X is my whole hand of lands".

## Combat

Combat is the one choice where clicking a creature is not enough — the engine
also asks *what* it is attacking (CR 508.1b), and only it knows which
planeswalkers are legal defenders. So the client carries a **focus**: the
thing the next declaration will be pointed at.

- With one legal defender (the usual two-player game with no planeswalkers)
  the focus starts on it and there is nothing to aim. The prompt bar says
  nothing about aiming and the key does nothing.
- Otherwise `C` walks the defenders and the prompt bar says
  `Aimed at <name> (2 of 3)`. A pointer can skip that: tapping a
  planeswalker — or, when blocking, tapping an attacker — aims at it directly.
- Tapping a creature then declares it *against the focus*. Tapping it again
  calls the declaration off.
- `Space` sends what stands; `O` sends nothing at all. Both are real answers,
  and the step does not end until one of them is given.

## Automation

Two independent things, both stored per account:

- **The phase rail** — one button per step of the turn, per side of the
  table. It was a strip across the top of the window and is now written on
  each seat's own mat: the twelve steps run along that seat's shelf, on the
  edge of the mat that reads as *above* the board from where the local player
  is sitting (`hud::seatbar`, `client-core/src/seatbar.rs`).
  Green means "ask me here", red means "skip"; the untap row is grey and
  answers nothing, because no player receives priority there (CR 502.4). Nothing is red by default: a
  client that auto-passes without being asked loses games its player never
  agreed to lose. Two **presets** write the whole rail at once —
  `Stop everywhere`, which is that default said out loud, and
  `Competitive stops`, which keeps both of your own main phases, the whole of
  combat on both turns, and the end step of an opponent's. Neither ever reds a
  row where *this* seat is the one declaring: red is `DeclareNoAttackers` /
  `DeclareNoBlockers` there, not a pass, and a preset that skipped a
  declaration step would be answering it.
- **`AutoRules`** — four switches, all off by default: pass a window that
  offers nothing at all; pass through opponents' turns (priority only, never a
  block); answer "no attackers" when nothing can attack; the same for blocks.
- **Priority holds** — `F6` and `F7`, and the only automation that lives in
  the *engine* rather than in the client. The rail and `AutoRules` answer for
  a player who is still being asked; a hold tells the engine not to ask, which
  is what makes it fast enough to matter on a long stack and what makes it
  binding on a client that has been closed and reopened.

Three things about holds are load-bearing. Every one of them **cancels
itself** (`PriorityHold` in `crates/baylee-engine/src/choice.rs`): `F6` ends
when the stack empties *or* when anything is added to it, because the moment
somebody responds to what was being let through is exactly the moment a player
wants the question back. There is deliberately no "never ask me again" — a
hold that could outlive its reason is a hold that loses a game quietly.

Both keys **cancel** a running hold rather than replacing it, so a player who
has stopped being asked does not have to remember which key did it. And a
running hold is **drawn**, in the middle of the shelf and exactly where a
question would have stood: the sentence says the seat is not being asked and
the single answer beside it is the way out. Without that the state would have
no symptom at all — the shelf is empty because the seat is not being asked,
which is exactly what an idle turn looks like. The **autopilot** is drawn by
the same pair of lines, because to a player the two are one state ("you are
not being asked, and here is how to be asked again"); the difference is the
key cap, which only the engine hold wears, `F6` and `F7` being the keys that
cancel a hold and not a run of turns the client is playing out.

Unlike every other answer, a hold is sent while this seat is **not** the one
being asked; the engine accepts an automation setting from any seated player
at any time (`Engine::apply` handles it before the "who is being asked" gate),
and without that a hold could be set but never taken back.

That is also why the decision clock is anchored to `Session::decision_seq`
rather than to `Session::seq`: a hold produces a frame without moving the game,
and a clock counting frames would restart on every press of `F6` at the other
end of the table. Nobody but the seat being asked may wind its clock.

## Arming

There is no undo in the engine, so anything irreversible is **two-stage**: the
first tap on a card *arms* it and puts nothing on the wire, and a second tap on
the same card — or the confirm key, or the deed's own button in the prompt bar
— sends it. `Esc` disarms. Tapping a different card is a change of mind rather
than a confirmation: it disarms and arms the new one.

**Mana abilities are the exception and stay one tap.** Floating mana is the one
cheap mistake in the game — it empties at end of step and a wrong colour is
fixed by tapping another source — so asking for a confirmation there would put
a second click on the most common action a player makes. What counts as one is
`abilities::AbilityOption::mana`, read off the card's own `mana_ability` flag
(CR 605.1) — not `legal.mana_abilities`, which carries the CR 305.6 shortcut
and granted abilities but not a printed `{T}: Add {G}`, and not the mana-source
list, which reduces a permanent to the one tap it usually has.

An armed deed is re-checked against the engine's *current* offer everywhere it
is read — the two keys, the button, and the row that draws it — so a deed the
engine has withdrawn between the taps disarms and says so rather than being
sent. A `Run` is re-read too, and can come back a different answer: between the
two taps this seat holds priority, so the one thing that can have changed is
its own manual land tap, after which the spell is castable outright and the run
would float mana nobody asked for. Picking a row of the ability sheet arms
rather than sends: the sheet disambiguates, it does not confirm. §"The ability
sheet" below is the whole of that.

**Suspending is a fourth deed, and a run has two ends.** "Rather than cast this
card from your hand, pay {U} and exile it with four time counters on it"
(CR 702.62a) is not playing the card, so `Deed::Suspend` is its own arm rather
than a shape of `Deed::Play` — and one card can be both, which is why the tap
order in `input.rs` asks about casting first. A `Deed::Run` therefore carries a
`RunEnd` saying what the floated mana is spent on when the last land is tapped.
That choice is made when the run is armed and never when it lands: at the end
the engine's answer is a `LegalActions` with the card in *both* lists, and a run
that guessed there would cast a spell the player meant to suspend, which is not
an action anything can take back. The prompt bar says which — "Pay {U} and cast"
against "Pay {U} and suspend", the cost drawn as its own pips rather than
counted as a number of cards to tap.

**An armed deed is drawn as a sentence in two halves.** The card wears
`glow::ARMED` — a bright ring held tight against its printed edge, breathing in
place — and lifts to the height a chosen card sits at, keeping it when the
pointer leaves. Whatever an armed mana run would tap wears `glow::WILL_TAP`, a
cooler pulse a beat behind, because the price follows the verb and "Tap 3, then
cast" does not say *which* three. The armed card stops wearing the
`ACTIVATABLE` chase it accepted: one border carrying both would be saying the
same thing twice. `docs/client.md` §"The card surface" has the whole register.

## The ability sheet

A permanent with more than one thing to do opens a **sheet** when it is
activated: a piece of parchment anchored beside the card itself, one numbered
keycap per row, the ability's own printed sentence beside it, and its cost as
pips on the right. It replaced a row of buttons in the prompt bar, which named
the abilities of a card at the far side of the screen and could only say
"Ability 2" about the ones it had no words for.

**A row is sent by the digit drawn on it** — `1` through `9`, and `0` turns
the page when there is a second one. A page holds nine because the keycap is
the key, and there are nine digits that are not zero;
`baylee_client_core::abilitysheet` is the arithmetic. Those digits are read as
**typed characters and not as keymap actions**, the way a number choice reads
them: nine rebindable rows called "the fourth ability" would be naming a
position on a sheet rather than a thing a player does.

And it is drawn as a **key**: a square with its corners taken off, sharing
`sheet::KEYCAP_R` with the row it sits in so the two read as one object. It
was a disc for as long as it existed, and a disc with a numeral in it is a
bullet — the ornament that numbers a list, which is exactly the thing this
one is not.

**The keycap says what the press will do.** A row whose whole cost is paid
out of the card — a mana ability (CR 605.1), or one that asks nothing beyond
the `{T}` it already implies — goes through on the first press, for the reason
§Arming gives: the next untap step gives that cost back. Every other row
**arms** instead. The keycap turns gilt, the card wears `glow::ARMED` with
it, the sheet stays standing, and the same digit again sends it. A different
digit is a change of mind and moves the arming, exactly as tapping a different
card does. `Esc` disarms and leaves the sheet open; `Esc` again closes it.

**One row neither arms nor sends: a mana row the pips could not stand for.**
Harabaz Druid under a Great Divide Guide offers all five colours twice, and
one pip per colour is all there is, so the pips go to the tap a player can
predict and the Druid's own `{T}: Add X mana of any one color` stays a written
row. Pressing it steps *into* that tap — the sheet becomes a bubble of its
five colours with `X×` before them, and the press that answers is what taps
the card. `Esc` there is one step back to the sheet rather than the way out,
which is what `Esc` does everywhere else on this list: it takes back the last
thing the player said.

Clicking a row does all of the above identically — the pointer and the digit
go through one function, because a second click that re-armed while a second
press sent would be two answers to the same question. A player who would
rather not count reaches the same rows with the card cursor, and the activate
key takes the row the cursor is on.

**The cursor is two-dimensional here and nowhere else in the client**, because
this sheet is the only one with two directions on it: a permanent that makes
mana carries a centred **row** of coloured pips above its column of written
rows (`docs/client.md` §"The question itself is a sheet"). So `W` and `S` walk
the whole column, pips included, and turn the page by walking off the end of
one; `A` and `D` belong to the pip strip, wrapping inside it and never
reaching a sentence. From a written row either of them **arrives** on the
strip in one press, at the end it was travelling towards — rightwards at the
first pip, leftwards at the last. A sheet with no pips has nothing horizontal
on it, and there `A` and `D` step the list exactly as `W` and `S` do, so a
player holding one of them need not know which permanents have a header.
`baylee_client_core::abilitysheet::step_down` and `step_along` are the
arithmetic.

The sheet owns the keyboard while it stands, *after* an armed deed and before
the card cursor. That order is the arming rule seen from the other side:
arming is where choosing ends, so a confirm key that reached the sheet instead
would pick a second ability rather than send the first.

**The sheet takes no key it was not offered.** It reads the keyboard only
while nothing else is typing — the zone browser's filter box is the case that
exists — because the digit path drains the whole key queue rather than the
digits alone, so a sheet that read unconditionally would eat the letters going
into that box and open an ability with the digits.

## The cast chooser

A card in hand with more than one way to be **cast** opens the ability sheet's
sibling, in the prompt bar rather than beside the card: the same chooser the
engine's own `ChooseCastMode` opens, asked one step earlier because this
client is what floats the mana and the engine cannot count a spell's ways
until it is floating (`docs/client.md` §"Which way to cast it is asked before
anything is tapped").

The keys are the ability sheet's, one dimension shorter. `W`/`S` and `A`/`D`
all walk the single column and wrap; the primary key, confirm and the activate
key take the row the cursor is on; `Esc` puts the chooser away with nothing
said. Clicking a row is the same press through the same function.

**A row arms rather than sends**, which is §Arming's rule and not a second
one: a spell on the stack is the least undoable thing in the game, so the
press that answers *which way* leaves the deed standing in the bar and the
next press is what pays for it. The chooser closes on that first press, the
way the ability sheet closes when one of its rows arms.

One way is not a question and no chooser opens — the click then arms what it
always armed, in two presses rather than three.

While it stands it holds the **whole** prompt bar: the pass and skip-turn
answers are not drawn under it, because those answer the priority window it
was opened inside and not the question on the headline — and the keys already
went to the chooser, so the bar was drawing a primary button the keyboard
would not press. `Esc` is therefore the one way past it that is not an answer,
which is the rule the ability sheet and every dialog here already follow. A
click on **another card** also puts it away — §Arming's change of mind — and
takes the way that was chosen with it, exactly as `Esc` on an armed deed does.

And a row **reads like the ability sheet's**, which is the whole of what makes
the two siblings rather than lookalikes: a mode and an alternative cost are
printed sentences, so the row carries the card's own words in the player's own
language instead of "Mode 2" — `docs/client.md` §"Which ability is on the
stack" has the table that answers it.

## The sheet a question opened

A question whose answer is lying in a pile has nowhere on the table to be
answered, so the zone browser opens as a **dialog**: `Browser::follow` is the
only door that does it, and `Browser::answers_here` is the one predicate that
says the sheet is standing for a question — it was opened for a choice, the
choice is this seat's, and the choice has bounds. While that holds, the sheet
takes **both** of the first two keys above, and takes them before anything
else reads a key.

`Space` ticks the row the focus is on; `Enter` sends what is ticked. Both
consume the frame even when they change nothing. An `Enter` with nothing
ticked leaves the dialog standing rather than falling through, because what it
fell through to was the card the pointer happened to be resting on behind the
sheet — and opening *that* card's pile is not an answer to the question on the
screen. A tap on a pile is refused while a question is standing for the same
reason and a harder one: it would hand the sheet to the pile, which makes
`answers_here` false and leaves a dialog still on the screen with neither of
its own keys working.

That order is the shorter half of §"The ability sheet" above, read from the
same side: a surface holding a question keeps its keys until the question is
answered or cancelled, and `Esc` is the way out, at the rung the cancel ladder
in the table above gives the zone browser.

## Mouse

Hovering a card lifts it and shows the large tooltip (cursor shared with WASD
— one highlight, never two); clicking arms a playable card (a second click
plays it) or selects it for the pending choice; chosen cards stay raised /
accent-framed until the choice is answered.

A card in hand can be lit two ways. **Gold** is the engine offering it: the
mana is floating and one click casts it. **Indigo** is the client offering to
tap for it: the mana is not floating, the lands to make it are, and one click
taps them and then casts. Nothing is spent that a player would want to decide
— Phyrexian mana is never paid with life, and `{X}` is never guessed at — and
if anything about the board changes mid-way the taps stop and the turn comes
back with a line in the prompt bar saying why (see `docs/client.md`
§"Tapping lands for a spell"). Tapping a land by hand still works and always
did; this only removes the requirement. Every seat's bar is written on that
seat's own mat, on the shelf along the edge of the board it describes, and
clicking it switches board views (or points at that seat, while a choice is
asking for a face). Every step of the turn (Untap, Upkeep, Draw, Main 1,
Begin Combat, Attackers, Blockers, Damage, End of Combat, Main 2, End Step,
Cleanup) has its own tile on that bar, toggling green / red on click.
Fast-forwarding is `Tab` and `⇧Tab`, which have no buttons of their own.
Clicking your own permanent activates what it is offering: one ability arms
straight away (or goes through, if it makes mana), several open the ability
sheet on the card itself. The prompt bar carries the answers for
whatever is pending, including combat's "Aim next", "Attack"/"Block" and
"None". The hand bar scrolls horizontally
with the mouse wheel.

### Nothing a hand does moves the camera

It went in three removals, each one an owner report, and they are worth
reading together because the last only makes sense as the end of the series.

**The orbit**, first. A left drag turned the table, and the left button is
also the button that plays cards, so every click that travelled a pixel
turned the table a little — and worse, it switched the automatic framing off
for the rest of the session, because `table::frame_table` followed its own
shot only while the rig still equalled it exactly. Yaw and tilt are not
controls a hand should have anyway: every seat's bar is drawn upright on its
own mat, so turning the table only makes "which side am I on" ambiguous, and
`table::CAMERA_LEAN` is a measured trade rather than something a hand aims.

**The zoom**, next. The wheel had to be arbitrated against every scrolling
panel in the interface, and the referee did not hold at the surface it
mattered at (*„mit dem Rad scrollen scheint sich mit dem Kamera Zoom-In/Out
zu streiten"*). The capability went rather than the referee, and the pinch
with it.

**The pan**, on 14.09.2026, and with it the whole of the thing: *„Generelles
Camera Movement kann weg (also nicht nur die Maus Controls, sondern auch die
Keyboard Controls)"*. The report under it is what makes this page the right
place for the story — the arrows drove the table while a **text field** had
the keyboard. Every key on this page goes through `Fired::of` and stops at
`browser_keys` when a box is typing; `input::camera_controls` read `KeyCode`
directly, so it was the one route around that guard, and shift-arrow in the
zone dialog's filter panned the board instead of extending a selection. The
system is deleted.

What is left needs no arbitration at all. `CameraRig::home` frames the table
against the part of the window the table is seen through, `frame_table`
reapplies it on every seat count, focus and resize, and the only thing that
suspends it is a player asking to look at **one seat** (`F` walks them, `H`
comes home) — which is a viewpoint rather than a camera control, and is in
the keymap like every other action. A wheel is the interface's whenever there
is a UI node under the pointer, scrolling or not; over bare felt it is now
nobody's.

## The end screen

The verdict sheet is the one screen whose keys are not the duel's.
`DuelSet::Input` runs only in `DuelPhase::Playing`, so by the time the sheet
is up every binding above is off — which for a while meant the sheet could be
reached with a keyboard and not left with one, the only screen in the client
with no exit. `lobby::systems::leave_keys` answers there instead, and it reads
the buttons that are **actually drawn** rather than a list of its own, so a
key can never take a way out the sheet does not show.

| Action | Default | What it does |
|---|---|---|
| The click / confirm | `Enter` / `Space` | presses the lead answer — *play again* at a gateway's table, *back to the lobby* offline |
| Cancel | `Esc` | always the way back to the lobby |

It lives in the lobby plugin and not in `input.rs` because the verdict is the
duel's to say and the way *out* is the shell's: `DuelPlugin` is embeddable in
an application with no lobby behind it, and there the sheet's exit row stays
empty. Reading the row is what makes both cases one rule.

## Rules

Every `Pending` variant is operable without a pointer device; focus is always
visible; no action requires drag-and-drop (drag has a keyboard equivalent).
Unbinding an action is allowed — a pointer can still reach everything.
