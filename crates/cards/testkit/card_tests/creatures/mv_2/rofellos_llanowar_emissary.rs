//! `cards/creatures/mv_2/rofellos_llanowar_emissary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rofellos, Llanowar Emissary is a `{G}{G}` legend whose entire text
/// is `{T}: Add {G} for each Forest you control`. On the battlefield stand
/// three Forests under his control and one across the table, so exactly
/// three green is the only answer that reads both: two would mean the
/// opponent's Forest was counted, four that `ControlledByYou` has fallen,
/// and a single green that the amount was read as a fixed one. Before that,
/// **nothing** is tapped, so that “three and nothing else” is an exact
/// statement about Rofellos and not about three floating Forest mana.
#[test]
fn rofellos_adds_one_green_for_each_forest_you_control_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(31, forest())
        .battlefield(
            0,
            &[rofellos_llanowar_emissary(), forest(), forest(), forest()],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let rofellos = on_battlefield(&engine, p0, rofellos_llanowar_emissary())
        .expect("Rofellos is on the battlefield");
    let mine = all_on_battlefield(&engine, p0, forest());
    assert_eq!(mine.len(), 3, "three Forests on this side of the table");
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and one across it, which is nobody's Forest here"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats and nothing is tapped, so every drop below is his"
    );

    // A printed mana ability has an index to name it, so it stands as an
    // ordinary entry in `legal.abilities` — offered at an empty pool, because
    // its entire cost is its own {T} (CR 605.1).
    activate(&mut engine, p0, rofellos_llanowar_emissary(), 0);

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`Add {{G}}` names its color, so nothing is asked (CR 605.1): {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        3,
        "{{G}} for each of the three Forests that *you* control"
    );
    assert_eq!(pool.total(), 3, "drei, und kein anderes Mana daneben");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability doesn't use the stack"
    );
    assert!(is_tapped(&engine, rofellos), "the {{T}} was the price");
    assert!(
        mine.iter().all(|id| !is_tapped(&engine, *id)),
        "and no Forest has moved, so the green mana came from Rofellos"
    );
}
