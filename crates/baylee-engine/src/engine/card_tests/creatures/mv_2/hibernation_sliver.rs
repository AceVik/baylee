//! `cards/creatures/mv_2/hibernation_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);

fn hibernation_sliver() -> CardIndex {
    card_index("edb8a6d6-9e25-4048-a842-e854eece42c2")
}

/// Hibernation Sliver (`edb8a6d6-9e25-4048-a842-e854eece42c2`) — {U}{B} 2/2 Creature — Sliver (Implemented):
/// "All Slivers have 'Pay 2 life: Return this permanent to its owner's hand.'"
#[test]
fn hibernation_sliver_lets_slivers_pay_two_life_to_return_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hibernation_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sliver = on_battlefield(&engine, P0, hibernation_sliver()).unwrap();
    let idx = kit::offered(&engine, sliver)[0];
    kit::activate_object(&mut engine, P0, sliver, idx);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 18);
    assert!(on_battlefield(&engine, P0, hibernation_sliver()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .iter()
            .any(
                |&id| engine.state().object(id).unwrap().card.map(|c| c.index)
                    == Some(hibernation_sliver())
            )
    );
}

#[test]
fn opposing_sliver_pays_its_controllers_life_and_nonsliver_has_no_grant() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[hibernation_sliver()],
        &[index::METALLIC_SLIVER, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    reach_their_main_phase(&mut e, lk::P1);
    let sliver = lk::obj(&e, lk::P1, index::METALLIC_SLIVER);
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    assert!(kit::offered(&e, bear).is_empty());
    let ability = kit::offered(&e, sliver)[0];
    kit::activate_object(&mut e, lk::P1, sliver, ability);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::life(&e, lk::P1), 18);
    assert!(in_hand(&e, lk::P1, index::METALLIC_SLIVER).is_some());
    assert!(on_battlefield(&e, P0, hibernation_sliver()).is_some());
}
