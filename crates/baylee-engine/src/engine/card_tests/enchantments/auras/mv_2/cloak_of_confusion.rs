//! `cards/enchantments/auras/mv_2/cloak_of_confusion.rs`, played.
//!
//! Oracle:
//! - "Enchant creature you control"
//! - "Whenever enchanted creature attacks and isn't blocked, you may have it assign no combat damage this turn. If you do, defending player discards a card at random."
//!
//! The defending player follows the enchanted attacker at multiplayer tables too.

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

#[test]
fn cloak_of_confusion_discards_only_the_attacked_players_card_at_a_three_player_table() {
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .hand(0, &[cloak_of_confusion()])
        .hand(1, &[index::HILL_GIANT])
        .hand(2, &[index::SHIVAN_DRAGON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
    );
    lk::cast_at(&mut engine, P0, cloak_of_confusion(), &[bears], &[]);
    lk::attackers_offered(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p2))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p2, "the attacked opponent declares blockers");
    lk::block(&mut engine, &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::through_combat(&mut engine);
    assert_eq!(
        hand_size(&engine, P1),
        1,
        "the other opponent keeps their card"
    );
    assert_eq!(
        hand_size(&engine, p2),
        0,
        "the attacked player discarded at random"
    );
    assert_eq!(
        lk::life(&engine, p2),
        20,
        "the attacker assigned no combat damage"
    );
}
