//! `cards/instants/mv_4/thwart.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "615a149a-2869-4b18-bbbe-9ec1825049d7"
fn thwart() -> CardIndex {
    card_index("615a149a-2869-4b18-bbbe-9ec1825049d7")
}

/// `Thwart` (`Coverage::Partial`: base mana cost `{2}{U}{U}`):
/// "Counter target spell."
#[test]
fn thwart_counters_target_spell() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST, index::FOREST],
        &[index::ISLAND, index::ISLAND, index::ISLAND, index::ISLAND],
        &[index::GRIZZLY_BEARS],
        &[thwart()],
    );

    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, index::GRIZZLY_BEARS).expect("Grizzly Bears on stack");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, thwart());
    lk::aim(&mut engine, &[spell_id], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
}
