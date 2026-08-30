# Keyboard Map (spec)

The game client is fully playable keyboard-only (and equally mouse-only).
Binding defaults (rebindable, persisted per account):

| Action | Key | Status |
|---|---|---|
| Pass priority / confirm | `Space` / `Enter` | implemented |
| Yield to step / to turn | `Shift+Space` / `Ctrl+Enter` | planned |
| Cancel / back | `Esc` | implemented |
| Move the card cursor (hand → own board → opponents) | `W A S D` | implemented |
| Activate the card under the cursor (play / select) | `E` | implemented |
| Cycle opponents (inspect focus) | `Tab` / `Shift+Tab` | implemented (`Tab`) |
| Number choices (X) | arrows | implemented |
| Navigate cards / options | `H J K L` | planned |
| Auto-tap toggle | — (was `A`, now cursor left) | planned |
| Choice list confirm | `Enter` (list nav with arrows) | planned |
| Game log | `L` | planned |
| Automation menu for selection | `M` | planned |
| Mulligan keep / bottom | `K` / `B` | implemented |
| Yes / no | `Y` / `N` | implemented |

Mouse: hovering a card lifts it (cursor shared with WASD — one highlight,
never two); clicking plays a playable card or selects it for the pending
choice; chosen cards stay raised / accent-framed until the choice is
answered.

Rules: every `ChoiceRequest` type is operable without a pointer device;
focus is always visible; no action requires drag-and-drop (drag has a
keyboard equivalent). Lobby (Leptos) uses standard ARIA navigation.
