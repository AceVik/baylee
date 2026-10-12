#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch48_valakut_requires_five_other_mountains_and_rechecks_on_resolution() {
    for (existing, remove_one) in [(4, false), (5, false), (5, true)] {
        let mut board = vec![index::VALAKUT_THE_MOLTEN_PINNACLE];
        board.extend(std::iter::repeat_n(index::MOUNTAIN, existing));
        let mut e = prepared(&[index::MOUNTAIN], &board, &[]);
        let before = e.state.players[1].life;
        play_land(&mut e, P0, index::MOUNTAIN);
        if existing == 4 {
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[1].life, before);
            continue;
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        targets(&mut e, vec![], vec![P1]);
        if remove_one {
            // Fixture movement isolates the resolution recheck; the trigger itself
            // was caused by the actual land-play action above.
            let mountain = source(&e, P0, index::MOUNTAIN);
            e.state
                .move_object(
                    mountain,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[1].life,
            before - if remove_one { 0 } else { 3 }
        );
    }
}

#[test]
fn batch48_miter_rewards_destroyed_artifact_but_never_sacrificed_artifact() {
    let mut e = prepared(
        &[index::SHATTER],
        &[index::URZA_S_MITER, index::SOL_RING, index::BLACK_LOTUS],
        &[],
    );
    let ring = source(&e, P0, index::SOL_RING);
    let hand = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    cast(&mut e, index::SHATTER);
    targets(&mut e, vec![ring], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), hand);
    let library = library_size(&e, P0);
    ability(&mut e, P0, index::BLACK_LOTUS, 0);
    if matches!(e.pending(), Pending::ChooseColor { .. }) {
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P0), library);
}

#[test]
fn batch48_landing_transforms_only_when_three_creatures_attack_together() {
    for n in [2, 3] {
        let mut e = prepared(
            &[index::LEGION_S_LANDING],
            &[
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
            ],
            &[],
        );
        cast(&mut e, index::LEGION_S_LANDING);
        pass_until(&mut e, stack_is_empty);
        let landing = source(&e, P0, index::LEGION_S_LANDING);
        assert_eq!(token_count(&e, subtypes::creature::VAMPIRE), 1);
        let bears: Vec<_> = e
            .state
            .battlefield_seen()
            .filter(|id| {
                e.state
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS))
            })
            .take(n)
            .collect();
        attackers(&mut e, &bears);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        assert_eq!(face(&e, landing), u8::from(n == 3));
        if n == 3 {
            e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap();
            priority(&mut e, P0);
            for c in ManaColor::ALL {
                e.state.players[0].mana_pool.add(c, 5);
            }
            e.refresh_offer();
            ability(&mut e, P0, index::LEGION_S_LANDING, 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(token_count(&e, subtypes::creature::VAMPIRE), 2);
        }
    }
}

#[test]
fn batch48_storm_makes_one_treasure_per_damaged_player_in_one_damage_step() {
    let mut e = prepared(
        &[],
        &[
            index::STORM_THE_VAULT,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let bears: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS))
        })
        .collect();
    let before = e.state.players[1].life;
    attackers(&mut e, &bears);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert_eq!(e.state.players[1].life, before - 4);
    assert_eq!(token_count(&e, subtypes::artifact::TREASURE), 1);
}

#[test]
fn batch48_storm_transforms_at_five_artifacts_and_back_mana_counts_them() {
    let mut e = prepared(
        &[],
        &[
            index::STORM_THE_VAULT,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
        ],
        &[],
    );
    let vault = source(&e, P0, index::STORM_THE_VAULT);
    pass_until(&mut e, |e| face(e, vault) == 1 && stack_is_empty(e));
    priority(&mut e, P0);
    let before = e.state.players[0].mana_pool.available(ManaColor::Blue);
    ability(&mut e, P0, index::STORM_THE_VAULT, 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Blue),
        before + 5
    );
}
