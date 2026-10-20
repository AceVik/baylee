//! `cards/enchantments/auras/mv_2/cloak_of_confusion.rs`, played.
//!
//! Oracle:
//! - "Enchant creature you control"
//! - "Whenever enchanted creature attacks and isn't blocked, you may have it assign no combat damage this turn. If you do, defending player discards a card at random."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5d258409-28d3-4eac-9f1a-3e494db818bf"
fn cloak_of_confusion() -> CardIndex {
    card_index("5d258409-28d3-4eac-9f1a-3e494db818bf")
}

#[test]
fn cloak_of_confusion_trades_unblocked_combat_damage_for_random_discard() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[cloak_of_confusion()],
        &[index::HILL_GIANT],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, cloak_of_confusion());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);

    // Accept the MayDo trigger: assign no combat damage this turn, defending player discards at random.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("choose to assign no combat damage and make defending player discard at random");

    lk::through_combat(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        0,
        "defending player discarded their card at random"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "enchanted creature assigned no combat damage this turn"
    );
}
