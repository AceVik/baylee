//! `cards/lands/utility/eye_of_ugin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn eye_of_ugin_searches_colorless_creature_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, walking_ballista())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[eye_of_ugin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let eye = play_land(&mut engine, p0, eye_of_ugin());
    assert!(!is_tapped(&engine, eye));

    tap_all_mana_but(&mut engine, p0, Some(eye_of_ugin()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        7
    );

    activate(&mut engine, p0, eye_of_ugin(), 0);
    assert!(is_tapped(&engine, eye));

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search");
    };
    assert!(!options.is_empty());
    let chosen = options[0];

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(chosen).unwrap().zone, Zone::Hand);
}

#[test]
fn eye_prices_a_devoid_eldrazi_from_its_stack_characteristics() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[eye_of_ugin()])
        .hand(0, &[index::DROWNER_OF_TRUTH, index::WALKING_BALLISTA])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let state = e.dev_state_mut(p0).unwrap();
    state.players[0].mana_pool.add(ManaColor::Green, 2);
    state.players[0].mana_pool.add(ManaColor::Colorless, 3);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::DROWNER_OF_TRUTH);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        e.state()
            .object(*e.state().zones.list(ZoneLocation::Stack).last().unwrap())
            .unwrap()
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::EMPTY
    );
}
#[test]
fn eye_does_not_discount_a_colorless_non_eldrazi() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[eye_of_ugin()])
        .hand(0, &[index::WALKING_BALLISTA])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_with_floating(&mut e, p0, index::WALKING_BALLISTA);
    assert!(matches!(e.pending(), Pending::ChooseNumber { max: 0, .. }));
}
