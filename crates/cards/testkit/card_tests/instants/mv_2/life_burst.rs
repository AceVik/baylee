//! `cards/instants/mv_2/life_burst.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "dc10544f-dd30-4d4a-b6e6-bbed7d1ed4a7"
fn life_burst() -> CardIndex {
    card_index("dc10544f-dd30-4d4a-b6e6-bbed7d1ed4a7")
}

/// Life Burst ({1}{W}): target player gains 4 life, plus 4 for each card named
/// Life Burst already in EACH graveyard (never counting itself on the stack).
#[test]
fn life_burst_gains_four_plus_four_per_life_burst_in_all_graveyards() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[life_burst(), life_burst()],
        &[life_burst()],
    );

    // 1. 0 Life Bursts in any graveyard -> P0 gains 4 (20 -> 24).
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, life_burst());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 24, "0 in graveyards -> +4 life");

    // 2. P1 casts Life Burst targeting P1 with 1 in P0's GY -> P1 gains 8 (20 -> 28).
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P1, life_burst());
    engine.apply(P1, PlayerAction::ChoosePlayer(P1)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 28, "1 in opponent's GY -> +8 life");

    // 3. P0 casts second Life Burst with 2 across both GYs -> P0 gains 12 (24 -> 36).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, life_burst());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 36, "2 across all GYs -> +12 life");
}
