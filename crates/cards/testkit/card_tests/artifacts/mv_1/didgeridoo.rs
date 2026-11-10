//! Didgeridoo — `{1}` Artifact (Homelands)
//!
//! `{3}`: You may put a Minotaur permanent card from your hand onto the
//! battlefield.

#[allow(clippy::wildcard_imports)]
use super::*;

fn didgeridoo() -> CardIndex {
    card_index("e9a2cb10-6526-4a5c-bda7-e2f328b377dc")
}

fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn didgeridoo_puts_minotaur_permanent_card_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[didgeridoo(), hurloon_minotaur(), bears()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let didg_hand = in_hand(&engine, p0, didgeridoo()).unwrap();
    let minotaur_hand = in_hand(&engine, p0, hurloon_minotaur()).unwrap();
    let bears_hand = in_hand(&engine, p0, bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: didg_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let didg = on_battlefield(&engine, p0, didgeridoo()).unwrap();

    // Activate {3} on Didgeridoo: only Hurloon Minotaur is offered, not Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: didg,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    assert!(
        options.contains(&minotaur_hand),
        "Minotaur permanent card in hand is offered"
    );
    assert!(
        !options.contains(&bears_hand),
        "non-Minotaur creature card in hand is not offered"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![minotaur_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, hurloon_minotaur()).is_some(),
        "Hurloon Minotaur entered the battlefield from hand via Didgeridoo"
    );
}
