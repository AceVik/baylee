# baylee

A high-performance, memory-efficient Magic: The Gathering platform written in
Rust: a deterministic rules engine (one process per game, up to 8 players),
a gateway service (accounts, decks, lobbies, image cache), and WASM clients
(Leptos lobby, Bevy 2.5D game table).

**Status: M4 shipped** (gateway: accounts, decks, lobby, hosted games) +
M5 game client (Bevy, vs-AI). See `docs/architecture.md` for the master plan
and `AGENTS.md` for build/test commands. Docs carry per-section status
markers: **[Implemented]** = code exists, **[Spec]** = design target only.

## Workspace layout

| Crate | Purpose |
|---|---|
| `baylee-core` | Shared foundations: ids, colors, types/subtypes, mana, presets (wasm-safe) |
| `baylee-protocol` | Binary WS protocol (protobuf, wasm-safe) |
| `baylee-engine` | Deterministic rules kernel (no I/O, no async) |
| `baylee-cards-dsl` | Card authoring framework (data model + builders) |
| `baylee-cards` | Compiled card registry (one file per card) |
| `baylee-cards-codegen` | Scryfall/catalog/forge-reference code generation |
| `baylee-ai` | Heuristic AI controllers (difficulty profiles) |
| `baylee-engine-server` | Binary: one process per game, WS transport; attaches to a gateway |
| `baylee-agent` | Binary: starts engine processes for a gateway (protocol only — no rules, no cards) |
| `baylee-gateway` | Binary: accounts, decks, lobby, and routing between seats and engines |
| `baylee-view` | Per-seat wire view (projected characteristics, hidden-info filtered) |
| `baylee-catalog` | Card text in PostgreSQL: Scryfall bulk ingest, i18n lookup, search |
| `baylee-client-core` | Renderer-agnostic client brain: layout, board model, interaction, image policy |
| `baylee-client` | Bevy 2.5D duel client (native + browser); see `docs/client.md` |

## Legal

Unofficial, non-commercial fan project (see `NOTICE` and `docs/legal.md`).
Code licensed AGPL-3.0-only.
