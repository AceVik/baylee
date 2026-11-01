# Architecture

Baylee is a Cargo workspace: a deterministic rules engine, the servers that
host it (gateway, agent, one engine process per game, the feedback service),
and the Bevy client for desktop, mobile and the browser. Accounts live in
Baylee Central, a separate repository (`docs/central-login.md`). The
normative detail of each part is in the document named beside it; this page
is the frame they hang in.

Invariants that hold everywhere: determinism (seeded RNG, ordered
structures, a journal), the engine never interprets a `PrintRef`, hidden
information is unrepresentable in a view, and the gateway runs no rules.

## Crates

- Graph: core → engine → gamehost → engine-server; gamehost builds view and links `baylee-ai`; engine's choice taxonomy feeds client-core and `baylee-ai` (plays from the view; uses client-core's payment matching); core, view, protocol → client-core → client (Bevy); client-core links protocol only for shared rules such as `names::username`. protocol → gateway (axum), agent (`tokio::process`, no rules). Each arrow drops a capability; test at the lowest layer possible.
- `baylee-view` depends only on `baylee-core` ids, serde.
- `baylee-client-core` is the client brain, knows no renderer, holds most client tests. Only `baylee-client` needs a GPU.
- `baylee-protocol`: protobuf `Envelope`; `Pending`/`PlayerAction` ride as `serde_json`; shared by `LocalHost` and both servers.
- `baylee-deckio` (core → deckio → client-core): deck documents in four formats (Baylee text, JSON, YAML, Moxfield text), auto-detect, and a registry of deck-link sources that answer an instruction or a fetch plan; pure, never fetches (Moxfield is its text export only). `docs/deck-format.md`.
- `baylee-cardtext` sits under core, links only serde: `/catalog/text` wire shape and sentence pairing (`pick`, `align`, `verify`, `split_cost`) for catalog and client.
- Must build for `wasm32-unknown-unknown`: `baylee-cardtext`, `-core`, `-deckio`, `-protocol`, `-seal`, `-view`, `-client-core`, `-client`.

## Engine

`docs/engine-internals.md` is normative.

- Exposes essentially `pending()`, `apply(player, action)`, `state()`, `journal()`, `snapshot_hash()`; during the opening-hand window every seat decides at once: `pending()` shows the lowest open seat, `pending_for(seat)`/`awaited()` are queries (`engine/mulligan.rs`). Advances only via `apply`, which validates against what `Pending` enumerated. Never add action methods. Combat options come from `Pending::ChooseAttackers`/`ChooseBlockers`, not the client.
- For training and evaluation lookahead only, `fork_for(seat, DeckKnowledge::Known, policy_seed)` returns a disposable copy of the whole engine with every card `seat` cannot see dealt again from the policy seed, new ids for them, an independent random stream and a journal that withholds what `seat` was not shown; anything it cannot fork faithfully is a `ForkRefusal`. `Engine` itself is not `Clone`. Invariant: for fixed seat-visible facts and policy seed the fork's hidden zones and future randomness are the same whatever the real hidden cards and RNG, and forking or advancing a fork leaves the real engine's state, hash and RNG unchanged. It covers the hidden arrangement, not history: a public card's `version` still counts its hidden moves (London mulligan, Brainstorm). `Session::fork_for` wraps it in a `SimulationSession` (gamehost, under its forwarded `simulation` feature): the same chairs, the table's own agent views and house (`session/table.rs`), house agents reseeded from the policy seed, and no record, log or envelopes. Not for live play over the gateway: it exists only under the engine's `simulation` feature, which `cargo dev gate shipped-features` keeps out of every hosting build. `docs/engine-internals.md` §"Forking a simulation".
- Layers: cached projection, one `u64` generation compare. Events: propose → replacement → apply → journal → triggers. SBAs: fixpoint before each priority grant. Loops: Brent over `loop_signature`, never `snapshot_hash`. `docs/engine-internals.md` is normative.
- The cleanup step discards first (CR 514.1, "until end of turn" effects still apply), then ends the turn's effects (514.2), then checks once (514.3a). `Engine::cleanup` (`Due`/`Checking`/`Open`) remembers which, because the check and a window look alike. A state-based action or trigger there gives the active player priority, and a round of passes on an empty stack begins another cleanup step. What the check performed is noted where it happens (`cleanup_check_acted`), never read off the journal (704.5n and 704.5q journal nothing). No standing order in client-core passes a cleanup window. `docs/engine-internals.md` §"The cleanup step checks once".
- Every change of control is a layer-2 `GainControl` effect (`resolve::gain_control`, CR 613.1b), even one that lasts the game; `base_controller` is only the default controller, written where an object arrives. Never write a controller by hand. Every `ContinuousEffect` names its `origin` (`Static`, CR 611.3, or `Resolution`, 611.2); a static's controller, like every `ReplacementEntry`'s, is projection output restamped to its source's current controller each refresh (last known once the source has left), so "you" follows a stolen source (CR 109.5). A player leaving follows CR 800.4a–c (`sba::eliminate_player`, `docs/engine-internals.md` §"A player leaving the game").
- Deterministic: seeded ChaCha8, no `HashMap` iteration in hot paths; `std::time`, `std::random`, `algebraic_*` floats banned in engine and core. Synchronous; async only as transport (engine-server, gateway, agent).
- No card text in the engine: abilities are `AbilityRef { card, index }`, reserved indices (`SPELL`, `ENTERS`, …) down from `u32::MAX`; it also keys a seat's standing answers, which the client keeps in its preferences (`ability_orders`).
- An `AbilityList` carries its `PrintedFace`, as do the view's `rules` fields: a copy shows the copied card's text. Never recover a face from a list's address; identical lists are merged constants.

## Hidden information and seats

- New offline neural consumers can opt into `Session::view_for_with_context`
  and `observe_view_with_context`: the ordinary authorized agent view plus
  the same owner-only casting/target context that sockets receive. These add
  no clock, teammate sharing or hidden-state access. Existing view methods
  remain unchanged for archived model/data contracts.
- Payment-context consumers separately opt into
  `Session::view_for_with_payment_context` and
  `observe_view_with_payment_context`. Their optional owner-only
  `PlayerView.announced_cast` names the in-progress spell through X/payment
  and nested mana-color choices, including suspended miracle payment. It is
  observation, not cancellation permission: `casting` retains its existing
  meaning and `CancelCast` can remain illegal during a nested question.
  Default, socket and existing `with_context` projections leave the new field
  absent (`None`, omitted from serialization), preserving archived Encoder-7
  bytes. Encoder 8 consumes this opt-in context offline; live seats must wait
  for an explicit wire capability rather than infer it from `casting`.
- The view carries projected characteristics; clients never run layers.
- Experimental native announcement consumers separately opt into
  `Session::view_for_with_announcement_context` and
  `observe_view_with_announcement_context`. These extend payment context with
  `announced_x`: absent before the X choice, `Some(0)` for settled zero. The
  resource owner and its authorized CR 722 controller may observe it; unrelated
  seats may not. The read-only projection follows active and suspended casts,
  including miracle and nested mana choices, without granting cancellation.
  Default, socket, Encoder-7 and Encoder-8 projections remain unchanged. No
  encoder or model consumes this experimental field yet.
- Hidden information is unrepresentable: libraries and other hands are counts; a face-down `card` is `None` unless entitled. `crates/baylee-gamehost/src/view/tests/hidden.rs` (and its siblings) tests each guarantee; add one per new one. One deliberate exception: in debug builds only (`cfg(debug_assertions)`), an LLM seat's reasoning (`AiLog`) is forwarded to every seat at the table as a sparring tool (owner, 06.10.2026; `docs/protocol.md` §"An AI seat's reasoning"); release builds drop it.
- Bump `VIEW_VERSION` (`crates/baylee-view/src/lib.rs`) on any breaking view change; never restate its value.
- The game log (`crates/baylee-gamehost/src/log.rs`) tells each seat its own lines, naming an object only as that seat's view would; an object the log never saw is `Hidden` to all. Every `StateDelta` carries a view and a `LogTail { from, entries }` in `log_json`; several frames may repeat a view and `seq`, so clients read the log from every frame and append by `from`. Each entry carries the host's time, which the caller sets (`Session::tell_time`, `EngineRunner::tell_time`). The Session reads no clock, because gamehost also runs in the client's `LocalHost`, on wasm too. AI agents are never handed it. `docs/protocol.md` §"The game log".
- A teammate's hand is in a seat's view only while its owner shares it with that seat, both are on one team and both are still in a game that is going. Sharing is a seat setting (`SeatSettingMsg` → `Session::seat_setting`), not a `PlayerAction`: no journal, hash or clock. An AI chair accepts a teammate's request at once and is shown nothing; `Session::agent_view` is the one view an agent answers from. `docs/protocol.md` §"A teammate's hand".
- A per-ability policy never acts silently: where it made the difference the engine journals `GameEvent::AutoAnswered`, and `Session` keeps each seat's latest 16 as `PlayerView::policy_acts`, numbered over the game, cleared by the seat's own answer (not by a clock), and sent to that seat alone. The log writes no line for it. `docs/protocol.md` §"What a policy answered".
- Granted abilities are offered under `choice::granted_ability(n)` (`GRANTED_ABILITY` is slot 0, up to `GRANTED_SLOTS` = 8); `PublicObject::granted_mana` names the output. Offer and projection share `effects::granted_activated` and `baylee_cards_dsl::simple_mana`; no second path. `docs/protocol.md` §"Granted mana".
- `GameStatic.prints` is `Option<PrintEntry>` per index; the index is the `PrintRef` objects point at, so hide with `None`, never shorten the list. A seat gets its deck's printings plus cards seen; `Session` re-sends before the view needing it.
- AI plays only through `HeuristicAgent::act(&PlayerView, &Pending)`, never an `Engine`/`GameState`. (An in-process training or evaluation harness may search on `Engine::fork_for` copies, which hold none of the real hidden cards; that is not a hosted seat's path.) Scouting only for a current `SeatKind::Ai` (`scouting::request`); reports are never serialized, sent over the protocol, merged into views or prints, or retained. The harness uses the same guarded adapter as a hosted AI. `docs/house-ai.md`.
- `SeatKind::Driven`: AI seat taken by a socket (`Session::take_over`; `release` returns it to the retained agent). View senders ask `answers_over_socket()`, never "is human".
- `SeatKind::StandIn`: house answers for an absent player (`Session::stand_in`; `SeatAttached` → `hand_back`). Each awaited seat has one clock (in the mulligan window every deciding seat is awaited): `Deadline::Decide` (answers once) or, socketless, `Deadline::StandIn` (`reconnect_window_secs`, hands the chair over) only while another player's socket is at the table; with nobody else there the game pauses (no clock, no house move) under the table's `Deadline::Hold` (24 h, then the house concedes). `SeatDetached.left` (`POST /lobby/depart`, a client quitting on purpose) stands in at once, or ends the game with no player left. A waiting room keeps a gone player's chair `BAYLEE_ROOM_GRACE_SECS` (60) after their last socket closed (`gateway/src/departure.rs`). `docs/protocol.md` §"Leaving, and losing the connection".
- An away chair keeps `is_ai` false (`SeatIdentity.away`). The roster rides in `GameStatic`; any chair change marks every seat's roster stale.
- Engine-server harness: `JoinGame.seat_token` is a seat number; joining an AI chair takes it over, disconnecting returns it. Each socket sends only its own seat's envelopes.

## The gateway runs no rules

`docs/protocol.md` is normative.

- An agent spawns one `baylee-engine-server` per game, which dials the gateway. gateway→agent `StartEngine`; engine→gateway `EngineHello`/`SeatFrame`/`GameEnded`/`GameRecordChunk`/`RecordFlushed`; gateway→engine `GameSetup`/`SeatAttached`/`SeatFrame`/`FlushRecord`; seat sockets forwarded byte for byte.
- An engine must attach within 30 seconds. Failed setup, pre-hello exit and lost agent ownership close the exact assignment. An agent control connection has a 90-second silence deadline; on disconnect or SIGTERM/Ctrl-C the agent stops and reaps its own engines before redialling/exiting. Player socket disconnects retain their separate reconnect/hold semantics.
- The gateway links neither `baylee-engine` nor `baylee-gamehost`; its e2e tests pull `baylee-engine`, `baylee-engine-server`, `baylee-gamehost` (to replay a record), `baylee-client-core` (to build a report as the client does), `baylee-view` and `baylee-feedback` as dev-dependencies only. No agent → `POST /lobby/games` is 503.
- `SeatFrame { seat, envelope }` nests an encoded player `Envelope` the gateway never decodes; keep that shape.
- A replay is an engine job, not a game (`replays.rs`, `docs/protocol.md` §"Replays"): `StartEngine` with a `replay-…` id, then `ProjectRecord` (the stored record, which goes only to that engine) and `TapeChunk`s back. `records::for_projection` has one caller (`replays.rs`), `for_seated` one (`report.rs`); the seat comes from `game_record_seat`, never the request.
- Secrets: `BAYLEE_AGENT_TOKEN` on `/agent/ws`, per-game token on `/engine/ws`, seat token on `/games/{id}/ws`; not interchangeable. A secret never rides in a socket's address (#294): the session or seat token buys a single-use ticket over `POST /ws-ticket` (`Authorization`), and `/lobby/ws` and `/games/{id}/ws` open with `?ticket=` (`wsticket::Tickets`, memory only, never logged). After its secret each peer states `PROTOCOL_VERSION` (a hello field for agent and engine; `?protocol=` on the seat socket, built only by `baylee_protocol::seat_socket_path`, which takes the ticket), and the gateway refuses a mismatch in one sentence (`version_refusal`), so gateway, agent, engine and clients ship as one build. A seat socket is bounded in size (`MAX_SEAT_FRAME`) and rate (`seatrate::Allowance`, GCRA, measured against a real client in #284); a flood is closed with 1008, never dropped frame by frame. `docs/protocol.md` §"Which side checks the protocol".
- The decision clocks run in the engine (`EngineRunner::clocks`, `crates/baylee-engine-server/src/lib.rs`), at most one per awaited seat, each anchored to `Session::asked_at(seat)`, a `decision_seq` (questions, not frames), never for a socketless seat, whose frames the engine (not the gateway) drops. Losing the engine link ends the game.
- Nothing runs before the curtain: the table opens (`Curtain`, sent last in its batch) when every human seat has sent `SeatReady` or `CURTAIN_SECS` after `GameSetup`; until then attaches get `GameStatic` and their view, no question, no clock, no AI move, and early actions are dropped. Every clock is anchored at curtain-up. Deploy the engine-server before clients. `docs/protocol.md` §"The curtain".
- One process per game is the panic boundary; the hosting path wraps no rules call in `catch_unwind`.
- Gateway e2e tests spawn real gateways (own schema each, pool of two; CI has `postgres:18-alpine`) with the engine in-process (`EngineRunner`); `e2e_processes` is `#[ignore]`d.
