//! General animation changes and source-relative temporary static grants.

use super::*;

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn contains(body: &CardBody, expected: &str) {
    let rendered = format!("{} {}", body.statics, body.abilities.join(" "));
    assert!(
        compact(&rendered).contains(&compact(expected)),
        "expected {expected:?}, got {rendered}"
    );
}

fn land_animation(changes: &str) -> String {
    format!(
        "Name:Carved Observatory\nTypes:Land\n\
         A:AB$ Animate | Cost$ 2 U | Defined$ Self | {changes}"
    )
}

fn source_grant(key: &str, cost: &str, extra: &str, rule: &str) -> String {
    format!(
        "Name:Ember Cartographer\nManaCost:2 G\nTypes:Creature Wizard\nPT:2/2\n\
         A:AB$ Animate | Cost$ {cost} | Defined$ Self | {key}$ MarkedRule{extra}\n\
         SVar:MarkedRule:{rule}"
    )
}

const DAMAGE_RULE: &str = "Mode$ CantRegenerate | ValidCard$ Creature.DamagedBy";
const GRANT: &str = "Modifier::GrantStatic { filter: &Filter::And(&[Filter::CREATURE, \
    Filter::DealtDamageBySourceThisTurn]), modifier: &Modifier::CantBeRegenerated }";

#[test]
fn animation_adds_each_recognized_supertype_instead_of_a_subtype() {
    for (word, constant) in [
        ("Basic", "BASIC"),
        ("Legendary", "LEGENDARY"),
        ("Snow", "SNOW"),
    ] {
        let body = read(&land_animation(&format!("Types$ {word}")));
        assert_eq!(body.abilities.len(), 1);
        contains(
            &body,
            &format!(
                "Effect::continuous(&Filter::This, Modifier::AddSupertype(SupertypeSet::{constant}), \
                 Duration::UntilEndOfTurn)"
            ),
        );
        assert!(!body.abilities[0].contains("AddSubtype"));
        assert!(!body.abilities[0].contains("AddType("));
    }
}

#[test]
fn animation_does_not_claim_unsupported_world_tenure_tracking() {
    for change in ["Types$ World", "RemoveTypes$ World"] {
        for duration in [
            "",
            " | Duration$ Permanent",
            " | Duration$ UntilEndOfCombat",
        ] {
            assert!(refused(&land_animation(&format!("{change}{duration}"))));
        }
    }
}

#[test]
fn animation_removes_each_recognized_supertype_without_changing_card_types() {
    for (word, constant) in [
        ("Basic", "BASIC"),
        ("Legendary", "LEGENDARY"),
        ("Snow", "SNOW"),
    ] {
        let body = read(&land_animation(&format!("RemoveTypes$ {word}")));
        assert_eq!(body.abilities.len(), 1);
        contains(
            &body,
            &format!(
                "Effect::continuous(&Filter::This, Modifier::RemoveSupertype(SupertypeSet::{constant}), \
                 Duration::UntilEndOfTurn)"
            ),
        );
        assert!(!body.abilities[0].contains("RemoveType("));
    }
}

#[test]
fn mixed_animation_keeps_card_subtype_and_supertype_changes() {
    let body = read(&land_animation(
        "Types$ Creature,Goblin,Legendary | RemoveTypes$ Snow",
    ));
    for modifier in [
        "Modifier::AddType(TypeSet::CREATURE)",
        "Modifier::AddSubtype(subtypes::creature::GOBLIN)",
        "Modifier::AddSupertype(SupertypeSet::LEGENDARY)",
        "Modifier::RemoveSupertype(SupertypeSet::SNOW)",
    ] {
        contains(&body, modifier);
    }
    assert_eq!(body.abilities[0].matches("Effect::continuous(").count(), 4);
}

#[test]
fn snow_and_nonsnow_target_filters_retain_their_distinct_restrictions() {
    for (valid, change, expected_filter, modifier) in [
        (
            "Land.Snow",
            "RemoveTypes$ Snow",
            "Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)])",
            "Modifier::RemoveSupertype(SupertypeSet::SNOW)",
        ),
        (
            "Land.nonSnow",
            "Types$ Snow",
            "Filter::And(&[Filter::LAND, Filter::Not(&Filter::HasSupertype(SupertypeSet::SNOW))])",
            "Modifier::AddSupertype(SupertypeSet::SNOW)",
        ),
    ] {
        let body = read(&format!(
            "Name:Seasonal Survey\nManaCost:1 R\nTypes:Instant\n\
             A:SP$ Animate | ValidTgts$ {valid} | {change}"
        ));
        contains(&body, expected_filter);
        contains(&body, modifier);
        assert!(body.abilities[0].contains("targets = Some(TargetReq::one("));
    }
}

#[test]
fn supertype_animation_honors_each_existing_explicit_duration() {
    for (duration, expected) in [
        ("Permanent", "Duration::Indefinitely"),
        ("UntilEndOfCombat", "Duration::UntilEndOfCombat"),
        ("UntilHostLeavesPlay", "Duration::WhileSourceOnBattlefield"),
    ] {
        let body = read(&land_animation(&format!(
            "Types$ Snow | Duration$ {duration}"
        )));
        contains(
            &body,
            &format!(
                "Effect::continuous(&Filter::This, Modifier::AddSupertype(SupertypeSet::SNOW), {expected})"
            ),
        );
    }
}

#[test]
fn supertype_removal_refuses_subtypes_card_types_and_unread_semantics() {
    for change in [
        "RemoveTypes$ Goblin",
        "RemoveTypes$ Creature",
        "RemoveTypes$ Snow,Goblin",
        "RemoveTypes$ Legendary,Artifact",
        "RemoveTypes$ UnknownSupertype",
        "Types$ Snow,UnknownType",
        "Types$ Snow | Duration$ NextUpkeep",
        "Types$ Snow | RememberObjects$ Self",
        "RemoveTypes$ Snow | RemoveAllAbilities$ True",
    ] {
        assert!(refused(&land_animation(change)), "accepted {change}");
    }
}

#[test]
fn either_static_reference_spelling_grants_source_relative_regeneration_prevention() {
    for (key, cost) in [("staticAbilities", "1 U"), ("StaticAbilities", "2 G")] {
        let body = read(&source_grant(key, cost, "", DAMAGE_RULE));
        assert_eq!(body.abilities.len(), 1);
        contains(
            &body,
            &format!("Effect::continuous(&Filter::This, {GRANT}, Duration::UntilEndOfTurn)"),
        );
        assert_eq!(body.abilities[0].matches("Effect::continuous(").count(), 1);
        assert!(!body.abilities[0].contains("DamagedThisWayCantBeRegenerated"));
        assert!(!body.abilities[0].contains("TargetDamagedCreatures"));
        assert!(!body.abilities[0].contains("GrantTriggered"));
        assert!(!body.abilities[0].contains("targets ="));
    }
}

#[test]
fn a_temporary_static_grant_composes_with_an_ordinary_supertype_change() {
    let body = read(&source_grant(
        "StaticAbilities",
        "R T",
        " | Types$ Snow",
        DAMAGE_RULE,
    ));
    contains(&body, GRANT);
    contains(&body, "Modifier::AddSupertype(SupertypeSet::SNOW)");
    assert_eq!(body.abilities[0].matches("Effect::continuous(").count(), 2);
}

#[test]
fn source_static_grants_preserve_supported_duration_and_refuse_unknown_duration() {
    for (duration, expected) in [
        ("Permanent", "Duration::Indefinitely"),
        ("UntilEndOfCombat", "Duration::UntilEndOfCombat"),
    ] {
        let body = read(&source_grant(
            "StaticAbilities",
            "3 W",
            &format!(" | Duration$ {duration}"),
            DAMAGE_RULE,
        ));
        contains(
            &body,
            &format!("Effect::continuous(&Filter::This, {GRANT}, {expected})"),
        );
    }
    assert!(refused(&source_grant(
        "StaticAbilities",
        "U",
        " | Duration$ UntilNextTurn",
        DAMAGE_RULE,
    )));
}

#[test]
fn damage_static_grants_refuse_other_subjects_and_independent_target_questions() {
    let source = source_grant("StaticAbilities", "1 W", "", DAMAGE_RULE);
    for (from, to) in [
        ("Defined$ Self", "Defined$ Enchanted"),
        ("Defined$ Self", "Defined$ Equipped"),
        ("Defined$ Self", "Defined$ Targeted | ValidTgts$ Creature"),
        ("Defined$ Self", "ValidTgts$ Creature"),
        ("Defined$ Self", "Defined$ Self | ValidTgts$ Artifact"),
    ] {
        assert!(refused(&source.replace(from, to)), "accepted {to}");
    }
}

#[test]
fn damage_static_grants_require_exactly_one_unambiguous_reference() {
    for extra in [
        " | StaticAbilities$ MarkedRule",
        " | staticAbilities$ MarkedRule",
    ] {
        assert!(
            refused(&source_grant("StaticAbilities", "2 R", extra, DAMAGE_RULE,)),
            "accepted duplicate {extra}"
        );
    }
    for key in ["StaticAbilities", "staticAbilities"] {
        let source = source_grant(key, "G", "", DAMAGE_RULE);
        for references in [
            "MarkedRule & SecondRule",
            "MarkedRule,SecondRule",
            "MissingRule",
        ] {
            let script = format!(
                "{}\nSVar:SecondRule:{DAMAGE_RULE}",
                source.replace("$ MarkedRule\n", &format!("$ {references}\n"))
            );
            assert!(refused(&script), "accepted {references}");
        }
    }
    assert!(refused(&source_grant(
        "staticAbilities",
        "B",
        " | StaticAbilities$ MarkedRule",
        DAMAGE_RULE,
    )));
}

#[test]
fn damage_static_grants_refuse_changed_child_meaning_or_unclaimed_parameters() {
    for rule in [
        "Mode$ CantBlock | ValidCard$ Creature.DamagedBy",
        "Mode$ CantRegenerate | ValidCard$ Card.DamagedBy",
        "Mode$ CantRegenerate | ValidCard$ Creature.DamagedBy+YouCtrl",
        "Mode$ CantRegenerate | ValidCard$ Creature.IsRemembered",
        "Mode$ CantRegenerate | ValidCard$ Card.Self",
        "Mode$ CantRegenerate | ValidCard$ Creature.DamagedBy | EffectZone$ Command",
        "Mode$ CantRegenerate | ValidCard$ Creature.DamagedBy | Affected$ Creature",
    ] {
        assert!(
            refused(&source_grant("StaticAbilities", "1 B", "", rule)),
            "accepted {rule}"
        );
    }
    for extra in [
        " | RememberObjects$ Self",
        " | ForgetOnMoved$ Battlefield",
        " | OverwriteAbilities$ True",
    ] {
        assert!(
            refused(&source_grant("staticAbilities", "3 G", extra, DAMAGE_RULE,)),
            "accepted {extra}"
        );
    }
}

#[test]
fn the_grant_only_damage_filter_is_not_accepted_as_an_incidental_spell_target() {
    assert!(refused(
        "Name:Measure the Ash\nManaCost:U\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.DamagedBy | NumDmg$ 2"
    ));
}
