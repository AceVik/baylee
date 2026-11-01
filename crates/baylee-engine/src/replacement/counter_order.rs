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
    kind: Option<CounterKind>,
    n: u16,
}

impl CounterAddition {
    fn all_kinds(n: u16) -> Self {
        Self { kind: None, n }
    }

    fn applies_to(self, kind: CounterKind) -> bool {
        self.kind.is_none_or(|additional| additional == kind)
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
            if addition.applies_to(*kind) {
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

    /// Return the proposed additive replacement, or finish every forced step.
    pub(crate) fn question(&mut self) -> Option<u16> {
        loop {
            if self.doubles == 0 {
                while !self.additions.is_empty() {
                    self.add(0);
                }
                return None;
            }
            if self.additions.is_empty() {
                if self.by_effect {
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
            if !self.by_effect && self.next + 1 == self.additions.len() {
                self.add(self.next);
                continue;
            }
            return Some(self.additions[self.next].n);
        }
    }

    pub(crate) fn answer(&mut self, apply: bool) {
        if apply {
            self.add(self.next);
        } else {
            self.next += 1;
            if self.next == self.additions.len() {
                debug_assert!(self.by_effect, "last initial additive is mandatory");
                self.double();
            }
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
                        additional,
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
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::{
        effects::{ContinuousEffect, EffectFilter, EffectOrigin},
        object::ObjectKind,
        zone::ZoneLocation,
    };
    use baylee_cards_dsl::{Duration, Filter, Modifier, ReplacementRule};
    use baylee_core::ids::{EffectId, ObjectId};
    use std::collections::BTreeSet;

    fn results(mut plan: CounterPlacementPlan) -> BTreeSet<u16> {
        if plan.question().is_none() {
            return BTreeSet::from([plan.counters[0].1]);
        }
        let mut yes = plan.clone();
        yes.answer(true);
        plan.answer(false);
        results(yes).union(&results(plan)).copied().collect()
    }

    fn reference(n: u16, additions: Vec<u16>, doubles: u16, effect: bool) -> BTreeSet<u16> {
        if additions.is_empty() && (doubles == 0 || !effect) {
            return BTreeSet::from([n]);
        }
        let mut out = BTreeSet::new();
        for index in 0..additions.len() {
            let mut rest = additions.clone();
            let extra = rest.remove(index);
            out.extend(reference(n.saturating_add(extra), rest, doubles, true));
        }
        if effect && doubles > 0 {
            out.extend(reference(n.saturating_mul(2), additions, doubles - 1, true));
        }
        out
    }

    #[test]
    fn decision_tree_matches_every_legal_replacement_permutation() {
        for n in 1..=4 {
            for additions in [vec![], vec![1], vec![1, 1], vec![1, 2]] {
                for doubles in 0..=3 {
                    for effect in [false, true] {
                        let plan = CounterPlacementPlan::new(
                            DamageSourceRef {
                                object: ObjectId::new(0, 0),
                                version: 0,
                            },
                            PlayerId::new(0),
                            CounterKind::Charge,
                            n,
                            additions.clone(),
                            doubles,
                            effect,
                        );
                        assert_eq!(
                            results(plan),
                            reference(n, additions.clone(), doubles, effect)
                        );
                    }
                }
            }
        }
    }

    fn setup() -> (GameState, ObjectId) {
        let mut state =
            GameState::from_preset(&preset(571, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(0);
        let name = state.names.intern("counter target");
        let target = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let modifier = Modifier::AdditionalCounterPlacement { n: 1 };
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: player,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::Dsl(&Filter::ControlledByYou),
            modifier,
        });
        state
            .replacement_rules
            .push(crate::state::ReplacementEntry {
                source: target,
                controller: player,
                rule: ReplacementRule::DoubleCounterPlacement {
                    object_filter: &Filter::ControlledByYou,
                },
            });
        (state, target)
    }

    #[test]
    fn placement_waits_for_recipient_choice_and_preserves_placing_player() {
        for (add_first, expected) in [(true, 4), (false, 3)] {
            let (mut state, target) = setup();
            let before = state.journal.last_seq();
            super::super::put_counters_by(
                &mut state,
                PlayerId::new(1),
                target,
                CounterKind::Charge,
                1,
            );
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::Charge),
                0
            );
            assert_eq!(
                state.journal.last_seq(),
                before,
                "no speculative placement event"
            );
            assert!(
                matches!(pending(&mut state),Some(Pending::YesNo {player,..}) if player==PlayerId::new(0))
            );
            answer(&mut state, add_first);
            assert!(pending(&mut state).is_none());
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::Charge),
                expected
            );
            assert!(
                matches!(state.journal.entries().last().unwrap().event,crate::event::GameEvent::CounterChanged {by:Some(player),new,..} if player==PlayerId::new(1) && new==expected)
            );
        }
    }

    #[test]
    fn cost_counters_enable_doubling_only_after_an_additive_replacement() {
        let (mut state, target) = setup();
        super::super::record_counters_by(
            &mut state,
            PlayerId::new(0),
            target,
            CounterKind::Loyalty,
            1,
        );
        assert!(
            !has_pending(&state),
            "single additive must precede the effect-only doubler"
        );
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty),
            4
        );
        let (mut state, target) = setup();
        state.effects.remove_where(|_| true);
        super::super::record_counters_by(
            &mut state,
            PlayerId::new(0),
            target,
            CounterKind::Loyalty,
            1,
        );
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty),
            1
        );
    }

    #[test]
    fn simultaneous_kinds_share_one_order_and_zero_does_not_create_counters() {
        let (mut state, target) = setup();
        super::super::put_counters_by(&mut state, PlayerId::new(0), target, CounterKind::Charge, 0);
        assert!(!has_pending(&state));
        super::super::counters::put_counter_group_by(
            &mut state,
            PlayerId::new(0),
            target,
            vec![(CounterKind::Charge, 1), (CounterKind::Shield, 2)],
        );
        assert!(pending(&mut state).is_some());
        answer(&mut state, true);
        assert!(pending(&mut state).is_none());
        let object = state.object(target).unwrap();
        assert_eq!(object.counters.get(CounterKind::Charge), 4);
        assert_eq!(object.counters.get(CounterKind::Shield), 6);
    }
    #[test]
    fn simultaneous_batch_waits_for_all_apnap_choices_and_player_counters() {
        let (mut state, first) = setup();
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        state.turn.active = p1;
        let name = state.names.intern("second counter target");
        let second = state.create_bare(p1, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let forced = state.create_bare(p0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let make = |id, additions, doubles| {
            CounterPlacementPlan::new(
                DamageSourceRef {
                    object: id,
                    version: state.object(id).unwrap().version,
                },
                p0,
                CounterKind::Charge,
                1,
                additions,
                doubles,
                true,
            )
        };
        let plans = vec![
            make(first, vec![1], 1),
            make(forced, vec![1], 0),
            make(second, vec![1], 1),
        ];
        let before = state.journal.last_seq();
        enqueue_batch(&mut state, plans, vec![(p0, CounterKind::Energy, 1)]);
        assert!(matches!(pending(&mut state), Some(Pending::YesNo {player,..}) if player == p1));
        answer(&mut state, true);
        assert!(matches!(pending(&mut state), Some(Pending::YesNo {player,..}) if player == p0));
        assert_eq!(state.journal.last_seq(), before);
        assert_eq!(state.players[0].energy, 0);
        for id in [first, forced, second] {
            assert_eq!(
                state.object(id).unwrap().counters.get(CounterKind::Charge),
                0
            );
        }
        answer(&mut state, false);
        assert!(pending(&mut state).is_none());
        for (id, expected) in [(first, 3), (forced, 2), (second, 4)] {
            assert_eq!(
                state.object(id).unwrap().counters.get(CounterKind::Charge),
                expected
            );
        }
        assert_eq!(state.players[0].energy, 1);
    }
    #[test]
    fn forced_arithmetic_matches_repeated_operations_at_saturation() {
        for by_effect in [false, true] {
            for additions in [vec![], vec![1, 2, u16::MAX], vec![1; 70_000]] {
                let mut actual = CounterPlacementPlan::new(
                    DamageSourceRef {
                        object: ObjectId::new(0, 0),
                        version: 0,
                    },
                    PlayerId::new(0),
                    CounterKind::Charge,
                    1,
                    additions,
                    0,
                    by_effect,
                );
                actual
                    .counters
                    .extend([(CounterKind::Shield, 0), (CounterKind::P1P1, 32_768)]);
                let mut expected = actual.clone();
                // Independent linear reference performs every individual addition.
                for extra in std::mem::take(&mut expected.additions) {
                    for (kind, amount) in &mut expected.counters {
                        if extra.applies_to(*kind) {
                            *amount = amount.saturating_add(extra.n);
                        }
                    }
                    expected.by_effect = true;
                    expected.next = 0;
                }
                assert_eq!(actual.question(), None);
                assert_eq!(actual, expected);
            }
            for doubles in [0, 1, 2, 15, 16, 17, u16::MAX] {
                let mut actual = CounterPlacementPlan::new(
                    DamageSourceRef {
                        object: ObjectId::new(0, 0),
                        version: 0,
                    },
                    PlayerId::new(0),
                    CounterKind::Charge,
                    1,
                    Vec::new(),
                    doubles,
                    by_effect,
                );
                actual
                    .counters
                    .extend([(CounterKind::Shield, 0), (CounterKind::P1P1, 32_768)]);
                let mut expected = actual.clone();
                if by_effect {
                    while expected.doubles > 0 {
                        expected.double();
                    }
                }
                assert_eq!(actual.question(), None);
                assert_eq!(actual, expected);
            }
        }
    }
}
