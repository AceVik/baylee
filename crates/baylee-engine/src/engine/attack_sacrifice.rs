//! The permanents chosen for an attack declaration's sacrifice costs
//! (CR 508.1h). Choices accumulate before mana, life, or permanents move.

use super::{CardLookup, Engine, EngineError, ObjectId, Pending, PlayerId};
use crate::state::GameState;
use baylee_core::ids::Defender;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AttackSacrificeCost {
    pub(super) count: u8,
    filter: &'static baylee_cards_dsl::Filter,
    you: PlayerId,
    source: ObjectId,
}

impl AttackSacrificeCost {
    fn fingerprint(self) -> u64 {
        crate::state::structural_fingerprint(&(
            self.count,
            format!("{:?}", self.filter),
            self.you,
            self.source,
        ))
    }
}

impl From<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)> for AttackSacrificeCost {
    fn from(
        (count, filter, you, source): (u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId),
    ) -> Self {
        Self {
            count,
            filter,
            you,
            source,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AttackSacrificePayment {
    cost: AttackSacrificeCost,
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
    cost: AttackSacrificeCost,
    reserved: &BTreeSet<ObjectId>,
) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|&id| {
            !reserved.contains(&id)
                && state.object(id).is_some_and(|permanent| {
                    permanent.controller == player
                        && crate::eval::matches(
                            cost.filter,
                            state,
                            permanent,
                            cost.you,
                            cost.source,
                        )
                })
        })
        .collect()
}

/// Whether `costs` can all still be paid from what `reserved` leaves, each
/// permanent once (CR 508.1h). A matching and not a walk down the list: a land
/// spent on the cost any land pays may be the Island a stricter cost needs, and
/// the order costs were determined in says nothing about which pays which.
fn payable<'a>(
    state: &GameState,
    player: PlayerId,
    costs: impl IntoIterator<Item = &'a AttackSacrificeCost>,
    reserved: &BTreeSet<ObjectId>,
) -> bool {
    let menus: Vec<Vec<ObjectId>> = costs
        .into_iter()
        .flat_map(|cost| {
            let menu = options(state, player, *cost, reserved);
            std::iter::repeat_n(menu, usize::from(cost.count))
        })
        .collect();
    crate::casting::distinct_answers_exist(&menus)
}

/// What the front cost may be paid with: only permanents that leave the rest of
/// `work` payable, because a question nothing can answer stops the game. The
/// player gets the whole of the choice CR 508.1j gives and none of the dead ends.
fn answers(state: &GameState, work: &AttackSacrificeWork) -> Vec<ObjectId> {
    let mut owed: Vec<AttackSacrificeCost> = work.remaining.iter().copied().collect();
    let front = owed[0];
    owed[0].count -= 1;
    options(state, work.player, front, &work.reserved)
        .into_iter()
        .filter(|id| {
            let mut reserved = work.reserved.clone();
            reserved.insert(*id);
            payable(state, work.player, &owed, &reserved)
        })
        .collect()
}

pub(crate) fn valid_payments(
    state: &GameState,
    player: PlayerId,
    payments: &[AttackSacrificePayment],
) -> bool {
    let mut reserved = BTreeSet::new();
    payments.iter().all(|payment| {
        let legal = options(state, player, payment.cost, &reserved);
        let valid = payment.objects.len() == usize::from(payment.cost.count)
            && payment.objects.iter().all(|id| legal.contains(id));
        reserved.extend(payment.objects.iter().copied());
        valid
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

impl<L: CardLookup> Engine<L> {
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
        if !payable(&self.state, player, &costs, &BTreeSet::new()) {
            return Err(EngineError::IllegalAction(
                "cannot pay sacrifice cost required to declare those attackers",
            ));
        }
        self.attack_sacrifice = Some(AttackSacrificeWork {
            player,
            attackers,
            remaining: costs.into(),
            payments: Vec::new(),
            reserved: BTreeSet::new(),
            mana,
            life,
            mana_reach,
        });
        self.ask_next_attack_sacrifice();
        Ok(())
    }

    /// Another player left while a sacrifice was being chosen, and what they
    /// owned went with them (CR 800.4a). The question is asked again from what
    /// remains; if the costs can no longer be paid the declaration is illegal,
    /// and with nothing paid it is simply made again (CR 508.1). Returns whether
    /// a declaration was waiting.
    pub(super) fn refresh_attack_sacrifice(&mut self) -> bool {
        let Some(work) = &self.attack_sacrifice else {
            return false;
        };
        if valid_payments(&self.state, work.player, &work.payments)
            && payable(&self.state, work.player, &work.remaining, &work.reserved)
        {
            self.ask_next_attack_sacrifice();
        } else {
            self.attack_sacrifice = None;
            self.awaiting_answer = false;
            self.run_until_choice();
        }
        true
    }

    fn ask_next_attack_sacrifice(&mut self) {
        let work = self
            .attack_sacrifice
            .as_ref()
            .expect("attack cost in progress");
        let count = work
            .remaining
            .front()
            .expect("a remaining sacrifice cost")
            .count;
        self.pending = Pending::ChooseCards {
            player: work.player,
            options: answers(&self.state, work),
            min: count,
            max: count,
            prompt: crate::choice::ChoicePrompt::CostSacrifice,
            total: None,
        };
        self.awaiting_answer = true;
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
        let legal = options(&self.state, work.player, cost, &work.reserved);
        if objects.len() != usize::from(cost.count) || objects.iter().any(|id| !legal.contains(id))
        {
            return Err(EngineError::IllegalAction(
                "those permanents cannot pay this attack sacrifice cost",
            ));
        }
        work.reserved.extend(objects.iter().copied());
        // Each permanent offered leaves the rest payable on its own; several
        // chosen together may not.
        if !payable(&self.state, work.player, &work.remaining, &work.reserved) {
            return Err(EngineError::IllegalAction(
                "those permanents would leave another attack cost unpayable",
            ));
        }
        work.payments.push(AttackSacrificePayment { cost, objects });
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
        self.attack_sacrifice = Some(work);
        self.ask_next_attack_sacrifice();
        Ok(())
    }
}
