//! Wave 27e scriptgen readers mapped onto Wave 15e paper-MTG DSL primitives:
//!
//! 1. `ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard { filter, reveal }`
//!    for `R:Event$ Moved | Destination$ Graveyard | ReplaceWith$ DBShuffle`
//!    (*Blightsteel Colossus*, *Darksteel Colossus*, *Progenitus*, *Nexus of
//!    Fate*, *Legacy Weapon*).
//! 2. `ReplacementRule::EnterWithCountersFromSource { filter }`
//!    (*Blue, Loyal Raptor*) and `Effect::PutEachCounterKindFromSource { target }`
//!    (`PutCounter | CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1`),
//!    plus multi-zone `GreatestCardPower` `K:etbCounter:` counts
//!    (*Ambitious Dragonborn*).
//! 3. `Modifier::ModifyLifeGain { who, op }` (`LifeGainOp::{Multiply, Add}`)
//!    for `R:Event$ GainLife` replacements (*Boon Reflection*, *Alhammarret's
//!    Archive*, *Rhox Faithmender*, *Cleric Class*, *Honor Troll*, *Phial of
//!    Galadriel*, etc.).
//! 4. `Modifier::PreventDamageRemovingCounters { kind, then }`
//!    (*Bloatfly Swarm* counter-removing prevention with `DamageRider`,
//!    including `Effect::Rad`), `DamageAmountReplacement::Subtract(u32)`
//!    (*Benevolent Unicorn*, *Lashknife Barrier*), generalized
//!    `Modifier::ReplaceDamage` amount caps (*Divine Presence*), and
//!    per-recipient `PreventThenRider` / *Crumbling Sanctuary* exile-top,
//!    *Undead Alchemist* / *Angel of Suffering* mill, and `Effect::Rad` riders.

use super::{Params, Tx, counter_kind};

impl Tx<'_> {
    /// Atomic-rollback entry point for Wave 27e `R:` replacement lines
    /// (`Event$ Moved` shuffle-instead-of-graveyard, `Event$ Moved`
    /// `EnterWithCountersFromSource`, and `Event$ GainLife` `ModifyLifeGain`).
    pub(super) fn wave27e_replacement(&mut self, spec: &str) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_enter_mods = self.body.enter_modifiers.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave27e_replacement_inner(spec);
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.enter_modifiers.truncate(saved_enter_mods);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave27e_replacement_inner(&mut self, spec: &str) -> Option<()> {
        let (event, p) = Params::parse(spec)?;
        match event.as_str() {
            "Moved" => self
                .wave27e_shuffle_instead_of_graveyard(p.clone())
                .or_else(|| self.wave27e_enter_with_counters_from_source(p)),
            "GainLife" => self.wave27e_modify_life_gain(p),
            _ => None,
        }
    }

    /// `R:Event$ Moved | Destination$ Graveyard | ReplaceWith$ <DBShuffle>`
    /// -> `ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard { filter, reveal }`.
    ///
    /// Strictly requires "from anywhere" (`Origin$` absent or `All`) so
    /// battlefield-only ("would die") shuffles stay refused.
    fn wave27e_shuffle_instead_of_graveyard(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Destination").as_deref() != Some("Graveyard")
            || !matches!(p.take("Origin").as_deref(), None | Some("All"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let active_zones = p.take("ActiveZones");
        let valid = p.take("ValidCard")?;
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }

        let is_self = valid == "Card.Self";
        if is_self {
            if !matches!(active_zones.as_deref(), None | Some("All")) {
                return None;
            }
        } else if active_zones.as_deref() != Some("Battlefield") || valid.contains("Self") {
            return None;
        }

        let (api, mut body) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || body.take("Destination").as_deref() != Some("Library")
            || body.take("Shuffle").as_deref() != Some("True")
            || !matches!(body.take("Origin").as_deref(), None | Some("All"))
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
        {
            return None;
        }
        let reveal = match body.take("Reveal").as_deref() {
            Some("True") => true,
            None | Some("False") => false,
            Some(_) => return None,
        };
        if !body.exhausted() {
            return None;
        }

        let filter_ref = if is_self {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            if expr == "Filter::Any" {
                "&Filter::Any".to_string()
            } else {
                format!("&{}", self.body.filter_static("SHUFFLE_GY", &expr))
            }
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard {{ \
             filter: {filter_ref}, reveal: {reveal} }})"
        ));
        Some(())
    }

    /// `R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Battlefield |
    /// ReplacementResult$ Updated | ValidCard$ <filter> | ReplaceWith$ <PutCounter>`
    /// with `DB$ PutCounter | ETB$ True | CounterType$ EachFromSource |
    /// EachFromSource$ Self | Defined$ ReplacedCard | CounterNum$ 1`
    /// -> `ReplacementRule::EnterWithCountersFromSource { filter }` (*Blue, Loyal Raptor*).
    fn wave27e_enter_with_counters_from_source(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(p.take("Origin").as_deref(), None | Some("Any"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let valid = p.take("ValidCard")?;
        if valid.contains("Self") {
            return None;
        }
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }

        let (api, mut body) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if api != "PutCounter"
            || body.take("ETB").as_deref() != Some("True")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || body.take("CounterType").as_deref() != Some("EachFromSource")
            || body.take("EachFromSource").as_deref() != Some("Self")
            || body.take("CounterNum").as_deref() != Some("1")
            || !body.exhausted()
        {
            return None;
        }

        let expr = self.filter_expr(&valid)?;
        let filter_ref = if expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("ENTER", &expr))
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::EnterWithCountersFromSource {{ \
             filter: {filter_ref} }})"
        ));
        Some(())
    }

    /// `DB$ PutCounter | CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1`
    /// -> `Effect::PutEachCounterKindFromSource { target }`.
    ///
    /// Strictly requires `CounterNum$ 1` and `EachFromSource$ Self`: scripts
    /// without `CounterNum$ 1` copy *all* counters of each kind (a different
    /// quantity than one counter of each kind) and must stay refused.
    pub(super) fn wave27e_put_each_counter_kind_from_source(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if self.on_a_spell {
            return self.deny("counter `EachFromSource`".to_string());
        }
        if p.take("CounterType").as_deref() != Some("EachFromSource")
            || p.take("EachFromSource").as_deref() != Some("Self")
            || p.take("CounterNum").as_deref() != Some("1")
            || p.has("ETB")
        {
            return self.deny("counter `EachFromSource`".to_string());
        }
        p.take("IsCurse");
        let spec = match p.take("Defined").as_deref() {
            Some("Self") if !targets_here => "TargetSpec::ThisObject".to_string(),
            Some(
                "TriggeredCardLKICopy"
                | "TriggeredCard"
                | "TriggeredNewCardLKICopy"
                | "TriggeredNewCard",
            ) if target.is_none()
                && matches!(
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
                ) =>
            {
                "TargetSpec::EventObject".to_string()
            }
            None if targets_here
                && !targets_a_player
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                target?.to_string()
            }
            Some("Targeted" | "ParentTarget")
                if !targets_here
                    && !targets_a_player
                    && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                target?.to_string()
            }
            _ => return self.deny("counter `EachFromSource`".to_string()),
        };
        Some(vec![format!(
            "Effect::PutEachCounterKindFromSource {{ target: {spec} }}"
        )])
    }

    /// Multi-zone `GreatestCardPower` `K:etbCounter:` counts (`Count$ValidGraveyard,Battlefield
    /// <valid>$GreatestCardPower` or `Battlefield,Graveyard`, e.g. *Ambitious Dragonborn*).
    pub(super) fn wave27e_etb_amount(&mut self, def: &str) -> Option<String> {
        let rest = def
            .strip_prefix("Count$ValidGraveyard,Battlefield ")
            .or_else(|| def.strip_prefix("Count$ValidBattlefield,Graveyard "))?;
        let valid_raw = rest.strip_suffix("$GreatestCardPower")?.trim();
        if valid_raw.is_empty() || valid_raw.contains('$') || valid_raw.contains("Self") {
            return None;
        }
        let bf_valid = Self::strip_other_atoms(valid_raw);
        let gy_valid = Self::ctrl_to_own_for_graveyard(&bf_valid)?;
        let bf_inner = self.filter_expr(&bf_valid)?;
        let gy_inner = self.filter_expr(&gy_valid)?;
        let bf_clause = super::constants::on_the_battlefield(&bf_inner);
        let gy_clause = Self::with_zone_prefix("Filter::InZone(ZoneRef::Graveyard)", &gy_inner);
        let combined = format!("Filter::Or(&[{bf_clause}, {gy_clause}])");
        let name = self.body.filter_static("COUNT", &combined);
        Some(format!("Amount::GreatestPowerAmong(&{name})"))
    }

    fn strip_other_atoms(spec: &str) -> String {
        spec.split(',')
            .map(|alt| {
                let alt = alt.trim();
                let Some((base, atoms)) = alt.split_once('.') else {
                    return alt.to_string();
                };
                let kept: Vec<&str> = atoms
                    .split('+')
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && *a != "Other" && *a != "StrictlyOther")
                    .collect();
                if kept.is_empty() {
                    base.trim().to_string()
                } else {
                    format!("{}.{}", base.trim(), kept.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn ctrl_to_own_for_graveyard(spec: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in spec.split(',') {
            let alt = alt.trim();
            if alt.is_empty() {
                return None;
            }
            let Some((base, atoms)) = alt.split_once('.') else {
                out.push(alt.to_string());
                continue;
            };
            let mut mapped = Vec::new();
            for atom in atoms.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                match atom {
                    "YouCtrl" => mapped.push("YouOwn"),
                    "OppCtrl" => mapped.push("OppOwn"),
                    "YouDontCtrl" => return None,
                    other => mapped.push(other),
                }
            }
            if mapped.is_empty() {
                out.push(base.trim().to_string());
            } else {
                out.push(format!("{}.{}", base.trim(), mapped.join("+")));
            }
        }
        Some(out.join(","))
    }

    fn with_zone_prefix(zone_clause: &str, expr: &str) -> String {
        match expr
            .strip_prefix("Filter::And(&[")
            .and_then(|rest| rest.strip_suffix("])"))
        {
            Some(clauses) => format!("Filter::And(&[{zone_clause}, {clauses}])"),
            None => format!("Filter::And(&[{zone_clause}, {expr}])"),
        }
    }

    /// `R:Event$ GainLife | ActiveZones$ Battlefield | ReplaceWith$ <ReplaceEffect>`
    /// -> `Modifier::ModifyLifeGain { who, op }` (*Boon Reflection*, *Alhammarret's
    /// Archive*, *Rhox Faithmender*, *Cleric Class*, *Honor Troll*, *Phial of
    /// Galadriel*, *Angel of Vitality*, *Leyline of Hope*, etc.).
    fn wave27e_modify_life_gain(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
            Some(_) => return None,
        };
        let replace_with = p.take("ReplaceWith")?;
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut body) = self
            .svars
            .get(replace_with.trim())
            .and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if api != "ReplaceEffect" || body.take("VarName").as_deref() != Some("LifeGained") {
            return None;
        }
        let val_raw = body.take("VarValue")?;
        if !body.exhausted() {
            return None;
        }
        let resolved = Self::resolve_svar_str(self.svars, val_raw.trim())?;
        let op = if resolved == "ReplaceCount$LifeGained/Twice" {
            "LifeGainOp::Multiply(2)".to_string()
        } else if resolved == "ReplaceCount$LifeGained/Thrice" {
            "LifeGainOp::Multiply(3)".to_string()
        } else if let Some(n_str) = resolved.strip_prefix("ReplaceCount$LifeGained/Times.") {
            let factor = n_str.trim().parse::<u32>().ok().filter(|&n| n >= 2)?;
            format!("LifeGainOp::Multiply({factor})")
        } else {
            let n_str = resolved.strip_prefix("ReplaceCount$LifeGained/Plus.")?;
            let extra = n_str.trim().parse::<u32>().ok().filter(|&n| n >= 1)?;
            format!("LifeGainOp::Add({extra})")
        };
        self.push_static_with_cond(
            "Filter::Any",
            &format!("Modifier::ModifyLifeGain {{ who: {who}, op: {op} }}"),
            &cond,
        );
        Some(())
    }

    pub(super) fn resolve_svar_str<'a>(
        svars: &'a std::collections::BTreeMap<String, String>,
        raw: &'a str,
    ) -> Option<&'a str> {
        if raw.contains('$') {
            Some(raw)
        } else {
            svars.get(raw).map(|s| s.trim())
        }
    }

    /// `R:Event$ DamageDone | ReplaceWith$ <ReplaceEffect>` with
    /// `ReplaceCount$DamageAmount/Minus.<n>` -> `Modifier::ReplaceDamage` with
    /// `DamageAmountReplacement::Subtract(n)` (*Benevolent Unicorn*,
    /// *Lashknife Barrier*).
    pub(super) fn wave27e_damage_subtract_rule(
        &mut self,
        orig: &Params,
        zone_arg: &str,
    ) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave27e_damage_subtract_rule_inner(orig.clone(), zone_arg);
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave27e_damage_subtract_rule_inner(&mut self, mut p: Params, zone_arg: &str) -> Option<()> {
        if p.has("AlwaysReplace")
            || p.has("DamageAmount")
            || p.has("DamageTarget")
            || p.has("ExecuteMode")
            || p.has("Optional")
            || p.has("OptionalDecider")
            || p.has("Prevent")
            || p.has("PreventionEffect")
        {
            return None;
        }
        let combat = match p.take("IsCombat").as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => return None,
        };
        let replace_name = p.take("ReplaceWith")?;
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        let cond = self.prevention_condition(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut rep) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))?;
        rep.drop_prose();
        if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("DamageAmount") {
            return None;
        }
        let val_raw = rep.take("VarValue")?;
        if !rep.exhausted() {
            return None;
        }
        let resolved = Self::resolve_svar_str(self.svars, val_raw.trim())?;
        let n_str = resolved.strip_prefix("ReplaceCount$DamageAmount/Minus.")?;
        let sub = n_str.trim().parse::<u32>().ok().filter(|&n| n >= 1)?;
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let recipients = self.damage_recipients_expr(valid_target.as_deref())?;
        let src_ref = if source_expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else if source_expr == "Filter::This" {
            "&Filter::This".to_string()
        } else {
            format!("&{}", self.body.filter_static("SOURCE", &source_expr))
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::ReplaceDamage {{ \
             sources: {src_ref}, recipients: {recipients}, combat: {combat}, \
             replacement: DamageAmountReplacement::Subtract({sub}) }}{zone_arg}{cond})"
        ));
        Some(())
    }

    /// Generalized `R:Event$ DamageDone | DamageAmount$ GE<cap+1>|GT<cap>`
    /// across non-player-only recipients (`Permanent,Player`, permanent filters)
    /// -> `Modifier::ReplaceDamage` with `DamageAmountReplacement::Cap(cap)`
    /// (*Divine Presence*). Leaves player-only targets on `replace_player_damage_rule`.
    pub(super) fn wave27e_replace_damage_amount(&mut self, orig: &Params) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave27e_replace_damage_amount_inner(orig.clone());
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave27e_replace_damage_amount_inner(&mut self, mut p: Params) -> Option<()> {
        if matches!(
            p.peek("ValidTarget"),
            Some(
                "You"
                    | "Opponent"
                    | "Player.Opponent"
                    | "Player"
                    | "EnchantedPlayer"
                    | "Player.EnchantedBy"
                    | "Player.EnchantedPlayer"
            )
        ) {
            return None;
        }
        let combat = match p.take("IsCombat").as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => return None,
        };
        let dmg_cmp = p.take("DamageAmount")?;
        let replace_name = p.take("ReplaceWith")?;
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        let cond = self.prevention_condition(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut rep) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))?;
        rep.drop_prose();
        if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("DamageAmount") {
            return None;
        }
        let var_val = rep.take("VarValue")?;
        if !rep.exhausted() {
            return None;
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
            return None;
        };
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => self.prevention_source_filter(vs)?,
        };
        let recipients = self.damage_recipients_expr(valid_target.as_deref())?;
        let src_ref = if source_expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else if source_expr == "Filter::This" {
            "&Filter::This".to_string()
        } else {
            format!("&{}", self.body.filter_static("SOURCE", &source_expr))
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::ReplaceDamage {{ \
             sources: {src_ref}, recipients: {recipients}, combat: {combat}, \
             replacement: {replacement} }}{cond})"
        ));
        Some(())
    }

    /// `R:Event$ DamageDone | ValidTarget$ Card.Self+counters_GE1_<KIND> |
    /// AlwaysReplace$ True | PreventionEffect$ True | ReplaceWith$ <RemoveCounter>`
    /// where `RemoveCounter` has `RememberAmount$ True` and chains into a
    /// `DamageRider` reading `Count$RememberedNumber` (*Bloatfly Swarm* ->
    /// `Modifier::PreventDamageRemovingCounters { kind, then: Some(...) }`).
    ///
    /// Leaves plain counter-removing preventions without a rider on
    /// `counters_prevent_damage_rule`.
    pub(super) fn wave27e_prevent_damage_removing_counters(&mut self, orig: &Params) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave27e_prevent_damage_removing_counters_inner(orig.clone());
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave27e_prevent_damage_removing_counters_inner(&mut self, mut p: Params) -> Option<()> {
        let valid_target = p.take("ValidTarget")?;
        let required_kind_raw = valid_target.strip_prefix("Card.Self+counters_GE1_")?;
        if required_kind_raw.is_empty() {
            return None;
        }
        if p.take("AlwaysReplace").as_deref() != Some("True")
            || p.take("PreventionEffect").as_deref() != Some("True")
        {
            return None;
        }
        let replace_name = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }

        let (rep_api, mut rep) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))?;
        rep.drop_prose();
        if rep_api != "RemoveCounter"
            || !matches!(
                rep.take("Defined").as_deref(),
                Some("Self" | "ReplacedTarget")
            )
            || rep.take("RememberAmount").as_deref() != Some("True")
        {
            return None;
        }
        let kind_raw = rep.take("CounterType")?;
        if kind_raw.trim() != required_kind_raw {
            return None;
        }
        let kind = counter_kind(kind_raw.trim())?;
        let num_var = rep.take("CounterNum")?;
        if Self::resolve_svar_str(self.svars, num_var.trim()) != Some("ReplaceCount$DamageAmount") {
            return None;
        }
        let sub_name = rep.take("SubAbility")?;
        if !rep.exhausted() {
            return None;
        }
        let then_eff = self.wave27e_parse_removed_counter_rider(sub_name.trim())?;
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::PreventDamageRemovingCounters {{ \
             kind: {kind}, then: Some(DamageRider::new(&[{then_eff}])) }})"
        ));
        Some(())
    }

    /// Parses a `DamageRider` follow-up after `DB$ RemoveCounter | RememberAmount$ True`
    /// whose count is `Count$RememberedNumber` (with optional `DB$ Cleanup | ClearRemembered$ True`).
    fn wave27e_parse_removed_counter_rider(&mut self, svar_name: &str) -> Option<String> {
        let body = self.svars.get(svar_name)?.clone();
        let (api, mut rep) = Params::parse(&body)?;
        rep.drop_prose();
        if let Some(sub_name) = rep.take("SubAbility")
            && !self.is_clear_remembered_cleanup(sub_name.trim())
        {
            return None;
        }
        let is_rem_num = |raw: &str| {
            Self::resolve_svar_str(self.svars, raw.trim()) == Some("Count$RememberedNumber")
        };
        let eff = match api.as_str() {
            "Radiation" => {
                let who = Self::rider_player_rel(rep.take("Defined").as_deref(), false, false)?;
                let num = rep.take("Num")?;
                if !is_rem_num(&num) || !rep.exhausted() {
                    return None;
                }
                format!(
                    "Effect::Rad {{ who: {who}, amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                )
            }
            "GainLife" => {
                if !matches!(rep.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                let amt = rep.take("LifeAmount")?;
                if !is_rem_num(&amt) || !rep.exhausted() {
                    return None;
                }
                "Effect::GainLife { amount: Amount::ThisWay(ThisWay::DamagePrevented) }".to_string()
            }
            "Mill" => {
                let who = Self::rider_player_rel(rep.take("Defined").as_deref(), false, false)?;
                let num = rep.take("NumCards")?;
                if !is_rem_num(&num) || !rep.exhausted() {
                    return None;
                }
                format!(
                    "Effect::Mill {{ amount: Amount::ThisWay(ThisWay::DamagePrevented), \
                     target: {who} }}"
                )
            }
            "Dig" => {
                if !matches!(rep.take("Defined").as_deref(), None | Some("You"))
                    || rep.take("ChangeNum").as_deref() != Some("All")
                    || !matches!(rep.take("ChangeValid").as_deref(), None | Some("Card"))
                    || rep.take("DestinationZone").as_deref() != Some("Exile")
                {
                    return None;
                }
                let dig_num = rep.take("DigNum")?;
                if !is_rem_num(&dig_num) || !rep.exhausted() {
                    return None;
                }
                "Effect::ExileTop { who: PlayerRel::You, \
                 count: Amount::ThisWay(ThisWay::DamagePrevented) }"
                    .to_string()
            }
            "PutCounter" => {
                if self.on_a_spell || !matches!(rep.take("Defined").as_deref(), None | Some("Self"))
                {
                    return None;
                }
                let kind_raw = rep.take("CounterType")?;
                let kind = counter_kind(kind_raw.trim())?;
                let num = rep.take("CounterNum")?;
                if !is_rem_num(&num) || !rep.exhausted() {
                    return None;
                }
                format!(
                    "Effect::AddCounterSource {{ kind: {kind}, \
                     amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                )
            }
            "Token" => {
                if !matches!(rep.take("TokenOwner").as_deref(), None | Some("You")) {
                    return None;
                }
                let amt = rep.take("TokenAmount")?;
                let stem = rep.take("TokenScript")?;
                if !is_rem_num(&amt) || !rep.exhausted() {
                    return None;
                }
                let tok = self.tokens?.body_fixed(stem.trim(), self.cats)?;
                format!(
                    "Effect::CreateTokenN {{ token: &generated_tokens::{}, \
                     amount: Amount::ThisWay(ThisWay::DamagePrevented) }}",
                    tok.constant
                )
            }
            _ => return None,
        };
        Some(eff)
    }

    fn is_clear_remembered_cleanup(&self, svar_name: &str) -> bool {
        let Some((api, mut p)) = self.svars.get(svar_name).and_then(|b| Params::parse(b)) else {
            return false;
        };
        p.drop_prose();
        api == "Cleanup" && p.take("ClearRemembered").as_deref() == Some("True") && p.exhausted()
    }

    /// Per-recipient `PreventThenRider` (*Crumbling Sanctuary* exile-top,
    /// *Undead Alchemist* mill, *Angel of Suffering* double mill, and
    /// `Effect::Rad` riders) on `Modifier::ReplaceOrPreventDamageWithThen`.
    ///
    /// Only claims shapes that `prevention.rs::replace_or_prevent_damage_with_then_rule`
    /// does not already handle (per-target player riders, `Radiation` riders,
    /// and multiplied damage-prevented mill/exile/rad counts).
    pub(super) fn wave27e_damage_then_rule(
        &mut self,
        orig: &Params,
        zone_arg: &str,
        is_replacement: bool,
    ) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave27e_damage_then_rule_inner(orig.clone(), zone_arg, is_replacement);
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave27e_damage_then_rule_inner(
        &mut self,
        mut p: Params,
        zone_arg: &str,
        is_replacement: bool,
    ) -> Option<()> {
        if p.has("DamageAmount")
            || p.has("DamageTarget")
            || p.has("Optional")
            || p.has("OptionalDecider")
        {
            return None;
        }
        if !is_replacement
            && (p.take("PreventionEffect").as_deref() != Some("True") || p.has("Prevent"))
        {
            return None;
        }
        if !matches!(p.take("AlwaysReplace").as_deref(), None | Some("True")) {
            return None;
        }
        let exec_mode = p.take("ExecuteMode");
        if !matches!(exec_mode.as_deref(), None | Some("PerSource" | "PerTarget")) {
            return None;
        }
        let combat = match p.take("IsCombat").as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => return None,
        };
        let cond = self.prevention_condition(&mut p)?;
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        let replace_name = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }
        let recipients = self.damage_recipients_expr(valid_target.as_deref())?;
        let player_recipient = recipients.starts_with("DamageRecipients::Players(");
        let you_only = recipients == "DamageRecipients::Players(PlayerRel::You)";
        if exec_mode.as_deref() == Some("PerTarget") && !player_recipient {
            return None;
        }
        let (then_eff, is_wave27e_extension) = self.wave27e_parse_damage_rider(
            replace_name.trim(),
            player_recipient,
            you_only,
            exec_mode.as_deref() == Some("PerTarget"),
        )?;
        if !is_wave27e_extension {
            return None;
        }
        let source_ref = match valid_source.as_deref() {
            None => "&Filter::Any".to_string(),
            Some(vs) => {
                let expr = self.prevention_source_filter(vs)?;
                if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else if expr == "Filter::This" {
                    "&Filter::This".to_string()
                } else {
                    format!("&{}", self.body.filter_static("SOURCE", &expr))
                }
            }
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen {{ \
             sources: {source_ref}, recipients: {recipients}, combat: {combat}, \
             is_replacement: {is_replacement}, then: DamageRider::new(&[{then_eff}]) }}{zone_arg}{cond})"
        ));
        Some(())
    }

    fn wave27e_parse_damage_rider(
        &self,
        svar_name: &str,
        player_recipient: bool,
        you_only: bool,
        is_per_target: bool,
    ) -> Option<(String, bool)> {
        let body = self.svars.get(svar_name)?;
        let (api, mut rep) = Params::parse(body)?;
        rep.drop_prose();
        if rep.has("SubAbility") {
            return None;
        }
        match api.as_str() {
            "Dig" => {
                let def = rep.take("Defined");
                let is_replaced_target = def.as_deref() == Some("ReplacedTarget");
                let who = Self::rider_player_rel(def.as_deref(), player_recipient, you_only)?;
                if rep.take("ChangeNum").as_deref() != Some("All")
                    || !matches!(rep.take("ChangeValid").as_deref(), None | Some("Card"))
                    || rep.take("DestinationZone").as_deref() != Some("Exile")
                {
                    return None;
                }
                let dig_num = rep.take("DigNum")?;
                let (amt_expr, scaled) = self.damage_prevented_amount_expr(&dig_num)?;
                if !rep.exhausted() {
                    return None;
                }
                let eff = format!("Effect::ExileTop {{ who: {who}, count: {amt_expr} }}");
                Some((eff, is_replaced_target || is_per_target || scaled))
            }
            "Mill" => {
                let def = rep.take("Defined");
                let is_replaced_target = def.as_deref() == Some("ReplacedTarget");
                let who = Self::rider_player_rel(def.as_deref(), player_recipient, you_only)?;
                let num = rep.take("NumCards")?;
                let (amt_expr, scaled) = self.damage_prevented_amount_expr(&num)?;
                if !rep.exhausted() {
                    return None;
                }
                let eff = format!("Effect::Mill {{ amount: {amt_expr}, target: {who} }}");
                Some((eff, is_replaced_target || is_per_target || scaled))
            }
            "Radiation" => {
                let def = rep.take("Defined");
                let who = Self::rider_player_rel(def.as_deref(), player_recipient, you_only)?;
                let num = rep.take("Num")?;
                let (amt_expr, _) = self.damage_prevented_amount_expr(&num)?;
                if !rep.exhausted() {
                    return None;
                }
                let eff = format!("Effect::Rad {{ who: {who}, amount: {amt_expr} }}");
                Some((eff, true))
            }
            _ => None,
        }
    }

    fn rider_player_rel(
        defined: Option<&str>,
        player_recipient: bool,
        you_only: bool,
    ) -> Option<&'static str> {
        match defined {
            None | Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some("ReplacedTarget") if you_only => Some("PlayerRel::You"),
            Some("ReplacedTarget") if player_recipient => Some("PlayerRel::DamagedPlayer"),
            _ => None,
        }
    }

    fn damage_prevented_amount_expr(&self, raw: &str) -> Option<(String, bool)> {
        let resolved = Self::resolve_svar_str(self.svars, raw.trim())?;
        if resolved == "ReplaceCount$DamageAmount" {
            return Some((
                "Amount::ThisWay(ThisWay::DamagePrevented)".to_string(),
                false,
            ));
        }
        let factor = if resolved == "ReplaceCount$DamageAmount/Twice" {
            2
        } else if resolved == "ReplaceCount$DamageAmount/Thrice" {
            3
        } else {
            let n_str = resolved.strip_prefix("ReplaceCount$DamageAmount/Times.")?;
            n_str.trim().parse::<u32>().ok().filter(|&n| n >= 2)?
        };
        Some((
            format!(
                "Amount::Times {{ base: &Amount::ThisWay(ThisWay::DamagePrevented), \
                 factor: {factor} }}"
            ),
            true,
        ))
    }
}
