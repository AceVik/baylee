//! `cards/artifacts/mv_4/ice_cauldron.rs`, played.
//!
//! Oracle:
//! - "{X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact."
//! - "{T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn ice_cauldron() -> CardIndex {
    card_index("2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a")
}

#[test]
fn ice_cauldron_places_charge_counter_and_blocks_reactivation() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());

    activate(&mut engine, P0, ice_cauldron(), 0);
    if let Pending::ChooseNumber { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::ChooseNumber(0)).unwrap();
    }
    lk::settle(&mut engine);
    assert_eq!(
        engine
            .state()
            .object(cauldron)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );

    // Untap Ice Cauldron: ability 0 still cannot be activated while a charge counter is on it.
    engine
        .dev_state_mut(P0)
        .unwrap()
        .set_tapped(cauldron, false);
    engine.refresh_offer();
    assert!(
        !lk::offered(&engine, ice_cauldron(), P0, 0),
        "first ability cannot be activated while Ice Cauldron has a charge counter"
    );
}
