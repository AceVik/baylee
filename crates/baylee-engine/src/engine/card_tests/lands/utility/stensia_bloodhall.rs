//! `cards/lands/utility/stensia_bloodhall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stensia Bloodhall: "{T}: Add {C}." / "{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker."
/// Under `Coverage::Partial`, targeting players or planeswalkers in union is unsupported and omitted.
/// The land taps for its mana ability adding {C} to the mana pool.
#[test]
fn stensia_bloodhall_taps_for_colorless_and_omits_damage_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        // {3}{B}{R} is what the missing ability charges, and the offer is
        // filtered by `can_afford` — which reads the pool. Two Swamps and
        // three Mountains make exactly that price, in exactly those
        // colours.
        .battlefield(
            0,
            &[
                stensia_bloodhall(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, stensia_bloodhall()).expect("Bloodhall deployed");

    // Before the mana ability, not after it: the land's own {T} is part of
    // the price too, so a pin read off a tapped land proves nothing twice
    // over.
    tap_mana_except(&mut engine, p0, hall);
    let (black, red) = {
        let pool = &engine.state().players[0].mana_pool;
        (
            pool.available(ManaColor::Black),
            pool.available(ManaColor::Red),
        )
    };
    assert_eq!((black, red), (2, 3), "the {{3}}{{B}}{{R}} is floating");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == hall && *ai == 1),
        "no second ability is offered, and its price is standing here: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, stensia_bloodhall(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hall));
}
