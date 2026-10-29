# AGENTS.md — baylee

The map of this repository, for every agent and person working on it. It
names what always applies and where each subject is written down; the rules
themselves live in the documents it points to. Before you change an area,
read its document. `CLAUDE.md` imports this file.

Baylee plays Magic: The Gathering: a deterministic rules engine, the servers
that host it (gateway, agent, one engine process per game, a feedback
service) and a Bevy desktop client. Android, iOS and browser/wasm support are
coming soon and are not current release targets. Accounts live
in Baylee Central, a separate repository (`docs/central-login.md`). It is an
unofficial, non-commercial fan project.

## Always

- **Implementation language (owner, 29.10.2026; all agents).** Everything that
  can be implemented in Rust must be implemented in clean Rust. No shell or
  Python substitutes for automation, orchestration, data preparation, evaluation,
  monitoring or helper programs. Python is allowed only for PyTorch code and
  its directly associated tests; TypeScript only for web applications and their
  web-specific tooling. A Rust replacement also replaces its callers and tests.
  Existing CLI tools may be invoked directly; do not hide new program logic in
  shell pipelines or interpreter one-liners.
- **Gate before you push.** `cargo dev gate` and `cargo dev gate features`
  (with `DATABASE_URL`); `cargo dev land` runs both desktop gates and
  fast-forwards `main`. Wasm is an optional experimental check until browser
  support ships. A change lands after the local gates, not
  after CI (`docs/dev/commands.md`, `docs/releasing.md` §"CI").
- **Development features are local-only.** CI runs no dev-feature matrix;
  `shipped-features` only verifies that test hooks cannot reach shipped builds.
- **Determinism.** The engine and core use the seeded RNG only; no
  `std::time`, `std::random`, `algebraic_*` floats or `HashMap` iteration in
  hot paths; the engine is synchronous (`docs/dev/conventions.md`).
- **Hidden information is unrepresentable** in a view, and the gateway runs
  no rules (`docs/architecture.md`, `docs/protocol.md`).
- **Legal guardrails** bind every change to what a player sees: never copy
  the card-script corpus, no WotC assets (recordings of the game showing its
  card images are the one exception), check the rules we depend on, and
  stop and say so when something is clearly forbidden
  (`docs/legal.md` §"Guardrails for every change").
- **Never run full `codegen` without the corpus**: it rewrites every
  machine-owned card as a stub (`docs/cards.md`).
- **Card work** goes in release order, fixes the reader before a card, plays
  every card in its own test, and marks a batch `Card-Author:` /
  `Review: pending` for a review by another model (`docs/card-agent-brief.md`,
  `docs/cards.md`).
- **Secrets** never go into a file, an argument, a URL or a log; API keys
  come from the owner (`docs/dev/environment.md`, `docs/llm-seat.md`).
- **A client you start for a test, you close**, also on failure, and you
  check that it exited (`docs/dev/conventions.md`).
- **Leave code better than you found it.** Split a large file into a
  directory module (a move-only commit, then a change commit); card files
  are placed by codegen and never moved.
- **UUIDs are v7.** Generated files (`// GENERATED`) change only through
  `cargo run -p xtask -- codegen`.

## Where things are

| Subject | Read |
| --- | --- |
| Commands: build, test, gate, land, xtask, run | `docs/dev/commands.md` |
| Every environment variable, by process | `docs/dev/environment.md` |
| PostgreSQL, the gateway's store, the card catalog | `docs/dev/database.md` |
| How code is written and committed here | `docs/dev/conventions.md` |
| Development and release tools (`cargo dev`, `cargo tools`) | `docs/dev-tools.md` |
| Local PC/GPU monitoring dashboard (Baylee Pulse) | `docs/workstation-monitor.md` |
| Crates, engine, hidden information and seats, the gateway | `docs/architecture.md` |
| Words that mean one thing here | `docs/dictionary.md` |
| The engine's internals (normative) | `docs/engine-internals.md`, `docs/mana-notation.md`, `docs/pending-constraints.md`, `docs/cost-model.md` |
| Verification hooks, performance baselines | `docs/verification-hooks.md`, `docs/perf-baseline.md`, `docs/perf-client.md` |
| Live faults and the mechanics still to build | `docs/observed-faults.md`, `docs/mechanics-roadmap.md` |
| Cards: codegen, layout, ownership, writing one | `docs/cards.md`, `docs/card-agent-brief.md`, `docs/card-dsl.md`, `docs/card-identity.md` |
| Card batches: progress, held back, learnings | `docs/set-progress.md`, `docs/generated-held-back.md`, `docs/llm-learnings.md` |
| Decks, precons, formats | `docs/deck-format.md`, `docs/precons.md`, `docs/banlists-and-formats.md` |
| The house AI, the trained AI, language-model seats | `docs/house-ai.md`, `docs/trained-ai.md`, `docs/ai-coverage-todo.md`, `docs/llm-seat.md`, `docs/llm-protocol.md` |
| Training operations, current run pointers, research findings and agent takeover | `docs/training-runbook.md` |
| The wire protocol and every server route (normative) | `docs/protocol.md` |
| The same routes and sockets as OpenAPI and AsyncAPI, generated (`cargo dev api`, `cargo dev api serve`) | `docs/api/`, `docs/dev/commands.md` |
| Bug reports, the feedback service and its UI | `docs/feedback.md`, `web/feedback/README.md` |
| Baylee Central: sign-in, chat sync, credentials | `docs/central-login.md`, `docs/central-chat-sync.md`, `docs/native-credential-store.md`, `docs/native-transport-security.md`, `docs/password-vault.md` |
| What is kept about people, and for how long | `docs/privacy.md` |
| Licences, assets, the Fan Content Policy, Scryfall | `docs/legal.md`, `NOTICE`, `docs/gateway-legal/` |
| The client (normative) and its standing rules | `docs/client.md`, `crates/baylee-client/CLAUDE.md` |
| The interface's words, translations (Fluent files) | `crates/baylee-client-core/locales/`, `docs/client.md` §"The interface's own words" |
| The client's lobby, keys, mobile | `docs/client-lobby.md`, `docs/keyboard-map.md`, `docs/mobile.md` |
| Releases, CI, deploys | `docs/releasing.md`, `docs/deploy-hooks.md` |
| Designs not built yet | `docs/proposals/` |
| Dated reports, handovers, measurements | `docs/history/` |
