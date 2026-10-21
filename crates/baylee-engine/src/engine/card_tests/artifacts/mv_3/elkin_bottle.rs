//! `cards/artifacts/mv_3/elkin_bottle.rs`, played.
//!
//! Oracle:
//! - "{3}, {T}: Exile the top card of your library. Until the beginning of your next upkeep, you may play that card."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1e381361-7404-4654-bfb4-5ea65f6e4dce"
fn elkin_bottle() -> CardIndex {
    card_index("1e381361-7404-4654-bfb4-5ea65f6e4dce")
}

#[test]
fn elkin_bottle_exiles_top_card_and_allows_playing_it() {
    let mut engine = lk::game(SEED, &[elkin_bottle()], &[], &[index::GRIZZLY_BEARS], &[]);
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("in hand");
    let state = engine.dev_state_mut(P0).unwrap();
    state
        .move_object(
            bears,
            ZoneLocation::Library(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move Grizzly Bears to top of library");
    engine.refresh_offer();

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    activate(&mut engine, P0, elkin_bottle(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&bears),
        "top card of library was exiled"
    );

    // Cast the exiled Grizzly Bears using the play permission granted by Elkin Bottle.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: bears })
        .expect("cast exiled card granted by Elkin Bottle");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "exiled card was played onto the battlefield"
    );
}

/// The permission outlasts the turn — P0 may still cast the exiled Bolt in
/// P1's turn — and is gone by P0's next main phase (under the printed "until
/// the beginning of your next upkeep" as under the Partial's next turn).
#[test]
fn elkin_bottle_permission_lasts_through_the_opponents_turn_and_no_further() {
    let mut engine = lk::game(SEED, &[elkin_bottle()], &[], &[index::LIGHTNING_BOLT], &[]);
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).expect("in hand");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            bolt,
            ZoneLocation::Library(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move Lightning Bolt to top of library");
    engine.refresh_offer();

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    activate(&mut engine, P0, elkin_bottle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&bolt),
        "the top card of the library was exiled"
    );

    let castable_by_p0 = |e: &Engine<RegistryLookup>| matches!(e.pending(), Pending::Priority { player, legal } if *player == P0 && legal.castable.contains(&bolt));

    // P1's first main phase, P0 holding priority with {R} floating.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    assert!(
        castable_by_p0(&engine),
        "the exiled card may still be played in the opponent's turn"
    );

    // P0's next main phase: the permission is over.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    assert!(
        !castable_by_p0(&engine),
        "until the beginning of your next upkeep, and no longer"
    );
}
