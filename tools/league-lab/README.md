# Baylee league and deck balance lab

Offline Rust prototype, 22 November 2026. The league experiment runs a global 1v1 ladder of accounts. The deck experiment measures the cost of edits to ranked lists. Both sample wins and losses. The two experiments use separate ratings.

## Proposed player leagues

Every account opens at 1500 player Elo in Papier. Queue pairing uses player Elo. Each match transfers the rounded value of `24 × (score − expected)` points between the two players. The expected score uses the usual 400-point Elo scale. The highest tier for which the account meets all conditions becomes its daily title. Rating, match count and date of first ranked match carry across seasons.

| League | Minimum player Elo | Lifetime ranked matches | Additional requirement |
| --- | ---: | ---: | --- |
| Papier | Entry | 0 | |
| Holz | 1150 | 10 | |
| Eisen | 1275 | 30 | |
| Bronze | 1400 | 60 | |
| Silber | 1500 | 120 | |
| Gold | 1625 | 260 | |
| Diamant | 1775 | 600 | |
| Master | 2200 | 1840 | At least 184 full days since first ranked match |
| Grandmaster | 2350 | 2500 | Master age and activity requirements |
| Champion | 2490 | 3000 | Same age and activity requirements, globally in the first 100 eligible accounts |

Master and above require a ranked match in the last 14 elapsed days. Age is measured in elapsed days. Season, region and format share one Champion allocation. The prototype publishes one complete standings snapshot per day. Champions are ordered by descending integer Elo with stable account ID breaking an exact tie. Replacing the snapshot replaces the titles, so the outgoing and incoming holders never occupy the same seat at once. Production would need one authoritative atomic publication for all queues. A historical season badge could display its season separately from a current title.

## League simulation

Each of the 25 independent populations plays 364 days in four 91-day seasons. Pilots draw persistent normally distributed skill centred at 1500 Elo. Each gets an independent deck-strength offset every season (standard deviation 60 Elo). A win is drawn with probability `1 / (1 + 10^((strength_B − strength_A)/400))`. Elo updates use the public ratings before the result. Skill and activity are sampled independently. About 35% play two, 35% six and 30% ten matches per available day. One eighth join 14–59 days late and one tenth retire between days 240 and 329. Randomly shifted 200-point queue bands widen to 600 in one fifth of rounds. An unmatched player carries into the next occupied band. At most one player receives a bye per round.

Six 8000-account calibration seeds compared the three policies on identical matches. The higher thresholds were selected to put roughly 1% of accounts in Master or above and around 0.1–0.2% in Grandmaster or above after a year. These are design targets with fixed Elo thresholds. Population size and the spread of skill can change the percentages.

| Master / GM / Champion Elo | Master or higher at day 364 | GM or higher at day 364 |
| --- | ---: | ---: |
| 2060 / 2190 / 2340 | 2.70% | 0.55% |
| 2140 / 2290 / 2420 | 1.60% | 0.28% |
| **2200 / 2350 / 2490** | **1.06%** | **0.17%** |

### Independent verification and stress tests

| Population experiment | Accounts per run × seeds | Skill SD | Master or higher | GM or higher | Champions held at final snapshot |
| --- | ---: | ---: | ---: | ---: | ---: |
| Verification | 12,000 × 8 | 210 | 1.04% | 0.136% | 2–6 per run |
| Small population | 800 × 4 | 210 | 0.84% | 0.22% | 0 per run |
| Narrow skill distribution | 12,000 × 3 | 165 | 0.34% | 0.031% | 2 across 3 runs |
| Broad skill distribution | 12,000 × 3 | 255 | 1.98% | 0.342% | 45 across 3 runs |
| Global cap stress | 150,000 × 1 | 255 | 1.98% | 0.356% | 100 of 198 eligible |

Together the calibration, verification and stress runs contain **376,900,266 matches and 369,200 accounts**. Alternative thresholds and quarterly snapshots reuse the matches and count once in this total. Percentages use all accounts in each population, including late arrivals and retirees.

In verification, **998 of 96,000** held Master or higher at day 364, **131** held GM or higher and **34** held Champion across the eight independent populations. The seed ranges for Master and GM or higher were 0.98–1.13% and 0.092–0.183%. At some point during the year 2394 accounts reached Master and 398 reached GM. Of the 28,921 accounts assigned ten daily matches, 1359 reached Master within the observation period (4.70%). The first promotion happened after 184 elapsed days in every verification run. Among successful ten-a-day players the eight within-run medians were **205–227 days** and the corresponding 90th percentiles were 287–315 days. An inactive or late entrant can have fewer opportunities. These timing summaries exclude players who never qualified.

Of the verification holders seven days before the end, 790/990 (79.8%) still had Master or higher, 92/124 (74.2%) still had GM or higher and 22/31 (71.0%) still had Champion. The six-month climb leads to a title that can be lost at the next daily publication. In the large stress test 76 of 100 Champions held their title a week later. Only 449 of the 998 final Master-or-higher holders were in the top 1% of underlying player skill in verification. Deck strength, attendance and recent results also affect who qualifies. The model assumes stable pilot skill and synthetic activity. Actual Baylee participation and skill dispersion would be needed to forecast live occupancy.

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
cargo run --release --locked -- decks > decks.csv
cargo run --release --locked -- edit 1680 60 7
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

`all` runs the player suite. Each row of [leagues.csv](leagues.csv) covers one policy, seed and quarterly snapshot. Match counts are cumulative. The three rows per seed and day share a match history. The last-week retention columns describe day 364. [decks.csv](decks.csv) contains counts, mean losses, control advantages and recovery times per curve and edit band. The PoC uses fixture account indices as stable tie breakers. Full-scale indexing and persistent, concurrent standings publication belong to a production implementation.
