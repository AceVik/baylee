//! Activation prices, affordability and atomic cost payment.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

mod triggers;

impl<L: CardLookup> Engine<L> {
    pub(super) fn mana_activation_life(&self, source: ObjectId, index: u32) -> u32 {
        if self.state.effects.may_tax_mana_life()
            && self
                .activation_definition(source, index)
                .is_some_and(|ability| ability.is_mana_ability())
        {
            crate::mana_rules::life_price(&self.state, source)
        } else {
            0
        }
    }

    fn can_pay_random_discards(&self, player: PlayerId, cost: &Cost) -> bool {
        let needed = cost
            .parts
            .iter()
            .filter(|part| {
                matches!(
                    part,
                    CostPart::DiscardRandom | CostPart::Discard(_) | CostPart::DiscardSelf
                )
            })
            .count();
        self.state.zones.list(ZoneLocation::Hand(player)).len() >= needed
    }

    /// What an activated ability costs `player` right now: its printed cost
    /// less the generic mana its own printed reduction takes off ("This
    /// ability costs {1} less to activate for each legendary creature you
    /// control"). The offer and the activation both ask this, so an ability
    /// is never offered at one price and charged another.
    pub(crate) fn activation_price(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        cost: &Cost,
        reduction: Option<baylee_cards_dsl::CostReduction>,
    ) -> ActivationPrice {
        let (eligible, ordinary) = casting::activation_increases(&self.state, source);
        let increase = eligible.saturating_add(ordinary);
        let mut base_mana = cost.mana;
        if cost.parts.contains(&CostPart::PayAttachedManaCost)
            && let Some(attached) = self
                .state
                .object(source)
                .and_then(|o| o.attached_to)
                .and_then(|id| self.state.object(id))
        {
            base_mana = base_mana.combine(&attached.characteristics().mana_cost.with_x(0));
        }
        let full = base_mana.with_more_generic(increase);
        let mut mana = full;
        let off = if full.generic_total() > 0 {
            casting::reduction_amount(&self.state, reduction, player, source)
        } else {
            0
        };
        let equip = self
            .activation_definition(source, ability_index)
            .is_some_and(|ability| match ability {
                AbilityDef::Activated { effects, .. } => effects
                    .iter()
                    .any(|effect| matches!(effect, baylee_cards_dsl::Effect::AttachSelf { .. })),
                _ => false,
            });
        // Apply reductions with a local floor first, so they do not limit
        // an independent unrestricted reduction applied afterwards.
        if full.generic_total() > 0
            && self.state.effects.may_change_or_forbid_abilities()
            && let Some(object) = self.state.object(source)
        {
            for effect in self.state.effects.iter() {
                match effect.modifier {
                    baylee_cards_dsl::Modifier::AbilitiesCostLess { amount, minimum }
                        if crate::effects::applies_to(&self.state, effect, object) =>
                    {
                        // A minimum limits this reduction, never increases a free cost.
                        let reduction = amount.min(mana.cmc().saturating_sub(minimum));
                        mana = mana.with_less_generic(reduction);
                    }
                    baylee_cards_dsl::Modifier::EquipCostsLess(amount)
                        if crate::effects::applies_to(&self.state, effect, object)
                            && object.characteristics().subtypes.contains(
                                baylee_core::generated::subtypes::artifact::EQUIPMENT,
                            )
                            && equip =>
                    {
                        mana = mana.with_less_generic(amount);
                    }
                    _ => {}
                }
            }
        }
        mana = mana.with_less_generic(off);
        ActivationPrice {
            cost: Cost {
                mana,
                parts: cost.parts,
            },
            allowance: casting::base_allowance(&self.state, casting::SpendFor::Ability(source))
                .adjusted(
                    eligible,
                    ordinary,
                    full.generic_total() - mana.generic_total(),
                ),
        }
    }

    /// Whether `player` can pay an activation's `cost` some way that begins
    /// with the Phyrexian answers `settled` already gives: every way of
    /// paying the remaining Phyrexian symbols is tried, each paid with its
    /// mana or with 2 life (CR 107.4f), and the way counts when the pool
    /// covers the mana left and the player can pay the life (CR 119.4).
    /// A cost with no Phyrexian symbol is [`Self::can_afford`]'s question
    /// alone. The offer and every Phyrexian question ask this, so an ability
    /// is offered exactly when some answer to those questions pays for it.
    pub(crate) fn activation_affordable(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: &ActivationPrice,
        settled: &[bool],
        additional_life: u32,
    ) -> bool {
        let Ok(additional_life) = i32::try_from(additional_life) else {
            return false;
        };
        let symbols = cost.mana.phyrexian_count();
        // Eight symbols is 256 ways; no card prints more than four.
        let fixed = u32::try_from(settled.len()).unwrap_or(u32::MAX);
        if symbols > 8 || fixed > symbols {
            return false;
        }
        let base = settled
            .iter()
            .enumerate()
            .fold(0u32, |mask, (i, life)| mask | (u32::from(*life) << i));
        (0..(1u32 << (symbols - fixed))).any(|rest| {
            let mask = base | (rest << fixed);
            let Some(life) = (2 * i32::try_from(mask.count_ones()).unwrap_or(i32::MAX))
                .checked_add(additional_life)
            else {
                return false;
            };
            let settled_cost = cost.with_mana(cost.mana.with_phyrexian_settled(mask));
            self.state.can_pay_life(player, life)
                && self.can_plan_activation(player, source, &settled_cost, life)
        })
    }

    /// Whether `player`'s pool covers a bare mana cost paid for `what`.
    ///
    /// Split out of [`Self::can_afford`] for suspend, whose cost is a
    /// `ManaCost` and not a `Cost` — it has no parts to check. An offer that
    /// asked the same question a second way would be free to answer it
    /// differently, and this one is asked against the pool
    /// [`casting::pay_mana_for`] then spends: the plain counters plus the
    /// restricted mana `what` may use.
    pub(crate) fn can_pay_mana(
        &self,
        player: PlayerId,
        what: casting::SpendFor,
        cost: &baylee_core::mana::ManaCost,
    ) -> bool {
        // No mana symbol at all — `{T}`, a sacrifice, a granted "{T}: Add
        // one mana of any color" — is paid by every pool, the Lattice's
        // included (`mana_pay::payment_preferring` over no symbols pays
        // nothing and succeeds). Answered here because the offer asks it
        // once per permanent: the pool merge and the effect-table walk that
        // reads the Lattice below were most of what a board of granted mana
        // abilities cost.
        if cost.symbols().next().is_none() {
            return true;
        }
        let with_restricted = casting::spendable_pool(&self.state, player, what);
        let pool = with_restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        casting::source_affordable(&self.state, player, what, pool, cost, &[], None)
    }

    pub(super) fn can_pay_activation_price(
        &self,
        player: PlayerId,
        source: ObjectId,
        price: &ActivationPrice,
    ) -> bool {
        let what = casting::SpendFor::Ability(source);
        let planned = casting::spendable_pool(&self.state, player, what);
        let pool = planned
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
        casting::source_affordable(
            &self.state,
            player,
            what,
            pool,
            &price.mana,
            &[],
            Some(price.allowance),
        )
    }

    fn can_pay_commander_colors(&self, player: PlayerId) -> bool {
        !self.state.commanders[usize::from(player.get())].is_empty()
            && self
                .state
                .can_pay_life(player, i32::from(self.state.commander_colors(player).len()))
    }

    /// Source actions have separate requirements: exert only needs a
    /// battlefield permanent; tap symbols also require readiness and tap state.
    fn can_pay_source_action(&self, source: ObjectId, part: &CostPart) -> bool {
        self.state.object(source).is_some_and(|object| match part {
            CostPart::ExertSelf => object.zone == crate::zone::Zone::Battlefield,
            CostPart::TapSelf | CostPart::UntapSelf => {
                !crate::combat::summoning_sick(&self.state, object)
                    && object.status.contains(Status::TAPPED) == matches!(part, CostPart::UntapSelf)
            }
            _ => unreachable!("source action cost"),
        })
    }

    pub(super) fn source_cost_parts_ready(&self, source: ObjectId, cost: &Cost) -> bool {
        cost.parts.iter().all(|part| match part {
            CostPart::TapSelf | CostPart::UntapSelf | CostPart::ExertSelf => {
                self.can_pay_source_action(source, part)
            }
            CostPart::RemoveCounterSelf { kind, n } => self
                .state
                .object(source)
                .is_some_and(|obj| obj.counters.get(*kind) >= *n),
            CostPart::PayAttachedManaCost => self
                .state
                .object(source)
                .and_then(|o| o.attached_to)
                .and_then(|id| self.state.object(id))
                .is_some(),
            _ => true,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn can_afford(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        what: casting::SpendFor,
    ) -> bool {
        if !self.source_cost_parts_ready(source, cost) {
            return false;
        }
        if !self.can_pay_mana(player, what, &cost.mana) {
            return false;
        }
        for part in cost.parts {
            match part {
                CostPart::PayEnergy(_) => {
                    if u32::from(self.state.players[usize::from(player.get())].energy)
                        < cost.energy_payment(0)
                    {
                        return false;
                    }
                }
                CostPart::DiscardRandom => {
                    if !self.can_pay_random_discards(player, cost) {
                        return false;
                    }
                }
                CostPart::DiscardLastDrawnThisTurn => {
                    if self.state.last_drawn_in_hand(player).is_none() {
                        return false;
                    }
                }
                CostPart::PayCommanderColors => {
                    if !self.can_pay_commander_colors(player) {
                        return false;
                    }
                }
                CostPart::SacrificeX(_)
                | CostPart::TapX(_)
                | CostPart::TapN { .. }
                | CostPart::ExileFromSingleGraveyard { .. } => {
                    let count = match part {
                        CostPart::TapN { count, .. } => *count,
                        CostPart::ExileFromSingleGraveyard { count, .. } => u32::from(*count),
                        _ => self.activation_x.unwrap_or(0),
                    };
                    let count = usize::try_from(count).unwrap_or(usize::MAX);
                    if cost_wizard::menu(&self.state, player, source, cost, part).len() < count {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if !self.state.can_pay_life(player, i32::from(*n)) {
                        return false;
                    }
                }
                CostPart::ExileTopOfLibrary(_) => {
                    let needed: usize = cost
                        .parts
                        .iter()
                        .filter_map(|p| match p {
                            CostPart::ExileTopOfLibrary(n) => Some(usize::from(*n)),
                            _ => None,
                        })
                        .sum();
                    if self.state.zones.list(ZoneLocation::Library(player)).len() < needed {
                        return false;
                    }
                }
                // A pitch cost with nothing to pitch is not an offer. The
                // list is `casting::pitchable`, the same one the wizard's
                // `PitchChoice` stage puts in front of the player, because a
                // mode this accepts and that stage then refuses is a cast
                // that reverses itself under the player's hands.
                CostPart::ExileFromHand(filter) => {
                    if casting::pitchable(&self.state, player, source, filter).is_empty() {
                        return false;
                    }
                }
                // Crew (CR 702.122a): one question, so one total. The
                // creatures on the menu with a power above zero are the most
                // any answer can reach; a creature with a negative power
                // only lowers a total (CR 107.1b), so nobody would name it.
                CostPart::Crew(power) | CostPart::Saddle(power) => {
                    let menu = cost_wizard::menu(&self.state, player, source, cost, part);
                    if !cost_wizard::PowerPayment::new(&self.state, source, part)
                        .can_reach(&menu, *power)
                    {
                        return false;
                    }
                }
                // A cost that has to ask a question, asked of the board
                // instead of refused outright. `cost_wizard::options` is the
                // one reader of "what may pay this", and it is the same list
                // the player is shown a moment later — an offer whose
                // payment then finds nothing is the contradiction
                // `offer_tests` exists to catch.
                CostPart::Sacrifice(_)
                | CostPart::ExilePermanent(_)
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_)
                | CostPart::PutCounterOnOther { .. } => {
                    // **As many candidates as the cost asks questions.** A
                    // cost may print the same one more than once — Time
                    // Sieve's "Sacrifice five artifacts" is five
                    // `CostPart::Sacrifice` parts, one permanent and one
                    // question each — and each answer is a *different*
                    // permanent, because `cost_wizard` takes what was chosen
                    // out of the list before it asks again. Asking each
                    // occurrence whether the list is non-empty said yes to
                    // five sacrifices with one artifact on the table, and the
                    // payment then refused the second: the engine offered a
                    // thing and took it back, which is exactly the
                    // contradiction `offer_tests` exists to catch.
                    //
                    // Parts that are *equal* are counted, not parts of the
                    // same kind: two `Sacrifice` parts over different filters
                    // are two questions with two lists, and the overlap
                    // between them is a matching problem this does not
                    // pretend to solve. Nothing in the pool prints one.
                    let asked = cost.parts.iter().filter(|other| *other == part).count();
                    if cost_wizard::menu(&self.state, player, source, cost, part).len() < asked {
                        return false;
                    }
                }
                // The first four are paid off the source alone, so there is
                // nothing about the board to ask. `PayLifeX` is
                // [`paid_by_the_casting_wizard`]: accepted here and skipped
                // by `pay_cost`, which is right for a spell — the wizard caps
                // it at the caster's life — and would hand an activation half
                // its cost for free. Nothing in the pool prints one on an
                // activated ability, and
                // `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`
                // is what keeps it that way.
                //
                // `RemoveCounterSelfX` is in the same list for a different
                // reason and is **always affordable, deliberately**: zero is
                // a legal answer to "remove any number of storage counters",
                // so a storage land with nothing stored may still be tapped
                // for no mana at all — that is the card, not a hole. Nothing
                // loops on it because every cost that prints it prints `{T}`
                // or a mana part beside it, which is what
                // `baylee_ai::activate::consumes` reads.
                //
                // `PutCounterSelf` is the third reason in the same list: a
                // permanent can always take a counter, so there is nothing
                // to check. What bounds it is the counter, not the offer —
                // Devoted Druid's second -1/-1 leaves a 0/0 that CR 704.5f
                // puts in the graveyard, and a permanent in a graveyard is
                // offered nothing.
                CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::ExertSelf
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::DiscardHand
                | CostPart::SacrificeAll(_)
                | CostPart::SacrificeSelf
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::PayLifeX
                | CostPart::PayEnergyX
                | CostPart::PayAttachedManaCost => {}
            }
        }
        true
    }

    /// Pays a cost in the order the card prints it.
    ///
    /// `chosen` carries the answers to the parts that had to ask — a
    /// sacrifice, a discard — one per asking part and in the same order, put
    /// there by the stage in [`cost_wizard`] that asked. Two of the three
    /// callers pass an empty slice, and threading it is still the cheaper
    /// shape than the alternatives: paying those parts outside this function
    /// would either pay them out of the printed order or need a second payer
    /// beside this one, and a cost paid in two places is a cost that can be
    /// paid twice.
    ///
    /// Answers with what an effect of the ability may ask about the payment,
    /// for the ability to carry ([`crate::object::PaidRecord`]): the mana
    /// value of the permanent a `Sacrifice` part sacrificed, as it last
    /// existed (CR 608.2h), and the permanent a `TapOther` part tapped.
    ///
    /// # Errors
    /// [`EngineError::IllegalAction`] when the mana is not there, when a
    /// move refuses, or when an asking part has no answer left — the last of
    /// which means a caller paid without going through the stage that asks,
    /// and refusing is the point: the failure mode
    /// [`paid_by_the_casting_wizard`] documents is a part silently skipped,
    /// and a free sacrifice is worse than a refused activation.
    ///
    /// A refusal pays nothing. CR 732.1 reverses an action that cannot be
    /// completed and cancels "any payments already made", and a part that
    /// refuses after another was paid (a move of an object that is gone, a
    /// counter that is not there) used to leave the mana spent and the
    /// source tapped under an activation that never happened. The game is
    /// kept before the first part and put back on a refusal, whenever a part
    /// of the cost can refuse part way ([`can_refuse_part_way`]).
    ///
    /// The other half of 732.1, reversing the mana abilities activated while
    /// making the play, has nothing to undo here: this engine pays out of the
    /// pool, and the mana abilities that filled it were activated before the
    /// play began, so their mana stays in the pool, as it would.
    #[cfg(test)]
    pub(crate) fn pay_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        chosen: &[Vec<ObjectId>],
        x: u32,
    ) -> Result<crate::object::PaidRecord, EngineError> {
        let allowance =
            casting::live_allowance(&self.state, player, casting::SpendFor::Ability(source));
        self.pay_priced_cost(
            player,
            source,
            &ActivationPrice {
                cost: *cost,
                allowance,
            },
            chosen,
            x,
        )
    }

    pub(super) fn pay_priced_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &ActivationPrice,
        chosen: &[Vec<ObjectId>],
        x: u32,
    ) -> Result<crate::object::PaidRecord, EngineError> {
        if !cost.parts.iter().any(|part| {
            can_refuse_part_way(part) || matches!(part, CostPart::PutCounterSelf { .. })
        }) {
            return self.pay_cost_parts(player, source, cost, chosen, x);
        }
        let before = self.state.checkpoint();
        let scan_before = self.trigger_scan_seq;
        let queue_before = self.trigger_queue.len();
        let paid = self.pay_cost_parts(player, source, cost, chosen, x);
        if paid.is_err() {
            self.state.roll_back(before);
            self.trigger_scan_seq = scan_before;
            self.trigger_queue.truncate(queue_before);
        }
        paid
    }

    /// [`Self::pay_cost`] without the checkpoint: what it pays, part by part.
    #[allow(clippy::too_many_lines)] // one arm per `CostPart`, and the list is the point
    pub(super) fn pay_cost_parts(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &ActivationPrice,
        chosen: &[Vec<ObjectId>],
        x: u32,
    ) -> Result<crate::object::PaidRecord, EngineError> {
        let mut work = super::payment::CostPayment::new(
            &self.state,
            player,
            source,
            *cost,
            chosen.to_vec(),
            x,
        );
        if self.advance_cost_payment(&mut work)? {
            Ok(work.paid)
        } else {
            Err(EngineError::IllegalAction(
                "counter ordering requires a payment continuation",
            ))
        }
    }

    /// Resume at the first unpaid part; counter choices never replay a payment.
    pub(super) fn begin_cost_payment(
        &mut self,
        work: &mut super::payment::CostPayment,
    ) -> Result<bool, EngineError> {
        if !work.cost.parts.iter().any(can_refuse_part_way) {
            return self.advance_cost_payment(work);
        }
        // Announcement callers may consume a payment error and cancel normally.
        // Preserve the existing local transaction even when apply itself succeeds.
        let before = self.state.checkpoint();
        let scan_before = self.trigger_scan_seq;
        let queue_before = self.trigger_queue.len();
        let outcome = self.advance_cost_payment(work);
        if outcome.is_err() {
            self.state.roll_back(before);
            self.trigger_scan_seq = scan_before;
            self.trigger_queue.truncate(queue_before);
        }
        outcome
    }

    /// Continue an already begun payment without replaying its paid prefix.
    #[allow(clippy::too_many_lines)] // Exhaustive payment dispatch shares one cursor.
    pub(super) fn advance_cost_payment(
        &mut self,
        work: &mut super::payment::CostPayment,
    ) -> Result<bool, EngineError> {
        let player = work.player;
        let source = work.source;
        let cost = work.cost;
        let x = work.x;
        let mut answers = work.chosen[work.answer..].iter();
        let mut paid = std::mem::take(&mut work.paid);
        let subject_before = work.subject_before;
        let subject_seq = work.subject_seq;
        if !work.mana_paid && !cost.mana.is_empty() {
            // CR 107.3a, second half: while an activated ability is on the
            // stack, any X in its activation cost equals the announced
            // value. Without `with_x` the variable pip is worth nothing and
            // every X in an activation cost was free — a no-op for the cast
            // path, whose caller substitutes before it gets here, and the
            // whole of the cost for an activated one.
            let mana = cost.mana.with_x(x);
            // Out of the pool the offer read, restricted mana this ability
            // may spend included, and before any other part. A part that
            // sacrifices, discards or exiles the source would otherwise move
            // it before a restriction naming "abilities of creatures" reads
            // what it is. Freeze matching riders now; their delayed triggers
            // wait until a nonmana ability has reached the stack.
            let held_before = self.state.players[player.get() as usize].mana_pool.clone();
            paid.fixed_mana_cost = cost.mana.with_x(0);
            paid.mana_spending = casting::ability_mana_spending(&self.state, player, source);
            paid.spend_riders = casting::pay_mana_scoped(
                &mut self.state,
                player,
                casting::SpendFor::Ability(source),
                &mana,
                None,
                cost.allowance,
            )
            .ok_or(EngineError::IllegalAction("not enough mana"))?
            .into_iter()
            .filter(|(_, _, rider)| *rider != baylee_cards_dsl::SpendRider::None)
            .collect();
            casting::record_mana_payment(
                &mut paid,
                &held_before,
                &self.state.players[player.get() as usize].mana_pool,
            );
        }
        work.mana_paid = true;
        while let Some(part) = cost.parts.get(work.next) {
            work.next += 1;
            work.graveyard_batch.before_part(&mut self.state, part);
            if paid_by_the_casting_wizard(part) {
                continue;
            }
            match part {
                CostPart::TapSelf => {
                    if let (Some(src_ref), Some(tapped_ref)) =
                        (subject_before, self.state.source_identity(source))
                    {
                        self.state
                            .per_turn
                            .tapped_for_ability
                            .push((src_ref, tapped_ref));
                    }
                    self.state.set_tapped(source, true);
                    self.state.journal.record(GameEvent::ObjectTapped {
                        object: source,
                        cause: Cause::Cost,
                    });
                }
                CostPart::ExertSelf => {
                    let version = self.state.object(source).expect("exerted source").version;
                    let filter = crate::effects::EffectFilter::DslAbout(
                        &baylee_cards_dsl::Filter::And(&[
                            baylee_cards_dsl::Filter::Reference,
                            baylee_cards_dsl::Filter::ControlledByYou,
                        ]),
                        source,
                        version,
                    );
                    let timestamp = self.state.next_timestamp();
                    self.state
                        .effects
                        .register(crate::effects::ContinuousEffect {
                            id: baylee_core::ids::EffectId::new(0),
                            source: Some(source),
                            controller: player,
                            origin: crate::effects::EffectOrigin::Resolution,
                            layer: baylee_cards_dsl::Modifier::DoesNotUntap.layer(),
                            timestamp,
                            duration: baylee_cards_dsl::Duration::UntilYourNextUntapStep,
                            filter,
                            modifier: baylee_cards_dsl::Modifier::DoesNotUntap,
                        });
                }
                CostPart::UntapSelf => {
                    if self.state.set_tapped(source, false) {
                        self.state.journal.record(GameEvent::ObjectUntapped {
                            object: source,
                            cause: Cause::Cost,
                        });
                    }
                }
                CostPart::DiscardRandom => {
                    let hand = self.state.zones.list(ZoneLocation::Hand(player));
                    if hand.is_empty() {
                        return Err(EngineError::IllegalAction("no card to discard at random"));
                    }
                    let at = self.state.rng.below(hand.len() as u64) as usize;
                    let card = self.state.zones.list(ZoneLocation::Hand(player))[at];
                    if let Some(reference) = self.state.source_identity(card) {
                        paid.discarded.push(reference);
                    }
                    self.state.discard_card(card, player, Cause::Cost, None);
                }
                CostPart::DiscardLastDrawnThisTurn => {
                    let card =
                        self.state
                            .last_drawn_in_hand(player)
                            .ok_or(EngineError::IllegalAction(
                                "no card drawn this turn to discard",
                            ))?;
                    if let Some(reference) = self.state.source_identity(card) {
                        paid.discarded.push(reference);
                    }
                    self.state.discard_card(card, player, Cause::Cost, None);
                }
                CostPart::DiscardHand => {
                    let cards = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                    for card in cards {
                        if let Some(reference) = self.state.source_identity(card) {
                            paid.discarded.push(reference);
                        }
                        self.state.discard_card(card, player, Cause::Cost, None);
                    }
                }
                CostPart::DiscardSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    if let Some(reference) = self.state.source_identity(source) {
                        paid.discarded.push(reference);
                    }
                    self.state.record_discard(source, player, None, true);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                    self.state.record_discard_successor(source, player);
                }
                CostPart::SacrificeSelf => {
                    self.state.sacrifice_object(source, Cause::Cost)?;
                }
                CostPart::ReturnSelfToHand => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Hand(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::PayCommanderColors => {
                    if !self.can_pay_commander_colors(player) {
                        return Err(EngineError::IllegalAction(
                            "cannot pay commander identity life cost",
                        ));
                    }
                    let life = i32::from(self.state.commander_colors(player).len());
                    self.state.change_life(player, -life, Cause::Cost);
                }
                CostPart::SacrificeAll(_) => {
                    let cards = cost_wizard::options(&self.state, player, source, part);
                    for card in cards {
                        cost_wizard::pay(&mut self.state, player, part, card)?;
                    }
                }
                CostPart::SacrificeX(_) | CostPart::TapX(_) => {
                    let Some(cards) = answers.next() else {
                        return Err(EngineError::IllegalAction("unanswered counted cost"));
                    };
                    if cards.len() != x as usize {
                        return Err(EngineError::IllegalAction("wrong number of cost objects"));
                    }
                    let mut eligible = cost_wizard::options(&self.state, player, source, part);
                    eligible.sort_unstable();
                    let mut selected = cards.clone();
                    selected.sort_unstable();
                    if selected.windows(2).any(|pair| pair[0] == pair[1])
                        || selected
                            .iter()
                            .any(|id| eligible.binary_search(id).is_err())
                    {
                        return Err(EngineError::IllegalAction(
                            "a selected cost object is no longer payable",
                        ));
                    }
                    for &card in cards {
                        if matches!(part, CostPart::TapX(_))
                            && let (Some(src_ref), Some(tapped_ref)) =
                                (subject_before, self.state.source_identity(card))
                        {
                            self.state
                                .per_turn
                                .tapped_for_ability
                                .push((src_ref, tapped_ref));
                        }
                        let power = self
                            .state
                            .object(card)
                            .map(|o| i32::from(o.characteristics().power.unwrap_or(0)));
                        cost_wizard::pay(&mut self.state, player, part, card)?;
                        if matches!(part, CostPart::SacrificeX(_))
                            && let (Some(power), Some(version)) =
                                (power, self.state.object(card).map(|o| o.version))
                        {
                            paid.sacrificed_many.push((card, version, power));
                        }
                    }
                }
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
                }
                CostPart::ExileTopOfLibrary(n) => {
                    let count = usize::from(*n);
                    if self.state.zones.list(ZoneLocation::Library(player)).len() < count {
                        return Err(EngineError::IllegalAction(
                            "not enough cards in library to exile",
                        ));
                    }
                    for _ in 0..count {
                        let card = *self
                            .state
                            .zones
                            .list(ZoneLocation::Library(player))
                            .last()
                            .expect("library count checked");
                        let owner = self.state.object(card).map_or(player, |o| o.owner);
                        self.state.move_object(
                            card,
                            ZoneLocation::Exile(owner),
                            ZonePosition::Top,
                            Cause::Cost,
                        )?;
                    }
                }
                CostPart::ExileSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Exile(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                // Already skipped, by [`paid_by_the_casting_wizard`] above —
                // and *skipped* is the word, because an activation has no
                // wizard to pay them anywhere else. Named here rather than
                // swept into a `_` so that a new `CostPart` is still a
                // compile error in this match.
                CostPart::ExileFromHand(_) | CostPart::PayLifeX | CostPart::PayAttachedManaCost => {
                }
                // Through the removal door rather than off the object by
                // hand, because the journal entry and the projection
                // invalidation live there: a +1/+1 counter spent on Walking
                // Ballista's ping makes the creature smaller, and the
                // state-based action that then kills it reads a projection
                // this is what refreshes.
                //
                // Refusing an object with too few is `can_afford`'s job and
                // already done — but a cost is paid part by part, and a
                // part that *empties* the source (a sacrifice, an exile, a
                // bounce) may sit before this one in the printed order. So
                // the refusal is here too, after the door has said how many
                // it actually found, and it is a refusal rather than a
                // saturating shrug: paying two counters out of a permanent
                // holding one is not a discount.
                CostPart::RemoveCounterSelf { kind, n } => {
                    if crate::replacement::remove_counters_by(
                        &mut self.state,
                        source,
                        *kind,
                        *n,
                        Some(player),
                    ) < *n
                    {
                        return Err(EngineError::IllegalAction(
                            "not enough counters to pay the cost",
                        ));
                    }
                }
                // The number the player announced, through the same door as
                // the fixed one above — a removal takes no multiplier either
                // way (CR 614.16 is about counters being *put* on). The
                // refusal is the same too, and is not dead code: the bound
                // was read when the question was asked, and a counter can
                // leave in between (Thief of Blood in response).
                CostPart::PayEnergy(n) => {
                    if !self.state.pay_energy(player, u32::from(*n)) {
                        return Err(EngineError::IllegalAction("not enough energy"));
                    }
                }
                CostPart::PayEnergyX => {
                    if !self.state.pay_energy(player, x) {
                        return Err(EngineError::IllegalAction("not enough energy"));
                    }
                }
                // A cost is not initially an effect. Another replacement can
                // make effect-only doublers applicable (CR 614.16).
                CostPart::PutCounterSelf { kind, n } => {
                    crate::replacement::record_counters_by(
                        &mut self.state,
                        player,
                        source,
                        *kind,
                        *n,
                    );
                    if !crate::replacement::counter_order::has_pending(&self.state) {
                        self.state.refresh_characteristics();
                    }
                }
                CostPart::RemoveCounterSelfX { kind } => {
                    let want = u16::try_from(x).unwrap_or(u16::MAX);
                    if crate::replacement::remove_counters_by(
                        &mut self.state,
                        source,
                        *kind,
                        want,
                        Some(player),
                    ) < want
                    {
                        return Err(EngineError::IllegalAction(
                            "not enough counters to pay the cost",
                        ));
                    }
                }
                // Preserve one answer per asking part, including an empty
                // answer to crew/saddle 0. A following cost owns its own answer.
                CostPart::TapN { count, .. } => {
                    let selected = answers
                        .next()
                        .ok_or(EngineError::IllegalAction("unanswered tap group"))?;
                    let options = cost_wizard::menu(&self.state, player, source, &cost, part);
                    if selected.len() != usize::try_from(*count).unwrap_or(usize::MAX)
                        || selected
                            .iter()
                            .enumerate()
                            .any(|(i, id)| selected[..i].contains(id) || !options.contains(id))
                    {
                        return Err(EngineError::IllegalAction("invalid tap group"));
                    }
                    for &id in selected {
                        if let (Some(src_ref), Some(tapped_ref)) =
                            (subject_before, self.state.source_identity(id))
                        {
                            self.state
                                .per_turn
                                .tapped_for_ability
                                .push((src_ref, tapped_ref));
                        }
                        self.state.set_tapped(id, true);
                    }
                    for &id in selected {
                        self.state.journal.record(GameEvent::ObjectTapped {
                            object: id,
                            cause: Cause::Cost,
                        });
                    }
                }
                CostPart::ExileFromSingleGraveyard { count, .. } => {
                    let selected = answers.next().ok_or(EngineError::IllegalAction(
                        "unanswered graveyard exile group",
                    ))?;
                    let options = cost_wizard::menu(&self.state, player, source, &cost, part);
                    let first_owner = selected
                        .first()
                        .and_then(|id| self.state.object(*id))
                        .and_then(|o| o.zone_owner);
                    if selected.len() != usize::from(*count)
                        || selected.iter().enumerate().any(|(i, id)| {
                            selected[..i].contains(id)
                                || !options.contains(id)
                                || self.state.object(*id).and_then(|o| o.zone_owner) != first_owner
                        })
                    {
                        return Err(EngineError::IllegalAction("invalid graveyard exile group"));
                    }
                    for &id in selected {
                        cost_wizard::pay(&mut self.state, player, part, id)?;
                    }
                }
                CostPart::Crew(power) | CostPart::Saddle(power) => {
                    let Some(crew) = answers.next() else {
                        return Err(EngineError::IllegalAction("unanswered power cost"));
                    };
                    let payment = cost_wizard::PowerPayment::new(&self.state, source, part);
                    let total: i64 = crew.iter().map(|id| i64::from(payment.weight(*id))).sum();
                    if total < i64::from(*power) {
                        return Err(EngineError::IllegalAction("insufficient power for cost"));
                    }
                    for &creature in crew {
                        if let (Some(src_ref), Some(tapped_ref)) =
                            (subject_before, self.state.source_identity(creature))
                        {
                            self.state
                                .per_turn
                                .tapped_for_ability
                                .push((src_ref, tapped_ref));
                        }
                        cost_wizard::pay(&mut self.state, player, part, creature)?;
                    }
                }
                // The parts that had to ask, paid with the answers in the
                // order they were asked for, through the same doors as the
                // `SacrificeSelf`, `DiscardSelf` and `TapSelf` arms above —
                // so a sacrifice a player chose and one the card named
                // cannot come out as two different events, and neither can
                // a tap.
                CostPart::Sacrifice(_)
                | CostPart::ExilePermanent(_)
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_)
                | CostPart::PutCounterOnOther { .. } => {
                    let Some(&[card]) = answers.next().map(Vec::as_slice) else {
                        return Err(EngineError::IllegalAction(
                            "a cost that has to ask reached the payer unanswered",
                        ));
                    };
                    // "The sacrificed creature's mana value" (Birthing Pod)
                    // is read off the permanent as it last existed on the
                    // battlefield (CR 608.2h), so before it goes.
                    if matches!(
                        part,
                        CostPart::Sacrifice(_)
                            | CostPart::ExilePermanent(_)
                            | CostPart::ExileFromGraveyard(_)
                    ) {
                        paid.sacrificed = self.state.object(card).map(|o| (card, o.version));
                        paid.sacrificed_lki = self
                            .state
                            .object(card)
                            .map(crate::object::SacrificedLki::of);
                    }
                    // "The tapped creature" (station, CR 702.184a): which
                    // object, so its power can be read as the effect applies
                    // (CR 608.2h).
                    if matches!(part, CostPart::TapOther(_)) {
                        paid.tapped = self.state.object(card).map(|o| (card, o.version));
                        if let (Some(src_ref), Some(tapped_ref)) =
                            (subject_before, self.state.source_identity(card))
                        {
                            self.state
                                .per_turn
                                .tapped_for_ability
                                .push((src_ref, tapped_ref));
                        }
                    }
                    if matches!(part, CostPart::Discard(_))
                        && let Some(reference) = self.state.source_identity(card)
                    {
                        paid.discarded.push(reference);
                    }
                    cost_wizard::pay(&mut self.state, player, part, card)?;
                }
            }
            if crate::replacement::counter_order::has_pending(&self.state) {
                work.answer = work.chosen.len() - answers.len();
                work.paid = paid;
                return Ok(false);
            }
            // Detect events between cost parts while their subjects and Aura
            // attachments still exist; stack placement waits for priority.
            self.capture_cost_triggers();
        }
        work.graveyard_batch.finish_pending(&mut self.state);
        if let Some(before) = subject_before {
            self.note_mana_activation(before);
            paid.source_after_cost = crate::resolve::subjects::public_successor(
                &self.state,
                before,
                subject_seq,
                Cause::Cost,
            );
        }
        work.paid = paid;
        Ok(true)
    }
}
