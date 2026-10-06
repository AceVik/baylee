//! `cards/lands/gain/pym_technologies.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pym Technologies prints three lines on a land: it arrives tapped, it gains
/// its controller a life as it enters, and it taps for {G} or {U}. All three
/// are read off one turn cycle, because each is what keeps the others honest —
/// the untap step of the next turn is what says the tapped-ness came from the
/// entry rather than from a board that never advanced, and the life total is
/// the only evidence the enters-trigger fired at all (a land that simply
/// tapped would look identical on the battlefield). The colour question is
/// then read as the two colours the card prints and nothing else, off a pool
/// asserted empty beforehand so the one mana is the land's own.
#[test]
fn pym_technologies_enters_tapped_gains_a_life_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[pym_technologies()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, pym_technologies());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\", read on the permanent the play made"
    );

    // The enters-trigger resolves like any other: pass until the life total
    // has moved, which is the only thing on this board that can move it.
    pass_until(&mut engine, |e| e.state().players[0].life == 21);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you gain 1 life\" is the land's controller and not the table"
    );

    // A land that arrives tapped offers nothing with `{T}` until it has been
    // through an untap step of its own controller, so the mana ability is read
    // one turn later — and that step is also the control for the tap above.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it up: the tap above was the entry (CR 502.3) \
         and not a board that never reached one"
    );

    // `{T}: Add {G} or {U}` is a mana ability (CR 605.1), so it is an ordinary
    // `(source, index)` entry in `abilities` (#159) whose whole price is its
    // own tap — nothing has to be tapped first, and the pool is asserted empty
    // so that the one mana below is exact.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == pym_technologies()))
        })
        .expect("{T}: Add {G} or {U} is the only activated ability the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, and the ability owes no mana of its own"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}} or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "\"Add {{G}} or {{U}}\" offers both: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and the two colours are the whole menu: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the other branch was not taken"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting on one"
    );
    assert!(is_tapped(&engine, source), "the land paid its own {{T}}");
}
