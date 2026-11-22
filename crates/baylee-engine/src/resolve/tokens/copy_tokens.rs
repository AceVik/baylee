//! Generalized token-copy creation (`Effect::CreateTokenCopy` and
//! `Effect::ChooseAndCreateTokenCopy`, CR 707.2, CR 707.9, CR 111.10).

use crate::choice::{ChoicePrompt, Pending};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::eval;
use crate::resolve::{
    AwaitingOp, Resolution, amount2, attacking_tokens, players_of, source_version, subjects,
};
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use baylee_cards_dsl::{
    Amount, CopyCleanup, CopyMod, Duration, Effect, Filter, KeywordSet, Layer, Modifier, PlayerRel,
    TargetSpec,
};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId, PlayerId};

static EXILE_EVENT_OBJECT: &[Effect] = &[Effect::Exile {
    target: TargetSpec::EventObject,
}];

/// Parameters shared by targeted and untargeted token-copy instructions.
#[derive(Clone, Copy, Debug, Hash)]
pub struct TokenCopySpec {
    /// Characteristic exceptions applied in Layer 1.
    pub mods: &'static [CopyMod],
    /// Whether created tokens enter tapped.
    pub tapped: bool,
    /// Whether created tokens enter attacking (CR 508.4).
    pub attacking: bool,
    /// Whether created tokens gain haste until end of turn.
    pub haste_until_eot: bool,
    /// Delayed sacrifice or exile cleanup scheduled per token.
    pub cleanup: CopyCleanup,
}

pub(super) fn create_for_target(
    state: &mut GameState,
    res: &mut Resolution,
    target: TargetSpec,
    controller: PlayerRel,
    amount: Amount,
    spec: TokenCopySpec,
) -> Option<Pending> {
    let you = res.controller;
    let count = amount2(&amount, state, you, res);
    if count == 0 {
        res.subject.found.clear();
        return None;
    }
    let originals = resolve_originals(state, res, target);
    if originals.is_empty() {
        res.subject.found.clear();
        return None;
    }
    let recipients = players_of(controller, state, you, res);
    let mut all_made = Vec::new();
    for recipient in recipients {
        for &original in &originals {
            let made = super::create_modified_token_copies_for(
                state, res, recipient, original, spec.mods, count,
            );
            apply_token_riders(state, res, recipient, &made, spec);
            all_made.extend(made);
        }
    }
    super::remember_created_tokens(state, res, &all_made);
    if spec.attacking {
        attacking_tokens::start(state, res, all_made)
    } else {
        None
    }
}

pub(super) fn choose_and_create(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    amount: Amount,
    spec: TokenCopySpec,
) -> Option<Pending> {
    let you = res.controller;
    let count = amount2(&amount, state, you, res);
    if count == 0 {
        res.subject.found.clear();
        return None;
    }
    let context = res.rule_context();
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state
                .object(id)
                .is_some_and(|obj| eval::matches_with_context(filter, state, obj, you, context))
        })
        .collect();
    match options.as_slice() {
        [] => {
            res.subject.found.clear();
            None
        }
        [only] => finish_chosen(state, res, &[*only], count, spec),
        _ => {
            res.awaiting = Some(AwaitingOp::ChooseTokenCopy { count, spec });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
    }
}

pub(in crate::resolve) fn finish_chosen(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
    count: u32,
    spec: TokenCopySpec,
) -> Option<Pending> {
    let original = state.source_identity(*chosen.first()?)?;
    let you = res.controller;
    let made = super::create_modified_token_copies_for(state, res, you, original, spec.mods, count);
    apply_token_riders(state, res, you, &made, spec);
    super::remember_created_tokens(state, res, &made);
    if spec.attacking {
        attacking_tokens::start(state, res, made)
    } else {
        None
    }
}

fn resolve_originals(
    state: &GameState,
    res: &Resolution,
    target: TargetSpec,
) -> Vec<DamageSourceRef> {
    let targets = match target {
        TargetSpec::ThisObject => return subjects::source(state, res).into_iter().collect(),
        TargetSpec::EventObject => return subjects::event(state, res).into_iter().collect(),
        TargetSpec::CardInFirstTargetsGraveyard(_) => res.second_targets.as_slice(),
        _ => res.targets.as_slice(),
    };
    targets
        .iter()
        .filter_map(|&id| state.source_identity(id))
        .collect()
}

fn apply_token_riders(
    state: &mut GameState,
    res: &Resolution,
    controller: PlayerId,
    made: &[ObjectId],
    spec: TokenCopySpec,
) {
    for &id in made {
        if spec.tapped {
            state.set_tapped(id, true);
        }
        if spec.haste_until_eot {
            let timestamp = state.next_timestamp();
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: Some(res.source),
                controller,
                origin: EffectOrigin::Resolution,
                layer: Layer::Ability,
                timestamp,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::object(state, id),
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            });
        }
    }
    schedule_cleanup(state, res, controller, made, spec.cleanup);
}

pub(in crate::resolve) fn schedule_cleanup(
    state: &mut GameState,
    res: &Resolution,
    controller: PlayerId,
    made: &[ObjectId],
    cleanup: CopyCleanup,
) {
    if cleanup == CopyCleanup::None {
        return;
    }
    let src_version = source_version(state, res)
        .or_else(|| state.object(res.source).map(|o| o.version))
        .unwrap_or(0);
    for &id in made {
        let version = state.object(id).map_or(0, |o| o.version);
        let delayed = match cleanup {
            CopyCleanup::None => None,
            CopyCleanup::SacrificeAtNextEndStep => Some((
                DelayedWhen::NextEndStep,
                DelayedAction::Sacrifice { card: id, version },
            )),
            CopyCleanup::SacrificeAtEndOfCombat => Some((
                DelayedWhen::EndOfCombatThisTurn,
                DelayedAction::Sacrifice { card: id, version },
            )),
            CopyCleanup::ExileAtNextEndStep => Some((
                DelayedWhen::NextEndStep,
                DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: src_version,
                    effects: EXILE_EVENT_OBJECT,
                    text: res.text,
                    object: id,
                    version,
                    subject: None,
                },
            )),
            CopyCleanup::ExileAtEndOfCombat => Some((
                DelayedWhen::EndOfCombatThisTurn,
                DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: src_version,
                    effects: EXILE_EVENT_OBJECT,
                    text: res.text,
                    object: id,
                    version,
                    subject: None,
                },
            )),
        };
        if let Some((when, action)) = delayed {
            state.delayed.push(DelayedTrigger {
                controller,
                when,
                action,
            });
        }
    }
}
