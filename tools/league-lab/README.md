# Baylee league and deck balance lab

Offline Rust prototype, 22 November 2026. The league experiment runs a global 1v1 ladder of accounts. The deck experiment measures the cost of edits to ranked lists. Both sample wins and losses. The two experiments use separate ratings.

## Proposed player leagues

The global 1v1 ladder publishes the highest earned rank once a day. Accounts start in Papier with 1500 player Elo. Matchmaking and Elo updates use the player's rating. Match count, rating and the date of the first ranked match carry across seasons. A win or loss transfers the rounded value of `24 × (score − expected)` points between the two accounts.

| League | Entry Elo | Lifetime ranked matches | Divisions in ascending order |
| --- | ---: | ---: | --- |
| Papier | Entry | 0 | Single placement rank |
| Holz | 1200 | 20 | III, II, I |
| Eisen | 1320 | 60 | III, II, I |
| Bronze | 1440 | 120 | III, II, I |
| Silber | 1560 | 240 | III, II, I |
| Gold | 1680 | 440 | IV, III, II, I |
| Platin | 1840 | 800 | IV, III, II, I |
| Diamant | 2000 | 1240 | IV, III, II, I |
| Master | 2240 | 2000 | Single title |
| Grandmaster | 2350 | 2500 | Single title |
| Champion | 2490 | 3000 | Up to 100 eligible accounts worldwide |

Gold opens the middle ranks. Platin is the climb from there to high Elo, which begins at Diamant. Each division splits the gap to the next named league into equal steps in Elo and games, rounded up. A player must meet both thresholds. For example, Gold IV starts at 1680/440, Gold III at 1720/530, Gold II at 1760/620 and Gold I at 1800/710. Platin IV then opens at 1840/800. `ladder` prints all 28 ranks.

A demotion buffer of 15 Elo applies only within the same named league. Someone who has earned Gold III can hold it at 1705 Elo. At 1704 they fall to Gold IV. Promotion to Gold III still requires the full 1720 Elo. At 1679 they fall below Gold. Division changes have no effect on queue pairing. Each daily publication can move through multiple divisions if the conditions warrant it.

Master and above additionally require 184 full days since the first ranked match and a ranked match in the last 14 elapsed days. At ten matches per day, even a perfect season-spanning streak reaches 2000 matches on playing day 200. Champions are the first 100 eligible accounts across queues, ordered by descending integer Elo and then by stable account ID. One authoritative snapshot is required for a global production allocation.

## League simulation

We replayed the previous ladder and the revised one on identical results in 33 independent populations, including eight new seeds (610–617) of 12,000 accounts each. The combined suite covers **474,498,228 distinct simulated matches** with 465,200 accounts over four 91-day seasons. Alternative scoring or division policies reuse those results. Each account draws persistent normally distributed skill around 1500 and an independent deck effect each season (SD 60 Elo). Wins follow the 400-point Elo logistic curve on actual strength. Published Elo updates use ratings before the match. About 35% play two, 35% six and 30% ten matches per available day. An eighth join 14–59 days late, a tenth retire between days 240 and 329. The randomly offset 200-point queue bands widen to 600 for one fifth of rounds. Up to one player has a bye per round.

### How hard is the climb?

The eight new reference populations (skill SD 210) generated **97,597,962 matches**. Shares below count all 96,000 accounts at day 364, including retirees and late arrivals. Each row includes the leagues above it.

| Milestone | Previous ladder | Revised ladder | Revised count |
| --- | ---: | ---: | ---: |
| Gold or higher | 35.01% | 29.09% | 27,928 |
| Platin or higher | — | 9.75% | 9357 |
| Diamant or higher | 19.79% | 4.21% | 4038 |
| Master or higher | 1.04% | 0.73% | 702 |
| Grandmaster or higher | 0.14% | 0.14% | 135 |

Among the 28,590 ten-a-day accounts, 14,999 reached Gold at some point, 4432 reached Diamant and 962 (3.36%) reached Master. For those who attained the respective milestone, the eight within-run medians ranged from 46–49 elapsed days for Gold, 146–161 for Diamant and 218–235 for Master. The previous ranges on the same matches were 26–28, 72–78 and 203–222 days. The earliest Master promotion in each new field was after 199 elapsed days.

The separate 150,000-player stress field filled all 100 Champion seats from 198 candidates. The four 800-player fields each finished with an empty Champion list. Varying the population's skill SD from 165 to 255 put 2.30% to 6.07% of accounts in Diamant or higher and 0.21% to 1.48% in Master or higher at day 364. Shares are simulation outcomes. Both skill spread and participation would need measurement after launch.

### Three or four divisions?

Five configurations replayed the same matches in six calibration and eight new reference fields. We measured daily publications over the last 28 days. An eligible observation is a day on which the account played and finished in the same named league as the previous day. A change counts one promotion or demotion independent of the number of divisions crossed. A reversal returns to the exact division held two publications earlier after one day away. League-boundary crossings are outside this measurement.

| Divisions and buffer | Holz–Silber | Gold–Platin | Diamant |
| --- | ---: | ---: | ---: |
| Three, no buffer | 36.43 | 23.31 | 22.46 |
| Four, no buffer | 48.49 | 31.01 | 32.74 |
| Three, 15 Elo | 25.49 | 17.40 | 17.20 |
| Four, 15 Elo | 33.55 | 22.97 | 25.03 |
| **Three below Gold, four from Gold, 15 Elo** | **25.49** | **22.97** | **25.03** |

Values are division changes per 100 eligible account-days in the eight new fields. In Gold and Platin, four divisions with the buffer still give more visible steps than three (22.97 versus 17.40 changes), while next-day reversals drop from 8.27 to 3.78 compared with four unbuffered divisions. In Holz through Silber, three buffered divisions avoid about eight extra changes per 100 days. The selected ladder has 28 ranks including Papier and the three elite titles. The experiment measures pacing and volatility. Whether players enjoy that cadence needs playtesting.

## Deck changes

Count copies by rules identity in the combined registered main deck and sideboard. The overlap is the sum, over card identities, of the smaller copy count in the two lists. Let `q = 1 − overlap / max(old_size, new_size)`. This also handles additions, cuts, reprints and exchanges between zones. Each format and match mode has its own registry. At first ranked use, apply the selected quadratic curve to the confirmed reference score:

```text
loss       = round((reference_elo − 150) × (0.2q + 0.8q²))
starting   = reference_elo − loss
```

A new unrelated pool starts at 150. Starting from 1680 Elo, seven changed copies out of sixty cost 52 points. At 25%, 50%, 75% and 100%, the new scores are **1527, 1221, 762 and 150**. The slope increases throughout. At full replacement the visible score has the same floor as a fresh list. Player Elo handles matchmaking independently.

1. Look up the canonical card pool first. A known pool immediately resumes its stored score, including all later wins and losses. Switching cards between main and sideboard resolves to that same pool.
2. For a new pool, choose the closest confirmed reference in the format. An exact distance tie takes the lower inherited score. All intermediate versions are measured directly against the confirmed lists. Each version keeps the results it earns. A reference can pass on at most the smaller of its season snapshot and current rating.
3. At a season boundary, a pool with at least 40 ranked matches **on that exact pool in that season** can become a new reference. Other variants retain their own scores without becoming references. This allows an established rebuild to support future edits. Renames, copies, one intervening win or a season change alone cannot create a new reference. In a cold format, pools without a confirmed reference start at the floor until the first references qualify.

### Deck simulation

We replayed the **same 12,000 independent edit histories** under powers 1.5, 2 and 3. Each policy saw 1,517,497 synthetic match results. Each history starts with 60, 75 or 100 copies, including repeated identities, and an established score between 1300 and 2100. One route submits the final list immediately. The other replaces one to six copies per step, plays a rated match after each intermediate edit and crosses a season boundary on longer routes. It also revisits earlier pools and tests main/side exchanges. The naive control makes every intermediate list the next reference. All three policies share draws, changes and opponents.

| Changed share | Mean loss, power 1.5 | Mean loss, power 2 | Mean loss, power 3 |
| --- | ---: | ---: | ---: |
| Up to 10% | 35 | 22 | 18 |
| Over 10% through 30% | 182 | 120 | 77 |
| Over 30% through 60% | 530 | 410 | 273 |
| Over 60% but below 100% | 1137 | 1052 | 914 |
| 100% | 1550 | 1550 | 1550 |

The quadratic choice keeps a small tuning session affordable and cuts roughly four hundred points for typical mid-sized rebuilds in this sample. At every size the largest observed difference between the direct and staged final starting score was **zero**, including all restoration checks. For a complete rebuild, the naive quadratic control gave staged editors a median **1287 extra Elo** over direct editors.

After the rebuild, each final pool plays 120 more matches against rated opponents in the 1250–1900 range. Its simulated playing strength regresses toward the field in proportion to the share changed, with a random upgrade or downgrade. Among the 1175 complete rebuilds, 941 (80.1%) regained half the lost points within 120 matches. For those that did, the median was 71 matches. This recovery experiment assumes an established opponent pool. A fresh server's deck economy, competitive demand for the ladder, Magic game balance and player retention require different evidence.

## Reproduction

From this directory, using the pinned lockfile:

```sh
cargo run --release --locked -- all > leagues.csv
cargo run --release --locked -- divisions > divisions.csv
cargo run --release --locked -- ladder
cargo run --release --locked -- decks > decks.csv
cargo run --release --locked -- edit 1680 60 7
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

`all` runs the player suite. Each row of [leagues.csv](leagues.csv) covers one policy, seed and quarterly snapshot. Match counts are cumulative. The two rows per seed and day share a match history. The last-week retention columns describe day 364. [divisions.csv](divisions.csv) gives the account-day denominators, promotions, demotions and reversals for each configuration and league band. [decks.csv](decks.csv) contains counts, mean losses, control advantages and recovery times per curve and edit band. The PoC uses fixture account indices as stable tie breakers. Full-scale indexing and persistent, concurrent standings publication belong to a production implementation.
