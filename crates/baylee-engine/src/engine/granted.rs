//! Dispatch for temporary special actions and their mana-payment windows.
use super::{CardLookup, Engine, EngineError, PaymentContinuation, PaymentWindow, PlayerId};
use crate::choice::{GrantedActionOffer, LegalActions};
use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
use baylee_core::ids::GrantedActionId;

#[cfg(test)]
thread_local! {
    /// Set, "can this seat make mana?" is asked as it was before
    /// [`Engine::settle_granted_offers`] and
    /// [`Engine::free_cast_makes_mana`]: of a list no payment window
    /// narrowed, where any activation reads as a mana source. A granted
    /// action the pool cannot pay is offered whenever the unfinished offer
    /// holds an activation, and a free cast's options count any activation
    /// as mana to come. The engine as it was, for `granted_offer_tests`'
    /// lockstep games.
    pub(in crate::engine) static ASK_MANA_AS_BEFORE: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

impl<L: CardLookup> Engine<L> {
    /// Every granted action of `player` the pool pays now, and every one
    /// with a mana cost besides: whether a payment window would have
    /// anything in it to make that mana with is asked of the finished
    /// offer, in [`Self::settle_granted_offers`].
    pub(super) fn offer_granted_actions(&self, player: PlayerId, legal: &mut LegalActions) {
        #[cfg(test)]
        if ASK_MANA_AS_BEFORE.with(std::cell::Cell::get) {
            let can_make_mana =
                legal.has_mana_source() || self.state.granted_colorless_capacity(player) > 0;
            legal.granted_actions = self
                .state
                .granted_actions
                .iter()
                .filter(|g| {
                    g.player == player
                        && (self.state.granted_action_payable(g)
                            || (matches!(g.offer.cost, SpecialActionCost::Mana(_))
                                && can_make_mana))
                })
                .map(|g| g.offer.clone())
                .collect();
            return;
        }
        legal.granted_actions = self
            .state
            .granted_actions
            .iter()
            .filter(|g| {
                g.player == player
                    && (matches!(g.offer.cost, SpecialActionCost::Mana(_))
                        || self.state.granted_action_payable(g))
            })
            .map(|g| g.offer.clone())
            .collect();
    }

    /// Keeps a granted action the pool cannot pay only while the payment
    /// window [`Self::start_granted_action`] would open for it has
    /// something in it to make mana with (CR 605.3a): the finished offer,
    /// narrowed as that window narrows it, without the unpaid grants
    /// themselves.
    ///
    /// Asked of the finished offer because the unfinished one is the wrong
    /// list to ask. Until `compute_legal` ends, `abilities` still holds
    /// every activation, also one whose mana nothing pays (it moves to
    /// `unpaid_abilities` last), and `LegalActions::has_mana_source` reads
    /// any entry there as a mana source. A seat with all its lands tapped
    /// and a `{3}` ability it could not pay was offered Nafs Asp's "pay
    /// {1}", and the window found nothing to make the {1} with and refused
    /// it (the v6 bug harvest, deal 359 of the priority probe).
    pub(super) fn settle_granted_offers(&self, player: PlayerId, legal: &mut LegalActions) {
        #[cfg(test)]
        if ASK_MANA_AS_BEFORE.with(std::cell::Cell::get) {
            return;
        }
        let unpaid = |offer: &GrantedActionOffer| {
            self.state
                .granted_actions
                .iter()
                .find(|g| g.player == player && g.offer.id == offer.id)
                .is_some_and(|g| !self.state.granted_action_payable(g))
        };
        if !legal.granted_actions.iter().any(unpaid) {
            return;
        }
        let mut window = legal.clone();
        window.granted_actions.retain(|offer| !unpaid(offer));
        self.narrow_to_mana(&mut window);
        if !window.has_mana_source() {
            legal.granted_actions.retain(|offer| !unpaid(offer));
        }
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
        self.pending = self.window_priority(player, legal);
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
    pub(super) cost: super::abilities::ActivationPrice,
    fixed_cost: baylee_core::mana::ManaCost,
    additional_life: u32,
    players: Vec<PlayerId>,
    choices: Vec<Vec<baylee_core::ids::ObjectId>>,
    second: Option<smallvec::SmallVec<[baylee_core::ids::ObjectId; 1]>>,
    references: crate::sources::TargetReferences,
    answered: bool,
    mode: Option<super::abilities::ActivationMode>,
    target_choice: super::targeting::ActivationTargetChoice,
    x: Option<u32>,
    graveyard: Option<PlayerId>,
    phyrexian: Vec<bool>,
    abilities: Option<(baylee_core::ids::ObjectId, crate::object::AbilityList)>,
    grant: Option<(
        baylee_core::ids::ObjectId,
        u32,
        crate::effects::GrantedAbility,
    )>,
    pub(super) previous: Option<Box<PaymentWindow>>,
    /// Where the window began, to give back what it made if the
    /// activation is not completed (CR 734.1).
    opened: Box<super::WindowStart>,
}
impl ActivationPayment {
    /// The permanent whose ability this payment activates.
    pub(super) const fn source(&self) -> baylee_core::ids::ObjectId {
        self.source.object
    }

    pub(super) fn fingerprint(&self) -> u64 {
        // Keep the historical field order for every payment that predates
        // imposed non-mana costs; only that new suffix extends the digest.
        let a = crate::state::structural_fingerprint(&(
            self.source,
            self.ability_index,
            &self.targets,
            self.cost.mana,
            self.fixed_cost,
            (self.additional_life, self.cost.allowance),
            &self.players,
            &self.choices,
            &self.second,
            &self.references,
        ));
        let imposed = self.cost.imposed_parts();
        let a = if imposed.is_empty() {
            a
        } else {
            a.wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&imposed))
        };
        let b = crate::state::structural_fingerprint(&(
            (self.answered, self.mode, &self.target_choice),
            self.x,
            self.graveyard,
            &self.phyrexian,
            self.grant,
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
        cost: &super::abilities::ActivationPrice,
        phyrexian_life: i32,
    ) -> bool {
        use baylee_cards_dsl::CostPart;
        let reserved =
            cost.parts
                .iter()
                .fold(u32::try_from(phyrexian_life).unwrap_or(0), |total, part| {
                    total.saturating_add(match part {
                        CostPart::PayLife(n) => u32::from(*n),
                        CostPart::PayCommanderColors => {
                            u32::from(self.state.commander_colors(player).len())
                        }
                        CostPart::PayLifeX => self.activation_x.unwrap_or(0),
                        _ => 0,
                    })
                });
        if i32::try_from(reserved).is_err()
            || !self
                .state
                .can_pay_life(player, i32::try_from(reserved).unwrap_or(i32::MAX))
        {
            return false;
        }
        let what = crate::casting::SpendFor::Ability(source);
        if !self.can_afford_parts(
            player,
            source,
            baylee_core::mana::ManaCost::ZERO,
            &cost.parts,
            what,
        ) {
            return false;
        }
        let mana = cost.mana.with_x(self.activation_x.unwrap_or(0));
        if mana.symbols().next().is_none() {
            return true;
        }
        let planned = crate::casting::planning_pool(&self.state, player, what, reserved);
        let pool = planned
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
        crate::casting::source_affordable(
            &self.state,
            player,
            what,
            pool,
            &mana,
            &[],
            Some(cost.allowance),
        )
    }

    /// Whether `player` has a mana ability (or a granted way to make mana)
    /// it could activate while paying for this ability (CR 601.2g).
    /// Reserve the source only if its printed cost needs its tap/untap symbol.
    pub(super) fn can_make_more_mana(
        &self,
        player: PlayerId,
        source: baylee_core::ids::ObjectId,
        index: u32,
    ) -> bool {
        let legal = self.compute_legal(player);
        self.makes_activation_mana(player, &legal, source, index)
    }

    /// [`Self::can_make_more_mana`] read off an offer already built.
    pub(super) fn makes_activation_mana(
        &self,
        player: PlayerId,
        legal: &LegalActions,
        source: baylee_core::ids::ObjectId,
        index: u32,
    ) -> bool {
        let reserved =
            self.activation_definition(source, index)
                .is_none_or(|ability| match ability {
                    baylee_cards_dsl::AbilityDef::Activated { cost, .. }
                    | baylee_cards_dsl::AbilityDef::ModalActivated { cost, .. }
                    | baylee_cards_dsl::AbilityDef::ActivatedConditional { cost, .. } => {
                        cost.parts.iter().any(|part| {
                            matches!(
                                part,
                                baylee_cards_dsl::CostPart::TapSelf
                                    | baylee_cards_dsl::CostPart::UntapSelf
                            )
                        })
                    }
                    _ => true,
                });
        let excluded = if reserved {
            source
        } else {
            baylee_core::ids::ObjectId::NO_SOURCE
        };
        self.state.granted_colorless_capacity(player) > 0
            || legal.mana_abilities.iter().any(|&id| id != excluded)
            || legal.granted_actions.iter().any(|offer| {
                matches!(
                    offer.effect,
                    crate::choice::GrantedActionKind::AddMana { .. }
                )
            })
            || legal
                .abilities
                .iter()
                .any(|&(id, index)| id != excluded && self.is_payment_mana_offer(id, index))
    }

    pub(super) fn open_activation_payment(
        &mut self,
        player: PlayerId,
        source: baylee_core::ids::ObjectId,
        ability_index: u32,
        targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
        price: (
            super::abilities::ActivationPrice,
            baylee_core::mana::ManaCost,
            u32,
        ),
    ) -> Result<(), EngineError> {
        let (cost, fixed_cost, additional_life) = price;
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
            fixed_cost,
            additional_life,
            players: std::mem::take(&mut self.activation_target_players),
            choices: std::mem::take(&mut self.activation_cost_choices),
            second: self.activation_second_targets.take(),
            references: std::mem::take(&mut self.activation_target_references),
            answered: std::mem::take(&mut self.activation_targets_answered),
            mode: self.activation_mode.take(),
            target_choice: std::mem::take(&mut self.activation_target_choice),
            x: self.activation_x.take(),
            graveyard: self.activation_graveyard.take(),
            phyrexian: std::mem::take(&mut self.activation_phyrexian),
            abilities: self.activating_abilities.take(),
            grant: self.activating_grant.take(),
            previous: self.mana_window.take().map(Box::new),
        };
        self.mana_window = Some(PaymentWindow {
            player,
            suspended: PaymentContinuation::Activation(Box::new(payment)),
        });
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        self.pending = self.window_priority(player, legal);
        self.awaiting_answer = true;
        Ok(())
    }

    pub(super) fn finish_activation_payment(
        &mut self,
        player: PlayerId,
        payment: ActivationPayment,
    ) -> bool {
        self.mana_window = payment.previous.map(|window| *window);
        // An activation its window leaves short is reversed, and the mana
        // abilities activated for it with it, as a cast's are (CR 734.1;
        // CR 602.2b follows CR 601.2h for the payment).
        if self.state.source_identity(payment.source.object) != Some(payment.source)
            || !self.can_pay_mana(
                player,
                crate::casting::SpendFor::Ability(payment.source.object),
                &payment.cost.mana,
            )
        {
            self.give_back_window(player, &payment.opened);
            self.reverse_activation(player);
            return false;
        }
        let opened = payment.opened;
        self.activation_target_players = payment.players;
        self.activation_cost_choices = payment.choices;
        self.activation_second_targets = payment.second;
        self.activation_target_references = payment.references;
        self.activation_targets_answered = payment.answered;
        self.activation_mode = payment.mode;
        self.activation_target_choice = payment.target_choice;
        self.activation_x = payment.x;
        self.activation_graveyard = payment.graveyard;
        self.activation_phyrexian = payment.phyrexian;
        self.activating_abilities = payment.abilities;
        self.activating_grant = payment.grant;
        if self
            .start_activation_with_price(
                player,
                payment.source.object,
                payment.ability_index,
                payment.targets,
                Some((payment.cost, payment.fixed_cost, payment.additional_life)),
            )
            .is_err()
        {
            self.give_back_window(player, &opened);
            self.reverse_activation(player);
            return false;
        }
        true
    }
}
