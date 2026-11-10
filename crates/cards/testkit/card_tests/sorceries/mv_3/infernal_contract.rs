//! `cards/sorceries/mv_3/infernal_contract.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn infernal_contract() -> CardIndex {
    card_index("4372205f-45e6-49c7-b1fd-13a41e20d4de")
}

/// Infernal Contract (4372205f-45e6-49c7-b1fd-13a41e20d4de) — {B}{B}{B} Sorcery:
/// "Draw four cards. You lose half your life, rounded up."
#[test]
fn infernal_contract_draws_four_cards_and_loses_half_life_rounded_up() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[infernal_contract()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, infernal_contract());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before_hand - 1 + 4
    );
    assert_eq!(engine.state().players[0].life, 10);
}

/// "half your life, rounded up": at 15 life the loss is 8, leaving 7.
#[test]
fn infernal_contract_rounds_the_life_loss_up_at_an_odd_total() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[infernal_contract()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    engine.dev_state_mut(P0).unwrap().players[0].life = 15;
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, infernal_contract());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 7);
}
