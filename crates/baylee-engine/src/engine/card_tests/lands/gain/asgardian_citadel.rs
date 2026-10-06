//! `cards/lands/gain/asgardian_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Asgardian Citadel prints three lines and this scenario plays all three:
/// it enters tapped, its arrival gains its controller 1 life, and its tap
/// symbol adds one mana of either of two colours — which is a *choice* and
/// not a fixed output, so the engine has to stop and ask.
///
/// The land is **played** rather than seeded onto a starting battlefield: a
/// `SeatSpec::starting_battlefield` placement is a `move_object(..,
/// Cause::Setup)`, which no replacement effect looks at, so a land that
/// enters tapped would arrive untapped there and the first line of the card
/// would be unmeasurable.
///
/// The colour choice needs a whole turn cycle, because a land that entered
/// tapped has nothing to pay a tap symbol with until its controller's next
/// untap step. The offer is read once while it is down — nothing to press,
/// and the ability costs no mana, so no missing pool can stand in for that —
/// and once after the untap step has stood it back up.
#[test]
fn asgardian_citadel_enters_tapped_gains_a_life_and_taps_for_red_or_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8801, forest())
        .hand(0, &[asgardian_citadel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, asgardian_citadel());
    assert!(
        entered_tapped(&engine, land),
        "the printed entry line is read at a real entry, and this land got \
         there by being played out of a hand"
    );

    // The arrival trigger is the only thing on this board that moves a life
    // total, so the +1 can only have come from the printed line.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "the enters trigger gains its controller one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before,
        "and it is its controller's life that moved, not the table's"
    );

    // Down, and therefore no tap symbol it could pay. This is the control
    // for the activation below: the ability's whole price is its own tap, so
    // the only thing keeping it out of the offer is the tapped status.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a tapped land prints no tap it can pay: {:?}",
        legal.abilities
    );

    // A whole turn cycle. `reach_their_main_phase` cannot be used twice —
    // the first call already sits in p0's first main — so the opponent's
    // main is reached first and p0's own turn is walked back into.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Citadel's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the activation below is not refused for the \
         wrong reason"
    );

    // Ability 0 is the arrival trigger; the mana ability is the second line
    // the card prints.
    activate(&mut engine, p0, asgardian_citadel(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is two colours and so a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options.len(),
        2,
        "\"{{R}} or {{W}}\" and nothing else: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "both halves of the printed line are on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "one land, one mana: the other half of the choice was not made too"
    );
    assert_eq!(
        pool.total(),
        1,
        "the Citadel is the only permanent on this board, so one mana is all \
         the pool can hold"
    );
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the tap symbol was the price");
}
