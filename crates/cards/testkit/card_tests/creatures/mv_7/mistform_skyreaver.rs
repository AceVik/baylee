//! `cards/creatures/mv_7/mistform_skyreaver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_skyreaver() -> CardIndex {
    card_index("6a291cf5-13cc-4e04-a565-f36650fbe5a1")
}

/// Mistform Skyreaver casts for `{5}{U}{U}` as a 6/6 Flying Illusion, can pay
/// `{1}` to become the chosen creature type until end of turn, and reverts
/// at cleanup.
#[test]
fn mistform_skyreaver_has_flying_and_changes_type_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(island(), 7), &[mistform_skyreaver()], &[]);
    let skyreaver = cast_and_resolve(&mut engine, mistform_skyreaver());
    assert_eq!(pt(&engine, skyreaver), (6, 6));
    assert!(keywords(&engine, skyreaver).contains(KeywordSet::FLYING));
    assert!(
        engine
            .state()
            .object(skyreaver)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_skyreaver(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(skyreaver)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::DRAGON));
    assert!(!subs.contains(subtypes::creature::ILLUSION));
    assert!(keywords(&engine, skyreaver).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        engine
            .state()
            .object(skyreaver)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );
}
