//! `cards/artifacts/mv_5/gilded_lotus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Gilded Lotus` is an artifact costing `{5}` under `Coverage::Implemented`.
/// It prints "{T}: Add three mana of any one color."
/// When its mana ability is activated, the engine prompts for a color choice across all mana colors.
/// Choosing blue adds exactly three blue mana to the pool and taps the artifact.
#[test]
fn gilded_lotus_taps_for_three_mana_of_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gilded_lotus()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lotus =
        on_battlefield(&engine, p0, gilded_lotus()).expect("Gilded Lotus is on the battlefield");
    assert!(!is_tapped(&engine, lotus), "Gilded Lotus starts untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana pool starts empty"
    );

    activate(&mut engine, p0, gilded_lotus(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt for Gilded Lotus, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is among offered mana colors: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing blue mana is legal");

    assert!(is_tapped(&engine, lotus), "Gilded Lotus tapped for mana");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 3, "three mana added in total");
    assert_eq!(
        pool.available(ManaColor::Blue),
        3,
        "three blue mana available in pool"
    );
}
