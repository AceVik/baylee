//! `cards/creatures/artifacts/mv_4/telethopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Telethopter` prints `Tap an untapped creature you control: This creature gains flying until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Telethopter` and an untapped `llanowar_elves()`.
/// Activating the ability prompts with `ChoicePrompt::CostTap` to tap the elf as the activation cost.
/// Upon resolution, `Telethopter` gains `KeywordSet::FLYING` until end of turn while the elf remains tapped.
#[test]
fn telethopter_taps_an_untapped_creature_to_gain_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[telethopter(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thopter = on_battlefield(&engine, p0, telethopter()).expect("telethopter is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is seated");

    assert!(!is_tapped(&engine, elf), "elf starts untapped");
    assert!(
        !keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "telethopter does not have flying initially"
    );

    activate(&mut engine, p0, telethopter(), 0);
    let Pending::ChooseCards {
        options,
        prompt,
        player,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostTap prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert!(options.contains(&elf), "the elf is offered to be tapped");

    engine
        .apply(player, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("tapping elf pays the cost");

    assert!(is_tapped(&engine, elf), "the elf is tapped to pay the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "`Telethopter` has flying after ability resolves"
    );
}
