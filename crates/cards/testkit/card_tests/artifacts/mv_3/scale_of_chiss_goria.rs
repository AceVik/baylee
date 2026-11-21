//! `cards/artifacts/mv_3/scale_of_chiss_goria.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scale_of_chiss_goria() -> CardIndex {
    card_index("1ab0a315-0d0d-4247-9bb9-107b35b353d6")
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

/// Scale of Chiss-Goria (1ab0a315-0d0d-4247-9bb9-107b35b353d6) — {3} Artifact:
/// "Flash"
/// "Affinity for artifacts"
/// "{T}: Target creature gets +0/+1 until end of turn."
///
/// Cast during the opponent's turn (Flash) with 2 own artifacts and 2 opponent
/// artifacts so affinity lowers {3} to {1} (not {0}), then taps for +0/+1
/// until end of turn (wearing off next turn, also legal on opponent's creature).
#[test]
fn scale_of_chiss_goria_flash_affinity_and_toughness_pump() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[steel_wall(), ornithopter(), forest()])
        .hand(0, &[scale_of_chiss_goria()])
        .battlefield(1, &[sol_ring(), sol_ring(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    // Move to p1's main phase and let p1 pass priority to p0 (instant window).
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.pending().asked(), Some(p0));

    let scale_hand = in_hand(&engine, p0, scale_of_chiss_goria()).unwrap();
    // 2 artifacts you control reduce {3} to {1}; opponent's 2 Sol Rings don't count.
    assert!(
        !priority_offer(&engine).castable.contains(&scale_hand),
        "needs {{1}}: opponent's artifacts do not count toward affinity"
    );

    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).castable.contains(&scale_hand),
        "Flash + affinity for 2 own artifacts makes Scale castable for {{1}} on opponent's turn"
    );
    cast_with_floating(&mut engine, p0, scale_of_chiss_goria());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let wall = on_battlefield(&engine, p0, steel_wall()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, wall), (0, 4));

    // Wait for p0 to hold priority again on p1's turn, then pump Steel Wall +0/+1.
    if engine.pending().asked() == Some(p1) {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    activate(&mut engine, p0, scale_of_chiss_goria(), 0);
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wall), (0, 5), "target creature got +0/+1");

    // Cross into p0's turn: +0/+1 wore off and Scale untapped; also pumps opponent's creature.
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, wall), (0, 4), "+0/+1 ended at end of turn");

    activate(&mut engine, p0, scale_of_chiss_goria(), 0);
    aim_at(&mut engine, p0, their_bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, their_bears),
        (2, 3),
        "'target creature' can target an opponent's creature too"
    );
}
