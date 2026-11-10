//! `cards/lands/tapland/tinder_farm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tinder Farm prints three sentences and each one is read off a different
/// place. "This land enters tapped" is only visible through a real land drop:
/// a `starting_battlefield` placement is a `move_object(.., Cause::Setup)` and
/// not an entry, so a seeded copy arrives standing — and while it is tapped,
/// neither `{T}` line is offered at all, which the empty pool says is about
/// the tap and not about a price. A turn cycle later (CR 502.3) the same
/// permanent taps for `{G}` with the stack untouched (CR 605.3b), and on the
/// turn after that it trades itself for `{R}{W}`: exactly one red and one
/// white, because CR 500.5 emptied the green between the two activations.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn tinder_farm_enters_tapped_then_taps_for_green_and_sells_itself_for_red_and_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[tinder_farm()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let farm = play_land(&mut engine, p0, tinder_farm());
    assert!(
        is_tapped(&engine, farm),
        "\"This land enters tapped\" — and this is a real land drop, not a \
         `starting_battlefield` placement, which is a move and would have left \
         it standing"
    );

    // A tapped land has no `{T}` to pay with, so neither printed line is
    // offered. Neither of them costs mana either, so the empty pool below is
    // what rules out `can_afford` as the reason.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that played it holds priority");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing on this board has made mana yet"
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == farm),
        "a tapped Tinder Farm offers nothing: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back, which is what the untap step needs.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the land's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, farm),
        "the untap step stood it back up (CR 502.3)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(farm, 0)),
        "`{{T}}: Add {{G}}` is the land's first printed line: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(farm, 1)),
        "and `{{T}}, Sacrifice this land: Add {{R}}{{W}}` is the other: {:?}",
        legal.abilities
    );

    // Pressed by object and not by card handle: the index-based helper would
    // also have matched any second copy, and this is the permanent whose tap
    // the assertions below are about.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: farm,
                ability_index: 0,
            },
        )
        .expect("the untapped land pays its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the green is already here"
    );
    assert!(is_tapped(&engine, farm), "the tap was the whole price");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "`{{G}}`");
    assert_eq!(
        pool.total(),
        1,
        "one mana, and nothing else on this board made any"
    );
    assert!(
        on_battlefield(&engine, p0, tinder_farm()).is_some(),
        "a mana ability costs the land nothing but its tap"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and a third turn for its controller"
    );
    assert!(!is_tapped(&engine, farm), "the land is back on its feet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 500.5 emptied the pool at the end of the step the green was made in"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: farm,
                ability_index: 1,
            },
        )
        .expect("the untapped land pays {{T}} and gives itself up");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "neither the tap nor the sacrifice of the source is a question: \
         sacrifice-as-self needs no menu, got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the sacrifice and the two mana never touch the stack"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "`{{R}}`");
    assert_eq!(pool.available(ManaColor::White), 1, "and `{{W}}` beside it");
    assert_eq!(
        pool.total(),
        2,
        "exactly the two the line prints — the previous turn's green is gone"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and no green, which a pool of two alone would not have told apart"
    );
    assert!(
        on_battlefield(&engine, p0, tinder_farm()).is_none(),
        "\"Sacrifice this land\" is the other half of the price"
    );
    assert!(
        in_graveyard(&engine, p0, tinder_farm()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
