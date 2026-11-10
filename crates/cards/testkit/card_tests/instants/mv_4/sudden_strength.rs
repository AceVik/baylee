//! `cards/instants/mv_4/sudden_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sudden Strength prints two sentences: "Target creature gets +3/+3 until end
/// of turn" and "Draw a card". Three readings make the card itself: the pump
/// lands on the creature the spell named and on no other, so an Elf beside the
/// host and an Elf across the table both stay printed 1/1s; the draw is a move
/// off the library and back into a hand the spell had just left; and the
/// printed duration is real, so a turn later the host is a 1/1 again with the
/// creature still standing. "Target creature" is any creature on either side of
/// the table, so the offer names the opponent's Elf as readily as mine.
#[test]
fn sudden_strength_pumps_only_the_creature_it_names_and_draws_a_card_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
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
        .hand(0, &[sudden_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays the control");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, sudden_strength());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be chosen: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
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
        (4, 4),
        "+3/+3 on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        in_graveyard(&engine, p0, sudden_strength()).is_some(),
        "an instant resolves into its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw put one card back"
    );

    // "until end of turn": a turn later the Elf is the 1/1 it was printed as,
    // and it is still on the battlefield — so what expired was the pump.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was cast in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
