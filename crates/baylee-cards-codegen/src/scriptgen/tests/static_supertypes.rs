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
