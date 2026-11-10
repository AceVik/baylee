//! `cards/artifacts/mv_4/power_matrix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Power Matrix — {4} artifact: "{T}: Target creature gets +1/+1 and gains
/// flying, first strike, and trample until end of turn." The card makes four
/// printed claims at once, so a Matrix that granted only one of the three
/// keywords would satisfy any test that read one of them. The second Elf under
/// the same seat and the third across the table are the controls for "target
/// creature" — neither may change, and the artifact itself is on neither menu
/// because it is no creature — while the walk into a later turn is the control
/// for "until end of turn", which no single-moment reading can see.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn power_matrix_pumps_and_arms_the_one_creature_it_targets_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board: Vec<CardIndex> = vec![forest(); 5];
    board.extend([llanowar_elves(), llanowar_elves()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[power_matrix()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Matrix");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing is granted yet"
    );

    // The {4} out of five Forests, with both Elves named as the printing kept
    // back: they are the creatures this test reads back afterwards, and
    // `tap_all_mana` would have spent their own `{T}: Add {G}` as well (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Forests and neither Elf: five green"
    );
    cast_with_floating(&mut engine, p0, power_matrix());
    pass_until(&mut engine, stack_is_empty);

    let matrix = on_battlefield(&engine, p0, power_matrix()).expect("the Matrix resolved");
    assert!(!is_tapped(&engine, matrix), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the cast's {{4}} is spent and one green is left floating"
    );

    // The whole price of the ability is the artifact's own {T}, so the line is
    // offered on this pool without any further mana being payable or needed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(matrix, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, power_matrix(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&matrix),
        "the Matrix is an artifact and no creature: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // artifact is still standing while this question is open.
    assert!(
        !is_tapped(&engine, matrix),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the pool is untouched while the target is still unanswered"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(is_tapped(&engine, matrix), "{{T}} is paid by the artifact");
    assert!(
        !stack_is_empty(&engine),
        "granting keywords is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, host), (2, 2), "+1/+1 on the creature it named");
    let granted = keywords(&engine, host);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "the printed flying reached the target"
    );
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike, which no reading of the pump alone shows"
    );
    assert!(
        granted.contains(KeywordSet::TRAMPLE),
        "and the printed trample, on the same granted set"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is untouched: the effect targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "and carries none of the keywords either"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor does the grant cross the table to the creature it could have named"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "\"target creature\" is one creature, not a side of the table"
    );
    assert!(
        !keywords(&engine, matrix).contains(KeywordSet::FLYING),
        "the artifact grants the keywords, it does not keep them"
    );

    // "until end of turn": a turn later the Elf is a printed 1/1 again, so the
    // +1/+1 and the three keywords were a duration and not a body.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    let after = keywords(&engine, host);
    assert!(
        !after.contains(KeywordSet::FLYING)
            && !after.contains(KeywordSet::FIRST_STRIKE)
            && !after.contains(KeywordSet::TRAMPLE),
        "all three keywords left with the turn: {after:?}"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the grant left rather than the creature"
    );
}
