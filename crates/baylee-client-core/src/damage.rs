//! Draft prevention allocations and descriptions of pending damage.

use baylee_engine::choice::{DamageChoiceId, DamagePartView, PlayerAction};

/// A prevention draft, one bounded integer per damage part, never per point.
#[derive(Clone, Debug)]
pub struct Allocation {
    choice: DamageChoiceId,
    parts: Vec<DamagePartView>,
    total: u32,
    amounts: Vec<u32>,
    focus: usize,
}

impl Allocation {
    /// An empty draft for the exact offer.
    #[must_use]
    pub fn new(choice: DamageChoiceId, parts: &[DamagePartView], total: u32) -> Self {
        Self {
            choice,
            parts: parts.to_vec(),
            total,
            amounts: vec![0; parts.len()],
            focus: 0,
        }
    }

    /// Whether another offer can safely retain this draft.
    #[must_use]
    pub fn same_offer(&self, other: &Self) -> bool {
        self.choice == other.choice && self.parts == other.parts && self.total == other.total
    }

    /// Amounts in the engine's part order.
    #[must_use]
    pub fn amounts(&self) -> &[u32] {
        &self.amounts
    }

    /// The part whose amount the number editor changes.
    #[must_use]
    pub const fn focus(&self) -> usize {
        self.focus
    }

    /// Selects a part by its display position; identities stay in `parts`.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.parts.len() {
            return false;
        }
        self.focus = index;
        true
    }

    /// Current value of the selected part.
    #[must_use]
    pub fn number(&self) -> u32 {
        self.amounts.get(self.focus).copied().unwrap_or(0)
    }

    /// Unassigned prevention. The sum is widened before comparison.
    #[must_use]
    pub fn remaining(&self) -> u32 {
        let used: u64 = self.amounts.iter().map(|&n| u64::from(n)).sum();
        u32::try_from(u64::from(self.total).saturating_sub(used)).unwrap_or(0)
    }

    /// Changes the selected share without exceeding either its damage or the total.
    pub fn set_number(&mut self, value: u32) -> u32 {
        let Some(part) = self.parts.get(self.focus) else {
            return 0;
        };
        let max = part
            .amount
            .min(self.number().saturating_add(self.remaining()));
        self.amounts[self.focus] = value.min(max);
        self.number()
    }

    /// Clears every share while keeping the current part selected.
    pub fn clear(&mut self) {
        self.amounts.fill(0);
    }

    /// One complete action, carrying stable IDs rather than row positions.
    #[must_use]
    pub fn answer(&self) -> Option<PlayerAction> {
        (self.remaining() == 0).then(|| PlayerAction::AllocatePrevention {
            choice: self.choice,
            allocation: self
                .parts
                .iter()
                .zip(&self.amounts)
                .map(|(part, &amount)| (part.id, amount))
                .collect(),
        })
    }
}

use crate::i18n::{Lang, Phrase};
use baylee_engine::choice::{DamageEffectKind, DamageEffectOption};
use baylee_engine::event::DamageTarget;

/// Source, recipient, amount and combat status of one pending damage part.
#[must_use]
pub fn part_label(
    lang: Lang,
    part: &DamagePartView,
    name: &impl Fn(DamageTarget) -> String,
) -> String {
    let combat = if part.is_combat {
        Phrase::DamageCombat
    } else {
        Phrase::DamageNoncombat
    };
    let mut label = Phrase::DamagePart.fill(
        lang,
        &[
            &name(DamageTarget::Object(part.source)),
            &name(part.recipient),
            &part.amount.to_string(),
            combat.text(lang),
        ],
    );
    if !part.preventable {
        label.push_str(Phrase::DamageUnpreventable.text(lang));
    }
    label
}

/// The effect's mechanics, provenance and exactly which damage it modifies.
#[must_use]
pub fn effect_label(
    lang: Lang,
    effect: &DamageEffectOption,
    parts: &[DamagePartView],
    origin: &str,
    name: &impl Fn(DamageTarget) -> String,
) -> String {
    let meaning = match effect.kind {
        DamageEffectKind::ReplaceAmount { amount } => {
            Phrase::DamageReplaceAmount.fill(lang, &[&amount.to_string()])
        }
        // `name` resolves a live handle, not an exact historical incarnation.
        // The origin already identifies the effect; do not relabel its source
        // using a later object that might occupy the same handle.
        DamageEffectKind::Reflect { to, .. } => {
            Phrase::DamageReflect.fill(lang, &[&name(DamageTarget::Player(to))])
        }
        DamageEffectKind::RemoveShieldCounter => Phrase::DamageShieldCounter.text(lang).to_string(),
        DamageEffectKind::PreventNext { remaining } => {
            Phrase::DamageShield.fill(lang, &[&remaining.to_string()])
        }
        DamageEffectKind::PreventThisEvent { remaining } => {
            Phrase::DamageEventShield.fill(lang, &[&remaining.to_string()])
        }
        DamageEffectKind::TurnFaceUp { .. } => Phrase::DamageTurnFaceUp.text(lang).to_string(),
        DamageEffectKind::PreventCombat => Phrase::DamagePreventCombat.text(lang).to_string(),
        DamageEffectKind::PreventAll => Phrase::DamagePreventAll.text(lang).to_string(),
        DamageEffectKind::Protection => Phrase::DamageProtection.text(lang).to_string(),
        DamageEffectKind::Redirect { to } => Phrase::DamageRedirect.fill(lang, &[&name(to)]),
        DamageEffectKind::RedirectNext { remaining, to } => {
            Phrase::DamageRedirectNext.fill(lang, &[&name(to), &remaining.to_string()])
        }
        DamageEffectKind::RemoveCounter { kind, remaining } => Phrase::DamageCounter.fill(
            lang,
            &[
                &crate::interaction::counter_label(kind, lang),
                &remaining.to_string(),
            ],
        ),
        DamageEffectKind::PreventFromSource {
            source,
            all_but,
            gain_life,
        } => {
            let mut line = Phrase::DamageSourceShield.fill(
                lang,
                &[&name(DamageTarget::Object(source)), &all_but.to_string()],
            );
            if gain_life {
                line.push_str(
                    &Phrase::DamageShieldLife
                        .fill(lang, &[&name(DamageTarget::Player(effect.controller))]),
                );
            }
            line
        }
    };
    let affected = parts
        .iter()
        .filter(|part| effect.parts.contains(&part.id))
        .map(|part| part_label(lang, part, name))
        .collect::<Vec<_>>()
        .join("; ");
    format!("{origin}: {meaning}\n{affected}")
}

/// Exact allocation requirement and the amount still unassigned.
#[must_use]
pub fn allocation_headline(
    lang: Lang,
    effect: &DamageEffectOption,
    total: u32,
    amounts: &[u32],
) -> String {
    let used: u64 = amounts.iter().map(|&n| u64::from(n)).sum();
    let phrase = match effect.kind {
        DamageEffectKind::RemoveCounter { .. } => Phrase::AllocateDamageCounters,
        DamageEffectKind::RedirectNext { .. } => Phrase::AllocateRedirection,
        _ => Phrase::AllocatePrevention,
    };
    phrase.fill(
        lang,
        &[
            &total.to_string(),
            &u64::from(total).saturating_sub(used).to_string(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::{DamageEffectKind, DamageEffectOption, DamageTarget, Lang, effect_label};
    use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

    #[test]
    fn replacement_reflection_and_shield_choices_name_their_actual_consequences() {
        let source = ObjectId::new(9, 0);
        let player = PlayerId::new(0);
        for (kind, english, german) in [
            (
                DamageEffectKind::ReplaceAmount { amount: 4 },
                "Set each indicated source's damage to each recipient to 4; this is not prevention",
                "Setze den Schaden jeder angegebenen Quelle an jedem Empfänger auf 4",
            ),
            (
                DamageEffectKind::Reflect {
                    source: DamageSourceRef {
                        object: source,
                        version: 1,
                    },
                    to: player,
                },
                "This effect's source also deals the indicated damage to player",
                "Die Quelle dieses Effekts fügt player zusätzlich den angegebenen Schaden zu",
            ),
            (
                DamageEffectKind::RemoveShieldCounter,
                "Remove one shield counter and prevent this event's damage",
                "Entferne eine Schildmarke und verhindere den Schaden dieses Ereignisses",
            ),
        ] {
            let option = DamageEffectOption {
                id: 1,
                source: Some(source),
                ability: None,
                controller: player,
                kind,
                parts: Vec::new(),
            };
            let name = |target| match target {
                DamageTarget::Object(_) => "source".to_owned(),
                DamageTarget::Player(_) => "player".to_owned(),
            };
            assert!(effect_label(Lang::En, &option, &[], "origin", &name).contains(english));
            assert!(effect_label(Lang::De, &option, &[], "origin", &name).contains(german));
        }
    }
}
