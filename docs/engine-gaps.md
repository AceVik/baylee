# Engine gaps — a ranking from four views and 23 test notes

This file is a **measurement of the card files as they stand**: 24 gaps
reconciled out of four separate views of the pool plus 23 test notes, ranked
by cards over depth, each with the first card that would close it and the
test that would prove it — and, in §5, eight contradictions between the views
named and deliberately left undecided.

`docs/mechanics-roadmap.md` answers a different question and has a different
lifetime. It is the plan: mechanic families, the hook each one needs, an
engine size class, a batch order, and it is meant to hold still. This file is
re-derived whenever the pool moves, which is why the counts live here and not
there — a family and its hook survive a card batch, a count does not. The
roadmap proves that against itself: C2b is headed "no card in the pool needs
it yet" and says "Nothing in the 194-card pool actually *regenerates*", while
G19 below counts eight cards whose `//! Oracle:` header prints regeneration.

The cross-reference therefore runs one way from here: §1 names which roadmap
hook a gap is, and the roadmap carries no counts.

## 0. How the counting works (otherwise the ranking is worthless)

The four views count three different populations, and none of them says so. A
"5" in one view and a "56" in another are the same gap:

| Basis | What is counted | Size |
| --- | --- | --- |
| **Partial** | Cards that already write the thing in the DSL and are `Coverage::Partial` for exactly that reason | 5–10 per gap |
| **Header** | `//! Oracle:` headers over all card files | 1537 files, 884 of them `// GENERATED STUB` (counted by hand here) |
| **Script** | `SCRIPT.txt` of the card batches under `target/card-batch/` | 791 stub scripts |
| **Sample** | A random sample of 30 unimplemented cards handed to an external agent, which refused 22 of them and named the missing DSL variant for each (commit `5d63c6ba`) | 22 refusals — **a later measurement, not one of the four views** |

Header and Script are not convertible into one another: the header reads the
printed line, the script reads the reference API. Where a number was counted
by hand here it says so; everything else is attributed to the view it came
from.

**Depth** is a scale, not prose:

1. An enum variant plus one arm in an existing function. The machinery stands.
2. A new field or new type plus one arm. A reader has to move with it.
3. A new stage on an existing seam (wizard stage, plan, slot list).
4. A new projected characteristic, or a subsystem.

Depth and the roadmap's **S/M/L** are two scales and do not convert into one
another: depth says *which seam* changes, S/M/L says how many engine lines it
costs.

**Lever = cards over depth.** The "blocked together with" column is what stops
the same card being booked as a win in three rows.

---

## 1. What is the same gap

| Merged as | Out of these raw findings |
| --- | --- |
| **G1 An activation cannot ask a question** | `partial`#1, `dsl`#1, `offers`#1, `offers`#2 (crew is the *set* form of the same question), `stubs`#1 (the 20 real activation costs with Reveal/Return/tapXType/ExileFromGrave), tests `krark_clan_ironworks`, `viscera_seer`, `survival_of_the_fittest` |
| **G2 A counter as a cost** | `stubs`#2, `dsl`#2 |
| **G3 Surveil** | `stubs`#4 |
| **G4 "unless you pay" with a real cost** | `stubs`#3, test `emiel_the_blessed` (the same field `Amount` → `Cost`, but branching on the **yes** instead of on the refusal) |
| **G5 "doesn't untap"** | `stubs`#6 |
| **G6 A chosen color** | `stubs`#5 |
| **G7 Combat restrictions** | `partial`#10 (Brazen Borrower), `dsl`#5 ("can't block/can't attack") |
| **G8 Removing, moving and proliferating counters as an *effect*** | `stubs`#7, partly `dsl`#2 |
| **G9 Target slots** | `dsl`#4 (two kinds of target in one sentence), `offers`#3 (an activation knows exactly one target), `partial`#5 (`FightSide::Target(u8)` indexes exactly that list) |
| **G10 An activation ceiling** | `stubs`#8, `offers`#4, test `liliana_the_repentant` (Exhaust) |
| **G11 A graveyard cast with its own cost** | `partial`#2, tests `faithless_looting`, `open_communications`, `sevinne_s_reclamation` |
| **G12 Storm / "copy the spell I belong to"** | `partial`#3, tests `flusterstorm`, `brain_freeze`, `sevinne_s_reclamation` |
| **G13 Affinity** | `partial`#4, tests `thought_monitor`, `emry_lurker_of_the_loch` |
| **G14 A non-target choice at resolution** | `partial`#7 (populate, Frantic Search), test `frantic_search` — shares its primitive (`Pending::ChooseCards` + `PlayerAction::ChooseObjects`) with G1 |
| **G15 Subtracting a subtype** | `partial`#6, test `borg_queen_perfection_manifest` |
| **G16 Servo/Germ tokens, and a handle on a freshly created object** | `partial`#5, tests `nettlecyst`, `marionette_apprentice` |
| **G17 A static with a condition** | `partial`#8 |
| **G18 Ability removal, projected** | `partial`#9, test `tishana_s_tidebinder` |
| **G19 Regeneration** | `dsl`#7 |
| **G20 Fight** | `dsl`#3 |
| **G21 Impulse Draw** | `dsl`#6 |
| **G22 Activation cost reduction** | `dsl`#8 (channel lands, Training Grounds) — **not** the same thing as G13 |
| **G23 ActivationTiming is two-valued** | `offers`#5 |
| **G24 The hand sweep asks about no targets** | `offers`#6 — a **defect**, not a gap |

Not merged, because they are adjacent and still different: `storm_kiln_artist`
(magecraft needs an *event* "a spell was put on the stack as a copy", which
the engine deliberately does not journal) does not belong to G12.
`dualcaster_mage` (`TargetSpec::Spell` hard-wires `UNCOUNTERABLE`) is a
one-card matter of its own. `derevi_empyrial_tactician` needs a third
`ActivationZone` plus an offer path, not G23. `golgari_thug` (Dredge) is an
"instead of drawing" replacement effect that `ReplacementRule` knows in none
of its four variants. `vampiric_tutor` is **not a gap** but a test workaround
(`optional: false` sends `min = 1`), and does not belong in this list.
`mental_misstep` (Phyrexian mana as life) is a real cost gap that **no** view
counted — carried unranked below.

**Which roadmap hook a gap is.** Six of these already have a name in
`docs/mechanics-roadmap.md`, and naming it here rather than there is what
keeps the counts in one file. G13 and G22 are both B2, the cost reducers. G11
is B9, the graveyard-casting family. G10's once-per-game half is C3's Exhaust,
which C3 itself calls a B1 flag. G19 is C2b. G5's "doesn't untap" is the
skip-untap flag C1 lists under Exert. The rest are not mapped here.

---

## 2. The ranking by lever

**The table stands in the order the ranking was drawn up in, and a later
measurement contradicts it.** G2 stands first because it unblocks the
commander Tayam. The
sample in §0 says otherwise: of the 22 cards the external agent refused,
**seven** named G1's missing variant, **two** G2's and **two** G5's (commit
`5d63c6ba`); how the remaining refusals break down is not in what was
reported. The two are different kinds of evidence — the ranking counts
`//! Oracle:` headers across the whole pool, the sample counts what 30
randomly drawn unimplemented cards actually ran into. **G1 is the larger lever
on that sample; G2 is the one with a named commander behind it.** The table is
left in its original order rather than re-sorted, so that the disagreement is
met where the ranking is.

| # | Gap | Cards (basis) | Depth | Lever | Rule or case | Blocked together with |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **G2** A counter as a cost, fixed number — **shipped**, see §1 | 43 cards (re-measured), 17 of them X storage lands | 1 | 20–40 | **case** | the X half with G1 |
| 2 | **G3** Surveil | 32 headers (by hand), 29 script | 1 | 32 | **case** | — |
| 3 | **G1** An activation asks a question | 50 headers with Sacrifice/Discard only (by hand); +9 tapXType, +4 crew, +20 script-counted activation costs, +~20 storage lands | 3 | ~27 | **new rule** on an existing seam | G2's X, G14, `offers`#2 |
| 4 | **G4** "unless you pay" takes a `Cost` | 17 headers (by hand), 36 script | 2 | 9–18 | **case** (field change) | 31 of the 36 need G1's CostParts |
| 5 | **G5** "doesn't untap" | 22 headers (by hand), 20 script | 2 | 11 | **new rule** (small) | 5 depletion lands also G2 |
| 6 | **G6** A chosen color | 29 headers (by hand, incl. "chosen color"), 24 script | 3 | ~10 | **new rule** | — |
| 7 | **G7** "can't block / can't attack / can block only" | 10 (7 "can't block" headers by hand, plus Brazen Borrower and Glacial Chasm) | 1.5 | ~7 | **case** at `can_block`, **rule** at `can_attack` | — |
| 8 | **G8** Removing, moving and proliferating counters as an effect | **contradictory: 1 against 13** | 1 | ? | **case** | G2 |
| 9 | **G20** Fight | 5 headers (by hand), 5 `dsl` | 1 (the word) / 3 (the reach) | 5 | **case**, the reach bound to **G9** | 4 of the 5 also G9 |
| 10 | **G19** Regeneration | 8 headers (by hand), 7 `dsl` | 2 | 4 | **new rule**, but with one funnel | Damn (`NoRegen$`) |
| 11 | **G21** Impulse Draw | 6 | 1.5 | 4 | **case** plus an expiry sweep | — |
| 12 | **G9** Target slots (how many, and a second kind) | 5 (`offers`) + 6 (`dsl`) = 11 | 3 | 3.7 | **new rule** | G20 |
| 13 | **G22** Activation cost reduction | 11 activated + 9 spells (`dsl`) | 2–3 | ~4 | spells: **case**; activation: **rule** (no seam) | — |
| 14 | **G23** The ActivationTiming window | 5 (`offers`) | 2 | 2.5 | **case** | — |
| 15 | **G16** Servo/Germ + `last_created` | 2 | 1 (data) + 2 (handle) | ~2 | data: **case**; handle: **rule** | — |
| 16 | **G10** An activation ceiling | **contradictory: 3 against 5** | 2 | 2 | **case** (field) plus a tally | — |
| 17 | **G13** Affinity | 2 | 1 | 2 | **case** | — |
| 18 | **G14** A non-target choice | 2 plus populate | 2 | ~1.5 | **new rule**, G1's primitive | G1 |
| 19 | **G11** A graveyard cast with its own cost | 3 | 2 | 1.5 | **case** (field change `bool` → struct) | — |
| 20 | **G15** Subtracting a subtype | 2 | 3 (layer 4 writes no subtypes today) | 0.7 | **new rule** | — |
| 21 | **G12** Storm | 3 | 3 (effect + `Amount` + cast-zone condition) | 1 | **new rule** | G11 (Sevinne) |
| 22 | **G17** A static with a condition | 1 measured, pool reach **unevidenced** | 2 | ? | **case** (field) | — |
| 23 | **G18** Ability removal, projected | 1 | 4 | 0.25 | **new rule** | — |
| 24 | **G24** The hand sweep checks no targets | 3, latent and primed | 1 | defect | **defect**, not a gap | G22 (channel lands) |
| — | Phyrexian mana as life (`mental_misstep`) | **not counted** | ? | ? | **case** in `mana_pay` | — |

---

## 3. The top six

Each entry is the same five: **(a)** what was found, **(b)** where it changes,
**(c)** the shape, **(d)** the test that proves it, **(e)** the first card.

### 1. G2 — a cost cannot take a counter off — **the fixed-number half shipped**

The entry below is what the audit proposed; what follows it is what was
measured when the work was actually done, because three of the audit's numbers
were wrong and one of its names was.

**(a)** `CostPart` (`crates/baylee-cards-dsl/src/cost.rs:8-31`, read here) has
eleven variants and not one of them names a counter. The *arithmetic* already
exists, at exactly one hard-wired place: according to view `dsl`,
`crates/baylee-engine/src/engine/abilities.rs:1169-1202` pays a loyalty cost
by reading `o.counters.get(CounterKind::Loyalty)`, refusing when there are too
few, writing `set` and journalling `GameEvent::CounterChanged` — the
`counters` map is already generic over `CounterKind`, and the only way to
reach it is `AbilityDef::Loyalty`, which `can_afford` never sees.
**(b)** `crates/baylee-cards-dsl/src/cost.rs` (the variant) and
`crates/baylee-engine/src/engine/abilities.rs` (`can_afford`, `pay_cost`).
**(c)** `CostPart::RemoveCounter { kind: CounterKind, n: u16 }`, plus in
`can_afford` `state.object(source).is_some_and(|o| o.counters.get(*kind) >=
*n)`, plus in `pay_cost` the `fn spend_counters(state, id, kind, n)` lifted
out of the loyalty block, which both callers then use. No question to the
player, so **independent of G1**.
**(d)** An engine test that puts a vivid land with a charge counter onto the
battlefield, activates it twice for mana and sees `can_afford == false` the
second time — plus the counter-check that `GameEvent::CounterChanged` is in
the journal. Without that second half the test shows only that something
failed.
**(e)** **Vivid Grove** (or another of the five vivid lands): the printed line
is exactly "{T}, Remove a charge counter: Add one mana of any color", one
clause and no second one. Gemstone Mine is the worse first card, because its
"if there are no counters, sacrifice it" is a second sentence.

#### What was measured when it was built (2026-09-16)

Counted over every `//! Oracle:` header in the pool rather than by hand:
**43** cards print a counter in an activation cost, not 40. **32** of them are
activated abilities; **31** of those take the counter off the source, and the
one exception is **Tayam, Luminous Enigma** — "Remove three counters from
among creatures you control", which is a question to a player and belongs with
the `Sacrifice(_)` family. So the variant shipped is `RemoveCounterSelf`, not
`RemoveCounter`: naming the source in the variant is what lets `can_afford` be
`counters.get(kind) >= n` and the payment be arithmetic, instead of putting a
chooser with one legal answer in front of a player. Tayam gets
`CostPart::RemoveCounterAmong` when G1 lands, and is the reason that is a
second variant rather than a wider first one.

And **17** of the pool's lands are X/any-number storage lands, not the "~20
of them a fixed number" the ranking assumed — the split is closer to even than
the entry reads.

**Shipped:** `CostPart::RemoveCounterSelf { kind, n }` and
`EnterModifier::WithCounters { kind, n }`, which are one unit rather than two
because neither is a card on its own: a land that enters with counters and
cannot spend them is a tapland, and an ability that spends counters a land
never gets is an ability nothing can afford. They take opposite doors —
`replacement::put_counters` for the counters a permanent arrives with
(CR 614.1c, so a Doubling Season applies) and `replacement::remove_counters`
for the cost (no multiplier exists in Magic for a removal). Seven cards came
out of `landgen` in full: the five Vivid lands, Mirrodin's Core and Tendo Ice
Bridge.

**Still open, and it is the X half.** The storage lands need an X in an
*activation* cost, which is exactly what does not exist: the documentation at
`abilities.rs:1375ff` says itself that `PayLifeX` is paid by the cast wizard
and skipped by `pay_cost`. X in an activation is G1's question. The depletion
lands (`Effect::IfNoCountersOnSelf`) and Devoted Druid (`PutCounterSelf` plus
an activation limit, G10) are the other two pieces.

### 2. G3 — no Surveil

**(a)** The DSL can look at the library (`Effect::Scry`, `ScryFor`), mill it
(`Mill`) and pick a card out of it (`LookAtTopPick`), but it cannot put
anything it has looked at into the graveyard. `rg -in 'surveil'` over
`crates/baylee-cards-dsl/src` and `crates/baylee-engine/src`: **no hits**
(checked here).
**(b)** `crates/baylee-cards-dsl/src/effect.rs`, directly beside `Scry`
(line 646 according to view `stubs`), and the matching resolve arm in
`crates/baylee-engine/src/resolve/`.
**(c)** The same pairing Scry already has:
`Effect::Surveil { amount: Amount }` and
`Effect::SurveilFor { player: PlayerRel, amount: Amount }`. The only
difference from Scry is the choice per card looked at — "graveyard or on top"
instead of "bottom or top". The `Pending` that Scry asks with is reused; a new
`Pending` here would be the warning sign that somebody is building more than
is needed.
**(d)** An engine test with a stacked library: Surveil 1, the answer
"graveyard", then `graveyard.len() == 1` **and** `library.len()` smaller by
one **and** the second-from-top card is now the top one. The counter-check
with the answer "on top" has to leave the graveyard empty — otherwise the test
only proves that something moved.
**(e)** **Elegant Parlor** (a surveil land, `TrigSurveil:DB$ Surveil |
Amount$ 1`): one line, no second mechanism. Conduit Pylons is its equivalent
twin.

This is the purest case of "existing rule, missing case" in the whole list: 32
cards for one enum variant beside one that is already standing there.

### 3. G1 — an activation cannot ask a question while its cost is being paid

> **Closed.** `engine::cost_wizard` asks the question and `start_activation`
> carries the answers back through `PlanKind::PayActivationCost`, so
> `choice_cost_unpayable` is gone and the six cards named below are
> `Coverage::Implemented`. The reading below is the state that motivated it
> and is kept as the argument; the counts it quotes are what is now
> *reachable*, not what is still blocked.

**(a)** `crates/baylee-engine/src/engine/abilities.rs:32` (read here):
`choice_cost_unpayable` matches exactly `CostPart::Sacrifice(_) |
CostPart::Discard(_)`; `can_afford` breaks off there **before it ever looks at
the board**, and according to view `offers`, `pay_cost` at `:1375-1379`
returns `IllegalAction("choice costs are not supported yet (M2)")` for the
same two — **after** emptying the mana pool and with no rollback. The
function's own documentation names the way out: "the day an activation can ask
a player which card to discard, this answers `false` and every reader relaxes
at once." Five cards already write the cost and are `Coverage::Partial` for
that reason (`viscera_seer`, `ashnod_s_altar`, `recurring_nightmare`,
`krark_clan_ironworks`, `survival_of_the_fittest`); **50** card files were
counted by hand here whose `//! Oracle:` header prints an activation line with
a chosen sacrifice or discard.
**(b)** A new module under `crates/baylee-engine/src/engine/` beside
`cast_wizard.rs`, plus the three gates in `abilities.rs`
(`choice_cost_unpayable`, `can_afford`, `pay_cost`), plus a prompt variant in
`crates/baylee-engine/src/choice.rs`.
**(c)** **Three views propose three shapes, all on the same seam** — between
`Pending::ChooseTargets` and `pay_cost`: `partial` wants
`ChoicePrompt::PayActivationCost` on the existing
`Pending::ChooseCards`/`PlayerAction::ChooseObjects` and nothing else new;
`dsl` wants `PlanKind::ActivationCost { source, ability, remaining, chosen }`
with `resume_activation_cost`, on the same seam as `PlanKind::LoyaltyPlayer`;
`offers` wants an `Engine::activation_plan: Option<ActivationPlan>` of its own
plus new `TargetPrompt::CostSacrifice`/`CostDiscard`, expressly so that it is
not read as a target choice. None is chosen here (see §5). All three agree
that `choice_cost_unpayable` disappears afterwards, that `can_afford` gets one
board question each (`!eval::matches_in_zone(...).is_empty()`), and that
`CostPart` has to gain `TapOther(&Filter)` at the same time — the third
variant, which is simply missing.
**(d)** Two tests. First the positive one: Viscera Seer plus two creatures,
activate, answer the question, the chosen creature is in the graveyard and
Scry has happened. Second, and more important, the guard against the
contradiction: a test that offers the activation, accepts it, and checks that
the mana pool is **untouched** when a cost is refused — that is the side
finding view `offers` made, and the one place where this rule destroys state
today. Alongside it, `offer_tests.rs:441`/`:490` and
`no_implemented_card_hides_an_ability_the_engine_will_never_offer` keep
running as the existing guards.
**(e)** **Viscera Seer**: one line, `cost!(Sacrifice(&Filter::YOUR_CREATURE))`
is already written, and the card is `Partial` for that alone. Krark-Clan
Ironworks and Survival of the Fittest fall in the same commit.

G1 is the only line in the list that is a **gatekeeper**: G2's X half, the 20
script-counted Reveal/Return/Tap/Exile costs, crew (4 cards, as the set form)
and G14 all share its primitive. By ratio alone it stands at 3, by downstream
effect at 1. The sample of 30 unimplemented cards puts it first outright:
seven of the 22 refusals named a cost that has to ask a question while it is
being paid (commit `5d63c6ba`).

### 4. G4 — "unless you pay" knows only generic mana

**(a)** `Effect::PlayerMayPayOr { player, mana: Amount, effect }`
(`effect.rs:868` according to view `stubs`) takes a mana amount and nothing
else; `EnterModifier` (`crates/baylee-cards-dsl/src/lib.rs:299-322`) has,
beside Tapped/TappedUnless/TappedUnlessCount, only `TappedOrPayLife(u16)`.
That makes every printed "unless you reveal/tap/return/sacrifice" unsayable.
17 card files with "unless … pay" in the header were counted by hand here;
view `stubs` counts 36 scripts with a non-pure `UnlessCost$`, 33 of them with
no PayLife either.
**(b)** `crates/baylee-cards-dsl/src/effect.rs:868` and
`crates/baylee-cards-dsl/src/lib.rs` beside line 317.
**(c)** No new enum, a **field change**: `PlayerMayPayOr { player, cost: Cost,
effect }`. `Cost { mana, parts }` covers the existing case as `Cost { mana,
parts: &[] }`, and every new `CostPart` out of G1 comes along for free.
Alongside it `EnterModifier::TappedUnlessCost(Cost)`, which turns
`TappedOrPayLife(1)` into `TappedUnlessCost(cost!(PayLife(1)))` and sends the
reveal lands down the same road. Emiel the Blessed is the same change with a
different branch — there the clause runs off the **yes** instead of off the
refusal; whether that becomes a second field (`on_paid`) or a second variant
is open, and should be decided in the same commit.
**(d)** A test with a mana-amount case that is green today, **unchanged** —
that is the proof that the field change breaks nothing — plus a new one that
refuses a `PayLife` cost and sees the alternative effect.
**(e)** As a *first card*, only one that uses a cost that already exists: a
land with "enters tapped unless you pay 1 life", through
`TappedUnlessCost(cost!(PayLife(1)))`. The reveal lands (Ancient Amphitheatre)
and tap lands (Command Bridge) profit only with G1's `CostPart`s — 31 of the
36 hang on that.

### 5. G5 — no "doesn't untap"

**(a)** `ReplacementRule`
(`crates/baylee-cards-dsl/src/static_ability.rs:336-368` according to view
`stubs`) has four variants, all about tokens, counters or
trigger counts, none about untapping; `Modifier` has 34 variants (read here,
lines 61-188) and none of them is it. `rg -E 'CantUntap|SkipUntap'`: no hits.
22 card files print it in the header (counted by hand here) — Basalt Monolith,
Grim Monolith, Mana Vault, the five depletion lands, ten filter lands with
"doesn't untap during your next untap step", the storage lands with "You may
choose not to untap".
**(b)** `crates/baylee-cards-dsl/src/static_ability.rs` (`Modifier`,
`Duration`) and `crates/baylee-cards-dsl/src/filter.rs`.
**(c)** `Modifier::DoesNotUntap` and `Modifier::MayChooseNotToUntap`, both on
layer 6 through `Modifier::layer` — the derived layer stays the modifier's
business, as `static_ability!(filter, modifier)` demands (CR 613.1). Alongside
them `Duration::UntilYourNextUntapStep`. For the five depletion lands that is
not enough: `static_ability!` needs a counter predicate there that `Filter`
does not have, so additionally `Filter::HasCounter(CounterKind, u8)` — which
those same lands also need for G2, which is why the two should be done
together.
**(d)** A test across two turns: tap Basalt Monolith, run through the untap
step, it is still tapped; then a counter-check with a second permanent that
does untap in the same step — without that counter-check the test only proves
that the untap step did not run at all.
**(e)** **Basalt Monolith**: the rule is the card's only special clause, and
it needs neither a filter predicate nor a duration, only `DoesNotUntap`.

"New rule" — but a small one: the untap step is a place in the code, not a
subsystem.

### 6. G6 — no chosen color

**(a)** The engine remembers a chosen *subtype* (`GameObject::chosen_subtype`,
`crates/baylee-engine/src/object.rs:530` according to view `stubs`) and no
chosen color; `EnterModifier` has `ChooseSubtype` and no color counterpart;
`Filter` has `MatchesChosenTypeOfSource` and no color counterpart;
`ManaSource` is Fixed/Choice/CommanderIdentity/LandColor, where `Choice` is a
choice *at activation* and not one fixed when the permanent entered. 29 card
files name a chosen color (counted by hand here, "choose a color" plus "chosen
color"), and view `stubs` counts 24 scripts, 14 of them with
`Produced$ Chosen`.
**(b)** `crates/baylee-cards-dsl/src/lib.rs` (`EnterModifier`),
`crates/baylee-cards-dsl/src/effect.rs` (`Effect`, `ManaSource`, `Amount`),
`crates/baylee-engine/src/object.rs` (the state field).
**(c)** `EnterModifier::ChooseColor { exclude: ColorSet }` (which covers the
gate lands' "Exclude$ white"), `Effect::ChooseColor { player: PlayerRel,
exclude: ColorSet }` for the activated form, `ManaSource::Chosen` and
`ManaSource::ChoiceWithChosen(&[ManaColor])` for "Combo W Chosen", and
`GameObject::chosen_color: Option<Color>` beside `chosen_subtype`. Nykthos
needs `Amount::DevotionToChosen` on top of that — a gap of its own, and it
should not hold the color choice up.
**(d)** A test with Mirage Mesa: ETB, choose a color, then tap for exactly
that color and **not** for another — the second half is the actual test,
because a `ManaSource::Choice` would pass the first.
**(e)** **Mirage Mesa** (`ETBReplacement:Other:ChooseColor` plus
`Produced$ Chosen`): only the choice on entering and the mana source, no
`Exclude`, no devotion.

---

## 4. New rule against existing rule, missing case

**Existing rule, missing case** (the machinery stands, what is missing is a
name for it):

- **G2** — the counter arithmetic including the journal entry existed in the
  loyalty block and was merely unreachable; `replacement::remove_counters` is
  the door it was lifted into.
- **G3** — Scry and Mill are there; Surveil is their third combination.
- **G4** — `Cost` instead of `Amount`, a field change.
- **G13 Affinity** — `casting::printed_reduction` is the place the sum is
  taken and it already has one arm; the second is missing.
- **G20 Fight** — according to view `dsl`,
  `deal_to_object_with_loyalty(…, source)` already takes the damage source as
  a parameter, and `Amount::TargetPower`/`SourcePower` work out both numbers.
  **But:** 4 of the 5 cards print two different kinds of target and therefore
  hang on G9 as well, which is a new rule. So: *the word* is a case, *the
  reach* is bound to a rule.
- **G21 Impulse Draw** — `Rider::PlayableFromExileFor` exists, is hashed and
  is read by both consumers; what is missing is the expiry and an effect that
  sets it.
- **G7 can't block** — `combat::can_block` already consults the effect table
  through `eval::protected_from`; **`can_attack` does not** and reads only
  zone, controller, type, DEFENDER, TAPPED and summoning sickness. Half case,
  half rule.
- **G23** and **G10** — one field each on both activation arms.
- **G11** — `Rider::Flashback` and the exiling on resolution exist in the
  engine according to view `partial`; only the DSL door is missing (`bool` →
  struct).
- **G16** (the token half) — two `TokenDef` statics in `tokens.rs`. Confirmed
  here: neither "Servo" nor "Germ" appears there.

**New rule** (there is no place that could ask the question today):

- **G1** — the activation path has no stage at which to stop. The seam is
  there, the rule is not.
- **G9** — `res.targets` is a flat list and `eval::target_options` is called
  once per ability; there is nowhere a second target list.
- **G6** — a new state field on the object, which has to go through
  serialisation and the view.
- **G14** — a choice at resolution that is expressly *not* a target choice, so
  that hexproof, shroud, ward and "becomes the target of" never see it.
- **G19 Regeneration** — new, but with a single funnel (`sba::destroy`), which
  makes it cheaper than its card count suggests.
- **G12 Storm**, **G15 subtracting a subtype** (layer 4 writes only `types`
  today, not `subtypes`), **G18 ability removal** (`abilities` has to become a
  projected characteristic), **G22** for the activation half (for spells it is
  a case), **Dredge**, **command-zone activation** (Derevi).

**Neither — a defect:**

- **G24**: the hand branch of the offer sweep (`abilities.rs:334ff`) checks
  zone and timing and then jumps straight to `can_afford`; the `target` field
  is not even destructured. `start_activation` then refuses the same
  activation with "no legal targets" — the engine contradicts itself. Latent
  today (0 of the 37 `ActivationZone::Hand` cards has a target), live from the
  first channel land onwards. That is a repair and not an extension, and it
  belongs before G22.
- **G1 side finding**: `pay_cost` empties the mana pool before returning `Err`
  for those same two CostParts, with no rollback. State damage, not a missing
  capability.

---

## 5. Contradictions between the views — named, not decided

1. **How many cards G1 blocks: 5 / 49 / 50 / 56.** Not a contradiction about
   the pool but three bases: 5 = cards that already write the cost in the DSL
   and are `Partial`; 49/50 = `//! Oracle:` headers with a chosen sacrifice or
   discard (50 counted by hand here, 49 by view `offers` — a regex edge); 56 =
   the same plus 9 "Tap an/another untapped" from view `dsl`. Whoever holds
   the 5 against G2's 40 is comparing two different questions. The sample in
   §0 is a fourth basis: 7 of 22 refusals (commit `5d63c6ba`).

2. **What shape G1 takes: three proposals.**
   `ChoicePrompt::PayActivationCost` on the existing Pending (`partial`),
   `PlanKind::ActivationCost` + `resume_activation_cost` (`dsl`), an
   `Engine::activation_plan` of its own + `TargetPrompt::CostSacrifice`
   (`offers`). All three sit on the same seam and differ in whether the
   question is asked as a *card choice* or as a *target choice with a
   different prompt*. That is a decision for the commit, not for this ranking.

3. **Whether a cost variant carries `u16` or `Amount`.** `dsl` proposes
   `RemoveCounter { kind, n: u16 }`, `stubs` proposes `RemoveCounter { kind,
   amount: Amount }`. That decides the card count: with `Amount::X` the ~20
   storage lands fall with it, with `u16` they do not. What speaks against
   `Amount` is that X in an activation cost has no payment path today (the
   documentation at `abilities.rs` says so expressly for `PayLifeX`) — so
   `Amount` would be a variant the engine accepts and does not pay, the worse
   half of exactly the pair that documentation warns about.

4. **Removing counters: effect or cost — 1 against 13.** View `dsl` claims
   that exactly **one** card in the pool (Ojer Pakpatiq) uses the wording as
   an *effect*, "so this is a cost gap and not an effect gap". View `stubs`
   counts **13** stub scripts with `RemoveCounter`/`MoveCounter`/`Proliferate`
   as `DB$`/`AB$`. Both can be right — `dark_depths` uses `AB$ RemoveCounter`
   as an *effect behind* a mana cost, which the header reads as a cost — but
   G8's rank hangs on it and is therefore left open. Counting by hand does not
   help here: a header grep for "move a … counter" also matches "**Remove** a
   … counter" and returns 34 instead of a handful.

5. **An activation ceiling: 3 against 5, and `Option<u8>` against
   `ActivationLimit`.** `stubs` counts 3 scripts with `ActivationLimit$ 1` and
   proposes `limit: Option<u8>`; `offers` counts 5 cards and proposes
   `enum ActivationLimit { PerTurn(u8), PerGame(u8) }`. The number decides the
   shape: Urza's Fun House ("only once and only if") and Liliana's Exhaust
   need *per game*, the three Rangers only *per turn*. 4 files with "only once
   each turn" were counted by hand here, one of them (Jin-Gitaxias) a
   **trigger** that `once_per_turn` already covers — so 3 activated ones plus
   the two special cases.

6. **"No modifier takes anything away."** View `partial` says of Tishana's
   Tidebinder that the giving half exists (`GrantActivated`, `GrantTriggered`)
   and has no removing twin, "I grepped the enum and found none". But
   `Modifier` does have `RemoveKeyword(KeywordSet)` and `LoseKeywords`
   (`static_ability.rs:77-79`, read here). The finding holds only for
   **non-keyword abilities** — which the card itself says correctly ("reaches
   only keyword bits") and the summary does not.

7. **The reach of G17 (a static with a condition).** What is measured is
   **one** card (Blackbloom Rogue). The claim "the pool-wide reach is much
   larger" rests on `CLAUDE.md`, where `IsPresent`/`Condition` is named as the
   `S:` transcoder's top remaining blocker. That is a statement about the
   *reference corpus*, not about this pool, and it has not been counted here.
   G17 therefore stands with a "?" instead of a lever.

8. **Two views contradict the brief, not each other:** view `offers` records
   expressly that there is no `CostPart::Tap(&Filter)`. That is correct — the
   eleven variants were read here, and none of them chooses an object other
   than the source except `Sacrifice`, `Discard` and `ExileFromHand`.

---

## 6. What nobody counted

- **Phyrexian mana as life** (`mental_misstep`): `mana_pay::can_pay`/`pay`
  treat `ManaSymbol::Phyrexian(c)` as "one mana of c, otherwise refuse"; "{U}
  or 2 life" is unreachable. None of the four views carries it as a line. The
  card is `Coverage::Implemented` all the same, because the half that is
  missing hangs on the printed cost and not on the card's DSL — which means
  `validate` will never touch it.
- **`TargetSpec::Spell` hard-wires `UNCOUNTERABLE`** (`dualcaster_mage`):
  right for Counterspell, wrong for every copy effect. One card, depth 1, but
  visible only because a test noticed it while it was being written.
- **The magecraft copy half** (`storm_kiln_artist`): no event names "a spell
  is put on the stack as a copy", because `CopyTargetSpell` deliberately does
  not journal it (CR 707.10). That is not a missing variant but a missing
  *observability*.
