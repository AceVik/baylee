# Mechanics Roadmap — baylee

One upfront inventory of MTG mechanic families, mapped to the engine
capabilities they need. Goal: **no more card-sized engine changes** —
every future milestone is a family from this list, scheduled by value.

How many cards wait behind each of these is measured rather than planned
and lives in `docs/engine-gaps.md`, which ranks what the current pool runs
into by cards over depth; this file names the families and the hook each
one needs, because a pool count kept here goes stale between batches —
C2b's "no card in the pool needs it yet" already has, against the eight
regeneration cards that ranking counts.

§E is the deliberate exception to that, and it is dated for the reason the
rule exists. It is about the *rates* — what fraction of the corpus a reader
reads, how the build grows per card, how the refusals are distributed —
which are properties of the corpus and the compiler rather than of this
pool, and which do not go stale between batches. Where it does state a pool
count it names the command that reprints it, so a reader can tell the two
kinds apart.

Size classes: **S** (< 100 engine LOC), **M** (100–400), **L** (> 400).

---

## A. Already supported (M0–M3) — do not reschedule

Keywords: flying, first/double strike, deathtouch, lifelink, vigilance,
haste, hexproof, shroud, defender, indestructible, menace, reach, flash,
unblockable, changeling, prowess (synthetic), rebound, ward {1}/{2},
legendary/basic supertypes, legend rule.

Every keyword in that list is read by a rule and covered by a test. The
four at the end of the list — hexproof, shroud, defender, flash — were
printed on cards and enforced by nothing until 2026-08-31: the bit was set
in `KeywordSet` and no code ever asked for it. If you add a keyword to the
enum, add the rule that reads it and the test that proves it in the same
change, or it will sit here looking supported.

Cast machinery: alternative costs (pitch, evoke, commander-free),
additional costs (kicker), mandatory additional (pay X life), X spells,
modal spells, overload, modal triggers, MDFC faces (cast + land play),
miracle, delve, convoke (generic), flashback grants, free casts
(rebound/suspend finish), pitch-card exile.

Zones: tutors (SearchLibrary with destinations), graveyard recursion,
linked exile (Fiend Hunter), blink (immediate + end-step), phase-out,
bottom-of-library, reorder-top, scry, mill, bottom-from-hand.

Continuous/replacement: layer system (copy, **control**, text, type,
color, ability, P/T CDA/set/modify, counters), keyword/P-T/type/color
mods, cross-zone filters (Maskwood), token doubling, counter doubling,
trigger multipliers/suppressors, ETB tap/pay-life/choose-subtype
modifiers.

Layer 2 is `Modifier::GainControl` at `Layer::Control`: the effect's
controller controls the permanent for as long as the effect lasts, and
gets it back when it ends (Mind Control, Act of Treason, Sower of
Temptation). `GameObject::controller` is the projected answer that every
rule reads; `base_controller` is where it goes back to, and only a
*permanent* handover (`set_controller`) writes that. A control change
restarts summoning sickness in both directions (CR 302.6). The one-shot
`Effect::ChangeController` is still the right tool for a permanent
handover — Gilded Drake, Homeward Path, Aminatou −6.

Triggers: ETB, LTB, dies, spellcast, draws (incl. except-first), attacks,
becomes-target, exiled-from-battlefield, combat-damage-to-player,
step-begin, once-per-turn, synthetic (prowess/ward).

Players: monarch, extra turns, no-lose, no-life-loss, damage prevention
(to/from), protection (damage/target/block), search locks, no max hand
size, poison/energy/rad counter storage.

Commander (CR 903): the command zone as a starting zone (903.6), casting
out of it with the `{2}`-per-previous-cast tax (903.8), colour identity
(903.4) and the "if you control a commander" condition cards print, the
graveyard/exile return offered as a state-based action (903.9a), the
hand/library half as a real replacement effect (903.9b), and
twenty-one combat damage from one commander as its own loss condition
(903.10a). Commander-ness lives on a list of `ObjectId`s in the state,
because it belongs to the *card* and not to a zone: every reader that
looked in the command zone was wrong at the one moment it was asked.
903.9b is asked before the effect moves anything and answered into
`GameState::commander_redirect`, so the card never touches the zone it
was headed for — a correction afterwards would end in the same zone and
be a different game; `docs/engine-internals.md` has the sites it does
not reach yet. Still open: the 40 starting life and the
colour-identity deck check the gateway would have to enforce; and the
view, which carries neither "this is a commander" nor a damage tally, so
a player cannot see either the tax they are about to pay or the counter
that is about to kill them.

Planeswalkers: loyalty abilities/costs, 0-loyalty death, damage →
loyalty removal, **being attacked** (CR 508.1b — an attack names a
`Defender`, which is a player or one of their planeswalkers; trample goes
to whatever the creature is attacking, and an attack on a walker that has
left deals nothing to anyone).

Misc: amass, token creation (incl. copies), clone-on-enter (permanent +
until-EOT via layer 1), spell copies, control change/exchange, lifelink
counters, choose-a-type, join-forces-free casts, suspend.

## A2. House rules (deliberate departures from CR)

Three, all implemented and tested in `engine::house_rules_tests` and
`engine::loop_tests`. They look like rules bugs to anyone reading the engine
against the Comprehensive Rules, so they are listed here rather than buried:

1. **The first mulligan is free** (`HouseRules::mulligan_free_first`,
   default on) — CR 103.5 charges for every one.
2. **With three or more players nobody skips their first draw step** — CR
   103.8a skips it for the starting player in every game; the skip exists to
   blunt a duel's first-turn advantage, which does not apply at a table.
3. **A real endless loop resolves once and is then broken**
   (`LoopPolicy::RunOnceThenBreak`, default) — CR 104.4b makes it a draw.
   A large-but-finite pile of work is never mistaken for one; see
   `crate::loops` and `docs/engine-internals.md`.

## A3. Seat automation (delegating priority)

A seat can hand back decisions it does not want to make, without the engine
guessing on its behalf:

- `PriorityHold` — pass when there is nothing to do / until the stack empties
  / until a named object resolves / until end of turn. Every hold cancels
  itself the moment the board changes under it, so a seat can never be left
  auto-passing through something it would have responded to.
- `StandingAnswer` per `AbilityRef` — "always yes to Ondu Cleric's rally".
  A question that can lose the game carries no ability handle, so no standing
  answer can ever reach one.
- The client keeps standing answers in the account's preferences
  (`ability_orders`, `/settings`) and sends them into each new game as
  `SetAbilityPolicy` (`docs/protocol.md` §"Standing answers").


---

## B. Remaining engine extension points (genuinely new hooks)

These are the ONLY places needing new engine architecture. Everything in
section C maps onto these or onto section A.

| # | Hook | Shape | Size | Unblocks (acceptance) |
|---|------|-------|------|-----------------------|
| B1 | **Legality/activation conditions** | `Condition` on `AbilityDef::Activated` (ControlCount(filter), IfNotStartingPlayer, OnlyIfAttacked, etc.), checked in `can_afford`/legal enumeration | S | Mox Opal, Bleachbone Verge; future: metalcraft family, boast, imprinted abilities |
| B2 | **Cost reducers** | `Modifier::ReduceCost(filter, n)` consulted in `cast_options`/`wizard_cost` | S | Surgical Metamorph; future: affinity, goblin/tribal reducers, medallions |
| B3 | **Ability-granting statics** — **shipped in `53039b68`**, spelled as two typed modifiers rather than one `GrantAbility(&AbilityDef)` | `Modifier::GrantActivated { cost, effects, mana_ability }`, read by `effects::granted_activated` and offered under `choice::GRANTED_ABILITY`; `Modifier::GrantTriggered`, read by `trigger.rs`. Still open: a static that functions from a zone other than the battlefield — a `StaticAbility` has no field saying where its source works from (Riftstone Portal). | M | Chromatic Lantern, Urza's Saga ch. I/II — both `Coverage::Implemented`, together with Forgotten Monument and Wrenn and Realmbreaker; future: Nicol Bolas-style grants, level-up |
| B4 | **Mana provenance** | pool entries carry optional source object + rider; wizard checks riders on spend (uncounterable, restricted, scry-trigger) | M | Cavern of Souls, Path of Ancestry |
| B5 | **Tap events** | journal `ObjectTapped` already exists → `Trigger::BecomesTapped(filter)` | S | City of Brass; future: Verity Circle, freeze auras |
| B6 | **Comparative conditions** | `Filter::GreatestCmc(filter)` / `Modifier`-free eval helper | S | Padeem; future: "highest power" checks |
| B7 | **Sagas** | lore counters on ETB + after draw step (turn-based), `Trigger::ChapterUp(n)`, sacrifice SBA after final chapter | M | Urza's Saga, The True Scriptures; future: all sagas, read-ahead |
| B8 | **Emblems with abilities** | emblem objects exist → include command zone in trigger/static scan | S | Venser −8; future: all walker ults |
| B9 | **Disturb / graveyard casting family** | cast-from-graveyard zone permission + face/cost override, exile-on-resolve | M | Mirrorhall Mimic; future: flashback keyword, unearth, escape, jump-start, embalm/eternalize, aftermath, encore |
| B10 | **Player hexproof** | player-targeting prevention in target_options (spell sources) | S | Everybody Lives! rider |
| B11 | **Permanent-spell copy tokens** | copy resolution: permanent spells resolve as tokens | S | Reflections of Littjara rider, Double Major |
| B12 | **Classes / level-up** | class counters per permanent + level-gated ability sets (builds on B3) | M | Wizard Class; future: all classes, level-up creatures |

Bundles: **B1+B2+B5+B6+B8+B10+B11 are all S and independent** — one small
engine iteration. B3, B4, B7, B9, B12 are the remaining real milestones.

---

## C. Mechanic family taxonomy (the schedule)

### C0 — acceptance partials first (B-hooks above, already sized)

Covered by section B. All 194 acceptance cards are `Implemented` and no card
file carries a `NOT SUPPORTED` rider any more — the last five (Double Major,
General Tazri, Doubling Season, Force of Negation, Mycosynth Lattice) named
mechanics that had since landed, and `baylee-engine`'s `card_rider_tests`
pins each of them.

### C1 — commander staple families (P1)

| Family | Needs | Size |
|--------|-------|------|
| Cycling / typecycling | hand-zone activation (exists) + DiscardSelf (exists) + draw/search | S |
| Blood/Clue/Food/Treasure tokens | done — `baylee-cards::tokens` (defs + sac abilities) | — |
| Map tokens | token def blocked on explore (library-top reveal + counter-or-bottom) | S |
| Landfall triggers | ETB filter with HasType(LAND) (exists — pure card work) | S |
| Evoke | alt cost + sacrifice-on-ETB (EntersBattlefieldEvoked exists) | S |
| Channel | hand-zone activation + DiscardSelf cost | S |
| Exert | tapped-status rider + "doesn't untap next untap" (skip-untap flag) | S |
| Crew (vehicles) | tap-creatures cost (convoke-payment reuse) + type-becomes-creature effect | M |
| Equipment/auras extras: living weapon, For Mirrodin!, reconfigure | token-on-ETB + attach (exists) | S |
| Proliferate | AddCounterFilter on "each player/permanent with counters" | S |
| Infect/toxic/wither | poison on damage + M1M1 damage mode (counter storage exists) | M |
| Undying/persist | done — keyword bits read by `trigger.rs`, with `GameState::ltb_counters` answering the intervening `if` | — |
| Modular | dies → move counters to artifact creature | S |
| Ninjutsu | hand activation + unblocked-attacker swap | M |
| Unearth | B9 graveyard casting + exile-at-end-step | (B9) |
| Escape | B9 + pay-exile-cards cost | (B9) |
| Jump-start/embalm/eternalize/encore/aftermath | B9 variants | (B9) |
| Split cards (fuse, aftermath) | face machinery (exists) + both-halves mode | M |
| Adventures | face machinery + adventure-zone rider (Rider::Adventure exists) | M |
| Foretell (costs) | Rider::Foretell exists + exile-from-hand activation + reduced-cost cast | M |
| Plot | Rider::Plotted exists + exile activation + free sorcery cast | M |
| Bestow | aura-or-creature cast modes | M |
| Mutate | merge-on-cast + top-of-stack characteristics | L |
| Battles (siege) | new card type + defense counters + a third `Defender` case (the handle itself already exists) | L |
| Classes | B12 | (B12) |
| Sagas | B7 | (B7) |

### C2 — multiplayer/politics (P2, needs protocol player-choice first)

Goad, melee, myriad, will of the council/vote, council's dilemma,
temptation, join forces, assist, hidden agenda. Most need M3 protocol
multi-target choices; engine support is otherwise small.

### C2b — regeneration (**done**, 23.09.2026)

Built exactly as this entry sized it, and the sizing is worth keeping
because it held: a per-object shield count (`Object::regeneration_shields`)
cleared at cleanup and on leaving the battlefield, consumed by `sba::destroy`
instead of the object dying — tap it, remove it from combat, clear its marked
damage (CR 701.19a) — and a `no_regen` flag on `Effect::Destroy` and
`Effect::DestroyAll` for the clause eight cards in this pool print
(CR 701.19c). `Effect::Regenerate` is the shield-making half.

Seven cards were waiting on it and are `Coverage::Implemented` now:
Elephant Graveyard, Swarmyard, Accursed Duneyard, Yavimaya Hollow, Lotleth
Troll, Thrun the Last Troll and Spawning Pool, whose animation *grants* the
ability rather than printing it.

The one thing the entry did not foresee is the coupling that made the flag
non-optional: nine cards were already `Coverage::Implemented` with a plain
`Effect::destroy` **because no shield existed**, so shipping the shield
without the flag in the same change would have made all nine quietly wrong.
That is the general shape and not a detail about regeneration — a clause is
vacuous only until the mechanic it names exists, and the commit that builds
the mechanic is the last one that can still see which cards were relying on
its absence.

What it deliberately did **not** build: nothing in `baylee-view` carries the
shield, so a client cannot draw one and the house AI fires removal into a
shielded creature as if it were not there. That is a view change with a
`VIEW_VERSION` bump behind it and no card needs it to be correct.

### C2c — fight, and a second instance of "target" (**done**, 23.09.2026)

`Effect::Fight` (CR 701.14) and its one-sided sibling
`Effect::DamageEqualToPower`, both naming their creatures by `TargetSlot`,
and the reach they were bound to: a second instance of "target" as its own
list at every layer (CR 115.3), narrowed on its own (CR 608.2b), asked as its
own stage in the cast wizard and in activation. Khalni Ambush, Bridgeworks
Battle, Stump Stomp and Contested Cliffs play; the house AI chooses both
creatures by what the fight would do (`baylee-ai/src/fight.rs`).

Not built, and each is a smaller entry now: a **mode** carrying a second
instance (Archdruid's Charm's second mode — `ModeDef` has one `targets`),
CR 707.10c re-choosing the second instance on a copy (only the first is
offered), Golden Guardian's delayed "dies this turn" return, and Arena's
"target creature of an opponent's choice", which is a target the *opponent*
chooses. Noncombat lifelink (CR 702.15b) is a separate gap that fight damage
would reach first: no fighting card in the pool has lifelink yet.

### C3 — newer-set families (P2, as needed)

Energy economy (counters exist; spend/gain effects), The Ring tempts
(emblem-like attachment), Amass variants, Roles (aura tokens), Bargain
(additional cost: sacrifice artifact/enchantment/token), Celebration
(trigger variant), Discover/Descend/Craft/Map (Ixalan set — exile-top-N
play, craft-from-exile), Rad counters (storage exists), Toxic/Corrupted,
Forage/Offspring/Gift/Valiant/Expend (Bloomburrow trigger+cost
variants), Impending (suspend variant), Exhaust (once-ever activation —
B1 flag), Harmonize/Max-speed/Start-your-engines/Station/Warp (2025–26
sets; mostly counters + triggered variants), Omenpaths (MDFC ✓ done).

### C4 — explicitly out of scope (long tail, documented not to chase)

Day/night, dungeons/venture/initiative/undercity, attractions/stickers,
subgames, ante, conspiracies, planechase, archenemy schemes, vanguard,
meld, companion (deckbuilding rule — gateway M4), sideboard wishes
(M4), learned lessons (M4), perpetual (digital), boon (digital),
conjure (digital), draft mechanics.

---

## D. Recommended batch order

1. **E1 (one engine iteration, all S):** B1, B2, B5, B6, B8, B10, B11
   → upgrades 8+ acceptance partials to Implemented.
2. **E2:** B7 sagas (+ B3 grant-ability if saga chapters need it) →
   Urza's Saga, True Scriptures; unlocks a major card family.
3. **E3:** B9 graveyard-casting family → Mirrorhall Mimic + future
   flashback/unearth/escape batch.
4. **E4:** B12 classes (+ Wizard Class) — B3-dependent.
5. **E5:** B4 mana provenance — last, only 2 acceptance cards need it.
6. Then C1 commander staples by deck demand (local-model friendly:
   most are pure card work on existing vocabulary).

Rule going forward: a card that needs a missing family **starts a family
milestone** (engine + all cards of that family), never a single-card
hack.

---

## E. Every card there is

§D schedules families against the acceptance decks. This is the other half:
how the pool gets from the cards four decks needed to the ones the ledger
numbers, what each step costs, and what stops it. Every number below was
measured on 2026-09-18 and the command that produces it is named, because
the point of the section is that the plan is re-derivable rather than
believed.

### E1. The ceiling and the reach are different numbers

The ledger numbers **33 694** cards and the pool compiles **1616** — 4.8%.

Of those 33 694, **33 368 (99.0%) have a reference script**. The 326 that
do not are Un-set cards (Ashnod's Coupon, Goblin Mime, R&D's Secret Lair)
and meld results (Brisela, Chittering Host), and the second group is not a
card a deck holds. That is the **ceiling** on what a reader could ever
reach, and it is essentially everything.

The **reach** is a different number and much smaller: 4619 of the
reference's 33 826 scripts (13.7%) are read in full today
(`xtask transcode-report`). The distance between 99% and 13.7% is entirely
DSL work. Nothing has to be sourced, licensed or typed in.

A third number belongs beside them: **387 of the ledger's rows are not
cards anybody plays in a normal game** — 184 planes, 107 vanguards, 102
schemes, 29 conspiracies, 21 phenomena, 6 dungeons. They matter out of
proportion to 1.1%, because a plane's type line is `Plane — Zendikar` and
Magic's plane types have **no Scryfall catalog**, so there is no eighth
`SubtypeKind` to give them and every one of them is a `validate` finding
the day it enters the pool. Whether they are in scope at all is a corpus
question, and `data/corpus-keep.tsv` is hand-kept — it is not this
document's to decide.

### E2. Four producers, and the rule that orders them

| producer | reads | written in this pool | cost per card |
|---|---|---|---|
| `landgen` | a land's printed text | 535 | none |
| `scriptgen` + `transcode_card` | a reference script | 34 | none |
| DeepSeek V4.1 Flash lane | the printing and `docs/card-dsl.md` | 42 of 91 asked (46%) | ~0.1–0.4M input tokens per 30 cards |
| Gemini 3.8 Flash lane | the same | 19 of 93 asked (20%) | minutes of a quota that refreshes |
| by hand | anything | 328 | a person |

The pool stands at **1616 cards: 719 stubs, 569 machine-owned, 328
hand-written**, which is what `xtask validate` prints at the end of a run.
Of the 897 finished cards **63% were written by a reader** — and almost
all of that is `landgen`, because this pool is 1124 lands. The transcoder
has written 34.

**The rule: never spend a lane on a card a reader could write, and never
spend a person on a card a lane could write.**

It is not thrift. A reader is the only producer whose output is testable
*as a rule*: a card the transcoder wrote is a rule's output, so a wrong
transcoding is fixed once and hundreds of files follow, and the two
ownership markers are what make that safe — `codegen` rewrites every file
carrying either, on every run. A card a lane wrote is one card, and fixing
it fixes one card. `xtask adopt` is the one door out of that, and taking
it costs a card its rule.

### E3. The residue is not the population, and the difference is 4400 cards

`transcode-report --stubs` reads **0 of this pool's 711 stubs** in full,
and corpus-wide the same transcoder reads **13.7%**. Those two numbers look
like a contradiction and are a selection effect: a stub is by construction
a card no reader could write, so the stubs are exactly the transcoder's own
refusals and its rate over them is 0 by definition.

The consequence decides where a batch comes from, so it was measured
rather than reasoned about. **300 ledger cards nobody has attempted**
(seeded draw over the 31 776 candidates), added to the pool in a throwaway
worktree, `codegen`, `validate`:

```
Stichprobe 299: 42 vom Leser geschrieben, davon 42 Implemented
Trefferquote: 14.0%   (Korpusrate 13.7%)
validate: 1915 cards, 2 problems — beide Planes, keiner der 42
```

**14.0%, and every one of the 42 clean against its printing.** So roughly
**4400 finished cards are available today, at no cost and with no DSL work
at all**, and the pool holds 1616. The first batch is not an LLM batch. It
is adding the cards the reader already reads.

The extrapolation held. `xtask reach-list` was written afterwards and counts
the same population exactly rather than from a sample: 500 of them are in the
pool now and it names 3946 more, which is 4446 against an estimate of 4400
drawn from 299 cards. §E8 step 1 is where that count lives and moves.

The probe paid for itself twice over besides: 299 random ledger cards
found the plane-type hole above, and one collision — the pool naming a
card by its front face where the ledger names both — which `codegen`
turned into a duplicate module and, because `xtask` links `baylee-cards`,
into a state `codegen` itself could no longer repair.

Working the residue is the *other* job, and it is the one that grows the
DSL. Both are worth doing; a commit says which.

### E4. Two worklists, measuring two different gaps

`transcode-report --stubs` ranks what the **reader** cannot read: a script
refused, named by its *first* refusal.

`data/card-refusals.tsv` ranks what the **DSL** cannot say. It is 115
refusals written by a model that read the printed card and then went
looking for the variant that would express it, each naming what it could
not find and the nearest thing that exists. By the type the missing
variant belongs to: an `Effect` in 67, a `Filter` in 37, an
`EnterModifier` in 27, a `Modifier` in 17, a `CostPart` and an `Amount` in
15 each.

They are not the same list. A script can be refused for a sentence the DSL
says perfectly well — `Sacrifice`, `cost 'Sac'` and `cost 'tapXType'` all
were — and a card can be refused for a sentence no script shape covers. A
commit says which of the two it aimed at, the same way it already says
whether it ranked the corpus or the stubs.

Three arithmetic rules govern reading either one, and each was paid for:

- A script is listed under its **first** refusal, so closing a cause moves
  every card that had it to whatever it is refused for next. The cause
  going to zero measures the rule; the cards finished measure the residue.
- An entry standing in front of a card with two refusals is worth less
  than its count. Cut the top blocker out with a throwaway patch and
  re-measure before committing to it — `AlternateMode:` was 91 pool stubs
  and at most 16 of them.
- An entry is a **sentence**, not a subsystem. Four times now the thing it
  named was already built and one variant away from being sayable.

### E5. The tail is the plan, and it is 400 rules long

`transcode-report --causes 0` prints the whole distribution rather than the
top 30, and it is the number that decides how long this takes:

| top *n* causes | of the 29 207 refused scripts |
|---|---|
| 10 | 16.7% |
| 25 | 28.7% |
| 50 | 41.3% |
| 100 | 55.2% |
| 200 | 70.1% |
| 400 | 84.7% |
| 800 | 94.5% |

**1996 distinct causes**, and 875 of them stand in front of exactly one
script. There is no small set of rules that unlocks the corpus: the top ten
— `AlternateMode:`, `Charm`, an unreadable `Pump` value, the `DamageDone`
trigger, `Discard`, `Effect`, `PumpAll`, `ReduceCost`, `Dig`, and scripts
that read as an empty card — together account for one refusal in six.

Read the table with §E4's first rule in mind, or it flatters: closing a
cause moves its scripts to their *next* refusal, so the cumulative column
is an upper bound on what closing those causes yields and the count of
causes is a lower bound on the work. What the shape does say reliably is
that "every card" is a programme of some hundreds of DSL rules, and that
the last few per cent will never be a rule at all — 875 one-off causes is
where the lanes and the hand-written half live permanently.

### E6. What bounds throughput is the build, not the model

`baylee-cards` compiled against copies of its own card tree, `cargo check`,
`CARGO_INCREMENTAL=0`, every point measured twice (the repeats agree to
within 0.2%):

| cards | check | max RSS |
|---|---|---|
| 1 616 | 4.35 s | 808 MB |
| 3 232 | 6.77 s | 1002 MB |
| 4 848 | 9.16 s | 1183 MB |
| 6 464 | 11.6 s | 1361 MB |
| 9 696 | 16.5 s | 1733 MB |
| 12 928 | 21.5 s | 2083 MB |

**Linear, and not nearly linear** — least squares gives 1.517 ms and 115 KB
of resident memory per card over a fixed 1.85 s and 634 MB, with R² of
0.9999 on both. At the ledger's 33 694 cards that is **53 s for a check and
4.3 GB in one rustc process**.

The time is a nuisance and the memory is the constraint, because it is one
process and it is not divisible: 4.3 GB alongside the rest of a
`--workspace` build is what would break a 16 GB runner, not the developer's
machine. That is the argument for splitting `baylee-cards` when the pool
passes roughly ten thousand cards, and the split has a second reason that
is easy to miss — **`baylee-client` links `baylee-cards`**, so every card
in the pool compiles for `wasm32` and ships in the browser bundle (39 MB
optimized today) and in the APK. An engine process needs every `CardDef`; a
client needs the table it is sitting at, plus whatever `LocalHost` deals.
Whether that makes the split a Cargo feature or a crate boundary is a
decision about client scope and is §E7's.

Two other sizes for the same shelf. Card source today is 1617 files, 47 396
lines and 2.0 MB, so the ledger's size is about 33 700 files, ~990 000
lines and ~42 MB, with `cards/mod.rs` going from 3249 `#[path]`
declarations to about 68 000. And two of the generated tables do not grow
at all: `BY_INDEX` and `ABILITY_LINES` are indexed by `CardIndex` and are
**already** 33 694 rows long, mostly `None` and `&[]`. The pool filling up
makes them denser, not longer.

### E7. The gate that makes volume safe, and the one thing it cannot scale

Three checks stand between a written card and the deckbuilder offering it,
and two of them scale for nothing:

- **The honest-stub rule.** One unread clause and the card stays
  `Coverage::Unimplemented`. It is what makes a generated `Implemented`
  mean what a hand-written one means, and the probe above is what that is
  worth: 42 generated cards, zero findings against their printings.
- **`xtask validate` against the printing** — oracle text, type line, mana
  produced, cost, activation cost, player target, target count, optional
  clause, ability-defined P/T — for the whole pool in one run, from a
  cached payload. It is in `scripts/gate-rules.sh` as of 0d46aaa3, which
  is where it should have been all along — and running it locally still
  does **not** mean what CI means by it, because `fetch_named` answers from
  disk and never refetches: a developer validates against the day the cache
  was filled, CI starts cold and validates against live Scryfall.

  The half of that gap which would have *grown* with the ledger is closed
  (#50). The check used to hold a card's header against Scryfall's
  **current default printing** for the name, which a reprint set moves: 32
  of 1616 cards went red for a change nobody made, and at the ledger's size
  that is ~660 of 33 694 — a gate that fails on somebody else's release
  schedule is not a gate. A header names one printing by id, an id does not
  move, and that is what it is now read against. What is left of the gap
  scales with the pool but is *quiet*: oracle text, type lines and costs are
  still whatever the local cache holds, and only CI reads them live.
- **One played engine test per card**, which
  `.claude/hooks/require-card-tests.py` asks for.

The third does not scale, and pretending otherwise is how 33 694 cards
would arrive untested. It does not have to: the ownership split already
says what a test is *about*.

A **machine-owned** card is a rule's output, so what owes a test is the
rule — beside the rule, failing against the old code, which is the
regression rule this repo already states for a fix. A reader change that
moves *n* cards owes one played engine test per new rules shape it
reaches, plus a **reading sample** of the cards it wrote, read against
their printings by a lane that did not write them. That is the Gemini
lane's job by its own scoreboard: it refuses more, and its refusals are
precise engine tickets. The sample is a floor and a fraction rather than a
number — **every new rules shape at least once, and at least 20 cards or
5% of what the change moved, whichever is larger**. A disagreement found
that way is a reader bug, so it is fixed in the reader and gets its test
beside the reader, never in the card.

A **hand-written or lane-written** card owes exactly what it owes today: a
played engine test written by somebody other than whoever wrote the card.
That is the cross-lane rule, and the Mikaeus failure is why.

### E8. The order, and what is still the owner's to say

1. **Take the cards the reader already writes.** In batches small
   enough that a bad reader day is one revert, with `codegen --check` and
   `validate` green before each. This needs no DSL work and no lane.

   `cargo run -p xtask -- reach-list --out <file>` is what names them, and
   it walks the **ledger** rather than the corpus: a card with no row stops
   `codegen` outright, and the ledger's spelling is Scryfall's, which is what
   `data/card-pool.txt` requires. Measured 19.09 before batch 3: **3546** of
   the 31 261 unattempted rows that carry a reference script read in full.
   The number is a worklist and not a promise — `scriptgen` claiming every
   clause is one reader agreeing, while `stubgen::transcode_card` still needs
   a printing — but the one independent check of it came out exact: of the 300
   names the §E probe added at random it lists 42, and the probe finished 42.

   It moves when the *reader* does, and downwards is the healthy direction:
   the same command said 4112 over a pool five hundred cards smaller, and 66
   of that difference is one refusal learned in between (a spell's additional
   cost, #52). Re-measure before every batch rather than slicing the list the
   last one was cut from — twelve of batch 2's proposed names were cards that
   fix had since taken off it. A re-measure that moves by *exactly* the size
   of the last batch, as batch 3's did, is the other half of the reading: it
   says the reader stood still, so the batch is the same list one page on.

   The batch itself is five steps, in this order: append the names to
   `data/card-pool.txt`, fill the payload cache (`scryfall-cache`, one bulk
   download rather than one request per card), run `codegen` **twice** because
   it is two-phase, hold it with `codegen --check` and `validate`, and then
   the full gate — the pool-wide engine sweeps are what actually play the new
   cards, and they are in it.

   Expect a batch to find things in the **tools** rather than in the cards.
   Batch 1 was a hundred cards of Alpha and Arabian Nights, all hundred
   finished, and it turned up two: `validate` read "add three mana of any one
   color" as a promise of nothing, and `printed_tests` counted Auras against a
   ceiling somebody then had to raise by hand. Both are checks meeting printed
   text they had not seen, which is what volume does first.

   Batch 2 was four hundred, all four hundred finished, and it turned up
   three of a harder kind. One was in the **reader**: a spell's `Cost$` is its
   mana cost plus whatever else the card charges (CR 601.2b) and the spell
   branch dropped the second half in silence, so Crop Rotation was already
   shipping as a one-mana tutor that sacrifices no land. The other two were
   pool-wide sweeps meeting a shape they had assumed away — `claim_tests`
   holding a card's sentence against a press nobody made, because cycling is
   a button that is not the cast, and the Karoo family's population test
   meeting the first non-land ever to print that sentence. A batch's real
   yield is that kind of finding: each of them was one rule wrong for every
   card it touched, surfaced by whichever card happened to arrive first.

   Batch 3 was six hundred — the limit is not the cards but how many sweep
   failures one sitting can diagnose — and all six hundred finished. Its
   three findings were a reader, a tool and a lint. `scriptgen::pump_amount`
   read every `X` as the number the player announced, where its own sibling
   `amount` six lines above had demanded `SVar:X:Count$xPaid` and an ability
   that announces one since the day it was written: 207 reference scripts
   pump by `X`, all of them define `SVar:X`, and only 44 mean the announced
   number, so seven new cards *and three already in `main`* stood at
   `Coverage::Implemented` and pumped by nothing. `xtask`'s `quoted_value`
   stopped at the first `"` and could not read a card whose name prints one.
   And the five-basic-subtypes pool lint met the single card its claim is
   untrue of, and now asks the printing rather than the filter.

   Three batches, 1100 cards, every one of them finished, and eight findings
   — of which exactly one was in a card. That ratio is the argument for the
   step, and the reason to keep taking it in batches that a person reads.
2. **Then the residue, ranked `--stubs`**, one cause at a time, each cause
   cut out first to see what is behind it.
3. **Then the corpus ranking**, which is where a rule buys hundreds of
   cards that are not in the pool yet — and which pays off only in
   combination with step 1, because a rule with no card behind it in this
   pool finishes nothing here.
4. **The lanes take what is left of a batch**, DeepSeek for volume and
   Gemini for the reading sample and the refusals, never the same model on
   a card and its test.

Three decisions are not this document's:

- **Are the 387 variant-format objects in scope** — planes, schemes,
  vanguards, phenomena, conspiracies, dungeons? They need an eighth
  subtype kind that Scryfall publishes no catalog for.
- **Does `data/card-pool.txt` invert** from an allow-list into
  ledger-minus-exclusions once the pool is most of the ledger? The file is
  hand-kept and so is `data/corpus-keep.tsv`.
- **Does the client keep the whole pool**, or does the split in §E6 cut
  along what a seat can be dealt?
