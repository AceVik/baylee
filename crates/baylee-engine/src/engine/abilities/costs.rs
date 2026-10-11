//! Activation prices, affordability and atomic cost payment.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// What an activated ability costs `player` right now: its printed cost
    /// less the generic mana its own printed reduction takes off ("This
    /// ability costs {1} less to activate for each legendary creature you
    /// control"). The offer and the activation both ask this, so an ability
    /// is never offered at one price and charged another.
    pub(crate) fn activation_price(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        reduction: Option<baylee_cards_dsl::CostReduction>,
    ) -> Cost {
        let off = casting::reduction_amount(&self.state, reduction, player, source);
        let increase = casting::activation_increase(&self.state, source);
        Cost {
            mana: cost.mana.with_more_generic(increase).with_less_generic(off),
            parts: cost.parts,
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
        cost: &Cost,
        settled: &[bool],
    ) -> bool {
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
            let life = 2 * i32::try_from(mask.count_ones()).unwrap_or(i32::MAX);
            let settled_cost = Cost {
                mana: cost.mana.with_phyrexian_settled(mask),
                parts: cost.parts,
            };
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
        // Mycosynth Lattice: mana spends as though it were any colour, so a
        // five-colour activation cost is payable off five Islands.
        casting::affordable(&self.state, player, pool, cost)
    }

    pub(crate) fn can_afford(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        what: casting::SpendFor,
    ) -> bool {
        if !self.can_pay_mana(player, what, &cost.mana) {
            return false;
        }
        for part in cost.parts {
            match part {
                // CR 302.6, second sentence: a creature's activated ability
                // with the tap or the untap symbol in its cost cannot be
                // activated unless the creature has been under its
                // controller's control since their most recent turn began.
                // The check belongs here and not in the cost payment: a
                // creature tapped to pay someone *else's* cost (convoke,
                // crew) is not activating an ability of its own, and
                // summoning sickness has never stopped that.
                CostPart::TapSelf | CostPart::UntapSelf => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if crate::combat::summoning_sick(&self.state, obj) {
                        return false;
                    }
                    // {T} needs it untapped. {Q} ought to need it tapped and
                    // does not — `docs/observed-faults.md` entry 53.
                    if matches!(part, CostPart::TapSelf) && obj.status.contains(Status::TAPPED) {
                        return false;
                    }
                }
                CostPart::DiscardRandom => {
                    let needed = cost
                        .parts
                        .iter()
                        .filter(|part| {
                            matches!(
                                part,
                                CostPart::DiscardRandom
                                    | CostPart::Discard(_)
                                    | CostPart::DiscardSelf
                            )
                        })
                        .count();
                    if self.state.zones.list(ZoneLocation::Hand(player)).len() < needed {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if !self.state.can_pay_life(player, i32::from(*n)) {
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
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_) => {
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
                // Arithmetic on the source, and nobody is asked anything:
                // 39 of the pool's 40 counter costs take the counters off
                // the permanent whose ability it is, so there is exactly one
                // legal answer and it is not a question. See
                // `CostPart::RemoveCounterSelf`.
                CostPart::RemoveCounterSelf { kind, n } => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if obj.counters.get(*kind) < *n {
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
                CostPart::SacrificeSelf
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::PayLifeX => {}
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
    pub(crate) fn pay_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        chosen: &[Vec<ObjectId>],
        x: u32,
    ) -> Result<crate::object::PaidRecord, EngineError> {
        if !cost.parts.iter().any(can_refuse_part_way) {
            return self.pay_cost_parts(player, source, cost, chosen, x);
        }
        let before = self.state.checkpoint();
        let paid = self.pay_cost_parts(player, source, cost, chosen, x);
        if paid.is_err() {
            self.state.roll_back(before);
        }
        paid
    }

    /// [`Self::pay_cost`] without the checkpoint: what it pays, part by part.
    #[allow(clippy::too_many_lines)] // one arm per `CostPart`, and the list is the point
    pub(super) fn pay_cost_parts(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        chosen: &[Vec<ObjectId>],
        x: u32,
    ) -> Result<crate::object::PaidRecord, EngineError> {
        let mut answers = chosen.iter();
        let mut paid = crate::object::PaidRecord::default();
        let subject_before = self.state.source_identity(source);
        let subject_seq = self.state.journal.last_seq();
        if !cost.mana.is_empty() {
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
            // what it is. Nothing that admits an ability carries a rider, so
            // what was spent needs no second look.
            let held_before = self.state.players[player.get() as usize].mana_pool.clone();
            paid.fixed_mana_cost = cost.mana.with_x(0);
            paid.mana_spending = casting::mana_spending(&self.state, player);
            casting::pay_mana_for(
                &mut self.state,
                player,
                casting::SpendFor::Ability(source),
                &mana,
            )
            .ok_or(EngineError::IllegalAction("not enough mana"))?;
            casting::record_mana_payment(
                &mut paid,
                &held_before,
                &self.state.players[player.get() as usize].mana_pool,
            );
        }
        let mut graveyard_batch = crate::graveyard_order::PaymentBatch::new(&self.state);
        for part in cost.parts {
            graveyard_batch.before_part(&mut self.state, part);
            if paid_by_the_casting_wizard(part) {
                continue;
            }
            match part {
                CostPart::TapSelf => {
                    self.state.set_tapped(source, true);
                    self.state.journal.record(GameEvent::ObjectTapped {
                        object: source,
                        cause: Cause::Cost,
                    });
                }
                CostPart::UntapSelf => {
                    self.state.set_tapped(source, false);
                }
                CostPart::DiscardRandom => {
                    let hand = self.state.zones.list(ZoneLocation::Hand(player));
                    if hand.is_empty() {
                        return Err(EngineError::IllegalAction("no card to discard at random"));
                    }
                    let at = self.state.rng.below(hand.len() as u64) as usize;
                    let card = self.state.zones.list(ZoneLocation::Hand(player))[at];
                    self.state.discard_card(card, player, Cause::Cost, None);
                }
                CostPart::DiscardSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.record_discard(source, player, None, true);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::SacrificeSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
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
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
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
                CostPart::ExileFromHand(_) | CostPart::PayLifeX => {}
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
                    if crate::replacement::remove_counters(&mut self.state, source, *kind, *n) < *n
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
                // `record_counters` and not `put_counters`, which is the
                // door that applies the doubling replacements. CR 614.16:
                // "if an effect would put one or more counters on a
                // permanent" applies to what the effect of a resolving spell
                // or ability puts there and to what another replacement puts
                // there — a cost is neither. Doubling Season is the pool's
                // only such replacement and prints exactly that wording, so
                // this is the measured answer and not a cautious one.
                CostPart::PutCounterSelf { kind, n } => {
                    crate::replacement::record_counters(&mut self.state, source, *kind, *n);
                }
                CostPart::RemoveCounterSelfX { kind } => {
                    let want = u16::try_from(x).unwrap_or(u16::MAX);
                    if crate::replacement::remove_counters(&mut self.state, source, *kind, want)
                        < want
                    {
                        return Err(EngineError::IllegalAction(
                            "not enough counters to pay the cost",
                        ));
                    }
                }
                // Preserve one answer per asking part, including an empty
                // answer to crew/saddle 0. A following cost owns its own answer.
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
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_) => {
                    let Some(&[card]) = answers.next().map(Vec::as_slice) else {
                        return Err(EngineError::IllegalAction(
                            "a cost that has to ask reached the payer unanswered",
                        ));
                    };
                    // "The sacrificed creature's mana value" (Birthing Pod)
                    // is read off the permanent as it last existed on the
                    // battlefield (CR 608.2h), so before it goes.
                    if matches!(part, CostPart::Sacrifice(_)) {
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
                    }
                    cost_wizard::pay(&mut self.state, player, part, card)?;
                }
            }
        }
        graveyard_batch.finish(&mut self.state);
        if let Some(before) = subject_before {
            self.note_mana_activation(before);
            paid.source_after_cost = crate::resolve::subjects::public_successor(
                &self.state,
                before,
                subject_seq,
                Cause::Cost,
            );
        }
        Ok(paid)
    }
}
