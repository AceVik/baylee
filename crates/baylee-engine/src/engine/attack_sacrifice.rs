//! The permanents chosen for an attack declaration's sacrifice and enlist costs
//! (CR 508.1g–h, CR 702.154a–d). Choices accumulate before mana, life, or
//! permanents move.

use super::{CardLookup, Engine, EngineError, ObjectId, Pending, PlayerId};
use crate::state::GameState;
use baylee_core::ids::Defender;
use baylee_core::types::TypeSet;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug)]
pub(crate) enum AttackCostKind {
    Sacrifice {
        count: u8,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
    },
    Enlist,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AttackSacrificeCost {
    pub(super) kind: AttackCostKind,
    pub(super) source: ObjectId,
}

impl AttackSacrificeCost {
    pub(super) const fn enlist(attacker: ObjectId) -> Self {
        Self {
            kind: AttackCostKind::Enlist,
            source: attacker,
        }
    }

    pub(super) const fn is_enlist(self) -> bool {
        matches!(self.kind, AttackCostKind::Enlist)
    }

    pub(super) const fn is_empty_sacrifice(self) -> bool {
        matches!(self.kind, AttackCostKind::Sacrifice { count: 0, .. })
    }

    fn fingerprint(self) -> u64 {
        match self.kind {
            AttackCostKind::Sacrifice { count, filter, you } => {
                crate::state::structural_fingerprint(&(
                    count,
                    format!("{filter:?}"),
                    you,
                    self.source,
                ))
            }
            AttackCostKind::Enlist => {
                crate::state::structural_fingerprint(&(1_u8, "enlist", self.source))
            }
        }
    }
}

impl From<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)> for AttackSacrificeCost {
    fn from(
        (count, filter, you, source): (u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId),
    ) -> Self {
        Self {
            kind: AttackCostKind::Sacrifice { count, filter, you },
            source,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AttackSacrificePayment {
    pub(super) cost: AttackSacrificeCost,
    pub(super) objects: Vec<ObjectId>,
}

#[derive(Clone, Debug)]
pub(crate) struct AttackSacrificeWork {
    player: PlayerId,
    attackers: Vec<(ObjectId, Defender)>,
    remaining: VecDeque<AttackSacrificeCost>,
    payments: Vec<AttackSacrificePayment>,
    reserved: BTreeSet<ObjectId>,
    mana: u32,
    life: i32,
    mana_reach: u64,
}

fn options(
    state: &GameState,
    player: PlayerId,
    attackers: &[(ObjectId, Defender)],
    cost: AttackSacrificeCost,
    reserved: &BTreeSet<ObjectId>,
) -> Vec<ObjectId> {
    match cost.kind {
        AttackCostKind::Sacrifice { filter, you, .. } => state
            .battlefield_seen()
            .filter(|&id| {
                !reserved.contains(&id)
                    && state.object(id).is_some_and(|permanent| {
                        permanent.controller == player
                            && state.can_be_sacrificed(id, None)
                            && crate::eval::matches(filter, state, permanent, you, cost.source)
                    })
            })
            .collect(),
        // CR 702.154a, 702.154c: an untapped creature you control that you
        // didn't choose to attack with and that either has haste or has been
        // under your control continuously since this turn began.
        AttackCostKind::Enlist => state
            .battlefield_seen()
            .filter(|&id| {
                !reserved.contains(&id)
                    && !attackers.iter().any(|&(att, _)| att == id)
                    && state.object(id).is_some_and(|permanent| {
                        permanent.controller == player
                            && permanent
                                .characteristics()
                                .types
                                .contains(TypeSet::CREATURE)
                            && !permanent.status.contains(crate::object::Status::TAPPED)
                            && !crate::combat::summoning_sick(state, permanent)
                    })
            })
            .collect(),
    }
}

/// Whether mandatory `costs` can all still be paid from what `reserved` leaves,
/// each permanent once (CR 508.1h). Optional enlist costs (`min: 0`) never make
/// a declaration unpayable.
fn payable<'a>(
    state: &GameState,
    player: PlayerId,
    attackers: &[(ObjectId, Defender)],
    costs: impl IntoIterator<Item = &'a AttackSacrificeCost>,
    reserved: &BTreeSet<ObjectId>,
) -> bool {
    let menus: Vec<Vec<ObjectId>> = costs
        .into_iter()
        .filter_map(|cost| match cost.kind {
            AttackCostKind::Sacrifice { count, .. } => Some((
                options(state, player, attackers, *cost, reserved),
                usize::from(count),
            )),
            AttackCostKind::Enlist => None,
        })
        .flat_map(|(menu, count)| std::iter::repeat_n(menu, count))
        .collect();
    crate::casting::distinct_answers_exist(&menus)
}

/// What the front cost may be paid with: only permanents that leave the rest of
/// `work` payable, because a question nothing can answer stops the game. The
/// player gets the whole of the choice CR 508.1j gives and none of the dead ends.
fn answers(state: &GameState, work: &AttackSacrificeWork) -> Vec<ObjectId> {
    let mut owed: Vec<AttackSacrificeCost> = work.remaining.iter().copied().collect();
    let front = owed[0];
    match &mut owed[0].kind {
        AttackCostKind::Sacrifice { count, .. } => *count -= 1,
        AttackCostKind::Enlist => {
            owed.remove(0);
        }
    }
    options(state, work.player, &work.attackers, front, &work.reserved)
        .into_iter()
        .filter(|id| {
            let mut reserved = work.reserved.clone();
            reserved.insert(*id);
            payable(state, work.player, &work.attackers, &owed, &reserved)
        })
        .collect()
}

pub(crate) fn valid_payments(
    state: &GameState,
    player: PlayerId,
    attackers: &[(ObjectId, Defender)],
    payments: &[AttackSacrificePayment],
) -> bool {
    let mut reserved = BTreeSet::new();
    payments.iter().all(|payment| {
        let legal = options(state, player, attackers, payment.cost, &reserved);
        let count_ok = match payment.cost.kind {
            AttackCostKind::Sacrifice { count, .. } => payment.objects.len() == usize::from(count),
            AttackCostKind::Enlist => payment.objects.len() <= 1,
        };
        count_ok
            && payment
                .objects
                .iter()
                .all(|&id| legal.contains(&id) && reserved.insert(id))
    })
}

pub(crate) fn payments_fingerprint(payments: &[AttackSacrificePayment]) -> u64 {
    let mut fingerprint = 0_u64;
    for payment in payments {
        fingerprint = fingerprint
            .wrapping_mul(31)
            .wrapping_add(payment.cost.fingerprint())
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&payment.objects));
    }
    fingerprint
}

impl AttackSacrificeWork {
    pub(super) fn fingerprint(&self) -> u64 {
        let mut fingerprint =
            crate::state::structural_fingerprint(&(&self.player, &self.attackers));
        for cost in &self.remaining {
            fingerprint = fingerprint
                .wrapping_mul(31)
                .wrapping_add(cost.fingerprint());
        }
        fingerprint
            .wrapping_mul(31)
            .wrapping_add(payments_fingerprint(&self.payments))
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&(
                self.mana,
                self.life,
                self.mana_reach,
            )))
    }
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Starts the choices after determining the complete costs. No attacker
    /// is declared and no cost is paid until every choice has been made.
    pub(super) fn ask_attack_sacrifices(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, Defender)>,
        costs: Vec<AttackSacrificeCost>,
        mana: u32,
        life: i32,
        mana_reach: u64,
    ) -> Result<(), EngineError> {
        if !payable(&self.state, player, &attackers, &costs, &BTreeSet::new()) {
            return Err(EngineError::IllegalAction(
                "cannot pay sacrifice cost required to declare those attackers",
            ));
        }
        let work = AttackSacrificeWork {
            player,
            attackers,
            remaining: costs.into(),
            payments: Vec::new(),
            reserved: BTreeSet::new(),
            mana,
            life,
            mana_reach,
        };
        self.advance_attack_costs(work)
    }

    /// Another player left while a sacrifice was being chosen, and what they
    /// owned went with them (CR 800.4a). The question is asked again from what
    /// remains; if the costs can no longer be paid the declaration is illegal,
    /// and with nothing paid it is simply made again (CR 508.1). Returns whether
    /// a declaration was waiting.
    pub(super) fn refresh_attack_sacrifice(&mut self) -> bool {
        let Some(work) = self.attack_sacrifice.take() else {
            return false;
        };
        if valid_payments(&self.state, work.player, &work.attackers, &work.payments)
            && payable(
                &self.state,
                work.player,
                &work.attackers,
                &work.remaining,
                &work.reserved,
            )
            && self.advance_attack_costs(work).is_ok()
        {
            return true;
        }
        self.attack_sacrifice = None;
        self.awaiting_answer = false;
        self.run_until_choice();
        true
    }

    fn advance_attack_costs(&mut self, mut work: AttackSacrificeWork) -> Result<(), EngineError> {
        while let Some(&front) = work.remaining.front() {
            if front.is_enlist()
                && (work.reserved.contains(&front.source)
                    || options(
                        &self.state,
                        work.player,
                        &work.attackers,
                        front,
                        &work.reserved,
                    )
                    .is_empty())
            {
                work.remaining.pop_front();
                continue;
            }
            break;
        }
        if work.remaining.is_empty() {
            return self.finish_attack_declaration(
                work.player,
                work.attackers,
                work.payments,
                work.mana,
                work.life,
                work.mana_reach,
            );
        }
        let front = *work.remaining.front().expect("a remaining attack cost");
        let options = answers(&self.state, &work);
        let (min, max, prompt) = match front.kind {
            AttackCostKind::Sacrifice { count, .. } => {
                (count, count, crate::choice::ChoicePrompt::CostSacrifice)
            }
            AttackCostKind::Enlist => (0, 1, crate::choice::ChoicePrompt::CostTap),
        };
        let player = work.player;
        self.attack_sacrifice = Some(work);
        self.pending = Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            total: None,
        };
        self.awaiting_answer = true;
        Ok(())
    }

    pub(super) fn answer_attack_sacrifice(
        &mut self,
        objects: Vec<ObjectId>,
    ) -> Result<(), EngineError> {
        let Some(mut work) = self.attack_sacrifice.take() else {
            return Err(EngineError::IllegalAction(
                "no attack sacrifice choice is waiting for an answer",
            ));
        };
        let cost = work
            .remaining
            .pop_front()
            .expect("a remaining sacrifice cost");
        let legal = options(
            &self.state,
            work.player,
            &work.attackers,
            cost,
            &work.reserved,
        );
        let count_ok = match cost.kind {
            AttackCostKind::Sacrifice { count, .. } => objects.len() == usize::from(count),
            AttackCostKind::Enlist => objects.len() <= 1,
        };
        let mut picked = BTreeSet::new();
        if !count_ok
            || objects
                .iter()
                .any(|&id| !legal.contains(&id) || !picked.insert(id))
        {
            return Err(EngineError::IllegalAction(
                "those permanents cannot pay this attack cost",
            ));
        }
        work.reserved.extend(objects.iter().copied());
        // Each permanent offered leaves the rest payable on its own; several
        // chosen together may not.
        if !payable(
            &self.state,
            work.player,
            &work.attackers,
            &work.remaining,
            &work.reserved,
        ) {
            return Err(EngineError::IllegalAction(
                "those permanents would leave another attack cost unpayable",
            ));
        }
        work.payments.push(AttackSacrificePayment { cost, objects });
        self.advance_attack_costs(work)
    }
}
