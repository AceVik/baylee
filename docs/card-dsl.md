# Card DSL Cookbook (frozen at M2.S8)

**This is the authoring contract for card implementations — humans and LLMs
alike.** If a mechanic is not expressible with the vocabulary here, the card
gets `Coverage::Partial("exact reason")` and a `// NOT SUPPORTED:` comment.
Never hack around the DSL; extend the DSL instead (in a new milestone).

## File standard (one file per card)

Location: `crates/baylee-cards/src/cards/<slug>.rs`. Header is mandatory and
must be kept truthful (it's the human-verification surface):

```rust
//! Lightning Bolt — {R} — Instant
//! Oracle: Lightning Bolt deals 3 damage to any target.
//! Set: M11 #149 — Magic 2011 | Scryfall ID: <uuid> | Oracle ID: <uuid>
// IMPLEMENTED — one-line summary of the implementation.

use baylee_cards_dsl::prelude::*;
```

One import, and it is the only one most cards need — the prelude carries the
whole vocabulary plus the macros below. Add a second `use` line only for
something outside it: a subtype module
(`use baylee_core::generated::subtypes::creature;`), a token
(`use crate::tokens::…`), or a shared filter (`use crate::filters::…`).

Do **not** add `#![allow(unused_imports, missing_docs)]`. It used to be on
every card file because the generated import list was identical for every
card whether the card used it or not; it is gone, and with it the two dozen
genuinely dead imports it had been hiding.

Rules:

1. **Every oracle sentence maps to an ability/effect** or to an explicit
   `// NOT SUPPORTED: <reason>` comment near the affected ability.
2. Keep `index`/`oracle_id`/`scryfall_id`/`faces` data from the generated
   stub untouched. Only edit `coverage`, `keywords`, `abilities`. State only
   what the card prints — see *Only state what the card prints* below.
3. Declare layers + durations explicitly for continuous effects. The engine
   deregisters effects structurally — never hand-roll removal.
4. Every card ships with `#[cfg(test)] mod tests` (resolution, targeting,
   edge cases, one interaction test per non-trivial mechanic).

## Only state what the card prints

`CardDef` and `FaceDef` both carry a `DEFAULT` associated const, and every
card file ends its literals with a struct-update tail:

```rust
card! {
    index: 104,
    oracle_id: "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id: "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces: &[face! {
        name: "Ondu Cleric",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::CLERIC, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[/* … */],
}
```

`card!` and `face!` *are* those literals — they expand to `CardDef { … ,
..CardDef::DEFAULT }` and `FaceDef { … , ..FaceDef::DEFAULT }`. Two things
come with them: the tail can no longer be forgotten, and `card!` writes the
doc comment on the `pub static CARD` it defines. The three identity fields
are mandatory and come first, in the order codegen writes them.

Never write a field back just to restate its default (`loyalty: None`,
`delve: false`, `partner: PartnerKind::None`, …) — a reviewer should be able
to read the literal as the card's printed face. Adding a field to `FaceDef`
then costs one line in `baylee-cards-dsl` instead of one line in ~200 card
files.

Two defaults are the *pessimistic* value rather than the common one, and
both are load-bearing:

- `CardDef::DEFAULT.index` is `0`, which collides with card 0. The
  `every_card_sits_at_the_index_it_claims` test in `baylee-cards` turns a
  forgotten index into a build failure instead of a card that silently
  resolves as another one.

  Where the number comes from: `data/card-index.tsv`, the append-only ledger
  codegen keeps. A `CardIndex` is an identity, not a position — `DeckEntry`
  stores one, the gateway persists decks made of them, and a replay names
  them — so adding a card takes the next free index and never renumbers a
  card that is already there. A card that leaves the pool retires its index;
  the slot stays empty (`BY_INDEX` holds `None` there) rather than being
  handed to someone else. Never edit the ledger by hand: `codegen --check`
  fails if a run would change it.
- `CardDef::DEFAULT.coverage` is `Coverage::Unimplemented`, so a stub that
  was never finished cannot reach the deckbuilder as playable just because
  a line went missing. An implemented card writes
  `coverage: Coverage::Implemented` by hand.

`FaceDef::DEFAULT.castable_from_hand` is `true`; only disturb and adventure
backs opt out.

## Generated cards, and why they may say `Implemented`

Most card files are hand-written. Two of them are not, and the distinction
matters when you open one:

- **Lands** are read from their printed text by
  `crates/baylee-cards-codegen/src/landgen.rs`.
- **Everything else with a local forge-reference script** is read by
  `crates/baylee-cards-codegen/src/forgegen.rs` (the checkout is an
  automated lookup, never copied and never part of the build).

Both write the *same* file standard as this document describes: the macros,
one `use baylee_cards_dsl::prelude::*;`, the `//!` header `xtask validate`
checks, and `Coverage::Implemented` only when the reader consumed **every**
clause of the card. An unknown effect, a parameter no rule claimed, a
computed amount, a keyword that is data rather than a bit — any one and the
whole card is refused and generated as an ordinary stub instead. There is
deliberately no partial path: a card that claims `Implemented` while quietly
dropping "and they can't be regenerated" is worse than no card, because the
deckbuilder offers it as playable.

A generated file is hand-owned the moment it exists: `codegen` only writes
files that are missing or still carry `// GENERATED STUB`, so editing one is
exactly like editing any other card file.

If you want more cards generated, the lever is usually **this document's
vocabulary**, not the readers. `cargo run -p xtask -- forge-report` ranks
what the corpus is waiting on, and the top entries are effects the DSL cannot
express at all yet.

## The vocabulary

### Card faces & costs

- `mana_cost: baylee_core::mana!("{2}{W/U}{W/P}")` — compile-time parsed
  (generic, color, hybrid, 2-or, Phyrexian, hybrid-Phyrexian, snow, X/Y/Z).
- `FaceDef.alternative_costs: &[AlternativeCost { cost, condition }]` —
  pitch/evoke/conditional-free (conditions: `Always`, `NotYourTurn`,
  `CommanderControlled`).
- `FaceDef.additional_costs: &[Cost]` — kicker (optional, yes/no at cast).
- `FaceDef.mandatory_additional_costs: &[CostPart]` — e.g. `PayLifeX`.
- `Cost { mana, parts }` — parts: `TapSelf`, `UntapSelf`, `SacrificeSelf`,
  `Sacrifice(filter)`, `Discard(filter)`, `DiscardSelf` (cycling),
  `PayLife(n)`, `PayLifeX`, `ExileSelf`, `ExileFromHand(filter)`.

### Ability kinds

- `AbilityDef::Spell { effects, targets: Option<TargetReq> }`
- `AbilityDef::Activated { cost, effects, target, timing, mana_ability, zone }`
- `AbilityDef::Triggered { trigger, effects, targets, once_per_turn }`
- `AbilityDef::Static(StaticAbility { layer, filter, modifier, cross_zone })`
- `AbilityDef::Replacement(ReplacementRule)` — trigger multipliers/suppressors,
  token/counter doubling
- `AbilityDef::Loyalty { cost: i8, effects, target }`
- `AbilityDef::CopyOnEnter { target, mods: &[CopyMod] }`
- `AbilityDef::ModalSpell { modes: &[SpellMode] }` — overload & friends
- `AbilityDef::ModalTriggered { trigger, modes, once_per_turn }` — "choose
  one/up to one" ETB triggers (decline = an empty mode)
- `AbilityDef::Ward { mana }` — engine-level synthetic trigger (like
  prowess), supports ward {1}/{2}
- `AbilityDef::Suspend { counters }`

#### Write them through the macros

The five shapes that make up most of the pool have a macro that supplies the
fields the rules already imply, so an ability states what the card says and
nothing more:

```rust
mana_ability!(&[Effect::mana(ManaColor::Green, 1)])   // {T}: Add {G}
mana_ability!(SAC_COST, ANY_COLOR_MANA)               // any other cost
activated!(Cost::TAP, EFFECTS)                        // {T}: …
activated!(EQUIP, EFFECTS, timing: ActivationTiming::SorcerySpeed)
triggered!(Trigger::EntersBattlefield(&Filter::This), EFFECTS)
spell!(EFFECTS)
spell!(EFFECTS, targets: Some(TargetReq::one(&Filter::CREATURE)))
loyalty!(-3, EFFECTS, target: Some(TargetSpec::Object(&Filter::CREATURE)))
mode!(DRAW_EFFECTS)                                   // one arm of a modal
```

The required arguments come first and positionally, because they are the
ones an ability cannot be written without; everything after them is
`field: value` in any order, and anything left out takes its rules default:

| field | default | why that is the rules answer |
| --- | --- | --- |
| `timing` | `InstantSpeed` | CR 602.2 — unless the card restricts it |
| `mana_ability` | `false` | CR 605.1 makes it the exception |
| `zone` | `Battlefield` | CR 113.6 |
| `target` / `targets` | `None` | an ability targets only when it says "target" |
| `once_per_turn` | `false` | a trigger fires on every occurrence |

`mana_ability: false` is the load-bearing one: an ability wrongly marked
`true` would silently skip the stack, and no test would read that as a rules
bug. That is why it is a default you have to opt *out* of, and why a mana
ability gets its own macro rather than a flag.

A shape without a macro (`Static`, `Replacement`, `CopyOnEnter`, `Ward`,
`Suspend`, `ModalSpell`, `ModalTriggered`, `SagaChapter`, `Echo`,
`Prepared`) is written as the plain enum literal — those have no fields the
rules can supply for you.

### As-it-enters modifiers (`FaceDef::enter_modifiers`)

`Tapped`, `TappedUnless(filter)`, `TappedOrPayLife(n)`, `ChooseSubtype`
(Roaming Throne, Reflections of Littjara, Cavern of Souls — answer stored
on `obj.chosen_subtype`; creatures also gain the subtype in their base).

### Triggers

`EntersBattlefield(filter)`, `LeavesBattlefield(filter)`, `Dies(filter)`,
`SpellCast(filter)`, `Draws(rel)`, `DrawsExceptFirst(rel)`,
`FirstNoncreatureSpellCast(rel)`, `Attacks(filter)`, `BecomesTarget`,
`EntersBattlefieldEvoked`, `StepBegin { step, whose }`.

### Target specs (`TargetSpec`)

`Object(filter)`, `Spell(filter)`, `StackOrBattlefield(filter)`,
`CardInGraveyard(filter, rel)`, `ThisObject`, `EventObject` (implicit —
the object the trigger was about), `AbilityOnStack(filter)`,
`SpellOrAbility(filter)` (Ertai), `Player(rel)`, `AnyPlayer`.

### Filters (composable data)

`Any`, `This`, `Another`, `And(&[..])`, `Or(&[..])`, `Not(&..)`,
`HasType`, `LacksType`, `HasSupertype`, `HasSubtype`, `HasColor`,
`IsColorless`, `Monocolored`, `IsToken`, `ControlledByYou`,
`ControlledByOpponent`, `OwnedByYou`, `Tapped`, `Untapped`, `Attacking`,
`HasKeyword`, `CmcAtMost`, `CmcAtLeast`, `MatchesChosenTypeOfSource`
(Roaming Throne & co.), `InZone(ZoneRef)` (incl. `NotBattlefield` for
cross-zone effects).

**Compose them inline.** In `static` context a slice promotes to `'static`
automatically, so `Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])`
needs no named `static` at all. Give a filter a name only when the same card
refers to it twice.

**Reach for the named ones first.** `Filter` carries constants for the
predicates the pool kept reinventing — `CREATURE`, `ARTIFACT`,
`ENCHANTMENT`, `LAND`, `PLANESWALKER`, `NONLAND`, `NONCREATURE`,
`BASIC_LAND`, `NONTOKEN_CREATURE`, `INSTANT_OR_SORCERY`,
`ARTIFACT_OR_ENCHANTMENT`, `ANOTHER_CREATURE`, `YOUR_CREATURE`,
`OPPONENT_CREATURE`. "A creature" had been written out as
`HasType(TypeSet::CREATURE)` in a differently-named `static` in twenty-six
card files, which is twenty-six chances to type `LacksType` by accident and
no way to grep for the one that did.

A filter that is about *this pool* rather than about Magic goes in
`crates/baylee-cards/src/filters.rs` (`YOUR_ALLIES`, `ANOTHER_ALLY`), beside
`crate::tokens`, which draws the same line. It earns a place there by being
written twice; one card's own compound filter stays in that card's file,
where the oracle sentence it encodes is a line above it.

### Effects (ops)

Life/draw: `GainLife`, `GainLifeFor`, `GainLifeDoubleX`, `LoseLife`,
`DrawCards`, `DrawCardsFor`, `Scry`, `ScryFor`, `Mill`,
`RearrangeTopLibrary`/`ReorderTopLibrary`.
Combat/damage: `DealDamage`, `DealDamageToTargetController`.
Removal: `Destroy`, `DestroyAll`, `Exile`, `CounterTargetSpell`,
`CounterTargetAbility`, `CounterTargetSpellOrAbility`,
`TargetSourceLosesAbilities`, `SacrificeFilter`, `ReturnToHand`,
`ReturnAllToHand`, `RedirectTarget` (Misdirection).
Zones: `SearchLibrary`, `OptionalBasicLandSearchFor`, `GraveyardToTop`,
`GraveyardToHand`, `GraveyardToBattlefield`, `ExileGraveyard`, `Blink`,
`ExileLinked`, `ReturnLinkedToBattlefield`, `PutFromHandOnTop`,
`PutSourceOnTopOfLibrary`, `ExileAndReturnAtEndStep` (Venser +2, Eerie
Interlude), `BottomCardFromHand`, `WishToHand` (Karn's −2: a card you own
from outside the game or face-up in your exile).
Continuous: `CreateContinuousEffect` (any layer+filter+modifier+duration),
`PumpFilter` (a filter, where `Filter::This` is the *source*), `PumpTarget`
(the spell's or ability's targets, all of them — Giant Growth), both of which
carry a `KeywordSet` so "+2/+2 and gains trample" is one effect,
`SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`ExchangeControlOrSacrifice` (Gilded Drake), `PhaseOut`, `AttachSelf`.
Tokens/copy: `CreateToken`, `CreateTokenN`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CreateTokenCopyOfFirstToken`, `CopyTargetSpell`, `Amass`.
Costs/taxes: `PlayerMayPayOr`, `AddCounter`, `AddCounterFilter`,
`DrainAllCountersIntoSelf` (Thief of Blood), `AddMana`,
`DelayedManaAtNextFirstMain` (Mana Drain), `SacrificeSelf`,
`PayCostOrLoseLater`, `ExileTargetsCreateTokens`.
Conditional: `IfEventPowerAtLeast` (Tribute to the World Tree).
Utility: `UntapTarget`, `NegXFixed` (amount), `CreateTokenCopyOfFirstToken`,
`BecomeMonarch`, `Sequence(&[..])`.
Modal/sequence: `Sequence(&[..])`.

### Modifiers (layer effects)

`AddType`, `RemoveType`, `AddSubtype`, `AllCreatureTypes`,
`AllBasicLandTypes`, `AddColor`, `SetColor`, `AddKeyword`, `RemoveKeyword`,
`LoseKeywords`, `ModifyPT`, `SetPT`, `SwitchPT`, `LegendRuleOff`,
`CantActivateArtifacts`, `OpponentsCastAsSorcery`, `PlayersCantLose`,
`CantLoseLife`, `PreventDamageToIt`, `PreventDamageFromIt`,
`OpponentsCantSearch`, `NoMaxHandSize`, `GainControl`.

`GainControl` is layer 2 and must be paired with `Layer::Control` — any
other layer applies it out of order with respect to the effects that read
the controller. With `Filter::This` and `Duration::UntilEndOfTurn` it is
Act of Treason; with `WhileSourceOnBattlefield` it is Mind Control. Use it
rather than `Effect::ChangeController` whenever the control comes back:
the one-shot effect never returns the permanent.

New `Modifier` variants must be added to THREE places: the
"handled elsewhere" arm in `layers.rs`, the modifier hash in
`state.rs`, and whatever system enforces them (SBAs, combat, casting).

## Worked examples

A land with two basic land types must print its own mana ability. CR 305.6
grants one ability *per* basic type, and the engine's intrinsic shortcut
(`casting::intrinsic_mana`) can only return a single colour with no way to
ask which — so it declines any land with more than one, and such a land taps
for nothing at all unless the card supplies the choice itself:

```rust
abilities: &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::White,
    ManaColor::Black,
])])],
```

`baylee-cards`'s `a_land_with_two_basic_types_prints_its_own_mana_ability`
turns that into a build failure. A land with exactly one basic type (a plain
Swamp) may rely on the shortcut, though every basic in the pool prints the
ability anyway.

Every mana line is one `Effect::AddMana`, which answers three independent
questions — which colors (`ManaSource`), how much (`Amount`), and what the
mana may be spent on (`ManaRestriction`). Card files say it through the
constructors, which read like the printed line:

```rust
Effect::mana(ManaColor::Green, 1)                      // Add {G}.
Effect::mana(ManaColor::Colorless, 2)                  // Add {C}{C}.
Effect::mana_choice(&[ManaColor::White, ManaColor::Black])  // Add {W} or {B}.
Effect::mana_of_any_color()                            // Add one mana of any color.
Effect::mana_combination(COLORS, Amount::Fixed(2))     // …in any combination of colors.
Effect::mana_commander_identity()                      // …in your commander's color identity.
Effect::mana_land_color(true)                          // …a land you control could produce.
Effect::mana_dynamic(ManaColor::Black, Amount::CountOf { .. })
Effect::mana_of_any_color().restricted(&FILTER, SpendRider::Uncounterable)
```

`mana_combination` is not decoration: "in any combination of colors" is one
color pick *per mana*, while a plain choice picks one color for the whole
amount. Harabaz Druid was written with the wrong one and paid X² mana.

Fetchland (activated with composite cost + filtered search):

```rust
abilities: &[activated!(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::SacrificeSelf, CostPart::PayLife(1)],
    },
    &[Effect::SearchLibrary {
        filter: &LAND_TYPE_PAIR,
        finds: &[Find::BATTLEFIELD_TAPPED],
        optional: false,
    }]
)],
```

Note what is *not* written: this ability uses the stack, is activatable at
instant speed and functions on the battlefield, and all three are what the
rules already say. Only the cost and the effect are the card.

A search says *what* it looks for and *where* each find goes; the two rules
every printed search also obeys are derived, not declared, so a card cannot
get them wrong:

- **The library is always shuffled afterwards.** Of the 1015 printed
  searches in the forge reference, three do not say "then shuffle", and all
  three empty the library instead.
- **A find is revealed** when the search is narrower than "a card"
  (`Filter::Any`) *and* at least one destination is hidden (hand, top of
  library). Mystical Tutor reveals; Demonic Tutor does not; a fetchland does
  not, because the battlefield is public anyway. The reveal is journalled as
  `GameEvent::Revealed` — it is what holds the searcher to the filter.

Rally trigger (filter "self or another Ally you control"):

```rust
use crate::filters::YOUR_ALLIES;

abilities: &[triggered!(
    Trigger::EntersBattlefield(&YOUR_ALLIES),
    &[Effect::GainLife { amount: Amount::Fixed(1) }]
)],
```

The filter is shared rather than restated: the rally wording is "this
creature **or another** Ally", so the source is part of the match, and six
card files had each written that out. A seventh writing `Another` instead
would have been a silent rules bug in a card that still compiled.

Static anthem via layers (deregisters itself when the source leaves):

```rust
abilities: &[AbilityDef::Static(StaticAbility {
    layer: Layer::PtModify,
    filter: Filter::YOUR_CREATURE,
    modifier: Modifier::ModifyPT(1, 1),
    cross_zone: false,
})],
```

## Explicitly not supported yet (M3+)

Landed since the freeze (no longer blockers): MDFC face casting, miracle,
delve, convoke, flashback grants, protection (damage/target/block),
until-EOT layer-1 copies, extra turns, lifelink counters, search locks,
no-max-hand-size, damage prevention, choose-a-type, ward, monarch,
spell-copy target re-choice, sideboard / outside-the-game access.

- Multiplayer player-choice for targeted triggers (protocol M3) — use
  `PlayerRel::Opponent` (heads-up auto-resolve) + `Partial` note for MP.
- Sagas (Urza's Saga chapters, The True Scriptures): lore counters,
  chapter triggers, granted abilities, sacrifice after the last chapter.
- Disturb (Mirrorhall Mimic's back): graveyard face-casting.
- Activation conditions (Mox Opal metalcraft, Bleachbone Verge) — abilities
  currently activate unconditionally (`Partial` note).
- Mana-source tracking / restricted mana riders (Cavern of Souls
  uncounterable, Path of Ancestry scry) — pool mana has no provenance.
- Search takeover (Opposition Agent's real hijack; approximated as a lock).
- Tap events (City of Brass's becomes-tapped trigger).
- Comparative conditions (Padeem's greatest-cmc upkeep).
- Ability-granting statics (Chromatic Lantern's land grant; also blocks
  Urza's Saga ch. I/II).
- Emblems with triggered abilities (Venser −8) — engine supports emblem
  objects; trigger scan for command zone is pending.
- Cost reducers (Surgical Metamorph's not-starting-player {1} less).
- Permanent-spell copies resolving as tokens (Reflections of Littjara
  rider).
- Player hexproof (Everybody Lives! rider).
- Day/night, dungeons, initiative, battles, classes (Wizard Class levels),
  stickers/attractions, subgames, ante.

When you hit one of these: implement everything expressible, then
`Coverage::Partial("…")` + `// NOT SUPPORTED:` on the specific line.
