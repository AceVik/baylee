# Mana Notation

Canonical text notation (Scryfall-compatible). A `ManaCost` is a sequence of
braced symbols; `mana!("{2}{W/U}{W/P}")` parses at compile time (typos are
compile errors). Runtime parsing: `ManaCost::from_str`.

| Symbol | Meaning |
|---|---|
| `{W}{U}{B}{R}{G}` | colored mana |
| `{C}` | colorless mana |
| `{1}` … `{20}` | generic mana |
| `{X}{Y}{Z}` | variable mana (X = 0 for CMC on the stack) |
| `{W/U}` etc. | hybrid: one of two colors |
| `{2/W}` etc. | two generic OR one colored |
| `{W/P}` etc. | Phyrexian: one colored OR 2 life |
| `{G/U/P}` etc. | hybrid Phyrexian: one of two colors OR 2 life |
| `{S}` | snow mana (property of the producing source) |
| `{½}` | half generic mana (silver-bordered; supported, CMC 0) |
| `{∞}` | infinity (silver-bordered; supported, CMC 0) |

Rules:

- Symbols are stored in canonical order (variables/generic first, then
  WUBRG order, then colorless/snow/special); equality is order-insensitive.
- CMC: generic adds its value, `{2/W}` adds 2, every other symbol adds 1,
  variables and silver-bordered symbols add 0.
- `ManaPool`: six plain counters (W U B R G C) plus a side list of
  restricted mana (riders: spend-only restriction id, doesn't-empty, snow).
  Payment solving (hybrid chains, Phyrexian life-vs-mana) lives in the
  engine (`mana_pay`); only meaningfully distinct payment plans become
  `ChoiceRequest::PayMana` options, everything else auto-pays.
