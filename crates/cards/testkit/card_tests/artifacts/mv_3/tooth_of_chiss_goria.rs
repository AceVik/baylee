//! `cards/artifacts/mv_3/tooth_of_chiss_goria.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tooth_of_chiss_goria() -> CardIndex {
    card_index("e5e7e7e4-a774-4d68-8767-8ba2bae38250")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Tooth of Chiss-Goria (e5e7e7e4-a774-4d68-8767-8ba2bae38250) — {3} Artifact:
/// "Flash"
/// "Affinity for artifacts"
/// "{T}: Target creature gets +1/+0 until end of turn."
///
/// Cast during the opponent's turn (Flash) with 2 own artifacts and 2 opponent
/// artifacts so affinity lowers {3} to {1} (not {0}), then taps for +1/+0
/// until end of turn (wearing off next turn, also legal on opponent's creature).
#[test]
fn tooth_of_chiss_goria_flash_affinity_and_power_pump() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[steel_wall(), ornithopter(), forest()])
        .hand(0, &[tooth_of_chiss_goria()])
        .battlefield(1, &[sol_ring(), sol_ring(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.pending().asked(), Some(p0));

    let tooth_hand = in_hand(&engine, p0, tooth_of_chiss_goria()).unwrap();
    assert!(
        !priority_offer(&engine).castable.contains(&tooth_hand),
        "needs {{1}}: opponent's artifacts do not count toward affinity"
    );

    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).castable.contains(&tooth_hand),
        "Flash + affinity for 2 own artifacts makes Tooth castable for {{1}} on opponent's turn"
    );
    cast_with_floating(&mut engine, p0, tooth_of_chiss_goria());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, thopter), (0, 2));

    if engine.pending().asked() == Some(p1) {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    activate(&mut engine, p0, tooth_of_chiss_goria(), 0);
    aim_at(&mut engine, p0, thopter);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, thopter), (1, 2), "target creature got +1/+0");

    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, thopter), (0, 2), "+1/+0 ended at end of turn");

    activate(&mut engine, p0, tooth_of_chiss_goria(), 0);
    aim_at(&mut engine, p0, their_bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, their_bears),
        (3, 2),
        "'target creature' can target an opponent's creature too"
    );
}
