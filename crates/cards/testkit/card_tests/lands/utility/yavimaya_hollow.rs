//! `cards/lands/utility/yavimaya_hollow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Yavimaya Hollow: "{T}: Add {C}." / "{G}, {T}: Regenerate target creature."
///
/// "Target creature" and no subtype, which is the other half of the shape
/// the Elephant Graveyard shows: here the menu is the whole board and the
/// *price* is what has to be paid, so the `{G}` leaving the pool is the
/// assertion the Graveyard cannot make.
///
/// This pinned the missing shield on a board with no creature at all, so
/// the ability was absent for want of a target and the pin would never have
/// noticed the rule arriving.
#[test]
fn yavimaya_hollow_spends_a_green_to_shield_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(45, forest())
        .hand(0, &[yavimaya_hollow()])
        .battlefield(0, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, yavimaya_hollow());
    assert!(!is_tapped(&engine, hollow));
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, hollow);
    let green = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);
    assert!(green >= 1, "the Forest is the {{G}} the price is paid with");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hollow, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        legal.abilities.contains(&(hollow, 1)),
        "and the regeneration, with a green floating to pay for it"
    );

    activate(&mut engine, p0, yavimaya_hollow(), 1);
    let menu = aim_at(&mut engine, p0, elf);
    assert!(
        menu.contains(&elf),
        "`target creature` reaches every creature on the board: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("the Elves are still there")
            .regeneration_shields,
        1
    );
    assert!(is_tapped(&engine, hollow), "the {{T}} half of the cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green - 1,
        "and the {{G}} half: one green went into the price"
    );
}
