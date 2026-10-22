//! `cards/artifacts/mv_2/telim_tor_s_darts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn telim_tor_s_darts() -> CardIndex {
    card_index("0c978da4-9caa-4b4b-be52-889677962ec9")
}

/// Telim'Tor's Darts (`{2}`, `Coverage::Implemented`): `{2}, {T}` deals 1
/// damage to target player or planeswalker.
#[test]
fn telim_tor_s_darts_deals_1_damage_to_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[telim_tor_s_darts(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let darts = on_battlefield(&engine, p0, telim_tor_s_darts()).unwrap();
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

    assert_eq!(engine.state().players[1].life, 19);
}
