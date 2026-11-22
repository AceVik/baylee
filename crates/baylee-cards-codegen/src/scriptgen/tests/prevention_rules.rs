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

#[test]
fn source_only_damage_prevention_preserves_command_and_battlefield_zone_boundaries() {
    let command_combat = read(
        "Name:WinterExpanse\nTypes:Plane Skybreen\n\
         R:Event$ DamageDone | ActiveZones$ Command | Prevent$ True | IsCombat$ True | ValidSource$ Creature.nonGoblin",
    );
    assert!(
        command_combat.abilities[0].contains("Modifier::PreventDamageFromIt")
            && command_combat.abilities[0].contains("zone = ZoneRef::Command"),
        "{:?}",
        command_combat.abilities
    );

    let command_self = read(
        "Name:QuietHalo\nTypes:Plane Skybreen\n\
         R:Event$ DamageDone | ActiveZones$ Command | Prevent$ True | ValidSource$ Card.Self",
    );
    assert!(
        command_self.abilities[0].contains("Modifier::PreventAllDamageFromIt")
            && command_self.abilities[0].contains("zone = ZoneRef::Command"),
        "{:?}",
        command_self.abilities
    );

    assert!(refused(
        "Name:BadNoncombat\nTypes:Plane Skybreen\n\
         R:Event$ DamageDone | ActiveZones$ Command | Prevent$ True | IsCombat$ False | ValidSource$ Card.Self"
    ));
}

#[test]
fn counter_damage_prevention_and_refusals() {
    let direct_self = read(
        "Name:PrismMorph\nManaCost:3\nTypes:Artifact Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | ReplaceWith$ Shed | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Shed:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | CounterNum$ D\n\
         SVar:D:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        direct_self.abilities,
        ["static_ability!(Filter::This, Modifier::CountersPreventDamage(CounterKind::P1P1))"]
    );

    let guarded_self = read(
        "Name:PrismGuardian\nManaCost:2 G\nTypes:Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_P1P1 | ReplaceWith$ Shed | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Shed:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | CounterNum$ ReplaceCount$DamageAmount",
    );
    assert_eq!(
        guarded_self.abilities,
        ["static_ability!(Filter::This, Modifier::CountersPreventDamage(CounterKind::P1P1))"]
    );

    for bad in [
        "Name:FixedOneCounter\nTypes:Creature\nPT:2/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | ReplaceWith$ Shed | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Shed:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
        "Name:KindMismatch\nTypes:Creature\nPT:2/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_STUN | ReplaceWith$ Shed | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Shed:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ ReplaceCount$DamageAmount",
        "Name:ChainedTrigger\nTypes:Creature\nPT:2/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_P1P1 | ReplaceWith$ Shed | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Shed:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | CounterNum$ ReplaceCount$DamageAmount | RememberRemoved$ True",
        "Name:PutOnDamage\nTypes:Creature\nPT:2/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | ReplaceWith$ Grow | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Grow:DB$ PutCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | CounterNum$ ReplaceCount$DamageAmount",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave22d_prevention_and_static_effect_unlocks() {
    let wish_ring = read(
        "Name:Wish Band\nManaCost:5\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ 5 T Exile<1/CARDNAME> | ReplacementEffects$ NextDraw\n\
         SVar:NextDraw:Event$ Draw | ActiveZones$ Command | ValidPlayer$ You | ReplaceWith$ WishDraw\n\
         SVar:WishDraw:DB$ ChangeZone | Origin$ Sideboard | Destination$ Hand | ChangeType$ Card.YouOwn | ChangeNum$ 1 | Hidden$ True | Mandatory$ True | SubAbility$ Clean\n\
         SVar:Clean:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile",
    );
    assert!(
        wish_ring.abilities[0].contains(
            "Effect::continuous(&Filter::Any, Modifier::NextDrawWish, Duration::UntilEndOfTurn)"
        ),
        "{:?}",
        wish_ring.abilities
    );

    let etb_bonus = read(
        "Name:Simic Graftmage\nManaCost:G U\nTypes:Creature Wizard\nPT:2/2\n\
         A:AB$ Effect | Cost$ 1 G | ReplacementEffects$ BonusCounter\n\
         SVar:BonusCounter:Event$ Moved | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | ReplaceWith$ AddOne | ReplacementResult$ Updated | ActiveZones$ Command\n\
         SVar:AddOne:DB$ PutCounter | Defined$ ReplacedCard | CounterType$ P1P1 | CounterNum$ 1 | ETB$ True",
    );
    assert!(
        etb_bonus.abilities[0].contains(
            "Modifier::AdditionalCounterOnEntryByCount { kind: CounterKind::P1P1, count: PtCount::Fixed(1) }"
        ),
        "{:?}",
        etb_bonus.abilities
    );

    let guard_redirect = read(
        "Name:Royal Shield\nManaCost:3 W W\nTypes:Creature\nPT:2/5\n\
         A:AB$ Effect | Cost$ T | ReplacementEffects$ RedirectAll\n\
         SVar:RedirectAll:Event$ DamageDone | ActiveZones$ Command | IsCombat$ True | ValidSource$ Creature.unblocked | ValidTarget$ You | DamageTarget$ EffectSource | ReplaceWith$ RedirectToSelf\n\
         SVar:RedirectToSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ EffectSource | VarType$ Card",
    );
    assert!(
        guard_redirect.abilities[0].contains(
            "Effect::RedirectCombatDamageToYouThisTurn { by: &SOURCE1, to: DamageRedirectTo::ThisObject }"
        ) && guard_redirect.statics.contains("Filter::Unblocked"),
        "{}\n{:?}",
        guard_redirect.statics,
        guard_redirect.abilities
    );

    let you_and_yours = read(
        "Name:Forest Aegis\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ ShieldCombat\n\
         SVar:ShieldCombat:Event$ DamageDone | ActiveZones$ Command | IsCombat$ True | ValidTarget$ You,Creature.YouCtrl | Prevent$ True",
    );
    assert!(
        you_and_yours.abilities[0].contains(
            "Effect::PreventAllDamageFromMatchingThisTurn { target: Some(TargetSpec::Player(PlayerRel::You)), sources: &Filter::Any, combat_only: true }"
        ) && you_and_yours.abilities[0].contains(
            "Effect::continuous(&Filter::YOUR_CREATURE, Modifier::PreventDamageToIt, Duration::UntilEndOfTurn)"
        ),
        "{:?}",
        you_and_yours.abilities
    );

    let source_pacify = read(
        "Name:Trial Shield\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Permanent | RememberObjects$ Targeted | Duration$ UntilYourNextTurn | ReplacementEffects$ StopDmg\n\
         SVar:StopDmg:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Card.IsRemembered | Prevent$ True",
    );
    assert!(
        source_pacify.abilities[0].contains(
            "Effect::continuous(&Filter::This, Modifier::PreventAllDamageFromIt, Duration::UntilYourNextTurn)"
        ),
        "{:?}",
        source_pacify.abilities
    );

    let chosen_with_remembered = read(
        "Name:Sanctum Ward\nManaCost:1 W\nTypes:Enchantment\n\
         A:AB$ ChooseSource | Cost$ W PayLife<2> | Choices$ Card,Emblem | SubAbility$ Shield\n\
         SVar:Shield:DB$ Effect | RememberObjects$ Remembered | ReplacementEffects$ StopNext | SubAbility$ Clean | ConditionDefined$ ChosenCard | ConditionPresent$ Card,Emblem\n\
         SVar:StopNext:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | ValidTarget$ You | ReplaceWith$ EndShield | PreventionEffect$ True\n\
         SVar:EndShield:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:Clean:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        chosen_with_remembered.abilities[0].contains(
            "Effect::PreventNextFromChosenSource { sources: &Filter::Any, combat_only: false, all_but: 0, gain_life: false }"
        ),
        "{:?}",
        chosen_with_remembered.abilities
    );
}

#[test]
fn wave22d_defender_extra_land_emblem_and_prevention_refusals() {
    let defender_and_land = read(
        "Name:Watchful Scout\nManaCost:2 G\nTypes:Creature\nPT:0/4\nK:Defender\n\
         A:AB$ Effect | Cost$ 1 | StaticAbilities$ AttackOk | ExileOnMoved$ Battlefield\n\
         A:AB$ Effect | Cost$ 2 | Name$ Extra Land Drop | StaticAbilities$ MoreLand\n\
         SVar:AttackOk:Mode$ CanAttackDefender | ValidCard$ Card.EffectSource\n\
         SVar:MoreLand:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 1",
    );
    assert!(
        defender_and_land.abilities[0].contains("Modifier::AttacksDespiteDefender")
            && defender_and_land.abilities[1].contains("Modifier::ExtraLandDrops(1)"),
        "{:?}",
        defender_and_land.abilities
    );

    let emblem_land = read(
        "Name:EmblemLand\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Effect | Name$ Emblem — Extra Land | StaticAbilities$ MoreLand | Duration$ Permanent\n\
         SVar:MoreLand:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 1",
    );
    assert!(
        emblem_land.abilities[0].contains("Effect::CreateEmblem"),
        "Emblem extra-land effect must reach CreateEmblem, not UntilEndOfTurn: {:?}",
        emblem_land.abilities
    );

    for bad in [
        "Name:BadLandAllPlayers\nManaCost:2 G\nTypes:Sorcery\nA:SP$ Effect | Name$ Extra Land Drop | StaticAbilities$ MoreLand\nSVar:MoreLand:Mode$ Continuous | Affected$ Player | AdjustLandPlays$ 1",
        "Name:BadEtbGraveyard\nManaCost:G U\nTypes:Creature\nPT:2/2\nA:AB$ Effect | Cost$ 1 | ReplacementEffects$ BonusCounter\nSVar:BonusCounter:Event$ Moved | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | ReplaceWith$ AddOne | ReplacementResult$ Updated | ActiveZones$ Command\nSVar:AddOne:DB$ PutCounter | Defined$ ReplacedCard | CounterType$ P1P1 | CounterNum$ 1 | ETB$ True",
        "Name:BadRedirectOnSpell\nManaCost:2 W\nTypes:Instant\nA:SP$ Effect | ReplacementEffects$ RedirectAll\nSVar:RedirectAll:Event$ DamageDone | ActiveZones$ Command | IsCombat$ True | ValidSource$ Creature.unblocked | ValidTarget$ You | DamageTarget$ EffectSource | ReplaceWith$ RedirectToSelf\nSVar:RedirectToSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ EffectSource | VarType$ Card",
        "Name:BadWishNoSelfExile\nManaCost:5\nTypes:Artifact\nA:AB$ Effect | Cost$ 5 T | ReplacementEffects$ NextDraw\nSVar:NextDraw:Event$ Draw | ActiveZones$ Command | ValidPlayer$ You | ReplaceWith$ WishDraw\nSVar:WishDraw:DB$ ChangeZone | Origin$ Sideboard | Destination$ Hand | ChangeType$ Card.YouOwn | ChangeNum$ 1 | Hidden$ True | Mandatory$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}
