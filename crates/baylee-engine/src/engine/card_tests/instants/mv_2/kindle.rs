//! `cards/instants/mv_2/kindle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kindle() -> CardIndex {
    card_index("9397a23b-770a-43fc-8bc2-2c3abb0683c5")
}

/// Kindle (`9397a23b-770a-43fc-8bc2-2c3abb0683c5`) — {1}{R} Instant:
/// "Kindle deals X damage to any target, where X is 2 plus the number of cards named Kindle in all graveyards."
#[test]
fn kindle_deals_two_plus_the_number_of_kindles_in_all_graveyards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[kindle(), kindle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // First Kindle: 0 in graveyards -> 2 damage (life 18).
    cast_from_hand(&mut engine, P0, kindle());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);

    // Second Kindle: 1 Kindle in graveyard -> 2 + 1 = 3 damage (life 15).
    cast_from_hand(&mut engine, P0, kindle());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 15);
}

#[test]
fn kindle_counts_the_opponents_graveyard_but_not_other_card_names() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let mut e = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[kindle()], &[kindle()]);
    let bears = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    bury(&mut e, &[bears]);
    e.refresh_offer();
    lk::float(&mut e, P0, &[(ManaColor::Red, 2)]);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, P1, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut e, P1, kindle(), &[], &[P0]);
    assert_eq!(lk::life(&e, P0), 18);
    lk::cast_at(&mut e, P0, kindle(), &[], &[P1]);
    assert_eq!(lk::life(&e, P1), 17);
}
