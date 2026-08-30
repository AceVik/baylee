# Game Client (M5)

Bevy 2.5D duel client. Three crates, split by what can be tested without a GPU.

| Crate | Contains | Depends on a renderer? |
|---|---|---|
| `baylee-view` | The wire view: projected characteristics, per-seat filtering | no — not even the rules kernel |
| `baylee-client-core` | Table layout, board model, interaction state machine, image policy | no |
| `baylee-client` | Bevy plugin: 3D stage, overlay, input, texture cache | yes |

## The wire view

A client cannot run the layer system, so `PublicObject` carries **projected**
characteristics (name, P/T, types, colours, keywords), not the printed card.
An anthem, a clone, or an animated land arrives already resolved.

Hidden information is unrepresentable rather than merely omitted: library
contents have no field, another seat's hand is a count, and a face-down
permanent's `card` is `None` for anyone not entitled to look.

`GameStatic` (seats + print table) is sent once; `PlayerView` is a full
snapshot per change, which makes reconnects trivial.

## 2.5D

Cards are textured quads lying on a 3D table; everything a player *reads*
(prompt, hand, stack, life, threat lines) is a 2D overlay. Tapping is a
rotation, and focusing an opponent is a camera move rather than a re-layout.

## Eight seats

- Seats sit on a ring, local seat at the near edge, opponents clockwise **in
  turn order** — the player on your left acts after you.
- Pods get unequal space; the local pod is always largest, and focusing an
  opponent borrows from the other opponents, never from you.
- Lanes fan when crowded and report overflow when even fanning stops being
  legible.

## Grouping and the token summary

Identical permanents draw as one card with a `×N` badge. Two independent
guards keep that honest:

- objects merge only when every visible property matches (name, P/T, damage,
  counters, tap state, controller);
- objects with individual identity never merge, however identical they look —
  attacking, blocking, enchanted, equipped, or targeted by the stack.

Each seat also gets a text chip row (`12× 1/1 Soldier · 3× Treasure`) and a
one-line threat read (power ready, blockers, open mana, cards in hand), which
is what makes an unfocused pod useful at eight seats.

## Images and memory

Scryfall CDN, keyed by printing id — no API call is needed to render a board.
Board cards are fetched `small` (146×204); only the focused card is fetched
`normal`. That is the difference between ~36 MB and ~400 MB for a large table.
A byte-budgeted LRU (`TextureBudget`) decides evictions; the browser budget is
deliberately below the desktop one and the ordering is checked at compile time.

## Hosts

The renderer never touches a socket. It talks to a `DuelHost`:

- `LocalHost` runs an engine in-process (solo play, embedded duels, tests) and
  goes through the same protobuf envelopes a socket would carry;
- a networked host drains its transport into the same `HostMessage` stream.

## Embedding (the open-world plan)

`DuelPlugin` creates no window and no schedule of its own. An application adds
it, installs a host, and sends `DuelCommand::Open`; the duel takes the screen
and returns it on `Close`, reporting through `DuelReport`. `DuelSet::{Sync,
Input, Present}` let the host application order its own systems around it.

## Verification

- `cargo test -p baylee-client --test duel_flow` plays real games headlessly
  through the client's own path (host → view → board model → interaction).
- The wasm CI job builds `baylee-client` for `wasm32-unknown-unknown`.
- Browser entropy needs both `.cargo/config.toml`'s `getrandom_backend` cfg and
  the `wasm_js` feature; either alone is not enough.
