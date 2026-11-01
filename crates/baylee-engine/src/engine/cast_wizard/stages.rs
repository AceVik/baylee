//! Announced choices before paying for a cast.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn advance_cast_wizard_inner(&mut self) -> Result<(), EngineError> {
        let Some(mut wizard) = self.cast_wizard.clone() else {
            return Ok(());
        };
        if let Some(CastModeKind::EscapeGranted {
            face,
            modes,
            kicked,
            prototype,
            ..
        }) = wizard.option
        {
            wizard.announced = wizard.option;
            wizard.kicked = kicked;
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .expect("wizard card known");
            let single = def.abilities_for_face(face).iter().any(|ability|
                matches!(ability, AbilityDef::ModalSpell { choose, .. } if choose.is_one()));
            wizard.option = Some(if prototype {
                CastModeKind::Prototype
            } else if modes != 0 && single {
                CastModeKind::Mode(modes.trailing_zeros() as usize)
            } else if modes != 0 {
                CastModeKind::Modes(modes)
            } else if face == 0 && kicked {
                CastModeKind::Kicked
            } else if face == 0 {
                CastModeKind::Normal
            } else {
                CastModeKind::Face(face)
            });
            self.cast_wizard = Some(wizard.clone());
        }
        if wizard.option.is_some() && wizard.reduction.is_none() {
            wizard.reduction = Some(self.frozen_spell_reduction(&wizard));
            self.cast_wizard = Some(wizard.clone());
        }
        if let Some(CastModeKind::PlayLandFace(face)) = wizard.option {
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .ok_or(EngineError::IllegalAction("unknown land"))?;
            if face > 0 {
                self.state.switch_face(wizard.card, def, face);
            }
            casting::play_land_by_effect(&mut self.state, wizard.player, wizard.card)?;
            self.cast_wizard = None;
            if self.state.entries.contains(wizard.card) {
                self.suspend_nested_land_entry();
            } else {
                self.finish_nested_cast();
            }
            return Ok(());
        }
        match wizard.stage {
            WizardStage::ManaPermission => self.ask_north_star(wizard),
            WizardStage::Permission => self.choose_graveyard_permission(wizard),
            WizardStage::ChooseMode => {
                self.pending = Pending::ChooseCastMode {
                    player: wizard.player,
                    object: wizard.card,
                    options: wizard.options.clone(),
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::XValue => {
                if let Some(asked) = self.ask_phyrexian(&wizard) {
                    return asked;
                }
                // The chosen option's *printed* cost, and deliberately not
                // `wizard_cost`: that one substitutes the chosen X into the
                // cost, and the chosen X at this stage is still the zero the
                // wizard starts with. So `{X}{U}{U}{U}` arrived here as
                // `{0}{U}{U}{U}` — no variable left in it, `needs_x` false,
                // and every spell with an X in its printed cost was quietly
                // cast for X = 0 without anybody being asked. Nothing failed;
                // the question simply never appeared.
                let cost = chosen_option_cost(&wizard);
                // X is asked when the cost has a variable OR a mandatory
                // part scales with it (Toxic Deluge's pay-X-life).
                let pays_life_x = self
                    .wizard_face(&wizard)
                    .mandatory_additional_costs
                    .contains(&CostPart::PayLifeX);
                // A spell cast without paying its mana cost has exactly one
                // legal X, which is 0 (CR 107.3b), so a free cast is not
                // asked for it; a life payment that scales with X still is.
                let needs_x = (cost.has_variable() && !wizard.free) || pays_life_x;
                if needs_x {
                    // The mana for X may still be made during the cast (CR
                    // 601.2g): a cast whose mana comes after its choices, and
                    // a cast off the pool by a seat with a source left to
                    // tap, both open a payment window when the total needs
                    // it, so X is bounded only by the number type, as an
                    // activation's is (`abilities/x_value.rs`). An answer
                    // the window then cannot pay is reversed there and says
                    // so (`GameEvent::CastCancelled`, CR 601.2h, 732.1).
                    // Life payments still cannot exceed payable life.
                    let more_mana = !wizard.makes_mana_after_choices()
                        && self.can_make_mana_for_cast(wizard.player);
                    let mut max = if cost.has_variable() && !wizard.free && !more_mana {
                        self.x_resource_bound(&wizard)
                    } else {
                        u32::MAX
                    };
                    if pays_life_x {
                        let life = self.state.life_payable(wizard.player);
                        max = max.min(u32::try_from(life).unwrap_or(0));
                    }
                    // With nothing left to tap, the pool is all that pays,
                    // so the question offers only the X that mana pays:
                    // `{X}{W}` with one {W} floating and no land untapped is
                    // X = 0. Offered the pool's size instead, an X the fixed
                    // part left no mana for was taken and the cast then
                    // reversed at payment with nothing to show the caster
                    // why — Prismatic Ending and Finale of Devastation in the
                    // reports of 12.10.2026.
                    if !wizard.makes_mana_after_choices()
                        && !more_mana
                        && ((self.wizard_face(&wizard).x_mana_color.is_some()
                            || self.wizard_face(&wizard).x_mana_colors.is_some())
                            || (cost.has_variable()
                                && !wizard.free
                                && self.pool_alone_pays_x(&wizard)))
                    {
                        let mut probe = wizard.clone();
                        max = casting::greatest_affordable(max, |x| {
                            probe.x = x;
                            self.restricted_x_affordable(&probe)
                        });
                    }
                    self.pending = Pending::ChooseNumber {
                        player: wizard.player,
                        min: 0,
                        max,
                        reason: NumberPrompt::X,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::Kicker;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::DivideDamage => self.advance_damage_division(wizard),
            WizardStage::Targets => {
                let Some(req) = self.wizard_target_req(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let (min, max) = req.bounds(wizard.x);
                let max = self
                    .wizard_divided_amount(&wizard)
                    .map_or(max, |total| max.min(total));
                let spec = req.spec;
                if matches!(spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::ChoosePlayer;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                if max == 0 {
                    // Zero required targets (e.g. X = 0): skip targeting.
                    // This reads `max` off the requirement, before the
                    // options are known; the other way of having nothing to
                    // choose is a `max` the board cannot fill, and that one
                    // is decided below, once they are.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let options = self.wizard_target_options(&wizard, &spec);
                let player_options = eval::target_player_options(&self.state, &spec, wizard.player);
                // "Any target" is one set: a burn spell with every creature
                // hexproofed is still castable at a player's face, so the
                // count that has to reach `min` spans both halves.
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                if options.is_empty() && player_options.is_empty() {
                    // Nothing to point at, and `min` is zero or the check
                    // above would have refused the cast: the only answer is
                    // the empty list, so the spell goes on the stack with no
                    // targets instead of the caster being stopped for a
                    // choice they cannot make.
                    //
                    // Eerie Interlude and Clever Concealment are what reach
                    // it today — "any number of target permanents you
                    // control" is `min` 0, `max` 255, so the branch above
                    // never sees them, and either one is castable on an
                    // empty board. `collect_triggers` is the same rule on
                    // the trigger side.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let max = max
                    .min(u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX));
                let reason = if req.distinct_mana_values {
                    let mut values: Vec<_> = options
                        .iter()
                        .filter_map(|&id| {
                            self.state
                                .object(id)
                                .map(|object| (id, object.characteristics().mana_value()))
                        })
                        .collect();
                    values.sort_unstable_by_key(|(id, _)| *id);
                    TargetPrompt::DistinctManaValues(values)
                } else {
                    TargetPrompt::Targets
                };
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    player_options,
                    min,
                    max,
                    reason,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::SecondTargets => {
                // The second instance is asked the way the first is, and is
                // its own question: its own requirement, its own count, and
                // options that do **not** leave out what the first chose —
                // CR 115.3 lets one object be the target of both instances
                // as long as it fits both. The two spells that reach this
                // today name disjoint sets ("you control" / "you don't"), so
                // the sentence costs nothing there and is right for the card
                // that doesn't.
                let Some(req) = self.wizard_second_target_req(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::DivideDamage;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let options = self.wizard_target_options_relative(
                    &wizard,
                    &req.spec,
                    wizard.targets.first().copied(),
                );
                let player_options =
                    eval::target_player_options(&self.state, &req.spec, wizard.player);
                if options.len() + player_options.len() < req.min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                if (options.is_empty() && player_options.is_empty()) || req.max == 0 {
                    // "Up to one" with nothing to point at: the empty answer
                    // is the only one, for the reason the first stage gives.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::DivideDamage;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let (min, max) = req.bounds(wizard.x);
                let max = max
                    .min(u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::ChoosePlayer => {
                // The same enumeration the object half of targeting uses, and
                // for the same reason: "target opponent" is a choice over a
                // smaller set, not a different kind of choice (CR 115.1), so
                // the caster is out of it — and so is a teammate. Listing
                // every living player here was a hole even before teams: a
                // "target opponent" spell offered its own caster.
                let spec = self
                    .wizard_target_req(&wizard)
                    .map_or(TargetSpec::AnyPlayer, |req| req.spec);
                let options = eval::target_player_options(&self.state, &spec, wizard.player);
                // No player can be chosen (every opponent has hexproof, say):
                // the spell cannot take the step and cannot be cast (CR
                // 601.2c, 601.2), and an empty menu is a question with no
                // answer. `miracle_targets_available` reads the same list.
                if options.is_empty() {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                self.pending = Pending::ChoosePlayer {
                    player: wizard.player,
                    options,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Kicker => {
                let face = self.wizard_face(&wizard);
                if face.additional_costs.is_empty()
                    || wizard.kicked
                    || matches!(wizard.announced, Some(CastModeKind::EscapeGranted { .. }))
                    || (face.kicked_targets.is_some() && !wizard.free)
                {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Replicate;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::YesNo {
                    player: wizard.player,
                    prompt: YesNoPrompt::Kicker,
                    source: self.state.object(wizard.card).and_then(|o| {
                        o.card.map(|c| {
                            baylee_core::ids::AbilityRef::new(
                                c.index,
                                baylee_core::ids::AbilityRef::ADDITIONAL_COST,
                            )
                        })
                    }),
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Replicate => {
                let face = self.wizard_face(&wizard);
                let (cost, max) = face.replicate.map_or((ManaCost::ZERO, 0), |cost| {
                    (cost, self.replicate_bound(&wizard, face, &cost))
                });
                if max == 0 {
                    // Nothing to ask: no replicate, or not one payment the
                    // pool covers, and zero is then the only answer.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Targets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseNumber {
                    player: wizard.player,
                    min: 0,
                    max,
                    reason: NumberPrompt::Replicate { cost },
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::PitchChoice => {
                let filter = self.wizard_pitch_filter(&wizard);
                if let Some(filter) = filter {
                    // The same scan `can_afford` and `casting::can_cast` now
                    // run before this cast was ever offered: one reader, so
                    // the offer and the prompt cannot disagree about what is
                    // in the hand.
                    let options =
                        casting::pitchable(&self.state, wizard.player, wizard.card, filter);
                    if options.is_empty() {
                        self.cast_wizard = None;
                        return Err(EngineError::IllegalAction(
                            "no card to exile for the pitch cost",
                        ));
                    }
                    self.pending = Pending::ChooseCards {
                        player: wizard.player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::ReturnCreature;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::ReturnCreature => {
                if wizard.option != Some(CastModeKind::BeamMeUp) {
                    wizard.stage = WizardStage::Behold;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let options = casting::beam_return_options(&self.state, wizard.player, wizard.card);
                if options.is_empty() {
                    return Err(EngineError::IllegalAction("no creature can pay beam me up"));
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::CostReturn,
                    total: None,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Behold => {
                let face = self.wizard_face(&wizard);
                let Some(behold) = face.behold_or_pay else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let options =
                    casting::behold_options(&self.state, wizard.player, wizard.card, behold.filter);
                if options.is_empty() {
                    let mut wizard = wizard;
                    wizard.behold_paid_mana = true;
                    wizard.stage = WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let mut probe = wizard.clone();
                probe.behold_paid_mana = true;
                let can_pay_mana_instead = wizard.makes_mana_after_choices()
                    || self.can_make_mana_for_cast(wizard.player)
                    || self.wizard_pool_can_pay(&probe, wizard_total_cost(face, &probe));
                let min = u8::from(!can_pay_mana_instead);
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Escape => {
                self.ask_escape_payment(&wizard)?;
                Ok(())
            }
            WizardStage::Delve => {
                let face = self.wizard_face(&wizard);
                // The offer's own list (`casting::delve_sources`): the
                // graveyard less the card being cast (CR 601.2a).
                let graveyard =
                    crate::casting::delve_sources(&self.state, wizard.player, wizard.card);
                // How many may be exiled is bounded by the *cost*, not by the
                // graveyard. CR 702.66a is "for each generic mana in this
                // spell's total cost, you may exile a card from your
                // graveyard rather than pay that mana", so a seventh card has
                // nothing left to pay for once `{6}{U}{U}` has had six exiled
                // against it. This asked for up to the whole graveyard, and
                // `reduce_generic` cuts *up to* n — so every card past the
                // sixth was exiled for no reduction at all, which is a cost
                // spent for nothing and not merely an odd-looking prompt.
                // Same shape as the X question: a bound that cannot be
                // enforced where the answer is spent goes on the question.
                //
                // The bound is on `max` alone. *Which* cards go is the
                // player's, so the whole graveyard stays in `options` — a
                // truncated list would pick their six for them.
                let generic = usize::try_from(wizard_total_cost(face, &wizard).generic_total())
                    .unwrap_or(usize::MAX);
                let room = graveyard.len().min(generic);
                if !face.delve || room == 0 {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Convoke;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let max = u8::try_from(room).unwrap_or(u8::MAX);
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options: graveyard,
                    min: 0,
                    max,
                    prompt: ChoicePrompt::Delve,
                    total: None,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Convoke => {
                let face = self.wizard_face(&wizard);
                // Two keywords tap permanents to pay, and they differ in
                // what they tap and in how much of the cost the taps may pay
                // (#229). A waterbend's taps exist only once its cost is paid
                // and pay that cost's generic mana alone (CR 701.67b). Convoke
                // may pay the spell's whole total cost (CR 702.51a), but
                // `finish_cast` takes the taps off its generic mana only, so
                // the bound is the generic delve has left; #230 teaches the
                // payment a creature's colour and moves this bound with it.
                // A bound the payment cannot honour is a tap spent for
                // nothing, as in the delve stage above.
                let generic =
                    |cost: &ManaCost| usize::try_from(cost.generic_total()).unwrap_or(usize::MAX);
                let (sources, room) = if face.waterbend && wizard.kicked {
                    let waterbend = face
                        .additional_costs
                        .iter()
                        .fold(ManaCost::ZERO, |total, add| total.combine(&add.mana));
                    (
                        crate::casting::waterbend_sources(&self.state, wizard.player),
                        generic(&waterbend),
                    )
                } else if face.convoke
                    || crate::casting::has_improvise(&self.state, face, wizard.player)
                {
                    // Convoke's creatures and improvise's artifacts
                    // (CR 702.126a), each permanent once.
                    (
                        crate::casting::tap_to_pay_sources(&self.state, face, wizard.player),
                        generic(&wizard_total_cost(face, &wizard))
                            .saturating_sub(wizard.delve_exiles.len()),
                    )
                } else {
                    (Vec::new(), 0)
                };
                if sources.is_empty() || room == 0 {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Sacrifice;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // The bound is what is actually on the table, not a sentinel.
                // `99` is what a player casting a waterbend spell was shown:
                // "choose up to 99 targets", over two creatures, for a
                // question that is not targeting at all. *Which* permanents
                // tap is the player's, so every source stays in `options`.
                let max = u32::try_from(sources.len().min(room)).unwrap_or(u32::MAX);
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options: sources,
                    player_options: Vec::new(),
                    min: 0,
                    max,
                    reason: TargetPrompt::Convoke,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Sacrifice => {
                let face = self.wizard_face(&wizard);
                let asked = wizard.sacrifices.len();
                let Some(part) = additional_card_costs_for(face, wizard.option).nth(asked) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                // The reader `casting::can_cast_form` refused an empty board
                // with, less what an earlier part of this cost already named:
                // nothing is paid until every question is answered
                // (CR 601.2h), so one creature must not pay two sacrifices.
                let mut options = super::super::cost_wizard::options(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    part,
                );
                options.retain(|id| *id != wizard.card && !wizard.sacrifices.contains(id));
                options.retain(|candidate| {
                    let mut chosen = wizard.sacrifices.to_vec();
                    chosen.push(*candidate);
                    casting::additional_payment_capacity_for(
                        &self.state,
                        wizard.player,
                        wizard.card,
                        face,
                        wizard.option,
                        &chosen,
                    )
                    .is_some()
                });
                if options.is_empty() {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("nothing can pay this cost"));
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: super::super::cost_wizard::prompt(part),
                    total: None,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Done => self.cast_or_make_miracle_mana(wizard),
        }
    }
}
