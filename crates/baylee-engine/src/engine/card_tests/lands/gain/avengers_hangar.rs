//! `cards/lands/gain/avengers_hangar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Avengers Hangar prints three lines: it enters tapped, its arrival gains its
/// controller 1 life, and it taps for {W} or {U}. One turn cycle reads all
/// three, because the tapped entry is what makes the third line wait — until
/// its controller's untap step a `{T}` ability is not offered at all, so an
/// untapped Hangar in its arrival turn would prove nothing about the entry and
/// a life total read before the trigger resolved would prove nothing about the
/// trigger. The other seat's life is the control on "you gain": the card says
/// its controller, not the table.
#[test]
fn avengers_hangar_enters_tapped_gains_a_life_and_taps_for_white_or_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[avengers_hangar()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, avengers_hangar());
    pass_until(&mut engine, stack_is_empty);

    // "This land enters tapped." The cost is the tap symbol alone, so the
    // refusal below cannot be a price the pool does not cover — and nothing
    // has been tapped here, so the board is not the reason either.
    assert!(is_tapped(&engine, land), "the printed entry is tapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so no ability on this board is missing for want of mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&land)
            && !legal.abilities.iter().any(|(source, _)| *source == land),
        "a land that entered tapped offers no {{T}} ability at all: {:?}",
        legal.abilities
    );

    // "When this land enters, you gain 1 life."
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"you\" is the land's controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nobody else gained anything"
    );

    // The untap step is what makes the printed mana line readable at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it up");

    // Ability 0 is the entry trigger, ability 1 the printed mana ability.
    activate(&mut engine, p0, avengers_hangar(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question whenever both are producible, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "white or blue and nothing else: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "both printed colours are on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and no white beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
