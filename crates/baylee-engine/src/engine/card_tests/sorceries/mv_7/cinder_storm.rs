//! `cards/sorceries/mv_7/cinder_storm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Cinder Storm` is a sorcery costing `{6}{R}` under `Coverage::Implemented`.
/// It prints "Cinder Storm deals 7 damage to any target."
/// When cast from hand off seven Mountains targeting the opponent, it resolves and deals
/// 7 damage to that player, reducing their life total from 20 to 13.
#[test]
fn cinder_storm_deals_seven_damage_to_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[cinder_storm()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, cinder_storm());

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Cinder Storm, got {:?}",
            engine.pending()
        );
    };
    assert!(
        player_options.contains(&p1),
        "opponent is offered as legal player target: {player_options:?}"
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
        13,
        "Cinder Storm dealt 7 damage to opponent"
    );
    assert!(
        in_graveyard(&engine, p0, cinder_storm()).is_some(),
        "resolved Cinder Storm sits in caster's graveyard"
    );
}
