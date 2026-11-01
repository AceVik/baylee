//! Legal-action offers and payable activation windows.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    pub(super) fn activation_increases(&self, player: PlayerId) -> Vec<(ObjectId, u32)> {
        if !self.state.effects.may_change_or_forbid_abilities()
            || !self.state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::AbilitiesCostMore(_)
                )
            })
        {
            return Vec::new();
        }
        self.state
            .battlefield_seen()
            .filter(|id| {
                self.state
                    .object(*id)
                    .is_some_and(|o| o.controller == player)
            })
            .filter_map(|id| {
                let n = casting::activation_increase(&self.state, id);
                (n > 0).then_some((id, n))
            })
            .collect()
    }

    pub(super) fn spell_increases(
        &self,
        player: PlayerId,
    ) -> Vec<(ObjectId, crate::choice::CastModeKind, u32)> {
        use crate::choice::CastModeKind;
        if !self.state.effects.may_change_spell_costs()
            || !self.state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::SpellsCostMore(_)
                        | baylee_cards_dsl::Modifier::SpellsCostMorePerControlled(_)
                        | baylee_cards_dsl::Modifier::SpellsCostMoreMana(_)
                )
            })
        {
            return Vec::new();
        }
        let mut cards: Vec<_> = [
            ZoneLocation::Hand(player),
            ZoneLocation::Graveyard(player),
            ZoneLocation::Command(player),
            ZoneLocation::Exile(player),
        ]
        .into_iter()
        .flat_map(|zone| self.state.zones.list(zone).iter().copied())
        .collect();
        cards.extend(
            self.state
                .play_permissions
                .iter()
                .filter(|p| p.player == player)
                .map(|p| p.card),
        );
        cards.sort_unstable();
        cards.dedup();
        let mut out = Vec::new();
        for card in cards {
            let Some(def) = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            else {
                continue;
            };
            let commander = casting::commander_tax(&self.state, player, card);
            let increase = |face, form| {
                casting::face_increase(&self.state, player, card, def, face, form)
                    .saturating_sub(commander)
            };
            let normal = increase(0, None);
            if normal > 0 {
                out.push((card, CastModeKind::Normal, normal));
            }
            for face in 1..def.faces.len() {
                let n = increase(face, None);
                if n != normal {
                    out.push((card, CastModeKind::Face(face), n));
                }
            }
            if let Some(prototype) = def.faces[0].prototype {
                let n = increase(0, Some(casting::SpellForm::Prototype(prototype)));
                if n != normal {
                    out.push((card, CastModeKind::Prototype, n));
                }
            }
            if def.faces[0].disguise.is_some() || def.faces[0].morph.is_some() {
                let n = increase(0, Some(casting::SpellForm::Disguise));
                if n != normal {
                    out.push((card, CastModeKind::Disguise, n));
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn compute_legal(&self, player: PlayerId) -> LegalActions {
        let mut legal = LegalActions {
            can_pass: true,
            spell_increases: self.spell_increases(player),
            activation_increases: self.activation_increases(player),
            ..LegalActions::default()
        };
        let main_phase = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        // Check broad zone access once; native Mayhem opens only its own card.
        let graveyard_open = casting::land_zone_open(&self.state, player, Zone::Graveyard);
        let graveyard: &[ObjectId] = if graveyard_open
            || self
                .state
                .per_turn
                .discarded_to_graveyard
                .iter()
                .any(|(_, discarder)| *discarder == player)
        {
            self.state.zones.list(ZoneLocation::Graveyard(player))
        } else {
            &[]
        };
        for (&card, from_hand) in self
            .state
            .zones
            .list(ZoneLocation::Hand(player))
            .iter()
            .map(|c| (c, true))
            .chain(
                graveyard
                    .iter()
                    .filter(|card| {
                        graveyard_open
                            || casting::mayhem_land_permission(&self.state, player, **card)
                    })
                    .map(|c| (c, false)),
            )
            .chain(
                self.state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .last()
                    .filter(|card| casting::land_card_open(&self.state, player, **card))
                    .map(|card| (card, false)),
            )
        {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            // A modal card's land back is a land drop (CR 712.12); a
            // transforming card's is not (CR 712.8a) — see
            // `CardDef::land_faces_from_hand`, which `PlayLand` and the house
            // AI ask as well.
            let plays_as_land = obj.characteristics().types.contains(TypeSet::LAND)
                || obj
                    .card
                    .and_then(|c| self.lookup.card(c.index))
                    .is_some_and(|def| def.land_faces_from_hand().next().is_some());
            if plays_as_land
                && sorcery_timing
                && casting::has_a_land_drop_left(&self.state, player)
                && casting::land_card_open(&self.state, player, card)
            {
                legal.lands.push(card);
            }
            if !from_hand {
                continue;
            }
            match casting::can_cast(&self.state, &self.lookup, player, card) {
                Ok(()) if self.has_a_legal_target(player, card) => legal.castable.push(card),
                Err(casting::CastError::NotEnoughMana) => legal.payable.push(card),
                _ => {}
            }
        }
        // Permission can belong to someone other than the owner, so inspect
        // each exile zone once, in seat order, without copying any cards.
        for &card in self.state.players.iter().enumerate().flat_map(|(seat, _)| {
            self.state
                .zones
                .list(ZoneLocation::Exile(PlayerId::new(seat as u8)))
        }) {
            let permitted = casting::adventure_permission(&self.state, player, card)
                || casting::foretell_permission(&self.state, player, card)
                || self.state.object(card).is_some_and(|o| {
                    o.riders
                        .contains(&crate::object::Rider::PlayableFromExileFor(player))
                });
            if !permitted {
                continue;
            }
            if sorcery_timing
                && casting::has_a_land_drop_left(&self.state, player)
                && casting::land_card_open(&self.state, player, card)
                && self
                    .state
                    .object(card)
                    .is_some_and(|o| o.characteristics().types.contains(TypeSet::LAND))
            {
                legal.lands.push(card);
            }
            match casting::can_cast(&self.state, &self.lookup, player, card) {
                Ok(()) if self.has_a_legal_target(player, card) => legal.castable.push(card),
                Err(casting::CastError::NotEnoughMana) => legal.payable.push(card),
                _ => {}
            }
        }
        // "You may play that card this turn" (Dauthi Voidwalker, Expressive
        // Iteration): a permission for one object, wherever it lies — an
        // opponent's exile for Dauthi, which is why this walks the
        // permissions rather than a zone. A land is a land drop like any
        // other (CR 305.2), asked by `casting::play_land` through
        // `land_card_open`, which reads the same permission.
        for permission in &self.state.play_permissions {
            let card = permission.card;
            if permission.player != player
                || casting::play_permission(&self.state, player, card).is_none()
            {
                continue;
            }
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            if obj.zone == Zone::Hand {
                continue;
            }
            let plays_as_land = obj.characteristics().types.contains(TypeSet::LAND)
                || obj
                    .card
                    .and_then(|c| self.lookup.card(c.index))
                    .is_some_and(|def| def.land_faces_from_hand().next().is_some());
            // A permission to cast and not to play (Ragavan) opens no land
            // drop, and a land it names is not cast either (CR 305.9): the
            // castable probe below says so for it.
            if plays_as_land
                && !permission.cast_only
                && sorcery_timing
                && casting::has_a_land_drop_left(&self.state, player)
                && casting::land_card_open(&self.state, player, card)
                && !legal.lands.contains(&card)
            {
                legal.lands.push(card);
            }
            if !legal.castable.contains(&card) && !legal.payable.contains(&card) {
                match casting::can_cast(&self.state, &self.lookup, player, card) {
                    Ok(()) if self.has_a_legal_target(player, card) => legal.castable.push(card),
                    Err(casting::CastError::NotEnoughMana) => legal.payable.push(card),
                    _ => {}
                }
            }
        }
        // Commander (CR 903.8): your own commanders in the command zone.
        // The emblems sharing that zone are filtered out by `can_cast`,
        // which asks the marker list rather than the zone.
        for &card in self.state.zones.list(ZoneLocation::Command(player)) {
            match casting::can_cast(&self.state, &self.lookup, player, card) {
                Ok(()) if self.has_a_legal_target(player, card) => legal.castable.push(card),
                Err(casting::CastError::NotEnoughMana) => legal.payable.push(card),
                _ => {}
            }
        }
        // Flashback (CR 702.34) and disturb (CR 702.146): a card in your own
        // graveyard is castable when a grant or a printed face says so, and
        // `can_cast` asks both of those itself — for a graveyard card it is
        // exactly `flashback_ok || disturb_ok` that keeps it out of
        // `CastError::NotInHand`. So this sweep asks nothing. The pre-check
        // it used to carry was not a fast path, it was a second opinion:
        // it recognised only the `EffectFilter::ObjectIs` shape of a grant
        // and hid every filtered one from the offer.
        for &card in self.state.zones.list(ZoneLocation::Graveyard(player)) {
            match casting::can_cast(&self.state, &self.lookup, player, card) {
                Ok(()) if self.has_a_legal_target(player, card) => legal.castable.push(card),
                Err(casting::CastError::NotEnoughMana) => legal.payable.push(card),
                _ => {}
            }
        }
        // The grants are collected once rather than asked of the whole
        // effect table per permanent (`effects::grants`): an Ally board
        // carries thousands of "until end of turn" effects and grants
        // through one or two of them.
        let grants: smallvec::SmallVec<[&crate::effects::ContinuousEffect; 4]> =
            crate::effects::grants(&self.state).collect();
        let activation_locks: SmallVec<[&crate::effects::ContinuousEffect; 4]> = self
            .state
            .effects
            .iter()
            .filter(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CantActivateArtifacts
                        | baylee_cards_dsl::Modifier::CantActivate
                )
            })
            .collect();
        // A phased-out permanent is treated as though it does not exist
        // (CR 702.26b): nothing it has is offered, a mana ability included.
        for id in self.state.battlefield_seen() {
            // Karn's lock, asked on the offering side too. It stops every
            // activated ability of the permanent, a mana ability included —
            // CR 605.1 makes a mana ability a kind of activated ability, not
            // an exception to one. What it does *not* stop is a cast, which
            // is why this is two guards and not a `continue`: the Prepared
            // rider below offers a spell.
            let locked = self.activations_are_locked_by(id, activation_locks.iter().copied());
            if !locked
                && !casting::intrinsic_mana_choices(&self.state, &self.lookup, player, id)
                    .is_empty()
            {
                legal.mana_abilities.push(id);
            }
            // Activated abilities of controlled permanents.
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            if obj.controller == player
                && let Some(cost) = self.disguise_cost(id)
            {
                if self.can_pay_mana(
                    player,
                    casting::SpendFor::OtherFrom(baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: obj.version,
                    }),
                    &cost,
                ) {
                    legal.abilities.push((id, crate::choice::TURN_FACE_UP));
                } else {
                    legal
                        .unpaid_abilities
                        .push((id, crate::choice::TURN_FACE_UP, cost));
                }
            }
            // CR 709.5e: a locked door, as a sorcery, for its mana cost.
            if obj.controller == player && sorcery_timing {
                for half in self.unlockable_halves(id) {
                    legal.abilities.push((id, crate::choice::unlock_door(half)));
                }
            }
            // A token's abilities come from its definition rather than from a
            // card; everything below reads the same `AbilityDef`s either way.
            let offered = if locked {
                crate::copiable_abilities::AbilityDefs::EMPTY
            } else {
                obj.abilities(&self.lookup)
            };
            let granted =
                crate::effects::granted_activated_among(&self.state, grants.iter().copied(), id)
                    .filter(|_| !locked)
                    .map(|grant| (grant.id, grant.definition));
            for (i, ability) in offered
                .iter()
                .enumerate()
                .map(|(index, definition)| (index as u32, *definition))
                .chain(granted)
            {
                let Some(ability) = self.activation_offer_definition(player, id, i, &ability)
                else {
                    continue;
                };
                let ability = &ability;
                if ability.is_intrinsic_mana_ability()
                    && casting::intrinsic_mana_colors(&self.state, id).is_empty()
                {
                    continue;
                }
                match ability {
                    AbilityDef::Activated {
                        cost,
                        timing,
                        zone,
                        targets,
                        second_targets,
                        limit,
                        cost_reduction,
                        ..
                    } => {
                        if !match zone {
                            ActivationZone::Battlefield => obj.controller == player,
                            ActivationZone::BattlefieldOwner => obj.owner == player,
                            ActivationZone::BattlefieldAnyPlayer => true,
                            ActivationZone::BattlefieldOpponent => {
                                self.state.is_opponent(player, obj.controller)
                            }
                            ActivationZone::BattlefieldAttachedController => {
                                crate::eval::controller_of_attached(&self.state, id) == Some(player)
                            }
                            _ => false,
                        } {
                            continue; // hand-zone abilities are scanned below
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(player, id, i, *limit) {
                            continue;
                        }
                        if !self.source_cost_parts_ready(id, cost) {
                            continue;
                        }
                        let cost = self.activation_price(player, id, i, cost, *cost_reduction);
                        let payable = self.activation_affordable(
                            player,
                            id,
                            &cost,
                            &[],
                            self.mana_activation_life(id, i),
                        );
                        self.offer_activation_payment(
                            player,
                            id,
                            i,
                            &cost,
                            payable,
                            *targets,
                            *second_targets,
                            &mut legal,
                        );
                    }
                    AbilityDef::ActivatedConditional {
                        cost,
                        timing,
                        zone,
                        condition,
                        targets,
                        second_targets,
                        limit,
                        cost_reduction,
                        ..
                    } => {
                        if !match zone {
                            ActivationZone::Battlefield => obj.controller == player,
                            ActivationZone::BattlefieldOwner => obj.owner == player,
                            ActivationZone::BattlefieldAnyPlayer => true,
                            ActivationZone::BattlefieldOpponent => {
                                self.state.is_opponent(player, obj.controller)
                            }
                            ActivationZone::BattlefieldAttachedController => {
                                crate::eval::controller_of_attached(&self.state, id) == Some(player)
                            }
                            _ => false,
                        } {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(player, id, i, *limit) {
                            continue;
                        }
                        if !self.source_cost_parts_ready(id, cost) {
                            continue;
                        }
                        if !crate::eval::condition_holds_with_context(
                            &self.state,
                            player,
                            self.ability_rule_context(id, i),
                            *condition,
                        ) {
                            continue;
                        }
                        let cost = self.activation_price(player, id, i, cost, *cost_reduction);
                        let payable = self.activation_affordable(
                            player,
                            id,
                            &cost,
                            &[],
                            self.mana_activation_life(id, i),
                        );
                        self.offer_activation_payment(
                            player,
                            id,
                            i,
                            &cost,
                            payable,
                            *targets,
                            *second_targets,
                            &mut legal,
                        );
                    }
                    AbilityDef::Loyalty {
                        cost,
                        targets,
                        second_targets,
                        ..
                    } => {
                        // Loyalty abilities: sorcery timing, once per turn
                        // per walker, enough loyalty for negative costs.
                        if obj.controller != player
                            || !sorcery_timing
                            || self.loyalty_used_this_turn.contains(&id)
                        {
                            continue;
                        }
                        let price = self.activation_price(player, id, i, &Cost::FREE, None);
                        if !self.activation_affordable(player, id, &price, &[], 0) {
                            continue;
                        }
                        let loyalty = obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty);
                        if *cost < 0 && loyalty < (-*cost) as u16 {
                            continue;
                        }
                        if !self.ability_has_targets(player, id, i, *targets, *second_targets) {
                            continue;
                        }
                        legal.abilities.push((id, i));
                    }
                    _ => {}
                }
            }
            if obj.controller != player {
                continue;
            }
            // Keep the permanent's prepared-spell button as a shortcut to the
            // same exile copy and casting wizard used by ordinary spell offers.
            if obj.riders.contains(&crate::object::Rider::Prepared)
                && let Some(copy) = self.state.prepared_copy(id)
                && casting::can_cast(&self.state, &self.lookup, player, copy).is_ok()
                && self.has_a_legal_target(player, copy)
            {
                legal.abilities.push((id, crate::choice::PREPARED_CAST));
            }
        }
        // Hand-zone activations (cycling) and suspensions, and the
        // graveyard's (eternalize): the same arms, each ability asked
        // whether the zone it is in is the zone it works from.
        for (from_zone, from) in [
            (ZoneLocation::Command(player), ActivationZone::Command),
            (ZoneLocation::Hand(player), ActivationZone::Hand),
            (ZoneLocation::Graveyard(player), ActivationZone::Graveyard),
        ] {
            for &card in self.state.zones.list(from_zone) {
                let Some(obj) = self.state.object(card) else {
                    continue;
                };
                if obj.is_merged_component() {
                    continue;
                }
                let Some(card_ref) = obj.card else { continue };
                let Some(def) = self.lookup.card(card_ref.index) else {
                    continue;
                };
                for (i, ability) in def
                    .abilities_for_face(obj.face_index as usize)
                    .iter()
                    .enumerate()
                {
                    let Some(ability) =
                        self.activation_offer_definition(player, card, i as u32, ability)
                    else {
                        continue;
                    };
                    match &ability {
                        AbilityDef::Activated {
                            cost,
                            timing,
                            zone,
                            targets,
                            second_targets,
                            limit,
                            cost_reduction,
                            ..
                        } => {
                            if *zone != from {
                                continue;
                            }
                            if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                                continue;
                            }
                            if self.activation_limit_spent(player, card, i as u32, *limit) {
                                continue;
                            }
                            // The same probe as the battlefield arms, for the
                            // reason `ability_has_a_target` gives. This arm once
                            // asked only about the turn and the price, so
                            // Rustic Clachan's reinforce was offered on a board
                            // with no creature and then refused with "no legal
                            // targets". The source is the card in hand, so a
                            // filter saying "another" still reads it right.
                            let cost = self.activation_price(
                                player,
                                card,
                                i as u32,
                                cost,
                                *cost_reduction,
                            );
                            if self.activation_affordable(
                                player,
                                card,
                                &cost,
                                &[],
                                self.mana_activation_life(card, i as u32),
                            ) && self.ability_has_targets(
                                player,
                                card,
                                i as u32,
                                *targets,
                                *second_targets,
                            ) {
                                legal.abilities.push((card, i as u32));
                            }
                        }
                        AbilityDef::ActivatedConditional {
                            cost,
                            timing,
                            zone,
                            condition,
                            targets,
                            second_targets,
                            limit,
                            cost_reduction,
                            ..
                        } => {
                            // The same ability with a precondition on it — the
                            // battlefield scan above has both arms, and this one
                            // had only the first, so a cycling ability behind an
                            // "activate only if…" clause would never be offered
                            // at all. No card in the pool prints one today; the
                            // hole is closed rather than recorded, because the
                            // arm is four lines longer than the note would be.
                            if *zone != from {
                                continue;
                            }
                            if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                                continue;
                            }
                            if self.activation_limit_spent(player, card, i as u32, *limit) {
                                continue;
                            }
                            if !crate::eval::condition_holds_with_context(
                                &self.state,
                                player,
                                self.ability_rule_context(card, i as u32),
                                *condition,
                            ) {
                                continue;
                            }
                            let cost = self.activation_price(
                                player,
                                card,
                                i as u32,
                                cost,
                                *cost_reduction,
                            );
                            if self.activation_affordable(
                                player,
                                card,
                                &cost,
                                &[],
                                self.mana_activation_life(card, i as u32),
                            ) && self.ability_has_targets(
                                player,
                                card,
                                i as u32,
                                *targets,
                                *second_targets,
                            ) {
                                legal.abilities.push((card, i as u32));
                            }
                        }
                        // Suspend's first ability is an activated one with a cost
                        // and "activate only as a sorcery" on it (CR 702.62a) —
                        // "rather than cast this card from your hand, **pay
                        // {U}** and exile it", as Ancestral Vision's reminder
                        // text puts it. So it is offered on the same two
                        // conditions as every other activation above, and this
                        // was the one branch here that asked only about the
                        // turn: a card with suspend was offered as suspendable
                        // off an empty pool, and `actions.rs` — which *does* pay
                        // the cost — then refused it. An offer the answer
                        // disagrees with is worse than no offer, because the
                        // client draws it as something to click.
                        AbilityDef::Suspend { cost, .. }
                            if from == ActivationZone::Hand
                                && sorcery_timing
                                && self.can_pay_mana(
                                    player,
                                    casting::SpendFor::OtherFrom(
                                        baylee_core::ids::DamageSourceRef {
                                            object: card,
                                            version: obj.version,
                                        },
                                    ),
                                    cost,
                                ) =>
                        {
                            legal.suspendable.push(card);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.offer_foretell(player, &mut legal);
        self.offer_granted_actions(player, &mut legal);
        self.settle_payable(player, &mut legal);
        self.narrow_under_chosen_names(&mut legal);
        self.narrow_under_split_second(&mut legal);
        self.narrow_to_mana_window(player, &mut legal);
        if self.commanded_player() == Some(player) {
            self.narrow_to_mana(&mut legal);
            crate::engine::constrained_mana::ManaActivationScope::ControlledLands.narrow(
                &self.state,
                player,
                &mut legal,
            );
        }
        // Planning hints obey every final lock too. They passed through the
        // same narrowing as actual offers, then leave the actionable list.
        legal
            .unpaid_abilities
            .retain(|(source, index, _)| legal.abilities.contains(&(*source, *index)));
        // Except an ability with {X} in its mana cost, while this seat can
        // still make mana: its X is announced before the mana is made and
        // the mana is made in its payment window (CR 602.2b, by 601.2b and
        // 601.2g), so no amount made beforehand is the right one to plan.
        // It is offered as a cast paid in its window is (`payable`), and an
        // activation the window leaves short is reversed (CR 732.1).
        let deferred: Vec<(ObjectId, u32)> = legal
            .unpaid_abilities
            .iter()
            .filter(|(source, index, mana)| {
                mana.has_variable() && self.makes_activation_mana(player, &legal, *source, *index)
            })
            .map(|&(source, index, _)| (source, index))
            .collect();
        legal
            .unpaid_abilities
            .retain(|(source, index, _)| !deferred.contains(&(*source, *index)));
        legal.abilities.retain(|(source, index)| {
            !legal
                .unpaid_abilities
                .iter()
                .any(|(s, i, _)| s == source && i == index)
        });
        legal
    }

    /// Check non-mana costs with the same reader as activation. An unpaid
    /// hint does not grant permission to activate or manufacture any mana.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn offer_activation_payment(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
        cost: &ActivationPrice,
        payable: bool,
        targets: Option<baylee_cards_dsl::TargetReq>,
        second_targets: Option<baylee_cards_dsl::TargetReq>,
        legal: &mut LegalActions,
    ) {
        let minimum = self.activation_x_minimum(player, source, index);
        if self
            .activation_resource_x_max(player, source, cost)
            .is_some_and(|max| minimum > max)
        {
            return;
        }
        let minimum_cost = cost.with_mana(cost.mana.with_x(minimum));
        let payable = payable
            && (minimum == 0
                || self.activation_affordable(
                    player,
                    source,
                    &minimum_cost,
                    &[],
                    self.mana_activation_life(source, index),
                ));
        if payable {
            if self.ability_has_targets(player, source, index, targets, second_targets) {
                legal.abilities.push((source, index));
            }
        } else if cost.mana != baylee_core::mana::ManaCost::ZERO
            && self.activation_affordable(
                player,
                source,
                &cost.with_mana(baylee_core::mana::ManaCost::ZERO),
                &[],
                self.mana_activation_life(source, index),
            )
            && self.ability_has_targets(player, source, index, targets, second_targets)
        {
            legal.abilities.push((source, index));
            legal.unpaid_abilities.push((source, index, cost.mana));
        }
    }
}
