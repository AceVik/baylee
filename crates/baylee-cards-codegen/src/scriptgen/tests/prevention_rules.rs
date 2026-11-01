//! Original synthetic fixtures for spell-source prevention and actual damage recipients.

use super::*;

fn spell_source_prevention(source: &str, zone: &str) -> String {
    format!(
        "Name:Prismatic Quiet\nManaCost:2 W U\nTypes:Enchantment\n\
         R:Event$ DamageDone | ValidSource$ {source} | Prevent$ True{zone}"
    )
}

#[test]
fn persistent_spell_prevention_keeps_the_source_filter_on_the_stack() {
    for source in [
        "Instant",
        "Sorcery.YouCtrl",
        "Instant.Red,Sorcery.Blue+OppCtrl",
    ] {
        let source_filter = filter(source);
        for zone in ["", " | ActiveZones$ Battlefield"] {
            let body = read(&spell_source_prevention(source, zone));
            assert_eq!(
                body.abilities,
                [format!(
                    "static_ability!(Filter::And(&[Filter::InZone(ZoneRef::Stack), \
                     {source_filter}]), Modifier::PreventAllDamageFromIt)"
                )],
                "source {source}, zone {zone}"
            );
        }
    }
}

#[test]
fn persistent_spell_prevention_composes_with_existing_permanent_rules() {
    let script = format!(
        "{}\nK:Cumulative upkeep:3\n\
         R:Event$ Untap | ValidCard$ Card.Self | Layer$ CantHappen | \
         ValidStepTurnToController$ You",
        spell_source_prevention("Instant.Green,Sorcery.Green", "")
    );
    let body = read(&script);
    let abilities = body.abilities.join("");
    assert_eq!(body.abilities.len(), 3, "{abilities}");
    assert_eq!(abilities.matches("PreventAllDamageFromIt").count(), 1);
    assert!(abilities.contains("Filter::InZone(ZoneRef::Stack)"));
    assert!(abilities.contains("Modifier::DoesNotUntap"));
    assert!(abilities.contains("Amount::CountersOnSource(counters::AGE)"));
    assert!(abilities.contains("factor: 3"));
}

#[test]
fn persistent_spell_prevention_refuses_different_sources_zones_and_replacement_rules() {
    let script = spell_source_prevention("Instant.Red,Sorcery.YouCtrl", "");
    for source in [
        "Creature",
        "Card",
        "Any",
        "Instant.Red,Creature.Red",
        "Spell",
        "Instant.UnknownPredicate",
    ] {
        assert!(refused(&spell_source_prevention(source, "")), "{source}");
    }
    for zone in ["Hand", "Graveyard", "Command", "All", "Battlefield,Hand"] {
        assert!(
            refused(&spell_source_prevention(
                "Instant.Red,Sorcery.YouCtrl",
                &format!(" | ActiveZones$ {zone}")
            )),
            "{zone}"
        );
    }
    for (from, to) in [
        ("Prevent$ True", "Prevent$ False"),
        ("Prevent$ True", "Prevent$ Maybe"),
        (" | Prevent$ True", ""),
        ("ValidSource$ Instant.Red,Sorcery.YouCtrl | ", ""),
        ("Event$ DamageDone", "Event$ DamageDoneOnce"),
    ] {
        assert!(refused(&script.replace(from, to)), "{from} -> {to}");
    }
    for extra in [
        "ValidTarget$ You",
        "ValidTarget$ Creature",
        "CombatDamage$ True",
        "IsCombat$ False",
        "IsPresent$ Creature.YouCtrl",
        "CheckSVar$ Gate",
        "ReplaceWith$ Redirect",
        "ReplacementResult$ Updated",
        "Secondary$ True",
        "UnknownFlag$ True",
        "Prevent$ True",
    ] {
        assert!(refused(&format!("{script} | {extra}")), "{extra}");
    }
}

fn remembered_damage(amount: u8, targets: &str) -> String {
    format!(
        "Name:Ember Geometry\nTypes:Sorcery\nManaCost:4 R\n\
         A:SP$ DealDamage | NumDmg$ {amount} | ValidTgts$ {targets} | \
         RememberDamaged$ True | SubAbility$ SealRecipients\n\
         SVar:SealRecipients:DB$ Effect | RememberObjects$ Remembered.Creature | \
         ExileOnMoved$ Battlefield | StaticAbilities$ DenyMending | IsCurse$ True\n\
         SVar:DenyMending:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered"
    )
}

#[test]
fn remembered_damage_reads_actual_recipients_instead_of_the_announced_target() {
    // A player target can redirect damage to a creature too. The unit rider
    // delegates recipient tracking to damage resolution, without a target shield.
    for (amount, targets, target_expr) in [
        (5, "Any", "TargetSpec::AnyTarget"),
        (7, "Creature", "TargetSpec::Object("),
        (2, "Player", "TargetSpec::Player(PlayerRel::Chosen)"),
    ] {
        let script = remembered_damage(amount, targets);
        for text in [
            script.clone(),
            script.replace("ExileOnMoved$ Battlefield", "ForgetOnMoved$ Battlefield"),
            script.replace(" | IsCurse$ True", ""),
        ] {
            let body = read(&text);
            assert_eq!(body.abilities.len(), 1);
            let ability = &body.abilities[0];
            let damage = format!(
                "Effect::DealDamage {{ amount: Amount::Fixed({amount}), target: {target_expr}"
            );
            let damage_at = ability
                .find(&damage)
                .expect("the original damage is emitted");
            let rider_at = ability
                .find("Effect::DamagedThisWayCantBeRegenerated")
                .expect("actual damage recipients receive the rider");
            assert!(damage_at < rider_at, "{ability}");
            assert_eq!(ability.matches("Effect::").count(), 2, "{ability}");
            assert!(
                !ability.contains("Effect::CantBeRegeneratedThisTurn"),
                "{ability}"
            );
            assert!(!ability.contains("Effect::IfTargetMatches"), "{ability}");
            if targets == "Creature" {
                assert!(
                    format!("{}{ability}", body.statics).contains("Filter::CREATURE"),
                    "{body:?}"
                );
            }
        }
    }
}

#[test]
fn remembered_damage_accepts_only_an_optional_exact_final_memory_cleanup() {
    let plain = remembered_damage(6, "Any");
    let cleanup = format!(
        "{}\nSVar:EraseRecipients:DB$ Cleanup | ClearRemembered$ True",
        plain.replace(
            "StaticAbilities$ DenyMending",
            "StaticAbilities$ DenyMending | SubAbility$ EraseRecipients"
        )
    );
    assert_eq!(read(&plain).abilities, read(&cleanup).abilities);
    for (from, to) in [
        ("ClearRemembered$ True", "ClearRemembered$ False"),
        ("ClearRemembered$ True", "ClearRemembered$ Sometimes"),
        ("DB$ Cleanup | ClearRemembered$ True", "DB$ Cleanup"),
        ("DB$ Cleanup", "DB$ Draw"),
        ("ClearRemembered$ True", "ClearChosenCard$ True"),
        (
            "ClearRemembered$ True",
            "ClearRemembered$ True | ClearChosenCard$ True",
        ),
        (
            "ClearRemembered$ True",
            "ClearRemembered$ True | UnknownFlag$ True",
        ),
        (
            "ClearRemembered$ True",
            "ClearRemembered$ True | SubAbility$ AnotherStep",
        ),
    ] {
        assert!(refused(&cleanup.replace(from, to)), "{from} -> {to}");
    }
    let after_cleanup = format!(
        "{}\nSVar:AnotherStep:DB$ Draw | NumCards$ 1",
        cleanup.replace(
            "ClearRemembered$ True",
            "ClearRemembered$ True | SubAbility$ AnotherStep"
        )
    );
    assert!(refused(&after_cleanup), "cleanup must terminate the chain");
}

#[test]
fn later_damage_memory_cannot_capture_an_earlier_unremembered_instruction() {
    let script = remembered_damage(2, "Creature").replace(
        "A:SP$ DealDamage | NumDmg$ 2 | ValidTgts$ Creature | ",
        "A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | SubAbility$ LaterPulse\n\
         SVar:LaterPulse:DB$ DealDamage | Defined$ You | NumDmg$ 2 | ",
    );
    assert!(
        refused(&script),
        "a later memory has a narrower instruction scope"
    );
    let earlier_nondamage = remembered_damage(2, "Creature").replace(
        "A:SP$ DealDamage | NumDmg$ 2 | ValidTgts$ Creature | ",
        "A:SP$ Draw | NumCards$ 1 | SubAbility$ LaterPulse\n\
         SVar:LaterPulse:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | ",
    );
    assert!(
        refused(&earlier_nondamage),
        "this narrow reader requires the initial instruction"
    );
}

#[test]
fn remembered_damage_refuses_incomplete_or_changed_recipient_payloads() {
    let script = remembered_damage(4, "Any");
    for (from, to) in [
        ("RememberDamaged$ True", "RememberDamaged$ False"),
        ("RememberDamaged$ True", "RememberDamaged$ Maybe"),
        ("RememberDamaged$ True | ", ""),
        (" | SubAbility$ SealRecipients", ""),
        ("DB$ Effect", "DB$ Cleanup"),
        ("RememberObjects$ Remembered.Creature | ", ""),
        ("Remembered.Creature", "Remembered"),
        ("Remembered.Creature", "Remembered.Player"),
        ("Remembered.Creature", "Remembered.Creature+YouCtrl"),
        ("Remembered.Creature", "Targeted"),
        ("Remembered.Creature", "ParentTarget"),
        ("ExileOnMoved$ Battlefield | ", ""),
        ("ExileOnMoved$ Battlefield", "ExileOnMoved$ Graveyard"),
        ("ExileOnMoved$ Battlefield", "ForgetOnMoved$ Hand"),
        (
            "ExileOnMoved$ Battlefield",
            "ExileOnMoved$ Battlefield | Duration$ Permanent",
        ),
        (
            "ExileOnMoved$ Battlefield",
            "ExileOnMoved$ Battlefield | ForgetOnMoved$ Battlefield",
        ),
        (
            "StaticAbilities$ DenyMending",
            "StaticAbilities$ MissingStatic",
        ),
        (
            "StaticAbilities$ DenyMending",
            "StaticAbilities$ DenyMending,OtherStatic",
        ),
        ("StaticAbilities$ DenyMending | ", ""),
        ("IsCurse$ True", "IsCurse$ False"),
        ("Mode$ CantRegenerate", "Mode$ CantBlock"),
        ("Card.IsRemembered", "Creature.IsRemembered"),
        ("Card.IsRemembered", "Card.Self"),
        (" | ValidCard$ Card.IsRemembered", ""),
    ] {
        assert!(refused(&script.replace(from, to)), "{from} -> {to}");
    }
    for (from, extra) in [
        ("RememberDamaged$ True", "DamageMap$ False"),
        ("RememberDamaged$ True", "UnknownFlag$ True"),
        (
            "RememberObjects$ Remembered.Creature",
            "ConditionPresent$ Creature",
        ),
        (
            "RememberObjects$ Remembered.Creature",
            "RememberPlayers$ Remembered",
        ),
        ("RememberObjects$ Remembered.Creature", "UnknownFlag$ True"),
        ("ValidCard$ Card.IsRemembered", "AffectedZone$ Graveyard"),
        ("ValidCard$ Card.IsRemembered", "UnknownFlag$ True"),
    ] {
        assert!(
            refused(&script.replace(from, &format!("{from} | {extra}"))),
            "{from} with {extra}"
        );
    }
    let extra_effect = format!(
        "{}\nSVar:AnotherStep:DB$ Draw | NumCards$ 1",
        script.replace("IsCurse$ True", "IsCurse$ True | SubAbility$ AnotherStep")
    );
    assert!(refused(&extra_effect), "an unrelated trailing effect");
    assert!(refused(&script.replace(
        "StaticAbilities$ DenyMending",
        "StaticAbilities$ DenyMending | Triggers$ SideEffect"
    )));
    let multiple_statics = format!(
        "{}\nSVar:OtherStatic:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered",
        script.replace(
            "StaticAbilities$ DenyMending",
            "StaticAbilities$ DenyMending,OtherStatic"
        )
    );
    assert!(refused(&multiple_statics), "exactly one referenced static");
}
