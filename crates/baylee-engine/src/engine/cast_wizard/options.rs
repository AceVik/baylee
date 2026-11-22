//! Available ways to cast a spell.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// All legal ways to cast `card` right now.
    pub(in crate::engine) fn cast_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        pay_in_window: bool,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        self.cast_options_with_routes(player, card, pay_in_window)
            .map(|(options, _)| options)
    }

    /// All legal ways to cast `card` right now, paired with any continuous
    /// `MayCastWithAltCost` routes indexed by position in the returned slice.
    #[allow(clippy::too_many_lines, clippy::type_complexity)] // one branch per printed way to cast; splitting hides the list
    pub(in crate::engine) fn cast_options_with_routes(
        &self,
        player: PlayerId,
        card: ObjectId,
        pay_in_window: bool,
    ) -> Result<(Vec<CastModeDesc>, Vec<(usize, casting::AltCastGrantRoute)>), EngineError> {
        let obj = self
            .state
            .object(card)
            .ok_or(EngineError::IllegalAction("no such card"))?;
        let obj = casting::playable_exile_projection(&self.state, player, obj);
        let card_ref = obj
            .card
            .ok_or(EngineError::IllegalAction("not card-backed"))?;
        let def = self
            .lookup
            .card(card_ref.index)
            .ok_or(EngineError::IllegalAction("unknown card"))?;
        let face = &def.faces[0];
        if obj.zone == crate::zone::Zone::Library {
            let paying = if pay_in_window {
                casting::Paying::DuringTheCast
            } else {
                casting::Paying::FromThePool
            };
            if casting::library_top_life_open(&self.state, player, card) {
                return casting::library_top_life_options(
                    &self.state,
                    &self.lookup,
                    player,
                    card,
                    paying,
                )
                .map(|opts| (opts, Vec::new()))
                .map_err(|error| EngineError::Cast(crate::casting::CastFailure::Legality(error)));
            }
            let (alt_pairs, diag) =
                casting::alt_cast_grant_options(&self.state, &self.lookup, player, card, paying);
            if alt_pairs.is_empty() {
                let err = diag.unwrap_or(casting::CastError::NotInHand);
                return Err(EngineError::Cast(crate::casting::CastFailure::Legality(
                    err,
                )));
            }
            let mut opts = Vec::with_capacity(alt_pairs.len());
            let mut routes = Vec::with_capacity(alt_pairs.len());
            for (i, (mut desc, route)) in alt_pairs.into_iter().enumerate() {
                desc.index = u32::try_from(i).expect("bounded cast options");
                opts.push(desc);
                routes.push((i, route));
            }
            return Ok((opts, routes));
        }
        let commanded = self.commanded_card(card);
        // Each way of casting is asked as the spell it makes: what a lock on
        // the front stops, a back face it does not reach still casts.
        let forbidden =
            |face: usize| casting::face_is_forbidden(&self.state, player, &obj, def, face);
        if commanded
            && (!casting::may_begin_casting(&self.state, player)
                || (0..def.faces.len()).all(forbidden))
        {
            return Err(EngineError::IllegalAction("casting prohibited"));
        }
        let defer_mana = pay_in_window || (commanded && self.free_cast_makes_mana(player));
        let paying = if pay_in_window {
            casting::Paying::DuringTheCast
        } else {
            casting::Paying::FromThePool
        };
        // Restricted mana this spell may be paid with counts here for the
        // same reason the convoke count below does: this probe and
        // `casting::can_cast`'s have to be the same probe, or the Cavern's
        // Ally is offered in `LegalActions` and refused the moment it is
        // taken.
        let with_restricted =
            casting::planning_pool(&self.state, player, casting::SpendFor::Spell(card), 0);
        let pool = with_restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        // Commander tax (CR 903.8): {2} more generic for each previous cast
        // of this commander from the command zone. It is a cost increase, so
        // it lands on every way of casting the card, an alternative cost
        // included (CR 601.2f) — and on the affordability probes too, or a
        // mode would be offered that the player then cannot pay for.
        let adjustment = casting::face_adjustment(&self.state, player, card, def, 0, None);
        let preview_delta =
            casting::preview_cast_metric_bounds(&self.state, player, card, def, 0, &adjustment);
        let preview_adjustment = adjustment.for_preview(preview_delta);
        let printed_reduction = casting::printed_reduction(&self.state, face, player, card);
        let price = |cost: ManaCost| adjustment.price(cost, printed_reduction);
        let preview_price = |cost: ManaCost| preview_adjustment.price(cost, printed_reduction);
        let back_price = |i: usize| {
            casting::face_adjustment(&self.state, player, card, def, i, None).price(
                def.faces[i].mana_cost,
                casting::printed_reduction(&self.state, &def.faces[i], player, card),
            )
        };
        let back_preview_price = |i: usize| {
            let adj = casting::face_adjustment(&self.state, player, card, def, i, None);
            let p = casting::preview_cast_metric_bounds(&self.state, player, card, def, i, &adj);
            adj.for_preview(p).price(
                def.faces[i].mana_cost,
                casting::printed_reduction(&self.state, &def.faces[i], player, card),
            )
        };
        // Convoke (CR 702.51a) pays {1} per untapped creature and
        // delve (CR 702.66a) pays {1} per card exiled from the graveyard, so
        // both are part of what "afford" means. It has to be the same count
        // `casting::can_cast` uses, or the spell is offered in
        // `LegalActions` and then refused here as "no way to cast this
        // spell" — which is what happened, and it is why the count lives in
        // one function rather than in each of them. A creature also pays a
        // coloured symbol of its colour (CR 702.51a), which `less` applies
        // the same way `can_cast` does (`casting::keyword_priced`).
        let reduction = casting::keyword_reduction(&self.state, face, player, card);
        let less =
            |cost: ManaCost| casting::keyword_priced(&self.state, face, player, &cost, reduction);
        let allowance = preview_adjustment.allowance(
            &self.state,
            casting::SpendFor::Spell(card),
            printed_reduction.saturating_add(reduction),
        );
        let back_allowance = |i| {
            let adj = casting::face_adjustment(&self.state, player, card, def, i, None);
            let p = casting::preview_cast_metric_bounds(&self.state, player, card, def, i, &adj);
            adj.for_preview(p).allowance(
                &self.state,
                casting::SpendFor::SpellOnFace(card, i),
                casting::printed_reduction(&self.state, &def.faces[i], player, card),
            )
        };
        // Mycosynth Lattice: every probe below asks whether the pool covers a
        // cost, and under the Lattice any mana answers any pip.
        let afford = |cost: &baylee_core::mana::ManaCost| {
            defer_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::Spell(card),
                    pool,
                    &less(preview_price(*cost)),
                    &[],
                    Some(allowance),
                )
        };
        let afford_normal = |cost: &ManaCost| {
            afford(cost)
                || casting::north_star_face_allowed(&self.state, player, card, 0)
                    && casting::normal_spell_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::Spell(card),
                        pool,
                        &less(preview_price(*cost)),
                        &face.mana_cost,
                        allowance,
                    )
        };
        let mut options =
            casting::granted_escape_options(&self.state, &self.lookup, player, card, paying);
        // Disturb and Aftermath casts come from the owner's graveyard.
        let disturb_cast = self.state.object(card).is_some_and(|o| {
            o.zone == crate::zone::Zone::Graveyard && o.zone_owner == Some(player)
        }) && def.faces.iter().any(|f| f.disturb || f.aftermath);
        if disturb_cast {
            for (i, back) in def.faces.iter().enumerate().skip(1) {
                if (back.disturb || back.aftermath)
                    && !forbidden(i)
                    && casting::face_timing_allows(&self.state, player, card, def, i)
                    && (defer_mana
                        || casting::source_affordable(
                            &self.state,
                            player,
                            casting::SpendFor::SpellOnFace(card, i),
                            pool,
                            &back_preview_price(i).with_x(0),
                            &[],
                            Some(back_allowance(i)),
                        ))
                    && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, i)
                {
                    options.push(CastModeDesc {
                        index: (options.len()) as u32,
                        kind: CastModeKind::Face(i),
                        cost: back_price(i),
                    });
                }
            }
            // A graveyard permission or grant for the front (Muldrotha, Past in
            // Flames, printed flashback/jump-start/escape) is offered below too.
            let front_too = self.state.object(card).is_some_and(|o| {
                casting::graveyard_cast_permission(&self.state, player, o).is_some()
                    || casting::flashback_granted(&self.state, card)
                    || !casting::graveyard_face_permissions(&self.state, player, card, 0).is_empty()
                    || face.flashback.is_some()
                    || face.jump_start
                    || o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::RETRACE)
                    || face.escape.is_some()
            });
            if !front_too && !casting::has_alt_cast_grant(&self.state, &self.lookup, player, card) {
                return Ok((options, Vec::new()));
            }
        }
        // Printed flashback (CR 702.34a) from the graveyard: its own cost,
        // "rather than paying its mana cost" — and beside it, where a grant
        // also reaches the card (Past in Flames) or a permission casts any
        // spell from there (Forgotten Cellar), the mana cost as `Normal`.
        // Nothing else is cast from a graveyard: one alternative cost at a
        // time (CR 118.9a).
        let in_graveyard = self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard);
        // Alternative graveyard costs coexist with ordinary permissions.
        if in_graveyard {
            if let Some(escape) = face.escape {
                let fodder = casting::escape_exile_options(&self.state, player, card).len();
                if afford(&escape.cost.with_x(0))
                    && (fodder >= usize::from(escape.exile) || defer_mana)
                {
                    options.push(CastModeDesc {
                        index: 0,
                        kind: CastModeKind::Escape,
                        cost: price(escape.cost),
                    });
                }
            }
            if let Some(flashback) = face.flashback
                && afford(&flashback.with_x(0))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    Some(CastModeKind::Flashback),
                    &[],
                )
                .is_some()
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Flashback,
                    cost: price(flashback),
                });
            }
            if let Some(beam) = face.beam_me_up
                && afford(&beam.with_x(0))
                && !casting::beam_return_options(&self.state, player, card).is_empty()
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::BeamMeUp,
                    cost: price(beam),
                });
            }
            if face.jump_start
                && face.types.intersects(
                    baylee_core::types::TypeSet::INSTANT
                        .union(baylee_core::types::TypeSet::SORCERY),
                )
                && afford(&face.mana_cost.with_x(0))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    Some(CastModeKind::JumpStart),
                    &[],
                )
                .is_some()
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::JumpStart,
                    cost: price(face.mana_cost),
                });
            }
            if self.state.object(card).is_some_and(|o| {
                o.zone_owner == Some(player)
                    && o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::RETRACE)
            }) && !face.types.contains(baylee_core::types::TypeSet::LAND)
                && afford(&face.mana_cost.with_x(0))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    Some(CastModeKind::Retrace),
                    &[],
                )
                .is_some()
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Retrace,
                    cost: price(face.mana_cost),
                });
            }
        }
        // Conditional cost reduction printed on the card (Surgical
        // Metamorph & co.), through the same reader `can_cast` uses — this
        // was the second place the two probes disagreed, and in the other
        // direction from convoke: the wizard knew the discount and the offer
        // did not, so the seat entitled to it was never shown the card.
        let normal_cost = face.mana_cost;
        let min_normal_cost = casting::face_base_mana_cost(&self.state, player, card, face);
        // A modal spell (CR 700.2) has no "no mode" way to be cast: every one
        // of its effects sits under a mode, so a `Normal` option resolves to
        // nothing at all. `progress` looks for an `AbilityDef::Spell` first
        // and falls back to the object's `mode_index`, and a card whose only
        // spell ability is `ModalSpell` cast as `Normal` has neither — it went
        // hand → stack → graveyard and did nothing, which is how Damn was
        // found. The four cards this reaches (damn, cyclonic_rift,
        // heliod_s_intervention, sheoldred_s_edict) stay castable: a mode's
        // cost defaults to the face's, so `Mode(0)` carries exactly the cost
        // `Normal` was offering. `casting::face_has_a_legal_target` already
        // asks this same question of `abilities_for_face`, in the same
        // direction and for the same reason. The two clauses beside it hold
        // the guard to exactly "casting with no mode chosen does nothing": a
        // permanent spell arrives on the battlefield whether a mode was
        // picked or not, and a plain `Spell` printed beside the modes is what
        // `progress` finds first.
        let abilities = def.abilities_for_face(0);
        let modal_only = abilities.iter().any(|a| a.modal_spell().is_some())
            && !abilities
                .iter()
                .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Spell { .. }))
            && !face.types.is_permanent();
        let foretell_cost = face
            .foretell
            .filter(|_| casting::foretell_permission(&self.state, player, card));
        if let Some(cost) = foretell_cost
            && casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Foretell(cost)),
                paying,
            )
            .is_ok()
            && afford(&cost.with_x(0))
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
        {
            options.push(CastModeDesc {
                index: options.len() as u32,
                kind: CastModeKind::Foretell,
                cost: price(cost),
            });
        }
        let plot_ok = casting::plot_permission(&self.state, player, card);
        let front_timing_ok = commanded
            || casting::timing_allows(
                &self.state,
                player,
                obj.characteristics().types,
                obj.characteristics().keywords,
            );
        if !modal_only
            && plot_ok
            && casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Plot),
                paying,
            )
            .is_ok()
            && afford(&ManaCost::ZERO)
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
        {
            options.push(CastModeDesc {
                index: options.len() as u32,
                kind: CastModeKind::Plot,
                cost: price(ManaCost::ZERO),
            });
        }
        // Normal cost (X probed with 0; the real check happens at payment).
        // Guarded by the same CR 202.1b question `can_cast` asks, and for the
        // reason every probe in this function is paired with one there: an
        // option offered here that the offer does not know about is a mode a
        // player can pick and be refused for. Foretell alone grants only its
        // alternative cost; a separate play permission can also grant Normal.
        let front_additional_ok =
            casting::additional_payment_capacity_for(&self.state, player, card, face, None, &[])
                .is_some();
        if front_timing_ok
            && front_additional_ok
            && !modal_only
            && ((foretell_cost.is_none() && !plot_ok)
                || casting::play_permission(&self.state, player, card).is_some())
            && !face.types.contains(baylee_core::types::TypeSet::LAND)
            && (commanded
                || casting::can_cast_form_paying(
                    &self.state,
                    &self.lookup,
                    player,
                    card,
                    None,
                    paying,
                )
                .is_ok())
            && casting::has_a_printed_cost(&face.mana_cost)
            && afford_normal(&min_normal_cost.with_x(0))
            && (face.kicked_targets.is_none()
                || casting::ordinary_targets_reachable(&self.state, def, player, card))
        {
            // Keep the ordinary cast first beside alternative graveyard costs.
            // The final pass assigns indices after every option is assembled.
            options.insert(
                0,
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: price(normal_cost),
                },
            );
        }
        if front_timing_ok
            && !face.additional_costs.is_empty()
            && (face.kicked_targets.is_some() || !afford_normal(&face.mana_cost.with_x(0)))
            && casting::additional_payment_capacity_for(
                &self.state,
                player,
                card,
                face,
                Some(CastModeKind::Kicked),
                &[],
            )
            .is_some()
        {
            let cost = casting::kicked_mana_cost(face);
            let restricted = casting::planning_pool(
                &self.state,
                player,
                casting::SpendFor::KickedSpell(card, 0, None),
                0,
            );
            let kicked_pool = restricted
                .as_ref()
                .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
            if (defer_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::KickedSpell(card, 0, None),
                    kicked_pool,
                    &less(preview_price(cost).with_x(0)),
                    &[],
                    Some(allowance),
                )
                || casting::north_star_face_allowed(&self.state, player, card, 0)
                    && casting::normal_spell_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::KickedSpell(card, 0, None),
                        kicked_pool,
                        &less(preview_price(cost).with_x(0)),
                        &face.mana_cost,
                        allowance,
                    ))
                && face.kicked_targets.map_or_else(
                    || casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0),
                    |req| casting::kicked_targets_reachable(&self.state, def, player, card, req),
                )
            {
                options.push(CastModeDesc {
                    index: options.len() as u32,
                    kind: CastModeKind::Kicked,
                    cost: price(cost),
                });
            }
        }
        if let Some(prototype) = face.prototype
            && casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Prototype(prototype)),
                paying,
            )
            .is_ok()
        {
            options.push(CastModeDesc {
                index: options.len() as u32,
                kind: CastModeKind::Prototype,
                cost: casting::face_adjustment(
                    &self.state,
                    player,
                    card,
                    def,
                    0,
                    Some(casting::SpellForm::Prototype(prototype)),
                )
                .price(prototype.cost, printed_reduction),
            });
        }
        if face.has_face_down_cast() {
            let cost = const { ManaCost::parse("{3}") };
            if casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Disguise),
                paying,
            )
            .is_ok()
            {
                options.push(CastModeDesc {
                    index: options.len() as u32,
                    kind: CastModeKind::Disguise,
                    cost: casting::face_adjustment(
                        &self.state,
                        player,
                        card,
                        def,
                        0,
                        Some(casting::SpellForm::Disguise),
                    )
                    .price(cost, 0),
                });
            }
        }
        if let Some(bestow) = face.bestow
            && casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Bestow(bestow)),
                paying,
            )
            .is_ok()
        {
            options.push(CastModeDesc {
                index: options.len() as u32,
                kind: CastModeKind::Bestow,
                cost: casting::face_adjustment(
                    &self.state,
                    player,
                    card,
                    def,
                    0,
                    Some(casting::SpellForm::Bestow(bestow)),
                )
                .price(bestow.mana, printed_reduction),
            });
        }
        if let Some((_, cost)) = face.impending
            && casting::can_cast_form_paying(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Impending(cost)),
                paying,
            )
            .is_ok()
        {
            options.push(CastModeDesc {
                index: options.len() as u32,
                kind: CastModeKind::Impending,
                cost: casting::face_adjustment(
                    &self.state,
                    player,
                    card,
                    def,
                    0,
                    Some(casting::SpellForm::Impending(cost)),
                )
                .price(cost, printed_reduction),
            });
        }
        // Alternative costs (pitch, evoke, conditional free).
        if front_timing_ok {
            for (i, alt) in face.alternative_costs.iter().enumerate() {
                let condition_ok = match alt.condition {
                    AltCondition::Always => true,
                    AltCondition::NotYourTurn => self.state.turn.active != player,
                    AltCondition::CommanderControlled => self.has_commander_on_battlefield(player),
                    AltCondition::If(condition) => {
                        crate::eval::condition_holds(&self.state, player, card, *condition)
                    }
                };
                let taxed = baylee_cards_dsl::Cost {
                    mana: if defer_mana {
                        ManaCost::ZERO
                    } else {
                        price(alt.cost.mana)
                    },
                    ..alt.cost
                };
                // The spell's own payment, restricted mana included: `can_cast`
                // probes an alternative cost against the same pool, and a probe
                // that read the plain pool here offered an evoke off a Cavern in
                // `LegalActions` and then refused it as "no way to cast this
                // spell".
                if !condition_ok
                    || !self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card))
                    || casting::additional_payment_capacity_for(
                        &self.state,
                        player,
                        card,
                        face,
                        Some(CastModeKind::Alternative(i)),
                        &[],
                    )
                    .is_none()
                {
                    continue;
                }
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Alternative(i),
                    cost: price(alt.cost.mana),
                });
            }
        }
        // Dash (CR 702.109a): its cost "rather than its mana cost", which is
        // an alternative cost (CR 118.9) paid by the rules for one (601.2b,
        // 601.2f–h) — asked of the pool the way the alternatives above are.
        if front_timing_ok && let Some(dash) = face.dash {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(dash)
                },
                parts: &[],
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card)) {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Dash,
                    cost: price(dash),
                });
            }
        }
        // Blitz (CR 702.152a): its cost (mana + non-mana parts) "rather than its mana cost".
        if front_timing_ok && let Some(blitz) = face.blitz {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(blitz.mana)
                },
                ..blitz
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    Some(CastModeKind::Blitz),
                    &[],
                )
                .is_some()
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Blitz,
                    cost: price(blitz.mana),
                });
            }
        }
        // Warp (CR 702.185a): cast from your hand for its warp cost.
        if front_timing_ok
            && obj.zone == crate::zone::Zone::Hand
            && obj.zone_owner == Some(player)
            && let Some(warp) = face.warp
        {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(warp.mana)
                },
                ..warp
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    Some(CastModeKind::Warp),
                    &[],
                )
                .is_some()
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Warp,
                    cost: price(warp.mana),
                });
            }
        }
        // Sneak (CR 702.190a): cast during your declare blockers step at instant
        // speed by paying its sneak cost and returning an unblocked creature you
        // control to its owner's hand.
        if (!plot_ok || casting::play_permission(&self.state, player, card).is_some())
            && casting::in_sneak_window(&self.state, player)
            && let Some(sneak) = casting::sneak_cost_for(&self.state, player, card, Some(def), 0)
            && !casting::sneak_candidates(&self.state, player, card, face, sneak).is_empty()
        {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(sneak.mana)
                },
                ..sneak
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card)) {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Sneak,
                    cost: price(sneak.mana),
                });
            }
        }
        // Awaken N—[cost] (CR 702.113a): alternative cost adding target land you control.
        if let Some((_, awaken)) = super::wave10b::face_awaken(def, 0)
            && super::wave10b::awaken_has_legal_land(&self.state, player, card)
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
        {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(awaken.mana)
                },
                ..awaken
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card))
                && casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    face,
                    super::CostChoice {
                        option: Some(CastModeKind::Awaken),
                        awaken_parts: awaken.parts,
                        ..Default::default()
                    },
                    &[],
                )
                .is_some()
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Awaken,
                    cost: price(awaken.mana),
                });
            }
        }
        // Emerge (CR 702.119a–c): its emerge cost reduced by the sacrificed
        // permanent's mana value, "rather than paying this spell's mana cost".
        if front_timing_ok && let Some(emerge) = face.emerge {
            let candidates = casting::emerge_candidates(&self.state, player, card, face, emerge);
            let best_payable_mv = candidates
                .into_iter()
                .filter_map(|(_, mv)| {
                    let cost_for_mv =
                        adjustment.price(emerge.cost, printed_reduction.saturating_add(mv));
                    let allow = adjustment.allowance(
                        &self.state,
                        casting::SpendFor::Spell(card),
                        printed_reduction
                            .saturating_add(reduction)
                            .saturating_add(mv),
                    );
                    (defer_mana
                        || casting::source_affordable(
                            &self.state,
                            player,
                            casting::SpendFor::Spell(card),
                            pool,
                            &less(cost_for_mv.with_x(0)),
                            &[],
                            Some(allow),
                        ))
                    .then_some(mv)
                })
                .max();
            if let Some(best_mv) = best_payable_mv {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Emerge,
                    cost: adjustment.price(emerge.cost, printed_reduction.saturating_add(best_mv)),
                });
            }
        }
        if front_timing_ok
            && let Some(mutate) = face.mutate
            && casting::kicked_targets_reachable(
                &self.state,
                def,
                player,
                card,
                casting::MUTATE_TARGET_REQ,
            )
        {
            let taxed = baylee_cards_dsl::Cost {
                mana: if defer_mana {
                    ManaCost::ZERO
                } else {
                    price(mutate)
                },
                parts: &[],
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card)) {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Mutate,
                    cost: price(mutate),
                });
            }
        }
        // MDFC backs (CR 712.8b) and adventures (CR 716), through the reader
        // `can_cast` uses — land faces are played and disturb backs came out
        // above. This loop asked the faces and nothing else, so the adventure
        // was offered again out of the exile its own resolution had put the
        // card in, and at instant speed forever: the offer read the exiled
        // object's *current* face, which was still the instant, so Swift
        // Spiral was `{1}{W}` in `LegalActions` every turn and the wizard
        // priced and cast it. CR 716.3d is the rule it broke.
        let on_adventure = casting::adventure_only_permission(&self.state, player, card);
        for (i, _) in casting::castable_back_faces(def, on_adventure) {
            if forbidden(i)
                || (!commanded && !casting::face_timing_allows(&self.state, player, card, def, i))
                || casting::additional_payment_capacity_for(
                    &self.state,
                    player,
                    card,
                    &def.faces[i],
                    Some(CastModeKind::Face(i)),
                    &[],
                )
                .is_none()
            {
                continue;
            }
            let restricted = casting::planning_pool(
                &self.state,
                player,
                casting::SpendFor::SpellOnFace(card, i),
                0,
            );
            let pool = restricted
                .as_ref()
                .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
            if (defer_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::SpellOnFace(card, i),
                    pool,
                    &back_preview_price(i).with_x(0),
                    &[],
                    Some(back_allowance(i)),
                )
                || casting::north_star_face_allowed(&self.state, player, card, i)
                    && casting::normal_spell_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::SpellOnFace(card, i),
                        pool,
                        &back_preview_price(i).with_x(0),
                        &def.faces[i].mana_cost,
                        back_allowance(i),
                    ))
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, i)
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Face(i),
                    cost: back_price(i),
                });
            }
        }
        // Fuse (CR 702.102a–c): from hand only, cast both halves of a split
        // card with combined characteristics and combined mana cost.
        if !commanded
            && self
                .state
                .object(card)
                .is_some_and(|o| o.zone == crate::zone::Zone::Hand)
            && def.fuse
            && crate::object::reads_both_halves(def)
            && def.faces.len() >= 2
        {
            let raw_obj = self.state.object(card).expect("card in hand");
            let fused_adj =
                casting::spell_adjustment_with_def(&self.state, player, raw_obj, Some(def));
            let fused_printed =
                casting::printed_reduction(&self.state, &def.faces[0], player, card)
                    .saturating_add(casting::printed_reduction(
                        &self.state,
                        &def.faces[1],
                        player,
                        card,
                    ));
            let raw_fused = casting::fused_mana_cost(def);
            let fused_cost = fused_adj.price(raw_fused, fused_printed);
            let fused_allow = fused_adj.allowance(
                &self.state,
                casting::SpendFor::Spell(card),
                fused_printed.saturating_add(reduction),
            );
            if !casting::cast_is_forbidden(&self.state, player, raw_obj)
                && (defer_mana
                    || casting::source_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::Spell(card),
                        pool,
                        &less(fused_cost.with_x(0)),
                        &[],
                        Some(fused_allow),
                    )
                    || casting::north_star_face_allowed(&self.state, player, card, 0)
                        && casting::normal_spell_affordable(
                            &self.state,
                            player,
                            casting::SpendFor::Spell(card),
                            pool,
                            &less(fused_cost.with_x(0)),
                            &raw_fused,
                            fused_allow,
                        ))
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 1)
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u32,
                    kind: CastModeKind::Fused,
                    cost: fused_cost,
                });
            }
        }
        let plot_only = plot_ok
            && !commanded
            && foretell_cost.is_none()
            && casting::play_permission(&self.state, player, card).is_none();
        let plot_with_restricted = plot_only.then(|| {
            casting::planning_pool(
                &self.state,
                player,
                casting::SpendFor::SpellAs(card, casting::SpellForm::Plot),
                0,
            )
        });
        let plot_pool = plot_with_restricted
            .as_ref()
            .and_then(Option::as_ref)
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        let plot_adjustment = plot_only.then(|| {
            casting::face_adjustment(
                &self.state,
                player,
                card,
                def,
                0,
                Some(casting::SpellForm::Plot),
            )
        });
        let afford_plot = |cost: &ManaCost| {
            let adj = plot_adjustment.expect("plot_only");
            let allow = adj.allowance(
                &self.state,
                casting::SpendFor::SpellAs(card, casting::SpellForm::Plot),
                printed_reduction,
            );
            defer_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::SpellAs(card, casting::SpellForm::Plot),
                    plot_pool,
                    &adj.price(*cost, printed_reduction),
                    &[],
                    Some(allow),
                )
        };
        let modal_base = if plot_only {
            ManaCost::ZERO
        } else {
            face.mana_cost
        };
        let modal_price = |cost: ManaCost| {
            if let Some(adj) = plot_adjustment {
                adj.price(cost, printed_reduction)
            } else {
                price(cost)
            }
        };
        // Modal spells (overload & friends): one option per mode — or, for a
        // spell that chooses more than one, one per set of modes it may be
        // cast with, each priced with its modes' own costs (CR 700.2h).
        for ability in def.abilities {
            let Some((modes, choose, can_repeat)) = ability.modal_spell() else {
                continue;
            };
            if !(front_additional_ok
                && (front_timing_ok || plot_only && casting::sorcery_window(&self.state, player)))
            {
                continue;
            }
            if can_repeat {
                for packed in casting::repeated_mode_sets(modes, choose) {
                    let cost = casting::repeated_mode_set_cost(modal_base, modes, packed);
                    let affordable = if plot_only {
                        afford_plot(&cost.with_x(0))
                    } else {
                        afford_normal(&cost.with_x(0))
                    };
                    if affordable
                        && casting::repeated_mode_set_has_legal_targets(
                            &self.state,
                            &self.lookup,
                            player,
                            card,
                            packed,
                        )
                    {
                        options.push(CastModeDesc {
                            index: (options.len()) as u32,
                            kind: CastModeKind::RepeatedModes(packed),
                            cost: modal_price(cost),
                        });
                    }
                }
                continue;
            }
            if !choose.is_one() {
                for set in casting::mode_sets(modes, choose) {
                    let cost = casting::mode_set_cost(modal_base, modes, set);
                    let affordable = if plot_only {
                        afford_plot(&cost.with_x(0))
                    } else {
                        afford_normal(&cost.with_x(0))
                    };
                    if affordable
                        && casting::mode_set_has_legal_targets(
                            &self.state,
                            &self.lookup,
                            player,
                            card,
                            set,
                        )
                    {
                        options.push(CastModeDesc {
                            index: (options.len()) as u32,
                            kind: CastModeKind::Modes(set),
                            cost: modal_price(cost),
                        });
                    }
                }
                continue;
            }
            for (i, mode) in modes.iter().enumerate() {
                if plot_only && mode.cost_override.is_some() {
                    continue;
                }
                let cost = mode.cost_override.unwrap_or(modal_base);
                // Affordable *and* pointable. Every other option in this
                // function is paired with the probe `can_cast` makes, and a
                // mode was the one that was not: Cyclonic Rift on an empty
                // board offered "return target nonland permanent you don't
                // control" and answered the press with "not enough legal
                // targets", leaving the question standing so an agent pressed
                // it again on the next pass.
                if (if plot_only {
                    afford_plot(&cost.with_x(0))
                } else if mode.cost_override.is_some() {
                    afford(&cost.with_x(0))
                } else {
                    afford_normal(&cost.with_x(0))
                }) && casting::mode_has_a_legal_target(
                    &self.state,
                    &self.lookup,
                    player,
                    card,
                    i,
                ) {
                    options.push(CastModeDesc {
                        index: (options.len()) as u32,
                        kind: CastModeKind::Mode(i),
                        cost: modal_price(cost),
                    });
                }
            }
            if let Some(entwine) = face.entwine {
                let set = casting::entwine_set(modes);
                let cost = modal_base.combine(&entwine.mana);
                let affordable = if plot_only {
                    afford_plot(&cost.with_x(0))
                } else {
                    afford_normal(&cost.with_x(0))
                };
                if affordable
                    && casting::additional_payment_capacity_for(
                        &self.state,
                        player,
                        card,
                        face,
                        Some(CastModeKind::Modes(set)),
                        &[],
                    )
                    .is_some()
                    && casting::mode_set_has_legal_targets(
                        &self.state,
                        &self.lookup,
                        player,
                        card,
                        set,
                    )
                {
                    options.push(CastModeDesc {
                        index: (options.len()) as u32,
                        kind: CastModeKind::Modes(set),
                        cost: modal_price(cost),
                    });
                }
            }
        }
        // Timing, per way of casting (CR 601.3): the front's for every option
        // that casts the front, and a back face's own for that face
        // (`casting::face_timing_allows`, CR 712.8c, 716.3a). An enchantment
        // with an instant Adventure, cast while the stack is not empty, is
        // offered as the Adventure alone. Renumbered, because an option's
        // index is its place in this list.
        // A split card's front is its left half, where the card is both
        // (`casting::split_front`, CR 709.3a).
        let split_front = casting::split_front(&self.state, &obj);
        let front_spell = split_front.as_ref().unwrap_or(&obj);
        let front = front_spell.characteristics();
        let front_timing =
            casting::permitted_spell_types(&self.state, player, card, front.types, 0)
                && (commanded
                    || casting::timing_allows(&self.state, player, front.types, front.keywords))
                && casting::spell_condition_allows(&self.state, player, card, def, 0);
        let front_now =
            front_timing && !casting::cast_is_forbidden(&self.state, player, front_spell);
        options.retain(|o| match o.kind {
            // Land plays, Bestow Auras, and Impending casts (`can_cast_form_paying`)
            // have already passed their own timing, permission, and lock checks.
            CastModeKind::PlayLandFace(_) | CastModeKind::Bestow | CastModeKind::Impending => true,
            // The lock was asked of these as what they are, a prototype or
            // face-down spell (`casting::can_cast_form_paying`).
            CastModeKind::Prototype | CastModeKind::Disguise => front_timing,
            CastModeKind::Foretell => {
                casting::permitted_spell_types(&self.state, player, card, face.types, 0)
                    && (commanded
                        || casting::timing_allows(&self.state, player, face.types, face.keywords))
                    && casting::spell_condition_allows(&self.state, player, card, def, 0)
            }
            CastModeKind::Plot => {
                casting::sorcery_window(&self.state, player)
                    && !face.types.contains(baylee_core::types::TypeSet::LAND)
                    && casting::permitted_spell_types(&self.state, player, card, face.types, 0)
                    && casting::spell_condition_allows(&self.state, player, card, def, 0)
                    && !casting::cast_is_forbidden(&self.state, player, front_spell)
            }
            CastModeKind::Mode(_) | CastModeKind::Modes(_) | CastModeKind::RepeatedModes(_)
                if plot_only =>
            {
                casting::sorcery_window(&self.state, player)
                    && !face.types.contains(baylee_core::types::TypeSet::LAND)
                    && casting::permitted_spell_types(&self.state, player, card, face.types, 0)
                    && casting::spell_condition_allows(&self.state, player, card, def, 0)
                    && !casting::cast_is_forbidden(&self.state, player, front_spell)
            }
            CastModeKind::Face(i) | CastModeKind::EscapeGranted { face: i, .. } => {
                (commanded || casting::face_timing_allows(&self.state, player, card, def, i))
                    && casting::spell_condition_allows(&self.state, player, card, def, i)
            }
            CastModeKind::Fused => {
                let raw_c = obj.characteristics();
                casting::permitted_spell_types(&self.state, player, card, raw_c.types, 0)
                    && (commanded
                        || casting::timing_allows(&self.state, player, raw_c.types, raw_c.keywords))
                    && casting::spell_condition_allows(&self.state, player, card, def, 0)
                    && casting::spell_condition_allows(&self.state, player, card, def, 1)
                    && !casting::cast_is_forbidden(&self.state, player, &obj)
            }
            CastModeKind::Sneak => {
                casting::in_sneak_window(&self.state, player)
                    && !face.types.contains(baylee_core::types::TypeSet::LAND)
                    && casting::permitted_spell_types(&self.state, player, card, face.types, 0)
                    && casting::spell_condition_allows(&self.state, player, card, def, 0)
                    && !casting::cast_is_forbidden(&self.state, player, front_spell)
            }
            _ => front_now,
        });
        if in_graveyard
            && !commanded
            && casting::play_permission(&self.state, player, card).is_none()
        {
            options.retain(|option| {
                let face_index = match option.kind {
                    CastModeKind::Face(face) => face,
                    _ => 0,
                };
                matches!(
                    option.kind,
                    CastModeKind::Escape
                        | CastModeKind::Flashback
                        | CastModeKind::JumpStart
                        | CastModeKind::Retrace
                        | CastModeKind::BeamMeUp
                        | CastModeKind::EscapeGranted { .. }
                ) || (matches!(option.kind, CastModeKind::Face(_))
                    && (def.faces[face_index].disturb || def.faces[face_index].aftermath))
                    || !casting::graveyard_face_permissions(&self.state, player, card, face_index)
                        .is_empty()
                    || (face_index == 0
                        && casting::mana_cost_mode(def, face_index, option.kind)
                        && casting::flashback_granted(&self.state, card))
            });
        }
        if foretell_cost.is_some()
            && !commanded
            && casting::play_permission(&self.state, player, card).is_none()
        {
            options.retain(|o| matches!(o.kind, CastModeKind::Foretell));
        }
        if plot_only {
            options.retain(|o| {
                if modal_only {
                    matches!(
                        o.kind,
                        CastModeKind::Mode(_)
                            | CastModeKind::Modes(_)
                            | CastModeKind::RepeatedModes(_)
                    )
                } else {
                    matches!(o.kind, CastModeKind::Plot)
                }
            });
        }
        if commanded
            && self.state.turn.active == player
            && casting::has_a_land_drop_left(&self.state, player)
        {
            for face in def.land_faces_from_hand() {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::PlayLandFace(face),
                    cost: ManaCost::ZERO,
                });
            }
        }
        // "The player plays that card if able" (Word of Command): a way of
        // casting it whose price the commanded player's lands cannot pay is
        // no way they are able to play it. Taken off before it is offered,
        // so the controller cannot choose a mode that is then reversed while
        // another would have been played, and a card no mode pays is not
        // played at all. The exact reader only: where it cannot tell, the
        // mode stays.
        let mut alt_routes: Vec<(usize, casting::AltCastGrantRoute)> = Vec::new();
        if !commanded {
            let (alt_pairs, _) =
                casting::alt_cast_grant_options(&self.state, &self.lookup, player, card, paying);
            for (desc, route) in alt_pairs {
                let idx = options.len();
                options.push(desc);
                alt_routes.push((idx, route));
            }
        }
        if commanded {
            options.retain(|o| {
                matches!(o.kind, CastModeKind::PlayLandFace(_))
                    || self.commanded_cost_feasible(player, card, &less(o.cost.with_x(0)))
                        != Some(false)
            });
        }
        for (i, option) in options.iter_mut().enumerate() {
            option.index = u32::try_from(i).expect("cast option index fits the protocol");
        }
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        Ok((options, alt_routes))
    }
}
