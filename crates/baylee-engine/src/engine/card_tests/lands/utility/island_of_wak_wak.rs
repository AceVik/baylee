//! `cards/lands/utility/island_of_wak_wak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn island_of_wak_wak() -> CardIndex {
    card_index("d427e61d-5b30-4d2a-bad2-2e7f016036ca")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Island of Wak-Wak: "{T}: Target creature with flying has base power 0
/// until end of turn."
///
/// Read from the printed text alone. Base power is set (CR 613.4b, layer 7b)
/// and toughness is not mentioned, so a 4/4 Serra Angel becomes 0/4; a
/// +3/+3 pump applies after (layer 7c) and lands on the new base, whichever
/// of the two came first. The menu is "creature with flying", so a ground
/// creature is not on it, and "until end of turn" returns the 4/4 on the next
/// turn (CR 514.2).
#[test]
fn island_of_wak_wak_sets_a_fliers_base_power_to_zero_and_leaves_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island_of_wak_wak(), forest()])
        .hand(0, &[giant_growth()])
        .battlefield(1, &[serra_angel(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, angel), (4, 4), "printed 4/4");

    activate(&mut engine, p0, island_of_wak_wak(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&angel), "a flier is a legal target");
    assert!(
        !options.contains(&elves),
        "a creature without flying is not: {options:?}"
    );
    aim_at(&mut engine, p0, angel);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, angel),
        (0, 4),
        "base power 0, toughness untouched"
    );

    // Layer 7c adds on top of the 7b base: +3/+3 is 3/7, not 7/7.
    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, angel), (3, 7), "0 + 3 power, 4 + 3 toughness");

    // Until end of turn: the next turn the Angel is itself again.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, angel),
        (4, 4),
        "both effects ended with the turn"
    );
}

/// The pump first, the Wak-Wak second: layers, not timestamps, decide
/// (CR 613.1, 613.4), so the result is the same 3/7.
#[test]
fn island_of_wak_wak_applied_after_a_pump_still_sets_base_not_total_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island_of_wak_wak(), forest()])
        .hand(0, &[giant_growth()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is out");

    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, angel), (7, 7), "pumped first");

    activate(&mut engine, p0, island_of_wak_wak(), 0);
    aim_at(&mut engine, p0, angel);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, angel),
        (3, 7),
        "base 0 under the +3/+3 that was already there"
    );
}
