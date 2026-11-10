//! `cards/lands/gain/birnin_zana_plaza.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Birnin Zana Plaza prints three lines, and one game plays all three: it
/// enters tapped, its arrival gains its controller 1 life, and its `{T}` adds
/// {G} or {W}. Reading the card cannot tell an entry rule from a land that
/// simply never untaps, so the permanent is read twice — tapped and one life
/// heavier in the turn it arrived, then stood back up and offering its `{T}`
/// in its controller's next main phase. The colour question is the third
/// line, and it is the half a default would hide: naming green has to leave
/// exactly one green in the pool and no white.
#[test]
fn birnin_zana_plaza_enters_tapped_gains_a_life_and_taps_for_green_or_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[birnin_zana_plaza()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plaza = play_land(&mut engine, p0, birnin_zana_plaza());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        entered_tapped(&engine, plaza),
        "\"This land enters tapped\""
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the land's controller, not to the table"
    );

    // What the entry rule costs, in the very turn it was paid: a `{T}` that
    // is already tapped is not offered, and nothing else on this board is
    // holding the ability back.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&plaza)
            && !legal.abilities.iter().any(|(source, _)| *source == plaza),
        "a land that entered tapped has no {{T}} to offer: {:?}",
        legal.abilities
    );

    // A full turn cycle, which is where the entry rule stops mattering.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, plaza),
        "the untap step of its controller's next turn stood it back up, so \
         the printed sentence is an entry rule and not a permanent one"
    );

    // The printed `{T}: Add {G} or {W}`. It is a mana ability a card prints,
    // so it is an ordinary entry in `legal.abilities` and not the CR 305.6
    // shortcut the basics live in (#159).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == plaza)
        .expect("an untapped Plaza offers its {T}");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the offered ability activates");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours are printed, so there is a choice to make: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "and they are the two the card prints: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "one tap, one mana: the other half of the choice was not taken as well"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, plaza), "the Plaza paid its own {{T}}");
}
