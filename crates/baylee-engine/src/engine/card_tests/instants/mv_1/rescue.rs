//! `cards/instants/mv_1/rescue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rescue — {U} Instant: "Return target permanent you control to its owner's
/// hand." The load-bearing word is *permanent*: the offer has to hold the
/// Island this seat controls — a creature-only filter would have dropped it —
/// and it has to decline the Elf across the table, which only
/// `Filter::ControlledByYou` keeps off the menu. The bounce is then read as the
/// Elf arriving in its owner's hand while the land beside it never moves, and
/// the {{U}} as the one blue the single Island put in the pool.
#[test]
fn rescue_returns_a_permanent_you_control_to_the_hand_of_the_seat_that_owns_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[rescue()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // Only the Island is tapped: `tap_all_mana` would also have pressed the
    // Elves' own printed `{T}: Add {G}` (#159), and one blue is the whole of
    // the price the card prints.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island, one blue — the Elves stayed untapped and made nothing"
    );

    cast_with_floating(&mut engine, p0, rescue());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&mine),
        "the creature under my own control is on the menu: {options:?}"
    );
    assert!(
        options.contains(&land),
        "\"target permanent\" is not \"target creature\": the Island I control \
         is a permanent too, and a creature-only filter would have dropped it: \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" declines the Elf across the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "\"return ... to its owner's hand\": the Elf is in the hand of the seat \
         that owns it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and it has left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, island()).is_some(),
        "the permanent Rescue did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did the creature across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}} it charges came out of the pool"
    );
}
