//! An-Havva Inn — `{1}{G}{G}` Sorcery (Homelands)
//!
//! You gain X plus 1 life, where X is the number of green creatures on the
//! battlefield.

#[allow(clippy::wildcard_imports)]
use super::*;

fn an_havva_inn() -> CardIndex {
    card_index("3cd72848-7b99-4d46-96f5-53f297c9d2a0")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn an_havva_inn_gains_one_plus_green_creatures_on_battlefield_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[an_havva_inn()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                bears(),     // green creature #1
                gray_ogre(), // red creature (ignored)
            ],
        )
        .battlefield(1, &[bears()]) // opponent's green creature #2
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let inn_hand = in_hand(&engine, p0, an_havva_inn()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: inn_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 2 green creatures across the battlefield -> p0 gains 2 + 1 = 3 life (20 -> 23).
    assert_eq!(engine.state().players[0].life, 23);
}
