//! `cards/lands/gain/dimension_x.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dimension X prints three lines that pull in different directions: it
/// enters tapped, it gains its controller a life when it does, and once it
/// stands up it taps for {R} or {W} through a printed mana ability — an
/// ordinary `(source, index)` entry in `LegalActions::abilities`, not the
/// CR 305.6 shortcut in `mana_abilities` (#159). The mana half therefore has
/// to wait a whole turn cycle, because in the turn it arrives the land is
/// tapped and the offer is missing for a reason that has nothing to do with
/// the card's text; the same offer, read again after the untap step, is what
/// makes the assertion the card's own doing.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn dimension_x_enters_tapped_gains_a_life_and_taps_for_one_of_its_two_colors() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[dimension_x()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, dimension_x());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a seeded permanent would have missed \
         the replacement effect entirely and arrived standing"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\", read after the trigger \
         resolved rather than off the entry"
    );

    // The tapped half of the entry, and the control for the offer below: a
    // permanent that came in tapped has no untapped {T} to pay with, so the
    // ability is not offered at all this turn.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that played the land");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a land that entered tapped is no mana source until its controller's \
         next untap step (CR 502.3): {:?}",
        legal.abilities
    );

    // A full turn cycle, so that a rule which was reached — and not a turn
    // that never came — is what stands the land back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step came round and left it standing"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the land's controller's main phase");
    assert!(
        !legal.mana_abilities.contains(&land),
        "no basic land type and no granted ability: this is the printed \
         {{T}}: Add, which is an (source, index) entry and never the CR 305.6 \
         shortcut (#159)"
    );
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("the printed mana ability is offered on the untapped land");

    // Nothing else on the board makes mana, so the pool is empty and the one
    // mana read below has exactly one possible source.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing has been tapped or floated"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability the offer named activates");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{W}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "both printed colors, and neither of them missing: {options:?}"
    );
    assert_eq!(options.len(), 2, "the two halves of the printed line");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not the other half of the printed line"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, with no second source anywhere on the board"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
