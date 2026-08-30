# Banlists & Formats

**Status: [Spec].** No banlist, format-legality, singleton, or color-
identity check exists in the gateway today — deck validation is
registry-membership + count rules only. This document is the design
target, not a description of shipped behavior.

Legality lives entirely in the gateway; the engine enforces game rules only.

- `banlists(id, name, format, builtin, updated_at)` +
  `banlist_entries(banlist_id, oracle_id, status: Banned | BannedAsCommander)`.
- Builtin presets (official Commander, official Highlander) are seeded and
  refreshable; users can create/edit/copy banlists; each lobby picks one
  (or none). House-rule overrides per lobby are possible.
- Deck validation = format rules (size, singleton, color identity incl.
  MDFC back faces) + commander eligibility + chosen banlist + coverage
  (all cards implemented?).

## Commander eligibility & partners

Metadata derived at codegen (`CommanderRule`, `PartnerKind` on `CardDef`):
legendary creatures and cards whose oracle text says "can be your
commander" are eligible; partner families (`Partner`, `Partner with X`,
`Choose a Background`, `Friends forever`, `Doctor's companion`) allow
exactly two commanders. The engine's command zone is a list: N commanders,
per-commander tax and combat damage.

## House rules (per game preset)

Free-first mulligan (default on), loop policy (`RunOnceThenBreak` default
| `CompRulesDraw`), decision timeout (600 s default), reconnect window
(60 s default), time-extension votes, timing normalization (anti-tell
auto-pass), takebacks. Bundled, versioned, shareable.
