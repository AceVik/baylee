//! `cards/artifacts/mv_2/scalding_tongs.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scalding_tongs() -> CardIndex {
    card_index("939a0cdf-b5e8-4d7d-adb3-f7b2fd7aab8f")
}

/// `Scalding Tongs` ({2}, Artifact, `Coverage::Implemented`):
/// `At the beginning of your upkeep, if you have three or fewer cards in hand,
/// this artifact deals 1 damage to any target.`
#[test]
fn scalding_tongs_deals_one_damage_on_upkeep_when_controller_has_three_or_fewer_cards_in_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[scalding_tongs()])
        .hand(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    // On p0's upkeep, hand size is 2 <= 3, so Scalding Tongs triggers and asks for a target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}

/// Outside the printed hand-size bound, no target is asked and no damage dealt.
#[test]
fn upkeep_with_four_cards_does_not_trigger() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[scalding_tongs()])
        .hand(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    assert!(e.state().zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(e.state().players[1].life, 20);
}
