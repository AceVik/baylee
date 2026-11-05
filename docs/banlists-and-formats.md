# Banlists & Formats

**Status: [Implemented], steps 1 to 3 of the plan below.** The deck *shapes*
are one pure predicate, `baylee_cards::formats::check` (constructed: at least
60 cards and 4 copies; Highlander: 100 singletons; Commander: one or two leaders
that may lead together, 100 singletons with them, every card inside their
colour identity as the registry records it, back faces included; basic
lands exempt from every copy limit), and the whole question is one pure function
beside it, `formats::validate` (§"Built: the check"). The trained AI's deck
generator calls `check`; the deck builder and the gateway call `validate`.

Legality lives entirely in the gateway; the engine enforces game rules only and
has no format, list or legality code. The deck builder shows the same verdict
live, from the same function; the gateway's is the one that counts.

House-rule overrides per lobby are out of scope for now: a lobby's host picks one
list or none, and that is all a lobby can say about legality.

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
have them (`docs/client.md` §"What the search box understands"). `list:` came with
own lists (§"Built: own lists and the lobby"), in the same code: `list:"My
Commander"` reads the lists the client holds, official and its own, by name in any
case, and says so where it holds none (`docs/client.md` §"Ban lists").

**The search.** Every card search matches the client's language and English
and no other, `/catalog/search`'s rules text included (`card_search_text`, one
vector per face and language; `docs/dev/database.md`).

## Built: the check

Step 2 of the plan, 4 November 2026.

`baylee_cards::formats::validate` takes a deck (`DeckCards`: main, sideboard,
leaders, one entry per copy), a format, the pool's table and a list, and answers a
`Report`: every `Problem`, the cards with no script, and what could not be asked.
It is pure and deterministic, reads nothing but its arguments and the compiled
registry, and took 12 µs for sixty cards in a debug build. Its words are
`baylee_cardtext`'s, so a refusal crosses the wire as data and each client words
it.

| Problem | Comes from |
| --- | --- |
| `too_few`, `wrong_size`, `sideboard_too_big` | the format's shape; the copy limit runs over deck and sideboard (CR 100.4a), a sideboard is fifteen, a Commander deck has none (CR 903.5e) |
| `too_many_copies` | the shape's limit, per card over every row and printing, basics aside |
| `leaders`, `not_a_leader`, `cannot_lead_together`, `leaders_not_taken` | commander eligibility, by `decks::leader_of` |
| `outside_identity` | every face of the card, a modal double-faced card's back included |
| `banned`, `restricted`, `not_legal` | the format's verdict in the table; restricted means one copy |
| `list_banned`, `list_restricted`, `list_banned_as_commander` | the chosen list |
| `unknown_card` | a card this build has no definition for |

**A list replaces the format's bans, not its pool.** Without a list the format's
own verdicts stand. A list says what is banned and restricted instead, because that
is what taking a card off a copy of an official list means; `not_legal` still
stands, so a Modern list with one card unbanned is not every card ever printed. A
list for `freeform` keeps to no format at all.

**What a format's shape is** is `formats::SHAPES`. `brawl`, `standardbrawl`,
`competitivebrawl`, `oathbreaker` and `tlr` are left out on purpose: a Brawl deck
may be led by a planeswalker (CR 903.12c) and Standard Brawl is sixty cards, so
asked as Commander they would refuse decks that are fine. Their verdicts are read
and the report says the shape was not asked. Pauper Commander's rule about its
commander's rarity is not asked either; the table's verdicts are.

**A deck is called legal only when everything was asked.** `Report::standing` is
`Legal`, `Illegal`, or `Unknown`: no table (offline, no catalog), a format the table
does not hold, a card it says nothing about, or a shape not known. A deck builder
that has rows waiting for the pool or lost a card is `Unknown` as well.

**Coverage is its own notice.** `Report::unscripted` lists the cards with no script;
a format-legal deck may hold one, and it is never a problem.

**In the deck builder** `DeckBuilder` holds a format and a list, runs the check after
every edit (`deckbuilder/legality.rs`) and keeps the answer, because a row reads it as
it is drawn; there is no debounce, the check is that cheap. `DeckBuilder::overview` is
what the foot draws and `deckcheck` words a problem once, for the builder and a
refusal alike. The format is saved with the deck; the list is a question put to it,
held for the session and not saved.

**Saving a deck is never refused for being illegal.** `POST /decks` asks what a game
could not be built from (a line that does not read, a card this pool lacks, 250 cards
a list) and nothing a format asks: four copies and who may lead moved out of it into
`validate`. A deck is told how it stands in the builder, and on `GET /decks`
(`legality`), which the deck tile reads, so one that became illegal after a ban says
so without being opened.

## Built: own lists and the lobby

Step 3 of the plan, 4 November 2026. Routes: `docs/protocol.md` §"Ban lists".

**The store.** `banlists(id, name, format, builtin, owner, version, updated_at)` and
`banlist_entries(banlist_id, oracle_id, status)`, migration 28. An official list is a
`builtin` row with no owner, one a format, brought in step with `card_legalities`
when the catalog's stamp moves (`banlists::keep_official`), so copying, picking and
freezing one is what it is for any other list. It holds the pool's cards and no
other: a deck here holds none, and a card outside the pool stays out of what the
gateway serves of the feed (`docs/legal.md` §3). A list keys on the oracle id, so
an own list outlives a build. `version` counts up with the cards and the format.

**An own list** is its owner's and gone with the account. A list that is not the
caller's reads as no list at all on every route, and nothing says whose one is: the
owner is personal data (`docs/privacy.md` §"Ban lists"). Limits: 50 lists an account,
2000 cards a list, a name of 60 characters.

**The lobby.** The host picks one list or none, and the format (`POST …/list`). With a
list, a deck it rejects cannot sit down at any door (join, the chair, ready, start)
and its player reads why: the refusal is a `409` carrying every `Problem`, which the
client words in the sheet over whatever screen the deck came from. With none,
everything may. A room holds its own copy of the list, read again before every check
while it waits. Anyone who can see the table sees that it has a list; its name and
cards are for those at the table.

**Frozen.** The start writes the version the decks passed onto the game and nothing
reads the list for it again, so neither an edit, a deletion nor an ingest reaches a
running game. The gateway runs no rules: a card banned mid-game is played out.

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
  join; with none, everything may. House-rule overrides per lobby are out of
  scope for now.

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
