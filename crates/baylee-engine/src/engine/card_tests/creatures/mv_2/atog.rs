//! `cards/creatures/mv_2/atog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Atog — {1}{R} 1/2 — "Sacrifice an artifact: This creature gets +2/+2
/// until end of turn." The cost names no artifact, so the engine has to ask
/// which one, and the menu is half the card: your own Sol Ring is on it, the
/// Elves beside it are a creature and not an artifact, and the Sol Ring
/// across the table is not yours to spend (CR 701.21a). Eating the ring is
/// what makes the pump readable with an empty mana pool — a printed 1/2
/// becomes a 3/4 without a single land on the board, which is what tells a
/// sacrifice cost from a mana one — and the walk into the next turn is what
/// says the +2/+2 is a duration rather than a counter.
#[test]
fn atog_eats_an_artifact_you_control_for_two_and_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(79, forest())
        .battlefield(0, &[atog(), quiet_artifact(), llanowar_elves()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let atog_id = on_battlefield(&engine, p0, atog()).expect("the Atog is on the table");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, atog_id), (1, 2), "a printed 1/2 before it eats");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and no mana is on the board: the whole price is the artifact"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(atog_id, 0)),
        "an artifact to eat is the only thing the cost needs: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, atog(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which artifact, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the artifact you control is on the menu: {options:?}"
    );
    assert_eq!(options.len(), 1, "and it is the whole menu: {options:?}");
    assert!(
        !options.contains(&atog_id),
        "the Atog is a creature and no artifact: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "nor are the Elves, which are no artifacts either: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact is in its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, atog_id),
        (3, 4),
        "+2/+2 from the one artifact it ate"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the artifact the cost did not name is untouched"
    );
    assert!(
        !is_tapped(&engine, atog_id),
        "the price is the sacrifice alone: the Atog never taps"
    );

    // The pump is a duration and not a counter: one turn later it is gone.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, atog_id),
        (1, 2),
        "\"until end of turn\" — back to the 1/2 the card prints"
    );
}
