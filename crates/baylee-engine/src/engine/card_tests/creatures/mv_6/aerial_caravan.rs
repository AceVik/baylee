//! `cards/creatures/mv_6/aerial_caravan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d13aff7c-f0ee-44e2-ae1c-62cd86ad46a6"
fn aerial_caravan() -> CardIndex {
    card_index("d13aff7c-f0ee-44e2-ae1c-62cd86ad46a6")
}

/// Aerial Caravan ({4}{U}{U} 4/3 Flying Creature — Human Soldier):
/// "{1}{U}{U}: Exile the top card of your library. Until end of turn, you may
/// play that card."
#[test]
fn aerial_caravan_exiles_top_card_and_allows_playing_it_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[aerial_caravan(), index::HILL_GIANT],
        &[],
    );
    let giant = in_hand(&engine, P0, index::HILL_GIANT).expect("in hand");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            giant,
            ZoneLocation::Library(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("put Hill Giant on top of library");
    engine.refresh_offer();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 4), (ManaColor::Blue, 2)],
    );
    cast_with_floating(&mut engine, P0, aerial_caravan());
    lk::settle(&mut engine);

    let caravan = lk::obj(&engine, P0, aerial_caravan());
    assert_eq!(pt(&engine, caravan), (4, 3));

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
    );
    activate(&mut engine, P0, aerial_caravan(), 0);
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&giant),
        "top card of library was exiled"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: giant })
        .expect("may play exiled card until end of turn");
    lk::settle(&mut engine);

    assert!(
        on_battlefield(&engine, P0, index::HILL_GIANT).is_some(),
        "exiled card resolved onto the battlefield"
    );

    // Next turn, Aerial Caravan attacks in the air past ground Grizzly Bears.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let offer = lk::attack(&mut engine, &[caravan]);
    assert!(
        !lk::may_block(&offer, bears, caravan),
        "ground Grizzly Bears cannot block flying Aerial Caravan"
    );
}
