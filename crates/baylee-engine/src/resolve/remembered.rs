//! Resolution-scoped remembered objects (`CR 400.7`, `CR 608.2c`, `CR 608.2h`)
//! for [`baylee_cards_dsl::Effect::CaptureAffected`] and
//! [`baylee_cards_dsl::Effect::ForEachRemembered`].

use std::sync::Arc;

use baylee_cards_dsl::{Amount, Effect, Filter};
use baylee_core::{
    ids::{DamageSourceRef, NameRef, ObjectId, PlayerId},
    types::TypeSet,
};

use super::{
    Pending, Resolution, flatten, matches_in_resolution, matches_in_resolution_with_chars, subjects,
};
use crate::{
    event::{Cause, Departure, GameEvent},
    object::{Characteristics, GameObject},
    state::GameState,
    zone::Zone,
};

/// The nine traditional card types (`CR 205.2a`) compared by
/// [`Filter::SharesCardTypeWithRemembered`].
pub const PAPER_CARD_TYPES: TypeSet = TypeSet::ARTIFACT
    .union(TypeSet::CREATURE)
    .union(TypeSet::ENCHANTMENT)
    .union(TypeSet::INSTANT)
    .union(TypeSet::KINDRED)
    .union(TypeSet::LAND)
    .union(TypeSet::PLANESWALKER)
    .union(TypeSet::SORCERY)
    .union(TypeSet::BATTLE);

/// Frozen `CR 608.2h` characteristics of an object immediately before the
/// capturing instruction moved it, or as created/revealed/chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RememberedLki {
    /// Controller immediately before departure (or owner in non-battlefield zones).
    pub controller: PlayerId,
    /// Card or token owner (`CR 108.3`, `CR 111.2`).
    pub owner: PlayerId,
    /// Mana value (`CR 202.3`).
    pub mana_value: u32,
    /// Power when the object had one (`CR 208.3`).
    pub power: Option<i16>,
    /// Toughness when the object had one (`CR 208.3`).
    pub toughness: Option<i16>,
    /// Types on the object (`CR 205.2a`).
    pub card_types: TypeSet,
    /// Interned name (`CR 201.2`).
    pub name: NameRef,
}

impl RememberedLki {
    /// Capture live or off-battlefield CDA characteristics of `obj`.
    #[must_use]
    pub fn from_object(state: &GameState, obj: &GameObject) -> Self {
        let chars = obj.characteristics();
        let (power, toughness) = if let Some(baylee_cards_dsl::Modifier::SetPT(p, t)) =
            state.off_battlefield_pt_cda(obj)
        {
            (Some(p), Some(t))
        } else {
            (chars.power, chars.toughness)
        };
        let controller = if matches!(obj.zone, Zone::Battlefield | Zone::Stack) {
            obj.controller
        } else {
            obj.owner
        };
        Self {
            controller,
            owner: obj.owner,
            mana_value: chars.mana_value(),
            power,
            toughness,
            card_types: chars.types,
            name: chars.name,
        }
    }

    fn from_ltb(owner: PlayerId, controller: PlayerId, chars: &Characteristics) -> Self {
        Self {
            controller,
            owner,
            mana_value: chars.mana_value(),
            power: chars.power,
            toughness: chars.toughness,
            card_types: chars.types,
            name: chars.name,
        }
    }
}

/// One object remembered by [`Effect::CaptureAffected`]: its resulting/current
/// incarnation (`reference`, `CR 400.7`) plus its frozen pre-transition LKI
/// (`lki`, `CR 608.2h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RememberedObject {
    /// Incarnation after `first` finished (or departed handle for a ceased token).
    pub reference: DamageSourceRef,
    /// Frozen pre-transition or live characteristics.
    pub lki: RememberedLki,
}

/// Execution phase of an active [`CaptureFrame`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::resolve) enum CapturePhase {
    /// `first` is running; reaching the sentinel harvests affected objects and
    /// transitions into `RunningThen`.
    RunningFirst {
        since_seq: u64,
        pre_lki: Vec<(ObjectId, RememberedLki)>,
        explicit_ids: Vec<ObjectId>,
        saved_found: Vec<DamageSourceRef>,
    },
    /// `then` is running with `SubjectContext::remembered_objects` bound;
    /// reaching the sentinel restores `saved_remembered`.
    RunningThen,
}

/// Suspension- and nesting-safe scope frame for [`Effect::CaptureAffected`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::resolve) struct CaptureFrame {
    pub(crate) tail_from_end: usize,
    pub(crate) saved_remembered: Option<Arc<[RememberedObject]>>,
    pub(crate) phase: CapturePhase,
}

fn snapshot_pre_lki(state: &mut GameState, res: &Resolution) -> Vec<(ObjectId, RememberedLki)> {
    state.refresh_characteristics();
    let mut snap: Vec<(ObjectId, RememberedLki)> = Vec::new();
    let mut record = |id: ObjectId| {
        if let Some(obj) = state.object(id) {
            snap.push((id, RememberedLki::from_object(state, obj)));
        }
    };
    for id in state.battlefield_seen().collect::<Vec<_>>() {
        record(id);
    }
    for &id in state.zones.list(crate::zone::ZoneLocation::Stack) {
        record(id);
    }
    for &id in res
        .targets
        .iter()
        .chain(res.second_targets.iter())
        .chain(res.subject.third_targets().iter())
    {
        record(id);
    }
    snap.sort_by_key(|&(id, _)| id);
    snap.dedup_by_key(|&mut (id, _)| id);
    snap
}

fn push_unique(out: &mut Vec<ObjectId>, id: ObjectId) {
    if !out.contains(&id) {
        out.push(id);
    }
}

fn harvest_captured(
    state: &mut GameState,
    res: &mut Resolution,
    since_seq: u64,
    pre_lki: &[(ObjectId, RememberedLki)],
    explicit_ids: &[ObjectId],
    saved_found: Vec<DamageSourceRef>,
) -> Arc<[RememberedObject]> {
    state.refresh_characteristics();
    let mut ids: Vec<ObjectId> = Vec::new();
    let mut departures: Vec<(ObjectId, Departure)> = Vec::new();

    for entry in state.journal.entries().iter().filter(|e| e.seq > since_seq) {
        if let GameEvent::ZoneChanged { object, cause, .. } = entry.event
            && let Some(dep) = entry.departure.as_deref()
        {
            departures.push((object, dep.clone()));
            let _ = cause;
        }
    }

    if explicit_ids.is_empty() {
        for entry in state.journal.entries().iter().filter(|e| e.seq > since_seq) {
            if let GameEvent::ZoneChanged { object, cause, .. } = entry.event
                && matches!(
                    cause,
                    Cause::Effect | Cause::Destroy | Cause::Spell | Cause::Ability
                )
            {
                push_unique(&mut ids, object);
            }
        }
        if ids.is_empty() {
            for entry in state.journal.entries().iter().filter(|e| e.seq > since_seq) {
                if let GameEvent::Revealed { cards, .. } = &entry.event {
                    for &card in cards {
                        push_unique(&mut ids, card);
                    }
                }
            }
        }
        if ids.is_empty() {
            for entry in state.journal.entries().iter().filter(|e| e.seq > since_seq) {
                if let GameEvent::ControllerChanged { object, .. } = entry.event {
                    push_unique(&mut ids, object);
                }
            }
        }
        if ids.is_empty() {
            for &r in &res.subject.found {
                push_unique(&mut ids, r.object);
            }
        }
    } else {
        for &id in explicit_ids {
            push_unique(&mut ids, id);
        }
    }

    if res.subject.found.is_empty() && !saved_found.is_empty() {
        res.subject.found = saved_found;
    }

    let mut out: Vec<RememberedObject> = Vec::with_capacity(ids.len());
    for id in ids {
        let reference = state.source_identity(id).unwrap_or(DamageSourceRef {
            object: id,
            version: 0,
        });
        let lki = if let Ok(idx) = pre_lki.binary_search_by_key(&id, |&(oid, _)| oid) {
            pre_lki[idx].1
        } else if let Some((_, dep)) = departures.iter().rfind(|(oid, _)| *oid == id)
            && let Some(ltb_chars) = state.last_known_characteristics(id)
            && let Some(obj) = state.object_or_departed(id)
        {
            RememberedLki::from_ltb(obj.owner, dep.controller, ltb_chars)
        } else if let Some(obj) = state.object_or_departed(id) {
            let mut base = RememberedLki::from_object(state, obj);
            if let Some((_, dep)) = departures.iter().rfind(|(oid, _)| *oid == id) {
                base.controller = dep.controller;
                if base.card_types.contains(TypeSet::CREATURE) {
                    base.power = Some(dep.power);
                    base.toughness = Some(dep.toughness);
                }
            }
            base
        } else {
            continue;
        };
        out.push(RememberedObject { reference, lki });
    }
    Arc::from(out)
}

/// Entry, transition, and exit handler for [`Effect::CaptureAffected`].
pub(super) fn exec_capture_affected(
    state: &mut GameState,
    res: &mut Resolution,
    first: &'static [Effect],
    then: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let top_matches = res
        .subject
        .capture_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);

    if top_matches {
        let is_running_first = matches!(
            res.subject.capture_frames.last().map(|f| &f.phase),
            Some(CapturePhase::RunningFirst { .. })
        );
        if is_running_first {
            let Some(CaptureFrame {
                tail_from_end,
                saved_remembered,
                phase:
                    CapturePhase::RunningFirst {
                        since_seq,
                        pre_lki,
                        explicit_ids,
                        saved_found,
                    },
            }) = res.subject.capture_frames.pop()
            else {
                unreachable!()
            };
            let captured =
                harvest_captured(state, res, since_seq, &pre_lki, &explicit_ids, saved_found);
            res.subject.remembered_objects = Some(Arc::clone(&captured));
            state.resolution_remembered = Some((res.source, captured));
            res.subject.capture_frames.push(CaptureFrame {
                tail_from_end,
                saved_remembered,
                phase: CapturePhase::RunningThen,
            });
            let mut body = flatten(then);
            body.push(op);
            let next = res.pc + 1;
            res.effects.splice(next..next, body);
            return None;
        }
        let frame = res.subject.capture_frames.pop().expect("matched above");
        res.subject.remembered_objects = frame.saved_remembered;
        state.resolution_remembered = res
            .subject
            .remembered_objects
            .as_ref()
            .map(|r| (res.source, Arc::clone(r)));
        return None;
    }

    if then.is_empty() {
        if !first.is_empty() {
            let next = res.pc + 1;
            res.effects.splice(next..next, flatten(first));
        }
        return None;
    }

    if first.is_empty() {
        let empty: Arc<[RememberedObject]> = Arc::from(&[][..]);
        let saved_remembered = res.subject.remembered_objects.replace(Arc::clone(&empty));
        state.resolution_remembered = Some((res.source, empty));
        res.subject.capture_frames.push(CaptureFrame {
            tail_from_end: after_len,
            saved_remembered,
            phase: CapturePhase::RunningThen,
        });
        let mut body = flatten(then);
        body.push(op);
        let next = res.pc + 1;
        res.effects.splice(next..next, body);
        return None;
    }

    let pre_lki = snapshot_pre_lki(state, res);
    let saved_found = std::mem::take(&mut res.subject.found);
    let since_seq = state.journal.last_seq();
    res.subject.capture_frames.push(CaptureFrame {
        tail_from_end: after_len,
        saved_remembered: res.subject.remembered_objects.clone(),
        phase: CapturePhase::RunningFirst {
            since_seq,
            pre_lki,
            explicit_ids: Vec::new(),
            saved_found,
        },
    });
    let mut body = flatten(first);
    body.push(op);
    let next = res.pc + 1;
    res.effects.splice(next..next, body);
    None
}

/// Entry and exit handler for [`Effect::CaptureMatching`].
pub(super) fn exec_capture_matching(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    then: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let top_matches = res
        .subject
        .capture_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if top_matches {
        let frame = res.subject.capture_frames.pop().expect("matched above");
        res.subject.remembered_objects = frame.saved_remembered;
        state.resolution_remembered = res
            .subject
            .remembered_objects
            .as_ref()
            .map(|r| (res.source, Arc::clone(r)));
        return None;
    }
    if then.is_empty() {
        return None;
    }
    let you = res.controller;
    state.refresh_characteristics();
    let captured_vec: Vec<RememberedObject> = state
        .battlefield_seen()
        .filter_map(|id| {
            let obj = state.object(id)?;
            matches_in_resolution(filter, state, obj, you, res, None).then(|| RememberedObject {
                reference: DamageSourceRef {
                    object: id,
                    version: obj.version,
                },
                lki: RememberedLki::from_object(state, obj),
            })
        })
        .collect();
    let captured: Arc<[RememberedObject]> = Arc::from(captured_vec);
    let saved_remembered = res
        .subject
        .remembered_objects
        .replace(Arc::clone(&captured));
    state.resolution_remembered = Some((res.source, captured));
    res.subject.capture_frames.push(CaptureFrame {
        tail_from_end: after_len,
        saved_remembered,
        phase: CapturePhase::RunningThen,
    });
    let mut body = flatten(then);
    body.push(op);
    let next = res.pc + 1;
    res.effects.splice(next..next, body);
    None
}

/// Entry and exit handler for [`Effect::RememberTargetsThen`].
pub(super) fn exec_remember_targets_then(
    state: &mut GameState,
    res: &mut Resolution,
    then: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let top_matches = res
        .subject
        .capture_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if top_matches {
        let frame = res.subject.capture_frames.pop().expect("matched above");
        res.subject.remembered_objects = frame.saved_remembered;
        state.resolution_remembered = res
            .subject
            .remembered_objects
            .as_ref()
            .map(|r| (res.source, Arc::clone(r)));
        return None;
    }
    if then.is_empty() {
        return None;
    }
    state.refresh_characteristics();
    let captured_vec: Vec<RememberedObject> = res
        .targets
        .iter()
        .filter_map(|&id| {
            let obj = state.object_or_departed(id)?;
            Some(RememberedObject {
                reference: DamageSourceRef {
                    object: id,
                    version: obj.version,
                },
                lki: RememberedLki::from_object(state, obj),
            })
        })
        .collect();
    let captured: Arc<[RememberedObject]> = Arc::from(captured_vec);
    let saved_remembered = res
        .subject
        .remembered_objects
        .replace(Arc::clone(&captured));
    state.resolution_remembered = Some((res.source, captured));
    res.subject.capture_frames.push(CaptureFrame {
        tail_from_end: after_len,
        saved_remembered,
        phase: CapturePhase::RunningThen,
    });
    let mut body = flatten(then);
    body.push(op);
    let next = res.pc + 1;
    res.effects.splice(next..next, body);
    None
}

fn remembered_item_matches_filter(
    filter: &'static Filter,
    state: &GameState,
    res: &Resolution,
    rem: &RememberedObject,
) -> bool {
    if matches!(filter, Filter::Any) {
        return true;
    }
    let you = res.controller;
    if subjects::is_current(state, rem.reference)
        && let Some(obj) = state.object(rem.reference.object)
    {
        return matches_in_resolution(filter, state, obj, you, res, None);
    }
    let Some(obj) = state.object_or_departed(rem.reference.object) else {
        return false;
    };
    let mut chars = obj.characteristics().clone();
    chars.name = rem.lki.name;
    chars.types = rem.lki.card_types;
    chars.power = rem.lki.power;
    chars.toughness = rem.lki.toughness;
    match filter {
        Filter::ControlledByYou => rem.lki.controller == you,
        Filter::ControlledByOpponent => state.is_opponent(rem.lki.controller, you),
        Filter::ControlledBy(rel) => {
            super::players_of(*rel, state, you, res).contains(&rem.lki.controller)
        }
        _ => matches_in_resolution_with_chars(filter, state, obj, &chars, you, res, None),
    }
}

/// Entry and step-advance handler for [`Effect::ForEachRemembered`].
pub(super) fn exec_for_each_remembered(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let resuming = res
        .subject
        .repeat_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if !resuming {
        if effects.is_empty() {
            return None;
        }
        let list = res.subject.remembered_objects.clone()?;
        let cap = super::immediate::REPEAT_CAP as usize;
        let mut steps: Vec<super::wave3_ops::RepeatStep> = list
            .iter()
            .copied()
            .filter(|rem| remembered_item_matches_filter(filter, state, res, rem))
            .take(cap)
            .map(super::wave3_ops::RepeatStep::Remembered)
            .collect();
        if steps.is_empty() {
            return None;
        }
        steps.reverse();
        res.subject
            .repeat_frames
            .push(super::wave3_ops::RepeatFrame {
                tail_from_end: after_len,
                steps: super::wave3_ops::RepeatSteps::Items(steps),
                saved_event_object: res.event_object,
                saved_event_identity: res.subject.event_identity(),
                saved_event_player: res.subject.event_player(),
            });
    }
    super::wave3_ops::advance_repeat_frame(state, res, effects, op)
}

/// Evaluate one of the four `Remembered` [`Filter`] variants against `obj` / `chars`.
#[must_use]
pub(crate) fn matches_remembered_filter(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &Characteristics,
    remembered: Option<&[RememberedObject]>,
) -> bool {
    let is_rem = !obj.status.contains(crate::object::Status::PHASED_OUT)
        && remembered.is_some_and(|list| {
            list.iter()
                .any(|r| r.reference.object == obj.id && r.reference.version == obj.version)
        });
    match filter {
        Filter::IsRemembered => is_rem,
        Filter::NotRemembered => !is_rem,
        Filter::SharesCardTypeWithRemembered => {
            let my_types = chars.types.intersection(PAPER_CARD_TYPES);
            !my_types.is_empty()
                && remembered.is_some_and(|list| {
                    list.iter()
                        .any(|r| r.lki.card_types.intersection(my_types) != TypeSet::EMPTY)
                })
        }
        Filter::SharesNameWithRemembered => {
            !state.names.get(chars.name).is_empty()
                && remembered.is_some_and(|list| {
                    list.iter().any(|r| {
                        r.lki.name == chars.name && !state.names.get(r.lki.name).is_empty()
                    })
                })
        }
        _ => unreachable!("non-remembered filter routed to matches_remembered_filter"),
    }
}

/// Evaluate one of the four `Remembered*` [`Amount`] variants (`CR 107.1b`, `CR 608.2h`).
#[must_use]
pub(crate) fn remembered_amount(amount: &Amount, remembered: Option<&[RememberedObject]>) -> u32 {
    let Some(list) = remembered else {
        return 0;
    };
    match amount {
        Amount::RememberedCount => u32::try_from(list.len()).unwrap_or(u32::MAX),
        Amount::RememberedManaValue => list
            .iter()
            .fold(0_u32, |acc, r| acc.saturating_add(r.lki.mana_value)),
        Amount::RememberedPower => {
            let sum: i64 = list
                .iter()
                .map(|r| i64::from(r.lki.power.unwrap_or(0)))
                .sum();
            u32::try_from(sum.max(0)).unwrap_or(u32::MAX)
        }
        Amount::RememberedToughness => {
            let sum: i64 = list
                .iter()
                .map(|r| i64::from(r.lki.toughness.unwrap_or(0)))
                .sum();
            u32::try_from(sum.max(0)).unwrap_or(u32::MAX)
        }
        _ => 0,
    }
}

/// Deduplicated seats of [`baylee_cards_dsl::PlayerRel::RememberedController`]
/// still in the game (`CR 608.2h`, `CR 800.4a`).
#[must_use]
pub(crate) fn remembered_controllers(
    state: &GameState,
    remembered: Option<&[RememberedObject]>,
) -> Vec<PlayerId> {
    let Some(list) = remembered else {
        return Vec::new();
    };
    let mut out: Vec<PlayerId> = Vec::new();
    for r in list {
        let p = r.lki.controller;
        if !state.has_left(p) && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}
