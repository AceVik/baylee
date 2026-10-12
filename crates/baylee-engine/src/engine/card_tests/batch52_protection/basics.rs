#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn ring_draws_for_new_burden_count_and_upkeep_loses_life_not_damage() {
    let mut e = prepared(&[], &[index::THE_ONE_RING], &[]);
    let ring = obj(&e, P0, index::THE_ONE_RING);
    let hand = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    activate(&mut e, index::THE_ONE_RING, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), hand + 1);
    let life = e.state.players[0].life;
    next_main(&mut e, P0);
    assert_eq!(e.state.players[0].life, life - 1);
    activate(&mut e, index::THE_ONE_RING, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.zones.list(ZoneLocation::Hand(P0)).len(),
        hand + 4,
        "one normal draw plus two burden draws"
    );
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Battlefield);
}
#[test]
fn ring_cast_grants_protection_and_blink_does_not_regrant() {
    let mut e = prepared(&[index::THE_ONE_RING], &[], &[index::PRODIGAL_SORCERER]);
    cast(&mut e, P0, index::THE_ONE_RING);
    pass_until(&mut e, stack_is_empty);
    let sorcerer = obj(&e, P1, index::PRODIGAL_SORCERER);
    priority(&mut e, P1);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: sorcerer,
            ability_index: 0,
        },
    )
    .unwrap();
    assert!(
        e.apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    // Finish the real activation with a legal target, then expire protection.
    let ring = obj(&e, P0, index::THE_ONE_RING);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P0);
    // Explicit noncast reentry fixture isolates the cast-provenance condition.
    e.state
        .move_object(
            ring,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            ring,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    e.state.set_tapped(sorcerer, false);
    priority(&mut e, P1);
    e.refresh_offer();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: sorcerer,
            ability_index: 0,
        },
    )
    .unwrap();
    targets(&mut e, vec![], vec![P0]);
    let life = e.state.players[0].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 1);
}
#[test]
fn teferi_phases_all_permanents_preserves_incarnations_and_exiles_spell() {
    let mut e = prepared(
        &[index::TEFERI_S_PROTECTION],
        &[index::GRIZZLY_BEARS, index::SOL_RING, index::FOREST],
        &[],
    );
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    let ring = obj(&e, P0, index::SOL_RING);
    let forest = obj(&e, P0, index::FOREST);
    let version = e.state.object(bear).unwrap().version;
    let spell = in_hand(&e, P0, index::TEFERI_S_PROTECTION).unwrap();
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    for id in [bear, ring, forest] {
        assert!(
            e.state
                .object(id)
                .unwrap()
                .status
                .contains(crate::object::Status::PHASED_OUT)
        );
    }
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
    next_main(&mut e, P0);
    assert_eq!(e.state.object(bear).unwrap().version, version);
    assert!(
        !e.state
            .object(bear)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}
#[test]
fn teferi_protection_stops_untargeted_damage() {
    let mut e = prepared(&[index::TEFERI_S_PROTECTION, index::HURRICANE], &[], &[]);
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    let before = e.state.journal.last_seq();
    cast(&mut e, P0, index::HURRICANE);
    let asked = e.pending().asked().unwrap();
    e.apply(asked, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    assert!(
        !e.state
            .journal
            .entries()
            .iter()
            .any(|entry| entry.seq > before
                && matches!(
                    entry.event,
                    crate::event::GameEvent::DamageDealt {
                        target: crate::event::DamageTarget::Player(P0),
                        amount: 1..,
                        ..
                    }
                )),
        "protection prevents damage; unchanged life alone would also pass with just the life lock"
    );
    assert!(
        e.state
            .journal
            .entries()
            .iter()
            .any(|entry| entry.seq > before
                && matches!(
                    entry.event,
                    crate::event::GameEvent::DamageDealt {
                        target: crate::event::DamageTarget::Player(P1),
                        amount: 1,
                        ..
                    }
                )),
        "Hurricane still damages the unprotected opponent"
    );
}
#[test]
fn teferi_life_lock_rejects_gain_loss_and_payment_at_shared_life_door() {
    let mut e = prepared(&[index::TEFERI_S_PROTECTION], &[], &[]);
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    // Transparent central-door fixture: all printed effect was established by real spell cast.
    e.state.change_life(P0, 3, crate::event::Cause::Effect);
    assert_eq!(e.state.players[0].life, life);
    e.state.change_life(P0, -3, crate::event::Cause::Effect);
    assert_eq!(e.state.players[0].life, life);
    assert!(!e.state.can_pay_life(P0, 1));
    next_main(&mut e, P0);
    assert!(e.state.can_pay_life(P0, 1));
    e.state.change_life(P0, 3, crate::event::Cause::Effect);
    assert_eq!(e.state.players[0].life, life + 3);
}
