//! `cards/enchantments/mv_1/need_for_speed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Need for Speed` (`Coverage::Implemented`):
/// "Sacrifice a land: Target creature gains haste until end of turn."
///
/// Verifies that activating `Need for Speed` requires sacrificing a land and grants
/// `KeywordSet::HASTE` to the targeted creature until end of turn.
#[test]
fn need_for_speed_sacrifices_land_to_grant_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1322, forest())
        .battlefield(0, &[need_for_speed(), mountain(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("mountain deployed");
    assert!(!keywords(&engine, elf).contains(KeywordSet::HASTE));

    activate(&mut engine, p0, need_for_speed(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Need for Speed, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf), "elf is a legal creature target");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&my_mountain),
        "controlled land is offered to sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_mountain],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::HASTE),
        "target creature gained haste"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "sacrificed land is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_none(),
        "sacrificed land left the battlefield"
    );
}
