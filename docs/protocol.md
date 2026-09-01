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

The table travels in `GameStatic.prints`: `scryfall_id`, `lang` and `finish`
per entry, which is everything the client needs to key the Scryfall CDN. It is
sent **per seat**, and an entry the seat has not earned is `None` — the table
is shared by the whole game and deduplicated per card, so a seat handed all of
it would be handed every decklist at the table. A seat knows its own deck's
printings from the start and earns the rest by seeing the cards; the host
re-sends the payload, ahead of the view that needs it, when one is earned. The
hole keeps the index, because the index *is* the `PrintRef`. The path end to
end is

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

## Capabilities, not a dev flag

`GamePreset` carried `dev_mode: bool` — "enables `DevCommand`s (never in
normal lobbies)". Nothing ever read it, and it arrived **inbound** in
`GamePresetMsg`, so the one thing it could do was let whoever opened the
socket ask to be trusted. Field 3 is now `reserved`.

`SeatSpec::capabilities` replaces it: per seat, `Default` is nothing, and
the host is the only thing that grants any. `dev_commands` is what the
engine checks — `Engine::dev_state_mut(seat)` returns `None` for every seat
without it, where the old `state_mut_dev()` took no seat and asked nobody.
`see_hidden` is reserved for a judge or replay view and is granted by
nothing yet.

The gateway builds lobby presets with no capabilities at all, and
`gamehost::preset` has the test that says a wire preset cannot grant itself
one.

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

## The opening payload, and a client that is not the server

`GameStaticMsg{game_id, view_version, static_json}` is the one message a
networked client cannot do without and could not previously receive. The seat
roster and the print table are built from the `GamePreset`, which a client
never sees — so `LocalHost` built them out of the preset it happened to be
holding, and nothing on the wire carried them. A socket client had a print
table of length zero and every `PrintRef` named no card.

`Session::game_static_envelope` now produces it and both servers send it
first: the gateway straight down each seat socket at connect (and again on a
reconnect, since that is a new socket), the engine-server on `CreateGame`,
`Join` and `Resume`. `LocalHost` takes it off the wire too, through the same
`host_message` decoder the networked host uses — a field only the in-process
path filled in would be missing in exactly the case nobody tests at a desk.

`view_version` is duplicated outside `static_json` on purpose: a client that
cannot render this version must be able to say so without first decoding the
structure whose shape is what changed.

Sending it exposed the one hidden-information leak the view design had no
field for. The print table is deduplicated **per card across the whole game**,
so `prints` is the union of every decklist at the table — a seat that received
it could subtract its own deck and read the opponent's. `prints` is therefore
`Vec<Option<PrintEntry>>` now, filled in per seat: `own_prints` seeds a seat
with the printings of its own deck, sideboard, opening hand and starting
battlefield, and `Session::reveal` marks the rest as the seat's views show
them, re-sending `GameStaticMsg` **before** the view that points at the new
entry. A hole rather than a shorter list, because the index is the `PrintRef`
every object in every view carries. That is **`VIEW_VERSION` 6 → 7**.

What stays readable is the table's *length* — how many distinct cards are in
play across every deck, and therefore how many the opponent plays that you do
not. That is a deck-diversity number, not a decklist, and hiding it would mean
padding the table with entries no object points at.

`GameStatic::print()` already returned `Option<&PrintEntry>`, so almost no
client code changed; what did change is that `textures.rs` no longer preloads
the whole table (it was prefetching art for every card in the opponent's deck)
and `cardtext.rs` re-asks the catalog when the table grows.

`Session::describe(game_id, names)` supplies the parts the rules kernel has
never heard of. `Session::snapshot` deliberately reveals nothing: it rebuilds a
state a `pump` already showed that seat, so the printings were earned there.
The gateway's lag resync does re-send the payload around it, because the update
that granted a printing travelled the very broadcast that socket just dropped
messages from.

`NetworkHost` (`crates/baylee-client/src/net.rs`) is the second
`DuelHost`. It is handed a `SeatTicket{gateway, game_id, seat, seat_token}`
and connects to `/games/{id}/ws?token=…`; `poll` drains the socket without
blocking and `submit` sends a `PlayerActionMsg`. The token is *not* repeated
in each frame — the socket is already bound to one seat of one game, and the
seat comes from the token rather than from anything the client says later.
The seat in the ticket is only a hint: the host believes `GameStatic.your_seat`.

`reconnect()` re-dials and sends `ResumeGame{last_seq}`. It is deliberately
not automatic: only the application knows whether a player is still sitting
there, and a host that redialled by itself would hammer a gateway that is
down.

`ewebsock` is the transport for the same reason `ehttp` is the HTTP client —
one API that is a background thread natively and the browser's own
`WebSocket` on wasm. The browser's socket handle is neither `Send` nor
`Sync` and a Bevy resource must be both, so on wasm it is wrapped in
`SendWrapper`: a run-time thread check that panics rather than an
`unsafe impl` that would be a claim nobody could enforce.

Where a ticket comes from: `BAYLEE_GAME` + `BAYLEE_SEAT_TOKEN` in the
environment natively, `?game=…&token=…` in the page URL in a browser — which
is how a web lobby hands a player to the table. Without one the client plays
solo against the house AI, in process, exactly as before. A ticket that is
present but unusable is a hard stop, not a quiet fall back to solo play:
somebody is waiting at that table.

Still not done here: the client has no lobby of its own. Logging in, listing
games and picking a deck are HTTP calls somebody still has to make; the
client only knows what to do once it has the ticket.

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
