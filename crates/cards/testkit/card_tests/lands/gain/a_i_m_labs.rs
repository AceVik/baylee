//! `cards/lands/gain/a_i_m_labs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A.I.M. Labs prints three sentences and one turn cycle reads all three:
/// "This land enters tapped", "When this land enters, you gain 1 life", and
/// "{T}: Add {U} or {B}". The turn cycle is not scenery — a land that arrives
/// tapped has spent its `{T}` for the turn (CR 502.3 is what hands it back),
/// so the untapped land asserted in between is what tells the mana line from a
/// land that was never turned sideways at all. The land is played off the
/// land drop rather than seeded onto a battlefield, because
/// `SeatSpec::starting_battlefield` is a placement and not an entry: no
/// enters-tapped modifier would ever look at it. And the pool is read against
/// nothing else, since the Labs is the only permanent p0 has — one blue and
/// no black is exact rather than a difference.
#[test]
fn aims_labs_enters_tapped_gains_a_life_and_taps_for_blue_or_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, island())
        .hand(0, &[a_i_m_labs()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let labs = play_land(&mut engine, p0, a_i_m_labs());
    // The arrival trigger is answered on the way; the land is already on the
    // battlefield, and tapped, while the life is still on the stack.
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, labs),
        "the printed sentence is `This land enters tapped`"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "`When this land enters, you gain 1 life` — off the land drop, which \
         a seeded permanent would never have fired"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the land's controller, not the table"
    );

    // A tapped land has no `{T}` left this turn, and the activation below
    // would be refused on one. Across the opponent's turn and back, because
    // `reach_their_main_phase(p0)` answers "you are already there" from the
    // main phase this started in.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, labs),
        "the untap step ran and stood it back up"
    );

    // The index is taken out of the offer rather than guessed: the ETB
    // trigger sits in the same ability list and is no activation at all, so
    // the mana ability is the only entry on this object either list carries.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == labs)
        .expect("an untapped Labs under a printed `{T}: Add {U} or {B}` is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("its whole price is its own tap, and nothing else is tapped");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both halves of the printed disjunction: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and no third colour: this is a two-colour land, not a Reflecting Pool"
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
        "and not the other half"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, labs), "the Labs paid its own {{T}}");
}
