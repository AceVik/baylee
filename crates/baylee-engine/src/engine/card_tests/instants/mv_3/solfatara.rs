//! `cards/instants/mv_3/solfatara.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MOUNTAIN;

fn solfatara() -> CardIndex {
    card_index("6053a192-fcf9-4b06-9f46-e84eaadaa882")
}

/// `Solfatara` ({2}{R}, Instant, `Coverage::Partial`) prevents land plays this
/// turn and draws a card at the beginning of the next turn's upkeep.
#[test]
fn solfatara_prevents_playing_lands_this_turn_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .hand(0, &[solfatara(), MOUNTAIN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land_in_hand = in_hand(&engine, p0, MOUNTAIN).unwrap();
    cast_from_hand(&mut engine, p0, solfatara());
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: land_in_hand })
            .is_err()
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
