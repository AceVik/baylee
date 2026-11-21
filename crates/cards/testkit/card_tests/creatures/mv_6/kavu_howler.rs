//! `cards/creatures/mv_6/kavu_howler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn kavu_howler() -> CardIndex {
    card_index("635c0a7d-d832-4c10-a5f0-9cde19152cbb")
}

fn yavimaya_kavu() -> CardIndex {
    card_index("285c9f49-4aeb-468c-bb41-f5ccb5257e2b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Kavu Howler (635c0a7d-d832-4c10-a5f0-9cde19152cbb) — `{4}{G}{G}` 4/5 Creature — Kavu:
/// "When this creature enters, reveal the top four cards of your library. Put all
/// Kavu cards revealed this way into your hand and the rest on the bottom of your
/// library in any order."
#[test]
fn kavu_howler_puts_all_revealed_kavu_from_top_four_into_hand_and_bottoms_rest() {
    let lands = vec![forest(); 6];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .hand(
            0,
            &[
                kavu_howler(),
                kavu_howler(),
                yavimaya_kavu(),
                grizzly_bears(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let miss_bear = hand_to_library_top(&mut engine, P0, grizzly_bears());
    let hit_a = hand_to_library_top(&mut engine, P0, yavimaya_kavu());
    let miss_island = hand_to_library_top(&mut engine, P0, island());
    let hit_b = hand_to_library_top(&mut engine, P0, kavu_howler());

    cast_from_hand(&mut engine, P0, kavu_howler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_island, miss_bear]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let howler = on_battlefield(&engine, P0, kavu_howler()).expect("Kavu Howler entered");
    assert_eq!(pt(&engine, howler), (4, 5));
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0));
    assert!(hand.contains(&hit_a) && hand.contains(&hit_b));
    assert!(!hand.contains(&miss_bear) && !hand.contains(&miss_island));
}
