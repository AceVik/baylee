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
        "RemoveTypes$ Snow | SetName$ Foo",
        "RemoveTypes$ Snow | RemoveAllAbilities$ False",
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

#[test]
fn animate_remove_card_types_emits_become_types_and_preserves_cr_guards() {
    let witness = read(
        "Name:Starbound Witness\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True | RemoveAllAbilities$ True",
    );
    contains(
        &witness,
        "Effect::continuous(&Filter::This, Modifier::BecomeTypes { types: TypeSet::ENCHANTMENT, subtypes: &[] }, Duration::UntilEndOfTurn)",
    );
    contains(&witness, "Modifier::LoseAllAbilities");

    let twotype = read(
        "Name:Arcane Forging\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Permanent | Types$ Creature,Goblin,Wizard | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True | Power$ 3 | Toughness$ 3",
    );
    contains(
        &twotype,
        "Modifier::BecomeTypes { types: TypeSet::CREATURE, subtypes: &[subtypes::creature::GOBLIN, subtypes::creature::WIZARD] }",
    );
    contains(&twotype, "Modifier::SetPT(3, 3)");

    let legend_goblin = read(
        "Name:Royal Crown\nManaCost:3 R\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Creature,Legendary,Goblin | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True | Power$ 4 | Toughness$ 4",
    );
    contains(
        &legend_goblin,
        "Modifier::BecomeTypes { types: TypeSet::CREATURE, subtypes: &[subtypes::creature::GOBLIN] }",
    );
    contains(
        &legend_goblin,
        "Modifier::AddSupertype(SupertypeSet::LEGENDARY)",
    );

    let clue = read(
        "Name:Clue Transmute\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Permanent | Types$ Artifact,Clue | RemoveCardTypes$ True",
    );
    contains(
        &clue,
        "Modifier::BecomeTypes { types: TypeSet::ARTIFACT, subtypes: &[subtypes::artifact::CLUE] }",
    );

    let full_strip = read(
        "Name:Animated Evidence\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Permanent | Types$ Artifact,Creature,Clue | \
         RemoveCardTypes$ True | RemoveSubTypes$ True | Power$ 2 | Toughness$ 2",
    );
    contains(
        &full_strip,
        "Modifier::BecomeTypes { types: TypeSet::ARTIFACT.union(TypeSet::CREATURE), subtypes: &[subtypes::artifact::CLUE] }",
    );

    for bad in [
        "Name:MixedNoStrip\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Artifact,Creature,Goblin | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True",
        "Name:NoCardType\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Goblin | RemoveCardTypes$ True",
        "Name:MismatchedSubtype\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment,Goblin | \
         RemoveCardTypes$ True | RemoveSubTypes$ True",
        "Name:DropSuper\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment | \
         RemoveCardTypes$ True | RemoveSupertypes$ True",
        "Name:KeepSuperFalse\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment | \
         RemoveCardTypes$ True | KeepSupertypes$ False",
        "Name:RemoveAllPlusRemoveKw\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment | \
         RemoveCardTypes$ True | RemoveAllAbilities$ True | RemoveKeywords$ Flying",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }

    let strip_then_grant = read(
        "Name:RemoveAllThenGrant\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature | Types$ Enchantment | \
         RemoveCardTypes$ True | RemoveAllAbilities$ True | Keywords$ Flying",
    );
    let lose_pos = strip_then_grant.abilities[0]
        .find("Modifier::LoseAllAbilities")
        .expect("LoseAllAbilities emitted");
    let kw_pos = strip_then_grant.abilities[0]
        .find("Modifier::AddKeyword(KeywordSet::FLYING)")
        .expect("AddKeyword emitted");
    assert!(
        lose_pos < kw_pos,
        "LoseAllAbilities must precede granted keywords (CR 613.7): {}",
        strip_then_grant.abilities[0]
    );
}

#[test]
fn wave22d_animate_durations_flags_dynamic_pt_player_all_and_static_grants() {
    let flags_and_durations = read(
        "Name:Shifting Relic\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | ValidTgts$ Creature | IsCurse$ True | Optional$ True | OptionQuestion$ Shrink? | \
         Duration$ UntilYourNextTurn | Power$ 1 | HiddenKeywords$ CARDNAME can't block.\n\
         A:AB$ Animate | Cost$ 2 T | ValidTgts$ Artifact.YouCtrl | Duration$ UntilUntaps | Toughness$ 5 | AddAllCreatureTypes$ True\n\
         A:AB$ Animate | Cost$ 3 | ValidTgts$ Land.YouCtrl | Duration$ AsLongAsControl | Types$ Creature | Power$ 2 | Toughness$ 2\n\
         A:AB$ Animate | Cost$ 4 | ValidTgts$ Land.YouCtrl | Duration$ AsLongAsInPlay | Types$ Creature | Power$ 3 | Toughness$ 3",
    );
    assert!(
        flags_and_durations.abilities[0].contains("Effect::MayDo { effects: &[")
            && flags_and_durations.abilities[0].contains("Duration::UntilYourNextTurn")
            && flags_and_durations.abilities[0].contains("Modifier::SetPower(1)")
            && flags_and_durations.abilities[0]
                .contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
        "{:?}",
        flags_and_durations.abilities[0]
    );
    assert!(
        flags_and_durations.abilities[1]
            .contains("Duration::WhileSourceTapped { control: false, power_limited: false }")
            && flags_and_durations.abilities[1].contains("Modifier::SetToughness(5)")
            && flags_and_durations.abilities[1].contains("Modifier::AllCreatureTypes"),
        "{:?}",
        flags_and_durations.abilities[1]
    );
    assert!(
        flags_and_durations.abilities[2].contains("Duration::WhileYouControlSource"),
        "{:?}",
        flags_and_durations.abilities[2]
    );
    assert!(
        flags_and_durations.abilities[3].contains("Duration::WhileSourceOnBattlefield"),
        "{:?}",
        flags_and_durations.abilities[3]
    );

    let dynamic_pt = read(
        "Name:Adaptive Golem\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ X | Defined$ Self | Types$ Creature | Power$ X | Toughness$ Y\n\
         A:AB$ Animate | Cost$ 2 | ValidTgts$ Artifact | Types$ Creature | Power$ P | Toughness$ P\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:SVar$X/Plus.1\n\
         SVar:P:Targeted$CardManaCost",
    );
    assert!(
        dynamic_pt.abilities[0].contains(
            "Effect::SetPTFilter { filter: &Filter::This, power: Amount::X, toughness: Amount::Plus { base: &Amount::X, offset: 1 }, duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        dynamic_pt.abilities[0]
    );
    assert!(
        dynamic_pt.abilities[1].contains(
            "Effect::SetPTFilter { filter: &Filter::This, power: Amount::TargetCmc, toughness: Amount::TargetCmc, duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        dynamic_pt.abilities[1]
    );

    let player_sweep = read(
        "Name:Mass Metamorph\nManaCost:1 U U\nTypes:Instant\n\
         A:SP$ AnimateAll | ValidTgts$ Player | ValidCards$ Creature.TargetedPlayerCtrl | \
         Types$ Goblin | RemoveCreatureTypes$ True | RemoveAllAbilities$ True | Colors$ Blue | OverwriteColors$ True | Power$ 1 | Toughness$ 1",
    );
    assert!(
        player_sweep.abilities[0].contains(
            "Effect::ModifyFilter { filter: &Filter::CREATURE, controlled_by: Some(PlayerRel::Chosen), modifier: Modifier::LoseAllAbilities, duration: Duration::UntilEndOfTurn }"
        ) && player_sweep.abilities[0].contains(
            "Effect::ModifyFilter { filter: &Filter::CREATURE, controlled_by: Some(PlayerRel::Chosen), modifier: Modifier::SetPT(1, 1), duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        player_sweep.abilities[0]
    );

    let player_sweep_implicit = read(
        "Name:Sudden Shrink\nManaCost:1 B B\nTypes:Instant\n\
         A:SP$ AnimateAll | ValidTgts$ Player | ValidCards$ Creature | \
         RemoveAllAbilities$ True | Power$ 0 | Toughness$ 2",
    );
    assert!(
        player_sweep_implicit.abilities[0].contains(
            "Effect::ModifyFilter { filter: &Filter::CREATURE, controlled_by: Some(PlayerRel::Chosen), modifier: Modifier::LoseAllAbilities, duration: Duration::UntilEndOfTurn }"
        ) && player_sweep_implicit.abilities[0].contains(
            "Effect::ModifyFilter { filter: &Filter::CREATURE, controlled_by: Some(PlayerRel::Chosen), modifier: Modifier::SetPT(0, 2), duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        player_sweep_implicit.abilities[0]
    );
}

#[test]
fn wave22d_animate_static_grants_and_refusals() {
    let granted_statics = read(
        "Name:Living Engine\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | Types$ Creature | staticAbilities$ UnblockableST\n\
         A:AB$ Animate | Cost$ 2 | Defined$ Self | Types$ Creature | Power$ 4 | Toughness$ 4 | staticAbilities$ IslandhomeST\n\
         A:AB$ Animate | Cost$ 3 | Defined$ Self | Types$ Creature | staticAbilities$ CounterPtST\n\
         A:AB$ Animate | Cost$ 4 | Defined$ Self | Types$ Creature | Power$ 2 | Toughness$ 2 | staticAbilities$ AuraTapST\n\
         SVar:UnblockableST:Mode$ CantBlockBy | ValidAttacker$ Creature.Self\n\
         SVar:IslandhomeST:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ controlsIsland\n\
         SVar:CounterPtST:Mode$ Continuous | AffectedDefined$ Self | SetPower$ C | SetToughness$ C\n\
         SVar:C:Count$CardCounters.CHARGE\n\
         SVar:AuraTapST:Mode$ Continuous | Affected$ Goblin.YouCtrl | AddAbility$ TapPing\n\
         SVar:TapPing:AB$ Mana | Cost$ T | Produced$ R",
    );
    assert!(
        granted_statics.abilities[0].contains("Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)"),
        "{:?}",
        granted_statics.abilities[0]
    );
    assert!(
        granted_statics.abilities[1].contains("Modifier::CantAttackUnlessDefenderControls(&"),
        "{:?}",
        granted_statics.abilities[1]
    );
    assert!(
        granted_statics.abilities[2]
            .contains("Modifier::SetPTToCount(PtCount::CountersOnThis(CounterKind::Charge))"),
        "{:?}",
        granted_statics.abilities[2]
    );
    assert!(
        granted_statics.abilities[3].contains("Modifier::GrantStatic { filter: &GRANTED_STATIC")
            && granted_statics.abilities[3].contains("Modifier::GrantActivated"),
        "{:?}",
        granted_statics.abilities[3]
    );

    for bad in [
        "Name:BadSingleDynamic\nManaCost:X U\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Creature | Power$ X\nSVar:X:Count$xPaid",
        "Name:BadHiddenMustBlock\nManaCost:1 R\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Creature | HiddenKeywords$ CARDNAME must be blocked if able.",
        "Name:BadSweepMixedCtrl\nManaCost:2 U\nTypes:Instant\nA:SP$ AnimateAll | ValidTgts$ Player | ValidCards$ Creature.TargetedPlayerCtrl,Artifact | Power$ 1 | Toughness$ 1",
        "Name:BadSweepWrongCtrl\nManaCost:2 U\nTypes:Instant\nA:SP$ AnimateAll | ValidTgts$ Player | ValidCards$ Creature.YouCtrl | Power$ 1 | Toughness$ 1",
        "Name:BadTargetedStaticGrant\nManaCost:1 U\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Creature | staticAbilities$ U\nSVar:U:Mode$ CantBlockBy | ValidAttacker$ Creature.Self",
        "Name:BadUnlessMonarch\nManaCost:2 U\nTypes:Artifact\nA:AB$ Animate | Cost$ 1 | Defined$ Self | Types$ Creature | Power$ 2 | Toughness$ 2 | staticAbilities$ M\nSVar:M:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ isMonarch",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave23d_animate_qualified_subjects_valid_filter_and_general_static_grants() {
    let conditional_target = read(
        "Name:Shaping Binder\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/2\n\
         A:AB$ Animate | Cost$ 1 U | ValidTgts$ Artifact,Creature | RemoveAllAbilities$ True | Duration$ UntilYourNextTurn | SubAbility$ DBShrink\n\
         A:AB$ Animate | Cost$ 2 U | ValidTgts$ Artifact.YouCtrl | Types$ Artifact,Creature | SubAbility$ DBNonGoblin\n\
         SVar:DBShrink:DB$ Animate | Defined$ Targeted.Creature | Power$ 2 | Toughness$ 2 | Duration$ UntilYourNextTurn\n\
         SVar:DBNonGoblin:DB$ Animate | Defined$ Targeted.nonCreature+nonGoblin | Power$ 1 | Toughness$ 1 | IfDesc$ True",
    );
    assert!(
        conditional_target.abilities[0].contains(
            "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: &[Effect::continuous(&Filter::This, Modifier::SetPT(2, 2), Duration::UntilYourNextTurn)] }"
        ),
        "{:?}",
        conditional_target.abilities[0]
    );
    assert!(
        conditional_target.abilities[1].contains(
            "Effect::IfTargetMatches { filter: &ANIMATE_CHECK1, then: &[Effect::continuous(&Filter::This, Modifier::SetPT(1, 1), Duration::UntilEndOfTurn)] }"
        ),
        "{:?}\n{}",
        conditional_target.abilities[1],
        conditional_target.statics
    );

    let conditional_self_and_statics = read(
        "Name:Living Plate\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 2 | Defined$ Self.nonCreature | Power$ 0 | Toughness$ 0 | Types$ Creature,Artifact,Goblin | RemoveCreatureTypes$ True | Keywords$ Flying | staticAbilities$ StaticPump\n\
         A:AB$ Animate | Cost$ 3 | Defined$ Self | Types$ Creature,Wizard | staticAbilities$ GravePt\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | staticAbilities$ CanAttack | Duration$ Permanent\n\
         A:AB$ AnimateAll | Cost$ 4 | ValidCards$ Creature.YouCtrl | staticAbilities$ Unblocked\n\
         A:AB$ PutCounter | Cost$ 2 G | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBMatchPower\n\
         SVar:StaticPump:Mode$ Continuous | Affected$ Card.Self | AddPower$ X | AddToughness$ X\n\
         SVar:GravePt:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Battlefield | SetPower$ Y | SetToughness$ Y\n\
         SVar:CanAttack:Mode$ CanAttackDefender | ValidCard$ Card.Self\n\
         SVar:Unblocked:Mode$ AssignCombatDamageAsUnblocked | ValidCard$ Card.Self | Optional$ True\n\
         SVar:DBMatchPower:DB$ Animate | Power$ P | Toughness$ P | Defined$ Valid Creature.YouCtrl+Other | Optional$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n\
         SVar:Y:Count$ValidGraveyard Creature.YouOwn\n\
         SVar:P:Count$CardPower",
    );
    assert!(
        conditional_self_and_statics.abilities[0]
            .contains("Effect::IfCondition { condition: Condition::SourceMatches(&Filter::NONCREATURE), then: &[")
            && conditional_self_and_statics.abilities[0].contains(
                "Modifier::GrantStatic { filter: &Filter::This, modifier: &Modifier::ModifyPTByCount(PtCount::YouControl(&Filter::YOUR_ARTIFACT)) }"
            ),
        "{:?}",
        conditional_self_and_statics.abilities[0]
    );
    assert!(
        conditional_self_and_statics.abilities[1].contains(
            "Modifier::GrantStatic { filter: &Filter::This, modifier: &Modifier::SetPTToCount(PtCount::InGraveyards { who: PlayerRel::You, filter: &Filter::CREATURE }) }"
        ),
        "{:?}",
        conditional_self_and_statics.abilities[1]
    );
    assert!(
        conditional_self_and_statics.abilities[2].contains(
            "Modifier::GrantStatic { filter: &Filter::This, modifier: &Modifier::AttacksDespiteDefender }"
        ),
        "{:?}",
        conditional_self_and_statics.abilities[2]
    );
    assert!(
        conditional_self_and_statics.abilities[3].contains(
            "Modifier::GrantStatic { filter: &Filter::This, modifier: &Modifier::MayAssignCombatDamageAsThoughUnblocked }"
        ),
        "{:?}",
        conditional_self_and_statics.abilities[3]
    );
    assert!(
        conditional_self_and_statics.abilities[4]
            .contains("Effect::MayDo { effects: &[Effect::SetPTFilter { filter: &ANIMATE_ALL")
            && conditional_self_and_statics.abilities[4]
                .contains("power: Amount::SourcePower, toughness: Amount::SourcePower"),
        "{:?}",
        conditional_self_and_statics.abilities[4]
    );

    for bad in [
        "Name:BadPlayerQual\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | ValidTgts$ Player | NumCards$ 1 | SubAbility$ DBAnim\nSVar:DBAnim:DB$ Animate | Defined$ Targeted.Creature | Power$ 2 | Toughness$ 2",
        "Name:BadUntargetedQual\nManaCost:1 U\nTypes:Artifact\nA:AB$ Animate | Cost$ 1 | Defined$ Targeted.Creature | Power$ 2 | Toughness$ 2",
        "Name:BadRelativeQual\nManaCost:1 U\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Creature | Keywords$ Flying | SubAbility$ DBAnim\nSVar:DBAnim:DB$ Animate | Defined$ Targeted.Other | Power$ 2 | Toughness$ 2",
        "Name:BadSelfQualOnSpell\nManaCost:1 U\nTypes:Instant\nA:SP$ Animate | Defined$ Self.nonCreature | Power$ 2 | Toughness$ 2",
        "Name:BadIfDescFalse\nManaCost:1 U\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Artifact | Keywords$ Flying | SubAbility$ DBAnim\nSVar:DBAnim:DB$ Animate | Defined$ Targeted.nonCreature | Power$ 1 | Toughness$ 1 | IfDesc$ False",
        "Name:BadConditionalStaticGrant\nManaCost:2 R\nTypes:Land\nA:AB$ Animate | Cost$ 1 R | Defined$ Self | Power$ 2 | Toughness$ 1 | Types$ Creature | staticAbilities$ ST\nSVar:ST:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ First Strike | Condition$ PlayerTurn",
        "Name:BadValidGraveyard\nManaCost:2 B\nTypes:Artifact\nA:AB$ Animate | Cost$ 1 | Defined$ ValidGraveyard Creature.YouOwn | staticAbilities$ Play\nSVar:Play:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}
