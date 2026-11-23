//! Counter replacement ordering, with no placement committed before its decision.
use crate::{
    choice::{Pending, YesNoPrompt},
    state::GameState,
};
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::{DamageSourceRef, PlayerId};
use std::collections::VecDeque;

/// One additive replacement. Ordinary additions affect every kind in the event;
/// entry additions name the kind(s) introduced by their ability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CounterAddition {
    pub(crate) kind: Option<CounterKind>,
    pub(crate) extra_kinds: smallvec::SmallVec<[CounterKind; 2]>,
    pub(crate) n: u16,
}

impl std::hash::Hash for CounterAddition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.n.hash(state);
        if !self.extra_kinds.is_empty() {
            self.extra_kinds.hash(state);
        }
    }
}

/// Question payload returned when two counter replacements do not commute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CounterQuestion {
    pub(crate) kind: Option<CounterKind>,
    pub(crate) n: u16,
}

impl CounterAddition {
    fn all_kinds(n: u16) -> Self {
        Self {
            kind: None,
            extra_kinds: smallvec::SmallVec::new(),
            n,
        }
    }

    pub(crate) fn single(kind: CounterKind, n: u16) -> Self {
        Self {
            kind: Some(kind),
            extra_kinds: smallvec::SmallVec::new(),
            n,
        }
    }

    fn applies_to(&self, kind: CounterKind, amount: u16) -> bool {
        match self.kind {
            None => amount > 0,
            Some(primary) => primary == kind || self.extra_kinds.contains(&kind),
        }
    }

    fn prompt(&self) -> CounterQuestion {
        CounterQuestion {
            kind: if self.extra_kinds.is_empty() {
                self.kind
            } else {
                None
            },
            n: self.n,
        }
    }
}

/// One general counter-modification rule (`ReplacementRule::ModifyCounterPlacement`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CounterModStep {
    pub(crate) kind: Option<CounterKind>,
    pub(crate) effect_only: bool,
    pub(crate) op: baylee_cards_dsl::CounterReplaceOp,
}

impl CounterModStep {
    fn is_additive(self) -> bool {
        matches!(self.op, baylee_cards_dsl::CounterReplaceOp::Add(_))
    }

    fn applies_to(self, kind: CounterKind, amount: u16, by_effect: bool) -> bool {
        (!self.effect_only || by_effect) && amount > 0 && self.kind.is_none_or(|k| k == kind)
    }
}

const APPLIED_DOUBLING: u8 = 1 << 0;
const APPLIED_MOD_RULES: u8 = 1 << 1;
const APPLIED_ENTRY_FROM_SOURCE: u8 = 1 << 2;

/// A counter event awaiting replacement ordering; every amount shares its order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CounterPlacementPlan {
    pub(crate) recipient: DamageSourceRef,
    pub(crate) by: PlayerId,
    pub(crate) counters: Vec<(CounterKind, u16)>,
    pub(crate) additions: Vec<CounterAddition>,
    pub(crate) modifiers: Vec<CounterModStep>,
    pub(crate) doubles: u16,
    pub(crate) by_effect: bool,
    pub(crate) maximum: Option<u16>,
    next: usize,
    applied_flags: u8,
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
            modifiers: Vec::new(),
            doubles,
            by_effect,
            maximum: None,
            next: 0,
            applied_flags: 0,
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
        self.applied_flags |= APPLIED_DOUBLING;
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
                addition.kind.is_some_and(|primary| {
                    std::iter::once(primary)
                        .chain(addition.extra_kinds.iter().copied())
                        .any(|kind| self.counters.contains(&(kind, 0)))
                })
            })
    }

    /// Return the proposed additive replacement, or finish every forced step.
    pub(crate) fn question(&mut self) -> Option<CounterQuestion> {
        if !self.modifiers.is_empty() {
            return self.question_with_modifiers();
        }
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
                    self.applied_flags |= APPLIED_DOUBLING;
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
            return Some(self.additions[self.next].prompt());
        }
    }

    pub(crate) fn answer(&mut self, apply: bool) {
        if !self.modifiers.is_empty() {
            self.answer_with_modifiers(apply);
            return;
        }
        if apply {
            self.add(self.next);
        } else {
            self.next += 1;
        }
    }

    fn apply_mod_step(&mut self, index: usize) {
        use baylee_cards_dsl::CounterReplaceOp as Op;
        let step = self.modifiers.remove(index);
        self.applied_flags |= APPLIED_MOD_RULES;
        for (kind, amount) in &mut self.counters {
            if !step.applies_to(*kind, *amount, self.by_effect) {
                continue;
            }
            match step.op {
                Op::Add(n) => {
                    *amount = amount.saturating_add(n);
                    self.by_effect = true;
                }
                Op::Sub(n) => *amount = amount.saturating_sub(n),
                Op::Multiply(factor) => {
                    *amount = amount.saturating_mul(factor);
                    if factor > 1 {
                        self.by_effect = true;
                    }
                }
                Op::HalveFloor => *amount /= 2,
            }
        }
        self.next = 0;
    }

    fn applicable_mod(&self, start: usize, additive: bool) -> Option<usize> {
        self.modifiers
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(i, step)| {
                (step.is_additive() == additive
                    && self
                        .counters
                        .iter()
                        .any(|&(k, amt)| step.applies_to(k, amt, self.by_effect)))
                .then_some(i)
            })
    }

    fn has_applicable_scale(&self) -> bool {
        self.can_double() || self.applicable_mod(0, false).is_some()
    }

    fn apply_one_scale(&mut self) {
        if let Some(idx) = self.modifiers.iter().position(|step| {
            matches!(step.op, baylee_cards_dsl::CounterReplaceOp::Sub(_))
                && self
                    .counters
                    .iter()
                    .any(|&(k, amt)| step.applies_to(k, amt, self.by_effect))
        }) {
            self.apply_mod_step(idx);
        } else if self.can_double() {
            self.double();
        } else if let Some(idx) = self.modifiers.iter().position(|step| {
            matches!(step.op, baylee_cards_dsl::CounterReplaceOp::Multiply(_))
                && self
                    .counters
                    .iter()
                    .any(|&(k, amt)| step.applies_to(k, amt, self.by_effect))
        }) {
            self.apply_mod_step(idx);
        } else if let Some(idx) = self.applicable_mod(0, false) {
            self.apply_mod_step(idx);
        }
    }

    fn next_additive_slot(&self, start: usize) -> Option<usize> {
        let base_len = self.additions.len();
        if start < base_len
            && let Some(idx) = self.applicable_addition(start)
        {
            return Some(idx);
        }
        let mod_start = start.saturating_sub(base_len);
        self.applicable_mod(mod_start, true)
            .map(|idx| base_len + idx)
    }

    fn apply_additive_slot(&mut self, slot: usize) {
        let base_len = self.additions.len();
        if slot < base_len {
            self.add(slot);
        } else {
            self.apply_mod_step(slot - base_len);
        }
    }

    fn question_with_modifiers(&mut self) -> Option<CounterQuestion> {
        loop {
            let has_scale = self.has_applicable_scale();
            if self.next_additive_slot(0).is_none() {
                while self.has_applicable_scale() {
                    self.apply_one_scale();
                }
                return None;
            }
            if !has_scale && self.additions_commute() {
                while let Some(slot) = self.next_additive_slot(0) {
                    self.apply_additive_slot(slot);
                }
                if !self.has_applicable_scale() {
                    return None;
                }
                continue;
            }
            let Some(slot) = self.next_additive_slot(self.next) else {
                self.apply_one_scale();
                continue;
            };
            self.next = slot;
            if !has_scale && self.next_additive_slot(self.next + 1).is_none() {
                self.apply_additive_slot(self.next);
                continue;
            }
            let base_len = self.additions.len();
            return Some(if slot < base_len {
                self.additions[slot].prompt()
            } else {
                let step = self.modifiers[slot - base_len];
                let n = match step.op {
                    baylee_cards_dsl::CounterReplaceOp::Add(n)
                    | baylee_cards_dsl::CounterReplaceOp::Sub(n) => n,
                    _ => 1,
                };
                CounterQuestion { kind: step.kind, n }
            });
        }
    }

    fn answer_with_modifiers(&mut self, apply: bool) {
        if apply {
            self.apply_additive_slot(self.next);
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

fn collect_replacement_modifiers(
    state: &GameState,
    by: PlayerId,
    object: &crate::object::GameObject,
    additions: &mut Vec<CounterAddition>,
) -> (Vec<CounterModStep>, usize) {
    let mut modifiers = Vec::new();
    let mut effect_doubles = 0usize;
    for entry in &state.replacement_rules {
        match entry.rule {
            baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter } => {
                if crate::eval::matches(
                    object_filter,
                    state,
                    object,
                    entry.controller,
                    entry.source,
                ) {
                    effect_doubles += 1;
                }
            }
            baylee_cards_dsl::ReplacementRule::ModifyCounterPlacement {
                object_filter,
                kind,
                placer,
                effect_only,
                op,
            } => {
                let placer_ok = placer.is_none_or(|rel| {
                    crate::eval::players(rel, state, entry.controller)
                        .is_some_and(|ps| ps.contains(&by))
                });
                if placer_ok
                    && crate::eval::matches(
                        object_filter,
                        state,
                        object,
                        entry.controller,
                        entry.source,
                    )
                {
                    if kind.is_none()
                        && effect_only
                        && op == baylee_cards_dsl::CounterReplaceOp::Multiply(2)
                    {
                        effect_doubles += 1;
                    } else if kind.is_none()
                        && !effect_only
                        && let baylee_cards_dsl::CounterReplaceOp::Add(n) = op
                    {
                        if n > 0 {
                            additions.push(CounterAddition::all_kinds(n));
                        }
                    } else {
                        modifiers.push(CounterModStep {
                            kind,
                            effect_only,
                            op,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    (modifiers, effect_doubles)
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
                    additions.push(CounterAddition::single(kind, n));
                }
            }
            _ => {}
        }
    }
    let entry_from_source = if entering && object.zone == crate::zone::Zone::Battlefield {
        collect_entry_counters_from_sources(state, object, &mut normalized, &mut additions)
    } else {
        false
    };
    if normalized.is_empty() {
        return None;
    }
    let (modifiers, effect_doubles) =
        collect_replacement_modifiers(state, by, object, &mut additions);
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
    plan.modifiers = modifiers;
    plan.maximum = maximum;
    if entry_from_source {
        plan.applied_flags |= APPLIED_ENTRY_FROM_SOURCE;
    }
    Some(plan)
}

fn collect_entry_counters_from_sources(
    state: &GameState,
    object: &crate::object::GameObject,
    normalized: &mut Vec<(CounterKind, u16)>,
    additions: &mut Vec<CounterAddition>,
) -> bool {
    let mut any = false;
    let excluded = crate::entry::unavailable(state);
    for entry in &state.replacement_rules {
        let baylee_cards_dsl::ReplacementRule::EnterWithCountersFromSource { filter } = entry.rule
        else {
            continue;
        };
        if entry.source == object.id || excluded.contains(&entry.source) {
            continue;
        }
        let Some(source_obj) = state.object(entry.source) else {
            continue;
        };
        if source_obj.zone != crate::zone::Zone::Battlefield
            || source_obj
                .status
                .contains(crate::object::Status::PHASED_OUT)
            || source_obj.characteristics().abilities_lost.is_some()
            || source_obj.characteristics().rules_text_lost
        {
            continue;
        }
        if !crate::eval::matches(filter, state, object, entry.controller, entry.source) {
            continue;
        }
        let mut held_kinds = smallvec::SmallVec::<[CounterKind; 4]>::new();
        for (kind, count) in source_obj.counters.iter() {
            if count > 0
                && !held_kinds.contains(&kind)
                && super::counter_limits::room(state, object.id, kind) != Some(0)
            {
                held_kinds.push(kind);
            }
        }
        if let Some((&primary, rest)) = held_kinds.split_first() {
            for &kind in &held_kinds {
                if !normalized.iter().any(|(held, _)| *held == kind) {
                    normalized.push((kind, 0));
                }
            }
            additions.push(CounterAddition {
                kind: Some(primary),
                extra_kinds: rest.iter().copied().collect(),
                n: 1,
            });
            any = true;
        }
    }
    any
}

/// Commit an entirely ordered event; the caller must finish `question` first.
pub(crate) fn commit_plan(state: &mut GameState, plan: CounterPlacementPlan) {
    if state
        .object(plan.recipient.object)
        .is_none_or(|object| object.version != plan.recipient.version)
    {
        return;
    }
    #[cfg(any(test, feature = "test-support"))]
    if plan.applied_flags != 0 {
        let object = state
            .object(plan.recipient.object)
            .expect("checked recipient");
        for entry in &state.replacement_rules {
            match entry.rule {
                baylee_cards_dsl::ReplacementRule::EnterWithCountersFromSource { filter }
                    if (plan.applied_flags & APPLIED_ENTRY_FROM_SOURCE) != 0 =>
                {
                    if entry.source != object.id
                        && crate::eval::matches(
                            filter,
                            state,
                            object,
                            entry.controller,
                            entry.source,
                        )
                    {
                        crate::ability_log::replaced(entry);
                    }
                }
                baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter }
                    if (plan.applied_flags & APPLIED_DOUBLING) != 0 =>
                {
                    if crate::eval::matches(
                        object_filter,
                        state,
                        object,
                        entry.controller,
                        entry.source,
                    ) {
                        crate::ability_log::replaced(entry);
                    }
                }
                baylee_cards_dsl::ReplacementRule::ModifyCounterPlacement {
                    object_filter,
                    placer,
                    ..
                } => {
                    let placer_ok = placer.is_none_or(|rel| {
                        crate::eval::players(rel, state, entry.controller)
                            .is_some_and(|ps| ps.contains(&plan.by))
                    });
                    if placer_ok
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
                _ => {}
            }
        }
    }
    for (kind, amount) in plan.counters {
        if amount == 0 {
            continue;
        }
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
