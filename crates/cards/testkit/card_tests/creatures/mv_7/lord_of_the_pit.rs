//! `cards/creatures/mv_7/lord_of_the_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lord of the Pit — "At the beginning of your upkeep, sacrifice a creature
/// other than this creature. If you can't, this creature deals 7 damage to
/// you." With another creature the seat must give it up, and the Lord is not
/// among the choices; with none the Lord deals 7 to its controller.
#[test]
fn lord_of_the_pit_eats_another_creature_or_deals_seven_to_you() {
    let p0 = PlayerId::new(0);
    let lord = card_index("ea152809-85a2-4fde-8251-3b1f267e4443");

    let mut engine = Duel::new(96, basic_forest())
        .battlefield(0, &[lord, quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, lord).expect("the Lord is out");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("so is the Elf");
    let start = engine.state().players[0].life;
    let (asked, options) = picks_before_main(&mut engine, p0).expect("the upkeep asks");
    assert_eq!(asked, p0);
    assert_eq!(options, vec![elf], "a creature other than the Lord");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert_eq!(picks_before_main(&mut engine, p0), None);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(demon).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().players[0].life,
        start,
        "no damage once it ate"
    );

    let mut engine = Duel::new(96, basic_forest())
        .battlefield(0, &[lord])
        .start();
    keep_mulligans(&mut engine);
    let start = engine.state().players[0].life;
    assert_eq!(
        picks_before_main(&mut engine, p0),
        None,
        "nothing to ask about"
    );
    assert_eq!(
        engine.state().players[0].life,
        start - 7,
        "it can't, so 7 to you"
    );
}

/// The Lord's body, which its upkeep test never reads: a black 7/7 Demon with
/// flying and trample (and nothing else), standing on the battlefield.
#[test]
fn lord_of_the_pit_is_a_seven_seven_flying_trampling_demon() {
    let p0 = PlayerId::new(0);
    let lord = card_index("ea152809-85a2-4fde-8251-3b1f267e4443");
    let mut engine = Duel::new(96, basic_forest())
        .battlefield(0, &[lord])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, lord).expect("the Lord is out");

    let c = engine.state().object(demon).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(7), Some(7)));
    assert_eq!(
        c.keywords,
        KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
        "flying and trample, no more"
    );
    assert_eq!(c.types, TypeSet::CREATURE);
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::DEMON)
            && c.subtypes.iter().count() == 1,
        "a Demon"
    );
    assert_eq!(
        c.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "black, from its {{4}}{{B}}{{B}}{{B}}"
    );
}
