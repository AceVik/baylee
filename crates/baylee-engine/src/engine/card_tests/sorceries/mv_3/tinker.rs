//! `cards/sorceries/mv_3/tinker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tinker() -> CardIndex {
    card_index("254878f0-be90-4653-a395-0c41258fceaf")
}

fn thran_lens() -> CardIndex {
    card_index("0ac723bf-ce86-4028-a693-0ef498e4c82d")
}

fn thran_war_machine() -> CardIndex {
    card_index("b3b8fe34-4f69-4555-bcac-04563d02c99f")
}

#[test]
fn tinker_sacrifices_artifact_and_tutors_artifact_to_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, thran_war_machine())
        .battlefield(0, &[island(), island(), island(), thran_lens()])
        .hand(0, &[tinker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tinker());
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        panic!("expected ChooseCards for Tinker library search");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, thran_lens()).is_some());
    assert!(on_battlefield(&engine, p0, thran_war_machine()).is_some());
}

#[test]
fn tinker_cannot_sacrifice_an_opponents_artifact() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[sol_ring()], &[tinker()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 3)]);
    let spell = in_hand(&engine, lk::P0, tinker()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(
        !legal.castable.contains(&spell) && !legal.payable.contains(&spell),
        "mana is ready, but no owned sacrifice exists"
    );
    assert!(
        engine
            .apply(lk::P0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
}
