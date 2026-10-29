//! `cards/artifacts/mv_2/star_compass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Star Compass — {2} Artifact
/// "This artifact enters tapped.
/// {T}: Add one mana of any color that a basic land you control could produce."
#[test]
fn star_compass_enters_tapped_on_cast() {
    let compass = card_index("b5e9599f-9561-4e3f-9904-9a4dab98cabb");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[compass])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, compass);
    pass_until(&mut engine, stack_is_empty);

    let compass_id = on_battlefield(&engine, p0, compass).expect("Star Compass on board");
    assert!(is_tapped(&engine, compass_id));
}

#[test]
fn star_compass_produces_only_colors_of_basic_lands_you_control() {
    let compass = card_index("b5e9599f-9561-4e3f-9904-9a4dab98cabb");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[compass, island(), mountain(), badlands()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, compass, 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a color choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Blue, ManaColor::Red]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}
