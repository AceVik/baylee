//! `cards/instants/mv_3/kaervek_s_spite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kaervek_s_spite() -> CardIndex {
    card_index("7d69be1e-701a-4bcb-a83e-38b4174f7828")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

#[test]
fn kaervek_s_spite_sacrifices_all_permanents_and_discards_hand_to_drain_5() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), sol_ring()])
        .hand(0, &[kaervek_s_spite(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    kit::float(&mut e, P0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut e, P0, kaervek_s_spite());
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .all(|id| { e.state().object(*id).is_none_or(|o| o.controller != P0) }),
        "all P0 permanents sacrificed"
    );
    assert!(
        e.state().zones.list(ZoneLocation::Hand(P0)).is_empty(),
        "hand discarded"
    );
    assert_eq!(e.state().players[1].life, 15);
}

#[test]
fn kaervek_s_spite_works_with_zero_permanents_and_empty_hand_from_floating_mana() {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[kaervek_s_spite()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    kit::float(&mut e, P0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut e, P0, kaervek_s_spite());
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 15);
}
