//! `cards/lands/pain/pine_barrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pine Barrens prints three lines: it enters tapped, it taps for `{C}`, and
/// it taps for `{B}` or `{G}` at the price of one damage to its controller.
/// The two mana lines are the whole card, so both are played — and each on a
/// turn of its own, because a land that enters tapped has no `{T}` to spend
/// and the untap step is the only thing that separates "the line is missing"
/// from "the land is still lying down". The choice offered for the second tap
/// is what tells the printed `{B}` or `{G}` from the five colours of "any
/// colour", and the life total is what tells the second line from the first.
#[test]
fn pine_barrens_enters_tapped_and_pays_one_life_for_black_or_green_but_not_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8241, forest())
        .hand(0, &[pine_barrens()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, pine_barrens());
    assert!(
        entered_tapped(&engine, land),
        "the printed first line: this land enters tapped"
    );

    // A land that comes in tapped has nothing to offer this turn (CR 302.6's
    // cousin for permanents that tap), so the first mana line is read on the
    // controller's next turn, one full turn cycle away.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so what follows is the card's \
         reading and not a turn that never came"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing else on this board makes mana"
    );

    // Ability 0 is `{T}: Add {C}`.
    activate(&mut engine, p0, pine_barrens(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — one colourless, and no colour at all"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the colourless line prints no damage clause and takes no life"
    );

    // Round again for the second tap: the land spent its {T} above.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untapped once more");

    // Ability 1 is `{T}: Add {B} or {G}`, and the two colours are the whole
    // menu: "any colour" would offer five.
    activate(&mut engine, p0, pine_barrens(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{B}} or {{G}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and not the five of \"any colour\": {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "the printed pair is what is on offer: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    // The damage clause is fixed at "you", so the driver is only here in case
    // the engine asks for the seat it already knows; nothing is left pending.
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "a mana ability uses no stack: the tap finishes where it started"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "`This land deals 1 damage to you` — the second line costs a point of \
         life where the colourless one did not"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage is dealt to the land's controller and never across the table"
    );
}
