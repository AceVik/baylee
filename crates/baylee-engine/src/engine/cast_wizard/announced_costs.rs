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

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
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
    pub(super) fn announced_cost_payable(&self, paying: &CastWizard) -> bool {
        let face = self.wizard_face(paying);
        self.state
            .can_pay_life(paying.player, paying.total_life_cost(face))
            && casting::additional_payment_capacity_for(
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
                || {
                    let mut cost = wizard_total_cost(face, paying);
                    let mandatory_wb = super::wizard_mandatory_waterbend_generic(face, paying);
                    if (face.waterbend && (paying.kicked || paying.optional_cost_paid))
                        || mandatory_wb > 0
                    {
                        let mut waterbend = if face.waterbend && paying.kicked {
                            face.additional_costs
                                .iter()
                                .fold(ManaCost::ZERO, |sum, add| sum.combine(&add.mana))
                        } else {
                            ManaCost::ZERO
                        };
                        if face.waterbend
                            && paying.optional_cost_paid
                            && let Some(opt) = face.optional_cost
                        {
                            waterbend = waterbend.combine(&opt.mana);
                        }
                        waterbend = waterbend.with_more_generic(mandatory_wb);
                        let waterbend_generic = waterbend.generic_total();
                        let taps = u32::try_from(
                            casting::waterbend_sources(&self.state, paying.player).len(),
                        )
                        .unwrap_or(u32::MAX)
                        .min(waterbend_generic);
                        cost = cost.with_less_generic(taps);
                    } else {
                        let red = casting::keyword_reduction(
                            &self.state,
                            face,
                            paying.player,
                            paying.card,
                        );
                        cost = casting::keyword_priced_for(
                            &self.state,
                            paying.card,
                            paying.face_index(),
                            face,
                            paying.player,
                            &cost,
                            red,
                        );
                    }
                    self.wizard_pool_can_pay(paying, cost)
                })
    }

    /// The bargain question (`CR 702.166a`), or `None` when the face does not
    /// print `bargain` or no eligible permanent can satisfy it alongside the
    /// cast's other costs.
    pub(super) fn ask_bargain(&mut self, wizard: &CastWizard) -> Option<Pending> {
        if !self.wizard_face(wizard).bargain
            || wizard.option == Some(CastModeKind::Disguise)
            || wizard.masked
        {
            return None;
        }
        let mut paying = wizard.clone();
        paying.bargained = true;
        self.refresh_target_cost_adjustments(&mut paying);
        if !self.announced_cost_payable(&paying) {
            return None;
        }
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
            prompt: YesNoPrompt::Bargain,
            source,
        })
    }

    /// The offspring question (`CR 702.175a`), or `None` when the face prints
    /// none or the combined cost cannot be paid.
    pub(super) fn ask_offspring(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let printed = self.wizard_face(wizard).offspring?;
        if wizard.option == Some(CastModeKind::Disguise) || wizard.masked {
            return None;
        }
        let mut paying = wizard.clone();
        paying.offspring = true;
        if !self.announced_cost_payable(&paying) {
            return None;
        }
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
            prompt: YesNoPrompt::Offspring {
                cost: printed.mana,
                parts: crate::choice::BuybackParts::of(printed.parts),
            },
            source,
        })
    }

    /// The casualty question (`CR 702.153a`), or `None` when the face prints
    /// none or no controlled creature with projected power `>= min_power` can
    /// pay it alongside the cast's other costs.
    pub(super) fn ask_casualty(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let min_power = self.wizard_effective_casualty(wizard)?;
        if wizard.option == Some(CastModeKind::Disguise) || wizard.masked {
            return None;
        }
        let mut paying = wizard.clone();
        paying.casualty = true;
        paying.casualty_power = Some(min_power);
        if !self.announced_cost_payable(&paying) {
            return None;
        }
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
            prompt: YesNoPrompt::Casualty { min_power },
            source,
        })
    }

    /// The gift promise or multiplayer recipient question (`CR 702.174a`), or
    /// `None` when the face prints no gift or no living opponent exists.
    pub(super) fn ask_gift(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let kind = self.wizard_face(wizard).gift?;
        if wizard.option == Some(CastModeKind::Disguise) || wizard.masked {
            return None;
        }
        let opponents = eval::players(
            baylee_cards_dsl::PlayerRel::Opponent,
            &self.state,
            wizard.player,
        )
        .unwrap_or_default();
        if opponents.is_empty() {
            return None;
        }
        if wizard.gift_promised {
            return Some(Pending::ChoosePlayer {
                player: wizard.player,
                options: opponents,
            });
        }
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
            prompt: YesNoPrompt::Gift { kind },
            source,
        })
    }

    /// The squad question (`CR 702.157a`), or `None` when the face prints none
    /// or not even one squad payment can be made.
    pub(super) fn ask_squad(&mut self, wizard: &CastWizard) -> Option<Pending> {
        let printed = self.wizard_face(wizard).squad?;
        if wizard.option == Some(CastModeKind::Disguise) || wizard.masked {
            return None;
        }
        let max = self.squad_bound(wizard, &printed);
        (max > 0).then_some(Pending::ChooseNumber {
            player: wizard.player,
            min: 0,
            max,
            reason: NumberPrompt::Squad {
                cost: printed.mana,
                parts: crate::choice::BuybackParts::of(printed.parts),
            },
        })
    }

    fn squad_bound(&self, wizard: &CastWizard, cost: &baylee_cards_dsl::Cost) -> u32 {
        let face = self.wizard_face(wizard);
        let parts_bound = if cost.parts.is_empty() {
            REPLICATE_CEILING
        } else {
            let mut probe = wizard.clone();
            casting::greatest_affordable(REPLICATE_CEILING, |n| {
                probe.squad = u8::try_from(n).unwrap_or(u8::MAX);
                self.state
                    .can_pay_life(probe.player, probe.total_life_cost(face))
                    && casting::additional_payment_capacity_for(
                        &self.state,
                        probe.player,
                        probe.card,
                        face,
                        probe.cost_choice(),
                        &[],
                    )
                    .is_some()
            })
        };
        if parts_bound == 0 {
            return 0;
        }
        let mana_bound = if cost.mana == ManaCost::ZERO
            || wizard.makes_mana_after_choices()
            || self.can_make_mana_for_cast(wizard.player)
        {
            REPLICATE_CEILING
        } else {
            let mana_each = cost.mana.with_x(wizard.x);
            self.replicate_bound(wizard, face, &mana_each)
        };
        parts_bound.min(mana_bound)
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
    /// count, the number of dual/multi kicker costs paid, or 1 for a single
    /// kicker taken.
    pub(super) fn times_kicked(&self) -> u8 {
        let mask_count = u8::try_from(self.kicked_mask.count_ones()).unwrap_or(u8::MAX);
        self.multikicked.max(mask_count).max(u8::from(self.kicked))
    }

    /// The non-mana costs this cast has announced.
    pub(crate) fn cost_choice(&self) -> CostChoice {
        let mut splice_parts = [None; 8];
        for (slot, &(_, _, _, cost)) in splice_parts.iter_mut().zip(self.spliced.iter()) {
            *slot = Some(cost.parts);
        }
        let route_extra_parts = self
            .selected_alt_grant()
            .and_then(|r| r.extra_cost)
            .map_or(&[][..], |c| c.parts);
        CostChoice {
            option: self.option,
            alternative_cost: self.alternative_cost,
            buyback: self.buyback.is_some(),
            bargained: self.bargained,
            optional_cost_paid: self.optional_cost_paid,
            offspring: self.offspring,
            casualty: self.casualty,
            casualty_power: self.casualty_power,
            squad: self.squad,
            kicked: self.kicked,
            kicked_mask: self.kicked_mask,
            emerge_sacrifice: self.emerge_sacrifice,
            either_decided: self.either_decided,
            either_first_mask: self.either_first_mask,
            returned: self.returned,
            sneak_parts: self.sneak_parts,
            conspired: self.conspired,
            awaken_parts: self.awaken_parts,
            route_extra_parts,
            splice_parts,
        }
    }
}
