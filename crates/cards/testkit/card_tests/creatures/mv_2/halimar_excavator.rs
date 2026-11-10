//! `cards/creatures/mv_2/halimar_excavator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Halimar Excavator's rally mills a player the controller *chose*.
///
/// It was written as `PlayerRel::Opponent` with no target requirement at
/// all, so it milled the opponent by construction: the controller could
/// never mill themselves, and a player who could not legally be targeted
/// was milled anyway. The printed line is "target player mills X", and it
/// is not optional — with nobody else legal the controller has to point it
/// at themselves, which is why the requirement's `min` is one.
#[test]
fn halimar_excavator_mills_the_player_it_targeted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(57, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    cast_from_hand(&mut engine, p0, halimar_excavator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "either player may be targeted, the controller included",
    );
    assert_eq!(min, 1, "\"target player mills X\" is not optional");

    // Aimed at the controller, which is the half the old card could not do.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 1,
        "one Ally on the battlefield, so the player it named mills one card",
    );
}
