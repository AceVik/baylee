//! Counter placement, removal, movement, and doubling helpers.

use super::{Chain, Params, Tx, chains::LineFlags, counter_kind};
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
    /// - Legacy `RemoveAllCountersTarget`, `RemoveUpToCountersTarget`,
    ///   `RemoveAllCountersFilter`, `ChangeAttachedCounters`, `RemoveCounterSelf`,
    ///   `RemoveAllCountersSelf` preserved when applicable
    /// - Generalized `Effect::RemoveCounters` for any/all counter kinds, dynamic counts,
    ///   player counters, `AnyNumber`, and single-permanent `Choices$`
    #[allow(clippy::too_many_lines)]
    pub(super) fn remove_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if p.has("Choices") {
            return self.remove_counter_choice_effect(p, target, targets_here);
        }
        let code = p.take("CounterType")?;
        let up_to = match p.take("UpTo").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if p.has("RememberRemoved") || p.has("RememberAmount") {
            // Handled via `remove_counter_remembered_chain` / `counter_remembered_amount_chain`
            // except legacy `CounterNum$ All` on targets.
            if code.eq_ignore_ascii_case("All")
                || code.eq_ignore_ascii_case("Any")
                || p.peek("CounterNum") != Some("All")
            {
                return None;
            }
        }
        if code.eq_ignore_ascii_case("All") {
            if !matches!(p.peek("CounterNum"), None | Some("All")) || up_to {
                return None;
            }
            p.take("CounterNum");
            let defined = p.take("Defined");
            if single_target
                && target.is_some()
                && Self::targets_current_recipient(targets_here, defined.as_deref())
            {
                return Some(vec!["Effect::RemoveAllCountersTarget".to_string()]);
            }
            if !targets_here
                && target.is_none()
                && let Some(who) = defined
                    .as_deref()
                    .and_then(|d| self.wave11_player_of_line(Some(d), None, false))
            {
                return Some(vec![format!(
                    "Effect::RemoveCounters {{ subject: RemoveCounterSubject::Player({who}), kind: None, count: RemoveCounterCount::All }}"
                )]);
            }
            return None;
        }
        if code.eq_ignore_ascii_case("Any") {
            let num_raw = p.take("CounterNum").unwrap_or_else(|| "1".to_string());
            if up_to
                && let Some(n) = self.plain_num(&num_raw).and_then(|v| u16::try_from(v).ok())
                && n > 0
                && single_target
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                && Self::targets_current_recipient(targets_here, p.peek("Defined"))
            {
                p.take("Defined");
                return Some(vec![format!(
                    "Effect::RemoveUpToCountersTarget {{ maximum: {n} }}"
                )]);
            }
            let count = self.parse_remove_counter_count(&num_raw, up_to, true)?;
            let subject = self.resolve_remove_counter_subject(
                p.take("Defined").as_deref(),
                target,
                targets_here,
            )?;
            return Some(vec![format!(
                "Effect::RemoveCounters {{ subject: {subject}, kind: None, count: {count} }}"
            )]);
        }
        let Some(kind) = counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let all = p.peek("CounterNum") == Some("All");
        let mut remember_removed = false;
        let num_raw = p.take("CounterNum").unwrap_or_else(|| "1".to_string());
        if all {
            match p.take("RememberRemoved").as_deref() {
                None => {}
                Some("True") => remember_removed = true,
                Some(_) => return None,
            }
        }
        let defined = p.take("Defined");
        if all
            && !up_to
            && single_target
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && Self::targets_current_recipient(targets_here, defined.as_deref())
        {
            self.chain_prev_removed_counters = remember_removed;
            return Some(vec![format!(
                "Effect::RemoveAllCountersFilter {{ filter: &Filter::Reference, kind: {kind} }}"
            )]);
        }
        if let Some(n) = self.plain_num(&num_raw).and_then(|v| u16::try_from(v).ok())
            && n > 0
            && !all
            && !up_to
            && !remember_removed
        {
            if defined.as_deref() == Some("Enchanted") && target.is_none() && !self.on_a_spell {
                return Some(vec![format!(
                    "Effect::ChangeAttachedCounters {{ kind: {kind}, amount: Amount::Fixed({n}), remove: true }}"
                )]);
            }
            if target.is_none() && self.is_self_counter_subject(defined.as_deref()) {
                return Some(vec![format!(
                    "Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}"
                )]);
            }
        }
        if all && !up_to && target.is_none() && self.is_self_counter_subject(defined.as_deref()) {
            return Some(vec![format!(
                "Effect::RemoveAllCountersSelf {{ kind: {kind} }}"
            )]);
        }
        if remember_removed {
            return None;
        }
        let count = self.parse_remove_counter_count(&num_raw, up_to, false)?;
        let subject =
            self.resolve_remove_counter_subject(defined.as_deref(), target, targets_here)?;
        Some(vec![format!(
            "Effect::RemoveCounters {{ subject: {subject}, kind: Some({kind}), count: {count} }}"
        )])
    }

    fn remove_counter_choice_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || target.is_some()
            || p.has("Defined")
            || p.has("ChoiceNum")
            || p.has("ChoiceZone")
            || p.has("RememberRemoved")
            || p.has("RememberAmount")
        {
            return None;
        }
        // `RemoveCounterSubject::Choice` picks at most one permanent; refuse multi-permanent wording.
        let lower = self.oracle.to_ascii_lowercase();
        if lower.contains("among ")
            || lower.contains("each of ")
            || lower.contains("any number of permanents")
        {
            return None;
        }
        let choices = p.take("Choices")?;
        let optional_choice = match p.take("ChoiceOptional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let code = p.take("CounterType")?;
        let kind_expr = if code.eq_ignore_ascii_case("All") || code.eq_ignore_ascii_case("Any") {
            "None".to_string()
        } else {
            let Some(kind) = counter_kind(&code) else {
                return self.deny(format!("counter `{code}`"));
            };
            format!("Some({kind})")
        };
        let up_to = match p.take("UpTo").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let num_raw = p.take("CounterNum").unwrap_or_else(|| "1".to_string());
        let count = self.parse_remove_counter_count(&num_raw, up_to, kind_expr == "None")?;
        let expr = self.filter_expr(&choices)?;
        let filter = self.body.filter_static("REM_CHOICE", &expr);
        let eff = format!(
            "Effect::RemoveCounters {{ subject: RemoveCounterSubject::Choice(&{filter}), kind: {kind_expr}, count: {count} }}"
        );
        Some(if optional_choice {
            vec![format!("Effect::MayDo {{ effects: &[{eff}] }}")]
        } else {
            vec![eff]
        })
    }

    /// Generalized `RemoveCounterAll` for battlefield sweeps (`Aether Snap`, *Dust of Moments*, *Heartmender*).
    pub(super) fn remove_counter_all_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || (target.is_some() && !prior_target_used)
            || !matches!(p.take("ValidZone").as_deref(), None | Some("Battlefield"))
        {
            return None;
        }
        let all_types = match p.take("AllCounterTypes").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let kind_expr = if all_types {
            if p.has("CounterType") || p.has("CounterNum") {
                return None;
            }
            "None".to_string()
        } else {
            let code = p.take("CounterType")?;
            let Some(kind) = counter_kind(&code) else {
                return self.deny(format!("counter `{code}`"));
            };
            format!("Some({kind})")
        };
        let count = if all_types
            || p.peek("CounterNum") == Some("All")
            || p.peek("AllCounters") == Some("True")
        {
            p.take("CounterNum");
            p.take("AllCounters");
            "RemoveCounterCount::All".to_string()
        } else {
            let num_raw = p.take("CounterNum")?;
            let amt = self.amount_or_count_ext(&num_raw)?;
            format!("RemoveCounterCount::Exact({amt})")
        };
        let valid = p.take("ValidCards")?;
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("COUNTERS", &expr);
        Some(vec![format!(
            "Effect::RemoveCounters {{ subject: RemoveCounterSubject::Filter(&{filter}), kind: {kind_expr}, count: {count} }}"
        )])
    }

    fn parse_remove_counter_count(
        &mut self,
        raw: &str,
        up_to: bool,
        is_any_kind: bool,
    ) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed == "All" {
            return (!up_to).then(|| "RemoveCounterCount::All".to_string());
        }
        if trimmed == "Any" {
            return (!up_to).then(|| "RemoveCounterCount::AnyNumber".to_string());
        }
        let amt = self.amount_or_count_ext(trimmed)?;
        if amt == "Amount::Fixed(0)" {
            return None;
        }
        let oracle_upto = is_any_kind && self.oracle.to_ascii_lowercase().contains("up to ");
        Some(if up_to || oracle_upto {
            format!("RemoveCounterCount::UpTo({amt})")
        } else {
            format!("RemoveCounterCount::Exact({amt})")
        })
    }

    fn resolve_remove_counter_subject(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<String> {
        if target.is_some() && Self::targets_current_recipient(targets_here, defined) {
            return Some("RemoveCounterSubject::Targets".to_string());
        }
        if targets_here {
            return None;
        }
        match defined {
            Some("Enchanted" | "Equipped") if !self.on_a_spell => {
                Some("RemoveCounterSubject::Attached".to_string())
            }
            def if target.is_none() && self.is_self_counter_subject(def) => {
                Some("RemoveCounterSubject::Source".to_string())
            }
            Some(def) if target.is_none() => self
                .wave11_player_of_line(Some(def), None, false)
                .map(|who| format!("RemoveCounterSubject::Player({who})")),
            _ => None,
        }
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

    /// Lower `RemoveCounter | RememberRemoved$/RememberAmount$` or
    /// `MoveCounter | RememberPut$/RememberAmount$` followed by child steps
    /// reading `Count$RememberedSize` / `Remembered$Amount` / `Count$RememberedNumber`
    /// and closing cleanly at `DB$ Cleanup | ClearRemembered$ True`.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn counter_remembered_amount_chain(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        second_target_here: bool,
        exact_two: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        let flag_key = match api {
            "RemoveCounter" => {
                if probe.take("RememberRemoved").as_deref() == Some("True") {
                    if probe.has("RememberAmount")
                        && probe.take("RememberAmount").as_deref() != Some("True")
                    {
                        return None;
                    }
                    "RememberRemoved"
                } else if probe.take("RememberAmount").as_deref() == Some("True") {
                    "RememberAmount"
                } else {
                    return None;
                }
            }
            "MoveCounter" => {
                if probe.take("RememberPut").as_deref() == Some("True") {
                    if probe.has("RememberAmount")
                        && probe.take("RememberAmount").as_deref() != Some("True")
                    {
                        return None;
                    }
                    "RememberPut"
                } else if probe.take("RememberAmount").as_deref() == Some("True") {
                    "RememberAmount"
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        let _ = flag_key;
        let this_way_amt = if api == "MoveCounter" {
            "Amount::ThisWay(ThisWay::CountersMoved)"
        } else {
            "Amount::ThisWay(ThisWay::CountersRemovedAny)"
        };
        let first_effs = if api == "MoveCounter" {
            self.move_counter_effect(
                &mut probe,
                target,
                flags.targets_here,
                second_target_here,
                exact_two,
            )?
        } else {
            self.remove_counter_effect(&mut probe, target, flags.targets_here, flags.single_target)?
        };
        if !probe.exhausted() {
            return None;
        }

        let mut out = first_effs;
        let mut saw_cleanup = false;
        let mut saw_consumer = false;
        let saved_amt = self.chain_remembered_amount_expr.replace(this_way_amt);
        let mut cursor = sub.take();
        let ok = (|| -> Option<()> {
            while let Some(child_name) = cursor.take() {
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
                    saw_cleanup = true;
                    *sub = next_sub;
                    break;
                }
                if cp.has("ConditionPresent") || cp.has("ConditionDefined") || cp.has("Condition") {
                    return None;
                }
                let cond_opt = if let Some(var) = cp.peek("ConditionCheckSVar") {
                    let def = self.svars.get(var.trim())?.trim().to_string();
                    if !matches!(
                        def.as_str(),
                        "Count$RememberedSize" | "Remembered$Amount" | "Count$RememberedNumber"
                    ) {
                        return None;
                    }
                    cp.take("ConditionCheckSVar");
                    let cmp = cp.take("ConditionSVarCompare");
                    let threshold: u32 = match cmp.as_deref() {
                        None | Some("GE1" | "GT0") => 1,
                        Some("EQ0" | "LE0" | "LT1") => 0,
                        Some(c) if c.starts_with("GE") => c[2..].parse().ok()?,
                        _ => return None,
                    };
                    saw_consumer = true;
                    Some(if threshold == 0 {
                        format!("Condition::Not(&Condition::AmountAtLeast({this_way_amt}, 1))")
                    } else {
                        format!("Condition::AmountAtLeast({this_way_amt}, {threshold})")
                    })
                } else {
                    None
                };
                if child_body.contains("Remembered") {
                    saw_consumer = true;
                } else if cond_opt.is_none() {
                    *sub = Some(child_name);
                    break;
                }
                let next_sub = cp.take("SubAbility");
                let single_spec = cp.to_node_spec(&child_api);
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
                if let Some(cond) = cond_opt {
                    out.push(format!(
                        "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                        child_chain.effects.join(", ")
                    ));
                } else {
                    out.extend(child_chain.effects);
                }
                cursor = next_sub;
            }
            Some(())
        })();
        self.chain_remembered_amount_expr = saved_amt;
        ok?;
        if !(saw_consumer && (saw_cleanup || self.on_a_spell)) {
            return None;
        }
        *p = probe;
        Some(out)
    }

    pub(super) fn multiply_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        let factor: u16 = match p.take("Multiplier").as_deref().map(str::trim) {
            None => 2,
            Some(raw) => raw.parse::<u16>().ok().filter(|&n| n >= 2)?,
        };
        let code_opt = p.take("CounterType");
        let kind_opt = match code_opt.as_deref() {
            None => None,
            Some(c) if c.eq_ignore_ascii_case("Any") || c.eq_ignore_ascii_case("All") => None,
            Some(c) => {
                let Some(k) = counter_kind(c) else {
                    return self.deny(format!("counter `{c}`"));
                };
                Some(k)
            }
        };
        let is_obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let defined = p.take("Defined");
        if let Some(kind) = &kind_opt
            && factor == 2
        {
            if is_obj_target
                && !single_target
                && Self::targets_current_recipient(targets_here, defined.as_deref())
            {
                return Some(vec![format!(
                    "Effect::RememberTargetsThen {{ then: &[Effect::DoubleCountersFilter {{ filter: &Filter::IsRemembered, kind: {kind} }}] }}"
                )]);
            }
            let is_single_obj = single_target && is_obj_target;
            let legacy_filter = match defined.as_deref() {
                None if targets_here && is_single_obj => Some("&Filter::Reference".to_string()),
                Some("Targeted" | "ParentTarget") if !targets_here && is_single_obj => {
                    Some("&Filter::Reference".to_string())
                }
                Some("Self") if !self.on_a_spell && !targets_here => {
                    Some("&Filter::This".to_string())
                }
                Some("Enchanted" | "Equipped") if !self.on_a_spell && !targets_here => {
                    Some("&Filter::AttachedToBySource".to_string())
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
                ) if !targets_here => Some(self.triggered_event_filter(def, target)?.to_string()),
                Some("Remembered") if self.capture_scope.is_some() && !targets_here => {
                    Some("&Filter::IsRemembered".to_string())
                }
                Some(def) if !targets_here && (target.is_none() || prior_target_used) => {
                    let valid = def.strip_prefix("Valid ")?.trim();
                    let expr = self.filter_expr(valid)?;
                    let name = self.body.filter_static("DOUBLE", &expr);
                    Some(format!("&{name}"))
                }
                _ => None,
            };
            if let Some(filter) = legacy_filter {
                return Some(vec![format!(
                    "Effect::DoubleCountersFilter {{ filter: {filter}, kind: {kind} }}"
                )]);
            }
        }
        let subject =
            if is_obj_target && Self::targets_current_recipient(targets_here, defined.as_deref()) {
                "CounterSubject::Targets".to_string()
            } else if targets_here {
                return None;
            } else {
                match defined.as_deref() {
                    Some("Self") if !self.on_a_spell => "CounterSubject::Source".to_string(),
                    Some("Enchanted" | "Equipped") if !self.on_a_spell => {
                        "CounterSubject::Attached".to_string()
                    }
                    Some(def)
                        if !self.on_a_spell
                            && self.is_event_object_defined(def)
                            && target.is_none() =>
                    {
                        "CounterSubject::EventObject".to_string()
                    }
                    Some(def) if target.is_none() => {
                        let who = self.wave11_player_of_line(Some(def), None, false)?;
                        format!("CounterSubject::Player({who})")
                    }
                    _ => return None,
                }
            };
        let kind_expr = kind_opt.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
        Some(vec![format!(
            "Effect::MultiplyCounters {{ subject: {subject}, kind: {kind_expr}, factor: {factor} }}"
        )])
    }

    /// Lower `MoveCounter` (legacy 1-counter `Effect::MoveCounter` / `MoveCounterToEvent`
    /// plus generalized `Effect::MoveCounters`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn move_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        second_target_here: bool,
        exact_two: bool,
    ) -> Option<Vec<String>> {
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let wrap = |eff: String| {
            if optional {
                format!("Effect::MayDo {{ effects: &[{eff}] }}")
            } else {
                eff
            }
        };
        p.take("IsCurse");
        let raw_num = p.take("CounterNum");
        let raw_kind = p.take("CounterType")?;
        let each_not_on = raw_kind.eq_ignore_ascii_case("EachNotOn");
        let non_zero = match p.take("NonZero").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let valid_source = p.take("ValidSource");
        let valid_defined = p.take("ValidDefined");
        let source = p.take("Source");
        let defined = p.take("Defined");

        let is_legacy_single = !each_not_on
            && !non_zero
            && !exact_two
            && valid_source.is_none()
            && valid_defined.is_none()
            && matches!(raw_num.as_deref(), None | Some("1"));
        if is_legacy_single {
            let kind_opt = if raw_kind.eq_ignore_ascii_case("Any") {
                None
            } else {
                Some(super::counter_kind(&raw_kind)?)
            };
            if second_target_here {
                if source.as_deref() != Some("ParentTarget")
                    || !matches!(defined.as_deref(), None | Some("Targeted"))
                    || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                {
                    return None;
                }
                let kind_expr =
                    kind_opt.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
                return Some(vec![wrap(format!(
                    "Effect::MoveCounter {{ from: TargetSlot::First, to: TargetSlot::Second, kind: {kind_expr} }}"
                ))]);
            }
            if source.as_deref() == Some("Self")
                && !self.on_a_spell
                && !targets_here
                && target.is_none()
                && defined
                    .as_deref()
                    .is_some_and(|d| self.is_event_object_defined(d))
            {
                let kind = kind_opt?;
                return Some(vec![wrap(format!(
                    "Effect::MoveCounterToEvent {{ kind: {kind} }}"
                ))]);
            }
            let is_obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
            let legacy_pair = match (source.as_deref(), defined.as_deref()) {
                (Some("Self"), None | Some("Targeted"))
                    if !self.on_a_spell && targets_here && is_obj_target =>
                {
                    Some(("TargetSlot::This", "TargetSlot::First"))
                }
                (Some("Self"), Some("Targeted" | "ParentTarget"))
                    if !self.on_a_spell && !targets_here && is_obj_target =>
                {
                    Some(("TargetSlot::This", "TargetSlot::First"))
                }
                (None | Some("Targeted"), Some("Self"))
                    if !self.on_a_spell && targets_here && is_obj_target =>
                {
                    Some(("TargetSlot::First", "TargetSlot::This"))
                }
                (Some("Targeted" | "ParentTarget"), Some("Self"))
                    if !self.on_a_spell && !targets_here && is_obj_target =>
                {
                    Some(("TargetSlot::First", "TargetSlot::This"))
                }
                _ => None,
            };
            if let Some((from, to)) = legacy_pair {
                let kind_expr =
                    kind_opt.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
                return Some(vec![wrap(format!(
                    "Effect::MoveCounter {{ from: {from}, to: {to}, kind: {kind_expr} }}"
                ))]);
            }
        }

        let (kind_expr, count_expr) = if each_not_on {
            if !matches!(raw_num.as_deref(), None | Some("1")) || non_zero {
                return None;
            }
            (
                "None".to_string(),
                "MoveCounterCount::OneOfEachKindNotOnDest".to_string(),
            )
        } else {
            let k_expr =
                if raw_kind.eq_ignore_ascii_case("Any") || raw_kind.eq_ignore_ascii_case("All") {
                    "None".to_string()
                } else {
                    format!("Some({})", counter_kind(&raw_kind)?)
                };
            let c_expr = match raw_num.as_deref().map(str::trim) {
                Some("All") if !non_zero => "MoveCounterCount::All".to_string(),
                Some("Any") => format!("MoveCounterCount::AnyNumber {{ min_one: {non_zero} }}"),
                None => {
                    if non_zero {
                        return None;
                    }
                    "MoveCounterCount::Exact(1)".to_string()
                }
                Some(raw) => {
                    if non_zero {
                        return None;
                    }
                    let n = self.plain_num(raw).and_then(|v| u16::try_from(v).ok())?;
                    if n == 0 {
                        return None;
                    }
                    format!("MoveCounterCount::Exact({n})")
                }
            };
            (k_expr, c_expr)
        };

        let is_obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let from_expr = if let Some(vs) = valid_source {
            if source.is_some() {
                return None;
            }
            let expr = self.filter_expr(&vs)?;
            let filter = self.body.filter_static("MOVE_SRC", &expr);
            format!("MoveCounterSource::Filter(&{filter})")
        } else if exact_two {
            if !targets_here || !is_obj_target || source.is_some() {
                return None;
            }
            "MoveCounterSource::FirstTarget".to_string()
        } else if second_target_here {
            if source.as_deref() != Some("ParentTarget") || !is_obj_target {
                return None;
            }
            "MoveCounterSource::Slot(TargetSlot::First)".to_string()
        } else {
            match source.as_deref() {
                Some("Self") if !self.on_a_spell => "MoveCounterSource::Source".to_string(),
                None | Some("Targeted") if targets_here && is_obj_target => {
                    "MoveCounterSource::FirstTarget".to_string()
                }
                Some("Targeted" | "ParentTarget") if !targets_here && is_obj_target => {
                    "MoveCounterSource::FirstTarget".to_string()
                }
                _ => return None,
            }
        };

        let to_expr = if let Some(vd) = valid_defined {
            if defined.is_some() || targets_here {
                return None;
            }
            let expr = self.filter_expr(&vd)?;
            let filter = self.body.filter_static("MOVE_DST", &expr);
            format!("MoveCounterDest::DistributeAmong(&{filter})")
        } else if exact_two {
            if defined.is_some() {
                return None;
            }
            "MoveCounterDest::SecondTarget".to_string()
        } else if second_target_here {
            if !matches!(defined.as_deref(), None | Some("Targeted")) {
                return None;
            }
            "MoveCounterDest::Slot(TargetSlot::Second)".to_string()
        } else {
            match defined.as_deref() {
                Some("Self") if !self.on_a_spell => "MoveCounterDest::Source".to_string(),
                Some("Enchanted" | "Equipped") if !self.on_a_spell && !targets_here => {
                    "MoveCounterDest::Attached".to_string()
                }
                None | Some("Targeted") if targets_here && is_obj_target => {
                    "MoveCounterDest::FirstTarget".to_string()
                }
                Some("Targeted" | "ParentTarget") if !targets_here && is_obj_target => {
                    "MoveCounterDest::FirstTarget".to_string()
                }
                Some(def)
                    if !self.on_a_spell
                        && !targets_here
                        && target.is_none()
                        && self.is_event_object_defined(def) =>
                {
                    "MoveCounterDest::EventObject".to_string()
                }
                _ => return None,
            }
        };

        Some(vec![wrap(format!(
            "Effect::MoveCounters {{ from: {from_expr}, to: {to_expr}, kind: {kind_expr}, count: {count_expr} }}"
        ))])
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
