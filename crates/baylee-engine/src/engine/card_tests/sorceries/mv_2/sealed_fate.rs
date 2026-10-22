//! `cards/sorceries/mv_2/sealed_fate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sealed_fate() -> CardIndex {
    card_index("0b9b5856-9396-4a55-8675-58cd171df708")
}

/// Sealed Fate (`{X}{U}{B}`, `Coverage::Partial`): casts with X targeting an
/// opponent and looks at the top card of that opponent's library.
#[test]
fn sealed_fate_casts_with_x_and_looks_at_opponents_top_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp(), forest()])
        .hand(0, &[sealed_fate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, sealed_fate());
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(options.len(), 1);
    assert_eq!(prompt, crate::choice::ChoicePrompt::LookAtLibrary);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sealed_fate()).is_some());
}
