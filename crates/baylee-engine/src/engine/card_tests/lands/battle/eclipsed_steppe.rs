//! `cards/lands/battle/eclipsed_steppe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eclipsed Steppe is a `Land — Plains Swamp` that prints `{T}: Add {W} or
/// {B}` beside "This land enters tapped unless you control two or more basic
/// lands." Both sides of that condition are played, because either branch
/// alone is passed by a land that always enters tapped or by one that never
/// does: under a lone Forest it arrives tapped, and under exactly two Forests
/// it arrives standing and can be spent the same turn. The colour question is
/// then the other printed half — white and black and nothing else — and the
/// black it makes is mana no Forest on that board could have produced.
#[test]
fn eclipsed_steppe_arrives_tapped_under_one_basic_land_and_untapped_under_two() {
    let p0 = PlayerId::new(0);

    // One basic land: the threshold is not met, so it enters tapped.
    let mut lonely = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[eclipsed_steppe()])
        .start();
    keep_mulligans(&mut lonely);
    reach_main_phase(&mut lonely, p0);
    let tapped = play_land(&mut lonely, p0, eclipsed_steppe());
    assert!(
        entered_tapped(&lonely, tapped),
        "one basic land is not the two the card asks for, so the Steppe \
         enters tapped"
    );

    // Two basic lands: the printed threshold is met exactly.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[eclipsed_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let steppe = play_land(&mut engine, p0, eclipsed_steppe());
    assert!(
        !entered_tapped(&engine, steppe),
        "with two basic lands under its controller the Steppe enters \
         untapped, which is the half a land that always comes in tapped \
         would fail"
    );

    // The two Forests are tapped and the Steppe kept back: it is the source
    // whose printed line this half is about.
    tap_all_mana_but(&mut engine, p0, Some(eclipsed_steppe()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests paid in, and the Steppe is still standing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // The offer splits a mana source across two lists (#159): a land with
    // basic land types may be named by the CR 305.6 shortcut alone, while the
    // ability the card actually prints is an ordinary `(source, index)` entry.
    if legal.abilities.iter().any(|(source, _)| *source == steppe) {
        activate(&mut engine, p0, eclipsed_steppe(), 0);
    } else {
        assert!(
            legal.mana_abilities.contains(&steppe),
            "the Steppe is a mana source, so one of the two lists names it: \
             {:?} / {:?}",
            legal.mana_abilities,
            legal.abilities
        );
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: steppe })
            .expect("the offer listed it");
    }

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert_eq!(
        options.len(),
        2,
        "white and black, and no third colour: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "a Plains and a Swamp between them make exactly these two: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and no Forest on this board could have \
         made it"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and only the colour that was named"
    );
    assert_eq!(pool.total(), 3, "two Forests and the Steppe, nothing else");
    assert!(is_tapped(&engine, steppe), "the Steppe paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability resolves without ever using the stack"
    );
}
