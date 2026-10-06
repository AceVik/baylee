//! `cards/lands/gain/jwar_isle_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jwar Isle Refuge prints three sentences — it enters tapped, its entry
/// gains its controller 1 life, and it taps for {U} or {B} — and all three
/// are read off one *played* land. `SeatSpec::starting_battlefield` would
/// have placed the permanent with `Cause::Setup`, which no replacement
/// effect looks at, so a board built that way arrives untapped whatever the
/// card says and would pass an "enters tapped" claim it never tested. The
/// colour question is asked one turn later on purpose: a land that entered
/// tapped is still down for the whole turn it arrived in, so its `{T}` is
/// not even offered until its controller's untap step has stood it back up.
#[test]
fn jwar_isle_refuge_enters_tapped_gains_a_life_and_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4371, forest())
        .hand(0, &[jwar_isle_refuge()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let refuge = play_land(&mut engine, p0, jwar_isle_refuge());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        entered_tapped(&engine, refuge),
        "\"This land enters tapped\" — a real land drop, so the entry \
         modifier was the thing that was asked"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\" — the life belongs to \
         the land's controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and to nobody else on the table"
    );

    // A whole turn cycle, because the arrival turn's tap is already spent.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, refuge),
        "the untap step ran and stood the land back up"
    );

    // The only thing on offer is the mana ability: ability 0 is the entry
    // trigger and no trigger is ever an activation.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == refuge)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        vec![1],
        "the printed `{{T}}` line is an ordinary `(source, index)` entry and \
         nothing else on the card is offered: {offered:?}"
    );

    activate(&mut engine, p0, jwar_isle_refuge(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both halves of the printed line are available: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else is — no third colour to default to: {options:?}"
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
        pool.available(ManaColor::Black),
        0,
        "the colour that was named is the only one made, and {{B}} was not it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, refuge), "the land paid its own {{T}}");
}
