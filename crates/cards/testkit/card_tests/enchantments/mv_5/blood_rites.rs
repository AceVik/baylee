//! `cards/enchantments/mv_5/blood_rites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Blood Rites` is an enchantment costing `{3}{R}{R}` under `Coverage::Implemented`.
/// It prints "{1}{R}, Sacrifice a creature: This enchantment deals 2 damage to any target."
/// Under CR 601.2c and CR 601.2h, activating the ability prompts for the target first
/// and then prompts with `ChoicePrompt::CostSacrifice` to sacrifice a controlled creature.
/// Upon resolution, the 2 damage destroys an opponent's 2/2 creature (such as `Desert Drake`).
#[test]
fn blood_rites_sacrifices_creature_to_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[blood_rites(), llanowar_elves(), mountain(), mountain()],
        )
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls Llanowar Elves");
    assert_eq!(pt(&engine, drake), (2, 2));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, blood_rites(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Blood Rites, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&drake),
        "opponent's creature is an offered target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drake],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt,
        options: sacrifice_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected CostSacrifice prompt for Blood Rites, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "prompt is CostSacrifice"
    );
    assert!(
        sacrifice_options.contains(&elf),
        "Llanowar Elves is offered as sacrifice: {sacrifice_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "sacrificed creature is in p0's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, desert_drake()).is_some(),
        "damaged creature was destroyed and placed in p1's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, desert_drake()).is_none(),
        "target creature is no longer on the battlefield"
    );
}
