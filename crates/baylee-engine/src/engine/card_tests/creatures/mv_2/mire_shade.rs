//! `cards/creatures/mv_2/mire_shade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "b77a7f4a-1b3a-440e-977c-1ba39e450581"
fn mire_shade() -> CardIndex {
    card_index("b77a7f4a-1b3a-440e-977c-1ba39e450581")
}

#[test]
fn mire_shade_sacrifices_swamp_at_sorcery_speed_for_p1p1_counter() {
    let mut engine = lk::game(SEED, &[mire_shade(), swamp()], &[], &[], &[]);
    let shade = lk::obj(&engine, P0, mire_shade());
    let sw = lk::obj(&engine, P0, swamp());

    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: shade,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![sw] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, shade), (2, 2));
    assert!(on_battlefield(&engine, P0, swamp()).is_none());
}

/// "Sacrifice a Swamp": a Forest is not offered, and only the Swamp can go.
#[test]
fn mire_shade_offers_only_swamps_for_the_sacrifice() {
    let mut engine = lk::game(SEED, &[mire_shade(), swamp(), forest()], &[], &[], &[]);
    let shade = lk::obj(&engine, P0, mire_shade());
    let sw = lk::obj(&engine, P0, swamp());

    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: shade,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!(options, vec![sw], "the Forest is not a Swamp");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![sw] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, shade), (2, 2));
    assert!(on_battlefield(&engine, P0, forest()).is_some());
}

/// "Activate only as a sorcery": offered in the main phase, not in the end step.
#[test]
fn mire_shade_is_not_offered_outside_the_main_phase() {
    let mut engine = lk::game(SEED, &[mire_shade(), swamp()], &[], &[], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    assert!(
        lk::offered(&engine, mire_shade(), P0, 0),
        "offered in the main phase with {{B}} in the pool"
    );

    lk::to_step(&mut engine, P0, Step::End);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    assert!(
        !lk::offered(&engine, mire_shade(), P0, 0),
        "in the end step the ability is not offered"
    );
}
