# Keyboard Map (spec)

The game client is fully playable keyboard-only (and equally mouse-only).
Binding defaults (rebindable, persisted per account):

| Action | Key | Status |
|---|---|---|
| The click (card under cursor → phase toggle → pass) | `Space` | implemented |
| Confirm / pass (never toggles anything else) | `Enter` | implemented |
| Cancel: phase selection, then half-built selection | `Esc` | implemented |
| Move the card cursor (hand → own board → opponents) | `W A S D` | implemented |
| Activate the card under the cursor (play / select) | `E` | implemented |
| Inspect an opponent's board (toggle) | `Shift+1..9` | implemented |
| Select a phase/step button (the rail's keyboard cursor) | `Shift+W` / `Shift+S` | implemented |
| Fast-forward to next phase (decisions still yours) | `Tab` | implemented |
| Number choices (X) | arrows | implemented |
| Mulligan keep / bottom | `K` / `B` | implemented |
| Yes / no | `Y` / `N` | implemented |
| Yield to step / to turn | `Shift+Space` / `Ctrl+Enter` | planned |
| Navigate cards / options | `H J K L` | planned |
| Choice list confirm | `Enter` (list nav with arrows) | planned |
| Game log | `L` | planned |
| Automation menu for selection | `M` | planned |

Mouse: hovering a card lifts it and shows the large tooltip (cursor
shared with WASD — one highlight, never two); clicking plays a playable
card or selects it for the pending choice; chosen cards stay raised /
accent-framed until the choice is answered. Player tabs at the top switch
board views; every step of the turn (Untap, Upkeep, Draw, Main 1, Begin
Combat, Attackers, Blockers, Damage, End of Combat, Main 2, End Step,
Cleanup) has its own rail button toggling green (take priority) / red
(skip) on click; the rail's "Next ▶" and "End ⏭" buttons fast-forward
like `Tab`. The hand bar scrolls horizontally with the mouse wheel.

Rules: every `ChoiceRequest` type is operable without a pointer device;
focus is always visible; no action requires drag-and-drop (drag has a
keyboard equivalent). Lobby (Leptos) uses standard ARIA navigation.
