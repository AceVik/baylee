//! Counter-derived continuous effects share the layer planner (CR 613.7c).
//!
//! Reuse the layer planner and its object index instead of a second ordering
//! mechanism or additional storage in every game object.
use baylee_cards_dsl::{CounterKind, Duration, Modifier};
use baylee_core::ids::{EffectId, ObjectId};

use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::state::GameState;

pub(super) fn placed(state: &mut GameState, id: ObjectId, kind: CounterKind) {
    let modifier = if kind == CounterKind::Hone {
        Modifier::HoneEquipment
    } else if let Some(keyword) = kind.keyword() {
        Modifier::AddKeyword(keyword)
    } else if let Some((power, toughness)) = kind.power_toughness() {
        // One counter's contribution; the Counter origin reads the current
        // count, so removal preserves this placement's timestamp (613.7c).
        Modifier::ModifyPT(power, toughness)
    } else {
        return;
    };
    let Some(object) = state.object(id).filter(|o| o.counters.get(kind) > 0) else {
        return;
    };
    let version = object.version;
    let controller = object.controller;
    let hone = kind == CounterKind::Hone;
    let filter = if hone {
        EffectFilter::Dsl(&baylee_cards_dsl::Filter::AttachedToBySource)
    } else {
        EffectFilter::ObjectIs(id, version)
    };
    // CR 613.7c updates every counter of this kind when a new one arrives.
    // Removal never refreshes the timestamp.
    state.effects.remove_where(|effect| {
        if effect.origin != EffectOrigin::Counter(kind) {
            return false;
        }
        if hone {
            effect.source == Some(id)
        } else {
            matches!(effect.filter, EffectFilter::ObjectIs(object, v) if object == id && v == version)
        }
    });
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: hone.then_some(id),
        controller,
        origin: EffectOrigin::Counter(kind),
        layer: modifier.layer(),
        timestamp,
        duration: if hone {
            Duration::WhileSourceOnBattlefield
        } else {
            Duration::WhileCounterRemains(kind)
        },
        filter,
        modifier,
    });
}
