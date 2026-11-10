//! `cards/creatures/mv_2/nantuko_shade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nantuko Shade is a {B}{B} 2/1 with one printed line: "{B}: This creature
/// gets +1/+1 until end of turn." Three activations out of three Swamps are
/// what separate a repeatable pump from a one-shot — 2/1, then 3/2, 4/3, 5/4
/// — while the Llanowar Elves beside it stay a printed 1/1, which is
/// `Filter::This` being read and not "creatures you control".
///
/// The empty pool before the first activation is the other half, because
/// `can_afford` reads the pool rather than the untapped Swamps: with nothing
/// floating the card offers nothing at all. And the pump has to *lapse* —
/// `Duration::UntilEndOfTurn` ends it in the next cleanup (CR 514.2) — or the
/// Shade would be a 5/4 for the rest of the game.
#[test]
fn nantuko_shade_pumps_itself_one_black_at_a_time_and_lets_the_pump_lapse() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), llanowar_elves(), nantuko_shade()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shade = on_battlefield(&engine, p0, nantuko_shade()).expect("the Shade is on the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, shade), (2, 1), "the printed 2/1");

    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(src, _)| *src == shade)
        )
    };
    assert!(
        !offered(&engine),
        "`can_afford` reads the pool and not the untapped Swamps, so with \
         nothing floating the one line the card prints is absent from the offer"
    );

    // Three Swamps, and the Elf kept back: a mana creature tapped for mana is
    // a creature whose status changed for a reason of its own, and this is
    // the creature the static beside it has to leave alone.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps, three black, and nothing off the Elf"
    );
    assert!(
        offered(&engine),
        "with {{B}} floating the ability is on offer"
    );

    activate(&mut engine, p0, nantuko_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, shade), (3, 2), "one {{B}} is +1/+1, and stacks");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "one black out of the three was the price"
    );

    activate(&mut engine, p0, nantuko_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, shade),
        (4, 3),
        "a second activation on the same creature: +2/+2, not a fresh pump"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "a second black out of the pool"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "`Filter::This`: the creature beside the Shade never moves"
    );

    activate(&mut engine, p0, nantuko_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, shade), (5, 4), "and a third");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three activations spent the whole pool"
    );
    assert!(
        !offered(&engine),
        "and an empty pool is no offer again, so a fourth activation cannot \
         be had for free"
    );

    // `Duration::UntilEndOfTurn`: the next cleanup ends it (CR 514.2).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, shade),
        (2, 1),
        "past the end of the turn the Shade is the printed 2/1 again"
    );
}
