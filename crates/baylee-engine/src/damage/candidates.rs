//! Recompute the applicable effects after every selection and redirection.

use super::{
    Candidate, DamageEffectKind as Kind, DamageEffectOption, DamageTarget, DamageWork, EffectKey,
    GameState, Part,
};
use crate::prevention::{ShieldKind, ShieldOrigin, still_a_creature};
use crate::zone::Zone;
use baylee_cards_dsl::Modifier;

impl DamageWork {
    pub(super) fn candidates(&mut self, state: &GameState) -> Vec<Candidate> {
        let mut out: Vec<Candidate> = Vec::new();
        for part in &self.parts {
            if part.view.amount == 0 || !part.recipient_exists(state) {
                continue;
            }
            for (id, shield, origin) in state.shields.identified() {
                if !shield.protects.covers(state, part.view.recipient) {
                    continue;
                }
                let kind = match shield.kind {
                    ShieldKind::Next(remaining) if remaining > 0 => Kind::PreventNext { remaining },
                    ShieldKind::AllCombat if part.view.is_combat => Kind::PreventCombat,
                    ShieldKind::NextFrom {
                        source,
                        all_but,
                        gain_life,
                        combat_only,
                    } if (!combat_only || part.view.is_combat)
                        && state
                            .damage_source(part.view.source, part.source_version)
                            .is_some_and(|actual| source.deals_as(state, actual))
                        && part.view.amount > all_but =>
                    {
                        Kind::PreventFromSource {
                            source: source.id,
                            all_but,
                            gain_life,
                        }
                    }
                    ShieldKind::RedirectNextFrom { source, to }
                        if still_a_creature(state, part.view.recipient)
                            && !state.has_left(to)
                            && state
                                .damage_source(part.view.source, part.source_version)
                                .is_some_and(|actual| source.deals_as(state, actual)) =>
                    {
                        Kind::Redirect {
                            to: DamageTarget::Player(to),
                        }
                    }
                    _ => continue,
                };
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
            let kind = match fx.modifier {
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
                    if !crate::eval::matches_projected(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        fx.controller,
                        fx.source.unwrap_or(object.id),
                    ) {
                        continue;
                    }
                    Kind::Protection
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
                        .find(|(key, _)| *key == EffectKey::Continuous(fx.id))
                        .map_or(0, |(_, n)| *n);
                    if remaining == 0 || applied >= part.view.amount {
                        continue;
                    }
                    Kind::RemoveCounter { kind, remaining }
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
                    let fits = crate::eval::matches_projected(
                        filter,
                        state,
                        source,
                        source.characteristics(),
                        player,
                        this,
                    );
                    if !fits {
                        continue;
                    }
                    let Some(to) = state.battlefield_seen().find(|&id| {
                        still_a_creature(state, DamageTarget::Object(id))
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
            let origin = fx.source.map(|source| ShieldOrigin {
                source,
                ability: None,
            });
            add(
                out,
                part,
                EffectKey::Continuous(fx.id),
                kind,
                origin,
                fx.controller,
            );
        }
    }
}

fn add(
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
