//! `cards/creatures/mv_2/shocker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn shocker() -> CardIndex {
    card_index("8981d1d1-e102-4a3d-96ad-3904c83d1611")
}

/// `Shocker` ({1}{R}, 1/1 Creature — Insect, `Coverage::Implemented`):
/// `Whenever this creature deals damage to a player, that player discards all
/// the cards in their hand, then draws that many cards.`
#[test]
fn shocker_makes_damaged_player_discard_their_hand_and_draw_that_many_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[shocker()])
        .hand(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    assert_eq!(hand_before, 3);

    let insect = on_battlefield(&engine, p0, shocker()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(insect, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 3);
}
