//! Batch 43: graveyard behavior through real cards.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch43_drafna_targets_only_the_chosen_graveyard_and_preserves_order() {
    for count in 0..=2 {
        let mut e = prepared(
            &[
                index::DRAFNA_S_RESTORATION,
                index::SOL_RING,
                index::MANA_VAULT,
                index::FOREST,
            ],
            &[],
            &[],
        );
        let a = hand_to_graveyard(&mut e, P0, index::SOL_RING);
        let b = hand_to_graveyard(&mut e, P0, index::MANA_VAULT);
        let wrong = hand_to_graveyard(&mut e, P0, index::FOREST);
        cast_with_floating(&mut e, P0, index::DRAFNA_S_RESTORATION);
        e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
        let hash = e.snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![wrong],
                    players: vec![]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), hash);
        target(&mut e, [a, b][..count].to_vec(), vec![]);
        if count == 2 {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
            e.apply(
                P0,
                PlayerAction::Arrange {
                    piles: vec![vec![a, b]],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        let lib = library(&e, P0);
        if count > 0 {
            assert_eq!(lib.last(), Some(&a));
        }
        if count > 1 {
            assert_eq!(lib[lib.len() - 2], b);
        }
        assert_eq!(e.state().object(wrong).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn batch43_drafna_illegal_card_target_does_not_block_the_remaining_targets() {
    let mut e = prepared(
        &[
            index::DRAFNA_S_RESTORATION,
            index::SOL_RING,
            index::MANA_VAULT,
        ],
        &[],
        &[],
    );
    let a = hand_to_graveyard(&mut e, P0, index::SOL_RING);
    let b = hand_to_graveyard(&mut e, P0, index::MANA_VAULT);
    cast_with_floating(&mut e, P0, index::DRAFNA_S_RESTORATION);
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    target(&mut e, vec![a, b], vec![]);
    e.state
        .move_object(
            b,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library(&e, P0).last(), Some(&a));
    assert_eq!(e.state().object(b).unwrap().zone, Zone::Hand);
}

#[test]
fn batch43_drafna_copy_retargets_the_player_and_all_dependent_graveyard_cards() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(
            0,
            &[
                index::DRAFNA_S_RESTORATION,
                index::FORK,
                index::SOL_RING,
                index::MANA_VAULT,
            ],
        )
        .hand(1, &[index::WINTER_ORB, index::PITHING_NEEDLE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    let sol_ring = hand_to_graveyard(&mut e, P0, index::SOL_RING);
    let mana_vault = hand_to_graveyard(&mut e, P0, index::MANA_VAULT);
    let winter_orb = hand_to_graveyard(&mut e, P1, index::WINTER_ORB);
    let needle = hand_to_graveyard(&mut e, P1, index::PITHING_NEEDLE);
    let spell = in_hand(&e, P0, index::DRAFNA_S_RESTORATION).unwrap();
    cast_with_floating(&mut e, P0, index::DRAFNA_S_RESTORATION);
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    target(&mut e, vec![sol_ring, mana_vault], vec![]);
    cast_with_floating(&mut e, P0, index::FORK);
    target(&mut e, vec![spell], vec![]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    target(&mut e, vec![], vec![P1]);
    for id in [winter_orb, needle] {
        let Pending::ChooseTargets { options, min, .. } = e.pending() else {
            panic!("dependent target")
        };
        assert_eq!(
            *min, 1,
            "changing the player cannot keep an old graveyard target legal"
        );
        assert!(options.contains(&id));
        assert!(!options.contains(&sol_ring));
        target(&mut e, vec![id], vec![]);
    }
    for cards in [vec![winter_orb, needle], vec![sol_ring, mana_vault]] {
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        e.apply(P0, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library(&e, P0).last(), Some(&sol_ring));
    assert_eq!(library(&e, P1).last(), Some(&winter_orb));
}
