//! `cards/creatures/mv_3/mistform_dreamer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_dreamer() -> CardIndex {
    card_index("5c679c43-5990-4dcf-a36e-865c17bdea32")
}

/// Mistform Dreamer has flying and `{1}: becomes the chosen creature type
/// until end of turn`, reverting back to Illusion at cleanup.
#[test]
fn mistform_dreamer_has_flying_and_changes_type_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), mistform_dreamer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dreamer = on_battlefield(&engine, p0, mistform_dreamer()).expect("Mistform Dreamer seated");
    assert_eq!(pt(&engine, dreamer), (2, 1));
    assert!(keywords(&engine, dreamer).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_dreamer(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WIZARD))
        .expect("choose Wizard");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(dreamer)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::WIZARD));
    assert!(!subs.contains(subtypes::creature::ILLUSION));
    assert!(keywords(&engine, dreamer).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let subs_after = &engine
        .state()
        .object(dreamer)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs_after.contains(subtypes::creature::ILLUSION));
    assert!(!subs_after.contains(subtypes::creature::WIZARD));
}
