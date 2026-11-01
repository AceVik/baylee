//! Recompute the applicable effects after every selection and redirection.

use super::{
    Candidate, DamageEffectKind as Kind, DamageEffectOption, DamageTarget, DamageWork, EffectKey,
    GameState, Part,
};
use crate::prevention::{ShieldKind, ShieldOrigin, damageable_recipient};
use crate::zone::Zone;
use baylee_cards_dsl::Modifier;

impl DamageWork {
    #[allow(clippy::too_many_lines)]
    pub(super) fn candidates(&mut self, state: &GameState) -> Vec<Candidate> {
        let mut out: Vec<Candidate> = Vec::new();
        for part in &self.parts {
            if part.view.amount == 0 || !part.recipient_exists(state) {
                continue;
            }
            masked_candidates(state, part, &mut out);
            self.batch57_candidates(state, part, &mut out);
            for (id, shield, origin) in state.shields.identified() {
                if !shield.protects.covers(state, part.view.recipient) {
                    continue;
                }
                let kind = match shield.kind {
                    ShieldKind::Next(remaining) if remaining > 0 => Kind::PreventNext { remaining },
                    ShieldKind::RedirectNext { remaining, to }
                        if remaining > 0
                            && damageable_recipient(state, part.view.recipient)
                            && !state.has_left(to) =>
                    {
                        Kind::RedirectNext {
                            remaining,
                            to: DamageTarget::Player(to),
                        }
                    }
                    ShieldKind::AllCombat if part.view.is_combat => Kind::PreventCombat,
                    ShieldKind::AllFrom { source }
                        if state
                            .damage_source(part.view.source, part.source_version)
                            .is_some_and(|actual| source.deals_as(state, actual)) =>
                    {
                        Kind::PreventAll
                    }
                    ShieldKind::NextFrom {
                        source,
                        all_but,
                        gain_life,
                        combat_only,
                    } if (!combat_only || part.view.is_combat)
                        && state
                            .damage_source(part.view.source, part.source_version)
                            .is_some_and(|actual| source.deals_as(state, actual))
                        && (all_but == u32::from(u8::MAX) || part.view.amount > all_but) =>
                    {
                        Kind::PreventFromSource {
                            source: source.id,
                            all_but,
                            gain_life,
                        }
                    }
                    ShieldKind::RedirectNextFrom { source, to }
                        if damageable_recipient(state, part.view.recipient)
                            && !state.has_left(to)
                            && state
                                .damage_source(part.view.source, part.source_version)
                                .is_some_and(|actual| source.deals_as(state, actual)) =>
                    {
                        Kind::Redirect {
                            to: DamageTarget::Player(to),
                        }
                    }
                    _ => match super::batch57::shield_kind(state, part, shield.kind)
                        .or_else(|| super::ice_combat::shield_kind(state, part, *shield))
                    {
                        Some(kind) => kind,
                        None => continue,
                    },
                };
                if matches!(kind, Kind::Redirect { .. } | Kind::RedirectNext { .. })
                    && self.recipient_locked(state, part.view.recipient)
                {
                    continue;
                }
                add(
                    &mut out,
                    part,
                    EffectKey::Shield(id),
                    kind,
                    origin,
                    shield.controller,
                );
            }
            if let Some((remaining, origin, controller)) = self.paid
                && remaining > 0
            {
                add(
                    &mut out,
                    part,
                    EffectKey::Paid,
                    Kind::PreventThisEvent { remaining },
                    Some(origin),
                    controller,
                );
            }
            self.continuous_candidates(state, part, &mut out);
        }
        for candidate in &mut out {
            let identity = (candidate.key, candidate.recipient);
            let index = if let Some(i) = self.option_keys.iter().position(|key| *key == identity) {
                i
            } else {
                self.option_keys.push(identity);
                self.option_keys.len() - 1
            };
            candidate.option.id = u32::try_from(index).expect("too many damage effect options");
        }
        out
    }

    #[allow(clippy::too_many_lines)] // One exhaustive table of supported continuous damage modifiers.
    fn continuous_candidates(&self, state: &GameState, part: &Part, out: &mut Vec<Candidate>) {
        for fx in state.effects.iter() {
            let mut key = EffectKey::Continuous(fx.id);
            let kind = match fx.modifier {
                Modifier::PlayerProtectionFromEverything { who } => {
                    let DamageTarget::Player(player) = part.view.recipient else {
                        continue;
                    };
                    if !crate::eval::players(who, state, fx.controller)
                        .is_some_and(|players| players.contains(&player))
                    {
                        continue;
                    }
                    Kind::Protection
                }
                Modifier::PlayerProtectionFromOpponents { who } => {
                    let DamageTarget::Player(player) = part.view.recipient else {
                        continue;
                    };
                    if !crate::eval::players(who, state, fx.controller)
                        .is_some_and(|players| players.contains(&player))
                    {
                        continue;
                    }
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    if !state.is_opponent(source.controller, player) {
                        continue;
                    }
                    Kind::Protection
                }
                Modifier::ProtectionFrom(filter) => {
                    let DamageTarget::Object(target) = part.view.recipient else {
                        continue;
                    };
                    let Some(object) = state
                        .object(target)
                        .filter(|o| crate::effects::applies_to(state, fx, o))
                    else {
                        continue;
                    };
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    if !crate::eval::matches_projected_with_context(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        fx.controller,
                        crate::text_changes::RuleContext {
                            source: fx.source.unwrap_or(object.id),
                            text: state.effect_text(fx),
                            resolved_name: None,
                            reference: None,
                        },
                    ) {
                        continue;
                    }
                    Kind::Protection
                }
                Modifier::ProtectionFromExiledWithSourceTypes => {
                    let DamageTarget::Object(target) = part.view.recipient else {
                        continue;
                    };
                    let Some(object) = state.object(target) else {
                        continue;
                    };
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    if !crate::eval::protection_from_exiled_types(state, fx, object, source) {
                        continue;
                    }
                    Kind::Protection
                }

                Modifier::PreventDamageFrom(filter) | Modifier::PreventCombatDamageFrom(filter) => {
                    if matches!(fx.modifier, Modifier::PreventCombatDamageFrom(_))
                        && !part.view.is_combat
                    {
                        continue;
                    }
                    let DamageTarget::Object(target) = part.view.recipient else {
                        continue;
                    };
                    let Some(object) = state
                        .object(target)
                        .filter(|o| crate::effects::applies_to(state, fx, o))
                    else {
                        continue;
                    };
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    if !crate::eval::matches_projected_with_context(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        fx.controller,
                        crate::text_changes::RuleContext {
                            source: fx.source.unwrap_or(object.id),
                            text: state.effect_text(fx),
                            resolved_name: None,
                            reference: None,
                        },
                    ) {
                        continue;
                    }
                    Kind::PreventAll
                }
                Modifier::PreventAllDamageFromIt
                    if state
                        .damage_source(part.view.source, part.source_version)
                        .is_some_and(|o| crate::effects::applies_to(state, fx, o)) =>
                {
                    Kind::PreventAll
                }
                Modifier::PreventDamageToYouFrom(filter) => {
                    if part.view.recipient != DamageTarget::Player(fx.controller) {
                        continue;
                    }
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    if !crate::eval::matches_projected_with_context(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        fx.controller,
                        crate::text_changes::RuleContext {
                            source: fx.source.unwrap_or(source.id),
                            text: state.effect_text(fx),
                            resolved_name: None,
                            reference: None,
                        },
                    ) {
                        continue;
                    }
                    Kind::PreventAll
                }
                Modifier::PreventDamageFromIt
                    if part.view.is_combat
                        && state
                            .object(part.view.source)
                            .is_some_and(|o| crate::effects::applies_to(state, fx, o)) =>
                {
                    Kind::PreventCombat
                }
                Modifier::PreventDamageToIt
                    if part.view.is_combat
                        && matches!(part.view.recipient,
                    DamageTarget::Object(id) if state.object(id).is_some_and(|o| crate::effects::applies_to(state,fx,o))) =>
                {
                    Kind::PreventCombat
                }
                Modifier::CountersPreventDamage(kind) => {
                    let DamageTarget::Object(id) = part.view.recipient else {
                        continue;
                    };
                    let Some(obj) = state
                        .object(id)
                        .filter(|o| crate::effects::applies_to(state, fx, o))
                    else {
                        continue;
                    };
                    let remaining = self.counter_remaining(state, obj.id, kind);
                    let applied = part
                        .counters_applied
                        .iter()
                        .find(|(k, _)| *k == EffectKey::Continuous(fx.id))
                        .map_or(0, |(_, n)| *n);
                    if remaining == 0 || applied >= part.view.amount {
                        continue;
                    }
                    Kind::RemoveCounter { kind, remaining }
                }
                Modifier::MayRedirectDamageToYou => {
                    let DamageTarget::Object(target) = part.view.recipient else {
                        continue;
                    };
                    if state.has_left(fx.controller)
                        || self.recipient_locked(state, part.view.recipient)
                        || !state.object(target).is_some_and(|o| {
                            o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                                && crate::effects::applies_to(state, fx, o)
                        })
                    {
                        continue;
                    }
                    key = EffectKey::MayRedirect(fx.id);
                    Kind::Redirect {
                        to: DamageTarget::Player(fx.controller),
                    }
                }
                Modifier::RedirectDamageToYou(filter) => {
                    let DamageTarget::Player(player) = part.view.recipient else {
                        continue;
                    };
                    if player != fx.controller || state.has_left(player) {
                        continue;
                    }
                    let Some(source) = state.damage_source(part.view.source, part.source_version)
                    else {
                        continue;
                    };
                    let this = fx.source.unwrap_or(part.view.source);
                    let fits = crate::eval::matches_projected_with_context(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        player,
                        crate::text_changes::RuleContext {
                            source: this,
                            text: state.effect_text(fx),
                            resolved_name: None,
                            reference: None,
                        },
                    );
                    if !fits {
                        continue;
                    }
                    let Some(to) = state.battlefield_seen().find(|&id| {
                        damageable_recipient(state, DamageTarget::Object(id))
                            && state
                                .object(id)
                                .is_some_and(|o| crate::effects::applies_to(state, fx, o))
                    }) else {
                        continue;
                    };
                    Kind::Redirect {
                        to: DamageTarget::Object(to),
                    }
                }
                _ => continue,
            };
            if matches!(kind, Kind::Redirect { .. } | Kind::RedirectNext { .. })
                && self.recipient_locked(state, part.view.recipient)
            {
                continue;
            }
            let origin = fx.source.map(|source| ShieldOrigin {
                source,
                ability: None,
            });
            add(out, part, key, kind, origin, fx.controller);
        }
    }
}

pub(super) fn add(
    out: &mut Vec<Candidate>,
    part: &Part,
    key: EffectKey,
    kind: Kind,
    origin: Option<ShieldOrigin>,
    controller: baylee_core::ids::PlayerId,
) {
    if part.applied.contains(&key) {
        return;
    }
    if let Some(candidate) = out
        .iter_mut()
        .find(|c| c.key == key && c.recipient == part.view.recipient)
    {
        candidate.option.parts.push(part.view.id);
    } else {
        out.push(Candidate {
            key,
            recipient: part.view.recipient,
            option: DamageEffectOption {
                id: 0,
                source: origin.map(|o| o.source),
                ability: origin.and_then(|o| o.ability),
                controller,
                kind,
                parts: vec![part.view.id],
            },
        });
    }
}

impl Part {
    pub(super) fn recipient_exists(&self, state: &GameState) -> bool {
        match self.view.recipient {
            DamageTarget::Player(p) => !state.has_left(p),
            DamageTarget::Object(id) => state.object(id).is_some_and(|o| {
                o.zone == Zone::Battlefield && self.recipient_version == Some(o.version)
            }),
        }
    }
}

fn masked_candidates(state: &GameState, part: &Part, out: &mut Vec<Candidate>) {
    let source = state
        .object(part.view.source)
        .filter(|object| part.source_version == Some(object.version))
        .map(|object| object.id);
    let recipient = match part.view.recipient {
        DamageTarget::Object(id) => Some(id),
        DamageTarget::Player(_) => None,
    };
    for object in source
        .into_iter()
        .chain(recipient.filter(|id| Some(*id) != source))
    {
        if !state.awaits_masked_reveal(object) {
            continue;
        }
        let reference = state
            .source_identity(object)
            .expect("present masked permanent");
        add(
            out,
            part,
            EffectKey::FaceUp(reference),
            Kind::TurnFaceUp { object: reference },
            Some(ShieldOrigin {
                source: object,
                ability: None,
            }),
            state
                .object(object)
                .expect("present masked permanent")
                .controller,
        );
    }
}
