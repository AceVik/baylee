//! `cards/lands/filter/sungrass_prairie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sungrass Prairie — Land, oracle text `{1}, {T}: Add {G}{W}`.
///
/// The printed price is not the land's own tap but the tap *plus* `{1}`,
/// which is the whole card: `tap_all_mana` deliberately leaves a bigger
/// price alone, so the Mountain is tapped by hand first and the Prairie kept
/// back. The Mountain is the control in both directions — with an empty pool
/// the line is not offered at all (`legal.abilities` is filtered through
/// `can_afford`, which reads the pool), and afterwards not one red is left,
/// so the `{G}{W}` that appeared could not have come off anything but the
/// Prairie's own activation.
#[test]
fn sungrass_prairie_charges_one_mana_for_a_green_and_a_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1837, mountain())
        .battlefield(0, &[sungrass_prairie(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let prairie = on_battlefield(&engine, p0, sungrass_prairie()).expect("the Prairie is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&prairie),
        "a printed {{1}}, {{T}} mana ability is an ordinary `(source, index)` \
         entry and not the CR 305.6 shortcut: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == prairie),
        "with an empty pool the {{1}} cannot be paid, so the line is not \
         offered: {:?}",
        legal.abilities
    );

    let taken = tap_mana_except(&mut engine, p0, prairie);
    assert_eq!(taken, 1, "the Mountain is tapped and the Prairie kept back");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red, which is the entire price"
    );

    // Ability 0 is the card's only line; the `{1}` is paid out of the pool.
    activate(&mut engine, p0, sungrass_prairie(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{G}}");
    assert_eq!(pool.available(ManaColor::White), 1, "{{W}}");
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "the {{1}} came out of the Mountain's red, so nothing red is left"
    );
    assert_eq!(
        pool.total(),
        2,
        "two mana, off the two lands that were tapped"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, prairie),
        "the {{T}} was part of the price"
    );
}
