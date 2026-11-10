//! `cards/creatures/mv_1/goblin_digging_team.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Roots: the pool's quietest Wall — a `0/5` whose whole text is
/// one mana ability, so a Wall on this board contributes no other rule.
/// Goblin Digging Team — {R} — 1/1 Goblin: "{T}, Sacrifice this creature:
/// Destroy target Wall."
///
/// Both halves of the price are why this board carries two Walls. The
/// sacrifice and the tap are paid at CR 601.2h, *after* the target is named
/// at CR 601.2c, so while the choice is open the Goblin must still be on the
/// battlefield and still untapped — and then be gone. "Target Wall" is a
/// subtype and not a side, so the Wall under the Goblin's own controller
/// shares the menu with the one across the table, and the Llanowar Elves
/// beside it are the control: a creature that is not a Wall.
#[test]
fn goblin_digging_team_taps_and_sacrifices_itself_to_destroy_the_wall_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(271, forest())
        .battlefield(0, &[goblin_digging_team(), wall_of_roots()])
        .battlefield(1, &[wall_of_roots(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let team = on_battlefield(&engine, p0, goblin_digging_team()).expect("the Goblin is out");
    let my_wall = on_battlefield(&engine, p0, wall_of_roots()).expect("my Wall is out");
    let their_wall = on_battlefield(&engine, p1, wall_of_roots()).expect("their Wall is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(team, 0)),
        "a Wall is on the table, so the one line the Goblin prints is \
         offered, and its whole price is its own body and its own {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_digging_team(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"destroy target Wall\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    // CR 601.2c comes before CR 601.2h: the target is named while the Goblin
    // is still standing and still untapped, and the cost is paid after.
    assert!(
        on_battlefield(&engine, p0, goblin_digging_team()).is_some(),
        "the sacrifice is the last step, so the Goblin is still on the table"
    );
    assert!(!is_tapped(&engine, team), "and its {{T}} is unpaid");
    assert!(
        options.contains(&my_wall) && options.contains(&their_wall),
        "\"target Wall\" is a subtype and not a side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elves are a creature and no Wall — the filter is read: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_wall],
            },
        )
        .expect("the Wall the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wall_of_roots()).is_some(),
        "the Wall that was named is destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_roots()).is_some(),
        "and the Wall the ability did not name is untouched, so the target \
         was read off the choice and not off the board"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did anything point at the creature that is not a Wall"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_digging_team()).is_none(),
        "the Goblin sacrificed itself to pay"
    );
    assert!(
        in_graveyard(&engine, p0, goblin_digging_team()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
}
