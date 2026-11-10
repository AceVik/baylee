//! `cards/instants/mv_3/solfatara.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MOUNTAIN;

fn solfatara() -> CardIndex {
    card_index("6053a192-fcf9-4b06-9f46-e84eaadaa882")
}

/// `Solfatara` ({2}{R}, Instant, `Coverage::Partial`): "Draw a card at the
/// beginning of the next turn's upkeep." The caster draws once, at the next
/// upkeep, and not at the one after; the opponent draws nothing extra.
/// The land lock is left out, so a land is still playable (it would otherwise
/// bind every player, the caster too).
#[test]
fn solfatara_draws_a_card_at_the_next_upkeep_only_and_leaves_land_plays_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .hand(0, &[solfatara(), MOUNTAIN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land_in_hand = in_hand(&engine, p0, MOUNTAIN).unwrap();
    cast_from_hand(&mut engine, p0, solfatara());
    pass_until(&mut engine, stack_is_empty);

    // The land lock is not part of the card: the land goes down.
    engine
        .apply(p0, PlayerAction::PlayLand { card: land_in_hand })
        .expect("Solfatara locks no one out of playing lands");

    // Hand now: nothing (Solfatara cast, the Mountain played).
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after_cast, 0);

    // The next turn is P1's: the card comes at its upkeep, to the caster.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 1
    );
    // P1 drew their own turn's card only (a Duel opens with no hand).
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 1);

    // Only once: on P0's own turn they draw their draw-step card and no more.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 2,
        "one extra card at the next upkeep, then the ordinary draw step"
    );
}
