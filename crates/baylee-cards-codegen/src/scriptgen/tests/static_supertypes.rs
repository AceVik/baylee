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
