//! `cards/creatures/mv_4/hoar_shade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hoar Shade is a printed 1/2 Shade whose whole text is one line:
/// "{B}: This creature gets +1/+1 until end of turn."
///
/// Six Swamps make every part of that sentence readable off the board: the
/// `{3}{B}` brings the Shade in and leaves exactly two black, each activation
/// spends one of them for one point of power *and* toughness (`(1, 2)` becomes
/// `(2, 3)` and then `(3, 4)` — a pump that had read only the power would leave
/// the toughness at two), and the price is the mana alone, so the Shade is
/// still standing afterwards. The empty pool at the end says where the offer
/// comes from, because `can_afford` reads the pool rather than the untapped
/// Swamps, and the turn boundary is what tells "until end of turn" from a
/// counter: the Shade is the printed 1/2 again while the card is still on the
/// battlefield.
#[test]
fn hoar_shade_pumps_a_black_at_a_time_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hoar_shade()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Six Swamps tapped: {3}{B} for the creature, and one black per printed
    // activation is what is left beside it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        6,
        "six Swamps tapped for six black"
    );
    cast_with_floating(&mut engine, p0, hoar_shade());
    pass_until(&mut engine, stack_is_empty);
    let shade = on_battlefield(&engine, p0, hoar_shade()).expect("the Shade resolved");
    assert_eq!(pt(&engine, shade), (1, 2), "the printed 1/2 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}}{{B}} is spent and exactly two black is left to pump with"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // rather than the untapped lands — so the claim is made with the mana
    // already floating, where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shade, 0)),
        "with two black in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, hoar_shade(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability (CR 605.1), so it uses the stack"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the {{B}} is the price and is paid as the ability is activated"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, shade),
        (2, 3),
        "one black, one +1/+1 — and +1/+1 is both halves, not just the power"
    );
    assert!(
        !is_tapped(&engine, shade),
        "the price is the mana alone: the Shade was never tapped for its own pump"
    );

    // The same line again: a Shade is paid for as often as the mana lasts, so
    // the second activation has to land on top of the first rather than
    // replacing it.
    activate(&mut engine, p0, hoar_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, shade),
        (3, 4),
        "and the second black is a second +1/+1 on the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "six black: four for the cast and one for each activation"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(shade, 0)),
        "an empty pool pays no {{B}} for a price that is only {{B}}, so the line \
         drops out of the offer — and there are five untapped Swamps standing, \
         which is exactly what `can_afford` does not read: {:?}",
        legal.abilities
    );

    // "Until end of turn" is a duration and not a counter. The Shade is still
    // on the battlefield a turn later and its body is the printed one again,
    // so the two grants expired rather than the creature having gone anywhere.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, hoar_shade()).is_some(),
        "the Shade is still standing, so the pump left rather than the creature"
    );
    assert_eq!(
        pt(&engine, shade),
        (1, 2),
        "the two +1/+1 grants lasted the turn they were made in and no longer"
    );
}
