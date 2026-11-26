//! Atomic payment and moving the announced spell onto the stack.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// [`Self::finish_cast`] without the checkpoint.
    #[allow(clippy::too_many_lines)] // payment is a flat checklist; extraction would obscure it
    pub(super) fn pay_and_cast(&mut self, wizard: &CastWizard) -> Result<(), EngineError> {
        let face = self.wizard_face(wizard);
        // Freeze externally imposed costs before any payment can change the board.
        let imposed =
            casting::additional_spell_costs(&self.state, wizard.player, wizard.card, face);
        if wizard.escapes()
            && wizard.escape_exiles.len() != usize::from(wizard.escape_count(face).unwrap_or(0))
        {
            return Err(EngineError::IllegalAction("escape cost is incomplete"));
        }
        // Total mana: option cost (with X) + kicker mana when taken.
        let mut total = wizard_total_cost(face, wizard);
        let player = wizard.player;
        // Validate every fixed life payment before touching the mana pool. A
        // cancelled cast that has looked at or moved its library cannot always
        // be rewound by the generic priority checkpoint alone.
        let library_top_life = if let Some(CastModeKind::LibraryTopLife {
            life,
            face: route_face,
            ..
        }) = wizard.announced
        {
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .ok_or(EngineError::IllegalAction("unknown library top card"))?;
            let Some(_) = def.faces.get(route_face) else {
                return Err(EngineError::IllegalAction("unknown library top face"));
            };
            let total_life = wizard.total_life_cost(face);
            let open_by_grant = wizard.selected_alt_grant().is_some_and(|r| {
                r.life == Some(life)
                    && casting::alt_cast_route_still_open(
                        &self.state,
                        &self.lookup,
                        player,
                        wizard.card,
                        &r,
                    )
            });
            if route_face != wizard.face_index()
                || life != casting::library_top_life_value(def, route_face)
                || (!casting::library_top_life_open(&self.state, player, wizard.card)
                    && !open_by_grant)
                || !self.state.can_pay_life(player, total_life)
            {
                return Err(EngineError::IllegalAction(
                    "the library top life cost is no longer payable",
                ));
            }
            Some(life)
        } else {
            if let Some(route) = wizard.selected_alt_grant()
                && !casting::alt_cast_route_still_open(
                    &self.state,
                    &self.lookup,
                    player,
                    wizard.card,
                    &route,
                )
            {
                return Err(EngineError::IllegalAction(
                    "the alternative cost permission is no longer active",
                ));
            }
            None
        };
        let prepared_source = self.state.prepared_copy_source(wizard.card);
        // Delve (CR 702.66) and convoke (CR 702.51) each pay for {1} of the
        // generic part, so the count comes off the cost before anything is
        // paid — but the exiling and the tapping happen *after* the mana
        // does, below. They used to happen here, and a cast that then could
        // not pay the rest returned an error with the graveyard already
        // exiled and the creatures already tapped: the spell went back to
        // hand and the costs stayed spent. CR 601.2h reverses the whole
        // casting when a cost cannot be paid, and there is no half of it to
        // keep. A convoke tap pays a coloured symbol of its colour where it
        // can (CR 702.51a) and generic mana otherwise, the same reading the
        // offer and the payment window make (`wizard_paid_total`).
        let (convoked, colored) = total.convoked_colored(&self.convoke_tap_colors(wizard));
        total = convoked;
        let reduction = u32::try_from(wizard.delve_exiles.len() + wizard.convoke_taps.len())
            .unwrap_or(u32::MAX)
            .saturating_sub(colored)
            .saturating_add(wizard.harmonize_reduction(&self.state))
            .saturating_add(wizard.assist_paid);
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
            let spend = spend_for(&self.state, wizard, face);
            let Some((spent, x_paid)) = casting::pay_mana_scoped_recording_x(
                &mut self.state,
                player,
                spend,
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
        let target_life = i32::try_from(wizard.target_life_increase).unwrap_or(i32::MAX);
        if target_life > 0
            && !self
                .state
                .can_pay_life(player, wizard.total_life_cost(face))
        {
            self.cast_wizard = None;
            return Err(EngineError::IllegalAction("cannot pay life cost"));
        }
        let owed_life = phyrexian_life(wizard);
        if owed_life > 0 {
            self.state.change_life(player, -owed_life, Cause::Cost);
        }
        if target_life > 0 {
            self.state.change_life(player, -target_life, Cause::Cost);
        }
        if let Some(life) = library_top_life {
            let life = i32::try_from(life).unwrap_or(i32::MAX);
            self.state.change_life(player, -life, Cause::Cost);
        }
        if let Some(route) = wizard.selected_alt_grant()
            && route.once_per_turn
        {
            self.state
                .per_turn
                .graveyard_allowances_used
                .push((player, route.effect_id));
        }
        let mut delved: SmallVec<[ObjectId; 8]> = SmallVec::new();
        for &card in &wizard.delve_exiles {
            delved.push(self.state.with_exiler(player, |state| {
                state.move_object(
                    card,
                    ZoneLocation::Exile(player),
                    ZonePosition::Top,
                    Cause::Cost,
                )
            })?);
        }
        let convoked_count = if crate::casting::has_convoke(
            &self.state,
            wizard.card,
            face,
            player,
            wizard.face_index(),
        ) {
            u16::try_from(
                wizard
                    .convoke_taps
                    .iter()
                    .filter(|&&id| {
                        self.state.object(id).is_some_and(|o| {
                            o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                        })
                    })
                    .count(),
            )
            .unwrap_or(u16::MAX)
        } else {
            0
        };
        for &creature in &wizard.convoke_taps {
            self.state.set_tapped(creature, true);
        }
        if let Some(harmonized) = wizard.harmonize_tap {
            self.state.set_tapped(harmonized, true);
        }
        // One ordered answer stream feeds both the selected alternative cost
        // and any mandatory additional costs. No object can pay two parts.
        let mut cost_objects = wizard.cost_objects.iter();
        let mut counter_removals = wizard.counter_removals.iter();
        let mut evidence_groups = wizard.evidence_exiles.iter();
        let mut sacrificed_lki = None;
        let mut sacrificed_object = None;
        let mut spell_cost_exiles: Vec<baylee_core::ids::DamageSourceRef> = Vec::new();
        let mut beheld_cost_power: Option<(Option<baylee_core::ids::DamageSourceRef>, i32, u32)> =
            None;
        let record_exiled_cost = |state: &crate::state::GameState,
                                  exiles: &mut Vec<baylee_core::ids::DamageSourceRef>,
                                  exiled: ObjectId| {
            if let Some(ex_ref) = state.source_identity(exiled).filter(|_| {
                state
                    .object(exiled)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
            }) && !exiles.contains(&ex_ref)
            {
                exiles.push(ex_ref);
            }
        };
        let snapshot_beheld_power = |state: &crate::state::GameState, chosen: ObjectId| {
            state.object(chosen).map(|o| {
                let ch = o.characteristics();
                let p = i32::from(ch.power.unwrap_or(0));
                let mv = ch.mana_value();
                let bf_ref = (o.zone == crate::zone::Zone::Battlefield).then_some(
                    baylee_core::ids::DamageSourceRef {
                        object: chosen,
                        version: o.version,
                    },
                );
                (bf_ref, p, mv)
            })
        };
        // Each selected card pays exactly one exile-from-hand part.
        for &card in &wizard.pitch {
            let owner = self.state.object(card).map_or(player, |o| o.owner);
            self.state.move_object(
                card,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Cost,
            )?;
            record_exiled_cost(&self.state, &mut spell_cost_exiles, card);
        }
        // Non-mana parts of the chosen alternative or flashback cost.
        for part in
            &super::cost_parts::wizard_alternative_parts_expanded(face, wizard.cost_choice())
        {
            if !paid_as_an_alternative_cost(part) {
                continue;
            }
            match part {
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
                }
                CostPart::DealDamageToYou(n) => {
                    if *n > 0 {
                        let assignment = crate::damage::Assignment {
                            source: wizard.card,
                            source_version: self.state.object(wizard.card).map(|o| o.version),
                            recipient: crate::event::DamageTarget::Player(player),
                            amount: u32::from(*n),
                            is_combat: false,
                        };
                        let mut work =
                            crate::damage::DamageWork::new(&mut self.state, vec![assignment]);
                        work.finish_cost_damage(&mut self.state);
                    }
                }
                CostPart::Forage => {
                    let first = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction(
                            "an object cost the cast never asked about",
                        ))?;
                    if self
                        .state
                        .object(first)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                    {
                        sacrificed_object = self.state.object(first).map(|o| (first, o.version));
                        sacrificed_lki = self
                            .state
                            .object(first)
                            .map(crate::object::SacrificedLki::of);
                        super::super::cost_wizard::pay(&mut self.state, player, part, first)?;
                    } else {
                        let second =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "incomplete graveyard forage cost",
                                ))?;
                        let third =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "incomplete graveyard forage cost",
                                ))?;
                        for id in [first, second, third] {
                            super::super::cost_wizard::pay(&mut self.state, player, part, id)?;
                            record_exiled_cost(&self.state, &mut spell_cost_exiles, id);
                        }
                    }
                }
                CostPart::Sacrifice(_)
                | CostPart::ReturnToHand(_)
                | CostPart::PutCounterOnOther { .. }
                | CostPart::Behold(_)
                | CostPart::BeholdAndExile(_)
                | CostPart::RevealFromHand(_) => {
                    let chosen = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction(
                            "an object cost the cast never asked about",
                        ))?;
                    // Only a sacrifice feeds sentences about the sacrificed
                    // permanent; returning one is a different payment.
                    if matches!(part, CostPart::Sacrifice(_))
                        || (matches!(part, CostPart::BeholdAndExile(_))
                            && self
                                .state
                                .object(chosen)
                                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield))
                    {
                        sacrificed_object = self.state.object(chosen).map(|o| (chosen, o.version));
                        sacrificed_lki = self
                            .state
                            .object(chosen)
                            .map(crate::object::SacrificedLki::of);
                    }
                    if matches!(
                        part,
                        CostPart::Behold(_)
                            | CostPart::BeholdAndExile(_)
                            | CostPart::RevealFromHand(_)
                    ) {
                        beheld_cost_power = snapshot_beheld_power(&self.state, chosen);
                    }
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    if matches!(part, CostPart::BeholdAndExile(_)) {
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, chosen);
                    }
                }
                CostPart::CollectEvidence(mv) => {
                    let chosen = evidence_groups.next().ok_or(EngineError::IllegalAction(
                        "unanswered collect evidence cost",
                    ))?;
                    let mut total_mv = 0u32;
                    for &card in chosen {
                        let Some(obj) = self.state.object(card).filter(|o| {
                            o.zone == crate::zone::Zone::Graveyard
                                && o.zone_owner == Some(player)
                                && o.is_card()
                        }) else {
                            return Err(EngineError::IllegalAction(
                                "collect evidence card is not in your graveyard",
                            ));
                        };
                        total_mv = total_mv.saturating_add(obj.characteristics().mana_value());
                    }
                    if total_mv < u32::from(*mv) {
                        return Err(EngineError::IllegalAction(
                            "insufficient mana value for collect evidence",
                        ));
                    }
                    for &card in chosen {
                        super::super::cost_wizard::pay(&mut self.state, player, part, card)?;
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, card);
                    }
                }
                CostPart::TapN { count, .. } => {
                    for _ in 0..*count {
                        let chosen =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "the cast did not choose every tap",
                                ))?;
                        super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    }
                }
                CostPart::RemoveAnyCounter { .. } | CostPart::RemoveAnyCounterX { .. } => {
                    let ctx = eval::live_context(&self.state, wizard.card);
                    super::super::cost_wizard::counter_remove::pay_counter_removals(
                        &mut self.state,
                        player,
                        ctx,
                        part,
                        wizard.x,
                        &mut counter_removals,
                    )?;
                }
                // Pitch cards were moved above; keep every other no-op variant
                // named so a new cost cannot silently become free.
                CostPart::ExileFromHand(_)
                | CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::ExertSelf
                | CostPart::RevealChosenPlayer
                | CostPart::SacrificeSelf
                | CostPart::ExilePermanent(_)
                | CostPart::Discard(_)
                | CostPart::DiscardRandom
                | CostPart::DiscardHand
                | CostPart::DiscardLastDrawnThisTurn
                | CostPart::PayEnergy(_)
                | CostPart::PayEnergyX
                | CostPart::DiscardSelf
                | CostPart::RevealSelfFromHand
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::TapOther(_)
                | CostPart::TapX(_)
                | CostPart::SacrificeX(_)
                | CostPart::SacrificeAll(_)
                | CostPart::PayCommanderColors
                | CostPart::Crew(_)
                | CostPart::Saddle(_)
                | CostPart::ExileFromGraveyard(_)
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::ExileFromSingleGraveyard { .. }
                | CostPart::PayAttachedManaCost
                | CostPart::ExileTopOfLibrary(_)
                | CostPart::SacrificeGrantor
                | CostPart::ReturnUnblockedAttacker
                | CostPart::DiscardX(_)
                | CostPart::RevealFromHandX(_)
                | CostPart::ExileFromGraveyardX(_)
                | CostPart::CraftMaterials { .. }
                | CostPart::Either(_, _)
                | CostPart::Waterbend(_)
                | CostPart::WaterbendX
                | CostPart::PayLifeX => {}
            }
        }
        // Mandatory additional cost parts (e.g. Toxic Deluge's pay X life).
        if wizard.option == Some(CastModeKind::Emerge) {
            let emerge = face
                .emerge
                .ok_or(EngineError::IllegalAction("no emerge cost on this spell"))?;
            let victim = wizard
                .emerge_sacrifice
                .ok_or(EngineError::IllegalAction("no permanent chosen for emerge"))?;
            sacrificed_object = self.state.object(victim).map(|o| (victim, o.version));
            sacrificed_lki = self
                .state
                .object(victim)
                .map(crate::object::SacrificedLki::of);
            super::super::cost_wizard::pay(
                &mut self.state,
                player,
                &CostPart::Sacrifice(emerge.from),
                victim,
            )?;
        }
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
        let mut sneak_defender = None;
        if wizard.option == Some(CastModeKind::Sneak) {
            let returned = wizard.returned.ok_or(EngineError::IllegalAction(
                "no unblocked creature returned for sneak",
            ))?;
            sneak_defender = self
                .state
                .combat
                .attackers()
                .iter()
                .find(|a| a.creature == returned)
                .map(|a| {
                    let ver = match a.defending {
                        baylee_core::ids::Defender::Planeswalker(id)
                        | baylee_core::ids::Defender::Battle(id) => {
                            self.state.object(id).map(|o| o.version)
                        }
                        baylee_core::ids::Defender::Player(_) => None,
                    };
                    (a.defending, ver)
                });
            super::super::cost_wizard::pay(
                &mut self.state,
                player,
                &CostPart::ReturnUnblockedAttacker,
                returned,
            )?;
        }
        if let Some(beheld) = wizard.beheld {
            beheld_cost_power = snapshot_beheld_power(&self.state, beheld);
            if self
                .state
                .object(beheld)
                .is_some_and(|o| o.zone == crate::zone::Zone::Hand)
            {
                self.state.journal.record(GameEvent::Revealed {
                    player,
                    cards: vec![beheld],
                });
            }
        }
        let mut graveyard_batch = crate::graveyard_order::PaymentBatch::new(&self.state);
        for part in super::wizard_mandatory_parts(face, wizard.cost_choice()) {
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
                CostPart::DealDamageToYou(n) => {
                    if *n > 0 {
                        let assignment = crate::damage::Assignment {
                            source: wizard.card,
                            source_version: self.state.object(wizard.card).map(|o| o.version),
                            recipient: crate::event::DamageTarget::Player(player),
                            amount: u32::from(*n),
                            is_combat: false,
                        };
                        let mut work =
                            crate::damage::DamageWork::new(&mut self.state, vec![assignment]);
                        work.finish_cost_damage(&mut self.state);
                    }
                }
                CostPart::Forage => {
                    let first = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction(
                            "an object cost the cast never asked about",
                        ))?;
                    if self
                        .state
                        .object(first)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                    {
                        sacrificed_object = self.state.object(first).map(|o| (first, o.version));
                        sacrificed_lki = self
                            .state
                            .object(first)
                            .map(crate::object::SacrificedLki::of);
                        super::super::cost_wizard::pay(&mut self.state, player, part, first)?;
                    } else {
                        let second =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "incomplete graveyard forage cost",
                                ))?;
                        let third =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "incomplete graveyard forage cost",
                                ))?;
                        for id in [first, second, third] {
                            super::super::cost_wizard::pay(&mut self.state, player, part, id)?;
                            record_exiled_cost(&self.state, &mut spell_cost_exiles, id);
                        }
                    }
                }
                CostPart::DiscardX(_)
                | CostPart::RevealFromHandX(_)
                | CostPart::ExileFromGraveyardX(_) => {
                    for _ in 0..wizard.x {
                        let chosen =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "an object cost the cast never asked about",
                                ))?;
                        if matches!(part, CostPart::DiscardX(_))
                            && let Some(reference) = self.state.source_identity(chosen)
                        {
                            paid_mana.discarded.push(reference);
                        }
                        super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                        if matches!(part, CostPart::ExileFromGraveyardX(_)) {
                            record_exiled_cost(&self.state, &mut spell_cost_exiles, chosen);
                        }
                    }
                }
                CostPart::PayEnergy(n) => {
                    if !self.state.pay_energy(player, u32::from(*n)) {
                        return Err(EngineError::IllegalAction("not enough energy"));
                    }
                }
                CostPart::PayEnergyX => {
                    if !self.state.pay_energy(player, wizard.x) {
                        return Err(EngineError::IllegalAction("not enough energy"));
                    }
                }
                CostPart::Sacrifice(_) | CostPart::ExilePermanent(_) => {
                    let chosen = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction(
                            "an object cost the cast never asked about",
                        ))?;
                    // Both payments can refer back to the permanent as it was
                    // on the battlefield (Soul Exchange, CR 608.2h).
                    sacrificed_object = self.state.object(chosen).map(|o| (chosen, o.version));
                    sacrificed_lki = self
                        .state
                        .object(chosen)
                        .map(crate::object::SacrificedLki::of);
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    if matches!(part, CostPart::ExilePermanent(_)) {
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, chosen);
                    }
                }
                CostPart::ReturnToHand(_)
                | CostPart::TapOther(_)
                | CostPart::PutCounterOnOther { .. }
                | CostPart::ExileFromGraveyard(_)
                | CostPart::ExileFromHand(_)
                | CostPart::Discard(_)
                | CostPart::Behold(_)
                | CostPart::BeholdAndExile(_)
                | CostPart::RevealFromHand(_) => {
                    let chosen = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction(
                            "an object cost the cast never asked about",
                        ))?;
                    if matches!(part, CostPart::BeholdAndExile(_))
                        && self
                            .state
                            .object(chosen)
                            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                    {
                        sacrificed_object = self.state.object(chosen).map(|o| (chosen, o.version));
                        sacrificed_lki = self
                            .state
                            .object(chosen)
                            .map(crate::object::SacrificedLki::of);
                    }
                    if matches!(
                        part,
                        CostPart::Behold(_)
                            | CostPart::BeholdAndExile(_)
                            | CostPart::RevealFromHand(_)
                    ) {
                        beheld_cost_power = snapshot_beheld_power(&self.state, chosen);
                    }
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    if matches!(
                        part,
                        CostPart::ExileFromGraveyard(_)
                            | CostPart::ExileFromHand(_)
                            | CostPart::BeholdAndExile(_)
                    ) {
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, chosen);
                    }
                }
                CostPart::CollectEvidence(mv) => {
                    let chosen = evidence_groups.next().ok_or(EngineError::IllegalAction(
                        "unanswered collect evidence cost",
                    ))?;
                    let mut total_mv = 0u32;
                    for &card in chosen {
                        let Some(obj) = self.state.object(card).filter(|o| {
                            o.zone == crate::zone::Zone::Graveyard
                                && o.zone_owner == Some(player)
                                && o.is_card()
                        }) else {
                            return Err(EngineError::IllegalAction(
                                "collect evidence card is not in your graveyard",
                            ));
                        };
                        total_mv = total_mv.saturating_add(obj.characteristics().mana_value());
                    }
                    if total_mv < u32::from(*mv) {
                        return Err(EngineError::IllegalAction(
                            "insufficient mana value for collect evidence",
                        ));
                    }
                    for &card in chosen {
                        super::super::cost_wizard::pay(&mut self.state, player, part, card)?;
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, card);
                    }
                }
                CostPart::SacrificeX(_) => {
                    for _ in 0..wizard.x {
                        let chosen =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "an object cost the cast never asked about",
                                ))?;
                        sacrificed_object = self.state.object(chosen).map(|o| (chosen, o.version));
                        sacrificed_lki = self
                            .state
                            .object(chosen)
                            .map(crate::object::SacrificedLki::of);
                        super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    }
                }
                CostPart::SacrificeAll(_) => {
                    let targets =
                        super::super::cost_wizard::options(&self.state, player, wizard.card, part);
                    for chosen in targets {
                        if chosen != wizard.card {
                            super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                        }
                    }
                }
                CostPart::TapX(_) | CostPart::Waterbend(_) | CostPart::WaterbendX => {
                    if let CostPart::TapX(_) = part {
                        for _ in 0..wizard.x {
                            let chosen =
                                cost_objects
                                    .next()
                                    .copied()
                                    .ok_or(EngineError::IllegalAction(
                                        "the cast did not choose every tap",
                                    ))?;
                            super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                        }
                    }
                }
                CostPart::TapN { count, .. } => {
                    for _ in 0..*count {
                        let chosen =
                            cost_objects
                                .next()
                                .copied()
                                .ok_or(EngineError::IllegalAction(
                                    "the cast did not choose every tap",
                                ))?;
                        super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    }
                }
                CostPart::RemoveAnyCounter { .. } | CostPart::RemoveAnyCounterX { .. } => {
                    let ctx = eval::live_context(&self.state, wizard.card);
                    super::super::cost_wizard::counter_remove::pay_counter_removals(
                        &mut self.state,
                        player,
                        ctx,
                        part,
                        wizard.x,
                        &mut counter_removals,
                    )?;
                }
                CostPart::ExileTopOfLibrary(n) => {
                    let count = usize::from(*n);
                    if self.state.zones.list(ZoneLocation::Library(player)).len() < count {
                        return Err(EngineError::IllegalAction(
                            "not enough cards in library to exile",
                        ));
                    }
                    for _ in 0..count {
                        let top = *self
                            .state
                            .zones
                            .list(ZoneLocation::Library(player))
                            .last()
                            .expect("library count checked");
                        let owner = self.state.object(top).map_or(player, |o| o.owner);
                        self.state.move_object(
                            top,
                            ZoneLocation::Exile(owner),
                            ZonePosition::Top,
                            Cause::Cost,
                        )?;
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, top);
                    }
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
                // Random discard is paid in the CR 601.2h random step below.
                // Unsupported activation-only variants stay named to prevent silent free costs.
                CostPart::DiscardRandom
                | CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::ExertSelf
                | CostPart::RevealChosenPlayer
                | CostPart::SacrificeSelf
                | CostPart::PayCommanderColors
                | CostPart::Crew(_)
                | CostPart::Saddle(_)
                | CostPart::DiscardSelf
                | CostPart::RevealSelfFromHand
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::ExileFromSingleGraveyard { .. }
                | CostPart::PayAttachedManaCost
                | CostPart::ReturnUnblockedAttacker
                | CostPart::CraftMaterials { .. }
                | CostPart::Either(_, _)
                | CostPart::SacrificeGrantor => {}
            }
        }
        // Buyback's parts name no object any "sacrificed creature" of the
        // spell reads: that sentence is linked to the printed additional
        // cost (CR 607.2i), so the receipt above is not touched.
        let buyback = super::cost_parts::wizard_buyback_parts_expanded(face, wizard.cost_choice());
        for part in &buyback {
            graveyard_batch.before_part(&mut self.state, part);
            if !super::paid_as_a_buyback_cost(part) {
                continue;
            }
            match part {
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
                }
                CostPart::DealDamageToYou(n) => {
                    if *n > 0 {
                        let assignment = crate::damage::Assignment {
                            source: wizard.card,
                            source_version: self.state.object(wizard.card).map(|o| o.version),
                            recipient: crate::event::DamageTarget::Player(player),
                            amount: u32::from(*n),
                            is_combat: false,
                        };
                        let mut work =
                            crate::damage::DamageWork::new(&mut self.state, vec![assignment]);
                        work.finish_cost_damage(&mut self.state);
                    }
                }
                CostPart::Forage => {
                    let first = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction("an unanswered buyback cost"))?;
                    if self
                        .state
                        .object(first)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                    {
                        super::super::cost_wizard::pay(&mut self.state, player, part, first)?;
                    } else {
                        let second = cost_objects
                            .next()
                            .copied()
                            .ok_or(EngineError::IllegalAction("an unanswered buyback cost"))?;
                        let third = cost_objects
                            .next()
                            .copied()
                            .ok_or(EngineError::IllegalAction("an unanswered buyback cost"))?;
                        for id in [first, second, third] {
                            super::super::cost_wizard::pay(&mut self.state, player, part, id)?;
                        }
                    }
                }
                CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::PutCounterOnOther { .. }
                | CostPart::Behold(_)
                | CostPart::BeholdAndExile(_)
                | CostPart::RevealFromHand(_) => {
                    let chosen = cost_objects
                        .next()
                        .copied()
                        .ok_or(EngineError::IllegalAction("an unanswered buyback cost"))?;
                    super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                    if matches!(part, CostPart::BeholdAndExile(_)) {
                        record_exiled_cost(&self.state, &mut spell_cost_exiles, chosen);
                    }
                }
                CostPart::RemoveAnyCounter { .. } => {
                    let ctx = eval::live_context(&self.state, wizard.card);
                    super::super::cost_wizard::counter_remove::pay_counter_removals(
                        &mut self.state,
                        player,
                        ctx,
                        part,
                        wizard.x,
                        &mut counter_removals,
                    )?;
                }
                _ => {}
            }
        }
        // Last, as `additional_card_costs_for` asks them, so the nth answer
        // pays the nth part. Like buyback they leave the receipt alone: an
        // imposed Swamp is no "sacrificed creature" (CR 607.2i).
        for part in &imposed {
            graveyard_batch.before_part(&mut self.state, part);
            let chosen = cost_objects
                .next()
                .copied()
                .ok_or(EngineError::IllegalAction("an unanswered imposed cost"))?;
            super::super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
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
        // CR 601.2h: costs with a random element are paid after the others,
        // out of the hand the others left. The card being cast is on the
        // stack by now in all but this object's bookkeeping (CR 601.2a).
        for _ in super::wizard_cost_parts(face, wizard.cost_choice())
            .filter(|part| matches!(part, CostPart::DiscardRandom))
        {
            let hand: Vec<_> = self
                .state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|id| *id != wizard.card)
                .collect();
            if hand.is_empty() {
                return Err(EngineError::IllegalAction("no card to discard at random"));
            }
            let at = self.state.rng.below(hand.len() as u64) as usize;
            self.state.discard_card(hand[at], player, Cause::Cost, None);
        }

        // Move the card to the stack as a spell. The targeting requirement
        // rides along on the object: a copy of this spell may be retargeted
        // during resolution (CR 707.10c), and the resolver has no card lookup
        // of its own to re-derive it from.
        let target_req = self.wizard_target_req(wizard);
        let second_target_req = self.wizard_second_target_req(wizard);
        let extra_target_instances: Vec<crate::object::ExtraTargetInstance> = wizard
            .extra_targets
            .iter()
            .enumerate()
            .filter_map(|(i, (targets, players))| {
                let (_, _, req) = self.wizard_extra_clause(wizard, 2 + i)?;
                Some(crate::object::ExtraTargetInstance {
                    targets: targets.clone(),
                    players: *players,
                    req: Some(req),
                })
            })
            .collect();
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
        // "Was foretold" belongs to the spell even when another permission
        // chooses a different cost. Capture it before leaving exile ends the
        // exiled card's identity, then write it on the spell after that move.
        let foretold_turn = self.state.object(card).and_then(|object| {
            object.riders.iter().find_map(|rider| match rider {
                crate::object::Rider::Foretold { turn } => Some(*turn),
                _ => None,
            })
        });
        let controlled_subtypes_on_cast: smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]> = {
            let mut wanted: smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]> =
                smallvec::SmallVec::new();
            let face_idx = wizard.face_index();
            if let Some(def) = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            {
                let f = &def.faces[face_idx.min(def.faces.len() - 1)];
                for em in f.enter_modifiers {
                    if let baylee_cards_dsl::EnterModifier::WithCountersIf { condition, .. } = em {
                        collect_cast_subtypes(condition, &mut wanted);
                    }
                }
                for ab in def.abilities_for_face(face_idx.min(def.faces.len() - 1)) {
                    collect_ability_cast_subtypes(ab, &mut wanted);
                }
            }
            wanted
                .into_iter()
                .filter(|&sub| {
                    self.state.battlefield_seen().any(|id| {
                        id != card
                            && self.state.object(id).is_some_and(|o| {
                                o.controller == player && o.characteristics().subtypes.contains(sub)
                            })
                    })
                })
                .collect()
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
            obj.set_extra_targets(extra_target_instances);
            obj.x_value = wizard.x;
            obj.times_kicked = wizard.times_kicked();
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::KickedMask(_)));
            if face.additional_costs.len() > 1 && wizard.kicked_mask != 0 {
                obj.riders
                    .push(crate::object::Rider::KickedMask(wizard.kicked_mask));
            }
            obj.replicated = wizard.replicated;
            obj.alt_cast =
                matches!(
                    wizard.option,
                    Some(CastModeKind::Alternative(_) | CastModeKind::Emerge)
                ) || matches!(wizard.announced, Some(CastModeKind::LibraryTopLife { .. }))
                    || wizard.selected_alt_grant().is_some_and(|r| r.is_alt_cost);
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
                let aftermath_face = obj
                    .card
                    .and_then(|c| self.lookup.card(c.index))
                    .and_then(|d| d.faces.get(wizard.face_index()))
                    .is_some_and(|f| f.aftermath);
                if matches!(
                    wizard.option,
                    Some(
                        CastModeKind::Flashback | CastModeKind::JumpStart | CastModeKind::BeamMeUp
                    )
                ) || aftermath_face
                    || wizard.permission == Some(casting::GraveyardPermission::Flashback)
                    || wizard
                        .selected_alt_grant()
                        .is_some_and(|r| r.exile_as_flashback)
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
            obj.riders.retain(|r| *r != crate::object::Rider::Blitzed);
            if wizard.option == Some(CastModeKind::Blitz) {
                obj.riders.push(crate::object::Rider::Blitzed);
            }
            obj.riders.retain(|r| {
                !matches!(
                    r,
                    crate::object::Rider::WarpedSpell
                        | crate::object::Rider::WarpedInExile(_)
                        | crate::object::Rider::MayhemPaid
                        | crate::object::Rider::Sneaked
                        | crate::object::Rider::NinjutsuDefender(_, _)
                )
            });
            if wizard.option == Some(CastModeKind::Warp) {
                obj.riders.push(crate::object::Rider::WarpedSpell);
            }
            if wizard.option == Some(CastModeKind::Mayhem) {
                obj.riders.push(crate::object::Rider::MayhemPaid);
            }
            if wizard.option == Some(CastModeKind::Sneak) {
                obj.riders.push(crate::object::Rider::Sneaked);
                if let Some((defender, target_ver)) = sneak_defender {
                    obj.riders
                        .push(crate::object::Rider::NinjutsuDefender(defender, target_ver));
                }
            }
            obj.riders.retain(|r| *r != crate::object::Rider::Bargained);
            if wizard.bargained {
                obj.riders.push(crate::object::Rider::Bargained);
            }
            obj.riders
                .retain(|r| *r != crate::object::Rider::OptionalCostPaid);
            if wizard.optional_cost_paid {
                obj.riders.push(crate::object::Rider::OptionalCostPaid);
            }
            obj.riders.retain(|r| {
                !matches!(
                    r,
                    crate::object::Rider::SpellCostExile(_)
                        | crate::object::Rider::ControlledSubtypeOnCast(_)
                        | crate::object::Rider::BeheldCostPower(_, _, _)
                )
            });
            for ex in spell_cost_exiles {
                obj.riders.push(crate::object::Rider::SpellCostExile(ex));
            }
            for sub in controlled_subtypes_on_cast {
                obj.riders
                    .push(crate::object::Rider::ControlledSubtypeOnCast(sub));
            }
            if let Some((src, power, mv)) = beheld_cost_power {
                obj.riders
                    .push(crate::object::Rider::BeheldCostPower(src, power, mv));
            }
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::SquadPaid(_)));
            if wizard.squad > 0 {
                obj.riders
                    .push(crate::object::Rider::SquadPaid(wizard.squad));
            }
            obj.riders
                .retain(|r| *r != crate::object::Rider::OffspringPaid);
            if wizard.offspring {
                obj.riders.push(crate::object::Rider::OffspringPaid);
            }
            obj.riders
                .retain(|r| *r != crate::object::Rider::CasualtyPaid);
            if wizard.casualty {
                obj.riders.push(crate::object::Rider::CasualtyPaid);
            }
            obj.riders.retain(|r| {
                !matches!(
                    r,
                    crate::object::Rider::Awakened(_)
                        | crate::object::Rider::Spliced { .. }
                        | crate::object::Rider::ConspirePaid(_)
                        | crate::object::Rider::RepeatedModes(_)
                )
            });
            if wizard.option == Some(CastModeKind::Awaken)
                && let Some((count, _)) = obj
                    .card
                    .and_then(|c| self.lookup.card(c.index))
                    .and_then(|d| super::wave10b::face_awaken(d, wizard.face_index()))
            {
                obj.riders.push(crate::object::Rider::Awakened(count));
            }
            for &(_, s_card, s_face, _) in &wizard.spliced {
                obj.riders.push(crate::object::Rider::Spliced {
                    card: s_card,
                    face: s_face,
                });
            }
            if wizard.conspired > 0 {
                obj.riders
                    .push(crate::object::Rider::ConspirePaid(wizard.conspired));
            }
            obj.riders.retain(|r| *r != crate::object::Rider::Fused);
            if wizard.option == Some(CastModeKind::Fused) {
                obj.riders.push(crate::object::Rider::Fused);
            }
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::SuspendHaste(_)));
            obj.riders.retain(|r| *r != crate::object::Rider::Mutating);
            if wizard.option == Some(CastModeKind::Mutate) {
                obj.riders.push(crate::object::Rider::Mutating);
            }
            obj.riders.retain(|r| *r != crate::object::Rider::Bestowed);
            if wizard.option == Some(CastModeKind::Bestow) {
                obj.riders.push(crate::object::Rider::Bestowed);
            }
            obj.riders
                .retain(|r| *r != crate::object::Rider::ImpendingPaid);
            if wizard.option == Some(CastModeKind::Impending) {
                obj.riders.push(crate::object::Rider::ImpendingPaid);
            }
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::GiftPromisedTo(_)));
            if let Some(recipient) = wizard.gift_recipient {
                obj.riders
                    .push(crate::object::Rider::GiftPromisedTo(recipient));
            }
            if wizard.buyback.is_some() {
                obj.riders.push(crate::object::Rider::Buyback);
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
            if let Some(CastModeKind::RepeatedModes(packed)) = wizard.option {
                obj.riders.push(crate::object::Rider::RepeatedModes(packed));
            }
        }
        if !wizard.spliced.is_empty() {
            let cards: Vec<ObjectId> = wizard.spliced.iter().map(|&(id, ..)| id).collect();
            self.state
                .journal
                .record(GameEvent::Revealed { player, cards });
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
        // MDFC back-face cast: the object becomes its chosen face (CR 712.8b).
        // A split card is both halves in hand and only the half cast on the
        // stack (CR 709.3b), its left half too — unless cast fused (CR 702.102b,
        // CR 709.4d), where it keeps both halves' combined characteristics.
        let i = wizard.face_index();
        let def = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index));
        if wizard.option != Some(CastModeKind::Fused)
            && (i != 0 || def.is_some_and(crate::object::reads_both_halves))
        {
            let def = def.expect("wizard card known");
            self.state.switch_face(card, def, i);
        }
        let cast_origin_zone = self.state.object(card).map(|o| o.zone);
        self.state
            .move_object(card, ZoneLocation::Stack, ZonePosition::Top, Cause::Spell)?;
        // The cards delve exiled to pay this spell's cost are exiled with it:
        // a permanent's "cards exiled with [this object]" refers to exactly
        // those cards of the spell that became it (CR 607.2q; Murktide
        // Regent). Keep the exact exiles on the spell so their links can
        // follow it when it resolves, including replacement effects on entry.
        if !delved.is_empty()
            && let Some(version) = self
                .state
                .object(card)
                .map(crate::object::Rider::version_of)
        {
            for &exiled in &delved {
                if let Some(object) = self
                    .state
                    .object_mut(exiled)
                    .filter(|o| o.zone == crate::zone::Zone::Exile)
                {
                    object.riders.push(crate::object::Rider::ExiledWith {
                        host: card,
                        version,
                    });
                    let exile = baylee_core::ids::DamageSourceRef {
                        object: exiled,
                        version: object.version,
                    };
                    self.state
                        .object_mut(card)
                        .expect("the spell on the stack")
                        .riders
                        .push(crate::object::Rider::DelvedCard(exile));
                }
            }
        }
        if let Some(turn) = foretold_turn {
            self.state
                .object_mut(card)
                .expect("cast spell")
                .riders
                .push(crate::object::Rider::Foretold { turn });
        }
        if wizard.exile_instead_of_graveyard
            || wizard
                .selected_alt_grant()
                .is_some_and(|r| r.exile_instead_of_graveyard)
        {
            self.state
                .object_mut(card)
                .expect("cast spell")
                .riders
                .push(crate::object::Rider::ExileInsteadOfGraveyard);
        }
        if let Some(route) = wizard.selected_alt_grant() {
            let obj = self.state.object_mut(card).expect("cast spell");
            if route.entry_finality && !obj.riders.contains(&crate::object::Rider::EntryFinality) {
                obj.riders.push(crate::object::Rider::EntryFinality);
            }
            if route.evoke_sacrifice {
                obj.riders.push(crate::object::Rider::EvokedGrant);
            }
            if let Some(life) = route.grant_death_exile_gain_life {
                obj.riders
                    .push(crate::object::Rider::GrantDeathExileAndGainLife(life));
            }
            if route.grant_leave_exile {
                obj.riders
                    .push(crate::object::Rider::GrantLeaveExileReplacement);
            }
        }
        self.state
            .object_mut(card)
            .expect("cast spell")
            .cast_history
            .set_caster(Some(player));
        if self.state.per_turn.track_cast_origin_zone.0
            && let Some(origin) = cast_origin_zone
            && matches!(
                origin,
                crate::zone::Zone::Exile | crate::zone::Zone::Library | crate::zone::Zone::Command
            )
            && let Some(obj) = self.state.object_mut(card)
        {
            obj.riders
                .push(crate::object::Rider::CastOriginZone(origin));
        }
        // What was paid, written after the move for the reason the move
        // gives it up: the spell on the stack is the object that reads it.
        // A free cast spent no mana (CR 601.2h pays nothing it was not
        // asked for), whatever its printed cost says.
        let mana_spent_on_spell = paid_mana.mana_spent;
        if (paid_mana.mana_spent > 0 || sacrificed_lki.is_some())
            && let Some(obj) = self.state.object_mut(card)
        {
            paid_mana.sacrificed_lki = sacrificed_lki;
            paid_mana.sacrificed = sacrificed_object;
            obj.paid = Some(Box::new(paid_mana));
        }
        if convoked_count > 0
            && let Some(obj) = self.state.object_mut(card)
        {
            obj.riders
                .push(crate::object::Rider::ConvokedCount(convoked_count));
        }
        if matches!(wizard.option, Some(CastModeKind::Prototype))
            && let Some(prototype) = face.prototype
            && let Some(obj) = self.state.object_mut(card)
        {
            if obj.original_base.is_none() {
                obj.original_base = Some(std::sync::Arc::clone(&obj.base));
            }
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
        if matches!(wizard.option, Some(CastModeKind::Bestow)) {
            self.state.invalidate_projections();
        }
        if wizard.from_suspend
            && self.state.object(card).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            })
        {
            if let Some(obj) = self.state.object_mut(card) {
                obj.riders.push(crate::object::Rider::SuspendHaste(player));
            }
            let ts = self.state.next_timestamp();
            let filter = crate::effects::EffectFilter::object(&self.state, card);
            self.state
                .effects
                .register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(card),
                    controller: player,
                    layer: baylee_cards_dsl::Layer::Ability,
                    filter,
                    modifier: baylee_cards_dsl::Modifier::AddKeyword(
                        baylee_cards_dsl::KeywordSet::HASTE,
                    ),
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::WhileSourceInZone(
                        baylee_cards_dsl::ZoneRef::Stack,
                    ),
                    origin: crate::effects::EffectOrigin::Resolution,
                });
            self.state.invalidate_projections();
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
            && (matches!(wizard.option, Some(CastModeKind::Bestow))
                || !face.types.contains(baylee_core::types::TypeSet::CREATURE))
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
        if self.state.per_turn.cast_spells_this_turn.tracked
            && let Some(obj) = self.state.object(card)
        {
            let mut snap = obj.clone();
            snap.base = std::sync::Arc::new(obj.characteristics().clone());
            snap.cache.clear();
            let version = obj.version;
            self.state
                .per_turn
                .cast_spells_this_turn
                .items
                .push(crate::state::CastSpellRecord {
                    source: baylee_core::ids::DamageSourceRef {
                        object: card,
                        version,
                    },
                    controller: player,
                    snapshot: std::sync::Arc::new(snap),
                });
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
        // A next-spell reduction is spent by the successful cast, even when
        // its generic component was already zero. Cancelling never reaches here.
        if let Some(reduction) = self
            .state
            .per_turn
            .next_spell_cost_reduction
            .get_mut(usize::from(player.get()))
        {
            *reduction = 0;
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
        if let Some(bindings) = wizard.target_group_bindings.as_deref() {
            if !bindings.first.is_empty() {
                self.state
                    .bind_one_per_player_groups(card, 0, &bindings.first);
            }
            if !bindings.second.is_empty() {
                self.state
                    .bind_one_per_player_groups(card, 1, &bindings.second);
            }
            for (extra_idx, groups) in &bindings.extra {
                if !groups.is_empty() {
                    self.state.bind_one_per_player_groups(
                        card,
                        u8::try_from(2 + extra_idx).expect("extra target group"),
                        groups,
                    );
                }
            }
        }
        self.store_damage_division(wizard);
        // CR 723.3c: announcing or cancelling never spends the designation.
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
        if wizard.option == Some(CastModeKind::Warp) {
            self.state.per_turn.spells_warped.0.push(player);
        }
        if self.state.per_turn.spells_cast_log.tracked {
            self.state.refresh_characteristics();
        }
        if let Some(object) = self.state.object(card) {
            let record = crate::state::SpellCastRecord {
                spell: baylee_core::ids::DamageSourceRef {
                    object: card,
                    version: object.version,
                },
                caster: player,
                snapshot: crate::state::freeze_object(object),
            };
            self.state.per_turn.spells_cast_log.items.push(record);
        }
        self.state.journal.record(GameEvent::SpellCast {
            instant_number,
            object: card,
            player,
            number: self.state.per_turn.spells_cast_by(player),
        });
        if wizard.option == Some(CastModeKind::Warp) {
            self.state.journal.record(GameEvent::SpellWarped {
                object: card,
                player,
            });
        }
        self.note_crime_if_any(player, card);
        self.note_expend_if_any(player, mana_spent_on_spell);
        self.cast_wizard = None;
        if !self.finish_nested_cast() {
            self.after_action(player);
        }
        Ok(())
    }
}

fn collect_cast_subtypes(
    cond: &baylee_cards_dsl::Condition,
    out: &mut smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]>,
) {
    use baylee_cards_dsl::Condition;
    match cond {
        Condition::ControlledSubtypeOnCast(sub) => {
            if !out.contains(sub) {
                out.push(*sub);
            }
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => collect_cast_subtypes(inner, out),
        Condition::All(list) | Condition::Any(list) => {
            for c in *list {
                collect_cast_subtypes(c, out);
            }
        }
        Condition::AmountCompare { left, right, .. } => {
            collect_amount_cast_subtypes(left, out);
            collect_amount_cast_subtypes(right, out);
        }
        Condition::AmountAtLeast(amt, _) | Condition::AmountIsEven(amt) => {
            collect_amount_cast_subtypes(amt, out);
        }
        _ => {}
    }
}

fn collect_amount_cast_subtypes(
    amt: &baylee_cards_dsl::Amount,
    out: &mut smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]>,
) {
    use baylee_cards_dsl::Amount;
    match amt {
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            collect_cast_subtypes(condition, out);
            collect_amount_cast_subtypes(then, out);
            collect_amount_cast_subtypes(otherwise, out);
        }
        Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. }
        | Amount::Plus { base: a, .. }
        | Amount::Negated(a)
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. } => collect_amount_cast_subtypes(a, out),
        Amount::Sum { left, right } | Amount::Difference { left, right } => {
            collect_amount_cast_subtypes(left, out);
            collect_amount_cast_subtypes(right, out);
        }
        _ => {}
    }
}

fn collect_effects_cast_subtypes(
    effects: &'static [baylee_cards_dsl::Effect],
    out: &mut smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]>,
) {
    use baylee_cards_dsl::Effect;
    Effect::walk(effects, &mut 0, &mut |eff| match eff {
        Effect::IfCondition { condition, .. } => collect_cast_subtypes(condition, out),
        Effect::DealDamage { amount, .. }
        | Effect::DealDamageEach { amount, .. }
        | Effect::GainLife { amount }
        | Effect::LoseLife { amount, .. }
        | Effect::DrawCards { amount }
        | Effect::AddCounter { amount, .. }
        | Effect::AddCounterSource { amount, .. } => collect_amount_cast_subtypes(amount, out),
        Effect::PumpTarget {
            power, toughness, ..
        }
        | Effect::PumpFilter {
            power, toughness, ..
        } => {
            collect_amount_cast_subtypes(power, out);
            collect_amount_cast_subtypes(toughness, out);
        }
        _ => {}
    });
}

fn collect_ability_cast_subtypes(
    ab: &baylee_cards_dsl::AbilityDef,
    out: &mut smallvec::SmallVec<[baylee_core::ids::SubtypeId; 2]>,
) {
    use baylee_cards_dsl::AbilityDef;
    match ab {
        AbilityDef::Spell {
            effects, condition, ..
        }
        | AbilityDef::Triggered {
            effects, condition, ..
        } => {
            if let Some(c) = condition {
                collect_cast_subtypes(c, out);
            }
            collect_effects_cast_subtypes(effects, out);
        }
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => {
            for m in *modes {
                collect_effects_cast_subtypes(m.effects, out);
            }
        }
        AbilityDef::Static(sa) => {
            if let Some(c) = &sa.condition {
                collect_cast_subtypes(c, out);
            }
        }
        _ => {}
    }
}
