//! `cards/lands/gain/graypelt_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Graypelt Refuge prints three sentences — "This land enters tapped", "When
/// this land enters, you gain 1 life", "{T}: Add {G} or {W}" — and the copy
/// played out of hand proves the first two: it is sideways the instant the
/// land drop lands, before any trigger has resolved, and the life total moves
/// only when that trigger comes off the stack. The copy already standing is
/// both the control for "enters tapped" (a board that taps everything on
/// arrival would have taken it too) and the body the `{T}` half is played on,
/// because the copy that just entered may not tap this turn at all.
#[test]
fn graypelt_refuge_enters_tapped_gains_a_life_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[graypelt_refuge()])
        .hand(0, &[graypelt_refuge()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let already = on_battlefield(&engine, p0, graypelt_refuge()).expect("a Refuge is standing");
    assert!(
        !is_tapped(&engine, already),
        "the copy that was already on the table is untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has gained life yet"
    );

    let land = play_land(&mut engine, p0, graypelt_refuge());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — read off the permanent the land drop \
         just made, with its enters trigger still on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life is not there yet: the trigger has not resolved"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\" — the land really entered, \
         which is why the trigger fired at all"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is the seat that played it"
    );
    assert!(
        !is_tapped(&engine, already),
        "only the copy that entered was tapped: nothing here taps a land for \
         being a land"
    );

    // `{T}: Add {G} or {W}` — on the untapped copy, because the one that just
    // entered is on its back until its controller's next untap step.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the tap"
    );
    activate(&mut engine, p0, graypelt_refuge(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat activating the land names the color");
    assert_eq!(
        options.len(),
        2,
        "\"{{G}} or {{W}}\" is two colors and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printed disjunction are offered: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colors the land offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not the other half of the disjunction"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, already),
        "the untapped copy paid its own {{T}}"
    );
    assert!(
        is_tapped(&engine, land),
        "and the one that entered tapped is still tapped"
    );
}
