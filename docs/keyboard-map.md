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
| The click (card under cursor → phase toggle → pass) | `Enter` | implemented |
| Confirm / pass (never toggles anything else) | `Space` | implemented |
| Cancel: preview, then phase selection, then half-built answer | `Esc` | implemented |
| Move the card cursor (hand → own board → opponents) | `W A S D` | implemented |
| Activate the card under the cursor (play / select) | `E` | implemented |
| Look at the next opponent's board (wraps home) | `F` | implemented |
| Look at your own board | `H` | implemented |
| Aim the next attack (or block) at the next defender | `C` / `⇧C` | implemented |
| Declare nothing — no attackers, or no blockers | `O` | implemented |
| Slide the own-board overlay down/up | `X` (or the knob) | implemented |
| Show a card's text instead of its art (while held) | `Cmd` / `Alt` | implemented |
| Keep the card text on (latch, persisted) | `T` | implemented |
| Open the zone browser (graveyards, exile, the stack) | `G` (or a pile chip) | implemented |
| Battlefield camera: pan | arrows (not while choosing a number), left-drag, touch-drag | implemented |
| Battlefield camera: zoom | `Shift+↑/↓`, wheel, pinch | implemented |
| Battlefield camera: rotate | `Shift+←/→`, right-drag, rotate gesture | implemented |
| Select a phase/step button (the rail's keyboard cursor) | `⇧W` / `⇧S` | implemented |
| Fast-forward to next phase (decisions still yours) | `Tab` | implemented |
| Fast-forward to the next turn | `⇧Tab` | implemented |
| Number choices (X) | arrows, digits, `⌫` (or the `−`/`+` buttons) | implemented |
| Mulligan keep / bottom | `K` / `B` | implemented |
| Yes / no | `Y` / `N` | implemented |
| Let the stack resolve (stop asking me) | `F6` | implemented |
| Nothing more this turn (stop asking me) | `F7` | implemented |
| Ask me again (cancel a hold) | `F6` / `F7`, or the chip | implemented |
| Game log | `L` | planned |
| Automation menu for selection | `M` | planned |

The camera controls are deliberately *not* in the keymap: they are held-key
analogue input rather than discrete actions, and a rebinding screen listing
"pan left" beside "keep this hand" would be describing two different kinds of
thing. That is also why the arrows have one exception written into the camera
rather than into the keymap: `NumberUp`/`NumberDown` *are* discrete actions
bound to the same keys, so while a number is being chosen the arrows belong to
the number and the table holds still. Without it the same press raised X and
panned the board out from under it.

A number is typed as well as stepped: a digit appends to what stands (`1`
then `2` reads 12), falls back to the digit alone when appending would leave
the offered range, and `⌫` takes one off. Twelve presses of `↑` is not a way
to say "X is my whole hand of lands".

## Combat

Combat is the one choice where clicking a creature is not enough — the engine
also asks *what* it is attacking (CR 508.1a), and only it knows which
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

- **The phase rail** — one button per step of the turn, per side of the table.
  Green means "ask me here", red means "skip". Nothing is red by default: a
  client that auto-passes without being asked loses games its player never
  agreed to lose.
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
running hold is **drawn**: an accent chip beside the concede button, with the
way out next to it. Without that the state would have no symptom at all — the
prompt bar is empty because the seat is not being asked, which is exactly what
an idle turn looks like.

Unlike every other answer, a hold is sent while this seat is **not** the one
being asked; the engine accepts an automation setting from any seated player
at any time (`Engine::apply` handles it before the "who is being asked" gate),
and without that a hold could be set but never taken back.

That is also why the decision clock is anchored to `Session::decision_seq`
rather than to `Session::seq`: a hold produces a frame without moving the game,
and a clock counting frames would restart on every press of `F6` at the other
end of the table. Nobody but the seat being asked may wind its clock.

## Mouse

Hovering a card lifts it and shows the large tooltip (cursor shared with WASD
— one highlight, never two); clicking plays a playable card or selects it for
the pending choice; chosen cards stay raised / accent-framed until the choice
is answered.

A card in hand can be lit two ways. **Gold** is the engine offering it: the
mana is floating and one click casts it. **Indigo** is the client offering to
tap for it: the mana is not floating, the lands to make it are, and one click
taps them and then casts. Nothing is spent that a player would want to decide
— Phyrexian mana is never paid with life, and `{X}` is never guessed at — and
if anything about the board changes mid-way the taps stop and the turn comes
back with a line in the prompt bar saying why (see `docs/client.md`
§"Tapping lands for a spell"). Tapping a land by hand still works and always
did; this only removes the requirement. Player tabs at the top switch board views; every step of the turn
(Untap, Upkeep, Draw, Main 1, Begin Combat, Attackers, Blockers, Damage, End
of Combat, Main 2, End Step, Cleanup) has its own rail button toggling green /
red on click; the rail's "Next ▶" and "End ⏭" buttons fast-forward like `Tab`.
Clicking your own permanent activates what it is offering: one ability goes
straight through, several open a chooser on its own row of the prompt bar
("Tap for G", "+1", "Ability 2"). The prompt bar carries the answers for
whatever is pending, including combat's "Aim next", "Attack"/"Block" and
"None". The hand bar scrolls horizontally
with the mouse wheel.

## Rules

Every `Pending` variant is operable without a pointer device; focus is always
visible; no action requires drag-and-drop (drag has a keyboard equivalent).
Unbinding an action is allowed — a pointer can still reach everything.
