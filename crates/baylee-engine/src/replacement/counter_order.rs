//! Counter replacement ordering, with no placement committed before its decision.
use crate::{
    choice::{Pending, YesNoPrompt},
    state::GameState,
};
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::{DamageSourceRef, PlayerId};
use std::collections::VecDeque;

/// One additive replacement. Ordinary additions affect every kind in the event;
/// entry additions name exactly the kind printed by their ability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CounterAddition {
    pub(crate) kind: Option<CounterKind>,
    pub(crate) n: u16,
}

impl CounterAddition {
    fn all_kinds(n: u16) -> Self {
        Self { kind: None, n }
    }

    fn applies_to(self, kind: CounterKind, amount: u16) -> bool {
        self.kind
            .map_or(amount > 0, |additional| additional == kind)
    }
}

/// A counter event awaiting replacement ordering; every amount shares its order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CounterPlacementPlan {
    pub(crate) recipient: DamageSourceRef,
    pub(crate) by: PlayerId,
    pub(crate) counters: Vec<(CounterKind, u16)>,
    pub(crate) additions: Vec<CounterAddition>,
    pub(crate) doubles: u16,
    pub(crate) by_effect: bool,
    pub(crate) maximum: Option<u16>,
    next: usize,
    applied_doubling: bool,
}

impl CounterPlacementPlan {
    pub(crate) fn new(
        recipient: DamageSourceRef,
        by: PlayerId,
        kind: CounterKind,
        n: u16,
        additions: Vec<u16>,
        doubles: u16,
        by_effect: bool,
    ) -> Self {
        Self {
            recipient,
            by,
            counters: vec![(kind, n)],
            additions: additions
                .into_iter()
                .map(CounterAddition::all_kinds)
                .collect(),
            doubles,
            by_effect,
            maximum: None,
            next: 0,
            applied_doubling: false,
        }
    }

    fn add(&mut self, index: usize) {
        let addition = self.additions.remove(index);
        for (kind, amount) in &mut self.counters {
            if addition.applies_to(*kind, *amount) {
                *amount = amount.saturating_add(addition.n);
            }
        }
        // CR 614.16: this replacement itself puts counters on the permanent,
        // making effect-only doublers applicable even to an original cost
        // or turn-based counter placement.
        self.by_effect = true;
        self.next = 0;
    }

    fn double(&mut self) {
        self.applied_doubling = true;
        self.doubles -= 1;
        for (_, amount) in &mut self.counters {
            *amount = amount.saturating_mul(2);
        }
        self.next = 0;
    }

    fn applicable_addition(&self, start: usize) -> Option<usize> {
        self.additions
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(index, addition)| {
                self.counters
                    .iter()
                    .any(|&(kind, amount)| addition.applies_to(kind, amount))
                    .then_some(index)
            })
    }

    fn can_double(&self) -> bool {
        self.doubles > 0 && self.by_effect && self.counters.iter().any(|&(_, n)| n > 0)
    }

    /// An entry addition can introduce a new kind. An ordinary addition
    /// affects that kind only if the entry addition was applied first.
    fn additions_commute(&self) -> bool {
        !self
            .additions
            .iter()
            .any(|addition| addition.kind.is_none())
            || !self.additions.iter().any(|addition| {
                addition
                    .kind
                    .is_some_and(|kind| self.counters.contains(&(kind, 0)))
            })
    }

    /// Return the proposed additive replacement, or finish every forced step.
    pub(crate) fn question(&mut self) -> Option<CounterAddition> {
        loop {
            if self.doubles == 0 && self.additions_commute() {
                while let Some(index) = self.applicable_addition(0) {
                    self.add(index);
                }
                return None;
            }
            if self.applicable_addition(0).is_none() {
                if self.can_double() {
                    // Saturating nonnegative multiplication is associative;
                    // an unrepresentable power of two saturates every positive
                    // amount, while a zero amount remains zero.
                    let multiplier = 1u16
                        .checked_shl(u32::from(self.doubles))
                        .unwrap_or(u16::MAX);
                    for (_, amount) in &mut self.counters {
                        *amount = amount.saturating_mul(multiplier);
                    }
                    self.applied_doubling = true;
                    self.doubles = 0;
                    self.next = 0;
                }
                return None;
            }
            // CR 616.1f: reconsider applicability after every replacement.
            // A zero placeholder for Giada is not a counter placement yet.
            let Some(next) = self.applicable_addition(self.next) else {
                debug_assert!(self.can_double(), "last applicable additive is mandatory");
                self.double();
                continue;
            };
            self.next = next;
            if !self.can_double() && self.applicable_addition(self.next + 1).is_none() {
                self.add(self.next);
                continue;
            }
            return Some(self.additions[self.next]);
        }
    }

    pub(crate) fn answer(&mut self, apply: bool) {
        if apply {
            self.add(self.next);
        } else {
            self.next += 1;
        }
    }
}

/// Capture the applicable rules without mutating the recipient or journal.
pub(crate) fn plan(
    state: &GameState,
    by: PlayerId,
    id: baylee_core::ids::ObjectId,
    counters: Vec<(CounterKind, u16)>,
    by_effect: bool,
    maximum: Option<u16>,
) -> Option<CounterPlacementPlan> {
    let counters = counters
        .into_iter()
        .filter(|(kind, _)| super::counter_limits::room(state, id, *kind) != Some(0))
        .collect();
    let object = state.object(id)?;
    build_plan(state, by, object, counters, by_effect, maximum, false)
}

/// Entry replacements read the prospective permanent before its battlefield move.
pub(crate) fn build_plan(
    state: &GameState,
    by: PlayerId,
    object: &crate::object::GameObject,
    counters: Vec<(CounterKind, u16)>,
    by_effect: bool,
    maximum: Option<u16>,
    entering: bool,
) -> Option<CounterPlacementPlan> {
    let mut normalized: Vec<(CounterKind, u16)> = Vec::new();
    for (kind, n) in counters.into_iter().filter(|&(_, n)| n > 0) {
        if let Some((_, amount)) = normalized.iter_mut().find(|(held, _)| *held == kind) {
            *amount = amount.saturating_add(n);
        } else {
            normalized.push((kind, n));
        }
    }
    let recipient = DamageSourceRef {
        object: object.id,
        version: object.version,
    };
    let mut additions: Vec<CounterAddition> = Vec::new();
    for effect in state.effects.iter() {
        if object.zone != crate::zone::Zone::Battlefield
            || !crate::effects::applies_to(state, effect, object)
        {
            continue;
        }
        match effect.modifier {
            baylee_cards_dsl::Modifier::AdditionalCounterPlacement { n } if n > 0 => {
                additions.push(CounterAddition::all_kinds(n));
            }
            baylee_cards_dsl::Modifier::AdditionalCounterOnEntryByCount { kind, count }
                if entering =>
            {
                let count_object = effect
                    .source
                    .and_then(|source| state.object(source))
                    .unwrap_or(object);
                let n = u16::try_from(
                    crate::layers::counts::live_pt_count(
                        state,
                        count_object,
                        effect.controller,
                        count,
                    )
                    .max(0),
                )
                .unwrap_or(u16::MAX);
                if n > 0 {
                    if !normalized.iter().any(|(held, _)| *held == kind) {
                        normalized.push((kind, 0));
                    }
                    additions.push(CounterAddition {
                        kind: Some(kind),
                        n,
                    });
                }
            }
            _ => {}
        }
    }
    if normalized.is_empty() {
        return None;
    }
    let effect_doubles = state
        .replacement_rules
        .iter()
        .filter(|entry| {
            let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter } =
                entry.rule
            else {
                return false;
            };
            crate::eval::matches(object_filter, state, object, entry.controller, entry.source)
        })
        .count();
    let player_doubles = state
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                effect.modifier,
                baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou
            ) && effect.controller == by
                && object.zone == crate::zone::Zone::Battlefield
                && crate::effects::applies_to(state, effect, object)
        })
        .count();
    let doubles = effect_doubles.saturating_add(player_doubles);
    let by_effect = by_effect || player_doubles > 0;
    let mut plan = CounterPlacementPlan::new(
        recipient,
        by,
        normalized[0].0,
        normalized[0].1,
        Vec::new(),
        u16::try_from(doubles).unwrap_or(u16::MAX),
        by_effect,
    );
    plan.counters = normalized;
    plan.additions = additions;
    plan.maximum = maximum;
    Some(plan)
}

/// Commit an entirely ordered event; the caller must finish `question` first.
pub(crate) fn commit_plan(state: &mut GameState, plan: CounterPlacementPlan) {
    if state
        .object(plan.recipient.object)
        .is_none_or(|object| object.version != plan.recipient.version)
    {
        return;
    }
    #[cfg(test)]
    if plan.applied_doubling {
        let object = state
            .object(plan.recipient.object)
            .expect("checked recipient");
        for entry in &state.replacement_rules {
            if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter } =
                entry.rule
                && crate::eval::matches(
                    object_filter,
                    state,
                    object,
                    entry.controller,
                    entry.source,
                )
            {
                crate::ability_log::replaced(entry);
            }
        }
    }
    for (kind, amount) in plan.counters {
        let amount = plan.maximum.map_or(amount, |maximum| {
            amount.min(
                maximum.saturating_sub(
                    state
                        .object(plan.recipient.object)
                        .map_or(0, |object| object.counters.get(kind)),
                ),
            )
        });
        super::counters::commit_counters_by(state, plan.by, plan.recipient.object, kind, amount);
    }
}

/// A simultaneous instruction finishes every replacement decision before writes.
#[derive(Clone, Debug, Hash)]
struct CounterBatch {
    plans: Vec<CounterPlacementPlan>,
    order: Vec<usize>,
    next: usize,
    players: Vec<(PlayerId, CounterKind, i32)>,
}

#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct CounterChoices(VecDeque<CounterBatch>);

/// Freeze one instruction, including player counters that have no ordering choice.
pub(crate) fn enqueue_batch(
    state: &mut GameState,
    plans: Vec<CounterPlacementPlan>,
    players: Vec<(PlayerId, CounterKind, i32)>,
) {
    let mut order: Vec<_> = (0..plans.len()).collect();
    order.sort_by_key(|&index| {
        let controller = state
            .object(plans[index].recipient.object)
            .map_or(state.turn.active, |object| object.controller);
        (usize::from(controller.get()) + state.players.len() - usize::from(state.turn.active.get()))
            % state.players.len()
    });
    state.counter_choices.0.push_back(CounterBatch {
        plans,
        order,
        next: 0,
        players,
    });
    // Forced batches may finish synchronously. A pending decision leaves the
    // entire instruction untouched until the engine presents it.
    let _ = pending(state);
}

pub(crate) fn has_pending(state: &GameState) -> bool {
    !state.counter_choices.0.is_empty()
}

/// Offer decisions in APNAP order; commit only after the whole event is ordered.
pub(crate) fn pending(state: &mut GameState) -> Option<Pending> {
    loop {
        let mut batch = state.counter_choices.0.pop_front()?;
        while let Some(&index) = batch.order.get(batch.next) {
            let plan = &mut batch.plans[index];
            let controller = state
                .object(plan.recipient.object)
                .filter(|object| object.version == plan.recipient.version)
                .map(|object| object.controller);
            if let Some(controller) = controller
                && let Some(additional) = plan.question()
            {
                if state.has_left(controller) {
                    plan.answer(true);
                    continue;
                }
                let permanent = plan.recipient.object;
                state.counter_choices.0.push_front(batch);
                return Some(Pending::YesNo {
                    player: controller,
                    prompt: YesNoPrompt::ApplyAdditionalCountersNext {
                        permanent,
                        additional: additional.n,
                        kind: additional.kind,
                    },
                    source: None,
                });
            }
            batch.next += 1;
        }
        for plan in batch.plans {
            commit_plan(state, plan);
        }
        for (player, kind, amount) in batch.players {
            state.change_player_counters(player, kind, amount);
        }
    }
}

pub(crate) fn answer(state: &mut GameState, apply: bool) {
    let batch = state
        .counter_choices
        .0
        .front_mut()
        .expect("offered counter order");
    batch.plans[batch.order[batch.next]].answer(apply);
}

#[cfg(test)]
mod tests;
