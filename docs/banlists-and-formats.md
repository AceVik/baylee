# Banlists & Formats

**Status: [Spec], one part built.** The deck *shapes* exist as one pure
predicate, `baylee_cards::formats::check` (constructed: at least 60 cards
and 4 copies; Highlander: 100 singletons; Commander: one or two leaders
that may lead together, 100 singletons with them, every card inside their
colour identity as the registry records it, back faces included; basic
lands exempt from every copy limit). The trained AI's deck generator calls
it today. The gateway does not yet: its deck validation is still
registry-membership + count rules only, and no banlist exists. The rest of
this document is the design target, not a description of shipped behavior.

Legality lives entirely in the gateway; the engine enforces game rules only.
The deck builder shows the same verdict live, from the same function; the
gateway's is the one that counts.

## The plan (agreed with the owner, 4 November 2026)

**Official lists are not kept by hand.** Scryfall states per card and format
`legal`, `banned`, `restricted` or `not_legal`, and the catalog ingest already
writes that to `card_legalities` (`oracle_id`, `legalities` as JSON). Each
ingest brings bans and set rotation with it.

**Own lists** live in the gateway's store:

- `banlists(id, name, format, builtin, updated_at)` +
  `banlist_entries(banlist_id, oracle_id, status: Banned | Restricted |
  BannedAsCommander)`. `Restricted` is one copy, as in Vintage.
- A list starts empty or as a copy of an official one; users can create, edit
  and copy them.
- A lobby's host picks one list or none. With a list, a deck it rejects cannot
  join; with none, everything may. House-rule overrides per lobby are possible.

**Validation is one pure function** that extends `formats::check`: deck, format
and list in; problems out, each with its card and reason. It is format rules
(size, singleton, color identity incl. MDFC back faces) + commander eligibility
+ the chosen list, and it stays out of the engine. Coverage is its own notice,
not a legality fault: a format-legal deck may still hold a card without a script.

**The client** gets the statuses as a compact, versioned table from the gateway
(as `/pool`), caches it, and calls the same function while a deck is built.

**Filters** are new keys of the shared query language, written once and so
alive in the deck builder, the zone dialog and the card-name chooser alike:
`r:rare`, `s:m21`, `f:modern`, `banned:commander`, `list:"My Commander"`. Every
card search matches the client's language and English, and no other language.

**What to keep straight:**

- Rarity and set belong to a printing, not to a card: in the pool a filter means
  "some printing matches", in a deck it may mean the printing chosen.
- A game freezes the list's version as it starts, so a refresh cannot invalidate
  a running game. A saved deck may become illegal later: it is told so, and
  saving is never refused.
- Scryfall's terms need a look before this grows (`docs/legal.md` could not
  reach their API page at its last check).

**Order:** the data and the filter keys; live validation in the deck builder;
own lists and the lobby's check.

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
