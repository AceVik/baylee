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
```

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
pub static CARD: CardDef = CardDef {
    index: CardIndex::new(104),
    oracle_id: "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id: "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces: &[FaceDef {
        name: "Ondu Cleric",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::CLERIC, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[/* … */],
    ..CardDef::DEFAULT
};
```

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
- `CardDef::DEFAULT.coverage` is `Coverage::Unimplemented`, so a stub that
  was never finished cannot reach the deckbuilder as playable just because
  a line went missing. An implemented card writes
  `coverage: Coverage::Implemented` by hand.

`FaceDef::DEFAULT.castable_from_hand` is `true`; only disturb and adventure
backs opt out.

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
`PumpFilter`, `SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`ExchangeControlOrSacrifice` (Gilded Drake), `PhaseOut`, `AttachSelf`.
Tokens/copy: `CreateToken`, `CreateTokenN`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CreateTokenCopyOfFirstToken`, `CopyTargetSpell`, `Amass`.
Costs/taxes: `PlayerMayPayOr`, `AddCounter`, `AddCounterFilter`,
`DrainAllCountersIntoSelf` (Thief of Blood), `AddMana`, `AddManaChoice`
(Amount-driven), `AddManaDynamic`, `AddManaCommanderIdentity` (Command
Tower), `AddManaLandColor` (Exotic Orchard/Reflecting Pool),
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

Fetchland (activated with composite cost + filtered search):

```rust
abilities: &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::SacrificeSelf, CostPart::PayLife(1)],
    },
    effects: &[Effect::SearchLibrary {
        filter: &LAND_TYPE_PAIR,
        dest: SearchDest::Battlefield,
        tapped: true,
        shuffle: true,
        optional: false,
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
}],
```

Rally trigger (filter "self or another Ally you control"):

```rust
static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
]);
abilities: &[AbilityDef::Triggered {
    trigger: Trigger::EntersBattlefield(&ALLY_ETB),
    effects: &[Effect::GainLife { amount: Amount::Fixed(1) }],
    targets: None,
    once_per_turn: false,
}],
```

Static anthem via layers (deregisters itself when the source leaves):

```rust
abilities: &[AbilityDef::Static(StaticAbility {
    layer: Layer::PtModify,
    filter: Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]),
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
