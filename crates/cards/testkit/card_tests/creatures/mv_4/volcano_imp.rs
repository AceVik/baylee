//! `cards/creatures/mv_4/volcano_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volcano Imp prints two lines: flying, and "{1}{R}: This creature gains first
/// strike until end of turn." Neither the keyword nor the price is visible in
/// the card file, so the board plays both: the Imp is seated untapped with two
/// Mountains behind it, and the first strike is only bought once the mana is
/// really in the pool, because `can_afford` reads the pool and not the untapped
/// lands. Walking a whole turn afterwards is what tells the printed "until end
/// of turn" from a body that keeps the keyword, and the flying is re-read there
/// so that the pump — and not the creature — is what expired.
#[test]
fn volcano_imp_flies_and_buys_itself_first_strike_for_a_red_and_a_generic() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(211, mountain())
        .battlefield(0, &[volcano_imp(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let imp = on_battlefield(&engine, p0, volcano_imp()).expect("the Imp is on the table");
    assert_eq!(pt(&engine, imp), (2, 2), "the body the card prints");
    let printed = keywords(&engine, imp);
    assert!(
        printed.contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent: {printed:?}"
    );
    assert!(
        !printed.contains(KeywordSet::FIRST_STRIKE),
        "nothing has bought first strike yet"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the *pool* rather than the two untapped Mountains: with nothing
    // floating the {1}{R} is unpayable and the line is not there at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(imp, 0)),
        "{{1}}{{R}} is not two mana on the board, so nothing is offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, and the Imp taps for no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(imp, 0)),
        "with two mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed "{1}{R}: This creature gains first strike until
    // end of turn." A self-pump may still arrive as a target question with a
    // menu of one, so both shapes are driven here; either way the answer is the
    // Imp itself, and both prices are read after it is given (CR 601.2c before
    // CR 601.2h).
    activate(&mut engine, p0, volcano_imp(), 0);
    for _ in 0..16 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert!(
                    options.contains(&imp),
                    "the Imp is the creature its own pump is about: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![imp] })
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the pump resolves: {other:?}"),
        }
    }
    assert!(
        at_rest(&engine, p0),
        "the pump resolves back to a quiet priority, got {:?}",
        engine.pending()
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
    let armed = keywords(&engine, imp);
    assert!(
        armed.contains(KeywordSet::FIRST_STRIKE),
        "the Imp bought the first strike its ability prints: {armed:?}"
    );
    assert!(
        armed.contains(KeywordSet::FLYING),
        "and keeps the flying it was printed with: {armed:?}"
    );

    // "until end of turn": a turn later the keyword is gone and the creature is
    // still standing, so the grant was a duration and not a body.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, volcano_imp()).is_some(),
        "the Imp survived the turn it was pumped in"
    );
    let after = keywords(&engine, imp);
    assert!(
        !after.contains(KeywordSet::FIRST_STRIKE),
        "the grant lasted the turn it was made in and no longer: {after:?}"
    );
    assert!(
        after.contains(KeywordSet::FLYING),
        "while the printed flying, which no duration touches, is still on it: {after:?}"
    );
}
