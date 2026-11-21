//! `cards/sorceries/mv_3/trash_for_treasure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn trash_for_treasure() -> CardIndex {
    card_index("e69279f6-50a1-4db5-83be-30a2ea7fc6a9")
}

/// Trash for Treasure (e69279f6-50a1-4db5-83be-30a2ea7fc6a9) — {2}{R} Sorcery:
/// "As an additional cost to cast this spell, sacrifice an artifact. Return
/// target artifact card from your graveyard to the battlefield."
#[test]
fn trash_for_treasure_sacrifices_artifact_and_returns_target_artifact_from_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            darksteel_pendant(),
            aurochs(),
        ],
        &[],
        &[trash_for_treasure(), sol_ring(), llanowar_elves()],
        &[lightning_greaves()],
    );
    let pendant = lk::obj(&engine, P0, darksteel_pendant());
    let gy_ring = hand_to_graveyard(&mut engine, P0, sol_ring());
    let gy_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let their_gy_art = hand_to_graveyard(&mut engine, P1, lightning_greaves());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, trash_for_treasure());
    let menu = aim_at(&mut engine, P0, gy_ring);
    assert_eq!(menu, vec![gy_ring]);
    assert!(!menu.contains(&gy_elf) && !menu.contains(&their_gy_art));

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(options, vec![pendant]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![pendant],
            },
        )
        .unwrap();
    assert_eq!(lk::zone_of(&engine, pendant), Zone::Graveyard);

    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, gy_ring), Zone::Battlefield);
    assert_eq!(engine.state().object(gy_ring).unwrap().controller, P0);

    let mut no_art = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[sol_ring()],
        &[trash_for_treasure(), lightning_greaves()],
        &[],
    );
    hand_to_graveyard(&mut no_art, P0, lightning_greaves());
    tap_all_mana(&mut no_art, P0);
    assert!(!lk::castable(&no_art, P0, trash_for_treasure()));
}
