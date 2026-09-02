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
| The click (card under cursor → phase toggle → pass) | `Space` | implemented |
| Confirm / pass (never toggles anything else) | `Enter` | implemented |
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
| Battlefield camera: pan | arrows, left-drag, touch-drag | implemented |
| Battlefield camera: zoom | `Shift+↑/↓`, wheel, pinch | implemented |
| Battlefield camera: rotate | `Shift+←/→`, right-drag, rotate gesture | implemented |
| Select a phase/step button (the rail's keyboard cursor) | `⇧W` / `⇧S` | implemented |
| Fast-forward to next phase (decisions still yours) | `Tab` | implemented |
| Fast-forward to the next turn | `⇧Tab` | implemented |
| Number choices (X) | arrows | implemented |
| Mulligan keep / bottom | `K` / `B` | implemented |
| Yes / no | `Y` / `N` | implemented |
| Game log | `L` | planned |
| Automation menu for selection | `M` | planned |

The camera controls are deliberately *not* in the keymap: they are held-key
analogue input rather than discrete actions, and a rebinding screen listing
"pan left" beside "keep this hand" would be describing two different kinds of
thing.

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
- `Enter` sends what stands; `O` sends nothing at all. Both are real answers,
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
The prompt bar carries the answers for whatever is pending, including combat's
"Aim next", "Attack"/"Block" and "None". The hand bar scrolls horizontally
with the mouse wheel.

## Rules

Every `Pending` variant is operable without a pointer device; focus is always
visible; no action requires drag-and-drop (drag has a keyboard equivalent).
Unbinding an action is allowed — a pointer can still reach everything.
