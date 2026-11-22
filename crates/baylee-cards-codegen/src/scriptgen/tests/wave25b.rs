//! Wave 25b tests: Wave 12/13c `TargetConstraint` (`SameController` on
//! triggers, `OnePerPlayer` on spells/abilities/triggers) and Wave 13d
//! `FaceDef::optional_cost` (`S: Mode$ OptionalCost`),
//! `Condition::OptionalCostPaid`, `Amount::IfCondition`, `CostPart::Behold`,
//! and `CostPart::CollectEvidence`.

use super::*;

fn dragon_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.push("Dragon".into());
    c.creature.push("Elf".into());
    c.creature.push("Hero".into());
    c.artifact.push("Treasure".into());
    c.normalize();
    c
}

fn read_dragon(text: &str) -> CardBody {
    let c = dragon_cats();
    let parsed = parse(text);
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

fn refused_dragon(text: &str) -> bool {
    let c = dragon_cats();
    transcode(&parse(text), &c, None).is_none()
}

// -----------------------------------------------------------------------------
// Scope 1: Triggered `TargetConstraint::SameController` & `OnePerPlayer`
// -----------------------------------------------------------------------------

#[test]
fn triggered_targets_with_same_controller_all_shapes() {
    // Arashin Sunshield / Griffnaut Tracker shape: ETB exile up to 2 cards
    // from a single graveyard.
    let sunshield = read(
        "Name:Arashin Sunshield\nManaCost:3 W\nTypes:Creature Human Warrior\nPT:3/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ True | ValidTgts$ Card\n\
         A:AB$ Tap | Cost$ W T | ValidTgts$ Creature",
    );
    assert!(
        sunshield.abilities[0].contains(
            "TargetReq::up_to(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer), 2)\
             .with_constraint(TargetConstraint::SameController)"
        ),
        "{:?}",
        sunshield.abilities
    );

    // Griffnaut Tracker shape: Flying + ETB exile up to 2 cards from a single graveyard.
    let tracker = read(
        "Name:Griffnaut Tracker\nManaCost:3 W\nTypes:Creature Human Detective\nPT:3/2\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ True | ValidTgts$ Card",
    );
    assert!(
        tracker.abilities[0].contains(".with_constraint(TargetConstraint::SameController)"),
        "{:?}",
        tracker.abilities
    );

    // Soul-Shackled Zombie shape: ETB exile up to 2 cards from a single graveyard,
    // remembering changed cards for follow-up conditional life drain.
    let zombie = read(
        "Name:Soul-Shackled Zombie\nManaCost:3 B\nTypes:Creature Zombie\nPT:4/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ True | RememberChanged$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        zombie.abilities[0].contains(".with_constraint(TargetConstraint::SameController)"),
        "{:?}",
        zombie.abilities
    );

    // Exact-count triggered `TargetsWithSameController$ True` (target 2 sorceries).
    let exact = read(
        "Name:GravePair\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetsWithSameController$ True | ValidTgts$ Card.Sorcery | TargetMin$ 2 | TargetMax$ 2",
    );
    assert!(
        exact.abilities[0].contains(
            "TargetReq::exactly(TargetSpec::CardInGraveyard(&Filter::SORCERY, PlayerRel::EachPlayer), 2)\
             .with_constraint(TargetConstraint::SameController)"
        ),
        "{:?}",
        exact.abilities
    );

    // Refusals on triggered `TargetsWithSameController$`: single target, player
    // target, invalid value, missing ValidTgts$.
    for bad in [
        "Name:TrigSingle\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetMin$ 1 | TargetMax$ 1 | TargetsWithSameController$ True",
        "Name:TrigPlayer\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ LoseLife | ValidTgts$ Player | TargetMin$ 1 | TargetMax$ 2 | LifeAmount$ 1 | TargetsWithSameController$ True",
        "Name:TrigBadVal\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ Maybe",
        "Name:TrigNoTgt\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ Draw | Defined$ You | NumCards$ 1 | TargetsWithSameController$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn one_per_player_targets_battlefield_graveyard_mandatory_and_optional() {
    // Optional all-players battlefield (Nils / Battle at the Helvault shape).
    let opt_all = read(
        "Name:CounterEach\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Put\n\
         SVar:Put:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1 | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
    );
    assert!(
        opt_all.abilities[0].contains(
            "TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))\
             .with_constraint(TargetConstraint::OnePerPlayer)"
        ),
        "{:?}",
        opt_all.abilities
    );

    // Mandatory all-players battlefield (`TargetMin$ OneEach | TargetMax$ OneEach`).
    let mand_all = read(
        "Name:DestroyEach\nManaCost:3 B R\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Permanent.nonLand | TargetMin$ OneEach | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
    );
    assert!(
        mand_all.abilities[0].contains(
            "TargetReq::one(TargetSpec::Object(&Filter::NONLAND))\
             .with_constraint(TargetConstraint::OnePerPlayer)"
        ),
        "{:?}",
        mand_all.abilities
    );

    // Optional per-opponent graveyard (Sepulchral Primordial shape).
    let gy_opp = read(
        "Name:SepulchralEcho\nManaCost:5 B B\nTypes:Creature Avatar\nPT:5/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | ValidTgts$ Creature.OppCtrl | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        gy_opp.abilities[0].contains(
            "TargetReq::up_to_one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::Opponent))\
             .with_constraint(TargetConstraint::OnePerPlayer)"
        ),
        "{:?}",
        gy_opp.abilities
    );

    // Optional per-player graveyard (`PlayerCountPlayers$Amount`).
    let gy_all = read(
        "Name:AllGraveExile\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
    );
    assert!(
        gy_all.abilities[0].contains(
            "TargetReq::up_to_one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer))\
             .with_constraint(TargetConstraint::OnePerPlayer)"
        ),
        "{:?}",
        gy_all.abilities
    );

    // Each other player (`PlayerCountOther$Amount` + `YouDontCtrl`).
    let each_other = read(
        "Name:ExileOtherEach\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.YouDontCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOther$Amount",
    );
    assert!(
        each_other.abilities[0].contains(".with_constraint(TargetConstraint::OnePerPlayer)")
            && !each_other.abilities[0].contains("ObjectOfEachOpponent"),
        "{:?}",
        each_other.abilities
    );

    // Mandatory per-opponent multi-alternative (`Filter::Or`) wraps with top-level
    // `Filter::And(&[..., Filter::ControlledByOpponent])` so engine `filter_player_scope`
    // scopes seats strictly to opponents.
    let mand_opp_or = read(
        "Name:MandOppDisenchant\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Artifact.OppCtrl,Enchantment.OppCtrl | TargetMin$ OneEach | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        mand_opp_or.abilities[0].contains(".with_constraint(TargetConstraint::OnePerPlayer)")
            && mand_opp_or.statics.contains(
                "Filter::And(&[Filter::ARTIFACT_OR_ENCHANTMENT, Filter::ControlledByOpponent])"
            ),
        "{}\n{:?}",
        mand_opp_or.statics,
        mand_opp_or.abilities
    );
}

#[test]
fn one_per_player_multi_target_gain_control_animate_and_refusals() {
    // Multi-target GainControl (`Blatant Thievery` mandatory OnePerPlayer &
    // `Mass Mutiny` per-opponent) lowers to `Effect::ModifyTargets`.
    let blatant = read(
        "Name:Blatant Thievery\nManaCost:4 U U U\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Permanent.OppCtrl | TargetMin$ OneEach | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        blatant.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::GainControl, duration: Duration::Indefinitely }"
        ) && blatant
            .abilities[0]
            .contains(".with_constraint(TargetConstraint::OnePerPlayer)"),
        "{:?}",
        blatant.abilities
    );

    let mass_mutiny = read(
        "Name:Mass Mutiny\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | LoseControl$ EOT | Untap$ True | AddKWs$ Haste\n\
         SVar:OneEach:PlayerCountOpponents$Amount\n\
         Oracle:For each opponent, gain control of up to one target creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn.",
    );
    assert!(
        mass_mutiny.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::GainControl, duration: Duration::UntilEndOfTurn }"
        ) && mass_mutiny.abilities[0].contains("Effect::UntapTarget")
            && mass_mutiny
                .abilities[0]
                .contains("TargetSpec::ObjectOfEachOpponent(&Filter::CREATURE)"),
        "{:?}",
        mass_mutiny.abilities
    );

    // Kitesail Larcenist shape: multi-target `OnePerPlayer` `Animate` rewrites
    // `Effect::continuous(&Filter::This, ...)` to `Effect::ModifyTargets`.
    let larcenist = read_dragon(
        "Name:Kitesail Larcenist\nManaCost:2 U\nTypes:Creature Human Pirate\nPT:2/3\n\
         K:Flying\nK:Ward:1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ TrigAnimate\n\
         SVar:TrigAnimate:DB$ Animate | RemoveAllAbilities$ True | Types$ Artifact,Treasure | Duration$ AsLongAsInPlay | Abilities$ TreasureSac | RemoveCardTypes$ True | ValidTgts$ Artifact.Other,Creature.Other | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount\n\
         SVar:TreasureSac:AB$ Mana | Cost$ T Sac<1/CARDNAME/this artifact> | Produced$ Any",
    );
    assert!(
        larcenist.abilities[1].contains("Effect::ModifyTargets {")
            && !larcenist.abilities[1].contains("Effect::continuous(&Filter::This")
            && larcenist.abilities[1].contains(".with_constraint(TargetConstraint::OnePerPlayer)"),
        "{:?}",
        larcenist.abilities
    );

    // Refusals: scope mismatch (OppCtrl with PlayerCountPlayers), conflicting
    // constraints (SameController + OnePerPlayer), unsupported multi-target API.
    for bad in [
        "Name:ScopeMismatch\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
        "Name:Conflict\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | TargetsWithSameController$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
        "Name:BadFightMulti\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Fight | ValidTgts$ Creature | TargetMin$ OneEach | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountPlayers$Amount",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

// -----------------------------------------------------------------------------
// Scope 2 & 3: `FaceDef::optional_cost`, `Condition::OptionalCostPaid`,
//              `Amount::IfCondition`, `CostPart::Behold`, `CostPart::CollectEvidence`
// -----------------------------------------------------------------------------

#[test]
fn optional_cost_voltage_surge_piercing_exhale_dispelling_exhale_and_osseous_exhale() {
    // Voltage Surge: Sac<1/Artifact> + Count$OptionalGenericCostPaid.4.2.
    let surge = read(
        "Name:Voltage Surge\nManaCost:R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Sac<1/Artifact>\n\
         A:SP$ DealDamage | NumDmg$ X | ValidTgts$ Creature,Planeswalker\n\
         SVar:X:Count$OptionalGenericCostPaid.4.2",
    );
    assert_eq!(
        surge.optional_cost.as_deref(),
        Some("Some(cost!(Sacrifice(&Filter::YOUR_ARTIFACT)))")
    );
    assert!(
        surge.abilities[0].contains("Effect::IfCondition { condition: Condition::OptionalCostPaid")
            && surge.abilities[0].contains("Amount::Fixed(4)")
            && surge.abilities[0].contains("Amount::Fixed(2)"),
        "{:?}",
        surge.abilities
    );

    // Dispelling Exhale: Behold<1/Dragon> + UnlessCost$ X -> Amount::IfCondition.
    let dispelling = read_dragon(
        "Name:Dispelling Exhale\nManaCost:1 U\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Dragon>\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ X\n\
         SVar:X:Count$OptionalGenericCostPaid.4.2",
    );
    assert_eq!(
        dispelling.optional_cost.as_deref(),
        Some("Some(cost!(Behold(&COST1)))")
    );
    assert!(
        dispelling
            .statics
            .contains("static COST1: Filter = Filter::HasSubtype("),
        "optional_cost static must survive prune_unused_statics: {}",
        dispelling.statics
    );
    assert!(
        dispelling.abilities[0].contains(
            "Amount::IfCondition { condition: &Condition::OptionalCostPaid, then: &Amount::Fixed(4), otherwise: &Amount::Fixed(2) }"
        ),
        "{:?}",
        dispelling.abilities
    );

    // Piercing Exhale: Behold<1/Dragon> + two-target bite + Condition$ OptionalCost surveil.
    let piercing = read_dragon(
        "Name:Piercing Exhale\nManaCost:1 G\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Dragon>\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ SoulsDamage\n\
         SVar:SoulsDamage:DB$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X | DamageSource$ ParentTarget | SubAbility$ DBSurveil\n\
         SVar:DBSurveil:DB$ Surveil | Condition$ OptionalCost | ConditionOptionalPaid$ True | Amount$ 2\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert_eq!(
        piercing.optional_cost.as_deref(),
        Some("Some(cost!(Behold(&COST1)))")
    );
    assert!(
        piercing.abilities[0].contains(
            "Effect::IfCondition { condition: Condition::OptionalCostPaid, then: &[Effect::surveil(2)], otherwise: &[] }"
        ),
        "{:?}",
        piercing.abilities
    );
}

#[test]
fn optional_cost_hyperion_osseous_and_undergrowth() {
    // Hyperion's Atomic Vision: Behold<1/Hero> + Condition$ OptionalCost scry 2.
    let hyperion = read_dragon(
        "Name:Hyperion's Atomic Vision\nManaCost:1 W\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Hero>\n\
         A:SP$ Destroy | ValidTgts$ Creature.tapped | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 2 | Condition$ OptionalCost | ConditionOptionalPaid$ True",
    );
    assert_eq!(
        hyperion.optional_cost.as_deref(),
        Some("Some(cost!(Behold(&COST1)))")
    );
    assert!(
        hyperion.abilities[0].contains("Condition::OptionalCostPaid")
            && hyperion.abilities[0].contains("Effect::scry(2)"),
        "{:?}",
        hyperion.abilities
    );

    // Osseous Exhale: Behold<1/Dragon> + Condition$ OptionalCost gain life.
    let osseous = read_dragon(
        "Name:Osseous Exhale\nManaCost:1 W\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Dragon>\n\
         A:SP$ DealDamage | ValidTgts$ Creature.attacking,Creature.blocking | NumDmg$ 5 | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Condition$ OptionalCost | ConditionOptionalPaid$ True | Defined$ You | LifeAmount$ 2",
    );
    assert_eq!(
        osseous.optional_cost.as_deref(),
        Some("Some(cost!(Behold(&COST1)))")
    );
    assert!(
        osseous.abilities[0].contains("Condition::OptionalCostPaid")
            && osseous.abilities[0].contains("Effect::gain_life(2)"),
        "{:?}",
        osseous.abilities
    );

    // Undergrowth: mana optional cost `{2}{R}` + `ConditionOptionalPaid$ False` / `True` merged.
    let undergrowth = read(
        "Name:Undergrowth\nManaCost:G\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 2 R\n\
         A:SP$ Fog | Condition$ OptionalCost | ConditionOptionalPaid$ False | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Condition$ OptionalCost | ConditionOptionalPaid$ True | StaticAbilities$ KWPump\n\
         SVar:KWPump:Mode$ Continuous | AffectedZone$ Battlefield | Affected$ Creature.nonRed | AddKeyword$ Prevent all combat damage that would be dealt by CARDNAME.",
    );
    assert_eq!(
        undergrowth.optional_cost.as_deref(),
        Some("Some(cost!(\"{2}{R}\"))")
    );
    assert!(
        undergrowth.abilities[0]
            .contains("Effect::IfCondition { condition: Condition::OptionalCostPaid, then: &[")
            && undergrowth.abilities[0]
                .contains("otherwise: &[Effect::PreventAllCombatDamageThisTurn]"),
        "{:?}",
        undergrowth.abilities
    );
}

#[test]
fn optional_cost_collect_evidence_inspector_sprite_pollen_and_mask() {
    // Vitu-Ghazi Inspector: CollectEvidence<6> + ETB intervening-if CastSA>Collected$Amount.
    let inspector = read(
        "Name:Vitu-Ghazi Inspector\nManaCost:1 G\nTypes:Creature Elf Detective\nPT:1/3\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<6>\n\
         K:Reach\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | CheckSVar$ Z | ValidCard$ Card.Self | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 2\n\
         SVar:Z:CastSA>Collected$Amount",
    );
    assert_eq!(
        inspector.optional_cost.as_deref(),
        Some("Some(cost!(CollectEvidence(6)))")
    );
    assert!(
        inspector.abilities[0].contains("condition = Some(Condition::OptionalCostPaid)"),
        "{:?}",
        inspector.abilities
    );

    // Crimestopper Sprite: CollectEvidence<6> + ETB sub-ability ConditionDefined$ CastSA>Collected.
    let sprite = read(
        "Name:Crimestopper Sprite\nManaCost:2 U\nTypes:Creature Faerie Detective\nPT:2/2\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<6>\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | ValidTgts$ Creature | SubAbility$ DBStun\n\
         SVar:DBStun:DB$ PutCounter | Defined$ Targeted | CounterType$ STUN | CounterNum$ 1 | ConditionDefined$ CastSA>Collected | ConditionPresent$ Card",
    );
    assert_eq!(
        sprite.optional_cost.as_deref(),
        Some("Some(cost!(CollectEvidence(6)))")
    );
    assert!(
        sprite.abilities[0].contains(
            "Effect::IfCondition { condition: Condition::OptionalCostPaid, then: &[Effect::AddCounter { kind: CounterKind::Stun, amount: Amount::Fixed(1) }], otherwise: &[] }"
        ),
        "{:?}",
        sprite.abilities
    );

    // Analyze the Pollen: CollectEvidence<8> + complementary ConditionDefined$ Collected arms merged.
    let pollen = read(
        "Name:Analyze the Pollen\nManaCost:G\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<8>\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ConditionDefined$ Collected | ConditionPresent$ Card | ConditionCompare$ EQ0 | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land,Creature | ChangeNum$ 1 | ConditionDefined$ Collected | ConditionPresent$ Card",
    );
    assert_eq!(
        pollen.optional_cost.as_deref(),
        Some("Some(cost!(CollectEvidence(8)))")
    );
    assert!(
        pollen.abilities[0].contains("Effect::IfCondition { condition: Condition::OptionalCostPaid, then: &[Effect::SearchLibrary")
            && pollen.abilities[0].contains("], otherwise: &[Effect::SearchLibrary"),
        "{:?}",
        pollen.abilities
    );

    // Behind the Mask: CollectEvidence<6> + Count$Compare Z GE1.<A>.<B> where Z = Collected$Amount.
    let mask = read(
        "Name:Behind the Mask\nManaCost:U\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<6>\n\
         A:SP$ Animate | ValidTgts$ Artifact,Creature | Power$ X | Toughness$ Y | Types$ Artifact,Creature\n\
         SVar:X:Count$Compare Z GE1.1.4\n\
         SVar:Y:Count$Compare Z GE1.1.3\n\
         SVar:Z:Collected$Amount",
    );
    assert_eq!(
        mask.optional_cost.as_deref(),
        Some("Some(cost!(CollectEvidence(6)))")
    );
    assert!(
        mask.abilities[0].contains("Effect::IfCondition { condition: Condition::OptionalCostPaid")
            && mask.abilities[0].contains("Modifier::SetPT(1, 1)")
            && mask.abilities[0].contains("Modifier::SetPT(4, 3)"),
        "{:?}",
        mask.abilities
    );
}

#[test]
fn behold_and_collect_evidence_cost_parts_in_activated_spell_kicker_and_unless() {
    // Activated ability with CollectEvidence<3> (Forensic Researcher shape).
    let researcher = read(
        "Name:Forensic Researcher\nManaCost:2 U\nTypes:Creature Merfolk Detective\nPT:1/3\n\
         A:AB$ Tap | Cost$ T CollectEvidence<3> | ValidTgts$ Creature.YouDontCtrl",
    );
    assert!(
        researcher.abilities[0].contains("cost!(TapSelf, CollectEvidence(3))"),
        "{:?}",
        researcher.abilities
    );

    // Mandatory spell additional cost with Behold<1/Dragon> and CollectEvidence<4>.
    let sp_behold = read_dragon(
        "Name:DragonTax\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Draw | Cost$ 1 R Behold<1/Dragon> | NumCards$ 2",
    );
    assert_eq!(
        sp_behold.mandatory_additional_costs,
        vec!["CostPart::Behold(&COST1)"]
    );

    let sp_evidence = read(
        "Name:EvidenceTax\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 U CollectEvidence<4> | NumCards$ 2",
    );
    assert_eq!(
        sp_evidence.mandatory_additional_costs,
        vec!["CostPart::CollectEvidence(4)"]
    );

    // Kicker with CollectEvidence<3> / Behold<1/Dragon>.
    let kicked_ev = read(
        "Name:KickedEvidence\nManaCost:1 B\nTypes:Sorcery\n\
         K:Kicker:CollectEvidence<3>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        kicked_ev.additional_costs,
        vec!["cost!(CollectEvidence(3))"]
    );

    // Elven Passage: search library tapped, then optional Behold<1/Elf> to untap found land.
    let elven = read_dragon(
        "Name:Elven Passage\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T PayLife<1> Sac<1/CARDNAME/this land> | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land | RememberChanged$ True | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Remembered | UnlessCost$ Behold<1/Elf> | UnlessPayer$ You | UnlessSwitched$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        elven.abilities[0]
            .contains("Effect::MayBehold { filter: &COST2, then: &[Effect::UntapFound] }"),
        "{:?}",
        elven.abilities
    );

    // Optional trigger price with Behold<1/Dragon> (Sarkhan, Dragon Ascendant ETB shape).
    let sarkhan_etb = read_dragon(
        "Name:DragonCaller\nManaCost:1 R\nTypes:Creature Human Druid\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ Behold<1/Dragon> | NumCards$ 1",
    );
    assert!(
        sarkhan_etb.abilities[0].contains("CostPart::Behold(&COST1)"),
        "{:?}",
        sarkhan_etb.abilities
    );

    // Token scripts with `S:Mode$ OptionalCost` stay refused in tokengen.
    assert!(
        crate::tokengen::read(
            "Name:Opt Token\nColors:red\nTypes:Creature Goblin\nPT:1/1\n\
             S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 1",
            &dragon_cats()
        )
        .is_none(),
        "tokengen must refuse OptionalCost"
    );
}

#[test]
fn optional_cost_behold_and_collect_evidence_refusals() {
    for bad in [
        // Defiler global OptionalCost with ReduceColor$ / Activator$ / non-Self ValidCard$.
        "Name:DefilerOfVigor\nManaCost:3 G G\nTypes:Creature\nPT:6/6\n\
         S:Mode$ OptionalCost | ValidCard$ Permanent.Green | ValidSA$ Spell | Activator$ You | Cost$ PayLife<2> | ReduceColor$ G",
        // Duplicate S:Mode$ OptionalCost.
        "Name:DupOpt\nManaCost:R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 1\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 2\n\
         A:SP$ Draw | NumCards$ 1",
        // Unknown parameter on S:Mode$ OptionalCost (Condition$).
        "Name:CondOpt\nManaCost:R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 1 | Condition$ Threshold\n\
         A:SP$ Draw | NumCards$ 1",
        // Land with S:Mode$ OptionalCost.
        "Name:LandOpt\nTypes:Land\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 1",
        // Optional-cost-only target (Hulk's Thunderclap 3rd target shape) must be refused!
        "Name:OptOnlyTgt\nManaCost:1 G\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Dragon>\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Artifact | Condition$ OptionalCost | ConditionOptionalPaid$ True",
        // Condition$ OptionalCost without S:Mode$ OptionalCost.
        "Name:MissingOpt\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ 2 | Condition$ OptionalCost | ConditionOptionalPaid$ True",
        // Behold count > 1.
        "Name:BeholdTwo\nManaCost:2 R\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<2/Dragon>\n\
         A:SP$ Draw | NumCards$ 1",
        // CollectEvidence<X> and CollectEvidence<0>.
        "Name:CollectX\nManaCost:2 B G\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ CollectEvidence<X> 2 B G | NumCards$ 1\nSVar:X:Count$xPaid",
        "Name:CollectZero\nManaCost:1 U\nTypes:Creature\nPT:1/1\n\
         A:AB$ Draw | Cost$ T CollectEvidence<0> | NumCards$ 1",
        // Alchemy Conjure / Perpetual with OptionalCost must remain strictly refused.
        "Name:Spotlight Falcon\nManaCost:2 W W\nTypes:Artifact Creature Bird\nPT:4/3\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<5>\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | CheckSVar$ X | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigConjure\n\
         SVar:TrigConjure:DB$ MakeCard | Conjure$ True | Name$ Spotlight Falcon | Zone$ Hand\n\
         SVar:X:CastSA>Collected$Amount",
        "Name:Territorial Strike\nManaCost:1 B G\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Behold<1/Creature.Dragon/Dragon creature>\n\
         A:SP$ Destroy | ValidTgts$ Permanent.nonLand | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ RevealedCards.!Self | NumAtt$ +2 | NumDef$ +2 | PumpZone$ Battlefield,Hand | Duration$ Perpetual",
    ] {
        assert!(refused_dragon(bad), "must refuse: {bad}");
    }
}
