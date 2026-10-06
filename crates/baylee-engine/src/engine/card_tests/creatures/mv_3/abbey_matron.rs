//! `cards/creatures/mv_3/abbey_matron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Abbey Matron prints two lines: a 1/3 for {2}{W} and "{W}, {T}: This
/// creature gets +0/+3 until end of turn." Both numbers of the pump are
/// readable thereby — a 1/6 after activation rules out a "+3/+0" or
/// a "+0/+0", and the 1/3 before it says that the printed body does not
/// arrive already pumped. The cost is {W} **and** its own tap symbol,
/// so `tap_all_mana` does not tap the card along with it (#159): one half
/// is the white mana that has disappeared from the pool after activation,
/// the other the tapped Cleric. The Goblin next to it is the control for
/// `Filter::This` — a pump without a filter would have dragged it along.
#[test]
fn abbey_matron_pumps_only_herself_for_white_and_a_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), abbey_matron(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let matron = on_battlefield(&engine, p0, abbey_matron()).expect("the Matron is out");
    let bystander = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin is out");
    assert_eq!(
        pt(&engine, matron),
        (1, 3),
        "a printed 1/3 before anything is paid for"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and a printed 1/1 beside her"
    );

    // The whole price is one white and her own tap. `tap_all_mana` leaves her
    // standing: it presses only abilities whose entire cost is their own {T}
    // (#159), and {W}, {T} is a larger price than that.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "the one Plains, and the Matron left alone");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "one white floating");
    assert_eq!(pool.total(), 1, "and nothing else on this board makes mana");
    assert!(!is_tapped(&engine, matron), "she is still standing");

    // The offer is read off the pool, so it is claimed only now.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(matron, 0)),
        "with {{W}} floating and the Cleric untapped her one line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, abbey_matron(), 0);
    assert!(is_tapped(&engine, matron), "{{T}} is half the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{W}} is spent: a pump costs no more than the card prints"
    );
    assert!(
        !stack_is_empty(&engine),
        "pumping is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        pt(&engine, matron),
        (1, 3),
        "CR 601.2h paid the price but nothing has resolved yet"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, matron),
        (1, 6),
        "+0/+3: the power stays where it was printed and only the toughness grows"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "`Filter::This` is the whole of what the pump reaches"
    );
}
