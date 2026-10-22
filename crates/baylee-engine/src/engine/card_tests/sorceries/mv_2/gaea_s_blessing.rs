//! `cards/sorceries/mv_2/gaea_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GAEA_S_BLESSING, GRIZZLY_BEARS};

/// `Gaea's Blessing` ({1}{G}, Sorcery) shuffles up to three target cards from
/// target player's graveyard into their library and draws a card.
#[test]
fn gaea_s_blessing_shuffles_targeted_graveyard_card_into_library_and_draws() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[GAEA_S_BLESSING, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = hand_to_graveyard(&mut engine, p0, GRIZZLY_BEARS);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, GAEA_S_BLESSING);
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
