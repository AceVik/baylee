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

    #[allow(clippy::too_many_lines)] // Flat dispatch over damage-replacement candidates.
    pub(super) fn apply_batch57(&mut self, state: &mut GameState, candidate: &Candidate) -> bool {
        match candidate.option.kind {
            DamageEffectKind::ReplaceAmount { amount } => {
                let mut replaced = Vec::new();
                let mut replaced_total = 0_u32;
                let first_event_source = candidate
                    .option
                    .parts
                    .first()
                    .and_then(|pid| self.parts.iter().find(|p| p.view.id == *pid))
                    .map(|p| p.view.source);
                for part in &mut self.parts {
                    if candidate.option.parts.contains(&part.view.id) {
                        let source = (part.view.source, part.source_version);
                        let before = part.view.amount;
                        part.view.amount = if replaced.contains(&source) {
                            0
                        } else {
                            replaced.push(source);
                            amount
                        };
                        replaced_total =
                            replaced_total.saturating_add(before.saturating_sub(part.view.amount));
                        part.applied.push(candidate.key);
                    }
                }
                if replaced_total > 0
                    && let EffectKey::Continuous(fx_id) = candidate.key
                    && let Some(fx) = state.effects.iter().find(|fx| fx.id == fx_id)
                    && let baylee_cards_dsl::Modifier::ReplaceOrPreventDamageWithThen {
                        is_replacement: true,
                        then,
                        ..
                    } = fx.modifier
                {
                    self.record_then_rider(super::PreventThenRider {
                        key: EffectKey::Continuous(fx_id),
                        controller: fx.controller,
                        source: fx.source.unwrap_or(baylee_core::ids::ObjectId::NO_SOURCE),
                        source_version: fx
                            .source
                            .and_then(|id| state.object(id).map(|o| o.version)),
                        event_object: first_event_source,
                        prevented: replaced_total,
                        text: state.effect_text(fx),
                        chosen_subtype: fx.filter.chosen_subtype().or_else(|| {
                            fx.source
                                .and_then(|id| state.object(id))
                                .and_then(|o| o.chosen_subtype)
                        }),
                        chosen_color: fx
                            .source
                            .and_then(|id| state.object(id))
                            .and_then(|o| o.chosen_color),
                        then,
                    });
                }
            }
            DamageEffectKind::MultiplyDamage { factor } => {
                for part in &mut self.parts {
                    if candidate.option.parts.contains(&part.view.id) {
                        if part.view.amount > 0 {
                            part.view.amount = part.view.amount.saturating_mul(u32::from(factor));
                        }
                        part.applied.push(candidate.key);
                    }
                }
            }
            DamageEffectKind::AddDamage { extra } => {
                let mut increased = Vec::new();
                for part in &mut self.parts {
                    if candidate.option.parts.contains(&part.view.id) {
                        let key = (part.view.source, part.source_version, part.view.recipient);
                        if part.view.amount > 0 && !increased.contains(&key) {
                            increased.push(key);
                            part.view.amount = part.view.amount.saturating_add(u32::from(extra));
                        }
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
        use baylee_cards_dsl::Modifier;
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
        let Some(source) = state.damage_source(part.view.source, part.source_version) else {
            return;
        };
        for effect in state.effects.iter() {
            match effect.modifier {
                Modifier::ReplacePlayerDamage {
                    sources,
                    recipients,
                    combat,
                    replacement,
                } => {
                    self.push_amount_replacement_candidate(
                        effect,
                        sources,
                        baylee_cards_dsl::DamageRecipients::Players(recipients),
                        combat,
                        replacement,
                        state,
                        part,
                        source,
                        out,
                    );
                }
                Modifier::ReplaceDamage {
                    sources,
                    recipients,
                    combat,
                    replacement,
                } => {
                    self.push_amount_replacement_candidate(
                        effect,
                        sources,
                        recipients,
                        combat,
                        replacement,
                        state,
                        part,
                        source,
                        out,
                    );
                }
                Modifier::MultiplyDamageFrom { .. } | Modifier::AddDamageFrom { .. } => {
                    push_scaled_damage_candidate(effect, state, part, source, out);
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_amount_replacement_candidate(
        &self,
        effect: &crate::effects::ContinuousEffect,
        sources: &'static baylee_cards_dsl::Filter,
        recipients: baylee_cards_dsl::DamageRecipients,
        combat: Option<bool>,
        replacement: baylee_cards_dsl::DamageAmountReplacement,
        state: &GameState,
        part: &super::Part,
        source: &crate::object::GameObject,
        out: &mut Vec<Candidate>,
    ) {
        use baylee_cards_dsl::DamageAmountReplacement;
        if combat.is_some_and(|combat| combat != part.view.is_combat)
            || !damage_recipients_match(effect, recipients, state, part.view.recipient)
            || !damage_source_matches_effect(effect, sources, state, source)
        {
            return;
        }
        let key = match replacement {
            DamageAmountReplacement::HalfDown | DamageAmountReplacement::HalfUp => {
                EffectKey::ContinuousForSource(
                    effect.id,
                    baylee_core::ids::DamageSourceRef {
                        object: part.view.source,
                        version: part.source_version.unwrap_or(source.version),
                    },
                )
            }
            DamageAmountReplacement::Cap(_) | DamageAmountReplacement::FloorAtEffectSourcePower => {
                EffectKey::Continuous(effect.id)
            }
        };
        let total = self
            .parts
            .iter()
            .filter(|other| {
                other.view.source == part.view.source
                    && other.source_version == part.source_version
                    && other.view.recipient == part.view.recipient
                    && !other.applied.contains(&key)
            })
            .map(|part| part.view.amount)
            .fold(0, u32::saturating_add);
        if total == 0 {
            return;
        }
        let amount = match replacement {
            DamageAmountReplacement::Cap(cap) if total > cap => cap,
            DamageAmountReplacement::Cap(_) => return,
            DamageAmountReplacement::FloorAtEffectSourcePower => {
                let power = effect
                    .source
                    .and_then(|id| state.object(id))
                    .and_then(|o| o.characteristics().power)
                    .and_then(|power| u32::try_from(power).ok())
                    .unwrap_or(0);
                if total >= power {
                    return;
                }
                power
            }
            DamageAmountReplacement::HalfDown => total / 2,
            DamageAmountReplacement::HalfUp => total.div_ceil(2),
        };
        super::candidates::add(
            out,
            part,
            key,
            DamageEffectKind::ReplaceAmount { amount },
            effect.source.map(|source| crate::prevention::ShieldOrigin {
                source,
                ability: None,
            }),
            effect.controller,
        );
    }
}

fn push_scaled_damage_candidate(
    effect: &crate::effects::ContinuousEffect,
    state: &GameState,
    part: &super::Part,
    source: &crate::object::GameObject,
    out: &mut Vec<Candidate>,
) {
    use baylee_cards_dsl::Modifier;
    if part.view.amount == 0 {
        return;
    }
    let (sources, combat_only, noncombat_only, recipients, kind) = match effect.modifier {
        Modifier::MultiplyDamageFrom {
            sources,
            factor,
            combat_only,
            noncombat_only,
            recipients,
        } if factor > 1 => (
            sources,
            combat_only,
            noncombat_only,
            recipients,
            DamageEffectKind::MultiplyDamage { factor },
        ),
        Modifier::AddDamageFrom {
            sources,
            extra,
            combat_only,
            noncombat_only,
            recipients,
        } if extra > 0 => (
            sources,
            combat_only,
            noncombat_only,
            recipients,
            DamageEffectKind::AddDamage { extra },
        ),
        _ => return,
    };
    if (combat_only && !part.view.is_combat)
        || (noncombat_only && part.view.is_combat)
        || !damage_recipients_match(effect, recipients, state, part.view.recipient)
        || !damage_source_matches_effect(effect, sources, state, source)
    {
        return;
    }
    super::candidates::add(
        out,
        part,
        EffectKey::Continuous(effect.id),
        kind,
        effect.source.map(|source| crate::prevention::ShieldOrigin {
            source,
            ability: None,
        }),
        effect.controller,
    );
}

pub(super) fn damage_source_matches_effect(
    effect: &crate::effects::ContinuousEffect,
    sources: &'static baylee_cards_dsl::Filter,
    state: &GameState,
    source: &crate::object::GameObject,
) -> bool {
    let fx_source = effect.source.unwrap_or(source.id);
    let fx_obj = state.object(fx_source);
    let chosen_color = fx_obj.and_then(|o| o.chosen_color);
    let chosen_subtype = fx_obj.and_then(|o| o.chosen_subtype);
    crate::eval::matches_damage_source_with_choices(
        sources,
        state,
        source,
        effect.controller,
        crate::text_changes::RuleContext {
            source: fx_source,
            text: state.effect_text(effect),
            resolved_name: None,
            resolved_subtype: chosen_subtype,
            reference: None,
        },
        chosen_color,
        chosen_subtype,
    )
}

pub(super) fn damage_recipients_match(
    effect: &crate::effects::ContinuousEffect,
    recipients: baylee_cards_dsl::DamageRecipients,
    state: &GameState,
    recipient: DamageTarget,
) -> bool {
    use baylee_cards_dsl::DamageRecipients;
    match recipients {
        DamageRecipients::Any => true,
        DamageRecipients::Players(who) => match recipient {
            DamageTarget::Player(player) => {
                super::candidates::effect_affects_player(effect, who, state, player)
            }
            DamageTarget::Object(_) => false,
        },
        DamageRecipients::Permanents(filter) => match recipient {
            DamageTarget::Object(target) => state.object(target).is_some_and(|object| {
                if matches!(effect.filter, crate::effects::EffectFilter::ObjectIs(..))
                    && !crate::effects::applies_to(state, effect, object)
                {
                    return false;
                }
                if matches!(effect.filter, crate::effects::EffectFilter::ObjectIs(..))
                    && *filter == baylee_cards_dsl::Filter::This
                {
                    return true;
                }
                let fx_source = effect.source.unwrap_or(object.id);
                let chosen_subtype = state.object(fx_source).and_then(|o| o.chosen_subtype);
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    effect.controller,
                    crate::text_changes::RuleContext {
                        source: fx_source,
                        text: state.effect_text(effect),
                        resolved_name: None,
                        resolved_subtype: chosen_subtype,
                        reference: None,
                    },
                )
            }),
            DamageTarget::Player(_) => false,
        },
        DamageRecipients::PlayersOrTheirPermanents(who) => {
            let player = match recipient {
                DamageTarget::Player(player) => player,
                DamageTarget::Object(target) => {
                    let Some(object) = state.object(target) else {
                        return false;
                    };
                    object.controller
                }
            };
            super::candidates::effect_affects_player(effect, who, state, player)
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
