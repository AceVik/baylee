//! `cards/artifacts/mv_1/wayfarer_s_bauble.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wayfarer's Bauble` prints `{{2}}, {{T}}, Sacrifice this artifact: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Wayfarer's Bauble` and two copies of `forest()`.
/// Activating the bauble sacrifices it, searches the library for a basic forest via `ChoicePrompt::SearchLibrary`,
/// and puts that forest onto the battlefield tapped.
#[test]
fn wayfarer_s_bauble_fetches_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), wayfarer_s_bauble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(wayfarer_s_bauble()));
    activate(&mut engine, p0, wayfarer_s_bauble(), 0);

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

    assert!(
        in_graveyard(&engine, p0, wayfarer_s_bauble()).is_some(),
        "`Wayfarer's Bauble` was sacrificed"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("searched land exists")
            .zone,
        Zone::Battlefield,
        "searched land is on battlefield"
    );
    assert!(is_tapped(&engine, chosen), "searched land entered tapped");
}
