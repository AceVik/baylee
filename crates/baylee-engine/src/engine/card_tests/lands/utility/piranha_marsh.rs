//! `cards/lands/utility/piranha_marsh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Piranha Marsh` is a utility land under `Coverage::Implemented` that enters tapped,
/// causes target player to lose 1 life when it enters, and taps for `{B}`.
/// When played from hand, it enters tapped and places its enters-the-battlefield trigger on the stack.
/// When the trigger resolves targeting the opponent, they lose 1 life. Passing to the next turn untaps the
/// land, allowing it to produce one black mana.
#[test]
fn piranha_marsh_enters_tapped_drains_life_and_taps_for_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[piranha_marsh()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let marsh = play_land(&mut engine, p0, piranha_marsh());
    assert!(
        is_tapped(&engine, marsh),
        "Piranha Marsh enters the battlefield tapped"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for ETB trigger, got {:?}",
            engine.pending()
        );
    };
    assert!(
        player_options.contains(&p1),
        "opponent is offered as target player for life loss: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            },
        )
        .expect("targeting opponent is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "opponent lost 1 life from the enters-the-battlefield trigger"
    );

    // Pass to next turn so Piranha Marsh untaps and can be tapped for mana.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, marsh),
        "Piranha Marsh untaps in its controller's untap step"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "Piranha Marsh taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Piranha Marsh produces exactly one black mana"
    );
}
