//! `cards/enchantments/auras/mv_2/cloak_of_confusion.rs`, played.
//!
//! Oracle:
//! - "Enchant creature you control"
//! - "Whenever enchanted creature attacks and isn't blocked, you may have it assign no combat damage this turn. If you do, defending player discards a card at random."
//!
//! Played at a table of two, where the card is whole (it is Partial for
//! larger tables: see the card file).

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

/// P0's Grizzly Bears wearing the Cloak, P1 holding one Hill Giant and
/// controlling `p1_board`.
fn cloaked_bears(p1_board: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        p1_board,
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
    let options = aim_at(&mut engine, P0, bears);
    let mine = |id: &ObjectId| {
        engine
            .state()
            .object(*id)
            .is_some_and(|o| o.controller == P0)
    };
    assert!(
        options.iter().all(mine),
        "enchant creature you control: no opponent's creature is offered"
    );
    pass_until(&mut engine, stack_is_empty);
    (engine, bears)
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

#[test]
fn cloak_of_confusion_trades_unblocked_combat_damage_for_random_discard() {
    let (mut engine, bears) = cloaked_bears(&[]);

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::through_combat(&mut engine);

    assert_eq!(
        hand_size(&engine, P1),
        0,
        "defending player discarded their card at random"
    );
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "enchanted creature assigned no combat damage this turn"
    );
}

#[test]
fn cloak_of_confusion_declined_deals_damage_and_discards_nothing() {
    let (mut engine, bears) = cloaked_bears(&[]);

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    lk::through_combat(&mut engine);

    assert_eq!(
        hand_size(&engine, P1),
        1,
        "no discard without the \"if you do\""
    );
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "the Bears dealt their combat damage"
    );
}

#[test]
fn cloak_of_confusion_does_not_trigger_when_the_creature_is_blocked() {
    let (mut engine, bears) = cloaked_bears(&[index::WALL_OF_STONE]);
    let wall = lk::obj(&engine, P1, index::WALL_OF_STONE);

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(wall, bears)]);
    // `pass_until` would answer a "may" yes on the way; nothing asks.
    lk::through_combat(&mut engine);

    assert_eq!(
        hand_size(&engine, P1),
        1,
        "a blocked creature is not \"attacks and isn't blocked\""
    );
}
