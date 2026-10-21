//! `cards/instants/mv_2/incinerate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn incinerate() -> CardIndex {
    card_index("d8fd7a34-8418-4e98-b79b-119c4348c667")
}

/// Incinerate (d8fd7a34-8418-4e98-b79b-119c4348c667) — {1}{R} Instant:
/// - "Incinerate deals 3 damage to any target. A creature dealt damage this way can't be regenerated this turn."
#[test]
fn incinerate_kills_a_creature_through_a_regeneration_shield_and_can_target_a_player() {
    let mut engine = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN, index::MOUNTAIN])
        .hand(0, &[incinerate()])
        .battlefield(1, &[index::DRUDGE_SKELETONS, index::SWAMP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let skel = on_battlefield(&engine, P1, index::DRUDGE_SKELETONS).expect("Skeletons seated");

    // P0 casts Incinerate targeting P1's Drudge Skeletons.
    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, incinerate());
    aim_at(&mut engine, P0, skel);

    // In response, P1 activates Drudge Skeletons' "{B}: Regenerate this creature."
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    activate(&mut engine, P1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, index::DRUDGE_SKELETONS).is_some(),
        "Incinerate's 'can't be regenerated this turn' prevents the regeneration shield from saving Drudge Skeletons"
    );
}

#[test]
fn incinerate_deals_three_damage_to_target_player() {
    let mut engine = lk::game(
        SEED,
        &[index::MOUNTAIN, index::MOUNTAIN],
        &[],
        &[incinerate()],
        &[],
    );
    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, incinerate());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().players[1].life, 17);
}
