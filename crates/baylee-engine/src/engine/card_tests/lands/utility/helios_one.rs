//! `cards/lands/utility/helios_one.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// With no mana available, HELIOS One can only pay for its colorless tap.
#[test]
fn helios_one_without_mana_only_offers_its_free_colorless_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4102, forest()).hand(0, &[helios_one()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, helios_one());
    assert!(
        !entered_tapped(&engine, land),
        "nothing on the card enters it tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land drop spends nothing, so whatever is in the pool next is the land's"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending());
    };
    let offered = deeds(&legal, &[land]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "the energy activations need mana that this fixture cannot pay: {offered:?}"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the only mana source on the battlefield is HELIOS One"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{C}}, as printed");
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
