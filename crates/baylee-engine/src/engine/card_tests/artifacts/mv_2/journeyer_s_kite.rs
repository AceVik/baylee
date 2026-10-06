//! `cards/artifacts/mv_2/journeyer_s_kite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Journeyer's Kite` prints `{{3}}, {{T}}: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Journeyer's Kite` and three copies of `forest()`.
/// Paying three mana activates the kite without sacrificing it, prompting a library search
/// via `ChoicePrompt::SearchLibrary` and putting the chosen basic land card into hand.
#[test]
fn journeyer_s_kite_searches_basic_land_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), journeyer_s_kite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, Some(journeyer_s_kite()));
    activate(&mut engine, p0, journeyer_s_kite(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert!(!options.is_empty(), "library contains basic land cards");

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("chose basic land");

    pass_until(&mut engine, stack_is_empty);

    let kite = on_battlefield(&engine, p0, journeyer_s_kite()).expect("kite still on battlefield");
    assert!(
        is_tapped(&engine, kite),
        "`Journeyer's Kite` tapped to pay its cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "searched land was placed into hand"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("searched object exists")
            .zone,
        Zone::Hand,
        "searched land is in hand"
    );
}
