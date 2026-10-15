//! `cards/lands/caves/secret_tunnel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secret Tunnel: "This land can't be blocked." / "{T}: Add {C}." / "{4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn."
/// The static ability grants unblockable to the land itself, and activating ability 1 adds `{C}` to the mana pool.
#[test]
fn secret_tunnel_is_unblockable_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[secret_tunnel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = on_battlefield(&engine, p0, secret_tunnel()).expect("Secret Tunnel deployed");
    assert!(keywords(&engine, tunnel).contains(KeywordSet::UNBLOCKABLE));

    activate(&mut engine, p0, secret_tunnel(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tunnel));
}

/// Both targets share Wolf; an unrelated Bear and an opponent's Wolf are
/// excluded. The resolved ability removes both creatures from the block menu.
#[test]
fn secret_tunnel_requires_shared_creature_types_and_prevents_both_blocks() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                secret_tunnel(),
                index::TUNDRA_WOLVES,
                index::YOUNG_WOLF,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::TUNDRA_WOLVES, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let first = on_battlefield(&e, p0, index::TUNDRA_WOLVES).unwrap();
    let second = on_battlefield(&e, p0, index::YOUNG_WOLF).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let enemy = on_battlefield(&e, p1, index::TUNDRA_WOLVES).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, secret_tunnel(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("first pair target")
    };
    assert!(options.contains(&first));
    assert!(options.contains(&second));
    assert!(!options.contains(&bear));
    assert!(!options.contains(&enemy));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![first],
            players: vec![],
        },
    )
    .unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("expected second creature target, got {:?}", e.pending());
    };
    assert!(options.contains(&second));
    assert!(!options.contains(&first));
    assert!(!options.contains(&bear));
    assert!(!options.contains(&enemy));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![second],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(
        &e,
        on_battlefield(&e, p0, secret_tunnel()).unwrap()
    ));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(keywords(&e, first).contains(KeywordSet::UNBLOCKABLE));
    assert!(keywords(&e, second).contains(KeywordSet::UNBLOCKABLE));
    let blocks = super::super::super::legends_kit::attack(&mut e, &[first, second, bear]);
    assert!(
        !blocks
            .iter()
            .any(|block| block.attackers.contains(&first) || block.attackers.contains(&second))
    );
    assert!(blocks.iter().any(|block| block.attackers.contains(&bear)));
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(!keywords(&e, first).contains(KeywordSet::UNBLOCKABLE));
    assert!(!keywords(&e, second).contains(KeywordSet::UNBLOCKABLE));
}

/// A surviving target retains its benefit using the departed Wolf's last
/// creature types, whichever target left before resolution.
#[test]
fn secret_tunnel_uses_departed_targets_last_creature_types() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for remove_first in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[secret_tunnel(), index::TUNDRA_WOLVES, index::YOUNG_WOLF],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let first = on_battlefield(&e, p0, index::TUNDRA_WOLVES).unwrap();
        let second = on_battlefield(&e, p0, index::YOUNG_WOLF).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 4);
        e.refresh_offer();
        activate(&mut e, p0, secret_tunnel(), 2);
        for target in [first, second] {
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
        }
        let (departed, survivor) = if remove_first {
            (first, second)
        } else {
            (second, first)
        };
        crate::sba::destroy(e.dev_state_mut(p0).unwrap(), departed);
        pass_until(&mut e, stack_is_empty);
        assert!(keywords(&e, survivor).contains(KeywordSet::UNBLOCKABLE));
    }
}

/// Enough mana and two controlled creatures do not suffice when their
/// creature types differ: the target procedure must have a complete pair.
#[test]
fn secret_tunnel_is_not_offered_without_a_shared_type_pair() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[secret_tunnel(), index::TUNDRA_WOLVES, index::GRIZZLY_BEARS],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tunnel = on_battlefield(&e, p0, secret_tunnel()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending());
    };
    assert!(!legal.abilities.contains(&(tunnel, 2)));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tunnel,
                ability_index: 2
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
}

#[test]
fn secret_tunnel_rechecks_creature_types_on_resolution() {
    use baylee_core::generated::{index, subtypes};
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[secret_tunnel(), index::TUNDRA_WOLVES, index::YOUNG_WOLF],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let first = on_battlefield(&e, p0, index::TUNDRA_WOLVES).unwrap();
    let second = on_battlefield(&e, p0, index::YOUNG_WOLF).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, secret_tunnel(), 2);
    for target in [first, second] {
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    }
    // A transparent type-change fixture isolates the resolution restriction.
    // The production projection is invalidated and rebuilt before resolving.
    e.state.object_mut(second).unwrap().base_mut().subtypes =
        baylee_core::types::SubtypeSet::from_slice(&[subtypes::creature::BEAR]);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, first).contains(KeywordSet::UNBLOCKABLE));
    assert!(!keywords(&e, second).contains(KeywordSet::UNBLOCKABLE));
}

#[test]
fn secret_tunnel_requires_four_mana_and_an_untapped_land() {
    use crate::object::Status;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[secret_tunnel(), index::TUNDRA_WOLVES, index::YOUNG_WOLF],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let tunnel = on_battlefield(&e, p0, secret_tunnel()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state.players[0]
            .mana_pool
            .add(ManaColor::Colorless, if tapped { 4 } else { 3 });
        if tapped {
            state
                .object_mut(tunnel)
                .unwrap()
                .status
                .insert(Status::TAPPED);
        }
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(tunnel, 2)));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: tunnel,
                    ability_index: 2
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
}

#[test]
fn secret_tunnel_itself_cannot_be_blocked_when_it_is_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[secret_tunnel(), gray_ogre()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tunnel = on_battlefield(&e, p0, secret_tunnel()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let blocker = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    // Isolate the static printed clause with an animated-land fixture.
    let base = e.state.object_mut(tunnel).unwrap().base_mut();
    base.types = base.types.union(TypeSet::CREATURE);
    base.power = Some(2);
    base.toughness = Some(2);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let blocks = super::super::super::legends_kit::attack(&mut e, &[tunnel, ogre]);
    assert!(!super::super::super::legends_kit::may_block(
        &blocks, blocker, tunnel
    ));
    assert!(super::super::super::legends_kit::may_block(
        &blocks, blocker, ogre
    ));
}

#[test]
fn secret_tunnel_does_not_count_empty_or_noncreature_subtypes_as_shared() {
    use baylee_core::{
        generated::{index, subtypes},
        types::SubtypeSet,
    };
    let p0 = PlayerId::new(0);
    for subtype in [
        SubtypeSet::EMPTY,
        SubtypeSet::from_slice(&[subtypes::land::CAVE]),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[secret_tunnel(), index::TUNDRA_WOLVES, index::YOUNG_WOLF],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let tunnel = on_battlefield(&e, p0, secret_tunnel()).unwrap();
        let wolves = [
            on_battlefield(&e, p0, index::TUNDRA_WOLVES).unwrap(),
            on_battlefield(&e, p0, index::YOUNG_WOLF).unwrap(),
        ];
        for wolf in wolves {
            e.state.object_mut(wolf).unwrap().base_mut().subtypes = subtype;
        }
        e.state.invalidate_projections();
        e.sync_static_effects();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 4);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(tunnel, 2)));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: tunnel,
                    ability_index: 2
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
}

fn secret_tunnel_copies(e: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == card)
        })
        .collect()
}

fn secret_tunnel_choose(e: &mut Engine<RegistryLookup>, target: Option<ObjectId>) {
    e.apply(
        PlayerId::new(0),
        PlayerAction::ChooseTargets {
            objects: target.into_iter().collect(),
            players: vec![],
        },
    )
    .unwrap();
}

fn secret_tunnel_redirect_pair(change: u8) {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                secret_tunnel(),
                index::TUNDRA_WOLVES,
                index::TUNDRA_WOLVES,
                index::YOUNG_WOLF,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[index::DEFLECTING_SWAT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let wolves = secret_tunnel_copies(&e, index::TUNDRA_WOLVES);
    let third = on_battlefield(&e, p0, index::YOUNG_WOLF).unwrap();
    let bears = secret_tunnel_copies(&e, index::GRIZZLY_BEARS);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, secret_tunnel(), 2);
    for target in [wolves[0], wolves[1]] {
        secret_tunnel_choose(&mut e, Some(target));
    }
    let ability = e.state().zones.list(ZoneLocation::Stack)[0];
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 3);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, index::DEFLECTING_SWAT);
    secret_tunnel_choose(&mut e, Some(ability));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("first replacement")
    };
    assert!(options.contains(&third));
    assert!(options.contains(&bears[0]));
    let first = match change {
        0 => Some(third),
        1 => Some(bears[0]),
        _ => None,
    };
    secret_tunnel_choose(&mut e, first);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("second replacement")
    };
    if change == 1 {
        assert!(options.contains(&bears[1]));
        assert!(!options.contains(&third));
        assert!(!options.contains(&wolves[0]));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![third],
                    players: vec![]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    } else {
        assert!(!options.contains(&bears[0]));
    }
    let second = match change {
        0 => None,
        1 => Some(bears[1]),
        _ => Some(third),
    };
    secret_tunnel_choose(&mut e, second);
    pass_until(&mut e, stack_is_empty);
    let expected = match change {
        0 => [third, wolves[1]],
        1 => [bears[0], bears[1]],
        _ => [wolves[0], third],
    };
    for target in [wolves[0], wolves[1], third, bears[0], bears[1]] {
        assert_eq!(
            keywords(&e, target).contains(KeywordSet::UNBLOCKABLE),
            expected.contains(&target)
        );
    }
}

#[test]
fn secret_tunnel_swat_can_replace_first_and_retain_second_target() {
    secret_tunnel_redirect_pair(0);
}

#[test]
fn secret_tunnel_swat_can_replace_both_with_a_different_shared_type_pair() {
    secret_tunnel_redirect_pair(1);
}

#[test]
fn secret_tunnel_swat_can_retain_first_and_replace_second_target() {
    secret_tunnel_redirect_pair(2);
}
