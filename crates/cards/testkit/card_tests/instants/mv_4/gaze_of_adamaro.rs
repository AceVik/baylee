//! `cards/instants/mv_4/gaze_of_adamaro.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "91a69925-1e9b-433a-ad58-88bfd3c4bc46"
fn gaze_of_adamaro() -> CardIndex {
    card_index("91a69925-1e9b-433a-ad58-88bfd3c4bc46")
}

/// Gaze of Adamaro ({2}{R}{R}): deals damage to target player equal to the
/// number of cards in THAT player's hand (not the caster's).
#[test]
fn gaze_of_adamaro_deals_damage_equal_to_target_players_hand_size() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[gaze_of_adamaro(), gaze_of_adamaro()],
        &[forest(), forest(), forest(), forest()],
    );

    // 1. Target P1 (4 cards in hand) while P0 has 1 card left after casting -> P1 takes 4.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 8)]);
    cast_with_floating(&mut engine, P0, gaze_of_adamaro());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 16, "P1 had 4 cards -> took 4");
    assert_eq!(lk::life(&engine, P0), 20);

    // 2. Target P0 (0 cards left in hand after casting 2nd Gaze) -> P0 takes 0.
    cast_with_floating(&mut engine, P0, gaze_of_adamaro());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "P0 had 0 cards left -> took 0");
}
