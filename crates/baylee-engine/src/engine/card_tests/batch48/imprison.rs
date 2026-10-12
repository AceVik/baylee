//! Exact event identity and optional payments for Imprison.
#[allow(clippy::wildcard_imports)]
use super::*;

// Combat phase changes correctly empty prepared()'s initial floating mana.
// This existing-harness mana fixture funds the real payment continuation; it
// does not bypass that payment or assume accepting an unaffordable cost pays it.
fn imprison_payment(e: &mut Engine<RegistryLookup>, accept: bool) {
    if accept {
        let payer = e.pending().asked().expect("Imprison payment payer");
        e.state.players[usize::from(payer.get())]
            .mana_pool
            .add(ManaColor::Colorless, 1);
    }
    yes(e, accept);
}

#[test]
fn batch48_imprison_counters_exact_triggering_tap_ability_or_destroys_aura() {
    for accept in [false, true] {
        let mut e = prepared(&[index::IMPRISON], &[], &[index::PRODIGAL_SORCERER]);
        let sorcerer = source(&e, P1, index::PRODIGAL_SORCERER);
        cast(&mut e, index::IMPRISON);
        targets(&mut e, vec![sorcerer], vec![]);
        pass_until(&mut e, stack_is_empty);
        let before = e.state.players[0].life;
        ability(&mut e, P1, index::PRODIGAL_SORCERER, 0);
        targets(&mut e, vec![], vec![P0]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        imprison_payment(&mut e, accept);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].life, before - i32::from(!accept));
        assert_eq!(on_battlefield(&e, P0, index::IMPRISON).is_some(), accept);
        assert!(is_tapped(&e, sorcerer));
    }
}

#[test]
fn batch48_imprison_paid_attack_trigger_removes_creature_from_combat() {
    let mut e = prepared(&[index::IMPRISON], &[index::GRIZZLY_BEARS], &[]);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    attackers(&mut e, &[bear]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    imprison_payment(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert!(!e.state.combat.is_attacking(bear));
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(e.state.players[1].life, before);
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_some());
}

#[test]
fn imprison_ignores_mana_abilities_and_abilities_without_a_tap_symbol() {
    for card in [index::LLANOWAR_ELVES, index::SEWER_RATS] {
        let mut e = prepared(&[index::IMPRISON], &[], &[card]);
        let creature = source(&e, P1, card);
        cast(&mut e, index::IMPRISON);
        targets(&mut e, vec![creature], vec![]);
        pass_until(&mut e, stack_is_empty);
        let aura = source(&e, P0, index::IMPRISON);
        ability(&mut e, P1, card, 0);
        // A stray Imprison payment prompt fails pass_until rather than being
        // silently accepted by a generic answer helper.
        pass_until(&mut e, stack_is_empty);
        assert_eq!(source(&e, P0, index::IMPRISON), aura);
        if card == index::LLANOWAR_ELVES {
            assert!(is_tapped(&e, creature));
        } else {
            assert_eq!(
                e.state.object(creature).unwrap().characteristics().power,
                Some(2)
            );
        }
    }
}

#[test]
fn imprison_does_not_look_back_for_an_activation_completed_after_sacrificing_its_creature() {
    let mut e = prepared(
        &[index::IMPRISON],
        &[index::SOL_RING],
        &[index::SCAVENGER_FOLK],
    );
    let folk = source(&e, P1, index::SCAVENGER_FOLK);
    let ring = source(&e, P0, index::SOL_RING);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![folk], vec![]);
    pass_until(&mut e, stack_is_empty);
    ability(&mut e, P1, index::SCAVENGER_FOLK, 0);
    targets(&mut e, vec![ring], vec![]);
    // The ability becomes activated only after its tap and sacrifice costs.
    // Imprison's activation observer has no leaves-the-battlefield exception.
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::SOL_RING).is_none());
    assert!(on_battlefield(&e, P1, index::SCAVENGER_FOLK).is_none());
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_none());
}

#[test]
fn imprison_declined_attack_payment_leaves_attacker_in_combat() {
    let mut e = prepared(&[index::IMPRISON], &[index::GRIZZLY_BEARS], &[]);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    attackers(&mut e, &[bear]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(e.pending().asked(), Some(P0));
    imprison_payment(&mut e, false);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(e.state.players[1].life, before - 2);
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_none());
}

#[test]
fn imprison_only_unblocks_attackers_that_had_exactly_one_blocker_this_combat() {
    for multiple in [false, true] {
        let mut e = prepared(
            &[index::IMPRISON],
            &[index::HILL_GIANT],
            &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        );
        let attacker = source(&e, P0, index::HILL_GIANT);
        let enchanted = source(&e, P1, index::GRIZZLY_BEARS);
        let other = source(&e, P1, index::SAVANNAH_LIONS);
        cast(&mut e, index::IMPRISON);
        targets(&mut e, vec![enchanted], vec![]);
        pass_until(&mut e, stack_is_empty);
        let before = e.state.players[1].life;
        attackers(&mut e, &[attacker]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        let blockers = if multiple {
            vec![(enchanted, attacker), (other, attacker)]
        } else {
            vec![(enchanted, attacker)]
        };
        e.apply(P1, PlayerAction::DeclareBlockers { blockers })
            .unwrap();
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(e.pending().asked(), Some(P0), "Aura controller pays");
        imprison_payment(&mut e, true);
        pass_until(&mut e, |e| {
            e.state.turn.step == crate::turn::Step::CombatEnd
        });
        assert_eq!(
            e.state.players[1].life,
            before - if multiple { 0 } else { 3 }
        );
        assert!(is_tapped(&e, enchanted));
        assert!(on_battlefield(&e, P0, index::IMPRISON).is_some());
    }
}

#[test]
fn imprison_does_not_reclassify_a_formerly_multiple_blocked_attacker_as_sole_blocked() {
    let mut e = prepared(
        &[index::IMPRISON, index::UNSUMMON],
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
    );
    let attacker = source(&e, P0, index::HILL_GIANT);
    let enchanted = source(&e, P1, index::GRIZZLY_BEARS);
    let other = source(&e, P1, index::SAVANNAH_LIONS);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![enchanted], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    attackers(&mut e, &[attacker]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(enchanted, attacker), (other, attacker)],
        },
    )
    .unwrap();
    // Actual response removes the other blocker before Imprison resolves.
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![other], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    imprison_payment(&mut e, true);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(e.state.players[1].life, before);
    assert!(is_tapped(&e, enchanted));
}

#[test]
fn imprison_counters_the_original_activation_without_countering_its_copy() {
    let mut e = prepared(
        &[index::IMPRISON, index::VIRTUE_OF_KNOWLEDGE],
        &[index::PRODIGAL_SORCERER],
        &[],
    );
    let sorcerer = source(&e, P0, index::PRODIGAL_SORCERER);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![sorcerer], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[0].life;
    ability(&mut e, P0, index::PRODIGAL_SORCERER, 0);
    targets(&mut e, vec![], vec![P0]);
    let original = e
        .state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.ability.is_some_and(|loc| loc.source == sorcerer))
        })
        .expect("actual original activated ability");
    cast(&mut e, index::VIRTUE_OF_KNOWLEDGE);
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let mode = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
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
    targets(&mut e, vec![], vec![]); // Keep the copied ability's original target.
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    imprison_payment(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life,
        before - 1,
        "copy resolves; original is countered"
    );
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_some());
}

#[test]
fn imprison_ignores_intrinsic_mana_activation_and_resulting_triggered_mana_ability() {
    let mut e = prepared(
        &[index::IMPRISON],
        &[index::MANA_FLARE],
        &[index::DRYAD_ARBOR],
    );
    let arbor = source(&e, P1, index::DRYAD_ARBOR);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![arbor], vec![]);
    pass_until(&mut e, stack_is_empty);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..}if *player==P1),
    );
    let before = e.state.players[1].mana_pool.available(ManaColor::Green);
    let start = e.state.journal.len();
    e.apply(P1, PlayerAction::ActivateManaAbility { source: arbor })
        .unwrap();
    // Mana Flare is a real triggered mana ability, not another activated ability.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[1].mana_pool.available(ManaColor::Green),
        before + 2
    );
    assert!(is_tapped(&e, arbor));
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_some());
    assert_eq!(
        e.state.journal.entries()[start..]
            .iter()
            .filter(|entry| matches!(
                entry.event,
                crate::event::GameEvent::AbilityActivated { .. }
            ))
            .count(),
        1
    );
    assert!(
        !matches!(e.pending(), Pending::YesNo { .. }),
        "neither mana activation nor triggered mana production requests Imprison payment"
    );
}

mod edges;
