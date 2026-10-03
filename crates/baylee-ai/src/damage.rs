//! Deterministic choices over public damage offers, without simulating hidden state.

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{
    DamageEffectKind, DamageEffectOption, DamagePartView, Pending, PlayerAction,
};
use baylee_engine::event::DamageTarget;
use baylee_view::PlayerView;

fn value(view: &PlayerView, target: DamageTarget, hostile: &dyn Fn(PlayerId) -> bool) -> i64 {
    let (owner, weight) = match target {
        DamageTarget::Player(player) => (player, 100),
        DamageTarget::Object(id) => view
            .object(id)
            .map_or((view.seat, 80), |object| (object.controller, 120)),
    };
    if hostile(owner) { -weight } else { weight }
}

fn score(
    view: &PlayerView,
    effect: &DamageEffectOption,
    parts: &[DamagePartView],
    hostile: &dyn Fn(PlayerId) -> bool,
) -> i64 {
    let affected = parts.iter().filter(|part| effect.parts.contains(&part.id));
    let amount: u64 = affected.clone().map(|part| u64::from(part.amount)).sum();
    let capacity = match effect.kind {
        DamageEffectKind::PreventNext { remaining }
        | DamageEffectKind::PreventThisEvent { remaining } => u64::from(remaining).min(amount),
        DamageEffectKind::RemoveCounter { .. } => amount.min(1),
        _ => amount,
    };
    let mut left = capacity;
    let mut result = 0_i64;
    for part in affected {
        let share = u64::from(part.amount).min(left);
        left -= share;
        let weight = value(view, part.recipient, hostile);
        let benefit = match effect.kind {
            DamageEffectKind::Redirect { to } => {
                i64::from(part.amount) * (weight - value(view, to, hostile))
            }
            _ if !part.preventable => 0,
            DamageEffectKind::PreventFromSource {
                all_but, gain_life, ..
            } => {
                i64::from(part.amount.saturating_sub(all_but))
                    * (weight
                        + if gain_life {
                            if hostile(effect.controller) {
                                -100
                            } else {
                                100
                            }
                        } else {
                            0
                        })
            }
            _ => i64::try_from(share).unwrap_or(i64::MAX / 256) * weight,
        };
        result = result.saturating_add(benefit);
    }
    // Preserve finite shields when a reusable or event-only effect does the same job.
    result.saturating_mul(4).saturating_add(match effect.kind {
        DamageEffectKind::PreventCombat | DamageEffectKind::Protection => 3,
        DamageEffectKind::PreventThisEvent { .. } => 2,
        DamageEffectKind::RemoveCounter { .. } => -1,
        _ => 0,
    })
}

pub(crate) fn answer(
    view: &PlayerView,
    pending: &Pending,
    hostile: &dyn Fn(PlayerId) -> bool,
) -> Option<PlayerAction> {
    match pending {
        Pending::ChooseDamageEffect {
            choice,
            damage,
            options,
            ..
        } => options
            .iter()
            .max_by_key(|effect| {
                (
                    score(view, effect, damage, hostile),
                    std::cmp::Reverse(effect.id),
                )
            })
            .map(|effect| PlayerAction::ChooseDamageEffect {
                choice: *choice,
                effect: effect.id,
            }),
        Pending::AllocatePrevention {
            choice,
            damage,
            total,
            ..
        } => {
            let mut parts: Vec<_> = damage.iter().collect();
            parts.sort_by_key(|part| {
                (
                    std::cmp::Reverse(if part.preventable {
                        value(view, part.recipient, hostile)
                    } else {
                        0
                    }),
                    part.id,
                )
            });
            let mut remaining = *total;
            let allocation = parts
                .into_iter()
                .map(|part| {
                    let amount = part.amount.min(remaining);
                    remaining -= amount;
                    (part.id, amount)
                })
                .collect();
            (remaining == 0).then_some(PlayerAction::AllocatePrevention {
                choice: *choice,
                allocation,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::ObjectId;
    use baylee_engine::choice::{
        DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView,
    };

    #[test]
    fn uses_free_prevention_first_and_preserves_finite_shields() {
        let view = ViewBuilder::new(2).build();
        let me = view.seat;
        let part = DamagePartView {
            id: 41,
            source: ObjectId::new(4, 0),
            recipient: DamageTarget::Player(me),
            amount: 5,
            is_combat: true,
            preventable: true,
        };
        let finite = DamageEffectOption {
            id: 7,
            source: None,
            ability: None,
            controller: me,
            kind: DamageEffectKind::PreventNext { remaining: 5 },
            parts: vec![41],
        };
        let mut free = finite.clone();
        free.id = 88;
        free.kind = DamageEffectKind::PreventCombat;
        let pending = Pending::ChooseDamageEffect {
            player: me,
            choice: DamageChoiceId { batch: 3, step: 1 },
            damage: vec![part],
            options: vec![finite, free],
        };
        let action = answer(&view, &pending, &|p| p != me).unwrap();
        assert!(matches!(
            action,
            PlayerAction::ChooseDamageEffect { effect: 88, .. }
        ));
        assert_eq!(pending.answer_fault(&action), None);
    }

    #[test]
    fn allocation_fills_exact_total_with_large_parts_and_no_point_loop() {
        let view = ViewBuilder::new(2).build();
        let me = view.seat;
        let pending = Pending::AllocatePrevention {
            player: me,
            choice: DamageChoiceId { batch: 3, step: 2 },
            effect: DamageEffectOption {
                id: 7,
                source: None,
                ability: None,
                controller: me,
                kind: DamageEffectKind::PreventNext {
                    remaining: u32::MAX,
                },
                parts: vec![13, 29],
            },
            damage: vec![
                DamagePartView {
                    id: 13,
                    source: ObjectId::new(4, 0),
                    recipient: DamageTarget::Player(me),
                    amount: 3,
                    is_combat: true,
                    preventable: true,
                },
                DamagePartView {
                    id: 29,
                    source: ObjectId::new(5, 0),
                    recipient: DamageTarget::Player(me),
                    amount: u32::MAX,
                    is_combat: true,
                    preventable: true,
                },
            ],
            total: u32::MAX,
        };
        let action = answer(&view, &pending, &|p| p != me).unwrap();
        assert_eq!(pending.answer_fault(&action), None);
        assert!(
            matches!(action, PlayerAction::AllocatePrevention { allocation, .. } if allocation == vec![(13, 3), (29, u32::MAX - 3)])
        );
    }
}
