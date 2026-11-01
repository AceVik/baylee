//! `cards/artifacts/mv_2/telim_tor_s_darts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn telim_tor_s_darts() -> CardIndex {
    card_index("0c978da4-9caa-4b4b-be52-889677962ec9")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Telim'Tor's Darts (`{2}`, `Coverage::Implemented`): `{2}, {T}` deals 1
/// damage to target player or planeswalker. A creature is not on the menu, the
/// activation costs `{2}` and the artifact's own tap, and exactly 1 damage goes
/// through.
#[test]
fn telim_tor_s_darts_deals_1_damage_to_target_player_but_not_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[telim_tor_s_darts(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let darts = on_battlefield(&engine, p0, telim_tor_s_darts()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: darts,
                ability_index: 0,
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&bear),
        "a creature is neither a player nor a planeswalker"
    );
    assert!(player_options.contains(&p1), "the opponent is a target");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .is_err(),
        "the Bears cannot be named"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19, "exactly 1 damage");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the controller is spared"
    );
    assert!(is_tapped(&engine, darts), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} was spent"
    );
}
