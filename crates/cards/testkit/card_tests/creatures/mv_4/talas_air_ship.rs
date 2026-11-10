//! `cards/creatures/mv_4/talas_air_ship.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "8cb3f047-bd8b-4a34-8a2b-ced2cc0487eb"

/// Talas Air Ship is `{3}{U}` for a 3/2 Human Pirate whose whole text is
/// "Flying". Nothing about that sentence can be read off the card file, so the
/// board is built to make each half of it falsifiable in one cast: four Islands
/// and no more, so the `{3}{U}` is a payment the pool actually had to supply,
/// and a Llanowar Elves beside them as the control for the keyword — a 1/1
/// ground creature under the same seat is offered the board and must not be
/// flying. `(3, 2)` is the only body that reads both printed numbers: a `(3, 3)`
/// would be a toughness the card never had, and a `(0, 0)` a card whose numbers
/// were never applied.
#[test]
fn talas_air_ship_lands_as_a_three_two_flier_for_the_four_mana_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[talas_air_ship()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the control on this board is a ground creature, so `flying` on the \
         Ship below is a reading and not a board-wide grant"
    );

    // The four Islands are exactly the `{3}{U}`, and the Elf is named as the
    // printing kept back: tapping it for its own `{T}: Add {G}` would put a
    // green mana in the pool beside the four blue and leave the creature the
    // keyword is measured against out of the board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands in the pool, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, talas_air_ship());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, talas_air_ship()).is_some()
    });

    let ship = on_battlefield(&engine, p0, talas_air_ship()).expect("the Ship resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed {{3}}{{U}} came out of the pool: an empty pool afterwards \
         is what says the four Islands were the price and not scenery"
    );
    assert_eq!(pt(&engine, ship), (3, 2), "the body the card prints");
    assert!(
        keywords(&engine, ship).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "and it reaches only the creature that prints it"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0,
        "no blue is left floating either, so the cast spent what it was given"
    );
}
