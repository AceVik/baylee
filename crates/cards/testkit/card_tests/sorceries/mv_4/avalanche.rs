//! `cards/sorceries/mv_4/avalanche.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn avalanche() -> CardIndex {
    card_index("a349e526-882c-454f-ba79-ad17340f8153")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

fn snow_covered_island() -> CardIndex {
    card_index("5b2460a5-6ae5-4cad-ba94-1a9e98e6e4c0")
}

/// Avalanche (a349e526-882c-454f-ba79-ad17340f8153) — {X}{2}{R}{R} Sorcery:
/// - "Destroy X target snow lands."
#[test]
fn avalanche_destroys_x_target_snow_lands_and_excludes_nonsnow_lands() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
            ],
        )
        .battlefield(1, &[snow_covered_forest(), snow_covered_island(), island()])
        .hand(0, &[avalanche()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let snow_forest =
        on_battlefield(&engine, P1, snow_covered_forest()).expect("Snow-Covered Forest seated");
    let snow_island =
        on_battlefield(&engine, P1, snow_covered_island()).expect("Snow-Covered Island seated");
    let nonsnow_forest = on_battlefield(&engine, P0, forest()).expect("Forest seated");
    let nonsnow_island = on_battlefield(&engine, P1, island()).expect("Island seated");

    // Tap 6 Mountains (leaving the nonsnow Forest untapped) so X = 2 ({2}{2}{R}{R}).
    tap_all_mana_but(&mut engine, P0, Some(forest()));
    cast_with_floating(&mut engine, P0, avalanche());

    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseNumber for Avalanche, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    engine
        .apply(P0, PlayerAction::ChooseNumber(2))
        .expect("choose X = 2");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseTargets for Avalanche, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (2, 2), "X = 2 requires 2 targets");
    assert!(
        options.contains(&snow_forest) && options.contains(&snow_island),
        "snow lands are legal targets: {options:?}"
    );
    assert!(
        !options.contains(&nonsnow_forest) && !options.contains(&nonsnow_island),
        "nonsnow lands are excluded: {options:?}"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![snow_forest, snow_island],
                players: vec![],
            },
        )
        .expect("target both snow lands");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, snow_covered_forest()).is_none());
    assert!(on_battlefield(&engine, P1, snow_covered_island()).is_none());
    assert!(in_graveyard(&engine, P1, snow_covered_forest()).is_some());
    assert!(in_graveyard(&engine, P1, snow_covered_island()).is_some());
    assert!(
        on_battlefield(&engine, P1, island()).is_some(),
        "untouched nonsnow land remains"
    );
}
