//! `cards/sorceries/mv_2/rolling_thunder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn rolling_thunder() -> CardIndex {
    card_index("9c47888b-28a5-4c43-9ee4-a9059e3c367d")
}

/// `Rolling Thunder` ({X}{R}{R}, Sorcery, `Coverage::Implemented`):
/// `Rolling Thunder deals X damage divided as you choose among any number of targets.`
#[test]
fn rolling_thunder_deals_x_damage_divided_among_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[rolling_thunder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, rolling_thunder());
    // Choose X = 3 ({3}{R}{R} = 5 mana).
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![p1],
            },
        )
        .unwrap();
    // Divide 3 damage: 2 to Grizzly Bears, remaining 1 to p1.
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 19);
}
