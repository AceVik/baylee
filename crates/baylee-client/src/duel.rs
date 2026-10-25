//! The duel's methods: what the table reads off a `Duel`, and what a
//! host's frame or a player's answer does to it.

#[allow(clippy::wildcard_imports)] // the crate root's own vocabulary
use super::*;

impl Duel {
    /// The result, once the game has one.
    ///
    /// `Pending::GameOver` is the only question that is not a question: it
    /// carries the answer instead of asking for one, and three readers want
    /// it — the veil that darkens a table nobody is playing on any more, the
    /// end screen, and the prompt slip, which uses it to *stop* speaking.
    /// Written once here rather than matched three times.
    #[must_use]
    pub fn ending(&self) -> Option<&baylee_engine::win::GameResult> {
        match self.interaction.as_ref()?.pending() {
            Pending::GameOver(result) => Some(result),
            _ => None,
        }
    }

    /// The sentence the shelf speaks, if any: the client's own cast chooser
    /// while it stands, else the question being asked, else — with no
    /// question — the opening-hand wait (`Prompt::after_keeping`). Nothing
    /// once the game is over, which the end screen says instead.
    ///
    /// One method because two systems ask it: the ledge, which draws it, and
    /// the overlay, which rebuilds on it. Two copies of the chain would be
    /// two places for a third source to be added to one of.
    ///
    /// `texts` names the creatures a question is about in the player's
    /// language ([`crate::face::name_of`]); client-core decides which.
    #[must_use]
    pub fn headline(
        &self,
        lang: baylee_client_core::i18n::Lang,
        texts: &crate::cardtext::CardTexts,
    ) -> Option<String> {
        if self.ending().is_some() {
            return None;
        }
        if self.my_seat_eliminated() {
            return Some(
                baylee_client_core::i18n::Phrase::SeatEliminated
                    .text(lang)
                    .to_string(),
            );
        }
        let prompt = self
            .cast_menu
            .as_ref()
            .map(CastMenu::prompt)
            .or_else(|| {
                self.interaction
                    .as_ref()
                    .map(baylee_client_core::Interaction::prompt)
            })
            .or_else(|| {
                self.view
                    .as_ref()
                    .and_then(baylee_client_core::interaction::Prompt::after_keeping)
            })?;
        // Whose turn it is, for the one line that changes with it. A seat
        // holds priority on every turn at the table, so the bar has to be
        // told which one this is or it says "Your move" through the whole
        // game.
        let turn = self
            .view
            .as_ref()
            .map_or(baylee_client_core::Turn::Mine, |v| {
                baylee_client_core::Turn::of(v.active, v.seat)
            });
        let name = |id: baylee_core::ids::ObjectId| {
            let view = self.view.as_ref()?;
            Some(crate::face::name_of(view.object(id)?, view, texts))
        };
        if matches!(prompt, baylee_client_core::interaction::Prompt::Priority)
            && let Some(payment) = self
                .view
                .as_ref()
                .and_then(|v| v.owed.filter(|_| v.awaiting == Some(v.seat)))
        {
            return Some(baylee_client_core::interaction::payment_line(lang, payment));
        }
        Some(prompt.headline_naming_targets(
            lang,
            turn,
            self.statics.as_ref(),
            self.view.as_ref().is_some_and(|v| v.owed.is_some()),
            &name,
            &|source| {
                Some(crate::choices::target_label(
                    lang,
                    baylee_view::TargetRef::Object(source),
                    crate::choices::FaceNames {
                        view: self.view.as_ref(),
                        texts: Some(texts),
                    },
                    self.statics.as_ref(),
                ))
            },
        ))
    }

    /// Queues an action for the host.
    ///
    /// Queuing rather than sending directly keeps every mutation of the game on
    /// one system boundary, which is what lets input handlers stay plain
    /// functions of the board model.
    pub fn submit(&mut self, action: PlayerAction) {
        if !action.is_automation_setting()
            && self
                .interaction
                .as_ref()
                .is_some_and(|i| i.is_mine() && matches!(i.pending(), Pending::Priority { .. }))
        {
            self.stack_stop_requested = false;
        }
        if let PlayerAction::SetPriorityHold(hold) = &action {
            self.stack_stop_requested = matches!(
                hold,
                baylee_engine::choice::PriorityHold::UntilTopOfStack { .. }
                    | baylee_engine::choice::PriorityHold::Always
            );
            self.autopilot = None;
        }
        // Whatever was typed belonged to the question just answered.
        self.subtype_filter.clear();
        self.subtype_group = 0;
        // And so did the last refusal. Cleared here rather than when a new
        // question arrives, because the acting seat is re-sent its own
        // question every time anybody says anything — a refusal would have
        // flashed and been gone before it was read. It stands until this
        // player tries something else.
        self.last_error = None;
        // And so does a chime announcing a question this player is about to
        // answer. The standing orders and the autopilot answer in the same
        // half-frame that installs a question (`poll_host` → `run_autopilot`,
        // both in `DuelSet::Sync`, both ahead of the drain in
        // `DuelSet::Present`), so a question the player never saw makes no
        // sound. A player answering a question they *did* hear reaches this
        // line frames later, when the queue no longer holds it, and the call
        // is then the no-op it should be.
        self.cues.retract(baylee_client_core::cue::Cue::YourMove);
        // And a grant that only hands this action back says nothing
        // (DESIGN-v7 §4.3, rule 3).
        self.cues.note_own_action();
        // A "pay or …" answered yes opens the window "Don't pay" will have
        // to name the price of; any other answer to a yes-or-no forgets it.
        if let PlayerAction::YesNo(yes) = &action
            && let Some(Pending::YesNo { prompt, .. }) =
                self.interaction.as_ref().map(Interaction::pending)
        {
            self.pay_reason = (*yes
                && matches!(
                    prompt,
                    baylee_engine::choice::YesNoPrompt::PayPact { .. }
                        | baylee_engine::choice::YesNoPrompt::PayTax { .. }
                        | baylee_engine::choice::YesNoPrompt::PayMana { .. }
                ))
            .then_some(*prompt);
        }
        self.outbox.push(action);
    }

    /// Installs the question the host is asking.
    ///
    /// The line this draws is what everything in it obeys: state that belongs
    /// to the *previous question* is cleared, state that belongs to *this
    /// player* is not. The acting seat is re-sent its own question every time
    /// anybody at the table says anything, so a refusal or an armed deed
    /// dropped here would be dropped by the opponent pressing `F6`.
    ///
    /// A method rather than a match arm because that is the only way a test
    /// can ask what a re-sent question does.
    /// A new view of the table.
    ///
    /// Two things beyond storing it, and they are here rather than at the
    /// message loop for the reason [`Self::receive_choice`] is: a method is
    /// the only shape a test can ask a question of. Cards the engine is
    /// *showing* this seat live in no zone the table can draw, so a reveal
    /// opens the sheet that does — on the edge, never per frame. And a life
    /// total that moved exists only as the difference between this view and
    /// the last one, so it has to be read on the edge too: a frame later the
    /// previous total is gone.
    pub(crate) fn receive_view(&mut self, view: PlayerView) {
        self.stack_selected = self
            .stack_selected
            .filter(|id| view.stack.iter().any(|item| item.id == *id));
        // One reading of the difference, two things told about it: the number
        // over the bar and the sound in the room are the same event on the
        // same clock, which is what `Change::started` is for.
        let strikes = baylee_client_core::strike::between(self.view.as_ref(), &view);
        self.strikes.extend(strikes);
        let changes = self.life_flash.read(&view.seats);
        self.cues.note_life(&changes, view.seat);
        // The same edge, the second reading: a card that arrived in this hand
        // and a creature that grew exist only as the difference between this
        // view and the last, and a frame later the last one is gone. Unlike
        // the life ledger this one draws nothing, so it is read here and
        // nowhere else.
        let flow = self.tally.read(&view);
        self.cues.note_flow(&flow);
        // Two more readings on the same edge for the priority cue's policy
        // and the turn's click: whether the table moved by somebody else
        // since this seat last acted, and whether the turn passed.
        if baylee_client_core::cue::moved_by_another(self.view.as_ref(), &view) {
            self.cues.note_foreign();
        }
        self.cues.note_turn(view.active, view.turn);
        // The third reading on the same edge, and the one that is a *reset*
        // rather than a difference: the clock is restarted from what this
        // view says, and whether the sounds are re-armed is its own question
        // — see `DecisionClock::RESTART`, which tells a new question from a
        // correction by size, because the view carries no question identity.
        // The shelf counts this seat's own clock only (owner via the PM,
        // 08.10.2026): another seat's time stands beside its plate
        // (`seat_clocks`), and a second copy mid-shelf read as a stray. In
        // the mulligans `awaiting` is this seat while it still decides, so
        // the rule is the same there.
        let mine = view.awaiting == Some(view.seat);
        self.clock
            .sync(view.decision_remaining_ms.filter(|_| mine), mine);
        self.seat_clocks.sync(&view.clocks);
        self.lost_seats.sync(&view.lost);
        self.known_cards.extend(view.cards());
        // My turn beginning brings the camera home (DESIGN-v7 §2.4): an edge
        // read against the view before this one, never a state per frame.
        if baylee_client_core::tableview::comes_home(&baylee_client_core::tableview::HomeEdge {
            me: view.seat,
            active_before: self.view.as_ref().map(|v| v.active),
            active_now: Some(view.active),
            combat_question_opened: false,
        }) {
            self.visiting = None;
            self.follow_pending = None;
        }
        self.follow_the_turn(&view);
        self.table_mana = manasources::table_mana(&view);
        self.view = Some(view);
        let was_choosing = self.browser.for_choice();
        if let Some(v) = self.view.as_ref() {
            self.browser.saw_reveal(v);
        }
        self.clear_hover_for_new_browser(was_choosing);
        // The view and the question arrive in either order; capping only
        // ever lowers, so asking on both edges is safe.
        self.cap_x();
        self.refresh_owed_plan();
    }

    /// Caps a cast's X question at what this seat's own mana pays (#20).
    ///
    /// The engine bounds X by the mana already floating, but a cast whose
    /// mana is made in its payment window is bounded only by the number type
    /// there, so the question offered every X up to four billion. A player
    /// who announced more than the board pays had the cast reversed (CR
    /// 601.2h) or, worse, read the range as a promise. The cap is the
    /// planner's: the largest X the pool and the sources this seat could tap
    /// at its last priority pay ([`manaplan::greatest_x`]). Where nothing
    /// plans even X = 0 the sources are ones this client cannot read, and
    /// the engine's range stands.
    ///
    /// [`manaplan::greatest_x`]: baylee_client_core::manaplan::greatest_x
    fn cap_x(&mut self) {
        let Some(cap) = self.x_cap() else {
            return;
        };
        if let Some(interaction) = self.interaction.as_mut() {
            interaction.cap_number(cap);
        }
    }

    fn x_cap(&self) -> Option<u32> {
        let interaction = self.interaction.as_ref()?;
        let view = self.view.as_ref()?;
        let Pending::ChooseNumber {
            player,
            reason: baylee_engine::choice::NumberPrompt::X,
            ..
        } = interaction.pending()
        else {
            return None;
        };
        if *player != view.seat {
            return None;
        }
        let card = view.casting?;
        let legal = self.priority_legal.as_ref()?;
        let cost = spell_cost(view, legal, card)?;
        let pool = view
            .seat(baylee_client_core::decision::resource_player(view))?
            .mana_pool;
        baylee_client_core::manaplan::greatest_x(&cost, &pool, &manasources::sources(view, legal))
    }

    /// Works out afresh which lands would settle an open payment window.
    ///
    /// Both edges call it and neither is redundant: a view with no pending
    /// yet has an `owed` and no mana abilities to pay it with, and a pending
    /// arriving against a stale view would plan against the wrong pool.
    ///
    /// `owed` is the **total** and this passes it whole, because
    /// `manaplan::plan` spends the pool first by its own contract — so the
    /// plan is already for the remainder and nothing here subtracts anything.
    /// A second arithmetic would be a second thing to keep in step with the
    /// pool, which is the reason the view carries the total in the first
    /// place.
    pub(crate) fn refresh_owed_plan(&mut self) {
        self.keep_staging();
        self.owed_plan = self.compute_owed_plan();
    }

    /// The fixed cost this seat's own open window owes, if one is open.
    fn owed_cost(&self) -> Option<baylee_core::mana::ManaCost> {
        let view = self.view.as_ref()?;
        // Only while this seat is the one being asked. `owed` names what the
        // *awaited* seat owes, and the pair is one sentence.
        if view.awaiting != Some(view.seat) {
            return None;
        }
        match view.owed? {
            baylee_core::mana::ManaPayment::Fixed(cost) => Some(cost),
            baylee_core::mana::ManaPayment::AnyAmount { .. } => None,
        }
    }

    /// Keeps the staging in step with the window it is for: emptied when the
    /// window closes or another opens, and rid of every tap the engine no
    /// longer offers. A refused payment hands the same window back with the
    /// same sources, so what the player staged survives it.
    fn keep_staging(&mut self) {
        let Some(cost) = self.owed_cost() else {
            if self.pay_window.take().is_some() {
                self.pay_reason = None;
            }
            self.pay_staged.clear();
            return;
        };
        if self.pay_window != Some(cost) {
            self.pay_window = Some(cost);
            self.pay_staged.clear();
        }
        if let Some(legal) = self
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
        {
            self.pay_staged.keep_offered(legal);
        }
    }

    /// The pool, and what this seat could tap, for the payment it owes.
    fn payment_inputs(
        &self,
    ) -> Option<(
        baylee_core::mana::ManaCost,
        baylee_view::ManaPoolView,
        Vec<baylee_client_core::manaplan::Source>,
    )> {
        let cost = self.owed_cost()?;
        let view = self.view.as_ref()?;
        let pool = view
            .seat(baylee_client_core::decision::resource_player(view))?
            .mana_pool;
        let sources = self
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .map(|legal| manasources::sources(view, legal))
            .unwrap_or_default();
        Some((cost, pool, sources))
    }

    /// What "Pay now" would stage: the taps that pay what the pool and the
    /// staged taps leave owing, over the sources not staged yet.
    fn compute_owed_plan(&self) -> Option<baylee_client_core::manaplan::Plan> {
        self.interaction.as_ref()?.legal_actions()?;
        let (cost, pool, sources) = self.payment_inputs()?;
        self.pay_staged.suggest(&cost, &pool, &sources)
    }

    /// What the window still owes once the pool and the staged taps have
    /// paid what they can; empty when they cover it. `None` outside this
    /// seat's own fixed-cost window.
    #[must_use]
    pub fn payment_remaining(&self) -> Option<Vec<baylee_core::mana::ManaSymbol>> {
        let (cost, pool, sources) = self.payment_inputs()?;
        Some(self.pay_staged.remaining(&cost, &pool, &sources))
    }

    /// Whether the pool and the staged taps pay what is owed, so that Go is
    /// the answer rather than "Pay now".
    #[must_use]
    pub fn staged_covers(&self) -> bool {
        self.payment_inputs()
            .is_some_and(|(cost, pool, sources)| self.pay_staged.covers(&cost, &pool, &sources))
    }

    /// Stages a tap in the open window, or takes it back when its permanent
    /// is staged already. Nothing is sent. Returns whether anything changed.
    pub fn stage(&mut self, step: baylee_client_core::manaplan::Step) -> bool {
        if !self.paying() || self.mana_run.is_some() {
            return false;
        }
        self.pay_staged.toggle(step);
        self.last_error = None;
        self.owed_plan = self.compute_owed_plan();
        true
    }

    /// Stages a granted mana action in the open window, or takes it back.
    /// Nothing is sent. Returns whether the window took it.
    pub fn stage_granted(&mut self, granted: baylee_client_core::paystage::GrantedMana) -> bool {
        if !self.paying() || self.mana_run.is_some() {
            return false;
        }
        self.pay_staged.toggle_granted(granted);
        self.last_error = None;
        self.owed_plan = self.compute_owed_plan();
        true
    }

    /// Takes back the staged tap of `source`, if there is one.
    pub fn unstage(&mut self, source: ObjectId) -> bool {
        if !self.pay_staged.unstage(source) {
            return false;
        }
        self.last_error = None;
        self.owed_plan = self.compute_owed_plan();
        true
    }

    /// Takes back every staged tap (`Esc`, the shelf's undo). Returns whether
    /// anything was staged.
    pub fn unstage_all(&mut self) -> bool {
        if self.pay_staged.is_empty() {
            return false;
        }
        self.pay_staged.clear();
        self.last_error = None;
        self.owed_plan = self.compute_owed_plan();
        true
    }

    /// "Pay now": stages the plan for whatever the pool and the staged taps
    /// leave owing, visibly and without sending anything. Returns whether a
    /// plan was staged.
    pub fn stage_owed_plan(&mut self) -> bool {
        let Some(plan) = self.owed_plan.clone().filter(|p| !p.is_empty()) else {
            return false;
        };
        self.pay_staged.extend(&plan);
        self.last_error = None;
        self.owed_plan = self.compute_owed_plan();
        true
    }

    /// Go: sends the staged taps, in staging order, as one
    /// `PlayerAction::PayWindow`. The engine applies them all and settles
    /// the window, or refuses the whole of it in one sentence and hands the
    /// same window back — with the staging still here to correct.
    pub fn send_payment(&mut self) -> bool {
        if !self.paying() || self.mana_run.is_some() {
            return false;
        }
        let Some(steps) = self
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|legal| self.pay_staged.wire(legal))
        else {
            self.last_error = Some(Refusal::Said(Phrase::PlanLandGone));
            return false;
        };
        self.submit(PlayerAction::PayWindow { steps });
        true
    }

    /// What declining this window costs, as far as this client knows: a
    /// cast is taken back, a pact's debt loses the game. `None` where the
    /// effect's other branch is the card's to say.
    #[must_use]
    pub fn declining_costs(&self) -> Option<Phrase> {
        if !self.paying() {
            return None;
        }
        if self.view.as_ref().is_some_and(|v| v.casting.is_some()) {
            return Some(Phrase::DeclinePaymentCast);
        }
        match self.pay_reason {
            Some(baylee_engine::choice::YesNoPrompt::PayPact { .. }) => Some(Phrase::DeclinePact),
            _ => None,
        }
    }

    /// Whether this seat is in a payment window it owes a fixed amount in
    /// (CR 605.3a): a miracle's cost, a tax, a pact, a "you may cast it".
    #[must_use]
    pub fn paying(&self) -> bool {
        self.view.as_ref().is_some_and(|v| {
            v.awaiting == Some(v.seat)
                && matches!(v.owed, Some(baylee_core::mana::ManaPayment::Fixed(_)))
        })
    }

    /// The spell this seat may take back now (CR 732): the view's
    /// `casting`, which the engine names only in the caster's own view and
    /// only through the cast's own questions and its payment window, while
    /// this seat is the one asked. A cast an effect made, or a mana
    /// ability's colour asked inside the window, names none.
    #[must_use]
    pub fn cancellable_cast(&self) -> Option<ObjectId> {
        self.interaction.as_ref().filter(|i| i.is_mine())?;
        self.view.as_ref().and_then(|v| v.casting)
    }

    /// Takes the cast back (`PlayerAction::CancelCast`), and with it the run
    /// that was paying for it or waiting on its question: the engine puts
    /// the card, the lands and the floating mana back as they were, and a
    /// run that outlived it would tap them again. Nothing if no cast is
    /// cancellable now.
    pub fn cancel_cast(&mut self) {
        if self.cancellable_cast().is_none() {
            return;
        }
        self.mana_run = None;
        self.armed = None;
        self.cast_answer = None;
        self.submit(PlayerAction::CancelCast);
    }

    /// What a confirm press does in a payment window, by key or by button.
    ///
    /// Go when the pool and the staged taps cover what is owed
    /// ([`Self::send_payment`]); otherwise "Pay now", which stages the plan
    /// for the rest and sends nothing ([`Self::stage_owed_plan`]), so the
    /// next press is Go. With nothing that plans, it says so and sends
    /// nothing: passing the window short refuses the payment (#01), and
    /// declining is the other button's.
    pub fn confirm_payment(&mut self) {
        if !self.paying() || self.mana_run.is_some() {
            return;
        }
        if self.staged_covers() {
            self.send_payment();
        } else if !self.stage_owed_plan() {
            self.last_error = Some(Refusal::Said(Phrase::PaymentShort));
        }
    }

    /// What this client is proposing, if anything.
    ///
    /// An armed deed wins: it is a commitment the player made, and a payment
    /// window that opened underneath it does not get to relight the board.
    /// The engine withdraws the arming path's own option anyway, and every
    /// reader re-resolves an `Armed` before it fires one.
    #[must_use]
    pub fn proposing(&self) -> Proposing<'_> {
        if let Some(armed) = self.armed.as_ref() {
            return Proposing::Armed(armed);
        }
        if self.paying() && (self.owed_plan.is_some() || !self.pay_staged.is_empty()) {
            Proposing::Owed {
                plan: self.owed_plan.as_ref(),
                staged: self.pay_staged.steps(),
            }
        } else {
            Proposing::Nothing
        }
    }

    /// Who is offering this seat `object`, in the two lights of [`Reach`].
    ///
    /// The one predicate behind both drawings of a pile card, the table's and
    /// the zone browser's, so the two cannot light the same card differently.
    /// The engine's offer is its three lists — a land to play, a spell to
    /// cast, an ability to activate — because a pile card is not a
    /// permanent, and `activatable` alone would leave the Opt that Snapcaster
    /// Mage made castable dark once its `{U}` was floating.
    #[must_use]
    pub fn reach_of(&self, object: ObjectId) -> Option<Reach> {
        let offered = self.activatable.contains(&object)
            || self
                .interaction
                .as_ref()
                .and_then(Interaction::legal_actions)
                .is_some_and(|legal| {
                    legal.lands.contains(&object) || legal.castable.contains(&object)
                });
        if offered {
            Some(Reach::Offered)
        } else if self.reachable.contains(&object) || self.ability_reach.contains(&object) {
            Some(Reach::Taps)
        } else {
            None
        }
    }

    /// A newly opened chooser takes focus from the card that launched it.
    /// Re-sent views leave the player's hover inside that chooser intact.
    fn clear_hover_for_new_browser(&mut self, was_choosing: bool) {
        if !was_choosing && self.browser.for_choice() {
            self.hovered = None;
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// A versioned source chooser has no current-object preview to inherit.
    fn clear_hover_for_source_choice(&mut self, pending: &Pending) {
        if let Pending::ChooseDamageSource { choice, .. } = pending
            && self.interaction.as_ref().and_then(Interaction::decision_id)
                != Some(baylee_client_core::interaction::DecisionId::Source(*choice))
        {
            self.hovered = None;
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// A Raging River label is a question about one attacker, and the
    /// table's way of pointing at a card is the cursor: it goes onto the
    /// attacker as each label is asked, so the card is lifted on the table,
    /// stands in the preview, and the keyboard cursor starts from it. Played
    /// live, the two attackers looked alike and the preview showed whichever
    /// card the pointer had last crossed. Only when the label is new: a view
    /// re-sending the same question leaves a player's own hover alone.
    fn hover_the_river_label(&mut self, before: Option<ObjectId>) {
        let Some(attacker) = self
            .interaction
            .as_ref()
            .filter(|i| i.is_mine())
            .and_then(river_label)
        else {
            return;
        };
        if before != Some(attacker) {
            self.hovered = Some(attacker);
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// Makes `pending` the question, keeping what the last one carries over,
    /// and returns the attacker the last one was a river label for.
    fn install_question(&mut self, pending: Pending, seat: PlayerId) -> Option<ObjectId> {
        let label_before = self.interaction.as_ref().and_then(river_label);
        if let Some(legal) = self
            .interaction
            .as_ref()
            .filter(|i| matches!(i.pending(), Pending::Priority { player, .. } if *player == seat))
            .and_then(Interaction::legal_actions)
        {
            self.priority_legal = Some(legal.clone());
        }
        self.interaction = Some(Interaction::new_keeping(
            pending,
            self.view
                .as_ref()
                .map_or(seat, baylee_client_core::decision::resource_player),
            self.interaction.as_ref(),
        ));
        label_before
    }

    pub(crate) fn receive_choice(&mut self, pending: Pending) {
        self.clear_hover_for_source_choice(&pending);
        self.target_page = 0;
        self.target_filter = None;
        let seat = self.seat().unwrap_or(PlayerId::new(0));
        if !matches!(
            pending,
            Pending::ChooseSubtype { .. } | Pending::ChooseCardName { .. }
        ) {
            self.subtype_filter.clear();
            self.subtype_group = 0;
        }
        let combat_before = self.interaction.as_ref().is_some_and(my_combat_question);
        let label_before = self.install_question(pending, seat);
        self.cap_x();
        self.refresh_owed_plan();
        // My own attackers or blockers being asked for brings the camera home
        // as the question opens: its subject is my board (DESIGN-v7 §2.4).
        // My priority, a target or a yes-no do not — a player visits a seat
        // to target what is on it.
        if baylee_client_core::tableview::comes_home(&baylee_client_core::tableview::HomeEdge {
            me: seat,
            active_before: None,
            active_now: None,
            combat_question_opened: !combat_before
                && self.interaction.as_ref().is_some_and(my_combat_question),
        }) {
            // A layout arrangement keeps my board on the felt whatever the
            // seat of interest is, so my blockers question brings the
            // attacker across instead of the side across the ring: the
            // creatures to block stand on the felt (DESIGN-v8 §1.1).
            let blocking = self
                .interaction
                .as_ref()
                .is_some_and(|i| matches!(i.pending(), Pending::ChooseBlockers { .. }));
            let attacker = self.view.as_ref().map(|v| v.active).filter(|a| *a != seat);
            self.visiting = if self.arrangement.moves_cards() && blocking {
                attacker
            } else {
                None
            };
        }
        // The flank, not the state: `Cues` remembers whether the last
        // question was this seat's, so the acting seat being re-sent its own
        // question — which happens every time anybody at the table says
        // anything — is silent. A game ending is addressed to nobody, so it
        // clears the flag on its way past.
        self.cues
            .note_question(self.interaction.as_ref().is_some_and(Interaction::is_mine));
        if let Some(result) = self.ending() {
            let outcome = baylee_client_core::interaction::outcome(result, seat, self.my_team());
            self.cues.note_ending(outcome);
        }
        // Decided here and not per frame: a panel that re-decided
        // every frame whether to be open could never be closed.
        let was_choosing = self.browser.for_choice();
        if let Some(v) = self.view.as_ref() {
            self.browser.follow(v, self.interaction.as_ref());
        }
        self.clear_hover_for_new_browser(was_choosing);
        self.hover_the_river_label(label_before);
        // A chooser belongs to the choice it was opened under. It
        // would heal itself anyway — the options are rebuilt from the
        // current `LegalActions` — but a menu that outlives its
        // question is a menu a player has to dismiss.
        self.ability_menu = None;
        // The cast chooser goes with it, and for the same reason twice over:
        // it is a chooser, and it is a chooser about a card the game may have
        // just moved.
        self.cast_menu = None;
        // …and so is a half-pressed concession. The game moved on.
        self.concede_armed = false;
        self.leave_armed = false;
        // An armed deed survives this, and that is the point. Only a question
        // that is *not* this seat's priority window takes it — everything
        // armable is a priority-window action.
        if !matches!(
            self.interaction.as_ref().map(Interaction::pending),
            Some(Pending::Priority { player, .. }) if *player == seat
        ) {
            self.armed = None;
        }
        // A chosen way is owed to exactly one question, and this is where it
        // is written off when that question never comes: the seat is holding
        // priority again with nothing armed and no run going, so the spell is
        // on the stack and the engine had only one way to offer. Every step
        // in between fails one of the three — a run's taps come back as
        // priority *with* a run, a `ChooseColor` is not priority at all, and
        // the `ChooseCastMode` this is for is not either.
        if self.mana_run.is_none()
            && self.armed.is_none()
            && matches!(
                self.interaction.as_ref().map(Interaction::pending),
                Some(Pending::Priority { player, .. }) if *player == seat
            )
        {
            self.cast_answer = None;
        }
        rebuild_board(self);
    }

    /// The actions queued for the host but not yet sent.
    ///
    /// Read-only, and there for the tests that ask what a click *did*: the
    /// outbox is the one place a client's decision is visible before the
    /// engine has seen it, and a test that reached past it would be checking
    /// the engine rather than the client.
    #[must_use]
    pub fn outbox(&self) -> &[PlayerAction] {
        &self.outbox
    }

    /// Takes the queued actions, as `flush_outbox` does on a frame.
    ///
    /// The `pub` half of the same seam, for a test that drives more than one
    /// round trip: reading [`Self::outbox`] and never emptying it makes the
    /// second step of a run look like the first one repeated.
    ///
    /// **It is not the whole of what a frame does, and the remainder is not
    /// bookkeeping.** `flush_outbox` drops [`Self::interaction`] after
    /// sending, because the answer is on its way and the next choice replaces
    /// it — so between a send and that choice the seat has no legal actions
    /// at all, and every card in hand is neither `playable` nor offered. A
    /// harness that calls this and stops has closed that window, which is the
    /// one a player is looking at while they wait.
    pub fn take_outbox(&mut self) -> Vec<PlayerAction> {
        std::mem::take(&mut self.outbox)
    }

    /// The local seat, once the static payload has arrived.
    #[must_use]
    pub fn seat(&self) -> Option<PlayerId> {
        self.statics.as_ref().map(|s| s.your_seat)
    }

    /// The follow switch's half of a view arriving (DESIGN-v8 §1.1): a turn
    /// that changed hands shows its active seat, now or once nothing holds
    /// the table still; a seat waiting to be shown is shown when that is so.
    fn follow_the_turn(&mut self, view: &PlayerView) {
        let question_for_me = view.awaiting == Some(view.seat);
        let pointer_on_interest = self.hovered.is_some_and(|id| {
            view.object(id)
                .is_some_and(|o| Some(o.controller) == self.visiting)
        });
        let edge = baylee_client_core::tableview::FollowEdge {
            on: self.follow,
            me: view.seat,
            active_before: self.view.as_ref().map(|v| v.active),
            active_now: Some(view.active),
            question_for_me,
            pointer_on_interest,
        };
        match baylee_client_core::tableview::follow(&edge) {
            // A turn changed hands: the dwell starts again for its seat. A
            // turn the house plays in a blink is never shown; the camera
            // lands where a turn lasts (the owner, 08.10.).
            baylee_client_core::tableview::Follow::Show(seat)
            | baylee_client_core::tableview::Follow::Defer(seat) => {
                self.follow_pending = Some(seat);
                self.follow_since = self.follow_now;
            }
            baylee_client_core::tableview::Follow::Stay => {}
        }
    }

    /// Whether the seat waiting to be shown may be shown now: its turn has
    /// lasted [`FOLLOW_DWELL`], no decision of mine is open, and the pointer
    /// is not reading the board shown.
    ///
    /// A bare priority grant on an empty stack is not a decision: the house
    /// answers inside the engine, so every view of another seat's turn
    /// already awaits this seat, and a switch that waited for a view not
    /// awaiting it never moved (beta.6 QA). A target, a choice, a block or
    /// something on the stack to answer holds the table until it is
    /// answered.
    #[must_use]
    pub fn follow_due(&self) -> Option<PlayerId> {
        let seat = self.follow_pending.filter(|_| self.follow)?;
        if self.follow_now - self.follow_since < FOLLOW_DWELL {
            return None;
        }
        let deciding = self.interaction.as_ref().is_some_and(|i| {
            i.is_mine()
                && !(matches!(i.pending(), Pending::Priority { .. })
                    && self.view.as_ref().is_some_and(|v| v.stack.is_empty()))
        });
        let pointer_on_interest = self.hovered.is_some_and(|id| {
            self.view
                .as_ref()
                .and_then(|v| v.object(id))
                .is_some_and(|o| Some(o.controller) == self.visiting)
        });
        (!deciding && !pointer_on_interest).then_some(seat)
    }

    /// Shows the seat [`Self::follow_due`] names.
    pub fn follow_show(&mut self, seat: PlayerId) {
        self.follow_pending = None;
        if self.visiting != Some(seat) {
            self.visiting = Some(seat);
            self.follow_settling = true;
        }
    }

    /// The table as it stands once whatever is moving has arrived: a tear's
    /// end, else the layout. What the camera frames and the slab is cut to,
    /// so neither breathes with the tear's stages.
    #[must_use]
    pub fn settled_layout(&self) -> Option<&TableLayout> {
        self.tear.as_ref().map(Tear::to).or(self.layout.as_ref())
    }

    /// A seat's slot if its board is drawn: on the felt, or swinging out of
    /// a tear (DESIGN-v8 §0, the owner's tear).
    #[must_use]
    pub fn drawn_slot(&self, player: PlayerId) -> Option<&baylee_client_core::SeatSlot> {
        let layout = self.layout.as_ref()?;
        layout.shown(player).or_else(|| {
            self.tear
                .as_ref()
                .filter(|t| t.draws(player))
                .and_then(|_| layout.slot(player))
        })
    }

    /// The seat the **camera** visits: [`Self::visiting`] under a camera
    /// arrangement, nothing under a layout one, whose seat of interest is
    /// answered by moving cards while the camera stays home (DESIGN-v8 §0).
    #[must_use]
    pub fn camera_visit(&self) -> Option<PlayerId> {
        self.visiting.filter(|_| !self.arrangement.moves_cards())
    }

    /// Which tap the ability sheet is asking the colour of, if it is asking.
    ///
    /// [`Self::ability_tap`] and nothing else, gated on the sheet being open:
    /// the step *into* a tap belongs to the sheet it was taken on, so a value
    /// that outlives its sheet answers nothing rather than opening a bubble
    /// over the next card clicked.
    #[must_use]
    pub fn asking_tap(&self) -> Option<u32> {
        self.ability_menu.and(self.ability_tap)
    }

    /// The local seat's side, if the table has sides at all.
    ///
    /// The roster and not the view: a seat's team is in `GameStatic` and no
    /// `PlayerView` carries it. It is the one thing about the table that
    /// reading a `Victor::Team` needs — see
    /// [`baylee_client_core::interaction::outcome`] — and it is asked twice,
    /// by the end screen and by the sound the end of a game makes, which is
    /// why it is written once here.
    #[must_use]
    pub fn my_team(&self) -> Option<u8> {
        let statics = self.statics.as_ref()?;
        statics
            .seats
            .iter()
            .find(|s| s.player == statics.your_seat)
            .and_then(|s| s.team)
    }

    /// Whether this seat is being asked something right now.
    #[must_use]
    pub fn is_my_turn_to_act(&self) -> bool {
        self.interaction.as_ref().is_some_and(Interaction::is_mine)
    }

    /// Whether this player's own seat has been eliminated while the game
    /// goes on for the remaining seats (a multi-player table).
    #[must_use]
    pub fn my_seat_eliminated(&self) -> bool {
        !self.watching
            && self.replay.is_none()
            && self
                .view
                .as_ref()
                .and_then(|v| v.seat(v.seat))
                .is_some_and(baylee_view::SeatView::has_lost)
    }

    /// Whether this seat's own standing order is currently withholding its
    /// priority.
    ///
    /// Read off the view rather than remembered here on purpose: the engine
    /// drops a hold the moment its condition is met, and a client keeping its
    /// own copy would light an indicator for a hold that expired two
    /// resolutions ago.
    #[must_use]
    pub fn priority_held(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.priority_held)
    }

    /// What the two hold keys send, given which of them was pressed.
    ///
    /// One door for both keys and for the prompt bar's button, because the
    /// toggle rule is the part worth having in one place: a hold that is
    /// already running is **cancelled** by either key rather than replaced.
    /// A player who has stopped being asked and cannot remember which key did
    /// it should not have to guess to get the game back.
    ///
    /// `UntilStackEmpty` carries the stack depth this seat can see, and a
    /// stale view is safe by construction: if something was added since, the
    /// engine reads a depth above the one sent and cancels the hold on the
    /// spot — which is exactly right, because somebody just responded to what
    /// was being let through.
    ///
    /// `None` before the first view arrives: there is no game to hold yet.
    #[must_use]
    pub fn hold_action(&self, until_turn_ends: bool) -> Option<PlayerAction> {
        let view = self.view.as_ref()?;
        let hold = if view.priority_held {
            baylee_engine::choice::PriorityHold::Always
        } else if until_turn_ends {
            baylee_engine::choice::PriorityHold::UntilEndOfTurn { turn: view.turn }
        } else if let Some(object) = self
            .stack_selected
            .filter(|id| view.stack.iter().any(|obj| obj.id == *id))
        {
            baylee_engine::choice::PriorityHold::UntilTopOfStack { object }
        } else {
            baylee_engine::choice::PriorityHold::UntilStackEmpty {
                depth: u16::try_from(view.stack.len()).unwrap_or(u16::MAX),
            }
        };
        Some(PlayerAction::SetPriorityHold(hold))
    }

    /// Whether "let the stack resolve" is a thing this seat could ask for.
    ///
    /// Two facts, and each of them changes what the *same* press does rather
    /// than merely greying it out. On an empty stack [`hold_action(false)`]
    /// sends `UntilStackEmpty { depth: 0 }`, a hold that is over before it
    /// begins — a button promising to do nothing. And while a hold is already
    /// running it sends `Always`, which **cancels** that hold: the button
    /// under the F6 cap would do the opposite of what its label says. §4.4 of
    /// the ledge design draws that second state as its own sentence, which is
    /// why this is a predicate and not a disabled control.
    ///
    /// Read by both the drawing and the press, so a stack that emptied between
    /// the two cannot be held against.
    ///
    /// [`hold_action(false)`]: Duel::hold_action
    #[must_use]
    pub fn can_hold_for_stack(&self) -> bool {
        self.view
            .as_ref()
            .is_some_and(|v| !v.stack.is_empty() && !v.priority_held)
    }

    /// Let the whole current stack resolve, then pass until the next turn.
    ///
    /// This is deliberately not [`Duel::hold_action`]: a stack entry selected
    /// as a manual response point must not shorten "skip turn" into "stop at
    /// that entry". A later stack still cancels the engine hold, and a real
    /// decision still stops the client autopilot.
    pub fn skip_turn(&mut self) {
        let Some(view) = self.view.as_ref() else {
            return;
        };
        let turn = view.turn;
        let depth = u16::try_from(view.stack.len()).unwrap_or(u16::MAX);
        let hold = !view.stack.is_empty() && !view.priority_held;
        if hold {
            self.submit(PlayerAction::SetPriorityHold(
                baylee_engine::choice::PriorityHold::UntilStackEmpty { depth },
            ));
        }
        self.autopilot = Some(AutoPilot::ToNextTurn { from_turn: turn });
    }

    /// Whether the engine would take a draw offer right now.
    ///
    /// `Engine::offer_draw` refuses anything but the offerer's own priority,
    /// because the offer suspends a decision that has to be handed back
    /// untouched if anyone refuses (CR 104.4i). The button was drawn live
    /// whatever the game was doing, so the usual answer to pressing it was an
    /// `IllegalAction` in the prompt bar.
    ///
    /// Priority is the whole condition. The engine's other refusal — nobody
    /// left to offer to — cannot happen while this seat holds priority, since
    /// a game with one player in it has already ended.
    #[must_use]
    pub fn can_offer_draw(&self) -> bool {
        self.interaction.as_ref().is_some_and(|i| {
            i.is_mine() && matches!(i.pending(), baylee_engine::choice::Pending::Priority { .. })
        })
    }
}

impl Duel {
    /// Whether the hand's drawer stands open: the player's choice, or a
    /// question answered from the hand (`client-core::handdrawer`). Off a
    /// phone the drawer is not drawn, and this is read only there.
    #[must_use]
    pub fn hand_drawer_open(&self) -> bool {
        self.hand_drawer.open(
            self.view.as_ref(),
            self.interaction.as_ref().map(Interaction::pending),
        )
    }

    /// The snapshot the question in hand was asked at: what a fold of its
    /// sheet is kept against, so the next question opens unfolded.
    #[must_use]
    pub fn decision_seq(&self) -> Option<u64> {
        self.view.as_ref().map(|v| v.seq)
    }

    /// Folds the question's sheet to its pill, or opens it again. It answers
    /// nothing: the question stands, folded or not.
    pub fn fold_decision(&mut self) {
        let seq = self.decision_seq();
        self.decision_fold.toggle(seq);
    }

    /// The tab tapped, or `I`: open becomes shut and shut open.
    pub fn toggle_hand_drawer(&mut self) {
        let view = self.view.as_ref();
        let pending = self.interaction.as_ref().map(Interaction::pending);
        self.hand_drawer.toggle(view, pending);
    }

    /// How many cards in my hand are castable now (the tab says so: a
    /// priority with something castable does not open the drawer by
    /// itself).
    #[must_use]
    pub fn castable_in_hand(&self) -> usize {
        let (Some(view), Some(legal)) = (
            self.view.as_ref(),
            self.interaction
                .as_ref()
                .and_then(Interaction::legal_actions),
        ) else {
            return 0;
        };
        view.hand
            .iter()
            .filter(|card| legal.castable.contains(&card.id) || legal.lands.contains(&card.id))
            .count()
    }

    /// Whether a spell in my hand can be cast now (a land drop cannot be
    /// cancelled: the table tour's T8 asks for a cast).
    #[must_use]
    pub fn a_spell_is_castable(&self) -> bool {
        let (Some(view), Some(legal)) = (
            self.view.as_ref(),
            self.interaction
                .as_ref()
                .and_then(Interaction::legal_actions),
        ) else {
            return false;
        };
        view.hand
            .iter()
            .any(|card| legal.castable.contains(&card.id))
    }
}
