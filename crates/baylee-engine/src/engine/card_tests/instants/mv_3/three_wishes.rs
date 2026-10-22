//! `cards/instants/mv_3/three_wishes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn three_wishes() -> CardIndex {
    card_index("6b3110a5-fd38-4d09-9073-c8a6f3f17840")
}

/// `Three Wishes` ({1}{U}{U}, Instant, `Coverage::Partial`) exiles the top
/// three cards of your library and lets you play them until your next turn.
#[test]
fn three_wishes_exiles_top_three_cards_to_play_until_next_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[three_wishes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, three_wishes());
    pass_until(&mut engine, stack_is_empty);

    let exiled = engine.state().zones.list(ZoneLocation::Exile(p0)).to_vec();
    assert_eq!(exiled.len(), 3);

    engine
        .apply(p0, PlayerAction::PlayLand { card: exiled[0] })
        .expect("can play exiled land from Three Wishes");
    assert_eq!(
        engine.state().object(exiled[0]).unwrap().zone,
        Zone::Battlefield
    );
}
