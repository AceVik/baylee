//! Persistent and effect-created damage prevention shields.

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Persistent `R: Event$ DamageDone` prevention rules:
    /// - Spell-source `PreventAllDamageFromIt` (`ValidSource$ Instant,Sorcery`);
    /// - Infinite damage prevention to you (`Modifier::PreventDamageToYouFrom`) or
    ///   matching permanents (`Modifier::PreventDamageFrom` /
    ///   `Modifier::PreventCombatDamageFrom`);
    /// - Finite per-source prevention (`Modifier::PreventDamagePerSource` and
    ///   `Modifier::PreventDamageToAffectedPerSource` for Spheres, Urza's Armor,
    ///   Cover of Winter, Daunting Defender, Guardian Seraph, Reidane, etc.).
    pub(super) fn spell_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let secondary = p.take("Secondary");
        if !matches!(p.take("ActiveZones").as_deref(), None | Some("Battlefield"))
            || !matches!(secondary.as_deref(), None | Some("True"))
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        if matches!(
            p.peek("DamageTarget"),
            Some("Self" | "Enchanted" | "Equipped")
        ) && p.peek("ReplaceWith").is_some()
        {
            return self.redirect_damage_to_you(p);
        }
        if p.peek("ReplaceWith").is_some() {
            if p.peek("AlwaysReplace") == Some("True") && p.peek("ValidTarget") == Some("Card.Self")
            {
                return self.counters_prevent_damage_rule(p);
            }
            if p.has("DamageAmount") && !p.has("PreventionEffect") && !p.has("Prevent") {
                return self.replace_player_damage_rule(p);
            }
            return self.per_source_damage_prevention(p);
        }
        if p.take("Prevent").as_deref() != Some("True") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let combat_only = match p.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let cond = self.prevention_condition(&mut p)?;
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }

        let Some(target_raw) = valid_target else {
            let valid = valid_source?;
            if Self::is_stack_spell_source(&valid) {
                if combat_only || !cond.is_empty() || secondary.is_some() {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                }
                let filter = self.filter_expr(&valid)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::And(&[Filter::InZone(ZoneRef::Stack), {filter}]), \
                     Modifier::PreventAllDamageFromIt)"
                ));
                return Some(());
            }
            if combat_only {
                let filter = self.filter_expr(&valid)?;
                self.body.abilities.push(format!(
                    "static_ability!({filter}, Modifier::PreventDamageFromIt{cond})"
                ));
                return Some(());
            }
            if valid.contains("Self")
                || valid.contains("EnchantedBy")
                || valid.contains("EquippedBy")
            {
                let filter = self.filter_expr(&valid)?;
                self.body.abilities.push(format!(
                    "static_ability!({filter}, Modifier::PreventAllDamageFromIt{cond})"
                ));
                return Some(());
            }
            return self.deny(format!(
                "a prevention source outside instant/sorcery spells `{valid}`"
            ));
        };

        if valid_source
            .as_deref()
            .is_some_and(Self::is_stack_spell_source)
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        self.emit_target_damage_prevention(valid_source.as_deref(), &target_raw, combat_only, &cond)
    }

    fn emit_target_damage_prevention(
        &mut self,
        valid_source: Option<&str>,
        target_raw: &str,
        combat_only: bool,
        cond: &str,
    ) -> Option<()> {
        let source_expr = match valid_source {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };

        let target_parts: Vec<&str> = target_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if target_parts.is_empty() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let has_you = target_parts.contains(&"You");
        let perm_alts: Vec<&str> = target_parts
            .iter()
            .copied()
            .filter(|&part| part != "You")
            .collect();
        if has_you && combat_only {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let target_filter = if perm_alts.is_empty() {
            None
        } else {
            let Some(filter) = self.prevention_permanent_target(&perm_alts.join(",")) else {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            };
            Some(filter)
        };

        let src_ref = self.body.filter_static("SOURCE", &source_expr);
        if has_you {
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::PreventDamageToYouFrom(&{src_ref}){cond})"
            ));
        }
        if let Some(target_filter) = target_filter {
            let modifier = if combat_only {
                format!("Modifier::PreventCombatDamageFrom(&{src_ref})")
            } else {
                format!("Modifier::PreventDamageFrom(&{src_ref})")
            };
            self.body.abilities.push(format!(
                "static_ability!({target_filter}, {modifier}{cond})"
            ));
        }
        Some(())
    }

    fn is_stack_spell_source(valid: &str) -> bool {
        valid.split(',').all(|alternative| {
            matches!(
                alternative.trim().split(['.', '+']).next(),
                Some("Instant" | "Sorcery")
            )
        })
    }

    fn per_source_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        if p.take("PreventionEffect").as_deref() != Some("True") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let combat_only = match p.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let cond = self.prevention_condition(&mut p)?;
        let valid_source = p.take("ValidSource");
        let Some(valid_target) = p.take("ValidTarget") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let amount_expr = self.replace_damage_amount(&replace_name)?;
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let src_ref = self.body.filter_static("SOURCE", &source_expr);

        let alts: Vec<&str> = valid_target.split(',').map(str::trim).collect();
        let has_you = alts.contains(&"You");
        let perm_alts: Vec<&str> = alts.into_iter().filter(|&a| a != "You").collect();

        if has_you {
            let affected_filter = if perm_alts.is_empty() {
                // PreventDamagePerSource checks `applies_to` for Object targets
                // and `include_you` for `you`, so `You`-only shields must match
                // zero permanents (`Filter::Not(&Filter::Any)`).
                "Filter::Not(&Filter::Any)".to_string()
            } else {
                self.prevention_permanent_target(&perm_alts.join(","))?
            };
            self.body.abilities.push(format!(
                "static_ability!({affected_filter}, Modifier::PreventDamagePerSource {{ \
                 sources: &{src_ref}, include_you: true, combat_only: {combat_only}, \
                 amount: {amount_expr} }}{cond})"
            ));
            return Some(());
        }

        let affected_filter = self.prevention_permanent_target(&valid_target)?;
        self.body.abilities.push(format!(
            "static_ability!({affected_filter}, Modifier::PreventDamageToAffectedPerSource {{ \
             sources: &{src_ref}, combat_only: {combat_only}, amount: {amount_expr} }}{cond})"
        ));
        Some(())
    }

    fn replace_damage_amount(&mut self, svar_name: &str) -> Option<String> {
        let body = self.svars.get(svar_name.trim()).cloned()?;
        let (api, mut rep) = Params::parse(&body)?;
        rep.drop_prose();
        if api != "ReplaceDamage" {
            return None;
        }
        let raw = rep.take("Amount")?;
        if !rep.exhausted() {
            return None;
        }
        if let Ok(n) = raw.parse::<u32>() {
            return (n > 0).then(|| format!("Amount::Fixed({n})"));
        }
        let def = if raw.starts_with("Count$") {
            raw
        } else {
            self.svars.get(raw.trim())?.clone()
        };
        self.static_count_expr(&def)
    }

    /// Persistent `R: Event$ DamageDone | ValidTarget$ You | DamageTarget$ Self|Enchanted|Equipped`
    /// with `DB$ ReplaceEffect | VarName$ Affected | VarValue$ Self|Enchanted|Equipped | VarType$ Card`
    /// -> `Modifier::RedirectDamageToYou(&sources)` (`Empyrial Archangel`,
    /// `Veteran Bodyguard`, `Martyrs of Korlis`, `Pariah`, `Pariah's Shield`).
    fn redirect_damage_to_you(&mut self, mut p: Params) -> Option<()> {
        let Some(dmg_target) = p.take("DamageTarget") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let (affected_filter, expected_var) = match dmg_target.as_str() {
            "Self" => ("Filter::This", "Self"),
            "Enchanted" => ("Filter::AttachedToBySource", "Enchanted"),
            "Equipped" => {
                // `Filter::AttachedToBySource` matches nothing when unequipped,
                // so the script's redundant `IsPresent$ Card.equipping | PresentDefined$ Self`
                // guard is already inherent to the filter.
                if p.peek("IsPresent") == Some("Card.equipping")
                    && p.peek("PresentDefined") == Some("Self")
                {
                    p.take("IsPresent");
                    p.take("PresentDefined");
                }
                ("Filter::AttachedToBySource", "Equipped")
            }
            _ => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        if p.take("ValidTarget").as_deref() != Some("You")
            || p.has("IsCombat")
            || p.has("Prevent")
            || p.has("PreventionEffect")
            || p.has("AlwaysReplace")
            || p.has("Optional")
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let cond = self.prevention_condition(&mut p)?;
        let valid_source = p.take("ValidSource");
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let Some((api, mut rep)) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        rep.drop_prose();
        if api != "ReplaceEffect"
            || rep.take("VarName").as_deref() != Some("Affected")
            || rep.take("VarValue").as_deref() != Some(expected_var)
            || rep.take("VarType").as_deref() != Some("Card")
            || !rep.exhausted()
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let src_ref = self.body.filter_static("SOURCE", &source_expr);
        self.body.abilities.push(format!(
            "static_ability!({affected_filter}, Modifier::RedirectDamageToYou(&{src_ref}){cond})"
        ));
        Some(())
    }

    /// Persistent `R: Event$ DamageDone` capping or flooring damage dealt to a
    /// player (`Modifier::ReplacePlayerDamage` with `DamageAmountReplacement::Cap`
    /// or `FloorAtEffectSourcePower`, e.g. `Forethought Amulet`, `Ojer Axonil`).
    fn replace_player_damage_rule(&mut self, mut p: Params) -> Option<()> {
        let recipients = match p.take("ValidTarget").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            _ => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let combat = match p.take("IsCombat").as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let Some(dmg_cmp) = p.take("DamageAmount") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let valid_source = p.take("ValidSource");
        let cond = self.prevention_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let Some((api, mut rep)) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        rep.drop_prose();
        if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("DamageAmount") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(var_val) = rep.take("VarValue") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if !rep.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let replacement = if let Some(ge_str) = dmg_cmp.trim().strip_prefix("GE")
            && let (Ok(threshold), Ok(cap)) = (ge_str.parse::<u32>(), var_val.trim().parse::<u32>())
            && threshold == cap.saturating_add(1)
        {
            format!("DamageAmountReplacement::Cap({cap})")
        } else if let Some(gt_str) = dmg_cmp.trim().strip_prefix("GT")
            && let (Ok(threshold), Ok(cap)) = (gt_str.parse::<u32>(), var_val.trim().parse::<u32>())
            && threshold == cap
        {
            format!("DamageAmountReplacement::Cap({cap})")
        } else if let Some(lt_var) = dmg_cmp.trim().strip_prefix("LT")
            && lt_var.trim() == var_val.trim()
            && self.svars.get(lt_var.trim()).map(|s| s.trim()) == Some("Count$CardPower")
        {
            "DamageAmountReplacement::FloorAtEffectSourcePower".to_string()
        } else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let src_ref = self.body.filter_static("SOURCE", &source_expr);
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::ReplacePlayerDamage {{ \
             sources: &{src_ref}, recipients: {recipients}, combat: {combat}, \
             replacement: {replacement} }}{cond})"
        ));
        Some(())
    }

    /// Persistent `R: Event$ DamageDone | ValidTarget$ Card.Self |
    /// AlwaysReplace$ True | PreventionEffect$ True` whose replacement prevents
    /// up to `CardCounters.<KIND>` damage while removing that many `<KIND>`
    /// counters (`Rock Hydra` -> `Modifier::CountersPreventDamage`).
    fn counters_prevent_damage_rule(&mut self, mut p: Params) -> Option<()> {
        if p.take("AlwaysReplace").as_deref() != Some("True")
            || p.take("PreventionEffect").as_deref() != Some("True")
            || p.take("ValidTarget").as_deref() != Some("Card.Self")
            || p.has("IsCombat")
            || p.has("ValidSource")
            || p.has("Condition")
            || p.has("IsPresent")
            || p.has("CheckSVar")
            || p.has("PlayerTurn")
            || p.has("Optional")
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let Some((rep_api, mut rep)) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        rep.drop_prose();
        let Some(amt_var) = rep.take("Amount") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(sub_name) = rep.take("SubAbility") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if rep_api != "ReplaceDamage" || !rep.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some((rem_api, mut rem)) = self
            .svars
            .get(sub_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        rem.drop_prose();
        if rem_api != "RemoveCounter" || rem.take("Defined").as_deref() != Some("Self") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(kind_raw) = rem.take("CounterType") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(num_var) = rem.take("CounterNum") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(kind) = super::counter_kind(kind_raw.trim()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if !rem.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(amt_def) = self.svars.get(amt_var.trim()).map(|s| s.trim()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some((lhs, rhs)) = amt_def.split_once("/LimitMax.") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let lhs_var = lhs.trim().strip_prefix("SVar$").map(str::trim);
        let lhs_ok = lhs_var == Some(num_var.trim())
            && self.svars.get(num_var.trim()).map(|s| s.trim())
                == Some("ReplaceCount$DamageAmount");
        let expected_cap = format!("Count$CardCounters.{}", kind_raw.trim());
        let rhs_ok = self.svars.get(rhs.trim()).map(|s| s.trim()) == Some(expected_cap.as_str());
        if !lhs_ok || !rhs_ok {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CountersPreventDamage({kind}))"
        ));
        Some(())
    }

    fn prevention_condition(&mut self, p: &mut Params) -> Option<String> {
        let your_turn = match p.take("PlayerTurn").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let rest = self.condition_any(p)?;
        if !your_turn {
            return Some(rest);
        }
        if let Some(existing) = rest
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return Some(format!(
                ", condition = Some(Condition::All(&[Condition::YourTurn, {existing}]))"
            ));
        }
        Some(", condition = Some(Condition::YourTurn)".to_string())
    }

    fn prevention_permanent_target(&self, valid: &str) -> Option<String> {
        let has_player_atom = valid.split(',').any(|alt| {
            matches!(
                alt.trim().split(['.', '+']).next(),
                Some("You" | "Player" | "Opponent")
            )
        });
        if has_player_atom {
            return None;
        }
        self.filter_expr(valid)
    }

    fn prevention_source_filter(&self, valid: &str) -> Option<String> {
        let normalized = match valid {
            "Card.OppCtrl,Emblem.OppCtrl" | "Emblem.OppCtrl,Card.OppCtrl" => "Card.OppCtrl",
            "Card.YouDontCtrl,Emblem.YouDontCtrl" | "Emblem.YouDontCtrl,Card.YouDontCtrl" => {
                "Card.YouDontCtrl"
            }
            "Card.YouCtrl,Emblem.YouCtrl" | "Emblem.YouCtrl,Card.YouCtrl" => "Card.YouCtrl",
            "Card,Emblem" | "Emblem,Card" => return Some("Filter::Any".to_string()),
            other => other,
        };
        if normalized == "Card.ChosenColor" && !self.has_chosen_color() {
            return self.deny(format!(
                "a prevention source outside instant/sorcery spells `{valid}`"
            ));
        }
        let mut alts = Vec::new();
        for raw_alt in normalized.split(',') {
            let alt = raw_alt.trim();
            let (base, atoms) = alt.split_once('.').unwrap_or((alt, ""));
            let mut battlefield = false;
            let stack_spell = base == "Spell";
            let mut kept_atoms = Vec::new();
            for atom in atoms.split('+').filter(|a| !a.is_empty()) {
                if atom == "inZoneBattlefield" {
                    battlefield = true;
                } else if atom == "ChosenColorSource" {
                    if !self.has_chosen_color() {
                        return self.deny(format!(
                            "a prevention source outside instant/sorcery spells `{valid}`"
                        ));
                    }
                    kept_atoms.push("ChosenColor".to_string());
                } else if let Some(color) = atom.strip_suffix("Source") {
                    if color == "Colorless" || super::color_atom(color).is_some() {
                        kept_atoms.push(color.to_string());
                    } else {
                        kept_atoms.push(atom.to_string());
                    }
                } else {
                    kept_atoms.push(atom.to_string());
                }
            }
            if stack_spell && battlefield {
                return None;
            }
            if stack_spell && kept_atoms.is_empty() {
                alts.push("Filter::InZone(ZoneRef::Stack)".to_string());
                continue;
            }
            let effective_base = if stack_spell { "Card" } else { base };
            let rebuilt = if kept_atoms.is_empty() {
                effective_base.to_string()
            } else {
                format!("{effective_base}.{}", kept_atoms.join("+"))
            };
            let expr = self.filter_expr(&rebuilt)?;
            if battlefield {
                alts.push(super::constants::on_the_battlefield(&expr));
            } else if stack_spell {
                alts.push(format!(
                    "Filter::And(&[Filter::InZone(ZoneRef::Stack), {expr}])"
                ));
            } else {
                alts.push(expr);
            }
        }
        Some(match alts.len() {
            0 => return None,
            1 => alts.remove(0),
            _ => format!("Filter::Or(&[{}])", alts.join(", ")),
        })
    }

    /// Activated or spell `Effect | ReplacementEffects$` prevention shield
    /// without `ChooseSource` / `ChooseColor` (e.g., Circle of Solace:
    /// `Effect::PreventNextFromMatchingSources` /
    /// `Effect::PreventAllDamageFromMatchingThisTurn`).
    ///
    /// Staged for follow-up wiring in `regeneration.rs` / `chains.rs`.
    /// Extracts an optional `ValidTgts$` + `RememberObjects$ Targeted` target
    /// binding on an `Effect | ReplacementEffects$` block, updating
    /// `chain.target` and returning `(target_spec, allowed_damage_valid_targets)`.
    pub(super) fn bind_effect_prevention_target(
        &mut self,
        effect: &mut Params,
        chain: &mut Chain,
        context: &str,
    ) -> Option<(Option<String>, Option<&'static [&'static str]>)> {
        let Some(valid) = effect.take("ValidTgts") else {
            return Some((None, None));
        };
        if chain.target.is_some() {
            return self.deny(format!("{context} with a second target"));
        }
        if effect.take("RememberObjects").as_deref() != Some("Targeted") {
            return self.deny(format!("{context} without `RememberObjects$ Targeted`"));
        }
        let spec = self.target_spec(&valid, "Effect")?;
        let expected = if spec.starts_with("TargetSpec::Object(") {
            if !matches!(
                effect.take("ForgetOnMoved").as_deref(),
                None | Some("Battlefield")
            ) || !matches!(
                effect.take("ExileOnMoved").as_deref(),
                None | Some("Battlefield")
            ) {
                return self.deny(format!("{context} with another `ForgetOnMoved$`"));
            }
            if valid.split(',').all(|alt| {
                alt.trim()
                    .split(['.', '+'])
                    .next()
                    .is_some_and(|b| b == "Creature")
            }) {
                &["Card.IsRemembered", "Creature.IsRemembered"][..]
            } else if valid.split(',').all(|alt| {
                alt.trim()
                    .split(['.', '+'])
                    .next()
                    .is_some_and(|b| b == "Land")
            }) {
                &["Card.IsRemembered", "Land.IsRemembered"][..]
            } else {
                &["Card.IsRemembered"][..]
            }
        } else if spec == "TargetSpec::AnyTarget" {
            if !matches!(
                effect.take("ForgetOnMoved").as_deref(),
                None | Some("Battlefield")
            ) || !matches!(
                effect.take("ExileOnMoved").as_deref(),
                None | Some("Battlefield")
            ) {
                return self.deny(format!("{context} with another `ForgetOnMoved$`"));
            }
            &[
                "Player.IsRemembered,Card.IsRemembered",
                "Card.IsRemembered,Player.IsRemembered",
            ][..]
        } else if spec == "TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)" {
            &[
                "Player.IsRemembered,Planeswalker.IsRemembered",
                "Planeswalker.IsRemembered,Player.IsRemembered",
            ][..]
        } else if matches!(
            spec.as_str(),
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"
        ) {
            &["Player.IsRemembered"][..]
        } else {
            return self.deny(format!("{context} targeting `{valid}`"));
        };
        chain.target = Some(spec.clone());
        Some((Some(spec), Some(expected)))
    }

    /// Activated or spell `Effect | ReplacementEffects$` prevention shield
    /// without `ChooseSource` / `ChooseColor` (e.g., Circle of Solace:
    /// `Effect::PreventNextFromMatchingSources` /
    /// `Effect::PreventAllDamageFromMatchingThisTurn`).
    pub(super) fn prevention_effect_shield(
        &mut self,
        mut p: Params,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        p.drop_prose();
        p.take("Name");
        p.take("NonStackingEffect");
        let replacement = p.take("ReplacementEffects")?;
        let (bound_spec, expected_target) =
            self.bind_effect_prevention_target(&mut p, chain, "`Effect` prevention")?;
        let remember_self = bound_spec.is_none() && p.peek("RememberObjects") == Some("Self");
        if remember_self {
            p.take("RememberObjects");
            if !matches!(
                p.take("ForgetOnMoved").as_deref(),
                None | Some("Battlefield")
            ) || !matches!(
                p.take("ExileOnMoved").as_deref(),
                None | Some("Battlefield")
            ) {
                return self.deny("`Effect` prevention with another `ForgetOnMoved$`".into());
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let body = self.svars.get(&replacement)?.clone();
        let (event, mut shield) = Params::parse(&body)?;
        shield.drop_prose();
        if event == "Destroy"
            && let Some(effects) = self.next_destruction_clears_damage(
                bound_spec.as_deref(),
                expected_target,
                shield.clone(),
            )
        {
            return Some(effects);
        }
        if event != "DamageDone"
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
        {
            return self.deny("an effect prevention with another event or zone".into());
        }
        if bound_spec.is_none()
            && !remember_self
            && shield.peek("Optional") == Some("True")
            && shield.peek("DamageTarget") == Some("You")
            && let Some(effects) = self.may_redirect_creature_damage_to_you(shield.clone())
        {
            return Some(effects);
        }
        let valid_source = shield.take("ValidSource");
        let valid_target = shield.take("ValidTarget");
        let combat_only = match shield.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return self.deny("an effect prevention with another `IsCombat$`".into()),
        };
        let next_only = if shield.take("Prevent").as_deref() == Some("True") {
            false
        } else if shield.take("PreventionEffect").as_deref() == Some("True")
            && shield
                .take("ReplaceWith")
                .is_some_and(|rw| self.exiles_itself(&rw))
        {
            true
        } else {
            return self.deny("an effect prevention with another replacement".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let target_expr = self.prevention_shield_target_expr(
            bound_spec,
            expected_target,
            remember_self,
            valid_target.as_deref(),
        )?;
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let src_ref = self.body.filter_static("SOURCE", &source_expr);
        let variant = if next_only {
            "Effect::PreventNextFromMatchingSources"
        } else {
            "Effect::PreventAllDamageFromMatchingThisTurn"
        };
        Some(vec![format!(
            "{variant} {{ target: {target_expr}, sources: &{src_ref}, \
             combat_only: {combat_only} }}"
        )])
    }

    fn prevention_shield_target_expr(
        &mut self,
        bound_spec: Option<String>,
        expected_target: Option<&[&str]>,
        remember_self: bool,
        valid_target: Option<&str>,
    ) -> Option<String> {
        let expr = match (bound_spec, expected_target, remember_self, valid_target) {
            (Some(spec), Some(expected), false, Some(actual)) if expected.contains(&actual) => {
                format!("Some({spec})")
            }
            (None, None, true, Some("Card.IsRemembered" | "Creature.IsRemembered"))
            | (None, None, false, Some("Card.Self" | "Creature.Self")) => {
                "Some(TargetSpec::Source)".to_string()
            }
            (None, None, false, None) => "None".to_string(),
            (None, None, false, Some("You")) => {
                "Some(TargetSpec::Player(PlayerRel::You))".to_string()
            }
            (None, None, false, Some("Player")) => {
                "Some(TargetSpec::Player(PlayerRel::EachPlayer))".to_string()
            }
            (
                None,
                None,
                false,
                Some(
                    "Card.EnchantedBy"
                    | "Creature.EnchantedBy"
                    | "Card.EquippedBy"
                    | "Creature.EquippedBy",
                ),
            ) => "Some(TargetSpec::Attached)".to_string(),
            _ => return self.deny("an effect prevention with another recipient".into()),
        };
        Some(expr)
    }

    fn next_destruction_clears_damage(
        &self,
        bound_spec: Option<&str>,
        expected_target: Option<&[&str]>,
        mut shield: Params,
    ) -> Option<Vec<String>> {
        if !bound_spec.is_some_and(|s| s.starts_with("TargetSpec::Object(")) {
            return None;
        }
        let expected = expected_target?;
        if !matches!(
            shield.take("ActiveZones").as_deref(),
            None | Some("Command")
        ) {
            return None;
        }
        let valid_card = shield.take("ValidCard")?;
        if !expected.contains(&valid_card.as_str()) {
            return None;
        }
        let replace_name = shield.take("ReplaceWith")?;
        if !shield.exhausted() {
            return None;
        }
        let (api, mut heal) = self
            .svars
            .get(replace_name.trim())
            .and_then(|s| Params::parse(s))?;
        heal.drop_prose();
        if api != "HealDamage" || heal.take("Defined").as_deref() != Some("ReplacedCard") {
            return None;
        }
        let exile = heal.take("SubAbility")?;
        if !heal.exhausted() || !self.exiles_itself(&exile) {
            return None;
        }
        Some(vec![
            "Effect::ModifyTargets { modifier: Modifier::NextDestructionClearsDamage, \
             duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    fn may_redirect_creature_damage_to_you(&self, mut shield: Params) -> Option<Vec<String>> {
        if shield.take("ValidTarget").as_deref() != Some("Creature")
            || shield.take("Optional").as_deref() != Some("True")
            || shield.take("OptionalDecider").as_deref() != Some("You")
            || shield.take("DamageTarget").as_deref() != Some("You")
        {
            return None;
        }
        let replace_name = shield.take("ReplaceWith")?;
        if !shield.exhausted() {
            return None;
        }
        let (api, mut rep) = self
            .svars
            .get(replace_name.trim())
            .and_then(|s| Params::parse(s))?;
        rep.drop_prose();
        if api != "ReplaceEffect"
            || rep.take("VarName").as_deref() != Some("Affected")
            || rep.take("VarValue").as_deref() != Some("You")
            || rep.take("VarType").as_deref() != Some("Player")
            || !rep.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::CreateContinuousEffect { layer: Layer::Text, filter: &Filter::CREATURE, \
             modifier: Modifier::MayRedirectDamageToYou, duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// Resolves `Choices$` and `RememberChosen$` for `ChooseSource` into
    /// `(source_filter_expr, valid_source_rechecks)`.
    pub(super) fn chosen_source_choices(
        &self,
        choices: &str,
        remember_chosen: bool,
    ) -> Option<(String, Vec<String>)> {
        if remember_chosen {
            if choices != "Card,Emblem" {
                return self.deny(format!("chosen source `{choices}`"));
            }
            return Some((
                "Filter::Any".to_string(),
                vec!["Card.IsRemembered,Emblem.IsRemembered".to_string()],
            ));
        }
        if choices == "Card,Emblem" {
            return Some((
                "Filter::Any".to_string(),
                vec![
                    "Card.ChosenCardStrict,Emblem.ChosenCard".to_string(),
                    "Card.ChosenCard,Emblem.ChosenCard".to_string(),
                ],
            ));
        }
        if choices.contains(',') {
            return self.deny(format!("chosen source `{choices}`"));
        }
        let atoms = choices.strip_prefix("Card.").unwrap_or(choices);
        let filter = if atoms == "ChosenColorSource" {
            if !self.has_chosen_color() {
                return self.deny(format!("chosen source `{choices}`"));
            }
            "Filter::OfSourceChosenColor".to_string()
        } else {
            let read = match atoms.strip_suffix("Source") {
                Some("Colorless") => "Card.Colorless".to_string(),
                Some(color) if super::color_atom(color).is_some() => format!("Card.{color}"),
                Some(_) => return self.deny(format!("chosen source `{choices}`")),
                None => choices.to_string(),
            };
            self.filter_expr(&read)?
        };
        Some((
            filter,
            vec![
                format!("Card.ChosenCardStrict+{atoms}"),
                format!("Card.ChosenCard+{atoms}"),
            ],
        ))
    }

    /// Reads the `DB$ Effect` line and optional trailing `DB$ Cleanup` of a
    /// `ChooseSource` shield, returning `(effect_params, replacement_svar)`.
    pub(super) fn read_chosen_source_effect(
        &self,
        sub: Option<&str>,
        remember_chosen: bool,
    ) -> Option<(Params, String)> {
        let Some(effect_name) = sub else {
            return self.deny("`ChooseSource` with nothing chosen for".to_string());
        };
        let Some(body) = self.svars.get(effect_name).cloned() else {
            return self.deny(format!("`SubAbility$ {effect_name}` names no SVar"));
        };
        let Some((api, mut effect)) = Params::parse(&body) else {
            return self.deny(format!("`SubAbility$ {effect_name}` is no ability"));
        };
        if api != "Effect" {
            return self.deny("`ChooseSource` followed by something other than its shield".into());
        }
        effect.drop_prose();
        let expected_defined = if remember_chosen {
            "Remembered"
        } else {
            "ChosenCard"
        };
        if remember_chosen && effect.take("RememberObjects").as_deref() != Some("Remembered") {
            return self.deny("a chosen-source shield on another condition".to_string());
        }
        let condition_holds = effect.take("ConditionDefined").as_deref() == Some(expected_defined)
            && matches!(
                effect.take("ConditionPresent").as_deref(),
                Some("Card" | "Card,Emblem")
            )
            && matches!(
                effect.take("ConditionCompare").as_deref(),
                None | Some("GE1")
            );
        if !condition_holds {
            return self.deny("a chosen-source shield on another condition".to_string());
        }
        let cleanup = effect.take("SubAbility");
        let Some(replacement) = effect.take("ReplacementEffects") else {
            return self.deny("a chosen-source `Effect` with no replacement".to_string());
        };
        if let Some(name) = cleanup {
            let clears = self
                .svars
                .get(&name)
                .and_then(|body| Params::parse(body))
                .is_some_and(|(c_api, mut c)| {
                    c_api == "Cleanup"
                        && c.take("ClearChosenCard").as_deref() == Some("True")
                        && (!remember_chosen
                            || c.take("ClearRemembered").as_deref() == Some("True"))
                        && c.exhausted()
                });
            if !clears {
                return self.deny(format!("`{name}` after a chosen-source shield"));
            }
        }
        Some((effect, replacement))
    }

    /// `ChooseSource` followed by targeted `DB$ Effect | ValidTgts$ ...`
    /// (`Charm Peddler`, `Samite Blessing`, `Shieldmage Advocate` ->
    /// `Effect::PreventFromChosenSource { target: Some(...), ... }`).
    ///
    /// Staged for follow-up wiring in `chains.rs`.
    #[allow(dead_code)]
    pub(super) fn prevent_from_chosen_source_chained(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("`ChooseSource` with no `Choices$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseSource.{key}`"));
        }
        let (source_filter, rechecks) = self.chosen_source_choices(&choices, false)?;
        let (mut effect, replacement) = self.read_chosen_source_effect(sub, false)?;
        let (target_spec, expected_target) = self.bind_chosen_source_target(&mut effect, chain)?;
        if let Some(key) = effect.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let (event, mut shield) = Params::parse(self.svars.get(&replacement)?)?;
        shield.drop_prose();
        if event != "DamageDone"
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
            || !shield
                .take("ValidSource")
                .is_some_and(|src| rechecks.iter().any(|r| r == &src))
        {
            return self.deny("a chosen-source replacement with another source".into());
        }
        let vt = shield.take("ValidTarget");
        let target_expr = match (target_spec, expected_target, vt.as_deref()) {
            (Some(spec), Some(expected), Some(actual)) if expected.contains(&actual) => {
                format!("Some({spec})")
            }
            (None, None, Some("You")) => "Some(TargetSpec::Player(PlayerRel::You))".to_string(),
            (None, None, None) => "None".to_string(),
            _ => return self.deny("a chosen-source shield with a mismatched target".into()),
        };
        let next_only = if shield.take("Prevent").as_deref() == Some("True") {
            false
        } else if shield.take("PreventionEffect").as_deref() == Some("True")
            && shield
                .take("ReplaceWith")
                .is_some_and(|rw| self.exiles_itself(&rw))
        {
            true
        } else {
            return self.deny("a chosen-source shield with another replacement".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let src_ref = self.body.filter_static("SOURCE", &source_filter);
        Some(vec![format!(
            "Effect::PreventFromChosenSource {{ target: {target_expr}, sources: &{src_ref}, \
             next_only: {next_only}, combat_only: false, all_but: 0, gain_life: false }}"
        )])
    }

    fn bind_chosen_source_target(
        &mut self,
        effect: &mut Params,
        chain: &mut Chain,
    ) -> Option<(Option<String>, Option<&'static [&'static str]>)> {
        let valid_tgts = effect.take("ValidTgts");
        let remember = effect.take("RememberObjects");
        let imprint = effect.take("ImprintCards");
        let forget = effect.take("ForgetOnMoved");
        let Some(valid) = valid_tgts else {
            return Some((None, None));
        };
        if chain.target.is_some() {
            return self.deny("a chosen-source shield with a second target".to_string());
        }
        let bound_ok = imprint.as_deref() == Some("Targeted")
            || (matches!(
                remember.as_deref(),
                Some("Targeted" | "TargetedPlayer & ThisTargetedCard")
            ) && matches!(forget.as_deref(), None | Some("Battlefield")));
        if !bound_ok {
            return self.deny("a chosen-source shield with unbound target".to_string());
        }
        let spec = self.target_spec(&valid, "Effect")?;
        chain.target = Some(spec.clone());
        let expected = if spec == "TargetSpec::AnyTarget" {
            &["Player.IsRemembered,Card.IsRemembered", "Card.IsImprinted"][..]
        } else if spec.starts_with("TargetSpec::Object(") {
            &[
                "Card.IsImprinted",
                "Card.IsRemembered",
                "Creature.IsRemembered",
            ][..]
        } else if matches!(
            spec.as_str(),
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"
        ) {
            &["Player.IsRemembered"][..]
        } else {
            return self.deny(format!("a chosen-source shield targeting `{valid}`"));
        };
        Some((Some(spec), Some(expected)))
    }
}
