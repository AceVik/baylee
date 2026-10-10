//! Dispatch for temporary special actions and their mana-payment windows.
use super::{
    CardLookup, Engine, EngineError, PaymentContinuation, PaymentWindow, Pending, PlayerId,
};
use crate::choice::LegalActions;
use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
use baylee_core::ids::GrantedActionId;

impl<L: CardLookup> Engine<L> {
    pub(super) fn offer_granted_actions(&self, player: PlayerId, legal: &mut LegalActions) {
        let can_make_mana =
            legal.has_mana_source() || self.state.granted_colorless_capacity(player) > 0;
        legal.granted_actions = self
            .state
            .granted_actions
            .iter()
            .filter(|g| {
                g.player == player
                    && (self.state.granted_action_payable(g)
                        || (matches!(g.offer.cost, SpecialActionCost::Mana(_)) && can_make_mana))
            })
            .map(|g| g.offer.clone())
            .collect();
    }

    pub(super) fn start_granted_action(
        &mut self,
        player: PlayerId,
        id: GrantedActionId,
    ) -> Result<(), EngineError> {
        let action = self
            .state
            .granted_actions
            .iter()
            .find(|g| g.player == player && g.offer.id == id)
            .cloned()
            .ok_or(EngineError::IllegalAction("special action expired"))?;
        if self.mana_window.is_some() && action.offer.timing != SpecialActionTiming::ManaAbility {
            return Err(EngineError::IllegalAction(
                "special action requires priority",
            ));
        }
        if self.state.granted_action_payable(&action) {
            if !self.state.take_granted_action(player, id) {
                return Err(EngineError::IllegalAction("special action payment failed"));
            }
            self.after_action(player);
            return Ok(());
        }
        let SpecialActionCost::Mana(cost) = action.offer.cost else {
            return Err(EngineError::IllegalAction("cannot pay special action"));
        };
        if self.mana_window.is_some() {
            return Err(EngineError::IllegalAction("nested special action payment"));
        }
        self.mana_window = Some(PaymentWindow {
            player,
            suspended: PaymentContinuation::GrantedAction { id, cost },
        });
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        if !legal.has_mana_source() {
            self.mana_window = None;
            return Err(EngineError::IllegalAction("cannot make payment mana"));
        }
        self.pending = Pending::Priority {
            player,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
        Ok(())
    }
}

/// Announcement kept apart from any mana ability activated while paying it.
#[derive(Clone, Debug)]
pub(super) struct ActivationPayment {
    source: baylee_core::ids::DamageSourceRef,
    ability_index: u32,
    targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    pub(super) cost: baylee_core::mana::ManaCost,
    players: Vec<PlayerId>,
    choices: Vec<baylee_core::ids::ObjectId>,
    second: Option<smallvec::SmallVec<[baylee_core::ids::ObjectId; 1]>>,
    references: crate::sources::TargetReferences,
    answered: bool,
    x: Option<u32>,
    graveyard: Option<PlayerId>,
    phyrexian: Vec<bool>,
    abilities: Option<(baylee_core::ids::ObjectId, crate::object::AbilityList)>,
    pub(super) previous: Option<Box<PaymentWindow>>,
    /// Where the window began, to give back what it made if the
    /// activation is not completed (CR 732.1).
    opened: Box<super::WindowStart>,
}
impl ActivationPayment {
    pub(super) fn fingerprint(&self) -> u64 {
        let a = crate::state::structural_fingerprint(&(
            self.source,
            self.ability_index,
            &self.targets,
            self.cost,
            &self.players,
            &self.choices,
            &self.second,
            &self.references,
        ));
        let b = crate::state::structural_fingerprint(&(
            self.answered,
            self.x,
            self.graveyard,
            &self.phyrexian,
            self.abilities
                .as_ref()
                .map(|(id, list)| (id, &list.abilities, list.printed, list.token)),
        ));
        a.wrapping_mul(31)
            .wrapping_add(b)
            .wrapping_mul(31)
            .wrapping_add(self.previous.as_ref().map_or(0, |window| {
                window
                    .suspended
                    .fingerprint()
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(window.player.get()) + 1)
            }))
            .wrapping_mul(31)
            .wrapping_add(self.opened.fingerprint())
    }
}
impl<L: CardLookup> Engine<L> {
    /// Reserve explicit life costs before counting optional life-to-mana actions.
    pub(super) fn can_plan_activation(
        &self,
        player: PlayerId,
        source: baylee_core::ids::ObjectId,
        cost: &baylee_cards_dsl::Cost,
        phyrexian_life: i32,
    ) -> bool {
        use baylee_cards_dsl::CostPart;
        let reserved =
            cost.parts
                .iter()
                .fold(u32::try_from(phyrexian_life).unwrap_or(0), |total, part| {
                    total.saturating_add(match part {
                        CostPart::PayLife(n) => u32::from(*n),
                        CostPart::PayLifeX => self.activation_x.unwrap_or(0),
                        _ => 0,
                    })
                });
        if !self
            .state
            .can_pay_life(player, i32::try_from(reserved).unwrap_or(i32::MAX))
        {
            return false;
        }
        let what = crate::casting::SpendFor::Ability(source);
        if !self.can_afford(
            player,
            source,
            &baylee_cards_dsl::Cost {
                mana: baylee_core::mana::ManaCost::ZERO,
                parts: cost.parts,
            },
            what,
        ) {
            return false;
        }
        let planned = crate::casting::planning_pool(&self.state, player, what, reserved);
        let pool = planned
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
        crate::casting::affordable(
            &self.state,
            player,
            pool,
            &cost.mana.with_x(self.activation_x.unwrap_or(0)),
        )
    }

    /// Whether `player` has a mana ability (or a granted way to make mana)
    /// it could activate while paying for an ability of `source` (CR
    /// 601.2g): the payment window's own offer, asked before announcing an
    /// X it would pay. The source's own mana abilities are not counted: an
    /// ability whose cost taps it cannot be paid by tapping it for mana, and
    /// a land whose only other mana is already floating would otherwise be
    /// asked for an X nothing can pay.
    pub(super) fn can_make_more_mana(
        &self,
        player: PlayerId,
        source: baylee_core::ids::ObjectId,
    ) -> bool {
        let legal = self.compute_legal(player);
        self.makes_mana_besides(player, &legal, source)
    }

    /// [`Self::can_make_more_mana`] read off an offer already built.
    pub(super) fn makes_mana_besides(
        &self,
        player: PlayerId,
        legal: &LegalActions,
        source: baylee_core::ids::ObjectId,
    ) -> bool {
        self.state.granted_colorless_capacity(player) > 0
            || legal.mana_abilities.iter().any(|&id| id != source)
            || legal.granted_actions.iter().any(|offer| {
                matches!(
                    offer.effect,
                    crate::choice::GrantedActionKind::AddMana { .. }
                )
            })
            || legal
                .abilities
                .iter()
                .any(|&(id, index)| id != source && self.is_mana_offer(id, index))
    }

    pub(super) fn open_activation_payment(
        &mut self,
        player: PlayerId,
        source: baylee_core::ids::ObjectId,
        ability_index: u32,
        targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
        cost: baylee_core::mana::ManaCost,
    ) -> Result<(), EngineError> {
        let source = self
            .state
            .source_identity(source)
            .ok_or(EngineError::IllegalAction("activation source left"))?;
        let opened = Box::new(self.window_start(player));
        let payment = ActivationPayment {
            opened,
            source,
            ability_index,
            targets,
            cost,
            players: std::mem::take(&mut self.activation_target_players),
            choices: std::mem::take(&mut self.activation_cost_choices),
            second: self.activation_second_targets.take(),
            references: std::mem::take(&mut self.activation_target_references),
            answered: std::mem::take(&mut self.activation_targets_answered),
            x: self.activation_x.take(),
            graveyard: self.activation_graveyard.take(),
            phyrexian: std::mem::take(&mut self.activation_phyrexian),
            abilities: self.activating_abilities.take(),
            previous: self.mana_window.take().map(Box::new),
        };
        self.mana_window = Some(PaymentWindow {
            player,
            suspended: PaymentContinuation::Activation(Box::new(payment)),
        });
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        self.pending = Pending::Priority {
            player,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
        Ok(())
    }

    pub(super) fn finish_activation_payment(
        &mut self,
        player: PlayerId,
        payment: ActivationPayment,
    ) {
        self.mana_window = payment.previous.map(|window| *window);
        // An activation its window leaves short is reversed, and the mana
        // abilities activated for it with it, as a cast's are (CR 732.1;
        // CR 602.2b follows CR 601.2h for the payment).
        if self.state.source_identity(payment.source.object) != Some(payment.source)
            || !self.can_pay_mana(
                player,
                crate::casting::SpendFor::Ability(payment.source.object),
                &payment.cost,
            )
        {
            self.give_back_window(player, &payment.opened);
            self.reverse_activation(player);
            return;
        }
        let opened = payment.opened;
        self.activation_target_players = payment.players;
        self.activation_cost_choices = payment.choices;
        self.activation_second_targets = payment.second;
        self.activation_target_references = payment.references;
        self.activation_targets_answered = payment.answered;
        self.activation_x = payment.x;
        self.activation_graveyard = payment.graveyard;
        self.activation_phyrexian = payment.phyrexian;
        self.activating_abilities = payment.abilities;
        if self
            .start_activation(
                player,
                payment.source.object,
                payment.ability_index,
                payment.targets,
            )
            .is_err()
        {
            self.give_back_window(player, &opened);
            self.reverse_activation(player);
        }
    }
}
