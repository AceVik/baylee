# Playernet v4 Deck Corpus & Split Report (2026-10-22)

- **Importer Version:** `v4.2.0-2026-10-25`
- **Working Card Pool:** `10058` cards (`hash = ba4e04e87e72044cfa28d967e072d103ca306ad68a983d8b8c1aa9b703726a05`), with `471` cards (`5%`) held out in `deckgen::held_out(&working, 20)`
- **Accepted Active Decks:** `1200` (`535` unmodified original `is_derived = false`, `665` rule-validated adapted precon derivations `is_derived = true`)
- **Near-Duplicates Dropped (`J >= 0.88`):** `23`
- **Max Observed Cross-Split Jaccard (`train` vs `val`/`test`):** `0.7910` (strictly `<= 0.85`)
- **Rejected Precon Decks Logged Outside Pool:** `1146` (`/home/ace/baylee-data/v4/decks/rejected.v4.json`)

## Counts by Split

| Split | Decks |
|---|---:|
| `test` | 158 |
| `train` | 963 |
| `val` | 79 |

## Counts by Holdout Tier

| Holdout Tier | Decks |
|---|---:|
| `eval_benchmark` | 15 |
| `train_active` | 963 |
| `unseen_cards` | 66 |
| `unseen_deck_known_cards` | 77 |
| `val_known_cards` | 79 |

## Counts by Format

| Format | Decks |
|---|---:|
| `commander` | 319 |
| `constructed` | 795 |
| `highlander` | 76 |
| `starter_limited` | 10 |

## Counts by Source

| Source | Decks |
|---|---:|
| `baylee_eval` | 15 |
| `baylee_house` | 5 |
| `curated_archetype` | 105 |
| `curated_coverage` | 314 |
| `curated_holdout_cards` | 42 |
| `mtgjson_precon` | 54 |
| `mtgjson_precon_adapted` | 665 |

## Counts by Primary Archetype

| Archetype | Decks |
|---|---:|
| `aggro` | 126 |
| `control` | 423 |
| `midrange` | 498 |
| `ramp` | 153 |

## Counts by Strategy Tag

| Strategy Tag | Decks |
|---|---:|
| `aggro` | 126 |
| `artifacts` | 130 |
| `blink_etb` | 290 |
| `burn` | 505 |
| `control` | 423 |
| `counters` | 187 |
| `enchantments` | 172 |
| `graveyard` | 337 |
| `lands` | 206 |
| `lifegain` | 294 |
| `midrange` | 683 |
| `ramp` | 334 |
| `reanimator` | 134 |
| `sacrifice` | 249 |
| `spellslinger` | 647 |
| `tempo` | 147 |
| `tokens` | 387 |
| `tribal` | 511 |
| `voltron` | 22 |
