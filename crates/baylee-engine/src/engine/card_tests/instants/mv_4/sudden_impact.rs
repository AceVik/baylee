//! `cards/instants/mv_4/sudden_impact.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sudden_impact() -> CardIndex {
    card_index("2bf65e7d-6d31-4ad6-93cd-623612d3175d")
}

/// `Sudden Impact` ({3}{R}, Instant, `Coverage::Implemented`):
/// `Sudden Impact deals damage to target player equal to the number of cards in
/// that player's hand.`
#[test]
fn sudden_impact_deals_damage_to_target_player_equal_to_their_hand_size() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[sudden_impact()])
        .hand(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sudden_impact());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn empty_hand_takes_zero_and_you_can_target_yourself() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for target in [p0, p1] {
        let mut e = lk::game(SEED, &[], &[], &[sudden_impact()], &[]);
        lk::float(&mut e, p0, &[(ManaColor::Red, 4)]);
        cast_with_floating(&mut e, p0, sudden_impact());
        e.apply(p0, PlayerAction::ChoosePlayer(target)).unwrap();
        lk::settle(&mut e);
        assert_eq!(e.state().players[0].life, 20);
        assert_eq!(e.state().players[1].life, 20);
    }
}
