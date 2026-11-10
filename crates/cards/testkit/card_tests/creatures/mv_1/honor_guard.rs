//! `cards/creatures/mv_1/honor_guard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Honor Guard prints one line — "{W}: This creature gets +0/+1 until end of
/// turn" — and that line is the whole card, so the scenario has to read its
/// price, its direction and its reach. Three Plains are tapped *before*
/// anything is claimed, because `can_afford` reads the mana pool and not the
/// untapped lands; the pool is then read again after the two activations, so
/// the {W} is shown to be spent rather than merely printed. Resolving the two
/// activations one at a time is what makes "+0/+1" a stacking effect instead
/// of one payment read twice, and the Elf beside the Guard is the control for
/// `Filter::This`: the pump lands on the Guard and on nothing else.
#[test]
fn honor_guard_pays_white_to_grow_its_own_toughness_and_leaves_the_elf_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), quiet_creature()])
        .hand(0, &[honor_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "three Plains tapped for three white before the ability is claimed"
    );
    cast_with_floating(&mut engine, p0, honor_guard());
    pass_until(&mut engine, stack_is_empty);

    let guard = on_battlefield(&engine, p0, honor_guard()).expect("the Guard resolved");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert_eq!(pt(&engine, guard), (1, 1), "a printed 1/1");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "and casting it spent the one white its cost asks for"
    );

    // First activation: a real payment, put on the stack, and resolved
    // before the second one is asked for.
    activate(&mut engine, p0, honor_guard(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it waits for the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, guard), (1, 2), "{{W}}: +0/+1 until end of turn");

    activate(&mut engine, p0, honor_guard(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, guard),
        (1, 3),
        "a second activation is a second +0/+1"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "two activations, two white, and nothing left over"
    );

    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "`Filter::This` names the Guard: the Elf beside it is exactly what it was printed as"
    );
}
