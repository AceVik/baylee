# Protocol

Binary WebSocket protocol (protobuf, `baylee-protocol`, wasm-safe).
Schema: `crates/baylee-protocol/proto/baylee/v1/transport.proto`.

## Printings (which art the client draws)

Rules identity and *presentation* identity are two different things, and a
deck may legitimately hold the same card in several printings — three Islands
from three sets, one of them foil. So a card reference on the wire is a pair:

- `card_index` — the rules identity. The engine reads this and nothing else.
- `print_ref` — an index into a per-game print table. **The engine never
  interprets it**; it copies it onto the object at setup and carries it
  through every zone change, so the client is told exactly which of the
  duplicate cards it is looking at.

The table itself travels once, in `GameStatic.prints`: `scryfall_id`, `lang`
and `finish` per entry, which is everything the client needs to key the
Scryfall CDN. The path end to end is

```
DeckEntry { card, print }        (preset, per copy in the deck)
  → CardRef { index, print }     (engine, on the object)
    → CardIdentity { index, print, face }   (view, per seat)
      → GameStatic.prints[print]            (what to actually draw)
```

`baylee-gamehost`'s view tests follow one deck entry along that path,
including through a zone change, because a printing that silently reset to
entry 0 would be invisible in every other test — the game would play
perfectly and show the wrong art.

## Remembered answers (`/automation`)

A seat can tell the engine "always say yes to this ability" — that is
`PlayerAction::SetStandingAnswer`, addressed by `AbilityRef { card, index }`.
The handle names a *card's* ability and nothing about the table it is set at,
which is what makes it a preference the gateway can keep:

- `GET /automation` → `{ "answers": [{ "card", "ability", "yes" }, …] }`
- `PUT /automation` replaces the list (at most 512 entries; card indices are
  checked against the registry, duplicates collapsed, order normalised).

The list is replayed into a seat as `SetStandingAnswer` actions when its
socket opens — before the first pump, so a question the player never wanted
to see is already covered when the opening hand arrives. Setting a standing
answer is not a game action (the pending question stays exactly as it was),
so a reconnect simply restates what the seat already has.

Storing a handle the registry does not know is refused rather than kept: it
could never fire, and it would fail silently — the seat would just be asked a
question it believed it had answered for good.

## v0 (M0)
Transport handshake + preset transfer:
`Hello{protocol_version, card_pool_hash}` / `HelloAck`, `JoinGame`,
`ResumeGame{last_seq}`, `GamePresetMsg`, `Heartbeat`, `Error`, wrapped in
an `Envelope` oneof. Card references are `{card_index, print_ref}`;
`card_pool_hash` invalidates client caches.

## v1 (M3, shipped 2026-08-29)
`CreateGame` / `GameCreated`, `ChoiceRequest{game_id, seq,
pending_json}`, `PlayerActionMsg{game_id, seat_token, action_json}`,
`StateDelta` (reserved). **Design decision (documented):** the full
engine choice taxonomy (`Pending`, `PlayerAction`) travels as
**serde_json payloads inside the protobuf frames** — the wire stays
binary protobuf, but the taxonomy evolves without proto churn. A typed
protobuf mapping of the taxonomy is a protocol v2 item, together with
per-player hidden-information filtering (`FullView`/`Delta`), timers
(`TimeExtensionRequest/Vote/Result`), `SetAutomationRules`, dev-mode
`DevCommand`, and spectator streams.

That seam has since paid for itself twice, and both times the feature was
filed under v2 before anyone checked what it actually touched: the copy
target re-choice (CR 707.10c) and the agreed draw (CR 104.4a) both shipped
as new `Pending`/`PlayerAction` variants with **no proto change at all**.
Before scheduling something behind protocol v2, check whether it needs the
wire or only the taxonomy the wire carries.

`ResumeGame{last_seq}` is answered as of 2026-08-31, by both servers.
`Session::resume` is deliberately **read-only**, where `pump` advances the
game: rebuilding a client through `pump` would drive every AI seat forward
as a side effect of somebody reconnecting. A client that is already current
(`last_seq >= seq`) is sent nothing rather than a redundant re-render. The
gateway now also uses that snapshot when a seat's socket lags behind the
broadcast, instead of dropping the player.

`HouseRules::decision_timeout_secs` is enforced as of 2026-08-31. It was
carried from the preset through `baylee-gamehost` into the proto and read by
nobody. The clock lives in the **gateway**, never in the engine or the
session: the rules kernel is deterministic, and a session that timed itself
would replay differently on every machine. `Session` only answers two
clock-free questions — who owes an answer, and what a legal answer would be —
and the gateway anchors a deadline to the session's sequence number so it
restarts when the game moves rather than whenever the task wakes up. On
expiry the house agent answers for the seat, because it is legal for every
`Pending`; a timeout that produced an illegal action would leave the same
seat stuck on the same question forever.

Still open here: `time_extension_votes`, which unlike everything above does
need the wire — `TimeExtensionRequest/Vote/Result` are not in the proto — and
`reconnect_window_secs`, which needs the gateway to hold a seat open rather
than only rebuild it.

Server: `baylee-engine-server` (tokio + tokio-tungstenite), one process,
games as `Session`s — engine + human seat + auto-driven AI seats
(baylee-ai). E2E smoke: real binary over a real socket
(`crates/baylee-engine-server/tests/e2e.rs`).
