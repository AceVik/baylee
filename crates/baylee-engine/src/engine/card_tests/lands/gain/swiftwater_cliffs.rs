//! `cards/lands/gain/swiftwater_cliffs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Swiftwater Cliffs prints three lines: it enters tapped, its arrival gains
/// its controller 1 life, and it taps for one mana of either of its two
/// colours. The turn it arrives settles the first two — the tapped status is
/// the entry replacement and the life total is the trigger, and the offer it
/// names nothing in is what a land lying down costs the player who played it.
/// The mana line then needs a whole turn cycle, because the untap step is the
/// only thing that stands a land back up, and the choice it asks has to name
/// blue and red and no third colour.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn swiftwater_cliffs_enters_tapped_gains_a_life_and_taps_for_blue_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[swiftwater_cliffs()])
        .start();
    keep_mulligans(&mut engine);
    let my_life = engine.state().players[0].life;
    let their_life = engine.state().players[1].life;

    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, swiftwater_cliffs());
    // The arrival and its trigger waited out rather than assumed: this land
    // is the only permanent on the board, so the life total is the only place
    // the trigger is visible at all.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life > my_life
    });

    assert!(
        entered_tapped(&engine, land),
        "\"this land enters tapped\" — the entry replacement, and not a land \
         somebody tapped after it arrived"
    );
    assert_eq!(
        engine.state().players[0].life,
        my_life + 1,
        "\"when this land enters, you gain 1 life\", once off one arrival"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life,
        "and the seat across the table gains nothing from it"
    );

    // Tapped, so its own `{T}` is off the table. The offer and not the
    // battlefield is what says so, because a land that is still down is
    // exactly a land whose mana is unavailable.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a land that entered tapped offers nothing until it stands up: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&land),
        "and it is no CR 305.6 source either: it prints its own ability, which \
         is an index into `abilities` and never the shortcut"
    );

    // A whole turn cycle, because the untap step is the only thing that
    // stands a land back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran; without this the line below would be read off a \
         land that is simply still down"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("the printed tap ability is offered on an untapped land with nothing floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("a printed mana ability is payable with nothing but its own tap");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{U}} or {{R}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped is the one that names it");
    for color in [ManaColor::Blue, ManaColor::Red] {
        assert!(
            options.contains(&color),
            "\"{{U}} or {{R}}\" offers {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"or\" is one of the two colours, never both of them"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap and nothing else");
    assert!(
        is_tapped(&engine, land),
        "and the land paid its own {{T}} for it"
    );
    assert!(
        stack_is_empty(&engine),
        "`CR 605.3b`: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
