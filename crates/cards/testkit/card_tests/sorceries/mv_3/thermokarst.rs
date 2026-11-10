//! Thermokarst — `{1}{G}{G}` Sorcery (Ice Age)
//!
//! Destroy target land. If that land was a snow land, you gain 1 life.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thermokarst() -> CardIndex {
    card_index("f3b071a1-7501-4424-979a-780fa90eecb8")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

#[test]
fn thermokarst_gains_life_for_snow_land_and_not_for_regular_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thermokarst(), thermokarst()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[snow_covered_forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let snow_land = on_battlefield(&engine, p1, snow_covered_forest()).unwrap();
    let regular_land = on_battlefield(&engine, p1, forest()).unwrap();

    // Branch 1: destroy a snow land -> destroyed and gain 1 life.
    let tk1 = in_hand(&engine, p0, thermokarst()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: tk1 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![snow_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(snow_land).unwrap().zone,
        Zone::Graveyard,
        "snow land is destroyed"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "destroying a snow land gains 1 life"
    );

    // Branch 2: destroy a non-snow land -> destroyed and gain 0 life.
    let tk2 = in_hand(&engine, p0, thermokarst()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: tk2 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![regular_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(regular_land).unwrap().zone,
        Zone::Graveyard,
        "regular land is destroyed"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "destroying a non-snow land gains no life"
    );
}
