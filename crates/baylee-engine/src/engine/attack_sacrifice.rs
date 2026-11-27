//! The permanents and Phyrexian choices for an attack declaration's costs
//! (CR 508.1g–h, CR 702.154a–d, CR 107.4f). Choices accumulate before mana,
//! life, or permanents move.

use super::{CardLookup, Engine, EngineError, ObjectId, Pending, PlayerId};
use crate::combat::{can_afford_attack_mana_and_phyrexian, mana_symbol_of};
use crate::state::GameState;
use baylee_core::ids::Defender;
use baylee_core::mana::{ManaColor, ManaCost};
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
    ReturnToHand {
        count: u8,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
    },
    Tap {
        count: u8,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
    },
    Phyrexian {
        color: ManaColor,
    },
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

    pub(super) const fn return_to_hand(
        count: u8,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
        source: ObjectId,
    ) -> Self {
        Self {
            kind: AttackCostKind::ReturnToHand { count, filter, you },
            source,
        }
    }

    pub(super) const fn tap(
        count: u8,
        filter: &'static baylee_cards_dsl::Filter,
        you: PlayerId,
        source: ObjectId,
    ) -> Self {
        Self {
            kind: AttackCostKind::Tap { count, filter, you },
            source,
        }
    }

    pub(super) const fn phyrexian(color: ManaColor, source: ObjectId) -> Self {
        Self {
            kind: AttackCostKind::Phyrexian { color },
            source,
        }
    }

    pub(super) const fn is_enlist(self) -> bool {
        matches!(self.kind, AttackCostKind::Enlist)
    }

    pub(super) const fn is_empty_cost(self) -> bool {
        matches!(
            self.kind,
            AttackCostKind::Sacrifice { count: 0, .. }
                | AttackCostKind::ReturnToHand { count: 0, .. }
                | AttackCostKind::Tap { count: 0, .. }
        )
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
            AttackCostKind::ReturnToHand { count, filter, you } => {
                crate::state::structural_fingerprint(&(
                    255_u8,
                    554_u16,
                    1_u8,
                    count,
                    format!("{filter:?}"),
                    you,
                    self.source,
                ))
            }
            AttackCostKind::Tap { count, filter, you } => crate::state::structural_fingerprint(&(
                255_u8,
                554_u16,
                2_u8,
                count,
                format!("{filter:?}"),
                you,
                self.source,
            )),
            AttackCostKind::Phyrexian { color } => {
                crate::state::structural_fingerprint(&(255_u8, 554_u16, 3_u8, color, self.source))
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
    mana: ManaCost,
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
        AttackCostKind::ReturnToHand { filter, you, .. } => state
            .battlefield_seen()
            .filter(|&id| {
                !reserved.contains(&id)
                    && state.object(id).is_some_and(|permanent| {
                        permanent.controller == player
                            && crate::eval::matches(filter, state, permanent, you, cost.source)
                    })
            })
            .collect(),
        // Hollow Warrior: an untapped permanent you control not declared as an
        // attacker matching `filter`.
        AttackCostKind::Tap { filter, you, .. } => state
            .battlefield_seen()
            .filter(|&id| {
                !reserved.contains(&id)
                    && !attackers.iter().any(|&(att, _)| att == id)
                    && state.object(id).is_some_and(|permanent| {
                        permanent.controller == player
                            && !permanent.status.contains(crate::object::Status::TAPPED)
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
        AttackCostKind::Phyrexian { .. } => Vec::new(),
    }
}

fn remaining_phyrexian<'a>(
    costs: impl IntoIterator<Item = &'a AttackSacrificeCost>,
) -> Vec<ManaColor> {
    costs
        .into_iter()
        .filter_map(|cost| match cost.kind {
            AttackCostKind::Phyrexian { color } => Some(color),
            _ => None,
        })
        .collect()
}

/// Whether mandatory permanent-choice `costs` can all still be paid from what
/// `reserved` leaves, each permanent once (CR 508.1h). Optional enlist costs
/// (`min: 0`) never make a declaration unpayable.
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
            AttackCostKind::Sacrifice { count, .. }
            | AttackCostKind::ReturnToHand { count, .. }
            | AttackCostKind::Tap { count, .. } => Some((
                options(state, player, attackers, *cost, reserved),
                usize::from(count),
            )),
            AttackCostKind::Enlist | AttackCostKind::Phyrexian { .. } => None,
        })
        .flat_map(|(menu, count)| std::iter::repeat_n(menu, count))
        .collect();
    crate::casting::distinct_answers_exist(&menus)
}

/// What the front permanent cost may be paid with: only permanents that leave
/// the rest of `work` payable, because a question nothing can answer stops the
/// game. The player gets the whole of the choice CR 508.1j gives and none of
/// the dead ends.
fn answers(state: &GameState, work: &AttackSacrificeWork) -> Vec<ObjectId> {
    let mut owed: Vec<AttackSacrificeCost> = work.remaining.iter().copied().collect();
    let front = owed[0];
    match &mut owed[0].kind {
        AttackCostKind::Sacrifice { count, .. }
        | AttackCostKind::ReturnToHand { count, .. }
        | AttackCostKind::Tap { count, .. } => *count -= 1,
        AttackCostKind::Enlist | AttackCostKind::Phyrexian { .. } => {
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
            AttackCostKind::Sacrifice { count, .. }
            | AttackCostKind::ReturnToHand { count, .. }
            | AttackCostKind::Tap { count, .. } => payment.objects.len() == usize::from(count),
            AttackCostKind::Enlist => payment.objects.len() <= 1,
            AttackCostKind::Phyrexian { .. } => payment.objects.is_empty(),
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
        let pure_generic = ManaCost::ZERO.with_more_generic(self.mana.generic_total());
        let tail = if self.mana == pure_generic {
            crate::state::structural_fingerprint(&(
                self.mana.generic_total(),
                self.life,
                self.mana_reach,
            ))
        } else {
            crate::state::structural_fingerprint(&(
                255_u8,
                554_u16,
                4_u8,
                crate::state::mana_cost_fingerprint(&self.mana),
                self.life,
                self.mana_reach,
            ))
        };
        fingerprint
            .wrapping_mul(31)
            .wrapping_add(payments_fingerprint(&self.payments))
            .wrapping_mul(31)
            .wrapping_add(tail)
    }
}

/// Mandatory tap costs for a blocker declaration (Hollow Warrior, CR 509.1d).
#[derive(Clone, Debug)]
pub(crate) struct BlockTapWork {
    pub(crate) defending: PlayerId,
    pub(crate) blockers: Vec<(ObjectId, ObjectId)>,
    pub(crate) remaining: VecDeque<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)>,
    pub(crate) tapped: Vec<ObjectId>,
    pub(crate) reserved: BTreeSet<ObjectId>,
    pub(crate) window: bool,
}

impl BlockTapWork {
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut fp = crate::state::structural_fingerprint(&(
            255_u8,
            554_u16,
            5_u8,
            self.defending,
            &self.blockers,
            &self.tapped,
            self.window,
        ));
        for &(count, filter, you, source) in &self.remaining {
            fp = fp
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    count,
                    format!("{filter:?}"),
                    you,
                    source,
                )));
        }
        fp
    }
}

pub(crate) fn block_tap_options(
    state: &GameState,
    defending: PlayerId,
    blockers: &[(ObjectId, ObjectId)],
    filter: &'static baylee_cards_dsl::Filter,
    you: PlayerId,
    source: ObjectId,
    reserved: &BTreeSet<ObjectId>,
) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|&id| {
            !reserved.contains(&id)
                && !blockers.iter().any(|&(b, _)| b == id)
                && state.object(id).is_some_and(|permanent| {
                    permanent.controller == defending
                        && !permanent.status.contains(crate::object::Status::TAPPED)
                        && crate::eval::matches(filter, state, permanent, you, source)
                })
        })
        .collect()
}

pub(crate) fn block_tap_payable(
    state: &GameState,
    defending: PlayerId,
    blockers: &[(ObjectId, ObjectId)],
    costs: &[(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)],
    reserved: &BTreeSet<ObjectId>,
) -> bool {
    let menus: Vec<Vec<ObjectId>> = costs
        .iter()
        .flat_map(|&(count, filter, you, source)| {
            let menu = block_tap_options(state, defending, blockers, filter, you, source, reserved);
            std::iter::repeat_n(menu, usize::from(count))
        })
        .collect();
    crate::casting::distinct_answers_exist(&menus)
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Starts the choices after determining the complete costs. No attacker
    /// is declared and no cost is paid until every choice has been made.
    pub(super) fn ask_attack_sacrifices(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, Defender)>,
        costs: Vec<AttackSacrificeCost>,
        mana: ManaCost,
        life: i32,
        mana_reach: u64,
    ) -> Result<(), EngineError> {
        if !payable(&self.state, player, &attackers, &costs, &BTreeSet::new()) {
            return Err(EngineError::IllegalAction(
                "cannot pay permanent cost required to declare those attackers",
            ));
        }
        let phyrexian = remaining_phyrexian(&costs);
        if !can_afford_attack_mana_and_phyrexian(
            &self.state,
            player,
            mana,
            life,
            &phyrexian,
            mana_reach,
        ) {
            return Err(EngineError::IllegalAction(
                "cannot pay mana or life required to declare those attackers",
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

    /// Another player left while an attack cost was being chosen, and what
    /// they owned went with them (CR 800.4a). The question is asked again from
    /// what remains; if the costs can no longer be paid the declaration is
    /// illegal, and with nothing paid it is simply made again (CR 508.1).
    /// Returns whether a declaration was waiting.
    pub(super) fn refresh_attack_sacrifice(&mut self) -> bool {
        let Some(work) = self.attack_sacrifice.take() else {
            return false;
        };
        let phyrexian = remaining_phyrexian(&work.remaining);
        if valid_payments(&self.state, work.player, &work.attackers, &work.payments)
            && payable(
                &self.state,
                work.player,
                &work.attackers,
                &work.remaining,
                &work.reserved,
            )
            && can_afford_attack_mana_and_phyrexian(
                &self.state,
                work.player,
                work.mana,
                work.life,
                &phyrexian,
                work.mana_reach,
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

    #[allow(clippy::too_many_lines)]
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
            if let AttackCostKind::Phyrexian { color } = front.kind {
                let tail_phyrexian = remaining_phyrexian(work.remaining.iter().skip(1));
                let life_ok = can_afford_attack_mana_and_phyrexian(
                    &self.state,
                    work.player,
                    work.mana,
                    work.life.saturating_add(2),
                    &tail_phyrexian,
                    work.mana_reach,
                );
                let mana_with = work.mana.with_symbol(mana_symbol_of(color), 1);
                let mana_ok = can_afford_attack_mana_and_phyrexian(
                    &self.state,
                    work.player,
                    mana_with,
                    work.life,
                    &tail_phyrexian,
                    work.mana_reach,
                );
                match (life_ok, mana_ok) {
                    (false, false) => {
                        return Err(EngineError::IllegalAction(
                            "cannot pay Phyrexian tax required to declare those attackers",
                        ));
                    }
                    (true, false) => {
                        work.life = work.life.saturating_add(2);
                        work.remaining.pop_front();
                        continue;
                    }
                    (false, true) => {
                        work.mana = mana_with;
                        work.remaining.pop_front();
                        continue;
                    }
                    (true, true) => {
                        let player = work.player;
                        self.attack_sacrifice = Some(work);
                        self.pending = Pending::YesNo {
                            player,
                            prompt: crate::choice::YesNoPrompt::PayLife {
                                amount: 2,
                                permanent: None,
                            },
                            source: None,
                        };
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                }
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
            AttackCostKind::ReturnToHand { count, .. } => {
                (count, count, crate::choice::ChoicePrompt::CostReturn)
            }
            AttackCostKind::Tap { count, .. } => {
                (count, count, crate::choice::ChoicePrompt::CostTap)
            }
            AttackCostKind::Enlist => (0, 1, crate::choice::ChoicePrompt::CostTap),
            AttackCostKind::Phyrexian { .. } => {
                unreachable!("Phyrexian attack tax handled in loop above")
            }
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

    pub(super) fn answer_attack_phyrexian(&mut self, pay_life: bool) -> Result<(), EngineError> {
        let Some(mut work) = self.attack_sacrifice.take() else {
            return Err(EngineError::IllegalAction(
                "no attack Phyrexian choice is waiting for an answer",
            ));
        };
        let Some(front) = work.remaining.pop_front() else {
            return Err(EngineError::IllegalAction(
                "no attack Phyrexian choice is waiting for an answer",
            ));
        };
        let AttackCostKind::Phyrexian { color } = front.kind else {
            return Err(EngineError::IllegalAction(
                "attack cost waiting is not a Phyrexian choice",
            ));
        };
        if pay_life {
            work.life = work.life.saturating_add(2);
        } else {
            work.mana = work.mana.with_symbol(mana_symbol_of(color), 1);
        }
        self.advance_attack_costs(work)
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
            AttackCostKind::Sacrifice { count, .. }
            | AttackCostKind::ReturnToHand { count, .. }
            | AttackCostKind::Tap { count, .. } => objects.len() == usize::from(count),
            AttackCostKind::Enlist => objects.len() <= 1,
            AttackCostKind::Phyrexian { .. } => false,
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
