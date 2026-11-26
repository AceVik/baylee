//! Regeneration restrictions created by resolving effects.

use super::{Params, Tx};

impl Tx<'_> {
    /// Consume a complete damage-memory suffix only on the initial instruction.
    pub(super) fn take_damage_recipient_regeneration(
        &self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        initial: bool,
    ) -> Option<bool> {
        if !matches!(api, "DealDamage" | "DamageAll") || !p.has("RememberDamaged") {
            return Some(false);
        }
        // Runtime damage facts span the resolution, not one instruction.
        if !initial {
            return self.deny("damage-recipient memory after an earlier instruction".into());
        }
        if p.peek("RememberDamaged") != Some("True") {
            return self.deny("a nontrue damage-recipient memory flag".into());
        }
        if let Some(replace) = p.peek("ReplaceDyingDefined")
            && !p.has("ReplaceDyingCondition")
            && matches!(p.peek("DamageSource"), None | Some("Self"))
            && self.remembered_damage_cleanup_only(sub.as_deref())
        {
            let marker = match replace {
                "Remembered" => Some("__DamagedThisWay"),
                "Remembered.Creature" if self.on_a_spell => Some("__DamagedThisWayCreature"),
                _ => None,
            };
            if let Some(marker) = marker {
                p.take("RememberDamaged");
                for (k, v) in &mut p.entries {
                    if k == "ReplaceDyingDefined" {
                        *v = marker.to_string();
                        break;
                    }
                }
                return Some(false);
            }
        }
        if api != "DealDamage" {
            return Some(false);
        }
        if p.peek("Defined") == Some("You")
            && p.peek("OptionalDecider") == Some("You")
            && self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            return Some(false);
        }
        p.take("RememberDamaged");
        self.damage_recipient_regeneration(sub.as_deref()?)?;
        *sub = None;
        Some(true)
    }

    /// Consume `RememberDamaged$ True` on `DealDamage | Defined$ You | UnlessCost$`
    /// followed by `DB$ Tap | [Defined$ Self] | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You`
    /// and a `ClearRemembered$ True` cleanup (*Minion of Leshrac*, *Mishra's War Machine*).
    pub(super) fn take_damage_to_you_tap_self(
        &self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        initial: bool,
    ) -> Option<bool> {
        if !initial
            || api != "DealDamage"
            || self.on_a_spell
            || p.peek("RememberDamaged") != Some("True")
            || p.peek("Defined") != Some("You")
            || !p.has("UnlessCost")
            || !matches!(p.peek("UnlessResolveSubs"), None | Some("WhenNotPaid"))
        {
            return Some(false);
        }
        let Some(sub_name) = sub.as_deref() else {
            return Some(false);
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return Some(false);
        };
        let Some((tap_api, mut sp)) = Params::parse(body) else {
            return Some(false);
        };
        sp.drop_prose();
        if tap_api != "Tap"
            || !matches!(sp.take("Defined").as_deref(), None | Some("Self"))
            || sp.take("ConditionPlayerDefined").as_deref() != Some("Remembered")
            || sp.take("ConditionPlayerContains").as_deref() != Some("You")
        {
            return Some(false);
        }
        let cleanup_name = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return Some(false);
        }
        let cleanup_body = self.svars.get(cleanup_name.trim())?;
        let (cleanup_api, mut cp) = Params::parse(cleanup_body)?;
        cp.drop_prose();
        if cleanup_api != "Cleanup"
            || cp.take("ClearRemembered").as_deref() != Some("True")
            || !cp.exhausted()
        {
            return Some(false);
        }
        p.take("RememberDamaged");
        if p.peek("UnlessResolveSubs") == Some("WhenNotPaid") {
            p.take("UnlessResolveSubs");
        }
        *sub = None;
        Some(true)
    }

    /// Returns `true` iff the `SubAbility$` chain reaches a `Cleanup` with
    /// `ClearRemembered$ True` and no earlier instruction in the chain reads
    /// `Remembered` (preserving any follow-up after `Cleanup`, e.g. shuffle or
    /// self-exile, and any unrelated intervening step such as `Scry`).
    fn remembered_damage_cleanup_only(&self, start: Option<&str>) -> bool {
        let mut cur = start.map(|s| s.trim().to_string());
        for _ in 0..8 {
            let Some(name) = cur else {
                return false;
            };
            let Some(body) = self.svars.get(name.as_str()) else {
                return false;
            };
            let Some((api, mut sp)) = Params::parse(body) else {
                return false;
            };
            sp.drop_prose();
            if api == "Cleanup" && sp.peek("ClearRemembered") == Some("True") {
                return true;
            }
            if sp
                .entries
                .iter()
                .any(|(k, v)| k.contains("Remember") || v.contains("Remembered"))
            {
                return false;
            }
            cur = sp
                .entries
                .iter()
                .find_map(|(k, v)| (k == "SubAbility").then(|| v.trim().to_string()));
        }
        false
    }

    /// A temporary source static ability grant on `Animate`.
    /// Every referenced static is read as an exact rule, never as an ignored label.
    pub(super) fn animate_regeneration_grant(
        &mut self,
        p: &mut Params,
        source_only: bool,
        duration: &str,
    ) -> Option<Vec<String>> {
        let lower = p.take("staticAbilities");
        let upper = p.take("StaticAbilities");
        let name = match (lower, upper) {
            (None, None) => return Some(Vec::new()),
            (Some(name), None) | (None, Some(name)) if source_only => name,
            _ => {
                return self.deny(
                    "a static animation grant with multiple references or another subject".into(),
                );
            }
        };
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.contains(',') {
            return self.deny("a regeneration restriction naming multiple or no statics".into());
        }
        let body = self.svars.get(trimmed)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        match mode.as_str() {
            "CantRegenerate" => {
                let valid = st.take("ValidCard")?;
                if !st.exhausted() {
                    return self.deny(
                        "a referenced static other than an unconditional regeneration restriction"
                            .into(),
                    );
                }
                if valid != "Creature.DamagedBy" {
                    return self.deny(format!("a granted regeneration rule on `{valid}`"));
                }
                Some(vec![Self::animate_expr(
                    "Modifier::GrantStatic { filter: &Filter::And(&[Filter::CREATURE, \
                     Filter::DealtDamageBySourceThisTurn]), modifier: &Modifier::CantBeRegenerated }",
                    duration,
                )])
            }
            "CantBlockBy" => {
                let attacker = st.take("ValidAttacker")?;
                if !matches!(attacker.as_str(), "Creature.Self" | "Card.Self") || !st.exhausted() {
                    return self.deny(
                        "a referenced static other than an unconditional regeneration restriction"
                            .into(),
                    );
                }
                Some(vec![Self::animate_expr(
                    "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)",
                    duration,
                )])
            }
            "CantAttack" => self.animate_cant_attack_grant(st, duration),
            "CantBlock" => {
                let valid = st.take("ValidCard")?;
                if !matches!(valid.as_str(), "Creature.Self" | "Card.Self") || !st.exhausted() {
                    return self.deny(
                        "a referenced static other than an unconditional regeneration restriction"
                            .into(),
                    );
                }
                Some(vec![Self::animate_expr(
                    "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)",
                    duration,
                )])
            }
            "MustAttack" => {
                let valid = st.take("ValidCreature")?;
                if !matches!(valid.as_str(), "Creature.Self" | "Card.Self") || !st.exhausted() {
                    return self.deny(
                        "a referenced static other than an unconditional regeneration restriction"
                            .into(),
                    );
                }
                Some(vec![Self::animate_expr(
                    "Modifier::AttacksEachCombat",
                    duration,
                )])
            }
            "AssignNoCombatDamage" => {
                let valid = st.take("ValidCard")?;
                if !matches!(valid.as_str(), "Creature.Self" | "Card.Self") || !st.exhausted() {
                    return self.deny(
                        "a referenced static other than an unconditional regeneration restriction"
                            .into(),
                    );
                }
                Some(vec![Self::animate_expr(
                    "Modifier::AssignsNoCombatDamage",
                    duration,
                )])
            }
            "Continuous" => self.animate_continuous_grant(st, duration),
            _ => self.deny(
                "a referenced static other than an unconditional regeneration restriction".into(),
            ),
        }
    }

    fn animate_cant_attack_grant(&mut self, mut st: Params, duration: &str) -> Option<Vec<String>> {
        let valid = st.take("ValidCard")?;
        let unless = st.take("UnlessDefender")?;
        if !matches!(valid.as_str(), "Card.Self" | "Creature.Self") || !st.exhausted() {
            return self.deny(
                "a referenced static other than an unconditional regeneration restriction".into(),
            );
        }
        let (modifier, wanted) = if let Some(wanted) = unless.strip_prefix("controls") {
            ("CantAttackUnlessDefenderControls", wanted)
        } else if let Some(wanted) = unless.strip_prefix("!controls")
            && wanted.starts_with("Creature")
        {
            ("CantAttackIfDefenderControls", wanted)
        } else {
            return self.deny(format!("`UnlessDefender$ {unless}`"));
        };
        let wanted_expr = self.filter_expr(wanted)?;
        let name = self.body.filter_static("DEFENDER_CONTROLS", &wanted_expr);
        Some(vec![Self::animate_expr(
            &format!("Modifier::{modifier}(&{name})"),
            duration,
        )])
    }

    fn animate_continuous_grant(&mut self, mut st: Params, duration: &str) -> Option<Vec<String>> {
        if !matches!(st.take("EffectZone").as_deref(), None | Some("Battlefield")) {
            return self.deny(
                "a referenced static other than an unconditional regeneration restriction".into(),
            );
        }
        if st.peek("AffectedDefined") == Some("Self")
            && (st.has("SetPower") || st.has("SetToughness"))
        {
            st.take("AffectedDefined");
            let set_p = st.take("SetPower");
            let set_t = st.take("SetToughness");
            if !st.exhausted() {
                return self.deny(
                    "a referenced static other than an unconditional regeneration restriction"
                        .into(),
                );
            }
            let modifier =
                self.computed_set_pt_modifier("Filter::This", set_p.as_deref(), set_t.as_deref())?;
            return Some(vec![Self::animate_expr(&modifier, duration)]);
        }
        if let Some(affected) = st.take("Affected")
            && let Some(ab_name) = st.take("AddAbility")
            && !ab_name.contains(',')
            && st.exhausted()
        {
            let filter_expr = self.filter_expr(&affected)?;
            let filter_static = self.body.filter_static("GRANTED_STATIC", &filter_expr);
            let super::statics::GrantedActivated::Modifier(grant_mod) =
                self.build_grant_activated_modifier(ab_name.trim(), "AddAbility$", false)?
            else {
                unreachable!("for_static_this is false");
            };
            return Some(vec![Self::animate_expr(
                &format!(
                    "Modifier::GrantStatic {{ filter: &{filter_static}, modifier: &{grant_mod} }}"
                ),
                duration,
            )]);
        }
        self.deny("a referenced static other than an unconditional regeneration restriction".into())
    }

    /// Validate the complete remembered-recipient consumer and optional cleanup.
    /// Runtime resolution already records actual recipients, including redirection.
    pub(super) fn damage_recipient_regeneration(&self, name: &str) -> Option<()> {
        let (api, mut p) = Params::parse(self.svars.get(name)?)?;
        p.drop_prose();
        if api != "Effect" {
            return self.deny("a damage memory consumed by another effect or recipient set".into());
        }
        let remembers_creatures = match p.take("RememberObjects").as_deref() {
            Some("Remembered.Creature") => true,
            Some("Remembered")
                if p.take("ConditionDefined").as_deref() == Some("Remembered")
                    && p.take("ConditionPresent").as_deref() == Some("Creature")
                    && matches!(p.take("ConditionCompare").as_deref(), None | Some("GE1")) =>
            {
                true
            }
            _ => false,
        };
        if !remembers_creatures {
            return self.deny("a damage memory consumed by another effect or recipient set".into());
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield")
            || !matches!(p.take("IsCurse").as_deref(), None | Some("True"))
        {
            return self
                .deny("a damage-recipient restriction with another lifetime or flag".into());
        }
        let rule = p.take("StaticAbilities")?;
        if self.no_regeneration_filter(rule.trim())? != "Card.IsRemembered" {
            return self.deny("a damage memory used by another static subject".into());
        }
        let cleanup = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        if let Some(name) = cleanup {
            let (api, mut p) = Params::parse(self.svars.get(&name)?)?;
            p.drop_prose();
            if api != "Cleanup"
                || p.take("ClearRemembered").as_deref() != Some("True")
                || !p.exhausted()
            {
                return self
                    .deny("a damage memory followed by something other than its cleanup".into());
            }
        }
        Some(())
    }

    /// The complete static restriction, returning only the filter it actually names.
    fn no_regeneration_filter(&self, name: &str) -> Option<String> {
        if name.is_empty() || name.contains(',') {
            return self.deny("a regeneration restriction naming multiple or no statics".into());
        }
        let (mode, mut p) = Params::parse(self.svars.get(name)?)?;
        p.drop_prose();
        let valid = p.take("ValidCard")?;
        if mode != "CantRegenerate" || !p.exhausted() {
            return self.deny(
                "a referenced static other than an unconditional regeneration restriction".into(),
            );
        }
        Some(valid)
    }

    /// An `Effect` line: a static ability that lasts the turn, read by the
    /// one sentence its static says. Every other static stays refused.
    pub(super) fn static_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let snapshot = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(effects) = self.static_effect_inner(p, target) {
            return Some(effects);
        }
        p.clone_from(&snapshot);
        self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
        self.body.statics.truncate(saved_statics_len);
        self.wave28c_static_effect(p, target)
    }

    fn static_effect_inner(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        if let Some(rep_name) = p.peek("ReplacementEffects") {
            if target.is_none() {
                if let Some((event, _)) = self
                    .svars
                    .get(rep_name.trim())
                    .and_then(|s| Params::parse(s))
                {
                    match event.as_str() {
                        "AddCounter" => return self.add_counter_turn_effect(p),
                        "Draw" => {
                            let saved_params = p.clone();
                            let saved_unclaimed = self.unclaimed.borrow().clone();
                            if let Some(fx) = self.next_draw_wish_effect(p) {
                                return Some(fx);
                            }
                            *p = saved_params;
                            *self.unclaimed.borrow_mut() = saved_unclaimed;
                        }
                        "Moved" => {
                            let saved_params = p.clone();
                            let saved_unclaimed = self.unclaimed.borrow().clone();
                            let saved_statics_len = self.body.statics.len();
                            if let Some(fx) = self.etb_additional_counter_turn_effect(p) {
                                return Some(fx);
                            }
                            *p = saved_params.clone();
                            (*self.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
                            self.body.statics.truncate(saved_statics_len);
                            if let Some(fx) = self.permanents_enter_tapped_effect(p) {
                                return Some(fx);
                            }
                            *p = saved_params;
                            *self.unclaimed.borrow_mut() = saved_unclaimed;
                            self.body.statics.truncate(saved_statics_len);
                        }
                        _ => {}
                    }
                }
                let saved_params = p.clone();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics_len = self.body.statics.len();
                if let Some(fx) = self.redirect_combat_damage_to_source_effect(p) {
                    return Some(fx);
                }
                p.clone_from(&saved_params);
                (*self.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
                self.body.statics.truncate(saved_statics_len);
                if let Some(fx) = self.prevent_combat_damage_from_effect(p) {
                    return Some(fx);
                }
                *p = saved_params;
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                if let Some(fx) =
                    self.prevention_effect_shield(p.clone(), &mut super::Chain::default())
                {
                    while let Some(key) = p.first_key().map(str::to_string) {
                        p.take(&key);
                    }
                    return Some(fx);
                }
                return None;
            }
            let saved_params = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            if let Some(fx) = self.prevent_target_source_damage_effect(p, target) {
                return Some(fx);
            }
            *p = saved_params;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return self.replace_target_white_mana(p, target);
        }
        if p.peek("Triggers").is_some() {
            return self.delayed_destroy_target_effect(p, target);
        }
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        if statics.contains(',') {
            return self.combat_damage_toughness_bundle_effect(p, target, &statics);
        }
        let body = self.svars.get(&statics)?.clone();
        let (mode, _) = Params::parse(&body)?;
        match mode.as_str() {
            "Continuous" => self
                .extra_land_drops_effect(p)
                .or_else(|| self.continuous_turn_effect(p, target)),
            "MustAttack" => self.must_attack_effect(p, target),
            "CanAttackDefender" => self.can_attack_defender_effect(p, target),
            "CombatDamageToughness" => self.combat_damage_toughness_effect(p, target),
            "CastWithFlash" => self.cast_with_flash_effect(p, target),
            "CantLoseLife" => self.cant_lose_life_effect(p, target),
            "CantPreventDamage" => self.cant_prevent_combat_damage_effect(p, target),
            "CantAttack" | "CantBlock" | "CantAttack,CantBlock" => {
                self.cant_attack_block_turn_effect(p, target)
            }
            "CantChangeLife" => self.life_total_lock(p),
            "CantBlockBy" => self.unblockable_effect(p, target),
            "CantRegenerate" => self.cant_regenerate_effect(p, target),
            "AssignNoCombatDamage" => self.assign_no_combat_damage_effect(p),
            _ => None,
        }
    }

    fn can_attack_defender_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let dur = p.peek("Duration").map(str::to_string);
        if !matches!(
            dur.as_deref(),
            None | Some("UntilEndOfTurn" | "UntilHostLeavesPlayOrEOT")
        ) {
            return None;
        }
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CanAttackDefender" {
            return None;
        }
        let valid = st.take("ValidCard")?;
        if !st.exhausted() {
            return None;
        }
        if valid == "Card.EffectSource" && !self.on_a_spell && target.is_none() {
            p.take("StaticAbilities");
            p.take("Duration");
            if p.peek("ExileOnMoved") == Some("Battlefield") {
                p.take("ExileOnMoved");
            }
            if p.peek("ForgetOnMoved") == Some("Battlefield") {
                p.take("ForgetOnMoved");
            }
            return Some(vec![
                "Effect::ModifySource { modifier: Modifier::AttacksDespiteDefender, \
                 duration: Duration::UntilEndOfTurn }"
                    .to_string(),
            ]);
        }
        if valid == "Card.IsRemembered"
            && dur.as_deref() != Some("UntilHostLeavesPlayOrEOT")
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("RememberObjects") == Some("Targeted")
        {
            p.take("StaticAbilities");
            p.take("Duration");
            p.take("RememberObjects");
            if let Some(exile) = p.peek("ExileOnMoved")
                && exile == "Battlefield"
            {
                p.take("ExileOnMoved");
            }
            return Some(vec![
                "Effect::ModifyTargets { modifier: Modifier::AttacksDespiteDefender, \
                 duration: Duration::UntilEndOfTurn }"
                    .to_string(),
            ]);
        }
        if target.is_some()
            || p.has("RememberObjects")
            || dur.as_deref() == Some("UntilHostLeavesPlayOrEOT")
            || valid.contains("Remembered")
            || valid.contains("EffectSource")
        {
            return None;
        }
        let expr = self.filter_expr(&valid)?;
        p.take("StaticAbilities");
        p.take("Duration");
        let filter = self.body.filter_static("EFFECT", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, Modifier::AttacksDespiteDefender, \
             Duration::UntilEndOfTurn)"
        )])
    }

    fn combat_damage_toughness_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        self.combat_damage_toughness_bundle_effect(p, target, &statics)
    }

    /// Reads `CombatDamageToughness` (optionally bundled with
    /// `CanAttackDefender`, e.g. `Bill the Pony`, `Baldin`-style, or defender
    /// animation abilities).
    fn combat_damage_toughness_bundle_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        statics_raw: &str,
    ) -> Option<Vec<String>> {
        let parsed_statics = self.parse_toughness_bundle_statics(statics_raw)?;
        let duration = match p.peek("Duration") {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let mut out = Vec::with_capacity(parsed_statics.len());
        let mut used_remembered = false;
        for (mode, valid) in parsed_statics {
            if mode == "CombatDamageToughness"
                && let Some(base) = valid.strip_suffix("+powerLTtoughness")
                && target.is_none()
                && !p.has("RememberObjects")
                && !base.contains("Remembered")
                && !base.contains("EffectSource")
            {
                let expr = self.filter_expr(base)?;
                let filter = self.body.filter_static("EFFECT", &expr);
                out.push(format!(
                    "Effect::continuous(&{filter}, \
                     Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater, {duration})"
                ));
                continue;
            }
            let modifier = if mode == "CombatDamageToughness" {
                "Modifier::AssignCombatDamageUsingToughness"
            } else {
                "Modifier::AttacksDespiteDefender"
            };
            if valid == "Card.EffectSource" && !self.on_a_spell && target.is_none() {
                out.push(format!(
                    "Effect::ModifySource {{ modifier: {modifier}, duration: {duration} }}"
                ));
            } else if valid == "Card.IsRemembered"
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                && matches!(
                    p.peek("RememberObjects"),
                    Some("Targeted" | "ParentTarget" | "ThisTargetedCard")
                )
            {
                used_remembered = true;
                out.push(format!(
                    "Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"
                ));
            } else if target.is_none()
                && !p.has("RememberObjects")
                && !valid.contains("Remembered")
                && !valid.contains("EffectSource")
            {
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("EFFECT", &expr);
                out.push(format!(
                    "Effect::continuous(&{filter}, {modifier}, {duration})"
                ));
            } else {
                return None;
            }
        }
        p.take("StaticAbilities");
        p.take("Duration");
        if used_remembered {
            p.take("RememberObjects");
        }
        if matches!(p.peek("ExileOnMoved"), Some("Battlefield")) {
            p.take("ExileOnMoved");
        }
        if matches!(p.peek("ForgetOnMoved"), Some("Battlefield")) {
            p.take("ForgetOnMoved");
        }
        Some(out)
    }

    fn parse_toughness_bundle_statics(&self, statics_raw: &str) -> Option<Vec<(String, String)>> {
        let names: Vec<&str> = statics_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if names.is_empty() {
            return None;
        }
        let mut saw_toughness = false;
        let mut parsed_statics = Vec::with_capacity(names.len());
        for name in names {
            let body = self.svars.get(name)?;
            let (mode, mut st) = Params::parse(body)?;
            st.drop_prose();
            if !matches!(
                st.take("EffectZone").as_deref(),
                None | Some("Battlefield" | "Command")
            ) {
                return None;
            }
            if !matches!(st.take("Secondary").as_deref(), None | Some("True")) {
                return None;
            }
            if mode == "CombatDamageToughness" {
                saw_toughness = true;
            } else if mode != "CanAttackDefender" {
                return None;
            }
            let valid = st.take("ValidCard")?;
            if !st.exhausted() {
                return None;
            }
            parsed_statics.push((mode, valid));
        }
        saw_toughness.then_some(parsed_statics)
    }

    fn cast_with_flash_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() || p.has("RememberObjects") {
            return None;
        }
        let duration = match p.peek("Duration") {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CastWithFlash" || st.take("ValidSA").as_deref() != Some("Spell") {
            return None;
        }
        let who = match st.take("Caster").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            _ => return None,
        };
        if let Some(zone) = st.take("EffectZone")
            && !matches!(zone.as_str(), "Battlefield" | "Command")
        {
            return None;
        }
        if let Some(az) = st.take("AffectedZone") {
            let mut zones: Vec<&str> = az.split(',').map(str::trim).collect();
            zones.sort_unstable();
            zones.dedup();
            if zones != ["Command", "Exile", "Graveyard", "Hand", "Library"] {
                return None;
            }
        }
        let valid_card = st.take("ValidCard")?;
        if !st.exhausted() {
            return None;
        }
        let modifier = if who == "PlayerRel::You" {
            match valid_card.trim() {
                "Card" => "Modifier::SpellsHaveFlash(None)".to_string(),
                "Sorcery" => "Modifier::SorceriesHaveFlash".to_string(),
                other => {
                    let bare_types: Option<Vec<&'static str>> = other
                        .split(',')
                        .map(|alt| super::card_type_const(alt.trim()))
                        .collect();
                    if let Some(mut types) = bare_types
                        && !types.is_empty()
                    {
                        types.dedup();
                        let joined =
                            types
                                .iter()
                                .skip(1)
                                .fold(types[0].to_string(), |mut acc, t| {
                                    acc.push_str(".union(");
                                    acc.push_str(t);
                                    acc.push(')');
                                    acc
                                });
                        format!("Modifier::SpellsHaveFlash(Some({joined}))")
                    } else {
                        let expr = self.filter_expr(other)?;
                        let filter = self.body.filter_static("FLASH", &expr);
                        format!("Modifier::GrantFlash {{ who: {who}, filter: &{filter} }}")
                    }
                }
            }
        } else {
            let expr = self.filter_expr(valid_card.trim())?;
            let filter = self.body.filter_static("FLASH", &expr);
            format!("Modifier::GrantFlash {{ who: {who}, filter: &{filter} }}")
        };
        p.take("StaticAbilities");
        p.take("Duration");
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, {modifier}, {duration})"
        )])
    }

    fn cant_lose_life_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() || p.has("RememberObjects") {
            return None;
        }
        let duration = match p.peek("Duration") {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantLoseLife" {
            return None;
        }
        let who = match st.take("ValidPlayer").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            _ => return None,
        };
        if !st.exhausted() {
            return None;
        }
        p.take("StaticAbilities");
        p.take("Duration");
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, Modifier::CantLoseLife {{ who: {who} }}, {duration})"
        )])
    }

    fn cant_prevent_combat_damage_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("RememberObjects")
            || !matches!(p.peek("Duration"), None | Some("UntilEndOfTurn"))
        {
            return None;
        }
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantPreventDamage" {
            return None;
        }
        let combat_only = match (
            st.take("IsCombat").as_deref(),
            st.take("CombatDamage").as_deref(),
        ) {
            (Some("True"), None) | (None, Some("True")) => true,
            (None, None) => false,
            _ => return None,
        };
        let valid_source = st.take("ValidSource");
        if !st.exhausted() {
            return None;
        }
        let filter = match valid_source {
            None => "Filter::Any".to_string(),
            Some(_) if !combat_only => return None,
            Some(valid) => {
                let expr = self.filter_expr(&valid)?;
                self.body.filter_static("EFFECT", &expr)
            }
        };
        let mod_str = if combat_only {
            "Modifier::CombatDamageCantBePrevented"
        } else {
            "Modifier::DamageCantBePrevented"
        };
        p.take("StaticAbilities");
        p.take("Duration");
        Some(vec![format!(
            "Effect::continuous(&{filter}, {mod_str}, Duration::UntilEndOfTurn)"
        )])
    }

    fn cant_attack_block_turn_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("RememberObjects")
            || !matches!(p.peek("Duration"), None | Some("UntilEndOfTurn"))
        {
            return None;
        }
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        let kw = match mode.as_str() {
            "CantAttack" => "KeywordSet::CANT_ATTACK",
            "CantBlock" => "KeywordSet::CANT_BLOCK",
            "CantAttack,CantBlock" => "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)",
            _ => return None,
        };
        if !matches!(
            st.take("AffectedZone").as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        let valid = st.take("ValidCard")?;
        if !st.exhausted() {
            return None;
        }
        if valid == "Card.EffectSource" && !self.on_a_spell {
            p.take("StaticAbilities");
            p.take("Duration");
            p.take("IsCurse");
            return Some(vec![format!(
                "Effect::ModifySource {{ modifier: Modifier::AddKeyword({kw}), \
                 duration: Duration::UntilEndOfTurn }}"
            )]);
        }
        if valid.contains("Remembered") || valid.contains("EffectSource") {
            return None;
        }
        let expr = self.filter_expr(&valid)?;
        p.take("StaticAbilities");
        p.take("Duration");
        p.take("IsCurse");
        let filter = self.body.filter_static("EFFECT", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, Modifier::AddKeyword({kw}), Duration::UntilEndOfTurn)"
        )])
    }

    fn add_counter_turn_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !matches!(p.peek("Duration"), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let name = p.peek("ReplacementEffects")?.trim().to_string();
        let (event, mut r) = Params::parse(self.svars.get(&name)?)?;
        r.drop_prose();
        if event != "AddCounter"
            || r.take("ActiveZones").as_deref() != Some("Battlefield")
            || r.has("ValidCounterType")
            || r.has("EffectOnly")
        {
            return None;
        }
        let valid_card = r.take("ValidCard")?;
        let replace_with = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let (api, mut body) = self
            .svars
            .get(&replace_with)
            .and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if api != "ReplaceCounter"
            || body.has("ValidCounterType")
            || !matches!(body.take("ChooseCounter").as_deref(), None | Some("True"))
        {
            return None;
        }
        let amt_var = body.take("Amount")?;
        if !body.exhausted() {
            return None;
        }
        let n: u16 = self
            .svars
            .get(amt_var.trim())?
            .trim()
            .strip_prefix("ReplaceCount$CounterNum/Plus.")?
            .trim()
            .parse()
            .ok()?;
        if n == 0 {
            return None;
        }
        let mut cleaned = Vec::new();
        for alt in valid_card.split(',') {
            let stripped = alt
                .trim()
                .strip_suffix("+inZoneBattlefield")
                .or_else(|| alt.trim().strip_suffix(".inZoneBattlefield"))?;
            if stripped.is_empty() {
                return None;
            }
            cleaned.push(stripped);
        }
        let expr = self.filter_expr(&cleaned.join(","))?;
        p.take("ReplacementEffects");
        p.take("Duration");
        let filter = self.body.filter_static("EFFECT", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, Modifier::AdditionalCounterPlacement {{ n: {n} }}, \
             Duration::UntilEndOfTurn)"
        )])
    }

    /// "Creatures target player controls attack this turn if able": bind the
    /// chosen player's current creatures, not every creature or the controller's.
    fn must_attack_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        if target != Some("TargetSpec::Player(PlayerRel::Chosen)")
            || p.take("RememberObjects").as_deref() != Some("Targeted")
        {
            return None;
        }
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "MustAttack"
            || st.take("ValidCreature").as_deref() != Some("Creature.RememberedPlayerCtrl")
            || !st.exhausted()
            || p.take("Duration").is_some()
        {
            return None;
        }
        Some(vec![
            "Effect::ModifyFilter { filter: &Filter::CREATURE, controlled_by: Some(PlayerRel::Chosen), modifier: Modifier::AttacksEachCombat, duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// "You may play up to N additional lands this turn": a player-scoped
    /// continuous effect, with the set of lands deliberately left open.
    fn extra_land_drops_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "Continuous" || st.take("Affected").as_deref() != Some("You") {
            return None;
        }
        let amount = st.take("AdjustLandPlays")?.parse::<u8>().ok()?;
        if !st.exhausted()
            || !matches!(p.peek("AILogic"), None | Some("Always"))
            || p.peek("Duration").is_some()
            || p.peek("Name")
                .is_some_and(|n| n.trim_start().starts_with("Emblem"))
        {
            return None;
        }
        p.take("StaticAbilities");
        p.take("AILogic");
        p.take("Name");
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, Modifier::ExtraLandDrops({amount}), Duration::UntilEndOfTurn)"
        )])
    }

    /// A turn-long `Effect` whose single `Continuous` static grants a keyword
    /// or combat restriction (`AddHiddenKeyword$ CARDNAME can't block.`,
    /// `AddKeyword$ ...`) to every battlefield permanent matching `Affected$`
    /// (e.g. `Order // Chaos` — "Creatures can't block this turn").
    fn continuous_turn_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() || p.has("RememberObjects") {
            return None;
        }
        if !matches!(p.peek("Duration"), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let statics = p.take("StaticAbilities")?;
        p.take("Duration");
        p.take("IsCurse");
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "Continuous" {
            return None;
        }
        if !matches!(
            st.take("AffectedZone").as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        let affected = st.take("Affected")?;
        if affected.contains("Remembered") || affected.contains("EffectSource") {
            return None;
        }
        let kw = if let Some(hidden) = st.take("AddHiddenKeyword") {
            match hidden.trim() {
                "CARDNAME can't block." => "KeywordSet::CANT_BLOCK".to_string(),
                "CARDNAME can't attack." => "KeywordSet::CANT_ATTACK".to_string(),
                "CARDNAME can't attack or block." => {
                    "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)".to_string()
                }
                _ => return None,
            }
        } else {
            let raw = st.take("AddKeyword")?;
            let each: Option<Vec<&str>> = raw
                .split('&')
                .map(|k| super::keyword_const(k.trim()))
                .collect();
            let bits = each?;
            if bits.is_empty() {
                return None;
            }
            bits.join(".union(") + &")".repeat(bits.len().saturating_sub(1))
        };
        if !st.exhausted() {
            return None;
        }
        let expr = self.filter_expr(&affected)?;
        let filter = self.body.filter_static("EFFECT", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, Modifier::AddKeyword({kw}), Duration::UntilEndOfTurn)"
        )])
    }

    /// "Assigns no combat damage this turn" (Laccolith Whelp, Laccolith Grunt,
    /// Laccolith Warrior, Laccolith Titan): an `Effect` whose one static is
    /// `Mode$ AssignNoCombatDamage | ValidCard$ Card.EffectSource`.
    fn assign_no_combat_damage_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if self.on_a_spell {
            return None;
        }
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "AssignNoCombatDamage"
            || st.take("ValidCard").as_deref() != Some("Card.EffectSource")
            || !st.exhausted()
        {
            return None;
        }
        if !matches!(
            p.take("Duration").as_deref(),
            None | Some("UntilHostLeavesPlayOrEOT" | "UntilEndOfCombat")
        ) {
            return None;
        }
        Some(vec![
            "Effect::ModifySource { modifier: Modifier::AssignsNoCombatDamage, \
             duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// "The next time you would draw a card this turn, instead put a card you own
    /// from outside the game into your hand" (`Ring of Ma'rûf` -> `Modifier::NextDrawWish`).
    fn next_draw_wish_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let name = p.take("ReplacementEffects")?;
        if !p.exhausted() {
            return None;
        }
        let (event, mut r) = Params::parse(self.svars.get(name.trim())?)?;
        r.drop_prose();
        if event != "Draw"
            || r.take("ValidPlayer").as_deref() != Some("You")
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
        {
            return None;
        }
        let rw = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let (api, mut body) = Params::parse(self.svars.get(rw.trim())?)?;
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Origin").as_deref() != Some("Sideboard")
            || body.take("Destination").as_deref() != Some("Hand")
            || body.take("ChangeType").as_deref() != Some("Card.YouOwn")
            || body.take("ChangeNum").as_deref() != Some("1")
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(body.take("Mandatory").as_deref(), None | Some("True"))
        {
            return None;
        }
        let sub = body.take("SubAbility")?;
        if !body.exhausted() || !self.exiles_itself(sub.trim()) {
            return None;
        }
        Some(vec![
            "Effect::continuous(&Filter::Any, Modifier::NextDrawWish, Duration::UntilEndOfTurn)"
                .to_string(),
        ])
    }

    /// "This turn, each [matching] creature you control enters with N additional
    /// <kind> counter(s) on it" (`Combine Guildmage`, `Zameck Guildmage` ->
    /// `Modifier::AdditionalCounterOnEntryByCount`).
    fn etb_additional_counter_turn_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let name = p.take("ReplacementEffects")?;
        if !p.exhausted() {
            return None;
        }
        let (event, mut r) = Params::parse(self.svars.get(name.trim())?)?;
        r.drop_prose();
        if event != "Moved"
            || r.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
            || r.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let valid_card = r.take("ValidCard")?;
        let rw = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let (api, mut body) = Params::parse(self.svars.get(rw.trim())?)?;
        body.drop_prose();
        if api != "PutCounter"
            || body.take("ETB").as_deref() != Some("True")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
        {
            return None;
        }
        let kind_raw = body.take("CounterType")?;
        let kind = super::counter_kind(kind_raw.trim())?;
        let n: u16 = body
            .take("CounterNum")?
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&v| v > 0)?;
        if !body.exhausted() {
            return None;
        }
        let expr = self.filter_expr(&valid_card)?;
        let filter_static = self.body.filter_static("EFFECT", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter_static}, \
             Modifier::AdditionalCounterOnEntryByCount {{ kind: {kind}, count: PtCount::Fixed({n}) }}, \
             Duration::UntilEndOfTurn)"
        )])
    }

    /// "All combat damage that would be dealt to you by [sources] this turn is
    /// dealt to [this creature] instead" (`Kjeldoran Royal Guard` ->
    /// `Effect::RedirectCombatDamageToYouThisTurn`).
    fn redirect_combat_damage_to_source_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if self.on_a_spell
            || !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
        {
            return None;
        }
        let name = p.take("ReplacementEffects")?;
        if !p.exhausted() {
            return None;
        }
        let (event, mut r) = Params::parse(self.svars.get(name.trim())?)?;
        r.drop_prose();
        if event != "DamageDone"
            || r.take("IsCombat").as_deref() != Some("True")
            || r.take("ValidTarget").as_deref() != Some("You")
            || r.take("DamageTarget").as_deref() != Some("EffectSource")
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
        {
            return None;
        }
        let valid_source = r.take("ValidSource");
        let rw = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let (api, mut rep) = Params::parse(self.svars.get(rw.trim())?)?;
        rep.drop_prose();
        if api != "ReplaceEffect"
            || rep.take("VarName").as_deref() != Some("Affected")
            || rep.take("VarValue").as_deref() != Some("EffectSource")
            || rep.take("VarType").as_deref() != Some("Card")
            || !rep.exhausted()
        {
            return None;
        }
        let src_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let src_ref = self.body.filter_static("SOURCE", &src_expr);
        Some(vec![format!(
            "Effect::RedirectCombatDamageToYouThisTurn {{ by: &{src_ref}, to: DamageRedirectTo::ThisObject }}"
        )])
    }

    /// "Until your next turn / until end of turn, prevent all [combat] damage
    /// target permanent would deal" (`Gideon of the Trials` `+1` ->
    /// `Effect::continuous(&Filter::This, Modifier::PreventAllDamageFromIt, ...)`).
    fn prevent_target_source_damage_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        if p.take("RememberObjects").as_deref() != Some("Targeted") {
            return None;
        }
        if let Some(forget) = p.peek("ForgetOnMoved")
            && forget == "Battlefield"
        {
            p.take("ForgetOnMoved");
        }
        if let Some(exile) = p.peek("ExileOnMoved")
            && exile == "Battlefield"
        {
            p.take("ExileOnMoved");
        }
        let name = p.take("ReplacementEffects")?;
        if !p.exhausted() {
            return None;
        }
        let (event, mut r) = Params::parse(self.svars.get(name.trim())?)?;
        r.drop_prose();
        if event != "DamageDone"
            || r.take("Prevent").as_deref() != Some("True")
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
            || r.take("ValidSource").as_deref() != Some("Card.IsRemembered")
        {
            return None;
        }
        let modifier = match r.take("IsCombat").as_deref() {
            None => "Modifier::PreventAllDamageFromIt",
            Some("True") => "Modifier::PreventDamageFromIt",
            Some(_) => return None,
        };
        if !r.exhausted() {
            return None;
        }
        Some(vec![format!(
            "Effect::continuous(&Filter::This, {modifier}, {duration})"
        )])
    }

    fn prevent_combat_damage_from_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let name = p.take("ReplacementEffects")?;
        if !p.exhausted() {
            return None;
        }
        let (event, mut r) = Params::parse(self.svars.get(name.trim())?)?;
        r.drop_prose();
        if event != "DamageDone"
            || r.take("Prevent").as_deref() != Some("True")
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
        {
            return None;
        }
        let is_combat = match r.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let valid_source = r.take("ValidSource");
        let valid_target = r.take("ValidTarget");
        if !r.exhausted() {
            return None;
        }
        if let Some(tgt) = valid_target.as_deref()
            && tgt.contains(',')
        {
            let parts: Vec<&str> = tgt
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            let has_you = parts.contains(&"You");
            let perm_parts: Vec<&str> = parts.into_iter().filter(|&s| s != "You").collect();
            if has_you
                && !perm_parts.is_empty()
                && !perm_parts
                    .iter()
                    .any(|s| matches!(*s, "Player" | "Opponent" | "Player.Opponent"))
            {
                let src_expr = match valid_source.as_deref() {
                    None => "Filter::Any".to_string(),
                    Some(vs) => self.prevention_source_filter(vs)?,
                };
                let src_ref = self.body.filter_static("SOURCE", &src_expr);
                let perm_filter = self.filter_expr(&perm_parts.join(","))?;
                let perm_static = self.body.filter_static("EFFECT", &perm_filter);
                let perm_mod = match (is_combat, valid_source.is_some()) {
                    (true, false) => "Modifier::PreventDamageToIt".to_string(),
                    (true, true) => format!("Modifier::PreventCombatDamageFrom(&{src_ref})"),
                    (false, _) => format!("Modifier::PreventDamageFrom(&{src_ref})"),
                };
                return Some(vec![
                    format!(
                        "Effect::PreventAllDamageFromMatchingThisTurn {{ \
                         target: Some(TargetSpec::Player(PlayerRel::You)), \
                         sources: &{src_ref}, combat_only: {is_combat} }}"
                    ),
                    format!(
                        "Effect::continuous(&{perm_static}, {perm_mod}, Duration::UntilEndOfTurn)"
                    ),
                ]);
            }
        }
        let (valid, modifier) = match (valid_source, valid_target, is_combat) {
            (Some(src), None, true) => (src, "Modifier::PreventDamageFromIt"),
            (Some(src), None, false) => (src, "Modifier::PreventAllDamageFromIt"),
            (None, Some(tgt), true) => (tgt, "Modifier::PreventDamageToIt"),
            (None, Some(tgt), false) => (tgt, "Modifier::PreventDamageFrom(&Filter::Any)"),
            _ => return None,
        };
        let filter = self.filter_expr(&valid)?;
        let filter_static = self.body.filter_static("EFFECT", &filter);
        Some(vec![format!(
            "Effect::continuous(&{filter_static}, {modifier}, Duration::UntilEndOfTurn)"
        )])
    }

    /// "Destroy target blocking creature at end of combat" (Silent Assassin):
    /// an `Effect` remembering `Targeted` with a `Triggers$` delayed trigger
    /// at `Phase$ EndCombat` that destroys `Remembered`.
    fn delayed_destroy_target_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
        let trig_name = p.take("Triggers")?;
        if p.take("RememberObjects").as_deref() != Some("Targeted") {
            return None;
        }
        p.take("IsCurse");
        p.take("Name");
        let trig_body = self.svars.get(trig_name.trim())?.clone();
        let (mode, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if mode != "Phase" || tp.take("Phase").as_deref() != Some("EndCombat") {
            return None;
        }
        let exec_name = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (exec_api, mut ep) = Params::parse(&exec_body)?;
        ep.drop_prose();
        if exec_api != "Destroy"
            || ep.take("Defined").as_deref() != Some("Remembered")
            || !ep.exhausted()
        {
            return None;
        }
        Some(vec![format!(
            "Effect::AtEndOfCombat {{ about: {target}, \
             effects: &[Effect::destroy(TargetSpec::EventObject)] }}"
        )])
    }

    /// "It can't be regenerated this turn" (Disintegrate, Carbonize): an
    /// `Effect` whose one static says no regeneration applies to what it
    /// remembers, which is the line's target, and which ends when that
    /// leaves the battlefield — `Effect::CantBeRegeneratedThisTurn`, kept
    /// for that object only (CR 400.7, 701.19c).
    ///
    /// "If it's a creature" (`ConditionDefined$ ParentTarget |
    /// ConditionPresent$ Creature`) is asked of the target as the line
    /// resolves. Actual damage-recipient memory is the separate complete
    /// chain read by [`Self::damage_recipient_regeneration`].
    pub(super) fn cant_regenerate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        if self.no_regeneration_filter(statics.trim())? != "Card.IsRemembered" {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let target = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        })?;
        if !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget")
        ) {
            return None;
        }
        let effect = format!("Effect::CantBeRegeneratedThisTurn {{ target: {target} }}");
        let creature = match (p.peek("ConditionDefined"), p.peek("ConditionPresent")) {
            (None, None) => false,
            (Some("ParentTarget" | "Targeted"), Some("Creature")) => {
                p.take("ConditionDefined");
                p.take("ConditionPresent");
                true
            }
            _ => return None,
        };
        Some(vec![if creature {
            format!("Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{effect}] }}")
        } else {
            effect
        }])
    }

    /// `ReplaceDyingDefined$` — "if that creature would die this turn,
    /// exile it instead" (Magma Spray), a replacement on the line's own
    /// target for the rest of the turn, or on actual damage recipients when
    /// validated alongside `RememberDamaged$ True` and its cleanup.
    pub(super) fn exile_if_dies(&mut self, defined: &str, target: Option<&str>) -> Option<String> {
        match defined {
            "__DamagedThisWay" => return Some("Effect::ExileDamagedThisWayIfDies".to_string()),
            "__DamagedThisWayCreature" => {
                let filter = self.body.filter_static(
                    "DAMAGED",
                    "Filter::And(&[Filter::CREATURE, Filter::DealtDamageBySourceThisTurn])",
                );
                return Some(format!(
                    "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &{filter}, \
                     only_found: false, then: &[Effect::ExileIfDiesThisTurn {{ target: TargetSpec::EventObject }}] }}"
                ));
            }
            _ => {}
        }
        let Some(target) = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        }) else {
            return self.deny(format!(
                "`ReplaceDyingDefined$ {defined}` with no object target"
            ));
        };
        let exile = format!("Effect::ExileIfDiesThisTurn {{ target: {target} }}");
        match defined {
            "Targeted" | "ThisTargetedCard" => Some(exile),
            "ThisTargetedCard.Creature" => Some(format!(
                "Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{exile}] }}"
            )),
            other => self.deny(format!("`ReplaceDyingDefined$ {other}`")),
        }
    }

    /// `AtEOT$` on a `Token`, `Pump`, or `Animate` line — delayed step
    /// cleanup (CR 603.7: Stone Giant, Chimeric Coils, Attack-in-the-Box,
    /// Ayara, Furnace Queen, Kami of Industry, token end-step/upkeep clauses).
    pub(super) fn at_next_end_step(
        &mut self,
        api: &str,
        what: &str,
        target: Option<&str>,
    ) -> Option<String> {
        if api == "Token" {
            return self.token_at_next_end_step(what, target);
        }
        let aimed = target.is_some_and(|t| {
            !matches!(
                t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        });
        let aimed_you_ctrl =
            aimed && target.is_some_and(|t| self.target_spec_requires_your_control(t));
        let self_subject = target.is_none()
            && !self.on_a_spell
            && !self
                .type_line
                .split_whitespace()
                .any(|w| matches!(w, "Aura" | "Equipment"))
            && self.non_targeted_at_eot_targets_self(api, what);
        match (api, what) {
            ("Pump" | "Animate", "Destroy") if aimed => Some(
                "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            ("Pump", "Sacrifice") if self_subject || aimed_you_ctrl => {
                Some("Effect::AtNextEndStep { effects: &[Effect::SacrificeEvent] }".to_string())
            }
            ("Animate", "Sacrifice") if aimed || self_subject => {
                Some("Effect::AtNextEndStep { effects: &[Effect::SacrificeEvent] }".to_string())
            }
            ("Pump" | "Animate", "Exile") if aimed || self_subject => Some(
                "Effect::AtNextEndStep { effects: &[Effect::exile(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            ("Pump" | "Animate", "YourExile") if aimed || self_subject => {
                let remember = if aimed {
                    "TriggerRemember::FirstTarget"
                } else {
                    "TriggerRemember::Source"
                };
                Some(format!(
                    "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ step: DelayedStep::EndStep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }}, condition: None, targets: None, remember: {remember}, effects: &[Effect::exile(TargetSpec::EventObject)] }}"
                ))
            }
            ("Pump" | "Animate", "YourSacrifice") if self_subject || aimed_you_ctrl => {
                let remember = if aimed_you_ctrl {
                    "TriggerRemember::FirstTarget"
                } else {
                    "TriggerRemember::Source"
                };
                Some(format!(
                    "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ step: DelayedStep::EndStep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }}, condition: None, targets: None, remember: {remember}, effects: &[Effect::SacrificeEvent] }}"
                ))
            }
            _ => self.deny(format!("`AtEOT$ {what}` on `{api}`")),
        }
    }

    fn token_at_next_end_step(&mut self, what: &str, target: Option<&str>) -> Option<String> {
        let targets_player = matches!(
            target,
            Some(
                "TargetSpec::Player(PlayerRel::Chosen)"
                    | "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
            )
        );
        match what {
            "Exile" => Some(
                "Effect::AtNextEndStepForCreatedTokens { effects: &[Effect::exile(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            "Sacrifice" if !targets_player => Some(
                "Effect::AtNextEndStepForCreatedTokens { effects: &[Effect::SacrificeEvent] }"
                    .to_string(),
            ),
            "SacrificeCombat" if !targets_player => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::CombatEnd, whose: None, this_turn_only: true, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::SacrificeEvent] }"
                    .to_string(),
            ),
            "ExileCombat" => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::CombatEnd, whose: None, this_turn_only: true, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::exile(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            "YourExile" => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::EndStep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::exile(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            "YourSacrifice" if !targets_player => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::EndStep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::SacrificeEvent] }"
                    .to_string(),
            ),
            "YourExileUpkeep" => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::Upkeep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::exile(TargetSpec::EventObject)] }"
                    .to_string(),
            ),
            "YourSacrificeUpkeep" if !targets_player => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::Upkeep, whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::SacrificeEvent] }"
                    .to_string(),
            ),
            "SacrificeUpkeep" if !targets_player => Some(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::Upkeep, whose: None, this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EachCreatedToken, effects: &[Effect::SacrificeEvent] }"
                    .to_string(),
            ),
            _ => self.deny(format!("`AtEOT$ {what}` on `Token`")),
        }
    }

    fn non_targeted_at_eot_targets_self(&self, api: &str, what: &str) -> bool {
        let mut saw_svar = false;
        for body in self.svars.values() {
            let Some((parsed_api, parsed)) = Params::parse(body) else {
                continue;
            };
            if parsed_api == api && parsed.peek("AtEOT") == Some(what) && !parsed.has("ValidTgts") {
                saw_svar = true;
                if !matches!(parsed.peek("Defined"), None | Some("Self")) {
                    return false;
                }
            }
        }
        self.trigger_mode.is_none() || saw_svar
    }

    fn target_spec_requires_your_control(&self, target: &str) -> bool {
        let Some(inner) = target
            .strip_prefix("TargetSpec::Object(&")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return false;
        };
        if matches!(
            inner,
            "Filter::YOUR_CREATURE"
                | "Filter::YOUR_OTHER_CREATURE"
                | "Filter::YOUR_ARTIFACT"
                | "Filter::YOUR_PERMANENT"
                | "Filter::YOUR_NONLAND_PERMANENT"
                | "Filter::YOUR_MODIFIED_CREATURE"
                | "Filter::YOUR_LAND"
                | "Filter::YOUR_ENCHANTMENT"
                | "Filter::YOUR_PLANESWALKER"
                | "Filter::ControlledByYou"
        ) {
            return true;
        }
        if inner.contains("::") {
            return false;
        }
        let prefix = format!("static {inner}: Filter = ");
        let Some(rhs) = self
            .body
            .statics
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix))
            .and_then(|rest| rest.strip_suffix(';'))
        else {
            return false;
        };
        let Some(slice) = rhs
            .strip_prefix("Filter::And(&[")
            .and_then(|s| s.strip_suffix("])"))
        else {
            return false;
        };
        let mut depth = 0usize;
        let mut start = 0usize;
        for (idx, ch) in slice.char_indices() {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    if slice[start..idx].trim() == "Filter::ControlledByYou" {
                        return true;
                    }
                    start = idx + 1;
                }
                _ => {}
            }
        }
        slice[start..].trim() == "Filter::ControlledByYou"
    }

    /// `DB$ Effect | ReplacementEffects$ <R>` where `<R>` is
    /// `Event$ Moved | ValidCard$ <filter> | Destination$ Battlefield |
    /// ReplacementResult$ Updated | ReplaceWith$ <DB$ Tap | ETB$ True>`
    /// -> `Effect::AddGlobalTempModifier { modifier: Modifier::PermanentsEnterTapped(...) }`
    /// (*Due Respect*).
    fn permanents_enter_tapped_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.has("RememberObjects")
            || p.has("Triggers")
            || p.has("StaticAbilities")
            || p.has("Abilities")
            || p.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        p.drop_prose();
        p.take("Name");
        if let Some(v) = p.take("NonStackingEffect")
            && v != "True"
        {
            return None;
        }
        if let Some(v) = p.take("Stackable")
            && v != "False"
        {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let rep_name = p.take("ReplacementEffects")?;
        if rep_name.contains(',') || !p.exhausted() {
            return None;
        }
        let rep_body = self.svars.get(rep_name.trim())?.clone();
        let (event, mut r) = Params::parse(&rep_body)?;
        r.drop_prose();
        if event != "Moved"
            || !matches!(
                r.take("ActiveZones").as_deref(),
                None | Some("Command" | "Battlefield")
            )
            || !matches!(r.take("Origin").as_deref(), None | Some("Any"))
            || r.take("Destination").as_deref() != Some("Battlefield")
            || r.take("ReplacementResult").as_deref() != Some("Updated")
        {
            return None;
        }
        let valid_card = r.take("ValidCard")?;
        if valid_card.contains("Self") {
            return None;
        }
        let rw = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let rw_body = self.svars.get(rw.trim())?.clone();
        let (tap_api, mut tp) = Params::parse(&rw_body)?;
        tp.drop_prose();
        if tap_api != "Tap"
            || tp.take("ETB").as_deref() != Some("True")
            || !matches!(
                tp.take("Defined").as_deref(),
                None | Some("ReplacedCard" | "Self")
            )
            || !tp.exhausted()
        {
            return None;
        }
        let filter_ref = if valid_card == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&valid_card)?;
            if expr == "Filter::Any" {
                "&Filter::Any".to_string()
            } else {
                format!("&{}", self.body.filter_static("ENTER", &expr))
            }
        };
        Some(vec![format!(
            "Effect::AddGlobalTempModifier {{ filter: &Filter::Any,              modifier: Modifier::PermanentsEnterTapped({filter_ref}), duration: {duration} }}"
        )])
    }
}
