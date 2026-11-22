//! Static type lists distinguish supertypes from card types and subtypes.

use super::*;

fn continuous(change: &str) -> String {
    format!(
        "Name:Seasonal Atlas\nManaCost:2 R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Land | {change}"
    )
}

#[test]
fn static_type_lists_add_and_remove_each_supported_supertype() {
    for (word, constant) in [
        ("Basic", "BASIC"),
        ("Legendary", "LEGENDARY"),
        ("Snow", "SNOW"),
        ("Ongoing", "ONGOING"),
        ("Host", "HOST"),
    ] {
        for (key, modifier) in [
            ("AddType", "AddSupertype"),
            ("RemoveType", "RemoveSupertype"),
        ] {
            let body = read(&continuous(&format!("{key}$ {word}")));
            assert_eq!(
                body.abilities,
                [format!(
                    "static_ability!(Filter::LAND, Modifier::{modifier}(SupertypeSet::{constant}))"
                )]
            );
        }
    }
}

#[test]
fn a_static_type_list_retains_mixed_card_supertype_and_subtype_changes() {
    let body = read(&continuous("AddType$ Artifact Legendary Goblin"));
    assert_eq!(body.abilities.len(), 3);
    for modifier in [
        "Modifier::AddType(TypeSet::ARTIFACT)",
        "Modifier::AddSupertype(SupertypeSet::LEGENDARY)",
        "Modifier::AddSubtype(subtypes::creature::GOBLIN)",
    ] {
        assert!(
            body.abilities
                .iter()
                .any(|ability| ability == &format!("static_ability!(Filter::LAND, {modifier})"))
        );
    }

    let removed = read(&continuous("RemoveType$ Artifact Snow"));
    assert_eq!(removed.abilities.len(), 2);
    assert!(removed.abilities.iter().any(|ability| {
        ability == "static_ability!(Filter::LAND, Modifier::RemoveSupertype(SupertypeSet::SNOW))"
    }));
    assert!(removed.abilities.iter().any(|ability| {
        ability == "static_ability!(Filter::LAND, Modifier::RemoveType(TypeSet::ARTIFACT))"
    }));
}

#[test]
fn static_supertype_changes_still_refuse_unsupported_or_unread_rules() {
    for change in [
        "AddType$ World",
        "RemoveType$ World",
        "AddType$ Snow World",
        "RemoveType$ Snow World",
        "AddType$ UnknownSupertype",
        "RemoveType$ UnknownSupertype",
        "RemoveType$ Snow Goblin",
        "RemoveType$ Snow | Unknown$ True",
        "RemoveType$ Snow | Duration$ UntilEndOfTurn",
    ] {
        assert!(refused(&continuous(change)), "accepted {change}");
    }
}

#[test]
fn wave22d_cant_put_counter_hidden_keywords_colors_and_must_attack() {
    let counter_locks = read(
        "Name:Counter Ward\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ CantPutCounter | ValidCard$ Creature.OppCtrl | CounterType$ P1P1\n\
         S:Mode$ CantPutCounter | ValidCard$ Creature.inZoneBattlefield,Artifact.inZoneBattlefield\n\
         S:Mode$ CantPutCounter | ValidCard$ Card.Self\n\
         S:Mode$ CantPutCounter | ValidPlayer$ Player\n\
         S:Mode$ CantPutCounter | ValidPlayer$ You | CounterType$ POISON",
    );
    assert_eq!(counter_locks.abilities.len(), 5);
    assert!(counter_locks.abilities[0].contains(
        "ReplacementRule::ModifyCounterPlacement { object_filter: &Filter::OPPONENT_CREATURE, kind: Some(CounterKind::P1P1), placer: None, effect_only: false, op: CounterReplaceOp::Sub(u16::MAX) }"
    ));
    assert!(counter_locks.abilities[1].contains(
        "ReplacementRule::ModifyCounterPlacement { object_filter: &COUNTER_TARGET1, kind: None, placer: None, effect_only: false, op: CounterReplaceOp::Sub(u16::MAX) }"
    ));
    assert!(counter_locks.abilities[2].contains(
        "ReplacementRule::ModifyCounterPlacement { object_filter: &Filter::This, kind: None, placer: None, effect_only: false, op: CounterReplaceOp::Sub(u16::MAX) }"
    ));
    assert!(counter_locks.abilities[3].contains(
        "ReplacementRule::ModifyPlayerCounterPlacement { recipient: PlayerRel::EachPlayer, kind: None, placer: None, op: CounterReplaceOp::Sub(u16::MAX) }"
    ));
    assert!(counter_locks.abilities[4].contains(
        "ReplacementRule::ModifyPlayerCounterPlacement { recipient: PlayerRel::You, kind: Some(CounterKind::Poison), placer: None, op: CounterReplaceOp::Sub(u16::MAX) }"
    ));

    let keyword_and_hidden = read(
        "Name:Archetype Sentinel\nManaCost:3 W\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Creature.OppCtrl | CantHaveKeyword$ Hexproof & Trample\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddHiddenKeyword$ CARDNAME can only attack alone.\n\
         S:Mode$ Continuous | Affected$ Creature.OppCtrl | AddHiddenKeyword$ CARDNAME can't attack or block.\n\
         S:Mode$ Continuous | Affected$ Artifact | SetColor$ Colorless\n\
         S:Mode$ Continuous | Affected$ Creature | AddColor$ All\n\
         S:Mode$ MustAttack | ValidCreature$ Card.Self | Secondary$ True",
    );
    assert!(
        keyword_and_hidden.abilities[0].contains(
            "Modifier::CantHaveKeywords(KeywordSet::HEXPROOF.union(KeywordSet::TRAMPLE))"
        ),
        "{:?}",
        keyword_and_hidden.abilities
    );
    assert!(
        keyword_and_hidden.abilities[1].contains("Modifier::AttacksAloneOnly"),
        "{:?}",
        keyword_and_hidden.abilities
    );
    assert!(
        keyword_and_hidden.abilities[2].contains(
            "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))"
        ),
        "{:?}",
        keyword_and_hidden.abilities
    );
    assert!(
        keyword_and_hidden.abilities[3].contains("Modifier::SetColor(ColorSet::EMPTY)"),
        "{:?}",
        keyword_and_hidden.abilities
    );
    assert!(
        keyword_and_hidden.abilities[4].contains(
            "Modifier::AddColor(ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black, Color::Red, Color::Green]))"
        ),
        "{:?}",
        keyword_and_hidden.abilities
    );
    assert!(
        keyword_and_hidden.abilities[5].contains("Modifier::AttacksEachCombat"),
        "{:?}",
        keyword_and_hidden.abilities
    );

    let conditional_no_untap = read(
        "Name:Frost Sleeper\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/3\n\
         R:Event$ Untap | ValidCard$ Card.Self | ValidStepTurnToController$ You | Layer$ CantHappen | IsPresent$ Goblin.YouCtrl",
    );
    assert!(
        conditional_no_untap.abilities[0].contains("Modifier::DoesNotUntap")
            && conditional_no_untap.abilities[0]
                .contains("condition = Some(Condition::ControlCount("),
        "{:?}",
        conditional_no_untap.abilities
    );

    for bad in [
        "Name:BadGyCounter\nTypes:Enchantment\nS:Mode$ CantPutCounter | ValidCard$ Creature.inZoneGraveyard",
        "Name:BadCondCounter\nTypes:Enchantment\nS:Mode$ CantPutCounter | ValidCard$ Creature | IsPresent$ Island.YouCtrl",
        "Name:BadBothCounter\nTypes:Enchantment\nS:Mode$ CantPutCounter | ValidCard$ Creature | ValidPlayer$ You",
        "Name:BadUnknownCounter\nTypes:Enchantment\nS:Mode$ CantPutCounter | ValidCard$ Creature | CounterType$ NOT_A_COUNTER",
        "Name:BadAddColorless\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature | AddColor$ Colorless",
        "Name:BadCantHaveKw\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature | CantHaveKeyword$ FakeKeyword",
        "Name:BadHiddenKw\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature | AddHiddenKeyword$ CARDNAME must be blocked if able.",
        "Name:BadSecondaryMustAttack\nTypes:Creature\nPT:2/2\nS:Mode$ MustAttack | ValidCreature$ Card.Self | Secondary$ False",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave23d_gain_activated_abilities_of_and_refusals() {
    let bf_self = read(
        "Name:Ooze Mimic\nManaCost:2 G U\nTypes:Creature Ooze\nPT:4/6\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Creature.Other+counters_GE1_P1P1\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Land\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Creature.Legendary+YouCtrl | GainsValidAbilities$ Activated\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | EffectZone$ Battlefield | GainsAbilitiesOf$ Land",
    );
    assert_eq!(bf_self.abilities.len(), 4);
    assert!(
        bf_self.abilities[0].contains("Modifier::GainActivatedAbilitiesOf { filter: &DONOR")
            && bf_self.abilities[0].contains("zone: AbilityDonorZone::Battlefield }")
            && bf_self.statics.contains("Filter::Another")
            && bf_self
                .statics
                .contains("Filter::CountersAtLeast(CounterKind::P1P1, 1)"),
        "{:?}\n{}",
        bf_self.abilities[0],
        bf_self.statics
    );
    assert_eq!(
        bf_self.abilities[1],
        "static_ability!(Filter::This, Modifier::GainActivatedAbilitiesOf { filter: &Filter::LAND, zone: AbilityDonorZone::Battlefield })"
    );
    assert!(
        bf_self.abilities[2].contains("Modifier::GainActivatedAbilitiesOf { filter: &DONOR")
            && bf_self.abilities[2].contains("zone: AbilityDonorZone::Battlefield }"),
        "{:?}",
        bf_self.abilities[2]
    );
    assert_eq!(
        bf_self.abilities[3],
        "static_ability!(Filter::YOUR_CREATURE, Modifier::GainActivatedAbilitiesOf { filter: &Filter::LAND, zone: AbilityDonorZone::Battlefield })"
    );

    let gy_top_exile = read(
        "Name:Archive Mimic\nManaCost:2 B B\nTypes:Creature\nPT:4/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Creature | GainsAbilitiesOfZones$ Graveyard\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Artifact.YouOwn | GainsAbilitiesOfZones$ Graveyard\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Goblin.OppOwn | GainsAbilitiesOfZones$ Graveyard\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | TopCardOfLibraryIs$ Goblin | GainsAbilitiesOf$ Goblin.TopLibrary+YouCtrl | GainsAbilitiesOfZones$ Library\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | TopCardOfLibraryIs$ Creature,Artifact | GainsAbilitiesOf$ Creature.TopLibrary+YouCtrl,Artifact.TopLibrary+YouCtrl | GainsAbilitiesOfZones$ Library\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Card.IsImprinted+ExiledWithSource | GainsAbilitiesOfZones$ Exile\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Card.YouOwn+counters_GE1_OIL | GainsAbilitiesOfZones$ Exile\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | GainsAbilitiesOf$ Card.counters_GE1_OIL | GainsAbilitiesOfZones$ Exile",
    );
    assert_eq!(
        gy_top_exile.abilities[0],
        "static_ability!(Filter::This, Modifier::GainActivatedAbilitiesOf { filter: &Filter::CREATURE, zone: AbilityDonorZone::Graveyard(PlayerRel::EachPlayer) })"
    );
    assert_eq!(
        gy_top_exile.abilities[1],
        "static_ability!(Filter::This, Modifier::GainActivatedAbilitiesOf { filter: &Filter::ARTIFACT, zone: AbilityDonorZone::Graveyard(PlayerRel::You) })"
    );
    assert!(
        gy_top_exile.abilities[2]
            .contains("zone: AbilityDonorZone::Graveyard(PlayerRel::EachOpponent) }"),
        "{:?}",
        gy_top_exile.abilities[2]
    );
    assert!(
        gy_top_exile.abilities[3].contains("zone: AbilityDonorZone::TopOfYourLibrary }"),
        "{:?}",
        gy_top_exile.abilities[3]
    );
    assert!(
        gy_top_exile.abilities[4].contains("zone: AbilityDonorZone::TopOfYourLibrary }"),
        "{:?}",
        gy_top_exile.abilities[4]
    );
    assert_eq!(
        gy_top_exile.abilities[5],
        "static_ability!(Filter::This, Modifier::GainActivatedAbilitiesOf { filter: &Filter::Any, zone: AbilityDonorZone::LinkedExile })"
    );
    assert!(
        gy_top_exile.abilities[6].contains("zone: AbilityDonorZone::Exile(PlayerRel::You) }"),
        "{:?}",
        gy_top_exile.abilities[6]
    );
    assert!(
        gy_top_exile.abilities[7]
            .contains("zone: AbilityDonorZone::Exile(PlayerRel::EachPlayer) }"),
        "{:?}",
        gy_top_exile.abilities[7]
    );

    for bad in [
        "Name:BadLoyalty\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Planeswalker.Other | GainsValidAbilities$ Activated.Loyalty",
        "Name:BadNonMana\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Land.OppCtrl | GainsValidAbilities$ Activated.!ManaAbility",
        "Name:BadTriggers\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Card.IsImprinted+ExiledWithSource | GainsTriggerAbsOf$ Card.IsImprinted+ExiledWithSource | GainsAbilitiesOfZones$ Exile",
        "Name:BadLimit\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Card.YouOwn+counters_GE1_OIL | GainsAbilitiesOfZones$ Exile | GainsAbilitiesLimitPerTurn$ 1",
        "Name:BadManaConv\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Creature.OppCtrl | GainsValidAbilities$ Activated | ManaConversion$ AnyType->AnyColor",
        "Name:BadUnimprintedExile\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Creature.ExiledWithSource | GainsAbilitiesOfZones$ Exile",
        "Name:BadOtherNonSelf\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | GainsAbilitiesOf$ Creature.Other",
        "Name:BadGyCtrl\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Creature.YouCtrl | GainsAbilitiesOfZones$ Graveyard",
        "Name:BadTopMismatch\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | TopCardOfLibraryIs$ Wizard | GainsAbilitiesOf$ Goblin.TopLibrary+YouCtrl | GainsAbilitiesOfZones$ Library",
        "Name:BadLibNoTop\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Goblin.YouCtrl | GainsAbilitiesOfZones$ Library",
        "Name:BadEffectZoneGy\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Graveyard | GainsAbilitiesOf$ Creature",
        "Name:BadDonorZoneHand\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Creature | GainsAbilitiesOfZones$ Hand",
        "Name:BadSharesName\nTypes:Creature\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | GainsAbilitiesOf$ Creature.YouCtrl+!sharesNameWith Self",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave23d_may_cast_with_alt_cost_and_secondary_untap() {
    let grants = read(
        "Name:Aether Conduit\nManaCost:3 U\nTypes:Enchantment\n\
         S:Mode$ Continuous | MayPlay$ True | MayPlayAltManaCost$ 0 | MayPlayLimit$ 1 | MayPlayDontGrantZonePermissions$ True | Affected$ Card.nonLand+cmcLEX | AffectedZone$ Hand,Graveyard,Library,Exile,Command\n\
         S:Mode$ Continuous | Affected$ Card.nonLand+cmcLEY | MayPlay$ True | MayPlayWithoutManaCost$ True | AffectedZone$ Hand,Graveyard,Exile,Command,Library | MayPlayDontGrantZonePermissions$ True\n\
         S:Mode$ Continuous | Affected$ Creature.cmcLE3 | MayPlay$ True | MayPlayPlayer$ Player | MayPlayWithoutManaCost$ True | MayPlayWithFlash$ True | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Library,Exile,Command\n\
         S:Mode$ Continuous | Affected$ Card.nonLand+YouOwn | MayPlay$ True | MayPlayWithoutManaCost$ True | AffectedZone$ Hand\n\
         S:Mode$ Continuous | Condition$ PlayerTurn | Affected$ Card.Enchantment+YouCtrl+nonLand | MayPlayLimit$ 1 | MayPlay$ True | MayPlayAltManaCost$ PayLife<ConvertedManaCost> | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Exile,Library,Command\n\
         S:Mode$ Continuous | MayPlay$ True | MayPlayAltManaCost$ 1 U | MayPlayLimit$ 1 | MayPlayDontGrantZonePermissions$ True | Affected$ Card.YouCtrl+nonLand | AffectedZone$ Exile\n\
         S:Mode$ Continuous | EffectZone$ Command | Affected$ Card.nonLand+YouOwn | AffectedZone$ Hand | MayPlay$ True | MayPlayWithoutManaCost$ True\n\
         S:Mode$ Continuous | Affected$ Card.IsImprinted+ExiledWithSource | AffectedZone$ Exile | MayPlay$ True | MayPlayWithoutManaCost$ True\n\
         S:Mode$ Continuous | Affected$ Card.nonLand | AffectedZone$ Hand,Exile | MayPlay$ True | MayPlayDontGrantZonePermissions$ True | MayPlayAltManaCost$ 2\n\
         SVar:X:Count$CardCounters.TIME\n\
         SVar:Y:Count$Valid Land.YouCtrl",
    );
    assert_eq!(grants.abilities.len(), 9);
    assert_eq!(
        grants.abilities[0],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::AnyPermitted, grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: Some(Amount::CountersOnSource(CounterKind::Time)), once_per_turn: true, price: AltCastPrice::Mana(ManaCost::ZERO) })"
    );
    assert_eq!(
        grants.abilities[1],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::AnyPermitted, grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: Some(Amount::CountOf { filter: &Filter::YOUR_LAND, zone: ZoneSel::Battlefield }), once_per_turn: false, price: AltCastPrice::Mana(ManaCost::ZERO) })"
    );
    assert_eq!(
        grants.abilities[2],
        "static_ability!(Filter::CREATURE, Modifier::MayCastWithAltCost { who: PlayerRel::EachPlayer, zones: AltCastZones::AnyPermitted, grant_zone_permission: false, allow_lands: false, with_flash: true, max_mana_value: Some(Amount::Fixed(3)), once_per_turn: false, price: AltCastPrice::Mana(ManaCost::ZERO) })"
    );
    assert_eq!(
        grants.abilities[3],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Hand]), grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Mana(ManaCost::ZERO) })"
    );
    assert_eq!(
        grants.abilities[4],
        "static_ability!(Filter::ENCHANTMENT, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::AnyPermitted, grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: true, price: AltCastPrice::PayLifeEqualToManaValue }, condition = Some(Condition::YourTurn))"
    );
    assert_eq!(
        grants.abilities[5],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Exile]), grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: true, price: AltCastPrice::Mana(mana!(\"{1}{U}\")) })"
    );
    assert_eq!(
        grants.abilities[6],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Hand]), grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Mana(ManaCost::ZERO) }, zone = ZoneRef::Command)"
    );
    assert_eq!(
        grants.abilities[7],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::LinkedExile, grant_zone_permission: true, allow_lands: true, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Mana(ManaCost::ZERO) })"
    );
    assert_eq!(
        grants.abilities[8],
        "static_ability!(Filter::Any, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Hand, ZoneRef::Exile]), grant_zone_permission: false, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Mana(mana!(\"{2}\")) })"
    );

    let untap_secondary = read(
        "Name:Frost Clamp\nManaCost:2 U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         R:Event$ Untap | ValidCard$ Card.EnchantedBy | ValidStepTurnToController$ You | Layer$ CantHappen | Secondary$ True",
    );
    assert!(
        untap_secondary
            .abilities
            .iter()
            .any(|a| a == "static_ability!(Filter::AttachedToBySource, Modifier::DoesNotUntap)"),
        "{:?}",
        untap_secondary.abilities
    );

    for bad in [
        "Name:BadEvidence\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.YouCtrl+nonLand | MayPlay$ True | MayPlayAltManaCost$ CollectEvidence<10> | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Exile,Library,Command",
        "Name:BadEnergy\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.YouCtrl+nonLand | MayPlay$ True | MayPlayAltManaCost$ PayEnergy<8> | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Exile,Library,Command",
        "Name:BadCmcMana\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand | MayPlay$ True | MayPlayAltManaCost$ ConvertedManaCost | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Exile,Library,Command",
        "Name:BadLimit2\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+YouOwn | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayLimit$ 2 | AffectedZone$ Hand",
        "Name:BadPlayerActive\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand | MayPlayPlayer$ Player.Active | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Hand,Graveyard,Exile,Library,Command",
        "Name:BadZoneAll\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayDontGrantZonePermissions$ True | AffectedZone$ All",
        "Name:BadLibHand\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+TopLibrary+YouCtrl,Card.nonLand+YouOwn | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayDontGrantZonePermissions$ True | AffectedZone$ Library,Hand",
        "Name:BadOppHand\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+OppOwn | MayPlay$ True | MayPlayWithoutManaCost$ True | AffectedZone$ Hand",
        "Name:BadEachPlayerHandNoFlag\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand | MayPlayPlayer$ Player | MayPlay$ True | MayPlayWithoutManaCost$ True | AffectedZone$ Hand",
        "Name:BadLandAndLimit\nTypes:Artifact\nS:Mode$ Continuous | Affected$ Card.IsImprinted+ExiledWithSource | AffectedZone$ Exile | MayPlay$ True | MayPlayLimit$ 1 | MayPlayWithoutManaCost$ True",
        "Name:BadUnimprintedAltExile\nTypes:Artifact\nS:Mode$ Continuous | Affected$ Card.ExiledWithSource | AffectedZone$ Exile | MayPlay$ True | MayPlayWithoutManaCost$ True",
        "Name:BadEffectZoneAll\nTypes:Instant\nS:Mode$ Continuous | EffectZone$ All | Affected$ Card.Self | AffectedZone$ Hand,Graveyard,Exile,Library,Command | MayPlay$ True | MayPlayAltManaCost$ 0 | MayPlayDontGrantZonePermissions$ True\nA:SP$ Draw | NumCards$ 1",
        "Name:BadIgnoreColor\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+YouOwn | AffectedZone$ Hand | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayIgnoreColor$ True",
        "Name:BadBothPrices\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+YouOwn | AffectedZone$ Hand | MayPlay$ True | MayPlayAltManaCost$ 0 | MayPlayWithoutManaCost$ True",
        "Name:BadFreeFalse\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Card.nonLand+YouOwn | AffectedZone$ Hand | MayPlay$ True | MayPlayWithoutManaCost$ False",
        "Name:BadSecondaryUntap\nTypes:Artifact\nR:Event$ Untap | ValidCard$ Card.Self | ValidStepTurnToController$ You | Layer$ CantHappen | Secondary$ False",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}
