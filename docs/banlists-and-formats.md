# Banlists & Formats

**Status: [Spec], two parts built.** The deck *shapes* exist as one pure
predicate, `baylee_cards::formats::check` (constructed: at least 60 cards
and 4 copies; Highlander: 100 singletons; Commander: one or two leaders
that may lead together, 100 singletons with them, every card inside their
colour identity as the registry records it, back faces included; basic
lands exempt from every copy limit). The trained AI's deck generator calls
it today. The gateway does not yet: its deck validation is still
registry-membership + count rules only, and no banlist exists. The data and
the filter keys are built too (§"Built: the data and the filter keys"); the
rest of this document is the design target, not a description of shipped
behavior.

Legality lives entirely in the gateway; the engine enforces game rules only.
The deck builder shows the same verdict live, from the same function; the
gateway's is the one that counts.

## Built: the data and the filter keys

Step 1 of the plan below, 4 November 2026.

**What the ingest really provides.** Every printing in Scryfall's feed carries
the whole map, and `card_legalities` keeps all of it, one row per card. On the
feed of 4.11.2026 that is 23 formats: `alchemy`, `brawl`, `commander`,
`competitivebrawl`, `duel`, `future`, `gladiator`, `historic`, `legacy`,
`modern`, `oathbreaker`, `oldschool`, `pauper`, `paupercommander`, `penny`,
`pioneer`, `predh`, `premodern`, `standard`, `standardbrawl`, `timeless`, `tlr`,
`vintage`. Nothing names them here: the list follows the feed, so a new one
needs no release (a client shows one it has no words for by its key).

A card's printings agree on 22 of the 23. **Old School admits printings**, so
779 of 39 287 cards say `legal` or `restricted` on an old printing and
`not_legal` on a reprint, and the row used to be whichever came last. Each
format now keeps the strongest word (`banned`, `restricted`, `legal`,
`not_legal`: `rows::VERDICT_ORDER`), because `not_legal` says only that this
printing is out. An ingest forgets the old rows when its first batch is in
hand, or a lifted ban would win over the new `legal` for ever.

**The table.** `GET /pool/facts` (`docs/protocol.md` §"Pool facts") sends, for
each pool card the catalog knows, its verdicts and its distinct `(set, rarity)`
pairs, keyed on the registry index. `baylee_cardtext::PoolFacts` is the shape
both ends read and the seam step 2 stands on: `PoolFacts::of(index)` gives a
`CardFacts` whose `legality(format)` is what the validation function will ask,
and `baylee-cardtext` links serde and nothing else, so `baylee_cards::formats`
can take it without the engine ever seeing it. Measured on the same feed, for
the 12 542 pool cards: 394 569 bytes (98 kB gzipped; the gateway does not
compress), 43 887 pairs in 659 sets, 407 distinct verdict strings, under
0.5 MB decoded.

`/pool` and `/printings` were the two places a client could have read this
from, and neither serves: `/pool` carries no rarity, set or format, and
`/printings` carries set and rarity for one card at a time, for a picker.

Scryfall's terms ("You may not simply repackage, republish, or proxy Scryfall
data", read 4.11.2026) are why it stops there: `docs/legal.md` §3.

**The keys.** `r:`, `s:`, `f:`, `banned:` and `restricted:` are written once, in
`cardquery`, so the deck builder, the zone dialog and the card-name chooser all
have them (`docs/client.md` §"What the search box understands"). `list:` waits
for own lists.

**The search.** Every card search matches the client's language and English
and no other, `/catalog/search`'s rules text included (`card_search_text`, one
vector per face and language; `docs/dev/database.md`).

Not built: validation (`formats::check` knows no list yet), own lists, and the
lobby's check.

## The plan (agreed with the owner, 4 November 2026)

**Official lists are not kept by hand.** Scryfall states per printing and format
`legal`, `banned`, `restricted` or `not_legal`, and the catalog ingest merges
that into `card_legalities` (`oracle_id`, `legalities` as JSON). Each ingest
brings bans and set rotation with it.

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
  "some printing matches", in a deck it may mean the printing chosen. Built:
  no list searched today shows a printing the table can name, so "some
  printing" is the one reading (`docs/client.md` §"What the search box
  understands").
- A game freezes the list's version as it starts, so a refresh cannot invalidate
  a running game. A saved deck may become illegal later: it is told so, and
  saving is never refused.
- Scryfall's terms were read for this table on 4.11.2026 and bind it to what
  the client's features use (`docs/legal.md` §3); a new field needs that look
  again.

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
