//! `cards/creatures/artifacts/mv_4/flint_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn flint_golem_mills_defending_player_three_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("0993c5f1-1e8f-4871-9904-6c5a00405f36")])
        .battlefield(1, &[index::WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let golem = on_battlefield(
        &engine,
        p0,
        card_index("0993c5f1-1e8f-4871-9904-6c5a00405f36"),
    )
    .expect("Flint Golem");
    let wall = on_battlefield(&engine, p1, index::WALL_OF_WOOD).expect("Wall of Wood");
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p1)).len();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(golem, Defender::Player(p1))],
            },
        )
        .expect("declare attackers");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, golem)],
            },
        )
        .expect("declare blockers");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).len(),
        lib_before - 3
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );
}
