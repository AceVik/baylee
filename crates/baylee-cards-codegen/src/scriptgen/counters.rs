//! Counter placement, removal, movement, and doubling helpers.

use super::{Chain, Params, Tx, counter_kind};
use std::fmt::Write as _;

impl Tx<'_> {
    fn targets_current_recipient(targets_here: bool, defined: Option<&str>) -> bool {
        (targets_here && defined.is_none())
            || (!targets_here && matches!(defined, Some("Targeted" | "ParentTarget")))
    }

    fn is_self_counter_subject(&self, defined: Option<&str>) -> bool {
        matches!(defined, None | Some("Self"))
            || (matches!(
                defined,
                Some(
                    "TriggeredCardLKICopy" | "TriggeredBlockerLKICopy" | "TriggeredAttackerLKICopy"
                )
            ) && self.trigger_is_self)
    }

    /// Filter naming the triggering permanent (`TriggeredCard*`, `TriggeredAttacker*`,
    /// `TriggeredBlocker*`, `TriggeredTarget*`, `TriggeredSource*`, `DelayTriggerRemembered*`)
    /// when a counter effect has no spell/ability target.
    pub(super) fn triggered_event_filter(&self, def: &str, target: Option<&str>) -> Option<&str> {
        if target.is_some() {
            return None;
        }
        match def {
            "TriggeredCardLKICopy"
            | "TriggeredCard"
            | "TriggeredNewCardLKICopy"
            | "TriggeredNewCard"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZone" | "TurnFaceUp" | "Transformed")
                ) =>
            {
                Some("&Filter::Reference")
            }
            "TriggeredCardLKICopy" | "TriggeredCard"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Exerted" | "Enlisted" | "Taps" | "Untaps" | "TurnFaceUp" | "Transformed")
                ) =>
            {
                Some("&Filter::Reference")
            }
            "TriggeredAttackerLKICopy" | "TriggeredAttacker"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                    && self
                        .block_line
                        .as_ref()
                        .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks)) =>
            {
                Some("&Filter::Reference")
            }
            "TriggeredBlockerLKICopy" | "TriggeredBlocker"
                if self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked)) =>
            {
                Some("&Filter::Reference")
            }
            "TriggeredTargetLKICopy" | "TriggeredTarget"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ) =>
            {
                Some("&Filter::Reference")
            }
            "DelayTriggerRemembered" | "DelayTriggerRememberedLKI" if self.in_delayed => {
                Some("&Filter::Reference")
            }
            "TriggeredSourceLKICopy" | "TriggeredSource"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce" | "DamageDealtOnce")
                ) =>
            {
                if self.trigger_is_self {
                    Some("&Filter::This")
                } else if self.damage_trigger_binds_source {
                    Some("&Filter::Reference")
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Lower `RemoveCounter` (single-node):
    /// - `CounterType$ All` on a single object or player target -> `Effect::RemoveAllCountersTarget`
    /// - `CounterType$ Any | UpTo$ True` on a single object target -> `Effect::RemoveUpToCountersTarget`
    /// - concrete `CounterType$ <K>`:
    ///   - `CounterNum$ All` -> `RemoveAllCountersFilter` (target) or `RemoveAllCountersSelf`
    ///   - `Defined$ Enchanted` -> `ChangeAttachedCounters { remove: true }`
    ///   - `Defined$ Self/None` -> `RemoveCounterSelf`
    pub(super) fn remove_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let code = p.take("CounterType")?;
        if code.eq_ignore_ascii_case("All") {
            if !matches!(p.take("CounterNum").as_deref(), None | Some("All"))
                || p.take("RememberRemoved").is_some()
            {
                return None;
            }
            let defined = p.take("Defined");
            if single_target
                && target.is_some()
                && Self::targets_current_recipient(targets_here, defined.as_deref())
            {
                return Some(vec!["Effect::RemoveAllCountersTarget".to_string()]);
            }
            return None;
        }
        if code.eq_ignore_ascii_case("Any") {
            if p.take("UpTo").as_deref() != Some("True") || p.take("RememberRemoved").is_some() {
                return None;
            }
            let n: u16 = self
                .plain_num(p.take("CounterNum").as_deref().unwrap_or("1"))?
                .try_into()
                .ok()?;
            if n == 0 {
                return None;
            }
            let defined = p.take("Defined");
            if single_target
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                && Self::targets_current_recipient(targets_here, defined.as_deref())
            {
                return Some(vec![format!(
                    "Effect::RemoveUpToCountersTarget {{ maximum: {n} }}"
                )]);
            }
            return None;
        }
        let Some(kind) = counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let all = p.peek("CounterNum") == Some("All");
        let mut remember_removed = false;
        let n: u16 = if all {
            p.take("CounterNum");
            match p.take("RememberRemoved").as_deref() {
                None => {}
                Some("True") => remember_removed = true,
                Some(_) => return None,
            }
            0
        } else {
            let num: u16 = self
                .plain_num(p.take("CounterNum").as_deref().unwrap_or("1"))?
                .try_into()
                .ok()?;
            if num == 0 {
                return None;
            }
            num
        };
        let defined = p.take("Defined");
        if all
            && single_target
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && Self::targets_current_recipient(targets_here, defined.as_deref())
        {
            self.chain_prev_removed_counters = remember_removed;
            return Some(vec![format!(
                "Effect::RemoveAllCountersFilter {{ filter: &Filter::Reference, kind: {kind} }}"
            )]);
        }
        if !all && defined.as_deref() == Some("Enchanted") && target.is_none() && !self.on_a_spell {
            return Some(vec![format!(
                "Effect::ChangeAttachedCounters {{ kind: {kind}, amount: Amount::Fixed({n}), remove: true }}"
            )]);
        }
        if target.is_none() && self.is_self_counter_subject(defined.as_deref()) {
            return Some(vec![if all {
                format!("Effect::RemoveAllCountersSelf {{ kind: {kind} }}")
            } else {
                format!("Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}")
            }]);
        }
        None
    }

    /// Lower `RemoveCounter | RememberRemoved$ True | SubAbility$ ...` where child
    /// nodes check `Count$RememberedSize` / `Remembered$Amount` into `RemoveCounterSelf`
    /// followed by `Effect::IfCondition` on `ThisWay::CountersRemoved(kind)`.
    pub(super) fn remove_counter_remembered_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if targets_here || target.is_some() || p.take("RememberRemoved").as_deref() != Some("True")
        {
            return None;
        }
        let defined = p.take("Defined");
        if !self.is_self_counter_subject(defined.as_deref()) {
            return None;
        }
        let code = p.take("CounterType")?;
        if code.eq_ignore_ascii_case("All") || code.eq_ignore_ascii_case("Any") {
            return None;
        }
        let kind = counter_kind(&code)?;
        let n: u16 = self
            .plain_num(p.take("CounterNum").as_deref().unwrap_or("1"))?
            .try_into()
            .ok()?;
        if n == 0 || !p.exhausted() {
            return None;
        }
        let mut out = vec![format!(
            "Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}"
        )];
        let mut saw_gate = false;
        while let Some(child_name) = sub.take() {
            let child_body = self.svars.get(child_name.trim())?.clone();
            let (child_api, mut cp) = Params::parse(&child_body)?;
            if child_api == "Cleanup" {
                cp.drop_prose();
                if cp.take("ClearRemembered").as_deref() != Some("True") {
                    return None;
                }
                let next_sub = cp.take("SubAbility");
                if !cp.exhausted() {
                    return None;
                }
                *sub = next_sub;
                continue;
            }
            if let Some(var) = cp.peek("ConditionCheckSVar").and_then(|var| {
                let def = self.svars.get(var.trim())?.trim();
                matches!(def, "Count$RememberedSize" | "Remembered$Amount").then(|| var.to_string())
            }) {
                let _ = var;
                let (cond, next_sub, effs) =
                    self.eval_remembered_removed_child(&child_api, cp, &kind, n, chain)?;
                out.push(format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                    effs.join(", ")
                ));
                saw_gate = true;
                *sub = next_sub;
            } else if !child_body.contains("Remembered") {
                *sub = Some(child_name);
                break;
            } else {
                return None;
            }
        }
        saw_gate.then_some(out)
    }

    fn eval_remembered_removed_child(
        &mut self,
        child_api: &str,
        mut cp: Params,
        kind: &str,
        removed_max: u16,
        chain: &mut Chain,
    ) -> Option<(String, Option<String>, Vec<String>)> {
        if cp.has("ConditionPresent") || cp.has("ConditionDefined") || cp.has("Condition") {
            return None;
        }
        cp.take("ConditionCheckSVar");
        let cmp = cp.take("ConditionSVarCompare");
        let threshold = match cmp.as_deref() {
            None | Some("GE1" | "GT0") => 1u32,
            Some("EQ1") if removed_max == 1 => 1u32,
            Some("EQ0" | "LE0" | "LT1") => 0u32,
            Some(c) if c.starts_with("GE") => c[2..]
                .parse::<u32>()
                .ok()
                .filter(|&k| k >= 1 && k <= u32::from(removed_max))?,
            _ => return None,
        };
        let cond = if threshold == 0 {
            format!(
                "Condition::Not(&Condition::AmountAtLeast(Amount::ThisWay(ThisWay::CountersRemoved({kind})), 1))"
            )
        } else {
            format!(
                "Condition::AmountAtLeast(Amount::ThisWay(ThisWay::CountersRemoved({kind})), {threshold})"
            )
        };
        let next_sub = cp.take("SubAbility");
        let single_spec = cp.to_node_spec(child_api);
        let mut child_chain = Chain {
            target: chain.target.clone(),
            target_is_x: chain.target_is_x,
            target_up_to_x: chain.target_up_to_x,
            target_up_to: chain.target_up_to,
            target_exact: chain.target_exact,
            target_range: chain.target_range,
            target_unique: chain.target_unique,
            target_constraint: chain.target_constraint,
            second_target: chain.second_target.clone(),
            second_target_is_x: chain.second_target_is_x,
            second_target_up_to: chain.second_target_up_to,
            second_target_range: chain.second_target_range,
            second_target_constraint: chain.second_target_constraint,
            third_target: chain.third_target.clone(),
            third_target_up_to: chain.third_target_up_to,
            third_target_range: chain.third_target_range,
            third_target_constraint: chain.third_target_constraint,
            ..Chain::default()
        };
        self.chain_inner(&single_spec, &mut child_chain)?;
        if child_chain.effects.is_empty() {
            return None;
        }
        chain.target = child_chain.target;
        chain.target_is_x = child_chain.target_is_x;
        chain.target_up_to_x = child_chain.target_up_to_x;
        chain.target_up_to = child_chain.target_up_to;
        chain.target_exact = child_chain.target_exact;
        chain.target_range = child_chain.target_range;
        chain.target_unique = child_chain.target_unique;
        chain.target_constraint = child_chain.target_constraint;
        chain.second_target = child_chain.second_target;
        chain.second_target_is_x = child_chain.second_target_is_x;
        chain.second_target_up_to = child_chain.second_target_up_to;
        chain.second_target_range = child_chain.second_target_range;
        chain.second_target_constraint = child_chain.second_target_constraint;
        chain.third_target = child_chain.third_target;
        chain.third_target_up_to = child_chain.third_target_up_to;
        chain.third_target_range = child_chain.third_target_range;
        chain.third_target_constraint = child_chain.third_target_constraint;
        chain.could_add_mana |= child_chain.could_add_mana;
        chain.moves_library |= child_chain.moves_library;
        Some((cond, next_sub, child_chain.effects))
    }

    pub(super) fn multiply_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if !matches!(
            p.take("Multiplier").as_deref().map(str::trim),
            None | Some("2")
        ) {
            return None;
        }
        let code = p.take("CounterType")?;
        let Some(kind) = counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let is_obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let defined = p.take("Defined");
        if is_obj_target
            && !single_target
            && Self::targets_current_recipient(targets_here, defined.as_deref())
        {
            return Some(vec![format!(
                "Effect::RememberTargetsThen {{ then: &[Effect::DoubleCountersFilter {{ filter: &Filter::IsRemembered, kind: {kind} }}] }}"
            )]);
        }
        let is_single_obj = single_target && is_obj_target;
        let filter = match defined.as_deref() {
            None if targets_here && is_single_obj => "&Filter::Reference".to_string(),
            Some("Targeted" | "ParentTarget") if !targets_here && is_single_obj => {
                "&Filter::Reference".to_string()
            }
            Some("Self") if !self.on_a_spell && !targets_here => "&Filter::This".to_string(),
            Some("Enchanted" | "Equipped") if !self.on_a_spell && !targets_here => {
                "&Filter::AttachedToBySource".to_string()
            }
            Some(
                def @ ("TriggeredCardLKICopy"
                | "TriggeredCard"
                | "TriggeredNewCardLKICopy"
                | "TriggeredNewCard"
                | "TriggeredAttackerLKICopy"
                | "TriggeredAttacker"
                | "TriggeredBlockerLKICopy"
                | "TriggeredBlocker"
                | "TriggeredTargetLKICopy"
                | "TriggeredTarget"
                | "TriggeredSourceLKICopy"
                | "TriggeredSource"
                | "DelayTriggerRemembered"
                | "DelayTriggerRememberedLKI"),
            ) if !targets_here => self.triggered_event_filter(def, target)?.to_string(),
            Some("Remembered") if self.capture_scope.is_some() && !targets_here => {
                "&Filter::IsRemembered".to_string()
            }
            Some(def) if !targets_here && (target.is_none() || prior_target_used) => {
                let valid = def.strip_prefix("Valid ")?.trim();
                let expr = self.filter_expr(valid)?;
                let name = self.body.filter_static("DOUBLE", &expr);
                format!("&{name}")
            }
            _ => return None,
        };
        Some(vec![format!(
            "Effect::DoubleCountersFilter {{ filter: {filter}, kind: {kind} }}"
        )])
    }
}

impl Params {
    /// Re-serializes remaining unconsumed parameters into a synthetic `DB$` spec.
    pub(super) fn to_node_spec(&self, api: &str) -> String {
        let mut out = format!("DB$ {api}");
        for (k, v) in &self.entries {
            let _ = write!(out, " | {k}$ {v}");
        }
        out
    }
}
