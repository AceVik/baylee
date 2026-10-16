//! `cards/artifacts/mv_4/arcane_spyglass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn arcane_spyglass() -> CardIndex {
    card_index("c2fbdc80-ad5d-4fe0-a860-3e4f452e6302")
}

/// "{2}, {T}, Sacrifice a land: Draw a card and put a charge counter on this
/// artifact." The land is chosen as a cost; the draw and the counter follow.
#[test]
fn sacrificing_a_land_draws_and_charges_the_spyglass() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcane_spyglass(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spyglass = on_battlefield(&engine, p0, arcane_spyglass()).expect("out");
    let lands_before = all_on_battlefield(&engine, p0, forest()).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, arcane_spyglass(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the land to sacrifice is asked for, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(
        !options.contains(&spyglass),
        "the Spyglass is no land: {options:?}"
    );
    let land = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("a listed land");
    assert!(is_tapped(&engine, spyglass), "{{T}} is part of the price");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        lands_before - 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a card was drawn"
    );
    assert_eq!(counters_on(&engine, spyglass, CounterKind::Charge), 1);
}

/// "Remove three charge counters from this artifact: Draw a card." Two
/// counters are not enough; three are, and they all go.
#[test]
fn three_charge_counters_buy_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcane_spyglass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spyglass = on_battlefield(&engine, p0, arcane_spyglass()).expect("out");

    set_counters(&mut engine, spyglass, CounterKind::Charge, 2);
    assert!(
        !priority_offer(&engine).abilities.contains(&(spyglass, 1)),
        "two counters cannot pay for three"
    );
    set_counters(&mut engine, spyglass, CounterKind::Charge, 3);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, arcane_spyglass(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, spyglass, CounterKind::Charge), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert!(!is_tapped(&engine, spyglass), "this line has no {{T}}");
}
