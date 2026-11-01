//! Announcing, choosing and paying for ordinary activated abilities.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Capture a granted ability's effects independently of its grantor. Cost
    /// choices, payment and paid-source identities use the ordinary activation path.
    pub(super) fn push_granted_ability(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        targets: SmallVec<[ObjectId; 2]>,
        granted: &crate::effects::GrantedAbility,
    ) -> ObjectId {
        let captured = self
            .activating_abilities
            .take()
            .filter(|(id, _)| *id == source);
        let object = self.state.object(source);
        let acquisition = granted.acquisition.unwrap_or_else(|| {
            captured.as_ref().map_or_else(
                || object.map_or(0, GameObject::rules_acquisition),
                |(_, list)| list.acquisition,
            )
        });
        let version = object.map(|object| object.version);
        let name = object.map_or(NameRef::new(0), |o| o.base.name);
        let card = object.and_then(|o| o.card).map(|c| c.index);
        let base = self.state.bare_base(name);
        let id = self.state.arena.insert_with(|id| {
            GameObject::new_ability_on_stack(
                id,
                player,
                AbilityLoc {
                    card,
                    index: granted.id,
                    source,
                },
                targets,
                base,
            )
        });
        if let Some(identity) = self.state.source_identity(id) {
            self.state.text_changes.set(identity, granted.text);
        }
        if let Some(object) = self.state.object_mut(id) {
            object.riders.push(crate::object::Rider::GrantedActivated);
            if acquisition != 0 {
                object
                    .riders
                    .push(crate::object::Rider::RulesAcquisition(acquisition));
            }
            if let Some(version) = version {
                object
                    .riders
                    .push(crate::object::Rider::AbilitySourceVersion(version));
            }
            object.target_req = match granted.definition {
                AbilityDef::Activated { targets, .. }
                | AbilityDef::ActivatedConditional { targets, .. }
                | AbilityDef::Loyalty { targets, .. } => targets,
                _ => None,
            };
        }
        self.synthetic_fx.insert(id, granted.effects);
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.capture_linked_references(id, granted.effects);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index: granted.id,
            controller: player,
        });
        id
    }

    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        self.start_activation_with_price(player, source, ability_index, targets, None)
    }

    #[allow(clippy::too_many_lines)] // activation is a staged checklist; extraction would obscure it
    pub(crate) fn start_activation_with_price(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        locked_price: Option<(
            baylee_core::mana::ManaCost,
            baylee_core::mana::ManaCost,
            u32,
            casting::SourceAllowance,
        )>,
    ) -> Result<(), EngineError> {
        // The player half of a choice this activation has already answered,
        // put here by `apply`. Read out before anything at all can return,
        // so that an activation refused further down cannot leave it
        // standing for the next one to inherit.
        let chosen_players = std::mem::take(&mut self.activation_target_players);
        self.activation_target_choice.controller = Some(player);
        // Prepared cast (`choice::PREPARED_CAST`): pay the linked
        // spell's cost, put a copy on the stack, unprepare the source.
        if ability_index == crate::choice::TURN_FACE_UP {
            return self.turn_face_up(player, source);
        }
        if let Some(half) = crate::choice::door_to_unlock(ability_index) {
            return self.unlock_door(player, source, half);
        }
        if ability_index == crate::choice::PREPARED_CAST {
            return self.start_prepared_cast(player, source);
        }
        // Karn's lock, ahead of every route below it. It used to sit inside
        // the printed-ability branch, which the granted and loyalty branches
        // return before reaching — so a locked artifact's *granted* ability
        // passed under it. Structural rather than observed: the offer guard
        // above refuses first, and no test gets this far.
        if self.artifact_activations_are_locked(source) {
            return Err(EngineError::IllegalAction(
                "activated abilities of artifacts can't be activated (Karn)",
            ));
        }
        // A granted ability follows the same announcements, cost choices and
        // payment as a printed one (CR 602.2b). Its effects are captured below
        // before paying can remove either the recipient or the grantor.
        let granted = if crate::choice::granted_slot(ability_index).is_some() {
            let granted = self
                .granted_definition(source, ability_index)
                .ok_or(EngineError::IllegalAction("no granted ability"))?;
            self.activating_grant = Some((source, ability_index, granted));
            Some(granted)
        } else {
            None
        };
        let granted_definition = granted.map(|grant| grant.definition);
        // Loyalty abilities route to their own activation path first.
        if let Some(cost) = self.loyalty_cost(source, ability_index) {
            return self.start_loyalty_activation(player, source, ability_index, targets, cost);
        }
        if self.announce_activation_mode(player, source, ability_index)? {
            return Ok(());
        }
        let (cost, effects, (first, second), mana_ability, zone, limit, cost_reduction) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            // Deliberately no `obj.card` here. The list this ability was
            // offered from is the object's *own* (`GameObject::abilities`),
            // which answers for a Treasure out of its definition and for a
            // token copy out of the rules text it copied — so demanding a
            // card refused an ability the engine had just offered, which is
            // the failure mode this engine treats as worse than either half
            // of it alone. The card index it used to read was never used
            // for anything: it was discarded again further down.
            //
            // The object's own list, which is what `legal_actions` indexed
            // when it offered this. Reading the card's instead was the
            // "two probes must agree" bug once more: a Glasspool Mimic
            // copying Werefox Bodyguard was offered the sacrifice ability
            // at index 1 and refused it with "no such ability", because
            // Glasspool Mimic's printed list is one entry long.
            let ability = granted_definition
                .as_ref()
                .or_else(|| obj.abilities(&self.lookup).get(ability_index as usize))
                .ok_or(EngineError::IllegalAction("no such ability"))?;
            if ability.is_intrinsic_mana_ability()
                && casting::intrinsic_mana_colors(&self.state, source).is_empty()
            {
                return Err(EngineError::IllegalAction("no basic land mana ability"));
            }
            let selected = match ability {
                AbilityDef::ModalActivated { modes, .. } => {
                    let mode = self
                        .activation_mode_index(source, ability_index)
                        .ok_or(EngineError::IllegalAction("activation mode not chosen"))?;
                    super::modes::selected_definition(ability, &modes[usize::from(mode)])
                }
                _ => *ability,
            };
            match &selected {
                AbilityDef::Activated {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    limit,
                    cost_reduction,
                    ..
                } => (
                    *cost,
                    *effects,
                    (*first, *second_targets),
                    *mana_ability
                        && baylee_cards_dsl::mana_rule::activated_mana_ability(
                            cost,
                            effects,
                            first.is_some() || second_targets.is_some(),
                        ),
                    *zone,
                    *limit,
                    *cost_reduction,
                ),
                AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    condition,
                    limit,
                    cost_reduction,
                    ..
                } => {
                    if !crate::eval::condition_holds_with_context(
                        &self.state,
                        player,
                        self.ability_rule_context(source, ability_index),
                        *condition,
                    ) {
                        return Err(EngineError::IllegalAction("activation condition not met"));
                    }
                    (
                        *cost,
                        *effects,
                        (*first, *second_targets),
                        *mana_ability
                            && baylee_cards_dsl::mana_rule::activated_mana_ability(
                                cost,
                                effects,
                                first.is_some() || second_targets.is_some(),
                            ),
                        *zone,
                        *limit,
                        *cost_reduction,
                    )
                }
                _ => return Err(EngineError::IllegalAction("not an activated ability")),
            }
        };
        // Classification belongs to the whole ability, including when the chosen
        // mode itself does not add mana.
        let mana_ability = if self.activation_mode.is_some() {
            self.activation_definition(source, ability_index)
                .is_some_and(|a| a.is_mana_ability())
        } else {
            mana_ability
        };
        if !mana_ability && self.non_mana_activations_are_locked(source) {
            return Err(EngineError::IllegalAction(
                "non-mana activated abilities of this permanent can't be activated",
            ));
        }
        // The price, determined once and before anything is paid (CR 601.2f
        // through CR 602.2b): Boseiju's channel discards the very card whose
        // count of legendary creatures lowers it, and a total cost is locked
        // in once determined. The offer asked the same helper, so the ability
        // is charged what it was offered at.

        // Read before the cost moves the card (CR 702.29c needs to know,
        // after the discard, that it paid a cycling ability's cost).
        let cycling = self.state.object(source).is_some_and(|o| {
            o.abilities(&self.lookup)
                .get(ability_index as usize)
                .is_some_and(AbilityDef::is_cycling)
        });
        // Read before any cost is paid, because a cost may move the source
        // and a moved copy is no longer one — see `Engine::activating_abilities`.
        self.activating_abilities = self.state.object(source).map(|o| {
            (
                source,
                o.ability_list(&self.lookup)
                    .with_base_text(self.state.text_changes.get(
                        baylee_core::ids::DamageSourceRef {
                            object: source,
                            version: o.version,
                        },
                    )),
            )
        });
        // Zone validation (battlefield abilities vs. hand abilities).
        let in_right_zone = match zone {
            ActivationZone::Battlefield => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player),
            ActivationZone::BattlefieldOwner => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.owner == player),
            ActivationZone::BattlefieldAnyPlayer => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            ActivationZone::BattlefieldOpponent => self.state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield && self.state.is_opponent(player, o.controller)
            }),
            ActivationZone::BattlefieldAttachedController => {
                self.state
                    .object(source)
                    .is_some_and(|o| o.zone == Zone::Battlefield)
                    && crate::eval::controller_of_attached(&self.state, source) == Some(player)
            }
            ActivationZone::Command => self.state.object(source).is_some_and(|o| {
                o.zone == Zone::Command && o.owner == player && !o.is_merged_component()
            }),
            ActivationZone::Hand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player)),
            ActivationZone::Graveyard => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Graveyard && o.zone_owner == Some(player)),
        };
        if !in_right_zone {
            return Err(EngineError::IllegalAction(
                "ability not usable from this zone",
            ));
        }
        let _ = zone;
        if self.announce_activation_x(player, source, ability_index, &cost)? {
            return Ok(());
        }
        let variable_mana = cost.mana.has_variable();
        let (mana, fixed_mana_cost, additional_life, allowance) =
            locked_price.unwrap_or_else(|| {
                let price = |x| {
                    self.activation_price(
                        player,
                        source,
                        ability_index,
                        &Cost {
                            mana: cost.mana.with_x(x),
                            parts: cost.parts,
                        },
                        cost_reduction,
                    )
                };
                let selected = price(self.activation_x.unwrap_or(0));
                (
                    selected.mana,
                    price(0).mana,
                    if mana_ability {
                        crate::mana_rules::life_price(&self.state, source)
                    } else {
                        0
                    },
                    selected.allowance,
                )
            });
        let cost = ActivationPrice {
            cost: Cost {
                mana,
                parts: cost.parts,
            },
            allowance,
        };
        // Each Phyrexian symbol: 2 life or its mana, announced beside the X
        // (CR 601.2b through CR 602.2b; CR 118.13a). Asked only where both
        // answers still leave a way to pay; where one does, it is taken
        // without asking, and where neither does the activation is refused.
        let symbols = cost.mana.phyrexian_count();
        while u32::try_from(self.activation_phyrexian.len()).unwrap_or(u32::MAX) < symbols {
            let mut with_life = self.activation_phyrexian.clone();
            with_life.push(true);
            let mut with_mana = self.activation_phyrexian.clone();
            with_mana.push(false);
            let life_ok =
                self.activation_affordable(player, source, &cost, &with_life, additional_life);
            let mana_ok =
                self.activation_affordable(player, source, &cost, &with_mana, additional_life);
            match (life_ok, mana_ok) {
                (true, true) => {
                    self.pending_plan = Some(PlanKind::ChoosePhyrexianLife {
                        source,
                        ability_index,
                    });
                    self.pending = Pending::YesNo {
                        player,
                        prompt: crate::choice::YesNoPrompt::PayLife { amount: 2 },
                        // The ability being activated, which is what the
                        // question is about.
                        source: self
                            .state
                            .printed_ability_list(source)
                            .and_then(|list| list.entry(ability_index as usize))
                            .and_then(|entry| entry.provenance.ability_ref())
                            .or_else(|| {
                                self.state.object(source).and_then(|o| o.card).map(|card| {
                                    baylee_core::ids::AbilityRef::new(card.index, ability_index)
                                })
                            }),
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                (true, false) => self.activation_phyrexian.push(true),
                (false, true) => self.activation_phyrexian.push(false),
                (false, false) => {
                    self.activation_phyrexian.clear();
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("cannot pay the cost"));
                }
            }
        }
        let by_life = self
            .activation_phyrexian
            .iter()
            .enumerate()
            .fold(0u32, |mask, (i, life)| mask | (u32::from(*life) << i));
        let phyrexian_life = 2 * i32::try_from(by_life.count_ones()).unwrap_or(i32::MAX);
        let activation_life = i32::try_from(additional_life)
            .ok()
            .and_then(|life| life.checked_add(phyrexian_life))
            .ok_or(EngineError::IllegalAction(
                "activation life cost exceeds available life",
            ))?;
        let cost = cost.with_mana(cost.mana.with_phyrexian_settled(by_life));
        // Targets, unless this activation has already answered them. That is
        // a flag and not a look at the lists, because an empty list is an
        // answer too: "up to one" answered with nothing re-enters here with
        // no objects and no players (CR 115.6). Reading the lists asked the
        // same question again forever. Before that it sent an ability whose
        // targets are all players back to the same question, because only
        // the objects were read.
        //
        // This used to be written twice — once here and once after the cost
        // was paid, identically — and the second copy was unreachable:
        // nothing between them touches `targets` or `target`. What it left
        // behind is a comment claiming the engine chooses targets after
        // paying, which is neither what it does nor what the rules say
        // (CR 601.2c chooses targets, CR 601.2h pays; an activation follows
        // the same order by CR 602.2b).
        if !self.activation_targets_answered
            && let Some(req) = first
        {
            // The count is read, and is no longer assumed to be one: "tap
            // two target lands" is two, and "up to one" is none or one
            // (CR 601.2c, by CR 602.2b). X is answered by now (CR 601.2b
            // comes first), so an X count is a number here.
            let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                self.activation_x,
                eval::target_domain::walls(first, second),
            );
            // "Target cards from a single graveyard" (Unlicensed Hearse):
            // which graveyard is asked first, when more than one holds a
            // card to choose, and the targets are then that graveyard's.
            if let baylee_cards_dsl::TargetSpec::CardInGraveyard(
                _,
                baylee_cards_dsl::PlayerRel::Chosen,
            ) = req.spec
            {
                let graveyard_of = |state: &crate::state::GameState, id: &ObjectId| {
                    state.object(*id).and_then(|o| o.zone_owner)
                };
                if let Some(chosen) = self.activation_graveyard.take() {
                    options.retain(|id| graveyard_of(&self.state, id) == Some(chosen));
                } else {
                    let mut graveyards: Vec<PlayerId> = options
                        .iter()
                        .filter_map(|id| graveyard_of(&self.state, id))
                        .collect();
                    graveyards.sort_unstable();
                    graveyards.dedup();
                    if graveyards.len() > 1 {
                        self.pending_plan = Some(PlanKind::ChooseActivationGraveyard {
                            source,
                            ability_index,
                        });
                        self.pending = Pending::ChoosePlayer {
                            player,
                            options: graveyards,
                        };
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                }
            }
            // Players are the other half of the same choice, and asking for
            // objects alone made three implemented lands dead: Nephalia
            // Drownyard, Duskmantle and Orzhova all say "target player",
            // whose object list is empty by construction — so each was
            // offered in `LegalActions` (which does add the two counts) and
            // then refused here with "no legal targets", the two-probes
            // disagreement this engine treats as the worst kind.
            let player_options = eval::target_player_options_from(
                &self.state,
                &req.spec,
                player,
                self.ability_rule_context(source, ability_index),
            );
            if options.len() + player_options.len() < min as usize {
                // The number was answered before this question was asked, so
                // an activation that dies here has one to throw away.
                self.activation_x = None;
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            if max == 0 || (options.is_empty() && player_options.is_empty()) {
                // The only answer is none at all, which is a legal one
                // (CR 115.6), so nobody is asked for it. The cast wizard
                // skips the same question the same way.
                self.activation_targets_answered = true;
            } else {
                let plan = PlanKind::ActivateAbility {
                    source,
                    ability_index,
                };
                let Some(chooser) = self.activation_target_chooser(req, player, plan.clone())?
                else {
                    return Ok(());
                };
                self.pending_plan = Some(plan);
                let max = max
                    .min(u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player: chooser,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        // The second instance of the word "target" (Contested Cliffs), asked
        // once the first has its answer and before anything is paid — both
        // are CR 601.2c, and the cost is CR 601.2h. Its options leave in what
        // the first instance chose, because CR 115.3 lets one object be
        // chosen once for each instance.
        if let Some(req) = second
            && self.activation_second_targets.is_none()
        {
            // A requirement naming the first target (Gauntlets of Chaos:
            // "that shares one of those types with it") reads it here.
            let mut context = self.ability_rule_context(source, ability_index);
            context.reference = targets.first().copied();
            let options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                context,
                self.activation_x,
                eval::target_domain::walls(first, second),
            );
            let player_options = eval::target_player_options_from(
                &self.state,
                &req.spec,
                player,
                self.ability_rule_context(source, ability_index),
            );
            if options.len() + player_options.len() < req.min as usize {
                self.activation_x = None;
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            if (options.is_empty() && player_options.is_empty()) || req.max == 0 {
                self.activation_second_targets = Some(SmallVec::new());
            } else {
                let plan = PlanKind::ActivateAbilitySecondTargets {
                    source,
                    ability_index,
                    targets,
                    target_players: chosen_players,
                };
                let Some(chooser) = self.activation_target_chooser(req, player, plan.clone())?
                else {
                    return Ok(());
                };
                self.pending_plan = Some(plan);
                let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
                let max = max
                    .min(u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player: chooser,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        // The mana is the payment window's question when this seat can still
        // make some (CR 601.2g, by 602.2b): only the rest is asked here, and
        // the window below is opened for the mana the pool does not cover.
        let deferred = variable_mana
            && !self.can_pay_activation_price(
                player,
                source,
                &cost.with_mana(cost.mana.with_x(self.activation_x.unwrap_or(0))),
            )
            && self.can_make_more_mana(player, source, ability_index);
        let planned = if deferred {
            cost.with_mana(baylee_core::mana::ManaCost::ZERO)
        } else {
            cost
        };
        if !self.can_plan_activation(player, source, &planned, activation_life)
            || !self.state.can_pay_life(player, activation_life)
        {
            self.activation_cost_choices.clear();
            self.activation_x = None;
            self.activation_phyrexian.clear();
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        // CR 601.2h, and the one step of it the player has to take: a cost
        // that says "sacrifice a creature" names no creature. Asked after
        // `can_afford` and never before it, so nobody is made to choose what
        // to give up for an activation that cannot happen — and asked one
        // part at a time, because a cost with two asking parts is two
        // questions and answering them together would lose which answer
        // belongs to which.
        let wanted = cost_wizard::answers_wanted(&cost);
        // X = 0 still supplies one (empty) answer for its cost part.
        while self.activation_cost_choices.len() < wanted
            && self.activation_x == Some(0)
            && matches!(
                cost_wizard::asking_parts(&cost).nth(self.activation_cost_choices.len()),
                Some(CostPart::SacrificeX(_) | CostPart::TapX(_))
            )
        {
            self.activation_cost_choices.push(Vec::new());
        }
        if self.activation_cost_choices.len() < wanted {
            let asked = self.activation_cost_choices.len();
            let part = cost_wizard::asking_parts(&cost)
                .nth(asked)
                .expect("fewer answers in hand than the cost has asking parts");
            let mut options = cost_wizard::menu(&self.state, player, source, &cost, part);
            // Nothing is paid until every question has an answer (CR 601.2h
            // pays the whole cost at once), so the board the second question
            // is asked of still holds the object the first one named. Taking
            // the answers so far off the menu is what stops a cost with two
            // sacrifice parts from eating one permanent twice. No card in
            // the pool prints such a cost today; this is one line and is
            // right for the one that does.
            options.retain(|id| {
                !self
                    .activation_cost_choices
                    .iter()
                    .any(|answer| answer.contains(id))
            });
            let prompt = cost_wizard::prompt(part);
            // `can_afford` asked the same question of the same function a
            // moment ago, so an empty list here is not a board this engine
            // can be in. Refusing rather than asserting all the same: this
            // is a rules path, and one process per game is the panic
            // boundary.
            if options.is_empty() && !matches!(part, CostPart::Crew(0) | CostPart::Saddle(0)) {
                self.activation_cost_choices.clear();
                self.activation_x = None;
                return Err(EngineError::IllegalAction("nothing can pay this cost"));
            }
            self.pending_plan = Some(PlanKind::PayActivationCost {
                source,
                ability_index,
                targets,
                target_players: chosen_players,
                remaining: match part {
                    CostPart::TapN { count, .. } => Some(*count),
                    CostPart::ExileFromSingleGraveyard { count, .. } => Some(u32::from(*count)),
                    CostPart::SacrificeX(_) | CostPart::TapX(_) => {
                        Some(self.activation_x.unwrap_or(0))
                    }
                    _ => None,
                },
                chosen: Vec::new(),
            });
            // One object per question, except crew's, whose one question
            // is answered with any number of creatures with total power N or
            // greater (CR 702.122a). The total is stated with each
            // creature's power beside it, so an answer short of it is
            // refused for a reason the question gives, and the offer above
            // (`can_afford`) asked the same sum of the same menu.
            let (max, total) = if let CostPart::Crew(power) | CostPart::Saddle(power) = part {
                let payment = cost_wizard::PowerPayment::new(&self.state, source, part);
                let weights = options.iter().map(|id| payment.weight(*id)).collect();
                (
                    u8::try_from(options.len()).unwrap_or(u8::MAX),
                    Some(crate::choice::CardTotal {
                        of: crate::choice::Measure::Power,
                        weights,
                        at_least: Some(i32::from(*power)),
                        at_most: None,
                    }),
                )
            } else if let CostPart::TapN { count, .. } = part {
                (u8::try_from(*count).unwrap_or(u8::MAX), None)
            } else if let CostPart::ExileFromSingleGraveyard { count, .. } = part {
                // A batch cannot state "all from one graveyard" when several
                // graveyards are offered. The first card chooses the graveyard;
                // PayActivationCost then narrows the remaining card menu to it.
                // Keep the existing batch when only one graveyard can pay.
                let first_owner = options
                    .first()
                    .and_then(|id| self.state.object(*id))
                    .and_then(|object| object.zone_owner);
                let several_graveyards = options.iter().any(|id| {
                    self.state.object(*id).and_then(|object| object.zone_owner) != first_owner
                });
                (
                    if several_graveyards {
                        (*count).min(1)
                    } else {
                        *count
                    },
                    None,
                )
            } else if matches!(part, CostPart::SacrificeX(_) | CostPart::TapX(_)) {
                (
                    u8::try_from(self.activation_x.unwrap_or(0)).unwrap_or(u8::MAX),
                    None,
                )
            } else {
                (1, None)
            };
            self.pending = Pending::ChooseCards {
                player,
                options,
                min: if matches!(
                    part,
                    CostPart::SacrificeX(_)
                        | CostPart::TapX(_)
                        | CostPart::TapN { .. }
                        | CostPart::ExileFromSingleGraveyard { .. }
                ) {
                    max
                } else {
                    u8::from(!matches!(part, CostPart::Crew(0) | CostPart::Saddle(0)))
                },
                max,
                prompt,
                total,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        if !self.can_pay_activation_price(
            player,
            source,
            &cost.with_mana(cost.mana.with_x(self.activation_x.unwrap_or(0))),
        ) {
            self.activation_target_players = chosen_players;
            return self.open_activation_payment(
                player,
                source,
                ability_index,
                targets,
                (
                    cost.mana.with_x(self.activation_x.unwrap_or(0)),
                    fixed_mana_cost,
                    additional_life,
                    cost.allowance,
                ),
            );
        }
        let answers = std::mem::take(&mut self.activation_cost_choices);
        // Taken rather than read, for `activation_cost_choices`' reason one
        // line up: the number belongs to this activation and to no other.
        let x = self.activation_x.take().unwrap_or(0);
        self.activation_targets_answered = false;
        self.activation_phyrexian.clear();
        let activated_source_version = self.state.object(source).map(|o| o.version);
        let activated_text = granted.map_or_else(
            || {
                self.activating_abilities.as_ref().map_or(
                    self.state.ability_text(source, ability_index),
                    |(_, list)| list.base_text(ability_index as usize),
                )
            },
            |grant| grant.text,
        );
        let circled_colors = if granted.is_some() {
            // An ability-only grant does not copy the donor's deck-building choices.
            baylee_core::color::ColorSet::EMPTY
        } else {
            self.activating_abilities.as_ref().map_or_else(
                || {
                    self.state
                        .printed_ability_list(source)
                        .map_or(baylee_core::color::ColorSet::EMPTY, |list| {
                            list.circled_colors(ability_index as usize)
                        })
                },
                |(_, list)| list.circled_colors(ability_index as usize),
            )
        };
        let acquisition = granted
            .and_then(|grant| grant.acquisition)
            .unwrap_or_else(|| {
                self.activating_abilities.as_ref().map_or_else(
                    || {
                        self.state
                            .object(source)
                            .map_or(0, GameObject::rules_acquisition)
                    },
                    |(_, list)| list.acquisition,
                )
            });
        let completion = super::payment::ActivationCompletion {
            player,
            source,
            ability_index,
            targets,
            chosen_players,
            cost,
            x,
            fixed_mana_cost,
            activation_life,
            activated_source_version,
            activated_text,
            circled_colors,
            acquisition,
            cycling,
            limit,
            effects,
            mana_ability,
            granted,
            second,
            first,
            mode_index: self.activation_mode.take().map(|mode| mode.mode),
        };
        let mut work =
            super::payment::CostPayment::new(&self.state, player, source, cost, answers, x);
        if !self.begin_cost_payment(&mut work)? {
            self.activation_payment = Some(super::payment::ActivationPayment::new(
                super::payment::ActivationPaymentKind::Ordinary(Box::new(
                    super::payment::OrdinaryPayment { work, completion },
                )),
            ));
            self.offer_counter_order();
            return Ok(());
        }
        self.complete_paid_activation(completion, work.paid);
        Ok(())
    }
    pub(super) fn record_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        version: Option<u32>,
        cost: &baylee_cards_dsl::Cost,
        ability: Option<ObjectId>,
    ) {
        self.state.journal.record(GameEvent::AbilityActivated {
            source: baylee_core::ids::DamageSourceRef {
                object: source,
                version: version.expect("activated source identity"),
            },
            ability: ability.and_then(|id| self.state.source_identity(id)),
            player,
            tap_cost: cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf),
            mana_ability: ability.is_none(),
        });
        // Capture before an immediate mana instruction changes its observers.
        self.capture_cost_triggers();
    }
}
