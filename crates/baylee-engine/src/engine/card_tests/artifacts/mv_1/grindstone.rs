//! `cards/artifacts/mv_1/grindstone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grindstone() -> CardIndex {
    card_index("97560adc-814e-450c-9ca7-f9364e910a0b")
}

/// Grindstone (`97560adc-814e-450c-9ca7-f9364e910a0b`) — {1} Artifact (Partial):
/// "{3}, {T}: Target player mills two cards."
#[test]
fn grindstone_mills_two_cards_from_target_player() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grindstone(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(P1)).len();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, grindstone(), 0);
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

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        gy_before + 2
    );
}

#[test]
fn grindstone_can_mill_its_controller_and_requires_three_mana_and_a_tap() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(SEED, &[grindstone()], &[], &[], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 2)]);
    assert!(!lk::offered(&e, grindstone(), P0, 0));
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    lk::use_ability(&mut e, P0, grindstone(), 0, &[], &[P0]);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)).len(), 2);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P1)).len(), 0);
    assert_eq!(lk::pool_total(&e, P0), 0);
    assert!(lk::tapped(&e, lk::obj(&e, P0, grindstone())));
}
