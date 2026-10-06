//! `cards/lands/utility/eye_of_ugin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eye of Ugin prints `Colorless Eldrazi spells you cast cost {2} less to cast` and `{7}, {T}: Search
/// your library for a colorless creature card, reveal it, put it into your hand, then shuffle.`
/// The card is marked `Coverage::Partial` because static spell cost reductions are not expressible.
/// Backed by a library of `walking_ballista` cards and seven `forest` lands providing seven floating mana,
/// activating ability index 0 searches a colorless creature into hand and taps Eye of Ugin.
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
