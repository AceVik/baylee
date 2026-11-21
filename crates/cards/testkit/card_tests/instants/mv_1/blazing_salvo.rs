//! `cards/instants/mv_1/blazing_salvo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e6681d22-821a-47c8-9c0d-042a7d44f843"
fn blazing_salvo() -> CardIndex {
    card_index("e6681d22-821a-47c8-9c0d-042a7d44f843")
}

/// Blazing Salvo ({R}): if the target creature's controller accepts 5 damage,
/// Blazing Salvo deals 5 to that player and 0 to the creature.
#[test]
fn blazing_salvo_deals_five_to_controller_when_they_accept() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[blazing_salvo()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, blazing_salvo());
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });

    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, .. } if *player == P1
    ));
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 15, "P1 took 5 damage");
    assert_eq!(
        on_battlefield(&engine, P1, index::HILL_GIANT),
        Some(giant),
        "Hill Giant is spared"
    );
    assert_eq!(lk::damage_on(&engine, giant), 0);
}

/// If the creature's controller declines the 5 damage, Blazing Salvo deals 3
/// damage to the creature and 0 to its controller.
#[test]
fn blazing_salvo_deals_three_to_creature_when_controller_declines() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[blazing_salvo()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, blazing_salvo());
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 20, "P1 took no damage");
    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "3 damage killed the 3/3 Hill Giant"
    );
}
