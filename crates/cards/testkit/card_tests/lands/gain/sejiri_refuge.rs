//! `cards/lands/gain/sejiri_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sejiri Refuge prints three lines — "This land enters tapped", "When this
/// land enters, you gain 1 life", and "{T}: Add {W} or {U}" — and the first
/// one is why this board needs a real land drop: `SeatSpec::starting_
/// battlefield` places a permanent with `Cause::Setup`, which no replacement
/// effect ever looks at, so a land built that way would sit there untapped
/// whatever the card says. It is played out of hand instead, the enters
/// trigger is walked to its resolution, and the printed mana ability is read
/// one untap step later — a land that entered tapped has no `{T}` to pay in
/// the turn it arrived (CR 502.3), so the same board is read twice with only
/// the untap step between the readings.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sejiri_refuge_enters_tapped_gains_a_life_and_only_then_taps_for_white_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sejiri_refuge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    assert_eq!(
        engine.state().players[1].life,
        life_before,
        "both seats start level, so the life read below belongs to the land \
         and not to a seat that was already ahead"
    );

    play_land(&mut engine, p0, sejiri_refuge());
    let land =
        on_battlefield(&engine, p0, sejiri_refuge()).expect("the land is on the battlefield");
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — the one line a placement would have missed"
    );

    // "When this land enters, you gain 1 life", and nothing else on the way.
    pass_until(&mut engine, |e| {
        at_rest(e, p0) && e.state().players[0].life == life_before + 1
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "the enters trigger resolved for the land's controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before,
        "and the life is gained by that controller, not by the table"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the trigger resolved and priority came back: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "and it is p0 who holds it on their own main phase"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land)
            && !legal.mana_abilities.contains(&land),
        "a land that entered tapped cannot pay {{T}} this turn, so the printed \
         mana ability is not offered at all (CR 502.3): {:?}",
        legal.abilities
    );

    // A whole turn cycle later the untap step has stood it back up, and the
    // same reading is taken again: same board, same card, one untap step
    // between them — which is what makes the refusal above about the tapped
    // land rather than about a card whose ability was never written.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so what follows is about the ability and not \
         about a land that never came back"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "p0's main phase hands priority back: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "to p0, and nobody else");
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == land),
        "an untapped Sejiri Refuge offers its printed {{T}}: {:?}",
        legal.abilities
    );

    // Ability 0 is the enters trigger, which is never offered as an
    // activation; ability 1 is the mana ability.
    activate(&mut engine, p0, sejiri_refuge(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no others: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "white and blue, because \"or\" is a choice and not a both: {options:?}"
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
        pool.available(ManaColor::White),
        0,
        "the other half of the disjunction was not added alongside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: nothing else on this board produced any"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
