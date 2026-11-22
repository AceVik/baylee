# Ranked matchmaking and upset scoring

The selected configuration is `balanced / evidence`. It runs a single player rating across legal 1v1 ranked queues, publishes the existing 28 ranks daily, and maintains a separate rating for each deck version. The offline lab can replay the following decisions using only arrival times and match results.

## Choosing an opponent

Partition tickets by compatible rules and match format. On each five-second tick process older tickets first. Compare candidates by player rating. A new account starts at 1500. During its first 20 results an internal placement estimate updates faster (K=56). Its contribution to pairing tapers linearly to zero, with its difference from public Elo limited to 240 before tapering. Deck score and league badge are excluded from the queue key.

| Maximum rating difference | At most 6 arrivals in this format over the last 120 seconds | More than 6 arrivals |
| ---: | ---: | ---: |
| 100 | From admission | From admission |
| 200 | After 10 s | After 20 s |
| 350 | After 20 s | After 45 s |
| 600 | After 35 s | After 75 s |
| Open | After 60 s | After 120 s |

The older ticket determines the window. A gap above 350 additionally gives the younger ticket ten seconds to find another peer. When the older ticket reaches 120 seconds, its next compatible arrival can play immediately. A lone player stays in the queue until another player arrives or the search is cancelled. Timeouts in the simulation model when a person gives up.

Within the window, differences up to 80 receive the same distance cost. In busy queues a rotating subset, about a third of tickets, may search up to 300 from the start and treats these neighbouring ratings equally. The other tickets favour closer pairings. This connects nearby parts of the ladder and still gives evenly matched games priority in most searches. Recent repeat opponents carry a cost of 140 per prior encounter, up to three. Older candidates receive up to 45 points of waiting credit. Remaining ties follow admission order. An available repeat can play even when it is the only possible match.

## What a surprise result is worth

At equal Elo and neutral balances the winner gains 12 points and the loser gives up 12. Above 150 points of public rating difference the two individual stakes blend linearly toward the wide-match quotes. The full quotes apply from a gap of 350. The table assumes two established accounts, a fresh pairing and room in both accounts' adjustment balances.

| Confirmed previous surprises for this player's role | Underdog wins | Favourite loses |
| ---: | ---: | ---: |
| 0 | +20 | −14 |
| 1 | +24 | −18 |
| 2 | +28 | −22 |
| 3 or more | +32 | −26 |

Read each column using that player's own history. For instance, an underdog with three earlier surprises can gain 32 while a favourite losing for the first time loses 14. If the underdog has played fewer than 20 ranked matches the favourite's full-gap loss is capped at **10**, including when the favourite already has a losing streak.

A qualifying earlier match had a public gap of at least 250 and an opponent with at least 20 ranked games. For each role track the last three distinct opponents in its run of surprises. Evidence expires after 12 personal games or 14 days. The current opponent is excluded when quoting an escalation. An expected qualifying result against an established opponent clears that role's run. Matches against provisional opponents contribute no evidence about the other player. The third and fourth encounter with the same account within six hours pay half stakes, the fifth and later pay one quarter. This applies to both sides and outcomes. Repeated opponents still count toward completed-match requirements. Coordinated accounts deliberately trading wins are outside the simulation.

For a routine favourite win, let `p = 0.04 + 0.92 / (1 + 10^(gap/400))` be the modelled underdog probability. Each side's stake reverses direction and is multiplied by `p/(1−p)`. This makes both outcomes small for the favourite doing what was expected. The 4% floor is a modelling assumption about game variance. Updates carry fractional Elo internally and round for the standings.

### Keeping the rating pool stable

Asymmetric awards can create or remove points. In the reserved 160-account fields, omitting the accounting limit raised the average score by 44.24 points per account in one year. With the limit the change was +0.38.

Each match has a conserved transfer plus two personal adjustments. Split the difference between the quoted gain and loss across the two accounts, each with a signed allowance bounded by 64. Use the other account's remaining room when necessary. If both are full, reduce the larger absolute stake by the uncovered difference. During ordinary outcomes each account releases up to one point of its outstanding adjustment, limited to a quarter of the outcome's magnitude. This always preserves the direction of a win or loss. A qualifying upset has no amortization. The total change in raw population Elo equals the sum of the bounded personal balances for a closed population. A saturated balance can therefore make an upset pay less than its nominal quote. Points, streaks and repeated opponents are calculated from pre-result state.

## Experiments and measurements

All results are synthetic. The population study ran 40 independent 364-day fields, 25 development seeds and 15 reserved verification seeds. Each field compared eight policy combinations. Arrival times, patience, intrinsic player skill and seasonal deck effects are shared across alternatives. The same unordered pair in the same time slot receives the same random draw. A changed opponent produces a different match. Every account can enter at most once per slot. Slots are two hours apart, with arrivals spread over up to 30 minutes. Patience is uniformly 90–240 seconds. The two formats split the larger fields approximately 75/25. The two-player stress case puts both in the same format with 700 intrinsic Elo difference and tries ten matches daily. Other fields retain the previous 2/6/10 activity mix, late entries and retirements. The true outcome uses the skill difference plus the seasonal deck effect, with the 4% upset floor.

The selected policy completed **46,571,322 matches from 93,434,028 ticket entries** with 46,070 simulated accounts. The 371,540,009 matches across all eight alternatives include counterfactual replays and should not be reported as independent evidence. Verification seeds end in 50–52 for each field size.

| Accounts per field | Served, fixed ±200 | Served, selected | Matched wait P95, selected | P95 through any exit | Selected games with true underdog chance below 25% |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 0.36% | 100% | 65 s | 65 s | 100% |
| 24 | 51.61% | 77.05% | 128 s | 200 s | 42.7% |
| 160 | 81.95% | 96.30% | 83 s | 108 s | 37.9% |
| 1600 | 98.65% | 99.84% | 33 s | 35 s | 10.2% |
| 5000 | 99.66% | 99.95% | 20 s | 20 s | 6.6% |

The table holds the scoring rule fixed across queues. Served rates pool tickets. Time percentiles and lopsided shares are averages of the three verification runs per size. The 24-account first-available control served 77.18%. Much of the remaining loss occurred even when any compatible opponent could play immediately. A fixed window left 37.22% of the strongest decile's requests unserved at 160 accounts, versus 4.56% with the selected queue. In the 5000-account fields, always preferring the closest feasible peer reduced lopsided games to 2.6%, but the annual rating error against intrinsic skill rose from 162 to 255 Elo RMS. First-available reduced that error to 80 while producing 49.8% lopsided games. The chosen compromise favours match quality while keeping neighbouring skill groups connected.

### Points in isolation

Six controlled fixtures each run 12,000 independent 100-game histories. Three scoring rules replay the same 7.2 million outcomes. Opponents have established fixed ratings unless specified. An underrated newcomer's first ten queue ratings are compared with its true skill. Error at games 20, 50 and 100 uses its published rating.

| Fixture | Classic K=24 | Fixed caps | Selected caps and streaks |
| --- | ---: | ---: | ---: |
| Well-rated favourite, mean absolute rating error after 100 | 55.8 | 34.4 | 34.8 |
| Overrated veteran, median games to get within 100 of true skill | 50 | 57 | 54 |
| Underrated newcomer, same median | 50 | 60 | 55 |

Time medians use the episodes that reached the band by game 100. In the selected policy these were 11,908/12,000 overrated and 11,821/12,000 underrated episodes. Even a genuinely well-rated favourite sometimes drew a streak: 10.2% of its wide upset losses had prior independent evidence; the average loss across those upsets was 14.45. Repeatedly facing entirely new, stronger-than-advertised opponents is a harder case. Each wide loss stayed at 10, but the veteran's mean error still reached 238 after 100 such matches. The model also has a tradeoff under low outcome variance: with a true zero upset floor, classic Elo ended closer (32 versus 55 mean absolute error); at a true 10% floor the selected rule did better (70 versus 119).

### Queue, player leagues and decks together

Each completed match feeds both players' next queue rating, the next evidence check, and the existing daily league and division publication. Gold, Diamant and Master retain their established Elo, volume and age thresholds. In the three reserved 5000-account populations, **30.86%** finished at Gold or above, **5.10%** at Diamant or above, and **0.75%** at Master or above. At 1600 accounts these were 29.00%, 3.48% and 0.31%. The small populations are insufficient for a precise estimate of elite occupancy. Across the reserved runs there were 85,456 division changes in 349,849 eligible active account-days in the last 28 days. Global Champion allocation still goes through the shared top-100 standings code.

A separate paired run drove six 160-account fields through four seasons with real PoC deck registries. Each pilot starts with 60 main and 15 sideboard copies, replaces 15 in the second season and 45 relative to the original in the third, and returns to the original card pool in the fourth with a main/side swap. Deck ratings update on every result using both pilots' pre-match Elo. The fourth season also restores the original deck's simulated playing strength. A shadow world gets the very same physical decks and outcomes with deck scoring disabled. Across **941,857 matches**, every matched pair, start tick, winner and both resulting player Elo values agreed. All **917 returning versions** recovered their own last earned score exactly. A changed deck can still affect future player Elo by winning or losing.

## Reproducing the evidence

Run these from this directory. The release binary with no arguments also runs the ranked population suite.

```sh
cargo run --release --locked -- ranked > ranked.csv
cargo run --release --locked -- rating-trials > rating-trials.csv
cargo run --release --locked -- ranked-decks > ranked-decks.csv
cargo run --release --locked -- payouts
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

[Population metrics](ranked.csv) include both matched waits and all-ticket exits, points, repeat exposure, prediction error, rating drift and league totals. [Controlled trials](rating-trials.csv) include convergence denominators and caps. [Deck integration](ranked-decks.csv) records per-seed comparison counts. `payouts` quotes both outcomes from cloned snapshots for different personal streaks, placement status and prior encounters.

The PoC is single-writer and offline. A live ranked service needs one active ticket per account, an atomic claim of both tickets, and a single idempotent settlement per authoritative completed match covering player ratings, evidence, pair history, deck updates and league progress. A cancelled search or unplayed match produces no settlement. Production queue throughput, actual patience, win probability and enjoyment of wide matches require live measurement.
