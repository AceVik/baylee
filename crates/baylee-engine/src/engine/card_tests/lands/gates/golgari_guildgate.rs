//! `cards/lands/gates/golgari_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Golgari Guildgate prints two lines: "This land enters tapped" and
/// "{T}: Add {B} or {G}". The tapped entry is read off the permanent the
/// moment it is played, and it is what makes the rest of the test a *turn*
/// long: a land that arrived tapped has no untapped `{T}` to offer in the
/// main phase it came down in, so the mana line can only be read after a
/// real untap step. The colour question is the half the card cannot answer
/// by itself — both black and green are on the menu (CR 105.4) — and the
/// pool afterwards holds the one that was named and not a default.
#[test]
fn golgari_guildgate_enters_tapped_and_taps_for_the_colour_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1107, forest())
        .hand(0, &[golgari_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let gate = play_land(&mut engine, p0, golgari_guildgate());
    assert!(entered_tapped(&engine, gate), "\"This land enters tapped\"");

    // What that sentence costs on the turn it is paid: a tapped Gate is
    // offered nothing, and there is no other mana source on this board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == gate),
        "a tapped land has no {{T}} to pay, so its mana ability is not \
         offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floated on the way"
    );

    // A whole turn cycle. The Gate stands up in p0's own untap step, which
    // runs only on the turn after the one it entered on.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, gate), "the untap step ran");

    // Index 0: the land's whole printed text is the one mana ability.
    activate(&mut engine, p0, golgari_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "both halves of the printed choice: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the other half of the choice was never made"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
