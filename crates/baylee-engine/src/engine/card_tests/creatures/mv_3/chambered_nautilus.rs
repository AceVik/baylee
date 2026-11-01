//! `cards/creatures/mv_3/chambered_nautilus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0e1267d4-0155-4503-829a-03264c8d14a0"
fn chambered_nautilus() -> CardIndex {
    card_index("0e1267d4-0155-4503-829a-03264c8d14a0")
}

/// Chambered Nautilus ({2}{U} 2/2 Creature — Nautilus Beast):
/// "Whenever this creature becomes blocked, you may draw a card."
#[test]
fn chambered_nautilus_may_draw_a_card_when_blocked() {
    let mut engine = lk::game(
        SEED,
        &[chambered_nautilus()],
        &[index::GIANT_TORTOISE],
        &[],
        &[],
    );
    let nautilus = lk::obj(&engine, P0, chambered_nautilus());
    let tortoise = lk::obj(&engine, P1, index::GIANT_TORTOISE);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::attack(&mut engine, &[nautilus]);
    lk::block(&mut engine, &[(tortoise, nautilus)]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::through_combat(&mut engine);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Chambered Nautilus drew a card when it became blocked"
    );
}
