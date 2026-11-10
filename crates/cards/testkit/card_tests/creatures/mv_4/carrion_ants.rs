//! `cards/creatures/mv_4/carrion_ants.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Carrion Ants prints one line — "{{1}}: This creature gets +1/+1 until end
/// of turn" — on a printed 0/1 body, so the whole card is a pump that is
/// repeatable and costs mana. Two activations out of one pool are what reads
/// that: the body goes 0/1 → 1/2 → 2/3, which is a *staple* of +1/+1 and not
/// a single set-to, and the Elf standing beside it stays a printed 1/1 so
/// `Filter::This` is read rather than assumed. The third read is the price:
/// with the pool emptied the same line is no longer offered, which is how a
/// real {{1}} differs from a label on a free ability.
#[test]
fn carrion_ants_pumps_itself_for_one_mana_each_time_and_only_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[carrion_ants()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Swamps pay the {2}{B}{B} and leave two black behind — the one mana
    // for each of the first two activations. The Elf is named as the printing
    // kept back, so "six" is the lands and the creature beside the Ants is
    // neither a source nor a tap.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Swamps tapped and no creature paid in"
    );
    cast_with_floating(&mut engine, p0, carrion_ants());
    pass_until(&mut engine, stack_is_empty);
    let ants = on_battlefield(&engine, p0, carrion_ants()).expect("the Ants resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, ants), (0, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cast's four mana is spent and exactly two black is left"
    );

    // Mana before the claim: the ability's price is read off the pool, not off
    // the untapped lands, and ability 0 is the only line the card prints.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ants, 0)),
        "with two black floating the pump is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, carrion_ants(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ants), (1, 2), "one activation, one +1/+1");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} came out of the pool"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the static reaches the creature it is printed on and no other"
    );

    activate(&mut engine, p0, carrion_ants(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, ants),
        (2, 3),
        "a second activation stacks on the first: the pump is repeatable"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the two activations emptied is empty"
    );

    // The price, read where the engine reads it: nothing floats, so the same
    // line is withheld from the offer rather than refused after the press.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ants, 0)),
        "{{1}} is not zero: on an empty pool the pump is not offered at all: {:?}",
        legal.abilities
    );
}
