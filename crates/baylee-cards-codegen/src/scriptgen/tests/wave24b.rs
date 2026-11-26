//! Wave 24b: cross-zone Layer 4/5 statics, off-battlefield Flash, non-P/T CDAs,
//! conditional self-stack Cascade, static granted keyword abilities, phase/SVar
//! static conditions, Graveyard+Exile / `HandChosen` counts, and `RaiseCost` `DiscardX`.

use super::*;

fn wave24b_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Assassin".into(),
            "Goblin".into(),
            "Knight".into(),
            "Merfolk".into(),
            "Mount".into(),
            "Sliver".into(),
            "Spirit".into(),
            "Wizard".into(),
        ],
        artifact: vec![
            "Blood".into(),
            "Clue".into(),
            "Equipment".into(),
            "Vehicle".into(),
        ],
        land: vec![
            "Desert".into(),
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w24b(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = wave24b_cats();
    transcode(&parsed, &cats, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, None)
        )
    })
}

fn refused_w24b(text: &str) -> bool {
    let cats = wave24b_cats();
    transcode(&parse(text), &cats, None).is_none()
}

#[test]
fn all_zones_layer45_statics_cover_you_ctrl_and_unscoped() {
    let nexus = read_w24b(
        "Name:Maskwood Nexus\nManaCost:4\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AffectedZone$ All | \
         AddAllCreatureTypes$ True | Description$ Creatures you control are every creature type.",
    );
    assert_eq!(nexus.abilities.len(), 1);
    assert!(
        nexus.abilities[0].contains("Modifier::AllCreatureTypes")
            && nexus.abilities[0].contains("Filter::ControlledByYou")
            && nexus.abilities[0].contains("Filter::OwnedByYou")
            && nexus.abilities[0].contains("Filter::InZone(ZoneRef::NotBattlefield)")
            && nexus.abilities[0].contains("Filter::Not(&Filter::InZone(ZoneRef::Stack))"),
        "{:?}",
        nexus.abilities
    );

    let adaptation = read_w24b(
        "Name:Arcane Adaptation\nManaCost:2 U\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AffectedZone$ All | \
         AddType$ ChosenType",
    );
    assert!(
        adaptation
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AddChosenSubtype")
                && a.contains("Filter::InZone(ZoneRef::NotBattlefield)")),
        "{:?}",
        adaptation.abilities
    );

    let encroaching = read_w24b(
        "Name:Encroaching Mycosynth\nManaCost:3 U\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Permanent.nonLand+YouCtrl | AffectedZone$ All | \
         AddType$ Artifact",
    );
    assert_eq!(encroaching.abilities.len(), 1);
    assert!(
        encroaching.abilities[0].contains("Modifier::AddType(TypeSet::ARTIFACT)")
            && encroaching.abilities[0].contains("Filter::NONLAND")
            && encroaching.abilities[0].contains("Filter::InZone(ZoneRef::NotBattlefield)"),
        "{:?}",
        encroaching.abilities
    );

    let roshan_shape = read_w24b(
        "Name:Roshan Shape\nManaCost:2 B R\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl+Other | AffectedZone$ All | \
         AddType$ Assassin",
    );
    assert!(
        roshan_shape.abilities[0].contains("Filter::ANOTHER_CREATURE")
            && roshan_shape.abilities[0]
                .contains("Modifier::AddSubtype(subtypes::creature::ASSASSIN)")
            && roshan_shape.abilities[0].contains("Filter::InZone(ZoneRef::NotBattlefield)"),
        "{:?}",
        roshan_shape.abilities
    );

    let rukarumel_shape = read_w24b(
        "Name:Rukarumel Shape\nManaCost:W U B R G\nTypes:Creature\nPT:3/3\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl+!token,Sliver.YouCtrl | \
         AffectedZone$ All | AddType$ ChosenType",
    );
    assert!(
        rukarumel_shape.abilities[0].contains("Modifier::AddChosenSubtype")
            && rukarumel_shape.abilities[0].contains("subtypes::creature::SLIVER")
            && rukarumel_shape.abilities[0].contains("Filter::InZone(ZoneRef::NotBattlefield)"),
        "{:?}",
        rukarumel_shape.abilities
    );

    let lattice = read_w24b(
        "Name:Mycosynth Lattice\nManaCost:6\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Permanent | AddType$ Artifact\n\
         S:Mode$ Continuous | Affected$ Card | SetColor$ Colorless | AffectedZone$ All\n\
         S:Mode$ ManaConvert | ManaConversion$ AnyType->AnyColor",
    );
    assert!(
        lattice.abilities.iter().any(|a| {
            a.contains("Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::NotBattlefield)])")
                && a.contains("Modifier::SetColor(ColorSet::EMPTY)")
        }),
        "{:?}",
        lattice.abilities
    );
}

#[test]
fn all_zones_layer45_statics_refuse_unsupported_clauses() {
    for bad in [
        // Alchemy P/T change across all zones (The Muckslinger Gang).
        "Name:Muckslinger\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Goblin.YouCtrl | AffectedZone$ All | AddPower$ 1 | AddToughness$ 1",
        // Keyword grant across all zones (Disguise Agent).
        "Name:Disguise Agent\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.YouOwn | AffectedZone$ All | AddKeyword$ Flying",
        // `RemoveCreatureTypes$ True` (Conspiracy).
        "Name:Conspiracy\nManaCost:3 B B\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AffectedZone$ All | \
         AddType$ ChosenType | RemoveCreatureTypes$ True",
        // `AddColor$ ChosenColor` (Painter's Servant).
        "Name:Painters Servant\nManaCost:2\nTypes:Artifact Creature\nPT:1/3\n\
         S:Mode$ Continuous | Affected$ Card | AddColor$ ChosenColor | AffectedZone$ All",
        // `MayPlay$ True` across all zones (Weftwalking).
        "Name:Weftwalking\nManaCost:4 U U\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl | AffectedZone$ All | MayPlay$ True",
        // Alchemy perpetual remembered-card filter.
        "Name:Perpetual\nManaCost:1 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ All | AddType$ Artifact",
    ] {
        assert!(refused_w24b(bad), "should refuse: {bad}");
    }
}

#[test]
fn offboard_flash_grants_only_flash_across_all_non_battlefield_zones() {
    let teferi = read_w24b(
        "Name:Teferi, Mage of Zhalfir\nManaCost:2 U U U\nTypes:Legendary Creature Human Wizard\nPT:3/4\n\
         K:Flash\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn+!token | \
         AffectedZone$ Hand,Graveyard,Exile,Library,Command,Stack | AddKeyword$ Flash\n\
         S:Mode$ CantBeCast | ValidCard$ Card | OnlySorcerySpeed$ True | Caster$ Opponent",
    );
    assert!(
        teferi.abilities.iter().any(|a| {
            a.contains("Filter::InZone(ZoneRef::NotBattlefield)")
                && a.contains("Filter::OwnedByYou")
                && a.contains("Modifier::AddKeyword(KeywordSet::FLASH)")
        }),
        "{:?}",
        teferi.abilities
    );

    for bad in [
        // Non-Flash off-board keyword grant (Falkenrath Gorger / Madness).
        "Name:BadMadness\nManaCost:R\nTypes:Creature\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | \
         AffectedZone$ Hand,Graveyard,Exile,Library,Command,Stack | AddKeyword$ Haste",
        // Partial non-battlefield zone list.
        "Name:BadPartialZones\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | \
         AffectedZone$ Hand,Graveyard | AddKeyword$ Flash",
        // `MayPlay` alongside `AddKeyword$ Flash`.
        "Name:BadMayPlayFlash\nManaCost:2 U\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | \
         AffectedZone$ Hand,Graveyard,Exile,Library,Command,Stack | AddKeyword$ Flash | MayPlay$ True",
    ] {
        assert!(refused_w24b(bad), "should refuse: {bad}");
    }
}

#[test]
fn non_pt_color_and_type_cdas_emit_expected_zones() {
    // Non-permanent color CDA (Ghostfire): off-battlefield only.
    let ghostfire = read_w24b(
        "Name:Ghostfire\nManaCost:2 R\nTypes:Instant\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | SetColor$ Colorless\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3",
    );
    let cda_statics: Vec<&String> = ghostfire
        .abilities
        .iter()
        .filter(|a| a.contains("Modifier::SetColor(ColorSet::EMPTY)"))
        .collect();
    assert_eq!(cda_statics.len(), 1, "{:?}", ghostfire.abilities);
    assert_eq!(
        cda_statics[0],
        "static_ability!(Filter::This, Modifier::SetColor(ColorSet::EMPTY), zone = ZoneRef::NotBattlefield)"
    );

    // Permanent color/type CDAs (Transguild Courier, Mistform Ultimus, Packbeast): both zones.
    let courier = read_w24b(
        "Name:Transguild Courier\nManaCost:4\nTypes:Artifact Creature Golem\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | SetColor$ All",
    );
    let all_colors = "ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black, Color::Red, Color::Green])";
    assert_eq!(
        courier.abilities,
        [
            format!("static_ability!(Filter::This, Modifier::SetColor({all_colors}))"),
            format!(
                "static_ability!(Filter::This, Modifier::SetColor({all_colors}), zone = ZoneRef::NotBattlefield)"
            ),
        ]
    );

    let ultimus = read_w24b(
        "Name:Mistform Ultimus\nManaCost:3 U\nTypes:Legendary Creature Illusion\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | AddAllCreatureTypes$ True",
    );
    assert_eq!(
        ultimus.abilities,
        [
            "static_ability!(Filter::This, Modifier::AllCreatureTypes)",
            "static_ability!(Filter::This, Modifier::AllCreatureTypes, zone = ZoneRef::NotBattlefield)",
        ]
    );

    let packbeast = read_w24b(
        "Name:Stonework Packbeast\nManaCost:2\nTypes:Artifact Creature Beast\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | AddType$ Goblin & Wizard",
    );
    assert_eq!(packbeast.abilities.len(), 4, "{:?}", packbeast.abilities);
    assert!(
        packbeast.abilities.contains(
            &"static_ability!(Filter::This, Modifier::AddSubtype(subtypes::creature::GOBLIN), zone = ZoneRef::NotBattlefield)".to_string()
        ) && packbeast.abilities.contains(
            &"static_ability!(Filter::This, Modifier::AddSubtype(subtypes::creature::WIZARD))".to_string()
        )
    );
}

#[test]
fn conditional_self_flash_split_second_and_stack_cascade_and_refusals() {
    let rattlewurm = read_w24b(
        "Name:Colossal Rattlewurm\nManaCost:2 G G\nTypes:Creature Wurm\nPT:6/5\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | AddKeyword$ Flash | IsPresent$ Desert.YouCtrl",
    );
    assert!(
        rattlewurm.abilities[0].contains("Modifier::AddKeyword(KeywordSet::FLASH)")
            && rattlewurm.abilities[0].contains("zone = ZoneRef::NotBattlefield")
            && rattlewurm.abilities[0].contains("Condition::ControlCount(")
            && rattlewurm.statics.contains("subtypes::land::DESERT"),
        "{:?}\n{}",
        rattlewurm.abilities,
        rattlewurm.statics
    );

    let take_for_a_ride = read_w24b(
        "Name:Take for a Ride\nManaCost:2 R\nTypes:Sorcery\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | AddKeyword$ Flash | \
         CheckSVar$ Count$CommittedCrimeThisTurn.1.0\n\
         A:SP$ GainControl | ValidTgts$ Creature | LoseControl$ EOT | Untap$ True | AddKWs$ Haste",
    );
    assert!(
        take_for_a_ride.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::FLASH), \
                  zone = ZoneRef::NotBattlefield, \
                  condition = Some(Condition::CommittedCrimeThisTurn(PlayerRel::You)))"
        }),
        "{:?}",
        take_for_a_ride.abilities
    );

    let molten = read_w24b(
        "Name:Molten Disaster\nManaCost:X R R\nTypes:Sorcery\n\
         K:Kicker:R\n\
         S:Mode$ Continuous | IsPresent$ Card.Self+kicked | PresentZone$ Stack | \
         CharacteristicDefining$ True | AddKeyword$ Split second\n\
         A:SP$ DamageAll | ValidCards$ Creature.withoutFlying | ValidPlayers$ Player | NumDmg$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        molten.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::SPLIT_SECOND), \
                  zone = ZoneRef::Stack, condition = Some(Condition::KickedWith(0)))"
        }),
        "{:?}",
        molten.abilities
    );

    let marauder = read_w24b(
        "Name:Bloodbraid Marauder\nManaCost:1 R\nTypes:Creature Human Berserker\nPT:3/1\n\
         S:Mode$ Continuous | EffectZone$ Stack | AffectedZone$ Stack | Affected$ Card.Self | \
         Condition$ Delirium | AddKeyword$ Cascade",
    );
    assert_eq!(
        marauder.abilities,
        [
            "triggered!(Trigger::SpellCast(&Filter::This), &[Effect::Cascade], \
             condition = Some(Condition::TriggerOnly(&Condition::GraveyardTypesAtLeast(4))))"
        ]
    );

    for bad in [
        // Alchemy Coalition Construct (`AddType$ ChosenType` CDA).
        "Name:Coalition Construct\nManaCost:2\nTypes:Artifact Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | AddType$ ChosenType",
        // Commander pregame color choice (`SetColor$ ChosenColor`).
        "Name:Prismatic Piper\nManaCost:5\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | CharacteristicDefining$ True | SetColor$ ChosenColor",
        // Non-Flash keyword CDA.
        "Name:BadNonFlashCdaKeyword\nManaCost:1 U\nTypes:Sorcery\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | AddKeyword$ Flying\n\
         A:SP$ Draw | NumCards$ 1",
        // Self-Flash CDA with unclaimed parameter.
        "Name:BadFlashExtraParam\nManaCost:1 U\nTypes:Sorcery\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | AddKeyword$ Flash | Unknown$ True\n\
         A:SP$ Draw | NumCards$ 1",
        // Kicked Split second without Kicker declared.
        "Name:BadNoKickerSplitSecond\nManaCost:R\nTypes:Sorcery\n\
         S:Mode$ Continuous | IsPresent$ Card.Self+kicked | PresentZone$ Stack | \
         CharacteristicDefining$ True | AddKeyword$ Split second\n\
         A:SP$ Draw | NumCards$ 1",
        // Non-self battlefield Cascade grant (Wildsear / Zhulodok: Cascade reads source MV).
        "Name:Wildsear\nManaCost:3 R G\nTypes:Creature\nPT:6/6\n\
         S:Mode$ Continuous | AffectedZone$ Stack | Affected$ Creature.YouCtrl | AddKeyword$ Cascade",
        // Unconditional self-stack Cascade.
        "Name:BadUncondCascade\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | EffectZone$ Stack | AffectedZone$ Stack | Affected$ Card.Self | \
         AddKeyword$ Cascade",
    ] {
        assert!(refused_w24b(bad), "should refuse: {bad}");
    }
}

#[test]
fn static_granted_protection_landwalk_exalted_and_prevention_keywords() {
    let exalted = read_w24b(
        "Name:Sublime Archangel\nManaCost:2 W W\nTypes:Creature Angel\nPT:4/3\n\
         S:Mode$ Continuous | Affected$ Creature.Other+YouCtrl | AddKeyword$ Exalted",
    );
    assert!(
        exalted.abilities[0].contains("Trigger::AttacksAlone(&Filter::YOUR_CREATURE)")
            && exalted.abilities[0].contains("Effect::PumpEventObject"),
        "{:?}",
        exalted.abilities
    );

    let prot_colors = read_w24b(
        "Name:Etched Champion\nManaCost:3\nTypes:Artifact Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | Condition$ Metalcraft | \
         AddKeyword$ Protection from each color",
    );
    assert!(
        prot_colors.abilities[0].contains("Modifier::ProtectionFrom(&Filter::HasColor(")
            && prot_colors.abilities[0].contains("Color::White")
            && prot_colors.abilities[0].contains("Color::Green"),
        "{:?}",
        prot_colors.abilities
    );

    let prot_all = read_w24b(
        "Name:Vexilus Praetor\nManaCost:3 W\nTypes:Creature\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Protection from everything",
    );
    assert!(
        prot_all.abilities[0].contains("Modifier::ProtectionFrom(&Filter::Any)"),
        "{:?}",
        prot_all.abilities
    );

    let boots = read_w24b(
        "Name:Trailblazers Boots\nManaCost:2\nTypes:Artifact Equipment\nK:Equip:2\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | Affected$ Creature | \
         AddKeyword$ Landwalk:Land.nonBasic:Nonbasic land",
    );
    assert!(
        boots
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::LandwalkMatching(&Filter::NONBASIC_LAND)")),
        "{:?}",
        boots.abilities
    );

    let prev = read_w24b(
        "Name:Prevention Suite\nManaCost:3 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | \
         AddKeyword$ Prevent all combat damage that would be dealt to CARDNAME.\n\
         S:Mode$ Continuous | Affected$ Creature.OppCtrl | \
         AddKeyword$ Prevent all combat damage that would be dealt by CARDNAME.\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | \
         AddKeyword$ Prevent all combat damage that would be dealt to and dealt by CARDNAME.\n\
         S:Mode$ Continuous | Affected$ Creature.OppCtrl | \
         AddKeyword$ Prevent all damage that would be dealt by CARDNAME.\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | \
         AddKeyword$ Prevent all damage that would be dealt to CARDNAME.",
    );
    assert!(
        prev.abilities
            .iter()
            .any(|a| a.contains("Modifier::PreventDamageToIt"))
    );
    assert!(
        prev.abilities
            .iter()
            .any(|a| a.contains("Modifier::PreventDamageFromIt"))
    );
    assert!(
        prev.abilities
            .iter()
            .any(|a| a.contains("Modifier::PreventAllDamageFromIt"))
    );
    assert!(
        prev.abilities
            .iter()
            .any(|a| a.contains("Modifier::PreventDamageFrom(&Filter::Any)"))
    );
}

#[test]
fn static_granted_triggered_and_activated_keywords_and_refusals() {
    let triggered_grants = read_w24b(
        "Name:Triggered Keyword Grants\nManaCost:4 W B\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Annihilator:2\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Afflict:3\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Afterlife:1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Poisonous:3\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Renown:1",
    );
    let joined = triggered_grants.abilities.join("\n");
    assert!(
        joined.contains("Effect::SacrificeAmountForPlayers"),
        "{joined}"
    );
    assert!(
        joined.contains("KeywordSet::AFFLICT") && joined.contains("Trigger::BecomesBlocked"),
        "{joined}"
    );
    assert!(
        joined.contains("KeywordSet::AFTERLIFE")
            && joined.contains("generated_tokens::SPIRIT_1_1_WHITE_BLACK_FLYING"),
        "{joined}"
    );
    assert!(joined.contains("Effect::Poison"), "{joined}");
    assert!(
        joined.contains("KeywordSet::RENOWN")
            && joined.contains("Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_"),
        "{joined}"
    );
    assert!(
        triggered_grants
            .statics
            .contains("Effect::BecomeRenowned(1)"),
        "{}",
        triggered_grants.statics
    );

    let activated_grants = read_w24b(
        "Name:Activated Keyword Grants\nManaCost:3 W R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Vehicle.YouCtrl | AddKeyword$ Crew:2\n\
         S:Mode$ Continuous | Affected$ Mount.YouCtrl | AddKeyword$ Saddle:2\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Outlast:G W\n\
         S:Mode$ Continuous | Affected$ Equipment.YouCtrl | Condition$ Metalcraft | AddKeyword$ Equip:0\n\
         S:Mode$ Continuous | Affected$ Equipment.YouCtrl | AddKeyword$ Equip:0:Creature.YouCtrl+Knight:Knight\n\
         S:Mode$ Continuous | Affected$ Clue.YouCtrl | AddType$ Equipment | AddKeyword$ Equip:2",
    );
    assert!(
        activated_grants.statics.contains("crew!(2)")
            && activated_grants.statics.contains("saddle!(2)")
            && activated_grants
                .statics
                .contains("Effect::AddCounterSource")
            && activated_grants.statics.contains("equip!(cost!(\"{0}\"))")
            && activated_grants
                .statics
                .contains("subtypes::creature::KNIGHT")
            && activated_grants.statics.contains("equip!(cost!(\"{2}\"))"),
        "{}",
        activated_grants.statics
    );

    for bad in [
        "Name:BadRemoveExalted\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature | RemoveKeyword$ Exalted",
        "Name:BadCantHaveAnnihilator\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature | CantHaveKeyword$ Annihilator:1",
        "Name:BadRemoveProtAll\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature | RemoveKeyword$ Protection from everything",
        "Name:BadEquipNonEquipment\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Equip:2",
        "Name:BadEquipOpp\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Equipment.YouCtrl | AddKeyword$ Equip:1:Creature.OppCtrl:opponent",
        "Name:BadMustBeBlockedBy\nManaCost:2 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ MustBeBlockedBy Creature.Goblin",
        "Name:BadGrantToxic\nManaCost:2 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Toxic:1",
    ] {
        assert!(refused_w24b(bad), "should refuse: {bad}");
    }
}

#[test]
fn phases_extended_counts_and_defender_attack_conditions() {
    let cyclops = read_w24b(
        "Name:Piston-Fist Cyclops\nManaCost:1 UR UR\nTypes:Creature Cyclops\nPT:4/3\n\
         K:Defender\n\
         S:Mode$ CanAttackDefender | ValidCard$ Card.Self | CheckSVar$ X | SVarCompare$ GE1\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert!(
        cyclops
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AttacksDespiteDefender")
                && a.contains("condition = Some(")),
        "{:?}",
        cyclops.abilities
    );

    let end_step_cost = read_w24b(
        "Name:End Step Discount\nManaCost:3 W B\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | \
         Phases$ End of Turn | PlayerTurn$ You | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert!(
        end_step_cost.abilities.iter().any(|a| {
            a.contains("Modifier::SpellsCostLess(2)")
                && a.contains("Condition::DuringStep(StepKind::End)")
                && a.contains("Condition::YourTurn")
        }),
        "{:?}",
        end_step_cost.abilities
    );

    let huskburster = read_w24b(
        "Name:Huskburster Swarm\nManaCost:7 B\nTypes:Creature Insect\nPT:6/6\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Activator$ You | \
         EffectZone$ All | Amount$ X\n\
         SVar:X:Count$ValidGraveyard,Exile Creature.YouOwn",
    );
    assert!(
        huskburster.abilities.iter().any(|a| {
            a.contains("ZoneSel::GraveyardOf(PlayerRel::You)") && a.contains("ZoneSel::ExileAll")
        }) && huskburster.statics.contains("Filter::OwnedByYou"),
        "{:?}\n{}",
        huskburster.abilities,
        huskburster.statics
    );

    let nyxathid = read_w24b(
        "Name:Nyxathid\nManaCost:1 B B\nTypes:Creature Elemental\nPT:7/7\n\
         K:ETBReplacement:Other:ChooseOpp\n\
         SVar:ChooseOpp:DB$ ChoosePlayer | Defined$ You | Choices$ Opponent\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ -X | AddToughness$ -X\n\
         SVar:X:Count$ValidHand Card.ChosenCtrl",
    );
    assert!(
        nyxathid
            .abilities
            .iter()
            .any(|a| a.contains("ZoneSel::HandChosen")),
        "{:?}",
        nyxathid.abilities
    );
}

#[test]
fn amount_compare_ai_hints_and_raise_cost_discard_x() {
    let cmp_static = read_w24b(
        "Name:Amount Compare Static\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ Indestructible | \
         CheckSVar$ X | SVarCompare$ EQY\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n\
         SVar:Y:Count$Valid Creature.YouCtrl",
    );
    assert!(
        cmp_static
            .abilities
            .iter()
            .any(|a| { a.contains("Condition::AmountCompare") && a.contains("AmountCmp::Eq") }),
        "{:?}",
        cmp_static.abilities
    );

    let ai_hint = read_w24b(
        "Name:AI Hint Static\nManaCost:1 B\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 1 | AddToughness$ 1 | \
         AddSVar$ HasAttackEffect\n\
         SVar:HasAttackEffect:TRUE",
    );
    assert!(
        ai_hint
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ModifyPT(1, 1)")),
        "{:?}",
        ai_hint.abilities
    );

    let discard_x = read_w24b(
        "Name:Discard X Cost\nManaCost:X U\nTypes:Sorcery\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Activator$ You | Type$ Spell | \
         Cost$ Discard<X/Creature/creature(s)> | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        discard_x
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("CostPart::DiscardX(")),
        "{:?}",
        discard_x.mandatory_additional_costs
    );
}

#[test]
fn wave24b_refuses_engine_gaps_and_non_paper_scripts() {
    for bad in [
        // `HandChosen` without `ChooseOpponent` enter modifier.
        "Name:BadHandChosen\nManaCost:2 B\nTypes:Creature\nPT:5/5\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ -X | AddToughness$ -X\n\
         SVar:X:Count$ValidHand Card.ChosenCtrl",
        // `Count$ValidExile Card.OppOwn` (2HG team semantics: Not(OwnedByYou) != OppOwn).
        "Name:Umbris Shape\nManaCost:3 U B\nTypes:Creature\nPT:1/1\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$ValidExile Card.OppOwn",
        // `AmountCompare` with all-players-max SVar (`PlayerCountPlayers$HighestCardsInHand`).
        "Name:AllPlayersHighestHand Shape\nManaCost:4 G\nTypes:Creature\nPT:4/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 3 | AddToughness$ 3 | \
         CheckSVar$ X | SVarCompare$ GTY\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:PlayerCountPlayers$HighestCardsInHand",
        // `RaiseCost` `Discard<X/...>` without `X` in ManaCost / `Count$xPaid`.
        "Name:BadDiscardXNoX\nManaCost:2 U\nTypes:Sorcery\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Activator$ You | Type$ Spell | \
         Cost$ Discard<X/Creature> | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ 1",
        // `RaiseCost` `Discard<X/Random>` (random discard not supported by `DiscardX`).
        "Name:BadDiscardXRandom\nManaCost:X R\nTypes:Sorcery\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Activator$ You | Type$ Spell | \
         Cost$ Discard<X/Random> | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$xPaid",
        // `RaiseCost` `ValidSpell$ Activated.ManaAbility | EffectZone$ All` (Thran Portal).
        "Name:Thran Portal Shape\nTypes:Land\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | ValidSpell$ Activated.ManaAbility | \
         Cost$ PayLife<1> | EffectZone$ All",
        // `AffectedZone$ Library` / `Graveyard` `MayPlay$ True` (engine/DSL gaps).
        "Name:Graveyard Mutate\nManaCost:3 B G\nTypes:Creature\nPT:4/4\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | AffectedZone$ Graveyard | \
         ValidSA$ Spell.Dash | MayPlay$ True",
        // Alchemy / Un-set / Planechase strictly refused.
        "Name:Alchemy MakeCard\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ MakeCard | Name$ Lightning Bolt",
        "Name:Alchemy ValidBattlefieldHand\nManaCost:4 U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ 1\n\
         SVar:X:Count$ValidBattlefield,Hand Artifact.YouCtrl",
        "Name:Unset Sticker\nManaCost:1 W\nTypes:Creature\nPT:2/2\nK:StickerSheet",
        "Name:Planechase\nTypes:Plane\n\
         T:Mode$ PlaneswalkedTo | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    ] {
        assert!(refused_w24b(bad), "should refuse: {bad}");
    }
}
