//! Atomic payment and moving the announced spell onto the stack.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    /// [`Self::finish_cast`] without the checkpoint.
    #[allow(clippy::too_many_lines)] // payment is a flat checklist; extraction would obscure it
    pub(super) fn pay_and_cast(&mut self, wizard: &CastWizard) -> Result<(), EngineError> {
        let face = self.wizard_face(wizard);
        if wizard.escapes()
            && wizard.escape_exiles.len() != usize::from(wizard.escape_count(face).unwrap_or(0))
        {
            return Err(EngineError::IllegalAction("escape cost is incomplete"));
        }
        // Total mana: option cost (with X) + kicker mana when taken.
        let mut total = wizard_total_cost(face, wizard);
        let player = wizard.player;
        let prepared_source = self.state.prepared_copy_source(wizard.card);
        // Delve (CR 702.66) and convoke (CR 702.51) each pay for {1} of the
        // generic part, so the count comes off the cost before anything is
        // paid — but the exiling and the tapping happen *after* the mana
        // does, below. They used to happen here, and a cast that then could
        // not pay the rest returned an error with the graveyard already
        // exiled and the creatures already tapped: the spell went back to
        // hand and the costs stayed spent. CR 601.2h reverses the whole
        // casting when a cost cannot be paid, and there is no half of it to
        // keep.
        let reduction = (wizard.delve_exiles.len() + wizard.convoke_taps.len()) as u32;
        if reduction > 0 {
            total = reduce_generic(&total, reduction);
        }
        let (total, restriction) = self.restricted_x_payment(wizard, total);
        let payment_cost = self.north_star_payment(wizard, total);
        // What the pool held of each color before the payment, restricted
        // units included: the colors it holds less of afterwards are the
        // colors spent (converge).
        let held_before = self.state.players[player.get() as usize].mana_pool.clone();
        let mut paid_mana = crate::object::PaidRecord {
            mana_spending: casting::spell_mana_spending(&self.state, player, wizard.card),
            ..Default::default()
        };
        // A free cast pays nothing but what is added to it: a kicker, the
        // replicate cost, or the costs of the modes it chose (CR 118.9d).
        let owed = pays_mana(wizard) || total != ManaCost::ZERO;
        let mut spent_riders = SmallVec::new();
        if owed {
            // Restricted mana (Cavern of Souls & co.) is spent where the
            // spell may spend it, and a rider applies for each entry that
            // actually paid. A failed payment has touched nothing.
            let allowance = self.wizard_source_allowance(wizard);
            let Some((spent, x_paid)) = casting::pay_mana_scoped_recording_x(
                &mut self.state,
                player,
                spend_for(wizard, face),
                &payment_cost,
                restriction,
                allowance,
            ) else {
                self.cast_wizard = None;
                return Err(EngineError::IllegalAction("cannot pay the total cost"));
            };
            casting::record_mana_payment(
                &mut paid_mana,
                &held_before,
                &self.state.players[player.get() as usize].mana_pool,
            );
            paid_mana.mana_types_spent_on_x = x_paid;
            spent_riders = spent;
        }
        // The mana is paid, so the rest of the cost may now be spent: the
        // Phyrexian symbols announced as life first (CR 107.4f, 119.4).
        let owed_life = phyrexian_life(wizard);
        if owed_life > 0 {
            self.state.change_life(player, -owed_life, Cause::Cost);
        }
        for &card in &wizard.delve_exiles {
            let _ = self.state.move_object(
                card,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Cost,
            )?;
        }
        for &creature in &wizard.convoke_taps {
            self.state.set_tapped(creature, true);
        }
        // Non-mana parts of the chosen alternative cost (pay life etc.).
        if let Some(CastModeKind::Alternative(i)) = wizard.option {
            let alt = &face.alternative_costs[i];
            for part in alt.cost.parts {
                if !paid_as_an_alternative_cost(part) {
                    continue;
                }
                match part {
                    CostPart::PayLife(n) => {
                        self.state.change_life(player, -i32::from(*n), Cause::Cost);
                    }
                    CostPart::ExileFromHand(_) => {
                        for card in &wizard.pitch {
                            let owner = self.state.object(*card).map_or(player, |o| o.owner);
                            self.state.move_object(
                                *card,
                                ZoneLocation::Exile(owner),
                                ZonePosition::Top,
                                Cause::Cost,
                            )?;
                        }
                    }
                    // Already skipped, by [`paid_as_an_alternative_cost`]
                    // above. Named rather than swept into a `_` so that a new
                    // `CostPart` is still a compile error in this match: the
                    // question "and is the new one paid here?" has to be asked
                    // once per list, and a wildcard answers it with silence.
                    CostPart::TapSelf
                    | CostPart::UntapSelf
                    | CostPart::ExertSelf
                    | CostPart::SacrificeSelf
                    | CostPart::Sacrifice(_)
                    | CostPart::ExilePermanent(_)
                    | CostPart::Discard(_)
                    | CostPart::DiscardRandom
                    | CostPart::DiscardHand
                    | CostPart::DiscardLastDrawnThisTurn
                    | CostPart::PayEnergy(_)
                    | CostPart::PayEnergyX
                    | CostPart::DiscardSelf
                    | CostPart::ExileSelf
                    | CostPart::ReturnSelfToHand
                    | CostPart::TapN { .. }
                    | CostPart::TapOther(_)
                    | CostPart::TapX(_)
                    | CostPart::SacrificeX(_)
                    | CostPart::SacrificeAll(_)
                    | CostPart::PayCommanderColors
                    | CostPart::Crew(_)
                    | CostPart::Saddle(_)
                    | CostPart::ReturnToHand(_)
                    | CostPart::ExileFromGraveyard(_)
                    | CostPart::RemoveCounterSelf { .. }
                    | CostPart::RemoveCounterSelfX { .. }
                    | CostPart::PutCounterSelf { .. }
                    | CostPart::PutCounterOnOther { .. }
                    | CostPart::ExileFromSingleGraveyard { .. }
                    | CostPart::PayAttachedManaCost
                    | CostPart::ExileTopOfLibrary(_)
                    | CostPart::PayLifeX => {}
                }
            }
        }
        // Mandatory additional cost parts (e.g. Toxic Deluge's pay X life).
        if wizard.option == Some(CastModeKind::BeamMeUp) {
            let returned = wizard.returned.ok_or(EngineError::IllegalAction(
                "no creature returned for beam me up",
            ))?;
            super::super::cost_wizard::pay(
                &mut self.state,
                player,
                &casting::BEAM_RETURN,
                returned,
            )?;
        }
        if let Some(beheld) = wizard.beheld
            && self
                .state
                .object(beheld)
                .is_some_and(|o| o.zone == crate::zone::Zone::Hand)
        {
            self.state.journal.record(GameEvent::Revealed {
                player,
                cards: vec![beheld],
            });
        }
        let mut sacrificed = wizard.sacrifices.iter();
        let mut sacrificed_lki = None;
        let mut sacrificed_object = None;
        let mut graveyard_batch = crate::graveyard_order::PaymentBatch::new(&self.state);
        for part in super::wizard_mandatory_parts(face, wizard.option) {
            graveyard_batch.before_part(&mut self.state, part);
            if !paid_as_a_mandatory_additional_cost(part) {
                continue;
            }
            match part {
                CostPart::PayLifeX => {
                    self.state
                        .change_life(player, -(wizard.x as i32), Cause::Cost);
                }
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
                }
                // The answer the `Sacrifice` stage took for this part, through
                // the door an activation's sacrifice goes through. Its mana
                // value is read first, off the permanent as it last existed on
                // the battlefield (CR 608.2h): "the sacrificed creature's
                // mana value" is a question the spell asks as it resolves,
                // when the card is already in a graveyard and a copy has
                // stopped being one.
                CostPart::Sacrifice(_)
                | CostPart::ExilePermanent(_)
                | CostPart::ExileFromGraveyard(_) => {
                    let Some(&chosen) = sacrificed.next() else {
                        self.cast_wizard = None;
                        return Err(EngineError::IllegalAction(
                            "a sacrifice the cast never asked about",
                        ));
                    };
                    sacrificed_object = self.state.object(chosen).map(|o| (chosen, o.version));
                    sacrificed_lki = self
                        .state
                        .object(chosen)
                        .map(crate::object::SacrificedLki::of);
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                }
                CostPart::Discard(_) => {
                    let chosen = sacrificed
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction("unanswered discard cost"))?;
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                }
                CostPart::DiscardLastDrawnThisTurn => {
                    let card =
                        self.state
                            .last_drawn_in_hand(player)
                            .ok_or(EngineError::IllegalAction(
                                "no card drawn this turn to discard",
                            ))?;
                    self.state.discard_card(card, player, Cause::Cost, None);
                }
                CostPart::DiscardHand => {
                    for card in self.state.zones.list(ZoneLocation::Hand(player)).clone() {
                        if card != wizard.card {
                            self.state.discard_card(card, player, Cause::Cost, None);
                        }
                    }
                }
                // Already skipped, by [`paid_as_a_mandatory_additional_cost`]
                // above, and named for the reason the alternative-cost match
                // names its own — with the sharper edge that no `can_afford`
                // guards this list, so the three asking costs are skipped
                // here rather than refused.
                CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::ExertSelf
                | CostPart::SacrificeSelf
                | CostPart::DiscardRandom
                | CostPart::PayEnergy(_)
                | CostPart::PayEnergyX
                | CostPart::TapN { .. }
                | CostPart::TapOther(_)
                | CostPart::TapX(_)
                | CostPart::SacrificeX(_)
                | CostPart::SacrificeAll(_)
                | CostPart::PayCommanderColors
                | CostPart::Crew(_)
                | CostPart::Saddle(_)
                | CostPart::ReturnToHand(_)
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::PutCounterOnOther { .. }
                | CostPart::ExileFromSingleGraveyard { .. }
                | CostPart::PayAttachedManaCost
                | CostPart::ExileTopOfLibrary(_)
                | CostPart::ExileFromHand(_) => {}
            }
        }
        graveyard_batch.finish(&mut self.state);
        // Escape may exile a card sacrificed or discarded for this same
        // spell. All nonmana costs above are paid first (CR 601.2h).
        for &card in &wizard.escape_exiles {
            if !self.state.object(card).is_some_and(|object| {
                object.zone == crate::zone::Zone::Graveyard && object.zone_owner == Some(player)
            }) {
                return Err(EngineError::IllegalAction(
                    "escape card is not in your graveyard",
                ));
            }
            self.state.move_object(
                card,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Cost,
            )?;
        }

        // Move the card to the stack as a spell. The targeting requirement
        // rides along on the object: a copy of this spell may be retargeted
        // during resolution (CR 707.10c), and the resolver has no card lookup
        // of its own to re-derive it from.
        let target_req = self.wizard_target_req(wizard);
        let second_target_req = self.wizard_second_target_req(wizard);
        let card = wizard.card;
        // The face being cast prints "exile it instead" (a disturb back): the
        // object still shows the face it had in the graveyard, so the face is
        // read off the cast option, as the target requirement above is.
        let exiles_itself = {
            let face = wizard.face_index();
            self.state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .is_some_and(|def| {
                    def.abilities_for_face(face.min(def.faces.len() - 1))
                        .iter()
                        .any(|a| {
                            matches!(
                                a,
                                AbilityDef::Replacement(
                                    baylee_cards_dsl::ReplacementRule::ExileSelfInsteadOfGraveyard
                                )
                            )
                        })
                })
        };
        {
            let obj = self.state.object_mut(card).expect("wizard card exists");
            obj.kind = ObjectKind::Spell;
            obj.set_controller(player);
            obj.targets.clone_from(&wizard.targets);
            obj.target_players = wizard.target_players;
            obj.target_req = target_req;
            obj.set_second(wizard.second_targets.clone(), second_target_req);
            obj.set_second_players(wizard.second_target_players);
            obj.x_value = wizard.x;
            obj.kicked = wizard.kicked;
            obj.replicated = wizard.replicated;
            obj.alt_cast = matches!(wizard.option, Some(CastModeKind::Alternative(_)));
            obj.chosen_player = wizard.chosen_player;
            // Where it was cast from, read before it moves: a card cast from
            // exile under a play permission (Expressive Iteration) or from
            // the command zone was not cast from a hand, and rebound reads
            // this (CR 702.88a).
            obj.cast_history
                .set_from_hand(obj.zone == crate::zone::Zone::Hand);
            obj.cast_history
                .set_from_graveyard(obj.zone == crate::zone::Zone::Graveyard);
            // The chosen permission determines the replacement, independent
            // of other permissions that happen to be available.
            if obj.zone == crate::zone::Zone::Graveyard {
                if matches!(
                    wizard.option,
                    Some(CastModeKind::Flashback | CastModeKind::BeamMeUp)
                ) || wizard.permission == Some(casting::GraveyardPermission::Flashback)
                {
                    obj.riders.push(crate::object::Rider::Flashback);
                }
                obj.cast_history.set_from_hand(false);
            }
            // Every cast decides afresh: the same card cast later from a
            // hand, front face up, is not exiled for what it once was.
            obj.riders
                .retain(|r| *r != crate::object::Rider::ExileInsteadOfGraveyard);
            // Dash (CR 702.109a): the spell was cast for its dash cost, and
            // the permanent it becomes has haste and goes back at the next
            // end step (`Engine::finalize_spell` writes that trigger).
            obj.riders.retain(|r| *r != crate::object::Rider::Dashed);
            if wizard.option == Some(CastModeKind::Dash) {
                obj.riders.push(crate::object::Rider::Dashed);
            }
            obj.riders.retain(|r| *r != crate::object::Rider::Mutating);
            if wizard.option == Some(CastModeKind::Mutate) {
                obj.riders.push(crate::object::Rider::Mutating);
            }
            // Escape (CR 702.138b): the spell escaped, and so will the
            // permanent it becomes.
            obj.riders.retain(|r| *r != crate::object::Rider::Escaped);
            if wizard.escapes() {
                obj.riders.push(crate::object::Rider::Escaped);
            }
            if exiles_itself {
                obj.riders
                    .push(crate::object::Rider::ExileInsteadOfGraveyard);
            }
            obj.mode_index = match wizard.option {
                Some(CastModeKind::Mode(i)) => Some(i.try_into().expect("mode index fits u8")),
                _ => None,
            };
            obj.modes = match wizard.option {
                Some(CastModeKind::Modes(set)) => set,
                _ => 0,
            };
        }
        // Commander-cast tracking: casts from the command zone count, once
        // for the seat (Commander's Insight) and once for the commander
        // itself (CR 903.8's tax). Both here, so the two cannot drift.
        if self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Command)
        {
            if let Some(v) = self.state.commander_casts.get_mut(player.get() as usize) {
                *v = v.saturating_add(1);
            }
            if let Some(c) = self
                .state
                .commanders
                .get_mut(player.get() as usize)
                .and_then(|cs| cs.iter_mut().find(|c| c.object == card))
            {
                c.casts = c.casts.saturating_add(1);
            }
        }
        // MDFC back-face cast: the object becomes its chosen face (CR 712.11b).
        let i = wizard.face_index();
        if i != 0 {
            let def = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .expect("wizard card known");
            self.state.switch_face(card, def, i);
        }
        self.state
            .move_object(card, ZoneLocation::Stack, ZonePosition::Top, Cause::Spell)?;
        self.state
            .object_mut(card)
            .expect("cast spell")
            .cast_history
            .set_caster(Some(player));
        // What was paid, written after the move for the reason the move
        // gives it up: the spell on the stack is the object that reads it.
        // A free cast spent no mana (CR 601.2h pays nothing it was not
        // asked for), whatever its printed cost says.
        if (paid_mana.mana_spent > 0 || sacrificed_lki.is_some())
            && let Some(obj) = self.state.object_mut(card)
        {
            paid_mana.sacrificed_lki = sacrificed_lki;
            paid_mana.sacrificed = sacrificed_object;
            obj.paid = Some(Box::new(paid_mana));
        }
        if matches!(wizard.option, Some(CastModeKind::Prototype))
            && let Some(prototype) = face.prototype
            && let Some(obj) = self.state.object_mut(card)
        {
            obj.original_base = Some(std::sync::Arc::clone(&obj.base));
            obj.prototyped = true;
            let base = obj.base_mut();
            base.mana_cost = prototype.cost;
            base.colors = if base.keywords.contains(baylee_cards_dsl::KeywordSet::DEVOID) {
                baylee_core::color::ColorSet::EMPTY
            } else {
                prototype.cost.colors()
            };
            base.power = Some(prototype.power);
            base.toughness = Some(prototype.toughness);
            obj.cache.clear();
        }
        if wizard.masked || matches!(wizard.option, Some(CastModeKind::Disguise)) {
            self.make_disguised(card);
            if wizard.masked
                && let Some(object) = self.state.object_mut(card)
            {
                object.riders.push(crate::object::Rider::Masked);
            }
        }
        self.apply_spend_riders(player, wizard.card, &spent_riders);
        if self.state.object(card).is_some_and(|object| {
            object.characteristics().types.intersects(
                baylee_core::types::TypeSet::INSTANT.union(baylee_core::types::TypeSet::SORCERY),
            )
        }) {
            self.state.per_turn.instant_or_sorcery_cast[usize::from(player.get())] = true;
        }
        // Per-turn tracking for conditional triggers (Esper Sentinel).
        if !wizard.masked
            && !matches!(wizard.option, Some(CastModeKind::Disguise))
            && !face.types.contains(baylee_core::types::TypeSet::CREATURE)
            && let Some(v) = self
                .state
                .per_turn
                .noncreature_spells
                .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        if let Some(v) = self
            .state
            .per_turn
            .spells_cast
            .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        // "The next spell you cast this turn has improvise" is spent by this
        // cast, whatever it tapped (Archway of Innovation).
        if let Some(flag) = self
            .state
            .per_turn
            .improvise_next_spell
            .get_mut(player.get() as usize)
        {
            *flag = false;
        }
        // "If you do, you can't cast additional spells this turn": the
        // spell is cast, so the lock is on.
        if wizard.by_effect == Some(EffectCast::ThenNoMoreSpells)
            && let Some(v) = self
                .state
                .per_turn
                .no_more_spells
                .get_mut(player.get() as usize)
        {
            *v = true;
        }
        self.state
            .bind_target_references(card, &wizard.target_references);
        self.store_damage_division(wizard);
        // CR 722.3c: announcing or cancelling never spends the designation.
        if let Some(source) = prepared_source {
            self.state.unprepare(source);
        }
        if let Some(effect) = wizard.north_star {
            self.state.effects.remove_where(|entry| entry.id == effect);
        }
        self.apply_graveyard_cast_grants(wizard);
        self.apply_next_spell_grants(player, card);
        let instant_number = if self.state.object(card).is_some_and(|object| {
            object
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::INSTANT)
        }) {
            let count = &mut self.state.per_turn.instants_cast[usize::from(player.get())];
            *count = count.saturating_add(1);
            *count
        } else {
            0
        };
        self.state.journal.record(GameEvent::SpellCast {
            instant_number,
            object: card,
            player,
            number: self.state.per_turn.spells_cast_by(player),
        });
        self.cast_wizard = None;
        if !self.finish_nested_cast() {
            self.after_action(player);
        }
        Ok(())
    }
}
