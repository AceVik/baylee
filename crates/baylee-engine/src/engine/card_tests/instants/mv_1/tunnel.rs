//! `cards/instants/mv_1/tunnel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tunnel prints one sentence — "Destroy target Wall. It can't be
/// regenerated." — and the pool implements no regeneration anywhere, so the
/// whole card is the destroy plus the word *Wall*. The board therefore puts a
/// Wall and a creature that is not one under the same opponent: the offer has
/// to hold exactly the first, and the Elf beside it is the control that says
/// the subtype filter was read rather than that a lone creature turned out to
/// be legal. Afterwards the Wall is in its owner's graveyard and the Elf is
/// untouched — a spell that killed whatever it was aimed at would still leave
/// the first assertion green and the second one red.
#[test]
fn tunnel_destroys_a_wall_and_declines_a_creature_that_is_not_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[tunnel()])
        .battlefield(1, &[wall_of_roots(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, wall_of_roots()).expect("the Wall is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");

    // {R} off the one Mountain, and the spell asks for its target as it is
    // cast (CR 601.2c) rather than when it resolves.
    cast_from_hand(&mut engine, p0, tunnel());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Wall\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one target, and no \"up to\"");
    assert!(
        options.contains(&wall),
        "a Wall is the one thing Tunnel may point at: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "a creature that is no Wall is not a legal target: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the Wall is the whole menu, on a board that holds four creatures: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .expect("the Wall was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wall_of_roots()).is_some(),
        "the targeted Wall went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, wall_of_roots()).is_none(),
        "and it left the battlefield rather than merely changing hands"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, tunnel()).is_some(),
        "an instant that resolved is in the graveyard of the seat that cast it"
    );
}
