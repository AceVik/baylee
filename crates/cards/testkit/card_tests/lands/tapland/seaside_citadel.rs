//! `cards/lands/tapland/seaside_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seaside Citadel prints "This land enters tapped" and "{T}: Add {G}, {W},
/// or {U}". Neither half is readable off the card file, so the scenario plays
/// them in the order the land prints them: played on turn one it is tapped
/// and offers nothing at all — the offer is read with an empty pool on an
/// otherwise empty board, so its absence is the tap symbol and no cost the
/// engine could not cover — and after one turn cycle the untap step has stood
/// it back up and the one line it prints stops to ask which of the three
/// colours is being made, and then makes exactly one of them.
#[test]
fn seaside_citadel_enters_tapped_and_taps_for_one_of_its_three_colours() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[seaside_citadel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let citadel = play_land(&mut engine, p0, seaside_citadel());
    assert!(
        entered_tapped(&engine, citadel),
        "\"This land enters tapped\" — the printed entry modifier"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so a missing ability cannot be a price the pool \
         would not have covered"
    );
    assert!(
        deeds(&legal, &[citadel]).is_empty(),
        "a land that entered tapped has no {{T}} to pay with this turn: {:?}",
        legal.abilities
    );

    // One turn cycle: its controller's untap step is the only thing that can
    // stand it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !entered_tapped(&engine, citadel),
        "the untap step ran, so the land that came in tapped is standing"
    );

    activate(&mut engine, p0, seaside_citadel(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}}, {{W}}, or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options.len(),
        3,
        "three colours and no fourth, which is what tells this apart from the \
         single type a basic land taps for: {options:?}"
    );
    for colour in [ManaColor::Green, ManaColor::White, ManaColor::Blue] {
        assert!(
            options.contains(&colour),
            "\"or\" includes {colour:?}: {options:?}"
        );
    }

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
        pool.total(),
        1,
        "one mana off one tap: the other two colours it could have made are \
         not made as well"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, citadel), "the land paid its own {{T}}");
}
