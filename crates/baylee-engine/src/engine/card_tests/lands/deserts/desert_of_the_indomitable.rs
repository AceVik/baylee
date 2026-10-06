//! `cards/lands/deserts/desert_of_the_indomitable.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert of the Indomitable prints three lines — it enters tapped, it taps
/// for {G}, and it cycles for {1}{G} out of the hand — and no single copy can
/// show both of the last two, so the scenario runs two.
///
/// The played copy goes through `PlayLand`, where `EnterModifier::Tapped`
/// actually applies (`starting_battlefield` places a permanent without an
/// entry, so a board built that way would arrive untapped and measure
/// nothing); the tapped-out pool afterwards is what says so, because two
/// Forests make exactly two green where an untapped Desert would have made a
/// third. The second copy cycles from hand — the cost is the discard itself,
/// so it lands in the graveyard and the draw is a real card off the library —
/// and one turn cycle later the played land has untapped and taps for the one
/// green the last assertion counts.
#[test]
fn desert_of_the_indomitable_enters_tapped_cycles_from_hand_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(
            0,
            &[desert_of_the_indomitable(), desert_of_the_indomitable()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let played = play_land(&mut engine, p0, desert_of_the_indomitable());
    assert!(
        entered_tapped(&engine, played),
        "Desert of the Indomitable enters tapped, and a played land is a real entry"
    );

    // The whole pool this turn: the tapped Desert is a land that produces
    // nothing until its controller's next untap step.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Forests, and no third from the Desert: it entered tapped"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "both of them green, paid by the Forests alone"
    );

    // The second copy is still in hand, where its cycling line lives
    // (`ActivationZone::Hand`) — and it is offered now that the {1}{G} is
    // floating, because the offer is read off the pool.
    let cycling =
        in_hand(&engine, p0, desert_of_the_indomitable()).expect("the second copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cycling, 1)),
        "cycling is an activation a card in hand offers: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, desert_of_the_indomitable(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, desert_of_the_indomitable()).is_some(),
        "the cost is the discard itself, so the cycled card is in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the effect is a real draw off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn: the hand is the size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} was the price, and the two green in the pool paid it"
    );
    assert!(
        on_battlefield(&engine, p0, desert_of_the_indomitable()).is_some(),
        "the copy that was played is a permanent and did not move"
    );

    // One turn later the played land has untapped, which is what makes the
    // tap below a reading of its mana line rather than of a tapped land.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, played),
        "the untap step ran, so the Desert is standing again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and no mana survived the steps in between (CR 500.4)"
    );

    activate(&mut engine, p0, desert_of_the_indomitable(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "{{T}}: Add {{G}} — exactly one green, which is the whole of the line"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, played), "the land paid its own {{T}}");
}
