#[allow(clippy::wildcard_imports)]
use super::*;

fn bolt_can_hit_you(e: &mut Engine<RegistryLookup>) {
    cast(e, P0, index::LIGHTNING_BOLT);
    targets(e, vec![], vec![P0]);
    let life = e.state.players[0].life;
    pass_until(e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 3);
}

#[test]
fn preset_ring_was_not_cast_and_graveyard_cast_was_cast() {
    let mut e = prepared(&[index::LIGHTNING_BOLT], &[index::THE_ONE_RING], &[]);
    bolt_can_hit_you(&mut e);
    let mut e = prepared(
        &[index::YAWGMOTH_S_WILL, index::LIGHTNING_BOLT],
        &[index::THE_ONE_RING],
        &[],
    );
    let ring = obj(&e, P0, index::THE_ONE_RING);
    // Transparent zone setup only; permission and Ring casting are real actions.
    e.state
        .move_object(
            ring,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    cast(&mut e, P0, index::YAWGMOTH_S_WILL);
    pass_until(&mut e, stack_is_empty);
    priority(&mut e, P0);
    mana(&mut e);
    e.apply(P0, PlayerAction::CastSpell { card: ring }).unwrap();
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
}

#[test]
fn ring_leaves_before_its_cast_etb_trigger_resolves_but_protection_still_applies() {
    let mut e = prepared(
        &[index::THE_ONE_RING, index::BOOMERANG, index::LIGHTNING_BOLT],
        &[],
        &[],
    );
    cast(&mut e, P0, index::THE_ONE_RING);
    pass_until(&mut e, |e| {
        on_battlefield(e, P0, index::THE_ONE_RING).is_some()
            && matches!(e.pending(),Pending::Priority{player,..} if *player==P0)
    });
    let ring = obj(&e, P0, index::THE_ONE_RING);
    cast_target(&mut e, index::BOOMERANG, ring);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Hand);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
}

#[test]
fn ring_activation_draw_uses_burden_lki_after_source_is_bounced() {
    let mut e = prepared(&[index::BOOMERANG], &[index::THE_ONE_RING], &[]);
    let ring = obj(&e, P0, index::THE_ONE_RING);
    activate(&mut e, index::THE_ONE_RING, 2);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P0);
    let hand = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    activate(&mut e, index::THE_ONE_RING, 2);
    cast_target(&mut e, index::BOOMERANG, ring);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Hand);
    assert_eq!(
        e.state.zones.list(ZoneLocation::Hand(P0)).len(),
        hand + 1,
        "Boomerang spent, Ring returned, one burden draw remains"
    );
}

#[test]
fn real_jin_gitaxias_spell_copy_becomes_uncast_ring_token_without_protection() {
    let mut e = prepared(
        &[index::THE_ONE_RING, index::LIGHTNING_BOLT],
        &[index::JIN_GITAXIAS_PROGRESS_TYRANT],
        &[],
    );
    let original = in_hand(&e, P0, index::THE_ONE_RING).unwrap();
    cast(&mut e, P0, index::THE_ONE_RING);
    // Jin's event-object target may require the normal trigger target answer.
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(&mut e, vec![original], vec![]);
    }
    pass_until(&mut e, |e| {
        e.state
            .battlefield_seen()
            .any(|id| e.state.object(id).is_some_and(|o| o.card.is_none()))
            && matches!(e.pending(), Pending::Priority { .. })
    });
    // The original remains below the token's entry; before it resolves there must be no protection.
    assert_eq!(e.state.object(original).unwrap().zone, Zone::Stack);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    targets(&mut e, vec![], vec![P0]);
    let life = e.state.players[0].life;
    pass_until(&mut e, |e| {
        e.state.players[0].life == life - 3 && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(e.state.players[0].life, life - 3);
}

#[test]
fn actually_cast_copy_artifact_entering_as_ring_does_qualify() {
    let mut e = prepared(
        &[index::COPY_ARTIFACT, index::LIGHTNING_BOLT],
        &[],
        &[index::THE_ONE_RING],
    );
    let ring = obj(&e, P1, index::THE_ONE_RING);
    cast(&mut e, P0, index::COPY_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    choose(&mut e, vec![ring]);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
}

#[test]
fn ring_cast_by_another_player_does_not_protect_its_new_controller() {
    let mut e = prepared(&[index::THE_ONE_RING, index::LIGHTNING_BOLT], &[], &[]);
    let ring = in_hand(&e, P0, index::THE_ONE_RING).unwrap();
    cast(&mut e, P0, index::THE_ONE_RING);
    // Transparent spell-control fixture; no registered spell-stealing helper.
    // Actual casting and entry remain actions, while caster must remain P0.
    e.state.object_mut(ring).unwrap().set_controller(P1);
    e.state.invalidate_projections();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().controller, P1);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    targets(&mut e, vec![], vec![P1]);
    let life = e.state.players[1].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 3);
}

#[test]
fn noncast_graveyard_return_does_not_grant_ring_protection() {
    let mut e = prepared(&[index::LIGHTNING_BOLT], &[index::THE_ONE_RING], &[]);
    let ring = obj(&e, P0, index::THE_ONE_RING);
    // Explicit noncast-return fixture: Goblin Welder is an unimplemented stub,
    // and this pool has no verified implemented artifact reanimation helper.
    for zone in [ZoneLocation::Graveyard(P0), ZoneLocation::Battlefield] {
        e.state
            .move_object(
                ring,
                zone,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    bolt_can_hit_you(&mut e);
}

#[test]
fn ring_upkeep_life_loss_reads_burdens_after_ring_leaves() {
    let mut e = prepared(&[index::BOOMERANG], &[index::THE_ONE_RING], &[]);
    let ring = obj(&e, P0, index::THE_ONE_RING);
    activate(&mut e, index::THE_ONE_RING, 2);
    pass_until(&mut e, stack_is_empty);
    let old = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > old
            && e.state.turn.active == P0
            && e.state.turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(),Pending::Priority{player,..} if *player==P0)
    });
    let life = e.state.players[0].life;
    cast_target(&mut e, index::BOOMERANG, ring);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Hand);
    assert_eq!(
        e.state.players[0].life,
        life - 1,
        "upkeep uses the departed Ring's one burden counter"
    );
}
