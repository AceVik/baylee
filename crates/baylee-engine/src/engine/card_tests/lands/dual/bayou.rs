//! `cards/lands/dual/bayou.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bayou — Land — Swamp Forest: "{T}: Add {B} or {G}."
///
/// The whole card is that one line, and everything in it is the word *or*: a
/// Swamp pays for a black spell and asks nothing at all, so a Bayou that
/// answered without asking would be a card that had lost half its text.
/// The board is deliberately the land and nothing else — an untapped Bayou
/// off a single land drop — so the mana standing in the pool afterwards has
/// no other source it could have come off, and the menu has to name the two
/// basic land types the card carries and neither a third colour nor
/// colourless.
#[test]
fn bayou_taps_for_black_or_green_and_for_no_third_thing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[bayou()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, bayou());
    assert!(
        !is_tapped(&engine, land),
        "Bayou prints no enter modifier: it is usable the turn it is played, \
         which is the whole difference from a tapland"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating yet, so the pool below is this tap's alone"
    );

    // The one line is offered on one of the two lists (#159): the CR 305.6
    // shortcut a land with basic land types gets, or the ordinary
    // `(source, index)` entry a printed mana ability gets. Read rather than
    // assumed — a `{T}: Add` a card prints is a mana ability in the rules
    // (CR 605.1) whichever list happens to carry it.
    let route = {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        if legal.mana_abilities.contains(&land) {
            PlayerAction::ActivateManaAbility { source: land }
        } else {
            let (_, index) = legal
                .abilities
                .iter()
                .copied()
                .find(|(source, _)| *source == land)
                .expect("Bayou's {T} is offered on one of the two lists");
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: index,
            }
        }
    };
    engine
        .apply(p0, route)
        .expect("an untapped Bayou in your own main phase may be tapped");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(options.len(), 2, "two colours and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Black),
        "the Swamp half of the type line: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green),
        "and the Forest half: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "the other half of the choice was declined"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the Bayou paid its own {{T}}");
}
