//! `cards/creatures/mv_5/endless_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Endless Wurm` is a 9/9 Wurm costing `{3}{G}{G}` under `Coverage::Implemented` with trample.
/// It prints "At the beginning of your upkeep, sacrifice this creature unless you sacrifice an enchantment."
/// In its controller's upkeep, the trigger fires and prompts `ChoicePrompt::CostSacrifice` over controlled
/// enchantments. Choosing to sacrifice `Fastbond` pays the cost, placing `Fastbond` in the graveyard and
/// preserving `Endless Wurm` on the battlefield.
///
/// The Wurm is seated before the game starts, so the first upkeep it sees is
/// p0's very first one, and the question arrives *before* any main phase —
/// which is why this walks straight to it rather than to a main phase first.
#[test]
fn endless_wurm_sacrifices_enchantment_during_upkeep_to_survive() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[endless_wurm(), fastbond()])
        .start();
    keep_mulligans(&mut engine);

    let wurm = on_battlefield(&engine, p0, endless_wurm()).expect("Endless Wurm is present");
    let enchantment = on_battlefield(&engine, p0, fastbond()).expect("Fastbond is present");
    assert_eq!(pt(&engine, wurm), (9, 9), "printed body is 9/9");
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::TRAMPLE),
        "Endless Wurm has trample"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostSacrifice,
                ..
            }
        )
    });
    assert_eq!(
        (engine.state().turn.active, engine.state().turn.step),
        (p0, Step::Upkeep),
        "\"at the beginning of *your* upkeep\": the question is asked in the \
         controller's own upkeep"
    );

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards prompt for CostSacrifice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the Wurm's controller is the one who may pay");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "prompt is CostSacrifice"
    );
    // No yes-or-no is asked first: naming nothing is how the seat declines.
    assert_eq!((min, max), (0, 1), "sacrificing is optional, at most one");
    assert!(
        options.contains(&enchantment),
        "controlled enchantment is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .expect("sacrificing enchantment to pay the upkeep cost is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, fastbond()).is_none(),
        "Fastbond was sacrificed to pay the upkeep cost"
    );
    assert!(
        in_graveyard(&engine, p0, fastbond()).is_some(),
        "sacrificed enchantment is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, endless_wurm()).is_some(),
        "Endless Wurm survived on the battlefield"
    );
}
