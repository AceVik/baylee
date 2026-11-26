use super::*;

#[test]
fn wave30b_barbflare_gremlin() {
    let gremlin = read(
        "Name:Barbflare Gremlin\nManaCost:3 R\nTypes:Creature Gremlin\nPT:3/2\nK:First Strike\nK:Haste\n\
         T:Mode$ TapsForMana | ValidCard$ Land | PresentDefined$ Self | IsPresent$ Card.tapped | Execute$ TrigMana | TriggerZones$ Battlefield | Static$ True\n\
         SVar:TrigMana:DB$ ManaReflected | ColorOrType$ Type | ReflectProperty$ Produced | Defined$ TriggeredActivator | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | Defined$ TriggeredActivator | DamageSource$ TriggeredCard | NumDmg$ 1",
    );
    assert!(
        gremlin.abilities[0].contains("Effect::EventObjectDealsDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::ControllerOfEvent) }"),
        "{:?}",
        gremlin.abilities
    );
}

#[test]
fn wave30b_spinning_wheel_kick() {
    let kick = read(
        "Name:Spinning Wheel Kick\nManaCost:X X G G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Creature,Planeswalker | TargetMin$ X | TargetMax$ X | DamageSource$ ParentTarget | NumDmg$ Y\n\
         SVar:X:Count$xPaid\nSVar:Y:ParentTargeted$CardPower",
    );
    assert!(
        kick.abilities[0].contains("second_targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER)))")
            && kick.abilities[0].contains("Effect::ForEachTarget { effects: &[Effect::WithSecondTarget(&[Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER) }])] }"),
        "{:?}",
        kick.abilities
    );
}

#[test]
fn wave30b_halana_kessig_ranger() {
    let halana = read(
        "Name:Halana, Kessig Ranger\nManaCost:3 G\nTypes:Legendary Creature Human Archer Ranger\nPT:3/4\nK:Reach\nK:Partner\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.Other+YouCtrl | Execute$ TrigPayCost | TriggerZones$ Battlefield\n\
         SVar:TrigPayCost:AB$ ImmediateTrigger | Cost$ 2 | Execute$ TrigDealDamage\n\
         SVar:TrigDealDamage:DB$ DealDamage | DamageSource$ Spawner>TriggeredCardLKICopy | NumDmg$ X | ValidTgts$ Creature\n\
         SVar:X:Spawner>TriggeredCard$CardPower",
    );
    assert!(
        halana.abilities[0].contains("remember: TriggerRemember::EventObject")
            && halana.abilities[0].contains("Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::Object(&Filter::CREATURE) }"),
        "{:?}",
        halana.abilities
    );
}

#[test]
fn wave30b_ziatora_the_incinerator() {
    let ziatora = read(
        "Name:Ziatora, the Incinerator\nManaCost:3 B R G\nTypes:Legendary Creature Demon Dragon\nPT:6/6\nK:Flying\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | Execute$ DBTrigger | TriggerZones$ Battlefield\n\
         SVar:DBTrigger:AB$ ImmediateTrigger | Cost$ Sac<1/Creature.StrictlyOther/another creature> | Execute$ DBDealDmg | RememberObjects$ Sacrificed\n\
         SVar:DBDealDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ XPower\n\
         SVar:XPower:TriggerRemembered$CardPower",
    );
    assert!(
        ziatora.abilities[0].contains("Effect::PlayerMayPayCostThen")
            && ziatora.abilities[0].contains("remember: TriggerRemember::SacrificedObject")
            && ziatora.abilities[0].contains("Amount::EventPower"),
        "{:?}",
        ziatora.abilities
    );
}

fn read_w30b(text: &str) -> CardBody {
    let mut c = cats();
    c.creature.push("Giant".into());
    c.creature.push("Elemental".into());
    c.enchantment.push("Aura".into());
    c.normalize();
    let parsed = parse(text);
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

#[test]
fn wave30b_surtland_flinger() {
    let flinger = read_w30b(
        "Name:Surtland Flinger\nManaCost:3 R R\nTypes:Creature Giant Berserker\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigImmediate\n\
         SVar:TrigImmediate:AB$ ImmediateTrigger | Cost$ Sac<1/Creature.Other/another creature> | RememberObjects$ Sacrificed | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | ConditionDefined$ DelayTriggerRememberedLKI | ConditionPresent$ Giant | ConditionCompare$ EQ0 | ValidTgts$ Any | NumDmg$ X | SubAbility$ DBGiantDamage\n\
         SVar:DBGiantDamage:DB$ DealDamage | ConditionDefined$ DelayTriggerRememberedLKI | ConditionPresent$ Giant | ConditionCompare$ GE1 | Defined$ Targeted | NumDmg$ Y\n\
         SVar:X:TriggerRemembered$CardPower\nSVar:Y:TriggerRemembered$CardPower/Times.2",
    );
    assert!(
        flinger.abilities[0].contains("remember: TriggerRemember::SacrificedObject")
            && flinger.abilities[0].contains("Amount::IfCondition")
            && flinger.abilities[0].contains("factor: 2"),
        "{:?}",
        flinger.abilities
    );
}

#[test]
fn wave30b_moon_circuit_hacker() {
    let hacker = read(
        "Name:Moon-Circuit Hacker\nManaCost:1 U\nTypes:Enchantment Creature Human Ninja\nPT:2/1\nK:Ninjutsu:U\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Discard | Defined$ You | Mode$ TgtChoose | NumCards$ X | Cost$ Draw<1/You>\n\
         SVar:X:TriggeredSource$Valid Card.!ThisTurnEntered",
    );
    assert!(
        hacker.abilities[1]
            .contains("Effect::MayDo { effects: &[Effect::draw(1), Effect::IfCondition")
            && hacker.abilities[1]
                .contains("Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }"),
        "{:?}",
        hacker.abilities
    );
}

#[test]
fn wave30b_filigree_vector() {
    let vector = read(
        "Name:Filigree Vector\nManaCost:3 W\nTypes:Artifact Creature Phyrexian Construct\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigCounter\n\
         SVar:TrigCounter:DB$ TargetOnly | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ X | RememberTargets$ True | SubAbility$ DBChargeCounter\n\
         SVar:DBChargeCounter:DB$ TargetOnly | ValidTgts$ Artifact | TargetMin$ 0 | TargetMax$ Y | ImprintCards$ ThisTargetedCard | SubAbility$ DBPutCounterAll\n\
         SVar:DBPutCounterAll:DB$ PutCounterAll | ValidCards$ Creature.IsRemembered | CounterType$ P1P1 | CounterNum$ 1 | ValidCards2$ Artifact.IsImprinted | CounterType2$ CHARGE | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True | ClearRemembered$ True\n\
         SVar:X:Count$Valid Creature\nSVar:Y:Count$Valid Artifact",
    );
    assert!(
        vector.abilities[0].contains("Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }")
            && vector.abilities[0].contains("Effect::WithSecondTarget(&[Effect::AddCounter { kind: CounterKind::Charge, amount: Amount::Fixed(1) }])")
            && vector.abilities[0].contains("targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 255))")
            && vector.abilities[0].contains("second_targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::ARTIFACT), 255))"),
        "{:?}",
        vector.abilities
    );
}

#[test]
fn wave30b_reasonable_doubt() {
    let doubt = read(
        "Name:Reasonable Doubt\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ 2 | UnlessPayer$ ThisTargetedController | SubAbility$ DBSuspect\n\
         SVar:DBSuspect:DB$ AlterAttribute | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | Attributes$ Suspected",
    );
    assert!(
        doubt.abilities[0].contains("Effect::WithSecondTarget(&[Effect::Suspect { target: TargetSpec::Object(&Filter::CREATURE) }])")
            && doubt.abilities[0].contains("second_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE)))"),
        "{:?}",
        doubt.abilities
    );
}

#[test]
fn wave30b_once_and_future() {
    let once = read(
        "Name:Once and Future\nManaCost:3 G\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Card.YouOwn | SubAbility$ DBTarget\n\
         SVar:DBTarget:DB$ TargetOnly | TargetMin$ 0 | TargetMax$ 1 | TgtZone$ Graveyard | ValidTgts$ Card.YouOwn | TargetUnique$ True | RememberObjects$ ThisTargetedCard | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Library | Hidden$ True | Defined$ Remembered | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ DBReturn2\n\
         SVar:DBReturn2:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Hidden$ True | Defined$ Remembered | ConditionCheckSVar$ X | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Adamant.Green.1.0",
    );
    assert!(
        once.abilities[0].contains("TargetConstraint::DistinctFromPriorTargetGroups")
            && once.abilities[0].contains("Effect::GraveyardToTop")
            && once.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        once.abilities
    );
}

#[test]
fn wave30b_suffer_the_past() {
    let suffer = read(
        "Name:Suffer the Past\nManaCost:X B\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Player | SubAbility$ DBExileTargets\n\
         SVar:DBExileTargets:DB$ ChangeZone | TargetMin$ X | TargetMax$ X | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetsWithDefinedController$ ParentTarget | RememberChanged$ True | SubAbility$ DBSyphonLife\n\
         SVar:DBSyphonLife:DB$ LoseLife | Defined$ Targeted | LifeAmount$ Y | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\nSVar:Y:Remembered$Amount",
    );
    assert!(
        suffer.abilities[0].contains("second_targets = Some(TargetReq::x_targets(TargetSpec::CardInFirstTargetsGraveyard(&Filter::Any)))")
            && suffer.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)")
            && suffer.abilities[0].contains("PlayerRel::TargetPlayerOrController"),
        "{:?}",
        suffer.abilities
    );
}

#[test]
fn wave30b_relive_the_past() {
    let relive = read_w30b(
        "Name:Relive the Past\nManaCost:5 G W\nTypes:Sorcery\n\
         A:SP$ TargetOnly | TgtZone$ Graveyard | ValidTgts$ Artifact.YouOwn | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBLand\n\
         SVar:DBLand:DB$ TargetOnly | TgtZone$ Graveyard | ValidTgts$ Land.YouOwn | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBEnchantment\n\
         SVar:DBEnchantment:DB$ TargetOnly | TgtZone$ Graveyard | SubAbility$ DBChangeZoneAll | ValidTgts$ Enchantment.nonAura+YouOwn | TargetMin$ 0 | TargetMax$ 1\n\
         SVar:DBChangeZoneAll:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ Targeted | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Creature & Elemental | SetPower$ 5 | SetToughness$ 5",
    );
    assert!(
        relive.abilities[0].contains("Effect::WithAllTargets(&[Effect::MoveAndModify")
            && relive.abilities[0]
                .contains("third_targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard("),
        "{:?}",
        relive.abilities
    );
}

#[test]
fn wave30b_honest_refusal_boundaries() {
    for bad in [
        // `TargetMin$ 0 | TargetMax$ X` on Slot-2 `DealDamage` must stay refused (`x_targets` is exact X).
        "Name:UpToXSecond\nManaCost:X X G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ X | DamageSource$ ParentTarget | NumDmg$ Y\n\
         SVar:X:Count$xPaid\nSVar:Y:ParentTargeted$CardPower",
        // Mixed graveyard + battlefield 3-slot `TargetOnly` must stay refused.
        "Name:MixedZoneThree\nManaCost:4 W G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | TgtZone$ Graveyard | ValidTgts$ Artifact.YouOwn | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ B\n\
         SVar:B:DB$ TargetOnly | ValidTgts$ Land.YouCtrl | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ C\n\
         SVar:C:DB$ TargetOnly | TgtZone$ Graveyard | ValidTgts$ Creature.YouOwn | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ D\n\
         SVar:D:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ Targeted",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}
