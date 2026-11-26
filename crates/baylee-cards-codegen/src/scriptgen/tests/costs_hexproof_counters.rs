use super::*;

fn custom_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Rebel".into(),
        "Dragon".into(),
        "Ninja".into(),
        "Rogue".into(),
        "Bear".into(),
        "Druid".into(),
        "Cleric".into(),
        "Human".into(),
        "Scientist".into(),
        "Warrior".into(),
        "Orc".into(),
        "Berserker".into(),
        "Drake".into(),
        "Wolf".into(),
        "Soldier".into(),
        "Gorgon".into(),
        "Scout".into(),
    ]);
    c.artifact.extend(["Equipment".into(), "Vehicle".into()]);
    c.planeswalker.extend(["Kaya".into(), "Tezzeret".into()]);
    c.normalize();
    c
}

fn read_custom(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = custom_cats();
    let toks = tokens();
    transcode(&parsed, &c, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, Some(&toks))
        )
    })
}

fn refused_custom(text: &str) -> bool {
    let parsed = parse(text);
    let c = custom_cats();
    let toks = tokens();
    transcode(&parsed, &c, Some(&toks)).is_none()
}

#[test]
fn hexproof_from_variants_and_ignore_hexproof_matching() {
    let tam = read_custom(
        "Name:Tam, Mindful First-Year\nManaCost:1 G U\nTypes:Legendary Creature Gorgon Wizard\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.Other+YouCtrl | AddKeyword$ Hexproof:CardColors",
    );
    assert!(
        tam.abilities[0].contains("Modifier::HexproofFromOwnColors"),
        "{:?}",
        tam.abilities
    );

    let blade = read_custom(
        "Name:Dragonfire Blade\nManaCost:1\nTypes:Artifact Equipment\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | Affected$ Creature | AddPower$ 2 | AddToughness$ 2 | AddKeyword$ Hexproof:Card.MonoColor:monocolored\n\
         K:Equip:4:::ReduceCost$ X:This ability costs {1} less\n\
         SVar:X:Targeted$CardNumColors",
    );
    assert!(
        blade
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::HexproofFrom(&Filter::Monocolored)")),
        "{:?}",
        blade.abilities
    );
    assert!(
        blade.abilities.iter().any(|a| {
            a.contains("Modifier::EquipCostsLessByTargetAmount")
                && a.contains("EquipTargetMetric::ColorCount")
        }),
        "{:?}",
        blade.abilities
    );

    let drake = read_custom(
        "Name:Volatile Stormdrake\nManaCost:1 U\nTypes:Creature Drake\nPT:3/2\n\
         K:Flying\n\
         K:Hexproof:Activated:activated abilities\n\
         K:Hexproof:Triggered:triggered abilities",
    );
    assert_eq!(
        drake
            .abilities
            .iter()
            .filter(|a| a.contains("Modifier::HexproofFromAbilities"))
            .count(),
        1,
        "{:?}",
        drake.abilities
    );

    let veil = read_custom(
        "Name:Veil of Summer\nManaCost:G\nTypes:Instant\n\
         A:SP$ Draw | Defined$ You | ConditionCheckSVar$ X | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ AntiMagic | SubAbility$ DBPump\n\
         SVar:AntiMagic:Event$ Counter | ValidSA$ Spell.YouCtrl | Layer$ CantHappen\n\
         SVar:DBPump:DB$ Pump | Defined$ You & Valid Permanent.YouCtrl | KW$ Hexproof:Black & Hexproof:Blue\n\
         SVar:X:Count$ThisTurnCast_Card.OppCtrl+Blue,Card.OppCtrl+Black",
    );
    assert!(
        veil.abilities[0].contains("Modifier::PlayerHexproofFrom { who: PlayerRel::You")
            && veil.abilities[0].contains("Modifier::HexproofFrom("),
        "{:?}",
        veil.abilities
    );

    let spotlight = read_custom(
        "Name:Glaring Spotlight\nManaCost:1\nTypes:Artifact\n\
         S:Mode$ IgnoreHexproof | Activator$ You | ValidEntity$ Creature.OppCtrl",
    );
    assert!(
        spotlight.abilities[0].contains(
            "Modifier::IgnoreOpponentsHexproofMatching { permanent_filter: &Filter::CREATURE, players: false }"
        ),
        "{:?}",
        spotlight.abilities
    );

    let kaya = read_custom(
        "Name:Kaya, Bane of the Dead\nManaCost:3 WB WB WB\nTypes:Legendary Planeswalker Kaya\nLoyalty:7\n\
         S:Mode$ IgnoreHexproof | Activator$ You | ValidEntity$ Permanent.OppCtrl,Opponent\n\
         A:AB$ ChangeZone | Cost$ SubCounter<3/LOYALTY> | Planeswalker$ true | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile",
    );
    assert!(
        kaya.abilities[0].contains(
            "Modifier::IgnoreOpponentsHexproofMatching { permanent_filter: &Filter::Any, players: true }"
        ),
        "{:?}",
        kaya.abilities
    );

    for bad in [
        "Name:SoloTriggered\nManaCost:1 U\nTypes:Creature Drake\nPT:2/2\nK:Hexproof:Triggered:triggered abilities",
        "Name:ChosenHex\nManaCost:1 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ ChooseColor | Cost$ 1 | Defined$ You | SubAbility$ P\n\
         SVar:P:DB$ Pump | Defined$ Self | KW$ Hexproof:ChosenColor",
        "Name:OppIgnoreAll\nManaCost:1\nTypes:Artifact\n\
         S:Mode$ IgnoreHexproof | ValidEntity$ Permanent.OppCtrl,Opponent",
    ] {
        assert!(refused_custom(bad), "must refuse: {bad}");
    }
}

#[test]
fn flashback_dash_and_attached_power_amounts() {
    let catalyst = read_custom(
        "Name:Catalyst Stone\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Spell.Flashback | Activator$ You | Amount$ 2\n\
         S:Mode$ RaiseCost | ValidCard$ Card | ValidSpell$ Spell.Flashback | Activator$ Opponent | Amount$ 2",
    );
    assert!(
        catalyst.abilities[0].contains("Filter::CastForFlashback")
            && catalyst.abilities[0].contains("Modifier::SpellsCostLess(2)")
            && catalyst.abilities[1].contains("Filter::CastForFlashback")
            && catalyst.abilities[1].contains("Modifier::SpellsCostMore(2)"),
        "{:?}",
        catalyst.abilities
    );

    let warbringer = read_custom(
        "Name:Warbringer\nManaCost:3 R\nTypes:Creature Orc Berserker\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Spell.Dash | Activator$ You | Amount$ 2\n\
         K:Dash:2 R",
    );
    assert!(
        warbringer.abilities[0].contains("Filter::CastForDash")
            && warbringer.abilities[0].contains("Modifier::SpellsCostLess(2)"),
        "{:?}",
        warbringer.abilities
    );

    let mana_fb = read_custom(
        "Name:Memory Pool\nManaCost:no cost\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ B | RestrictValid$ Spell.Flashback,Spell.Dash",
    );
    assert!(
        mana_fb.statics.contains("Filter::CastForFlashback")
            && mana_fb.statics.contains("Filter::CastForDash")
            && mana_fb.abilities[0].contains(".restricted(&SPEND"),
        "{}\n{:?}",
        mana_fb.statics,
        mana_fb.abilities
    );

    let bind = read_custom(
        "Name:Bind the Monster\nManaCost:U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ You | NumDmg$ X\n\
         SVar:X:Enchanted$CardPower",
    );
    assert!(
        bind.abilities[1].contains("Amount::AttachedPower"),
        "{:?}",
        bind.abilities
    );

    // Altar of the Lost graveyard flashback filter must NOT conflate with CastForFlashback.
    assert!(refused_custom(
        "Name:GraveStone\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ Card.inZoneGraveyard+withFlashback",
    ));
}

#[test]
fn post_engine_cost_modifiers_and_refusals() {
    let doc = read_custom(
        "Name:Doc Aurlock, Grizzled Genius\nManaCost:G U\nTypes:Legendary Creature Bear Druid\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.wasCastFromYourGraveyard,Card.wasCastFromExile | Type$ Spell | Activator$ You | Amount$ 2\n\
         S:Mode$ ReduceCost | ValidSpell$ Static.Plotting | AffectedZone$ Hand | Activator$ You | Amount$ 2",
    );
    assert!(
        doc.abilities[1]
            .contains("Modifier::PlotActionCostsLess { who: PlayerRel::You, amount: 2 }"),
        "{:?}",
        doc.abilities
    );

    let doomsayer = read_custom(
        "Name:Exiled Doomsayer\nManaCost:1 W\nTypes:Creature Human Cleric\nPT:1/2\n\
         S:Mode$ RaiseCost | ValidCard$ Creature.faceDown | ValidSpell$ Static.MorphUp | Amount$ 2",
    );
    assert!(
        doomsayer.abilities[0].contains("Modifier::MorphTurnFaceUpCostsMore { amount: 2 }"),
        "{:?}",
        doomsayer.abilities
    );

    let brutal = read_custom(
        "Name:Brutal Suppression\nManaCost:R\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | Type$ Ability | ValidCard$ Rebel.!token | Cost$ Sac<1/Land> | AffectedZone$ Battlefield",
    );
    assert!(
        brutal.abilities[0].contains("Modifier::ActivatedAbilitiesCostAdditional")
            && brutal.abilities[0].contains("CostPart::Sacrifice(&Filter::YOUR_LAND)"),
        "{:?}",
        brutal.abilities
    );

    let prowler = read_custom(
        "Name:Cemetery Prowler\nManaCost:1 G G\nTypes:Creature Wolf\nPT:3/4\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Amount$ AffectedX | Activator$ You\n\
         SVar:AffectedX:Count$TypesSharedWith ValidExile Card.ExiledWithSource",
    );
    assert!(
        prowler.abilities[0].contains(
            "Modifier::SpellsCostLessPerSharedCardTypeWithExiled { who: PlayerRel::You }"
        ),
        "{:?}",
        prowler.abilities
    );

    let zimone_creature = read_custom(
        "Name:Zimone Creature Discount\nManaCost:1 G U\nTypes:Legendary Creature Human Wizard\nPT:0/4\n\
         S:Mode$ ReduceCost | ValidCard$ Creature | Type$ Spell | Activator$ You | Amount$ Y | CheckSVar$ X | SVarCompare$ EQ0\n\
         SVar:X:Count$ThisTurnCast_Creature.YouCtrl\n\
         SVar:Y:Count$CardCounters.P1P1",
    );
    assert!(
        zimone_creature.abilities[0].contains("Modifier::FirstSpellEachTurnCostAdjustByAmount")
            && zimone_creature.abilities[0].contains("Amount::CountersOnSource(CounterKind::P1P1)"),
        "{:?}",
        zimone_creature.abilities
    );

    let hojo = read_custom(
        "Name:Professor Hojo\nManaCost:1 G\nTypes:Legendary Creature Human Scientist\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.YouCtrl | ValidTarget$ Creature.YouCtrl+inZoneBattlefield | Type$ Ability | Activator$ You.Active | Amount$ 2 | CheckSVar$ X | SVarCompare$ LE1\n\
         SVar:X:Count$ThisTurnActivated_Activated.YouCtrl+IsTargeting Valid Creature.YouCtrl~inZoneBattlefield",
    );
    assert!(
        hojo.abilities[0].contains("Modifier::FirstActivatedAbilityEachTurnCostLess")
            && hojo.abilities[0].contains("your_turn_only: true"),
        "{:?}",
        hojo.abilities
    );

    let tezzeret = read_custom(
        "Name:Tezzeret\nManaCost:2 U U\nTypes:Legendary Planeswalker Tezzeret\nLoyalty:4\n\
         S:Mode$ ReduceCost | ValidCard$ Artifact.inZoneBattlefield | ValidSpell$ Activated.YouCtrl | Type$ Ability | Amount$ 2 | CheckSVar$ ArtAct | SVarCompare$ LE1 | AffectedZone$ Battlefield\n\
         SVar:ArtAct:Count$ThisTurnActivated_Activated.Artifact+YouCtrl+inZoneBattlefield",
    );
    assert!(
        tezzeret.abilities[0].contains("Modifier::FirstActivatedAbilityEachTurnCostLess")
            && tezzeret.abilities[0].contains("your_turn_only: false"),
        "{:?}",
        tezzeret.abilities
    );

    let berserker = read_custom(
        "Name:Dragonkin Berserker\nManaCost:1 R\nTypes:Creature Human Berserker\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Boast | Activator$ You | Amount$ X | EffectZone$ Battlefield\n\
         SVar:X:Count$Valid Dragon.YouCtrl",
    );
    assert!(
        berserker.abilities[0].contains("Modifier::ActivatedAbilitiesWithKeywordCostLess")
            && berserker.abilities[0].contains("ActivatedAbilityKind::Boast"),
        "{:?}",
        berserker.abilities
    );

    let blades = read_custom(
        "Name:Warrior's Blades\nManaCost:2 R W\nTypes:Artifact Equipment\n\
         K:Equip:3:::ReduceCost$ X:This ability costs {1} less\n\
         SVar:X:Targeted$CardCounters.P1P1",
    );
    assert!(
        blades.abilities.iter().any(|a| {
            a.contains("Modifier::EquipCostsLessByTargetAmount")
                && a.contains("EquipTargetMetric::Counters(CounterKind::P1P1)")
        }),
        "{:?}",
        blades.abilities
    );

    let liesa = read_custom(
        "Name:Liesa, Shroud of Dusk\nManaCost:2 W W B\nTypes:Legendary Creature Angel\nPT:5/5\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self+wasCastFromCommand | Type$ Spell | Amount$ X | EffectZone$ Stack\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self+wasCastFromCommand | Type$ Spell | Cost$ PayLife<X> | EffectZone$ Stack | Secondary$ True\n\
         SVar:X:Count$CommanderCastFromCommandZone/Times.2",
    );
    assert!(
        liesa.abilities[0].contains("Modifier::CommanderTaxReplacedWithLife { life_per_tax: 2 }")
            && liesa.abilities[0].contains("zone = ZoneRef::Command"),
        "{:?}",
        liesa.abilities
    );

    let siege = read_custom(
        "Name:Monastery Siege\nManaCost:2 U\nTypes:Enchantment\n\
         K:ETBReplacement:Other:SiegeChoice\n\
         SVar:SiegeChoice:DB$ GenericChoice | Choices$ Khans,Dragons | Defined$ You | SetChosenMode$ True\n\
         SVar:Khans:DB$ Nothing | SpellDescription$ Khans\n\
         SVar:Dragons:DB$ Nothing | SpellDescription$ Dragons\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.ChosenModeDragons | AddStaticAbility$ DragonsST\n\
         SVar:DragonsST:Mode$ RaiseCost | ValidTarget$ You,Card.YouCtrl+inZoneBattlefield | Activator$ Opponent | Type$ Spell | Amount$ 2 | Secondary$ True",
    );
    assert!(
        siege.abilities.iter().any(|a| {
            a.contains("Modifier::RaiseCostIfTargetsYouOrPermanent")
                && a.contains("extra_mana: mana!(\"{2}\")")
        }),
        "{:?}",
        siege.abilities
    );

    let interrogation = read_custom(
        "Name:Officious Interrogation\nManaCost:W U\nTypes:Instant\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Amount$ IncreaseCost | Relative$ True | Cost$ W U | EffectZone$ All\n\
         SVar:IncreaseCost:TargetedObjects$Amount/Minus.1\n\
         A:SP$ TargetOnly | ValidTgts$ Player | TargetMin$ 0 | TargetMax$ 2",
    );
    assert!(
        interrogation.abilities[0].contains("Modifier::AdjustSpellCostBeyondFirstTarget")
            && interrogation.abilities[0].contains("mana_per_extra_target: mana!(\"{W}{U}\")"),
        "{:?}",
        interrogation.abilities
    );

    let lullmage = read_custom(
        "Name:Lullmage's Domination\nManaCost:X U U U\nTypes:Sorcery\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ XGrave | Relative$ True | EffectZone$ All\n\
         SVar:XGrave:Count$Compare CheckTgt GE8.3.0\n\
         SVar:CheckTgt:TargetedController$CardsInGraveyard\n\
         A:SP$ GainControl | ValidTgts$ Creature.cmcEQX\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        lullmage.abilities[0].contains("Modifier::ReduceCostIfTargetsCheck")
            && lullmage.abilities[0]
                .contains("TargetCostCheck::TargetControllerGraveyardAtLeast(8)"),
        "{:?}",
        lullmage.abilities
    );

    let not_of_this_world = read_custom(
        "Name:Not of This World\nManaCost:7\nTypes:Kindred Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ CostReduction | Relative$ True | EffectZone$ All\n\
         A:SP$ Counter | TargetType$ Spell,Activated,Triggered | ValidTgts$ Card,Emblem | TargetValidTargeting$ Permanent.YouCtrl+inRealZoneBattlefield\n\
         SVar:CostReduction:Count$Compare CheckTgt GE1.7.0\n\
         SVar:CheckTgt:TargetedByTarget$Valid Card.powerGE7+YouCtrl",
    );
    assert!(
        not_of_this_world.abilities[0].contains("Modifier::ReduceCostIfTargetsCheck")
            && not_of_this_world.abilities[0]
                .contains("TargetCostCheck::TargetedStackTargetsYouControl("),
        "{:?}",
        not_of_this_world.abilities
    );

    let flying_drone = read_custom(
        "Name:Flying Drone\nManaCost:1 U\nTypes:Artifact Creature Robot Scout\nPT:1/2\n\
         A:AB$ Draw | Cost$ 1 U T | ReduceCost$ 1 U | ReduceAmount$ X | NumCards$ 1\n\
         SVar:X:Count$Compare Y GE1.1.0\n\
         SVar:Y:Count$ThisTurnEntered_Battlefield_Creature.Other+withFlying+YouCtrl",
    );
    assert!(
        flying_drone.abilities[0].contains("CostReduction::IfMana")
            && flying_drone.abilities[0].contains("mana: mana!(\"{1}{U}\")"),
        "{:?}",
        flying_drone.abilities
    );

    let raft = read_custom(
        "Name:Raft Security Officer\nManaCost:1 W\nTypes:Creature Human Soldier\nPT:1/3\n\
         A:AB$ Tap | Cost$ 2 T | ValidTgts$ Creature | ReduceCost$ X\n\
         SVar:X:AllTargeted$Valid Creature.powerLE3",
    );
    assert!(
        raft.abilities[0].contains("CostReduction::IfTargetsMatch")
            && raft.abilities[0].contains("min_count: 1, amount: 1"),
        "{:?}",
        raft.abilities
    );

    let wayta = read_custom(
        "Name:Wayta, Trainer Prodigy\nManaCost:R G W\nTypes:Legendary Creature Human Warrior\nPT:1/5\n\
         A:AB$ TargetOnly | Cost$ 2 G T | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight | ReduceCost$ X\n\
         SVar:DBFight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature | TargetUnique$ True\n\
         SVar:X:Count$Compare Y EQ2.2.0\n\
         SVar:Y:AllTargeted$Valid Creature.YouCtrl",
    );
    assert!(
        wayta.abilities[0].contains("CostReduction::IfTargetsMatch")
            && wayta.abilities[0].contains("min_count: 2, amount: 2"),
        "{:?}",
        wayta.abilities
    );

    // Honest refusals: unpaired Liesa half, mismatched CheckSVar target, non-Battlefield AttachedPower.
    for bad in [
        "Name:UnpairedLiesa\nManaCost:2 W W B\nTypes:Legendary Creature Angel\nPT:5/5\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self+wasCastFromCommand | Type$ Spell | Amount$ X | EffectZone$ Stack\n\
         SVar:X:Count$CommanderCastFromCommandZone/Times.2",
        "Name:BadHojoMismatch\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Artifact.inZoneBattlefield | ValidSpell$ Activated.YouCtrl | Type$ Ability | Amount$ 2 | CheckSVar$ X | SVarCompare$ LE1\n\
         SVar:X:Count$ThisTurnActivated_Activated.Creature+YouCtrl",
        "Name:SpellAttachedPower\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Enchanted$CardPower",
    ] {
        assert!(refused_custom(bad), "must refuse: {bad}");
    }
}
