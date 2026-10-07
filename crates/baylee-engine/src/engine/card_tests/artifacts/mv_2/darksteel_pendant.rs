//! `cards/artifacts/mv_2/darksteel_pendant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Darksteel Pendant` prints `Indestructible` and `{{1}}, {{T}}: Scry 1.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Darksteel Pendant` and a `forest()`.
/// The artifact possesses `KeywordSet::INDESTRUCTIBLE`. Floating one mana activates the ability,
/// which presents a scry arrangement (`ArrangePrompt::Scry`) to inspect the top card of the library.
#[test]
fn darksteel_pendant_has_indestructible_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), darksteel_pendant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pendant =
        on_battlefield(&engine, p0, darksteel_pendant()).expect("pendant is on battlefield");
    assert!(
        keywords(&engine, pendant).contains(KeywordSet::INDESTRUCTIBLE),
        "`Darksteel Pendant` has indestructible"
    );

    tap_all_mana_but(&mut engine, p0, Some(darksteel_pendant()));
    activate(&mut engine, p0, darksteel_pendant(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        cards,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(piles, scry_piles(1));

    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("kept card on top");

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, pendant), "`Darksteel Pendant` is tapped");
}
