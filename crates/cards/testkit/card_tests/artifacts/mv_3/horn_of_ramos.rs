//! `cards/artifacts/mv_3/horn_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Horn of Ramos prints two mana lines and neither of them costs mana: "{T}:
/// Add {G}" and "Sacrifice this artifact: Add {G}". One board therefore reads
/// the whole card — the tap line pays with its own {T}, which leaves the
/// artifact tapped but very much on the battlefield, and the sacrifice line,
/// which asks for no tap at all, is still offered and still makes green.
///
/// The only land on the table is an Island that never moves, so the green in
/// the pool has no other source on the board, and each half is read with the
/// pool empty beforehand: that is the state `can_afford` reads, which is what
/// says the price of either line is the artifact itself and not some mana the
/// board happened to be holding.
#[test]
fn horn_of_ramos_taps_and_then_sacrifices_itself_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(517, forest())
        .battlefield(0, &[island(), horn_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horn = on_battlefield(&engine, p0, horn_of_ramos()).expect("the Horn is on the table");
    let land = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: neither line the Horn prints costs any mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(horn, 0)) && legal.abilities.contains(&(horn, 1)),
        "both printed mana lines are payable on an empty pool, because each \
         one's whole price is the artifact itself: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed "{T}: Add {G}".
    activate(&mut engine, p0, horn_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, horn), "{{T}} was the whole price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green, off the Horn's own tap"
    );

    // The second line does not care that the artifact is already tapped: its
    // price is the artifact and nothing else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(horn, 0)),
        "a tapped artifact has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(horn, 1)),
        "\"Sacrifice this artifact: Add {{G}}\" asks for no tap, so a tapped \
         Horn still offers it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, horn_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "and the sacrifice line is a mana ability too (CR 605.3b)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "{{G}} twice, one from each printed line"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with them");
    assert!(
        on_battlefield(&engine, p0, horn_of_ramos()).is_none(),
        "the sacrifice is half of what the second line charges"
    );
    assert!(
        in_graveyard(&engine, p0, horn_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "no blue either: the Island was never tapped"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Island never moved, so the two green have no source other than \
         the artifact"
    );
}
