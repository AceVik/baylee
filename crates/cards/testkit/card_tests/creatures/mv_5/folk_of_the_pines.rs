//! `cards/creatures/mv_5/folk_of_the_pines.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Folk of the Pines is a {4}{G} 2/5 Dryad whose whole printed text is one
/// activated ability: "{1}{G}: This creature gets +1/+0 until end of turn."
/// Both halves of that price are the engine's answer rather than the card's, so
/// the board reads them apart: with an empty pool the line is not offered at
/// all, because `can_afford` reads the pool and not the untapped lands, and
/// once two Forests are really floating the Dryad becomes a 3/5 it never paid
/// its own `{T}` for. The last reading is the printed duration — one turn later
/// the body is a 2/5 again while the creature is still standing.
#[test]
fn folk_of_the_pines_pumps_itself_for_a_green_and_a_generic() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), folk_of_the_pines()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let folk = on_battlefield(&engine, p0, folk_of_the_pines()).expect("the Dryad is on the table");
    assert_eq!(
        pt(&engine, folk),
        (2, 5),
        "a printed 2/5 before anything is activated"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands: with nothing floating the {1}{G} is
    // unpayable, so the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(folk, 0)),
        "{{1}}{{G}} is not two, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Two Forests and nothing else on the board. {1}{G} is a green plus a
    // generic, and a Forest's {G} pays either half, so the two mana in the pool
    // are exactly the price the line charges. The Dryad prints no mana ability
    // of its own, which is why `tap_all_mana` leaves it standing.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "two Forests, and the Dryad is no mana route");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "two green: one for the coloured half of the cost and one for the generic"
    );
    assert_eq!(pool.total(), 2, "and nothing else is in the pool");
    assert!(
        !is_tapped(&engine, folk),
        "the pump costs mana and no {{T}}, so the Dryad is still standing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(folk, 0)),
        "with two green floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, folk_of_the_pines(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability waits on the stack"
    );
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the pump resolves"
    );

    assert_eq!(
        pt(&engine, folk),
        (3, 5),
        "+1/+0 on the creature the ability names — power up, toughness untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} came out of the pool, so both mana that were floating are spent"
    );
    assert!(
        !is_tapped(&engine, folk),
        "and the creature paid none of it with its own tap"
    );

    // "until end of turn" is part of the card: one turn later the Dryad is the
    // 2/5 it was printed as, and it is still standing, so the pump left rather
    // than the creature did.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(
        pt(&engine, folk),
        (2, 5),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, folk_of_the_pines()).is_some(),
        "and the Dryad is still on the table"
    );
}
