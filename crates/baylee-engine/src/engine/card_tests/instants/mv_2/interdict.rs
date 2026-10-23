//! `cards/instants/mv_2/interdict.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn interdict() -> CardIndex {
    card_index("93771480-2672-4bc7-81a8-011d3b8878b0")
}

/// Interdict (`93771480-2672-4bc7-81a8-011d3b8878b0`) — {1}{U} Instant (Partial):
/// "Counter target activated ability from an artifact, creature, enchantment, or land. ... Draw a card."
#[test]
fn interdict_counters_target_activated_ability_and_draws_a_card() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[index::PRODIGAL_SORCERER])
        .hand(0, &[interdict()])
        .hand(1, &[index::GIANT_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, P1, index::PRODIGAL_SORCERER, 0);
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    let sorcerer = on_battlefield(&engine, P1, index::PRODIGAL_SORCERER).unwrap();
    crate::engine::card_tests::legends_kit::float(&mut engine, P1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, P1, index::GIANT_GROWTH);
    aim_at(&mut engine, P1, sorcerer);
    let growth = in_hand(&engine, P1, index::GIANT_GROWTH);
    assert!(growth.is_none());
    let growth_spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    let ability_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    cast_from_hand(&mut engine, P0, interdict());
    let menu = aim_at(&mut engine, P0, ability_id);
    assert!(!menu.contains(&growth_spell));
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_after_cast + 1
    );
}
