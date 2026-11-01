//! `cards/artifacts/mv_2/jeweled_torque.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "91f36a03-6031-4834-98ee-20b36cdf5569"
fn jeweled_torque() -> CardIndex {
    card_index("91f36a03-6031-4834-98ee-20b36cdf5569")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// `Jeweled Torque` (`Coverage::Generated`):
/// "As this artifact enters, choose a color.
/// Whenever a player casts a spell of the chosen color, you may pay {2}. If you do, you gain 2 life."
///
/// Verifies choosing Red on entry, paying `{2}` on a Red spell to gain 2 life,
/// declining payment to gain 0 life, and ignoring spells of other colors.
#[test]
fn jeweled_torque_gains_two_life_when_paying_for_chosen_color_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9401, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[plains()])
        .hand(0, &[jeweled_torque(), shock(), shock()])
        .hand(1, &[savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, jeweled_torque());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Red spell #1: pay {2} -> gain 2 life (20 -> 22).
    cast_from_hand(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "paying {{2}} for Jeweled Torque gains 2 life"
    );

    // Red spell #2: decline to pay -> gain 0 life (stays 22).
    cast_from_hand(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "declining to pay {{2}} gains no life"
    );

    // Non-Red spell (Savannah Lions) on P1's turn does not trigger Jeweled Torque.
    assert!(walk_to_own_main(&mut engine, p1));
    cast_from_hand(&mut engine, p1, savannah_lions());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "white spell does not trigger Red Jeweled Torque"
    );
}
