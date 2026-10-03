use super::*;
use baylee_engine::choice::{DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView};
use baylee_engine::event::DamageTarget;

const ME: PlayerId = PlayerId::new(0);
const CHOICE: DamageChoiceId = DamageChoiceId { batch: 8, step: 3 };
fn part(id: u32, amount: u32) -> DamagePartView {
    DamagePartView {
        id,
        source: ObjectId::new(id + 100, 0),
        recipient: DamageTarget::Player(ME),
        amount,
        is_combat: true,
        preventable: true,
    }
}
fn effect() -> DamageEffectOption {
    DamageEffectOption {
        id: 91,
        source: None,
        ability: None,
        controller: ME,
        kind: DamageEffectKind::PreventNext { remaining: 8 },
        parts: vec![7, 42],
    }
}
fn allocation() -> Pending {
    Pending::AllocatePrevention {
        player: ME,
        choice: CHOICE,
        effect: effect(),
        damage: vec![part(7, 3), part(42, 5)],
        total: 6,
    }
}
#[test]
fn prevention_draft_requires_exact_total_and_preserves_stable_ids() {
    let mut i = Interaction::new(allocation(), ME);
    assert!(!i.can_confirm());
    assert_eq!(i.set_number(100), 3);
    assert_eq!(i.allocation().unwrap().remaining(), 3);
    assert!(i.choose_index(1));
    assert_eq!(i.set_number(100), 3);
    assert!(i.can_confirm());
    let action = i.confirm().unwrap();
    assert_eq!(
        action,
        PlayerAction::AllocatePrevention {
            choice: CHOICE,
            allocation: vec![(7, 3), (42, 3)]
        }
    );
    assert_eq!(allocation().answer_fault(&action), None);
    i.set_number(2);
    assert!(!i.can_confirm());
    i.cancel();
    assert_eq!(i.allocation().unwrap().remaining(), 6);
}
#[test]
fn refreshed_offer_keeps_only_the_exact_damage_decision() {
    let mut i = Interaction::new(allocation(), ME);
    i.set_number(3);
    let kept = Interaction::new_keeping(allocation(), ME, Some(&i));
    assert_eq!(kept.number(), 3);
    let mut next = allocation();
    if let Pending::AllocatePrevention { choice, .. } = &mut next {
        choice.step += 1;
    }
    let fresh = Interaction::new_keeping(next, ME, Some(&i));
    assert_eq!(fresh.number(), 0);
    let mut changed = allocation();
    if let Pending::AllocatePrevention { damage, .. } = &mut changed {
        damage[0].amount = 2;
    }
    assert_eq!(Interaction::new_keeping(changed, ME, Some(&i)).number(), 0);
    assert!(
        Interaction::new(allocation(), PlayerId::new(1))
            .confirm()
            .is_none()
    );
}
#[test]
fn prevention_accepts_large_numeric_values_without_per_point_work() {
    let pending = Pending::AllocatePrevention {
        player: ME,
        choice: CHOICE,
        effect: effect(),
        damage: vec![part(7, u32::MAX), part(42, u32::MAX)],
        total: u32::MAX,
    };
    let mut i = Interaction::new(pending.clone(), ME);
    assert_eq!(i.set_number(u32::MAX), u32::MAX);
    i.choose_index(1);
    assert_eq!(i.set_number(u32::MAX), 0);
    assert_eq!(pending.answer_fault(&i.confirm().unwrap()), None);
}
#[test]
fn damage_effect_selection_never_confuses_display_position_with_identity() {
    let pending = Pending::ChooseDamageEffect {
        player: ME,
        choice: CHOICE,
        damage: vec![part(7, 3)],
        options: vec![effect()],
    };
    let mut i = Interaction::new(pending.clone(), ME);
    assert!(!i.can_confirm());
    assert!(!i.choose_index(91));
    assert!(i.choose_index(0));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseDamageEffect {
            choice: CHOICE,
            effect: 91
        })
    );
    let stale = PlayerAction::ChooseDamageEffect {
        choice: DamageChoiceId { batch: 8, step: 2 },
        effect: 91,
    };
    assert!(pending.answer_fault(&stale).is_some());
    assert!(baylee_engine::choice::timeout_answer(&pending).is_none());
}
#[test]
fn damage_labels_explain_sources_targets_combat_and_remaining_shield() {
    for lang in [Lang::En, Lang::De] {
        let line =
            crate::damage::effect_label(lang, &effect(), &[part(7, 3)], "Shield", &|target| {
                format!("{target:?}")
            });
        assert!(line.contains("Shield") && line.contains('8') && line.contains('3'));
        assert!(line.contains("Player") && line.contains("Object"));
        assert!(!line.contains("DamageEffectKind"));
        let line =
            Interaction::new(allocation(), ME)
                .prompt()
                .headline(lang, Turn::Mine, None, false);
        assert!(line.contains('6'));
    }
}

#[test]
fn counter_allocation_does_not_promise_to_prevent_unpreventable_damage() {
    let mut pending = allocation();
    if let Pending::AllocatePrevention { effect, damage, .. } = &mut pending {
        effect.kind = DamageEffectKind::RemoveCounter {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            remaining: 6,
        };
        damage[0].preventable = false;
    }
    let interaction = Interaction::new(pending, ME);
    assert!(
        interaction
            .prompt()
            .headline(Lang::De, Turn::Mine, None, false)
            .contains("Markenentfernungen")
    );
}

#[test]
fn source_choices_keep_exact_versions_and_reset_replaced_offers() {
    use baylee_core::ids::{DamageSourceRef, SourceChoiceId};
    let old = DamageSourceRef {
        object: ObjectId::new(8, 0),
        version: 2,
    };
    let current = DamageSourceRef { version: 4, ..old };
    let offer = |id, options| Pending::ChooseDamageSource {
        player: ME,
        choice: SourceChoiceId::new(id),
        options,
    };
    let pending = offer(10, vec![old, current]);
    let mut i = Interaction::new(pending.clone(), ME);
    assert!(!i.can_confirm());
    assert!(!i.choose_index(2));
    assert!(i.choose_index(0));
    assert!(
        matches!(i.confirm(), Some(PlayerAction::ChooseDamageSource { source, .. }) if source == old)
    );
    let kept = Interaction::new_keeping(pending.clone(), ME, Some(&i));
    assert_eq!(kept.chosen_index(), Some(0));
    for changed in [offer(11, vec![old, current]), offer(10, vec![current, old])] {
        let fresh = Interaction::new_keeping(changed, ME, Some(&i));
        assert!(!fresh.can_confirm());
        assert_eq!(fresh.chosen_index(), None);
    }
    let mut changed_seat = Interaction::new_keeping(pending.clone(), PlayerId::new(1), Some(&i));
    assert!(!changed_seat.choose_index(0));
    assert!(changed_seat.confirm().is_none());
    assert!(baylee_engine::choice::timeout_answer(&pending).is_none());
}

#[test]
fn redirection_allocates_unpreventable_damage_with_exact_ids_and_remaining_amount() {
    let mut pending = allocation();
    if let Pending::AllocatePrevention { effect, damage, .. } = &mut pending {
        effect.kind = DamageEffectKind::RedirectNext {
            remaining: 6,
            to: DamageTarget::Player(PlayerId::new(1)),
        };
        damage[0].preventable = false;
        let label =
            crate::damage::effect_label(Lang::De, effect, damage, "Personal Incarnation", &|_| {
                "Empfänger".into()
            });
        assert!(label.contains("Schaden umleiten → Empfänger; noch 6"));
    }
    let mut interaction = Interaction::new(pending.clone(), ME);
    interaction.set_number(3);
    let headline = interaction
        .prompt()
        .headline(Lang::De, Turn::Mine, None, false);
    assert!(
        headline.contains("6 Schaden umleiten — 3 übrig"),
        "{headline}"
    );
    assert!(!interaction.can_confirm());
    interaction.choose_index(1);
    interaction.set_number(3);
    let action = interaction.confirm().unwrap();
    assert_eq!(pending.answer_fault(&action), None);
    assert_eq!(
        action,
        PlayerAction::AllocatePrevention {
            choice: CHOICE,
            allocation: vec![(7, 3), (42, 3)],
        }
    );
}
