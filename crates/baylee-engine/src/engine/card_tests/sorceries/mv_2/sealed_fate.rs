//! `cards/sorceries/mv_2/sealed_fate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sealed_fate() -> CardIndex {
    card_index("0b9b5856-9396-4a55-8675-58cd171df708")
}

/// Sealed Fate (`{X}{U}{B}`, `Coverage::Partial`): casts with X targeting an
/// opponent and looks at the top card of that opponent's library. The exile of
/// one of the looked-at cards is not played, so nothing moves: the card stays
/// on top of the library it was looked at in.
#[test]
fn sealed_fate_casts_with_x_and_looks_at_opponents_top_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp(), forest()])
        .hand(0, &[sealed_fate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let their_top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .expect("the opponent has a library");
    let their_library_size = engine.state().zones.list(ZoneLocation::Library(p1)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, sealed_fate());
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(
        engine.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err(),
        "\"target opponent\": the caster is not a legal target"
    );
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p0, "the caster is the one who looks");
    assert_eq!(
        options,
        vec![their_top],
        "the card shown is the top card of the opponent's library, not the caster's"
    );
    assert_eq!(prompt, crate::choice::ChoicePrompt::LookAtLibrary);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sealed_fate()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).len(),
        their_library_size,
        "NOT SUPPORTED: nothing is exiled, the library keeps every card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).last(),
        Some(&their_top),
        "the looked-at card stays on top"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .is_empty(),
        "NOT SUPPORTED: the exile of one of the looked-at cards is not played"
    );
}
