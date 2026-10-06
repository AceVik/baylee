//! `cards/creatures/mv_4/firescreamer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "b86c0c23-bb29-4af0-bbd4-50b3ae634375"

/// Firescreamer is a {3}{B} 2/2 Kavu whose whole text is "{R}: This creature
/// gets +1/+0 until end of turn." Four Swamps pay the cast down to an empty
/// pool, which is what makes the offer read before the Mountain is tapped a
/// control rather than a board that never had the red — and the single
/// Mountain then pays the {R}, so the (3, 2) that comes back is one point of
/// power for one mana and no toughness at all. The Elf across the table is the
/// other half of `Filter::This`, and walking a turn afterwards is what tells
/// the printed "until end of turn" from a counter the creature keeps.
#[test]
fn firescreamer_spends_red_for_one_power_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[firescreamer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Swamps and only those: the Mountain is kept back because it is the
    // one source of {R} on this board and the price of the ability below.
    let peak = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    tap_mana_except(&mut engine, p0, peak);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, four black: the Mountain is the ability's price and not the cast's"
    );
    cast_with_floating(&mut engine, p0, firescreamer());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let kavu = on_battlefield(&engine, p0, firescreamer()).expect("the Kavu resolved");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{B}} took the whole pool"
    );
    assert_eq!(pt(&engine, kavu), (2, 2), "the printed body");

    // `can_afford` reads the pool and not the untapped Mountain, so with
    // nothing floating the line is not there at all — the half a test that
    // only ever taps first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(kavu, 0)),
        "{{R}} is not red, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // The Mountain is the only untapped source left, so this is exactly one
    // red and the offer can now be read where the engine reads it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one Mountain, one red, and no land beside it still standing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kavu, 0)),
        "with {{R}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, firescreamer(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, kavu),
        (2, 2),
        "and nothing has been pumped while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, kavu),
        (3, 2),
        "+1/+0 is one point of power and no toughness"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "`Filter::This` reaches the Kavu and never across the table"
    );

    // "until end of turn": a turn later the Kavu is a printed 2/2 again, so
    // the +1/+0 was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, kavu),
        (2, 2),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, firescreamer()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
