//! `cards/lands/gain/bloodfell_caves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bloodfell Caves — Land — "This land enters tapped. When this land enters,
/// you gain 1 life. {T}: Add {B} or {R}."
///
/// The land is *played* and not seeded, because `starting_battlefield` puts a
/// permanent down with `Cause::Setup`: no replacement effect looks at a
/// placement, so a board built that way stands untapped whatever the card
/// prints and the enters-tapped line would read as passing. The life is read
/// off the trigger resolving rather than off the card file, and the two
/// colours are read on p0's *next* turn — a land that arrived tapped has no
/// untapped `{T}` in the turn it arrived, so the permanent that pays for them
/// is still the one that entered tapped.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bloodfell_caves_enters_tapped_gains_a_life_and_taps_for_black_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, forest())
        .hand(0, &[bloodfell_caves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let their_life = engine.state().players[1].life;

    let land = play_land(&mut engine, p0, bloodfell_caves());
    // The enters-trigger is on the stack the moment the land is; the life
    // total is what says it resolved rather than merely that it was put
    // there.
    pass_until(&mut engine, |e| {
        e.state().players[0].life == life_before + 1
    });

    assert_eq!(
        on_battlefield(&engine, p0, bloodfell_caves()),
        Some(land),
        "the card became the permanent it was played as"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a tapland's whole cost, and the reason \
         this had to be a real `PlayLand`: a seeded battlefield would have \
         left it standing"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life,
        "and the life is the entering player's: `you gain`, not each player"
    );

    // A land that arrived tapped gives nothing this turn, so the same
    // permanent is read again once its controller's untap step has been
    // through.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so the `{{T}}` below is payable"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect(
            "`{{T}}: Add {{B}} or {{R}}` is a mana ability the card prints, so it \
             is an ordinary `(source, index)` entry and not the CR 305.6 \
             shortcut that `mana_abilities` carries",
        );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is tapped"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana ability activates");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{B}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options.len(),
        2,
        "\"or\" is a choice between two colours, and colourless is no colour \
         at all (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "{{B}} and {{R}}, and nothing else on the menu: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "one choice, not both halves of it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
