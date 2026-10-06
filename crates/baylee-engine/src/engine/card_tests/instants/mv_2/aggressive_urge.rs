//! `cards/instants/mv_2/aggressive_urge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aggressive Urge prints two effects on one instant for `{1}{G}`: "Target
/// creature gets +1/+1 until end of turn" and "Draw a card."
///
/// The two printed effects are read where each one leaves a mark a test can
/// see. The pump is read off the board after the spell has resolved, on the
/// creature the target question named and on no other — so a second Elf
/// under the same seat and an Elf across the table are both standing there
/// to catch a `Filter` that had widened. The draw is read off the library
/// and the hand together, because a count alone would be satisfied by a
/// card that left the library without arriving anywhere.
///
/// The four Forests, and neither Elf tapped for any of it: a creature that
/// paid its own `{T}` toward the spell it is about to be the target of is a
/// creature whose board state already changed for a reason of its own.
#[test]
fn aggressive_urge_pumps_the_target_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[aggressive_urge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two of the four Forests pay the {1}{G}; the Elves are named as the
    // printing kept back, since they are the creatures this test reads back.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped and neither Elf"
    );
    cast_with_floating(&mut engine, p0, aggressive_urge());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the mana is
    // still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the spell is pointed at"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody pointed at is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the target and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{G}} came out of the four Forests"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it is in hand: the spell left the hand and the card it drew \
         took its place, so the count is where it started"
    );
}
