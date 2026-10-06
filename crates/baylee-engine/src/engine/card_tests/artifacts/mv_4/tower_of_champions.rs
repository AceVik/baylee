//! `cards/artifacts/mv_4/tower_of_champions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tower of Champions is `{4}` for one line — "`{8}`, `{T}`: Target creature
/// gets +6/+6 until end of turn" — and every part of that price is invisible in
/// the card file. Twelve Forests pay the `{4}` and leave exactly the `{8}` the
/// ability charges, so the offer only appears once the mana is really floating,
/// and the Elf across the table is offered the pump as readily as mine: "target
/// creature" is any creature, and the artifact itself is none. The two clauses
/// of the price are read in the rules' order — while the target question stands
/// (CR 601.2c) the Tower is still untapped and the pool still full, and the
/// `{T}` and the `{8}` go together when the answer lands (CR 601.2h). Walking a
/// whole turn afterwards is what tells the printed "until end of turn" from a
/// permanent grant.
#[test]
#[allow(clippy::too_many_lines)]
fn tower_of_champions_spends_eight_and_its_own_tap_for_six_six_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut board: Vec<CardIndex> = vec![forest(); 12];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[tower_of_champions()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Twelve Forests into the pool: `{4}` for the artifact and the `{8}` the
    // ability then charges, both inside this one main phase (CR 500.5). The
    // Elf is named as the printing kept back, because it is the creature the
    // pump is about to be aimed at and a mana creature tapped for the cost
    // would make "twelve" a count of thirteen.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, tower_of_champions());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_champions()).expect("the Tower resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the {{4}} is spent and exactly the {{8}} the ability charges is left"
    );
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — which is why the claim is made
    // with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, tower_of_champions(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&tower),
        "the Tower is an artifact and no creature: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays for it.
    assert!(
        !is_tapped(&engine, tower),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "and the {{8}} is still floating while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        is_tapped(&engine, tower),
        "{{T}} is paid by the Tower itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (7, 7),
        "+6/+6 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // "until end of turn": one turn later the Elf is a printed 1/1 again, so
    // the +6/+6 was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
