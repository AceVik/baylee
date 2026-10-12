//! Graveyard-source static acceptance; control/zone-only fixtures labelled.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{event::Cause, zone::ZonePosition};
use baylee_core::{generated::index, mana::ManaPool};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn grants(e: &Engine<RegistryLookup>, id: ObjectId) -> Vec<u32> {
    crate::effects::granted_activated(e.state(), id)
        .map(|grant| grant.id)
        .collect()
}
fn refresh(e: &mut Engine<RegistryLookup>) {
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}
fn reset(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        p.mana_pool = ManaPool::new();
    }
    e.refresh_offer();
}
fn discard(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let portal = in_hand(e, P0, index::RIFTSTONE_PORTAL).unwrap();
    activate(e, P0, index::TIRELESS_TRIBE, 0);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![portal],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
    portal
}
#[test]
fn real_discard_grants_owner_lands_and_coexists_with_intrinsic_mana() {
    let mut e = prepared(
        &[index::RIFTSTONE_PORTAL],
        &[index::TIRELESS_TRIBE, index::FOREST],
        &[index::FOREST],
    );
    let mine = obj(&e, P0, index::FOREST);
    let theirs = obj(&e, P1, index::FOREST);
    assert!(grants(&e, mine).is_empty());
    discard(&mut e);
    reset(&mut e);
    assert_eq!(grants(&e, mine).len(), 1);
    assert!(grants(&e, theirs).is_empty());
    e.apply(P0, PlayerAction::ActivateManaAbility { source: mine })
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P0
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..}if *player==P0)
    });
    reset(&mut e);
    let grant = grants(&e, mine)[0];
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: mine,
            ability_index: grant,
        },
    )
    .unwrap();
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("G/W choice")
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ManaColor::Green) && options.contains(&ManaColor::White));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
}
#[test]
fn hand_and_battlefield_portal_do_not_grant_and_printed_colorless_remains() {
    let mut e = prepared(&[index::RIFTSTONE_PORTAL], &[index::FOREST], &[]);
    let forest = obj(&e, P0, index::FOREST);
    assert!(grants(&e, forest).is_empty());
    let portal = play_land(&mut e, P0, index::RIFTSTONE_PORTAL);
    pass_until(&mut e, stack_is_empty);
    assert!(grants(&e, forest).is_empty());
    assert!(grants(&e, portal).is_empty());
    reset(&mut e);
    activate(&mut e, P0, index::RIFTSTONE_PORTAL, 0);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
}
#[test]
fn stolen_portal_sacrificed_by_controller_grants_owners_lands_in_owner_graveyard() {
    let mut e = prepared(
        &[],
        &[index::RIFTSTONE_PORTAL, index::FOREST],
        &[index::ZURAN_ORB, index::FOREST],
    );
    let portal = obj(&e, P0, index::RIFTSTONE_PORTAL);
    let mine = obj(&e, P0, index::FOREST);
    let theirs = obj(&e, P1, index::FOREST);
    // Transparent prior-control fixture: Control Magic enchants creatures and is
    // deliberately not used on a land. Sacrifice itself is the real Orb action.
    e.state.object_mut(portal).unwrap().set_controller(P1);
    refresh(&mut e);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..}if *player==P1),
    );
    activate(&mut e, P1, index::ZURAN_ORB, 0);
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![portal],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let object = e.state.object(portal).unwrap();
    assert_eq!(object.zone, Zone::Graveyard);
    assert_eq!(object.zone_owner, Some(P0));
    assert_eq!(grants(&e, mine).len(), 1);
    assert!(grants(&e, theirs).is_empty());
}
#[test]
fn actual_pit_exile_removes_grant_then_same_handle_graveyard_reentry_is_new_source() {
    let mut e = prepared(
        &[index::RIFTSTONE_PORTAL, index::PIT_OF_OFFERINGS],
        &[index::TIRELESS_TRIBE, index::FOREST],
        &[],
    );
    let forest = obj(&e, P0, index::FOREST);
    let portal = discard(&mut e);
    let old_version = e.state.object(portal).unwrap().version;
    let old_grant = grants(&e, forest)[0];
    play_land(&mut e, P0, index::PIT_OF_OFFERINGS);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![portal],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(portal).unwrap().zone, Zone::Exile);
    assert!(grants(&e, forest).is_empty());
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: forest,
                ability_index: old_grant
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    // Explicit source-incarnation fixture; no fictional land-reanimation helper.
    e.state
        .move_object(
            portal,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh(&mut e);
    assert_ne!(e.state.object(portal).unwrap().version, old_version);
    assert_eq!(grants(&e, forest).len(), 1);
    reset(&mut e);
    let new_grant = grants(&e, forest)[0];
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: forest,
            ability_index: new_grant,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
}

// Append to engine/card_tests/batch54_riftstone.rs; uses its existing helpers.
#[test]
fn graveyard_source_round_trip_without_sync_cannot_reactivate_the_old_grant() {
    let mut e = prepared(
        &[index::RIFTSTONE_PORTAL],
        &[index::TIRELESS_TRIBE, index::FOREST],
        &[],
    );
    let forest = obj(&e, P0, index::FOREST);
    let portal = discard(&mut e);
    let old_grant = grants(&e, forest)[0];
    let old_identity = e.state.source_identity(portal).unwrap();

    // Transparent incarnation/lifetime fixture: perform actual central zone
    // transitions without the engine's next static-registration sweep. No
    // claimed completed helper card performs this exact two-move instruction.
    e.state
        .move_object(
            portal,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(
        grants(&e, forest).is_empty(),
        "a graveyard grant expires at the move, before sync"
    );
    e.state
        .move_object(
            portal,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_ne!(e.state.source_identity(portal).unwrap(), old_identity);
    assert!(
        grants(&e, forest).is_empty(),
        "same zone and arena handle do not revive an old-incarnation grant"
    );

    refresh(&mut e);
    let new_grant = grants(&e, forest)[0];
    assert_ne!(
        new_grant, old_grant,
        "grant action identities must never recycle"
    );
    reset(&mut e);
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: forest,
                ability_index: old_grant,
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: forest,
            ability_index: new_grant,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
}

#[test]
fn two_actually_discarded_portals_keep_independent_grant_identities() {
    let mut e = prepared(
        &[index::RIFTSTONE_PORTAL, index::RIFTSTONE_PORTAL],
        &[index::TIRELESS_TRIBE, index::FOREST],
        &[],
    );
    let forest = obj(&e, P0, index::FOREST);
    let first = discard(&mut e);
    let first_grant = grants(&e, forest)[0];
    let second = discard(&mut e);
    assert_ne!(first, second);
    let both = grants(&e, forest);
    assert_eq!(both.len(), 2);
    assert!(both.contains(&first_grant));
    let second_grant = *both.iter().find(|id| **id != first_grant).unwrap();

    // Explicit graveyard-removal fixture isolates independent static lifetime;
    // both acquisitions above use the real Tireless Tribe discard action.
    e.state
        .move_object(
            first,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(
        grants(&e, forest),
        vec![second_grant],
        "removal is visible before sync"
    );
    refresh(&mut e);
    assert_eq!(grants(&e, forest), vec![second_grant]);
    reset(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: forest,
            ability_index: second_grant,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
    e.state
        .move_object(
            second,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(grants(&e, forest).is_empty());
    refresh(&mut e);
    assert!(grants(&e, forest).is_empty());
}
