//! `cards/instants/mv_1/entomb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "299fc083-0834-4064-8344-f895aff68867"
fn entomb() -> CardIndex {
    card_index("299fc083-0834-4064-8344-f895aff68867")
}

/// Entomb ({B}): searches your library for any card, puts it directly into
/// your graveyard (not your hand), and shrinks your library by 1.
#[test]
fn entomb_searches_library_and_puts_chosen_card_into_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[entomb(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Move Hill Giant from hand into library so we can find a unique card.
    let giant = in_hand(&engine, P0, index::HILL_GIANT).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            giant,
            ZoneLocation::Library(P0),
            baylee_engine::zone::ZonePosition::Bottom,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, entomb());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected library search, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(
        (min, max),
        (1, 1),
        "must find a card when library is non-empty"
    );
    assert!(options.contains(&giant), "Hill Giant is offered in library");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        in_graveyard(&engine, P0, index::HILL_GIANT),
        Some(giant),
        "chosen card moved straight from library to graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before - 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1,
        "Entomb went to graveyard; searched card did not go to hand"
    );
}
