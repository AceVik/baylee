//! Unit tests for reading the cycling family: "when you cycle this card"
//! and "whenever a player cycles a card" (`T:Mode$ Cycled`) and typecycling
//! (`K:TypeCycling`), on made-up scripts.

use super::*;

#[test]
fn when_you_cycle_this_card_is_the_cycled_this_trigger() {
    // Renewed Faith's shape: an instant that cycles, with a "you may".
    let body = read(
        "Name:Cycled life\nManaCost:2 W\nTypes:Instant\n\
         K:Cycling:1 W\n\
         A:SP$ GainLife | LifeAmount$ 6\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ When you cycle CARDNAME, you may gain 2 life.\n\
         SVar:TrigGainLife:DB$ GainLife | LifeAmount$ 2",
    );
    let rendered = body.abilities.join("\n");
    assert!(
        rendered.contains("cost!(\"{1}{W}\", DiscardSelf)")
            && rendered.contains("zone = ActivationZone::Hand"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "triggered!(Trigger::CycledThis, &[Effect::MayDo { effects: &[Effect::gain_life(2)] }])"
        ),
        "{rendered}"
    );

    // The Gempalms' shape: a creature, no "may", a pump of a creature type.
    let body = read(
        "Name:Cycled pump\nManaCost:5 W\nTypes:Creature Wizard\nPT:3/5\n\
         K:Cycling:2 W\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigPumpAll | TriggerDescription$ When you cycle CARDNAME, Wizard creatures get +1/+1 until end of turn.\n\
         SVar:TrigPumpAll:DB$ PumpAll | ValidCards$ Creature.Wizard | NumAtt$ +1 | NumDef$ +1",
    );
    let rendered = body.abilities.join("\n");
    assert!(
        rendered.contains("triggered!(Trigger::CycledThis, &[Effect::PumpFilter"),
        "{rendered}"
    );
}

#[test]
fn whenever_a_player_cycles_a_card_is_the_cycles_trigger() {
    let script = |trigger: &str| {
        format!(
            "Name:Cycles\nManaCost:2 R\nTypes:Enchantment\n\
             T:Mode$ Cycled | {trigger} | Execute$ TrigGainLife\n\
             SVar:TrigGainLife:DB$ GainLife | LifeAmount$ 2"
        )
    };
    for (trigger, expected) in [
        // Lightning Rift, Astral Slide: "whenever a player cycles a card".
        (
            "ValidCard$ Card | TriggerZones$ Battlefield",
            "Trigger::Cycles(PlayerRel::EachPlayer)",
        ),
        (
            "ValidCard$ Card.YouOwn | TriggerZones$ Battlefield",
            "Trigger::Cycles(PlayerRel::You)",
        ),
        (
            "ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield",
            "Trigger::Cycles(PlayerRel::You)",
        ),
        (
            "ValidCard$ Card.Self | Secondary$ True",
            "Trigger::CycledThis",
        ),
    ] {
        let body = read(&script(trigger));
        assert!(
            body.abilities[0].starts_with(&format!("triggered!({expected}, ")),
            "{trigger}: {:?}",
            body.abilities
        );
    }
}

#[test]
fn every_other_cycled_trigger_is_refused_by_name() {
    let script = |trigger: &str| {
        format!(
            "Name:Cycled\nManaCost:2 W\nTypes:Enchantment\nK:Cycling:2\n\
             T:Mode$ Cycled | {trigger} | Execute$ TrigGainLife\n\
             SVar:TrigGainLife:DB$ GainLife | LifeAmount$ 2"
        )
    };
    for (trigger, expected) in [
        (
            "ValidCard$ Card.Self | TriggerZones$ Battlefield",
            "`Cycled` on `Card.Self` from the battlefield",
        ),
        // "Whenever you cycle another card".
        (
            "ValidCard$ Card.Other+YouOwn | TriggerZones$ Battlefield",
            "`Cycled` on `Card.Other+YouOwn` from the battlefield",
        ),
        ("ValidCard$ Card", "`Cycled` on `Card`"),
        (
            "ValidCard$ Card.Other | ValidPlayer$ You | TriggerZones$ Battlefield",
            "a `Cycled` trigger with `ValidPlayer$`",
        ),
        (
            "ValidCard$ Card.Self | Secondary$ False",
            "`Secondary$ False`",
        ),
        (
            "OptionalDecider$ You",
            "a `Cycled` trigger with no `ValidCard$`",
        ),
    ] {
        let parsed = parse(&script(trigger));
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "silently accepted: {trigger}"
        );
        let reason = refusal_reason(&parsed, &cats(), None).unwrap_or_default();
        assert!(
            reason.contains(expected),
            "{trigger}: expected `{expected}`, got `{reason}`"
        );
    }
}

#[test]
fn typecycling_searches_for_its_type_from_the_hand() {
    let ability = |keyword: &str| {
        let body = read(&format!(
            "Name:Typecycler\nManaCost:4 U\nTypes:Creature Wizard\nPT:3/3\nK:{keyword}"
        ));
        assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
        (body.abilities[0].clone(), body.statics)
    };
    let (basic, basic_statics) = ability("TypeCycling:Basic:2");
    assert_eq!(
        basic,
        "activated!(cost!(\"{2}\", DiscardSelf), &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::HAND], optional: false }], zone = ActivationZone::Hand)"
    );
    // A basic land card, spelled as every generated basic land search
    // spells `Land.Basic`.
    assert_eq!(
        basic_statics.trim(),
        "static SEARCH1: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC)]);"
    );
    let (island, statics) = ability("TypeCycling:Island:1 U");
    assert!(
        island.starts_with(
            "activated!(cost!(\"{1}{U}\", DiscardSelf), &[Effect::SearchLibrary { filter: &SEARCH"
        ) && statics.contains("subtypes::land::ISLAND"),
        "{island}\n{statics}"
    );
    let (wizard, statics) = ability("TypeCycling:Wizard:3");
    assert!(
        wizard.contains("finds: &[Find::HAND], optional: false")
            && statics.contains("subtypes::creature::WIZARD"),
        "{wizard}\n{statics}"
    );
    // Artifact landcycling: the fifth field is the type's printed words.
    let (artifact_land, statics) = ability("TypeCycling:Land.Artifact:2:artifact land");
    assert!(
        artifact_land.contains("Effect::SearchLibrary")
            && statics.contains("Filter::And(&[Filter::LAND, Filter::ARTIFACT])"),
        "{artifact_land}\n{statics}"
    );
}

#[test]
fn a_typecycling_line_it_cannot_say_is_refused_by_name() {
    for (keyword, expected) in [
        (
            "TypeCycling:Basic",
            "a `TypeCycling` without a type and a cost",
        ),
        (
            "TypeCycling:Basic:2:basic land:extra",
            "a `TypeCycling` with a fifth field `extra`",
        ),
        ("TypeCycling:Affinity:2", "filter base `Affinity`"),
    ] {
        let parsed = parse(&format!(
            "Name:Typecycler\nManaCost:4 U\nTypes:Creature Wizard\nPT:3/3\nK:{keyword}"
        ));
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "silently accepted: {keyword}"
        );
        let reason = refusal_reason(&parsed, &cats(), None).unwrap_or_default();
        assert!(
            reason.contains(expected),
            "{keyword}: expected `{expected}`, got `{reason}`"
        );
    }
}
