//! Reading an effect and the chain of sub-abilities it hands on to, with
//! the "unless" a player may pay.

use super::{
    Chain, Params, Price, Tx, Unless, asks_for_an_object, counter_kind, generic_mana,
    is_supported_api,
};

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
        p.drop_prose();
        chain.could_add_mana |= api == "Mana";
        chain.moves_library |= matches!(api.as_str(), "Draw" | "Mill" | "Surveil" | "DigUntil")
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
                    self.chain_target(&api, &mut p, &valid, chain)?;
                }
                (targets_here, p.take("SubAbility"))
            };
        let second_target_here = !had_second_target && chain.second_target.is_some();
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
            let effects = self.choose_color_animate(p, sub.as_deref(), chain)?;
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "ChooseType" {
            return self.choose_type_chain(p, sub.as_deref(), chain);
        }
        if api == "ChoosePlayer" {
            let effects = self.choose_player_effect(&mut p, sub.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
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
            let effects = self.repeat_each_effect(p, sub.as_deref(), chain)?;
            chain.effects.extend(effects);
            return Some(());
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
        let at_end = if api == "Token"
            && p.peek("TokenAttacking") == Some("True")
            && p.peek("AtEOT") == Some("Sacrifice")
        {
            None
        } else {
            match p.take("AtEOT") {
                Some(what) => Some(self.at_next_end_step(&api, &what, target.as_deref())?),
                None => None,
            }
        };

        let effects = if let Some(split) = self
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
        } else if api == "ChangeZone"
            && targets_here
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && p.has("RememberChanged")
            && !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            let single_target = chain.target_exact.is_none()
                && !chain.target_is_x
                && chain.target_up_to.is_none_or(|n| n == 1);
            self.exile_until_end_effect(&mut p, &mut sub, target.as_deref(), single_target)?
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
        {
            self.exile_top_to_play_effect(&mut p, &mut sub)?
        } else if api == "Dig"
            && p.peek("ChangeNum") == Some("All")
            && p.peek("DestinationZone") == Some("Exile")
            && p.has("RememberChanged")
        {
            self.dig_exile_top_to_play_effect(
                &mut p,
                &mut sub,
                target.as_deref(),
                targets_here,
                targets_a_player,
            )?
        } else if api == "TargetOnly" {
            if !targets_here || second_target_here || had_second_target {
                return self.deny("`TargetOnly` without a single first target".to_string());
            }
            p.take("IsCurse");
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
        } else if api == "DelayedTrigger" {
            vec![self.delayed_trigger(&mut p, target.as_deref(), targets_here, chain)?]
        } else if api == "ImmediateTrigger" {
            vec![self.immediate_trigger(&mut p, target.as_deref(), chain, starts_whenever)?]
        } else {
            self.claim_followed_remember_flag(&api, &mut p, sub.as_deref());
            let Some(effects) = (if second_target_here {
                match api.as_str() {
                    "Pump" => {
                        self.pump_effect(&mut p, target.as_deref().expect("second target"), true)
                    }
                    "Fight" => self.fight_effect(&mut p, true),
                    "DealDamage" => self.bite_effect(&mut p),
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
                self.effect_of_inner(
                    &api,
                    &mut p,
                    target.as_deref(),
                    targets_here,
                    targets_a_player,
                    single_target,
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
        self.chain_sub_ability(sub, chain)
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

    fn effect_target(chain: &Chain) -> Option<String> {
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
                if let Some(cond) = self.svar_condition(&var, &cmp) {
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
        let valid = p.take("ConditionPresent").expect("checked above");
        let cond = self.present_clause(p, &valid)?;
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

    pub(super) fn resolve_target_spec(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        chain: &Chain,
    ) -> Option<String> {
        // A card in a graveyard is a different kind of target from a
        // permanent (CR 115.1 names both, and they are chosen from
        // different zones), so the zone the line moves *from* decides
        // the spec before the valid-string is read.
        let spec = if api == "ChangeZone" && p.peek("Origin") == Some("Graveyard") {
            match p.take("TgtZone").as_deref() {
                None | Some("Graveyard") => {}
                Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
            }
            let defined_ctrl = p.take("TargetsWithDefinedController");
            self.graveyard_target(valid, defined_ctrl.as_deref())
        } else {
            // "Target spell or permanent" (the Laces): the stack is a
            // zone a target may be chosen in only where the line says
            // so. The battlefield alone is what every target already
            // is.
            match p.take("TgtZone").as_deref() {
                None | Some("Battlefield") => {
                    if api == "Counter" && p.peek("TargetType") == Some("Activated") {
                        self.activated_ability_target(valid)
                    } else if api == "CopySpellAbility" {
                        match p.peek("TargetType") {
                            Some("Spell") => self.target_spec(valid, "Counter"),
                            Some("Activated") => self.activated_ability_target(valid),
                            Some("Activated,Triggered" | "Triggered,Activated")
                                if valid == "Card" =>
                            {
                                Some("TargetSpec::AbilityOnStack(&Filter::Any)".to_string())
                            }
                            Some(
                                "Activated.YouCtrl,Triggered.YouCtrl"
                                | "Triggered.YouCtrl,Activated.YouCtrl",
                            ) if valid == "Card" => Some(
                                "TargetSpec::AbilityOnStack(&Filter::ControlledByYou)".to_string(),
                            ),
                            _ => None,
                        }
                    } else if chain.target.is_some()
                        && (chain.target_unique || matches!(api, "Fight" | "DealDamage"))
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
                Some("Stack,Battlefield" | "Battlefield,Stack") => {
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

    /// Register a fresh target declaration without merging independent instances.
    #[allow(clippy::too_many_lines)]
    fn chain_target(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let spec = self.resolve_target_spec(api, p, valid, chain)?;
        if chain.target.is_some() {
            // A fresh ValidTgts declares another target instance even
            // when its restriction is identical. Never alias it to the
            // earlier list.
            if chain.second_target.is_some() || !spec.starts_with("TargetSpec::Object(") {
                return self.deny("unsupported independent target in one chain".to_string());
            }
            let single_object_first = chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
                && chain.target_up_to.is_none()
                && chain.target_exact.is_none()
                && !chain.target_is_x;
            let allowed = match api {
                "Pump" => {
                    p.peek("SubAbility").is_none()
                        && p.peek("Defined").is_none_or(|v| v == "Targeted")
                        && ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                            .iter()
                            .all(|key| p.peek(key).is_none())
                        && ["NumAtt", "NumDef"]
                            .iter()
                            .all(|key| p.peek(key).is_none_or(|value| value.parse::<i16>().is_ok()))
                }
                "Fight" => {
                    single_object_first
                        && p.peek("Defined") == Some("ParentTarget")
                        && ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                            .iter()
                            .all(|key| p.peek(key).is_none())
                }
                "DealDamage" => {
                    single_object_first
                        && p.peek("Defined").is_none()
                        && p.peek("DamageSource") == Some("ParentTarget")
                        && ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                            .iter()
                            .all(|key| p.peek(key).is_none())
                }
                "Destroy" => {
                    !chain.effects.is_empty()
                        && chain
                            .effects
                            .iter()
                            .all(|e| e.starts_with("Effect::destroy"))
                        && p.peek("SubAbility").is_none()
                        && p.peek("Defined").is_none_or(|v| v == "Targeted")
                        && ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                            .iter()
                            .all(|key| p.peek(key).is_none())
                }
                _ => false,
            };
            if !allowed {
                return self.deny("unsupported independent target in one chain".to_string());
            }
            if matches!(api, "Fight" | "DealDamage")
                && p.peek("TargetMin") == Some("0")
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
                let multi_target_ok =
                    matches!(api, "Destroy" | "Tap" | "Untap" | "Pump" | "PutCounter")
                        || (api == "ChangeZone"
                            && matches!(
                                (p.peek("Origin"), p.peek("Destination")),
                                (Some("Battlefield" | "Graveyard"), Some("Hand" | "Exile"))
                                    | (Some("Graveyard"), Some("Battlefield"))
                            ));
                if p.peek("TargetMin") == Some("1")
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
                } else if (matches!(api, "Fight" | "Attach" | "MustBlock" | "Effect")
                    || (api == "DealDamage" && !p.has("DividedAsYouChoose"))
                    || (api == "ChangeZone"
                        && matches!(
                            (p.peek("Origin"), p.peek("Destination")),
                            (Some("Battlefield" | "Graveyard"), Some("Library"))
                        )))
                    && p.peek("TargetMin") == Some("0")
                    && p.peek("TargetMax") == Some("1")
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
        let payer = match payer.as_deref() {
            Some("You") => "PlayerRel::You",
            // Paralyze's "that player may pay {4}": the enchanted
            // creature's controller (CR 303.4e names the Aura's host).
            Some("EnchantedController") => "PlayerRel::ControllerOfAttached",
            None | Some("TargetedController") if targets_a_controlled_object => {
                "PlayerRel::ControllerOfTarget"
            }
            Some("Targeted") if targets_a_player => "PlayerRel::Chosen",
            // Rhystic Study's "unless that player pays {1}": the player the
            // cast trigger's event is about, as `Defined$` reads the same
            // words (`Tx::player_rel`). Anywhere but a cast trigger they
            // name nobody this reader can see, and stay refused below.
            Some(who @ ("TriggeredActivator" | "TriggeredCardController"))
                if self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                self.player_rel(Some(who))?
            }
            Some("TriggeredCardController") if self.trigger_mode.as_deref() == Some("Drawn") => {
                self.player_rel(Some("TriggeredCardController"))?
            }
            Some("TriggeredTargetController")
                if self.trigger_mode.as_deref() == Some("DamageDone") =>
            {
                self.player_rel(Some("TriggeredTargetController"))?
            }
            Some("TriggeredDefendingPlayer")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerBlocked")
                ) =>
            {
                self.player_rel(Some("TriggeredDefendingPlayer"))?
            }
            Some("TriggeredPlayer") if self.trigger_mode.as_deref() == Some("Phase") => {
                "PlayerRel::ActivePlayer"
            }
            other => {
                return self.deny(format!(
                    "an unless-cost paid by `{}`",
                    other.unwrap_or("TargetedController")
                ));
            }
        };
        let price = self.unless_price_switched(cost.trim(), switched)?;
        Some(Unless {
            payer,
            price,
            switched,
            resolve_subs,
        })
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
            let Some(x) = self.amount_or_count(raw) else {
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
            ("", [one]) if switched && one.starts_with("PayLife(") && one.ends_with(')') => {
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
            (Price::Part(part), false) => format!(
                "Effect::PlayerMayPayCostOr {{ player: {payer}, cost: &CostPart::{part}, \
                 effect: &{one} }}"
            ),
            (Price::Part(part), true) => {
                if let Some(n) = part
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
        self.effect_of_inner(api, p, target, targets_here, targets_a_player, false)
    }

    #[allow(clippy::too_many_lines)]
    fn effect_of_inner(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
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
                // "Deals 4 damage to any target and 2 damage to you"
                // (Psionic Blast): the reference gathers both into one
                // simultaneous event and deals it at `DamageResolve`. The
                // engine journals one `DamageDealt` per recipient either
                // way, as it does for `DealDamageEach`, and nothing checks
                // state-based actions between two effects of one
                // resolution, so the two in sequence are the same event.
                if p.take("DamageMap").is_some_and(|v| v != "True") {
                    return None;
                }
                if damage_source.as_deref() == Some("Targeted") {
                    if !targets_here
                        || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                        || p.take("Defined").is_some()
                        || !self.is_power_of(&num_dmg, &["Targeted$CardPower"])
                    {
                        return None;
                    }
                    let dmg = "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::First }".to_string();
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
                    let n = self.amount_or_count(&num_dmg)?;
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
                            if self.trigger_mode.as_deref() == Some("ChangesZone")
                                && target.is_none() =>
                        {
                            "TargetSpec::EventObject".to_string()
                        }
                        Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                            if self.trigger_mode.as_deref() == Some("DamageDone")
                                && target.is_none() =>
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
                            self.player_of_line(Some(who), target, false)?
                        ),
                    }
                };
                let dmg = if let Some(src) = damage_source {
                    let power_names: &[&str] = match (src.as_str(), self.trigger_mode.as_deref()) {
                        ("TriggeredCard" | "TriggeredCardLKICopy", Some("ChangesZone")) => {
                            &["TriggeredCard$CardPower", "TriggeredCardLKICopy$CardPower"]
                        }
                        (
                            "TriggeredAttacker" | "TriggeredAttackerLKICopy",
                            Some("Attacks" | "AttackerUnblocked"),
                        ) => &[
                            "TriggeredAttacker$CardPower",
                            "TriggeredAttackerLKICopy$CardPower",
                        ],
                        ("TriggeredBlocker" | "TriggeredBlockerLKICopy", Some("Blocks")) => &[
                            "TriggeredBlocker$CardPower",
                            "TriggeredBlockerLKICopy$CardPower",
                        ],
                        _ => return None,
                    };
                    if self.is_power_of(&num_dmg, power_names) {
                        format!("Effect::EventObjectDealsDamageEqualToPower {{ target: {to} }}")
                    } else {
                        let n = self.amount_or_count(&num_dmg)?;
                        format!("Effect::EventObjectDealsDamage {{ amount: {n}, target: {to} }}")
                    }
                } else {
                    let n = self.amount_or_count(&num_dmg)?;
                    format!("Effect::DealDamage {{ amount: {n}, target: {to} }}")
                };
                let dmg = if turn_start_hand == Some(true) {
                    format!(
                        "Effect::IfCondition {{ condition: Condition::Not(&Condition::HandSizeAtTurnStartAtMost(0)), then: &[{dmg}], otherwise: &[] }}"
                    )
                } else {
                    dmg
                };
                p.claim_label("ConditionDescription", "ConditionPresent");
                let condition = match (
                    p.take("ConditionDefined").as_deref(),
                    p.take("ConditionPresent").as_deref(),
                    p.take("ConditionCompare").as_deref(),
                ) {
                    (None, None, None) => None,
                    (Some("Targeted"), Some("Land.Basic"), Some("EQ0")) => {
                        Some("Filter::NONBASIC_LAND")
                    }
                    _ => return None,
                };
                let dmg = if let Some(filter) = condition {
                    format!("Effect::IfTargetMatches {{ filter: &{filter}, then: &[{dmg}] }}")
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
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::gain_life({fixed})")],
                    ("PlayerRel::You", None) => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    (who, _) => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Dig" => self.dig_effect(p, target, targets_here, targets_a_player)?,
            "DigUntil" => {
                let snapshot = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(effects) = self.dig_until_effect(p, target, targets_a_player) {
                    effects
                } else {
                    *p = snapshot;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    self.wave4_dig_until_effect(p, target, targets_a_player)?
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
                let n = self.amount_or_count(p.take("NumCards").as_deref().unwrap_or("1"))?;
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
                let n = self.amount_or_count(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let may = match p.take("OptionalDecider").as_deref() {
                    None => false,
                    Some("You") => true,
                    Some(_) => return None,
                };
                if p.peek("Defined") == Some("TargetedAndYou") && targets_a_player {
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
                        format!("Effect::DrawCardsFor {{ amount: {n}, who: PlayerRel::Chosen }}");
                    return Some(if may {
                        vec![format!(
                            "Effect::MayDo {{ effects: &[{you_draw}, {other_draw}] }}"
                        )]
                    } else {
                        vec![you_draw, other_draw]
                    });
                }
                let draw = match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
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
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::DiscardHand {{ who: {who} }}")]
            }
            "Discard" if p.peek("Mode") == Some("TgtChoose") => {
                if p.peek("Cost").is_some() {
                    if p.take("Cost").as_deref() != Some("Draw<2/You>")
                        || p.take("Mode").as_deref() != Some("TgtChoose")
                        || p.take("NumCards").as_deref() != Some("1")
                        || p.take("Defined").as_deref() != Some("You")
                    {
                        return None;
                    }
                    return Some(vec![
                        "Effect::MayDo { effects: &[Effect::draw(2), Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }] }"
                            .to_string(),
                    ]);
                }
                p.take("Mode");
                p.take("IsCurse");
                if let Some(remember) = p.take("RememberDiscarded")
                    && remember != "True"
                {
                    return None;
                }
                let raw_n = p.take("NumCards").unwrap_or_else(|| "1".to_string());
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                if let Some(n) = self.plain_num(&raw_n).and_then(|n| u8::try_from(n).ok()) {
                    vec![format!(
                        "Effect::DiscardForPlayers {{ who: {who}, count: {n} }}"
                    )]
                } else {
                    let amount = self.amount_or_count(&raw_n)?;
                    vec![format!(
                        "Effect::DiscardAmountForPlayers {{ who: {who}, amount: {amount} }}"
                    )]
                }
            }
            "Discard" if p.peek("Mode") == Some("RevealDiscardAll") => {
                p.take("Mode");
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
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
                    Some(valid) => self.filter_expr(valid)?,
                };
                vec![format!("Effect::RevealHandDiscard {{ filter: &{filter} }}")]
            }
            "Discard" => {
                // Refuse other modes instead of turning a chosen discard
                // into a random one. Unconsumed qualifiers also refuse it.
                if p.take("Mode").as_deref() != Some("Random") {
                    return None;
                }
                let n = self.amount_or_count(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardRandom {{ who: {who}, count: {n} }}"
                )]
            }
            "Mill" => {
                let n = self.amount_or_count(&p.take("NumCards")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "PutCounter" => {
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
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = self.amount_or_count(p.take("CounterNum").as_deref().unwrap_or("1"))?;
                if matches!(code.as_str(), "ENERGY" | "POISON" | "RAD")
                    && (targets_a_player
                        || p.peek("Defined").is_some_and(|d| {
                            self.player_of_line(Some(d), target, targets_a_player)
                                .is_some()
                        }))
                {
                    let who = self.player_of_line(
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
                p.take("IsCurse");
                match p.take("Defined").as_deref() {
                    Some("Self") if target.is_some() || self.counter_on_source => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::This, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    None if self.counter_on_source => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::This, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    Some("Targeted" | "ParentTarget")
                        if !targets_here && target.is_some() && !targets_a_player => {}
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::AttachedToBySource, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    Some("TriggeredCardLKICopy" | "TriggeredCard")
                        if self.trigger_mode.as_deref() == Some("ChangesZone")
                            && target.is_none() =>
                    {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    Some(def)
                        if self.trigger_mode.as_deref() == Some("ChangesZone")
                            && target.is_none()
                            && (def.starts_with("TriggeredCardLKICopy.")
                                || def.starts_with("TriggeredCard.")) =>
                    {
                        let qual = def
                            .strip_prefix("TriggeredCardLKICopy.")
                            .or_else(|| def.strip_prefix("TriggeredCard."))?;
                        let q = self.filter_expr(&format!("Card.{qual}"))?;
                        let expr = format!("Filter::And(&[Filter::Reference, {q}])");
                        let name = self.body.filter_static("COUNTER", &expr);
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &{name}, kind: {kind}, amount: {n} }}"
                        )]);
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
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                             kind: {kind}, amount: {n} }}"
                        )]);
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
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                        if self.trigger_mode.as_deref() == Some("DamageDone")
                            && target.is_none() =>
                    {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                        if self.in_delayed && target.is_none() =>
                    {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    None | Some("Self") => {}
                    Some(_) => return None,
                }
                vec![format!(
                    "Effect::AddCounter {{ kind: {kind}, amount: {n} }}"
                )]
            }
            "PutCounterAll" => {
                if target.is_some() {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = self.amount_or_count(p.take("CounterNum").as_deref().unwrap_or("1"))?;
                p.take("IsCurse");
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("COUNTERS", &expr);
                vec![format!(
                    "Effect::AddCounterFilter {{ filter: &{filter}, kind: {kind}, amount: {n} }}"
                )]
            }
            "Proliferate" => {
                if target.is_some() {
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
                if self.on_a_spell
                    || p.take("Mode").as_deref() != Some("Transform")
                    || p.take("Defined").as_deref() != Some("Self")
                {
                    return None;
                }
                vec!["Effect::TransformSource".to_string()]
            }
            "RemoveCounterAll" => {
                if target.is_some() || p.take("AllCounters").as_deref() != Some("True") {
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
                if p.take("ClearRemembered").as_deref() != Some("True") || !p.exhausted() {
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
                    if p.take("RememberRemoved").as_deref() != Some("True") {
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
                let n = self.plain_num(p.take("ScryNum").as_deref().unwrap_or("1"))?;
                vec![format!("Effect::scry({n})")]
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
                    let n = self.amount_or_count(&raw)?;
                    vec![format!("Effect::Surveil {{ amount: {n} }}")]
                }
            }
            "Mana" => self.mana_effect(p)?,
            "Branch" => {
                let var = p.take("BranchConditionSVar")?;
                let cmp = p
                    .take("BranchConditionSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let cond = self.svar_condition(&var, &cmp)?;
                let true_sub = p.take("TrueSubAbility")?;
                let false_sub = p.take("FalseSubAbility")?;
                let true_body = self.svars.get(true_sub.trim())?.clone();
                let false_body = self.svars.get(false_sub.trim())?.clone();
                let mut then_chain = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                let mut else_chain = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                self.chain(&true_body, &mut then_chain)?;
                self.chain(&false_body, &mut else_chain)?;
                if then_chain.effects.is_empty() || else_chain.effects.is_empty() {
                    return None;
                }
                vec![format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
                    then_chain.effects.join(", "),
                    else_chain.effects.join(", ")
                )]
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
                let valid = p.take("ValidCards")?;
                let filter = self.filter_expr(&valid)?;
                if !self.on_a_spell
                    && (filter.contains("Filter::CmcAtMostX")
                        || filter.contains("Filter::CmcExactlyX"))
                {
                    return None;
                }
                let name = self.body.filter_static("SWEEP", &filter);
                vec![format!("Effect::SacrificeAll {{ filter: &{name} }}")]
            }
            // "Destroy all lands", "destroy all creatures. They can't be
            // regenerated": every permanent the valid-string names, none of
            // them a target (so an ability that also targets is refused
            // rather than read as a sweep of its target). `NoRegen$` is the
            // same two doors as the single destroy above (CR 701.19c).
            "DestroyAll" => {
                let valid = p.take("ValidCards")?;
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
                if target.is_some() {
                    return None;
                }
                if let Some(filter) = self.filter_expr(&valid) {
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
                    self.amount_or_count(&raw_amount)?
                };
                let to = match (p.take("Defined").as_deref(), target) {
                    (Some("Self"), None) if !self.on_a_spell => {
                        "TargetSpec::ThisObject".to_string()
                    }
                    (Some(who), None) => {
                        format!("TargetSpec::Player({})", self.player_rel(Some(who))?)
                    }
                    (None, Some(aimed)) => aimed.to_string(),
                    _ => return None,
                };
                vec![format!(
                    "Effect::PreventNextDamage {{ target: {to}, amount: {n} }}"
                )]
            }
            "RevealHand" => {
                if p.has("RememberRevealed") {
                    return None;
                }
                let lower = self.oracle.to_ascii_lowercase();
                if !(lower.contains("reveals their hand")
                    || lower.contains("reveal your hand")
                    || lower.contains("reveals his or her hand")
                    || lower.contains("reveal their hands"))
                    || lower.contains("look at")
                {
                    return None;
                }
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::RevealHand {{ who: {who} }}")]
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
                    None => self.amount_or_count(&raw_dmg)?,
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
            "Play" => {
                if target.is_some()
                    || p.take("Valid").as_deref() != Some("Card.IsImprinted+ExiledWithSource")
                    || p.take("ValidZone").as_deref() != Some("Exile")
                    || p.take("Amount").as_deref() != Some("All")
                    || p.take("WithoutManaCost").as_deref() != Some("True")
                    || p.take("ValidSA").as_deref() != Some("Spell")
                    || p.take("Optional").as_deref() != Some("True")
                    || p.take("CopyCard").as_deref() != Some("True")
                {
                    return None;
                }
                vec!["Effect::MayCopyCastLinkedExile".to_string()]
            }
            "Token" => self.token_effect(p, target, targets_a_player)?,
            "Investigate" => self.investigate_effect(p, targets_a_player)?,
            "Animate" => self.animate_effect(p, target)?,
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
            "AnimateAll" => self.animate_all_effect(p, target)?,
            "Pump" if p.peek("KW") == Some("Protection from everything") => {
                self.everything_protection(p)?
            }
            "Phases" => self.phase_your_permanents(p)?,
            "Pump" => self.pump_effect(p, aimed, false)?,
            "PumpAll" => self.pump_all_effect(p, target, targets_here, targets_a_player)?,
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
                    } else if target.is_none() && snapshot.peek("ReplacementEffects").is_some() {
                        *p = Params {
                            entries: Vec::new(),
                        };
                        *self.unclaimed.borrow_mut() = saved_unclaimed;
                        self.body.statics.truncate(saved_statics_len);
                        let mut dummy_chain = Chain::default();
                        self.prevention_effect_shield(snapshot, &mut dummy_chain)?
                    } else {
                        return None;
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
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::ShuffleLibrary {{ who: {who} }}")]
            }
            "ChangeZone" => self.change_zone(p, target, targets_here, single_target)?,
            "ChangeZoneAll" => self.shuffle_into_library(p, target, targets_a_player)?,
            // "Look at the top three cards of target player's library and
            // put them back in any order. You may have that player shuffle"
            // (Natural Selection). The ability's controller looks and
            // decides; a count the player announced is refused.
            "RearrangeTopOfLibrary" => {
                let count: u8 = p.take("NumCards")?.parse().ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
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
                    if target.is_some() {
                        return None;
                    }
                    return Some(vec![format!("Effect::TapAll {{ filter: &{filter} }}")]);
                }
                let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::TapAllOf {{ who: {who}, filter: &{filter} }}"
                )]
            }
            // "That player loses all unspent mana" (CR 106.4).
            "DrainMana" => {
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseUnspentMana {{ who: {who} }}")]
            }
            // "You may tap or untap target artifact, creature, or land"
            // (Twiddle): one of the two choices always does nothing, so
            // the choice is a yes or a no (`Effect::ToggleTapTarget`).
            "TapOrUntap" if target.is_some() && p.take("Defined").is_none() => {
                vec!["Effect::MayDo { effects: &[Effect::ToggleTapTarget] }".to_string()]
            }
            "Tap" => match p.take("Defined").as_deref() {
                None => vec!["Effect::TapTarget".to_string()],
                Some("Enchanted" | "Equipped") if target.is_none() => {
                    vec!["Effect::TapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                Some(def) if target.is_none() && self.is_event_object_defined(def) => {
                    vec!["Effect::TapEventObject".to_string()]
                }
                Some(_) => return None,
            },
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(_)) => vec!["Effect::UntapTarget".to_string()],
                (None, None) | (Some("Self"), _) => vec!["Effect::UntapSelf".to_string()],
                // "Untap enchanted creature" (Instill Energy).
                (Some("Enchanted" | "Equipped"), None) => {
                    vec!["Effect::UntapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                _ => return None,
            },
            "UntapAll" => {
                if target.is_some() {
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
                let dest = p.take("Destination");
                match (dest.as_deref(), p.take("TargetType").as_deref()) {
                    (None | Some("Graveyard"), Some("Spell")) => {
                        vec!["Effect::CounterTargetSpell".to_string()]
                    }
                    (Some("Exile"), Some("Spell")) => {
                        vec!["Effect::CounterTargetSpellToExile".to_string()]
                    }
                    (None | Some("Graveyard"), Some("Activated")) => {
                        vec!["Effect::CounterTargetAbility".to_string()]
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
                let n = self.amount_or_count(&p.take("Num")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::Poison {{ who: {who}, amount: {n} }}")]
            }
            "WinsGame" => {
                if target.is_some() || p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec!["Effect::WinGame".to_string()]
            }
            _ => return None,
        })
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

    fn copy_spell_ability_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if p.take("MayChooseTarget").as_deref() != Some("True") {
            return None;
        }
        let optional = match p.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !matches!(p.take("Controller").as_deref(), None | Some("You")) {
            return None;
        }
        let target_type = p.take("TargetType");
        let defined = p.take("Defined");
        let take_small_copies = |this: &Self, p: &mut Params| -> Option<usize> {
            match p.take("Amount") {
                None => Some(1),
                Some(raw) => usize::try_from(this.plain_num(&raw)?)
                    .ok()
                    .filter(|&n| (1..=4).contains(&n)),
            }
        };
        let (effect, count) = match (target_type.as_deref(), defined.as_deref()) {
            (Some("Spell"), None)
                if targets_here && target.is_some_and(|t| t.starts_with("TargetSpec::Spell(")) =>
            {
                (
                    "Effect::CopyTargetSpell { mods: &[] }".to_string(),
                    take_small_copies(self, p)?,
                )
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
                (
                    "Effect::CopyTargetAbility".to_string(),
                    take_small_copies(self, p)?,
                )
            }
            (None, Some("TriggeredSpellAbility"))
                if self.trigger_mode.as_deref() == Some("SpellCast")
                    && !targets_here
                    && target.is_none() =>
            {
                (
                    "Effect::CopyEventSpellOrAbility".to_string(),
                    take_small_copies(self, p)?,
                )
            }
            (None, Some("Parent")) if self.on_a_spell && !targets_here => {
                let amount = self.amount_or_count(p.take("Amount").as_deref().unwrap_or("1"))?;
                (format!("Effect::CopyThisSpell {{ amount: {amount} }}"), 1)
            }
            _ => return None,
        };
        let copies = vec![effect; count];
        Some(if optional {
            vec![format!(
                "Effect::MayDo {{ effects: &[{}] }}",
                copies.join(", ")
            )]
        } else {
            copies
        })
    }

    fn sub_is_api(&self, sub: Option<&str>, expected: &str) -> bool {
        sub.and_then(|name| self.svars.get(name.trim()))
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, _)| api == expected)
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
                "ChangeZone"
                    if p.peek("Origin") == Some("Battlefield")
                        && matches!(p.peek("Destination"), Some("Exile" | "Hand"))
                        && p.peek("RememberChanged") == Some("True") =>
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
        }
    }
}
