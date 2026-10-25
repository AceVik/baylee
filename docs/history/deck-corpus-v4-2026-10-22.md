# Playernet v4 Deck Corpus & Split Report (2026-10-22)

- **Importer Version:** `v4.2.0-2026-10-25`
- **Working Card Pool:** `10058` cards (`hash = ba4e04e87e72044cfa28d967e072d103ca306ad68a983d8b8c1aa9b703726a05`), with `471` cards (`5%`) held out in `deckgen::held_out(&working, 20)`
- **Accepted Active Decks:** `1201` (`536` unmodified original `is_derived = false`, `665` rule-validated adapted precon derivations `is_derived = true`)
- **Near-Duplicates Dropped (`J >= 0.88`):** `23`
- **Max Observed Cross-Split Jaccard (`train` vs `val`/`test`):** `0.7910` (strictly `<= 0.85`)
- **Rejected Precon Decks Logged Outside Pool:** `1146` (`/home/ace/baylee-data/v4/decks/rejected.v4.json`)

## Counts by Split

| Split | Decks |
|---|---:|
| `test` | 160 |
| `train` | 962 |
| `val` | 79 |

## Counts by Holdout Tier

| Holdout Tier | Decks |
|---|---:|
| `eval_benchmark` | 15 |
| `train_active` | 962 |
| `unseen_cards` | 68 |
| `unseen_deck_known_cards` | 77 |
| `val_known_cards` | 79 |

## Counts by Format

| Format | Decks |
|---|---:|
| `commander` | 319 |
| `constructed` | 796 |
| `highlander` | 76 |
| `starter_limited` | 10 |

## Counts by Source

| Source | Decks |
|---|---:|
| `baylee_eval` | 15 |
| `baylee_house` | 5 |
| `curated_archetype` | 105 |
| `curated_coverage` | 313 |
| `curated_holdout_cards` | 44 |
| `mtgjson_precon` | 54 |
| `mtgjson_precon_adapted` | 665 |

## Counts by Primary Archetype

| Archetype | Decks |
|---|---:|
| `aggro` | 129 |
| `control` | 427 |
| `midrange` | 496 |
| `ramp` | 149 |

## Counts by Strategy Tag

| Strategy Tag | Decks |
|---|---:|
| `aggro` | 129 |
| `artifacts` | 130 |
| `blink_etb` | 288 |
| `burn` | 502 |
| `control` | 427 |
| `counters` | 191 |
| `enchantments` | 171 |
| `graveyard` | 335 |
| `lands` | 211 |
| `lifegain` | 285 |
| `midrange` | 671 |
| `ramp` | 326 |
| `reanimator` | 130 |
| `sacrifice` | 256 |
| `spellslinger` | 650 |
| `tempo` | 148 |
| `tokens` | 389 |
| `tribal` | 513 |
| `voltron` | 20 |
