//! Announced choices before paying for a cast.
#[allow(clippy::wildcard_imports)]
use super::*;

enum WizardStep {
    Continue,
    Yield(Result<(), EngineError>),
}

enum GroupNarrow {
    Ready,
    Yielded,
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    fn narrow_wizard_same_controller_group(
        &mut self,
        wizard: &mut CastWizard,
        req: TargetReq,
        min: u32,
        options: &mut Vec<ObjectId>,
    ) -> GroupNarrow {
        let constrained = req.constraint()
            == Some(baylee_cards_dsl::TargetConstraint::SameController)
            || matches!(
                req.spec,
                TargetSpec::CardInGraveyard(_, baylee_cards_dsl::PlayerRel::Chosen)
            );
        if !constrained {
            return GroupNarrow::Ready;
        }
        if let Some(chosen) = wizard.target_group_player.take() {
            options.retain(|&id| eval::same_controller_group(&self.state, id) == Some(chosen));
            if matches!(
                req.spec,
                TargetSpec::CardInGraveyard(_, baylee_cards_dsl::PlayerRel::Chosen)
            ) && wizard.chosen_player.is_none()
            {
                wizard.chosen_player = Some(chosen);
            }
            return GroupNarrow::Ready;
        }
        let groups = eval::eligible_same_controller_groups(&self.state, options, min);
        match groups.as_slice() {
            [] => {
                options.clear();
                GroupNarrow::Ready
            }
            [only] => {
                options.retain(|&id| eval::same_controller_group(&self.state, id) == Some(*only));
                if matches!(
                    req.spec,
                    TargetSpec::CardInGraveyard(_, baylee_cards_dsl::PlayerRel::Chosen)
                ) && wizard.chosen_player.is_none()
                {
                    wizard.chosen_player = Some(*only);
                }
                GroupNarrow::Ready
            }
            _ => {
                self.pending = Pending::ChoosePlayer {
                    player: wizard.player,
                    options: groups,
                };
                self.cast_wizard = Some(wizard.clone());
                self.awaiting_answer = true;
                GroupNarrow::Yielded
            }
        }
    }

    pub(super) fn advance_cast_wizard_inner(&mut self) -> Result<(), EngineError> {
        loop {
            match self.advance_cast_wizard_step() {
                WizardStep::Continue => {}
                WizardStep::Yield(result) => return result,
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn advance_cast_wizard_step(&mut self) -> WizardStep {
        let Some(mut wizard) = self.cast_wizard.clone() else {
            return WizardStep::Yield(Ok(()));
        };
        let split = match wizard.option {
            Some(CastModeKind::EscapeGranted {
                face,
                modes,
                kicked,
                prototype,
                ..
            }) => Some((face, modes, kicked, prototype)),
            Some(CastModeKind::LibraryTopLife {
                face,
                modes,
                kicked,
                ..
            }) => Some((face, modes, kicked, false)),
            _ => None,
        };
        if let Some((face, modes, kicked, prototype)) = split {
            wizard.announced = wizard.option;
            wizard.kicked = kicked;
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .expect("wizard card known");
            wizard.option = Some(if prototype {
                CastModeKind::Prototype
            } else {
                casting::ordinary_face_kind(def, face, modes, kicked)
            });
            self.cast_wizard = Some(wizard.clone());
        }
        if wizard.option == Some(CastModeKind::Sneak) && wizard.sneak_parts.is_empty() {
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index));
            if let Some(sneak) = casting::sneak_cost_for(
                &self.state,
                wizard.player,
                wizard.card,
                def,
                wizard.face_index(),
            ) && !sneak.parts.is_empty()
            {
                wizard.sneak_parts = sneak.parts;
                self.cast_wizard = Some(wizard.clone());
            }
        }
        if wizard.option.is_some()
            && wizard.reduction.is_none()
            && (wizard.option != Some(CastModeKind::Emerge) || wizard.emerge_sacrifice.is_some())
        {
            wizard.reduction = Some(self.frozen_spell_reduction(&wizard));
            self.cast_wizard = Some(wizard.clone());
        }
        if let Some(CastModeKind::PlayLandFace(face)) = wizard.option {
            let Some(def) = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
            else {
                return WizardStep::Yield(Err(EngineError::IllegalAction("unknown land")));
            };
            if face > 0 {
                self.state.switch_face(wizard.card, def, face);
            }
            if let Err(err) =
                casting::play_land_by_effect(&mut self.state, wizard.player, wizard.card)
            {
                return WizardStep::Yield(Err(err.into()));
            }
            self.cast_wizard = None;
            if self.state.entries.contains(wizard.card) {
                self.suspend_nested_land_entry();
            } else {
                self.finish_nested_cast();
            }
            return WizardStep::Yield(Ok(()));
        }
        match wizard.stage {
            WizardStage::ManaPermission => WizardStep::Yield(self.ask_north_star(wizard)),
            WizardStage::Permission => WizardStep::Yield(self.choose_graveyard_permission(wizard)),
            WizardStage::ChooseMode => {
                self.pending = Pending::ChooseCastMode {
                    player: wizard.player,
                    object: wizard.card,
                    options: wizard.options.clone(),
                };
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::EmergeSacrifice => {
                if wizard.option != Some(CastModeKind::Emerge) || wizard.emerge_sacrifice.is_some()
                {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::XValue;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let face = self.wizard_face(&wizard);
                let Some(emerge) = face.emerge else {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "no emerge cost on this spell",
                    )));
                };
                let candidates = casting::emerge_candidates(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    face,
                    emerge,
                );
                let pool_only = !wizard.makes_mana_after_choices()
                    && !self.can_make_mana_for_cast(wizard.player);
                let options: Vec<ObjectId> = candidates
                    .into_iter()
                    .filter(|&(candidate, mv)| {
                        if !pool_only {
                            return true;
                        }
                        let mut probe = wizard.clone();
                        probe.emerge_sacrifice = Some(candidate);
                        let priced = self.emerge_option_cost(&probe, mv);
                        for opt in &mut probe.options {
                            if opt.kind == CastModeKind::Emerge {
                                opt.cost = priced;
                            }
                        }
                        probe.reduction = Some(self.frozen_spell_reduction(&probe));
                        self.wizard_pool_can_pay(&probe, wizard_total_cost(face, &probe))
                    })
                    .map(|(candidate, _)| candidate)
                    .collect();
                if options.is_empty() {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "nothing can pay this spell's emerge cost",
                    )));
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::CostSacrifice,
                    total: None,
                };
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::XValue => {
                if wizard.option == Some(CastModeKind::Emerge) && wizard.emerge_sacrifice.is_none()
                {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::EmergeSacrifice;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                if let Some(asked) = self.ask_phyrexian(&wizard) {
                    return WizardStep::Yield(asked);
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
                let face = self.wizard_face(&wizard);
                let disguised = wizard.option == Some(CastModeKind::Disguise) || wizard.masked;
                // X is asked when the cost has a variable OR a mandatory
                // part scales with it (Toxic Deluge's pay-X-life, SacrificeX, TapX, PayEnergyX).
                let scales_mandatory = !disguised && mandatory_scales_with_x(face);
                // A spell cast without paying its mana cost has exactly one
                // legal X, which is 0 (CR 107.3b), so a free cast is not
                // asked for it; a non-mana payment that scales with X still is.
                let waives_mana_cost = wizard.free
                    || wizard.option == Some(CastModeKind::Plot)
                    || wizard.alternative_cost == Some(CastModeKind::Plot);
                let needs_x = (cost.has_variable() && !waives_mana_cost) || scales_mandatory;
                if needs_x {
                    // The mana for X may still be made during the cast (CR
                    // 601.2g): a cast whose mana comes after its choices, and
                    // a cast off the pool by a seat with a source left to
                    // tap, both open a payment window when the total needs
                    // it, so X is bounded only by the number type, as an
                    // activation's is (`abilities/x_value.rs`). An answer
                    // the window then cannot pay is reversed there and says
                    // so (`GameEvent::CastCancelled`, CR 601.2h, 734.1).
                    // Life payments still cannot exceed payable life.
                    let more_mana = !wizard.makes_mana_after_choices()
                        && self.can_make_mana_for_cast(wizard.player);
                    let mut max = if cost.has_variable() && !waives_mana_cost && !more_mana {
                        self.x_resource_bound(&wizard)
                    } else {
                        u32::MAX
                    };
                    if scales_mandatory {
                        let life_x_count = face
                            .mandatory_additional_costs
                            .iter()
                            .filter(|p| matches!(p, CostPart::PayLifeX))
                            .count();
                        if life_x_count > 0 {
                            let life = self
                                .state
                                .life_payable(wizard.player)
                                .saturating_sub(wizard.reserved_life(face));
                            let avail = u32::try_from(life).unwrap_or(0)
                                / u32::try_from(life_x_count).unwrap_or(1);
                            max = max.min(avail);
                        }
                        let energy_x_count = face
                            .mandatory_additional_costs
                            .iter()
                            .filter(|p| matches!(p, CostPart::PayEnergyX))
                            .count();
                        if energy_x_count > 0 {
                            let avail = u32::from(
                                self.state.players[usize::from(wizard.player.get())].energy,
                            )
                            .saturating_sub(fixed_energy_cost(face, wizard.cost_choice()))
                                / u32::try_from(energy_x_count).unwrap_or(1);
                            max = max.min(avail);
                        }
                        if face.mandatory_additional_costs.iter().any(|p| {
                            matches!(
                                p,
                                CostPart::SacrificeX(_)
                                    | CostPart::TapX(_)
                                    | CostPart::DiscardX(_)
                                    | CostPart::ExileFromGraveyardX(_)
                                    | CostPart::RemoveAnyCounterX { .. }
                            )
                        }) {
                            let counter_cap =
                                super::super::cost_wizard::counter_remove::total_battlefield_counters(
                                    &self.state,
                                    wizard.player,
                                );
                            let cap = u32::try_from(
                                self.state
                                    .battlefield_view()
                                    .len()
                                    .max(
                                        self.state
                                            .zones
                                            .list(ZoneLocation::Hand(wizard.player))
                                            .len(),
                                    )
                                    .max(
                                        self.state
                                            .zones
                                            .list(ZoneLocation::Graveyard(wizard.player))
                                            .len(),
                                    ),
                            )
                            .unwrap_or(u32::MAX)
                            .max(counter_cap);
                            max = casting::greatest_affordable(max.min(cap), |x| {
                                casting::additional_payment_capacity_for_x(
                                    &self.state,
                                    wizard.player,
                                    wizard.card,
                                    face,
                                    wizard.cost_choice(),
                                    x,
                                    &[],
                                )
                                .is_some()
                            });
                        }
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
                        && ((face.x_mana_color.is_some() || face.x_mana_colors.is_some())
                            || (cost.has_variable()
                                && !waives_mana_cost
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
                    return WizardStep::Yield(Ok(()));
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::Kicker;
                self.cast_wizard = Some(wizard);
                WizardStep::Continue
            }
            WizardStage::DivideDamage => WizardStep::Yield(self.advance_damage_division(wizard)),
            WizardStage::Targets => {
                let Some(req) = self.wizard_target_req(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                let (min, max) = req.bounds(wizard.x);
                let max = self
                    .wizard_divided_amount(&wizard)
                    .map_or(max, |total| max.min(total));
                if min > max {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "not enough divided amount for minimum targets",
                    )));
                }
                let spec = req.spec;
                if matches!(spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::ChoosePlayer;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
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
                    return WizardStep::Continue;
                }
                let mut options = self.wizard_target_options(&wizard, &spec);
                let mut wizard = wizard;
                if matches!(
                    self.narrow_wizard_same_controller_group(&mut wizard, req, min, &mut options),
                    GroupNarrow::Yielded
                ) {
                    return WizardStep::Yield(Ok(()));
                }
                self.cast_wizard = Some(wizard.clone());
                let player_options = eval::target_player_options_from(
                    &self.state,
                    &spec,
                    wizard.player,
                    eval::live_context(&self.state, wizard.card),
                );
                // "Any target" is one set: a burn spell with every creature
                // hexproofed is still castable at a player's face, so the
                // count that has to reach `min` spans both halves.
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "not enough legal targets",
                    )));
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
                    return WizardStep::Continue;
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
                WizardStep::Yield(Ok(()))
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
                    wizard.stage = WizardStage::ExtraTargets;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                let reference = if matches!(
                    wizard.option,
                    Some(CastModeKind::Modes(_) | CastModeKind::RepeatedModes(_))
                ) {
                    self.wizard_modal_clause(&wizard, 1)
                        .and_then(|(_, is_second, _)| {
                            is_second.then(|| wizard.targets.first().copied()).flatten()
                        })
                } else {
                    wizard.targets.first().copied()
                };
                let mut options =
                    self.wizard_target_options_relative(&wizard, &req.spec, reference);
                let (min, max) = req.bounds(wizard.x);
                let mut wizard = wizard;
                if matches!(
                    self.narrow_wizard_same_controller_group(&mut wizard, req, min, &mut options),
                    GroupNarrow::Yielded
                ) {
                    return WizardStep::Yield(Ok(()));
                }
                self.cast_wizard = Some(wizard.clone());
                let player_options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    wizard.player,
                    eval::live_context(&self.state, wizard.card),
                );
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "not enough legal targets",
                    )));
                }
                if (options.is_empty() && player_options.is_empty()) || req.max == 0 {
                    // "Up to one" with nothing to point at: the empty answer
                    // is the only one, for the reason the first stage gives.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::ExtraTargets;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::ExtraTargets => {
                let clause_idx = 2 + wizard.extra_targets.len();
                let Some((_, is_second, req)) = self.wizard_extra_clause(&wizard, clause_idx)
                else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::DivideDamage;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                let reference = is_second
                    .then(|| wizard.clause_first_targets(clause_idx).0)
                    .flatten();
                let mut options =
                    self.wizard_target_options_relative(&wizard, &req.spec, reference);
                let (min, max) = req.bounds(wizard.x);
                let mut wizard = wizard;
                if matches!(
                    self.narrow_wizard_same_controller_group(&mut wizard, req, min, &mut options),
                    GroupNarrow::Yielded
                ) {
                    return WizardStep::Yield(Ok(()));
                }
                self.cast_wizard = Some(wizard.clone());
                let player_options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    wizard.player,
                    eval::live_context(&self.state, wizard.card),
                );
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "not enough legal targets",
                    )));
                }
                if (options.is_empty() && player_options.is_empty()) || req.max == 0 {
                    let mut wizard = wizard;
                    wizard.target_references.extra.push(Vec::new());
                    wizard
                        .extra_targets
                        .push((SmallVec::new(), baylee_core::ids::SeatSet::new()));
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
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
                WizardStep::Yield(Ok(()))
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
                let options = eval::target_player_options_from(
                    &self.state,
                    &spec,
                    wizard.player,
                    eval::live_context(&self.state, wizard.card),
                );
                // No player can be chosen (every opponent has hexproof, say):
                // the spell cannot take the step and cannot be cast (CR
                // 601.2c, 601.2), and an empty menu is a question with no
                // answer. `miracle_targets_available` reads the same list.
                if options.is_empty() {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "not enough legal targets",
                    )));
                }
                self.pending = Pending::ChoosePlayer {
                    player: wizard.player,
                    options,
                };
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Kicker => {
                let face = self.wizard_face(&wizard);
                let skip = face.additional_costs.is_empty()
                    || wizard.option == Some(CastModeKind::Disguise)
                    || wizard.masked
                    || matches!(
                        wizard.announced,
                        Some(
                            CastModeKind::EscapeGranted { .. }
                                | CastModeKind::LibraryTopLife { .. }
                        )
                    )
                    || (face.kicked_targets.is_some() && !wizard.free);
                if skip
                    || (face.additional_costs.len() == 1 && wizard.kicked)
                    || wizard.option == Some(CastModeKind::Kicked)
                {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Buyback;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let source = self.state.object(wizard.card).and_then(|o| {
                    o.card.map(|c| {
                        baylee_core::ids::AbilityRef::new(
                            c.index,
                            baylee_core::ids::AbilityRef::ADDITIONAL_COST,
                        )
                    })
                });
                if face.additional_costs.len() > 1 {
                    let count = face.additional_costs.len().min(8);
                    let mut idx = usize::from(wizard.kicker_cursor);
                    while idx < count {
                        let mut probe = wizard.clone();
                        probe.kicked = true;
                        probe.kicked_mask |= 1_u8 << idx;
                        if self.announced_cost_payable(&probe) {
                            let extra = &face.additional_costs[idx];
                            let cursor = u8::try_from(idx).expect("kicker index fits u8");
                            let mut wizard = wizard;
                            wizard.kicker_cursor = cursor;
                            self.cast_wizard = Some(wizard.clone());
                            self.pending = Pending::YesNo {
                                player: wizard.player,
                                prompt: YesNoPrompt::KickerOption {
                                    index: cursor,
                                    cost: extra.mana,
                                    parts: crate::choice::BuybackParts::of(extra.parts),
                                },
                                source,
                            };
                            self.awaiting_answer = true;
                            return WizardStep::Yield(Ok(()));
                        }
                        idx += 1;
                    }
                    let mut wizard = wizard;
                    wizard.kicker_cursor = u8::try_from(count).unwrap_or(u8::MAX);
                    wizard.stage = if self.wizard_needs_kicker_x(&wizard) {
                        WizardStage::KickerX
                    } else {
                        WizardStage::Buyback
                    };
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let mut probe = wizard.clone();
                probe.kicked = true;
                if !self.announced_cost_payable(&probe) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Buyback;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                self.pending = Pending::YesNo {
                    player: wizard.player,
                    prompt: YesNoPrompt::Kicker,
                    source,
                };
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::KickerX => {
                let face = self.wizard_face(&wizard);
                let more_mana = !wizard.makes_mana_after_choices()
                    && self.can_make_mana_for_cast(wizard.player);
                let kicker_has_mana_x =
                    face.additional_costs.iter().enumerate().any(|(idx, add)| {
                        (wizard.kicked_mask == 0
                            || (idx < 8 && (wizard.kicked_mask & (1_u8 << idx)) != 0))
                            && add.mana.has_variable()
                    });
                let mut max = if more_mana || !kicker_has_mana_x {
                    u32::MAX
                } else {
                    self.x_resource_bound(&wizard)
                };
                if super::wizard_mandatory_parts(face, wizard.cost_choice()).any(part_scales_with_x)
                {
                    let cap = u32::try_from(
                        self.state
                            .battlefield_view()
                            .len()
                            .max(
                                self.state
                                    .zones
                                    .list(ZoneLocation::Hand(wizard.player))
                                    .len(),
                            )
                            .max(
                                self.state
                                    .zones
                                    .list(ZoneLocation::Graveyard(wizard.player))
                                    .len(),
                            ),
                    )
                    .unwrap_or(u32::MAX);
                    let counter_cap =
                        super::super::cost_wizard::counter_remove::total_battlefield_counters(
                            &self.state,
                            wizard.player,
                        );
                    let life_cap = u32::try_from(self.state.life_payable(wizard.player))
                        .unwrap_or(0)
                        .max(u32::from(
                            self.state.players[usize::from(wizard.player.get())].energy,
                        ))
                        .max(counter_cap);
                    max = casting::greatest_affordable(max.min(cap.max(life_cap)), |x| {
                        casting::additional_payment_capacity_for_x(
                            &self.state,
                            wizard.player,
                            wizard.card,
                            face,
                            wizard.cost_choice(),
                            x,
                            &[],
                        )
                        .is_some()
                    });
                }
                if kicker_has_mana_x
                    && !wizard.makes_mana_after_choices()
                    && !more_mana
                    && (face.x_mana_color.is_some()
                        || face.x_mana_colors.is_some()
                        || self.pool_alone_pays_x(&wizard))
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Buyback => {
                let Some(question) = self.ask_buyback(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Bargain;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Bargain => {
                let Some(question) = self.ask_bargain(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Offspring;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Offspring => {
                let Some(question) = self.ask_offspring(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Casualty;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Casualty => {
                let Some(question) = self.ask_casualty(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Gift;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Gift => {
                if wizard.gift_recipient.is_some() {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Multikicker;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let Some(question) = self.ask_gift(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Multikicker;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Multikicker => {
                let Some(question) = self.ask_multikicker(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Squad;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Squad => {
                let Some(question) = self.ask_squad(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Replicate;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
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
                    wizard.stage = WizardStage::Splice;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                self.pending = Pending::ChooseNumber {
                    player: wizard.player,
                    min: 0,
                    max,
                    reason: NumberPrompt::Replicate { cost },
                };
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::EitherCost => {
                let face = self.wizard_face(&wizard);
                let mut wizard = wizard;
                while let (_, Some((first_cost, second_cost))) =
                    super::cost_parts::next_either_cost(face, wizard.cost_choice())
                {
                    let idx = wizard.either_decided;
                    let first_ok =
                        self.wizard_either_branch_payable(&wizard, face, true, first_cost);
                    let second_ok =
                        self.wizard_either_branch_payable(&wizard, face, false, second_cost);
                    match (first_ok, second_ok) {
                        (true, true) => {
                            let source = self
                                .state
                                .object(wizard.card)
                                .and_then(|o| o.card)
                                .map(|c| baylee_core::ids::AbilityRef::new(c.index, 0));
                            self.pending = Pending::YesNo {
                                player: wizard.player,
                                prompt: YesNoPrompt::ChooseFirstCostBranch { index: idx },
                                source,
                            };
                            self.cast_wizard = Some(wizard);
                            self.awaiting_answer = true;
                            return WizardStep::Yield(Ok(()));
                        }
                        (true, false) => {
                            if idx < 8 {
                                wizard.either_first_mask |= 1_u8 << idx;
                            }
                            if first_cost.mana != ManaCost::ZERO {
                                wizard.either_pays_mana = true;
                            }
                            wizard.either_decided = idx.saturating_add(1);
                        }
                        (false, true) => {
                            if second_cost.mana != ManaCost::ZERO {
                                wizard.either_pays_mana = true;
                            }
                            wizard.either_decided = idx.saturating_add(1);
                        }
                        (false, false) => {
                            self.cast_wizard = None;
                            return WizardStep::Yield(Err(EngineError::IllegalAction(
                                "neither branch of the additional cost can be paid",
                            )));
                        }
                    }
                }
                wizard.stage = WizardStage::PitchChoice;
                self.cast_wizard = Some(wizard);
                WizardStep::Continue
            }
            WizardStage::Splice => {
                let Some(question) = self.ask_splice(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Conspire;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Conspire => {
                let Some(question) = self.ask_conspire(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Targets;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                self.pending = question;
                self.awaiting_answer = true;
                WizardStep::Yield(Ok(()))
            }
            WizardStage::PitchChoice => {
                let parts = self.wizard_pitch_parts(&wizard);
                let filter = parts
                    .iter()
                    .filter_map(|part| match part {
                        CostPart::ExileFromHand(filter) => Some(*filter),
                        _ => None,
                    })
                    .nth(wizard.pitch.len());
                if let Some(filter) = filter {
                    let mut options =
                        casting::pitchable(&self.state, wizard.player, wizard.card, filter);
                    options.retain(|id| !wizard.pitch.contains(id));
                    options.retain(|id| {
                        let mut chosen = wizard.pitch.clone();
                        chosen.push(*id);
                        casting::pitch_costs_payable(
                            &self.state,
                            wizard.player,
                            wizard.card,
                            &parts,
                            &chosen,
                        )
                    });
                    if options.is_empty() {
                        self.cast_wizard = None;
                        return WizardStep::Yield(Err(EngineError::IllegalAction(
                            "no card to exile for the pitch cost",
                        )));
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
                    return WizardStep::Yield(Ok(()));
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::ReturnCreature;
                self.cast_wizard = Some(wizard);
                WizardStep::Continue
            }
            WizardStage::ReturnCreature => {
                if !matches!(
                    wizard.option,
                    Some(CastModeKind::BeamMeUp | CastModeKind::Sneak)
                ) {
                    wizard.stage = WizardStage::Behold;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let options = if wizard.option == Some(CastModeKind::Sneak) {
                    let face = self.wizard_face(&wizard);
                    let sneak = baylee_cards_dsl::Cost {
                        mana: ManaCost::ZERO,
                        parts: wizard.sneak_parts,
                    };
                    casting::sneak_candidates(&self.state, wizard.player, wizard.card, face, sneak)
                } else {
                    casting::beam_return_options(&self.state, wizard.player, wizard.card)
                };
                if options.is_empty() {
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        if wizard.option == Some(CastModeKind::Sneak) {
                            "no unblocked creature can pay sneak"
                        } else {
                            "no creature can pay beam me up"
                        },
                    )));
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Behold => {
                let face = self.wizard_face(&wizard);
                let Some(behold) = face.behold_or_pay else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                };
                let options =
                    casting::behold_options(&self.state, wizard.player, wizard.card, behold.filter);
                if options.is_empty() {
                    let mut wizard = wizard;
                    wizard.behold_paid_mana = true;
                    wizard.stage = WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Escape => WizardStep::Yield(self.ask_escape_payment(&wizard).map(|_| ())),
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
                    return WizardStep::Continue;
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Convoke => {
                let face = self.wizard_face(&wizard);
                // Two keywords tap permanents to pay, and they differ in
                // what they tap and in how much of the cost the taps may pay
                // (#229). A waterbend's taps exist only once its cost is paid
                // and pay that cost's generic mana alone (CR 701.67b). Convoke
                // may pay the spell's whole total cost (CR 702.51a): each
                // creature a coloured symbol of its colour or one generic
                // mana, so the bound is the coloured symbols the sources can
                // pay between them (`ManaCost::convoked_colored`) and the
                // generic delve has left. Improvise's artifacts pay generic
                // only (CR 702.126a). A bound the payment cannot honour is a
                // tap spent for nothing, as in the delve stage above.
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
                    let sources =
                        crate::casting::tap_to_pay_sources(&self.state, face, wizard.player);
                    let left = wizard_total_cost(face, &wizard).with_less_generic(
                        u32::try_from(wizard.delve_exiles.len()).unwrap_or(u32::MAX),
                    );
                    let (rest, colored) = left.convoked_colored(&crate::casting::tap_colors(
                        &self.state,
                        face,
                        &sources,
                    ));
                    let room = usize::try_from(colored)
                        .unwrap_or(usize::MAX)
                        .saturating_add(generic(&rest));
                    (sources, room)
                } else {
                    (Vec::new(), 0)
                };
                if sources.is_empty() || room == 0 {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Sacrifice;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::Sacrifice => {
                let face = self.wizard_face(&wizard);
                let parts = additional_card_costs_for_x(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    face,
                    wizard.cost_choice(),
                    wizard.x,
                );
                let tail =
                    casting::remaining_spell_card_costs(&self.state, &parts, &wizard.cost_objects);
                let Some(part) = tail.first() else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::RemoveCounters;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
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
                options.retain(|id| {
                    *id != wizard.card
                        && Some(*id) != wizard.emerge_sacrifice
                        && Some(*id) != wizard.returned
                        && !wizard.cost_objects.contains(id)
                });
                if matches!(part, CostPart::Forage) {
                    super::super::cost_wizard::prune_incomplete_forage_graveyard(
                        &self.state,
                        &mut options,
                    );
                }
                options.retain(|candidate| {
                    let mut chosen = wizard.cost_objects.to_vec();
                    chosen.push(*candidate);
                    casting::additional_payment_capacity_for_x(
                        &self.state,
                        wizard.player,
                        wizard.card,
                        face,
                        wizard.cost_choice(),
                        wizard.x,
                        &chosen,
                    )
                    .is_some()
                });
                if options.is_empty() {
                    self.cast_wizard = None;
                    return WizardStep::Yield(Err(EngineError::IllegalAction(
                        "nothing can pay this cost",
                    )));
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
                WizardStep::Yield(Ok(()))
            }
            WizardStage::RemoveCounters => {
                let face = self.wizard_face(&wizard);
                let all_parts: SmallVec<[CostPart; 4]> =
                    super::wizard_cost_parts(face, wizard.cost_choice())
                        .copied()
                        .collect();
                if !all_parts.iter().any(|p| {
                    matches!(
                        p,
                        CostPart::RemoveAnyCounter { .. } | CostPart::RemoveAnyCounterX { .. }
                    )
                }) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return WizardStep::Continue;
                }
                let card_parts = additional_card_costs_for_x(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    face,
                    wizard.cost_choice(),
                    wizard.x,
                );
                let mut excluded: SmallVec<[ObjectId; 4]> = SmallVec::new();
                if let Some(emerge) = wizard.emerge_sacrifice {
                    excluded.push(emerge);
                }
                if let Some(ret) = wizard.returned {
                    excluded.push(ret);
                }
                let mut cursor = 0_usize;
                for &part in &card_parts {
                    if cursor >= wizard.cost_objects.len() {
                        break;
                    }
                    if matches!(part, CostPart::Forage) {
                        if self
                            .state
                            .object(wizard.cost_objects[cursor])
                            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                        {
                            if !excluded.contains(&wizard.cost_objects[cursor]) {
                                excluded.push(wizard.cost_objects[cursor]);
                            }
                            cursor += 1;
                        } else {
                            cursor = cursor.saturating_add(3).min(wizard.cost_objects.len());
                        }
                    } else {
                        if matches!(
                            part,
                            CostPart::Sacrifice(_)
                                | CostPart::SacrificeX(_)
                                | CostPart::ExilePermanent(_)
                                | CostPart::ReturnToHand(_)
                        ) && !excluded.contains(&wizard.cost_objects[cursor])
                        {
                            excluded.push(wizard.cost_objects[cursor]);
                        }
                        cursor += 1;
                    }
                }
                let ctx = eval::live_context(&self.state, wizard.card);
                if let Some(chosen_obj) = wizard.pending_counter_object {
                    let kinds =
                        super::super::cost_wizard::counter_remove::valid_kinds_for_chosen_object(
                            &self.state,
                            wizard.player,
                            ctx,
                            (&all_parts, wizard.x),
                            &excluded,
                            &wizard.counter_removals,
                            chosen_obj,
                        );
                    match kinds.as_slice() {
                        [] => {
                            self.cast_wizard = None;
                            return WizardStep::Yield(Err(EngineError::IllegalAction(
                                "nothing can pay this cost",
                            )));
                        }
                        [only] => {
                            let mut wizard = wizard;
                            wizard.pending_counter_object = None;
                            wizard.counter_removals.push((chosen_obj, *only));
                            self.cast_wizard = Some(wizard);
                            return WizardStep::Continue;
                        }
                        _ => {
                            self.pending = Pending::ChooseCastMode {
                                player: wizard.player,
                                object: chosen_obj,
                                options: kinds
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, ck)| CastModeDesc {
                                        index: u32::try_from(index).unwrap_or(u32::MAX),
                                        kind: CastModeKind::Counter(ck),
                                        cost: ManaCost::ZERO,
                                    })
                                    .collect(),
                            };
                            self.awaiting_answer = true;
                            return WizardStep::Yield(Ok(()));
                        }
                    }
                }
                match super::super::cost_wizard::counter_remove::next_counter_step(
                    &self.state,
                    wizard.player,
                    ctx,
                    &all_parts,
                    wizard.x,
                    &excluded,
                    &wizard.counter_removals,
                ) {
                    super::super::cost_wizard::counter_remove::CounterStep::Complete => {
                        let mut wizard = wizard;
                        wizard.stage = WizardStage::Done;
                        self.cast_wizard = Some(wizard);
                        WizardStep::Continue
                    }
                    super::super::cost_wizard::counter_remove::CounterStep::Impossible => {
                        self.cast_wizard = None;
                        WizardStep::Yield(Err(EngineError::IllegalAction(
                            "nothing can pay this cost",
                        )))
                    }
                    super::super::cost_wizard::counter_remove::CounterStep::AutoPick(obj, ck) => {
                        let mut wizard = wizard;
                        wizard.counter_removals.push((obj, ck));
                        self.cast_wizard = Some(wizard);
                        WizardStep::Continue
                    }
                    super::super::cost_wizard::counter_remove::CounterStep::AskKind {
                        object,
                        ..
                    } => {
                        let mut wizard = wizard;
                        wizard.pending_counter_object = Some(object);
                        self.cast_wizard = Some(wizard);
                        WizardStep::Continue
                    }
                    super::super::cost_wizard::counter_remove::CounterStep::AskObject {
                        options,
                    } => {
                        self.pending = Pending::ChooseCards {
                            player: wizard.player,
                            options,
                            min: 1,
                            max: 1,
                            prompt: ChoicePrompt::CostRemoveCounter,
                            total: None,
                        };
                        self.awaiting_answer = true;
                        WizardStep::Yield(Ok(()))
                    }
                }
            }
            WizardStage::Done => WizardStep::Yield(self.cast_or_make_miracle_mana(wizard)),
        }
    }

    pub(in crate::engine) fn wizard_needs_kicker_x(&self, wizard: &CastWizard) -> bool {
        if !wizard.kicked
            || wizard.option == Some(CastModeKind::Kicked)
            || matches!(
                wizard.announced,
                Some(
                    CastModeKind::EscapeGranted { kicked: true, .. }
                        | CastModeKind::LibraryTopLife { kicked: true, .. }
                )
            )
        {
            return false;
        }
        let face = self.wizard_face(wizard);
        let disguised = wizard.option == Some(CastModeKind::Disguise) || wizard.masked;
        if (chosen_option_cost(wizard).has_variable() && !wizard.free)
            || (!disguised && mandatory_scales_with_x(face))
        {
            return false;
        }
        face.additional_costs.iter().enumerate().any(|(idx, add)| {
            (wizard.kicked_mask == 0 || (idx < 8 && (wizard.kicked_mask & (1_u8 << idx)) != 0))
                && (add.mana.has_variable() || add.parts.iter().any(part_scales_with_x))
        })
    }
}
