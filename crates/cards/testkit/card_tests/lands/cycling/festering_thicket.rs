//! `cards/lands/cycling/festering_thicket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Festering Thicket is a `Swamp Forest` that "enters tapped" and carries
/// `Cycling {2}` — an ability the card files under `ActivationZone::Hand`, so
/// the two halves need different parts of one game to be readable at all.
/// The entry is a real land drop rather than a `starting_battlefield` seat: a
/// permanent *placed* by the setup path is never replaced, so it would arrive
/// untapped whatever the card prints and the assertion would describe a board
/// no game reaches. The next turn reads the other two clauses off the same
/// permanent: once the untap step has stood it up it is a mana source, and the
/// copy still in hand cycles itself for `{2}` — a cost paid from a pool the
/// Forests filled, which is why the tap happens before the ability is asked
/// for. The uncycled copy is checked to still be standing, so the discard and
/// the draw cannot be read as the played land leaving and coming back.
#[test]
fn festering_thicket_enters_tapped_taps_for_mana_and_cycles_itself_for_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[festering_thicket(), festering_thicket()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, festering_thicket());
    assert!(
        entered_tapped(&engine, land),
        "the printed entry is tapped — and this is a real `PlayLand`, so the \
         entry replacement really ran"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&land)
            && !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped Swamp Forest has no {{T}} to spend, so it is offered nothing"
    );

    // Across the opponent's turn and back: the untap step is what the second
    // half of the card needs, and it has not happened yet.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and the Thicket: one mana off each, so the land is a \
         source and not scenery"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The mana ability is index 0 and cycling is 1, and cycling's price is
    // `{2}` plus the card itself — which is what the pool above is for, and
    // why the lands are tapped first and the ability asked for afterwards.
    activate(&mut engine, p0, festering_thicket(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, festering_thicket()).is_some(),
        "`DiscardSelf` is the cost, so the card is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the effect draws one card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn: the hand is the size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}} came out of the pool the lands filled"
    );
    assert!(
        on_battlefield(&engine, p0, festering_thicket()).is_some(),
        "the copy that was played is untouched by the copy that cycled"
    );
}
