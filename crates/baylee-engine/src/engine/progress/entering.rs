//! Objects entering: the arrival scan, the questions an entry asks, copies on entering, starting loyalty.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Applies as-it-enters-the-battlefield modifiers to permanents that
    /// entered since the last scan (CR 614.1c/d; taplands, shocklands).
    ///
    /// Permanents that enter together are one event, and each one's own
    /// replacements modify how it enters (CR 614.12). So every arrival's
    /// replacements that need nobody's answer are applied first: entering
    /// tapped, counters, starting loyalty, a Saga's lore counter, daybound's
    /// back face, a Room's door. The ones that ask are then asked one at a
    /// time, from [`Engine::entry_questions`], by seat in APNAP order and in
    /// the order the permanents entered (CR 101.4). A question used to return
    /// from the middle of the scan with the cursor already past every
    /// arrival, so every co-arrival after the one that asked got none of its
    /// replacements: a Urza's Saga fetched beside Steam Vents had no lore
    /// counter, and a planeswalker entering behind a shockland had no
    /// loyalty and died.
    ///
    /// Returns whether a modifier wrote to the board, which the caller reads
    /// twice over: the legal lists are recomputed from it, and the machine
    /// goes round one more pass. The second is the load-bearing one — this
    /// runs *after* the projection two steps up, so a counter placed here is
    /// invisible to the state-based actions two steps down until they have
    /// been refreshed once more.
    pub(crate) fn apply_enter_modifiers(&mut self) -> bool {
        let mut changed = false;
        // A question an earlier scan owes comes first. Nothing enters while
        // one is out, so the queue holds the rest of that batch's questions,
        // and the arrivals below wait for it to empty.
        if self.ask_owed_entry_question(&mut changed) {
            return true;
        }
        changed |= self.scan_arrivals();
        if self.ask_owed_entry_question(&mut changed) {
            return true;
        }
        changed
    }

    /// The half of [`Self::apply_enter_modifiers`] that asks nobody: every
    /// arrival since the last scan gets the replacements that need no answer,
    /// and the ones that ask are queued on [`Engine::entry_questions`],
    /// APNAP, for the caller to ask. Returns whether it wrote to the board.
    ///
    /// [`Engine::new`] runs it over the starting battlefield, where nobody
    /// may be asked anything yet: every seat is deciding its mulligan.
    #[allow(clippy::too_many_lines)] // the entry-modifier table is naturally flat
    pub(crate) fn scan_arrivals(&mut self) -> bool {
        use baylee_cards_dsl::EnterModifier;
        let mut changed = false;
        // `from` travels with the arrival because one modifier reads it:
        // the X a `{X}{X}` body enters with belongs to the spell that
        // became this permanent (CR 107.3m), so it exists on the way in
        // off the stack and nowhere else.
        let events: Vec<(ObjectId, PlayerId, Zone)> = self
            .state
            .journal
            .entries()
            .get(self.entry_scan_seq as usize..)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| match &e.event {
                GameEvent::ZoneChanged {
                    object,
                    from,
                    to: Zone::Battlefield,
                    ..
                } => Some((*object, *from)),
                _ => None,
            })
            .filter_map(|(id, from)| self.state.object(id).map(|o| (id, o.controller, from)))
            .collect();
        self.entry_scan_seq = self.state.journal.last_seq();
        let mut asks: Vec<(ObjectId, PlayerId, EntryAsk)> = Vec::new();
        for (id, controller, from_zone) in events {
            // CR 107.3m in one place, before anything reads it. The rule
            // gives an entering permanent's own abilities the X announced
            // for *the spell that became it*, and gives the permanent
            // itself an X of 0 — so the field means "the spell's number, or
            // nothing", and the one moment that is decidable is here, where
            // the arrival still knows which zone it came from. A Walking
            // Ballista that died at X = 1 and was reanimated, or one blinked
            // back from exile, was announced by nobody and comes down as the
            // 0/0 it prints (CR 107.3g: a card anywhere but the stack has an
            // X of 0); `move_object` deliberately carries the spell-shaped
            // fields through a zone change, so without this the old number
            // would still be sitting there.
            //
            // Normalised rather than guarded at each reader, because there
            // are now two of them and they are not alike: `WithCounters`
            // below is a replacement effect and could ask `from_zone`
            // itself, while an enters-the-battlefield *triggered* ability
            // is put on the stack by `collect_triggers` — a later step of
            // this same pass, with no arrival in its hands to ask.
            if from_zone != Zone::Stack
                && let Some(obj) = self.state.object_mut(id)
                && obj.x_value != 0
            {
                obj.x_value = 0;
            }
            // Daybound's first static ability (CR 702.145b): if it is
            // night, a permanent represented by a double-faced card
            // *enters* transformed. It is done here rather than left to
            // the fixpoint's later step because "enters transformed" means
            // the back face is what entered — the front face's own
            // enter-the-battlefield triggers were never on the board, and
            // `collect_triggers` runs after this in the same pass.
            //
            // It enters with its back face up (CR 712.14a); nothing turns
            // over, so it is `turn_over` and not `transform`: no
            // `Transformed` entry for a "transforms into" trigger or the
            // log to read, and no new timestamp over the one it entered
            // with (CR 613.7d). `transform` gave it both.
            if self.state.day_night == Some(DayNight::Night)
                && self
                    .state
                    .object(id)
                    .is_some_and(|o| o.face_index == 0 && o.zone == Zone::Battlefield)
                && let Some(def) = self
                    .state
                    .object(id)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                && def.faces.len() >= 2
                && def
                    .keywords_for_face(0)
                    .contains(baylee_cards_dsl::KeywordSet::DAYBOUND)
            {
                self.state.turn_over(id, def, 1);
                changed = true;
            }
            // A Room is given the unlocked designation of the half it was
            // cast as as it enters, and neither when it was not cast
            // (CR 709.5d). Here, like "enters transformed", so what the
            // trigger scan later in this pass finds is the half that entered.
            // The unlock is journalled because CR 709.5h triggers on the
            // designation however it was given.
            if let Some((def, half)) = self
                .state
                .object(id)
                .filter(|o| o.zone == Zone::Battlefield && !o.doors.is_room())
                .and_then(|o| Some((self.lookup.card(o.card?.index)?, o.face_index)))
                .filter(|(def, _)| def.has_shared_type_line())
            {
                let cast = from_zone == Zone::Stack && half < 2;
                self.state
                    .set_doors(id, def, if cast { 1 << half } else { 0 });
                if cast {
                    self.state
                        .journal
                        .record(GameEvent::DoorUnlocked { object: id, half });
                }
                changed = true;
            }
            // Echo (CR 702.30): register the pay-or-sacrifice choice at
            // the controller's next upkeep.
            if let Some(cost) = self.state.object(id).and_then(|o| {
                o.abilities(&self.lookup).iter().find_map(|a| match a {
                    baylee_cards_dsl::AbilityDef::Echo { cost } => Some(*cost),
                    _ => None,
                })
            }) {
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::PayCostOrSacrifice {
                        cost,
                        card: id,
                        version: self.state.object(id).map_or(0, |o| o.version),
                    },
                });
            }
            // A Saga takes a lore counter as it enters (CR 714.3a), and
            // "As [this permanent] enters …" is named as a replacement
            // effect in CR 614.1c — so the counter goes through the door
            // that applies the multiplying replacements, exactly as a
            // planeswalker's starting loyalty does forty lines below. CR
            // 614.16 says nothing about Sagas; what it gives is the shape,
            // and this engine had already read it that way once.
            //
            // Which chapters that triggers is then a window and not a
            // number — see [`Self::queue_saga_chapters`]. Under a Doubling
            // Season two counters land at once and chapters I *and* II are
            // owed; this site matched `chapter: 1` and would have run the
            // first one only.
            if self.is_saga(id)
                && let Some(old) = self
                    .state
                    .object(id)
                    .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::Lore))
            {
                let ts = self.state.next_timestamp();
                crate::replacement::put_counters(
                    &mut self.state,
                    id,
                    baylee_cards_dsl::CounterKind::Lore,
                    1,
                );
                let new = self
                    .state
                    .object(id)
                    .map_or(old, |o| o.counters.get(baylee_cards_dsl::CounterKind::Lore));
                if let Some(obj) = self.state.object_mut(id) {
                    obj.timestamp = ts;
                }
                if self.queue_saga_chapters(id, controller, old, new, ts) {
                    changed = true;
                }
            }
            // Planeswalkers enter with their printed loyalty counters
            // (CR 306.5b).
            if self.put_starting_loyalty(id) {
                changed = true;
            }
            // Clone-on-enter, for every door that is not a permanent spell.
            //
            // A spell asks in `finalize_spell`, before the move, which is
            // what CR 614.12a requires — so by the time that arrival reaches
            // this scan the question has been answered and asking again would
            // ask it twice. `from_zone` is what separates the two and is
            // already in hand: a permanent spell resolves off the **stack**,
            // a token is created `from: OutsideGame`, and everything else
            // comes out of a graveyard, a library, exile or a hand.
            //
            // Asked after the scan like every other question, and it still
            // stands in for this permanent's own modifiers, as it always has.
            if from_zone != Zone::Stack
                && self
                    .copy_on_enter_question(id)
                    .is_some_and(|(options, _)| options.iter().any(|&o| o != id))
            {
                asks.push((id, controller, EntryAsk::Copy));
                continue;
            }
            let Some((card, face_index)) = self
                .state
                .object(id)
                .and_then(|o| Some((o.card?, o.face_index)))
            else {
                continue;
            };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            // The permanent's *own* face, not the front one. A modal
            // double-faced card enters as its back face (Glasspool Shore is a
            // land that enters tapped), and reading `faces[0]` there asked the
            // creature half whether the land comes in tapped.
            let Some(face) = def.faces.get(face_index as usize) else {
                continue;
            };
            // A modifier that *asks* is applied last, whatever order the
            // card prints it in: it is queued, and asked once every
            // arrival's other modifiers are on the board. Uncharted Haven is
            // `ChooseColor` then `Tapped` and once entered untapped, when a
            // question returned from the middle of this loop; the same hole
            // had been under `ChooseSubtype` and `TappedOrPayLife` since
            // they were written.
            //
            // The *first* question wins, and a card with two would lose the
            // second. No printed card asks twice as it enters, and the day
            // one does this is a second pass rather than a second field.
            let mut asked: Option<&EnterModifier> = None;
            for modifier in face.enter_modifiers {
                match modifier {
                    EnterModifier::Tapped => {
                        self.state.set_tapped(id, true);
                        changed = true;
                    }
                    EnterModifier::TappedUnless(filter) => {
                        if !self.controls_at_least(filter, controller, id, 1) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    EnterModifier::TappedUnlessCount { filter, at_least } => {
                        if !self.controls_at_least(filter, controller, id, usize::from(*at_least)) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // The same count, the other way round: a fast land comes
                    // down tapped once the board is *past* its bound, which is
                    // the sentence a slow land's arm would answer backwards.
                    EnterModifier::TappedUnlessAtMost { filter, at_most } => {
                        if self.controls_count(filter, controller, id) > usize::from(*at_most) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // Both of these count *players*, and neither re-derives
                    // which ones count. `eval::players` is the one place that
                    // knows a teammate is not an opponent and that a player
                    // who has lost is out of the game — a hand-rolled
                    // `self.state.players.len() - 1` here would have been
                    // right in a duel and wrong at every table these lands
                    // are actually printed for.
                    EnterModifier::TappedUnlessOpponents { at_least } => {
                        let opponents = eval::players(PlayerRel::Opponent, &self.state, controller)
                            .map_or(0, |seats| seats.len());
                        if opponents < usize::from(*at_least) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    EnterModifier::TappedUnlessSomeoneAtOrBelow { life } => {
                        // "A player", so the controller is on the list too.
                        let low = eval::players(PlayerRel::EachPlayer, &self.state, controller)
                            .unwrap_or_default()
                            .into_iter()
                            .any(|seat| {
                                self.state
                                    .players
                                    .get(seat.get() as usize)
                                    .is_some_and(|p| p.life <= *life)
                            });
                        if !low {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // CR 614.1c: "this enters with N counters on it" is a
                    // replacement effect, so it goes through the door that
                    // lets a counter doubler have its say — a Vivid land
                    // under a Doubling Season brings four charge counters,
                    // not two (CR 614.16).
                    EnterModifier::WithCounters { kind, amount } => {
                        // CR 107.3m: a replacement effect on a permanent that
                        // refers to X uses the X chosen for the spell that
                        // became that object as it resolved. `x_value` is
                        // that announced number and asks nothing further
                        // here, because the top of this loop has already put
                        // it back to 0 for an arrival that came from
                        // anywhere but the stack.
                        let x = Some(self.state.object(id).map_or(0, |o| o.x_value));
                        let n = crate::eval::amount(amount, &self.state, controller, id, x);
                        if n > 0 {
                            let n = u16::try_from(n).unwrap_or(u16::MAX);
                            crate::replacement::put_counters(&mut self.state, id, *kind, n);
                            changed = true;
                        }
                    }
                    EnterModifier::LoseLifeEqualToLife => {
                        let life = self
                            .state
                            .players
                            .get(controller.get() as usize)
                            .map_or(0, |p| p.life);
                        if life > 0 {
                            self.state
                                .change_life(controller, -life, crate::event::Cause::Effect);
                            changed = true;
                        }
                    }
                    EnterModifier::Prepared => {
                        if let Some(obj) = self.state.object_mut(id)
                            && !obj.riders.contains(&crate::object::Rider::Prepared)
                        {
                            obj.riders.push(crate::object::Rider::Prepared);
                        }
                    }
                    EnterModifier::ChooseSubtype
                    | EnterModifier::ChooseBasicLandType
                    | EnterModifier::ChooseCardName
                    | EnterModifier::ChooseColor
                    | EnterModifier::ChooseOpponent
                    | EnterModifier::ChooseColorExcept(_)
                    | EnterModifier::TappedOrPayLife(_)
                    | EnterModifier::TappedUnlessReveal(_) => {
                        asked.get_or_insert(modifier);
                    }
                }
            }
            if let Some(modifier) = asked {
                asks.push((id, controller, EntryAsk::Modifier(modifier)));
            }
        }
        // CR 101.4: players choosing at the same time choose in APNAP
        // order. One player's questions keep the order the permanents
        // entered in (the sort is stable); CR 101.4c would let that player
        // choose the order, which is not offered.
        let active = self.state.turn.active.get();
        let seats = self.state.players.len() as u8;
        asks.sort_by_key(|(_, controller, _)| (controller.get() + seats - active) % seats);
        self.entry_questions.extend(asks);
        changed
    }

    /// Asks the next as-it-enters question [`Engine::entry_questions`]
    /// holds, and says whether one is out. One whose permanent has left the
    /// battlefield in the meantime is dropped. One that has no question to
    /// ask any more is settled without one: a shockland its controller can
    /// no longer pay for, or a reveal with nothing to reveal, enters tapped,
    /// and `changed` says the board was written.
    fn ask_owed_entry_question(&mut self, changed: &mut bool) -> bool {
        while let Some((id, controller, ask)) = self.entry_questions.pop_front() {
            if self
                .state
                .object(id)
                .is_none_or(|o| o.zone != Zone::Battlefield)
            {
                continue;
            }
            let asked = match ask {
                EntryAsk::Copy => self.check_copy_on_enter(id),
                EntryAsk::Modifier(modifier) => {
                    self.ask_entry_modifier(id, controller, modifier, changed)
                }
            };
            if asked {
                return true;
            }
        }
        false
    }

    /// The entry choice uses opponent relationships, never targeting restrictions.
    fn ask_enter_opponent(&mut self, id: ObjectId, controller: PlayerId) -> bool {
        let options =
            eval::players(PlayerRel::Opponent, &self.state, controller).unwrap_or_default();
        if options.is_empty() {
            return false;
        }
        self.pending_plan = Some(PlanKind::ChooseOpponent { object: id });
        self.pending = Pending::ChoosePlayer {
            player: controller,
            options,
        };
        self.awaiting_answer = true;
        true
    }

    /// Asks one permanent's as-it-enters question, or settles it without
    /// one when there is nothing to choose; says whether a question is out.
    fn ask_entry_modifier(
        &mut self,
        id: ObjectId,
        controller: PlayerId,
        modifier: &'static baylee_cards_dsl::EnterModifier,
        changed: &mut bool,
    ) -> bool {
        use baylee_cards_dsl::EnterModifier;
        match modifier {
            EnterModifier::ChooseOpponent => self.ask_enter_opponent(id, controller),
            EnterModifier::ChooseSubtype | EnterModifier::ChooseBasicLandType => {
                use baylee_core::generated::subtypes::land::{
                    FOREST, ISLAND, MOUNTAIN, PLAINS, SWAMP,
                };
                // A basic land type is one of the five (CR 205.3i), in the
                // order the rule names them, and nothing else.
                let options = if matches!(modifier, EnterModifier::ChooseBasicLandType) {
                    vec![PLAINS, ISLAND, SWAMP, MOUNTAIN, FOREST]
                } else {
                    (0..=349).map(baylee_core::ids::SubtypeId::new).collect()
                };
                self.pending_plan = Some(PlanKind::ChooseSubtype { object: id });
                self.pending = Pending::ChooseSubtype {
                    player: controller,
                    options,
                };
                self.awaiting_answer = true;
                true
            }
            EnterModifier::ChooseCardName => {
                self.pending_plan = Some(PlanKind::ChooseCardName { object: id });
                self.pending = Pending::ChooseCardName { player: controller };
                self.awaiting_answer = true;
                true
            }
            m @ (EnterModifier::ChooseColor | EnterModifier::ChooseColorExcept(_)) => {
                // Colorless is not a colour (CR 105.1), so it is not
                // among the options even though `ManaColor` carries it:
                // "choose a color" is one of the five.
                let except = match m {
                    EnterModifier::ChooseColorExcept(c) => Some(*c),
                    _ => None,
                };
                let options: Vec<_> = baylee_cards_dsl::ALL_MANA_COLORS
                    .iter()
                    .copied()
                    .filter(|c| Some(*c) != except)
                    .collect();
                self.pending_plan = Some(PlanKind::ChooseColor { object: id });
                self.pending = Pending::ChooseColor {
                    player: controller,
                    options,
                };
                self.awaiting_answer = true;
                true
            }
            EnterModifier::TappedOrPayLife(amount) => {
                let amount = *amount;
                // Asked when its turn comes, so a second shockland in the
                // same batch is asked against the life the first one left
                // (CR 614.12b).
                if self.state.can_pay_life(controller, i32::from(amount)) {
                    let source = self
                        .state
                        .object(id)
                        .and_then(|o| o.card)
                        .map(|c| AbilityRef::new(c.index, AbilityRef::ENTERS));
                    self.pending_plan = Some(PlanKind::EntryTap { object: id, amount });
                    self.pending = Pending::YesNo {
                        player: controller,
                        prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                        source,
                    };
                    self.awaiting_answer = true;
                    return true;
                }
                // Unpayable → tapped without a choice: nothing was asked.
                self.state.set_tapped(id, true);
                *changed = true;
                false
            }
            EnterModifier::TappedUnlessReveal(filter) => {
                // The one clause in this family read against a hidden
                // zone. Every `TappedUnless…` sibling asks
                // `controls_at_least`, which walks the battlefield; a
                // Faerie held in hand is on nobody's battlefield and the
                // question would always answer no.
                let filter: &Filter = filter;
                let options: Vec<ObjectId> = self
                    .state
                    .zones
                    .list(ZoneLocation::Hand(controller))
                    .iter()
                    .filter(|card| {
                        self.state
                            .object(**card)
                            .is_some_and(|o| eval::matches(filter, &self.state, o, controller, id))
                    })
                    .copied()
                    .collect();
                if options.is_empty() {
                    // Nothing to show → tapped, and no question asked.
                    // The same branch an unpayable `TappedOrPayLife`
                    // takes, and for the same reason: a prompt whose
                    // only legal answer is "no" is not a choice, and
                    // offering it would hand the opponent the
                    // information that the hand is empty of Faeries.
                    self.state.set_tapped(id, true);
                    *changed = true;
                    return false;
                }
                self.pending_plan = Some(PlanKind::EntryReveal { object: id });
                self.pending = Pending::ChooseCards {
                    player: controller,
                    options,
                    // Naming nothing is how the offer is declined —
                    // the card prints "you may", and a `min` of one
                    // would make the reveal compulsory.
                    min: 0,
                    max: 1,
                    prompt: ChoicePrompt::RevealOrEnterTapped,
                    total: None,
                };
                self.awaiting_answer = true;
                true
            }
            _ => unreachable!("only the asking modifiers are queued"),
        }
    }

    /// Whether `controller` controls `at_least` permanents matching `filter`,
    /// not counting `entering` itself.
    ///
    /// Both enters-tapped-unless clauses ask this one question, so they share
    /// the answer: a checkland is the count with `at_least: 1`, and two
    /// conditions written twice would drift the first time one of them
    /// learned something.
    ///
    /// The entering permanent is left out because its own clause is a
    /// replacement applied on the way in, and ten cards in the pool can see
    /// the difference. A slow land prints "two or more **other** lands" and
    /// `landgen` reads that as `And(&[ControlledByYou, LAND])` — which a slow
    /// land matches — so this line is where the word "other" is spoken for
    /// Deserted Beach and its nine siblings. Count the entering land and
    /// every one of them comes in untapped a land early.
    ///
    /// It used to say the opposite: that nothing in the pool could see it,
    /// every enters-tapped-unless clause naming something the land is not,
    /// with Mystic Sanctuary as the one cycle that could and saying "other"
    /// itself. Mystic Sanctuary is a generated stub carrying no
    /// enter-modifier at all, so it was standing in for the ten cards that
    /// really do this — while
    /// `card_tests::a_slow_land_counts_the_other_lands_and_never_itself` had
    /// been playing one all along and asserting the opposite.
    ///
    /// A card *could* say it for itself: `Filter::Another` is read by `eval`
    /// as `obj.id != this`, and the `this` passed below is the entering
    /// permanent. None of the ten uses it. That is a division of labour
    /// rather than an oversight, and
    /// `card_tests::the_only_lands_that_would_count_themselves_are_the_slow_ones`
    /// is the fence around it — the list is asked of `eval::matches`, so a
    /// new cycle on either side of the split is a build failure.
    fn controls_at_least(
        &self,
        filter: &baylee_cards_dsl::Filter,
        controller: PlayerId,
        entering: ObjectId,
        at_least: usize,
    ) -> bool {
        self.controls_count(filter, controller, entering) >= at_least
    }

    /// The count both bounds read, and the one place the entering permanent
    /// is skipped.
    ///
    /// Two sentences ask about the same number from opposite ends — a slow
    /// land wants at least two other lands and a fast land wants at most two
    /// — so the comparison belongs to the modifier and the counting does not.
    /// Written twice, the word "other" would have had two places to go
    /// missing.
    fn controls_count(
        &self,
        filter: &baylee_cards_dsl::Filter,
        controller: PlayerId,
        entering: ObjectId,
    ) -> usize {
        // A phased-out Swamp does not exist for a checkland (CR 702.26b),
        // and a phased-out land does not count against a fastland (#209).
        self.state
            .battlefield_seen()
            .filter(|other| {
                *other != entering
                    && self.state.object(*other).is_some_and(|o| {
                        eval::matches(filter, &self.state, o, controller, entering)
                    })
            })
            .count()
    }

    /// Checks a newly entered permanent for a clone-on-enter clause and
    /// presents the copy choice when valid targets exist. Returns `true`
    /// when a pending choice was produced.
    /// The clone choice's two halves that do not depend on when it is asked:
    /// which ability is on the object, and what the battlefield offers it.
    ///
    /// Split out because the question is asked from **two** places now and
    /// the difference between them is only the timing — a shared body is what
    /// stops the two from drifting into offering different lists.
    fn copy_on_enter_question(&self, id: ObjectId) -> Option<(Vec<ObjectId>, PlayerId)> {
        let (spec, _) = self.copy_on_enter(id)?;
        let controller = self.state.object(id)?.controller;
        Some((
            eval::target_options(&spec, &self.state, controller, id),
            controller,
        ))
    }

    /// The clone ability on `id`: what it may copy, and what the copy
    /// changes. One reader for the offer above and for the agent's
    /// explanation of it (`decision_context`), so the two cannot name
    /// different abilities.
    pub(in crate::engine) fn copy_on_enter(
        &self,
        id: ObjectId,
    ) -> Option<(TargetSpec, &'static [baylee_cards_dsl::CopyMod])> {
        self.state
            .object(id)?
            .abilities(&self.lookup)
            .iter()
            .find_map(|a| match a {
                AbilityDef::CopyOnEnter { target, mods }
                | AbilityDef::CopyOnEnterUntilEot { target, mods } => Some((*target, *mods)),
                _ => None,
            })
    }

    /// Publishes the clone choice and says whether it was published.
    ///
    /// `before_entry` is the rule, not a convenience. CR 614.12a: *"If a
    /// replacement effect that modifies how a permanent enters the
    /// battlefield requires a choice, that choice is made before the
    /// permanent enters the battlefield."* The answer carries it back so the
    /// handler knows whether it still owes the move.
    fn offer_copy_on_enter(
        &mut self,
        id: ObjectId,
        options: Vec<ObjectId>,
        controller: PlayerId,
        before_entry: bool,
    ) -> bool {
        if options.is_empty() {
            return false; // optional: simply doesn't copy
        }
        self.pending_plan = Some(PlanKind::CopyOnEnter {
            object: id,
            before_entry,
        });
        self.pending = Pending::ChooseTargets {
            player: controller,
            options,
            player_options: Vec::new(),
            min: 0,
            max: 1,
            reason: TargetPrompt::Targets,
        };
        self.awaiting_answer = true;
        true
    }

    /// The clone choice for a permanent **spell**, asked while it is still on
    /// the stack.
    ///
    /// This is CR 614.12a done rather than worked around. The permanent is
    /// not on the battlefield when the list is built, so it cannot be in it —
    /// which is also Glasspool Mimic's own ruling ("You may choose only a
    /// creature that's already on the battlefield") falling out of the timing
    /// instead of being spelled out by name. The `options.retain` that used
    /// to carry that ruling is gone from this path, and the test that
    /// replaced it asks where the Mimic *is* while the question stands, not
    /// whether it is in the list: a list it cannot be in is the stronger
    /// claim.
    ///
    /// The other half of the ruling — a creature entering at the same time is
    /// not a legal choice either — needs nothing here for the same reason it
    /// needed nothing before: permanents arrive one at a time.
    pub(crate) fn ask_copy_before_entry(&mut self, spell: ObjectId) -> bool {
        let Some((options, controller)) = self.copy_on_enter_question(spell) else {
            return false;
        };
        self.offer_copy_on_enter(spell, options, controller, true)
    }

    /// The same choice for a permanent that arrived by some **other** door.
    ///
    /// Reanimation, a search that puts a card onto the battlefield, a token
    /// copy of a clone — none of those go through `finalize_spell`, and this
    /// scan is the only place that sees them. They are still asked one step
    /// after the arrival, so the by-name exclusion below is still doing the
    /// work the timing does on the spell path, and it is still wrong in the
    /// same way the rules say it is: measured against the pool, every card
    /// carrying `CopyOnEnter` is a permanent spell, so the residue is a card
    /// put onto the battlefield by *another* card's effect.
    ///
    /// The Mimic is the one that reaches it: its errata'd filter is "a
    /// creature you control", and the permanent asking the question is one.
    /// Answering with itself made it a copy of a 0/0 Shapeshifter Rogue,
    /// which is the printed face and dies to the state-based action that
    /// follows. Cursed Mirror cannot: it asks as an artifact and its filter
    /// wants a creature.
    pub(crate) fn check_copy_on_enter(&mut self, id: ObjectId) -> bool {
        let Some((mut options, controller)) = self.copy_on_enter_question(id) else {
            return false;
        };
        options.retain(|&o| o != id);
        self.offer_copy_on_enter(id, options, controller, false)
    }

    /// CR 306.5b: a planeswalker enters with as many loyalty counters as its
    /// printed loyalty. Returns whether it put any.
    ///
    /// "Printed" is read off the entering permanent's copiable values and not
    /// off its card. CR 614.12 decides which replacement effects apply to a
    /// permanent entering from the permanent as it would exist on the
    /// battlefield, counting replacement effects that already modified how it
    /// enters, and a copy is one (CR 614.1c "enters as"). Loyalty is a
    /// copiable value (CR 707.2), counters are not. So a Spark Double that
    /// enters as a copy of Karn, the Great Creator is a planeswalker with
    /// Karn's printed 5, whatever Karn has now, and takes 5 here beside the
    /// one its own text adds. Read off the card, it was a 0/0 creature with no
    /// loyalty and entered with that one alone.
    ///
    /// Starting loyalty is counters put on the permanent as it enters, so the
    /// counter-placement replacements apply (CR 614.16): Doubling Season
    /// doubles it.
    pub(crate) fn put_starting_loyalty(&mut self, id: ObjectId) -> bool {
        let Some(loyalty) = crate::layers::copiable_values(&self.state, id).and_then(|values| {
            values
                .types
                .contains(baylee_core::types::TypeSet::PLANESWALKER)
                .then_some(values.loyalty)
                .flatten()
        }) else {
            return false;
        };
        crate::replacement::put_counters(
            &mut self.state,
            id,
            baylee_cards_dsl::CounterKind::Loyalty,
            loyalty,
        );
        true
    }

    /// Apply an entry copy through the same snapshot transaction as a
    /// resolving copy ability (CR 707.2, 707.9).
    pub(crate) fn apply_copy_choice(&mut self, id: ObjectId, target: ObjectId) {
        #[cfg(test)]
        crate::ability_log::copied_on_entry(&self.state, &self.lookup, id);
        let Some(own) = self.state.printed_ability_list(id) else {
            return;
        };
        let Some((index, mods, until_eot)) =
            own.abilities
                .iter()
                .enumerate()
                .find_map(|(index, ability)| match ability {
                    AbilityDef::CopyOnEnter { mods, .. } => Some((index, *mods, false)),
                    AbilityDef::CopyOnEnterUntilEot { mods, .. } => Some((index, *mods, true)),
                    _ => None,
                })
        else {
            return;
        };
        let text = own
            .base_text(index)
            .then(crate::eval::live_context(&self.state, id).text);
        crate::copiable_abilities::apply_copy(
            &mut self.state,
            crate::copiable_abilities::CopyApplication {
                source: id,
                target,
                mods,
                copy_index: u32::try_from(index).expect("ability index fits"),
                until_eot,
                resolving: None,
                text,
            },
        );
    }
}
