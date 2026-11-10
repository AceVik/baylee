//! `cards/artifacts/mv_2/thumbscrews.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thumbscrews() -> CardIndex {
    card_index("a8f1f55a-7a85-4338-b4d1-3d28f10f69a7")
}

/// `Thumbscrews` ({2}, Artifact, `Coverage::Implemented`):
/// `At the beginning of your upkeep, if you have five or more cards in hand,
/// this artifact deals 1 damage to target opponent or planeswalker.`
#[test]
fn thumbscrews_deals_one_damage_on_upkeep_when_controller_has_five_or_more_cards_in_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[thumbscrews()])
        .hand(0, &[plains(), plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);

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
        .battlefield(0, &[thumbscrews()])
        .hand(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    assert!(e.state().zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(e.state().players[1].life, 20);
}
