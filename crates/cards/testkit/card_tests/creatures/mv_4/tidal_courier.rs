//! `cards/creatures/mv_4/tidal_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tidal_courier() -> CardIndex {
    card_index("b0a0c17b-87c8-4610-adca-5e7f5ced49f6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Tidal Courier (b0a0c17b-87c8-4610-adca-5e7f5ced49f6) — `{3}{U}` 1/2 Creature — Merfolk:
/// - When this creature enters, reveal the top four cards of your library. Put all
///   Merfolk cards revealed this way into your hand and the rest on the bottom of
///   your library in any order.
/// - `{3}{U}`: This creature gains flying until end of turn.
#[test]
fn tidal_courier_puts_revealed_merfolk_into_hand_and_gains_flying_until_end_of_turn() {
    let lands = vec![island(); 8];
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &lands)
        .battlefield(1, &[grizzly_bears()])
        .hand(
            0,
            &[
                tidal_courier(),
                tidal_courier(),
                tidal_courier(),
                grizzly_bears(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let miss_bear = hand_to_library_top(&mut engine, P0, grizzly_bears());
    let hit_a = hand_to_library_top(&mut engine, P0, tidal_courier());
    let miss_forest = hand_to_library_top(&mut engine, P0, forest());
    let hit_b = hand_to_library_top(&mut engine, P0, tidal_courier());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, tidal_courier());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_forest, miss_bear]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let courier = on_battlefield(&engine, P0, tidal_courier()).expect("Tidal Courier entered");
    assert_eq!(pt(&engine, courier), (1, 2));
    assert!(!keywords(&engine, courier).contains(KeywordSet::FLYING));

    let hand = engine.state().zones.list(ZoneLocation::Hand(P0));
    assert!(hand.contains(&hit_a) && hand.contains(&hit_b));
    assert!(!hand.contains(&miss_bear) && !hand.contains(&miss_forest));

    // Pay {3}{U}: gains flying until end of turn.
    activate(&mut engine, P0, tidal_courier(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, courier).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, courier).contains(KeywordSet::FLYING),
        "flying ends at cleanup"
    );
}
