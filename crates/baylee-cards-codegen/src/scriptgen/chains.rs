//! Reading an effect and the chain of sub-abilities it hands on to, with
//! the "unless" a player may pay.

use baylee_core::types::SubtypeKind;

use super::{
    BlockRole, CLUE_TOKEN_SCRIPT, Chain, Params, Price, Tx, Unless, asks_for_an_object,
    counter_kind, generic_mana, is_supported_api, keyword_const,
};

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct LineFlags {
    pub(super) targets_here: bool,
    pub(super) targets_a_player: bool,
    pub(super) single_target: bool,
    pub(super) prior_target_used: bool,
}

impl Tx<'_> {
    /// One effect and everything its `SubAbility$` chain adds.
    #[allow(clippy::too_many_lines)]
    pub(super) fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an ability spec with no `$` in it".to_string());
        };
        let starts_whenever = api == "ImmediateTrigger"
            && p.peek("TriggerDescription")
                .or_else(|| p.peek("SpellDescription"))
                .is_some_and(|d| d.trim_start().starts_with("Whenever "));
        let graveyard_description =
            if p.peek("Origin") == Some("Graveyard") || p.peek("TgtZone") == Some("Graveyard") {
                p.take("SpellDescription")
            } else {
                None
            };
        p.drop_prose();
        chain.could_add_mana |= api == "Mana";
        chain.moves_library |= matches!(
            api.as_str(),
            "Draw" | "Mill" | "Surveil" | "DigUntil" | "PeekAndReveal" | "Clash"
        ) || (api == "Play" && p.peek("ValidZone") == Some("Library"))
            || (matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                && (p
                    .peek("Origin")
                    .is_some_and(|zones| zones.split(',').any(|zone| zone == "Library"))
                    != (p.peek("Destination") == Some("Library"))));
        let had_second_target = chain.second_target.is_some();
        if api == "Effect"
            && chain.target.is_none()
            && p.has("ValidTgts")
            && p.peek("ReplacementEffects").is_some()
            && !p.has("UnlessCost")
            && !p.has("Condition")
            && !p.has("ConditionCheckSVar")
            && !p.has("ConditionPresent")
            && !p.has("AtEOT")
            && !p.has("ReplaceDyingDefined")
        {
            let mut probe = p.clone();
            let shield_sub = probe.take("SubAbility");
            let saved_target = chain.target.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) = self.prevention_effect_shield(probe, chain) {
                Self::push_effects_merging_kicked(&mut chain.effects, effects);
                return self.chain_sub_ability(shield_sub, chain);
            }
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }
        if api == "PutCounter"
            && p.has("DividedAsYouChoose")
            && chain.effects.is_empty()
            && chain.target.is_none()
            && chain.second_target.is_none()
            && !p.has("UnlessCost")
            && !p.has("Condition")
            && !p.has("ConditionCheckSVar")
            && !p.has("ConditionPresent")
            && !p.has("ConditionDefined")
            && !p.has("AtEOT")
            && !p.has("ReplaceDyingDefined")
        {
            return self.wave24a_divided_counters_chain(p, chain);
        }
        if api == "DealDamage"
            && p.has("ExcessSVar")
            && chain.target.is_none()
            && chain.second_target.is_none()
            && !p.has("DamageSource")
            && !p.has("DividedAsYouChoose")
            && !p.has("UnlessCost")
            && !p.has("Condition")
            && !p.has("ConditionCheckSVar")
            && !p.has("ConditionPresent")
            && !p.has("ConditionDefined")
            && !p.has("AtEOT")
            && !p.has("ReplaceDyingDefined")
        {
            let saved_effects_len = chain.effects.len();
            let saved_target = chain.target.clone();
            let saved_up_to = chain.target_up_to;
            let saved_exact = chain.target_exact;
            let saved_is_x = chain.target_is_x;
            let saved_second = chain.second_target.clone();
            let saved_second_up_to = chain.second_target_up_to;
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if self.excess_damage_chain(spec, chain).is_some() {
                return Some(());
            }
            let first_err = self.unclaimed.borrow().clone();
            chain.effects.truncate(saved_effects_len);
            chain.target = saved_target;
            chain.target_up_to = saved_up_to;
            chain.target_exact = saved_exact;
            chain.target_is_x = saved_is_x;
            chain.second_target = saved_second;
            chain.second_target_up_to = saved_second_up_to;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            if self
                .wave24a_single_excess_damage_chain(spec, chain)
                .is_some()
            {
                return Some(());
            }
            if self.unclaimed.borrow().is_none() {
                if let Some(err) = first_err {
                    *self.unclaimed.borrow_mut() = Some(err);
                } else {
                    self.note("unclaimed parameter `DealDamage.ExcessSVar`".to_string());
                }
            }
            return None;
        }
        let mut local_kicked_target: Option<String> = None;
        let (targets_here, mut sub) =
            if let Some(child_sub) = self.try_kicked_target_replacement(&api, &mut p, chain) {
                (true, child_sub)
            } else if let Some(kicked_target) = self.try_kicked_only_target(&api, &mut p, chain) {
                local_kicked_target = Some(kicked_target);
                (true, p.take("SubAbility"))
            } else {
                p.claim_label("ValidTgtsDesc", "ValidTgts");
                let valid = p.take("ValidTgts");
                let targets_here = valid.is_some();
                if let Some(valid) = valid {
                    // CR 702.33g: a target of a part that happens only if the
                    // spell was kicked is chosen only then. A target line
                    // gated on kicking that no rule above turned into
                    // `kicked_targets` would be demanded on every cast.
                    if self.line_is_gated_on_kicking(&p) {
                        return self.deny(if chain.target.is_some() {
                            "a kicked-only second target".to_string()
                        } else if p.peek("Condition") == Some("Kicked") {
                            "an unsupported kicked-only target".to_string()
                        } else {
                            "a target behind a kicked condition read through \
                             `ConditionCheckSVar$` or `ConditionPresent$`"
                                .to_string()
                        });
                    }
                    self.chain_target(
                        &api,
                        &mut p,
                        &valid,
                        chain,
                        graveyard_description.as_deref(),
                    )?;
                }
                (targets_here, p.take("SubAbility"))
            };
        let second_target_here = !had_second_target && chain.second_target.is_some();
        if api == "DealDamage"
            && self.prints_simultaneous_damage_to_controller()
            && let Some((effects, next)) =
                self.damage_pair(p.clone(), sub.as_deref(), chain, targets_here)
        {
            chain.effects.extend(effects);
            return self.chain_sub_ability(next, chain);
        }
        // The chosen-source shield is three lines in the reference and one
        // sentence on the card: its `SubAbility$` is part of the sentence,
        // not the next one, so this rule reads the rest of the chain itself.
        if api == "ChooseSource" {
            let saved_p = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            let effects = if let Some(effects) = self.prevent_from_chosen_source(p, sub.as_deref())
            {
                effects
            } else {
                let sub_has_targets = sub
                    .as_deref()
                    .and_then(|s| self.svars.get(s))
                    .is_some_and(|body| body.contains("ValidTgts$"));
                if !sub_has_targets {
                    return None;
                }
                let first_err = self.unclaimed.borrow().clone();
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                if let Some(effects) =
                    self.prevent_from_chosen_source_chained(saved_p, sub.as_deref(), chain)
                {
                    effects
                } else {
                    if first_err.is_some() {
                        *self.unclaimed.borrow_mut() = first_err;
                    }
                    return None;
                }
            };
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "ChooseColor" {
            return self.choose_color_chain(p, sub.as_deref(), chain);
        }
        if api == "ChooseType" {
            return self.choose_type_chain(p, sub.as_deref(), chain);
        }
        if api == "ChoosePlayer" {
            let saved_p = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) = self.choose_player_effect(&mut p, sub.as_deref()) {
                chain.effects.extend(effects);
                return Some(());
            }
            p = saved_p;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            let effects = self.choose_player_chain(&mut p, &mut sub, targets_here, chain)?;
            chain.effects.extend(effects);
            return self.chain_sub_ability(sub, chain);
        }
        if api == "GenericChoice" {
            let saved_p = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) =
                self.guess_named_card_effect(p, sub.as_deref(), chain.target.as_deref())
            {
                chain.effects.extend(effects);
                return Some(());
            }
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            let effects = self.choose_one_effect(saved_p, chain.target.as_deref())?;
            chain.effects.extend(effects);
            return self.chain_sub_ability(sub, chain);
        }
        if api == "RepeatEach" {
            let saved_p = p.clone();
            let saved_effects_len = chain.effects.len();
            let saved_target = chain.target.clone();
            let saved_second = chain.second_target.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if self.repeat_each_chain(p, sub.as_deref(), chain).is_some() {
                return Some(());
            }
            let first_err = self.unclaimed.borrow().clone();
            chain.effects.truncate(saved_effects_len);
            chain.target = saved_target;
            chain.second_target = saved_second;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            if self
                .wave23a_repeat_each_object_chain(saved_p, sub.as_deref(), chain)
                .is_some()
            {
                return Some(());
            }
            if self.unclaimed.borrow().is_none() && first_err.is_some() {
                *self.unclaimed.borrow_mut() = first_err;
            }
            return None;
        }
        if api == "Repeat" {
            return self.repeat_process_chain(p, sub.as_deref(), chain);
        }
        if api == "Vote" {
            return self.vote_chain(p, sub.as_deref(), chain);
        }
        if matches!(api.as_str(), "ChooseCard" | "PeekAndReveal" | "Reveal") {
            let target = local_kicked_target.or_else(|| {
                if had_second_target {
                    None
                } else {
                    Self::effect_target(chain)
                }
            });
            let targets_a_player =
                targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
            return match api.as_str() {
                "ChooseCard" => self.choose_card_chain(
                    p,
                    sub.as_deref(),
                    target.as_deref(),
                    targets_a_player,
                    chain,
                ),
                "PeekAndReveal" if self.sub_is_api(sub.as_deref(), "TwoPiles") => self
                    .peek_and_reveal_two_piles_chain(
                        p,
                        sub.as_deref(),
                        targets_here,
                        targets_a_player,
                        chain,
                    ),
                "PeekAndReveal" => self.peek_and_reveal_chain(
                    p,
                    sub.as_deref(),
                    target.as_deref(),
                    targets_a_player,
                    chain,
                ),
                "Reveal" => self.reveal_from_hand_chain(
                    p,
                    sub.as_deref(),
                    target.as_deref(),
                    targets_a_player,
                    chain,
                ),
                _ => unreachable!(),
            };
        }
        // The remembered objects are the damage recipients, not this line's
        // original targets. Read their entire lifetime and consumer together;
        // accepting the flag alone would silently drop a rules clause.
        let prevent_damaged_regeneration = self.take_damage_recipient_regeneration(
            &api,
            &mut p,
            &mut sub,
            chain.effects.is_empty(),
        )?;
        let target = local_kicked_target.or_else(|| {
            if had_second_target {
                None
            } else {
                Self::effect_target(chain)
            }
        });
        // "Put a charge counter on this artifact. If you do, it deals damage
        // to any target": the counter line comes first and names the source,
        // the target is declared by a later sub-ability and belongs to the
        // whole ability (CR 603.3d). `AddCounter` would read that target as
        // the counter's recipient, so the line reads as one that targets,
        // which makes the `PutCounter` arm name the source explicitly.
        self.counter_on_source = target.is_none()
            && api == "PutCounter"
            && matches!(p.peek("Defined"), None | Some("Self"))
            && self.later_line_targets(sub.as_deref());
        // Whether an absent `Defined$` on this line means a chosen player:
        // only where this line declared the player target itself.
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        // "Sacrifice it unless you pay {U}", "counter target spell unless
        // its controller pays {2}": the price is the line's and not its
        // effect's, so it is read here for every API and wraps whatever the
        // line turns out to say.
        let unless = match p.take("UnlessCost") {
            Some(cost) => Some(self.unless(&cost, &mut p, target.as_deref(), sub.is_some())?),
            None => None,
        };
        let optional_counter = if api == "Counter" {
            match p.take("Optional").as_deref() {
                None => false,
                Some("True") => true,
                Some(other) => {
                    return self.deny(format!("`Counter` with `Optional$ {other}`"));
                }
            }
        } else {
            false
        };
        // "If that creature would die this turn, exile it instead": a rider
        // on whatever the line does to its target, read for every API.
        let dying = match p.take("ReplaceDyingDefined") {
            Some(defined) => Some(self.exile_if_dies(&defined, target.as_deref())?),
            None => None,
        };
        let at_end = if api == "CopyPermanent"
            || (api == "Token"
                && p.peek("TokenAttacking") == Some("True")
                && p.peek("AtEOT") == Some("Sacrifice"))
        {
            None
        } else {
            match p.take("AtEOT") {
                Some(what) => Some(self.at_next_end_step(&api, &what, target.as_deref())?),
                None => None,
            }
        };

        let effects = if api != "Branch"
            && let Some(split) = self
                .try_split_kicked_amount(
                    &api,
                    &mut p,
                    target.as_deref(),
                    targets_here,
                    targets_a_player,
                    chain,
                )
                .ok()?
        {
            split
        } else if api != "Branch"
            && let Some(split) = self
                .try_split_bargained_or_prototyped_amount(
                    &api,
                    &mut p,
                    target.as_deref(),
                    targets_here,
                    targets_a_player,
                    chain,
                )
                .ok()?
        {
            split
        } else if api == "Branch" {
            let single_target = chain.target_exact.is_none()
                && !chain.target_is_x
                && chain.target_up_to.is_none_or(|n| n == 1);
            let Some(effects) =
                self.wave24a_branch_effect(&mut p, target.as_deref(), single_target, chain)
            else {
                if self.unclaimed.borrow().is_none() {
                    self.note("effect `Branch`".to_string());
                }
                return None;
            };
            effects
        } else if api == "ChangeZone"
            && matches!(p.peek("Origin"), Some("Battlefield" | "Graveyard"))
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "Effect")
        {
            self.exile_with_permission_effect(&mut p, &mut sub, target.as_deref(), targets_here)?
        } else if api == "ChangeZone"
            && p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "Effect")
        {
            self.exile_chosen_from_hand_and_cast_effect(&mut p, &mut sub, targets_here)?
        } else if api == "ChangeZone"
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && (p.has("RememberChanged") || p.has("RememberTargets"))
            && (self.sub_is_api(sub.as_deref(), "ChangeZone")
                || self.sub_is_api(sub.as_deref(), "DelayedTrigger"))
            && (targets_here
                || (self.sub_is_api(sub.as_deref(), "ChangeZone")
                    && (matches!(p.peek("Defined"), Some("Targeted" | "ParentTarget"))
                        || (target.is_none()
                            && p.peek("Defined")
                                .is_some_and(|d| self.is_event_object_defined(d))))))
        {
            let single_target = chain.target_exact.is_none()
                && !chain.target_is_x
                && chain.target_up_to.is_none_or(|n| n == 1);
            self.exile_until_end_effect(
                &mut p,
                &mut sub,
                target.as_deref(),
                targets_here,
                single_target,
            )?
        } else if api == "PutCounter" && p.peek("Support").is_some() {
            if chain.target.is_some()
                || targets_here
                || had_second_target
                || p.peek("Defined").is_some()
            {
                return self.deny("`Support$` beside an existing target or `Defined$`".to_string());
            }
            let Some(n) = p
                .take("Support")
                .and_then(|s| s.trim().parse::<u8>().ok())
                .filter(|&n| n > 0)
            else {
                return self.deny("unreadable value in `PutCounter`".to_string());
            };
            let spec = if self.on_a_spell {
                "TargetSpec::Object(&Filter::CREATURE)".to_string()
            } else {
                let name = self.body.filter_static(
                    "TARGET",
                    "Filter::And(&[Filter::CREATURE, Filter::Another])",
                );
                format!("TargetSpec::Object(&{name})")
            };
            chain.target = Some(spec);
            chain.target_up_to = Some(n);
            vec![
                "Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }"
                    .to_string(),
            ]
        } else if api == "Earthbend" {
            if chain.target.is_some()
                || targets_here
                || had_second_target
                || p.has("Defined")
                || unless.is_some()
                || p.has("Condition")
                || p.has("ConditionPresent")
                || p.has("ConditionCheckSVar")
                || p.has("ConditionDefined")
            {
                return self.deny(
                    "`Earthbend` beside an existing target, `Defined$`, or condition".to_string(),
                );
            }
            let Some(n) = p
                .take("Num")
                .and_then(|s| s.trim().parse::<u16>().ok())
                .filter(|&n| n > 0)
            else {
                return self.deny("unreadable value in `Earthbend`".to_string());
            };
            chain.target = Some("TargetSpec::Object(&Filter::YOUR_LAND)".to_string());
            vec![format!("Effect::Earthbend({n})")]
        } else if api == "ChangeZone"
            && targets_here
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && (p.peek("RememberLKI") == Some("True") || p.peek("RememberChanged") == Some("True"))
            && self.sub_is_api(sub.as_deref(), "RepeatEach")
            && unless.is_none()
            && dying.is_none()
            && at_end.is_none()
            && !optional_counter
            && !self.has_linked_exile_reader()
        {
            self.exile_targets_create_tokens_effect(&mut p, &mut sub, target.as_deref())?
        } else if api == "ChangeZone"
            && !targets_here
            && p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Library")
            && p.peek("LibraryPosition") == Some("-1")
            && p.peek("RememberChanged") == Some("True")
            && sub.is_some()
            && unless.is_none()
            && dying.is_none()
            && at_end.is_none()
            && !optional_counter
        {
            self.hand_to_bottom_remembered_chain(&mut p, &mut sub, chain)?
        } else if api == "ChangeZone"
            && !targets_here
            && !self.on_a_spell
            && matches!(p.peek("Defined"), None | Some("Self"))
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "ChangeZone")
        {
            self.exile_self_return_as_face_effect(&mut p, &mut sub)?
        } else if api == "ChangeZone"
            && !targets_here
            && p.peek("Origin") == Some("Library")
            && p.peek("Destination") == Some("Battlefield")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "Untap")
        {
            self.search_library_then_untap_found(&mut p, &mut sub, target.as_deref())?
        } else if api == "ChangeZone"
            && !targets_here
            && p.peek("Origin") == Some("Library")
            && p.peek("Destination") == Some("Library")
            && p.peek("RememberChanged") == Some("True")
            && p.peek("Reveal") == Some("True")
            && p.peek("Shuffle") == Some("False")
            && self.sub_is_api(sub.as_deref(), "ChangeZone")
        {
            self.split_search_library_chain(&mut p, &mut sub, targets_here)?
        } else if api == "ChangeZone"
            && !targets_here
            && p.peek("Origin") == Some("Library")
            && p.peek("Shuffle") == Some("False")
            && !p.has("RememberChanged")
            && !p.has("Imprint")
            && matches!(p.peek("DefinedPlayer"), None | Some("You"))
            && sub.is_some()
        {
            self.search_library_before_shuffle(
                &mut p,
                sub.as_deref(),
                target.as_deref(),
                targets_here,
            )?
        } else if api == "ChangeZone"
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "AlterAttribute")
        {
            self.plot_remembered_effect(&mut p, &mut sub, target.as_deref(), targets_here)?
        } else if api == "ChangeZone"
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Battlefield")
            && p.peek("RememberChanged") == Some("True")
            && (self.sub_is_api(sub.as_deref(), "PutCounter")
                || self.sub_is_api(sub.as_deref(), "Animate")
                || sub
                    .as_deref()
                    .and_then(|s| self.svars.get(s.trim()))
                    .and_then(|b| Params::parse(b))
                    .is_some_and(|(s_api, sp)| {
                        s_api == "Pump" && sp.peek("Duration") == Some("Permanent")
                    }))
        {
            self.reanimate_remembered_modified(&mut p, &mut sub, target.as_deref(), targets_here)?
        } else if api == "ChangeZone"
            && !targets_here
            && matches!(p.peek("Defined"), None | Some("Self"))
            && p.peek("Origin") == Some("Stack")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && self.sub_is_api(sub.as_deref(), "PutCounter")
        {
            self.exile_source_with_counters_effect(&mut p, &mut sub)?
        } else if api == "Token"
            && p.peek("RememberTokens") == Some("True")
            && (self.sub_is_api(sub.as_deref(), "Attach")
                || self.sub_is_api(sub.as_deref(), "PutCounter"))
        {
            self.remembered_token_follow_up(&mut p, &mut sub, target.as_deref(), targets_a_player)?
        } else if api == "ChangeZone"
            && !targets_here
            && p.peek("Defined") == Some("TopOfLibrary")
            && p.peek("Origin") == Some("Library")
            && p.peek("Destination") == Some("Exile")
            && p.has("RememberChanged")
            && self.sub_is_api(sub.as_deref(), "Effect")
        {
            self.exile_top_to_play_effect(&mut p, &mut sub)?
        } else if api == "Dig"
            && (p.peek("ChangeNum") == Some("All")
                || (p.peek("DigNum") == Some("1")
                    && matches!(p.peek("ChangeNum"), None | Some("1"))))
            && p.peek("DestinationZone") == Some("Exile")
            && p.has("RememberChanged")
            && self.sub_is_api(sub.as_deref(), "Effect")
        {
            self.dig_exile_top_to_play_effect(
                &mut p,
                &mut sub,
                target.as_deref(),
                targets_here,
                targets_a_player,
            )?
        } else if api == "Dig"
            && p.peek("DestinationZone") == Some("Exile")
            && p.peek("ExileFaceDown") == Some("True")
            && p.peek("WithMayLook") == Some("True")
            && p.peek("RememberChanged") == Some("True")
            && matches!(p.peek("ChangeNum"), None | Some("1"))
        {
            self.dig_look_at_top_exile_face_down_effect(&mut p, &mut sub, targets_here)?
        } else if api == "TargetOnly" {
            if second_target_here && !had_second_target {
                if unless.is_some() || dying.is_some() || at_end.is_some() || optional_counter {
                    return self.deny("`TargetOnly` modifier with `SubAbility$`".to_string());
                }
                p.take("IsCurse");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `TargetOnly.{key}`"));
                    }
                    return None;
                }
                self.two_target_only_followup(&mut sub, chain)?
            } else if !targets_here || second_target_here || had_second_target {
                return self.deny("`TargetOnly` without a single first target".to_string());
            } else {
                p.take("IsCurse");
                if chain.target.as_deref().is_some_and(|t| {
                    t.starts_with("TargetSpec::Spell(")
                        || t.starts_with("TargetSpec::AbilityOnStack(")
                        || t.starts_with("TargetSpec::SpellOrAbility(")
                        || t.starts_with("TargetSpec::ActivatedAbility(")
                }) {
                    p.take("TargetType");
                }
                if sub.is_some() {
                    if unless.is_some() || dying.is_some() || at_end.is_some() || optional_counter {
                        return self.deny("`TargetOnly` modifier with `SubAbility$`".to_string());
                    }
                    if !p.exhausted() {
                        if let Some(key) = p.first_key() {
                            self.note(format!("unclaimed parameter `TargetOnly.{key}`"));
                        }
                        return None;
                    }
                    Vec::new()
                } else {
                    vec![
                        "Effect::PumpTarget { power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }"
                            .to_string(),
                    ]
                }
            }
        } else if (api == "Sacrifice"
            || (api == "SacrificeAll"
                && !targets_here
                && !self.on_a_spell
                && p.peek("Defined") == Some("Self")))
            && p.peek("RememberSacrificed") == Some("True")
            && sub.is_some()
            && !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            let Some(effects) = self.sacrifice_remembered_effect(
                &api,
                &mut p,
                &mut sub,
                target.as_deref(),
                targets_a_player,
                chain,
            ) else {
                if self.unclaimed.borrow().is_none() {
                    self.note("a sacrifice naming `RememberSacrificed`".to_string());
                }
                return None;
            };
            effects
        } else if api == "Phases"
            && targets_here
            && !self.on_a_spell
            && p.peek("WontPhaseInNormal") == Some("True")
            && self.sub_is_api(sub.as_deref(), "Effect")
        {
            self.phase_out_until_source_leaves_effect(&mut p, &mut sub)?
        } else if api == "FlipCoin" {
            self.flip_coin_effect(&mut p, &mut sub, chain)?
        } else if api == "RollDice" {
            self.roll_dice_effect(&mut p, &mut sub, chain)?
        } else if api == "Clash" {
            self.clash_effect(&mut p, targets_here, chain)?
        } else if api == "DigUntil"
            && !second_target_here
            && p.peek("RememberRevealed") == Some("True")
            && !p.has("RememberFound")
            && !p.has("ImprintFound")
            && sub.is_some()
            && unless.is_none()
            && dying.is_none()
            && at_end.is_none()
            && !optional_counter
        {
            let saved_p = p.clone();
            let saved_sub = sub.clone();
            let saved_target = chain.target.clone();
            let saved_up_to = chain.target_up_to;
            let saved_exact = chain.target_exact;
            let saved_is_x = chain.target_is_x;
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) = self.goblin_charbelcher_effect(&mut p, &mut sub, chain) {
                effects
            } else {
                p = saved_p;
                sub = saved_sub;
                chain.target = saved_target;
                chain.target_up_to = saved_up_to;
                chain.target_exact = saved_exact;
                chain.target_is_x = saved_is_x;
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                let Some(effects) = self.reveal_until_remembered_count_chain(
                    &mut p,
                    &mut sub,
                    target.as_deref(),
                    targets_a_player,
                    chain,
                ) else {
                    self.note("unreadable value in `DigUntil`".to_string());
                    return None;
                };
                effects
            }
        } else if api == "DigUntil"
            && !second_target_here
            && (p.peek("RememberFound") == Some("True") || p.peek("ImprintFound") == Some("True"))
            && sub.is_some()
            && unless.is_none()
            && dying.is_none()
            && at_end.is_none()
            && !optional_counter
        {
            let saved_p = p.clone();
            let saved_sub = sub.clone();
            let saved_target = chain.target.clone();
            let saved_up_to = chain.target_up_to;
            let saved_exact = chain.target_exact;
            let saved_is_x = chain.target_is_x;
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) = self.reveal_until_then_chain(
                &mut p,
                &mut sub,
                target.as_deref(),
                targets_a_player,
                chain,
            ) {
                effects
            } else {
                p = saved_p;
                sub = saved_sub;
                chain.target = saved_target;
                chain.target_up_to = saved_up_to;
                chain.target_exact = saved_exact;
                chain.target_is_x = saved_is_x;
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                if self.sub_is_api(sub.as_deref(), "ImmediateTrigger") {
                    self.claim_followed_remember_flag(&api, &mut p, sub.as_deref());
                    let single_target = chain.target_exact.is_none()
                        && !chain.target_is_x
                        && chain.target_up_to.is_none_or(|n| n == 1);
                    let prior_target_used = !targets_here && !chain.effects.is_empty();
                    let Some(effects) = self.effect_of_inner(
                        &api,
                        &mut p,
                        target.as_deref(),
                        LineFlags {
                            targets_here,
                            targets_a_player,
                            single_target,
                            prior_target_used,
                        },
                    ) else {
                        self.note("unreadable value in `DigUntil`".to_string());
                        return None;
                    };
                    effects
                } else {
                    self.note("unreadable value in `DigUntil`".to_string());
                    return None;
                }
            }
        } else if api == "DelayedTrigger" {
            vec![self.delayed_trigger(&mut p, target.as_deref(), targets_here, chain)?]
        } else if api == "ImmediateTrigger" {
            vec![self.immediate_trigger(&mut p, target.as_deref(), chain, starts_whenever)?]
        } else if api == "Effect"
            && !second_target_here
            && (p.peek("Triggers").is_some()
                || (p.peek("Duration") == Some("Permanent")
                    && p.peek("Name")
                        .is_some_and(|n| n.trim_start().starts_with("Emblem"))))
        {
            let snapshot = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effects) = self.effect_triggers_api(&mut p, target.as_deref(), chain) {
                effects
            } else {
                p.clone_from(&snapshot);
                self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
                self.body.statics.truncate(saved_statics_len);
                let single_target = chain.target_exact.is_none()
                    && !chain.target_is_x
                    && chain.target_up_to.is_none_or(|n| n == 1);
                let prior_target_used = !targets_here && !chain.effects.is_empty();
                let Some(effects) = self.effect_of_inner(
                    &api,
                    &mut p,
                    target.as_deref(),
                    LineFlags {
                        targets_here,
                        targets_a_player,
                        single_target,
                        prior_target_used,
                    },
                ) else {
                    self.note("unreadable value in `Effect`".to_string());
                    return None;
                };
                effects
            }
        } else {
            let single_target = chain.target_exact.is_none()
                && !chain.target_is_x
                && chain.target_up_to.is_none_or(|n| n == 1);
            let prior_target_used = !targets_here && !chain.effects.is_empty();
            let try_midnight_ritual = api == "ChangeZone"
                && targets_here
                && chain.target_is_x
                && p.peek("Origin") == Some("Graveyard")
                && p.peek("Destination") == Some("Exile")
                && !p.has("RememberChanged")
                && self.sub_is_api(sub.as_deref(), "Token")
                && unless.is_none()
                && dying.is_none()
                && at_end.is_none()
                && !optional_counter;
            let try_cz_if_moved = api == "ChangeZone"
                && ((p.peek("Origin") == Some("Battlefield")
                    && matches!(p.peek("Destination"), Some("Hand" | "Exile")))
                    || (p.peek("Origin") == Some("Graveyard")
                        && p.peek("Destination") == Some("Exile")))
                && p.peek("RememberChanged") == Some("True")
                && !self.sub_is_api(sub.as_deref(), "Effect")
                && !self.sub_is_api(sub.as_deref(), "ChangeZone")
                && !self.sub_is_api(sub.as_deref(), "DelayedTrigger")
                && !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
                && sub
                    .as_deref()
                    .and_then(|s| self.svars.get(s))
                    .is_some_and(|b| {
                        Params::parse(b).is_some_and(|(_, sp)| {
                            (sp.peek("ConditionDefined") == Some("Remembered")
                                && sp.peek("ConditionPresent").is_some())
                                || (p.peek("Origin") == Some("Graveyard")
                                    && sp
                                        .peek("ConditionCheckSVar")
                                        .and_then(|v| self.svars.get(v.trim()))
                                        .is_some_and(|expr| expr.trim() == "Remembered$Amount"))
                        })
                    });
            let try_mill_take_one = api == "Mill"
                && !targets_here
                && matches!(p.peek("Defined"), None | Some("You"))
                && p.peek("RememberMilled") == Some("True")
                && self.sub_is_api(sub.as_deref(), "ChangeZone");
            let mut special_effects: Option<Vec<String>> = None;
            if try_midnight_ritual {
                let saved_p = p.clone();
                let saved_sub = sub.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) =
                    self.midnight_ritual_exile_tokens_effect(&mut p, &mut sub, target.as_deref())
                {
                    special_effects = Some(effects);
                } else {
                    p = saved_p;
                    sub = saved_sub;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                }
            }
            if special_effects.is_none() && try_cz_if_moved {
                let saved_p = p.clone();
                let saved_sub = sub.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.change_zone_then_if_moved(
                    &mut p,
                    &mut sub,
                    target.as_deref(),
                    targets_here,
                    single_target,
                ) {
                    special_effects = Some(effects);
                } else {
                    p = saved_p;
                    sub = saved_sub;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                }
            }
            if special_effects.is_none() && try_mill_take_one {
                let saved_p = p.clone();
                let saved_sub = sub.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.mill_may_take_one_effect(&mut p, &mut sub) {
                    special_effects = Some(effects);
                } else {
                    p = saved_p;
                    sub = saved_sub;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                }
            }
            let try_remember_this_way = special_effects.is_none()
                && !second_target_here
                && sub.is_some()
                && !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
                && !self.sub_is_api(sub.as_deref(), "DelayedTrigger")
                && unless.is_none()
                && dying.is_none()
                && at_end.is_none()
                && !optional_counter
                && !p.has("Condition")
                && !p.has("ConditionPresent")
                && !p.has("ConditionCheckSVar")
                && !p.has("ConditionDefined")
                && ((matches!(api.as_str(), "Destroy" | "DestroyAll")
                    && p.peek("RememberDestroyed") == Some("True"))
                    || (api == "Mill" && p.peek("RememberMilled") == Some("True"))
                    || (api == "Discard" && p.peek("RememberDiscarded") == Some("True"))
                    || (matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                        && p.peek("RememberChanged") == Some("True"))
                    || (api == "Dig"
                        && p.peek("ChangeNum") == Some("All")
                        && p.peek("DestinationZone") == Some("Exile")
                        && p.peek("RememberChanged") == Some("True")
                        && !self.sub_is_api(sub.as_deref(), "Effect"))
                    || (api == "Dig"
                        && !targets_here
                        && p.peek("ChangeNum") == Some("All")
                        && p.peek("Reveal") == Some("True")
                        && p.peek("DestinationZone") == Some("Hand")
                        && p.peek("ChangeValid") == Some("Card")
                        && matches!(p.peek("Defined"), None | Some("You"))
                        && p.peek("RememberChanged") == Some("True")));
            let remembered_effects = if let Some(effects) = special_effects {
                Some(effects)
            } else if try_remember_this_way {
                let saved_p = p.clone();
                let saved_sub = sub.clone();
                let saved_target = chain.target.clone();
                let saved_up_to = chain.target_up_to;
                let saved_exact = chain.target_exact;
                let saved_is_x = chain.target_is_x;
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.remembered_this_way_chain(
                    &api,
                    &mut p,
                    &mut sub,
                    target.as_deref(),
                    LineFlags {
                        targets_here,
                        targets_a_player,
                        single_target,
                        prior_target_used,
                    },
                    chain,
                ) {
                    Some(effects)
                } else {
                    p = saved_p;
                    sub = saved_sub;
                    chain.target = saved_target;
                    chain.target_up_to = saved_up_to;
                    chain.target_exact = saved_exact;
                    chain.target_is_x = saved_is_x;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    None
                }
            } else {
                None
            };
            let remembered_effects = if let Some(effects) = remembered_effects {
                Some(effects)
            } else if !second_target_here
                && unless.is_none()
                && dying.is_none()
                && at_end.is_none()
                && !optional_counter
                && !p.has("Condition")
                && !p.has("ConditionPresent")
                && !p.has("ConditionCheckSVar")
                && !p.has("ConditionDefined")
                && ((api == "Mill" && p.peek("RememberMilled") == Some("True"))
                    || (matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                        && p.peek("RememberChanged") == Some("True")))
            {
                let saved_p = p.clone();
                let saved_sub = sub.clone();
                let saved_target = chain.target.clone();
                let saved_up_to = chain.target_up_to;
                let saved_exact = chain.target_exact;
                let saved_is_x = chain.target_is_x;
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.wave24a_remembered_chain(
                    &api,
                    &mut p,
                    &mut sub,
                    target.as_deref(),
                    LineFlags {
                        targets_here,
                        targets_a_player,
                        single_target,
                        prior_target_used,
                    },
                    chain,
                ) {
                    Some(effects)
                } else {
                    p = saved_p;
                    sub = saved_sub;
                    chain.target = saved_target;
                    chain.target_up_to = saved_up_to;
                    chain.target_exact = saved_exact;
                    chain.target_is_x = saved_is_x;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    None
                }
            } else {
                None
            };
            if let Some(effects) = remembered_effects {
                effects
            } else {
                self.claim_followed_remember_flag(&api, &mut p, sub.as_deref());
                let Some(effects) = (if second_target_here {
                    match api.as_str() {
                    "Pump" => {
                        self.pump_effect(&mut p, target.as_deref().expect("second target"), true)
                    }
                    "Fight"
                        if p.has("ExcessSVar")
                            && unless.is_none()
                            && dying.is_none()
                            && at_end.is_none()
                            && !optional_counter =>
                    {
                        self.wave24a_fight_excess_damage_chain(&mut p, &mut sub, chain)
                    }
                    "Fight" => self.fight_effect(&mut p, true),
                    "DealDamage"
                        if p.has("ExcessSVar")
                            && unless.is_none()
                            && dying.is_none()
                            && at_end.is_none()
                            && !optional_counter =>
                    {
                        self.wave24a_bite_excess_damage_chain(&mut p, &mut sub, chain)
                    }
                    "DealDamage" => self.bite_effect(&mut p),
                    "MoveCounter" => {
                        self.move_counter_effect(&mut p, target.as_deref(), true, true)
                    }
                    "Tap" => match p.take("Defined").as_deref() {
                        None | Some("Targeted") => {
                            Some(vec!["Effect::TapTargetSlot(TargetSlot::Second)".to_string()])
                        }
                        _ => None,
                    },
                    "Untap" => match p.take("Defined").as_deref() {
                        None | Some("Targeted") => Some(vec![
                            "Effect::UntapTargetSlot(TargetSlot::Second)".to_string(),
                        ]),
                        _ => None,
                    },
                    "Clone" => self.become_copy_effect(&mut p, target.as_deref(), true, true),
                    "ExchangeControl" => {
                        if p.take("Defined").as_deref() == Some("ParentTarget") {
                            Some(vec!["Effect::ExchangeControl".to_string()])
                        } else {
                            None
                        }
                    }
                    "Destroy" => match p.take("NoRegen").as_deref() {
                        None => Some(vec![
                            "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: false }"
                                .to_string(),
                        ]),
                        Some("True") => Some(vec![
                            "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: true }"
                                .to_string(),
                        ]),
                        Some(_) => None,
                    },
                    _ => None,
                }
                } else if api == "Fight" {
                    if targets_here
                        && target
                            .as_deref()
                            .is_some_and(|t| t.starts_with("TargetSpec::Object("))
                        && chain.target_exact.is_none()
                        && !chain.target_is_x
                        && chain.target_up_to.is_none_or(|n| n == 1)
                    {
                        self.fight_effect(&mut p, false)
                    } else {
                        None
                    }
                } else {
                    let single_target = chain.target_exact.is_none()
                        && !chain.target_is_x
                        && chain.target_up_to.is_none_or(|n| n == 1);
                    let prior_target_used = !targets_here && !chain.effects.is_empty();
                    self.effect_of_inner(
                        &api,
                        &mut p,
                        target.as_deref(),
                        LineFlags {
                            targets_here,
                            targets_a_player,
                            single_target,
                            prior_target_used,
                        },
                    )
                }) else {
                    // An API with no rule at all is a different report than a rule
                    // that met a value it cannot say — the first is a missing
                    // effect, the second is a missing case in one that exists.
                    if is_supported_api(&api) {
                        self.note(format!("unreadable value in `{api}`"));
                    } else {
                        self.note(format!("effect `{api}`"));
                    }
                    return None;
                };
                effects
            }
        };
        let mut effects =
            self.wrap_line_condition(&api, &mut p, &mut sub, target.as_deref(), effects)?;
        if prevent_damaged_regeneration {
            effects.push("Effect::DamagedThisWayCantBeRegenerated".to_string());
        }
        effects.extend(dying);
        effects.extend(at_end);
        let effects = match unless {
            Some(unless) => vec![self.unless_wrap(unless, effects, &mut sub, chain)?],
            None => effects,
        };
        let effects = if optional_counter {
            vec![format!(
                "Effect::MayDo {{ effects: &[{}] }}",
                effects.join(", ")
            )]
        } else {
            effects
        };
        Self::push_effects_merging_kicked(&mut chain.effects, effects);
        self.chain_sub_ability(sub, chain)?;
        if chain.second_target.is_some() {
            for effect in &mut chain.effects {
                if effect == "Effect::TapTarget" {
                    *effect = "Effect::TapTargetSlot(TargetSlot::First)".to_string();
                } else if effect == "Effect::UntapTarget" {
                    *effect = "Effect::UntapTargetSlot(TargetSlot::First)".to_string();
                }
            }
        }
        Some(())
    }

    /// Whether a line further down the `SubAbility$` chain declares a target.
    fn later_line_targets(&self, first: Option<&str>) -> bool {
        let mut seen = 0;
        let mut next = first.map(str::to_string);
        while let Some(name) = next.take() {
            seen += 1;
            let Some(body) = self.svars.get(&name) else {
                return false;
            };
            if seen > 16 {
                return false;
            }
            let Some((_, mut p)) = Params::parse(body) else {
                return false;
            };
            if p.has("ValidTgts") {
                return true;
            }
            next = p.take("SubAbility");
        }
        false
    }

    pub(super) fn chain_sub_ability(
        &mut self,
        sub: Option<String>,
        chain: &mut Chain,
    ) -> Option<()> {
        let Some(name) = sub else {
            return Some(());
        };
        let Some(body) = self.svars.get(&name).cloned() else {
            return self.deny(format!("`SubAbility$ {name}` names no SVar"));
        };
        self.chain(&body, chain)
    }

    // The requirement and the effect name the target differently when it
    // is a player: the wizard resolves `AnyPlayer`/`AnyOpponent` into the
    // spell's chosen player, and the effect then reads it back as
    // `PlayerRel::Chosen`. Handing the *requirement* to the effect
    // instead is how a burn spell ends up dealing damage to nothing at
    // all: `DealDamage` looks for an object target and finds none.
    pub(super) fn effect_target_of(spec: &str) -> String {
        match spec {
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent" => {
                "TargetSpec::Player(PlayerRel::Chosen)".to_string()
            }
            other => other.to_string(),
        }
    }

    pub(super) fn effect_target(chain: &Chain) -> Option<String> {
        chain
            .second_target
            .as_deref()
            .or(chain.target.as_deref())
            .map(Self::effect_target_of)
    }

    fn wrap_discarded_artifact_condition(
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        p.claim_label("ConditionDescription", "ConditionPresent");
        if !p.exhausted() {
            return None;
        }
        Some(vec![format!(
            "Effect::IfCondition {{ condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DiscardedMatching(&Filter::ARTIFACT)), 1), then: &[{}], otherwise: &[] }}",
            effects.join(", ")
        )])
    }

    /// "If an artifact card was discarded this way, untap …", read from three
    /// exact clauses. All three are peeked before any is taken: a line that
    /// differs in one keeps the others, for the general reading to read or
    /// refuse, and does not lose its condition to the comparison.
    fn is_discarded_artifact_untap(api: &str, p: &Params) -> bool {
        api == "Untap"
            && p.peek("ConditionDefined") == Some("Remembered")
            && p.peek("ConditionPresent") == Some("Card.Artifact")
            && p.peek("ConditionCompare") == Some("EQ1")
    }

    #[allow(clippy::too_many_lines)]
    fn wrap_line_condition(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        effects: Vec<String>,
    ) -> Option<Vec<String>> {
        if Self::is_discarded_artifact_untap(api, p) {
            for key in ["ConditionDefined", "ConditionPresent", "ConditionCompare"] {
                p.take(key);
            }
            return Self::wrap_discarded_artifact_condition(p, &effects);
        }
        if p.exhausted() {
            return Some(effects);
        }
        if let Some(wrapped) = self.wrap_kicked_condition(api, p, target, &effects).ok()? {
            return Some(wrapped);
        }
        if !p.has("ConditionPresent") {
            if let Some(cond_raw) = p.peek("Condition")
                && let Some(cond) = Self::named_condition(cond_raw)
                    .or_else(|| Self::wave7_named_condition(cond_raw))
            {
                p.take("Condition");
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                    effects.join(", ")
                )]);
            }
            if let (Some(var), Some(cmp), Some(sub_name)) = (
                p.peek("ConditionCheckSVar").map(str::to_string),
                p.peek("ConditionSVarCompare").map(str::to_string),
                sub.clone(),
            ) && matches!(cmp.trim(), "LE6" | "LT7")
                && self.svars.get(var.trim()).map(|s| s.trim())
                    == Some("Count$ValidGraveyard Card.YouOwn")
            {
                p.take("ConditionCheckSVar");
                p.take("ConditionSVarCompare");
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                let (sub_api, mut sp) = Params::parse(&sub_body)?;
                sp.drop_prose();
                if sp.has("SubAbility") || sp.has("ValidTgts") {
                    return None;
                }
                let sub_threshold = if sp.peek("Condition") == Some("Threshold") {
                    sp.take("Condition");
                    true
                } else if sp.peek("ConditionCheckSVar").map(str::trim) == Some(var.trim())
                    && sp.peek("ConditionSVarCompare").map(str::trim) == Some("GE7")
                {
                    sp.take("ConditionCheckSVar");
                    sp.take("ConditionSVarCompare");
                    true
                } else {
                    false
                };
                if !sub_threshold {
                    return None;
                }
                sp.take("ConditionDescription");
                let then_effects = self.effect_of(&sub_api, &mut sp, target, false, false)?;
                if !sp.exhausted() || then_effects.is_empty() {
                    return None;
                }
                *sub = None;
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{}], otherwise: &[{}] }}",
                    then_effects.join(", "),
                    effects.join(", ")
                )]);
            }
            if let Some(var) = p.peek("ConditionCheckSVar").map(str::to_string) {
                let cmp = p
                    .peek("ConditionSVarCompare")
                    .map_or_else(|| "GE1".to_string(), str::to_string);
                if let Some(cond) = self
                    .svar_condition(&var, &cmp)
                    .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                    .or_else(|| self.wave7_svar_condition(&var, &cmp))
                    .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, true))
                {
                    p.take("ConditionCheckSVar");
                    p.take("ConditionSVarCompare");
                    p.take("ConditionDescription");
                    if !p.exhausted() {
                        if let Some(key) = p.first_key() {
                            self.note(format!("unclaimed parameter `{api}.{key}`"));
                        }
                        return None;
                    }
                    return Some(vec![format!(
                        "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                        effects.join(", ")
                    )]);
                }
            }
            if p.has("ConditionNotPresent")
                && let Some(cond) = self.wave11_object_matches_condition(p, target, true)
            {
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                    effects.join(", ")
                )]);
            }
            if p.has("ConditionPhases")
                && let Some(cond) = Self::take_main_phase_condition(p)
            {
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                    effects.join(", ")
                )]);
            }
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        p.claim_label("ConditionDescription", "ConditionPresent");
        if let Some(cmp) = p.take("ConditionCompare") {
            if p.has("PresentCompare") {
                return self.deny("both `ConditionCompare$` and `PresentCompare$`".to_string());
            }
            p.entries.push(("PresentCompare".to_string(), cmp));
        }
        let cond = if matches!(
            p.peek("ConditionPresent"),
            Some(
                "Card.Self+PromisedGift"
                    | "Card.PromisedGift"
                    | "Card.!PromisedGift"
                    | "Card.Self+!PromisedGift"
            )
        ) && matches!(p.peek("ConditionDefined"), None | Some("Self"))
            && matches!(p.peek("ConditionZone"), None | Some("Stack"))
            && !p.has("PresentPlayer")
            && !p.has("PresentZone")
        {
            let valid = p.take("ConditionPresent").expect("checked above");
            p.take("ConditionDefined");
            p.take("ConditionZone");
            let cmp = p.take("PresentCompare");
            let positive_present = !valid.contains('!');
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                Some(other) => {
                    return self.deny(format!("promised-gift condition compared `{other}`"));
                }
            };
            if positive {
                "Condition::GiftWasPromised".to_string()
            } else {
                "Condition::Not(&Condition::GiftWasPromised)".to_string()
            }
        } else if matches!(
            p.peek("ConditionPresent"),
            Some(
                "Card.Self+CastSa Spell.Warp"
                    | "Card.CastSa Spell.Warp"
                    | "Card.!CastSa Spell.Warp"
                    | "Card.Self+!CastSa Spell.Warp"
            )
        ) && matches!(p.peek("ConditionDefined"), None | Some("Self"))
            && matches!(p.peek("ConditionZone"), None | Some("Stack"))
            && !p.has("PresentPlayer")
            && !p.has("PresentZone")
        {
            let valid = p.take("ConditionPresent").expect("checked above");
            p.take("ConditionDefined");
            p.take("ConditionZone");
            let cmp = p.take("PresentCompare");
            let positive_present = !valid.contains('!');
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                Some(other) => {
                    return self.deny(format!("warp condition compared `{other}`"));
                }
            };
            if positive {
                "Condition::WarpCostPaid".to_string()
            } else {
                "Condition::Not(&Condition::WarpCostPaid)".to_string()
            }
        } else if matches!(p.peek("ConditionDefined"), None | Some("Self"))
            && !p.has("PresentPlayer")
            && !p.has("PresentZone")
            && matches!(
                p.peek("ConditionPresent"),
                Some(
                    "Card.Self+IsMonstrous"
                        | "Card.IsMonstrous"
                        | "Card.Self+!IsMonstrous"
                        | "Card.!IsMonstrous"
                        | "Card.Self+monstrous"
                        | "Card.monstrous"
                        | "Card.Self+!monstrous"
                        | "Card.!monstrous"
                        | "Card.Self+IsSuspected"
                        | "Card.IsSuspected"
                        | "Card.Self+!IsSuspected"
                        | "Card.!IsSuspected"
                        | "Card.Self+suspected"
                        | "Card.suspected"
                        | "Card.Self+!suspected"
                        | "Card.!suspected"
                )
            )
        {
            let valid = p.take("ConditionPresent").expect("checked above");
            p.take("ConditionDefined");
            let cmp = p.take("PresentCompare");
            let base = if valid.to_ascii_lowercase().contains("monstrous") {
                "Condition::IsMonstrous"
            } else {
                "Condition::IsSuspected"
            };
            let positive_present = !valid.contains('!');
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                Some(other) => {
                    return self.deny(format!("monstrous/suspected condition compared `{other}`"));
                }
            };
            if positive {
                base.to_string()
            } else {
                format!("Condition::Not(&{base})")
            }
        } else if p.peek("ConditionDefined") == Some("Self")
            && !p.has("PresentPlayer")
            && !p.has("PresentZone")
            && matches!(
                p.peek("ConditionPresent"),
                Some("Card.bargained" | "Card.!bargained" | "Card.prototyped" | "Card.!prototyped")
            )
        {
            let valid = p.take("ConditionPresent").expect("checked above");
            p.take("ConditionDefined");
            let cmp = p.take("PresentCompare");
            let (base, positive_present) = match valid.as_str() {
                "Card.bargained" => ("Condition::Bargained", true),
                "Card.!bargained" => ("Condition::Bargained", false),
                "Card.prototyped" => ("Condition::Prototyped", true),
                _ => ("Condition::Prototyped", false),
            };
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                Some(other) => {
                    return self.deny(format!("bargained/prototyped condition compared `{other}`"));
                }
            };
            if positive {
                base.to_string()
            } else {
                format!("Condition::Not(&{base})")
            }
        } else if matches!(p.peek("ConditionDefined"), None | Some("Self"))
            && !p.has("PresentPlayer")
            && !p.has("PresentZone")
            && matches!(
                p.peek("ConditionPresent"),
                Some(
                    "Card.wasCastByYou"
                        | "Card.Self+wasCastByYou"
                        | "Card.wasCastByYou+Self"
                        | "Card.!wasCastByYou"
                        | "Card.Self+!wasCastByYou"
                        | "Card.wasCast"
                        | "Card.Self+wasCast"
                        | "Card.wasCast+Self"
                        | "Card.!wasCast"
                        | "Card.Self+!wasCast"
                )
            )
            && (p
                .peek("ConditionPresent")
                .is_some_and(|v| v.contains("wasCastByYou"))
                || (self.on_a_spell && p.has("ConditionPhases"))
                || self.oracle.to_ascii_lowercase().contains("if you cast"))
        {
            let valid = p.take("ConditionPresent").expect("checked above");
            p.take("ConditionDefined");
            let cmp = p.take("PresentCompare");
            let positive_present = !valid.contains('!');
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                Some(other) => {
                    return self.deny(format!("wasCast condition compared `{other}`"));
                }
            };
            let cast_cond = if positive {
                "Condition::SourceWasCastByYou".to_string()
            } else {
                "Condition::Not(&Condition::SourceWasCastByYou)".to_string()
            };
            if p.has("ConditionPhases") {
                let mut clauses = vec![cast_cond];
                Self::push_main_phase_clauses(p, &mut clauses)?;
                format!("Condition::All(&[{}])", clauses.join(", "))
            } else {
                cast_cond
            }
        } else if let Some(obj_cond) = self.wave11_object_matches_condition(p, target, false) {
            obj_cond
        } else {
            let valid = p.take("ConditionPresent").expect("checked above");
            if !p.has("ConditionDefined")
                && !p.has("PresentPlayer")
                && matches!(p.peek("PresentZone"), None | Some("Battlefield"))
                && Self::is_your_commander_present(&valid)
            {
                let cmp = p.take("PresentCompare");
                p.take("PresentZone");
                match cmp.as_deref() {
                    None | Some("GE1") => "Condition::ControlYourCommander".to_string(),
                    Some("EQ0") => "Condition::Not(&Condition::ControlYourCommander)".to_string(),
                    Some(other) => {
                        return self.deny(format!("commander presence compared `{other}`"));
                    }
                }
            } else if !p.has("ConditionDefined")
                && !p.has("PresentPlayer")
                && matches!(p.peek("PresentZone"), None | Some("Battlefield"))
                && matches!(
                    valid.as_str(),
                    "Creature.YouCtrl+modified"
                        | "Creature.modified+YouCtrl"
                        | "Card.YouCtrl+modified"
                        | "Card.modified+YouCtrl"
                        | "Creature.YouCtrl+isModified"
                        | "Creature.isModified+YouCtrl"
                )
            {
                let cmp = p.take("PresentCompare");
                p.take("PresentZone");
                match cmp.as_deref() {
                    None | Some("GE1" | "GT0") => {
                        "Condition::ControlCount(&Filter::YOUR_MODIFIED_CREATURE, 1)".to_string()
                    }
                    Some("EQ0" | "LT1") => {
                        "Condition::Not(&Condition::ControlCount(&Filter::YOUR_MODIFIED_CREATURE, 1))"
                            .to_string()
                    }
                    Some(other) => {
                        p.entries.push(("PresentCompare".to_string(), other.to_string()));
                        self.present_clause(p, &valid)?
                    }
                }
            } else {
                self.present_clause(p, &valid)?
            }
        };
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
            effects.join(", ")
        )])
    }

    fn take_main_phase_condition(p: &mut Params) -> Option<String> {
        let mut clauses = Vec::new();
        Self::push_main_phase_clauses(p, &mut clauses)?;
        Some(format!("Condition::All(&[{}])", clauses.join(", ")))
    }

    fn push_main_phase_clauses(p: &mut Params, clauses: &mut Vec<String>) -> Option<()> {
        let phases = p.take("ConditionPhases")?;
        let normalized: Vec<&str> = phases
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        match p.take("ConditionPlayerTurn").as_deref() {
            None => {}
            Some("True") => clauses.push("Condition::YourTurn".to_string()),
            Some(_) => return None,
        }
        match normalized.as_slice() {
            ["Main1", "Main2"] | ["Main2", "Main1"] => {
                clauses.push("Condition::Not(&Condition::BeforeStep(StepKind::FirstMain))".into());
                clauses.push("Condition::BeforeStep(StepKind::End)".into());
                clauses.push("Condition::Not(&Condition::DuringCombat)".into());
            }
            ["Main1"] => {
                clauses.push("Condition::Not(&Condition::BeforeStep(StepKind::FirstMain))".into());
                clauses.push("Condition::BeforeStep(StepKind::CombatBegin)".into());
            }
            ["Main2"] => {
                clauses.push("Condition::Not(&Condition::BeforeStep(StepKind::CombatEnd))".into());
                clauses.push("Condition::BeforeStep(StepKind::End)".into());
                clauses.push("Condition::Not(&Condition::DuringCombat)".into());
            }
            _ => return None,
        }
        Some(())
    }

    pub(super) fn resolve_target_spec(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        chain: &Chain,
        graveyard_description: Option<&str>,
    ) -> Option<String> {
        // A card in a graveyard is a different kind of target from a
        // permanent (CR 115.1 names both, and they are chosen from
        // different zones), so the zone the line moves *from* decides
        // the spec before the valid-string is read.
        let change_zone_targets_player = api == "ChangeZone"
            && p.peek("Origin") == Some("Graveyard")
            && matches!(valid, "Player" | "Opponent" | "Player.Opponent")
            && p.peek("Hidden") == Some("True")
            && p.has("DefinedPlayer");
        let spec = if (api == "ChangeZone"
            && p.peek("Origin") == Some("Graveyard")
            && !change_zone_targets_player)
            || (matches!(api, "Clone" | "CopyPermanent" | "TargetOnly" | "Play")
                && p.peek("TgtZone") == Some("Graveyard"))
            || (api == "Pump"
                && p.peek("TgtZone") == Some("Graveyard")
                && p.peek("PumpZone") == Some("Graveyard")
                && p.peek("KW").map(str::trim) == Some("Flashback"))
        {
            match p.take("TgtZone").as_deref() {
                None | Some("Graveyard") => {}
                Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
            }
            let defined_ctrl = p.take("TargetsWithDefinedController");
            self.graveyard_target(valid, defined_ctrl.as_deref(), graveyard_description)
        } else {
            // "Target spell or permanent" (the Laces): the stack is a
            // zone a target may be chosen in only where the line says
            // so. The battlefield alone is what every target already
            // is.
            match p.take("TgtZone").as_deref() {
                None | Some("Battlefield") => {
                    if api == "Counter" {
                        self.counter_or_stack_target(valid, p.peek("TargetType"), true)
                    } else if api == "CopySpellAbility" {
                        self.counter_or_stack_target(valid, p.peek("TargetType"), false)
                    } else if api == "ChangeTargets" {
                        self.change_targets_target(valid, p.peek("TargetType"))
                    } else if chain.target.is_some()
                        && (chain.target_unique
                            || matches!(api, "Fight" | "DealDamage" | "Clone" | "Tap" | "Untap"))
                        && p.peek("TargetUnique") == Some("True")
                        && api != "Counter"
                    {
                        p.take("TargetUnique");
                        let expr = self.filter_expr(valid)?;
                        let another =
                            format!("Filter::And(&[{expr}, Filter::Not(&Filter::Reference)])");
                        let name = self.body.filter_static("TARGET", &another);
                        Some(format!("TargetSpec::Object(&{name})"))
                    } else {
                        self.target_spec(valid, api)
                    }
                }
                Some("Stack") => self.counter_or_stack_target(valid, p.peek("TargetType"), true),
                Some("Stack,Battlefield" | "Battlefield,Stack") => {
                    if api == "ChangeZone"
                        && valid.contains("inZoneStack")
                        && !valid.split(',').any(|a| a.trim() == "Card.inZoneStack")
                    {
                        return self.deny(format!("restricted stack/battlefield target `{valid}`"));
                    }
                    self.spell_or_permanent_target(valid)
                }
                Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
            }
        };
        let Some(spec) = spec else {
            return self.deny(format!("target `{valid}`"));
        };
        Some(spec)
    }

    pub(super) fn counter_or_stack_target(
        &mut self,
        valid: &str,
        target_type: Option<&str>,
        allow_bare_spell: bool,
    ) -> Option<String> {
        let any_source = matches!(valid, "Card" | "Card,Emblem" | "Emblem,Card");
        let Some(raw) = target_type else {
            return if allow_bare_spell {
                self.target_spec(valid, "Counter")
            } else {
                None
            };
        };
        // `Spell`, `Activated`, `Triggered`, each optionally qualified by one
        // shared controller atom (`Spell.YouDontCtrl,Activated.YouDontCtrl`).
        let (mut spell, mut activated, mut triggered) = (false, false, false);
        let mut qualifier: Option<Option<&str>> = None;
        for part in raw.split(',').map(str::trim) {
            let (kind, qual) = match part.split_once('.') {
                Some((k, q)) => (k, Some(q)),
                None => (part, None),
            };
            let seen = match kind {
                "Spell" => &mut spell,
                "Activated" => &mut activated,
                "Triggered" => &mut triggered,
                _ => return None,
            };
            if *seen || qualifier.is_some_and(|q| q != qual) {
                return None;
            }
            *seen = true;
            qualifier = Some(qual);
        }
        let qualifier = qualifier.flatten();
        let ctrl = match qualifier {
            None => None,
            Some("YouCtrl") => Some("&Filter::ControlledByYou".to_string()),
            Some("OppCtrl") => Some("&Filter::ControlledByOpponent".to_string()),
            // "You don't control" is not "an opponent controls" (CR 102.3):
            // keep the negation the card prints.
            Some("YouDontCtrl") => {
                let expr = self.filter_expr("Card.YouDontCtrl")?;
                Some(format!("&{}", self.body.filter_static("TARGET", &expr)))
            }
            Some(_) => return None,
        };
        match (spell, activated, triggered) {
            (true, false, false) => match ctrl {
                None => self.target_spec(valid, "Counter"),
                Some(c) if any_source => Some(format!("TargetSpec::Spell({c})")),
                Some(_) => None,
            },
            (false, true, false) if ctrl.is_none() => self.activated_ability_target(valid),
            (false, true, true) if any_source => Some(format!(
                "TargetSpec::AbilityOnStack({})",
                ctrl.as_deref().unwrap_or("&Filter::Any")
            )),
            (true, true, true) if any_source => Some(format!(
                "TargetSpec::SpellOrAbility({})",
                ctrl.as_deref().unwrap_or("&Filter::Any")
            )),
            _ => None,
        }
    }

    /// Register a fresh target declaration without merging independent instances.
    #[allow(clippy::too_many_lines)]
    pub(super) fn chain_target(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        chain: &mut Chain,
        graveyard_description: Option<&str>,
    ) -> Option<()> {
        let has_target_unique = p.peek("TargetUnique").is_some();
        let mut spec = self.resolve_target_spec(api, p, valid, chain, graveyard_description)?;
        if chain.target.is_some() {
            // A fresh ValidTgts declares another target instance even
            // when its restriction is identical. Never alias it to the
            // earlier list.
            if chain.second_target.is_some() {
                return self.deny("unsupported independent target in one chain".to_string());
            }
            let is_obj_spec = spec.starts_with("TargetSpec::Object(");
            let single_object_first = chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
                && chain.target_up_to.is_none()
                && chain.target_exact.is_none()
                && !chain.target_is_x;
            let no_riders = |p: &Params| {
                ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                    .iter()
                    .all(|key| p.peek(key).is_none())
            };
            let allowed = match api {
                "Pump" => {
                    is_obj_spec
                        && !self.later_line_targets(p.peek("SubAbility"))
                        && p.peek("Defined").is_none_or(|v| v == "Targeted")
                        && no_riders(p)
                        && ["NumAtt", "NumDef"]
                            .iter()
                            .all(|key| p.peek(key).is_none_or(|value| value.parse::<i16>().is_ok()))
                }
                "Fight" => {
                    is_obj_spec
                        && single_object_first
                        && p.peek("Defined") == Some("ParentTarget")
                        && no_riders(p)
                }
                "DealDamage" => {
                    (is_obj_spec || spec == "TargetSpec::CreatureOrPlaneswalker")
                        && single_object_first
                        && p.peek("Defined").is_none()
                        && p.peek("DamageSource") == Some("ParentTarget")
                        && no_riders(p)
                }
                "MoveCounter" => {
                    is_obj_spec
                        && single_object_first
                        && p.peek("Source") == Some("ParentTarget")
                        && !has_target_unique
                        && no_riders(p)
                }
                "TargetOnly" => {
                    chain.effects.is_empty()
                        && !has_target_unique
                        && no_riders(p)
                        && self.can_two_target_only(p.peek("SubAbility"), chain, &spec)
                }
                "Tap" | "Untap" => {
                    is_obj_spec
                        && !self.later_line_targets(p.peek("SubAbility"))
                        && p.peek("Defined").is_none_or(|v| v == "Targeted")
                        && no_riders(p)
                }
                "Clone" => {
                    is_obj_spec
                        && single_object_first
                        && p.peek("CloneTarget") == Some("ParentTarget")
                        && no_riders(p)
                }
                "ExchangeControl" => {
                    is_obj_spec
                        && single_object_first
                        && chain.effects.is_empty()
                        && p.peek("Defined") == Some("ParentTarget")
                        && !has_target_unique
                        && no_riders(p)
                }
                "Destroy" => {
                    is_obj_spec
                        && !chain.effects.is_empty()
                        && chain.effects.iter().all(|e| {
                            e.starts_with("Effect::destroy")
                                || e == "Effect::TapTarget"
                                || e == "Effect::UntapTarget"
                                || e.starts_with("Effect::AddCounter")
                                || e.starts_with("Effect::DealDamage")
                                || e.starts_with("Effect::bounce(")
                                || e.starts_with("Effect::exile(")
                        })
                        && !self.later_line_targets(p.peek("SubAbility"))
                        && p.peek("Defined").is_none_or(|v| v == "Targeted")
                        && no_riders(p)
                }
                _ => false,
            };
            if !allowed {
                return self.deny("unsupported independent target in one chain".to_string());
            }
            if matches!(
                api,
                "Fight" | "DealDamage" | "Tap" | "Untap" | "Pump" | "Destroy" | "TargetOnly"
            ) && p.peek("TargetMin") == Some("0")
                && p.peek("TargetMax") == Some("1")
            {
                p.take("TargetMin");
                p.take("TargetMax");
                chain.second_target_up_to = Some(1);
            }
            chain.second_target = Some(spec);
        } else {
            if p.peek("TargetUnique") == Some("True")
                && p.peek("SubAbility")
                    .and_then(|sub| self.svars.get(sub.trim()))
                    .and_then(|body| Params::parse(body))
                    .is_some_and(|(_, child)| {
                        child.has("ValidTgts") && child.peek("TargetUnique") == Some("True")
                    })
            {
                p.take("TargetUnique");
                chain.target_unique = true;
            }
            if !self.try_kicked_target_count(api, p, valid, &spec)? {
                let target_only_multi_safe = api == "TargetOnly"
                    && p.peek("SubAbility").is_none_or(|sub_name| {
                        self.svars
                            .get(sub_name.trim())
                            .and_then(|body| Params::parse(body))
                            .is_some_and(|(sub_api, sp)| {
                                matches!(
                                    sub_api.as_str(),
                                    "Destroy"
                                        | "Tap"
                                        | "Untap"
                                        | "Pump"
                                        | "PutCounter"
                                        | "Detain"
                                        | "Goad"
                                        | "Phases"
                                        | "Animate"
                                ) || (sub_api == "DealDamage"
                                    && !sp.has("DividedAsYouChoose")
                                    && sp.peek("DamageSource").is_none())
                                    || (sub_api == "ChangeZone"
                                        && matches!(
                                            (sp.peek("Origin"), sp.peek("Destination")),
                                            (
                                                Some("Battlefield" | "Graveyard" | "Stack"),
                                                Some("Hand" | "Exile")
                                            ) | (Some("Graveyard"), Some("Battlefield"))
                                        ))
                            })
                    });
                let multi_target_ok = matches!(
                    api,
                    "Destroy"
                        | "Tap"
                        | "Untap"
                        | "Pump"
                        | "PutCounter"
                        | "CopyPermanent"
                        | "Phases"
                        | "Goad"
                        | "Detain"
                        | "Animate"
                ) || target_only_multi_safe
                    || (api == "DealDamage"
                        && !p.has("DividedAsYouChoose")
                        && p.peek("DamageSource").is_none())
                    || (api == "SetState" && p.peek("Mode") == Some("TurnFaceUp"))
                    || (api == "Effect" && p.peek("RememberObjects") == Some("Targeted"))
                    || (api == "ChangeZone"
                        && (matches!(
                            (p.peek("Origin"), p.peek("Destination")),
                            (
                                Some(
                                    "Battlefield"
                                        | "Graveyard"
                                        | "Stack"
                                        | "Battlefield,Stack"
                                        | "Stack,Battlefield"
                                ),
                                Some("Hand" | "Exile")
                            ) | (Some("Graveyard"), Some("Battlefield"))
                        ) || (matches!(p.peek("Origin"), Some("Battlefield" | "Graveyard"))
                            && p.peek("Destination") == Some("Library")
                            && p.peek("Shuffle") == Some("True"))
                            || (p.peek("Origin") == Some("Battlefield")
                                && p.peek("Destination") == Some("Library")
                                && !p.has("DestinationAlternative")
                                && p.peek("LibraryPosition")
                                    .is_some_and(|pos| pos == "-1" || !pos.starts_with('-')))
                            || (p.peek("Origin") == Some("Graveyard")
                                && p.peek("Destination") == Some("Library")
                                && p.peek("LibraryPosition") == Some("-1"))));
                if multi_target_ok
                    && p.peek("TargetsForEachPlayer") == Some("True")
                    && p.peek("TargetMin") == Some("0")
                    && spec.starts_with("TargetSpec::Object(")
                    && p.peek("TargetMax")
                        .map(|v| self.svars.get(v.trim()).map_or(v.trim(), |s| s.trim()))
                        == Some("PlayerCountOpponents$Amount")
                    && let Some(stripped) = Self::strip_opp_ctrl_from_valid(valid)
                {
                    let expr = self.filter_expr(&stripped)?;
                    let filter_name = self.body.filter_static("TARGET", &expr);
                    p.take("TargetsForEachPlayer");
                    p.take("TargetMin");
                    p.take("TargetMax");
                    spec = format!("TargetSpec::ObjectOfEachOpponent(&{filter_name})");
                    chain.target_up_to = Some(u8::MAX);
                } else if p.peek("TargetMin") == Some("1")
                    && matches!(p.peek("TargetMax"), None | Some("1"))
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                } else if self.has_x
                    && multi_target_ok
                    && let (Some(min), Some(max)) = (p.peek("TargetMin"), p.peek("TargetMax"))
                    && min == max
                    && self.svars.get(min.trim()).map(|s| s.trim()) == Some("Count$xPaid")
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                    chain.target_is_x = true;
                } else if (multi_target_ok
                    || matches!(
                        api,
                        "Fight"
                            | "Attach"
                            | "MustBlock"
                            | "Effect"
                            | "Connive"
                            | "Explore"
                            | "SetLife"
                            | "TargetOnly"
                            | "GainControl"
                            | "Counter"
                            | "LoseLife"
                            | "GainLife"
                            | "Draw"
                            | "Mill"
                            | "Discard"
                            | "RemoveCounter"
                            | "MoveCounter"
                            | "Regenerate"
                            | "PreventDamage"
                            | "Play"
                            | "Branch"
                    )
                    || (api == "DealDamage" && !p.has("DividedAsYouChoose"))
                    || (api == "ChangeZone"
                        && matches!(
                            (p.peek("Origin"), p.peek("Destination")),
                            (
                                Some(
                                    "Battlefield"
                                        | "Graveyard"
                                        | "Stack"
                                        | "Battlefield,Stack"
                                        | "Stack,Battlefield"
                                ),
                                Some("Library")
                            ) | (
                                Some("Exile"),
                                Some("Battlefield" | "Hand" | "Graveyard" | "Library")
                            ) | (Some("Battlefield"), Some("Graveyard"))
                        )))
                    && p.peek("TargetMin") == Some("0")
                    && matches!(p.peek("TargetMax"), None | Some("1"))
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                    chain.target_up_to = Some(1);
                } else if multi_target_ok
                    && p.peek("TargetMin") == Some("0")
                    && let Some(max) = p
                        .peek("TargetMax")
                        .and_then(|s| s.trim().parse::<u8>().ok())
                    && max > 0
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                    chain.target_up_to = Some(max);
                } else if multi_target_ok
                    && p.peek("TargetMin") == Some("0")
                    && let Some(max_var) = p.peek("TargetMax")
                    && self.svars.get(max_var.trim()).is_some_and(|def| {
                        let def = def.trim();
                        def == format!("Count$Valid {valid}")
                            || def
                                .strip_prefix("Count$Valid ")
                                .is_some_and(|cv| Self::same_valid_atoms(cv, valid))
                            || (spec.starts_with("TargetSpec::Object(")
                                && matches!(def, "Count$Valid Card" | "Count$Valid Permanent"))
                            || (api == "ChangeZone"
                                && p.peek("Origin") == Some("Graveyard")
                                && def.strip_prefix("Count$ValidGraveyard ").is_some_and(|cv| {
                                    cv == valid
                                        || cv == "Card"
                                        || (matches!(cv, "Card.YouOwn" | "Card.YouCtrl")
                                            && (valid.contains("YouOwn")
                                                || valid.contains("YouCtrl")))
                                        || Self::same_valid_atoms(
                                            &cv.replace("YouOwn", "YouCtrl"),
                                            &valid.replace("YouOwn", "YouCtrl"),
                                        )
                                }))
                    })
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                    chain.target_up_to = Some(u8::MAX);
                } else if (multi_target_ok
                    || (self.on_a_spell && api == "DealDamage" && p.peek("DamageSource").is_none()))
                    && let (Some(min), Some(max)) = (p.peek("TargetMin"), p.peek("TargetMax"))
                    && min == max
                    && let Ok(n) = min.trim().parse::<u8>()
                    && n > 1
                {
                    p.take("TargetMin");
                    p.take("TargetMax");
                    chain.target_exact = Some(n);
                }
            }
            chain.target = Some(spec);
        }
        Some(())
    }

    /// The "unless" of a line: `UnlessCost$` with the keys that qualify it.
    ///
    /// **The payer.** `UnlessPayer$ You` is the source's controller. Absent,
    /// the reference asks the controller of the line's target (its default
    /// is `TargetedController`), which is Mana Leak's "unless its controller
    /// pays" — so an absent payer is read only on a line that targets a
    /// spell or a permanent, and refused on one that targets nothing, where
    /// that default names nobody. On a cast trigger `TriggeredActivator` is
    /// the caster and `TriggeredCardController` the spell's controller, read
    /// as `Defined$` reads them (Rhystic Study, Mystic Remora). Every other
    /// payer is refused by name: `Player` is every player at once, a price
    /// no one question can put.
    ///
    /// **The subs.** Without `UnlessResolveSubs$` the rest of the chain runs
    /// either way, which is what wrapping this line alone gives. With it the
    /// rest runs on one answer only (Power Sink's "if that player doesn't,
    /// they tap all lands…"), a shape this does not read yet.
    pub(super) fn unless(
        &mut self,
        cost: &str,
        p: &mut Params,
        target: Option<&str>,
        has_sub: bool,
    ) -> Option<Unless> {
        let switched = match p.take("UnlessSwitched").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`UnlessSwitched$ {other}`")),
        };
        let resolve_subs = match p.take("UnlessResolveSubs").as_deref() {
            None => false,
            Some("WhenNotPaid") if has_sub && !switched => true,
            Some("WhenPaid") if has_sub && switched => true,
            Some(subs) => return self.deny(format!("`UnlessResolveSubs$ {subs}`")),
        };
        let payer = p.take("UnlessPayer");
        let targets_a_controlled_object = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        let targets_a_player = target
            .is_some_and(|t| t == "TargetSpec::AnyPlayer" || t.starts_with("TargetSpec::Player("));
        let payer = self.unless_payer(
            payer.as_deref(),
            targets_a_controlled_object,
            targets_a_player,
            switched,
        )?;
        let price = if payer == "PlayerRel::You" && switched {
            if let Some(draw_expr) = self.draw_cost_effect(cost.trim()) {
                Price::Part(format!("__DRAW__:{draw_expr}"))
            } else {
                self.unless_price_switched(cost.trim(), switched)?
            }
        } else {
            self.unless_price_switched(cost.trim(), switched)?
        };
        if payer == "ANY_PLAYER" {
            let valid_any_player = match &price {
                Price::Printed(_) => true,
                Price::Generic(mana) => mana
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|s| s.strip_suffix(')'))
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some(),
                Price::Part(_) => false,
            };
            if !valid_any_player {
                return self.deny("an unless-cost paid by `Player`".to_string());
            }
        }
        Some(Unless {
            payer,
            price,
            switched,
            resolve_subs,
        })
    }

    fn unless_payer(
        &self,
        payer: Option<&str>,
        targets_a_controlled_object: bool,
        targets_a_player: bool,
        switched: bool,
    ) -> Option<&'static str> {
        let trigger = self.trigger_mode.as_deref();
        match payer {
            Some("You") => Some("PlayerRel::You"),
            Some("EnchantedController") => Some("PlayerRel::ControllerOfAttached"),
            None | Some("TargetedController") if targets_a_controlled_object => {
                Some("PlayerRel::ControllerOfTarget")
            }
            Some("Targeted") if targets_a_player => Some("PlayerRel::Chosen"),
            Some(who @ ("TriggeredActivator" | "TriggeredCardController"))
                if matches!(trigger, Some("SpellCast" | "SpellCastOrCopy")) =>
            {
                self.player_rel(Some(who))
            }
            Some("TriggeredCardController") if trigger == Some("Drawn") => {
                self.player_rel(Some("TriggeredCardController"))
            }
            Some("TriggeredTargetController") if trigger == Some("DamageDone") => {
                self.player_rel(Some("TriggeredTargetController"))
            }
            Some("TriggeredDefendingPlayer")
                if matches!(trigger, Some("Attacks" | "AttackerBlocked")) =>
            {
                self.player_rel(Some("TriggeredDefendingPlayer"))
            }
            Some("TriggeredPlayer") if trigger == Some("Phase") => Some("PlayerRel::ActivePlayer"),
            Some(who) if self.wave11_trigger_player_rel(who).is_some() => {
                self.wave11_trigger_player_rel(who)
            }
            Some("Player") if !switched => Some("ANY_PLAYER"),
            other => self.deny(format!(
                "an unless-cost paid by `{}`",
                other.unwrap_or("TargetedController")
            )),
        }
    }

    pub(super) fn unless_price(&mut self, raw: &str) -> Option<Price> {
        self.unless_price_switched(raw, false)
    }

    /// An `UnlessCost$` value as a [`Price`].
    ///
    /// Read by [`Tx::cost_pieces`], the reader an activation cost goes
    /// through, and then held to what the "unless" effects can carry. Plain
    /// generic mana stays an [`Amount`] (`PlayerMayPayOr`); anything with a
    /// colour in it is printed exactly (`PlayerMayPayManaOr`), where it used
    /// to be refused — a `{U}` charged as `{1}` is a card anyone could keep
    /// with a Mountain. One part the player pays by naming an object is
    /// `PlayerMayPayCostOr` (or `PlayerMayPayCostThen` when switched); a
    /// switched `PayLife<N>` is `PlayerMayPayLifeThen`.
    fn unless_price_switched(&mut self, raw: &str, switched: bool) -> Option<Price> {
        // "Unless its controller pays {X}" (Power Sink): the X announced
        // for the source, and only where the card says that is what X is.
        if matches!(raw, "X" | "Y" | "Z") {
            let Some(x) = self.amount_or_count_ext(raw) else {
                return self.deny(format!("an unless-cost of `{raw}`"));
            };
            return Some(Price::Generic(x));
        }
        if switched
            && (raw == "Sac<1/CARDNAME>"
                || (raw.starts_with("Sac<1/CARDNAME/") && raw.find('>') == Some(raw.len() - 1)))
        {
            return Some(Price::Part("Sacrifice(&Filter::This)".to_string()));
        }
        let (mana, parts) = self.cost_pieces(raw)?;
        match (mana.as_str(), parts.as_slice()) {
            (m, []) if !m.is_empty() => Some(match generic_mana(m) {
                Some(n) => Price::Generic(format!("Amount::Fixed({n})")),
                None => Price::Printed(m.to_string()),
            }),
            ("", [one]) if asks_for_an_object(one) => Some(Price::Part(one.clone())),
            ("", [one])
                if (one.starts_with("PayLife(") && one.ends_with(')'))
                    || (one.starts_with("DealDamageToYou(") && one.ends_with(')'))
                    || one == "Forage"
                    || one.starts_with("SacrificeX(")
                    || one.starts_with("TapX(")
                    || one.starts_with("DiscardX(")
                    || one.starts_with("ExileFromGraveyardX(") =>
            {
                Some(Price::Part(one.clone()))
            }
            _ => self.deny(format!("an unless-cost of `{raw}`")),
        }
    }

    /// The line's effects behind its price. A tax runs them on a refusal and
    /// takes them as one effect (a `Sequence` when there are several); a
    /// switched price runs them on a payment and takes the list.
    pub(super) fn unless_wrap(
        &mut self,
        unless: Unless,
        mut effects: Vec<String>,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<String> {
        let Unless {
            payer,
            price,
            switched,
            resolve_subs,
        } = unless;
        if resolve_subs {
            let name = sub.take()?;
            let Some(body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let mut sub_chain = Chain {
                target: chain.target.clone(),
                second_target: chain.second_target.clone(),
                ..Chain::default()
            };
            self.chain(&body, &mut sub_chain)?;
            if sub_chain.target != chain.target || sub_chain.second_target != chain.second_target {
                return self.deny("`UnlessResolveSubs$ WhenNotPaid` with a new target".to_string());
            }
            chain.could_add_mana |= sub_chain.could_add_mana;
            chain.moves_library |= sub_chain.moves_library;
            effects.extend(sub_chain.effects);
        }
        let one = match effects.as_slice() {
            [] => return self.deny("an unless-cost on a line with no effect".to_string()),
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        let list = effects.join(", ");
        if payer == "ANY_PLAYER" {
            let cost_expr = match price {
                Price::Printed(cost) => format!("mana!(\"{cost}\")"),
                Price::Generic(mana) => {
                    let n = mana
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|s| s.strip_suffix(')'))
                        .and_then(|s| s.parse::<u32>().ok())?;
                    if n == 0 {
                        "ManaCost::ZERO".to_string()
                    } else {
                        format!("mana!(\"{{{n}}}\")")
                    }
                }
                Price::Part(_) => return None,
            };
            return Some(format!(
                "Effect::AnyPlayerMayPayManaOr {{ cost: {cost_expr}, effects: &[{list}] }}"
            ));
        }
        Some(match (price, switched) {
            (Price::Generic(mana), false) => format!(
                "Effect::PlayerMayPayOr {{ player: {payer}, mana: {mana}, effect: &{one} }}"
            ),
            (Price::Generic(mana), true) => format!(
                "Effect::PlayerMayPayThen {{ player: {payer}, mana: {mana}, effects: &[{list}] }}"
            ),
            (Price::Printed(cost), false) => format!(
                "Effect::PlayerMayPayManaOr {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effect: &{one} }}"
            ),
            (Price::Printed(cost), true) => format!(
                "Effect::PlayerMayPayManaThen {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effects: &[{list}] }}"
            ),
            (Price::Part(part), false) => {
                if let Some(n) = part
                    .strip_prefix("PayLife(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    format!(
                        "Effect::PlayerMayPayLifeOr {{ player: {payer}, life: Amount::Fixed({n}), \
                         effect: &{one} }}"
                    )
                } else {
                    format!(
                        "Effect::PlayerMayPayCostOr {{ player: {payer}, cost: &CostPart::{part}, \
                         effect: &{one} }}"
                    )
                }
            }
            (Price::Part(part), true) => {
                if let Some(draw_expr) = part.strip_prefix("__DRAW__:") {
                    format!("Effect::MayDo {{ effects: &[{draw_expr}, {list}] }}")
                } else if let Some(n) = part
                    .strip_prefix("PayLife(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    format!(
                        "Effect::PlayerMayPayLifeThen {{ player: {payer}, life: Amount::Fixed({n}), \
                         effects: &[{list}] }}"
                    )
                } else {
                    format!(
                        "Effect::PlayerMayPayCostThen {{ player: {payer}, cost: &CostPart::{part}, \
                         effect: &{one} }}"
                    )
                }
            }
        })
    }

    /// One effect API as the `Effect` expressions it stands for.
    ///
    /// Every parameter a rule reads is *taken* from `p`; the caller then
    /// refuses the card if anything is left, which is what stops an ignored
    /// `NoRegen$ True` from generating a card that does the wrong thing.
    // One arm per reference API, and the reasons a reading is what it is
    // live beside the arm that makes it.
    pub(super) fn effect_of(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        self.effect_of_inner(
            api,
            p,
            target,
            LineFlags {
                targets_here,
                targets_a_player,
                single_target: false,
                prior_target_used: false,
            },
        )
    }

    pub(super) fn effect_of_followup(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        self.effect_of_inner(
            api,
            p,
            target,
            LineFlags {
                targets_here: false,
                targets_a_player: false,
                single_target: false,
                prior_target_used: true,
            },
        )
    }

    pub(super) fn effect_of_with_prior(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        self.effect_of_inner(api, p, target, flags)
    }

    #[allow(clippy::too_many_lines)]
    fn effect_of_inner(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let LineFlags {
            targets_here,
            targets_a_player,
            single_target,
            prior_target_used,
        } = flags;
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
            "Amass" => self.amass_effect(p, target, targets_a_player)?,
            "Incubate" => self.incubate_effect(p, target, targets_a_player)?,
            "MultiplyCounter" => self.multiply_counter_effect(
                p,
                target,
                targets_here,
                single_target,
                prior_target_used,
            )?,
            "ChangeTargets" => {
                self.change_targets_effect(p, target, targets_here, single_target)?
            }
            "Monstrosity" => self.monstrosity_effect(p, target)?,
            "Suspect" => self.suspect_api_effect(p, target, targets_here)?,
            "Radiation" => self.radiation_effect(p, target, targets_a_player)?,
            "ExchangeControl" => {
                self.exchange_control_single_effect(p, target, targets_here, single_target)?
            }
            "SkipPhase" => self.skip_phase_effect(p, target, targets_a_player)?,
            "LosesGame" => {
                if targets_here || targets_a_player || p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec!["Effect::LoseGame".to_string()]
            }
            "MoveCounter" => self.move_counter_effect(p, target, targets_here, false)?,
            "AlterAttribute" => {
                self.alter_attribute_effect(p, target, targets_here, single_target)?
            }
            "BecomeMonarch" => self.become_monarch_effect(p, target, targets_a_player)?,
            "RingTemptsYou" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                vec!["Effect::RingTemptsYou".to_string()]
            }
            "Venture" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                match p.take("Dungeon").as_deref() {
                    None => vec!["Effect::VentureIntoTheDungeon".to_string()],
                    Some("Undercity") => vec!["Effect::VentureIntoUndercity".to_string()],
                    Some(_) => return None,
                }
            }
            "TakeInitiative" => {
                if targets_here && !targets_a_player {
                    return None;
                }
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                if !Self::is_single_player_rel(who) {
                    return None;
                }
                vec![format!("Effect::TakeInitiative({who})")]
            }
            "Detain" => self.detain_effect(p, target, targets_here, prior_target_used)?,
            "Mobilize" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("Self" | "You"))
                {
                    return None;
                }
                let raw = p.take("Num").or_else(|| p.take("Amount"))?;
                let n = self.amount_or_count_ext(&raw)?;
                vec![format!("Effect::Mobilize({n})")]
            }
            "Endure" => {
                if self.on_a_spell
                    || targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                {
                    return None;
                }
                let raw = p.take("Num").or_else(|| p.take("Amount"))?;
                let n = self.amount_or_count_ext(&raw)?;
                vec![format!("Effect::Endure({n})")]
            }
            "Firebend" | "Firebending" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                let raw = p.take("Num").or_else(|| p.take("Amount"))?;
                let n = self.amount_or_count_ext(&raw)?;
                vec![format!("Effect::Firebend({n})")]
            }
            "Cipher" => {
                if !self.on_a_spell
                    || targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                {
                    return None;
                }
                vec!["Effect::Cipher".to_string()]
            }
            "ReturnChampioned" => {
                if self.on_a_spell
                    || targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                {
                    return None;
                }
                vec!["Effect::ReturnChampioned".to_string()]
            }
            "AddPhase" => self.add_phase_effect(p, target, targets_here, prior_target_used)?,
            "ChooseNumber" => {
                self.choose_number_effect(p, target, targets_here, prior_target_used)?
            }
            "NameCard" => {
                if p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec![
                    "Effect::ChooseCardName { player: PlayerRel::You, types: TypeSet::EMPTY, excluded_types: TypeSet::EMPTY, remember: false }"
                        .to_string(),
                ]
            }
            "DealDamage" => {
                let num_dmg = p.take("NumDmg")?;
                let damage_source = p.take("DamageSource");
                let may = match p.take("OptionalDecider").as_deref() {
                    None => false,
                    Some("You") => true,
                    Some(_) => return None,
                };
                // Some reference chains gather assignments into a damage
                // map; the paired-recipient reader handles the printed
                // permanent-and-controller shape before this path.
                if p.take("DamageMap").is_some_and(|v| v != "True") {
                    return None;
                }
                if matches!(damage_source.as_deref(), Some("Targeted" | "ParentTarget")) {
                    if (damage_source.as_deref() == Some("ParentTarget") && targets_here)
                        || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                        || !self.is_power_of(
                            &num_dmg,
                            &["Targeted$CardPower", "ParentTargeted$CardPower"],
                        )
                    {
                        return None;
                    }
                    let to_slot = match p.take("Defined").as_deref() {
                        None if targets_here => "TargetSlot::First",
                        Some("Self") if !self.on_a_spell => "TargetSlot::This",
                        _ => return None,
                    };
                    let dmg = format!(
                        "Effect::DamageEqualToPower {{ dealer: TargetSlot::First, to: {to_slot} }}"
                    );
                    return Some(if may {
                        vec![format!("Effect::MayDo {{ effects: &[{dmg}] }}")]
                    } else {
                        vec![dmg]
                    });
                }
                let turn_start_hand = match p.peek("Defined") {
                    Some("You.CardsInHandAtBeginningOfTurn") => Some(true),
                    Some("You.!CardsInHandAtBeginningOfTurn") => Some(false),
                    _ => None,
                };
                if turn_start_hand.is_some() {
                    if damage_source.is_some() {
                        return None;
                    }
                    p.take("Defined");
                }
                if damage_source.is_none()
                    && turn_start_hand.is_none()
                    && !self.on_a_spell
                    && matches!(p.peek("Defined"), Some("Enchanted" | "Equipped"))
                {
                    p.take("Defined");
                    let n = self.amount_or_count_ext(&num_dmg)?;
                    let dmg = format!("Effect::DealDamageToAttached {{ amount: {n} }}");
                    return Some(if may {
                        vec![format!("Effect::MayDo {{ effects: &[{dmg}] }}")]
                    } else {
                        vec![dmg]
                    });
                }
                let to = if turn_start_hand.is_some() {
                    "TargetSpec::Player(PlayerRel::You)".to_string()
                } else {
                    let block_role = self.block_line.as_ref().map(|(r, _, _)| *r);
                    match p.take("Defined").as_deref() {
                        None => aimed.to_string(),
                        Some("Self") if !self.on_a_spell => "TargetSpec::ThisObject".to_string(),
                        Some("Targeted" | "ParentTarget") if !targets_here && target.is_some() => {
                            aimed.to_string()
                        }
                        Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                            if matches!(
                                self.trigger_mode.as_deref(),
                                Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                            ) || (self.trigger_mode.as_deref()
                                == Some("AttackerBlockedByCreature")
                                && block_role == Some(super::BlockRole::Blocks)) =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                            if self.trigger_mode.as_deref() == Some("Blocks")
                                || (self.trigger_mode.as_deref()
                                    == Some("AttackerBlockedByCreature")
                                    && block_role == Some(super::BlockRole::Blocked)) =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredAttackerLKICopy,TriggeredBlockerLKICopy")
                            if matches!(
                                self.trigger_mode.as_deref(),
                                Some("Attacks" | "Blocks")
                            ) =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredAttackerController")
                            if matches!(
                                self.trigger_mode.as_deref(),
                                Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                            ) || (self.trigger_mode.as_deref()
                                == Some("AttackerBlockedByCreature")
                                && block_role == Some(super::BlockRole::Blocks)) =>
                        {
                            "TargetSpec::Player(PlayerRel::ControllerOfEvent)".to_string()
                        }
                        Some("TriggeredBlockerController")
                            if self.trigger_mode.as_deref() == Some("Blocks")
                                || (self.trigger_mode.as_deref()
                                    == Some("AttackerBlockedByCreature")
                                    && block_role == Some(super::BlockRole::Blocked)) =>
                        {
                            "TargetSpec::Player(PlayerRel::ControllerOfEvent)".to_string()
                        }
                        Some("TriggeredCardLKICopy" | "TriggeredCard")
                            if matches!(
                                self.trigger_mode.as_deref(),
                                Some(
                                    "ChangesZone"
                                        | "Exerted"
                                        | "Enlisted"
                                        | "Taps"
                                        | "Untaps"
                                        | "TurnFaceUp"
                                        | "Transformed"
                                )
                            ) && target.is_none() =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredNewCardLKICopy" | "TriggeredNewCard")
                            if self.trigger_mode.as_deref() == Some("ChangesZone")
                                && target.is_none() =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                            if matches!(
                                self.trigger_mode.as_deref(),
                                Some("DamageDone" | "DamageDoneOnce")
                            ) && target.is_none() =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                            if self.in_delayed && target.is_none() =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        // "Deals 1 damage to that player" and every other
                        // player the line names without targeting one.
                        Some(who) => format!(
                            "TargetSpec::Player({})",
                            self.wave11_player_of_line(Some(who), target, false)?
                        ),
                    }
                };
                // Two DealDamage operations would let redirection deal
                // separate damage to the same recipient (CR 614.9, 120.4b).
                if matches!(
                    to.as_str(),
                    "TargetSpec::Player(PlayerRel::ControllerOfTarget)"
                        | "TargetSpec::Player(PlayerRel::ControllerOfEvent)"
                ) && self.prints_simultaneous_damage_to_controller()
                {
                    return self
                        .deny("simultaneous damage to a permanent and its controller".to_string());
                }
                let dmg = if let Some(src) = damage_source {
                    if src == "Self" {
                        let n = self.amount_or_count_ext(&num_dmg)?;
                        format!("Effect::DealDamage {{ amount: {n}, target: {to} }}")
                    } else if src == "Targeted"
                        && to == "TargetSpec::ThisObject"
                        && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                        && single_target
                        && self.is_power_of(&num_dmg, &["Targeted$CardPower"])
                    {
                        "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::This }"
                            .to_string()
                    } else {
                        let power_names: &[&str] =
                            match (src.as_str(), self.trigger_mode.as_deref()) {
                                (
                                    "TriggeredCard" | "TriggeredCardLKICopy",
                                    Some(
                                        "ChangesZone" | "Exerted" | "Enlisted" | "Sacrificed"
                                        | "Taps" | "Untaps" | "TurnFaceUp" | "Transformed",
                                    ),
                                ) => &["TriggeredCard$CardPower", "TriggeredCardLKICopy$CardPower"],
                                (
                                    "TriggeredNewCard" | "TriggeredNewCardLKICopy",
                                    Some("ChangesZone"),
                                ) => &[
                                    "TriggeredNewCard$CardPower",
                                    "TriggeredNewCardLKICopy$CardPower",
                                ],
                                (
                                    "TriggeredExploited" | "TriggeredExploitedLKICopy",
                                    Some("Exploited"),
                                ) => &[
                                    "TriggeredExploited$CardPower",
                                    "TriggeredExploitedLKICopy$CardPower",
                                ],
                                (
                                    "TriggeredTarget" | "TriggeredTargetLKICopy",
                                    Some("DamageDone" | "DamageDoneOnce"),
                                ) => &[
                                    "TriggeredTarget$CardPower",
                                    "TriggeredTargetLKICopy$CardPower",
                                ],
                                (
                                    "TriggeredAttacker" | "TriggeredAttackerLKICopy",
                                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked"),
                                ) => &[
                                    "TriggeredAttacker$CardPower",
                                    "TriggeredAttackerLKICopy$CardPower",
                                ],
                                (
                                    "TriggeredBlocker" | "TriggeredBlockerLKICopy",
                                    Some("Blocks"),
                                ) => &[
                                    "TriggeredBlocker$CardPower",
                                    "TriggeredBlockerLKICopy$CardPower",
                                ],
                                _ => return None,
                            };
                        if self.is_power_of(&num_dmg, power_names) {
                            format!("Effect::EventObjectDealsDamageEqualToPower {{ target: {to} }}")
                        } else {
                            let n = self.amount_or_count_ext(&num_dmg)?;
                            format!(
                                "Effect::EventObjectDealsDamage {{ amount: {n}, target: {to} }}"
                            )
                        }
                    }
                } else {
                    let n = self.amount_or_count_ext(&num_dmg)?;
                    format!("Effect::DealDamage {{ amount: {n}, target: {to} }}")
                };
                let dmg = if turn_start_hand == Some(true) {
                    format!(
                        "Effect::IfCondition {{ condition: Condition::Not(&Condition::HandSizeAtTurnStartAtMost(0)), then: &[{dmg}], otherwise: &[] }}"
                    )
                } else {
                    dmg
                };
                let dmg = if p.peek("ConditionDefined") == Some("Targeted")
                    && p.peek("ConditionPresent") == Some("Land.Basic")
                    && p.peek("ConditionCompare") == Some("EQ0")
                {
                    p.claim_label("ConditionDescription", "ConditionPresent");
                    p.take("ConditionDefined");
                    p.take("ConditionPresent");
                    p.take("ConditionCompare");
                    format!(
                        "Effect::IfTargetMatches {{ filter: &Filter::NONBASIC_LAND, then: &[{dmg}] }}"
                    )
                } else {
                    dmg
                };
                if may {
                    vec![format!("Effect::MayDo {{ effects: &[{dmg}] }}")]
                } else {
                    vec![dmg]
                }
            }
            // A number, or the X the player announced (Stream of Life's
            // "target player gains X life"): [`amount`] asks both of its
            // questions of an `X`, so a count spelled with the same letter
            // is still refused.
            "GainLife" => {
                let n = self.amount_or_count_ext(&p.take("LifeAmount")?)?;
                let fixed = n
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|r| r.strip_suffix(')'));
                let you_gain = match fixed {
                    Some(k) => format!("Effect::gain_life({k})"),
                    None => format!("Effect::GainLife {{ amount: {n} }}"),
                };
                if let Some(other_rel) =
                    self.split_you_and_player(p.peek("Defined"), target, targets_a_player)
                {
                    p.take("Defined");
                    return Some(vec![
                        you_gain,
                        format!("Effect::GainLifeFor {{ amount: {n}, who: {other_rel} }}"),
                    ]);
                }
                match self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )? {
                    "PlayerRel::You" => vec![you_gain],
                    who => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = self.amount_or_count_ext(&p.take("LifeAmount")?)?;
                if let Some(other_rel) =
                    self.split_you_and_player(p.peek("Defined"), target, targets_a_player)
                {
                    p.take("Defined");
                    return Some(vec![
                        format!("Effect::LoseLife {{ amount: {n}, target: PlayerRel::You }}"),
                        format!("Effect::LoseLife {{ amount: {n}, target: {other_rel} }}"),
                    ]);
                }
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Dig" => self.dig_effect(p, target, targets_here, targets_a_player)?,
            "DigUntil" => {
                let only_conds = |params: &Params| {
                    params
                        .entries
                        .iter()
                        .all(|(k, _)| k.starts_with("Condition"))
                };
                let snapshot = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                let is_named_card = matches!(
                    p.peek("Valid").map(str::trim),
                    Some("Card.NamedCard" | "NamedCard")
                );
                if !is_named_card
                    && let Some(effects) = self.dig_until_effect(p, target, targets_a_player)
                    && only_conds(p)
                {
                    effects
                } else {
                    p.clone_from(&snapshot);
                    self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
                    self.body.statics.truncate(saved_statics_len);
                    if let Some(effects) = self.wave4_dig_until_effect(p, target, targets_a_player)
                        && only_conds(p)
                    {
                        effects
                    } else {
                        p.clone_from(&snapshot);
                        self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
                        self.body.statics.truncate(saved_statics_len);
                        if let Some(effects) =
                            self.wave7_dig_until_effect(p, target, targets_a_player)
                            && only_conds(p)
                        {
                            effects
                        } else {
                            *p = snapshot;
                            *self.unclaimed.borrow_mut() = saved_unclaimed;
                            self.body.statics.truncate(saved_statics_len);
                            self.wave23a_dig_until_effect(p, target, targets_a_player)?
                        }
                    }
                }
            }
            "Draw"
                if matches!(
                    p.peek("Defined"),
                    Some("You.CardsInHandAtBeginningOfTurn" | "You.!CardsInHandAtBeginningOfTurn")
                ) =>
            {
                let had_cards =
                    p.take("Defined").as_deref() == Some("You.CardsInHandAtBeginningOfTurn");
                let n = self.amount_or_count_ext(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let draw = if let Some(fixed) = n
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|rest| rest.strip_suffix(')'))
                {
                    format!("Effect::draw({fixed})")
                } else {
                    format!("Effect::DrawCards {{ amount: {n} }}")
                };
                let condition = if had_cards {
                    "Condition::Not(&Condition::HandSizeAtTurnStartAtMost(0))"
                } else {
                    "Condition::HandSizeAtTurnStartAtMost(0)"
                };
                vec![format!(
                    "Effect::IfCondition {{ condition: {condition}, then: &[{draw}], otherwise: &[] }}"
                )]
            }
            // The same two readings as `GainLife`: Braingeyser's "target
            // player draws X cards".
            "Draw" => {
                let n = self.amount_or_count_ext(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let mut may = match p.take("OptionalDecider").as_deref() {
                    None => false,
                    Some("You") => true,
                    Some(_) => return None,
                };
                if !may
                    && !targets_a_player
                    && matches!(p.peek("Defined"), None | Some("You"))
                    && p.peek("Optional") == Some("True")
                {
                    p.take("Optional");
                    may = true;
                }
                if let Some(other_rel) =
                    self.split_you_and_player(p.peek("Defined"), target, targets_a_player)
                {
                    p.take("Defined");
                    let you_draw = if let Some(fixed) = n
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')'))
                    {
                        format!("Effect::draw({fixed})")
                    } else {
                        format!("Effect::DrawCards {{ amount: {n} }}")
                    };
                    let other_draw =
                        format!("Effect::DrawCardsFor {{ amount: {n}, who: {other_rel} }}");
                    return Some(if may {
                        vec![format!(
                            "Effect::MayDo {{ effects: &[{you_draw}, {other_draw}] }}"
                        )]
                    } else {
                        vec![you_draw, other_draw]
                    });
                }
                let draw = match (
                    self.wave11_player_of_line(
                        p.take("Defined").as_deref(),
                        target,
                        targets_a_player,
                    )?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => format!("Effect::draw({fixed})"),
                    ("PlayerRel::You", None) => format!("Effect::DrawCards {{ amount: {n} }}"),
                    (who, _) => {
                        format!("Effect::DrawCardsFor {{ amount: {n}, who: {who} }}")
                    }
                };
                if may {
                    vec![format!("Effect::MayDo {{ effects: &[{draw}] }}")]
                } else {
                    vec![draw]
                }
            }
            // "Target player discards a card": the discarding player
            // chooses, which is what discarding means unless the effect
            // says otherwise (CR 701.9b).
            // "Each player discards their hand" (Wheel of Fortune): every
            // card, nobody choosing.
            "Discard" if p.peek("Mode") == Some("Hand") => {
                p.take("Mode");
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::DiscardHand {{ who: {who} }}")]
            }
            "Discard" if p.peek("Mode") == Some("TgtChoose") => {
                if let Some(cost_raw) = p.peek("Cost").map(str::to_string) {
                    let draw_expr = self.draw_cost_effect(&cost_raw)?;
                    p.take("Cost");
                    if p.take("Mode").as_deref() != Some("TgtChoose")
                        || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                    {
                        return None;
                    }
                    let raw_n = p.take("NumCards").unwrap_or_else(|| "1".to_string());
                    let discard_expr = if let Some(n) =
                        self.plain_num(&raw_n).and_then(|n| u8::try_from(n).ok())
                    {
                        format!("Effect::DiscardForPlayers {{ who: PlayerRel::You, count: {n} }}")
                    } else {
                        let amount = self.amount_or_count_ext(&raw_n)?;
                        format!(
                            "Effect::DiscardAmountForPlayers {{ who: PlayerRel::You, amount: {amount} }}"
                        )
                    };
                    return Some(vec![format!(
                        "Effect::MayDo {{ effects: &[{draw_expr}, {discard_expr}] }}"
                    )]);
                }
                p.take("Mode");
                p.take("IsCurse");
                if let Some(remember) = p.take("RememberDiscarded")
                    && remember != "True"
                {
                    return None;
                }
                let raw_n = p.take("NumCards").unwrap_or_else(|| "1".to_string());
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                if let Some(n) = self.plain_num(&raw_n).and_then(|n| u8::try_from(n).ok()) {
                    vec![format!(
                        "Effect::DiscardForPlayers {{ who: {who}, count: {n} }}"
                    )]
                } else {
                    let amount = self.amount_or_count_ext(&raw_n)?;
                    vec![format!(
                        "Effect::DiscardAmountForPlayers {{ who: {who}, amount: {amount} }}"
                    )]
                }
            }
            "Discard" if p.peek("Mode") == Some("RevealDiscardAll") => {
                p.take("Mode");
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                let valid = p.take("DiscardValid")?;
                let expr = self.filter_expr_in_zone(&valid, true)?;
                let name = self.body.filter_static("DISCARD", &expr);
                vec![
                    format!("Effect::RevealHand {{ who: {who} }}"),
                    format!("Effect::DiscardMatching {{ who: {who}, filter: &{name} }}"),
                ]
            }
            "Discard" if p.peek("Mode") == Some("RevealYouChoose") => {
                p.take("Mode");
                if !targets_a_player
                    || p.take("Defined").is_some()
                    || !matches!(p.take("NumCards").as_deref(), None | Some("1"))
                {
                    return None;
                }
                let filter = match p.take("DiscardValid").as_deref() {
                    None | Some("Card") => "Filter::Any".to_string(),
                    Some(valid) => self.filter_expr_in_zone(valid, true)?,
                };
                vec![format!("Effect::RevealHandDiscard {{ filter: &{filter} }}")]
            }
            "Discard" => {
                // Refuse other modes instead of turning a chosen discard
                // into a random one. Unconsumed qualifiers also refuse it.
                if p.take("Mode").as_deref() != Some("Random") {
                    return None;
                }
                let n = self.amount_or_count_ext(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!(
                    "Effect::DiscardRandom {{ who: {who}, count: {n} }}"
                )]
            }
            "Mill" => {
                let n = self.amount_or_count_ext(&p.take("NumCards")?)?;
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "Bolster" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                let raw = p.take("Num").or_else(|| p.take("Amount"))?;
                let n = self.amount_or_count_ext(&raw)?;
                vec![format!("Effect::Bolster({n})")]
            }
            "PutCounter" => {
                if let Some(monstrosity) = p.take("Monstrosity") {
                    if self.on_a_spell
                        || target.is_some()
                        || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                    {
                        return None;
                    }
                    let n = self.amount_or_count_ext(&monstrosity)?;
                    return Some(vec![format!("Effect::Monstrosity({n})")]);
                }
                if let Some(adapt) = p.take("Adapt") {
                    if self.on_a_spell
                        || target.is_some()
                        || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                    {
                        return None;
                    }
                    let n: u32 = adapt.trim().parse().ok().filter(|&v| v > 0)?;
                    return Some(vec![format!(
                        "Effect::IfCondition {{ condition: Condition::Not(&Condition::CountersOnSelf(CounterKind::P1P1, 1)), then: &[Effect::AddCounterFilter {{ filter: &Filter::This, kind: CounterKind::P1P1, amount: Amount::Fixed({n}) }}], otherwise: &[] }}"
                    )]);
                }
                if let Some(bolster) = p.take("Bolster") {
                    if targets_here
                        || (target.is_some() && !prior_target_used)
                        || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                    {
                        return None;
                    }
                    let n = self.amount_or_count_ext(&bolster)?;
                    return Some(vec![format!("Effect::Bolster({n})")]);
                }
                let specs = if let Some(code) = p.take("CounterType") {
                    let Some(kind) = counter_kind(&code) else {
                        return self.deny(format!("counter `{code}`"));
                    };
                    let n =
                        self.amount_or_count_ext(p.take("CounterNum").as_deref().unwrap_or("1"))?;
                    if matches!(code.as_str(), "ENERGY" | "POISON" | "RAD")
                        && (targets_a_player
                            || p.peek("Defined").is_some_and(|d| {
                                self.wave11_player_of_line(Some(d), target, targets_a_player)
                                    .is_some()
                            }))
                    {
                        let who = self.wave11_player_of_line(
                            p.take("Defined").as_deref(),
                            target,
                            targets_a_player,
                        )?;
                        p.take("IsCurse");
                        let variant = match code.as_str() {
                            "ENERGY" => "Energy",
                            "POISON" => "Poison",
                            "RAD" => "Rad",
                            _ => unreachable!(),
                        };
                        return Some(vec![format!(
                            "Effect::{variant} {{ who: {who}, amount: {n} }}"
                        )]);
                    }
                    vec![(kind, n)]
                } else {
                    let codes = p.take("CounterTypes")?;
                    let raw_num = p.take("CounterNum");
                    let n = self.amount_or_count_ext(raw_num.as_deref().unwrap_or("1"))?;
                    match Self::collect_counter_specs(&codes, &n) {
                        Ok(specs) => specs,
                        Err(bad) => return self.deny(format!("counter `{bad}`")),
                    }
                };
                p.take("IsCurse");
                let emit_filter = |filter: &str| -> Vec<String> {
                    specs
                        .iter()
                        .map(|(kind, n)| {
                            format!(
                                "Effect::AddCounterFilter {{ filter: {filter}, kind: {kind}, \
                                 amount: {n} }}"
                            )
                        })
                        .collect()
                };
                let emit_target = || -> Vec<String> {
                    specs
                        .iter()
                        .map(|(kind, n)| {
                            format!("Effect::AddCounter {{ kind: {kind}, amount: {n} }}")
                        })
                        .collect()
                };
                match p.take("Defined").as_deref() {
                    Some("Self") if target.is_some() || self.counter_on_source => {
                        return Some(emit_filter("&Filter::This"));
                    }
                    None if self.counter_on_source => {
                        return Some(emit_filter("&Filter::This"));
                    }
                    Some("Self & Targeted" | "Targeted & Self")
                        if !self.on_a_spell
                            && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        let mut out = emit_target();
                        out.extend(emit_filter("&Filter::This"));
                        return Some(out);
                    }
                    Some("Targeted" | "ParentTarget")
                        if !targets_here && target.is_some() && !targets_a_player => {}
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(emit_filter("&Filter::AttachedToBySource"));
                    }
                    Some(
                        "TriggeredCardLKICopy"
                        | "TriggeredCard"
                        | "TriggeredNewCardLKICopy"
                        | "TriggeredNewCard",
                    ) if self.trigger_mode.as_deref() == Some("ChangesZone")
                        && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some("TriggeredCardLKICopy" | "TriggeredCard")
                        if matches!(
                            self.trigger_mode.as_deref(),
                            Some(
                                "Exerted"
                                    | "Enlisted"
                                    | "Taps"
                                    | "Untaps"
                                    | "TurnFaceUp"
                                    | "Transformed"
                            )
                        ) && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some(def)
                        if self.trigger_mode.as_deref() == Some("ChangesZone")
                            && target.is_none()
                            && (def.starts_with("TriggeredCardLKICopy.")
                                || def.starts_with("TriggeredCard.")
                                || def.starts_with("TriggeredNewCardLKICopy.")
                                || def.starts_with("TriggeredNewCard.")) =>
                    {
                        let qual = def
                            .strip_prefix("TriggeredCardLKICopy.")
                            .or_else(|| def.strip_prefix("TriggeredCard."))
                            .or_else(|| def.strip_prefix("TriggeredNewCardLKICopy."))
                            .or_else(|| def.strip_prefix("TriggeredNewCard."))?;
                        let q = self.filter_expr(&format!("Card.{qual}"))?;
                        let expr = format!("Filter::And(&[Filter::Reference, {q}])");
                        let name = self.body.filter_static("COUNTER", &expr);
                        return Some(emit_filter(&format!("&{name}")));
                    }
                    Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                        if (matches!(
                            self.trigger_mode.as_deref(),
                            Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                        ) || (self.trigger_mode.as_deref()
                            == Some("AttackerBlockedByCreature")
                            && self
                                .block_line
                                .as_ref()
                                .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks)))
                            && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                        if (self.trigger_mode.as_deref() == Some("Blocks")
                            || (self.trigger_mode.as_deref()
                                == Some("AttackerBlockedByCreature")
                                && self
                                    .block_line
                                    .as_ref()
                                    .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked)))
                            && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                        if matches!(
                            self.trigger_mode.as_deref(),
                            Some("DamageDone" | "DamageDoneOnce")
                        ) && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                        if self.in_delayed && target.is_none() =>
                    {
                        return Some(emit_filter("&Filter::Reference"));
                    }
                    Some(
                        "TriggeredCardLKICopy & Self"
                        | "Self & TriggeredCardLKICopy"
                        | "TriggeredCard & Self"
                        | "Self & TriggeredCard",
                    ) if self.trigger_mode.as_deref() == Some("ChangesZone")
                        && target.is_none() =>
                    {
                        let mut out = emit_filter("&Filter::Reference");
                        out.extend(emit_filter("&Filter::This"));
                        return Some(out);
                    }
                    Some(def)
                        if target.is_none()
                            && def.starts_with("Valid ")
                            && !def.contains("Remembered")
                            && !def.contains("Targeted") =>
                    {
                        let valid = def.strip_prefix("Valid ")?.trim();
                        let expr = self.filter_expr(valid)?;
                        let name = self.body.filter_static("COUNTER", &expr);
                        return Some(emit_filter(&format!("&{name}")));
                    }
                    None | Some("Self") => {}
                    Some(_) => return None,
                }
                emit_target()
            }
            "PutCounterAll" => {
                if targets_here || (target.is_some() && !prior_target_used) {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = self.amount_or_count_ext(p.take("CounterNum").as_deref().unwrap_or("1"))?;
                p.take("IsCurse");
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("COUNTERS", &expr);
                vec![format!(
                    "Effect::AddCounterFilter {{ filter: &{filter}, kind: {kind}, amount: {n} }}"
                )]
            }
            "Proliferate" => {
                if targets_here || (target.is_some() && !prior_target_used) {
                    return None;
                }
                let times = match p.take("Amount") {
                    None => 1usize,
                    Some(raw) => usize::try_from(self.plain_num(&raw)?)
                        .ok()
                        .filter(|&n| (1..=4).contains(&n))?,
                };
                vec!["Effect::Proliferate".to_string(); times]
            }
            "SetState" => {
                p.take("IsCurse");
                let optional = match p.take("Optional").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let eff = match p.take("Mode").as_deref() {
                    Some("Transform") => {
                        if self.on_a_spell
                            || targets_here
                            || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
                        {
                            return None;
                        }
                        "Effect::TransformSource".to_string()
                    }
                    Some("TurnFaceUp") => {
                        let to = match (p.take("Defined").as_deref(), target) {
                            (None | Some("Targeted" | "ParentTarget"), Some(aimed))
                                if aimed.starts_with("TargetSpec::Object(") =>
                            {
                                aimed.to_string()
                            }
                            (None | Some("Self"), None) if !self.on_a_spell => {
                                "TargetSpec::ThisObject".to_string()
                            }
                            (Some("Self"), _) if !self.on_a_spell && !targets_here => {
                                "TargetSpec::ThisObject".to_string()
                            }
                            (Some(def), None) if self.is_event_object_defined(def) => {
                                "TargetSpec::EventObject".to_string()
                            }
                            _ => return None,
                        };
                        format!("Effect::TurnFaceUp {{ target: {to} }}")
                    }
                    _ => return None,
                };
                if optional {
                    vec![format!("Effect::MayDo {{ effects: &[{eff}] }}")]
                } else {
                    vec![eff]
                }
            }
            "Manifest" => {
                if p.has("Defined") {
                    return None;
                }
                p.take("IsCurse");
                let who = self.wave11_player_of_line(
                    p.take("DefinedPlayer").as_deref(),
                    target,
                    targets_a_player,
                )?;
                let count = match p.take("Amount") {
                    None => "Amount::Fixed(1)".to_string(),
                    Some(raw) => self.amount_or_count_ext(&raw)?,
                };
                vec![format!(
                    "Effect::ManifestTop {{ who: {who}, count: {count} }}"
                )]
            }
            "ManifestDread" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
                {
                    return None;
                }
                let times = match p.take("Amount").as_deref() {
                    None | Some("1") => 1usize,
                    Some(raw) => {
                        let n = usize::try_from(self.plain_num(raw)?).ok()?;
                        if (1..=4).contains(&n) {
                            n
                        } else {
                            return None;
                        }
                    }
                };
                vec!["Effect::ManifestDread".to_string(); times]
            }
            "RemoveCounterAll" => {
                if targets_here
                    || (target.is_some() && !prior_target_used)
                    || p.take("AllCounters").as_deref() != Some("True")
                {
                    return None;
                }
                let kind = counter_kind(&p.take("CounterType")?)?;
                let valid = p.take("ValidCards")?;
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("COUNTERS", &expr);
                vec![format!(
                    "Effect::RemoveAllCountersFilter {{ filter: &{filter}, kind: {kind} }}"
                )]
            }
            "Cleanup" => {
                let rem = p.take("ClearRemembered");
                let col = p.take("ClearChosenColor");
                let named = p.take("ClearNamedCard");
                let player = p.take("ClearChosenPlayer");
                let any_true = rem.as_deref() == Some("True")
                    || col.as_deref() == Some("True")
                    || named.as_deref() == Some("True")
                    || player.as_deref() == Some("True");
                if !any_true
                    || rem.as_deref().is_some_and(|v| v != "True")
                    || col.as_deref().is_some_and(|v| v != "True")
                    || named.as_deref().is_some_and(|v| v != "True")
                    || player.as_deref().is_some_and(|v| v != "True")
                    || !p.exhausted()
                {
                    return None;
                }
                vec![]
            }
            "RemoveCounter" => {
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let all = p.peek("CounterNum") == Some("All");
                let n = if all {
                    p.take("CounterNum");
                    if !matches!(p.take("RememberRemoved").as_deref(), None | Some("True")) {
                        return None;
                    }
                    0
                } else {
                    self.plain_num(p.take("CounterNum").as_deref().unwrap_or("1"))?
                        .try_into()
                        .ok()?
                };
                if !all && n == 0 {
                    return None;
                }
                match p.take("Defined").as_deref() {
                    None | Some("Self") if target.is_none() => {}
                    _ => return None,
                }
                if all {
                    vec![format!("Effect::RemoveAllCountersSelf {{ kind: {kind} }}")]
                } else {
                    vec![format!(
                        "Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}"
                    )]
                }
            }
            "Scry" => {
                if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw = p.take("ScryNum").unwrap_or_else(|| "1".to_string());
                if let Some(n) = self.plain_num(&raw) {
                    vec![format!("Effect::scry({n})")]
                } else {
                    let n = self.amount_or_count_ext(&raw)?;
                    vec![format!("Effect::Scry {{ amount: {n} }}")]
                }
            }
            "Surveil" => {
                // `Amount$` and never `Defined$`: not one of the reference
                // corpus's 228 surveil lines names a player, because a
                // surveil is the controller's own library by construction
                // (CR 701.25a). 224 of them write `Amount$` and 219 of those
                // write a plain number; the rest announce an `X`, and the
                // constructor only fits the first kind.
                let raw = p.take("Amount")?;
                if let Some(n) = self.plain_num(&raw) {
                    vec![format!("Effect::surveil({n})")]
                } else {
                    let n = self.amount_or_count_ext(&raw)?;
                    vec![format!("Effect::Surveil {{ amount: {n} }}")]
                }
            }
            "Mana" => self.mana_effect(p)?,
            "ManaReflected" => self.mana_reflected_db_effect(p, target, targets_here)?,
            "Branch" => {
                let mut temp = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                self.wave24a_branch_effect(p, target, single_target, &mut temp)?
            }
            "Destroy" => {
                // `NoRegen$ True` is **read** and no longer merely consumed.
                // It was vacuous while this engine had no regeneration at
                // all — one door, and every card printing "it can't be
                // regenerated" was right for free — and the day a shield
                // existed the two doors stopped being the same function
                // (CR 701.19c). Any other value is a third thing the DSL
                // cannot say, so it refuses.
                let no_regen = match p.take("NoRegen").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let verb = if no_regen {
                    "destroy_no_regen"
                } else {
                    "destroy"
                };
                // "Destroy that creature" in a delayed trigger's body: the
                // object it remembers ([`Tx::delayed_trigger`]). Anywhere
                // else the key is left for the unclaimed check to refuse.
                let remembered = (self.in_delayed
                    && target.is_none()
                    && matches!(
                        p.peek("Defined"),
                        Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                    ))
                    || (matches!(
                        self.trigger_mode.as_deref(),
                        Some("DamageDone" | "DamageDoneOnce")
                    ) && target.is_none()
                        && matches!(
                            p.peek("Defined"),
                            Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                        ))
                    || (self.trigger_mode.as_deref() == Some("ChangesZone")
                        && target.is_none()
                        && matches!(
                            p.peek("Defined"),
                            Some("TriggeredCardLKICopy" | "TriggeredCard")
                        ))
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && target.is_none()
                        && ((p.peek("Defined") == Some("TriggeredBlockerLKICopy")
                            && self
                                .block_line
                                .as_ref()
                                .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked))
                            || (p.peek("Defined") == Some("TriggeredAttackerLKICopy")
                                && self
                                    .block_line
                                    .as_ref()
                                    .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks))));
                if remembered {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::EventObject)")]
                } else if !self.on_a_spell && target.is_none() && p.peek("Defined") == Some("Self")
                {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::ThisObject)")]
                } else {
                    if !targets_here && target.is_some() && p.peek("Defined") == Some("Targeted") {
                        p.take("Defined");
                    }
                    vec![format!("Effect::{verb}({aimed})")]
                }
            }
            "SacrificeAll" => {
                if targets_here {
                    return None;
                }
                if let Some(ctrl) = p.take("Controller")
                    && ctrl != "You"
                {
                    return self.deny(format!("`SacrificeAll` with `Controller$ {ctrl}`"));
                }
                if let Some(valid) = p.take("ValidCards") {
                    let filter = self.filter_expr(&valid)?;
                    if !self.on_a_spell
                        && (filter.contains("Filter::CmcAtMostX")
                            || filter.contains("Filter::CmcExactlyX"))
                    {
                        return None;
                    }
                    let name = self.body.filter_static("SWEEP", &filter);
                    vec![format!("Effect::SacrificeAll {{ filter: &{name} }}")]
                } else {
                    match p.take("Defined").as_deref() {
                        Some("Self") if !self.on_a_spell && target.is_none() => {
                            vec!["Effect::SacrificeSelf".to_string()]
                        }
                        Some("Equipped")
                            if !self.on_a_spell
                                && !self.cost_sacrifices_self
                                && target.is_none() =>
                        {
                            vec![
                                "Effect::SacrificeAll { filter: &Filter::AttachedToBySource }"
                                    .to_string(),
                            ]
                        }
                        Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                            if self.in_delayed && target.is_none() =>
                        {
                            vec!["Effect::SacrificeEvent".to_string()]
                        }
                        _ => return None,
                    }
                }
            }
            // "Destroy all lands", "destroy all creatures. They can't be
            // regenerated": every permanent the valid-string names, none of
            // them a target (so an ability that also targets is refused
            // rather than read as a sweep of its target). `NoRegen$` is the
            // same two doors as the single destroy above (CR 701.19c).
            "DestroyAll" => {
                let valid = p.take("ValidCards")?;
                // “Other than this creature” also excludes the source when
                // an effect has made it an artifact (Grim Giganotosaurus).
                let valid = if valid == "Creature.Other,Artifact"
                    && self
                        .oracle
                        .split_once(" all artifacts and creatures other than ")
                        .is_some_and(|(_, rest)| {
                            rest.starts_with("this creature.")
                                || rest
                                    .strip_prefix(self.script_name)
                                    .is_some_and(|tail| tail.starts_with('.'))
                        }) {
                    "Creature.Other,Artifact.Other"
                } else {
                    &valid
                };
                let no_regen = p.take("NoRegen");
                if target == Some("TargetSpec::Player(PlayerRel::Chosen)")
                    && let Some(stripped) = valid.strip_suffix("+TargetedPlayerCtrl")
                {
                    let no_regen = match no_regen.as_deref() {
                        None => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    let expr = self.filter_expr(stripped)?;
                    let name = self.body.filter_static("DESTROY", &expr);
                    return Some(vec![format!(
                        "Effect::DestroyAllOf {{ who: PlayerRel::Chosen, filter: &{name}, no_regen: {no_regen} }}"
                    )]);
                }
                if targets_here || (target.is_some() && !prior_target_used) {
                    return None;
                }
                if let Some(filter) = self.filter_expr(valid) {
                    // `Filter::CmcAtMostX` and `Filter::CmcExactlyX` read
                    // `this.x_value`, which is the announced X on a spell on
                    // the stack (`SP$`) and 0 on a permanent activating `AB$`.
                    if !self.on_a_spell
                        && (filter.contains("Filter::CmcAtMostX")
                            || filter.contains("Filter::CmcExactlyX"))
                    {
                        return None;
                    }
                    let verb = match no_regen.as_deref() {
                        None => "destroy_all",
                        Some("True") => "destroy_all_no_regen",
                        Some(_) => return None,
                    };
                    vec![format!("Effect::{verb}(&{filter})")]
                } else if no_regen.is_none() {
                    let stripped: Option<Vec<&str>> = valid
                        .split(',')
                        .map(|alt| alt.trim().strip_suffix(".cmcEQX"))
                        .collect();
                    let filter = self.filter_expr(&stripped?.join(","))?;
                    let value = self.amount_num("X")?;
                    vec![format!(
                        "Effect::DestroyAllWithManaValue {{ filter: &{filter}, value: {value} }}"
                    )]
                } else {
                    return None;
                }
            }
            // "X damage to each creature without flying and each player"
            // (Earthquake): `DealDamageEach` for the permanents and a
            // `DealDamage` for the players, which is the one spelling
            // `DealDamageEach` names for the second half. Nothing is
            // targeted, so an ability that also targets is refused.
            // Where the reference deals the damage `DamageMap$` gathered:
            // the `DealDamage` lines before it already did.
            "DamageResolve" => Vec::new(),
            // "Prevent the next N damage that would be dealt to any target
            // this turn" (Samite Healer; CR 615.7): a shield on what the
            // line targets, or on the player `Defined$` names. With
            // neither, nothing is shielded, and that is not a card.
            "PreventDamage" => {
                let raw_amount = p.take("Amount")?;
                let n = if self.svars.get(raw_amount.trim()).map(|d| d.trim())
                    == Some("Count$Domain")
                {
                    "Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)".to_string()
                } else {
                    self.amount_or_count_ext(&raw_amount)?
                };
                let to = match (p.take("Defined").as_deref(), target) {
                    (Some("Self"), None) if !self.on_a_spell => {
                        "TargetSpec::ThisObject".to_string()
                    }
                    (Some(who), None) => {
                        format!("TargetSpec::Player({})", self.wave11_player_rel(Some(who))?)
                    }
                    (None, Some(aimed)) => aimed.to_string(),
                    _ => return None,
                };
                vec![format!(
                    "Effect::PreventNextDamage {{ target: {to}, amount: {n} }}"
                )]
            }
            "RevealHand" => {
                if p.has("RememberRevealed")
                    || p.has("ImprintRevealed")
                    || p.has("RememberRevealedPlayer")
                    || p.has("RevealType")
                    || p.has("Optional")
                {
                    return None;
                }
                p.take("IsCurse");
                if p.peek("Look") == Some("True") {
                    p.take("Look");
                    let who = self.wave11_player_of_line(
                        p.take("Defined").as_deref(),
                        target,
                        targets_a_player,
                    )?;
                    if who != "PlayerRel::Chosen" {
                        return self.deny(format!("`RevealHand` `Look$ True` for `{who}`"));
                    }
                    vec!["Effect::LookAtChosenHand".to_string()]
                } else {
                    let lower = self.oracle.to_ascii_lowercase();
                    if !(lower.contains("reveals their hand")
                        || lower.contains("reveal your hand")
                        || lower.contains("reveals his or her hand")
                        || lower.contains("reveal their hands")
                        || lower.contains("reveal their hand"))
                        || lower.contains("look at")
                    {
                        return None;
                    }
                    let who = self.wave11_player_of_line(
                        p.take("Defined").as_deref(),
                        target,
                        targets_a_player,
                    )?;
                    vec![format!("Effect::RevealHand {{ who: {who} }}")]
                }
            }
            "CopySpellAbility" => self.copy_spell_ability_effect(p, target, targets_here)?,
            // "Prevent all combat damage that would be dealt this turn."
            // The bare line only: the reference narrows it with keys this
            // rule does not claim, and those refuse.
            "Fog" => vec!["Effect::PreventAllCombatDamageThisTurn".to_string()],
            "DamageAll" => {
                if targets_here {
                    return None;
                }
                let raw_dmg = p.take("NumDmg")?;
                let threshold_dmg = self
                    .svars
                    .get(raw_dmg.trim())
                    .and_then(|s| s.trim().strip_prefix("Count$Threshold."))
                    .and_then(|s| s.strip_suffix(".0"))
                    .and_then(|s| s.parse::<u32>().ok());
                let n = match threshold_dmg {
                    Some(v) => format!("Amount::Fixed({v})"),
                    None => self.amount_or_count_ext(&raw_dmg)?,
                };
                // The printed words for the two keys below ("each creature
                // and each player"), which say nothing those keys do not.
                p.take("ValidDescription");
                let mut out = Vec::new();
                if let Some(valid) = p.take("ValidCards") {
                    let filter = self.filter_expr(&valid)?;
                    let filter = self.body.filter_static("EACH", &filter);
                    out.push(format!(
                        "Effect::DealDamageEach {{ amount: {n}, filter: &{filter} }}"
                    ));
                }
                if let Some(players) = p.take("ValidPlayers") {
                    let who = self.player_rel(Some(&players))?;
                    out.push(format!(
                        "Effect::DealDamage {{ amount: {n}, target: TargetSpec::Player({who}) }}"
                    ));
                }
                if out.is_empty() {
                    return None;
                }
                if threshold_dmg.is_some() {
                    vec![format!(
                        "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{}], otherwise: &[] }}",
                        out.join(", ")
                    )]
                } else {
                    out
                }
            }
            "Regenerate" => {
                // Bare `AB$ Regenerate` is "regenerate CARDNAME" — 181 of
                // the reference's 275 lines writing this API name no target
                // at all — so the absent target is the source and not a
                // missing one, the same reading `Untap` makes two arms
                // below. `Defined$` names something else entirely (the
                // enchanted creature, a remembered object; 33 lines) and is
                // refused rather than guessed at, which leaves the 62 that
                // carry a `ValidTgts$` as the targeted half.
                match p.take("Defined").as_deref() {
                    None => {}
                    Some("Self") if target.is_none() && !self.on_a_spell => {}
                    // "Regenerate enchanted creature" (Regeneration): the
                    // Aura's host, which the filter binds to the source.
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(vec![
                            "Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"
                                .to_string(),
                        ]);
                    }
                    Some(_) => return None,
                }
                match target {
                    Some(t) => vec![format!("Effect::regenerate({t})")],
                    None => vec!["Effect::regenerate(TargetSpec::ThisObject)".to_string()],
                }
            }
            "ControlPlayer" => {
                if target != Some("TargetSpec::Player(PlayerRel::Chosen)") {
                    return None;
                }
                vec!["Effect::ControlPlayerNextTurn { player: PlayerRel::Chosen }".to_string()]
            }
            "Play" => self.play_effect(p, target, targets_here, targets_a_player)?,
            "Token" => self.token_effect(p, target, targets_a_player)?,
            "Investigate" => self.investigate_effect(p, targets_a_player)?,
            "Animate" => {
                let saved_p = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                let Some(effs) = self.animate_effect(p, target) else {
                    let animate_unclaimed = self.unclaimed.borrow().clone();
                    *p = saved_p;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    if let Some(effs) =
                        self.set_pt_by_count_filter_effect(false, p, target, single_target)
                    {
                        return Some(effs);
                    }
                    *self.unclaimed.borrow_mut() = animate_unclaimed;
                    return None;
                };
                if !single_target && target.is_some() {
                    effs.into_iter()
                        .map(|eff| {
                            let inner = eff
                                .strip_prefix("Effect::continuous(&Filter::This, ")?
                                .strip_suffix(')')?;
                            let (modifier, duration) = inner.rsplit_once(", ")?;
                            if !modifier.starts_with("Modifier::")
                                || !duration.starts_with("Duration::")
                            {
                                return None;
                            }
                            Some(format!(
                                "Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"
                            ))
                        })
                        .collect::<Option<Vec<_>>>()?
                } else {
                    effs
                }
            }
            "Debuff" => self.debuff_effect(p, target)?,
            "AnimateAll"
                if target.is_none()
                    && p.peek("ValidCards") == Some("Creature.OppCtrl")
                    && p.peek("RemoveKeywords") == Some("Hexproof & Shroud")
                    && p.peek("CantHaveKeyword") == Some("Hexproof & Shroud") =>
            {
                p.take("ValidCards");
                p.take("RemoveKeywords");
                p.take("CantHaveKeyword");
                vec!["Effect::continuous(&Filter::OPPONENT_CREATURE, Modifier::CantHaveKeywords(KeywordSet::HEXPROOF.union(KeywordSet::SHROUD)), Duration::UntilEndOfTurn)".to_string()]
            }
            "AnimateAll" => {
                let saved_p = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effs) = self.animate_all_effect(p, target) {
                    effs
                } else {
                    let animate_unclaimed = self.unclaimed.borrow().clone();
                    *p = saved_p;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    if let Some(effs) =
                        self.set_pt_by_count_filter_effect(true, p, target, single_target)
                    {
                        effs
                    } else {
                        *self.unclaimed.borrow_mut() = animate_unclaimed;
                        return None;
                    }
                }
            }
            "Pump" if p.peek("KW") == Some("Protection from everything") => {
                self.everything_protection(p)?
            }
            "Phases" => self.phase_effect(p, target, targets_here)?,
            "CopyPermanent" => {
                self.copy_permanent_effect(p, target, targets_here, targets_a_player)?
            }
            "Clone" => self.become_copy_effect(p, target, targets_here, false)?,
            "Pump" => self.pump_effect(p, aimed, false)?,
            "PumpAll" => {
                self.pump_all_effect(p, target, targets_here, targets_a_player, prior_target_used)?
            }
            "Effect" => {
                let snapshot = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.static_effect(p, target) {
                    effects
                } else {
                    p.clone_from(&snapshot);
                    self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
                    self.body.statics.truncate(saved_statics_len);
                    if let Some(effects) = self.wave3_effect_api(p, target) {
                        effects
                    } else {
                        p.clone_from(&snapshot);
                        self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
                        self.body.statics.truncate(saved_statics_len);
                        if let Some(effects) = self.grant_linked_exile_permission_effect(p, target)
                        {
                            effects
                        } else if target.is_none() && snapshot.peek("ReplacementEffects").is_some()
                        {
                            *p = Params::default();
                            *self.unclaimed.borrow_mut() = saved_unclaimed;
                            self.body.statics.truncate(saved_statics_len);
                            let mut dummy_chain = Chain::default();
                            self.prevention_effect_shield(snapshot, &mut dummy_chain)?
                        } else {
                            return None;
                        }
                    }
                }
            }
            "MustBlock" => {
                if !targets_here || !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
                    return None;
                }
                let this_combat_only = match p.take("Duration").as_deref() {
                    None | Some("UntilEndOfTurn") => false,
                    Some("UntilEndOfCombat") => true,
                    Some(_) => return None,
                };
                let attacker = match p.take("DefinedAttacker").as_deref() {
                    None if !self.on_a_spell => "TargetSpec::ThisObject",
                    Some("TriggeredAttacker" | "TriggeredAttackerLKICopy")
                        if self.trigger_mode.as_deref() == Some("Attacks") =>
                    {
                        "TargetSpec::EventObject"
                    }
                    _ => return None,
                };
                if attacker == "TargetSpec::ThisObject" && this_combat_only {
                    vec!["Effect::MustBlockSourceIfAbleThisCombat".to_string()]
                } else {
                    vec![format!(
                        "Effect::MustBlockIfAble {{ blocker: {aimed}, attacker: {attacker}, \
                         this_combat_only: {this_combat_only} }}"
                    )]
                }
            }
            "GainControl" => self.gain_control_effect(p, target, targets_here, targets_a_player)?,
            "Shuffle" => {
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::ShuffleLibrary {{ who: {who} }}")]
            }
            "ChangeZone" => self.change_zone_with_prior(
                p,
                target,
                targets_here,
                single_target,
                prior_target_used,
            )?,
            "ChangeZoneAll" => self.shuffle_into_library_with_prior(
                p,
                target,
                targets_a_player,
                prior_target_used,
            )?,
            // "Look at the top three cards of target player's library and
            // put them back in any order. You may have that player shuffle"
            // (Natural Selection). The ability's controller looks and
            // decides; a count the player announced is refused.
            "RearrangeTopOfLibrary" => {
                let count: u8 = p.take("NumCards")?.parse().ok()?;
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                let mut effects = vec![if who == "PlayerRel::You" {
                    format!("Effect::ReorderTopLibrary {{ count: {count} }}")
                } else {
                    format!("Effect::ReorderTopLibraryOf {{ who: {who}, count: {count} }}")
                }];
                match p.take("MayShuffle").as_deref() {
                    None => {}
                    Some("True") => effects.push(format!(
                        "Effect::MayDo {{ effects: &[Effect::ShuffleLibrary {{ who: {who} }}] }}"
                    )),
                    Some(_) => return None,
                }
                effects
            }
            "Sacrifice" => self.sacrifice_effect(p, target, targets_a_player)?,
            // "Tap enchanted creature" (Paralyze): the host, and not a
            // target.
            // "Tap all lands target player controls" (Mana Short): the
            // permanents of the players `Defined$` names, or of the line's
            // own player target. Without either it is every matching
            // permanent, `TapAll`, and the line must target nothing, for
            // a target it never uses is not a card.
            "TapAll" => {
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let defined = p.take("Defined");
                if defined.is_none() && !targets_a_player {
                    if targets_here || (target.is_some() && !prior_target_used) {
                        return None;
                    }
                    return Some(vec![format!("Effect::TapAll {{ filter: &{filter} }}")]);
                }
                let who =
                    self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::TapAllOf {{ who: {who}, filter: &{filter} }}"
                )]
            }
            // "That player loses all unspent mana" (CR 106.4).
            "DrainMana" => {
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::LoseUnspentMana {{ who: {who} }}")]
            }
            // "You may tap or untap target artifact, creature, or land"
            // (Twiddle): one of the two choices always does nothing, so
            // the choice is a yes or a no (`Effect::ToggleTapTarget`).
            "TapOrUntap" if target.is_some() && p.take("Defined").is_none() => {
                vec!["Effect::MayDo { effects: &[Effect::ToggleTapTarget] }".to_string()]
            }
            "Tap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(t))
                    if t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEachOpponent(") =>
                {
                    vec!["Effect::TapTarget".to_string()]
                }
                (None, None) | (Some("Self"), _) => vec!["Effect::TapSelf".to_string()],
                (Some("Targeted" | "ParentTarget"), Some(t))
                    if t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEachOpponent(") =>
                {
                    vec!["Effect::TapTarget".to_string()]
                }
                (Some("Enchanted" | "Equipped"), _) => {
                    vec!["Effect::TapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                (Some(def), None) if self.is_event_object_defined(def) => {
                    vec!["Effect::TapEventObject".to_string()]
                }
                _ => return None,
            },
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(t))
                    if t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEachOpponent(") =>
                {
                    vec!["Effect::UntapTarget".to_string()]
                }
                (Some("Targeted" | "ParentTarget"), Some(t))
                    if t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEachOpponent(") =>
                {
                    vec!["Effect::UntapTarget".to_string()]
                }
                (None, None) | (Some("Self"), _) => vec!["Effect::UntapSelf".to_string()],
                // "Untap enchanted creature" (Instill Energy).
                (Some("Enchanted" | "Equipped"), _) => {
                    vec!["Effect::UntapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                _ => return None,
            },
            "UntapAll" => {
                if targets_here || (target.is_some() && !prior_target_used) {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let expr = self.filter_expr(&valid)?;
                let name = self.body.filter_static("UNTAP_ALL", &expr);
                vec![format!("Effect::UntapAll {{ filter: &{name} }}")]
            }
            "RemoveFromCombat" => match (p.take("Defined").as_deref(), target) {
                (None | Some("Self"), None) => {
                    vec!["Effect::RemoveTargetFromCombat { unblock: false }".to_string()]
                }
                _ => return None,
            },
            // A fixed single skip owed by the resolving ability's controller.
            // Other players and counts are not represented by SkipNextTurn.
            "SkipTurn" => {
                if target.is_some()
                    || targets_a_player
                    || p.take("NumTurns").as_deref() != Some("1")
                    || p.take("Defined").as_deref() != Some("You")
                {
                    return None;
                }
                vec!["Effect::SkipNextTurn".to_string()]
            }
            // "Take an extra turn after this one" (Time Walk; CR 500.7).
            // One turn, and yours: another count or another player is a
            // different sentence.
            "AddTurn" => {
                if p.take("NumTurns").as_deref() != Some("1")
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                vec!["Effect::TakeExtraTurn".to_string()]
            }
            "ProtectionAll" => {
                if target.is_some() || targets_a_player || p.has("Defined") || p.has("ValidPlayers")
                {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("PROT", &expr);
                let duration = match p.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => return None,
                };
                match p.take("Gains").as_deref() {
                    Some("Choice") => {
                        if duration != "Duration::UntilEndOfTurn"
                            || p.take("Choices").as_deref() != Some("AnyColor")
                        {
                            return None;
                        }
                        vec![
                            "Effect::ChooseColor".to_string(),
                            format!(
                                "Effect::GrantProtectionFromChosenColorUntilEot {{ filter: &{filter} }}"
                            ),
                        ]
                    }
                    Some(gains) => {
                        let mut out = Vec::new();
                        for part in gains.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            let from =
                                self.protection_filter(&format!("Protection from {part}"))?;
                            out.push(format!(
                                "Effect::continuous(&{filter}, Modifier::ProtectionFrom(&{from}), {duration})"
                            ));
                        }
                        if out.is_empty() {
                            return None;
                        }
                        out
                    }
                    None => return None,
                }
            }
            "Protection" => {
                match (p.take("Defined").as_deref(), target) {
                    (None | Some("Targeted"), Some(_)) => {}
                    (None | Some("Self"), None) if !self.on_a_spell => {}
                    _ => return None,
                }
                let duration = match p.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => return None,
                };
                match p.take("Gains").as_deref() {
                    Some("Choice") => {
                        if p.take("Choices").as_deref() != Some("AnyColor") {
                            return None;
                        }
                        vec![format!(
                            "Effect::ProtectionFromChosenColor {{ duration: {duration} }}"
                        )]
                    }
                    Some(gains) => {
                        let mut out = Vec::new();
                        for part in gains.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            let from =
                                self.protection_filter(&format!("Protection from {part}"))?;
                            out.push(format!(
                                "Effect::continuous(&Filter::This, Modifier::ProtectionFrom(&{from}), {duration})"
                            ));
                        }
                        if out.is_empty() {
                            return None;
                        }
                        out
                    }
                    None => return None,
                }
            }
            "Counter" => {
                let defined = p.take("Defined");
                let inherited_target = match defined.as_deref() {
                    None => false,
                    Some("Targeted" | "ParentTarget") if !targets_here && target.is_some() => true,
                    Some(_) => return None,
                };
                let dest = p.take("Destination");
                let target_type = p.take("TargetType");
                let is_exile = match dest.as_deref() {
                    None | Some("Graveyard") => false,
                    Some("Exile") => true,
                    Some(_) => return None,
                };
                // The spec `counter_or_stack_target` read from `TargetType$`
                // (or the parent line's, when inherited) says what is countered.
                if target_type.is_none() && !(inherited_target || targets_here) {
                    return None;
                }
                match target {
                    Some(t) if t.starts_with("TargetSpec::Spell(") => {
                        if is_exile {
                            vec!["Effect::CounterTargetSpellToExile".to_string()]
                        } else {
                            vec!["Effect::CounterTargetSpell".to_string()]
                        }
                    }
                    Some(t)
                        if !is_exile
                            && (t.starts_with("TargetSpec::AbilityOnStack(")
                                || t.starts_with("TargetSpec::ActivatedAbility(")) =>
                    {
                        vec!["Effect::CounterTargetAbility".to_string()]
                    }
                    Some(t) if !is_exile && t.starts_with("TargetSpec::SpellOrAbility(") => {
                        vec!["Effect::CounterTargetSpellOrAbility".to_string()]
                    }
                    _ => return None,
                }
            }
            "Attach" => {
                if !matches!(p.take("Object").as_deref(), None | Some("Self")) {
                    return None;
                }
                if let Some(def) = p.take("Defined") {
                    if target.is_some() || !self.is_event_object_defined(&def) {
                        return None;
                    }
                    vec!["Effect::AttachSelf { target: TargetSpec::EventObject }".to_string()]
                } else {
                    let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
                    if !targets_here {
                        return None;
                    }
                    vec![format!("Effect::AttachSelf {{ target: {tgt} }}")]
                }
            }
            "Poison" => {
                let n = self.amount_or_count_ext(&p.take("Num")?)?;
                let who = self.wave11_player_of_line(
                    p.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                vec![format!("Effect::Poison {{ who: {who}, amount: {n} }}")]
            }
            "Goad" => self.goad_effect(p, target, targets_a_player)?,
            "Connive" => self.connive_effect(p, target, targets_here, single_target)?,
            "Explore" => self.explore_effect(p, target, targets_here, single_target)?,
            "Discover" => self.discover_effect(p, target, targets_here, prior_target_used)?,
            "SetLife" => self.set_life_effect(p, target, targets_a_player, prior_target_used)?,
            "WinsGame" => {
                if target.is_some() || p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec!["Effect::WinGame".to_string()]
            }
            _ => return None,
        })
    }

    fn collect_counter_specs<'a>(
        codes: &'a str,
        amount: &str,
    ) -> Result<Vec<(String, String)>, &'a str> {
        let fixed_base = amount
            .strip_prefix("Amount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u32>().ok());
        let mut out: Vec<(String, String)> = Vec::new();
        let mut fixed_counts: Vec<(String, u32)> = Vec::new();
        for raw in codes.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let Some(kind) = counter_kind(raw) else {
                return Err(raw);
            };
            if let Some(base) = fixed_base {
                if let Some((_, count)) = fixed_counts.iter_mut().find(|(k, _)| *k == kind) {
                    *count = count.checked_add(base).ok_or(raw)?;
                } else {
                    fixed_counts.push((kind, base));
                }
            } else {
                out.push((kind, amount.to_string()));
            }
        }
        if fixed_base.is_some() {
            if fixed_counts.is_empty() {
                return Err(codes);
            }
            Ok(fixed_counts
                .into_iter()
                .map(|(k, n)| (k, format!("Amount::Fixed({n})")))
                .collect())
        } else if out.is_empty() {
            Err(codes)
        } else {
            Ok(out)
        }
    }

    fn connive_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let player_only_target = target.is_some_and(|t| {
            matches!(
                t,
                "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::PlayerWhoAttackedThisTurn"
            ) || t.starts_with("TargetSpec::Player(")
                || t.starts_with("TargetSpec::OpponentWithMoreThan")
        });
        let ok = match defined.as_deref() {
            None if targets_here => {
                single_target && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            }
            Some("Targeted" | "ParentTarget") if !targets_here => {
                single_target && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            }
            None | Some("Self") if !targets_here => {
                !self.on_a_spell && (target.is_none() || player_only_target)
            }
            _ => false,
        };
        if !ok {
            return None;
        }
        let amount = match p.take("ConniveNum").as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(raw)?,
        };
        Some(vec![format!("Effect::Connive {{ amount: {amount} }}")])
    }

    fn explore_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let subject = match defined.as_deref() {
            None if targets_here => {
                if !single_target {
                    return None;
                }
                let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
                tgt.to_string()
            }
            Some("Targeted" | "ParentTarget") if !targets_here => {
                if !single_target {
                    return None;
                }
                let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
                tgt.to_string()
            }
            None | Some("Self") if !targets_here && !self.on_a_spell => {
                "TargetSpec::ThisObject".to_string()
            }
            Some(def)
                if !targets_here
                    && !matches!(
                        self.trigger_mode.as_deref(),
                        Some("SpellCast" | "SpellCastOrCopy")
                    )
                    && self.is_event_object_defined(def) =>
            {
                "TargetSpec::EventObject".to_string()
            }
            _ => return None,
        };
        let times: usize = match p.take("Num").as_deref() {
            None => 1,
            Some(raw) => raw
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|&n| (1..=4).contains(&n))?,
        };
        Some(vec![
            format!("Effect::Explore {{ target: {subject} }}");
            times
        ])
    }

    fn discover_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || (target.is_some() && !prior_target_used)
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        let raw = p.take("Num")?;
        if let Ok(n) = raw.trim().parse::<u8>() {
            return Some(vec![format!("Effect::Discover {{ mana_value: {n} }}")]);
        }
        let amount = self.amount_or_count_ext(&raw)?;
        Some(vec![format!(
            "Effect::DiscoverAmount {{ mana_value: {amount} }}"
        )])
    }

    fn set_life_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        let amount = self.amount_or_count_ext(&p.take("LifeAmount")?)?;
        let defined = p.take("Defined");
        let who = if defined.is_none() && !targets_a_player {
            if target.is_some() && !prior_target_used {
                return None;
            }
            "PlayerRel::You"
        } else {
            self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?
        };
        Some(vec![format!(
            "Effect::SetLifeTotal {{ who: {who}, amount: {amount} }}"
        )])
    }

    fn is_power_of(&self, raw: &str, expected: &[&str]) -> bool {
        let resolved = self
            .svars
            .get(raw.trim())
            .map_or(raw.trim(), String::as_str)
            .trim();
        expected.contains(&resolved)
    }

    fn fight_effect(&self, p: &mut Params, second_target: bool) -> Option<Vec<String>> {
        let may = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let (fighter, foe) = if second_target {
            if p.take("Defined").as_deref() != Some("ParentTarget") {
                return None;
            }
            ("TargetSlot::First", "TargetSlot::Second")
        } else {
            match p.take("Defined").as_deref() {
                Some("Self") if !self.on_a_spell => {}
                Some("TriggeredCardLKICopy" | "TriggeredCard") if self.is_etb_self_trigger => {}
                _ => return None,
            }
            ("TargetSlot::This", "TargetSlot::First")
        };
        let effect = format!("Effect::Fight {{ fighter: {fighter}, foe: {foe} }}");
        Some(if may {
            vec![format!("Effect::MayDo {{ effects: &[{effect}] }}")]
        } else {
            vec![effect]
        })
    }

    fn bite_effect(&self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Defined").is_some() || p.take("DamageSource").as_deref() != Some("ParentTarget")
        {
            return None;
        }
        let num_dmg = p.take("NumDmg")?;
        if !self.is_power_of(
            &num_dmg,
            &["ParentTargeted$CardPower", "Targeted$CardPower"],
        ) {
            return None;
        }
        let may = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let effect =
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::Second }"
                .to_string();
        Some(if may {
            vec![format!("Effect::MayDo {{ effects: &[{effect}] }}")]
        } else {
            vec![effect]
        })
    }

    #[allow(clippy::too_many_lines)]
    fn copy_spell_ability_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        // CR 111.10 / CR 111.13: a cast trigger whose Oracle creates a token
        // copy directly on the battlefield (e.g. Osgood, Operation Double) is
        // not a stack spell copy (`Effect::CopySpellOrAbility`).
        let oracle_lower = self.oracle.to_ascii_lowercase();
        if oracle_lower.contains("create a token that's a copy")
            && !oracle_lower.contains("copy target")
            && !oracle_lower.contains("copy that spell")
            && !oracle_lower.contains("copy it")
        {
            return self.deny(
                "`CopySpellAbility` on a card whose Oracle creates a token copy directly"
                    .to_string(),
            );
        }
        let may_choose = match p.take("MayChooseTarget").as_deref() {
            Some("True") => true,
            Some("False") => false,
            None => {
                let lower = self.oracle.to_ascii_lowercase();
                if lower.contains("you may choose new targets")
                    || lower.contains("may choose new targets for")
                {
                    true
                } else if lower.contains("choose new target") {
                    return None;
                } else {
                    false
                }
            }
            Some(_) => return None,
        };
        let optional = match p.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let controller = self.player_of_line(p.take("Controller").as_deref(), target, false)?;
        let mods = self.copy_mods(p, false)?;
        let target_type = p.take("TargetType");
        let defined = p.take("Defined");
        let raw_num = p.take("NumCopies").or_else(|| p.take("Amount"));
        let plain_small = raw_num
            .as_deref()
            .and_then(|r| usize::try_from(self.plain_num(r)?).ok())
            .filter(|&n| (1..=4).contains(&n));
        let is_legacy = may_choose
            && controller == "PlayerRel::You"
            && mods.is_empty()
            && (raw_num.is_none() || plain_small.is_some());
        if is_legacy
            && let Some((effect, n)) = self.legacy_copy_spell_ability(
                target_type.as_deref(),
                defined.as_deref(),
                raw_num.as_deref(),
                plain_small.unwrap_or(1),
                target,
                targets_here,
            )
        {
            let copies = vec![effect; n];
            return Some(if optional {
                vec![format!(
                    "Effect::MayDo {{ effects: &[{}] }}",
                    copies.join(", ")
                )]
            } else {
                copies
            });
        }
        let amount = match raw_num.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(raw)?,
        };
        let stack_target = target.filter(|t| {
            t.starts_with("TargetSpec::Spell(")
                || t.starts_with("TargetSpec::AbilityOnStack(")
                || t.starts_with("TargetSpec::SpellOrAbility(")
        });
        let copy_target = match (target_type.as_deref(), defined.as_deref()) {
            (
                Some(
                    "Spell"
                    | "Activated"
                    | "Activated,Triggered"
                    | "Triggered,Activated"
                    | "Activated.YouCtrl,Triggered.YouCtrl"
                    | "Triggered.YouCtrl,Activated.YouCtrl"
                    | "Spell,Activated,Triggered"
                    | "Activated,Triggered,Spell"
                    | "Spell,Triggered,Activated",
                ),
                None,
            ) if targets_here => stack_target?.to_string(),
            (None, Some("Targeted" | "ParentTarget")) => stack_target?.to_string(),
            (None, Some("TriggeredSpellAbility"))
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                ) && !targets_here
                    && target.is_none() =>
            {
                "TargetSpec::EventObject".to_string()
            }
            (None, Some("Parent")) if self.on_a_spell && !targets_here => {
                "TargetSpec::ThisObject".to_string()
            }
            _ => return None,
        };
        let effect = format!(
            "Effect::CopySpellOrAbility {{ target: {copy_target}, controller: {controller}, \
             amount: {amount}, mods: &[{}], may_choose_new_targets: {may_choose} }}",
            mods.join(", ")
        );
        Some(vec![Self::wrap_optional_copy_for(
            optional, controller, effect,
        )?])
    }

    fn wrap_optional_copy_for(optional: bool, controller: &str, effect: String) -> Option<String> {
        if !optional {
            return Some(effect);
        }
        if controller == "PlayerRel::You" {
            return Some(format!("Effect::MayDo {{ effects: &[{effect}] }}"));
        }
        matches!(
            controller,
            "PlayerRel::Chosen"
                | "PlayerRel::ControllerOfTarget"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::TargetPlayerOrController"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::DefendingPlayer"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::OwnerOfTarget"
        )
        .then(|| format!("Effect::MayDoFor {{ player: {controller}, effects: &[{effect}] }}"))
    }

    fn legacy_copy_spell_ability(
        &mut self,
        target_type: Option<&str>,
        defined: Option<&str>,
        raw_num: Option<&str>,
        count: usize,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<(String, usize)> {
        match (target_type, defined) {
            (Some("Spell"), None)
                if targets_here && target.is_some_and(|t| t.starts_with("TargetSpec::Spell(")) =>
            {
                Some(("Effect::CopyTargetSpell { mods: &[] }".to_string(), count))
            }
            (
                Some(
                    "Activated"
                    | "Activated,Triggered"
                    | "Triggered,Activated"
                    | "Activated.YouCtrl,Triggered.YouCtrl"
                    | "Triggered.YouCtrl,Activated.YouCtrl",
                ),
                None,
            ) if targets_here
                && target.is_some_and(|t| t.starts_with("TargetSpec::AbilityOnStack(")) =>
            {
                Some(("Effect::CopyTargetAbility".to_string(), count))
            }
            (None, Some("TriggeredSpellAbility"))
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                ) && !targets_here
                    && target.is_none() =>
            {
                Some(("Effect::CopyEventSpellOrAbility".to_string(), count))
            }
            (None, Some("Parent")) if self.on_a_spell && !targets_here => {
                let amount = match raw_num {
                    None => "Amount::Fixed(1)".to_string(),
                    Some(r) => self.amount_or_count_ext(r)?,
                };
                Some((format!("Effect::CopyThisSpell {{ amount: {amount} }}"), 1))
            }
            _ => None,
        }
    }

    pub(super) fn sub_is_api(&self, sub: Option<&str>, expected: &str) -> bool {
        sub.and_then(|name| self.svars.get(name.trim()))
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, _)| api == expected)
    }

    /// Wave-7/9b/16c/21b named conditions (`Condition$ Surge/Prowl/Blitz/AltCost/Bargain/Prototype/Warp/Monarch/Initiative/CompletedDungeon`).
    pub(super) fn wave7_named_condition(cond: &str) -> Option<&'static str> {
        Some(match cond.trim() {
            "Surge" => "Condition::Surge",
            "Prowl" => "Condition::Prowl",
            "Blitz" => "Condition::BlitzCostPaid",
            "AltCost" => "Condition::AltCostPaid",
            "Bargain" | "Bargained" => "Condition::Bargained",
            "Prototype" | "Prototyped" => "Condition::Prototyped",
            "Warp" | "Warped" => "Condition::WarpCostPaid",
            "Monarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" | "DungeonCompleted" => {
                "Condition::CompletedADungeon(PlayerRel::You)"
            }
            _ => return None,
        })
    }

    fn bargained_or_prototyped_pair(&self, raw: &str) -> Option<(&'static str, String, String)> {
        let def = self.svars.get(raw.trim())?.trim();
        let (cond, rest) = if let Some(rest) = def
            .strip_prefix("Count$Bargained.")
            .or_else(|| def.strip_prefix("Count$Bargain."))
        {
            ("Condition::Bargained", rest)
        } else if let Some(rest) = def.strip_prefix("Count$PromisedGift.") {
            ("Condition::GiftWasPromised", rest)
        } else if let Some(rest) = def
            .strip_prefix("Count$Warped.")
            .or_else(|| def.strip_prefix("Count$Warp."))
        {
            ("Condition::WarpCostPaid", rest)
        } else if let Some(rest) = def.strip_prefix("Count$Monarch.") {
            ("Condition::IsMonarch(PlayerRel::You)", rest)
        } else if let Some(rest) = def.strip_prefix("Count$Initiative.") {
            ("Condition::HasInitiative(PlayerRel::You)", rest)
        } else {
            let rest = def
                .strip_prefix("Count$Prototyped.")
                .or_else(|| def.strip_prefix("Count$Prototype."))?;
            ("Condition::Prototyped", rest)
        };
        let (a, b) = rest.split_once('.')?;
        if a.is_empty() || b.is_empty() || b.contains('.') {
            return None;
        }
        Some((cond, a.to_string(), b.to_string()))
    }

    /// Splits a line carrying `Count$Bargained.<A>.<B>` or
    /// `Count$Prototyped.<A>.<B>` into `Effect::IfCondition`.
    #[allow(clippy::too_many_lines)]
    fn try_split_bargained_or_prototyped_amount(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        chain: &Chain,
    ) -> Result<Option<Vec<String>>, ()> {
        if api == "Branch"
            || chain.second_target.is_some()
            || p.has("Condition")
            || p.has("ConditionCheckSVar")
            || p.has("ConditionPresent")
            || p.has("ConditionDefined")
        {
            return Ok(None);
        }
        let mut chosen_cond: Option<&'static str> = None;
        let mut p_then = p.clone();
        let mut p_else = p.clone();
        for idx in 0..p.entries.len() {
            let value = p.entries[idx].1.trim();
            let (sign, core) = if let Some(rest) = value.strip_prefix('-') {
                ("-", rest)
            } else if let Some(rest) = value.strip_prefix('+') {
                ("+", rest)
            } else {
                ("", value)
            };
            if let Some((cond, a, b)) = self.bargained_or_prototyped_pair(core) {
                if let Some(prev) = chosen_cond
                    && prev != cond
                {
                    return Ok(None);
                }
                chosen_cond = Some(cond);
                p_then.entries[idx].1 = if a == "0" {
                    "0".to_string()
                } else {
                    format!("{sign}{a}")
                };
                p_else.entries[idx].1 = if b == "0" {
                    "0".to_string()
                } else {
                    format!("{sign}{b}")
                };
            }
        }
        let Some(cond) = chosen_cond else {
            return Ok(None);
        };
        let single_target = chain.target_exact.is_none()
            && !chain.target_is_x
            && chain.target_up_to.is_none_or(|n| n == 1);
        let prior_target_used = !targets_here && !chain.effects.is_empty();
        let flags = LineFlags {
            targets_here,
            targets_a_player,
            single_target,
            prior_target_used,
        };
        let then_raw = self.effect_of_inner(api, &mut p_then, target, flags);
        let else_raw = self.effect_of_inner(api, &mut p_else, target, flags);
        let (Some(mut then_effects), Some(mut else_effects)) = (then_raw, else_raw) else {
            if self.unclaimed.borrow().is_none() {
                if super::is_supported_api(api) {
                    self.note(format!("unreadable value in `{api}`"));
                } else {
                    self.note(format!("effect `{api}`"));
                }
            }
            return Err(());
        };
        if let Some(key) = p_then.first_key().or_else(|| p_else.first_key()) {
            self.note(format!("unclaimed parameter `{api}.{key}`"));
            return Err(());
        }
        let is_noop_pump = |list: &[String]| {
            matches!(
                list,
                [one] if one.starts_with(
                    "Effect::PumpTarget { power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY,"
                )
            )
        };
        if is_noop_pump(&else_effects) && !then_effects.is_empty() {
            else_effects.clear();
        } else if is_noop_pump(&then_effects) && !else_effects.is_empty() {
            then_effects.clear();
        }
        if then_effects.is_empty() && else_effects.is_empty() {
            self.note(format!("unreadable value in `{api}`"));
            return Err(());
        }
        p.entries.clear();
        Ok(Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
            then_effects.join(", "),
            else_effects.join(", ")
        )]))
    }

    /// Wave-7 `CheckSVar$` / `ConditionCheckSVar$` conditions for Wave 6b
    /// alternative-cost & opponent-life-loss clauses.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave7_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let trimmed = var.trim();
        let raw = if let Some(body) = self.svars.get(trimmed) {
            body.trim()
        } else if trimmed.starts_with("Count$")
            || trimmed.starts_with("DungeonsCompleted$")
            || trimmed.starts_with("PlayerCount")
        {
            trimmed
        } else {
            return None;
        };
        let cmp = cmp.trim();
        match raw {
            "Count$Monarch.1.0" | "Count$Monarch" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::IsMonarch(PlayerRel::You)".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::IsMonarch(PlayerRel::You))".to_string())
                }
                _ => None,
            },
            "Count$Initiative.1.0" | "Count$Initiative" => match cmp {
                "GE1" | "EQ1" | "GT0" => {
                    Some("Condition::HasInitiative(PlayerRel::You)".to_string())
                }
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::HasInitiative(PlayerRel::You))".to_string())
                }
                _ => None,
            },
            "DungeonsCompleted$Amount" => match cmp {
                "GE1" | "GT0" => Some("Condition::CompletedADungeon(PlayerRel::You)".to_string()),
                "EQ0" | "LT1" | "LE0" => Some(
                    "Condition::Not(&Condition::CompletedADungeon(PlayerRel::You))".to_string(),
                ),
                _ => None,
            },
            "PlayerCountPlayers$HasPropertyisMonarch" => match cmp {
                "GE1" | "GT0" => Some("Condition::IsMonarch(PlayerRel::EachPlayer)".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::IsMonarch(PlayerRel::EachPlayer))".to_string())
                }
                _ => None,
            },
            "PlayerCountPlayers$HasPropertyhasInitiative" => match cmp {
                "GE1" | "GT0" => {
                    Some("Condition::HasInitiative(PlayerRel::EachPlayer)".to_string())
                }
                "EQ0" | "LT1" | "LE0" => Some(
                    "Condition::Not(&Condition::HasInitiative(PlayerRel::EachPlayer))".to_string(),
                ),
                _ => None,
            },
            "PlayerCountPropertyYou$DomainPlayer" => {
                let amt = "Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)";
                if let Some(n_raw) = cmp.strip_prefix("GE") {
                    let n = n_raw.parse::<u32>().ok()?;
                    (n >= 1).then(|| format!("Condition::AmountAtLeast({amt}, {n})"))
                } else if let Some(n_raw) = cmp.strip_prefix("GT") {
                    let n = n_raw.parse::<u32>().ok()?.checked_add(1)?;
                    Some(format!("Condition::AmountAtLeast({amt}, {n})"))
                } else if let Some(n_raw) = cmp.strip_prefix("LE") {
                    let n = n_raw.parse::<u32>().ok()?.checked_add(1)?;
                    Some(format!(
                        "Condition::Not(&Condition::AmountAtLeast({amt}, {n}))"
                    ))
                } else if let Some(n_raw) = cmp.strip_prefix("LT") {
                    let n = n_raw.parse::<u32>().ok()?;
                    (n >= 1).then(|| {
                        format!("Condition::Not(&Condition::AmountAtLeast({amt}, {n}))")
                    })
                } else if cmp == "EQ0" {
                    Some(format!(
                        "Condition::Not(&Condition::AmountAtLeast({amt}, 1))"
                    ))
                } else if let Some(n_raw) = cmp.strip_prefix("EQ") {
                    let n = n_raw.parse::<u32>().ok()?;
                    Some(format!(
                        "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::Equal, \
                         right: &Amount::Fixed({n}) }}"
                    ))
                } else {
                    None
                }
            }
            "Count$AltCost.1.0" | "Count$AltCost" => match cmp {
                "GE1" | "EQ1" => Some("Condition::AltCostPaid".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::AltCostPaid)".to_string()),
                _ => None,
            },
            "Count$AltCost.2.0" => match cmp {
                "GE1" | "GE2" | "EQ2" => Some("Condition::AltCostPaid".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::AltCostPaid)".to_string()),
                _ => None,
            },
            "Count$AltCost.0.2" => match cmp {
                "EQ0" | "LT1" => Some("Condition::AltCostPaid".to_string()),
                "GE1" | "GE2" | "EQ2" => {
                    Some("Condition::Not(&Condition::AltCostPaid)".to_string())
                }
                _ => None,
            },
            "Count$Blitz.1.0" | "Count$Blitz" => match cmp {
                "GE1" | "EQ1" => Some("Condition::BlitzCostPaid".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::BlitzCostPaid)".to_string()),
                _ => None,
            },
            "Count$Surge.1.0" | "Count$Surge" => match cmp {
                "GE1" | "EQ1" => Some("Condition::Surge".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::Surge)".to_string()),
                _ => None,
            },
            "Count$Prowl.1.0" | "Count$Prowl" => match cmp {
                "GE1" | "EQ1" => Some("Condition::Prowl".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::Prowl)".to_string()),
                _ => None,
            },
            "Count$Bargained.1.0" | "Count$Bargained" | "Count$Bargain.1.0" | "Count$Bargain"
                if self.body.bargain =>
            {
                match cmp {
                    "GE1" | "EQ1" | "GT0" => Some("Condition::Bargained".to_string()),
                    "EQ0" | "LT1" => Some("Condition::Not(&Condition::Bargained)".to_string()),
                    _ => None,
                }
            }
            "Count$PromisedGift.1.0" | "Count$PromisedGift" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::GiftWasPromised".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::GiftWasPromised)".to_string()),
                _ => None,
            },
            "Count$PromisedGift.0.1" => match cmp {
                "EQ0" | "LT1" => Some("Condition::GiftWasPromised".to_string()),
                "GE1" | "EQ1" | "GT0" => {
                    Some("Condition::Not(&Condition::GiftWasPromised)".to_string())
                }
                _ => None,
            },
            "Count$Prototyped.1.0"
            | "Count$Prototyped"
            | "Count$Prototype.1.0"
            | "Count$Prototype" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::Prototyped".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::Prototyped)".to_string()),
                _ => None,
            },
            "Count$LifeOppsLostThisTurn" => {
                if matches!(cmp, "EQ0" | "LT1") {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                } else if matches!(cmp, "GE1" | "GT0") {
                    Some("Condition::OpponentLostLifeThisTurn".to_string())
                } else if let Some(n) = cmp
                    .strip_prefix("GE")
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n >= 2)
                {
                    Some(format!(
                        "Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, {n})"
                    ))
                } else if let Some(n) = cmp
                    .strip_prefix("GT")
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n >= 1)
                {
                    let min = n.checked_add(1)?;
                    Some(format!(
                        "Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, {min})"
                    ))
                } else {
                    None
                }
            }
            "Count$LifeOppsLostThisTurn/LimitMax.1" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::OpponentLostLifeThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                }
                _ => None,
            },
            "Count$Spectacle.1.0"
            | "Count$Spectacle"
            | "PlayerCountOpponents$HasPropertyLostLifeThisTurn"
            | "PlayerCountRegisteredOpponents$HasPropertyLostLifeThisTurn" => match cmp {
                "GE1" | "EQ1" => Some("Condition::OpponentLostLifeThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                }
                _ => None,
            },
            "Count$Morbid.1.0" | "Count$Morbid" => match cmp {
                "GE1" | "EQ1" => Some("Condition::CreaturesDiedThisTurnAtLeast(1)".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string())
                }
                _ => None,
            },
            "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" => {
                if matches!(cmp, "EQ0" | "LT1") {
                    Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string())
                } else if let Some(n) = cmp
                    .strip_prefix("GE")
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n >= 1)
                {
                    Some(format!("Condition::CreaturesDiedThisTurnAtLeast({n})"))
                } else if let Some(n) = cmp.strip_prefix("GT").and_then(|s| s.parse::<u32>().ok()) {
                    let min = n.checked_add(1)?;
                    Some(format!("Condition::CreaturesDiedThisTurnAtLeast({min})"))
                } else {
                    None
                }
            }
            "Count$ThisTurnCast_Card.YouCtrl" | "Count$ThisTurnCast_Card.YouCtrl/LimitMax.1" => {
                match cmp {
                    "EQ0" | "LT1" => Some("Condition::YouCastNoSpellThisTurn".to_string()),
                    "GE1" | "GT0" | "EQ1" => {
                        Some("Condition::Not(&Condition::YouCastNoSpellThisTurn)".to_string())
                    }
                    _ => None,
                }
            }
            "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl"
            | "Count$ThisTurnCast_Sorcery.YouCtrl,Instant.YouCtrl" => match cmp {
                "GE1" | "GT0" => Some("Condition::CastInstantOrSorceryThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::CastInstantOrSorceryThisTurn)".to_string())
                }
                _ => None,
            },
            "Count$ThisTurnCast_Creature.YouCtrl"
            | "Count$ThisTurnCast_Creature.YouCtrl/LimitMax.1" => match cmp {
                "GE1" | "GT0" | "EQ1" => Some("Condition::CastCreatureThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::CastCreatureThisTurn)".to_string())
                }
                _ => None,
            },
            "Count$LifeYouGainedThisTurn" => Self::amount_at_least_cmp(
                "Amount::LifeGainedThisTurn(PlayerRel::You)",
                cmp,
            ),
            "Count$LifeYouGainedThisTurn/LimitMax.1" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some(
                    "Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 1)"
                        .to_string(),
                ),
                "EQ0" | "LT1" => Some(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 1))"
                        .to_string(),
                ),
                _ => None,
            },
            "Count$DamageYouTookThisTurn" => {
                Self::amount_at_least_cmp("Amount::DamageDealtToYouThisTurn", cmp)
            }
            "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" => Self::amount_at_least_cmp(
                "Amount::CreaturesEnteredUnderYourControlThisTurn",
                cmp,
            ),
            _ => {
                if let Some(n) = raw
                    .strip_prefix("PlayerCount$HasPropertyHasCardsInHand_Card_LE")
                    .and_then(|s| s.parse::<u8>().ok())
                {
                    match cmp {
                        "GE1" | "GT0" => Some(format!("Condition::APlayerHandSizeAtMost({n})")),
                        "EQ0" | "LT1" => {
                            Some(format!("Condition::Not(&Condition::APlayerHandSizeAtMost({n}))"))
                        }
                        _ => None,
                    }
                } else if let Some(sub_raw) = raw
                    .strip_prefix("Count$ThisTurnEntered_Battlefield_")
                    .and_then(|s| s.strip_suffix(".YouCtrl"))
                    && let Some(subtype) = self.single_subtype_const(sub_raw.trim())
                {
                    match cmp {
                        "GE1" | "GT0" => Some(format!(
                            "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{subtype}])"
                        )),
                        "EQ0" | "LT1" => Some(format!(
                            "Condition::Not(&Condition::EnteredSubtypeUnderYourControlThisTurn(&[{subtype}]))"
                        )),
                        _ => None,
                    }
                } else if raw == "Count$xPaid" && self.trigger_mode.is_none() {
                    if let Some(n) = cmp
                        .strip_prefix("GE")
                        .and_then(|s| s.parse::<u32>().ok())
                        .filter(|&n| n >= 1)
                    {
                        Some(format!("Condition::XAtLeast({n})"))
                    } else if let Some(n) = cmp
                        .strip_prefix("GT")
                        .and_then(|s| s.parse::<u32>().ok())
                    {
                        Some(format!("Condition::XAtLeast({})", n.checked_add(1)?))
                    } else if matches!(cmp, "EQ0" | "LT1") {
                        Some("Condition::Not(&Condition::XAtLeast(1))".to_string())
                    } else {
                        None
                    }
                } else {
                    Self::wave16_svar_condition(raw, cmp)
                }
            }
        }
    }

    fn amount_at_least_cmp(amount_expr: &str, cmp: &str) -> Option<String> {
        if matches!(cmp, "EQ0" | "LT1") {
            Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({amount_expr}, 1))"
            ))
        } else if let Some(n) = cmp
            .strip_prefix("GE")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&n| n >= 1)
        {
            Some(format!("Condition::AmountAtLeast({amount_expr}, {n})"))
        } else if let Some(n) = cmp.strip_prefix("GT").and_then(|s| s.parse::<u32>().ok()) {
            let min = n.checked_add(1)?;
            Some(format!("Condition::AmountAtLeast({amount_expr}, {min})"))
        } else {
            None
        }
    }

    fn can_two_target_only(&self, sub: Option<&str>, chain: &Chain, second_spec: &str) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        if sp.has("ValidTgts")
            || sp.peek("Defined") != Some("Targeted")
            || self.later_line_targets(sp.peek("SubAbility"))
        {
            return false;
        }
        let first_spec = chain.target.as_deref().unwrap_or_default();
        let both_obj = first_spec.starts_with("TargetSpec::Object(")
            && second_spec.starts_with("TargetSpec::Object(");
        let is_dmg_spec = |s: &str| {
            s.starts_with("TargetSpec::Object(")
                || matches!(
                    s,
                    "TargetSpec::AnyTarget"
                        | "TargetSpec::AnyPlayer"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::PlayerOrPlaneswalker"
                        | "TargetSpec::CreatureOrPlaneswalker"
                )
        };
        match api.as_str() {
            "Destroy" | "Tap" | "Untap" => both_obj,
            "Pump" => {
                both_obj
                    && ["NumAtt", "NumDef"].iter().all(|key| {
                        sp.peek(key)
                            .is_none_or(|value| value.parse::<i16>().is_ok())
                    })
            }
            "DealDamage" => {
                sp.peek("DamageSource").is_none()
                    && is_dmg_spec(first_spec)
                    && is_dmg_spec(second_spec)
            }
            _ => false,
        }
    }

    fn two_target_only_followup(
        &mut self,
        sub: &mut Option<String>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        let sub_name = sub.take()?;
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut dp) = Params::parse(&body)?;
        dp.drop_prose();
        if dp.has("ValidTgts") || dp.take("Defined").as_deref() != Some("Targeted") {
            return None;
        }
        dp.take("IsCurse");
        let next = dp.take("SubAbility");
        let out = match api.as_str() {
            "Destroy" => {
                let no_regen = match dp.take("NoRegen").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                vec![format!(
                    "Effect::DestroyTargetSlots {{ targets: &[TargetSlot::First, TargetSlot::Second], no_regen: {no_regen} }}"
                )]
            }
            "Tap" => vec![
                "Effect::TapTargetSlot(TargetSlot::First)".to_string(),
                "Effect::TapTargetSlot(TargetSlot::Second)".to_string(),
            ],
            "Untap" => vec![
                "Effect::UntapTargetSlot(TargetSlot::First)".to_string(),
                "Effect::UntapTargetSlot(TargetSlot::Second)".to_string(),
            ],
            "Pump" => {
                let first_spec = chain.target.as_deref()?;
                let second_spec = chain.second_target.as_deref()?;
                let mut dp_second = dp.clone();
                let mut combined = self.pump_effect(&mut dp, first_spec, false)?;
                let second_effs = self.pump_effect(&mut dp_second, second_spec, true)?;
                if !dp_second.exhausted() {
                    return None;
                }
                combined.extend(second_effs);
                combined
            }
            "DealDamage" => {
                if dp.take("DamageSource").is_some()
                    || dp.take("DamageMap").is_some_and(|v| v != "True")
                {
                    return None;
                }
                let amt = self.amount_or_count_ext(&dp.take("NumDmg")?)?;
                vec![format!(
                    "Effect::DealDamageToBothTargetInstances {{ amount: {amt} }}"
                )]
            }
            _ => return None,
        };
        if !dp.exhausted() || chain.second_target.is_none() {
            return None;
        }
        *sub = next;
        Some(out)
    }

    fn strip_opp_ctrl_from_valid(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let (base, rest) = alt.split_once('.')?;
            let mut saw_opp = false;
            let mut kept = Vec::new();
            for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if matches!(atom, "OppCtrl" | "YouDontCtrl") {
                    saw_opp = true;
                } else {
                    kept.push(atom);
                }
            }
            if !saw_opp {
                return None;
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    fn same_valid_atoms(a: &str, b: &str) -> bool {
        let normalize = |s: &str| -> Vec<(String, Vec<String>)> {
            let mut alts: Vec<(String, Vec<String>)> = s
                .split(',')
                .map(|alt| {
                    let alt = alt.trim();
                    let (base, rest) = alt.split_once('.').unwrap_or((alt, ""));
                    let mut atoms: Vec<String> = rest
                        .split('+')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(str::to_string)
                        .collect();
                    atoms.sort();
                    (base.trim().to_string(), atoms)
                })
                .collect();
            alts.sort();
            alts
        };
        normalize(a) == normalize(b)
    }

    /// Reads `Draw<N/You>` used as a loot cost or switched `UnlessCost$`.
    fn draw_cost_effect(&mut self, raw: &str) -> Option<String> {
        let inner = raw.trim().strip_prefix("Draw<")?.strip_suffix('>')?;
        let (count_raw, who) = inner.split_once('/')?;
        if who.trim() != "You" {
            return None;
        }
        let amount = self.amount_or_count_ext(count_raw.trim())?;
        if let Some(fixed) = amount
            .strip_prefix("Amount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if fixed == "0" {
                return None;
            }
            Some(format!("Effect::draw({fixed})"))
        } else {
            Some(format!("Effect::DrawCards {{ amount: {amount} }}"))
        }
    }

    fn claim_followed_remember_flag(&self, api: &str, p: &mut Params, sub: Option<&str>) {
        if self.sub_is_api(sub, "ImmediateTrigger") {
            match api {
                "Mill" if p.peek("RememberMilled") == Some("True") => {
                    p.take("RememberMilled");
                }
                "Discard" if p.peek("RememberDiscarded") == Some("True") => {
                    p.take("RememberDiscarded");
                }
                "Sacrifice" if p.peek("RememberSacrificed") == Some("True") => {
                    p.take("RememberSacrificed");
                }
                "DigUntil" if p.peek("RememberFound") == Some("True") => {
                    p.take("RememberFound");
                }
                "ChangeZone" | "ChangeZoneAll"
                    if matches!(
                        (p.peek("Origin"), p.peek("Destination")),
                        (Some("Battlefield"), Some("Exile" | "Hand"))
                            | (Some("Graveyard"), Some("Exile"))
                    ) && p.peek("RememberChanged") == Some("True") =>
                {
                    p.take("RememberChanged");
                }
                _ => {}
            }
        } else if self.sub_is_api(sub, "DelayedTrigger") {
            match api {
                "Token" if p.peek("RememberTokens") == Some("True") => {
                    p.take("RememberTokens");
                }
                "ChangeZone"
                    if p.peek("Origin") == Some("Battlefield")
                        && p.peek("Destination") == Some("Exile")
                        && p.peek("RememberChanged") == Some("True") =>
                {
                    p.take("RememberChanged");
                }
                _ => {}
            }
        } else if p.peek("RememberChanged") == Some("True")
            && ((api == "ChangeZone"
                && sub.is_none()
                && p.peek("Origin") == Some("Battlefield")
                && p.peek("Destination") == Some("Exile")
                && self.wave24a_unused_linked_exile_remember())
                || (api == "ChangeZone"
                    && self.on_a_spell
                    && sub.is_some()
                    && p.peek("Origin") == Some("Library")
                    && p.peek("Destination") == Some("Battlefield")
                    && !self.has_linked_exile_reader()
                    && self.wave24a_no_svar_mentions_remembered()))
        {
            p.take("RememberChanged");
        }
    }

    pub(super) fn single_subtype_const(&self, word: &str) -> Option<String> {
        if word.is_empty() || word.contains(['.', '+', ',', ' ', '!']) {
            return None;
        }
        let expr = self.filter_expr(word)?;
        expr.strip_prefix("Filter::HasSubtype(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::to_string)
    }

    fn wave11_trigger_player_rel(&self, defined: &str) -> Option<&'static str> {
        let trigger = self.trigger_mode.as_deref();
        match defined {
            "TriggeredPlayer" | "TriggeredCardController" if trigger == Some("Sacrificed") => {
                Some("PlayerRel::ControllerOfEvent")
            }
            "TriggeredAttackingPlayer" | "TriggeredPlayer"
                if matches!(
                    trigger,
                    Some("Attacked" | "AttackersDeclared" | "AttackersDeclaredOneTarget")
                ) =>
            {
                Some("PlayerRel::EventPlayer")
            }
            "TriggeredPlayer"
                if matches!(
                    trigger,
                    Some(
                        "Drawn"
                            | "Discarded"
                            | "Cycled"
                            | "LandPlayed"
                            | "LifeGained"
                            | "LifeLost"
                            | "SearchedLibrary"
                            | "FlippedCoin"
                            | "RolledDie"
                            | "Exerted"
                            | "Enlisted"
                            | "BecomesPlotted"
                            | "Plotted"
                    )
                ) =>
            {
                Some("PlayerRel::EventPlayer")
            }
            "TriggeredDefender"
                if matches!(
                    trigger,
                    Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                ) =>
            {
                Some("PlayerRel::DefendingPlayer")
            }
            "TriggeredActivatingPlayer" | "TriggeredPlayer"
                if trigger == Some("SpellCastOrCopy") =>
            {
                Some("PlayerRel::EventPlayer")
            }
            "TriggeredSourceController" if trigger == Some("BecomesTarget") => {
                Some("PlayerRel::EventPlayer")
            }
            "EnchantedPlayer" | "Player.EnchantedBy" => Some("PlayerRel::EnchantedPlayer"),
            _ => None,
        }
    }

    pub(super) fn wave11_player_rel(&self, defined: Option<&str>) -> Option<&'static str> {
        if let Some(who) = defined
            && let Some(rel) = self.wave11_trigger_player_rel(who)
        {
            return Some(rel);
        }
        self.player_rel(defined)
    }

    fn split_you_and_player(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        if defined == Some("TargetedAndYou") && targets_a_player {
            return Some("PlayerRel::Chosen");
        }
        let (lhs, rhs) = defined?.split_once('&')?;
        let (("You", other_raw) | (other_raw, "You")) = (lhs.trim(), rhs.trim()) else {
            return None;
        };
        let other_rel = self.wave11_player_of_line(Some(other_raw), target, false)?;
        (other_rel != "PlayerRel::You").then_some(other_rel)
    }

    pub(super) fn wave11_player_of_line(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        if defined.is_none() && targets_a_player {
            return Some("PlayerRel::Chosen");
        }
        if defined == Some("ParentTarget")
            && target == Some("TargetSpec::Player(PlayerRel::Chosen)")
        {
            return Some("PlayerRel::Chosen");
        }
        if defined == Some("TargetedController")
            && target == Some("TargetSpec::CreatureOrPlaneswalker")
        {
            return Some("PlayerRel::ControllerOfTarget");
        }
        if let Some(who) = defined
            && let Some(rel) = self.wave11_trigger_player_rel(who)
        {
            return Some(rel);
        }
        self.player_of_line(defined, target, targets_a_player)
    }

    fn wave11_object_matches_condition(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        negated_key: bool,
    ) -> Option<String> {
        if p.has("PresentPlayer") || !matches!(p.peek("PresentZone"), None | Some("Battlefield")) {
            return None;
        }
        let defined = p.peek("ConditionDefined")?;
        let what = match defined {
            "Targeted" | "ParentTarget"
                if target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                "&TargetSpec::Object(&Filter::Any)"
            }
            "Self" if !self.on_a_spell => "&TargetSpec::ThisObject",
            other if self.is_event_object_defined(other) => "&TargetSpec::EventObject",
            _ => return None,
        };
        let key = if negated_key {
            "ConditionNotPresent"
        } else {
            "ConditionPresent"
        };
        let valid = p.peek(key)?.to_string();
        let cmp = p
            .peek("PresentCompare")
            .or_else(|| p.peek("ConditionCompare"))
            .map(str::to_string);
        let positive = match (negated_key, cmp.as_deref()) {
            (false, None | Some("GE1" | "EQ1" | "GT0")) => true,
            (true, None) | (false, Some("EQ0" | "LT1")) => false,
            _ => return None,
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("COND", &expr);
        p.take("ConditionDefined");
        p.take(key);
        p.take("PresentCompare");
        p.take("ConditionCompare");
        p.take("PresentZone");
        let base = format!("Condition::ObjectMatches {{ what: {what}, filter: &{filter} }}");
        Some(if positive {
            base
        } else {
            format!("Condition::Not(&{base})")
        })
    }

    pub(super) fn wave11_amount_compare_condition(
        &mut self,
        var: &str,
        cmp: &str,
        in_resolution: bool,
    ) -> Option<String> {
        let var = var.trim();
        let cmp = cmp.trim();
        if !self.svars.contains_key(var) || cmp.len() < 3 {
            return None;
        }
        let (op_code, rhs_raw) = cmp.split_at(2);
        let op = match op_code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        let left = self.amount_or_count_ext(var)?;
        if left.starts_with("Amount::Fixed(") || left.contains("Neg") {
            return None;
        }
        let right = if let Ok(n) = rhs_raw.parse::<u32>() {
            format!("Amount::Fixed({n})")
        } else {
            if !self.svars.contains_key(rhs_raw) {
                return None;
            }
            let r = self.amount_or_count_ext(rhs_raw)?;
            if r.contains("Neg") {
                return None;
            }
            r
        };
        if left == right {
            return None;
        }
        if !in_resolution
            && (!Self::is_non_resolution_safe_amount(&left)
                || !Self::is_non_resolution_safe_amount(&right))
        {
            return None;
        }
        Some(format!(
            "Condition::AmountCompare {{ left: &{left}, op: {op}, right: &{right} }}"
        ))
    }

    fn is_non_resolution_safe_amount(expr: &str) -> bool {
        const RESOLUTION_ONLY: &[&str] = &[
            "Amount::X",
            "Amount::DoubleX",
            "Amount::XPlus",
            "Amount::XIf",
            "Amount::Devotion",
            "Amount::TargetPower",
            "Amount::TargetToughness",
            "Amount::TargetCmc",
            "Amount::Event",
            "Amount::LastDieRoll",
            "Amount::CoinsWon",
            "Amount::TargetsPutIntoGraveyard",
            "Amount::ThisWay",
            "Amount::Sacrificed",
            "Amount::ManaSpent",
            "Amount::ManaTypeSpentOnX",
            "Amount::TappedPower",
            "PlayerRel::Chosen",
            "PlayerRel::ControllerOfTarget",
            "PlayerRel::TargetPlayerOrController",
            "PlayerRel::SecondTarget",
            "PlayerRel::Recipient",
            "PlayerRel::EventPlayer",
            "PlayerRel::DamagedPlayer",
            "PlayerRel::ControllerOfEvent",
            "PlayerRel::OwnerOfEvent",
        ];
        !RESOLUTION_ONLY.iter().any(|needle| expr.contains(needle))
    }

    fn search_library_then_untap_found(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.take("RememberChanged").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        if p.take("Origin").as_deref() != Some("Library")
            || p.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let mut out = self.search_library(p, "Battlefield", target, false)?;
        let sub_name = sub.take()?;
        let body = self.svars.get(sub_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if s_api != "Untap"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("ReplaceDyingDefined")
            || sp.has("AtEOT")
        {
            return None;
        }
        let mut next_sub = sp.take("SubAbility");
        let untap_effects = self.wrap_line_condition(
            "Untap",
            &mut sp,
            &mut next_sub,
            target,
            vec!["Effect::UntapFound".to_string()],
        )?;
        if !sp.exhausted() {
            return None;
        }
        out.extend(untap_effects);
        if let Some(clean_name) = next_sub.as_deref()
            && let Some(clean_body) = self.svars.get(clean_name.trim()).cloned()
            && let Some((c_api, mut cp)) = Params::parse(&clean_body)
        {
            cp.drop_prose();
            if c_api == "Cleanup" && cp.take("ClearRemembered").as_deref() == Some("True") {
                let after_clean = cp.take("SubAbility");
                if !cp.exhausted() {
                    return None;
                }
                next_sub = after_clean;
            }
        }
        *sub = next_sub;
        Some(out)
    }

    // -------------------------------------------------------------------------
    // Wave 24a: Branch, Excess/Divided chains, and RememberMilled/RememberChanged
    // -------------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn wave24a_branch_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        single_target: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if chain.target.as_deref().is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(")
                || t.starts_with("TargetSpec::AbilityOnStack(")
                || t.starts_with("TargetSpec::SpellOrAbility(")
                || t.starts_with("TargetSpec::ActivatedAbility(")
        }) {
            p.take("TargetType");
        }
        let var = p.take("BranchConditionSVar")?;
        let cmp = p
            .take("BranchConditionSVarCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let true_sub = p.take("TrueSubAbility");
        let false_sub = p.take("FalseSubAbility");
        if true_sub.is_none() && false_sub.is_none() {
            return None;
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Branch.{key}`"));
        }
        let (cond, needs_target_only_else) =
            self.wave24a_branch_condition(&var, &cmp, target, single_target)?;
        if needs_target_only_else
            && chain.target_up_to.is_some()
            && let Some(ref f_name) = false_sub
            && !self.wave24a_sub_only_acts_on_target(f_name)
        {
            return None;
        }
        let mut run_branch = |sub_name: Option<String>| -> Option<Vec<String>> {
            let Some(name) = sub_name else {
                return Some(Vec::new());
            };
            let body = self.svars.get(name.trim())?.clone();
            let mut branch_chain = Chain {
                target: chain.target.clone(),
                target_is_x: chain.target_is_x,
                target_up_to: chain.target_up_to,
                target_exact: chain.target_exact,
                target_unique: chain.target_unique,
                second_target: chain.second_target.clone(),
                second_target_up_to: chain.second_target_up_to,
                ..Chain::default()
            };
            self.chain(&body, &mut branch_chain)?;
            if branch_chain.effects.is_empty()
                || branch_chain.target != chain.target
                || branch_chain.second_target != chain.second_target
                || branch_chain.target_up_to != chain.target_up_to
                || branch_chain.target_exact != chain.target_exact
                || branch_chain.target_is_x != chain.target_is_x
            {
                return None;
            }
            chain.could_add_mana |= branch_chain.could_add_mana;
            chain.moves_library |= branch_chain.moves_library;
            Some(branch_chain.effects)
        };
        let then_effects = run_branch(true_sub)?;
        let else_effects = run_branch(false_sub)?;
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
            then_effects.join(", "),
            else_effects.join(", ")
        )])
    }

    /// Checks that a branch arm (`FalseSubAbility$`) only runs `Defined$ Targeted`
    /// effects so that choosing 0 targets on a `TargetMin$ 0` ability is a no-op.
    fn wave24a_sub_only_acts_on_target(&self, start: &str) -> bool {
        let mut cursor = Some(start.trim().to_string());
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return false;
            }
            let Some(body) = self.svars.get(&name) else {
                return false;
            };
            let Some((api, sp)) = Params::parse(body) else {
                return false;
            };
            if api == "Cleanup" {
                cursor = sp.peek("SubAbility").map(|s| s.trim().to_string());
                continue;
            }
            if !matches!(sp.peek("Defined"), Some("Targeted" | "ParentTarget")) {
                return false;
            }
            cursor = sp.peek("SubAbility").map(|s| s.trim().to_string());
        }
        true
    }

    #[allow(clippy::too_many_lines)]
    fn wave24a_branch_condition(
        &mut self,
        var: &str,
        cmp: &str,
        target: Option<&str>,
        single_target: bool,
    ) -> Option<(String, bool)> {
        if let Some(cond) = self
            .svar_condition_clause(var, cmp)
            .or_else(|| self.wave7_svar_condition(var, cmp))
            .or_else(|| Self::wave16_svar_condition(var.trim(), cmp))
            .or_else(|| self.wave11_amount_compare_condition(var, cmp, true))
        {
            return Some((cond, false));
        }
        let raw = self
            .svars
            .get(var.trim())
            .map_or(var.trim(), String::as_str)
            .trim()
            .to_string();
        let cmp = cmp.trim();
        let bool_polarity = match cmp {
            "GE1" | "EQ1" | "GT0" => Some(true),
            "EQ0" | "LT1" | "LE0" => Some(false),
            _ => None,
        };
        if let Some(rest) = raw.strip_prefix("Count$Valid Card.Self+counters_GE")
            && let Some((num_raw, code)) = rest.split_once('_')
            && !code.contains(['+', '.', ',', ' ', '!'])
            && let Ok(n) = num_raw.parse::<u8>()
            && n >= 1
            && let Some(kind) = counter_kind(code)
            && let Some(pos) = bool_polarity
        {
            let base = format!("Condition::CountersOnSelf({kind}, {n})");
            return Some((
                if pos {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                },
                false,
            ));
        }
        let obj_target =
            single_target && target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        if obj_target
            && let Some(valid) = raw.strip_prefix("Targeted$Valid ")
            && let Some(pos) = bool_polarity
        {
            let expr = self.filter_expr(valid.trim())?;
            let filter_name = self.body.filter_static("BRANCH", &expr);
            let base = format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                 filter: &{filter_name} }}"
            );
            return Some((
                if pos {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                },
                true,
            ));
        }
        if obj_target && let Some(code) = raw.strip_prefix("Targeted$CardCounters.") {
            let code = code.trim();
            if matches!(code, "ENERGY" | "POISON" | "RAD") {
                return None;
            }
            let kind = counter_kind(code)?;
            let min_opt: Option<(u16, bool)> = if let Some(n_raw) = cmp.strip_prefix("GE") {
                let n = n_raw.parse::<u16>().ok()?;
                (n >= 1).then_some((n, true))
            } else if let Some(n_raw) = cmp.strip_prefix("GT") {
                let n = n_raw.parse::<u16>().ok()?.checked_add(1)?;
                Some((n, true))
            } else if matches!(cmp, "EQ0" | "LT1" | "LE0") {
                Some((1, false))
            } else {
                None
            };
            let (min_n, pos) = min_opt?;
            let filter_expr = format!("Filter::CountersAtLeast({kind}, {min_n})");
            let filter_name = self.body.filter_static("BRANCH", &filter_expr);
            let base = format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                 filter: &{filter_name} }}"
            );
            return Some((
                if pos {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                },
                true,
            ));
        }
        let event_valid = if self.trigger_mode.as_deref() == Some("ChangesZone") {
            raw.strip_prefix("TriggeredCard$Valid ")
        } else if matches!(
            self.trigger_mode.as_deref(),
            Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
        ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
            && self
                .block_line
                .as_ref()
                .is_some_and(|(r, _, _)| *r == BlockRole::Blocks))
        {
            raw.strip_prefix("TriggeredAttacker$Valid ")
        } else if self.trigger_mode.as_deref() == Some("Blocks")
            || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                && self
                    .block_line
                    .as_ref()
                    .is_some_and(|(r, _, _)| *r == BlockRole::Blocked))
        {
            raw.strip_prefix("TriggeredBlocker$Valid ")
        } else {
            None
        };
        if let Some(valid) = event_valid
            && let Some(pos) = bool_polarity
        {
            let expr = self.filter_expr(valid.trim())?;
            let filter_name = self.body.filter_static("BRANCH", &expr);
            let base = format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::EventObject, \
                 filter: &{filter_name} }}"
            );
            return Some((
                if pos {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                },
                false,
            ));
        }
        if raw == "TriggerCount$Result" && self.trigger_mode.as_deref() == Some("RolledDie") {
            let cond = if let Some(n_raw) = cmp.strip_prefix("GE") {
                let n = n_raw.parse::<u32>().ok()?;
                (n >= 1).then(|| format!("Condition::AmountAtLeast(Amount::EventAmount, {n})"))?
            } else if let Some(n_raw) = cmp.strip_prefix("GT") {
                let n = n_raw.parse::<u32>().ok()?.checked_add(1)?;
                format!("Condition::AmountAtLeast(Amount::EventAmount, {n})")
            } else if let Some(n_raw) = cmp.strip_prefix("LE") {
                let n = n_raw.parse::<u32>().ok()?.checked_add(1)?;
                format!("Condition::Not(&Condition::AmountAtLeast(Amount::EventAmount, {n}))")
            } else if let Some(n_raw) = cmp.strip_prefix("LT") {
                let n = n_raw.parse::<u32>().ok()?;
                (n >= 1).then(|| {
                    format!("Condition::Not(&Condition::AmountAtLeast(Amount::EventAmount, {n}))")
                })?
            } else {
                let n = cmp.strip_prefix("EQ")?.parse::<u32>().ok()?;
                format!(
                    "Condition::AmountCompare {{ left: &Amount::EventAmount, \
                     op: AmountCmp::Equal, right: &Amount::Fixed({n}) }}"
                )
            };
            return Some((cond, false));
        }
        None
    }

    #[allow(clippy::too_many_lines)]
    fn wave24a_divided_counters_chain(&mut self, mut p: Params, chain: &mut Chain) -> Option<()> {
        if self.trigger_mode.is_some()
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("you may distribute")
        {
            return self.deny("optional triggered divided counter distribution".to_string());
        }
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let Some(valid) = p.take("ValidTgts") else {
            return self.deny("`PutCounter` `DividedAsYouChoose$` without `ValidTgts$`".into());
        };
        let spec = self.target_spec(&valid, "PutCounter")?;
        if !spec.starts_with("TargetSpec::Object(") {
            return self.deny("a non-object divided counter target requirement".into());
        }
        let Some(code) = p.take("CounterType") else {
            return self.deny("`PutCounter` `DividedAsYouChoose$` without `CounterType$`".into());
        };
        if matches!(code.as_str(), "ENERGY" | "POISON" | "RAD") {
            return self.deny(format!("divided player counter `{code}`"));
        }
        let Some(kind) = counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let Some(raw_num) = p.take("CounterNum") else {
            return self.deny("`PutCounter` `DividedAsYouChoose$` without `CounterNum$`".into());
        };
        let Some(raw_div) = p.take("DividedAsYouChoose") else {
            return self.deny("missing `DividedAsYouChoose$`".into());
        };
        let Some(amount) = self.wave24a_divided_counter_amount(&raw_num) else {
            return self.deny(format!("divided counter count `{raw_num}`"));
        };
        if self.wave24a_divided_counter_amount(&raw_div).as_deref() != Some(amount.as_str()) {
            return self.deny("counter count and announced division have different totals".into());
        }
        let fixed_total = amount
            .strip_prefix("Amount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u32>().ok());
        if fixed_total == Some(0) {
            return self.deny("divided counter total of zero".into());
        }
        let target_min = p.take("TargetMin");
        let target_max = p.take("TargetMax");
        let target_up_to: Option<u8> = match (target_min.as_deref(), target_max.as_deref()) {
            (Some("0"), Some(raw_max)) => {
                if let Ok(n) = raw_max.trim().parse::<u8>() {
                    if n == 0 || n == u8::MAX {
                        return self.deny(format!(
                            "divided counter distribution with `TargetMax$ {raw_max}`"
                        ));
                    }
                    if let Some(total) = fixed_total
                        && u32::from(n) > total
                    {
                        return self
                            .deny("divided counter maximum targets exceeds counter total".into());
                    }
                    Some(n)
                } else if fixed_total.is_none()
                    && (raw_max.trim() == raw_num.trim() || raw_max.trim() == raw_div.trim())
                {
                    Some(u8::MAX)
                } else {
                    return self.deny(format!(
                        "divided counter distribution with `TargetMax$ {raw_max}`"
                    ));
                }
            }
            (Some("1"), Some("1")) => None,
            _ => {
                return self.deny(
                    "triggered divided counter distribution with unsupported target bounds".into(),
                );
            }
        };
        p.take("IsCurse");
        let next = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `PutCounter.{key}`"));
        }
        chain.target = Some(spec.clone());
        chain.target_up_to = target_up_to;
        chain.effects.push(format!(
            "Effect::PutCountersDivided {{ kind: {kind}, amount: {amount} }}"
        ));
        if let Some(next_name) = next {
            self.chain_sub_ability(Some(next_name), chain)?;
            if chain.target.as_deref() != Some(spec.as_str())
                || chain.second_target.is_some()
                || chain.target_is_x
                || chain.target_up_to != target_up_to
                || chain.target_exact.is_some()
                || chain.could_add_mana
                || chain
                    .effects
                    .iter()
                    .skip(1)
                    .any(|e| e.contains("DealDamageDivided") || e.contains("PutCountersDivided"))
            {
                return self.deny("an unsupported continuation after divided counters".into());
            }
        }
        Some(())
    }

    fn wave24a_divided_counter_amount(&mut self, raw: &str) -> Option<String> {
        const RESOLUTION_ONLY: &[&str] = &[
            "ThisWay",
            "TargetPower",
            "TargetToughness",
            "TargetCmc",
            "Sacrificed",
            "TappedPower",
            "EventAmount",
            "EventPower",
            "EventLastToughness",
            "EventManaValue",
            "LastDieRoll",
            "CoinsWonThisResolution",
            "Amount::Neg",
            "TargetsPutIntoGraveyard",
        ];
        let trimmed = raw.trim();
        if self.trigger_is_self
            && self.trigger_mode.as_deref() == Some("ChangesZone")
            && let Some(def) = self.svars.get(trimmed).map(|s| s.trim())
            && let Some(code) = def.strip_prefix("TriggeredCard$CardCounters.")
            && !matches!(code.trim(), "ENERGY" | "POISON" | "RAD")
            && let Some(kind) = counter_kind(code.trim())
        {
            return Some(format!("Amount::CountersOnSource({kind})"));
        }
        let expr = self.amount_or_count_ext(trimmed)?;
        if RESOLUTION_ONLY.iter().any(|bad| expr.contains(bad)) {
            return None;
        }
        Some(expr)
    }

    #[allow(clippy::too_many_lines)]
    fn wave24a_single_excess_damage_chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let (api, mut p) = Params::parse(spec)?;
        if api != "DealDamage" {
            return None;
        }
        p.drop_prose();
        let excess_var = p.take("ExcessSVar")?.trim().to_string();
        if excess_var.is_empty() || self.svars.contains_key(&excess_var) {
            return None;
        }
        let require_creature_target = match p.take("ExcessSVarCondition").as_deref() {
            None | Some("Card.targetedBy") => false,
            Some("Creature") => true,
            Some(_) => return None,
        };
        let sub_name = p.take("SubAbility")?;
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let target_spec = self.target_spec(&valid, "DealDamage")?;
        if matches!(
            target_spec.as_str(),
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"
        ) {
            return None;
        }
        match (
            p.take("TargetMin").as_deref(),
            p.take("TargetMax").as_deref(),
        ) {
            (None, None) | (Some("1"), None | Some("1")) => {}
            (Some("0"), None | Some("1")) => {
                chain.target_up_to = Some(1);
            }
            _ => return None,
        }
        let raw_dmg = p.take("NumDmg")?;
        let dmg = self.amount_or_count_ext(&raw_dmg)?;
        let may = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        if p.take("DamageMap").is_some_and(|v| v != "True") || !p.exhausted() {
            return None;
        }
        let dmg_eff = format!("Effect::DealDamage {{ amount: {dmg}, target: {target_spec} }}");
        let first_eff = if may {
            format!("Effect::MayDo {{ effects: &[{dmg_eff}] }}")
        } else {
            dmg_eff
        };
        chain.target = Some(target_spec);
        let follow = self.wave24a_excess_damage_followup(
            &sub_name,
            &excess_var,
            "Amount::ThisWay(ThisWay::ExcessDamage)",
            require_creature_target,
            chain,
        )?;
        chain.effects.push(first_eff);
        chain.effects.extend(follow);
        Some(())
    }

    fn wave24a_bite_excess_damage_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let excess_var = p.take("ExcessSVar")?.trim().to_string();
        if excess_var.is_empty() || self.svars.contains_key(&excess_var) {
            return None;
        }
        if !matches!(
            p.take("ExcessSVarCondition").as_deref(),
            None | Some("Card.targetedBy")
        ) {
            return None;
        }
        if p.take("DamageMap").is_some_and(|v| v != "True") {
            return None;
        }
        let mut out = self.bite_effect(p)?;
        if !p.exhausted() {
            return None;
        }
        let sub_name = sub.take()?;
        let follow = self.wave24a_excess_damage_followup(
            &sub_name,
            &excess_var,
            "Amount::ThisWay(ThisWay::ExcessDamage)",
            false,
            chain,
        )?;
        out.extend(follow);
        Some(out)
    }

    fn wave24a_fight_excess_damage_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let excess_var = p.take("ExcessSVar")?.trim().to_string();
        if excess_var.is_empty() || self.svars.contains_key(&excess_var) {
            return None;
        }
        let excess_amt = match p.take("ExcessSVarCondition").as_deref() {
            None => "Amount::ThisWay(ThisWay::ExcessDamage)",
            Some("Creature.targetedBy+OppCtrl")
                if chain
                    .second_target
                    .as_deref()
                    .is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                "Amount::ThisWayBy { what: ThisWay::ExcessDamage, \
                 who: PlayerRel::SecondTargetPlayerOrController }"
            }
            Some(_) => return None,
        };
        let mut out = self.fight_effect(p, true)?;
        if !p.exhausted() {
            return None;
        }
        let sub_name = sub.take()?;
        let follow =
            self.wave24a_excess_damage_followup(&sub_name, &excess_var, excess_amt, false, chain)?;
        out.extend(follow);
        Some(out)
    }

    #[allow(clippy::too_many_lines)]
    fn wave24a_excess_damage_followup(
        &mut self,
        sub_name: &str,
        excess_var: &str,
        excess_amt: &str,
        require_creature_target: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (sub_api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        sp.take("IfDesc");
        if sp.has("ValidTgts") || sp.has("UnlessCost") || sp.has("AtEOT") {
            return None;
        }
        let cond = if let Some(check_var) = sp.take("ConditionCheckSVar") {
            if check_var.trim() != excess_var {
                return None;
            }
            let min_excess: u32 = match sp.take("ConditionSVarCompare").as_deref() {
                None | Some("GE1" | "GT0") => 1,
                Some(cmp) => {
                    if let Some(n) = cmp.strip_prefix("GE").and_then(|s| s.parse::<u32>().ok())
                        && n >= 1
                    {
                        n
                    } else {
                        let n = cmp.strip_prefix("GT").and_then(|s| s.parse::<u32>().ok())?;
                        n.checked_add(1)?
                    }
                }
            };
            let base = format!("Condition::AmountAtLeast({excess_amt}, {min_excess})");
            Some(if require_creature_target {
                format!(
                    "Condition::All(&[Condition::ObjectMatches {{ \
                     what: &TargetSpec::Object(&Filter::Any), filter: &Filter::CREATURE }}, \
                     {base}])"
                )
            } else {
                base
            })
        } else {
            if sp.has("ConditionSVarCompare") || require_creature_target {
                return None;
            }
            None
        };
        let mut used_excess = cond.is_some();
        let resolve_amt = |tx: &mut Self, raw: &str, used: &mut bool| -> Option<String> {
            if raw.trim() == excess_var {
                *used = true;
                Some(excess_amt.to_string())
            } else {
                tx.amount_or_count_ext(raw)
            }
        };
        let next_sub = sp.take("SubAbility");
        if next_sub.is_some() {
            return None;
        }
        let effects = match sub_api.as_str() {
            "GainLife" => {
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw = sp.take("LifeAmount")?;
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                vec![format!("Effect::GainLife {{ amount: {amt} }}")]
            }
            "LoseLife" => {
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    chain.target.as_deref(),
                    false,
                )?;
                let raw = sp.take("LifeAmount")?;
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                vec![format!(
                    "Effect::LoseLife {{ amount: {amt}, target: {who} }}"
                )]
            }
            "Draw" => {
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    chain.target.as_deref(),
                    false,
                )?;
                let raw = sp.take("NumCards").unwrap_or_else(|| "1".to_string());
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                chain.moves_library = true;
                if who == "PlayerRel::You" {
                    if let Some(n) = amt
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|s| s.strip_suffix(')'))
                    {
                        vec![format!("Effect::draw({n})")]
                    } else {
                        vec![format!("Effect::DrawCards {{ amount: {amt} }}")]
                    }
                } else {
                    vec![format!(
                        "Effect::DrawCardsFor {{ amount: {amt}, who: {who} }}"
                    )]
                }
            }
            "Investigate" => {
                let who = match (sp.take("Defined"), sp.take("ValidPlayer")) {
                    (Some(_), Some(_)) => return None,
                    (Some(d), None) | (None, Some(d)) => self.player_rel(Some(&d)),
                    (None, None) => Some("PlayerRel::You"),
                };
                if who != Some("PlayerRel::You") {
                    return None;
                }
                let raw = sp.take("Num").unwrap_or_else(|| "1".to_string());
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                let tokens = self.tokens?;
                let tok = tokens.body_fixed(CLUE_TOKEN_SCRIPT, self.cats)?;
                let token = format!("&generated_tokens::{}", tok.constant);
                if amt == "Amount::Fixed(1)" {
                    vec![format!("Effect::CreateToken {{ token: {token} }}")]
                } else {
                    vec![format!(
                        "Effect::CreateTokenN {{ token: {token}, amount: {amt} }}"
                    )]
                }
            }
            "Token" => {
                if !matches!(sp.take("TokenOwner").as_deref(), None | Some("You")) {
                    return None;
                }
                let tapped = match sp.take("TokenTapped").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let raw = sp.take("TokenAmount").unwrap_or_else(|| "1".to_string());
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                let stem = sp.take("TokenScript")?;
                let tokens = self.tokens?;
                let tok = tokens.body_fixed(&stem, self.cats)?;
                let token = format!("&generated_tokens::{}", tok.constant);
                if tapped {
                    vec![format!(
                        "Effect::CreateTokenModified {{ token: {token}, amount: {amt}, \
                         tapped: true, haste: false }}"
                    )]
                } else if amt == "Amount::Fixed(1)" {
                    vec![format!("Effect::CreateToken {{ token: {token} }}")]
                } else {
                    vec![format!(
                        "Effect::CreateTokenN {{ token: {token}, amount: {amt} }}"
                    )]
                }
            }
            "Discover" => {
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw = sp.take("Num")?;
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                chain.moves_library = true;
                if let Some(n) = amt
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    vec![format!("Effect::Discover {{ mana_value: {n} }}")]
                } else {
                    vec![format!("Effect::DiscoverAmount {{ mana_value: {amt} }}")]
                }
            }
            "Amass" => {
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw_type = sp.take("Type")?;
                let subtype = self
                    .cats
                    .const_path_of(SubtypeKind::Creature, raw_type.trim())?;
                let stem = format!("b_0_0_{}_army", raw_type.trim().to_ascii_lowercase());
                let tokens = self.tokens?;
                let tok = tokens.body_fixed(&stem, self.cats)?;
                let raw_num = sp.take("Num")?;
                let amt = resolve_amt(self, &raw_num, &mut used_excess)?;
                vec![format!(
                    "Effect::Amass {{ token: &generated_tokens::{}, subtype: {subtype}, \
                     amount: {amt} }}",
                    tok.constant
                )]
            }
            "Scry" => {
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw = sp.take("ScryNum")?;
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                vec![format!("Effect::Scry {{ amount: {amt} }}")]
            }
            "Surveil" => {
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let raw = sp.take("Amount")?;
                let amt = resolve_amt(self, &raw, &mut used_excess)?;
                chain.moves_library = true;
                vec![format!("Effect::Surveil {{ amount: {amt} }}")]
            }
            _ => return None,
        };
        if !sp.exhausted() || !used_excess {
            return None;
        }
        Some(if let Some(c) = cond {
            vec![format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{}], otherwise: &[] }}",
                effects.join(", ")
            )]
        } else {
            effects
        })
    }

    #[allow(clippy::too_many_lines)]
    fn wave24a_remembered_chain(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if api == "Mill" && self.sub_is_api(sub.as_deref(), "Branch") {
            return self.wave24a_mill_remembered_branch_chain(p, sub, target, flags, chain);
        }
        if api == "Mill" && self.sub_is_api(sub.as_deref(), "Draw") {
            return self.wave24a_mill_remembered_compare_draw_chain(p, sub, target, flags, chain);
        }
        if api == "ChangeZone"
            && flags.targets_here
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && sub.is_some()
            && !self.sub_is_api(sub.as_deref(), "DelayedTrigger")
            && !self.sub_is_api(sub.as_deref(), "ChangeZone")
        {
            let saved_p = p.clone();
            let saved_sub = sub.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effs) =
                self.wave24a_exile_then_sub_return_at_end_step(p, sub, target, chain)
            {
                return Some(effs);
            }
            *p = saved_p;
            *sub = saved_sub;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }
        if api == "ChangeZone"
            && !flags.targets_here
            && p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Battlefield")
            && self.sub_is_api(sub.as_deref(), "Attach")
        {
            return self.wave24a_hand_to_bf_attach_if_equipment(p, sub);
        }
        if api == "ChangeZone"
            && flags.targets_here
            && flags.single_target
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Exile")
            && self.sub_is_api(sub.as_deref(), "CopyPermanent")
        {
            return self.wave24a_exile_gy_and_copy_permanent(p, sub, target);
        }
        if api == "ChangeZone"
            && flags.targets_here
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Battlefield")
            && (self.sub_is_api(sub.as_deref(), "Attach")
                || self.sub_is_api(sub.as_deref(), "Pump")
                || self.sub_is_api(sub.as_deref(), "CopyPermanent")
                || self.sub_is_api(sub.as_deref(), "Explore"))
        {
            return self.wave24a_reanimate_attach_or_temp_pump(p, sub, target, flags.single_target);
        }
        if api == "ChangeZoneAll"
            && !flags.targets_here
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Battlefield")
            && self.sub_is_api(sub.as_deref(), "PumpAll")
        {
            return self.wave24a_reanimate_all_modified(p, sub);
        }
        if api == "ChangeZoneAll"
            && !flags.targets_here
            && matches!(
                (p.peek("Origin"), p.peek("Destination")),
                (Some("Battlefield" | "Graveyard"), Some("Exile"))
            )
            && self.sub_is_api(sub.as_deref(), "Incubate")
        {
            return self.wave24a_cz_all_remembered_count_chain(p, sub, target, flags);
        }
        if api == "ChangeZone"
            && matches!(
                (p.peek("Origin"), p.peek("Destination")),
                (Some("Graveyard" | "Battlefield"), Some("Exile"))
                    | (Some("Graveyard"), Some("Hand"))
            )
            && sub.is_some()
        {
            let saved_p = p.clone();
            let saved_sub = sub.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effs) = self.wave24a_cz_remembered_followup(p, sub, target, flags, chain) {
                return Some(effs);
            }
            *p = saved_p;
            *sub = saved_sub;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }
        if api == "ChangeZone"
            && flags.targets_here
            && flags.single_target
            && p.peek("Origin") == Some("Graveyard")
            && matches!(
                p.peek("Destination"),
                Some("Hand" | "Battlefield" | "Library" | "Exile")
            )
            && sub.is_some()
        {
            return self.wave24a_cz_gy_remembered_target_stat_chain(p, sub, target, chain);
        }
        None
    }

    fn wave24a_take_optional_cleanup(
        &self,
        cursor: Option<String>,
        sub: &mut Option<String>,
    ) -> Option<()> {
        let Some(name) = cursor else {
            *sub = None;
            return Some(());
        };
        if let Some(body) = self.svars.get(name.trim())
            && let Some((c_api, mut cp)) = Params::parse(body)
        {
            cp.drop_prose();
            if c_api == "Cleanup" && cp.take("ClearRemembered").as_deref() == Some("True") {
                let after = cp.take("SubAbility");
                if !cp.exhausted() {
                    return None;
                }
                *sub = after;
                return Some(());
            }
        }
        *sub = Some(name);
        Some(())
    }

    /// `Mill | RememberMilled$ True | SubAbility$ DBBranch` where `DBBranch`
    /// branches on `Remembered$Valid <filter>` or `Remembered$Amount` (e.g.
    /// Lorehold Excavation).
    #[allow(clippy::too_many_lines)]
    fn wave24a_mill_remembered_branch_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let who = self.wave11_player_of_line(p.peek("Defined"), target, flags.targets_a_player)?;
        let is_single_mill =
            Self::is_single_player_rel(who) && matches!(p.peek("NumCards"), None | Some("1"));
        if p.take("RememberMilled").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let mut out = self.effect_of_with_prior("Mill", p, target, flags)?;
        if !p.exhausted() {
            return None;
        }
        let branch_name = sub.take()?;
        let branch_body = self.svars.get(branch_name.trim())?.clone();
        let (b_api, mut bp) = Params::parse(&branch_body)?;
        bp.drop_prose();
        if b_api != "Branch" || bp.has("ValidTgts") || bp.has("Defined") {
            return None;
        }
        let var = bp.take("BranchConditionSVar")?;
        let cmp = bp
            .take("BranchConditionSVarCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let true_sub = bp.take("TrueSubAbility");
        let false_sub = bp.take("FalseSubAbility");
        let after_branch = bp.take("SubAbility");
        if (true_sub.is_none() && false_sub.is_none()) || !bp.exhausted() {
            return None;
        }
        let raw = self
            .svars
            .get(var.trim())
            .map_or(var.trim(), String::as_str)
            .trim()
            .to_string();
        let filter_ref = if matches!(raw.as_str(), "Remembered$Amount" | "Count$RememberedSize") {
            "&Filter::Any".to_string()
        } else {
            let valid = raw
                .strip_prefix("Remembered$Valid ")
                .or_else(|| raw.strip_prefix("RememberedLKI$Valid "))?;
            let v = valid.trim();
            if matches!(v, "Card" | "Permanent") {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(v, true)?;
                format!("&{}", self.body.filter_static("MILLED", &expr))
            }
        };
        let amt = format!("Amount::ThisWay(ThisWay::Milled({filter_ref}))");
        let cond = match cmp.trim() {
            "GE1" | "GT0" => format!("Condition::AmountAtLeast({amt}, 1)"),
            "EQ1" if is_single_mill => format!("Condition::AmountAtLeast({amt}, 1)"),
            "EQ0" | "LT1" | "LE0" => format!("Condition::Not(&Condition::AmountAtLeast({amt}, 1))"),
            c if c.starts_with("GE") => {
                let n = c[2..].parse::<u32>().ok().filter(|&n| n >= 1)?;
                format!("Condition::AmountAtLeast({amt}, {n})")
            }
            _ => return None,
        };
        let mut run_arm = |arm: Option<String>| -> Option<Vec<String>> {
            let Some(name) = arm else {
                return Some(Vec::new());
            };
            let body = self.svars.get(name.trim())?.clone();
            if body.contains("Remembered") || body.contains("IsRemembered") {
                return None;
            }
            let mut branch_chain = Chain {
                target: chain.target.clone(),
                target_is_x: chain.target_is_x,
                target_up_to: chain.target_up_to,
                target_exact: chain.target_exact,
                target_unique: chain.target_unique,
                second_target: chain.second_target.clone(),
                second_target_up_to: chain.second_target_up_to,
                ..Chain::default()
            };
            self.chain(&body, &mut branch_chain)?;
            if branch_chain.effects.is_empty()
                || branch_chain.target != chain.target
                || branch_chain.second_target != chain.second_target
            {
                return None;
            }
            chain.could_add_mana |= branch_chain.could_add_mana;
            chain.moves_library |= branch_chain.moves_library;
            Some(branch_chain.effects)
        };
        let then_effs = run_arm(true_sub)?;
        let else_effs = run_arm(false_sub)?;
        self.wave24a_take_optional_cleanup(after_branch, sub)?;
        out.push(format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
            then_effs.join(", "),
            else_effs.join(", ")
        ));
        Some(out)
    }

    /// "You may put an [filter] card from your hand onto the battlefield. If
    /// it's an Equipment, attach it to [this]" (The Invincible Iron Man).
    fn wave24a_hand_to_bf_attach_if_equipment(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || p.take("Origin").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(p.take("Tapped").as_deref(), None | Some("False"))
            || !matches!(p.take("Mandatory").as_deref(), None | Some("False"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        p.take("SelectPrompt");
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let optional = match p.take("Optional").as_deref() {
            Some("You" | "True") => true,
            None => self.oracle.to_ascii_lowercase().contains("you may put"),
            Some(_) => return None,
        };
        if !optional {
            return None;
        }
        let raw_valid = p.take("ChangeType")?;
        if !p.exhausted() {
            return None;
        }
        let cleaned = Self::strip_your_graveyard_owner(&raw_valid).unwrap_or(raw_valid);
        let filter_expr = self.filter_expr_in_zone(&cleaned, true)?;
        let attach_name = sub.take()?;
        let attach_body = self.svars.get(attach_name.trim())?.clone();
        let (a_api, mut ap) = Params::parse(&attach_body)?;
        ap.drop_prose();
        if a_api != "Attach"
            || ap.take("Object").as_deref() != Some("Remembered")
            || ap.take("Defined").as_deref() != Some("Self")
            || ap.take("ConditionDefined").as_deref() != Some("Remembered")
            || !matches!(
                ap.take("ConditionPresent").as_deref(),
                Some("Equipment" | "Card.Equipment" | "Artifact.Equipment")
            )
            || !matches!(
                ap.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1" | "GT0")
            )
        {
            return None;
        }
        let after = ap.take("SubAbility");
        if !ap.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after, sub)?;
        let name = self.body.filter_static("PUT_FROM_HAND", &filter_expr);
        Some(vec![format!(
            "Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment {{ filter: &{name} }}"
        )])
    }

    fn wave24a_take_copy_riders(p: &mut Params) -> Option<(bool, bool, bool, &'static str)> {
        let tapped = match p
            .take("TokenTapped")
            .or_else(|| p.take("Tapped"))
            .as_deref()
        {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let attacking = match p.take("TokenAttacking").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let haste_until_eot = match (
            p.take("PumpKeywords").as_deref(),
            p.take("PumpDuration").as_deref(),
        ) {
            (None, None) => false,
            (Some("Haste"), None | Some("EOT" | "UntilEndOfTurn")) => true,
            _ => return None,
        };
        let cleanup = match p.take("AtEOT").or_else(|| p.take("AtEOTTrig")).as_deref() {
            None => "CopyCleanup::None",
            Some("Sacrifice") => "CopyCleanup::SacrificeAtNextEndStep",
            Some("Exile") => "CopyCleanup::ExileAtNextEndStep",
            Some("SacrificeCombat") => "CopyCleanup::SacrificeAtEndOfCombat",
            Some("ExileCombat") => "CopyCleanup::ExileAtEndOfCombat",
            Some(_) => return None,
        };
        Some((tapped, attacking, haste_until_eot, cleanup))
    }

    /// "Exile [up to one] target card from a graveyard. [If you do,] create a
    /// token that's a copy of that card / this creature" (Anikthea, Ardyn,
    /// Blastoneuron, Séance, Uchuulon).
    #[allow(clippy::too_many_lines)]
    fn wave24a_exile_gy_and_copy_permanent(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?;
        if self.overload_cost.is_some()
            || p.has("Hidden")
            || self.has_linked_exile_reader()
            || p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
            || !p.exhausted()
        {
            return None;
        }
        let copy_name = sub.take()?;
        let copy_body = self.svars.get(copy_name.trim())?.clone();
        let (c_api, mut cp) = Params::parse(&copy_body)?;
        cp.drop_prose();
        if c_api != "CopyPermanent"
            || cp.has("Populate")
            || cp.has("Choices")
            || cp.has("RememberTokens")
            || cp.has("AtEOTCondition")
            || cp.has("ConditionDefined")
            || cp.has("ConditionCheckSVar")
            || cp.has("ConditionPresent")
        {
            return None;
        }
        let defined = cp.take("Defined")?;
        let (tapped, attacking, haste_until_eot, cleanup) =
            Self::wave24a_take_copy_riders(&mut cp)?;
        let mods = self.copy_mods(&mut cp, false)?;
        let mods_list = mods.join(", ");
        let controller =
            self.player_of_line(cp.take("Controller").as_deref(), Some(target), false)?;
        let (copy_target, amount) = match defined.as_str() {
            "Remembered" => {
                let amt = match cp.take("NumCopies") {
                    None => "Amount::Fixed(1)".to_string(),
                    Some(raw) => self.amount_or_count_ext(&raw)?,
                };
                (target.to_string(), amt)
            }
            "Self" if !self.on_a_spell => {
                let raw = cp.take("NumCopies")?;
                let def = self
                    .svars
                    .get(raw.trim())
                    .map_or(raw.trim(), String::as_str)
                    .trim();
                if !matches!(def, "Remembered$Amount" | "Count$RememberedSize") {
                    return None;
                }
                (
                    "TargetSpec::ThisObject".to_string(),
                    "Amount::ThisWay(ThisWay::ExiledFromGraveyard(&Filter::Any))".to_string(),
                )
            }
            _ => return None,
        };
        let after = cp.take("SubAbility");
        if !cp.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after, sub)?;
        Some(vec![
            format!("Effect::exile({target})"),
            format!(
                "Effect::IfCondition {{ \
                 condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ExiledFromGraveyard(&Filter::Any)), 1), \
                 then: &[Effect::CreateTokenCopy {{ target: {copy_target}, controller: {controller}, \
                 amount: {amount}, mods: &[{mods_list}], tapped: {tapped}, attacking: {attacking}, \
                 haste_until_eot: {haste_until_eot}, cleanup: {cleanup} }}], \
                 otherwise: &[] }}"
            ),
        ])
    }

    fn wave24a_pump_kw_bits(raw: &str) -> Option<String> {
        let mut bits = Vec::new();
        for part in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            let bit = keyword_const(part)?;
            bits.push(bit);
        }
        if bits.is_empty() {
            return None;
        }
        bits.into_iter()
            .map(str::to_string)
            .reduce(|acc, k| format!("{acc}.union({k})"))
    }

    /// Reanimate from graveyard with Equipment attachment (`Pre-War Formalwear`)
    /// or temporary keyword/P&T pump (`Rydia, Summoner of Mist`).
    #[allow(clippy::too_many_lines)]
    fn wave24a_reanimate_attach_or_temp_pump(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?;
        if p.has("Hidden")
            || p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("GainControl").as_deref(), None | Some("True"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let counters = match p.take("WithCountersType") {
            None => {
                if p.has("WithCountersAmount") {
                    return None;
                }
                None
            }
            Some(code) => {
                let kind = counter_kind(&code)?;
                let raw_n = p
                    .take("WithCountersAmount")
                    .unwrap_or_else(|| "1".to_string());
                let amt = self.amount_or_count_ext(&raw_n)?;
                Some((kind, amt))
            }
        };
        if !p.exhausted() {
            return None;
        }
        let next_name = sub.take()?;
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&next_body)?;
        sp.drop_prose();
        if s_api == "Attach" {
            if !single_target
                || self.on_a_spell
                || tapped
                || !self.type_line.split_whitespace().any(|w| w == "Equipment")
                || sp.take("Defined").as_deref() != Some("Remembered")
            {
                return None;
            }
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            self.wave24a_take_optional_cleanup(after, sub)?;
            let ctr_expr = match counters {
                None => "None".to_string(),
                Some((kind, amt)) => {
                    let fixed = amt
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|s| s.strip_suffix(')'))
                        .and_then(|s| s.parse::<u16>().ok())?;
                    format!("Some(({kind}, {fixed}))")
                }
            };
            return Some(vec![
                format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, \
                     owner_control: false, counters: {ctr_expr} }}"
                ),
                format!("Effect::AttachSelf {{ target: {target} }}"),
            ]);
        }
        if s_api == "Pump" {
            if sp.take("Defined").as_deref() != Some("Remembered")
                || sp.has("ValidTgts")
                || sp.has("AtEOT")
                || sp.has("UnlessCost")
                || sp.has("PumpZone")
            {
                return None;
            }
            let duration = match sp.take("Duration").as_deref() {
                None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
                Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
                Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
                Some(_) => return None,
            };
            let mut mods = Vec::new();
            if let Some(kws) = sp.take("KW") {
                let bits = Self::wave24a_pump_kw_bits(&kws)?;
                mods.push(format!("Modifier::AddKeyword({bits})"));
            }
            match (sp.take("NumAtt"), sp.take("NumDef")) {
                (None, None) => {}
                (a, d) => {
                    let pa = self.plain_num(a.as_deref().unwrap_or("0"))?;
                    let pd = self.plain_num(d.as_deref().unwrap_or("0"))?;
                    let pa = i8::try_from(pa).ok()?;
                    let pd = i8::try_from(pd).ok()?;
                    if pa != 0 || pd != 0 {
                        mods.push(format!("Modifier::ModifyPT({pa}, {pd})"));
                    }
                }
            }
            let after = sp.take("SubAbility");
            if mods.is_empty() || !sp.exhausted() {
                return None;
            }
            self.wave24a_take_optional_cleanup(after, sub)?;
            let ctr_expr = match counters {
                None => "None".to_string(),
                Some((kind, amt)) => format!("Some(({kind}, {amt}))"),
            };
            return Some(vec![format!(
                "Effect::MoveAndModify {{ subject: MoveSubject::Target({target}), \
                 expected_origin: Some(ZoneRef::Graveyard), blink: false, owner_control: false, \
                 tapped: {tapped}, face: None, counters: {ctr_expr}, modifiers: &[{}], \
                 duration: {duration}, attach_source: false, when_source_leaves: None }}",
                mods.join(", ")
            )]);
        }
        if s_api == "CopyPermanent" {
            if !single_target
                || tapped
                || sp.take("Defined").as_deref() != Some("Remembered")
                || sp.has("Populate")
                || sp.has("Choices")
                || sp.has("RememberTokens")
                || sp.has("AtEOTCondition")
                || sp.has("ValidTgts")
            {
                return None;
            }
            let mut clauses = Vec::new();
            if let Some(var) = sp.take("ConditionCheckSVar") {
                let cmp = sp
                    .take("ConditionSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let clause = self
                    .svar_condition_clause(&var, &cmp)
                    .or_else(|| self.wave7_svar_condition(&var, &cmp))
                    .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, true))?;
                clauses.push(clause);
            } else if sp.has("ConditionSVarCompare") {
                return None;
            }
            let on_bf_filter_expr = if let Some(cdef) = sp.take("ConditionDefined") {
                if cdef != "Remembered"
                    || !matches!(
                        sp.take("ConditionCompare").as_deref(),
                        None | Some("GE1" | "EQ1" | "GT0")
                    )
                {
                    return None;
                }
                let present = sp.take("ConditionPresent")?;
                sp.take("ConditionDescription");
                if matches!(present.trim(), "Card" | "Permanent") {
                    "Filter::InZone(ZoneRef::Battlefield)".to_string()
                } else {
                    let inner = self.filter_expr(present.trim())?;
                    format!("Filter::And(&[Filter::InZone(ZoneRef::Battlefield), {inner}])")
                }
            } else {
                if sp.has("ConditionPresent") || sp.has("ConditionCompare") {
                    return None;
                }
                "Filter::InZone(ZoneRef::Battlefield)".to_string()
            };
            let bf_filter = self
                .body
                .filter_static("REANIMATE_COPY", &on_bf_filter_expr);
            clauses.push(format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                 filter: &{bf_filter} }}"
            ));
            let (cp_tapped, attacking, haste_until_eot, cleanup) =
                Self::wave24a_take_copy_riders(&mut sp)?;
            let mods = self.copy_mods(&mut sp, false)?;
            let mods_list = mods.join(", ");
            let controller =
                self.player_of_line(sp.take("Controller").as_deref(), Some(target), false)?;
            let amount = match sp.take("NumCopies") {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self.amount_or_count_ext(&raw)?,
            };
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            self.wave24a_take_optional_cleanup(after, sub)?;
            let ctr_expr = match counters {
                None => "None".to_string(),
                Some((kind, amt)) => {
                    let fixed = amt
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|s| s.strip_suffix(')'))
                        .and_then(|s| s.parse::<u16>().ok())?;
                    format!("Some(({kind}, {fixed}))")
                }
            };
            let full_cond = if clauses.len() == 1 {
                clauses.pop()?
            } else {
                format!("Condition::All(&[{}])", clauses.join(", "))
            };
            return Some(vec![
                format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, \
                     owner_control: false, counters: {ctr_expr} }}"
                ),
                format!(
                    "Effect::IfCondition {{ condition: {full_cond}, \
                     then: &[Effect::CreateTokenCopy {{ target: {target}, \
                     controller: {controller}, amount: {amount}, mods: &[{mods_list}], \
                     tapped: {cp_tapped}, attacking: {attacking}, \
                     haste_until_eot: {haste_until_eot}, cleanup: {cleanup} }}], \
                     otherwise: &[] }}"
                ),
            ]);
        }
        if s_api == "Explore" {
            if !single_target
                || tapped
                || sp.take("Defined").as_deref() != Some("Remembered")
                || sp.has("ValidTgts")
                || sp.has("AtEOT")
                || sp.has("UnlessCost")
                || sp.has("Condition")
                || sp.has("ConditionDefined")
                || sp.has("ConditionPresent")
                || sp.has("ConditionCheckSVar")
            {
                return None;
            }
            let num: usize = match sp.take("Num").as_deref().map(str::trim) {
                None => 1,
                Some(raw) => raw
                    .parse::<usize>()
                    .ok()
                    .filter(|&n| (1..=10).contains(&n))?,
            };
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            self.wave24a_take_optional_cleanup(after, sub)?;
            let ctr_expr = match counters {
                None => "None".to_string(),
                Some((kind, amt)) => {
                    let fixed = amt
                        .strip_prefix("Amount::Fixed(")
                        .and_then(|s| s.strip_suffix(')'))
                        .and_then(|s| s.parse::<u16>().ok())?;
                    format!("Some(({kind}, {fixed}))")
                }
            };
            let bf_filter = self
                .body
                .filter_static("REANIMATE_BF", "Filter::InZone(ZoneRef::Battlefield)");
            let explores: Vec<String> =
                vec![format!("Effect::Explore {{ target: {target} }}"); num];
            return Some(vec![
                format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, \
                     owner_control: false, counters: {ctr_expr} }}"
                ),
                format!(
                    "Effect::IfCondition {{ \
                     condition: Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                     filter: &{bf_filter} }}, \
                     then: &[{}], otherwise: &[] }}",
                    explores.join(", ")
                ),
            ]);
        }
        None
    }

    /// `ChangeZoneAll | Origin$ Graveyard | Destination$ Battlefield | RememberChanged$ True`
    /// followed by `PumpAll | ValidCards$ Card.IsRemembered | KW$ ...` (Wake the Past).
    fn wave24a_reanimate_all_modified(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("GainControl").as_deref(), None | Some("True"))
        {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_type = p.take("ChangeType")?;
        if !p.exhausted() {
            return None;
        }
        let cleaned = Self::strip_your_graveyard_owner(&raw_type)?;
        let filter_expr = self.filter_expr_in_zone(&cleaned, true)?;
        let next_name = sub.take()?;
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&next_body)?;
        sp.drop_prose();
        if s_api != "PumpAll"
            || !matches!(
                sp.take("ValidCards").as_deref(),
                Some("Card.IsRemembered" | "Permanent.IsRemembered")
            )
            || sp.has("Defined")
            || sp.has("AtEOT")
            || sp.has("UnlessCost")
        {
            return None;
        }
        let duration = match sp.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let mut mods = Vec::new();
        if let Some(kws) = sp.take("KW") {
            let bits = Self::wave24a_pump_kw_bits(&kws)?;
            mods.push(format!("Modifier::AddKeyword({bits})"));
        }
        match (sp.take("NumAtt"), sp.take("NumDef")) {
            (None, None) => {}
            (a, d) => {
                let pa = self.plain_num(a.as_deref().unwrap_or("0"))?;
                let pd = self.plain_num(d.as_deref().unwrap_or("0"))?;
                let pa = i8::try_from(pa).ok()?;
                let pd = i8::try_from(pd).ok()?;
                if pa != 0 || pd != 0 {
                    mods.push(format!("Modifier::ModifyPT({pa}, {pd})"));
                }
            }
        }
        let after = sp.take("SubAbility");
        if mods.is_empty() || !sp.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after, sub)?;
        let filter_name = self.body.filter_static("REANIMATE", &filter_expr);
        Some(vec![format!(
            "Effect::MoveAndModify {{ \
             subject: MoveSubject::AllInZone {{ zone: ZoneSel::GraveyardYou, filter: &{filter_name} }}, \
             expected_origin: Some(ZoneRef::Graveyard), blink: false, owner_control: false, \
             tapped: {tapped}, face: None, counters: None, modifiers: &[{}], \
             duration: {duration}, attach_source: false, when_source_leaves: None }}",
            mods.join(", ")
        )])
    }

    /// Handles `ChangeZone | RememberChanged$ True` follow-ups missed by
    /// `remembered_this_way_chain`:
    /// - `ConditionZone$ Exile` on `Graveyard/Battlefield -> Exile` (Corpse Appraiser)
    /// - Combined `ConditionDefined$ Remembered` + `Remembered$Valid` amount on
    ///   `Graveyard -> Exile` (Graveyard Trespasser // Graveyard Glutton)
    /// - `Defined$ TargetedOwner` on `Graveyard -> Exile` `LoseLife` (Carrion Locust)
    /// - `Graveyard -> Hand` single-target `ConditionDefined$ Remembered | ConditionPresent$`
    ///   (Innistrad Charm)
    #[allow(clippy::too_many_lines)]
    fn wave24a_cz_remembered_followup(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let origin = p.peek("Origin")?.to_string();
        let dest = p.peek("Destination")?.to_string();
        let is_gy_hand = origin == "Graveyard" && dest == "Hand";
        let is_exile = dest == "Exile" && matches!(origin.as_str(), "Graveyard" | "Battlefield");
        if !is_gy_hand && !is_exile {
            return None;
        }
        if is_gy_hand
            && (!flags.targets_here
                || !flags.single_target
                || chain.target_up_to.is_some()
                || p.has("Hidden")
                || !target.is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard(")))
        {
            return None;
        }
        if p.take("RememberChanged").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let mut out = self.effect_of_with_prior("ChangeZone", p, target, flags)?;
        if !p.exhausted() {
            return None;
        }
        let this_way_variant = if origin == "Graveyard" {
            ("ExiledFromGraveyard", true, "EXILED_GY")
        } else {
            ("ExiledFromBattlefield", false, "EXILED")
        };
        let is_single_action = flags.single_target
            && !p.has("Hidden")
            && matches!(p.peek("ChangeNum"), None | Some("1"));
        let mut cursor = sub.take();
        let mut consumed = false;
        let mut pending_cond: Option<String> = None;
        let mut pending_body: Vec<String> = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            if sp.peek("ConditionDefined") != Some("Remembered")
                || sp.peek("ConditionPresent").is_none()
                || sp.has("ValidTgts")
                || sp.has("UnlessCost")
                || sp.has("AtEOT")
            {
                *sub = Some(name);
                break;
            }
            if is_exile
                && let Some(cz) = sp.take("ConditionZone")
                && cz != "Exile"
            {
                return None;
            }
            let cmp = sp.take("ConditionCompare");
            let positive_n = match cmp.as_deref() {
                None | Some("GE1" | "GT0") => Some(1u32),
                Some("EQ1") if is_single_action => Some(1u32),
                Some("EQ0" | "LT1" | "LE0") if is_exile => Some(0u32),
                Some(c) if is_exile && c.starts_with("GE") => {
                    c[2..].parse::<u32>().ok().filter(|&n| n >= 1)
                }
                _ => None,
            }?;
            sp.take("ConditionDefined");
            let present = sp.take("ConditionPresent")?;
            sp.take("ConditionDescription");
            let cond = if is_gy_hand {
                let expr = self.filter_expr_in_zone(present.trim(), true)?;
                let filter_name = self.body.filter_static("RETURNED_GY", &expr);
                format!(
                    "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                     filter: &{filter_name} }}"
                )
            } else {
                let (variant, card_zone, prefix) = this_way_variant;
                let filter_ref = if matches!(present.trim(), "Card" | "Permanent") {
                    "&Filter::Any".to_string()
                } else {
                    let expr = self.filter_expr_in_zone(present.trim(), card_zone)?;
                    format!("&{}", self.body.filter_static(prefix, &expr))
                };
                let amt = format!("Amount::ThisWay(ThisWay::{variant}({filter_ref}))");
                if positive_n == 0 {
                    format!("Condition::Not(&Condition::AmountAtLeast({amt}, 1))")
                } else {
                    format!("Condition::AmountAtLeast({amt}, {positive_n})")
                }
            };
            let next = sp.take("SubAbility");
            let step_effects = if is_exile
                && matches!(s_api.as_str(), "LoseLife" | "GainLife")
                && (sp.peek("Defined") == Some("TargetedOwner")
                    || sp.peek("LifeAmount").is_some_and(|raw| {
                        self.wave24a_resolve_exiled_count_amount(raw, this_way_variant)
                            .is_some()
                    })) {
                let who = match sp.take("Defined").as_deref() {
                    Some("TargetedOwner")
                        if origin == "Graveyard"
                            && flags.single_target
                            && target
                                .is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard(")) =>
                    {
                        "PlayerRel::ControllerOfTarget"
                    }
                    other => self.wave11_player_of_line(other, target, false)?,
                };
                let raw_amt = sp.take("LifeAmount")?;
                let amt = self
                    .wave24a_resolve_exiled_count_amount(&raw_amt, this_way_variant)
                    .or_else(|| self.amount_or_count_ext(&raw_amt))?;
                if !sp.exhausted() {
                    return None;
                }
                if s_api == "LoseLife" {
                    vec![format!(
                        "Effect::LoseLife {{ amount: {amt}, target: {who} }}"
                    )]
                } else if who == "PlayerRel::You" {
                    vec![format!("Effect::GainLife {{ amount: {amt} }}")]
                } else {
                    vec![format!(
                        "Effect::GainLifeFor {{ amount: {amt}, who: {who} }}"
                    )]
                }
            } else {
                if sp
                    .entries
                    .iter()
                    .any(|(_, v)| v.contains("Remembered") || v.contains("IsRemembered"))
                {
                    return None;
                }
                chain.could_add_mana |= s_api == "Mana";
                chain.moves_library |= matches!(
                    s_api.as_str(),
                    "Draw" | "Mill" | "Surveil" | "Dig" | "DigUntil"
                );
                let effs = self.effect_of_followup(&s_api, &mut sp, target)?;
                if !sp.exhausted() {
                    return None;
                }
                effs
            };
            if step_effects.is_empty() {
                return None;
            }
            if pending_cond.as_deref() == Some(cond.as_str()) {
                pending_body.extend(step_effects);
            } else {
                if let Some(prev) = pending_cond.take() {
                    out.push(format!(
                        "Effect::IfCondition {{ condition: {prev}, then: &[{}], otherwise: &[] }}",
                        pending_body.join(", ")
                    ));
                    pending_body.clear();
                }
                pending_cond = Some(cond);
                pending_body.extend(step_effects);
            }
            consumed = true;
            cursor = next;
        }
        if let Some(prev) = pending_cond.take() {
            out.push(format!(
                "Effect::IfCondition {{ condition: {prev}, then: &[{}], otherwise: &[] }}",
                pending_body.join(", ")
            ));
        }
        consumed.then_some(out)
    }

    fn wave24a_resolve_exiled_count_amount(
        &mut self,
        raw: &str,
        this_way_variant: (&'static str, bool, &'static str),
    ) -> Option<String> {
        let def = self
            .svars
            .get(raw.trim())
            .map_or(raw.trim(), String::as_str)
            .trim()
            .to_string();
        let (variant, card_zone, prefix) = this_way_variant;
        let filter_ref = if matches!(def.as_str(), "Remembered$Amount" | "Count$RememberedSize") {
            "&Filter::Any".to_string()
        } else {
            let valid = def
                .strip_prefix("Remembered$Valid ")
                .or_else(|| def.strip_prefix("RememberedLKI$Valid "))?;
            let v = valid.trim();
            if matches!(v, "Card" | "Permanent") {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(v, card_zone)?;
                format!("&{}", self.body.filter_static(prefix, &expr))
            }
        };
        Some(format!("Amount::ThisWay(ThisWay::{variant}({filter_ref}))"))
    }

    /// Single-target `ChangeZone` from Graveyard (`RememberChanged$ True`)
    /// followed by effects reading the target's power/toughness/mana value
    /// (`Remembered$CardPower` / `CardToughness` / `CardManaCost`) via CR 608.2h
    /// `Amount::TargetPower` / `TargetToughness` / `TargetCmc` (Mirkwood Elk,
    /// Queen's Bay Paladin, Volcanic Vision, Golbez, Lagonna-Band Storyteller).
    #[allow(clippy::too_many_lines)]
    fn wave24a_cz_gy_remembered_target_stat_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?;
        if p.has("Hidden") || p.take("RememberChanged").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let dest = p.peek("Destination")?.to_string();
        let mut out = self.change_zone(p, Some(target), true, true)?;
        if !p.exhausted() {
            return None;
        }
        let mut cursor = sub.take();
        let mut consumed = false;
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 4 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            if sp.has("ValidTgts")
                || sp.has("UnlessCost")
                || sp.has("AtEOT")
                || sp.has("ConditionDefined")
                || sp.has("ConditionPresent")
            {
                *sub = Some(name);
                break;
            }
            let cond = if let Some(var) = sp.take("ConditionCheckSVar") {
                let cmp = sp
                    .take("ConditionSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                Some(
                    self.svar_condition_clause(&var, &cmp)
                        .or_else(|| self.wave7_svar_condition(&var, &cmp))
                        .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, true))?,
                )
            } else {
                if sp.has("ConditionSVarCompare") {
                    return None;
                }
                None
            };
            let next = sp.take("SubAbility");
            let step_eff = match s_api.as_str() {
                "GainLife" => {
                    let who = self.wave11_player_of_line(
                        sp.take("Defined").as_deref(),
                        Some(target),
                        false,
                    )?;
                    let amt = self.wave24a_remembered_target_stat(&sp.take("LifeAmount")?)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    if who == "PlayerRel::You" {
                        format!("Effect::GainLife {{ amount: {amt} }}")
                    } else {
                        format!("Effect::GainLifeFor {{ amount: {amt}, who: {who} }}")
                    }
                }
                "LoseLife" => {
                    let who = self.wave11_player_of_line(
                        sp.take("Defined").as_deref(),
                        Some(target),
                        false,
                    )?;
                    let amt = self.wave24a_remembered_target_stat(&sp.take("LifeAmount")?)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
                }
                "DamageAll" => {
                    if sp.has("ValidPlayers") || sp.has("Defined") {
                        return None;
                    }
                    sp.claim_label("ValidDescription", "ValidCards");
                    let valid = sp.take("ValidCards")?;
                    let amt = self.wave24a_remembered_target_stat(&sp.take("NumDmg")?)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    let expr = self.filter_expr(&valid)?;
                    let filter_name = self.body.filter_static("DAMAGE_ALL", &expr);
                    format!("Effect::DamageFilter {{ filter: &{filter_name}, amount: {amt} }}")
                }
                "Draw" => {
                    let who = self.wave11_player_of_line(
                        sp.take("Defined").as_deref(),
                        Some(target),
                        false,
                    )?;
                    let amt = self.wave24a_remembered_target_stat(&sp.take("NumCards")?)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    chain.moves_library = true;
                    if who == "PlayerRel::You" {
                        format!("Effect::DrawCards {{ amount: {amt} }}")
                    } else {
                        format!("Effect::DrawCardsFor {{ amount: {amt}, who: {who} }}")
                    }
                }
                _ => {
                    *sub = Some(name);
                    break;
                }
            };
            let gated = if let Some(c) = cond {
                format!(
                    "Effect::IfCondition {{ condition: {c}, then: &[{step_eff}], otherwise: &[] }}"
                )
            } else if dest == "Exile" && chain.target_up_to.is_some() {
                format!(
                    "Effect::IfCondition {{ \
                     condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ExiledFromGraveyard(&Filter::Any)), 1), \
                     then: &[{step_eff}], otherwise: &[] }}"
                )
            } else {
                step_eff
            };
            out.push(gated);
            consumed = true;
            cursor = next;
        }
        consumed.then_some(out)
    }

    fn wave24a_remembered_target_stat(&self, raw: &str) -> Option<&'static str> {
        let def = self
            .svars
            .get(raw.trim())
            .map_or(raw.trim(), String::as_str)
            .trim();
        match def {
            "Remembered$CardPower" | "RememberedLKI$CardPower" => Some("Amount::TargetPower"),
            "Remembered$CardToughness" | "RememberedLKI$CardToughness" => {
                Some("Amount::TargetToughness")
            }
            "Remembered$CardManaCost" | "RememberedLKI$CardManaCost" => Some("Amount::TargetCmc"),
            _ => None,
        }
    }

    /// `Mill | RememberMilled$ True | SubAbility$ DBDraw` where `DBDraw`'s
    /// `NumCards$` is `Count$Compare <Y> <CMP>.<A>.<B>` against
    /// `Y:Remembered$Valid <filter>` (e.g. Predict).
    #[allow(clippy::too_many_lines)]
    fn wave24a_mill_remembered_compare_draw_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let who_mill =
            self.wave11_player_of_line(p.peek("Defined"), target, flags.targets_a_player)?;
        let is_single_mill =
            Self::is_single_player_rel(who_mill) && matches!(p.peek("NumCards"), None | Some("1"));
        if p.take("RememberMilled").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let mut out = self.effect_of_with_prior("Mill", p, target, flags)?;
        if !p.exhausted() {
            return None;
        }
        let draw_name = sub.take()?;
        let draw_body = self.svars.get(draw_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&draw_body)?;
        sp.drop_prose();
        if s_api != "Draw"
            || sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.has("Optional")
            || sp.has("OptionalDecider")
            || sp.has("Condition")
            || sp.has("ConditionDefined")
            || sp.has("ConditionPresent")
            || sp.has("ConditionCheckSVar")
        {
            return None;
        }
        let who_draw = self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
        let raw_num = sp.take("NumCards")?;
        let def = self
            .svars
            .get(raw_num.trim())
            .map_or(raw_num.trim(), String::as_str)
            .trim()
            .to_string();
        let rest = def.strip_prefix("Count$Compare ")?;
        let (lhs_var, rhs_part) = rest.split_once(' ')?;
        let (cmp_raw, pair) = rhs_part.split_once('.')?;
        let (a_raw, b_raw) = pair.split_once('.')?;
        if a_raw.is_empty() || b_raw.is_empty() || b_raw.contains('.') {
            return None;
        }
        let lhs_def = self
            .svars
            .get(lhs_var.trim())
            .map_or(lhs_var.trim(), String::as_str)
            .trim()
            .to_string();
        let filter_ref = if matches!(
            lhs_def.as_str(),
            "Remembered$Amount" | "Count$RememberedSize"
        ) {
            "&Filter::Any".to_string()
        } else {
            let valid = lhs_def
                .strip_prefix("Remembered$Valid ")
                .or_else(|| lhs_def.strip_prefix("RememberedLKI$Valid "))?;
            let v = valid.trim();
            if matches!(v, "Card" | "Permanent") {
                "&Filter::Any".to_string()
            } else if matches!(v, "Card.NamedCard" | "NamedCard") {
                "&Filter::NamedBySource".to_string()
            } else {
                let expr = self.filter_expr_in_zone(v, true)?;
                format!("&{}", self.body.filter_static("MILLED", &expr))
            }
        };
        let amt_milled = format!("Amount::ThisWay(ThisWay::Milled({filter_ref}))");
        let cond = match cmp_raw.trim() {
            "GE1" | "GT0" => format!("Condition::AmountAtLeast({amt_milled}, 1)"),
            "EQ1" if is_single_mill => format!("Condition::AmountAtLeast({amt_milled}, 1)"),
            "EQ0" | "LT1" | "LE0" => {
                format!("Condition::Not(&Condition::AmountAtLeast({amt_milled}, 1))")
            }
            c if c.starts_with("GE") => {
                let n = c[2..].parse::<u32>().ok().filter(|&n| n >= 1)?;
                format!("Condition::AmountAtLeast({amt_milled}, {n})")
            }
            _ => return None,
        };
        let mut draw_arm = |raw_val: &str| -> Option<String> {
            let trimmed = raw_val.trim();
            if trimmed == "0" {
                return Some(String::new());
            }
            let n = self.amount_or_count_ext(trimmed)?;
            if who_draw == "PlayerRel::You" {
                if let Some(fixed) = n
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    return Some(format!("Effect::draw({fixed})"));
                }
                return Some(format!("Effect::DrawCards {{ amount: {n} }}"));
            }
            Some(format!(
                "Effect::DrawCardsFor {{ amount: {n}, who: {who_draw} }}"
            ))
        };
        let then_draw = draw_arm(a_raw)?;
        let else_draw = draw_arm(b_raw)?;
        let after = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after, sub)?;
        chain.moves_library = true;
        out.push(format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{then_draw}], \
             otherwise: &[{else_draw}] }}"
        ));
        Some(out)
    }

    /// `ChangeZone | ValidTgts$ ... | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True`
    /// followed by one non-Remembered sub-ability (e.g. `Investigate`) and an
    /// end-step `DelayedTrigger` returning `DelayTriggerRememberedLKI` to the
    /// battlefield under owners' control (Disorder in the Court).
    #[allow(clippy::too_many_lines)]
    fn wave24a_exile_then_sub_return_at_end_step(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            || self.has_linked_exile_reader()
            || p.has("Hidden")
            || p.take("Origin").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || !p.exhausted()
        {
            return None;
        }
        let mid_name = sub.take()?;
        let mid_body = self.svars.get(mid_name.trim())?.clone();
        if mid_body.contains("Remembered") || mid_body.contains("IsRemembered") {
            return None;
        }
        let (m_api, mut mp) = Params::parse(&mid_body)?;
        mp.drop_prose();
        if mp.has("ValidTgts") || mp.has("UnlessCost") || mp.has("AtEOT") {
            return None;
        }
        let del_name = mp.take("SubAbility")?;
        let del_body = self.svars.get(del_name.trim())?.clone();
        let (d_api, mut dp) = Params::parse(&del_body)?;
        dp.drop_prose();
        if d_api != "DelayedTrigger"
            || dp.take("Mode").as_deref() != Some("Phase")
            || !dp
                .take("Phase")
                .is_some_and(|ph| ph.eq_ignore_ascii_case("End of Turn"))
            || !matches!(dp.take("ValidPlayer").as_deref(), None | Some("Player"))
            || !matches!(
                dp.take("RememberObjects").as_deref(),
                Some("RememberedLKI" | "Remembered")
            )
        {
            return None;
        }
        if dp.peek("ConditionDefined") == Some("Remembered")
            && dp.peek("ConditionPresent") == Some("Card")
        {
            dp.take("ConditionDefined");
            dp.take("ConditionPresent");
        }
        let ret_name = dp.take("Execute")?;
        let after_del = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        let ret_body = self.svars.get(ret_name.trim())?.clone();
        let (r_api, mut rp) = Params::parse(&ret_body)?;
        rp.drop_prose();
        if r_api != "ChangeZone"
            || !matches!(
                rp.take("Defined").as_deref(),
                Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
            )
            || !matches!(rp.take("Origin").as_deref(), Some("Exile" | "All"))
            || rp.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let tapped = match rp.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !rp.exhausted() {
            return None;
        }
        let mid_effects = self.effect_of_followup(&m_api, &mut mp, target)?;
        if mid_effects.is_empty() || !mp.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after_del, sub)?;
        chain.could_add_mana |= m_api == "Mana";
        chain.moves_library |= matches!(
            m_api.as_str(),
            "Draw" | "Mill" | "Surveil" | "Dig" | "DigUntil"
        );
        let mut out = vec![if tapped {
            "Effect::ExileAndReturnTappedAtEndStep".to_string()
        } else {
            "Effect::ExileAndReturnAtEndStep".to_string()
        }];
        out.extend(mid_effects);
        Some(out)
    }

    /// `ChangeZoneAll | Origin$ Battlefield/Graveyard | Destination$ Exile | RememberChanged$ True`
    /// followed by `Incubate | Amount$ X` where `X: Remembered$Amount` (Sunfall).
    fn wave24a_cz_all_remembered_count_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let origin = p.peek("Origin")?.to_string();
        if p.take("RememberChanged").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let mut out = self.shuffle_into_library_with_prior(
            p,
            target,
            flags.targets_a_player,
            flags.prior_target_used,
        )?;
        if !p.exhausted() {
            return None;
        }
        let next_name = sub.take()?;
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&next_body)?;
        sp.drop_prose();
        if s_api != "Incubate"
            || sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.has("Condition")
            || sp.has("ConditionDefined")
            || sp.has("ConditionPresent")
            || sp.has("ConditionCheckSVar")
        {
            return None;
        }
        let this_way_variant = if origin == "Graveyard" {
            ("ExiledFromGraveyard", true, "EXILED_GY")
        } else {
            ("ExiledFromBattlefield", false, "EXILED")
        };
        let raw_amt = sp.take("Amount")?;
        let count = self.wave24a_resolve_exiled_count_amount(&raw_amt, this_way_variant)?;
        let who = self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
        let times: usize = match sp.take("Times").as_deref().map(str::trim) {
            None => 1,
            Some(raw) => raw
                .parse::<usize>()
                .ok()
                .filter(|&n| (1..=10).contains(&n))?,
        };
        let after = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        self.wave24a_take_optional_cleanup(after, sub)?;
        let one = format!("Effect::Incubate {{ who: {who}, count: {count} }}");
        out.extend(std::iter::repeat_n(one, times));
        Some(out)
    }

    /// True when a card uses `Defined$ ExiledWith` ONLY to return linked-exile
    /// cards to the battlefield (The Princess Takes Flight, The Measure of a Man)
    /// and no `SVar` reads `Remembered`/`IsRemembered`.
    fn wave24a_unused_linked_exile_remember(&self) -> bool {
        if self.oracle.to_ascii_lowercase().contains("exiled with ") {
            return false;
        }
        let mut readers = 0usize;
        for body in self.svars.values() {
            if body.contains("Remembered") || body.contains("IsRemembered") {
                let is_cleanup = Params::parse(body).is_some_and(|(api, mut cp)| {
                    cp.drop_prose();
                    api == "Cleanup"
                        && cp.take("ClearRemembered").as_deref() == Some("True")
                        && cp.exhausted()
                });
                if !is_cleanup {
                    return false;
                }
            }
            if !body.contains("ExiledWith") {
                continue;
            }
            readers += 1;
            let Some((api, mut rp)) = Params::parse(body) else {
                return false;
            };
            rp.drop_prose();
            rp.take("SubAbility");
            if api != "ChangeZone"
                || rp.take("Defined").as_deref() != Some("ExiledWith")
                || rp.take("Origin").as_deref() != Some("Exile")
                || rp.take("Destination").as_deref() != Some("Battlefield")
                || !rp.exhausted()
            {
                return false;
            }
        }
        readers > 0
    }

    fn wave24a_no_svar_mentions_remembered(&self) -> bool {
        !self.svars.values().any(|s| {
            s.contains("Remembered") || s.contains("IsRemembered") || s.contains("ClearRemembered")
        })
    }
}
