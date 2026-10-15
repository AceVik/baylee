//! Resolved source-bound damage replacement instructions.

use super::{origin, recipients};
use crate::choice::Pending;
use crate::event::DamageTarget;
use crate::prevention::{ChosenSource, RedirectDestination, Shield, ShieldKind, Shielded};
use crate::resolve::{AwaitingOp, Resolution};
use crate::state::GameState;
use baylee_cards_dsl::{DamageRedirectTo, Effect, Filter, TargetSpec};
use baylee_core::ids::DamageSourceRef;

/// Payload retained while the controller chooses a damage source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChosenShield {
    /// Redirect the next damage to this exact permanent.
    Redirect {
        /// Exact destination incarnation.
        to: DamageSourceRef,
    },
    /// This spell deals the reflected damage.
    Reflect {
        /// Exact reflecting spell incarnation.
        source: DamageSourceRef,
    },
}

pub(crate) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let chosen = match op {
        Effect::RedirectTargetSourceDamageThisTurn {
            source,
            only_to_you,
            to,
        } => {
            let reference = object(state, res, source)?;
            let source = ChosenSource::new_with_context(
                state,
                reference,
                &Filter::Any,
                res.controller,
                res.rule_context(),
            )?;
            let to = match to {
                DamageRedirectTo::ThisObject => {
                    RedirectDestination::Object(crate::resolve::subjects::source(state, res)?)
                }
                DamageRedirectTo::Target(target) => {
                    RedirectDestination::Object(object(state, res, target)?)
                }
                DamageRedirectTo::DamagingSourceController => RedirectDestination::SourceController,
            };
            push(
                state,
                res,
                if only_to_you {
                    Shielded::Player(res.controller)
                } else {
                    Shielded::Everything
                },
                ShieldKind::RedirectSource {
                    source,
                    to,
                    next: false,
                },
            );
            return None;
        }
        Effect::PreventDamageFromTargetingCausesThisTurn { target } => {
            let reference = object(state, res, target)?;
            push(
                state,
                res,
                Shielded::Object(reference.object, reference.version),
                ShieldKind::TargetingCauses,
            );
            return None;
        }
        Effect::RedirectNextChosenSourceDamageToTarget { target } => ChosenShield::Redirect {
            to: object(state, res, target)?,
        },
        Effect::ReflectNextChosenSourceDamage => ChosenShield::Reflect {
            source: crate::resolve::subjects::source(state, res)?,
        },
        _ => unreachable!("damage shield instruction"),
    };
    let options = crate::prevention::source_options_with_context(
        state,
        &Filter::Any,
        res.controller,
        res.rule_context(),
    );
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::DamageShieldSource { kind: chosen });
    Some(Pending::ChooseDamageSource {
        player: res.controller,
        choice: state.next_source_choice(),
        options,
    })
}

pub(crate) fn answer(
    state: &mut GameState,
    res: &Resolution,
    kind: ChosenShield,
    chosen: DamageSourceRef,
) {
    let Some(source) = ChosenSource::new_with_context(
        state,
        chosen,
        &Filter::Any,
        res.controller,
        res.rule_context(),
    ) else {
        return;
    };
    let kind = match kind {
        ChosenShield::Redirect { to } => ShieldKind::RedirectSource {
            source,
            to: RedirectDestination::Object(to),
            next: true,
        },
        ChosenShield::Reflect { source: reflection } => {
            ShieldKind::ReflectNextSource { source, reflection }
        }
    };
    push(state, res, Shielded::Player(res.controller), kind);
}

fn object(state: &GameState, res: &Resolution, target: TargetSpec) -> Option<DamageSourceRef> {
    let DamageTarget::Object(id) = *recipients(state, res, res.controller, target).first()? else {
        return None;
    };
    state.source_identity(id)
}

fn push(state: &mut GameState, res: &Resolution, protects: Shielded, kind: ShieldKind) {
    let origin = origin(state, res);
    state.shields.push_from(
        Shield {
            protects,
            kind,
            controller: res.controller,
        },
        Some(origin),
    );
}

pub(super) fn cause(state: &GameState, res: &Resolution) -> Option<crate::damage::DamageCause> {
    let object = state.object(res.on_stack)?;
    let mut targets = Vec::new();
    for (second, group) in [
        (false, object.targets.as_slice()),
        (
            true,
            object
                .second
                .as_ref()
                .map_or(&[][..], |s| s.targets.as_slice()),
        ),
    ] {
        for (index, &target) in group.iter().enumerate() {
            if let Some(reference) = state
                .recorded_target_reference(res.on_stack, second, u32::try_from(index).ok()?)
                .or_else(|| state.source_identity(target))
            {
                targets.push(reference);
            }
        }
    }
    Some(crate::damage::DamageCause {
        object: state.source_identity(res.on_stack)?,
        targets,
    })
}
