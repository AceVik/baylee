//! `cards/creatures/mv_3/gempalm_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn gempalm_sorcerer() -> CardIndex {
    card_index("c3162232-b713-44be-b2af-29d801279ab2")
}

fn fugitive_wizard() -> CardIndex {
    card_index("381fa2a9-69fc-4558-a0ac-fb99c6f8d77f")
}

fn firdoch_core() -> CardIndex {
    card_index("126c75c0-414b-4e55-8747-2b5566ce716f")
}

/// Cycling Gempalm Sorcerer grants flying until end of turn to Wizard creatures
/// on both sides (resolving before the draw), leaves non-Wizard creatures AND
/// noncreature Kindred Wizard permanents untouched, and expires at cleanup.
#[test]
fn gempalm_sorcerer_grants_flying_to_wizard_creatures_only_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                fugitive_wizard(),
                llanowar_elves(),
                firdoch_core(),
            ],
        )
        .battlefield(1, &[fugitive_wizard()])
        .hand(0, &[gempalm_sorcerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wiz0 = on_battlefield(&engine, p0, fugitive_wizard()).expect("P0 Wizard");
    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let core = on_battlefield(&engine, p0, firdoch_core()).expect("noncreature Changeling");
    let wiz1 = on_battlefield(&engine, p1, fugitive_wizard()).expect("P1 Wizard");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gempalm_sorcerer(), 0);
    // Trigger resolves before the cycling draw.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );
    assert!(keywords(&engine, wiz0).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, wiz1).contains(KeywordSet::FLYING));
    assert!(
        !keywords(&engine, elf0).contains(KeywordSet::FLYING),
        "non-Wizard creature does not gain flying"
    );
    assert!(
        !keywords(&engine, core).contains(KeywordSet::FLYING),
        "noncreature Kindred Wizard permanent does not gain flying"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, wiz0).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, wiz1).contains(KeywordSet::FLYING));
}
