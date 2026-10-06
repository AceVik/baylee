//! `cards/lands/towns/sharlayan_nation_of_scholars.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sharlayan, Nation of Scholars — a Town land, `Coverage::Implemented`:
/// "This land enters tapped" and "{T}: Add {W} or {U}".
///
/// Both printed lines are played, and the entry has to be a real land drop:
/// `starting_battlefield` seats a permanent with `Cause::Setup`, which no
/// replacement effect ever looks at, so a land placed that way would arrive
/// untapped whatever the card says. The mana line is read a turn later,
/// because a land that enters tapped has no `{T}` left in the turn it
/// arrives — and the question it asks is asserted as the menu it is, since a
/// Town that only ever made white would fill the same single-mana pool and
/// pass a test that never read it.
#[test]
fn sharlayan_nation_of_scholars_enters_tapped_and_taps_for_white_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sharlayan_nation_of_scholars()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A land drop and not a seeded board: the first printed line is an enter
    // modifier, and only an entry runs one.
    let town = play_land(&mut engine, p0, sharlayan_nation_of_scholars());
    assert!(entered_tapped(&engine, town), "\"This land enters tapped\"");

    // Its whole price is the `{T}` it just spent arriving, so the line is not
    // merely refused when pressed — it is absent from the offer entirely.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == town),
        "a tapped land has no {{T}} to pay with: {:?}",
        legal.abilities
    );

    // The untap step of its controller's next turn is the only thing that
    // hands the tap symbol back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, town), "the untap step stood it back up");

    // Ability 0 is the printed "{T}: Add {W} or {U}" — a mana ability a land
    // prints, so it is an ordinary `(source, index)` entry in `abilities` and
    // not the CR 305.6 shortcut, which only a basic land type reaches.
    activate(&mut engine, p0, sharlayan_nation_of_scholars(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, town), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "`or` is one mana of one colour, not both halves of the pair"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
