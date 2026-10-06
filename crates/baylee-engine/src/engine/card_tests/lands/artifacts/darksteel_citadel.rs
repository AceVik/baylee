//! `cards/lands/artifacts/darksteel_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Darksteel Citadel's whole card is its type line plus two words: an
/// **artifact land** that is indestructible and taps for `{C}`. Both halves
/// need a board that could fail them, so they share one play. The colourless
/// in the pool can only have come off the Citadel — it is the only permanent
/// its controller has — and it arrives through the printed `{T}: Add {C}`
/// rather than a basic land type, which is the difference between this card
/// and a Forest. Then an opponent's Vindicate ("destroy target permanent") is
/// aimed at it, and the permanent has to still be standing while the Vindicate
/// itself lies in a graveyard: a spell that was refused or countered would
/// leave the land alone for a reason that has nothing to do with the keyword.
#[test]
fn darksteel_citadel_taps_for_colorless_and_shrugs_off_a_destroy_effect() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[darksteel_citadel()])
        .battlefield(1, &[plains(), plains(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played rather than seeded: `starting_battlefield` places a permanent
    // without an entry, and the entry is what leaves it standing untapped.
    let citadel = play_land(&mut engine, p0, darksteel_citadel());
    let kinds = types(&engine, citadel);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land is both types at once: {kinds:?}"
    );
    assert!(
        !is_tapped(&engine, citadel),
        "and it is no tapland: it arrives standing, which is what lets it pay below"
    );

    // One permanent, so whatever reaches the pool came off the Citadel's own
    // `{T}: Add {C}` — it has no basic land type for CR 305.6 to shortcut.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "one permanent, one mana route");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(
        pool.total(),
        1,
        "and nothing else was on the board to add to it"
    );
    assert!(is_tapped(&engine, citadel), "which tapped the source");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // Across the table, and a destroy effect whose entire text is "destroy
    // target permanent".
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Vindicate targets a permanent, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&citadel),
        "\"target permanent\" reaches an artifact land: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![citadel],
            },
        )
        .expect("the target the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, darksteel_citadel()).is_some(),
        "indestructible (CR 702.12b): a destroy effect leaves it where it stands"
    );
    assert!(
        in_graveyard(&engine, p0, darksteel_citadel()).is_none(),
        "and nothing put it in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the Vindicate resolved rather than being refused, so the survival \
         above is the keyword and not a spell that never happened"
    );
}
