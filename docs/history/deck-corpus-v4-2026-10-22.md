# Playernet v4 Deck Corpus & Split Report (2026-10-22)

- **Importer Version:** `v4.2.0-2026-10-25`
- **Working Card Pool:** `10058` cards (`hash = ba4e04e87e72044cfa28d967e072d103ca306ad68a983d8b8c1aa9b703726a05`), with `471` cards (`5%`) held out in `deckgen::held_out(&working, 20)`
- **Accepted Active Decks:** `1222` (`557` unmodified original `is_derived = false`, `665` rule-validated adapted precon derivations `is_derived = true`)
- **Near-Duplicates Dropped (`J >= 0.88`):** `24`
- **Max Observed Cross-Split Jaccard (`train` vs `val`/`test`):** `0.8182` (strictly `<= 0.85`)
- **Rejected Precon Decks Logged Outside Pool:** `1146` (`/home/ace/baylee-data/v4/decks/rejected.v4.json`)

## Counts by Split

| Split | Decks |
|---|---:|
| `test` | 154 |
| `train` | 989 |
| `val` | 79 |

## Counts by Holdout Tier

| Holdout Tier | Decks |
|---|---:|
| `eval_benchmark` | 15 |
| `train_active` | 989 |
| `unseen_cards` | 64 |
| `unseen_deck_known_cards` | 75 |
| `val_known_cards` | 79 |

## Counts by Format

| Format | Decks |
|---|---:|
| `commander` | 329 |
| `constructed` | 800 |
| `highlander` | 83 |
| `starter_limited` | 10 |

## Counts by Source

| Source | Decks |
|---|---:|
| `baylee_eval` | 15 |
| `baylee_house` | 5 |
| `curated_archetype` | 104 |
| `curated_coverage` | 339 |
| `curated_holdout_cards` | 40 |
| `mtgjson_precon` | 54 |
| `mtgjson_precon_adapted` | 665 |

## Counts by Primary Archetype

| Archetype | Decks |
|---|---:|
| `aggro` | 84 |
| `control` | 380 |
| `midrange` | 635 |
| `ramp` | 123 |

## Counts by Strategy Tag

| Strategy Tag | Decks |
|---|---:|
| `aggro` | 84 |
| `artifacts` | 228 |
| `blink_etb` | 304 |
| `burn` | 523 |
| `control` | 380 |
| `counters` | 227 |
| `enchantments` | 155 |
| `graveyard` | 372 |
| `lands` | 170 |
| `lifegain` | 335 |
| `midrange` | 761 |
| `ramp` | 308 |
| `reanimator` | 152 |
| `sacrifice` | 284 |
| `spellslinger` | 627 |
| `tempo` | 137 |
| `tokens` | 388 |
| `tribal` | 513 |
| `voltron` | 21 |
