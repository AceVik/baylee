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
   stub untouched. Only edit `coverage`, `keywords`, `abilities`.
3. Declare layers + durations explicitly for continuous effects. The engine
   deregisters effects structurally — never hand-roll removal.
4. Every card ships with `#[cfg(test)] mod tests` (resolution, targeting,
   edge cases, one interaction test per non-trivial mechanic).

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
- `AbilityDef::Suspend { counters }`

### Triggers

`EntersBattlefield(filter)`, `LeavesBattlefield(filter)`, `Dies(filter)`,
`SpellCast(filter)`, `Draws(rel)`, `DrawsExceptFirst(rel)`,
`FirstNoncreatureSpellCast(rel)`, `Attacks(filter)`, `BecomesTarget`,
`EntersBattlefieldEvoked`, `StepBegin { step, whose }`.

### Filters (composable data)

`Any`, `This`, `Another`, `And(&[..])`, `Or(&[..])`, `Not(&..)`,
`HasType`, `LacksType`, `HasSupertype`, `HasSubtype`, `HasColor`,
`IsColorless`, `ControlledByYou`, `ControlledByOpponent`, `OwnedByYou`,
`Tapped`, `Untapped`, `HasKeyword`, `CmcAtMost`, `CmcAtLeast`,
`InZone(ZoneRef)` (incl. `NotBattlefield` for cross-zone effects).

### Effects (ops)

Life/draw: `GainLife`, `GainLifeFor`, `LoseLife`, `DrawCards`, `DrawCardsFor`,
`Scry`, `ScryFor`, `Mill`, `RearrangeTopLibrary`/`ReorderTopLibrary`.
Combat/damage: `DealDamage`, `DealDamageToTargetController`.
Removal: `Destroy`, `DestroyAll`, `Exile`, `CounterTargetSpell`,
`ReturnToHand`, `ReturnAllToHand`.
Zones: `SearchLibrary`, `OptionalBasicLandSearchFor`, `GraveyardToTop`,
`GraveyardToHand`, `GraveyardToBattlefield`, `ExileGraveyard`, `Blink`,
`ExileLinked`, `ReturnLinkedToBattlefield`, `PutFromHandOnTop`,
`PutSourceOnTopOfLibrary`.
Continuous: `CreateContinuousEffect` (any layer+filter+modifier+duration),
`PumpFilter`, `SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`PhaseOut`, `AttachSelf`.
Tokens/copy: `CreateToken`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CopyTargetSpell`, `Amass`.
Costs/taxes: `PlayerMayPayOr`, `AddCounter`, `AddCounterFilter`,
`AddMana`, `AddManaChoice` (Amount-driven), `AddManaDynamic`,
`SacrificeSelf`, `PayCostOrLoseLater`, `ExileTargetsCreateTokens`.
Modal/sequence: `Sequence(&[..])`.

### Modifiers (layer effects)

`AddType`, `RemoveType`, `AddSubtype`, `AllCreatureTypes`,
`AllBasicLandTypes`, `AddColor`, `SetColor`, `AddKeyword`, `RemoveKeyword`,
`LoseKeywords`, `ModifyPT`, `SetPT`, `SwitchPT`, `LegendRuleOff`,
`CantActivateArtifacts`, `OpponentsCastAsSorcery`.

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

- Multiplayer player-choice for targeted triggers (protocol M3) — use
  `PlayerRel::Opponent` (heads-up auto-resolve) + `Partial` note for MP.
- Target re-choice for spell copies (protocol M3).
- Sideboard / outside-the-game access (Karn's wish, companion) — gateway M4.
- MDFC face choice at cast (pathways, Glasspool Mimic) — M2.S8+.
- Emblems with triggered abilities (Venser −8) — engine supports emblem
  objects; trigger scan for command zone is pending.
- Protection from colors (Mother of Runes style).
- Day/night, dungeons, initiative, battles, sagas (Urza's Saga), classes
  (Wizard Class), stickers/attractions, subgames, ante.

When you hit one of these: implement everything expressible, then
`Coverage::Partial("…")` + `// NOT SUPPORTED:` on the specific line.
