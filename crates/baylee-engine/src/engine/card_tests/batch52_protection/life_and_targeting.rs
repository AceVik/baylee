#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn ring_protection_does_not_prevent_untargeted_life_loss() {
    let mut e = prepared(&[index::THE_ONE_RING, index::MAGGOT_CARRIER], &[], &[]);
    cast(&mut e, P0, index::THE_ONE_RING);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    cast(&mut e, P0, index::MAGGOT_CARRIER);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 1);
}

#[test]
fn protection_blocks_own_spell_target_as_well_as_opponents() {
    let mut e = prepared(&[index::THE_ONE_RING, index::LIGHTNING_BOLT], &[], &[]);
    cast(&mut e, P0, index::THE_ONE_RING);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, P0, index::LIGHTNING_BOLT);
    let before = e.state.snapshot_hash();
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
    assert_eq!(e.state.snapshot_hash(), before);
    targets(&mut e, vec![], vec![P1]);
    let life = e.state.players[1].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 3);
}

#[test]
fn teferi_blocks_actual_nondamage_loss_and_actual_gain_but_not_sacrifice_cost() {
    let mut e = prepared(
        &[
            index::TEFERI_S_PROTECTION,
            index::MAGGOT_CARRIER,
            index::ZURAN_ORB,
            index::FOREST,
        ],
        &[],
        &[],
    );
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    cast(&mut e, P0, index::MAGGOT_CARRIER);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    cast(&mut e, P0, index::ZURAN_ORB);
    pass_until(&mut e, stack_is_empty);
    let land = play_land(&mut e, P0, index::FOREST);
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state
            .object(land)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT),
        "later entries are not covered by the one-shot phase-out"
    );
    activate(&mut e, index::ZURAN_ORB, 0);
    choose(&mut e, vec![land]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state.players[0].life, life,
        "gain is prohibited, but sacrificing a land is still payable"
    );
}

#[test]
fn teferi_forbids_positive_life_activation_cost_without_spending_mana() {
    let mut e = prepared(&[index::TEFERI_S_PROTECTION, index::SEWER_RATS], &[], &[]);
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, P0, index::SEWER_RATS);
    pass_until(&mut e, stack_is_empty);
    priority(&mut e, P0);
    mana(&mut e);
    let rats = obj(&e, P0, index::SEWER_RATS);
    let before = e.state.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: rats,
                ability_index: 0
            }
        )
        .is_err()
    );
    assert_eq!(
        e.state.snapshot_hash(),
        before,
        "rejected cost pays neither mana nor life"
    );
    assert!(e.state.can_pay_life(P0, 0));
}

#[test]
fn life_lock_returns_before_arithmetic_and_does_not_record_life_loss() {
    let mut e = prepared(&[index::TEFERI_S_PROTECTION], &[], &[]);
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.snapshot_hash();
    // Explicit central-door boundary fixture after actual spell resolution.
    e.state
        .change_life(P0, i32::MAX, crate::event::Cause::Effect);
    e.state
        .change_life(P0, i32::MIN, crate::event::Cause::Effect);
    assert_eq!(
        e.state.snapshot_hash(),
        before,
        "no arithmetic, loss history, or journal entry is committed"
    );
}

#[test]
fn unpreventable_combat_damage_distinguishes_ring_protection_from_teferi_life_lock() {
    for protection in [index::THE_ONE_RING, index::TEFERI_S_PROTECTION] {
        let mut e = prepared(&[protection], &[], &[index::GRIZZLY_BEARS]);
        let bear = obj(&e, P1, index::GRIZZLY_BEARS);
        cast(&mut e, P0, protection);
        pass_until(&mut e, stack_is_empty);
        // Transparent independent unpreventability fixture; the protection comes from the actual card.
        let modifier = baylee_cards_dsl::Modifier::CombatDamageCantBePrevented;
        let filter = crate::effects::EffectFilter::object(&e.state, bear);
        let timestamp = e.state.next_timestamp();
        e.state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: P1,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier,
        });
        e.state.invalidate_projections();
        e.refresh_offer();
        pass_until(
            &mut e,
            |e| matches!(e.pending(),Pending::ChooseAttackers{player,..} if *player==P1),
        );
        e.apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(P0))],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
            .unwrap();
        let life = e.state.players[0].life;
        let before = e.state.journal.last_seq();
        pass_until(&mut e, |e| {
            e.state.journal.entries().iter().any(|entry| {
                entry.seq > before
                    && matches!(
                        entry.event,
                        crate::event::GameEvent::DamageDealt {
                            target: crate::event::DamageTarget::Player(P0),
                            amount: 2,
                            ..
                        }
                    )
            })
        });
        assert_eq!(
            e.state.players[0].life,
            if protection == index::THE_ONE_RING {
                life - 2
            } else {
                life
            }
        );
    }
}
