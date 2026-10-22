# Playernet v4 Deck Corpus & Split Report (2026-10-22)

- **Importer Version:** `v4.1.0-2026-10-22`
- **Working Card Pool:** `9265` cards (`hash = 36024d9a426ee4b359fdfc15e0594ba81044da8faaeed89a5696b5489a9cb0d1`), with `438` cards (`5%`) held out in `deckgen::held_out(&working, 20)`
- **Accepted Active Decks:** `852` (`193` unmodified original `is_derived = false`, `659` rule-validated adapted precon derivations `is_derived = true`)
- **Near-Duplicates Dropped (`J >= 0.88`):** `23`
- **Max Observed Cross-Split Jaccard (`train` vs `val`/`test`):** `0.7910` (strictly `<= 0.85`)
- **Rejected Precon Decks Logged Outside Pool:** `1150` (`/home/ace/baylee-data/v4/decks/rejected.v4.json`)

## Counts by Split

| Split | Decks |
|---|---:|
| `test` | 133 |
| `train` | 639 |
| `val` | 80 |

## Counts by Holdout Tier

| Holdout Tier | Decks |
|---|---:|
| `eval_benchmark` | 15 |
| `train_active` | 639 |
| `unseen_cards` | 41 |
| `unseen_deck_known_cards` | 77 |
| `val_known_cards` | 80 |

## Counts by Format

| Format | Decks |
|---|---:|
| `commander` | 49 |
| `constructed` | 754 |
| `highlander` | 41 |
| `starter_limited` | 8 |

## Counts by Source

| Source | Decks |
|---|---:|
| `baylee_eval` | 15 |
| `baylee_house` | 5 |
| `curated_archetype` | 105 |
| `curated_holdout_cards` | 18 |
| `mtgjson_precon` | 50 |
| `mtgjson_precon_adapted` | 659 |

## Counts by Primary Archetype

| Archetype | Decks |
|---|---:|
| `aggro` | 41 |
| `control` | 407 |
| `midrange` | 357 |
| `ramp` | 47 |

## Counts by Strategy Tag

| Strategy Tag | Decks |
|---|---:|
| `aggro` | 42 |
| `artifacts` | 133 |
| `blink_etb` | 147 |
| `control` | 410 |
| `counters` | 99 |
| `enchantments` | 40 |
| `graveyard` | 274 |
| `lands` | 164 |
| `midrange` | 393 |
| `ramp` | 217 |
| `reanimator` | 50 |
| `sacrifice` | 143 |
| `spellslinger` | 629 |
| `tempo` | 87 |
| `tokens` | 312 |
| `tribal` | 210 |
| `voltron` | 11 |
