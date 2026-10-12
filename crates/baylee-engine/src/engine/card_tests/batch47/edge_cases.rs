//! Payment-context, variable-cost and immutable discard-history regressions.
//!
//! Deferred Clergy case: with a pre-existing regeneration shield, its controller
//! chooses between that one-shot replacement and the standing regeneration
//! replacement (verified CR 701.19a/b and 616.1). The synchronous destruction
//! path currently selects standing regeneration; root owns the choice contract.
#[allow(clippy::wildcard_imports)]
use super::*;

fn reject_cave(e: &mut Engine<RegistryLookup>, cave: ObjectId) {
    let Pending::Priority { player, legal } = e.pending() else {
        panic!("payment priority")
    };
    let player = *player;
    assert!(!legal.abilities.contains(&(cave, 0)));
    assert!(!legal.mana_abilities.contains(&cave));
    for action in [
        PlayerAction::ActivateAbility {
            source: cave,
            ability_index: 0,
        },
        PlayerAction::ActivateManaAbility { source: cave },
    ] {
        let before = e.snapshot_hash();
        assert!(e.apply(player, action).is_err());
        assert_eq!(e.snapshot_hash(), before);
    }
}

#[test]
fn batch47_rhystic_priority_only_is_excluded_from_cast_payment_window() {
    let mut e = prepared(
        &[index::SOL_RING],
        &[index::RHYSTIC_CAVE, index::FOREST],
        &[],
    );
    let cave = on_battlefield(&e, P0, index::RHYSTIC_CAVE).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    set_pool(&mut e, P0, &[]);
    let ring = in_hand(&e, P0, index::SOL_RING).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: ring }).unwrap();
    reject_cave(&mut e, cave);
    e.apply(P0, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch47_rhystic_priority_only_is_excluded_from_resolution_tax_payment_window() {
    let mut e = prepared(
        &[index::RUPTURE_SPIRE],
        &[index::RHYSTIC_CAVE, index::FOREST],
        &[],
    );
    let cave = on_battlefield(&e, P0, index::RHYSTIC_CAVE).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    set_pool(&mut e, P0, &[]);
    let spire = play_land(&mut e, P0, index::RUPTURE_SPIRE);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    reject_cave(&mut e, cave);
    e.apply(P0, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spire).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch47_power_artifact_x_cost_reduction_and_floor_follow_announced_x() {
    for (auras, x, price) in [
        (1_u8, 0_u8, 0_u32),
        (1, 1, 1),
        (1, 2, 1),
        (1, 3, 1),
        (2, 5, 1),
    ] {
        let mut e = prepared(
            &[index::POWER_ARTIFACT, index::POWER_ARTIFACT],
            &[
                index::CANDELABRA_OF_TAWNOS,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
            ],
            &[],
        );
        let candelabra = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
        for _ in 0..auras {
            cast_at(&mut e, index::POWER_ARTIFACT, vec![candelabra]);
        }
        let lands: Vec<_> = e
            .state
            .battlefield_seen()
            .filter(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.controller == P0 && o.card.is_some_and(|c| c.index == index::FOREST)
                })
            })
            .take(usize::from(x))
            .collect();
        for land in &lands {
            e.state.set_tapped(*land, true);
        }
        set_pool(&mut e, P0, &[(ManaColor::Colorless, price)]);
        activate_as(&mut e, P0, candelabra, 0);
        e.apply(P0, PlayerAction::ChooseNumber(u32::from(x)))
            .unwrap();
        if matches!(e.pending(), Pending::ChooseTargets { .. }) {
            targets(&mut e, lands.clone(), vec![]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        assert!(lands.iter().all(|id| !is_tapped(&e, *id)));
        assert!(is_tapped(&e, candelabra));
    }
}

fn activate_edge(e: &mut Engine<RegistryLookup>, discarded: ObjectId) -> ObjectId {
    let edge = on_battlefield(e, P0, index::LAND_S_EDGE).unwrap();
    activate_as(e, P0, edge, 0);
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(e, vec![], vec![P1]);
    }
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![discarded],
        },
    )
    .unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(e, vec![], vec![P1]);
    }
    *e.state.zones.list(ZoneLocation::Stack).last().unwrap()
}

#[test]
fn batch47_lands_edge_remembers_land_discarded_into_exile_by_yawgmoths_will() {
    let mut e = prepared(
        &[index::YAWGMOTH_S_WILL, index::FOREST],
        &[index::LAND_S_EDGE],
        &[],
    );
    let will = in_hand(&e, P0, index::YAWGMOTH_S_WILL).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: will }).unwrap();
    pass_until(&mut e, stack_is_empty);
    let land = in_hand(&e, P0, index::FOREST).unwrap();
    let life = e.state.players[1].life;
    activate_edge(&mut e, land);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Exile);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 2);
}

#[test]
fn batch47_lands_edge_copy_keeps_discard_lki_after_card_leaves_graveyard() {
    for card in [index::FOREST, index::GRIZZLY_BEARS] {
        let mut e = prepared(
            &[card, index::VIRTUE_OF_KNOWLEDGE],
            &[index::LAND_S_EDGE],
            &[],
        );
        let discarded = in_hand(&e, P0, card).unwrap();
        let life = e.state.players[1].life;
        let original = activate_edge(&mut e, discarded);
        e.state
            .move_object(
                discarded,
                ZoneLocation::Exile(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let visions = in_hand(&e, P0, index::VIRTUE_OF_KNOWLEDGE).unwrap();
        e.apply(P0, PlayerAction::CastSpell { card: visions })
            .unwrap();
        if let Pending::ChooseCastMode { options, .. } = e.pending().clone() {
            let adventure = options
                .iter()
                .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
                .unwrap();
            e.apply(P0, PlayerAction::ChooseMode(adventure)).unwrap();
        }
        targets(&mut e, vec![original], vec![]);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseTargets {
                    reason: crate::choice::TargetPrompt::Retarget { .. },
                    ..
                }
            )
        });
        // Empty retarget answer retains the original target player.
        targets(&mut e, vec![], vec![]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[1].life,
            life - if card == index::FOREST { 4 } else { 0 }
        );
        assert_eq!(e.state.object(discarded).unwrap().zone, Zone::Exile);
        assert_eq!(
            e.state.zones.list(ZoneLocation::Hand(P0)).len(),
            0,
            "copy pays no additional discard"
        );
    }
}

#[test]
fn batch47_power_artifact_applies_its_minimum_after_cost_increases_and_reduction() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT],
        &[index::CANDELABRA_OF_TAWNOS, index::FOREST],
        &[],
    );
    let artifact = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    cast_at(&mut e, index::POWER_ARTIFACT, vec![artifact]);
    // No registered artifact activation-tax card supplies this combination.
    // Isolate only the independent increase using the existing continuous-effect API.
    let modifier = baylee_cards_dsl::Modifier::AbilitiesCostMore(2);
    let filter = crate::effects::EffectFilter::object(&e.state, artifact);
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier,
    });
    e.state.invalidate_projections();
    e.state.set_tapped(land, true);
    set_pool(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    activate_as(&mut e, P0, artifact, 0);
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    targets(&mut e, vec![land], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, land));
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "X=1 plus2 minus2 costs1"
    );
}

#[test]
fn batch47_rhystic_priority_only_is_excluded_from_activated_x_payment_window() {
    let mut e = prepared(
        &[],
        &[
            index::RHYSTIC_CAVE,
            index::CANDELABRA_OF_TAWNOS,
            index::FOREST,
        ],
        &[],
    );
    let cave = on_battlefield(&e, P0, index::RHYSTIC_CAVE).unwrap();
    let candelabra = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    set_pool(&mut e, P0, &[]);
    activate_as(&mut e, P0, candelabra, 0);
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(&mut e, vec![forest], vec![]);
    }
    reject_cave(&mut e, cave);
    e.apply(P0, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(&mut e, vec![forest], vec![]);
    }
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, forest));
    assert!(is_tapped(&e, candelabra));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}
