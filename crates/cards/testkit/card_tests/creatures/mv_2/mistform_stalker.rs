//! `cards/creatures/mv_2/mistform_stalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_stalker() -> CardIndex {
    card_index("0f9c4b56-7ad7-4262-8079-3eee5f0cdd3b")
}

/// Mistform Stalker can change its creature type for `{1}` until end of turn
/// and get +2/+2 and flying for `{2}{U}{U}` until end of turn, with both
/// effects expiring at cleanup.
#[test]
fn mistform_stalker_changes_type_and_pumps_with_flying_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                mistform_stalker(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stalker = on_battlefield(&engine, p0, mistform_stalker()).expect("Mistform Stalker seated");
    assert_eq!(pt(&engine, stalker), (1, 1));
    assert!(!keywords(&engine, stalker).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_stalker(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::GOBLIN))
        .expect("choose Goblin");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(stalker)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::GOBLIN));
    assert!(!subs.contains(subtypes::creature::ILLUSION));

    activate(&mut engine, p0, mistform_stalker(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, stalker), (3, 3));
    assert!(keywords(&engine, stalker).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, stalker), (1, 1));
    assert!(!keywords(&engine, stalker).contains(KeywordSet::FLYING));
    assert!(
        engine
            .state()
            .object(stalker)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );
}
