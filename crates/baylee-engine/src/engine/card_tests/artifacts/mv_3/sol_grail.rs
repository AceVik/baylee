//! `cards/artifacts/mv_3/sol_grail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sol Grail is `{3}` for two sentences that are only worth anything
/// together: "As this artifact enters, choose a color" and "{T}: Add one mana
/// of the chosen color." Reading the card file cannot tell that pairing from a
/// rock that makes whatever color it likes, so the color is **named** at the
/// entry question and the mana ability is then played on a board where the
/// answer has to be that one: the four Forests leave exactly one green
/// floating, so a Grail that asked a second time, or fell back to a default,
/// would leave two green in the pool instead of one green and one black.
#[test]
fn sol_grail_makes_the_color_it_was_asked_for_on_the_way_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[sol_grail()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests into the pool first: {3} of them pay for the artifact and
    // CR 500.5 keeps the one left over floating, since the whole scenario
    // stays inside this one main phase.
    cast_from_hand(&mut engine, p0, sol_grail());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the seat casting the artifact is the one naming"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"choose a color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let grail = on_battlefield(&engine, p0, sol_grail()).expect("the Grail resolved");
    assert!(!is_tapped(&engine, grail), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Forests less the {{3}} the artifact cost"
    );

    // Ability 0 is the printed "{T}: Add one mana of the chosen color", whose
    // whole price is the Grail's own tap — so it is offered here, and it must
    // not ask anything: the color was settled as the artifact entered.
    activate(&mut engine, p0, sol_grail(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the color was chosen on the way in, so nothing is asked now: {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named at the entry question"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the green is still the Forests' leftover, not a second mana off \
         the Grail"
    );
    assert_eq!(
        pool.total(),
        2,
        "one black from the Grail's tap and one green from the pool"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, grail), "the Grail paid its own {{T}}");
}
