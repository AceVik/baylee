//! Engine Wave 13a resolution operations:
//! - Generalized [`ChooseZone`] (`Hand`, `Exile`, `LinkedExile`, `Found`) and
//!   [`Effect::ChooseCardEx`] (`CR 608.2d`, `CR 609.3`, `CR 701.20a`)
//! - [`Effect::SeparateIntoTwoPiles`] (`CR 700.3`)

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::zone::Zone;
use baylee_cards_dsl::{ChooseMove, ChooseZone, Filter, ZoneSel};
use baylee_core::ids::DamageSourceRef;

/// Stage of a two-branch resolution continuation (`ChooseCardEx` `then` ->
/// `for_unchosen`, or `SeparateIntoTwoPiles` `chosen_pile` -> `unchosen_pile`).
#[derive(Clone, Debug, Hash)]
pub(in crate::resolve) enum BranchStage {
    /// First branch finished; run `second_effects` with `second_refs` bound,
    /// then restore `restore_refs`.
    RunSecond {
        second_refs: Vec<DamageSourceRef>,
        second_effects: &'static [Effect],
        restore_refs: Vec<DamageSourceRef>,
        require_exact_version: bool,
    },
    /// Second branch finished; restore `restore_refs` onto `SubjectContext::found`
    /// and `EventObject`.
    RestoreAfterSecond { restore_refs: Vec<DamageSourceRef> },
}

/// Re-entrant continuation frame stored on [`SubjectContext`] so interactive
/// `Pending` choices inside `then` / `chosen_pile` or `for_unchosen` /
/// `unchosen_pile` suspend and resume cleanly without losing the sibling group.
#[derive(Clone, Debug, Hash)]
pub(in crate::resolve) struct BranchFrame {
    pub(super) tail_from_end: usize,
    pub(super) stage: BranchStage,
}

/// Suspended state for [`Effect::ChooseCardEx`] while awaiting `Pending::ChooseCards`.
#[derive(Clone, Debug, Hash)]
pub struct ChooseCardExState {
    /// Choosing player seat.
    pub ask_player: PlayerId,
    /// Snapshotted eligible candidates `(id, version, zone)` in deterministic order.
    pub candidates: Vec<(ObjectId, u32, Zone)>,
    /// Whether chosen cards are explicitly revealed (`GameEvent::Revealed`).
    pub reveal_chosen: bool,
    /// Movement applied to chosen objects.
    pub movement: ChooseMove,
    /// Movement applied to eligible unchosen objects.
    pub not_chosen_movement: ChooseMove,
    /// Follow-up instructions for chosen objects.
    pub then: &'static [Effect],
    /// Follow-up instructions for unchosen objects.
    pub for_unchosen: &'static [Effect],
    /// Originating instruction used as continuation sentinel if both branches run.
    pub op: Effect,
}

/// Suspended state for [`ChooseMove::BottomOfLibraryChosenOrder`] inside [`Effect::ChooseCardEx`].
#[derive(Clone, Debug, Hash)]
pub struct ChooseCardExBottomState {
    /// Owner of the library currently being arranged.
    pub current_owner: PlayerId,
    /// Cards being arranged onto `current_owner`'s library bottom.
    pub current_cards: Vec<ObjectId>,
    /// Remaining `(owner, cards)` batches requiring `Pending::Arrange`.
    pub remaining_batches: Vec<(PlayerId, Vec<ObjectId>)>,
    /// Whether `current_cards` belong to `chosen_refs` (`true`) or `unchosen_refs` (`false`).
    pub current_is_chosen: bool,
    /// Remaining batches' membership flags matching `remaining_batches`.
    pub remaining_is_chosen: Vec<bool>,
    /// Post-movement chosen object references.
    pub chosen_refs: Vec<DamageSourceRef>,
    /// Post-movement unchosen object references.
    pub unchosen_refs: Vec<DamageSourceRef>,
    /// Follow-up instructions for chosen objects.
    pub then: &'static [Effect],
    /// Follow-up instructions for unchosen objects.
    pub for_unchosen: &'static [Effect],
    /// Originating instruction used as continuation sentinel.
    pub op: Effect,
}

/// Suspended state for [`Effect::SeparateIntoTwoPiles`].
#[derive(Clone, Debug, Hash)]
pub struct TwoPilesState {
    /// Snapshotted candidate incarnations in deterministic order.
    pub candidates: Vec<DamageSourceRef>,
    /// Relation naming who chooses one of the two piles.
    pub pile_chooser: PlayerRel,
    /// Instructions executed on the chosen pile.
    pub chosen_pile: &'static [Effect],
    /// Instructions executed on the unchosen pile.
    pub unchosen_pile: &'static [Effect],
    /// Originating instruction used as continuation sentinel.
    pub op: Effect,
}

/// Suspended pile-selection state for [`Effect::SeparateIntoTwoPiles`] after separation.
#[derive(Clone, Debug, Hash)]
pub struct TwoPilesSplitState {
    /// First pile incarnations.
    pub first: Vec<DamageSourceRef>,
    /// Second pile incarnations.
    pub second: Vec<DamageSourceRef>,
    /// Instructions executed on the chosen pile.
    pub chosen_pile: &'static [Effect],
    /// Instructions executed on the unchosen pile.
    pub unchosen_pile: &'static [Effect],
    /// Originating instruction used as continuation sentinel.
    pub op: Effect,
}

/// Collect eligible `(id, version, zone)` candidates for [`ChooseZone`].
pub(super) fn collect_choose_zone_candidates(
    state: &GameState,
    res: &Resolution,
    zone: ChooseZone,
    filter: &'static Filter,
    ask_player: PlayerId,
) -> Vec<(ObjectId, u32, Zone)> {
    let you = res.controller;
    let raw_ids: Vec<ObjectId> = match zone {
        ChooseZone::Battlefield => {
            let mut pairs: Vec<(ObjectId, PlayerId)> = state
                .battlefield_seen()
                .filter_map(|id| state.object(id).map(|o| (id, o.controller)))
                .collect();
            pairs.sort_by_key(|(_, c)| crate::battle::sba::apnap_distance(state, *c));
            pairs.into_iter().map(|(id, _)| id).collect()
        }
        ChooseZone::Graveyard(owner_rel) => {
            super::wave12a::apnap_seats_of(state, players_of(owner_rel, state, you, res))
                .into_iter()
                .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
                .collect()
        }
        ChooseZone::Hand(owner_rel) => {
            super::wave12a::apnap_seats_of(state, players_of(owner_rel, state, you, res))
                .into_iter()
                .flat_map(|p| state.zones.list(ZoneLocation::Hand(p)).iter().copied())
                .collect()
        }
        ChooseZone::Exile(owner_rel) => {
            super::wave12a::apnap_seats_of(state, players_of(owner_rel, state, you, res))
                .into_iter()
                .flat_map(|p| state.zones.list(ZoneLocation::Exile(p)).iter().copied())
                .collect()
        }
        ChooseZone::LinkedExile => {
            let mut cards = eval::linked_exile_cards(
                state,
                you,
                res.rule_context(),
                super::source_version(state, res),
                &Filter::Any,
            );
            for r in super::hidden_exile::linked(state, res) {
                if !cards.contains(&r.object) {
                    cards.push(r.object);
                }
            }
            cards.sort_by_key(|&id| {
                state.object(id).map_or(usize::MAX, |o| {
                    crate::battle::sba::apnap_distance(state, o.owner)
                })
            });
            cards
        }
        ChooseZone::Found | ChooseZone::Remembered => {
            let mut out = Vec::new();
            let refs: Vec<baylee_core::ids::DamageSourceRef> = match zone {
                ChooseZone::Found => res.subject.found.clone(),
                ChooseZone::Remembered => res
                    .subject
                    .remembered()
                    .map_or_else(Vec::new, |list| list.iter().map(|r| r.reference).collect()),
                _ => unreachable!(),
            };
            for r in refs {
                let Some(obj) = state.object(r.object) else {
                    continue;
                };
                if obj.version != r.version {
                    continue;
                }
                if obj.zone == Zone::Battlefield
                    && obj.status.contains(crate::object::Status::PHASED_OUT)
                {
                    continue;
                }
                if matches_choose_candidate(filter, state, obj, you, res, ask_player)
                    && !out.iter().any(|&(seen, _, _)| seen == r.object)
                {
                    out.push((r.object, obj.version, obj.zone));
                }
            }
            return out;
        }
    };
    let mut out = Vec::new();
    for id in raw_ids {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if matches_choose_candidate(filter, state, obj, you, res, ask_player) {
            out.push((id, obj.version, obj.zone));
        }
    }
    out
}

fn matches_choose_candidate(
    filter: &'static Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    res: &Resolution,
    ask_player: PlayerId,
) -> bool {
    if obj.zone == Zone::Exile
        && obj.status.contains(crate::object::Status::FACE_DOWN)
        && obj.may_look_at_exile(ask_player)
        && !matches!(filter, Filter::Any | Filter::FaceDown)
        && let Some(orig) = obj.original_base.as_deref()
    {
        return matches_in_resolution_with_chars(
            filter,
            state,
            obj,
            orig,
            you,
            res,
            Some(ask_player),
        );
    }
    matches_in_resolution(filter, state, obj, you, res, Some(ask_player))
}

pub(super) const fn choose_prompt_for(zone: ChooseZone, movement: ChooseMove) -> ChoicePrompt {
    match (zone, movement) {
        (ChooseZone::Graveyard(_), _) => ChoicePrompt::FromGraveyard,
        (_, ChooseMove::Hand) => ChoicePrompt::PutIntoHand,
        (_, ChooseMove::Battlefield { .. }) => ChoicePrompt::PutOntoBattlefield,
        (_, ChooseMove::TopOfLibrary) => ChoicePrompt::PutBackOnTop,
        (
            _,
            ChooseMove::BottomOfLibrary
            | ChooseMove::BottomOfLibraryChosenOrder
            | ChooseMove::BottomOfLibraryRandom,
        ) => ChoicePrompt::PutOnBottom,
        _ => ChoicePrompt::Generic,
    }
}

/// Entry and branch-continuation handler for [`Effect::ChooseCardEx`].
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub(super) fn choose_card_ex(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
    zone: ChooseZone,
    filter: &'static Filter,
    min: u8,
    max: Amount,
    reveal_chosen: bool,
    movement: ChooseMove,
    not_chosen_movement: ChooseMove,
    then: &'static [Effect],
    for_unchosen: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    if res
        .subject
        .branch_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len)
    {
        advance_branch_frame(state, res, op);
        return None;
    }
    let you = res.controller;
    let Some(ask_player) =
        super::wave12a::apnap_seats_of(state, players_of(chooser, state, you, res))
            .into_iter()
            .find(|&p| !state.has_left(p))
    else {
        clear_found_and_event(res);
        return None;
    };
    let candidates = collect_choose_zone_candidates(state, res, zone, filter, ask_player);
    if candidates.is_empty() {
        clear_found_and_event(res);
        return None;
    }
    let max_n = amount2(&max, state, you, res);
    let suspended = ChooseCardExState {
        ask_player,
        candidates: candidates.clone(),
        reveal_chosen,
        movement,
        not_chosen_movement,
        then,
        for_unchosen,
        op,
    };
    if max_n == 0 {
        return choose_card_ex_finish(state, res, suspended, &[]);
    }
    let avail = u8::try_from(candidates.len()).unwrap_or(u8::MAX);
    let max_clamped = u8::try_from(max_n).unwrap_or(u8::MAX).min(avail);
    let min_clamped = min.min(max_clamped);
    let options: Vec<ObjectId> = candidates.iter().map(|&(id, _, _)| id).collect();
    let prompt = choose_prompt_for(zone, movement);
    res.awaiting = Some(AwaitingOp::ChooseCardExOp(Box::new(suspended)));
    Some(Pending::ChooseCards {
        player: ask_player,
        options,
        min: min_clamped,
        max: max_clamped,
        prompt,
        total: None,
    })
}

/// Complete [`Effect::ChooseCardEx`] once `chosen` has been selected.
pub(super) fn choose_card_ex_finish(
    state: &mut GameState,
    res: &mut Resolution,
    mut spec: ChooseCardExState,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let still_valid = |id: ObjectId, ver: u32, zone: Zone, s: &GameState| {
        s.object(id)
            .is_some_and(|o| o.version == ver && o.zone == zone)
    };
    let mut valid_chosen = Vec::new();
    for &id in chosen {
        if !valid_chosen.contains(&id)
            && let Some(&(_, ver, z)) = spec.candidates.iter().find(|&&(cid, _, _)| cid == id)
            && still_valid(id, ver, z, state)
        {
            valid_chosen.push(id);
        }
    }
    let valid_unchosen: Vec<ObjectId> = spec
        .candidates
        .iter()
        .filter_map(|&(id, ver, z)| {
            (!valid_chosen.contains(&id) && still_valid(id, ver, z, state)).then_some(id)
        })
        .collect();

    if spec.reveal_chosen && !valid_chosen.is_empty() {
        record_revealed_by_owner(state, spec.ask_player, &valid_chosen);
        spec.reveal_chosen = false;
    }

    let you = res.controller;
    let mut planned = Vec::new();
    collect_planned_moves(state, you, &valid_chosen, spec.movement, &mut planned);
    collect_planned_moves(
        state,
        you,
        &valid_unchosen,
        spec.not_chosen_movement,
        &mut planned,
    );
    if !planned.is_empty()
        && let Some(pending) = ask_commander_replace(state, res, &planned)
    {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement retains owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::ChooseCardExOp(Box::new(spec))),
            chosen: valid_chosen,
            asked,
            remaining,
        });
        return Some(pending);
    }

    finish_choose_card_ex_moves(state, res, you, &valid_chosen, &valid_unchosen, &spec)
}

fn finish_choose_card_ex_moves(
    state: &mut GameState,
    res: &mut Resolution,
    you: PlayerId,
    valid_chosen: &[ObjectId],
    valid_unchosen: &[ObjectId],
    spec: &ChooseCardExState,
) -> Option<Pending> {
    if !valid_chosen.is_empty() {
        res.subject.note_captured_ids(valid_chosen);
    }
    let mut arrange_batches: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
    let mut arrange_is_chosen: Vec<bool> = Vec::new();
    let chosen_refs = apply_batch_choose_move(
        state,
        res,
        you,
        valid_chosen,
        spec.movement,
        true,
        &mut arrange_batches,
        &mut arrange_is_chosen,
    );
    let unchosen_refs = apply_batch_choose_move(
        state,
        res,
        you,
        valid_unchosen,
        spec.not_chosen_movement,
        false,
        &mut arrange_batches,
        &mut arrange_is_chosen,
    );
    if !arrange_batches.is_empty() {
        let (owner, cards) = arrange_batches.remove(0);
        let is_chosen = arrange_is_chosen.remove(0);
        let n = u32::try_from(cards.len()).unwrap_or(u32::MAX);
        res.awaiting = Some(AwaitingOp::ChooseCardExArrangeBottom(Box::new(
            ChooseCardExBottomState {
                current_owner: owner,
                current_cards: cards.clone(),
                remaining_batches: arrange_batches,
                current_is_chosen: is_chosen,
                remaining_is_chosen: arrange_is_chosen,
                chosen_refs,
                unchosen_refs,
                then: spec.then,
                for_unchosen: spec.for_unchosen,
                op: spec.op,
            },
        )));
        return Some(Pending::Arrange {
            player: you,
            cards,
            piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
            prompt: ArrangePrompt::Order,
        });
    }
    schedule_two_branches(
        state,
        res,
        chosen_refs,
        spec.then,
        unchosen_refs,
        spec.for_unchosen,
        false,
        spec.op,
    );
    None
}

/// Resume [`AwaitingOp::ChooseCardExArrangeBottom`] after `arranged` is chosen.
pub(super) fn choose_card_ex_resume_bottom(
    state: &mut GameState,
    res: &mut Resolution,
    mut bottom_state: ChooseCardExBottomState,
    arranged: &[ObjectId],
) -> Option<Pending> {
    let target_lib = ZoneLocation::Library(bottom_state.current_owner);
    let mut order: Vec<ObjectId> = arranged
        .iter()
        .copied()
        .filter(|id| bottom_state.current_cards.contains(id))
        .collect();
    for &id in &bottom_state.current_cards {
        if !order.contains(&id) {
            order.push(id);
        }
    }
    let refs_to_update = if bottom_state.current_is_chosen {
        &mut bottom_state.chosen_refs
    } else {
        &mut bottom_state.unchosen_refs
    };
    for card in order {
        if let Some(obj) = state.object_mut(card) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object(card, target_lib, ZonePosition::Bottom, Cause::Effect);
        if let Some(updated) = state.source_identity(card) {
            if let Some(slot) = refs_to_update.iter_mut().find(|r| r.object == card) {
                *slot = updated;
            } else {
                refs_to_update.push(updated);
            }
        }
    }
    if !bottom_state.remaining_batches.is_empty() {
        let (owner, cards) = bottom_state.remaining_batches.remove(0);
        let is_chosen = bottom_state.remaining_is_chosen.remove(0);
        let n = u32::try_from(cards.len()).unwrap_or(u32::MAX);
        bottom_state.current_owner = owner;
        bottom_state.current_cards.clone_from(&cards);
        bottom_state.current_is_chosen = is_chosen;
        res.awaiting = Some(AwaitingOp::ChooseCardExArrangeBottom(Box::new(
            bottom_state,
        )));
        return Some(Pending::Arrange {
            player: res.controller,
            cards,
            piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
            prompt: ArrangePrompt::Order,
        });
    }
    schedule_two_branches(
        state,
        res,
        bottom_state.chosen_refs,
        bottom_state.then,
        bottom_state.unchosen_refs,
        bottom_state.for_unchosen,
        false,
        bottom_state.op,
    );
    None
}

fn record_revealed_by_owner(state: &mut GameState, fallback: PlayerId, cards: &[ObjectId]) {
    let mut groups: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
    for &card in cards {
        let owner = state
            .object(card)
            .and_then(|o| o.zone_owner.or(Some(o.owner)))
            .unwrap_or(fallback);
        if let Some((_, list)) = groups.iter_mut().find(|(p, _)| *p == owner) {
            list.push(card);
        } else {
            groups.push((owner, vec![card]));
        }
    }
    for (player, group_cards) in groups {
        state.journal.record(GameEvent::Revealed {
            player,
            cards: group_cards,
        });
    }
}

pub(super) fn collect_planned_moves(
    state: &GameState,
    you: PlayerId,
    ids: &[ObjectId],
    movement: ChooseMove,
    out: &mut Vec<(ObjectId, ZoneLocation)>,
) {
    if matches!(movement, ChooseMove::None) {
        return;
    }
    for &id in ids {
        let Some(obj) = state.object(id) else {
            continue;
        };
        let owner = obj.owner;
        let dest = match movement {
            ChooseMove::None => unreachable!(),
            ChooseMove::Hand => ZoneLocation::Hand(owner),
            ChooseMove::Battlefield { .. } => ZoneLocation::Battlefield,
            ChooseMove::TopOfLibrary
            | ChooseMove::BottomOfLibrary
            | ChooseMove::BottomOfLibraryChosenOrder
            | ChooseMove::BottomOfLibraryRandom => {
                if obj.zone == Zone::Library && obj.zone_owner == Some(owner) {
                    continue;
                }
                ZoneLocation::Library(owner)
            }
            ChooseMove::Graveyard => ZoneLocation::Graveyard(owner),
            ChooseMove::Exile | ChooseMove::ExileLinked => {
                if obj.zone == Zone::Exile {
                    continue;
                }
                ZoneLocation::Exile(owner)
            }
        };
        let _ = you;
        out.push((id, dest));
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_batch_choose_move(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    ids: &[ObjectId],
    movement: ChooseMove,
    is_chosen: bool,
    arrange_batches: &mut Vec<(PlayerId, Vec<ObjectId>)>,
    arrange_is_chosen: &mut Vec<bool>,
) -> Vec<DamageSourceRef> {
    if ids.is_empty() {
        return Vec::new();
    }
    match movement {
        ChooseMove::BottomOfLibraryRandom => {
            let mut by_owner: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
            for &id in ids {
                let owner = state.object(id).map_or(you, |o| o.owner);
                if let Some((_, list)) = by_owner.iter_mut().find(|(p, _)| *p == owner) {
                    list.push(id);
                } else {
                    by_owner.push((owner, vec![id]));
                }
            }
            for (owner, mut group) in by_owner {
                state.rng.shuffle(&mut group);
                let lib = ZoneLocation::Library(owner);
                for card in group {
                    if let Some(obj) = state.object_mut(card) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = state.move_object(card, lib, ZonePosition::Bottom, Cause::Effect);
                }
            }
            ids.iter()
                .filter_map(|&id| state.source_identity(id))
                .collect()
        }
        ChooseMove::BottomOfLibraryChosenOrder => apply_bottom_chosen_order(
            state,
            you,
            ids,
            is_chosen,
            arrange_batches,
            arrange_is_chosen,
        ),
        other => {
            let mut refs = Vec::with_capacity(ids.len());
            for &id in ids {
                apply_single_choose_move(state, res, you, id, other);
                if let Some(r) = state.source_identity(id) {
                    refs.push(r);
                }
            }
            refs
        }
    }
}

fn apply_bottom_chosen_order(
    state: &mut GameState,
    you: PlayerId,
    ids: &[ObjectId],
    is_chosen: bool,
    arrange_batches: &mut Vec<(PlayerId, Vec<ObjectId>)>,
    arrange_is_chosen: &mut Vec<bool>,
) -> Vec<DamageSourceRef> {
    let mut by_owner: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
    for &id in ids {
        let owner = state.object(id).map_or(you, |o| o.owner);
        if let Some((_, list)) = by_owner.iter_mut().find(|(p, _)| *p == owner) {
            list.push(id);
        } else {
            by_owner.push((owner, vec![id]));
        }
    }
    let mut refs: Vec<DamageSourceRef> = Vec::with_capacity(ids.len());
    for (owner, group) in by_owner {
        let redirected: Vec<ObjectId> = group
            .iter()
            .copied()
            .filter(|&id| {
                state
                    .commander_redirect
                    .iter()
                    .any(|&(o, redirect)| o == id && redirect)
            })
            .collect();
        for &id in &redirected {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
            if let Some(r) = state.source_identity(id) {
                refs.push(r);
            }
        }
        let remaining: Vec<ObjectId> = group
            .into_iter()
            .filter(|id| !redirected.contains(id))
            .collect();
        if remaining.len() >= 2 {
            for &id in &remaining {
                if let Some(r) = state.source_identity(id) {
                    refs.push(r);
                }
            }
            arrange_batches.push((owner, remaining));
            arrange_is_chosen.push(is_chosen);
        } else if let Some(&only) = remaining.first() {
            if let Some(obj) = state.object_mut(only) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                only,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
            if let Some(r) = state.source_identity(only) {
                refs.push(r);
            }
        }
    }
    refs
}

pub(super) fn apply_single_choose_move(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    id: ObjectId,
    movement: ChooseMove,
) {
    let Some(current_zone) = state.object(id).map(|o| o.zone) else {
        return;
    };
    let owner = state.object(id).map_or(you, |o| o.owner);
    match movement {
        ChooseMove::None => {}
        ChooseMove::Hand => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::Battlefield { tapped } => {
            if tapped {
                state.restore_tapped(id, true);
            }
            put_found(state, you, id, SearchDest::Battlefield);
        }
        ChooseMove::TopOfLibrary => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::BottomOfLibrary
        | ChooseMove::BottomOfLibraryChosenOrder
        | ChooseMove::BottomOfLibraryRandom => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
        }
        ChooseMove::Graveyard => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::Exile => {
            if current_zone != Zone::Exile {
                if let Some(obj) = state.object_mut(id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        ChooseMove::ExileLinked => super::wave12a::link_in_exile(state, res, owner, id),
    }
}

// -----------------------------------------------------------------------------
// `Effect::SeparateIntoTwoPiles` (`CR 700.3`)
// -----------------------------------------------------------------------------

/// Entry and branch-continuation handler for [`Effect::SeparateIntoTwoPiles`].
#[allow(clippy::too_many_arguments)]
pub(super) fn separate_into_two_piles(
    state: &mut GameState,
    res: &mut Resolution,
    separator: PlayerRel,
    pile_chooser: PlayerRel,
    zone: ZoneSel,
    filter: &'static Filter,
    only_found: bool,
    chosen_pile: &'static [Effect],
    unchosen_pile: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    if res
        .subject
        .branch_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len)
    {
        advance_branch_frame(state, res, op);
        return None;
    }
    let you = res.controller;
    let raw = super::wave12a::snapshot_zone_candidates(state, res, zone);
    let mut candidates = Vec::new();
    for id in raw {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if only_found
            && !res
                .subject
                .found
                .iter()
                .any(|r| r.object == id && r.version == obj.version)
        {
            continue;
        }
        if !matches_in_resolution(filter, state, obj, you, res, None) {
            continue;
        }
        candidates.push(DamageSourceRef {
            object: id,
            version: obj.version,
        });
    }
    if candidates.is_empty() {
        clear_found_and_event(res);
        return None;
    }
    let sep_seats: Vec<PlayerId> =
        super::wave12a::apnap_seats_of(state, players_of(separator, state, you, res))
            .into_iter()
            .filter(|&p| !state.has_left(p))
            .collect();
    let pile_state = TwoPilesState {
        candidates,
        pile_chooser,
        chosen_pile,
        unchosen_pile,
        op,
    };
    match sep_seats.as_slice() {
        [] => None,
        [only] => ask_two_piles_separator(state, res, *only, pile_state),
        _ => {
            res.awaiting = Some(AwaitingOp::TwoPilesPickSeparator(Box::new(pile_state)));
            Some(Pending::ChoosePlayer {
                player: you,
                options: sep_seats,
            })
        }
    }
}

/// Ask `separator` to choose the first pile (`ChoicePrompt::FirstPile`).
pub(super) fn ask_two_piles_separator(
    state: &GameState,
    res: &mut Resolution,
    separator: PlayerId,
    pile_state: TwoPilesState,
) -> Option<Pending> {
    let options: Vec<ObjectId> = pile_state
        .candidates
        .iter()
        .filter(|r| {
            state
                .object(r.object)
                .is_some_and(|o| o.version == r.version)
        })
        .map(|r| r.object)
        .collect();
    if options.is_empty() {
        clear_found_and_event(res);
        return None;
    }
    let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
    res.awaiting = Some(AwaitingOp::TwoPilesFirstPile(Box::new(pile_state)));
    Some(Pending::ChooseCards {
        player: separator,
        options,
        min: 0,
        max,
        prompt: ChoicePrompt::FirstPile,
        total: None,
    })
}

/// Transition from `FirstPile` selection (`chosen`) to asking `pile_chooser` (`Pending::ChoosePile`).
pub(super) fn two_piles_after_separation(
    state: &GameState,
    res: &mut Resolution,
    pile_state: TwoPilesState,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let live: Vec<DamageSourceRef> = pile_state
        .candidates
        .into_iter()
        .filter(|r| {
            state
                .object(r.object)
                .is_some_and(|o| o.version == r.version)
        })
        .collect();
    if live.is_empty() {
        clear_found_and_event(res);
        return None;
    }
    let (first, second): (Vec<DamageSourceRef>, Vec<DamageSourceRef>) =
        live.into_iter().partition(|r| chosen.contains(&r.object));
    let you = res.controller;
    let chooser_seats: Vec<PlayerId> =
        super::wave12a::apnap_seats_of(state, players_of(pile_state.pile_chooser, state, you, res))
            .into_iter()
            .filter(|&p| !state.has_left(p))
            .collect();
    let split = TwoPilesSplitState {
        first,
        second,
        chosen_pile: pile_state.chosen_pile,
        unchosen_pile: pile_state.unchosen_pile,
        op: pile_state.op,
    };
    match chooser_seats.as_slice() {
        [] => None,
        [only] => Some(ask_two_piles_chooser(state, res, *only, split)),
        _ => {
            res.awaiting = Some(AwaitingOp::TwoPilesPickChooser(Box::new(split)));
            Some(Pending::ChoosePlayer {
                player: you,
                options: chooser_seats,
            })
        }
    }
}

/// Ask `chooser` to pick one of the two separated piles (`Pending::ChoosePile`).
pub(super) fn ask_two_piles_chooser(
    state: &GameState,
    res: &mut Resolution,
    chooser: PlayerId,
    split: TwoPilesSplitState,
) -> Pending {
    let first_ids: Vec<ObjectId> = split
        .first
        .iter()
        .filter(|r| {
            state
                .object(r.object)
                .is_some_and(|o| o.version == r.version)
        })
        .map(|r| r.object)
        .collect();
    let second_ids: Vec<ObjectId> = split
        .second
        .iter()
        .filter(|r| {
            state
                .object(r.object)
                .is_some_and(|o| o.version == r.version)
        })
        .map(|r| r.object)
        .collect();
    res.awaiting = Some(AwaitingOp::TwoPilesChoosePile(Box::new(split)));
    Pending::ChoosePile {
        player: chooser,
        piles: vec![first_ids, second_ids],
        label: None,
    }
}

/// Complete [`Effect::SeparateIntoTwoPiles`] once `index` (`0` or `1`) is chosen.
pub(super) fn two_piles_finish(
    state: &mut GameState,
    res: &mut Resolution,
    split: TwoPilesSplitState,
    index: usize,
) {
    let (raw_chosen, raw_unchosen) = if index == 0 {
        (split.first, split.second)
    } else {
        (split.second, split.first)
    };
    let chosen_refs = filter_live_refs(state, &raw_chosen, true);
    let unchosen_refs = filter_live_refs(state, &raw_unchosen, true);
    schedule_two_branches(
        state,
        res,
        chosen_refs,
        split.chosen_pile,
        unchosen_refs,
        split.unchosen_pile,
        true,
        split.op,
    );
}

// -----------------------------------------------------------------------------
// Shared two-branch continuation machinery
// -----------------------------------------------------------------------------

fn clear_found_and_event(res: &mut Resolution) {
    res.subject.found.clear();
    res.event_object = None;
    res.subject.set_event_identity(None);
}

fn filter_live_refs(
    state: &GameState,
    refs: &[DamageSourceRef],
    require_exact_version: bool,
) -> Vec<DamageSourceRef> {
    refs.iter()
        .filter_map(|&r| {
            let obj = state.object(r.object)?;
            if require_exact_version && obj.version != r.version {
                return None;
            }
            Some(DamageSourceRef {
                object: r.object,
                version: obj.version,
            })
        })
        .collect()
}

fn bind_refs(state: &GameState, res: &mut Resolution, refs: &[DamageSourceRef]) {
    res.subject.found.clear();
    if refs.is_empty() {
        res.event_object = None;
        res.subject.set_event_identity(None);
        return;
    }
    res.subject.found.extend_from_slice(refs);
    super::wave3_ops::bind_remembered_object(state, res, refs[0].object);
}

#[allow(clippy::too_many_arguments)]
fn schedule_two_branches(
    state: &GameState,
    res: &mut Resolution,
    first_refs: Vec<DamageSourceRef>,
    first_effects: &'static [Effect],
    second_refs: Vec<DamageSourceRef>,
    second_effects: &'static [Effect],
    require_exact_version: bool,
    sentinel_op: Effect,
) {
    let run_first = !first_refs.is_empty() && !first_effects.is_empty();
    let run_second = !second_refs.is_empty() && !second_effects.is_empty();
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let at = res.pc + 1;

    match (run_first, run_second) {
        (false, false) => {
            bind_refs(state, res, &first_refs);
        }
        (true, false) => {
            bind_refs(state, res, &first_refs);
            res.effects.splice(at..at, super::flatten(first_effects));
        }
        (false, true) => {
            bind_refs(state, res, &second_refs);
            let mut spliced = super::flatten(second_effects);
            spliced.push(sentinel_op);
            res.subject.branch_frames.push(BranchFrame {
                tail_from_end: after_len,
                stage: BranchStage::RestoreAfterSecond {
                    restore_refs: first_refs,
                },
            });
            res.effects.splice(at..at, spliced);
        }
        (true, true) => {
            bind_refs(state, res, &first_refs);
            let mut spliced = super::flatten(first_effects);
            spliced.push(sentinel_op);
            res.subject.branch_frames.push(BranchFrame {
                tail_from_end: after_len,
                stage: BranchStage::RunSecond {
                    second_refs,
                    second_effects,
                    restore_refs: first_refs,
                    require_exact_version,
                },
            });
            res.effects.splice(at..at, spliced);
        }
    }
}

fn advance_branch_frame(state: &GameState, res: &mut Resolution, sentinel_op: Effect) {
    let Some(frame) = res.subject.branch_frames.pop() else {
        return;
    };
    match frame.stage {
        BranchStage::RunSecond {
            second_refs,
            second_effects,
            restore_refs,
            require_exact_version,
        } => {
            let live_second = filter_live_refs(state, &second_refs, require_exact_version);
            if live_second.is_empty() || second_effects.is_empty() {
                let live_restore = filter_live_refs(state, &restore_refs, false);
                bind_refs(state, res, &live_restore);
                return;
            }
            bind_refs(state, res, &live_second);
            let at = res.pc + 1;
            let mut spliced = super::flatten(second_effects);
            spliced.push(sentinel_op);
            res.subject.branch_frames.push(BranchFrame {
                tail_from_end: frame.tail_from_end,
                stage: BranchStage::RestoreAfterSecond { restore_refs },
            });
            res.effects.splice(at..at, spliced);
        }
        BranchStage::RestoreAfterSecond { restore_refs } => {
            let live_restore = filter_live_refs(state, &restore_refs, false);
            bind_refs(state, res, &live_restore);
        }
    }
}
