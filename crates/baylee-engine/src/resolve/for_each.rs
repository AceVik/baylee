//! Instructions run once for each of several objects: each permanent a
//! filter matches (`Effect::ForEachMatching`) or each target
//! (`Effect::ForEachTarget`).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// One turn of the iteration: the instructions for the next object are
/// spliced in after `op`, followed by `op` again while objects remain.
pub(super) fn step(
    state: &GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
    op: Effect,
) {
    let you = res.controller;
    let mut remaining = res.subject.for_each_remaining.take().unwrap_or_else(|| {
        let context = res.rule_context();
        if matches!(op, Effect::ForEachTarget { .. }) {
            let mut targets: Vec<_> = res
                .targets
                .iter()
                .filter_map(|id| subjects::target(state, res, *id))
                .collect();
            targets.reverse();
            return targets;
        }
        let Effect::ForEachMatching { filter, .. } = op else {
            unreachable!("iteration instruction");
        };
        let mut matched: Vec<baylee_core::ids::DamageSourceRef> = state
            .battlefield_seen()
            .filter_map(|id| {
                let o = state.object(id)?;
                eval::matches_with_context(filter, state, o, you, context).then_some(
                    baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: o.version,
                    },
                )
            })
            .collect();
        matched.reverse();
        matched
    });
    while let Some(next_ref) = remaining.pop() {
        let Some(obj) = state
            .object(next_ref.object)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == next_ref.version)
        else {
            continue;
        };
        let controller = obj.controller;
        let power = obj.characteristics().power.unwrap_or(0);
        res.event_object = Some(next_ref.object);
        res.subject.set_event_identity(Some((next_ref, power)));
        res.subject.set_event_player(Some(controller));
        let mut spliced = flatten(effects);
        if !remaining.is_empty() {
            res.subject.for_each_remaining = Some(remaining);
            spliced.push(op);
        }
        let at = res.pc + 1;
        res.effects.splice(at..at, spliced);
        return;
    }
}
