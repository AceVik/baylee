# Preconstructed decks

**Status: [Implemented] (30.09.2026).** Wizards' retail decks, as lists this
repository holds, a status that says which of them this build can play, and
the rule that unlocks one. The owner's plan (30.09): train the house AI's
successor on real decks instead of random ones, offer the same decks to
players among the house decks, and later add community decks from sources
that allow it.

| What | Where |
|---|---|
| the importer, the status, the rule | `xtask/src/precons.rs` (`decks-import`, `decks-status`) |
| "which cards work" | `xtask/src/working.rs` (the trained AI's rule) |
| the lists | `data/decks/precon/<set>/<slug>.txt` |
| the status | `data/decks/precon/STATUS.tsv` |
| the gateway's embedded playable list | `crates/baylee-db/src/precons/generated.rs` |
| the licence | `NOTICE`, `docs/legal.md` clause 11 |

```bash
cargo run -p xtask -- decks-import [--archive <AllDeckFiles.tar.gz>] [--refresh]
cargo run -p xtask -- decks-status [--check]
cargo run -p xtask -- deck-check data/decks/precon/E02/sun-empire.txt
```

## The source

MTGJSON (https://mtgjson.com), MIT-licensed (`NOTICE`). The importer
downloads **one** file, `https://mtgjson.com/api/v5/AllDeckFiles.tar.gz`
(260 MB, all 3065 decks of build `5.3.0+20260929`), into the gitignored
`data/mtgjson-cache/`, and reads it from there on the next run (`--refresh`
downloads it again). It never fetches `DeckList.json` or a per-deck
`decks/<file>.json`: `https://mtgjson.com/robots.txt` says
`Disallow: /api/v5/*.json` to every agent, and the archive is not a `.json`
path. Every source states its licence in `precons::SOURCES` beside the code
that reads it.

## Which decks

A deck type is imported when the product is a deck somebody sits down and
plays as it comes. Every type MTGJSON used on 29.09.2026 has a decision in
`precons::TYPES`; a type that appears later is reported as `NO DECISION` and
not imported until it has one.

**Imported (1203 decks):** Theme Deck (220), Commander Deck (197), Intro Pack
(167), Arena Starter Deck (101), Shandalar Enemy Deck (55), Duel Deck (52),
Welcome Deck (50), Sample Deck (50), Planeswalker Deck (41), World
Championship Deck (32), Event Deck (26), Challenger Deck (22), Enhanced Deck
(20), Starter Deck (20), MTGO Theme Deck (18), Game Night Deck (15), Advanced
Deck (12), Planechase Deck (12), Starter Kit (12), Guild Kit (10), Clash Pack
(9), Archenemy Deck (8), Arena Starter Kit (8), Pioneer Challenger Deck (8),
Pro Tour Deck (8, 1996), Duel Of The Planeswalkers Deck (5), Historic Brawl
Precon Deck (5), Arena Promotional Deck (4), Brawl Deck (4), Spellslinger
Starter Kit (4), Premium Deck (3), MTGO Commander Deck (2), MTGO Duel Deck
(2), Modern Event Deck (1).

Alpha had no fixed precons; the earliest are the 1996 Pro Tour decks, the
1996 intro packs, the 1997 Shandalar decks and the 5th Edition starters.
Planechase and Archenemy decks are imported as their 60 cards: the planar
and scheme decks are not written, because the engine has no zone for them.
A Brawl deck is written with its commander; whether its leader may lead is
the status's question (below).

**Left out, and why:**

| Type | Decks | Why |
|---|---|---|
| Secret Lair Drop | 751 | a collector's drop of a few alternate-art cards (median 5), not a deck |
| Jumpstart | 570 | a 20-card half, a deck only once shuffled with another half |
| MTGO Redemption | 197 | a whole set redeemed for paper cards (121–383), not a deck |
| Deck Builder's Toolkit | 120 | cards and basic lands to build from, not a deck |
| Bundle Land Pack | 103 | the basic lands of a bundle |
| Box Set | 72 | land sets, anniversary and gift boxes: collections, not decks |
| Welcome Booster | 16 | a booster of ten cards |
| Advanced Pack | 8 | an add-on pack of a few cards |
| Demo Deck | 8 | a scripted tutorial of a dozen cards |
| San Diego Comic Con Promos | 7 | a set of promotional cards |
| Halfdeck | 5 | a 30-card half, like Jumpstart |
| Challenge Deck | 3 | an automated opponent with its own rules (Face the Hydra), not a player's deck |
| Dandan Deck | 1 | one library two players share, a variant this engine does not play |
| Enemy Deck | 1 | an empty list |

## The files

One file per deck, `data/decks/precon/<set>/<slug>.txt`: the deck's own set
code (upper case) and its name as lower-case ASCII with `-` between words.
A set code Windows cannot hold as a directory name gets a trailing `_`, and
there is one: Conflux is `CON`, so its decks are in `CON_/`.

Each file is Baylee text (`docs/deck-format.md`), so the builder's Import
reads it as it is:

```
# baylee deck export v1
# name: Sun Empire
# format: freeform
# source: MTGJSON 5.3.0+20260929, https://mtgjson.com, SunEmpire_E02.json
# licence: MIT (Copyright © 2018 – Present, Zach Halpern); see NOTICE
# type: Theme Deck
# set: E02
# released: 2017-11-24
# written by `cargo run -p xtask -- decks-import`; do not edit by hand
1 Aggravated Assault (E02) 25
1 Ancient Brontodon (XLN) 175
SB: …
```

- `CMD:` rows are the commanders, rows like any other; the main deck
  follows, then `SB:` the sideboard. Nothing else of MTGJSON's is kept (no
  tokens, planes, schemes, prices, text).
- Rows are sorted by name, then printing, and identical rows are merged, so
  a source reordering its list changes no file.
- The printing is the product's own: set and collector number, `[es]`/
  `[it]`/`[ja]` for the non-English decks, `*F*`/`*E*` for foil and etched.
  A row that could not say its printing back exactly through `deckrow`
  would be written without it and counted; on 29.09 none needed that.
- A card is matched on its **oracle id** through the ledger
  (`baylee_cards_index::ROWS`) and written under the **ledger's** name —
  never MTGJSON's, which names a reversible card twice
  (`Parhelion II // Parhelion II`) and a meld card together with what it
  melds into (`Bruna, the Fading Light // Brisela, Voice of Nightmares`).
- A card the ledger does not know is written as MTGJSON names it and
  reported, never dropped. On 29.09 that is **18 cards, all in one deck**
  (`FRC/multiverse-reforged`, released after the ledger was last written).
  The status then says the deck is not playable and why (`ledger`).
- The `# source:` line names the MTGJSON build the list was taken from. A
  re-import of an unchanged list leaves its file alone, so a new build
  rewrites only the lists it changed; a deck that left the source is
  removed, but only a file carrying the importer's own marker line is ever
  rewritten or removed.
- The run is deterministic and idempotent: the same archive writes the same
  bytes, and a second run writes nothing. Nothing in a test touches the
  network; the tests build MTGJSON-shaped JSON in memory.

## Which decks are playable: the rule and the status

A deck is **playable** when every card in it passes `precons::verdict`, and
`deck-check` prints the same verdict for any deck file:

1. the name is a card of this pool (`decks::by_name`), else `pool` (a real
   card the ledger knows) or `ledger` (no card at all);
2. it is `Coverage::Implemented`, else `stub`;
3. the engine's test code names it — `card_index("<oracle id>")`, its oracle
   id as a literal, or its `index::` constant, anywhere in the engine's test
   code — else `untested`. This is the trained AI's rule, copied whole from
   `baylee_train::working` (`c42/trained-ai`) into `xtask/src/working.rs`;
   the two copies must not drift until one of them goes;
4. a commander may lead (CR 903.3), and two may lead together, else `leader`.

`decks-status` writes `data/decks/precon/STATUS.tsv`, one line per deck in
key order: `deck` (the key, `<set>/<slug>`), `set`, `type`, `released`,
`name`, `cards` (commanders and sideboard included), `playable` (`yes`/`no`),
`failing` (distinct cards that fail) and `first_failing` (the first three,
each with its reason). It also writes the gateway's embedded list of the
playable decks. Both are a function of the lists, the pool and the engine's
tests, and `the_precon_status_is_the_pools` (an xtask test, so `gate.sh` and
CI's test job run it) fails when either differs from what they say today,
naming the file, the first line that differs and the command to run. A
commit that adds a card, a card test or a precon therefore regenerates both:

```bash
cargo run -p xtask -- decks-status
```

As each set is finished, its precons unlock in that commit.

**On 30.09.2026: 0 of 1203 decks are playable.** The pool has 2749 cards,
2336 of them implemented and 2335 of those tested; the nearest decks miss
four cards (`10E/white-deck-b`, `8ED/speed-scorch`), and nearly every
failing card is simply not in the pool yet (3569 of the first-three
entries say `pool`, 39 `stub`, 1 `ledger`).

## House decks

The playable precons are offered to players on the house-deck page
(Hausdecks), beside the decks this project seeded by hand, as decks of kind
`preconstructed` that belong to nobody. No precon ever needs a migration:

- `decks-status` embeds the playable lists in the gateway
  (`crates/baylee-db/src/precons/generated.rs`, `include_str!` of each
  file), so a deployed gateway needs no file beside it and offers exactly
  what its build plays.
- As the gateway starts, after its migrations, `baylee_db::precons::sync`
  makes the database agree with that list, in one transaction under an
  advisory lock, so two gateways on one database take turns. It first
  compares a stamp of the lists (and of `precons::SYNC_VERSION`) with the
  one it wrote last (`deck_sync`): the same build starting again is one read.
- A list is found again by its key (`deck.source`, unique). A new one
  becomes a deck at version 1. One whose cards changed leaves the state it
  replaces in the deck's history and moves one version on, as a player's
  save does, with the source's build as the change's summary; a copy names
  the version it came from, and that version stays readable. A rename or a
  new description is not a version.
- A precon the build no longer plays — its list left, or a card in it is no
  longer implemented or tested — is **withdrawn**: `offered` goes false, the
  listing stops showing it and a copy of it is refused (410), but the deck
  stays readable and is never deleted, so the copies players took keep
  playing and keep naming their deck. When it is playable again it is
  offered again as the same deck, history and all.
- A deck with no `source` — a player's, or a house deck a migration seeded —
  is never read or written by a sync, and a `CHECK` keeps `source` and
  `offered = false` off a player's deck.
- The listing (`precons::shared_decks`, the gateway's `GET /decks/shared`)
  shows the house's own decks first, then the precons. The client shows
  both on its one house-deck page, unchanged; with none playable today the
  page needs no grouping. Grouping by set and type is the client's next step
  once the list grows long, and is not built.

A precon's description is what the product was: its type, set and release
date (`Theme Deck · E02 · 2017-11-24`).

## For training

The trained-AI session reads, from a checkout of `main`:

- `data/decks/precon/STATUS.tsv` — take the lines with `playable` `yes`;
  `deck` is the file's path under `data/decks/precon/` without `.txt`. Only
  those decks are held to the trained AI's rule; a `no` deck has a card the
  rule refuses and must not be dealt.
- `data/decks/precon/<deck>.txt` — Baylee text. A row's name resolves with
  `baylee_cards::decks::by_name`, the same lookup the game uses (the ledger's
  name for the oracle id, so it is also `baylee_cards_index::row_by_name`).
  `CMD:` rows are the commanders, `SB:` rows the sideboard.
- The header's `# type:`, `# set:` and `# released:` say what the product was;
  `# format:` is `commander` when the deck has a commander, else `freeform`.

The status is regenerated in the same commit as any change to the pool or
the engine's tests, so a checkout's `STATUS.tsv` always describes that
checkout's pool.

## Community decks

Researched on 30.09.2026, **not imported**: nothing reads any of these sites.
The rules the research kept: a site's `robots.txt` and terms are read before
anything else, no bot protection is ever worked around, and no username,
player name or deck author is stored. Where the maker of a database sits in
the EU, its database right (§ 87b UrhG) forbids taking a substantial part
without permission; a US maker has none here (§ 127a UrhG), but its terms
still bind as a contract.

| Source | Allowed | Licence | Access | Attribution | People in the data |
|---|---|---|---|---|---|
| 17lands public datasets | **yes** | CC BY 4.0 | bulk files (gzip CSV) | "17Lands", a link and the licence | none: "anonymized" |
| Archidekt | **unclear**: ask in writing | none stated | undocumented read-only JSON API | staff ask for a link back | every deck has an owner |
| MTGTop8 | **unclear**: ask in writing | none stated | HTML only | none stated | player names |
| magic.gg / Wizards | **no** for automated collection; unclear for a few lists by hand | none (the Fan Content Policy is not a data licence) | HTML articles | the Fan Content Policy notice | player names |
| EDHREC | **no** | none stated | no documented API | — | second-hand decks of Archidekt and Moxfield users |

- **17lands** (https://www.17lands.com/public_datasets): "Unless otherwise
  noted, these data sets are licensed under a Creative Commons Attribution
  4.0 International License", and "We're sharing some of our aggregated data
  in an anonymized fashion". CC BY 4.0 §4 grants the database right as well
  ("the right to extract, reuse, reproduce, and Share all or a substantial
  portion of the contents of the database"). Its usage guidelines say "we
  discourage automated scraping of our API" and point to the dumps, so only
  the dumps are a source; trophy decks sit behind the API and are not in
  them. The data is MTG Arena **Limited** (draft and sealed), not
  constructed lists, and a row carries the drafter's overall win rate:
  anonymised but per person, so only card lists would be kept.
- **Archidekt** (https://archidekt.com/terms, 07.09.2018): "license to access
  the Site solely for your own personal, noncommercial use", "no part of the
  Site may be copied, reproduced, distributed, republished, downloaded … in
  any form or by any means", and no "software or automated agents or scripts
  … to generate automated searches, requests, or queries to the Site". Its
  staff wrote the opposite on its forum ("our API is open and public (as far
  as reading is concerned)", thread 40353, 2019; "You're more than welcome to
  use our API for whatever you want", thread 2832338, 2022), and the terms
  leave "User Content" to the users who made the decks. Only a written
  permission covering reading and republishing would settle it.
- **MTGTop8**: no terms page, no licence, no API, and `robots.txt` answers
  404. Its registrant is in France, so the database right applies, and taking
  the lists systematically is a substantial extraction (§ 87b (1) sentence 2
  UrhG) that needs permission. Decks are titled with the player's name.
- **magic.gg / Wizards**: `robots.txt` allows everything, but the footer
  links Wizards' terms (https://company.wizards.com/en/legal/terms, 10.12.2025),
  whose §2.1 allows use "solely for your individual and non-commercial use"
  and whose §2.2(i) forbids "Data mining: Use any unauthorized means,
  process, or software that accesses, collects, reads, intercepts, monitors,
  data scrapes, including without limitation, agents, robots, scripts, or
  spiders".
- **EDHREC** (https://edhrec.com/terms, 06.08.2024): the same terms as
  Archidekt's; `robots.txt` disallows `/deckpreview/`, where the decks are;
  its JSON host answers 403. Its FAQ: "EDHREC collects deck data from
  Archidekt, Moxfield, and Scryfall", so it is a second-hand source anyway.

**Recommendation.** Only 17lands' dumps are usable today, and they are
Limited data, not decks to offer. For constructed decks, ask Archidekt and
MTGTop8 (the one EU maker here) in writing for a permission that covers
training and republishing; leave EDHREC and Wizards alone. The text and data
mining exception (§ 44b, § 87c (1) no. 4 UrhG) might cover copies made only
for training where a site states no machine-readable reservation, but
§ 44b (2) — "Die Vervielfältigungen sind zu löschen, wenn sie für das Text
und Data Mining nicht mehr erforderlich sind." — means it never covers a
library of decks offered to players.
