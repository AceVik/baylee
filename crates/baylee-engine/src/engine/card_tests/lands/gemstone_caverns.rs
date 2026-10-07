//! `cards/lands/gemstone_caverns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gemstone Caverns is `Coverage::Partial`: the pre-game luck counter that
/// would turn its tap into "one mana of any color" cannot be expressed, and
/// what is built is the plain `{T}: Add {C}` on a legendary nonbasic land.
/// So the whole supported half is one play — put the land down and tap it —
/// and the pool afterwards holds exactly one *colourless* mana, which is the
/// negative that tells "Add {C}" apart from the unimplemented replacement:
/// nothing was ever asked to name a color.
/// The Caverns is the only permanent on the board (the 60-card backing deck
/// is a library and no land of it was played), so the colourless mana has no
/// other source to have come from, and the printed `{T}` is paid by the land
/// itself rather than through the basic-land-type shortcut of CR 305.6.
#[test]
fn gemstone_caverns_taps_for_one_colourless_and_names_no_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, forest())
        .hand(0, &[gemstone_caverns()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, gemstone_caverns()).expect("the Caverns are in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on an empty board");
    let caverns = on_battlefield(&engine, p0, gemstone_caverns()).expect("the land hit the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land nobody tapped is no mana"
    );

    // `{T}: Add {C}` is a printed ability on a nonbasic land, so it is an
    // ordinary entry in `abilities` and not the intrinsic CR 305.6 list.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "playing a land leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(caverns, 0)),
        "the land's own {{T}} is offered like any other printed ability: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, gemstone_caverns(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "`Add {{C}}` is the half of the card that is written"
    );
    assert_eq!(
        pool.total(),
        1,
        "and nothing came with it: only the one mana ability resolved"
    );
    assert!(
        is_tapped(&engine, caverns),
        "the {{T}} was paid by the Caverns"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "so the seat is back where it was and was never asked for a color — \
         the luck-counter replacement is not in play: {:?}",
        engine.pending()
    );
}
