//! `cards/lands/gates/dimir_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dimir Guildgate prints two sentences and both are played here: "This land
/// enters tapped" and "{T}: Add {U} or {B}." The entry is read off a real
/// `PlayLand`, because `starting_battlefield` seeds a permanent by placement
/// and no replacement effect ever looks at it — a board built that way is
/// untapped whatever the card says. The mana sentence needs a turn boundary,
/// since a land that entered tapped offers no `{T}` ability at all, and the
/// Gate standing back up in its controller's next main phase also proves the
/// tap above came from the entry rather than from a game that never advanced.
#[test]
fn dimir_guildgate_enters_tapped_and_then_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4041, forest())
        .hand(0, &[dimir_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, dimir_guildgate());
    assert!(
        entered_tapped(&engine, gate),
        "\"This land enters tapped\" — read off a real land drop"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == gate),
        "and while it is tapped its {{T}} ability is not even offered"
    );

    // Only its own untap step can stand it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !entered_tapped(&engine, gate),
        "the untap step ran, so the tap above was the entry and not a game \
         that never moved"
    );

    // Ability 0 is the printed "{T}: Add {U} or {B}."
    activate(&mut engine, p0, dimir_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "a two-colour mana ability is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the Gate names the colour");
    assert_eq!(
        options.len(),
        2,
        "the card prints two colours and the offer is exactly those: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "the blue and the black the card prints: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Green),
        "and not the colour of the Forest the deck is filled with: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Blue), 0, "one tap, one mana");
    assert_eq!(pool.total(), 1, "no other source is on this board");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
}
