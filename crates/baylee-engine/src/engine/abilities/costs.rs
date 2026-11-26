//! Activation prices, affordability and atomic cost payment.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

mod triggers;

/// Whether activation pricing evaluates `Modifier::ReduceCostIfTargets`
/// optimistically against legal target options (`Preview`) or against the
/// finalized chosen object targets (`Chosen`).
#[derive(Clone, Copy, Debug)]
pub(super) enum ActivationTargetPricing<'a> {
    Preview,
    Chosen(&'a [ObjectId]),
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
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

    fn can_pay_random_discards(&self, player: PlayerId, parts: &[CostPart]) -> bool {
        let needed = parts
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
    #[doc(hidden)]
    pub fn activation_price(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        cost: &Cost,
        reduction: Option<baylee_cards_dsl::CostReduction>,
    ) -> ActivationPrice {
        self.activation_price_for_targets(
            player,
            source,
            ability_index,
            cost,
            reduction,
            ActivationTargetPricing::Preview,
        )
    }

    pub(super) fn activation_price_with_x(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        cost: &Cost,
        x: u32,
        reduction: Option<baylee_cards_dsl::CostReduction>,
    ) -> ActivationPrice {
        self.activation_price_for_targets_x(
            player,
            source,
            ability_index,
            cost,
            Some(x),
            reduction,
            ActivationTargetPricing::Preview,
        )
    }

    pub(super) fn activation_price_for_targets(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        cost: &Cost,
        reduction: Option<baylee_cards_dsl::CostReduction>,
        target_pricing: ActivationTargetPricing<'_>,
    ) -> ActivationPrice {
        self.activation_price_for_targets_x(
            player,
            source,
            ability_index,
            cost,
            None,
            reduction,
            target_pricing,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "activation pricing requires full target/X/reduction context"
    )]
    pub(super) fn activation_price_for_targets_x(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        cost: &Cost,
        x: Option<u32>,
        reduction: Option<baylee_cards_dsl::CostReduction>,
        target_pricing: ActivationTargetPricing<'_>,
    ) -> ActivationPrice {
        if ability_index == crate::choice::TURN_FACE_UP {
            let spend = self.state.source_identity(source).map_or(
                casting::SpendFor::Ability(source),
                casting::SpendFor::OtherFrom,
            );
            return ActivationPrice {
                mana: cost.mana,
                parts: cost.parts.iter().copied().collect(),
                imposed: 0,
                allowance: casting::base_allowance(&self.state, spend),
                grantor: None,
            };
        }
        let (mut eligible, mut ordinary) = casting::activation_increases(&self.state, source);
        let extra_ab_inc = self.extra_ability_mana_increases(player, source, ability_index);
        ordinary = ordinary.saturating_add(extra_ab_inc.1);
        eligible = eligible.saturating_add(extra_ab_inc.0);
        let increase = eligible.saturating_add(ordinary);
        let mut base_mana = cost
            .mana
            .with_more_generic(waterbend_mana(cost.parts, x.unwrap_or(0)));
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
        let cycling = self
            .activation_definition(source, ability_index)
            .is_some_and(|ability| ability.is_cycling());
        let granted = self.granted_definition(source, ability_index);
        let reads_grantor = granted.is_some_and(|grant| grant_reads_grantor(grant.effects));
        let grantor = if reads_grantor || cost.parts.contains(&CostPart::SacrificeGrantor) {
            granted
                .and_then(|grant| grant.source)
                .and_then(|id| self.state.source_identity(id))
        } else {
            None
        };
        let mut parts: SmallVec<[CostPart; 4]> = cost.parts.iter().copied().collect();
        if matches!(
            self.activation_definition(source, ability_index),
            Some(AbilityDef::Encore { .. })
        ) && !parts.contains(&CostPart::ExileSelf)
        {
            parts.push(CostPart::ExileSelf);
        }
        // Apply reductions with a local floor first, so they do not limit
        // an independent unrestricted reduction applied afterwards. External
        // non-mana costs also apply when the printed activation has no generic mana.
        self.apply_activation_effect_adjustments(
            player,
            source,
            ability_index,
            &base_mana,
            &full,
            equip,
            cycling,
            target_pricing,
            &mut mana,
            &mut parts,
        );
        if let Some(own_reduction) = casting::mana_cost_reduction(&self.state, reduction, source) {
            mana = mana.with_less_mana(&own_reduction, false);
        }
        mana = mana.with_less_generic(off);
        let imposed = parts.len() - cost.parts.len();
        ActivationPrice {
            mana,
            parts,
            imposed,
            allowance: casting::base_allowance(&self.state, casting::SpendFor::Ability(source))
                .adjusted(
                    eligible,
                    ordinary,
                    full.generic_total().saturating_sub(mana.generic_total()),
                ),
            grantor,
        }
    }

    fn extra_ability_mana_increases(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
    ) -> (u32, u32) {
        if !self.state.effects.may_change_or_forbid_abilities() {
            return (0, 0);
        }
        let Some(object) = self.state.object(source) else {
            return (0, 0);
        };
        let def = self.activation_definition(source, ability_index);
        let is_mana = def.is_some_and(|a| a.is_mana_ability());
        let is_loyalty = matches!(def, Some(AbilityDef::Loyalty { .. }));
        let mut eligible = 0_u32;
        let mut ordinary = 0_u32;
        for effect in self.state.effects.iter() {
            let (who, filter, amount) = match effect.modifier {
                baylee_cards_dsl::Modifier::NonManaAbilitiesCostMore {
                    who,
                    filter,
                    amount,
                } if !is_mana => (who, filter, amount),
                baylee_cards_dsl::Modifier::LoyaltyAbilitiesCostMore {
                    who,
                    filter,
                    amount,
                } if is_loyalty => (who, filter, amount),
                _ => continue,
            };
            if amount == 0
                || !self.ability_target_reduction_source_matches(
                    player, source, object, effect, who, filter,
                )
            {
                continue;
            }
            if casting::effect_is_un(&self.state, effect.source) {
                eligible = eligible.saturating_add(amount);
            } else {
                ordinary = ordinary.saturating_add(amount);
            }
        }
        (eligible, ordinary)
    }

    pub(super) fn has_target_conditional_ability_reduction(
        &self,
        player: PlayerId,
        source: ObjectId,
    ) -> bool {
        if !self.state.effects.may_change_or_forbid_abilities() {
            return false;
        }
        let Some(object) = self.state.object(source) else {
            return false;
        };
        self.state
            .effects
            .iter()
            .any(|effect| match effect.modifier {
                baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                    who,
                    spell_filter,
                    reduction,
                    abilities: true,
                    ..
                } => {
                    reduction != baylee_core::mana::ManaCost::ZERO
                        && self.ability_target_reduction_source_matches(
                            player,
                            source,
                            object,
                            effect,
                            who,
                            spell_filter,
                        )
                }
                baylee_cards_dsl::Modifier::RaiseCostIfTargets {
                    who,
                    spell_filter,
                    extra_mana,
                    extra_life,
                    abilities: true,
                    ..
                } => {
                    (extra_mana != baylee_core::mana::ManaCost::ZERO || extra_life > 0)
                        && self.ability_target_reduction_source_matches(
                            player,
                            source,
                            object,
                            effect,
                            who,
                            spell_filter,
                        )
                }
                baylee_cards_dsl::Modifier::EquipCostsLessIfTargets { who, amount, .. } => {
                    amount > 0
                        && object
                            .characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                        && eval::players_for_source(
                            who,
                            &self.state,
                            effect.controller,
                            effect.source,
                        )
                        .is_some_and(|players| players.contains(&player))
                }
                _ => false,
            })
    }

    fn ability_target_reduction_source_matches(
        &self,
        player: PlayerId,
        source: ObjectId,
        object: &GameObject,
        effect: &crate::effects::ContinuousEffect,
        who: baylee_cards_dsl::PlayerRel,
        spell_filter: &'static baylee_cards_dsl::Filter,
    ) -> bool {
        let player_matches =
            eval::players_for_source(who, &self.state, effect.controller, effect.source)
                .is_some_and(|players| players.contains(&player));
        if !player_matches {
            return false;
        }
        let outer_matches = matches!(
            effect.filter,
            crate::effects::EffectFilter::Dsl(
                baylee_cards_dsl::Filter::Any | baylee_cards_dsl::Filter::This
            ) | crate::effects::EffectFilter::DslControlledBy(_, _)
        ) || crate::effects::applies_to(&self.state, effect, object);
        if !outer_matches {
            return false;
        }
        let context = crate::text_changes::RuleContext {
            source: effect.source.unwrap_or(source),
            text: self.state.effect_text(effect),
            resolved_name: None,
            resolved_subtype: effect.filter.chosen_subtype(),
            reference: None,
        };
        if object.zone == crate::zone::Zone::Battlefield {
            eval::matches_with_context(
                spell_filter,
                &self.state,
                object,
                effect.controller,
                context,
            )
        } else {
            let mut offboard = object.clone();
            offboard.controller = player;
            offboard.base_controller = player;
            eval::matches_with_context(
                spell_filter,
                &self.state,
                &offboard,
                effect.controller,
                context,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ability_pair_has_matching_target(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        first: Option<baylee_cards_dsl::TargetReq>,
        second: Option<baylee_cards_dsl::TargetReq>,
        third: Option<baylee_cards_dsl::TargetReq>,
        controller: PlayerId,
        rule_ctx: crate::text_changes::RuleContext,
        target_filter: &'static baylee_cards_dsl::Filter,
    ) -> bool {
        let wall_only = eval::target_domain::walls3(first, second, third);
        let ab_ctx = self.ability_rule_context(source, ability_index);
        let matches_any = |options: &[ObjectId]| {
            options.iter().any(|&tid| {
                self.state.object(tid).is_some_and(|target| {
                    eval::matches_with_context(
                        target_filter,
                        &self.state,
                        target,
                        controller,
                        rule_ctx,
                    )
                })
            })
        };
        let first_opts = first.map(|req| {
            let opts = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                ab_ctx,
                self.activation_x,
                wall_only,
            );
            (req, opts)
        });
        if let Some((req, ref opts)) = first_opts
            && (req.count_is_x || req.max > 0)
            && matches_any(opts)
        {
            return true;
        }
        for extra in [second, third].into_iter().flatten() {
            if !(extra.count_is_x || extra.max > 0) {
                continue;
            }
            if casting::spec_reads_reference(extra.spec) {
                if let Some((_, ref f_opts)) = first_opts
                    && f_opts.iter().any(|&reference| {
                        let mut ctx = ab_ctx;
                        ctx.reference = Some(reference);
                        let opts = eval::target_options_in_domain(
                            &extra.spec,
                            &self.state,
                            player,
                            ctx,
                            self.activation_x,
                            wall_only,
                        );
                        opts.len() >= usize::from(extra.min) && matches_any(&opts)
                    })
                {
                    return true;
                }
            } else {
                let opts = eval::target_options_in_domain(
                    &extra.spec,
                    &self.state,
                    player,
                    ab_ctx,
                    self.activation_x,
                    wall_only,
                );
                if matches_any(&opts) {
                    return true;
                }
            }
        }
        false
    }

    fn ability_preview_has_matching_target(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        controller: PlayerId,
        rule_ctx: crate::text_changes::RuleContext,
        target_filter: &'static baylee_cards_dsl::Filter,
    ) -> bool {
        let Some(ability) = self.activation_definition(source, ability_index) else {
            return false;
        };
        match ability {
            AbilityDef::Activated {
                targets,
                second_targets,
                third_targets,
                ..
            }
            | AbilityDef::ActivatedConditional {
                targets,
                second_targets,
                third_targets,
                ..
            } => self.ability_pair_has_matching_target(
                player,
                source,
                ability_index,
                targets,
                second_targets,
                third_targets,
                controller,
                rule_ctx,
                target_filter,
            ),
            AbilityDef::ModalActivated { modes, .. } => {
                if let Some(mode_idx) = self.activation_mode_index(source, ability_index)
                    && let Some(mode) = modes.get(usize::from(mode_idx))
                {
                    return self.ability_pair_has_matching_target(
                        player,
                        source,
                        ability_index,
                        mode.targets,
                        mode.second_targets,
                        None,
                        controller,
                        rule_ctx,
                        target_filter,
                    );
                }
                modes.iter().any(|mode| {
                    self.ability_has_targets(
                        player,
                        source,
                        ability_index,
                        mode.targets,
                        mode.second_targets,
                        None,
                    ) && self.ability_pair_has_matching_target(
                        player,
                        source,
                        ability_index,
                        mode.targets,
                        mode.second_targets,
                        None,
                        controller,
                        rule_ctx,
                        target_filter,
                    )
                })
            }
            _ => false,
        }
    }

    pub(super) fn ability_is_exhaust(&self, source: ObjectId, ability_index: u32) -> bool {
        self.activation_definition(source, ability_index)
            .is_some_and(|ability| {
                matches!(
                    ability,
                    AbilityDef::Activated {
                        limit: ActivationLimit::Exhaust,
                        ..
                    } | AbilityDef::ActivatedConditional {
                        limit: ActivationLimit::Exhaust,
                        ..
                    } | AbilityDef::ModalActivated {
                        limit: ActivationLimit::Exhaust,
                        ..
                    }
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn activated_ability_cost_reduction_matches(
        &self,
        player: PlayerId,
        source: ObjectId,
        object: &GameObject,
        effect: &crate::effects::ContinuousEffect,
        filter: &'static baylee_cards_dsl::Filter,
        exhaust_only: bool,
        ability_index: u32,
    ) -> bool {
        if exhaust_only && !self.ability_is_exhaust(source, ability_index) {
            return false;
        }
        if !crate::effects::source_is_present(&self.state, effect) {
            return false;
        }
        if *filter == baylee_cards_dsl::Filter::Any {
            return crate::effects::applies_to(&self.state, effect, object);
        }
        let outer_matches = matches!(
            effect.filter,
            crate::effects::EffectFilter::Dsl(
                baylee_cards_dsl::Filter::Any | baylee_cards_dsl::Filter::This
            )
        ) || crate::effects::applies_to(&self.state, effect, object);
        if !outer_matches {
            return false;
        }
        if !matches!(
            object.zone,
            crate::zone::Zone::Battlefield | crate::zone::Zone::Stack
        ) && !crate::state::filter_reaches_other_zones(filter)
        {
            return false;
        }
        let context = crate::text_changes::RuleContext {
            source: effect.source.unwrap_or(source),
            text: self.state.effect_text(effect),
            resolved_name: None,
            resolved_subtype: effect.filter.chosen_subtype(),
            reference: None,
        };
        if object.zone == crate::zone::Zone::Battlefield {
            eval::matches_with_context(filter, &self.state, object, effect.controller, context)
        } else {
            let mut offboard = object.clone();
            offboard.controller = player;
            offboard.base_controller = player;
            eval::matches_with_context(filter, &self.state, &offboard, effect.controller, context)
        }
    }

    pub(super) fn eval_activated_cost_reduction(
        &self,
        source: ObjectId,
        effect: &crate::effects::ContinuousEffect,
        amount: baylee_cards_dsl::Amount,
    ) -> u32 {
        let context = crate::text_changes::RuleContext {
            source: effect.source.unwrap_or(source),
            text: self.state.effect_text(effect),
            resolved_name: None,
            resolved_subtype: effect.filter.chosen_subtype(),
            reference: None,
        };
        eval::amount_with_context(&amount, &self.state, effect.controller, context, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_floored_activation_reductions(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        object: &GameObject,
        full: &baylee_core::mana::ManaCost,
        mana: &mut baylee_core::mana::ManaCost,
    ) {
        if full.generic_total() == 0 {
            return;
        }
        for effect in self.state.effects.iter() {
            match effect.modifier {
                baylee_cards_dsl::Modifier::AbilitiesCostLess { amount, minimum }
                    if minimum > 0 && crate::effects::applies_to(&self.state, effect, object) =>
                {
                    let reduction = amount.min(mana.cmc().saturating_sub(minimum));
                    *mana = mana.with_less_generic(reduction);
                }
                baylee_cards_dsl::Modifier::ActivatedAbilitiesCostLess {
                    filter,
                    amount,
                    min_mana,
                    exhaust_only,
                } if min_mana > 0
                    && self.activated_ability_cost_reduction_matches(
                        player,
                        source,
                        object,
                        effect,
                        filter,
                        exhaust_only,
                        ability_index,
                    ) =>
                {
                    let eval_amt = self.eval_activated_cost_reduction(source, effect, amount);
                    let reduction = eval_amt.min(mana.cmc().saturating_sub(u32::from(min_mana)));
                    *mana = mana.with_less_generic(reduction);
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_activation_effect_adjustments(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        base_mana: &baylee_core::mana::ManaCost,
        full: &baylee_core::mana::ManaCost,
        equip: bool,
        cycling: bool,
        target_pricing: ActivationTargetPricing<'_>,
        mana: &mut baylee_core::mana::ManaCost,
        parts: &mut SmallVec<[CostPart; 4]>,
    ) {
        if !self.state.effects.may_change_or_forbid_abilities() {
            return;
        }
        let Some(object) = self.state.object(source) else {
            return;
        };
        // Pass 1 (CR 118.7a): floored reductions first so a local minimum never
        // caps an independent unfloored reduction applied afterwards.
        self.apply_floored_activation_reductions(player, source, ability_index, object, full, mana);
        // Pass 2: unfloored generic reductions and non-mana imposed costs.
        for effect in self.state.effects.iter() {
            match effect.modifier {
                baylee_cards_dsl::Modifier::AbilitiesCostLess { amount, minimum: 0 }
                    if full.generic_total() > 0
                        && crate::effects::applies_to(&self.state, effect, object) =>
                {
                    *mana = mana.with_less_generic(amount);
                }
                baylee_cards_dsl::Modifier::ActivatedAbilitiesCostLess {
                    filter,
                    amount,
                    min_mana: 0,
                    exhaust_only,
                } if full.generic_total() > 0
                    && self.activated_ability_cost_reduction_matches(
                        player,
                        source,
                        object,
                        effect,
                        filter,
                        exhaust_only,
                        ability_index,
                    ) =>
                {
                    let eval_amt = self.eval_activated_cost_reduction(source, effect, amount);
                    *mana = mana.with_less_generic(eval_amt);
                }
                baylee_cards_dsl::Modifier::EquipCostsLess(amount)
                    if full.generic_total() > 0
                        && crate::effects::applies_to(&self.state, effect, object)
                        && object
                            .characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                        && equip =>
                {
                    *mana = mana.with_less_generic(amount);
                }
                baylee_cards_dsl::Modifier::CyclingCostsLess(amount)
                    if cycling && full.generic_total() > 0 =>
                {
                    let mut cycling_card = object.clone();
                    cycling_card.controller = player;
                    cycling_card.base_controller = player;
                    if crate::effects::matches_filter(&self.state, effect, &cycling_card) {
                        *mana = mana.with_less_generic(amount);
                    }
                }
                baylee_cards_dsl::Modifier::AbilitiesCostSacrificePerManaSymbol {
                    color,
                    sacrifice,
                } if crate::effects::applies_to(&self.state, effect, object) => {
                    casting::extend_sacrifices_per_mana_symbol(parts, base_mana, color, sacrifice);
                }
                _ => {}
            }
        }
        // Apply unrestricted target-conditional reductions after floored
        // reductions so a `minimum` on `AbilitiesCostLess` never caps them.
        self.apply_target_conditional_activation_reductions(
            player,
            source,
            ability_index,
            object,
            full,
            equip,
            target_pricing,
            mana,
            parts,
        );
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn apply_target_conditional_activation_reductions(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        object: &GameObject,
        full: &baylee_core::mana::ManaCost,
        equip: bool,
        target_pricing: ActivationTargetPricing<'_>,
        mana: &mut baylee_core::mana::ManaCost,
        parts: &mut SmallVec<[CostPart; 4]>,
    ) {
        for effect in self.state.effects.iter() {
            let context = crate::text_changes::RuleContext {
                source: effect.source.unwrap_or(source),
                text: self.state.effect_text(effect),
                resolved_name: None,
                resolved_subtype: effect.filter.chosen_subtype(),
                reference: None,
            };
            match effect.modifier {
                baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                    who,
                    spell_filter,
                    target_filter,
                    reduction,
                    abilities: true,
                } => {
                    if reduction == baylee_core::mana::ManaCost::ZERO
                        || !self.ability_target_reduction_source_matches(
                            player,
                            source,
                            object,
                            effect,
                            who,
                            spell_filter,
                        )
                    {
                        continue;
                    }
                    let target_matches = match target_pricing {
                        ActivationTargetPricing::Chosen(chosen) => chosen.iter().any(|&tid| {
                            self.state.object(tid).is_some_and(|target| {
                                eval::matches_with_context(
                                    target_filter,
                                    &self.state,
                                    target,
                                    effect.controller,
                                    context,
                                )
                            })
                        }),
                        ActivationTargetPricing::Preview => self
                            .ability_preview_has_matching_target(
                                player,
                                source,
                                ability_index,
                                effect.controller,
                                context,
                                target_filter,
                            ),
                    };
                    if target_matches {
                        *mana = mana.with_less_mana(&reduction, false);
                    }
                }
                baylee_cards_dsl::Modifier::EquipCostsLessIfTargets {
                    who,
                    target_filter,
                    amount,
                } if equip && amount > 0 && full.generic_total() > 0 => {
                    let outer_ok = matches!(
                        effect.filter,
                        crate::effects::EffectFilter::Dsl(
                            baylee_cards_dsl::Filter::Any | baylee_cards_dsl::Filter::This
                        ) | crate::effects::EffectFilter::DslControlledBy(_, _)
                    ) || crate::effects::applies_to(&self.state, effect, object);
                    let player_ok = eval::players_for_source(
                        who,
                        &self.state,
                        effect.controller,
                        effect.source,
                    )
                    .is_some_and(|players| players.contains(&player));
                    if !outer_ok
                        || !player_ok
                        || !object
                            .characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                    {
                        continue;
                    }
                    let target_matches = match target_pricing {
                        ActivationTargetPricing::Chosen(chosen) => chosen.iter().any(|&tid| {
                            self.state.object(tid).is_some_and(|target| {
                                eval::matches_with_context(
                                    target_filter,
                                    &self.state,
                                    target,
                                    effect.controller,
                                    context,
                                )
                            })
                        }),
                        ActivationTargetPricing::Preview => self
                            .ability_preview_has_matching_target(
                                player,
                                source,
                                ability_index,
                                effect.controller,
                                context,
                                target_filter,
                            ),
                    };
                    if target_matches {
                        *mana = mana.with_less_generic(amount);
                    }
                }
                baylee_cards_dsl::Modifier::RaiseCostIfTargets {
                    who,
                    spell_filter,
                    target_filter,
                    extra_mana,
                    extra_life,
                    abilities: true,
                } => {
                    if (extra_mana == baylee_core::mana::ManaCost::ZERO && extra_life == 0)
                        || !self.ability_target_reduction_source_matches(
                            player,
                            source,
                            object,
                            effect,
                            who,
                            spell_filter,
                        )
                    {
                        continue;
                    }
                    // On preview before targets are chosen, an optional/alternative
                    // non-taxed target keeps the ability reachable at base cost;
                    // once targets are chosen (or if no non-matching target exists),
                    // the tax applies.
                    let target_matches = match target_pricing {
                        ActivationTargetPricing::Chosen(chosen) => chosen.iter().any(|&tid| {
                            self.state.object(tid).is_some_and(|target| {
                                eval::matches_with_context(
                                    target_filter,
                                    &self.state,
                                    target,
                                    effect.controller,
                                    context,
                                )
                            })
                        }),
                        ActivationTargetPricing::Preview => {
                            let inv = baylee_cards_dsl::Filter::Any;
                            let _ = inv;
                            self.ability_preview_all_targets_match(
                                player,
                                source,
                                ability_index,
                                effect.controller,
                                context,
                                target_filter,
                            )
                        }
                    };
                    if target_matches {
                        if extra_mana != baylee_core::mana::ManaCost::ZERO {
                            *mana = mana.combine(&extra_mana);
                        }
                        if extra_life > 0 {
                            parts.push(CostPart::PayLife(
                                u16::try_from(extra_life).unwrap_or(u16::MAX),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn ability_preview_all_targets_match(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        controller: PlayerId,
        rule_ctx: crate::text_changes::RuleContext,
        target_filter: &'static baylee_cards_dsl::Filter,
    ) -> bool {
        let Some(ability) = self.activation_definition(source, ability_index) else {
            return false;
        };
        let (AbilityDef::Activated {
            targets: Some(req), ..
        }
        | AbilityDef::ActivatedConditional {
            targets: Some(req), ..
        }) = ability
        else {
            return false;
        };
        if req.min == 0 {
            return false;
        }
        let ab_ctx = self.ability_rule_context(source, ability_index);
        let wall_only = self.activation_wall_only(source, ability_index);
        let pls = eval::target_player_options_from(&self.state, &req.spec, player, ab_ctx);
        if !pls.is_empty() {
            return false;
        }
        let objs = eval::target_options_in_domain(
            &req.spec,
            &self.state,
            player,
            ab_ctx,
            self.activation_x,
            wall_only,
        );
        !objs.is_empty()
            && objs.iter().all(|&id| {
                self.state.object(id).is_some_and(|target| {
                    eval::matches_with_context(
                        target_filter,
                        &self.state,
                        target,
                        controller,
                        rule_ctx,
                    )
                })
            })
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
            CostPart::RevealChosenPlayer => {
                object.zone == crate::zone::Zone::Battlefield
                    && object.has_unrevealed_secret_chosen_player()
            }
            CostPart::TapSelf | CostPart::UntapSelf => {
                !crate::combat::summoning_sick(&self.state, object)
                    && object.status.contains(Status::TAPPED) == matches!(part, CostPart::UntapSelf)
            }
            _ => unreachable!("source action cost"),
        })
    }

    pub(super) fn source_cost_parts_ready(&self, source: ObjectId, parts: &[CostPart]) -> bool {
        parts.iter().all(|part| match part {
            CostPart::TapSelf
            | CostPart::UntapSelf
            | CostPart::ExertSelf
            | CostPart::RevealChosenPlayer => self.can_pay_source_action(source, part),
            CostPart::SacrificeSelf => self.state.can_be_sacrificed(source, None),
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
            CostPart::RevealSelfFromHand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand),
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
        self.can_afford_parts(player, source, cost.mana, cost.parts, what)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn can_afford_parts(
        &self,
        player: PlayerId,
        source: ObjectId,
        mana: baylee_core::mana::ManaCost,
        parts: &[CostPart],
        what: casting::SpendFor,
    ) -> bool {
        if let Some((idx, CostPart::Either(a, b))) = parts
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p, CostPart::Either(_, _)))
        {
            let x = self.activation_x.unwrap_or(0);
            let branch = |sub: &Cost| {
                let next_parts: SmallVec<[CostPart; 4]> = parts[..idx]
                    .iter()
                    .chain(sub.parts)
                    .chain(&parts[idx + 1..])
                    .copied()
                    .collect();
                self.can_afford_parts(
                    player,
                    source,
                    mana.combine(&sub.mana.with_x(x)),
                    &next_parts,
                    what,
                )
            };
            return branch(a) || branch(b);
        }
        if !self.source_cost_parts_ready(source, parts) {
            return false;
        }
        let x = self.activation_x.unwrap_or(0);
        let wb_total = waterbend_mana(parts, x);
        let wb_taps = if wb_total > 0 {
            waterbend_tap_capacity(&self.state, player, source, parts).min(wb_total)
        } else {
            0
        };
        let effective_mana = mana.with_less_generic(wb_taps);
        if !self.can_pay_mana(player, what, &effective_mana) {
            return false;
        }
        if !cost_wizard::counter_remove::can_afford_counter_parts(
            &self.state,
            player,
            eval::live_context(&self.state, source),
            parts,
            self.activation_x.unwrap_or(0),
            &[],
        ) {
            return false;
        }
        for part in parts {
            match part {
                CostPart::PayEnergy(_) => {
                    if u32::from(self.state.players[usize::from(player.get())].energy)
                        < energy_payment(parts, 0)
                    {
                        return false;
                    }
                }
                CostPart::DiscardRandom => {
                    if !self.can_pay_random_discards(player, parts) {
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
                | CostPart::DiscardX(_)
                | CostPart::RevealFromHandX(_)
                | CostPart::ExileFromGraveyardX(_)
                | CostPart::TapN { .. }
                | CostPart::ExileFromSingleGraveyard { .. }
                | CostPart::CraftMaterials { .. } => {
                    let count = match part {
                        CostPart::TapN { count, .. } => *count,
                        CostPart::ExileFromSingleGraveyard { count, .. } => u32::from(*count),
                        CostPart::CraftMaterials { min, .. } => u32::from(*min),
                        _ => self.activation_x.unwrap_or(0),
                    };
                    let count = usize::try_from(count).unwrap_or(usize::MAX);
                    if cost_wizard::menu(&self.state, player, source, parts, part).len() < count {
                        return false;
                    }
                }
                CostPart::Forage => {
                    let menu = cost_wizard::menu(&self.state, player, source, parts, part);
                    let foods = menu
                        .iter()
                        .filter(|&&id| {
                            self.state
                                .object(id)
                                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                        })
                        .count();
                    let gy = menu.len().saturating_sub(foods);
                    let needed = parts
                        .iter()
                        .filter(|other| matches!(other, CostPart::Forage))
                        .count();
                    if foods.saturating_add(gy / 3) < needed {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if !self.state.can_pay_life(player, i32::from(*n)) {
                        return false;
                    }
                }
                CostPart::ExileTopOfLibrary(_) => {
                    let needed: usize = parts
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
                    let menu = cost_wizard::menu(&self.state, player, source, parts, part);
                    if !cost_wizard::PowerPayment::new(&self.state, source, part)
                        .can_reach(&menu, *power)
                    {
                        return false;
                    }
                }
                CostPart::CollectEvidence(_) => {
                    let needed: u32 = parts
                        .iter()
                        .filter_map(|other| match other {
                            CostPart::CollectEvidence(mv) => Some(u32::from(*mv)),
                            _ => None,
                        })
                        .sum();
                    if needed > 0 {
                        let menu = cost_wizard::menu(&self.state, player, source, parts, part);
                        let avail: u32 = menu
                            .iter()
                            .filter_map(|&id| self.state.object(id))
                            .fold(0u32, |acc, o| {
                                acc.saturating_add(o.characteristics().mana_value())
                            });
                        if avail < needed {
                            return false;
                        }
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
                | CostPart::ReturnUnblockedAttacker
                | CostPart::ExileFromGraveyard(_)
                | CostPart::PutCounterOnOther { .. }
                | CostPart::Behold(_)
                | CostPart::BeholdAndExile(_)
                | CostPart::RevealFromHand(_) => {
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
                    let asked = parts.iter().filter(|other| *other == part).count();
                    if cost_wizard::menu(&self.state, player, source, parts, part).len() < asked {
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
                | CostPart::RevealChosenPlayer
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::RemoveAnyCounter { .. }
                | CostPart::RemoveAnyCounterX { .. }
                | CostPart::DiscardHand
                | CostPart::SacrificeAll(_)
                | CostPart::SacrificeSelf
                | CostPart::SacrificeGrantor
                | CostPart::DiscardSelf
                | CostPart::RevealSelfFromHand
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::PayLifeX
                | CostPart::PayEnergyX
                | CostPart::DealDamageToYou(_)
                | CostPart::Either(_, _)
                | CostPart::Waterbend(_)
                | CostPart::WaterbendX
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
    /// A refusal pays nothing. CR 734.1 reverses an action that cannot be
    /// completed and cancels "any payments already made", and a part that
    /// refuses after another was paid (a move of an object that is gone, a
    /// counter that is not there) used to leave the mana spent and the
    /// source tapped under an activation that never happened. The game is
    /// kept before the first part and put back on a refusal, whenever a part
    /// of the cost can refuse part way ([`can_refuse_part_way`]).
    ///
    /// The other half of 734.1, reversing the mana abilities activated while
    /// making the play, has nothing to undo here: this engine pays out of the
    /// pool, and the mana abilities that filled it were activated before the
    /// play began, so their mana stays in the pool, as it would.
    #[cfg(any(test, feature = "test-support"))]
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
            &ActivationPrice::new(cost, allowance),
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
            cost.clone(),
            chosen.to_vec(),
            Vec::new(),
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
        let cost = work.cost.clone();
        let x = work.x;
        let mut answers = work.chosen[work.answer..].iter();
        let mut counter_removals = work.counter_removals[work.counter_cursor..].iter();
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
                    self.state.record_tap_event(source, true, Cause::Cost);
                }
                CostPart::ExertSelf => {
                    crate::resolve::wave4_combat::exert_permanent(
                        &mut self.state,
                        source,
                        player,
                        Cause::Cost,
                    );
                }
                CostPart::RevealChosenPlayer => {
                    if self
                        .state
                        .object_mut(source)
                        .and_then(crate::object::GameObject::reveal_secret_chosen_player)
                        .is_none()
                    {
                        return Err(EngineError::IllegalAction(
                            "source has no unrevealed chosen player to reveal",
                        ));
                    }
                    self.state.invalidate_projections();
                }
                CostPart::UntapSelf => {
                    if self.state.set_tapped(source, false) {
                        self.state.record_tap_event(source, false, Cause::Cost);
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
                    if let Some(reference) = self.state.source_identity(source) {
                        paid.discarded.push(reference);
                    }
                    self.state.discard_card(source, player, Cause::Cost, None);
                }
                CostPart::RevealSelfFromHand => {
                    let Some(obj) = self
                        .state
                        .object_mut(source)
                        .filter(|o| o.zone == Zone::Hand && o.zone_owner == Some(player))
                    else {
                        return Err(EngineError::IllegalAction(
                            "source card is no longer in your hand",
                        ));
                    };
                    if !obj
                        .riders
                        .contains(&crate::object::Rider::RevealedFromOpeningHand)
                    {
                        obj.riders
                            .push(crate::object::Rider::RevealedFromOpeningHand);
                    }
                    self.state
                        .journal
                        .record(crate::event::GameEvent::Revealed {
                            player,
                            cards: vec![source],
                        });
                }
                CostPart::SacrificeSelf => {
                    self.state.sacrifice_object(source, Cause::Cost)?;
                }
                CostPart::SacrificeGrantor => {
                    let grantor = cost.grantor.ok_or(EngineError::IllegalAction(
                        "a grantor sacrifice requires a granted ability",
                    ))?;
                    let lki = self
                        .state
                        .object(grantor.object)
                        .filter(|object| {
                            object.version == grantor.version
                                && object.zone == Zone::Battlefield
                                && object.controller == player
                        })
                        .map(crate::object::SacrificedLki::of)
                        .ok_or(EngineError::IllegalAction(
                            "the granting permanent cannot be sacrificed",
                        ))?;
                    paid.sacrificed = Some((grantor.object, grantor.version));
                    paid.sacrificed_lki = Some(lki);
                    self.state.sacrifice_object(grantor.object, Cause::Cost)?;
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
                CostPart::SacrificeX(_)
                | CostPart::TapX(_)
                | CostPart::DiscardX(_)
                | CostPart::RevealFromHandX(_)
                | CostPart::ExileFromGraveyardX(_) => {
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
                        if matches!(part, CostPart::DiscardX(_))
                            && let Some(reference) = self.state.source_identity(card)
                        {
                            paid.discarded.push(reference);
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
                CostPart::DealDamageToYou(n) => {
                    if *n > 0 {
                        let assignment = crate::damage::Assignment {
                            source,
                            source_version: self.state.object(source).map(|o| o.version),
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
                    let Some(selected) = answers.next() else {
                        return Err(EngineError::IllegalAction("unanswered forage cost"));
                    };
                    match *selected.as_slice() {
                        [food]
                            if self.state.object(food).is_some_and(|o| {
                                o.zone == crate::zone::Zone::Battlefield
                                    && o.controller == player
                                    && o.characteristics()
                                        .subtypes
                                        .contains(baylee_core::generated::subtypes::artifact::FOOD)
                            }) =>
                        {
                            if work.next <= cost.parts.len().saturating_sub(cost.imposed) {
                                paid.sacrificed =
                                    self.state.object(food).map(|o| (food, o.version));
                                paid.sacrificed_lki = self
                                    .state
                                    .object(food)
                                    .map(crate::object::SacrificedLki::of);
                            }
                            self.state.sacrifice_object(food, Cause::Cost)?;
                        }
                        [a, b, c]
                            if a != b
                                && b != c
                                && a != c
                                && [a, b, c].into_iter().all(|id| {
                                    self.state.object(id).is_some_and(|o| {
                                        o.zone == crate::zone::Zone::Graveyard
                                            && o.zone_owner == Some(player)
                                    })
                                }) =>
                        {
                            for id in [a, b, c] {
                                cost_wizard::pay(&mut self.state, player, part, id)?;
                            }
                        }
                        _ => {
                            return Err(EngineError::IllegalAction("invalid forage payment"));
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
                CostPart::ExileFromHand(_)
                | CostPart::PayLifeX
                | CostPart::PayAttachedManaCost
                | CostPart::Either(_, _) => {}
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
                CostPart::RemoveAnyCounter { .. } | CostPart::RemoveAnyCounterX { .. } => {
                    let ctx = eval::live_context(&self.state, source);
                    cost_wizard::counter_remove::pay_counter_removals(
                        &mut self.state,
                        player,
                        ctx,
                        part,
                        x,
                        &mut counter_removals,
                    )?;
                }
                // Preserve one answer per asking part, including an empty
                // answer to crew/saddle 0. A following cost owns its own answer.
                CostPart::TapN { count, .. } => {
                    let selected = answers
                        .next()
                        .ok_or(EngineError::IllegalAction("unanswered tap group"))?;
                    let options = cost_wizard::menu(&self.state, player, source, &cost.parts, part);
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
                        self.state.record_tap_event(id, true, Cause::Cost);
                    }
                }
                CostPart::ExileFromSingleGraveyard { count, .. } => {
                    let selected = answers.next().ok_or(EngineError::IllegalAction(
                        "unanswered graveyard exile group",
                    ))?;
                    let options = cost_wizard::menu(&self.state, player, source, &cost.parts, part);
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
                CostPart::CraftMaterials { min, max, .. } => {
                    let selected = answers
                        .next()
                        .ok_or(EngineError::IllegalAction("unanswered craft material cost"))?;
                    let options = cost_wizard::menu(&self.state, player, source, &cost.parts, part);
                    if selected.len() < usize::from(*min)
                        || selected.len() > usize::from(*max)
                        || selected
                            .iter()
                            .enumerate()
                            .any(|(i, id)| selected[..i].contains(id) || !options.contains(id))
                    {
                        return Err(EngineError::IllegalAction("invalid craft material group"));
                    }
                    for &id in selected {
                        if let Some(obj) = self.state.object_mut(id)
                            && obj.is_card()
                        {
                            obj.kind = crate::object::ObjectKind::Card;
                        }
                        cost_wizard::pay(&mut self.state, player, part, id)?;
                        if let Some(exiled) = self
                            .state
                            .object(id)
                            .filter(|o| o.zone == crate::zone::Zone::Exile && o.is_card())
                        {
                            paid.crafted_materials
                                .push(baylee_core::ids::DamageSourceRef {
                                    object: id,
                                    version: exiled.version,
                                });
                        }
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
                CostPart::Waterbend(_) | CostPart::WaterbendX => {
                    let Some(taps) = answers.next() else {
                        return Err(EngineError::IllegalAction("unanswered waterbend cost"));
                    };
                    for &perm in taps {
                        cost_wizard::pay(&mut self.state, player, part, perm)?;
                    }
                }
                CostPart::CollectEvidence(mv) => {
                    let Some(cards) = answers.next() else {
                        return Err(EngineError::IllegalAction(
                            "unanswered collect evidence cost",
                        ));
                    };
                    let options = cost_wizard::menu(&self.state, player, source, &cost.parts, part);
                    if cards
                        .iter()
                        .enumerate()
                        .any(|(i, id)| cards[..i].contains(id) || !options.contains(id))
                    {
                        return Err(EngineError::IllegalAction("invalid collect evidence cards"));
                    }
                    let total: u32 = cards
                        .iter()
                        .filter_map(|&id| self.state.object(id))
                        .fold(0u32, |acc, o| {
                            acc.saturating_add(o.characteristics().mana_value())
                        });
                    if total < u32::from(*mv) {
                        return Err(EngineError::IllegalAction(
                            "insufficient mana value for collect evidence",
                        ));
                    }
                    for &card in cards {
                        cost_wizard::pay(&mut self.state, player, part, card)?;
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
                | CostPart::ReturnUnblockedAttacker
                | CostPart::ExileFromGraveyard(_)
                | CostPart::PutCounterOnOther { .. }
                | CostPart::Behold(_)
                | CostPart::BeholdAndExile(_)
                | CostPart::RevealFromHand(_) => {
                    let Some(&[card]) = answers.next().map(Vec::as_slice) else {
                        return Err(EngineError::IllegalAction(
                            "a cost that has to ask reached the payer unanswered",
                        ));
                    };
                    if matches!(part, CostPart::ReturnUnblockedAttacker) {
                        if self.state.combat.is_blocked(card) {
                            return Err(EngineError::IllegalAction(
                                "cannot return a blocked creature for ninjutsu",
                            ));
                        }
                        let defender = self.state.combat.defender_of(card).ok_or(
                            EngineError::IllegalAction("chosen creature is no longer attacking"),
                        )?;
                        let target_ver = match defender {
                            baylee_core::ids::Defender::Player(_) => None,
                            baylee_core::ids::Defender::Planeswalker(id)
                            | baylee_core::ids::Defender::Battle(id) => {
                                self.state.object(id).map(|o| o.version)
                            }
                        };
                        paid.ninjutsu_defender = Some((defender, target_ver));
                    }
                    // "The sacrificed creature's mana value" (Birthing Pod)
                    // is read off the permanent as it last existed on the
                    // battlefield (CR 608.2h), so before it goes. Only the
                    // printed cost's: that sentence is linked to it, and a
                    // Swamp a rule imposed is none of it (CR 607.2i).
                    if work.next <= cost.parts.len().saturating_sub(cost.imposed)
                        && (matches!(
                            part,
                            CostPart::Sacrifice(_)
                                | CostPart::ExilePermanent(_)
                                | CostPart::ExileFromGraveyard(_)
                        ) || (matches!(part, CostPart::BeholdAndExile(_))
                            && self
                                .state
                                .object(card)
                                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)))
                    {
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
                work.counter_cursor = work.counter_removals.len() - counter_removals.len();
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

pub(crate) fn waterbend_mana(parts: &[CostPart], x: u32) -> u32 {
    parts
        .iter()
        .filter_map(|p| match p {
            CostPart::Waterbend(n) => Some(u32::from(*n)),
            CostPart::WaterbendX => Some(x),
            _ => None,
        })
        .fold(0_u32, u32::saturating_add)
}

pub(crate) fn waterbend_tap_capacity(
    state: &crate::state::GameState,
    player: PlayerId,
    source: ObjectId,
    parts: &[CostPart],
) -> u32 {
    let mut sources = casting::waterbend_sources(state, player);
    if parts
        .iter()
        .any(|p| matches!(p, CostPart::TapSelf | CostPart::UntapSelf))
    {
        sources.retain(|&id| id != source);
    }
    u32::try_from(sources.len()).unwrap_or(u32::MAX)
}

fn grant_reads_grantor(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut reads = false;
    baylee_cards_dsl::Effect::walk(effects, &mut 0, &mut |effect| {
        reads |= matches!(
            effect,
            baylee_cards_dsl::Effect::GrantorDealsDamage { .. }
                | baylee_cards_dsl::Effect::RelocateAttachment {
                    attachment: baylee_cards_dsl::AttachmentSource::GrantedSource,
                    ..
                }
        );
    });
    reads
}
