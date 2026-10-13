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

/// "Colorless Eldrazi spells you cast cost {2} less": a colored Eldrazi
/// spell costs its full price. Crib Swap is white and has changeling, so it
/// is every creature type, Eldrazi included.
#[test]
fn eye_does_not_discount_a_colored_eldrazi_spell() {
    use crate::engine::card_tests::legends_kit as kit;
    let p0 = PlayerId::new(0);
    let mut e = kit::game(
        1300,
        &[eye_of_ugin()],
        &[llanowar_elves()],
        &[crib_swap()],
        &[],
    );
    let card = in_hand(&e, p0, crib_swap()).unwrap();
    assert!(
        e.state()
            .object(card)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELDRAZI),
        "Crib Swap is an Eldrazi spell, by changeling"
    );
    kit::float(&mut e, p0, &[(ManaColor::White, 1)]);
    assert!(
        !kit::castable(&e, p0, crib_swap()),
        "{{2}}{{W}} stays {{2}}{{W}}: one white is not enough, so Eye of Ugin took nothing off a white Eldrazi"
    );
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
    assert!(
        kit::castable(&e, p0, crib_swap()),
        "{{2}}{{W}} paid in full is castable"
    );
}

/// "…spells you cast": the opponent's colorless Eldrazi spell is priced in
/// full while our Eye stands.
#[test]
fn eye_does_not_discount_the_opponents_colorless_eldrazi_spell() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let p1 = PlayerId::new(1);
    let mut e = kit::game(1301, &[eye_of_ugin()], &[], &[], &[index::DROWNER_OF_TRUTH]);
    reach_their_main_phase(&mut e, p1);
    // Drowner of Truth is {5}{G/U}{G/U}: five mana would be enough only if
    // the Eye made it cheaper for the opponent too.
    kit::float(
        &mut e,
        p1,
        &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
    );
    assert!(
        !kit::castable(&e, p1, index::DROWNER_OF_TRUTH),
        "our Eye of Ugin does not make the opponent's colorless Eldrazi cost {{2}} less"
    );
    kit::float(&mut e, p1, &[(ManaColor::Colorless, 2)]);
    assert!(
        kit::castable(&e, p1, index::DROWNER_OF_TRUTH),
        "at its full seven mana it is castable"
    );
}
