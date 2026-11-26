//! `cards/creatures/artifacts/mv_1/icehide_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn icehide_golem() -> CardIndex {
    card_index("29b925e9-1e98-471c-9b23-e0372ac86485")
}

/// Icehide Golem's {S} cost cannot be paid using a non-snow Forest, and CAN be
/// paid using mana from a Snow-Covered Forest.
#[test]
fn icehide_golem_requires_snow_mana_and_enters_as_2_2() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), index::SNOW_COVERED_FOREST])
        .hand(0, &[icehide_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let normal_forest = on_battlefield(&engine, p0, forest()).unwrap();
    let snow_forest = on_battlefield(&engine, p0, index::SNOW_COVERED_FOREST).unwrap();
    let golem_card = in_hand(&engine, p0, icehide_golem()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: normal_forest,
            },
        )
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        !legal.castable.contains(&golem_card),
        "non-snow mana cannot cast {{S}}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: snow_forest,
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p0, icehide_golem());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let golem = on_battlefield(&engine, p0, icehide_golem()).expect("Icehide Golem entered");
    assert_eq!(pt(&engine, golem), (2, 2));
}
