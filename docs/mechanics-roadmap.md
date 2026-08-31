# Mechanics Roadmap — baylee

One upfront inventory of MTG mechanic families, mapped to the engine
capabilities they need. Goal: **no more card-sized engine changes** —
every future milestone is a family from this list, scheduled by value.

Size classes: **S** (< 100 engine LOC), **M** (100–400), **L** (> 400).

---

## A. Already supported (M0–M3) — do not reschedule

Keywords: flying, first/double strike, deathtouch, lifelink, vigilance,
haste, hexproof, indestructible, menace, reach, flash, unblockable,
changeling, prowess (synthetic), rebound, ward {1}/{2}, legendary/basic
supertypes, legend rule.

Cast machinery: alternative costs (pitch, evoke, commander-free),
additional costs (kicker), mandatory additional (pay X life), X spells,
modal spells, overload, modal triggers, MDFC faces (cast + land play),
miracle, delve, convoke (generic), flashback grants, free casts
(rebound/suspend finish), pitch-card exile.

Zones: tutors (SearchLibrary with destinations), graveyard recursion,
linked exile (Fiend Hunter), blink (immediate + end-step), phase-out,
bottom-of-library, reorder-top, scry, mill, bottom-from-hand.

Continuous/replacement: layer system (copy, text, type, color, ability,
P/T CDA/set/modify, counters), keyword/P-T/type/color mods, cross-zone
filters (Maskwood), token doubling, counter doubling, trigger
multipliers/suppressors, ETB tap/pay-life/choose-subtype modifiers.
**Layer 2 (control as a continuous effect) is NOT done** — control
change/exchange exists only as a one-shot resolution effect
(`change_controller`); there is no `Modifier` for it and `recompute`
discards layer 2. Track here, not under "already supported".

Triggers: ETB, LTB, dies, spellcast, draws (incl. except-first), attacks,
becomes-target, exiled-from-battlefield, combat-damage-to-player,
step-begin, once-per-turn, synthetic (prowess/ward).

Players: monarch, extra turns, no-lose, no-life-loss, damage prevention
(to/from), protection (damage/target/block), search locks, no max hand
size, poison/energy/rad counter storage.

Planeswalkers: loyalty abilities/costs, 0-loyalty death, damage →
loyalty removal.

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
- The gateway remembers standing answers per account and replays them into
  each new game (`/automation`, `docs/protocol.md`).


---

## B. Remaining engine extension points (genuinely new hooks)

These are the ONLY places needing new engine architecture. Everything in
section C maps onto these or onto section A.

| # | Hook | Shape | Size | Unblocks (acceptance) |
|---|------|-------|------|-----------------------|
| B1 | **Legality/activation conditions** | `ActivationCondition` on `AbilityDef::Activated` (ControlCount(filter), IfNotStartingPlayer, OnlyIfAttacked, etc.), checked in `can_afford`/legal enumeration | S | Mox Opal, Bleachbone Verge; future: metalcraft family, boast, imprinted abilities |
| B2 | **Cost reducers** | `Modifier::ReduceCost(filter, n)` consulted in `cast_options`/`wizard_cost` | S | Surgical Metamorph; future: affinity, goblin/tribal reducers, medallions |
| B3 | **Ability-granting statics** | `Modifier::GrantAbility(&'static AbilityDef)` — characteristics projection exposes granted abilities to activation enumeration | M | Chromatic Lantern, Urza's Saga ch. I/II; future: Nicol Bolas-style grants, level-up |
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

Covered by section B. After those land, all 194 acceptance cards are
`Implemented` except explicit out-of-scope riders.

### C1 — commander staple families (P1)

| Family | Needs | Size |
|--------|-------|------|
| Cycling / typecycling | hand-zone activation (exists) + DiscardSelf (exists) + draw/search | S |
| Blood/Clue/Food/Treasure/Map tokens | token defs with sac abilities (exists) + investigate/create variants | S |
| Landfall triggers | ETB filter with HasType(LAND) (exists — pure card work) | S |
| Evoke | alt cost + sacrifice-on-ETB (EntersBattlefieldEvoked exists) | S |
| Channel | hand-zone activation + DiscardSelf cost | S |
| Exert | tapped-status rider + "doesn't untap next untap" (skip-untap flag) | S |
| Crew (vehicles) | tap-creatures cost (convoke-payment reuse) + type-becomes-creature effect | M |
| Equipment/auras extras: living weapon, For Mirrodin!, reconfigure | token-on-ETB + attach (exists) | S |
| Proliferate | AddCounterFilter on "each player/permanent with counters" | S |
| Infect/toxic/wither | poison on damage + M1M1 damage mode (counter storage exists) | M |
| Undying/persist | dies-trigger with counter check + return-to-bf | S |
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
| Battles (siege) | new card type + defense counters + attack-the-battle | L |
| Classes | B12 | (B12) |
| Sagas | B7 | (B7) |

### C2 — multiplayer/politics (P2, needs protocol player-choice first)

Goad, melee, myriad, will of the council/vote, council's dilemma,
temptation, join forces, assist, hidden agenda. Most need M3 protocol
multi-target choices; engine support is otherwise small.

### C2b — regeneration (P2, no card in the pool needs it yet)

`Damn` and `Vindicate` both say "can't be regenerated", and the note in
`damn.rs` promised this entry — which was never written, so the family
was invisible. Nothing in the 194-card pool actually *regenerates*, so
the clause is currently vacuous rather than wrong.

What it needs when a regenerating card arrives: a per-object shield count
cleared at cleanup, consumed by destruction instead of the object dying
(tap, remove from combat, clear marked damage — CR 701.15), and a flag on
the destroying effect for the "can't be regenerated" clause that already
appears on several cards. Size S–M. Deliberately not built ahead of a
card: an effect with no card to exercise it is the exact shape of
`NthSpellCast` and `once_per_turn`, which sat declared-but-dead for
months.

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
