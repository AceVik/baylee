//! `cards/enchantments/mv_4/preferred_selection.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1d05d2d6-3ea1-4d8a-9f1c-6fc3f5e38ea9"
fn preferred_selection() -> CardIndex {
    card_index("1d05d2d6-3ea1-4d8a-9f1c-6fc3f5e38ea9")
}

#[test]
fn preferred_selection_triggers_at_upkeep_and_looks_at_top_two_cards() {
    let mut engine = lk::game(SEED, &[], &[], &[preferred_selection()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, preferred_selection());
    lk::settle(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        unreachable!();
    };
    assert_eq!(options.len(), 2, "looks at the top two cards of library");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    reach_main_phase(&mut engine, P0);

    assert!(on_battlefield(&engine, P0, preferred_selection()).is_some());
}
