//! `cards/sorceries/mv_4/taste_of_paradise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn taste_of_paradise() -> CardIndex {
    card_index("c9a20d03-b90a-43df-8ab4-ff0b38c90a08")
}

/// Taste of Paradise ({3}{G} Sorcery):
/// - "As an additional cost to cast this spell, you may pay {1}{G} any number of times."
/// - "You gain 3 life plus an additional 3 life for each additional {1}{G} you paid."
#[test]
fn taste_of_paradise_gains_three_life_or_six_when_additional_cost_is_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[taste_of_paradise(), taste_of_paradise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);

    // First copy without additional cost: decline kicker -> gains 3 life (20 -> 23).
    let first = in_hand(&engine, p0, taste_of_paradise()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 23);

    // Second copy with {1}{G} additional cost paid: accept kicker -> gains 6 life (23 -> 29).
    let second = in_hand(&engine, p0, taste_of_paradise()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 29);
}
