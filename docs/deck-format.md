# Deck Import/Export Format

**Status: [Implemented] for the text row; [Spec] for the files around it.**
The row grammar below is `crates/baylee-core/src/deckrow.rs`, and it is what
the gateway stores, what the deck builder writes and what an import reads —
one parser, so a deck that round-trips through a text file is the deck that
never left. Two zones are live (`cards` and `sideboard`); the `CMD:`/`MB:`
prefixes, the file headers, and the JSON document are still the design target.

Canonical JSON plus an MTGO/Arena-style text format. All per-card metadata
is optional on import; export always writes complete metadata.

## Text format

```
# baylee deck export v1
1 Lightning Bolt
1 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d
SB: 1 Karakas (EMA) 240 [de]
CMD: 1 Aminatou, the Fateshifter (C18) 37
```

- Prefixes: none = main, `SB:` sideboard, `CMD:` commander, `MB:` maybeboard.
- `(SET) number` = set code + collector number; `[xx]` = language;
  `*F*` foil, `*E*` etched, `*N*` non-foil; `scryfall=` = printing UUID.

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

## Import resolution (fallback chain)

`scryfall=` → set+number → exact name → fuzzy name → user's default print
preference → newest printing; language defaults to account setting.
Unknown/missing fields never fail the import; they resolve to defaults and
are listed in the import report.

## JSON format

`{"version":1,"name":...,"format":...,"cards":[{"zone":"main|side|commander|maybe","count":1,"scryfall_id":null,"set":null,"collector_number":null,"name":...,"lang":null,"finish":null}]}`
