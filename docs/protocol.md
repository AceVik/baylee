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

## Client preferences (`/settings`)

Keys and standing orders follow the **account**, not the machine: a player who
rebinds confirm at home finds it rebound at a friend's table.

- `GET /settings` → the stored object, or `{}` for an account that has never
  saved any. Never a 404: the client's own defaults are the right answer, and
  making it tell two failures apart buys nothing.
- `PUT /settings` replaces it. The body *is* the preferences object — there is
  no wrapper, because there is nothing else to say about it.

The gateway keeps the blob **opaque** and checks exactly two things: that it is
a JSON object, and that it is under 16 KiB. It cannot check more, and should
not: knowing what a keymap is would mean linking `baylee-client-core`, which
is the client's brain and pulls in the engine behind it — the one dependency
the gateway does not have. The second reason is deployment order: a client
that learns to remember a new preference must not need a gateway release
before it can store it.

The shape is `baylee_client_core::prefs::Preferences` — a `Keymap`, the phase
rail's `PhaseOrders`, and the `AutoRules` switches — and every field of it is
`#[serde(default)]`, so a blob written by an older or newer client still loads
with the rest defaulted rather than costing a player their bindings. A
corrupt blob decodes to the defaults rather than to an error, for the same
reason: preferences are a convenience, and a player mid-upgrade should get a
working keymap rather than a screen that will not open.

Not stored here: the preview's size, the interface language, and the gateway
address. Those are properties of a *device*, they stay in the client's own
local store, and putting them in the account would mean a phone and a desktop
fighting over one number.

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

## The gateway runs no rules

A game does not live in the gateway. It lives in an engine process that an
**agent** started and that dialled the gateway back; the gateway routes between
that process and the seats, and links neither `baylee-engine` nor
`baylee-gamehost`. The circle:

```
POST /lobby/games ─┐
                   v
              gateway ── StartEngine{game_id, engine_token, gateway_url} ──> agent
                   ^                                                          │ spawn
                   │                                                          v
                   └──────── EngineHello{game_id, token} ──── baylee-engine-server
                   │
                   ├── GameSetup / SeatAttached / SeatDetached / SeatFrame ──>
                   <── SeatFrame / GameEnded ──────────────────────────────────
                   │
                   └── the seat sockets, unchanged
```

Three sockets, three secrets, and no two of them are interchangeable:

| socket | who opens it | what proves it |
| --- | --- | --- |
| `GET /agent/ws` | an agent | `BAYLEE_AGENT_TOKEN`, from the gateway's own configuration |
| `GET /engine/ws` | one engine process | a token issued for exactly one game |
| `GET /games/{id}/ws` | a player | a seat token, scoped to one seat of one game |

A player's token opens neither of the first two. The engine token is minted
when the game is ordered, handed to the agent, and passed to the process it
starts as a command-line argument — it never leaves the machine the agent runs
on, and it is worth one game.

**`SeatFrame{seat, envelope}`** is what makes the routing possible without the
gateway understanding a word of the game: it nests an *encoded* player-facing
`Envelope` and tags it with a seat, so the gateway forwards the bytes it was
handed without decoding or re-encoding them. The player-facing protocol keeps
exactly the shape it had; a client cannot tell it is being proxied.

Two things moved out of the gateway with the rules:

- **The decision clock.** It has to sit where `awaiting_seat()` and `seq()` can
  be read, which is now the engine process. It is anchored to the sequence
  number it was armed at, so one seat's expired clock can never take another
  seat's decision, and it does not run for a seat with no socket — a player who
  walked away is not on a clock they cannot see.
- **The panic boundary.** One process per game *is* the boundary, so the
  `catch_unwind` the gateway used to wrap every rules call around is gone. A
  rules path that panics takes down exactly one game, and the agent reports the
  exit.

Standing answers travel as JSON (`baylee_protocol::StandingAnswer`) rather than
as actions: the gateway cannot build a `PlayerAction` any more, and the engine
has never heard of an account. `EngineRunner::standing_answers` is the other
half of that seam, and the gateway's own test reads its payload back with
exactly that function — a wrong handle fails silently, so nothing here may be
checked by eye alone.

A seat's frames are dropped in the *engine* while it has no socket, not one hop
later at the gateway. That is what keeps a seat's own opening payload first on
its wire: the frames another seat's arrival produced for a player who was not
there yet are gone before they can overtake it. Nothing is lost by it — every
attach pumps, and a pump re-sends the current view to every seat that is
present.

Losing the engine link ends the game. The state lives in that process and
nowhere else, so a link that closes before `GameEnded` is a game that cannot be
continued; the gateway marks it over rather than leaving a table that will
never move again.

### Running one

```bash
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway
BAYLEE_AGENT_TOKEN=<the same> ./target/debug/baylee-agent
```

The agent finds `baylee-engine-server` beside itself (`BAYLEE_ENGINE_BIN`
overrides), reconnects with backoff, and takes `BAYLEE_GATEWAY`,
`BAYLEE_AGENT_NAME` and `BAYLEE_AGENT_CAPACITY` (0 = no limit). The gateway
tells an engine to dial `BAYLEE_ENGINE_URL`, which defaults to
`ws://127.0.0.1:{PORT}/engine/ws` — right for a single box, wrong the moment an
agent runs somewhere else. With no agent connected, `POST /lobby/games` answers
`503`: there is nothing to run the game, and handing out a seat token for a
table that will never start would be worse.

## From an account to a seat

The websocket below is opened with a *seat token*, and there is exactly one
way to get one. The client walks it itself now (`crates/baylee-client/src/lobby.rs`),
which is what turned this from a curl recipe into a contract:

| step | call | answer |
| --- | --- | --- |
| sign up | `POST /auth/register` `{email, display_name, password}` | `{"ok":true}` |
| sign in | `POST /auth/login` `{email, password}` | `{token, expires_at}` |
| decks | `GET /decks` | `[{id, name, cards, sideboard, commander}]` |
| one deck | `GET /decks/{id}` | `{id, name, cards:[…], sideboard:[…], commander}` |
| save a deck | `POST /decks` `{name, cards:["N Card Name"], sideboard, commander}` | `{deck_id}` |
| edit one | `PUT /decks/{id}` — same body | `204` |
| throw one away | `DELETE /decks/{id}` | `204` |
| the card pool | `GET /pool?lang=de` | `{total, pool_hash, lang, has_text, cards:[…]}` |
| a card's printings | `GET /printings?card=42` | `{card, english_name, from_catalog, printings:[…]}` |
| tables | `GET /lobby/games` | `[{id, name, host, yours, state, seats:[…]}]` |
| open one | `POST /lobby/games` `{deck_id, mode:"ai"\|"open", seats, name}` | `{game_id, seat, seat_token}` |
| sit down | `POST /lobby/games/{id}/join` `{deck_id, seat?}` | `{game_id, seat, seat_token}` |
| arrange a chair | `POST /lobby/games/{id}/seats/{seat}` `{kind?, ai?, deck_id?}` | the seat |
| stand up | `POST /lobby/games/{id}/leave` | `204` |

Everything but the two auth calls, `/auth/config`, `/pool` and `/printings`
takes `Authorization: Bearer <token>`. A refusal is `{"error":"…"}` with a
status, and the string is written to be shown to a player as-is — the lobby
does.

`/pool` is the deck builder's card list, and one of the two routes with no
account behind it: it is reference data about what this build can play, the
same for everybody, and a sign-in page that cannot show it is worse than a
public one.
Each row carries the registry `index` (the rules identity a saved deck line
resolves to), the printed characteristics, and `coverage`
(`implemented` / `partial` / `unimplemented`) with the author's `note` — a
builder that offered a stub as though it played would be lying. `name`,
`type_line` and `oracle_text` come from the catalog when the gateway has one,
in the language `lang` asks for, falling back field by field to English;
`english_name` never does, because that is what a deck row is written with.
`has_text` says whether rules text was available at all. The whole pool is a
few hundred rows, so it is sent whole and filtered in the client; `total` and
`pool_hash` are there for the day it is not.

Each row also carries `oracle_id` and, when a catalog is configured,
`alt_names` — every *other* name the card is printed under, across every
language the catalog holds. That is what lets the builder show **one row per
card** and still find it when a player types the name on the card in their
hand: searching printings instead would list the same card once per set it
appeared in, which is the wrong answer to "do I own this". `alt_names` is
omitted when empty, because for two hundred cards in a dozen languages it is
otherwise the largest field in the response.

`GET /printings?card=<registry index>` is the other half of that trade: the
list stays short by naming cards, and a player who wants a particular piece of
cardboard asks for it. The answer is every printing of that card the catalog
knows — set, collector number, language, rarity, artist, frame effects, and
`finishes`, which is the list `docs/deck-format.md`'s `*F*` / `*E*` may name.
Newest set first.

Without a catalog it is **not** an error: the answer carries the one printing
codegen recorded, in English, plain, and `from_catalog:false`. A picker that
had to handle a `503` here would need a second code path for every gateway
without a database; one that always gets at least one printing does not, and
the deck row it writes is the same row either way. A card outside the registry
is a `404` — the question was about something this build cannot play.

Two distinctions the client has to keep straight. A `401` on a *signed*
request means the account token is spent (sign out and start over); a `401` on
the sign-in form means the password was wrong. And the seat token is not the
account token: it is scoped to one seat at one game, so losing it costs a game
rather than an account, which is why it is the only one that ever appears in a
URL.

`seat` in the answer is a hint. The table states which chair this is, in the
opening payload below, and the client believes the table.

### Rooms

A table with more than two chairs is a **room**, and the whole of it is
arranged before anyone plays. `POST /lobby/games` with `seats: 2..=4` opens
one; the host takes the first chair and every other chair starts open. `name`
is what the table is called in the list, and may be empty — the listing then
falls back to the host's display name.

`GET /lobby/games` describes a room completely, because deciding whether to sit
down means seeing what is already at the table:

```json
{ "id": "…", "name": "Kitchen table", "host": "viktor", "yours": false,
  "state": "waiting",
  "seats": [ { "seat": 0, "kind": "human", "ai": null, "taken": true,
               "player": "viktor", "you": false, "deck": "Mono-Green",
               "ready": true },
             { "seat": 1, "kind": "ai", "ai": "sharp", "taken": false,
               "player": null, "you": false, "deck": "", "ready": true } ] }
```

Never an account id — a `player` is a display name, and `you` / `yours` answer
"is that me" without the listing having to carry anyone's account. `kind` is
`"human"` or `"ai"`, and `ai` names a difficulty from `AIProfile::NAMED`
(`novice`, `steady`, `sharp`); one that does not exist is a `400` rather than a
quiet default, because a table that plays at another level than it advertises
is worse than one that says no.

Two authorities, and they do not overlap. The **host** arranges the table —
`POST …/seats/{seat}` with `kind` and `ai` — but not a chair someone is sitting
in (`409`). Every **player** sets exactly one thing, the deck they themselves
will play, through the same route with `deck_id`; a deck that is not theirs is
a `403`, and so is any attempt to arrange a seat that is not their own.

**There is no start button.** A room starts the moment every chair is ready: a
human chair with an account and a deck in it, or an AI chair, which is ready as
soon as it is configured (an AI the host gave no deck plays the house deck). It
is the same rule the two-seat open table has always followed when its second
player sat down, so a host who has given up waiting simply hands the last chair
to the AI, and that is the start.

`POST …/leave` frees a chair. A guest leaving leaves the chair open and the
room standing; the **host** leaving closes the room, because without them
nobody can arrange it, and a table that can never start should not be
advertised as one that might.

**A seat token is not always usable yet.** `mode:"ai"` and a join both order an
engine before answering, so the socket can be opened at once — it simply waits
(up to 30 s) for that engine to attach before the first frame arrives. That
readiness is *sticky*, and has to be: with a warm engine binary the attach
happens within a few milliseconds of the order, well before the player who
placed it has finished dialling, and a socket that could only be told as it
happened would sit out the whole timeout waiting for something that had
already arrived. An
`"open"` table orders nothing: it holds the seat and waits for a second player,
and a socket opened against it is accepted and then closed with nothing on it,
because there is no game yet to describe. The host of an open table has to wait
for its `state` to turn `"playing"` — there is nothing to push the news on, so
the lobby re-reads `GET /lobby/games` every two seconds until it does.

## The opening payload, and a client that is not the server

`GameStaticMsg{game_id, view_version, static_json}` is the one message a
networked client cannot do without and could not previously receive. The seat
roster and the print table are built from the `GamePreset`, which a client
never sees — so `LocalHost` built them out of the preset it happened to be
holding, and nothing on the wire carried them. A socket client had a print
table of length zero and every `PrintRef` named no card.

`Session::game_static_envelope` now produces it and it is always the first
thing on a seat's wire: `EngineRunner` sends it on `SeatAttached` (and again on
a reconnect, since that is a new socket), the engine-server's dev harness on
`CreateGame`, `Join` and `Resume`. `LocalHost` takes it off the wire too, through the same
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
