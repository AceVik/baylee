# Deck Import/Export Format

**Status: [Spec], with one zone landed.** The gateway accepts simple
`"N Card Name"` lines (registry-validated, counts 1–4, basics unlimited, ≤250
cards and ≤250 lines per list) in *two* lists now: `cards` and `sideboard`,
which the deck builder fills and which reach a seat as its `SeatSpec`. The
commander field is stored and validated but no format assembles from it yet.
Everything else below — print metadata, languages, the fallback chain, the
text and JSON files themselves — is still the design target for the catalog
milestone.

Canonical JSON plus an MTGO/Arena-style text format. All per-card metadata
is optional on import; export always writes complete metadata.

## Text format

```
# baylee deck export v1
1 Lightning Bolt (M11) 149 [EN] *F* scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d
SB: 1 Karakas (EMA) 240 [DE]
CMD: 1 Aminatou, the Fateshifter (C18) 37
```

- Prefixes: none = main, `SB:` sideboard, `CMD:` commander, `MB:` maybeboard.
- `(SET) number` = set code + collector number; `[XX]` = language;
  `*F*` = foil, `*E*` = etched; `scryfall=` = printing UUID.

## Import resolution (fallback chain)

`scryfall=` → set+number → exact name → fuzzy name → user's default print
preference → newest printing; language defaults to account setting.
Unknown/missing fields never fail the import; they resolve to defaults and
are listed in the import report.

## JSON format

`{"version":1,"name":...,"format":...,"cards":[{"zone":"main|side|commander|maybe","count":1,"scryfall_id":null,"set":null,"collector_number":null,"name":...,"lang":null,"finish":null}]}`
