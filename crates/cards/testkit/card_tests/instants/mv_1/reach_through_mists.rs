//! `cards/instants/mv_1/reach_through_mists.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reach Through Mists costs `{U}`, is an instant and prints exactly
/// one line: "Draw a card." It is therefore played where a sorcery
/// could no longer be played — in the end step of its own turn, in which
/// only the active player gets priority (CR 117.3a) —, and the one card
/// it draws is the previously top card of the library, which afterwards
/// lies in hand as the same object. The spell itself goes to the graveyard
/// after resolution, the library is shorter by exactly one, and the `{U}` has
/// disappeared from the pool, because the assertion about `castable` came
/// only after the tapping.
#[test]
fn reach_through_mists_draws_the_top_card_at_instant_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, island())
        .battlefield(0, &[island()])
        .hand(0, &[reach_through_mists()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene erste Hauptphase"
    );

    // Into the end step of its own turn: a sorcery would no longer be
    // playable here, an instant would.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let spell = in_hand(&engine, p0, reach_through_mists()).expect("the instant is in hand");

    // First the mana into the pool, then the assertion: `LegalActions` is
    // filtered behind `can_afford` and reads the pool, not the untapped
    // lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one Island, one blue mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{U}} is in the pool, so the spell is playable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, reach_through_mists());
    assert!(
        on_stack(&engine, reach_through_mists()).is_some(),
        "the spell is on the stack before anyone passes"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} is paid"
    );
    assert!(
        in_graveyard(&engine, p0, reach_through_mists()).is_some(),
        "a resolved instant goes to the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "exactly one card has left the library"
    );
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert!(
        hand.contains(&top),
        "\"Draw a card\": the previously top card of the library is now in \
         hand"
    );
    assert!(
        !hand.contains(&spell),
        "and the spell itself has gone — if it remained in hand, the hand \
         size would be unchanged and nothing would have left it"
    );
}
