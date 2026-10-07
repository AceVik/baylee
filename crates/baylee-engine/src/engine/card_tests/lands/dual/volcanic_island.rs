//! `cards/lands/dual/volcanic_island.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volcanic Island is a `Land — Island Mountain` whose whole text is the
/// intrinsic `{T}: Add {U} or {R}` (CR 305.6). Playing it and tapping it is
/// the entire card, and the two basic land types are exactly what make the
/// mana ability a *choice* rather than a default — a single-type land never
/// asks. After the answer the pool holds blue and no red, which no other
/// source on this board could have produced: nothing is tapped beforehand,
/// no other permanent is on the table, and the count is taken before the
/// land beside it could have contributed anything.
#[test]
fn volcanic_island_taps_for_whichever_of_its_two_colours_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[volcanic_island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, volcanic_island());
    assert!(
        !entered_tapped(&engine, land),
        "the card prints no entry modifier, so it stands untapped the turn it is played"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is tapped, so whatever lands below came off it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // The one line the card prints, in whichever of the two lists the engine
    // carries it (#159): a basic land type has no printed index to name, a
    // printed mana ability does, and `tap_all_mana` reads both.
    let action = if legal.mana_abilities.contains(&land) {
        PlayerAction::ActivateManaAbility { source: land }
    } else {
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(src, _)| *src == land)
            .expect("the land's own {T} is on one of the two lists (#159)");
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    engine.apply(p0, action).expect("the offer is honoured");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that taps is the seat that names the colour"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "\"Add {{U}} or {{R}}\" offers both of its basic land types: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "two colours, and no third: the card prints no colourless at all"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not the other one, so the answer decided the colour rather than a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
