//! `cards/lands/tapland/volatile_fjord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volatile Fjord — a snow Island Mountain: "This land enters tapped" and
/// "{T}: Add {U} or {R}".
///
/// The land is played with a real `PlayLand`, so the enters-tapped modifier
/// is the one the entry actually runs, and the *offer* read on that same turn
/// is what turns the clause into a consequence rather than a status bit: a
/// land paying no `{T}` is named by neither list of mana routes `deeds` reads.
///
/// The turn after, the board's other two sources are a Forest and a Mountain,
/// so naming blue puts exactly one blue in the pool that no standing source
/// could have made — and exactly one red, where an engine that fell back to
/// the mountain half would have left two.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn volatile_fjord_enters_tapped_and_taps_for_the_colour_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), mountain()])
        .hand(0, &[volatile_fjord()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let fjord = play_land(&mut engine, p0, volatile_fjord());
    assert!(
        entered_tapped(&engine, fjord),
        "\"This land enters tapped\" — and through a real land drop, which is \
         the placement the entry modifier is read at"
    );

    // Read as a consequence and not as a flag: a tapped land pays no `{T}`,
    // so it is a mana source in neither of the two lists an offer lives in.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop is a special action and leaves priority where it was, \
             got {:?}",
            engine.pending()
        )
    };
    assert!(
        deeds(&legal, &[fjord]).is_empty(),
        "a land that entered tapped is no mana route this turn: {:?} / {:?}",
        legal.mana_abilities,
        legal.abilities
    );

    // One turn around the table: the untap step is what stands it up, and the
    // two lands beside it are the control that the step really ran.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, fjord),
        "the untap step stood it back up"
    );
    let a_forest = on_battlefield(&engine, p0, forest()).expect("a Forest is on the board");
    assert!(
        !is_tapped(&engine, a_forest),
        "and the Forest beside it is up in the same step"
    );

    // Everything else on the board taps first, so what the Fjord adds here
    // cannot be confused with what a Forest or a Mountain made.
    let taken = tap_mana_except(&mut engine, p0, fjord);
    assert_eq!(
        taken, 2,
        "the Forest and the Mountain, and the Fjord kept back"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one green and one red floating, and no blue yet"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0,
        "nothing standing on this board is blue: the Fjord is the only source \
         that could make it"
    );

    // Mana first, then the claim: the offer is read off the pool and off the
    // untapped source, and the route is taken out of what it enumerated
    // rather than guessed at — a dual with two basic land types is both the
    // CR 305.6 shortcut and a printed mana ability.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let routes = deeds(&legal, &[fjord]);
    assert!(
        !routes.is_empty(),
        "an untapped land with a mana ability is a route the offer names: {legal:?}"
    );
    engine
        .apply(p0, routes[0].1.action(fjord))
        .expect("the route came out of the offer");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options.len(),
        2,
        "an Island and a Mountain, and no third answer: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "\"{{U}} or {{R}}\": {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, off the Fjord's own tap"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the Mountain's red alone — a fallback to the other half would have \
         left two"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the Forest's green"
    );
    assert_eq!(
        pool.total(),
        3,
        "three sources, three mana, nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, fjord), "the land paid its own {{T}}");
}
