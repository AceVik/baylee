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

#[test]
fn elkin_bottle_allows_playing_an_exiled_land() {
    let mut engine = lk::game(SEED, &[elkin_bottle()], &[], &[index::FOREST], &[]);
    let forest = in_hand(&engine, P0, index::FOREST).expect("in hand");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            forest,
            ZoneLocation::Library(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move Forest to top of library");
    engine.refresh_offer();

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    activate(&mut engine, P0, elkin_bottle(), 0);
    pass_until(&mut engine, stack_is_empty);
    engine
        .apply(P0, PlayerAction::PlayLand { card: forest })
        .expect("Elkin Bottle's unrestricted play permission includes lands");

    assert!(
        on_battlefield(&engine, P0, index::FOREST).is_some(),
        "the exiled Forest entered the battlefield through a land play"
    );
}

/// The permission outlasts the turn — P0 may still cast the exiled Bolt in
/// P1's turn — retains the printed next-upkeep lifetime rather than the
/// shorter next-turn lifetime, and is gone before P0's next upkeep priority.
#[test]
fn elkin_bottle_permission_lasts_through_the_opponents_turn_and_expires_at_upkeep() {
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
    let permission = engine
        .state()
        .play_permissions
        .iter()
        .find(|permission| permission.player == P0 && permission.card == bolt)
        .expect("Elkin Bottle granted P0 permission to play the exiled Bolt");
    assert_eq!(
        permission.window,
        baylee_cards_dsl::PlayWindow::UntilYourNextUpkeep.into(),
        "the permission survives P0's next turn start and expires as upkeep begins"
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

    // P0's next upkeep: the permission ended before either player got priority.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    assert!(
        !castable_by_p0(&engine),
        "until the beginning of your next upkeep, and no longer"
    );
}
