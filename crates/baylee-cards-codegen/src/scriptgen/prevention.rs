//! Persistent and effect-created damage prevention shields.

use super::{Chain, Params, Tx};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DamageScope {
    All,
    CombatOnly,
    NoncombatOnly,
}

enum DamageScale {
    Multiply(u8),
    Add(u8),
    HalfDown,
    HalfUp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RiderTarget {
    Other,
    You,
    Source,
    Attached,
}

impl Tx<'_> {
    /// Persistent `R: Event$ DamageDone` prevention rules:
    /// - Spell-source `PreventAllDamageFromIt` (`ValidSource$ Instant,Sorcery`);
    /// - Infinite damage prevention to you (`Modifier::PreventDamageToYouFrom`) or
    ///   matching permanents (`Modifier::PreventDamageFrom` /
    ///   `Modifier::PreventCombatDamageFrom`);
    /// - Finite per-source prevention (`Modifier::PreventDamagePerSource` and
    ///   `Modifier::PreventDamageToAffectedPerSource` for Spheres, Urza's Armor,
    ///   Cover of Winter, Daunting Defender, Guardian Seraph, Reidane, etc.).
    #[allow(clippy::too_many_lines)]
    pub(super) fn spell_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let secondary = p.take("Secondary");
        let active_zone = p.take("ActiveZones");
        let zone_arg = match active_zone.as_deref() {
            None | Some("Battlefield") => "",
            Some("Command") => ", zone = ZoneRef::Command",
            Some("Battlefield,Command" | "Command,Battlefield") => {
                ", zone = ZoneRef::BattlefieldOrCommand"
            }
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        if !matches!(secondary.as_deref(), None | Some("True")) {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        if matches!(
            p.peek("DamageTarget"),
            Some("Self" | "Enchanted" | "Equipped")
        ) && p.peek("ReplaceWith").is_some()
        {
            if !zone_arg.is_empty() {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
            return self.redirect_damage_to_you(p);
        }
        if p.peek("ReplaceWith").is_some() {
            let replace_api = p
                .peek("ReplaceWith")
                .and_then(|rw| self.svars.get(rw.trim()))
                .and_then(|body| Params::parse(body))
                .map(|(api, _)| api);
            if p.peek("AlwaysReplace") == Some("True")
                && p.peek("ValidTarget").is_some_and(|vt| {
                    vt == "Card.Self" || vt.starts_with("Card.Self+counters_GE1_")
                })
                && matches!(
                    replace_api.as_deref(),
                    None | Some("RemoveCounter" | "ReplaceDamage")
                )
            {
                if !zone_arg.is_empty() {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                }
                return self.counters_prevent_damage_rule(p);
            }
            if p.has("DamageAmount") && !p.has("PreventionEffect") && !p.has("Prevent") {
                if !zone_arg.is_empty() {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                }
                return self.replace_player_damage_rule(p);
            }
            if !p.has("PreventionEffect") && !p.has("Prevent") {
                if replace_api.as_deref() == Some("ReplaceEffect") {
                    return self.damage_multiplier_or_adder_rule(p, zone_arg);
                }
                return self.replace_or_prevent_damage_with_then_rule(p, zone_arg, true);
            }
            if replace_api.as_deref() == Some("ReplaceDamage") {
                if !zone_arg.is_empty() {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                }
                return self.per_source_damage_prevention(p);
            }
            return self.replace_or_prevent_damage_with_then_rule(p, zone_arg, false);
        }
        if p.take("Prevent").as_deref() != Some("True") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let scope = match p.take("IsCombat").as_deref() {
            None => DamageScope::All,
            Some("True") => DamageScope::CombatOnly,
            Some("False") => DamageScope::NoncombatOnly,
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
            return self.emit_source_only_damage_prevention(
                valid_source,
                scope,
                zone_arg,
                &cond,
                secondary.is_some(),
            );
        };

        if valid_source
            .as_deref()
            .is_some_and(Self::is_stack_spell_source)
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        self.emit_target_damage_prevention(
            valid_source.as_deref(),
            &target_raw,
            scope,
            zone_arg,
            &cond,
        )
    }

    fn emit_source_only_damage_prevention(
        &mut self,
        valid_source: Option<String>,
        scope: DamageScope,
        zone_arg: &str,
        cond: &str,
        has_secondary: bool,
    ) -> Option<()> {
        if scope == DamageScope::NoncombatOnly {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let combat_only = scope == DamageScope::CombatOnly;
        let valid = valid_source?;
        if Self::is_stack_spell_source(&valid) {
            if combat_only || !zone_arg.is_empty() || !cond.is_empty() || has_secondary {
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
                "static_ability!({filter}, Modifier::PreventDamageFromIt{zone_arg}{cond})"
            ));
            return Some(());
        }
        if valid.contains("Self") || valid.contains("EnchantedBy") || valid.contains("EquippedBy") {
            let filter = self.filter_expr(&valid)?;
            self.body.abilities.push(format!(
                "static_ability!({filter}, Modifier::PreventAllDamageFromIt{zone_arg}{cond})"
            ));
            return Some(());
        }
        self.deny(format!(
            "a prevention source outside instant/sorcery spells `{valid}`"
        ))
    }

    fn emit_target_damage_prevention(
        &mut self,
        valid_source: Option<&str>,
        target_raw: &str,
        scope: DamageScope,
        zone_arg: &str,
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
        if scope == DamageScope::NoncombatOnly {
            return self.emit_noncombat_damage_prevention(
                &source_expr,
                &target_parts,
                zone_arg,
                cond,
            );
        }
        let combat_only = scope == DamageScope::CombatOnly;
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
                "static_ability!(Filter::This, Modifier::PreventDamageToYouFrom(&{src_ref}){zone_arg}{cond})"
            ));
        }
        if let Some(target_filter) = target_filter {
            let modifier = if combat_only {
                format!("Modifier::PreventCombatDamageFrom(&{src_ref})")
            } else {
                format!("Modifier::PreventDamageFrom(&{src_ref})")
            };
            self.body.abilities.push(format!(
                "static_ability!({target_filter}, {modifier}{zone_arg}{cond})"
            ));
        }
        Some(())
    }

    fn emit_noncombat_damage_prevention(
        &mut self,
        source_expr: &str,
        target_parts: &[&str],
        zone_arg: &str,
        cond: &str,
    ) -> Option<()> {
        let mut player_rels: Vec<&'static str> = Vec::new();
        let mut perm_alts: Vec<&str> = Vec::new();
        for &part in target_parts {
            match part {
                "You" => player_rels.push("PlayerRel::You"),
                "Player" => player_rels.push("PlayerRel::EachPlayer"),
                "Opponent" | "Player.Opponent" => player_rels.push("PlayerRel::Opponent"),
                "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                    player_rels.push("PlayerRel::EnchantedPlayer");
                }
                _ => perm_alts.push(part),
            }
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
        let src_ref = self.body.filter_static("SOURCE", source_expr);
        for who in player_rels {
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::PreventNoncombatDamageToPlayers {{ \
                 who: {who}, from: &{src_ref} }}{zone_arg}{cond})"
            ));
        }
        if let Some(target_filter) = target_filter {
            self.body.abilities.push(format!(
                "static_ability!({target_filter}, Modifier::PreventNoncombatDamageFrom(&{src_ref}){zone_arg}{cond})"
            ));
        }
        Some(())
    }

    /// Persistent `R: Event$ DamageDone | ReplaceWith$ <ReplaceEffect>` damage
    /// multipliers (`Modifier::MultiplyDamageFrom`) and flat adders
    /// (`Modifier::AddDamageFrom`) over `DamageRecipients` (CR 614.1a).
    fn damage_multiplier_or_adder_rule(&mut self, mut p: Params, zone_arg: &str) -> Option<()> {
        if p.has("AlwaysReplace")
            || p.has("DamageAmount")
            || p.has("DamageTarget")
            || p.has("ExecuteMode")
            || p.has("Optional")
            || p.has("OptionalDecider")
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let (combat_only, noncombat_only) = match p.take("IsCombat").as_deref() {
            None => (false, false),
            Some("True") => (true, false),
            Some("False") => (false, true),
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        let cond = self.prevention_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(scale) = self.parse_damage_scale(replace_name.trim()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let source_expr = match valid_source.as_deref() {
            None => "Filter::Any".to_string(),
            Some(vs) => {
                let Some(expr) = self.prevention_source_filter(vs) else {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                };
                expr
            }
        };
        let Some(recipients) = self.damage_recipients_expr(valid_target.as_deref()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let src_ref = self.body.filter_static("SOURCE", &source_expr);
        let combat = match (combat_only, noncombat_only) {
            (true, false) => "Some(true)",
            (false, true) => "Some(false)",
            _ => "None",
        };
        let modifier = match scale {
            DamageScale::Multiply(factor) => format!(
                "Modifier::MultiplyDamageFrom {{ sources: &{src_ref}, factor: {factor}, \
                 combat_only: {combat_only}, noncombat_only: {noncombat_only}, \
                 recipients: {recipients} }}"
            ),
            DamageScale::Add(extra) => format!(
                "Modifier::AddDamageFrom {{ sources: &{src_ref}, extra: {extra}, \
                 combat_only: {combat_only}, noncombat_only: {noncombat_only}, \
                 recipients: {recipients} }}"
            ),
            DamageScale::HalfDown => format!(
                "Modifier::ReplaceDamage {{ sources: &{src_ref}, recipients: {recipients}, \
                 combat: {combat}, replacement: DamageAmountReplacement::HalfDown }}"
            ),
            DamageScale::HalfUp => format!(
                "Modifier::ReplaceDamage {{ sources: &{src_ref}, recipients: {recipients}, \
                 combat: {combat}, replacement: DamageAmountReplacement::HalfUp }}"
            ),
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, {modifier}{zone_arg}{cond})"
        ));
        Some(())
    }

    fn parse_damage_scale(&self, replace_name: &str) -> Option<DamageScale> {
        let (api, mut rep) = self
            .svars
            .get(replace_name)
            .and_then(|b| Params::parse(b))?;
        rep.drop_prose();
        if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("DamageAmount") {
            return None;
        }
        let val_raw = rep.take("VarValue")?;
        if !rep.exhausted() {
            return None;
        }
        let val = val_raw.trim();
        let resolved = if val.starts_with("ReplaceCount$") {
            val
        } else {
            self.svars.get(val)?.trim()
        };
        if resolved == "ReplaceCount$DamageAmount/Twice" {
            return Some(DamageScale::Multiply(2));
        }
        if resolved == "ReplaceCount$DamageAmount/Thrice" {
            return Some(DamageScale::Multiply(3));
        }
        if resolved == "ReplaceCount$DamageAmount/HalfDown" {
            return Some(DamageScale::HalfDown);
        }
        if resolved == "ReplaceCount$DamageAmount/HalfUp" {
            return Some(DamageScale::HalfUp);
        }
        if let Some(n_str) = resolved.strip_prefix("ReplaceCount$DamageAmount/Times.") {
            let factor = n_str.trim().parse::<u8>().ok()?;
            return (factor >= 2).then_some(DamageScale::Multiply(factor));
        }
        if let Some(n_str) = resolved.strip_prefix("ReplaceCount$DamageAmount/Plus.") {
            let extra = n_str.trim().parse::<u8>().ok()?;
            return (extra >= 1).then_some(DamageScale::Add(extra));
        }
        None
    }

    /// Persistent `R: Event$ DamageDone | ReplaceWith$ <rider>` where the rider
    /// scales linearly with `ReplaceCount$DamageAmount` -> Wave 14d
    /// `Modifier::ReplaceOrPreventDamageWithThen` (*Gloom Surgeon*, *Force
    /// Bubble*, *Purity*, *Hostility*, *The Mindskinner*, *Phytohydra*,
    /// *Lichenthrope*, *Phyrexian Hydra*, *Panther Habit*, *Delaying Shield*).
    fn replace_or_prevent_damage_with_then_rule(
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
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        if !is_replacement
            && (p.take("PreventionEffect").as_deref() != Some("True") || p.has("Prevent"))
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        if !matches!(p.take("AlwaysReplace").as_deref(), None | Some("True"))
            || !matches!(p.take("ExecuteMode").as_deref(), None | Some("PerSource"))
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let combat = match p.take("IsCombat").as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        let cond = self.prevention_condition(&mut p)?;
        let valid_source = p.take("ValidSource");
        let valid_target = p.take("ValidTarget");
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if !p.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let rider_target = match valid_target.as_deref() {
            Some("You") => RiderTarget::You,
            Some("Card.Self") => RiderTarget::Source,
            Some(
                "Card.EquippedBy"
                | "Creature.EquippedBy"
                | "Card.EnchantedBy"
                | "Creature.EnchantedBy",
            ) => RiderTarget::Attached,
            _ => RiderTarget::Other,
        };
        let Some(then_eff) =
            self.parse_damage_prevented_rider(replace_name.trim(), false, rider_target)
        else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let source_ref = match valid_source.as_deref() {
            None => "&Filter::Any".to_string(),
            Some(vs) => {
                let Some(expr) = self.prevention_source_filter(vs) else {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                };
                if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else if expr == "Filter::This" {
                    "&Filter::This".to_string()
                } else {
                    format!("&{}", self.body.filter_static("SOURCE", &expr))
                }
            }
        };
        let Some(recipients) = self.damage_recipients_expr(valid_target.as_deref()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen {{ \
             sources: {source_ref}, recipients: {recipients}, combat: {combat}, \
             is_replacement: {is_replacement}, then: DamageRider::new(&[{then_eff}]) }}{zone_arg}{cond})"
        ));
        Some(())
    }

    fn is_replaced_damage_amount(&self, raw: &str) -> bool {
        let trimmed = raw.trim();
        trimmed == "ReplaceCount$DamageAmount"
            || self
                .svars
                .get(trimmed)
                .is_some_and(|body| body.trim() == "ReplaceCount$DamageAmount")
    }

    /// Parses a non-interactive damage prevention/replacement rider whose count
    /// is `ReplaceCount$DamageAmount` (`Amount::ThisWay(ThisWay::DamagePrevented)`).
    #[allow(clippy::too_many_lines)]
    fn parse_damage_prevented_rider(
        &mut self,
        svar_name: &str,
        require_self_exile: bool,
        rider_target: RiderTarget,
    ) -> Option<String> {
        let body = self.svars.get(svar_name)?.clone();
        let (api, mut rep) = Params::parse(&body)?;
        rep.drop_prose();
        let sub = rep.take("SubAbility");
        if require_self_exile {
            if !sub.is_some_and(|name| self.exiles_itself(&name)) {
                return None;
            }
        } else if sub.is_some() {
            return None;
        }
        let effect = match api.as_str() {
            "GainLife" => {
                let def = rep.take("Defined");
                let who_ok = matches!(def.as_deref(), None | Some("You"))
                    || (def.as_deref() == Some("ReplacedTarget")
                        && rider_target == RiderTarget::You);
                let amt = rep.take("LifeAmount")?;
                if !who_ok || !self.is_replaced_damage_amount(&amt) || !rep.exhausted() {
                    return None;
                }
                "Effect::GainLife { amount: Amount::ThisWay(ThisWay::DamagePrevented) }".to_string()
            }
            "PutCounter" => {
                if self.on_a_spell {
                    return None;
                }
                let kind_raw = rep.take("CounterType")?;
                let kind = super::counter_kind(kind_raw.trim())?;
                let num_raw = rep.take("CounterNum")?;
                if !self.is_replaced_damage_amount(&num_raw) {
                    return None;
                }
                let def = rep.take("Defined");
                if !rep.exhausted() {
                    return None;
                }
                match def.as_deref() {
                    None | Some("Self") => format!(
                        "Effect::AddCounterSource {{ kind: {kind}, \
                         amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                    ),
                    Some("ReplacedTarget") if rider_target == RiderTarget::Source => format!(
                        "Effect::AddCounterSource {{ kind: {kind}, \
                         amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                    ),
                    Some("Equipped" | "Enchanted") => format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::AttachedToBySource, \
                         kind: {kind}, amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                    ),
                    Some("ReplacedTarget") if rider_target == RiderTarget::Attached => format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::AttachedToBySource, \
                         kind: {kind}, amount: Amount::ThisWay(ThisWay::DamagePrevented) }}"
                    ),
                    _ => return None,
                }
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
                if !self.is_replaced_damage_amount(&dig_num) || !rep.exhausted() {
                    return None;
                }
                "Effect::ExileTop { who: PlayerRel::You, \
                 count: Amount::ThisWay(ThisWay::DamagePrevented) }"
                    .to_string()
            }
            "Mill" => {
                let who = match rep.take("Defined").as_deref() {
                    None | Some("You") => "PlayerRel::You",
                    Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                    Some("Player") => "PlayerRel::EachPlayer",
                    _ => return None,
                };
                let num = rep.take("NumCards")?;
                if !self.is_replaced_damage_amount(&num) || !rep.exhausted() {
                    return None;
                }
                format!(
                    "Effect::Mill {{ amount: Amount::ThisWay(ThisWay::DamagePrevented), \
                     target: {who} }}"
                )
            }
            "Token" => {
                if !matches!(rep.take("TokenOwner").as_deref(), None | Some("You")) {
                    return None;
                }
                let amt = rep.take("TokenAmount")?;
                let stem = rep.take("TokenScript")?;
                if !self.is_replaced_damage_amount(&amt) || !rep.exhausted() {
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
        Some(effect)
    }

    fn damage_recipients_expr(&mut self, valid_target: Option<&str>) -> Option<String> {
        let Some(raw) = valid_target else {
            return Some("DamageRecipients::Any".to_string());
        };
        let parts: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        if parts == ["Any"]
            || (parts.len() == 2
                && ((parts.contains(&"Permanent") && parts.contains(&"Player"))
                    || (parts.contains(&"Card") && parts.contains(&"Player"))))
            || (parts.len() == 3
                && parts.contains(&"Permanent")
                && parts.contains(&"Player")
                && parts.contains(&"Planeswalker"))
        {
            return Some("DamageRecipients::Any".to_string());
        }
        if parts.len() == 1 {
            match parts[0] {
                "Player" => return Some("DamageRecipients::Players(PlayerRel::EachPlayer)".into()),
                "You" => return Some("DamageRecipients::Players(PlayerRel::You)".into()),
                "Opponent" | "Player.Opponent" => {
                    return Some("DamageRecipients::Players(PlayerRel::Opponent)".into());
                }
                "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                    return Some("DamageRecipients::Players(PlayerRel::EnchantedPlayer)".into());
                }
                _ => {}
            }
        }
        if parts.len() == 2 {
            let has_opp_player = parts.contains(&"Opponent") || parts.contains(&"Player.Opponent");
            let has_opp_perm =
                parts.contains(&"Permanent.OppCtrl") || parts.contains(&"Card.OppCtrl");
            if has_opp_player && has_opp_perm {
                return Some(
                    "DamageRecipients::PlayersOrTheirPermanents(PlayerRel::Opponent)".into(),
                );
            }
            let has_you = parts.contains(&"You");
            let has_you_perm =
                parts.contains(&"Permanent.YouCtrl") || parts.contains(&"Card.YouCtrl");
            if has_you && has_you_perm {
                return Some("DamageRecipients::PlayersOrTheirPermanents(PlayerRel::You)".into());
            }
        }
        let expr = self.prevention_permanent_target(&parts.join(","))?;
        if expr == "Filter::This" {
            return Some("DamageRecipients::Permanents(&Filter::This)".into());
        }
        let recip_ref = self.body.filter_static("RECIPIENT", &expr);
        Some(format!("DamageRecipients::Permanents(&{recip_ref})"))
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
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
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
        let required_kind = match p.take("ValidTarget").as_deref() {
            Some("Card.Self") => None,
            Some(raw) => {
                let kind_raw = raw.strip_prefix("Card.Self+counters_GE1_")?;
                if kind_raw.is_empty() || super::counter_kind(kind_raw).is_none() {
                    return self
                        .deny("a persistent spell prevention with another zone or result".into());
                }
                Some(kind_raw.to_string())
            }
            None => {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
        };
        if p.take("AlwaysReplace").as_deref() != Some("True")
            || p.take("PreventionEffect").as_deref() != Some("True")
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
        let (kind_raw, kind) = if rep_api == "RemoveCounter" {
            self.parse_direct_counter_prevention(rep)?
        } else {
            if rep_api != "ReplaceDamage" {
                return self
                    .deny("a persistent spell prevention with another zone or result".into());
            }
            self.parse_capped_counter_prevention(rep)?
        };
        if required_kind
            .as_deref()
            .is_some_and(|req| req != kind_raw.trim())
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CountersPreventDamage({kind}))"
        ));
        Some(())
    }

    /// Direct `DB$ RemoveCounter` removing all prevented damage worth of counters
    /// (`Defined$ Self|ReplacedTarget`, `CounterNum$ ReplaceCount$DamageAmount`).
    fn parse_direct_counter_prevention(&mut self, mut rep: Params) -> Option<(String, String)> {
        if !matches!(
            rep.take("Defined").as_deref(),
            Some("Self" | "ReplacedTarget")
        ) {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let Some(kind_raw) = rep.take("CounterType") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(num_var) = rep.take("CounterNum") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(kind) = super::counter_kind(kind_raw.trim()) else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if !rep.exhausted() {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let num_resolved = if num_var.trim() == "ReplaceCount$DamageAmount" {
            Some("ReplaceCount$DamageAmount")
        } else {
            self.svars.get(num_var.trim()).map(String::as_str)
        };
        if num_resolved.map(str::trim) != Some("ReplaceCount$DamageAmount") {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        Some((kind_raw, kind))
    }

    /// Two-step `DB$ ReplaceDamage` (`Amount$ <lhs>/LimitMax.<cap>`) + `SubAbility$ DB$ RemoveCounter`.
    fn parse_capped_counter_prevention(&mut self, mut rep: Params) -> Option<(String, String)> {
        let Some(amt_var) = rep.take("Amount") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        let Some(sub_name) = rep.take("SubAbility") else {
            return self.deny("a persistent spell prevention with another zone or result".into());
        };
        if !rep.exhausted() {
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
        if rem_api != "RemoveCounter"
            || !matches!(
                rem.take("Defined").as_deref(),
                Some("Self" | "ReplacedTarget")
            )
        {
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
        Some((kind_raw, kind))
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

    pub(super) fn prevention_source_filter(&self, valid: &str) -> Option<String> {
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
        let raw_alts: Vec<&str> = normalized.split(',').collect();
        let mut alts = Vec::new();
        for &raw_alt in &raw_alts {
            let alt = raw_alt.trim();
            if raw_alts.len() > 1 && super::targets::emblem_subsumed_by_card(alt, &raw_alts) {
                continue;
            }
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
                } else if atom == "SnowSource" {
                    kept_atoms.push("Snow".to_string());
                } else if matches!(atom, "nonSnowSource" | "!SnowSource") {
                    kept_atoms.push("nonSnow".to_string());
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
    #[allow(clippy::too_many_lines)]
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
        if event == "Counter"
            && bound_spec.is_none()
            && !remember_self
            && let Some(effects) = self.temp_spells_cant_be_countered_shield(shield.clone())
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
        if shield.peek("PreventionEffect") == Some("True")
            && shield
                .peek("ReplaceWith")
                .is_some_and(|rw| !self.exiles_itself(rw))
            && let Some(effects) = self.wave14d_prevention_effect_with_then(
                bound_spec.as_deref(),
                expected_target,
                remember_self,
                chain.target.is_some(),
                shield.clone(),
            )
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
        if remember_chosen {
            if effect.take("RememberObjects").as_deref() != Some("Remembered") {
                return self.deny("a chosen-source shield on another condition".to_string());
            }
        } else if effect.peek("RememberObjects") == Some("Remembered") {
            effect.take("RememberObjects");
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

    /// Parses `Event$ Counter | Layer$ CantHappen` into `(who, filter_ref)` for
    /// Wave 14d `Modifier::SpellsCantBeCountered` (*Prowling Serpopard*,
    /// *Destiny Spinner*, *Allosaurus Shepherd*, *Lier*, *Rhythm of the Wild*,
    /// *Taigam, Ojutai Master*, *Determined*, *Veil of Summer*).
    pub(super) fn spells_cant_be_countered_parts(
        &mut self,
        p: &mut Params,
    ) -> Option<(&'static str, String)> {
        if p.take("Layer").as_deref() != Some("CantHappen") {
            return None;
        }
        let valid_sa = p.take("ValidSA")?;
        let valid_card = p.take("ValidCard");
        let raw_list = if let Some(vc) = valid_card {
            if valid_sa.trim() != "Spell" || vc.trim() == "Card.Self" {
                return None;
            }
            vc
        } else {
            let mut rewritten = Vec::new();
            for alt in valid_sa.split(',').map(str::trim) {
                if alt == "Spell" {
                    rewritten.push("Card".to_string());
                } else {
                    let rest = alt.strip_prefix("Spell.")?;
                    rewritten.push(format!("Card.{rest}"));
                }
            }
            if rewritten.is_empty() {
                return None;
            }
            rewritten.join(",")
        };
        let mut common_who: Option<&'static str> = None;
        let mut cleaned_alts = Vec::new();
        for raw_alt in raw_list.split(',').map(str::trim) {
            if raw_alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = raw_alt.split_once('.').unwrap_or((raw_alt, ""));
            let mut alt_who: Option<&'static str> = None;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                match atom {
                    "YouCtrl" => {
                        if alt_who.is_some() {
                            return None;
                        }
                        alt_who = Some("PlayerRel::You");
                    }
                    "OppCtrl" => {
                        if alt_who.is_some() {
                            return None;
                        }
                        alt_who = Some("PlayerRel::EachOpponent");
                    }
                    other => {
                        if other.contains("Ctrl")
                            || other.contains("Own")
                            || other.contains("wasCast")
                            || other.contains("IsRemembered")
                            || other == "Self"
                        {
                            return None;
                        }
                        kept.push(other);
                    }
                }
            }
            let resolved_who = alt_who.unwrap_or("PlayerRel::EachPlayer");
            match common_who {
                None => common_who = Some(resolved_who),
                Some(prev) if prev != resolved_who => return None,
                Some(_) => {}
            }
            if kept.is_empty() {
                cleaned_alts.push(base.trim().to_string());
            } else {
                cleaned_alts.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        let who = common_who?;
        let cleaned = cleaned_alts.join(",");
        let filter_ref = if cleaned == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&cleaned, true)?;
            match expr.as_str() {
                "Filter::Any" => "&Filter::Any".to_string(),
                "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
                "Filter::INSTANT_OR_SORCERY" => "&Filter::INSTANT_OR_SORCERY".to_string(),
                "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
                "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
                _ => format!("&{}", self.body.filter_static("SPELL", &expr)),
            }
        };
        Some((who, filter_ref))
    }

    fn temp_spells_cant_be_countered_shield(&mut self, mut shield: Params) -> Option<Vec<String>> {
        if !matches!(
            shield.take("ActiveZones").as_deref(),
            None | Some("Command")
        ) {
            return None;
        }
        let (who, filter_ref) = self.spells_cant_be_countered_parts(&mut shield)?;
        if !shield.exhausted() {
            return None;
        }
        Some(vec![format!(
            "Effect::AddGlobalTempModifier {{ filter: &Filter::Any, \
             modifier: Modifier::SpellsCantBeCountered {{ who: {who}, filter: {filter_ref} }}, \
             duration: Duration::UntilEndOfTurn }}"
        )])
    }

    /// Wave 14d `Effect | ReplacementEffects$` prevention shields with a
    /// `ReplaceCount$DamageAmount` rider:
    /// - One-shot (`SubAbility$ ExileEffect`) -> `Effect::PreventNextDamageWithThen`
    ///   (*Awe Strike*).
    /// - Turn-long (no self-exile) -> `Effect::AddGlobalTempModifier` with
    ///   `Modifier::ReplaceOrPreventDamageWithThen` (*Inkshield*).
    #[allow(clippy::too_many_lines)]
    fn wave14d_prevention_effect_with_then(
        &mut self,
        bound_spec: Option<&str>,
        expected_target: Option<&[&str]>,
        remember_self: bool,
        chain_has_target: bool,
        mut shield: Params,
    ) -> Option<Vec<String>> {
        if shield.take("PreventionEffect").as_deref() != Some("True") {
            return None;
        }
        let replace_name = shield.take("ReplaceWith")?;
        let valid_source = shield.take("ValidSource");
        let valid_target = shield.take("ValidTarget");
        let is_combat = shield.take("IsCombat");
        if !matches!(shield.take("AlwaysReplace").as_deref(), None | Some("True"))
            || !matches!(
                shield.take("ExecuteMode").as_deref(),
                None | Some("PerSource")
            )
            || !shield.exhausted()
        {
            return None;
        }
        let rider_target = match valid_target.as_deref() {
            Some("You") => RiderTarget::You,
            Some("Card.Self") => RiderTarget::Source,
            Some("Card.IsRemembered" | "Creature.IsRemembered") if remember_self => {
                RiderTarget::Source
            }
            Some(
                "Card.EquippedBy"
                | "Creature.EquippedBy"
                | "Card.EnchantedBy"
                | "Creature.EnchantedBy",
            ) => RiderTarget::Attached,
            _ => RiderTarget::Other,
        };
        // Shape A: one-shot shield (`SubAbility$ ExileEffect`) -> `Effect::PreventNextDamageWithThen`.
        if let Some(then_eff) =
            self.parse_damage_prevented_rider(replace_name.trim(), true, rider_target)
        {
            let combat_only = match is_combat.as_deref() {
                None => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let (sources_spec, recipients_spec) = match (bound_spec, expected_target) {
                (Some(spec), Some(expected))
                    if spec.starts_with("TargetSpec::Object(")
                        && valid_source
                            .as_deref()
                            .is_some_and(|s| expected.contains(&s)) =>
                {
                    let recip = match valid_target.as_deref() {
                        None => "TargetSpec::AnyTarget".to_string(),
                        Some("You") => "TargetSpec::Player(PlayerRel::You)".to_string(),
                        Some("Player") => "TargetSpec::Player(PlayerRel::EachPlayer)".to_string(),
                        Some("Card.Self" | "Creature.Self") if !self.on_a_spell => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        _ => return None,
                    };
                    (spec.to_string(), recip)
                }
                (Some(spec), Some(expected))
                    if valid_source.is_none()
                        && valid_target
                            .as_deref()
                            .is_some_and(|t| expected.contains(&t)) =>
                {
                    ("TargetSpec::AnyTarget".to_string(), spec.to_string())
                }
                (None, None) => {
                    let src = match valid_source.as_deref() {
                        None => "TargetSpec::AnyTarget".to_string(),
                        Some("Card.IsRemembered" | "Creature.IsRemembered") if remember_self => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        Some("Card.Self" | "Creature.Self") if !self.on_a_spell => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        Some(vs) if !chain_has_target => {
                            let expr = self.prevention_source_filter(vs)?;
                            if expr == "Filter::Any" {
                                "TargetSpec::AnyTarget".to_string()
                            } else {
                                let src_ref = self.body.filter_static("SOURCE", &expr);
                                format!("TargetSpec::Object(&{src_ref})")
                            }
                        }
                        Some(_) => return None,
                    };
                    let recip = match valid_target.as_deref() {
                        None => "TargetSpec::AnyTarget".to_string(),
                        Some("You") => "TargetSpec::Player(PlayerRel::You)".to_string(),
                        Some("Player") => "TargetSpec::Player(PlayerRel::EachPlayer)".to_string(),
                        Some("Card.IsRemembered" | "Creature.IsRemembered") if remember_self => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        Some("Card.Self" | "Creature.Self") if !self.on_a_spell => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        _ => return None,
                    };
                    (src, recip)
                }
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::PreventNextDamageWithThen {{ sources: {sources_spec}, \
                 recipients: {recipients_spec}, combat_only: {combat_only}, \
                 then: DamageRider::new(&[{then_eff}]) }}"
            )]);
        }

        // Shape B: turn-long untargeted shield (no self-exile) ->
        // `Effect::AddGlobalTempModifier` with `Modifier::ReplaceOrPreventDamageWithThen` (*Inkshield*).
        if bound_spec.is_some() || remember_self {
            return None;
        }
        let then_eff =
            self.parse_damage_prevented_rider(replace_name.trim(), false, rider_target)?;
        let combat = match is_combat.as_deref() {
            None => "None",
            Some("True") => "Some(true)",
            Some("False") => "Some(false)",
            Some(_) => return None,
        };
        let source_ref = match valid_source.as_deref() {
            None => "&Filter::Any".to_string(),
            Some(vs) => {
                let expr = self.prevention_source_filter(vs)?;
                if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("SOURCE", &expr))
                }
            }
        };
        let recipients = self.damage_recipients_expr(valid_target.as_deref())?;
        Some(vec![format!(
            "Effect::AddGlobalTempModifier {{ filter: &Filter::Any, \
             modifier: Modifier::ReplaceOrPreventDamageWithThen {{ \
             sources: {source_ref}, recipients: {recipients}, combat: {combat}, \
             is_replacement: false, then: DamageRider::new(&[{then_eff}]) }}, \
             duration: Duration::UntilEndOfTurn }}"
        )])
    }
}
