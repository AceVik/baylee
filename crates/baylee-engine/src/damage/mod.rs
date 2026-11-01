//! One simultaneous damage event: collect, replace/prevent, then commit.
//!
//! No damage result is written until every affected player's choices have
//! finished (CR 120.4, 510.2, 615.7, 616.1). Parts retain their identity and
//! application history through redirection; new recipients can therefore
//! choose their own effects without allowing a redirect to loop (CR 614.5).

use crate::choice::{
    DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView, Pending, PlayerAction,
};
use crate::event::DamageTarget;
use crate::prevention::{ShieldKind, ShieldOrigin};
use crate::state::GameState;
use baylee_cards_dsl::{CounterKind, KeywordSet};
use baylee_core::ids::{EffectId, ObjectId, PlayerId};

mod batch57;
mod candidates;
mod commit;
mod counter_result;
mod ice_combat;
mod life_result;
#[cfg(test)]
mod prevention_tests;
mod recipient_lock;
#[cfg(test)]
mod tests;

/// Damage assigned before any replacement or prevention is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Assignment {
    pub source: ObjectId,
    pub source_version: Option<u32>,
    pub recipient: DamageTarget,
    pub amount: u32,
    pub is_combat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EffectKey {
    FaceUp(baylee_core::ids::DamageSourceRef),
    ShieldCounter(baylee_core::ids::DamageSourceRef),
    Shield(u64),
    Continuous(EffectId),
    MayRedirect(EffectId),
    Paid,
}

/// The instruction that caused damage, independent of its damage source.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DamageCause {
    pub object: baylee_core::ids::DamageSourceRef,
    pub targets: Vec<baylee_core::ids::DamageSourceRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Part {
    view: DamagePartView,
    cause: Option<DamageCause>,
    source_version: Option<u32>,
    recipient_version: Option<u32>,
    keywords: KeywordSet,
    controller: PlayerId,
    applied: Vec<EffectKey>,
    // Per-point prevention can apply once to each point of unpreventable
    // damage. Such points remain present, so count those applications.
    counters_applied: Vec<(EffectKey, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Candidate {
    key: EffectKey,
    option: DamageEffectOption,
    recipient: DamageTarget,
}

/// A damage event suspended only on its own replacement decisions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DamageWork {
    id: u64,
    step: u64,
    parts: Vec<Part>,
    option_keys: Vec<(EffectKey, DamageTarget)>,
    question: Option<PendingDamage>,
    paid: Option<(u32, ShieldOrigin, PlayerId)>,
    recipient_lock: Option<recipient_lock::RecipientLock>,
    life_gains: Vec<(PlayerId, u32)>,
    consumed_shields: Vec<u64>,
    counter_removals: Vec<(ObjectId, u32, CounterKind, u32)>,
    result_plans: Option<Vec<crate::replacement::counter_order::CounterPlacementPlan>>,
    result_question: Option<usize>,
    complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum PendingDamage {
    Effect(Vec<Candidate>),
    Allocation { candidate: Candidate, total: u32 },
    OptionalRedirect { candidate: Candidate },
}

pub(super) fn recipient_forbids_prevention_or_redirection(
    state: &GameState,
    recipient: DamageTarget,
) -> bool {
    let DamageTarget::Object(id) = recipient else {
        return false;
    };
    let Some(obj) = state.object(id) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::DamageToItCantBePreventedOrRedirected
        ) && crate::effects::applies_to(state, fx, obj)
    })
}

impl DamageWork {
    pub(crate) fn is_waiting_yes_no(&self) -> bool {
        self.result_question.is_some()
            || matches!(self.question, Some(PendingDamage::OptionalRedirect { .. }))
    }

    pub(crate) fn new(state: &mut GameState, assignments: Vec<Assignment>) -> Self {
        let id = state.next_damage_batch;
        state.next_damage_batch = id.checked_add(1).expect("damage identity exhausted");
        let parts = assignments
            .into_iter()
            .filter(|a| a.amount > 0)
            .enumerate()
            .map(|(index, a)| {
                let object = state.damage_source(a.source, a.source_version);
                let chars = object.map(crate::object::GameObject::characteristics);
                Part {
                    cause: None,
                    view: DamagePartView {
                        id: u32::try_from(index).expect("too many damage parts"),
                        source: a.source,
                        recipient: a.recipient,
                        amount: a.amount,
                        is_combat: a.is_combat,
                        preventable: !crate::combat::unpreventable(state, a.source, a.is_combat)
                            && !recipient_forbids_prevention_or_redirection(state, a.recipient),
                    },
                    source_version: a.source_version.or_else(|| object.map(|o| o.version)),
                    recipient_version: match a.recipient {
                        DamageTarget::Player(_) => None,
                        DamageTarget::Object(id) => state.object(id).map(|o| o.version),
                    },
                    keywords: chars.map_or(KeywordSet::default(), |c| c.keywords),
                    controller: object
                        .map(|o| o.controller)
                        .or_else(|| {
                            state
                                .ltb_controllers
                                .iter()
                                .find(|(id, _)| *id == a.source)
                                .map(|(_, p)| *p)
                        })
                        .unwrap_or(state.turn.active),
                    applied: Vec::new(),
                    counters_applied: Vec::new(),
                }
            })
            .collect();
        Self {
            id,
            step: 0,
            parts,
            option_keys: Vec::new(),
            question: None,
            paid: None,
            recipient_lock: None,
            life_gains: Vec::new(),
            consumed_shields: Vec::new(),
            counter_removals: Vec::new(),
            result_plans: None,
            result_question: None,
            complete: false,
        }
    }

    pub(crate) fn with_recipient_lock(
        mut self,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
        context: crate::text_changes::RuleContext,
    ) -> Self {
        self.recipient_lock = Some(recipient_lock::RecipientLock {
            filter,
            you,
            context,
        });
        self
    }

    fn recipient_locked(&self, state: &GameState, recipient: DamageTarget) -> bool {
        recipient_forbids_prevention_or_redirection(state, recipient)
            || self
                .recipient_lock
                .is_some_and(|lock| lock.matches(state, recipient))
    }

    /// Add prevention scoped to this event's lineage, including redirected
    /// descendants. It cannot leak into another instruction's damage.
    pub(crate) fn with_paid_prevention(
        mut self,
        amount: u32,
        origin: ShieldOrigin,
        player: PlayerId,
    ) -> Self {
        if amount > 0 {
            self.paid = Some((amount, origin, player));
        }
        self
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(self)
    }

    fn choice_id(&self) -> DamageChoiceId {
        DamageChoiceId {
            batch: self.id,
            step: self.step,
        }
    }

    fn views(&self, ids: &[u32]) -> Vec<DamagePartView> {
        self.parts
            .iter()
            .filter(|p| ids.contains(&p.view.id))
            .map(|p| p.view.clone())
            .collect()
    }

    /// Continue until a meaningful player decision, or until damage is dealt.
    pub(crate) fn advance(&mut self, state: &mut GameState) -> Option<Pending> {
        if self.complete {
            return None;
        }
        if self.result_plans.is_none() {
            if let Some(pending) = self.prepare(state) {
                return Some(pending);
            }
            self.plan_counter_results(state);
        }
        if let Some(pending) = self.prepare_counter_results(state) {
            return Some(pending);
        }
        self.commit(state);
        self.complete = true;
        None
    }

    fn prepare(&mut self, state: &mut GameState) -> Option<Pending> {
        loop {
            state.refresh_characteristics();
            for part in &mut self.parts {
                part.view.preventable =
                    !crate::combat::unpreventable(state, part.view.source, part.view.is_combat)
                        && !recipient_forbids_prevention_or_redirection(state, part.view.recipient)
                        && !self
                            .recipient_lock
                            .is_some_and(|lock| lock.matches(state, part.view.recipient));
            }
            let mut options = self.candidates(state);
            if options.is_empty() {
                return None;
            }
            // CR 616.1 and 101.4d: restart APNAP if a later player's choice
            // makes an earlier player's effect applicable again.
            let first = options
                .iter()
                .map(|c| Self::chooser(state, c.recipient))
                .min_by_key(|p| Self::apnap(state, *p))
                .expect("nonempty effects");
            options.retain(|c| Self::chooser(state, c.recipient) == first);
            if options.iter().any(|c| c.key != options[0].key) {
                self.step += 1;
                let mut ids: Vec<u32> = options
                    .iter()
                    .flat_map(|c| c.option.parts.iter().copied())
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                let pending = Pending::ChooseDamageEffect {
                    player: first,
                    choice: self.choice_id(),
                    damage: self.views(&ids),
                    options: options.iter().map(|c| c.option.clone()).collect(),
                };
                self.question = Some(PendingDamage::Effect(options));
                return Some(pending);
            }
            let candidate = options.pop().expect("one effect");
            if let Some(pending) = self.select(state, candidate, true) {
                return Some(pending);
            }
        }
    }

    /// The answer has already passed `Pending::answer_fault`, including the
    /// complete allocation and exact decision identity, before mutation.
    pub(crate) fn answer(
        &mut self,
        state: &mut GameState,
        answer: &PlayerAction,
    ) -> Option<Pending> {
        if let Some(index) = self.result_question.take() {
            let PlayerAction::YesNo(apply) = answer else {
                unreachable!("validated counter result answer")
            };
            self.result_plans.as_mut().expect("results planned")[index].answer(*apply);
            return self.advance(state);
        }
        let question = self.question.take().expect("damage decision waiting");
        match (question, answer) {
            (PendingDamage::Effect(options), PlayerAction::ChooseDamageEffect { effect, .. }) => {
                let candidate = options
                    .into_iter()
                    .find(|c| c.option.id == *effect)
                    .expect("offered effect");
                if let Some(pending) = self.select(state, candidate, false) {
                    return Some(pending);
                }
            }
            (
                PendingDamage::Allocation { candidate, .. },
                PlayerAction::AllocatePrevention { allocation, .. },
            ) => {
                self.apply_allocated(state, &candidate, allocation);
            }
            (PendingDamage::OptionalRedirect { candidate }, PlayerAction::YesNo(apply)) => {
                if *apply {
                    self.apply_effect(state, &candidate);
                } else {
                    for part in &mut self.parts {
                        if candidate.option.parts.contains(&part.view.id) {
                            part.applied.push(candidate.key);
                        }
                    }
                }
            }
            _ => unreachable!("validated damage answer"),
        }
        self.advance(state)
    }

    /// Concession can remove recipients or change a permanent's controller.
    /// No unperformed choice is retained across that changed event.
    pub(crate) fn refresh(&mut self, state: &mut GameState) -> Option<Pending> {
        self.question = None;
        self.result_question = None;
        self.advance(state)
    }

    fn chooser(state: &GameState, recipient: DamageTarget) -> PlayerId {
        match recipient {
            DamageTarget::Player(player) => player,
            DamageTarget::Object(id) => {
                state.object(id).map_or(state.turn.active, |o| o.controller)
            }
        }
    }

    fn apnap(state: &GameState, player: PlayerId) -> usize {
        (usize::from(player.get()) + state.players.len() - usize::from(state.turn.active.get()))
            % state.players.len()
    }

    fn select(
        &mut self,
        state: &mut GameState,
        candidate: Candidate,
        forced: bool,
    ) -> Option<Pending> {
        if matches!(candidate.key, EffectKey::MayRedirect(_)) && forced {
            self.step += 1;
            let player = candidate.option.controller;
            self.question = Some(PendingDamage::OptionalRedirect { candidate });
            return Some(Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::MayDo,
                source: None,
            });
        }
        let budget = match candidate.option.kind {
            DamageEffectKind::PreventNext { remaining }
            | DamageEffectKind::PreventThisEvent { remaining }
            | DamageEffectKind::RedirectNext { remaining, .. } => Some(remaining),
            DamageEffectKind::RemoveCounter { remaining, .. } => {
                Some(if forced { remaining } else { remaining.min(1) })
            }
            _ => None,
        };
        if let Some(budget) = budget {
            let damage: Vec<_> = self
                .views(&candidate.option.parts)
                .into_iter()
                .filter(|p| {
                    p.preventable
                        || matches!(
                            candidate.option.kind,
                            DamageEffectKind::RemoveCounter { .. }
                                | DamageEffectKind::RedirectNext { .. }
                        )
                })
                .map(|mut p| {
                    if !p.preventable
                        && matches!(
                            candidate.option.kind,
                            DamageEffectKind::RemoveCounter { .. }
                        )
                    {
                        let applied = self
                            .parts
                            .iter()
                            .find(|part| part.view.id == p.id)
                            .and_then(|part| {
                                part.counters_applied
                                    .iter()
                                    .find(|(key, _)| *key == candidate.key)
                            })
                            .map_or(0, |(_, n)| *n);
                        p.amount = p.amount.saturating_sub(applied);
                    }
                    p
                })
                .collect();
            let sum: u64 = damage.iter().map(|p| u64::from(p.amount)).sum();
            let total = u32::try_from(sum.min(u64::from(budget))).expect("bounded by shield");
            if damage.len() > 1 && total > 0 && u64::from(total) < sum {
                self.step += 1;
                let pending = Pending::AllocatePrevention {
                    player: Self::chooser(state, candidate.recipient),
                    choice: self.choice_id(),
                    effect: candidate.option.clone(),
                    damage,
                    total,
                };
                self.question = Some(PendingDamage::Allocation { candidate, total });
                return Some(pending);
            }
            let mut left = total;
            let allocation: Vec<_> = damage
                .iter()
                .map(|part| {
                    let share = left.min(part.amount);
                    left -= share;
                    (part.id, share)
                })
                .collect();
            self.apply_allocated(state, &candidate, &allocation);
        } else {
            self.apply_effect(state, &candidate);
        }
        None
    }

    fn apply_allocated(
        &mut self,
        state: &mut GameState,
        candidate: &Candidate,
        allocation: &[(u32, u32)],
    ) {
        if let DamageEffectKind::RedirectNext { to, .. } = candidate.option.kind {
            self.redirect_allocated(state, candidate, allocation, to);
            return;
        }
        if let DamageEffectKind::RemoveCounter { kind, .. } = candidate.option.kind {
            let DamageTarget::Object(target) = candidate.recipient else {
                unreachable!("counter recipient")
            };
            let version = state.object(target).expect("counter recipient").version;
            let mut remaining = self.counter_remaining(state, target, kind);
            let mut reserved = 0;
            for part in &mut self.parts {
                let Some(&(_, n)) = allocation.iter().find(|(id, _)| *id == part.view.id) else {
                    continue;
                };
                if n == 0 {
                    continue;
                }
                let taken = n.min(remaining);
                remaining -= taken;
                reserved += taken;
                if part.view.preventable {
                    part.view.amount -= taken;
                } else if let Some((_, applied)) = part
                    .counters_applied
                    .iter_mut()
                    .find(|(key, _)| *key == candidate.key)
                {
                    *applied += taken;
                } else {
                    part.counters_applied.push((candidate.key, taken));
                }
            }
            if let Some((_, _, _, n)) = self
                .counter_removals
                .iter_mut()
                .find(|(id, v, k, _)| *id == target && *v == version && *k == kind)
            {
                *n += reserved;
            } else {
                self.counter_removals
                    .push((target, version, kind, reserved));
            }
            return;
        }
        let mut prevented = 0_u32;
        for part in &mut self.parts {
            if !candidate.option.parts.contains(&part.view.id) {
                continue;
            }
            let n = allocation
                .iter()
                .find(|(id, _)| *id == part.view.id)
                .map_or(0, |(_, n)| *n);
            part.view.amount -= n;
            prevented += n;
            part.applied.push(candidate.key);
        }
        match candidate.key {
            EffectKey::Paid => {
                if let Some((left, _, _)) = &mut self.paid {
                    *left -= prevented;
                }
            }
            EffectKey::Shield(id) => {
                if let Some(i) = state.shields.position(id) {
                    let ShieldKind::Next(n) = state.shields[i].kind else {
                        unreachable!("finite shield")
                    };
                    if n == prevented {
                        state.shields.remove(i);
                    } else {
                        state.shields[i].kind = ShieldKind::Next(n - prevented);
                    }
                }
            }
            EffectKey::Continuous(_)
            | EffectKey::MayRedirect(_)
            | EffectKey::FaceUp(_)
            | EffectKey::ShieldCounter(_) => {
                unreachable!("finite prevention")
            }
        }
    }

    fn redirect_allocated(
        &mut self,
        state: &mut GameState,
        candidate: &Candidate,
        allocation: &[(u32, u32)],
        to: DamageTarget,
    ) {
        let mut redirected = 0_u32;
        let mut split = Vec::new();
        let first_id = self.parts.len();
        for part in &mut self.parts {
            let n = allocation
                .iter()
                .find(|(id, _)| *id == part.view.id)
                .map_or(0, |(_, n)| *n);
            if n == 0 {
                continue;
            }
            let mut moved = part.clone();
            moved.view.id = u32::try_from(first_id + split.len()).expect("damage part count");
            moved.view.amount = n;
            moved.view.recipient = to;
            moved.recipient_version = match to {
                DamageTarget::Player(_) => None,
                DamageTarget::Object(id) => state.object(id).map(|o| o.version),
            };
            moved.applied.push(candidate.key);
            part.view.amount -= n;
            part.applied.push(candidate.key);
            redirected += n;
            split.push(moved);
        }
        self.parts.extend(split);
        if let EffectKey::Shield(id) = candidate.key
            && let Some(i) = state.shields.position(id)
        {
            let ShieldKind::RedirectNext { remaining, to } = state.shields[i].kind else {
                unreachable!("finite redirection shield")
            };
            if remaining == redirected {
                state.shields.remove(i);
            } else {
                state.shields[i].kind = ShieldKind::RedirectNext {
                    remaining: remaining - redirected,
                    to,
                };
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn apply_effect(&mut self, state: &mut GameState, candidate: &Candidate) {
        if self.apply_batch57(state, candidate) {
            return;
        }
        if let DamageEffectKind::TurnFaceUp { object } = candidate.option.kind {
            if state
                .object(object.object)
                .is_some_and(|current| current.version == object.version)
            {
                state.reveal_masked(object.object);
            }
            state.refresh_characteristics();
            for part in &mut self.parts {
                if candidate.option.parts.contains(&part.view.id) {
                    part.applied.push(candidate.key);
                }
                if part.view.source == object.object
                    && part.source_version == Some(object.version)
                    && let Some(source) = state.object(object.object)
                {
                    part.keywords = source.characteristics().keywords;
                    part.controller = source.controller;
                }
            }
            return;
        }

        let mut prevented = 0_u32;
        let mut used = false;
        for part in &mut self.parts {
            if !candidate.option.parts.contains(&part.view.id) {
                continue;
            }
            let preventable =
                !crate::combat::unpreventable(state, part.view.source, part.view.is_combat)
                    && !recipient_forbids_prevention_or_redirection(state, part.view.recipient)
                    && !self
                        .recipient_lock
                        .is_some_and(|lock| lock.matches(state, part.view.recipient));
            match candidate.option.kind {
                DamageEffectKind::Redirect { to } => {
                    part.view.recipient = to;
                    part.recipient_version = match to {
                        DamageTarget::Player(_) => None,
                        DamageTarget::Object(id) => state.object(id).map(|o| o.version),
                    };
                    part.applied.push(candidate.key);
                    used = true;
                }
                DamageEffectKind::TurnFaceUp { .. }
                | DamageEffectKind::ReplaceAmount { .. }
                | DamageEffectKind::Reflect { .. }
                | DamageEffectKind::RemoveShieldCounter => unreachable!("handled first"),
                DamageEffectKind::RemoveCounter { .. } => unreachable!("allocated counters"),
                DamageEffectKind::PreventFromSource { all_but, .. } => {
                    let n = if !preventable {
                        0
                    } else if all_but == u32::from(u8::MAX) {
                        part.view.amount / 2
                    } else {
                        part.view.amount.saturating_sub(all_but)
                    };
                    part.view.amount -= n;
                    prevented = prevented.saturating_add(n);
                    used |= n > 0 || all_but == u32::from(u8::MAX);
                    part.applied.push(candidate.key);
                }
                DamageEffectKind::PreventCombat
                | DamageEffectKind::PreventAll
                | DamageEffectKind::Protection => {
                    if preventable {
                        part.view.amount = 0;
                    }
                    part.applied.push(candidate.key);
                }
                DamageEffectKind::PreventNext { .. }
                | DamageEffectKind::PreventThisEvent { .. }
                | DamageEffectKind::RedirectNext { .. } => unreachable!("allocated shield"),
            }
        }
        if used
            && let EffectKey::Shield(id) = candidate.key
            && let Some(i) = state.shields.position(id)
            && !matches!(
                state.shields[i].kind,
                ShieldKind::RedirectSource { next: false, .. } | ShieldKind::RedirectCombat { .. }
            )
        {
            if state.shields[i].protects == crate::prevention::Shielded::Everything
                && matches!(state.shields[i].kind, ShieldKind::NextFrom { .. })
            {
                if !self.consumed_shields.contains(&id) {
                    self.consumed_shields.push(id);
                }
            } else {
                state.shields.remove(i);
            }
        }
        if let DamageEffectKind::PreventFromSource {
            gain_life: true, ..
        } = candidate.option.kind
            && prevented > 0
        {
            if let Some((_, total)) = self
                .life_gains
                .iter_mut()
                .find(|(p, _)| *p == candidate.option.controller)
            {
                *total = total.saturating_add(prevented);
            } else {
                self.life_gains
                    .push((candidate.option.controller, prevented));
            }
        }
    }

    fn counter_remaining(&self, state: &GameState, id: ObjectId, kind: CounterKind) -> u32 {
        let Some(obj) = state.object(id) else {
            return 0;
        };
        let reserved = self
            .counter_removals
            .iter()
            .filter(|(target, version, k, _)| {
                *target == id && *version == obj.version && *k == kind
            })
            .map(|(_, _, _, n)| *n)
            .fold(0, u32::saturating_add);
        u32::from(obj.counters.get(kind)).saturating_sub(reserved)
    }

    /// Damage actually dealt, after redirection and prevention.
    pub(crate) fn dealt(&self) -> u32 {
        self.parts
            .iter()
            .map(|p| p.view.amount)
            .fold(0, u32::saturating_add)
    }

    pub(crate) fn damaged_players(&self) -> Vec<(ObjectId, PlayerId)> {
        let mut out = Vec::new();
        for part in &self.parts {
            if part.view.amount > 0
                && part.view.is_combat
                && let DamageTarget::Player(player) = part.view.recipient
                && !out.contains(&(part.view.source, player))
            {
                out.push((part.view.source, player));
            }
        }
        out
    }
}
