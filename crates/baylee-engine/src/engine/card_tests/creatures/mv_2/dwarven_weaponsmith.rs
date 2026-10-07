//! `cards/creatures/mv_2/dwarven_weaponsmith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dwarven Weaponsmith — "{T}, Sacrifice an artifact: Put a +1/+1 counter on
/// target creature. Activate only during your upkeep." The window is a
/// restriction on beginning the activation (CR 602.5): the same ability is
/// absent in a main phase and in the opponent's upkeep, and present in its
/// controller's. The target is any creature; the price is one artifact of
/// this seat's, paid after the target is named (CR 601.2c, then CR 601.2h).
#[test]
fn dwarven_weaponsmith_trades_an_artifact_for_a_counter_only_in_your_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[dwarven_weaponsmith(), darksteel_pendant(), llanowar_elves()],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let smith = on_battlefield(&engine, p0, dwarven_weaponsmith()).expect("seated");
    let pendant = on_battlefield(&engine, p0, darksteel_pendant()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    assert!(
        !priority_offer(&engine).abilities.contains(&(smith, 0)),
        "a main phase is not \"your upkeep\""
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        !priority_offer(&engine).abilities.contains(&(smith, 0)),
        "an opponent's upkeep is not yours either"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        priority_offer(&engine).abilities.contains(&(smith, 0)),
        "the window is the controller's own upkeep"
    );

    activate(&mut engine, p0, dwarven_weaponsmith(), 0);
    let options = aim_at(&mut engine, p0, elf);
    assert!(
        options.contains(&elf) && options.contains(&smith),
        "any creature may receive the counter: {options:?}"
    );
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost names an artifact and not a particular one, so it asks which: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(prompt, ChoicePrompt::CostSacrifice, "a price, not a search");
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert_eq!(
        options,
        vec![pendant],
        "the only artifact this seat controls"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![pendant],
            },
        )
        .expect("the offered artifact pays the price");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "a +1/+1 counter on the target"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "the Elf grew");
    assert!(is_tapped(&engine, smith), "{{T}} is half the price");
    assert!(
        in_graveyard(&engine, p0, darksteel_pendant()).is_some(),
        "and the artifact is the other half"
    );
}
