//! Available ways to cast a spell.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    /// All legal ways to cast `card` right now.
    #[allow(clippy::too_many_lines)] // one branch per printed way to cast; splitting hides the list
    pub(in crate::engine) fn cast_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        pay_in_window: bool,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        let obj = self
            .state
            .object(card)
            .ok_or(EngineError::IllegalAction("no such card"))?;
        let card_ref = obj
            .card
            .ok_or(EngineError::IllegalAction("not card-backed"))?;
        let def = self
            .lookup
            .card(card_ref.index)
            .ok_or(EngineError::IllegalAction("unknown card"))?;
        let face = &def.faces[0];
        let commanded = self.commanded_card(card);
        if commanded
            && (casting::cast_is_forbidden(&self.state, player, obj)
                || !casting::may_begin_casting(&self.state, player))
        {
            return Err(EngineError::IllegalAction("casting prohibited"));
        }
        let defer_mana =
            pay_in_window || (commanded && self.compute_legal(player).has_mana_source());
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
        let printed_reduction = casting::printed_reduction(&self.state, face, player, card);
        let price = |cost: ManaCost| adjustment.price(cost, printed_reduction);
        let back_price = |i: usize| {
            casting::face_adjustment(&self.state, player, card, def, i, None).price(
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
        // one function rather than in each of them.
        let reduction = casting::keyword_reduction(&self.state, face, player, card);
        let allowance = adjustment.allowance(
            &self.state,
            casting::SpendFor::Spell(card),
            printed_reduction.saturating_add(reduction),
        );
        let back_allowance = |i| {
            casting::face_adjustment(&self.state, player, card, def, i, None).allowance(
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
                    &price(*cost).with_less_generic(reduction),
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
                        &price(*cost).with_less_generic(reduction),
                        &face.mana_cost,
                        allowance,
                    )
        };
        let mut options =
            casting::granted_escape_options(&self.state, &self.lookup, player, card, paying);
        // Disturb casts come from the graveyard: no normal-cost option.
        let disturb_cast = self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
            && def.faces.iter().any(|f| f.disturb);
        if disturb_cast {
            for (i, back) in def.faces.iter().enumerate().skip(1) {
                if back.disturb
                    && casting::face_timing_allows(&self.state, player, card, def, i)
                    && (defer_mana
                        || casting::source_affordable(
                            &self.state,
                            player,
                            casting::SpendFor::SpellOnFace(card, i),
                            pool,
                            &back_price(i).with_x(0),
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
            // A graveyard permission for the front (Muldrotha, Wrenn's
            // emblem) is a second way, offered below beside the backs.
            let front_too = self.state.object(card).is_some_and(|o| {
                casting::graveyard_cast_permission(&self.state, player, o).is_some()
            });
            if !front_too {
                return Ok(options);
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
        let modal_only = abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
            && !abilities
                .iter()
                .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Spell { .. }))
            && !face.types.is_permanent();
        // Normal cost (X probed with 0; the real check happens at payment).
        // Guarded by the same CR 202.1b question `can_cast` asks, and for the
        // reason every probe in this function is paired with one there: an
        // option offered here that the offer does not know about is a mode a
        // player can pick and be refused for.
        if !modal_only
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
        if !face.additional_costs.is_empty()
            && (face.kicked_targets.is_some() || !afford_normal(&face.mana_cost.with_x(0)))
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
                    &price(cost).with_x(0).with_less_generic(reduction),
                    &[],
                    Some(allowance),
                )
                || casting::north_star_face_allowed(&self.state, player, card, 0)
                    && casting::normal_spell_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::KickedSpell(card, 0, None),
                        kicked_pool,
                        &price(cost).with_x(0).with_less_generic(reduction),
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
        if face.disguise.is_some() || face.morph.is_some() {
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
        // Alternative costs (pitch, evoke, conditional free).
        for (i, alt) in face.alternative_costs.iter().enumerate() {
            let condition_ok = match alt.condition {
                AltCondition::Always => true,
                AltCondition::NotYourTurn => self.state.turn.active != player,
                AltCondition::CommanderControlled => self.has_commander_on_battlefield(player),
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
            {
                continue;
            }
            options.push(CastModeDesc {
                index: (options.len()) as u32,
                kind: CastModeKind::Alternative(i),
                cost: price(alt.cost.mana),
            });
        }
        // Dash (CR 702.109a): its cost "rather than its mana cost", which is
        // an alternative cost (CR 118.9) paid by the rules for one (601.2b,
        // 601.2f–h) — asked of the pool the way the alternatives above are.
        if let Some(dash) = face.dash {
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
                    cost: taxed.mana,
                });
            }
        }
        if let Some(mutate) = face.mutate
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
                    cost: taxed.mana,
                });
            }
        }
        // MDFC backs (CR 712.11b) and adventures (CR 715), through the reader
        // `can_cast` uses — land faces are played and disturb backs came out
        // above. This loop asked the faces and nothing else, so the adventure
        // was offered again out of the exile its own resolution had put the
        // card in, and at instant speed forever: the offer read the exiled
        // object's *current* face, which was still the instant, so Swift
        // Spiral was `{1}{W}` in `LegalActions` every turn and the wizard
        // priced and cast it. CR 715.3d is the rule it broke.
        let on_adventure = casting::adventure_only_permission(&self.state, player, card);
        for (i, _) in casting::castable_back_faces(def, on_adventure) {
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
                    &back_price(i).with_x(0),
                    &[],
                    Some(back_allowance(i)),
                )
                || casting::north_star_face_allowed(&self.state, player, card, i)
                    && casting::normal_spell_affordable(
                        &self.state,
                        player,
                        casting::SpendFor::SpellOnFace(card, i),
                        pool,
                        &back_price(i).with_x(0),
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
        // Modal spells (overload & friends): one option per mode — or, for a
        // spell that chooses more than one, one per set of modes it may be
        // cast with, each priced with its modes' own costs (CR 700.2h).
        for ability in def.abilities {
            let AbilityDef::ModalSpell { modes, choose } = ability else {
                continue;
            };
            if !choose.is_one() {
                for set in casting::mode_sets(modes, *choose) {
                    let cost = casting::mode_set_cost(face.mana_cost, modes, set);
                    if afford_normal(&cost.with_x(0))
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
                            cost: price(cost),
                        });
                    }
                }
                continue;
            }
            for (i, mode) in modes.iter().enumerate() {
                let cost = mode.cost_override.unwrap_or(face.mana_cost);
                // Affordable *and* pointable. Every other option in this
                // function is paired with the probe `can_cast` makes, and a
                // mode was the one that was not: Cyclonic Rift on an empty
                // board offered "return target nonland permanent you don't
                // control" and answered the press with "not enough legal
                // targets", leaving the question standing so an agent pressed
                // it again on the next pass.
                if (if mode.cost_override.is_some() {
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
                        cost: price(cost),
                    });
                }
            }
            if let Some(entwine) = face.entwine {
                let set = casting::entwine_set(modes);
                let cost = face.mana_cost.combine(&entwine.mana);
                if afford_normal(&cost.with_x(0))
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
                        cost: price(cost),
                    });
                }
            }
        }
        // Timing, per way of casting (CR 601.3): the front's for every option
        // that casts the front, and a back face's own for that face
        // (`casting::face_timing_allows`, CR 712.11c, 715.3a). An enchantment
        // with an instant Adventure, cast while the stack is not empty, is
        // offered as the Adventure alone. Renumbered, because an option's
        // index is its place in this list.
        let front = obj.characteristics();
        let front_now = casting::permitted_spell_types(&self.state, player, card, front.types, 0)
            && (commanded
                || casting::timing_allows(&self.state, player, front.types, front.keywords))
            && casting::spell_condition_allows(&self.state, player, card, def, 0);
        options.retain(|o| match o.kind {
            CastModeKind::PlayLandFace(_) => true,
            CastModeKind::Face(i) | CastModeKind::EscapeGranted { face: i, .. } => {
                (commanded || casting::face_timing_allows(&self.state, player, card, def, i))
                    && casting::spell_condition_allows(&self.state, player, card, def, i)
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
                        | CastModeKind::BeamMeUp
                        | CastModeKind::EscapeGranted { .. }
                ) || (matches!(option.kind, CastModeKind::Face(_)) && def.faces[face_index].disturb)
                    || !casting::graveyard_face_permissions(&self.state, player, card, face_index)
                        .is_empty()
                    || (face_index == 0
                        && casting::mana_cost_mode(def, face_index, option.kind)
                        && casting::flashback_granted(&self.state, card))
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
        if commanded {
            options.retain(|o| {
                matches!(o.kind, CastModeKind::PlayLandFace(_))
                    || self.commanded_cost_feasible(
                        player,
                        card,
                        &o.cost.with_x(0).with_less_generic(reduction),
                    ) != Some(false)
            });
        }
        for (i, option) in options.iter_mut().enumerate() {
            option.index = u32::try_from(i).expect("cast option index fits the protocol");
        }
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        Ok(options)
    }
}
