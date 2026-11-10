//! `cards/creatures/mv_3/apprentice_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Apprentice Wizard — {1}{U}{U} — 0/1 Human Wizard: "{U}, {T}: Add {C}{C}{C}."
///
/// The price is a blue mana **and** the tap, and that is the whole card: with
/// an empty pool the {U} cannot be paid and `legal.abilities` does not list
/// the line at all, while `tap_all_mana` may not press it either — its whole
/// price is not its own tap (#159). What lands afterwards is three
/// *colourless*, which is the reading no other permanent on this board could
/// have produced: the two Islands make blue and nothing else, one blue is
/// gone to the price, and the mana arrives with an empty stack (CR 605.3b).
#[test]
fn apprentice_wizard_trades_a_blue_mana_and_its_tap_for_three_colourless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), apprentice_wizard()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wizard = on_battlefield(&engine, p0, apprentice_wizard()).expect("the Wizard is out");
    assert_eq!(pt(&engine, wizard), (0, 1), "a printed 0/1");
    assert!(!is_tapped(&engine, wizard), "and untapped to begin with");

    // `can_afford` reads the pool and not the untapped lands, so with nothing
    // floating the {U} half of the price is unpayable and the line is absent
    // from the offer rather than refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(wizard, 0)),
        "an empty pool pays no {{U}}, so the ability is not offered: {:?}",
        legal.abilities
    );

    // The Islands are the CR 305.6 shortcut and the Wizard is a larger price:
    // "{U}, {T}" is not a tap on its own, so the helper leaves it standing.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "the two Islands, and the Wizard left alone");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue, and nothing off the Wizard"
    );
    assert!(
        !is_tapped(&engine, wizard),
        "a price this kit may not pay on a test's behalf is not a mana route"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(wizard, 0)),
        "with the {{U}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, apprentice_wizard(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, wizard), "{{T}} was half the price");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "{{C}}{{C}}{{C}} — no land on this board produces a colourless mana"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one of the two blue paid the {{U}}, and the other is still floating"
    );
    assert_eq!(pool.total(), 4, "and nothing else came with it");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
