//! Amount replacements, reflection, and intrinsic shield counters.

use super::{
    Candidate, DamageCause, DamageEffectKind, DamageTarget, DamageWork, EffectKey, GameState,
};
use baylee_cards_dsl::CounterKind;

impl DamageWork {
    pub(crate) fn with_cause(mut self, cause: &DamageCause) -> Self {
        for part in &mut self.parts {
            part.cause = Some(cause.clone());
        }
        self
    }

    pub(super) fn apply_batch57(&mut self, state: &mut GameState, candidate: &Candidate) -> bool {
        match candidate.option.kind {
            DamageEffectKind::ReplaceAmount { amount } => {
                let mut replaced = Vec::new();
                for part in &mut self.parts {
                    if candidate.option.parts.contains(&part.view.id) {
                        let source = (part.view.source, part.source_version);
                        part.view.amount = if replaced.contains(&source) {
                            0
                        } else {
                            replaced.push(source);
                            amount
                        };
                        part.applied.push(candidate.key);
                    }
                }
            }
            DamageEffectKind::Reflect { source, to } => {
                let mut reflected = Vec::new();
                let first = self.parts.len();
                for part in &mut self.parts {
                    if !candidate.option.parts.contains(&part.view.id) {
                        continue;
                    }
                    part.applied.push(candidate.key);
                    let mut extra = part.clone();
                    extra.view.id =
                        u32::try_from(first + reflected.len()).expect("damage part count");
                    extra.view.source = source.object;
                    extra.source_version = Some(source.version);
                    extra.view.recipient = DamageTarget::Player(to);
                    extra.recipient_version = None;
                    extra.view.is_combat = false;
                    extra.cause = Some(DamageCause {
                        object: source,
                        targets: Vec::new(),
                    });
                    extra.keywords = baylee_cards_dsl::KeywordSet::default();
                    extra.controller = candidate.option.controller;
                    if let Some(object) = state.source_object(source) {
                        extra.controller = object.controller;
                        extra.keywords = object.characteristics().keywords;
                    }
                    // The additional damage is a new event within the same simultaneous
                    // result; earlier replacements on the original damage do not apply to it.
                    extra.applied.clear();
                    extra.applied.push(candidate.key);
                    extra.counters_applied.clear();
                    reflected.push(extra);
                }
                self.parts.extend(reflected);
                if let EffectKey::Shield(id) = candidate.key
                    && let Some(index) = state.shields.position(id)
                {
                    state.shields.remove(index);
                }
            }
            DamageEffectKind::RemoveShieldCounter => {
                let EffectKey::ShieldCounter(reference) = candidate.key else {
                    unreachable!("shield counter")
                };
                self.counter_removals.push((
                    reference.object,
                    reference.version,
                    CounterKind::Shield,
                    1,
                ));
                for part in &mut self.parts {
                    if candidate.option.parts.contains(&part.view.id) {
                        part.applied.push(candidate.key);
                        if part.view.preventable {
                            part.view.amount = 0;
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
}

impl DamageWork {
    pub(super) fn batch57_candidates(
        &self,
        state: &GameState,
        part: &super::Part,
        out: &mut Vec<Candidate>,
    ) {
        use baylee_cards_dsl::{DamageAmountReplacement, Modifier};
        if let DamageTarget::Object(id) = part.view.recipient
            && self.counter_remaining(state, id, CounterKind::Shield) > 0
            && let Some(reference) = state.source_identity(id)
        {
            super::candidates::add(
                out,
                part,
                EffectKey::ShieldCounter(reference),
                DamageEffectKind::RemoveShieldCounter,
                None,
                state.object(id).expect("recipient").controller,
            );
        }
        let DamageTarget::Player(player) = part.view.recipient else {
            return;
        };
        let Some(source) = state.damage_source(part.view.source, part.source_version) else {
            return;
        };
        for effect in state.effects.iter() {
            let Modifier::ReplacePlayerDamage {
                sources,
                recipients,
                combat,
                replacement,
            } = effect.modifier
            else {
                continue;
            };
            if combat.is_some_and(|combat| combat != part.view.is_combat)
                || !crate::eval::players(recipients, state, effect.controller)
                    .is_some_and(|players| players.contains(&player))
                || !crate::eval::matches_projected_with_context(
                    sources,
                    state,
                    source,
                    source.characteristics(),
                    effect.controller,
                    crate::text_changes::RuleContext {
                        source: effect.source.unwrap_or(source.id),
                        text: state.effect_text(effect),
                        resolved_name: None,
                        reference: None,
                    },
                )
            {
                continue;
            }
            let total = self
                .parts
                .iter()
                .filter(|other| {
                    other.view.source == part.view.source
                        && other.source_version == part.source_version
                        && other.view.recipient == part.view.recipient
                        && !other.applied.contains(&EffectKey::Continuous(effect.id))
                })
                .map(|part| part.view.amount)
                .fold(0, u32::saturating_add);
            let amount = match replacement {
                DamageAmountReplacement::Cap(cap) if total > cap => cap,
                DamageAmountReplacement::FloorAtEffectSourcePower => {
                    let power = effect
                        .source
                        .and_then(|id| state.object(id))
                        .and_then(|o| o.characteristics().power)
                        .and_then(|power| u32::try_from(power).ok())
                        .unwrap_or(0);
                    if total >= power {
                        continue;
                    }
                    power
                }
                DamageAmountReplacement::Cap(_) => continue,
            };
            super::candidates::add(
                out,
                part,
                EffectKey::Continuous(effect.id),
                DamageEffectKind::ReplaceAmount { amount },
                effect.source.map(|source| crate::prevention::ShieldOrigin {
                    source,
                    ability: None,
                }),
                effect.controller,
            );
        }
    }
}

pub(super) fn shield_kind(
    state: &GameState,
    part: &super::Part,
    shield: crate::prevention::ShieldKind,
) -> Option<DamageEffectKind> {
    use crate::prevention::{RedirectDestination, ShieldKind, damageable_recipient};
    match shield {
        ShieldKind::RedirectSource { source, to, .. } => {
            let actual = state.damage_source(part.view.source, part.source_version)?;
            if !source.deals_as(state, actual) {
                return None;
            }
            let to = match to {
                RedirectDestination::Object(reference) => {
                    let object = state.object(reference.object)?;
                    if object.version != reference.version
                        || object.status.contains(crate::object::Status::PHASED_OUT)
                    {
                        return None;
                    }
                    DamageTarget::Object(reference.object)
                }
                RedirectDestination::SourceController => DamageTarget::Player(actual.controller),
            };
            (to != part.view.recipient && damageable_recipient(state, to))
                .then_some(DamageEffectKind::Redirect { to })
        }
        ShieldKind::ReflectNextSource { source, reflection } => {
            let actual = state.damage_source(part.view.source, part.source_version)?;
            source
                .deals_as(state, actual)
                .then_some(DamageEffectKind::Reflect {
                    source: reflection,
                    to: actual.controller,
                })
        }
        ShieldKind::TargetingCauses => {
            let DamageTarget::Object(id) = part.view.recipient else {
                return None;
            };
            let reference = state.source_identity(id)?;
            part.cause
                .as_ref()
                .is_some_and(|cause| cause.targets.contains(&reference))
                .then_some(DamageEffectKind::PreventAll)
        }
        _ => None,
    }
}
