# Keyboard Map (spec)

The game client is fully playable keyboard-only (and equally mouse-only).
Binding defaults (rebindable, persisted per account):

| Action | Key |
|---|---|
| Pass priority / confirm | `Space` / `Enter` |
| Yield to step / to turn | `Shift+Space` / `Ctrl+Enter` |
| Cancel / back | `Esc` |
| Cycle zones (hand → battlefield → stack → graveyards) | `Tab` / `Shift+Tab` |
| Navigate cards / options | arrows, `H J K L` |
| Select / target / toggle | `Enter`, multi-select `Space` |
| Auto-tap toggle | `A` |
| Choice list confirm | `Enter` (list nav with arrows) |
| Game log | `L` |
| Automation menu for selection | `M` |
| Mulligan keep / bottom | `K` / `B` |

Rules: every `ChoiceRequest` type is operable without a pointer device;
focus is always visible; no action requires drag-and-drop (drag has a
keyboard equivalent). Lobby (Leptos) uses standard ARIA navigation.
