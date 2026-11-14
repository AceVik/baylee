# Tower echoes: evolution beyond the certified expedition

Design pass, 14 November 2026. The owner clarified that the Tower must use
other players' historical builds, played by the Expert house AI, with shops,
gold, equipment and consumables between manually played duels. This document
describes the new work; it is **not** evidence that these features already run.
The current five-floor certified expedition is documented in `../tower.md`.

The [official Oaken Tower store description](https://store.steampowered.com/app/3400960/Oaken_Tower/?l=english),
checked on 14 November 2026, describes PvP builds and buying/combining items;
its store page also identifies asynchronous multiplayer. Baylee borrows that
high-level progression idea only. Manual card play, exact same-stage cohorts,
the item catalog and the service trust model below are Baylee design decisions,
not claims about Oaken Tower's unpublished backend or copied content.

## Process boundary

The owner explicitly rejected hosting Tower gameplay in Central and then selected
a global Tower played on individual gateways. Runs, gold, shops, inventory,
snapshots, matching and Tower rankings belong to a dedicated global Tower service
in the **Baylee repository**. Existing gateway game processes execute duels.
Central keeps identity and confirmed permanent accomplishments, alongside its
already authorized global Daily/Weekly responsibilities. Switching gateways
between duels must preserve the same run and economy.

Client operations use bounded, versioned commands and public projections, never
table schemas. Only the Tower authority writes spendable currency and inventory;
the game process supplies the actual outcome. An unavailable Tower service leaves
the run waiting without consuming a defeat allowance or rerolling shops. Durable
result and accomplishment outboxes make transport retries idempotent.

| Component | Owns | Does not own |
| --- | --- | --- |
| Central identity | Accounts, sessions, visibility, deletion | Duel rules |
| Global Baylee Tower service | Runs, economy, inventory, snapshots, matching, reservations, Tower rankings | Accounts or private vault data |
| Tower package / balance worker | Validated package assembly, offline balance measurements | Player gold or claimed wins |
| Game process | Rules, legal actions, Expert decisions, durable outcome | Purchases or matchmaking |
| Central projections | Confirmed permanent cosmetic accomplishments; global calendar rankings | Tower economy or independent copies of spendable currency |
| Native client / website | Commands and authorized projections | Results, prices or opponent selection |

The global service reserves one active duel for a run and binds it to the selected
gateway/executor. Reconnecting resumes that reservation; changing gateways cannot
create a second opponent or spend the same loadout twice. Gateway switching is
supported at a completed-duel boundary, not as an implicit mid-game restart.

A gateway's claimed result alone is not trusted for global ranking. It returns
the bounded deterministic action record for the exact reserved package. A Tower
verification worker reconstructs the game with the frozen engine and Expert
policy, rejects illegal/missing/reordered actions and recomputes outcome and
consumed items. Only a verified terminal result commits progression and income.
Duplicates acknowledge the committed result; conflicting records are refused.
If verification is pending or unavailable, show that state without issuing a
second duel. Bound record size, action count, time, concurrency and retry work.
This verifies rule-consistent play, not that a human rather than a bot chose it.
Replay validation also cannot prove that an operator did not reveal hidden cards
or search alternate human actions offline. Ranked execution therefore requires
an explicitly trusted execution operator/instance; registration alone is not that
trust. Gateways may provide access through approved execution capacity. Untrusted
local execution must be clearly distinguished and cannot mint global ranked awards.

Tower privileges are distinct from the existing all-purpose publisher bridge.
Do not silently reuse a capability that also grants account or unrelated gateway
operations. Central must not trust arbitrary gateway achievement claims: define
which operators can award which achievements, distinguish local/unverified awards
where necessary, and bind deduplication to issuer, account, run and milestone.
Retain the accepted pinned dependency graph and first-party safe Rust constraints
in Central. Two services must never write the same run.

## Echo identity and fair matching

“Day” means the ordinal of a duel within a run, not the real calendar date.
A defeat advances that ordinal too, so both builds have had the same number of
shop/income opportunities. Calendar seasons and run stages are separate values.

Before a duel, atomically freeze the submitted legal deck, equipped items,
consumables, starting attributes, progression ordinal and rules version. The
eligible pool includes valid builds independent of whether they later win:
winner-only publication would bias the pool towards unusually strong builds.
The snapshot contains no password, token, private collection or replay of the
player's decisions. The Expert AI pilots the build afresh from its authorized
view and cannot see the human's hand or library.

Match the same progression ordinal and rules/item/card-pool version. Exclude
the same account and, where supply permits, previously encountered opponents.
Do not match a newer, richer stage just because the pool is small. Select and
persist the opponent, shuffle and policy seeds once per reservation. Reconnects,
duplicate requests and worker retries preserve that selection. Replacing a
technically unplayable package requires an explicit void and bounded recovery.

The opponent panel says “<display name> · Echo · Expert house AI” and names the
build's stage. It never claims that the owner is online or controlling the
opponent. Participation explains the use of the player's display name and build
before enrolling a run; ranking publication remains independently reversible.
Account deletion removes future match eligibility and public attribution.
Frozen in-flight packages need an explicit deletion/retention policy before
release. The opposing deck's hidden cards never become a public roster field.

When there is no eligible player build, use a versioned house build labelled
as a house opponent, never a fabricated player. Record fallback frequency so
the operator can see whether asynchronous multiplayer has a usable population.

## Economy and play loop

The player receives a starter deck and initial gold, visits the shop, chooses a
legal deck/equipment loadout and starts a duel. Every completed duel grants a
bounded income and leads to a fresh shop unless the run has ended. Losing must
still allow meaningful rebuilding; begin calibration with equal base income
for wins and losses, without compound-interest or victory-income snowballing.
Draws, deliberate concessions, technical voids and abandonment need explicit
accounting rules and tests. There is no currency purchase or account-level power.

Shop offers and prices are authoritative, versioned and stable for that visit.
Each purchase, reroll, sale and equip command has an idempotency identity and an
expected run revision. Concurrent requests cannot overspend, duplicate an item,
reroll after seeing a new opponent or equip an item during a duel. Prices and
effect magnitudes are tuning hypotheses until measured, not a claim of balance.

The initial catalog should have a small set of distinct choices:

* Card upgrades from a reviewed, fully implemented pool; owned cards can be
  moved between the run's collection and legal deck using the existing builder.
* Bounded starting-life upgrades, with life at the table visibly distinct from
  the expedition's remaining defeat allowance.
* A costly extra free mulligan, with a cap and per-seat rules.
* A costly opening-hand upgrade, with a separate stated maximum hand size.
* A bounded starting token, expressed through normal engine objects/rules.
* A healing potion and a one-mana potion with an ordinary legal color choice.

Permanent means “for the remainder of this run”. Across runs, rewards are
cosmetic badges, histories and appearances. Limit equipment slots and potion
slots to make substitutions meaningful and bound engine/search work. A sale
cannot return more than the paid price; free starter/basic cards have no farming
value. Include rules for surplus copies, full inventory and incompatible colors.

Potions are not direct client edits to life or mana. The engine offers their
legal use, consumes a charge exactly once, journals the effect and projects its
result. Resume/replay restores charge consumption. All seat implementations,
including Expert, trained and language-model seats, must understand and answer
the new choices before they can enter this mode. Expert evaluates healing when
useful and mana against a legal affordable plan; it does not blindly drink every
potion at the start of a game.

## Table and shop interface

The lobby entrance, art direction, progressive medals and accessible controls
from the current expedition are reusable. The new run overview shows stage,
victories, remaining defeats, gold and one clear next action. The shop has a
stable item grid with price, concise effect, owned count and affordability.
Purchases have a preview and confirmation; replacing an equipped item shows the
precise attribute change. The deck editor operates on the run's owned pool.

At the table, show the echo owner and explicit AI label on the opponent's plate.
An equipment strip summarizes permanent effects; a separate potion belt shows
remaining charges and actual legal availability. Tooltips explain timing and
the source of modified values. Potion activation uses the existing guarded
decision flow, including keyboard control and the mana-color chooser. Text and
icons accompany color; reduced motion keeps every status readable.

## Validation milestones

1. Pure rules and economy: versioned limits, purchases, loadouts and adversarial
   arithmetic/concurrency cases. Preserve the existing expedition's records.
2. Durable snapshots and matchmaking: same-stage/version selection, self-match
   exclusion, explicit fallback, restart/idempotency and deletion behavior.
3. Real execution: ordinary engine legality, Expert support, potion choices,
   replay/resume, disconnects and trustworthy outcome/charge accounting.
4. Native/web presentation: shop, editor, inventory, opponent plate, localized
   timestamps and state changes at supported sizes and with reduced motion.
5. Balance and independent review: bounded deterministic, side-swapped Expert
   comparisons across stages and item strategies, followed by manual play of
   representative builds. Measure first-player advantage, duel length, potion
   use, dominant purchases and failure rates. Re-run only changed cases.

Daily/Weekly keep their multiple-solution certification. A match against a
player's changing build is competitive, and cannot honestly promise a
certified human victory. Do not reuse the old 35-node proof as evidence for
this new mode. Record actual measurements and remaining limitations separately.

## Implementation evidence — domain foundation

`baylee-tower` now supplies the renderer-, network- and Central-independent rule
kernel: bounded catalog/owned-deck validation, run-only relics and consumables,
authoritative prices, deterministic shop offers, stock and reroll accounting,
optimistic revisions, frozen shopping during a duel, technical voids without
income, bounded terminal progression and same-stage/cohort echo selection.
The starting rules are five victories or three unsuccessful duels, at most
seven stages. A draw spends one defeat allowance. A win and an unsuccessful
completed duel both grant ten gold; a new run starts with twelve. These values
are initial calibration inputs, not measured balance conclusions.

Eighteen focused tests pass, covering unknown/unowned/duplicate cards, deck size,
copy and equipment limits, charge underflow, invalid saved counters, stale and
sold offers, income bounds, duplicate terminal transitions, shop locking, voids,
same-stage/build/catalog matching, self-match exclusion, repeated-contributor
weight and deterministic selection independent of input order. Strict all-target
Clippy passes. Tests execute in under 0.01 seconds after compilation.

This is a pure-domain milestone only. Database transactions and request
deduplication, authenticated service transport, engine items, verification
workers, actual opponent records, client shop and Central accomplishment delivery
remain to be connected and tested. A pure transition named `verified` is not a
network authorization boundary or evidence of a real verified duel.

The next engine milestone adds bounded per-seat opening rules, including redraw
size, free London mulligans and base maximum hand size. Thirty-seven targeted
core/engine tests pass (0.219 seconds of execution after compilation), covering
asymmetric bonuses, refusal at the zero-card limit without state mutation,
cleanup, deterministic continuations and existing house/replay behavior. Strict
all-target Clippy for core, engine and gamehost passes. Independent static review
reported no actionable findings. Ordinary public table setup leaves these
overrides empty. These are shared engine capabilities; the Tower shop and its
authoritative loadout assembly still need to apply them.

## Implementation evidence — supplies and private duel execution

The original healing/mana emblems and starting Plant now use ordinary engine
abilities and the append-only object registry. Five targeted engine tests cover
ownership, timing, charges, color legality, stack behavior, deterministic replay
and a registry that cannot provide the supply. House-policy tests cover the
limited healing resource and reachable mana payments; two real Session tests
exercise the projected abilities and Expert's color choice. Human card faces,
the language-model narrator and trained features each have a focused passing
test. Seven existing token-ledger/art/identity tests pass with the new entries.
The independent review's copied-token narration finding was corrected and the
follow-up static review found no remaining issue; its regression uses a Food
token with Prodigal Sorcerer's current rules, whose activated text stays current.

`baylee-tower::package` freezes owned shopping builds, bounds decoded material,
binds the exact executor/rules/catalog and derives the complete preset from
purchased resources. It grants no hidden-information or development capability.
Canonical artwork comes from that executor's registry. The opponent uses the
mode-specific `TOWER_EXPERT` policy documented in `../house-ai.md`, with no
host-load-adaptive foresight. These packages are private execution material,
not client-facing DTOs and not authorization credentials.

`baylee-gamehost::session::tower::verify` requires the stored preset, this exact
build, sequential input identities, matching state hashes and one final engine
result. Every house decision is recomputed through the frozen-policy replay
path. The verifier counts actual charge-removal events against the original
source identity and incarnation; elimination leaves unused inventory intact.
Bounds are 16 MiB, 50,000 lines, 128 KiB per line, 500 turns and 500,000 journal
events, plus a cooperative worker deadline between decisions. The service must
also supervise the worker's process memory and wall time: a cooperative check
cannot interrupt one running engine decision.

Nine focused package/narration/verifier tests pass in 0.071 seconds after
compilation, including adversarial package/sequence/hash/policy/outcome changes,
truncated/duplicated terminal records, work bounds and paid versus unused
charges. Existing record/resumption checks also pass after boxing the header's
preset to keep the enlarged enum compact; serialized JSON remains unchanged.
Independent verifier review found a live/replay mismatch after a rejected house
proposal. Both paths now apply the same engine-checked proposal and deterministic
recovery, then compare the accepted action; a mismatch discards the replay
session. Fourteen focused Tower/challenge/recovery tests pass (0.509 seconds),
plus the added fallback regression (0.055 seconds). Follow-up static review
reported no remaining concrete finding. A record cannot choose a recovery when
the actual policy proposal was legal.
An initial broad build exhausted disk space before tests ran. Removing only
this worktree's generated client/train artifacts recovered 85.9 GiB; the
successful regression run selects library tests and avoids a client rebuild.

These are tested execution components, not a running global Tower service.
Durable reservation transactions, scoped authentication, gateway transport,
verification job supervision, the native shop/belt, global ranking projection
and permanent accomplishment delivery are still outstanding. No full-duel
balance or completed global gameplay claim follows from the fixture tests.
