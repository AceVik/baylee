//! `cards/sorceries/mv_6/wildfire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wildfire() -> CardIndex {
    card_index("0da68ca2-07fd-4359-95a9-fb2f815e6ed0")
}

fn mahamoti_djinn() -> CardIndex {
    card_index("c39ea5f9-6ec0-4697-897b-779e326754a7")
}

/// Wildfire (0da68ca2-07fd-4359-95a9-fb2f815e6ed0) — {4}{R}{R} Sorcery:
/// "Each player sacrifices four lands of their choice. Wildfire deals 4 damage
/// to each creature."
#[test]
fn wildfire_makes_each_player_sacrifice_four_lands_and_deals_four_damage_to_creatures() {
    let mut p0_board = vec![mountain(); 5];
    p0_board.extend_from_slice(&[plains(), sol_ring(), fangren_hunter()]);
    let mut p1_board = vec![forest(); 4];
    p1_board.extend_from_slice(&[island(), aurochs(), mahamoti_djinn()]);
    let mut engine = lk::game(SEED, &p0_board, &p1_board, &[wildfire()], &[]);
    let my_plains = lk::obj(&engine, P0, plains());
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let my_hunter = lk::obj(&engine, P0, fangren_hunter());
    let their_island = lk::obj(&engine, P1, island());
    let their_beast = lk::obj(&engine, P1, aurochs());
    let their_djinn = lk::obj(&engine, P1, mahamoti_djinn());

    tap_mana_except(&mut engine, P0, my_ring);
    cast_with_floating(&mut engine, P0, wildfire());

    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(prompt, ChoicePrompt::KeepLands);
                assert!(!options.contains(&my_ring));
                let kept = if player == P0 {
                    assert_eq!((min, max), (2, 2), "6 lands - 4 sacrificed = 2 kept");
                    let extra_mtn = options.iter().copied().find(|id| *id != my_plains).unwrap();
                    vec![my_plains, extra_mtn]
                } else {
                    assert_eq!((min, max), (1, 1), "5 lands - 4 sacrificed = 1 kept");
                    vec![their_island]
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: kept })
                    .unwrap();
            }
            other => panic!("unexpected pending during Wildfire: {other:?}"),
        }
    }

    assert_eq!(lk::zone_of(&engine, my_plains), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Battlefield);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        6,
        "4 sacrificed Mountains + Fangren Hunter + Wildfire"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        5,
        "4 sacrificed Forests + Aurochs"
    );
    assert_eq!(lk::zone_of(&engine, my_ring), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, my_hunter), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_beast), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_djinn), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, their_djinn), 4);
    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 20);
}

/// Wildfire — a player controlling fewer than four lands sacrifices all lands
/// they control.
#[test]
fn wildfire_sacrifices_all_lands_when_a_player_controls_fewer_than_four() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
        ],
        &[forest(), island()],
        &[wildfire()],
        &[],
    );
    let their_forest = lk::obj(&engine, P1, forest());
    let their_island = lk::obj(&engine, P1, island());

    cast_from_hand(&mut engine, P0, wildfire());
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let chosen = options.into_iter().take(usize::from(min)).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert_eq!(lk::zone_of(&engine, their_forest), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Graveyard);
}
