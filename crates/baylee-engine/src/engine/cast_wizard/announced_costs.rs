//! Buyback and multikicker: the additional costs announced as a spell is cast
//! (CR 601.2b) that the cast may decline.
//!
//! Neither is a way of casting, so neither is a [`CastModeKind`]: each sits on
//! top of whichever way was chosen (CR 118.8a), a free cast's and an
//! alternative cost's included (CR 118.9d). So they are questions the wizard
//! asks, after the kicker's and before the targets, and the answers are added
//! to the total (`wizard_total_before_reduction`, CR 601.2f).
//!
//! A question is asked only when a yes could be paid. Mana that can still be
//! made counts: the cast opens a window for what the pool lacks (CR 601.2g),
//! and a total the window cannot reach is reversed there (CR 601.2h, 734.1),
//! as an X too large is. What cannot be made is not offered: the land the
//! player does not control, the life they do not have.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    /// The buyback question, or `None` when there is nothing to ask.
    pub(super) fn ask_buyback(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let less = self.buyback_on_offer(wizard)?;
        let printed = self.wizard_face(wizard).buyback?;
        let cost = printed.mana.with_less_generic(less);
        let parts = crate::choice::BuybackParts::of(printed.parts);
        let source = self.state.object(wizard.card).and_then(|object| {
            object.card.map(|card| {
                baylee_core::ids::AbilityRef::new(
                    card.index,
                    baylee_core::ids::AbilityRef::ADDITIONAL_COST,
                )
            })
        });
        Some(Pending::YesNo {
            player: wizard.player,
            prompt: YesNoPrompt::Buyback { cost, parts },
            source,
        })
    }

    /// What comes off the buyback cost, if this cast may announce it.
    ///
    /// A face-down spell has no text (CR 708.2), and with it no buyback.
    pub(in crate::engine) fn buyback_on_offer(&self, wizard: &CastWizard) -> Option<u32> {
        if wizard.option == Some(CastModeKind::Disguise) {
            return None;
        }
        self.wizard_face(wizard).buyback?;
        let less = self.wizard_buyback_less(wizard);
        let mut paying = wizard.clone();
        paying.buyback = Some(less);
        self.announced_cost_payable(&paying).then_some(less)
    }

    /// The generic mana that comes off the buyback cost (CR 118.7a), and off
    /// nothing else.
    fn wizard_buyback_less(&self, wizard: &CastWizard) -> u32 {
        let def = self
            .state
            .object(wizard.card)
            .and_then(|object| object.card)
            .and_then(|card| self.lookup.card(card.index))
            .expect("wizard card known");
        casting::face_adjustment(
            &self.state,
            wizard.player,
            wizard.card,
            def,
            wizard.face_index(),
            None,
        )
        .buyback_reduction
    }

    /// Whether the total of `paying`, an announcement this cast could make,
    /// can be paid: every part of it there is something to pay with, and its
    /// mana either floating or still to be made.
    fn announced_cost_payable(&self, paying: &CastWizard) -> bool {
        let face = self.wizard_face(paying);
        casting::additional_payment_capacity_for(
            &self.state,
            paying.player,
            paying.card,
            face,
            paying.cost_choice(),
            &[],
        )
        .is_some()
            && (paying.makes_mana_after_choices()
                || self.can_make_mana_for_cast(paying.player)
                || self.wizard_pool_can_pay(paying, wizard_total_cost(face, paying)))
    }

    /// The multikicker question, or `None` when there is nothing to ask.
    pub(super) fn ask_multikicker(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let cost = self.wizard_face(wizard).multikicker?;
        if wizard.option == Some(CastModeKind::Disguise) {
            return None;
        }
        let max = self.multikicker_bound(wizard, &cost);
        (max > 0).then_some(Pending::ChooseNumber {
            player: wizard.player,
            min: 0,
            max,
            reason: NumberPrompt::Multikicker { cost },
        })
    }

    /// The most times the caster may announce paying `cost`.
    ///
    /// Any number of times, says CR 702.33c, so what bounds the question is
    /// what can be paid. With a mana source left to tap the engine cannot
    /// tell how far it reaches, and the question is left as open as an X's
    /// (`WizardStage::XValue`); a client that plans its mana narrows it,
    /// as it narrows an X. With none, the pool is all there is.
    ///
    /// Asked of the spell as it would be once kicked, which is what restricted
    /// mana looks at (CR 106.6).
    fn multikicker_bound(&self, wizard: &CastWizard, cost: &ManaCost) -> u32 {
        if wizard.makes_mana_after_choices() || self.can_make_mana_for_cast(wizard.player) {
            return REPLICATE_CEILING;
        }
        let mut kicked = wizard.clone();
        kicked.kicked = true;
        self.replicate_bound(&kicked, self.wizard_face(wizard), cost)
    }
}

impl CastWizard {
    /// How many times this spell was kicked (CR 702.33d): a multikicker's
    /// count, or 1 for a kicker taken.
    pub(super) fn times_kicked(&self) -> u8 {
        self.multikicked.max(u8::from(self.kicked))
    }

    /// The non-mana costs this cast has announced.
    pub(super) const fn cost_choice(&self) -> CostChoice {
        CostChoice {
            option: self.option,
            alternative_cost: self.alternative_cost,
            buyback: self.buyback.is_some(),
        }
    }
}
