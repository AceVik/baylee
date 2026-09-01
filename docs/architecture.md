# Architecture

Master plan (Blueprint v1.3). baylee is a cargo workspace:

- **baylee-core** — wasm-safe foundations: typed ids, colors, type/subtype
  constants (generated), mana notation + `mana!`, game presets + print table.
- **baylee-engine** — deterministic rules kernel. Arena object model, layer
  projection with generation-cached characteristics, event pipeline
  (propose → replacement → apply → triggers → journal), APNAP priority +
  SBA fixpoint, enumerated-legality `ChoiceRequest` API (the ONLY way the
  game advances), compositional costs, unusual-casting subsystem, format
  modifiers, Rhai `ScriptedModifier` for custom modes, endless-loop
  detection (identical snapshot hash inside a decision-free segment).
- **baylee-cards-dsl / baylee-cards / baylee-cards-codegen** — compiled card
  registry keyed by Scryfall ids; per-card files with human-verifiable
  headers; codegen from Scryfall bulk + forge-reference index.
- **baylee-protocol** — protobuf WS messages; per-player filtered views;
  every card carries `{object_id, card_index, print_ref}`.
- **baylee-engine-server** — one process per game (up to 8 human/AI seats).
  Attached to a gateway with `--attach/--game/--token`; the argument-free
  listening mode is a loopback dev harness with no authentication.
- **baylee-agent** — starts those processes on behalf of a gateway. Depends on
  `baylee-protocol` and nothing else in the workspace: an agent has never heard
  of a card, a rule or a deck, which is why it can run where the gateway does
  not.
- **baylee-gateway** (M4) — [Implemented today]: axum + JSON-file store
  (parking_lot mutex, debounced background writer): auth (Argon2id,
  hashed bearer tokens), decks/validation, lobbies, and **routing** between
  seat sockets and one engine process per game — it links neither the engine
  nor gamehost (see "The gateway runs no rules" in docs/protocol.md).
  [Spec target]: SeaORM 2 + PostgreSQL 18, catalog
  search, banlists, image proxy/cache. The store move goes
  via SQLite first (see docs/protocol.md roadmap).
- **frontends** (M5) — Leptos lobby, Bevy 2.5D game client (WASM + native +
  mobile), reusable `ui-widgets` / `client-presentation` split (MMO-ready).

Key invariants: determinism (seeded RNG, ordered structures, journaled);
engine never interprets `PrintRef`; hidden information is unrepresentable
in views; automation rules are server-side standing orders.

Milestones: M0 foundations → M1 engine core → M2 layers/triggers/DSL freeze
→ M2.5 acceptance decks (~195 cards, waves W1–W8) → M3 engine-server + AI
→ M4 gateway → M5 frontends → M6 formats/mobile/polish.
