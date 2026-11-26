//! Resolving [`Effect::SetState`] state transitions: Kamigawa Flip (`CR 710`),
//! Transform (`CR 701.27`), `TurnFaceDown` (`CR 708.2`, `CR 712.16`), and
//! `TurnFaceUp` (`CR 708.8`, `CR 701.40g`).

#[allow(clippy::wildcard_imports)] // parent interpreter vocabulary
use super::*;
use crate::state::CardLookup as _;
use baylee_cards_dsl::{StateSubject, StateTransition};

pub(super) fn exec(
    state: &mut GameState,
    res: &mut Resolution,
    subject: StateSubject,
    transition: StateTransition,
    if_cant: &'static [Effect],
) -> Option<Pending> {
    let candidates = collect_candidates(state, res, subject, transition);
    let mut failed: Vec<ObjectId> = Vec::new();
    if let StateTransition::RevealAndTurnFaceUpIf { filter, optional } = transition {
        process_reveal_turn_up(state, res, candidates, filter, optional, if_cant, failed)
    } else {
        for id in candidates {
            if !apply_immediate(state, res, id, subject, transition) {
                failed.push(id);
            }
        }
        finish_failed(state, res, if_cant, failed)
    }
}

pub(super) fn resume_optional_turn_face_up(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if !matches!(
        res.awaiting,
        Some(AwaitingOp::SetStateOptionalTurnFaceUp { .. })
    ) {
        return None;
    }
    let Some(AwaitingOp::SetStateOptionalTurnFaceUp {
        current,
        remaining,
        mut failed,
    }) = res.awaiting.take()
    else {
        unreachable!("SetStateOptionalTurnFaceUp checked above");
    };
    let Some(Effect::SetState {
        transition: StateTransition::RevealAndTurnFaceUpIf { filter, optional },
        if_cant,
        ..
    }) = res.effects.get(res.pc).copied()
    else {
        unreachable!("SetStateOptionalTurnFaceUp must point at RevealAndTurnFaceUpIf");
    };
    if answer && !state.turn_permanent_face_up(current) {
        failed.push(current);
    }
    if let Some(pending) =
        process_reveal_turn_up(state, res, remaining, filter, optional, if_cant, failed)
    {
        return Some(Flow::Wait(pending));
    }
    res.pc += 1;
    Some(run(state, res))
}

fn process_reveal_turn_up(
    state: &mut GameState,
    res: &mut Resolution,
    candidates: Vec<ObjectId>,
    filter: &'static baylee_cards_dsl::Filter,
    optional: bool,
    if_cant: &'static [Effect],
    mut failed: Vec<ObjectId>,
) -> Option<Pending> {
    let mut iter = candidates.into_iter();
    while let Some(id) = iter.next() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if obj.zone != crate::zone::Zone::Battlefield
            || obj.status.contains(crate::object::Status::PHASED_OUT)
            || !obj.status.contains(crate::object::Status::FACE_DOWN)
        {
            failed.push(id);
            continue;
        }
        state.journal.record(GameEvent::Revealed {
            player: res.controller,
            cards: vec![id],
        });
        if !revealed_matches(state, res, id, filter) {
            failed.push(id);
            continue;
        }
        if optional {
            let remaining: Vec<ObjectId> = iter.collect();
            res.awaiting = Some(AwaitingOp::SetStateOptionalTurnFaceUp {
                current: id,
                remaining,
                failed,
            });
            return Some(Pending::YesNo {
                player: res.controller,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            });
        }
        if !state.turn_permanent_face_up(id) {
            failed.push(id);
        }
    }
    finish_failed(state, res, if_cant, failed)
}

fn finish_failed(
    state: &mut GameState,
    res: &mut Resolution,
    if_cant: &'static [Effect],
    failed: Vec<ObjectId>,
) -> Option<Pending> {
    if failed.is_empty() || if_cant.is_empty() {
        return None;
    }
    let failed_targets: SmallVec<[ObjectId; 2]> = SmallVec::from_slice(&failed);
    res.subject.found = failed
        .into_iter()
        .filter_map(|id| state.source_identity(id))
        .collect();
    run_nested_with(state, res, flatten(if_cant), failed_targets)
}

fn collect_candidates(
    state: &GameState,
    res: &Resolution,
    subject: StateSubject,
    transition: StateTransition,
) -> Vec<ObjectId> {
    let allow_exile = matches!(transition, StateTransition::TurnFaceUp);
    let in_valid_zone = |id: ObjectId| {
        state.object(id).is_some_and(|o| {
            (o.zone == crate::zone::Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT))
                || (allow_exile && o.zone == crate::zone::Zone::Exile)
        })
    };
    match subject {
        StateSubject::Source => subjects::source(state, res)
            .filter(|r| subjects::on_battlefield(state, *r))
            .map(|r| r.object)
            .into_iter()
            .collect(),
        StateSubject::Targets => zones::moving_objects(
            state,
            res,
            TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
        )
        .into_iter()
        .filter(|&id| in_valid_zone(id))
        .collect(),
        StateSubject::EventObject => subjects::event(state, res)
            .filter(|r| subjects::is_current(state, *r) && in_valid_zone(r.object))
            .map(|r| r.object)
            .into_iter()
            .collect(),
        StateSubject::Found => res
            .subject
            .found
            .iter()
            .copied()
            .filter(|r| subjects::is_current(state, *r) && in_valid_zone(r.object))
            .map(|r| r.object)
            .collect(),
        StateSubject::Remembered => {
            if let Some(remembered) = res.subject.remembered_objects.as_deref() {
                remembered
                    .iter()
                    .map(|entry| entry.reference)
                    .filter(|r| subjects::is_current(state, *r) && in_valid_zone(r.object))
                    .map(|r| r.object)
                    .collect()
            } else {
                res.subject
                    .found
                    .iter()
                    .copied()
                    .filter(|r| subjects::is_current(state, *r) && in_valid_zone(r.object))
                    .map(|r| r.object)
                    .collect()
            }
        }
        StateSubject::BattlefieldFilter(filter) => state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    eval::matches_with_context(filter, state, o, res.controller, res.rule_context())
                })
            })
            .collect(),
        StateSubject::LinkedExile => collect_linked_exile(state, res),
    }
}

fn collect_linked_exile(state: &GameState, res: &Resolution) -> Vec<ObjectId> {
    let Some(source) = subjects::source(state, res) else {
        return Vec::new();
    };
    let mut out = eval::linked_exile_cards(
        state,
        res.controller,
        res.rule_context(),
        Some(source.version),
        &baylee_cards_dsl::Filter::Any,
    );
    let src_acquisition = state
        .object(source.object)
        .filter(|o| o.version == source.version)
        .map(crate::object::GameObject::rules_acquisition);
    for player in &state.players {
        for &id in state.zones.list(ZoneLocation::Exile(player.id)) {
            if out.contains(&id) {
                continue;
            }
            let linked = state.object(id).is_some_and(|o| {
                o.is_card()
                    && o.riders.iter().any(|r| {
                        matches!(
                            r,
                            crate::object::Rider::LinkedExile {
                                source: s,
                                acquisition,
                            } if *s == source
                                && src_acquisition.is_none_or(|a| *acquisition == a)
                        )
                    })
            });
            if linked {
                out.push(id);
            }
        }
    }
    out
}

fn apply_immediate(
    state: &mut GameState,
    res: &Resolution,
    id: ObjectId,
    subject: StateSubject,
    transition: StateTransition,
) -> bool {
    match transition {
        StateTransition::Flip => {
            let Some(def) = state
                .object(id)
                .and_then(|o| {
                    o.printed_face()
                        .map(crate::object::PrintedFace::card)
                        .or_else(|| o.card.map(|c| c.index))
                })
                .and_then(|idx| state.bases.card(idx))
            else {
                return false;
            };
            state.flip_permanent(id, def)
        }
        StateTransition::Transform => {
            // CR 701.27g: if an activated or triggered ability of a permanent
            // instructs it to transform and it has already transformed since
            // the ability was put on the stack, do nothing.
            if matches!(subject, StateSubject::Source) {
                let printed = state.object(res.on_stack).and_then(|ability| {
                    ability
                        .own_origin
                        .and_then(crate::object::AbilityOrigin::printed)
                        .map(crate::object::PrintedFace::face)
                        .or_else(|| {
                            ability.riders.iter().find_map(|r| match r {
                                crate::object::Rider::AbilitySourceFace(face) => Some(*face),
                                _ => None,
                            })
                        })
                });
                if let Some(expected) = printed
                    && state.object(id).is_none_or(|o| o.face_index != expected)
                {
                    return false;
                }
            }
            let Some(obj) = state.object(id) else {
                return false;
            };
            let current_face = obj.face_index;
            if let Some(def) = obj.card.and_then(|c| state.bases.card(c.index)) {
                let target_face = 1 - usize::from(current_face.min(1));
                state.transform(id, def, target_face)
            } else if obj.token.is_some_and(|t| t.back_face.is_some()) {
                let target_face = 1 - current_face.min(1);
                state.transform_token(id, target_face)
            } else {
                false
            }
        }
        StateTransition::TurnFaceDown(spec) => state.turn_permanent_face_down(id, spec),
        StateTransition::TurnFaceUp => {
            if state
                .object(id)
                .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
            {
                if state
                    .object(id)
                    .is_some_and(|o| o.status.contains(crate::object::Status::FACE_DOWN))
                {
                    state.reveal_exiled_card(id);
                    true
                } else {
                    false
                }
            } else {
                state.turn_permanent_face_up(id)
            }
        }
        StateTransition::RevealAndTurnFaceUpIf { .. } => {
            unreachable!("handled by process_reveal_turn_up")
        }
    }
}

fn revealed_matches(
    state: &GameState,
    res: &Resolution,
    id: ObjectId,
    filter: &'static baylee_cards_dsl::Filter,
) -> bool {
    let Some(obj) = state.object(id) else {
        return false;
    };
    let copied = state
        .face_down_copied_bases
        .iter()
        .find(|(r, _)| r.object == id && r.version == obj.version)
        .map(|(_, b)| std::sync::Arc::clone(b));
    let Some(face_up_base) =
        copied.or_else(|| obj.original_base.as_ref().map(std::sync::Arc::clone))
    else {
        return false;
    };
    let mut revealed = obj.clone();
    revealed.status.remove(crate::object::Status::FACE_DOWN);
    revealed.base = face_up_base;
    revealed.cache.clear();
    eval::matches_with_context(filter, state, &revealed, res.controller, res.rule_context())
}
