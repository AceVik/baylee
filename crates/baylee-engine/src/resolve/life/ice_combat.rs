//! Source-bound prevention and combat redirection established on resolution.
use crate::choice::Pending;
use crate::prevention::{ChosenSource, RedirectDestination, Shield, ShieldKind, Shielded};
use crate::resolve::{Resolution, players_of, subjects};
use crate::state::GameState;
use baylee_cards_dsl::{DamageRedirectTo, Effect, Filter};

pub(in crate::resolve) fn exec(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
) -> Option<Pending> {
    let origin = super::origin(state, res);
    match op {
        Effect::PreventNextDamageFromSource { to, combat_only } => {
            let reference = baylee_core::ids::DamageSourceRef {
                object: res.source,
                version: crate::resolve::source_version(state, res).or_else(|| {
                    state
                        .source_identity(res.source)
                        .map(|source| source.version)
                })?,
            };
            let source = ChosenSource::new_with_context(
                state,
                reference,
                &Filter::Any,
                res.controller,
                res.rule_context(),
            )?;
            for player in players_of(to, state, res.controller, res) {
                state.shields.push_from(
                    Shield {
                        protects: Shielded::Player(player),
                        kind: ShieldKind::NextFrom {
                            source,
                            all_but: 0,
                            gain_life: false,
                            combat_only,
                        },
                        controller: res.controller,
                    },
                    Some(origin),
                );
            }
        }
        Effect::RedirectCombatDamageToYouThisTurn { by, to } => {
            let to = match to {
                DamageRedirectTo::ThisObject => {
                    RedirectDestination::Object(subjects::source(state, res)?)
                }
                DamageRedirectTo::Target(target) => {
                    let crate::event::DamageTarget::Object(id) =
                        *super::recipients(state, res, res.controller, target).first()?
                    else {
                        return None;
                    };
                    RedirectDestination::Object(state.source_identity(id)?)
                }
                DamageRedirectTo::DamagingSourceController => RedirectDestination::SourceController,
            };
            state.shields.push_from(
                Shield {
                    protects: Shielded::Player(res.controller),
                    kind: ShieldKind::RedirectCombat {
                        sources: by,
                        to,
                        context: res.rule_context(),
                    },
                    controller: res.controller,
                },
                Some(origin),
            );
        }
        _ => unreachable!("combat damage instruction"),
    }
    None
}
