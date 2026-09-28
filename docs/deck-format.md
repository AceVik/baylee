# Deck Import/Export Format

**Status: [Implemented].** The row grammar below is
`crates/baylee-core/src/deckrow.rs`, and it is what the gateway stores, what
the deck builder writes and what an import reads — one parser, so a deck that
round-trips through a text file is the deck that never left. The documents
around it — four formats, their detection, and a registry of deck links — are
`crates/baylee-deckio`, and the deck builder's Import and Export dialogs are
their one user (`baylee-client-core/src/deckbuilder/transfer.rs`, drawn by
`baylee-client/src/buildui/transfer.rs`).

`baylee-deckio` is pure: it links `baylee-core`, `serde`, `serde_json`,
`serde-saphyr` and `thiserror`, builds for `wasm32-unknown-unknown`, knows no
renderer and opens no connection. Everything it reads was pasted by the
player; everything it writes is handed back as a string for the client to copy
or save. **The gateway never sees a document and never fetches a third party on
a caller's behalf** (`docs/privacy.md`; #270 by analogy): the deck the player
then saves is an ordinary `POST /decks` of rows.

## The row

```
1 Lightning Bolt
1 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d
```

- `(SET) number` = set code + collector number; `[xx]` = language;
  `scryfall=` = printing UUID; `# …` = a note (at most 500 characters).
- Finish markers: `*F*` foil, `*E*` etched, `*H*` holographic, `*G*` glitter,
  `*S*` galaxy, `*N*` non-foil.
- A count is 1 to 1,000,000.

Everything after the name is optional and the groups narrow independently: a
row may say only "the German one", only "foil", or nothing at all. **A row
that names nothing is the old form** — `4 Lightning Bolt` — and still means
what it always meant, which is why every deck saved before printings existed
loads unchanged.

The groups may come in any order, because none of them can be mistaken for
another and insisting on an order would only make hand-written lists fail.
Two rules keep a card name from being eaten:

- a bare number is a collector number **only** when a set code stands in front
  of it, so `1 Borrowing 100,000 Arrows` keeps its arrows;
- a parenthesis is a set code only when it holds three to five alphanumerics,
  so `Erase (Not the Urza's Legacy One)` keeps its parenthetical.

Writing is the inverse of reading: `Display` on a row produces a string that
parses back to the same row, and the default finish is not written, so a row
that said `*N*` comes back plain and means the same thing.

## One document, four formats

Every format reads into, and writes from, one `Document`:

| Field | Meaning |
|---|---|
| `version` | `1` (`baylee_deckio::VERSION`); a reader refuses any other |
| `name`, `format` | the deck's name and format, both optional |
| `cards[]` | one entry per row: `zone` (`main`, `side`, `commander`, `maybe`; default `main`), `count`, `name`, `set`, `collector_number`, `lang`, `finish`, `scryfall_id`, `note` |

A commander is one entry in `zone: commander`; the stored deck (rows plus a
list of commander names) keeps its commander among the main rows, and
`Document::stored` / `Document::from_stored` translate between the two, so a
commander is neither lost nor counted twice. Every card entry must survive a
round trip through the row grammar, or the document is refused and the
refusal names the entry.

| Format | Key | File | Lossless | Reads | Writes |
|---|---|---|---|---|---|
| Baylee text | `baylee` | `.txt` | no | the row grammar with zone prefixes and `# name:` / `# format:` headers | the same, starting `# baylee deck export v1` |
| JSON | `json` | `.json` | **yes** | the document above | the document above, pretty, nulls written |
| YAML | `yaml` | `.yaml` | **yes** | the document above | the document above |
| Moxfield text | `moxfield` | `.txt` | no | Moxfield's text export (below) | text Moxfield's importer reads |

### Baylee text

```
# baylee deck export v1
# name: Aminatou's Blink
# format: commander
CMD: 1 Aminatou, the Fateshifter (C18) 37
1 Lightning Bolt (M11) 149 [de] *F*
SB: 1 Karakas (EMA) 240 [de]
MB: 1 Brainstorm
```

Prefixes: none = main, `SB:` sideboard, `CMD:` commander, `MB:` maybeboard. The text reader also takes section headers
(`Deck`, `Sideboard`, `Commander`, `Maybeboard`, …; any case, a trailing `:`
or a leading `//`), `4x` counts, Arena's `About` / `Name …` lines, and blank
lines; a line it cannot read is skipped and listed in the report, never
silently dropped.

What it cannot write: a collector number without a set code (the grammar
reads a bare number as part of the name). Everything else is in the row.

### JSON

```json
{
  "version": 1,
  "name": "Aminatou's Blink",
  "format": "commander",
  "cards": [
    { "zone": "commander", "count": 1, "name": "Aminatou, the Fateshifter",
      "set": "C18", "collector_number": "37", "lang": null, "finish": null,
      "scryfall_id": null, "note": null },
    { "zone": "main", "count": 1, "name": "Lightning Bolt", "set": "M11",
      "collector_number": "149", "lang": "de", "finish": "foil",
      "scryfall_id": "e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d", "note": null }
  ]
}
```

### YAML

The same document (a missing key takes its default; the writer writes every
key, `null` included):

```yaml
version: 1
name: Aminatou's Blink
cards:
- zone: commander
  count: 1
  name: Aminatou, the Fateshifter
  set: C18
  collector_number: '37'
- count: 1
  name: Lightning Bolt
  finish: foil
```

The YAML parser is `serde-saphyr` (maintained, pure Rust, no `unsafe`
dependency tree; `serde_yaml` is archived and `serde_yml` carries
RUSTSEC-2025-0068). Its default budget refuses alias bombs.

### Finishes in JSON and YAML

The wire words are `baylee_deckio`'s own, so the files do not change if
`baylee_core::preset::Finish` is renamed; case is ignored on reading.

| `Finish` | Word | Text marker |
|---|---|---|
| `Normal` | `nonfoil` | `*N*` (not written) |
| `Foil` | `foil` | `*F*` |
| `Etched` | `etched` | `*E*` |
| `Holographic` | `holographic` | `*H*` |
| `Glitter` | `glitter` | `*G*` |
| `Galaxy` | `galaxy` | `*S*` |

`null` (or a missing key) means the row names no finish. `nonfoil` is read as
the same thing, because the row grammar does not write the default.

## Moxfield

Moxfield has no public API and stands behind bot protection, so Baylee never
fetches `moxfield.com`: no proxy, no borrowed user agent, not from the client
and not from the gateway. Moxfield support is its **text export**, read and
written. A test reads a real export (`crates/baylee-deckio/tests/fixtures/`:
129 rows, a `SIDEBOARD:` split, double-faced names, `*F*`, collector numbers
such as `196p`, `DDR-7` and `250`).

```
1 Archangel Avacyn / Avacyn, the Purifier (SOI) 5
1 Sol Ring (40K) 245 *F*

SIDEBOARD:
1 Pyroblast (ICE) 213
```

- **Double-faced names**: Moxfield writes `A / B`; Baylee's (and Scryfall's)
  whole name is `A // B`. A lone ` / ` in a name is read as ` // `, and the
  writer turns ` // ` back into ` / `. The builder resolves a whole name
  through the pool's `alt_names`, and writes the whole name for every
  double-faced card.
- **Zones**: the reader follows `SIDEBOARD:`, `COMMANDER:` and `MAYBEBOARD:`
  headers (and the other text headers above). The writer puts commanders in a
  `COMMANDER:` block first, then a blank line, then the main deck, then
  `SIDEBOARD:`. Moxfield's own export has no commander block in the sample we
  hold; that its importer reads one is **unverified**.
- **What Moxfield text cannot hold, and the export therefore leaves out and
  says so**: the deck's name, languages, Scryfall ids, notes, the maybeboard,
  a collector number without a set, and the finishes Moxfield has no marker
  for (holographic, glitter, galaxy: written as plain rows). An export never
  claims a finish it did not write; `Written::losses` lists every kind with
  its row count, and the dialog shows it.

### A pasted link

A Moxfield deck link (`https://moxfield.com/decks/<id>`, with or without the
scheme or `www.`) is recognised and answered with an instruction, in the
player's language (`Phrase::ImportMoxfieldLink`): open the deck on Moxfield →
*More* → *Export* → *Copy for Moxfield*, and paste that. Any other `http(s)`
link is answered with "Baylee cannot read decks from <host>. Export the deck
there as a text list and paste the list here", and never fetched.

The recogniser is a registry (`baylee_deckio::source`): a `Source` looks at a
link and answers either `Answer::Instruction(…)` (what the player must do) or
`Answer::Fetch(FetchPlan { url, format })` (where a client could fetch the
text itself, and which format it will be in). Only Moxfield's instruction is
implemented. A future source is one `impl Source` added to `SOURCES`, an
`Instruction` or a `FetchPlan`, and — for a fetch — a client-side fetch that
honours the same size limit; until a source with a sanctioned, public
endpoint exists, the client answers a `Fetch` as "not fetched" rather than
fetch. The gateway takes no part in either.

## Detection

`baylee_deckio::import` answers a pasted text in this order:

1. over 256 KiB: refused;
2. a known deck link: its source's answer;
3. any other `http(s)://` link: refused, not fetched;
4. each format *sniffs* the text (`No`, `Plain`, `Likely`, `Certain`) and the
   highest wins, ties going to the earlier of JSON, YAML, Baylee text,
   Moxfield text. JSON is certain on a leading `{`; YAML on `---` or a first
   line of `version:`; Baylee text on its magic line, likely with zone
   prefixes; Moxfield text likely on a header, `4x` counts or ` / ` names. A
   plain list is Baylee text, which reads Moxfield's plain rows the same way.

The dialog says which format it read the text as before anything is taken.

## Limits

| Limit | Value |
|---|---|
| Pasted or read text | 256 KiB (`MAX_DOCUMENT_BYTES`) |
| Rows (entries) | 1,000 (`MAX_ROWS`) |
| A card or deck name | 200 characters, no control characters |
| A count | 1,000,000 (`deckrow::MAX_COUNT`) |
| A note | 500 characters (`deckrow::MAX_NOTE`) |

The text readers stop at the first row past the limit rather than build a
longer list first; every reader's document is then refused over it.

## Import into the builder

Importing replaces the deck in the builder with a new, unsaved deck (the
dialog says so first; a saved deck stays as it is). Each
row is held as written — printing, language, finish and note — and resolved
against the pool through the builder's one path (`hold_rows` →
`resolve_pending`), the same one a loaded deck takes, so the pool may arrive
after the import. A name matches a pool card's English name, any of its
`alt_names` (localised names and Scryfall's whole double-faced name), then its
English name ignoring case. **Unknown cards are listed and kept out of the
deck**, never dropped silently; the deck is then not saveable until they are
dealt with, as with any deck that names a card the pool lacks. The report
lists rows and copies read, lines skipped, cards unknown, rows that named no
printing (the default printing) or no language (the account's), a maybeboard
the builder does not keep, and a commander that is not a legal leader.

Which printing a game then shows is `baylee_cards::decks::DeckCard::chosen`:
the row's Scryfall id, language and finish, each falling back to the
registry's reference printing. A set and collector number without an id are
kept in the row and shown by the builder, but not yet resolved to an id when
the deck is stored — the chain below is still the design target for that
step.

### Resolution chain (design target)

`scryfall=` → set+number → exact name → fuzzy name → user's default print
preference → newest printing; language defaults to account setting.
Unknown/missing fields never fail the import; they resolve to defaults and
are listed in the import report.
