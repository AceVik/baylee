//! `cards/creatures/mv_4/balshan_collaborator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Balshan Collaborator prints two lines: flying, and "{B}: This creature
/// gets +1/+1 until end of turn." Both are the engine's answer rather than the
/// card file's, so the board is four Islands and two Swamps and nothing else —
/// the {3}{U} is paid out of the Islands alone, which empties the pool and
/// leaves the black sources untapped, and `legal.abilities` is filtered through
/// the pool, so the pump is not offered while nothing floats and is offered the
/// moment the Swamps are tapped. The (3, 3) after the activation is the +1/+1
/// landing on the creature whose ability this is, read in the same breath as
/// the keyword that costs no mana at all.
#[test]
fn balshan_collaborator_pumps_itself_for_black_and_flies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), swamp(), swamp()],
        )
        .hand(0, &[balshan_collaborator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The four Islands pay the {3}{U}; the Swamps are named as the price the
    // pump will charge, so whatever black is in the pool afterwards came off a
    // source this test tapped on purpose.
    tap_all_mana_but(&mut engine, p0, Some(swamp()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, and the Swamps held back for the pump"
    );
    cast_with_floating(&mut engine, p0, balshan_collaborator());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let bird =
        on_battlefield(&engine, p0, balshan_collaborator()).expect("the Collaborator resolved");
    assert_eq!(pt(&engine, bird), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} was the whole of what the Islands made"
    );

    // `can_afford` reads the pool and not the untapped lands: with nothing
    // floating, the {B} is unpayable and the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(bird, 0)),
        "{{B}} is not nothing, so the pump is not offered on an empty pool: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "the two Swamps, and no other source on this board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bird, 0)),
        "with black floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, balshan_collaborator(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bird),
        (3, 3),
        "+1/+1 on the creature whose ability this is"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one {{B}} bought the pump, and the second Swamp's black is still in the pool"
    );
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "the pump does not disturb the printed flying"
    );
}
