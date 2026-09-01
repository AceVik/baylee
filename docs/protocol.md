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

That seam has since paid for itself three times, and every time the feature
was filed under v2 before anyone checked what it actually touched: the copy
target re-choice (CR 707.10c), the agreed draw (CR 104.4a), and attacking
planeswalkers all shipped as `Pending`/`PlayerAction` changes with **no
proto change at all**. Before scheduling something behind protocol v2,
check whether it needs the wire or only the taxonomy the wire carries.

## The AI is a client (view version 6)

`HeuristicAgent::act` took `&Engine` and could read the whole `GameState`
— the opponent's hand, every library, every face-down permanent. The
crate's own header called that "convention, not enforcement". It now takes
`(&PlayerView, &Pending)`, the same pair a networked seat gets, so the
leak is not policed but absent: `baylee-ai` no longer depends on
`baylee-engine`'s state at all, and the acceptance-deck soak plays every
one of its games through the filtered view.

Two fields moved into the view to make that possible, and both are things
a human client wanted anyway: `SeatView::mana_pool` (floating mana is
public at a real table, and the seat deciding whether to tap another land
needs it) and `PublicObject::mana_value` (what a spell on the stack
actually cost). That is **`VIEW_VERSION` 5 → 6**.

`play_game` moved from `baylee-ai` to `baylee-gamehost::harness` for the
same reason: building a view takes the engine, which is the boundary the
agent may not cross.

## Combat enumerates its own answers

`Pending::ChooseAttackers` carries `attackers` beside `defenders`, and
`Pending::ChooseBlockers` carries a `BlockOption` per creature that may
block, naming the attackers it may block. Evasion is a pairing question,
so a flat list of "creatures that may block" would be wrong for every
flier on the table. `CombatCandidates` — the client's own guess at both —
is gone. No proto change: the taxonomy travels as JSON.

## Attacking a planeswalker (view version 3)

`PlayerAction::DeclareAttackers` carries `(ObjectId, Defender)` pairs
rather than `(ObjectId, PlayerId)`, where `Defender` is
`Player(PlayerId)` or `Planeswalker(ObjectId)`. `Pending::ChooseAttackers`
now carries the legal `defenders` list, and the engine validates a
declaration against exactly that list — "which planeswalkers may I attack"
(CR 508.1a) is a rules question, and a client re-deriving it would be a
second, divergent implementation of the rule. `AttackerView::defending`
changed the same way, which is what took **`VIEW_VERSION` from 2 to 3**;
a client checks that on `HelloAck` and refuses a host it cannot render.

`Defender` is an externally tagged serde enum over two transparent ids, so
the JSON is `{"Player":0}` or `{"Planeswalker":1234}` where a v2 payload
had a bare `0`. A v2 client therefore fails to deserialise rather than
silently reading a planeswalker attack as an attack on seat 0 — which is
the point of the version bump being mandatory rather than advisory.

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
