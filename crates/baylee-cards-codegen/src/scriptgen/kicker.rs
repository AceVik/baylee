//! Reading kicker branches, kicked-only targets, and kicked target replacements.

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Two consecutive lines of the same API whose `TargetMin$`/`TargetMax$`
    /// flip between `Count$Kicked.0.1` and `Count$Kicked.1.0` and whose other
    /// non-prose parameters are identical: a spell whose kicker replaces its
    /// target requirement (`FaceDef::kicked_targets`, CR 702.33a).
    ///
    /// Returns `Some(child_sub)` when matched (having consumed `ValidTgts$`,
    /// `TargetMin$`, `TargetMax$`, `TgtZone$`, and `SubAbility$` on both lines,
    /// and having populated `chain.target` and `self.body.kicked_targets`).
    /// Returns `None` without modifying `p` when the shape does not apply.
    pub(super) fn try_kicked_target_replacement(
        &mut self,
        api: &str,
        p: &mut Params,
        chain: &mut Chain,
    ) -> Option<Option<String>> {
        if !self.on_a_spell
            || !self.has_single_kicker
            || chain.target.is_some()
            || self.body.kicked_targets.is_some()
        {
            return None;
        }
        let valid1 = p.peek("ValidTgts")?.to_string();
        let min1 = p.peek("TargetMin")?;
        let max1 = p.peek("TargetMax")?;
        if min1 != max1 || self.kicked_pair(min1) != Some((0, 1)) {
            return None;
        }
        let sub_name = p.peek("SubAbility")?.to_string();
        let child_body = self.svars.get(&sub_name)?.clone();
        let (child_api, mut cp) = Params::parse(&child_body)?;
        if child_api != api {
            return None;
        }
        cp.drop_prose();
        let valid2 = cp.peek("ValidTgts")?.to_string();
        let min2 = cp.peek("TargetMin")?;
        let max2 = cp.peek("TargetMax")?;
        if min2 != max2 || self.kicked_pair(min2) != Some((1, 0)) {
            return None;
        }
        for forbidden in [
            "UnlessCost",
            "ReplaceDyingDefined",
            "AtEOT",
            "TargetUnique",
            "TargetsWithDefinedController",
            "DividedAsYouChoose",
        ] {
            if p.has(forbidden) || cp.has(forbidden) {
                return None;
            }
        }
        if let Some(cond) = cp.peek("Condition")
            && cond != "Kicked"
        {
            return None;
        }
        let mut p_rest = p.clone();
        for key in ["ValidTgts", "TargetMin", "TargetMax", "SubAbility"] {
            p_rest.take(key);
        }
        let mut cp_rest = cp.clone();
        for key in ["ValidTgts", "TargetMin", "TargetMax", "SubAbility"] {
            cp_rest.take(key);
        }
        if cp_rest.peek("Condition") == Some("Kicked") {
            cp_rest.take("Condition");
            cp_rest.take("ConditionDescription");
        }
        p_rest.entries.sort();
        cp_rest.entries.sort();
        if p_rest.entries != cp_rest.entries {
            return None;
        }
        let mut p_spec1 = p.clone();
        p_spec1.take("ValidTgts");
        let spec1 = self.resolve_target_spec(api, &mut p_spec1, &valid1, chain)?;
        let mut cp_spec2 = cp.clone();
        cp_spec2.take("ValidTgts");
        let spec2 = self.resolve_target_spec(api, &mut cp_spec2, &valid2, chain)?;
        if !Self::same_target_domain(&spec1, &spec2) {
            return None;
        }
        p.take("ValidTgts");
        p.take("TargetMin");
        p.take("TargetMax");
        p.take("SubAbility");
        p.take("TgtZone");
        let child_sub = cp.take("SubAbility");
        chain.target = Some(spec1);
        self.body.kicked_targets = Some(format!("Some(TargetReq::one({spec2}))"));
        Some(child_sub)
    }

    fn same_target_domain(a: &str, b: &str) -> bool {
        let kind = |s: &str| {
            if s.starts_with("TargetSpec::Object(") {
                Some(0u8)
            } else if s.starts_with("TargetSpec::Spell(") {
                Some(1)
            } else if s.starts_with("TargetSpec::CardInGraveyard(") {
                Some(2)
            } else if matches!(s, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") {
                Some(3)
            } else {
                None
            }
        };
        kind(a).is_some() && kind(a) == kind(b)
    }

    /// A terminal `Condition$ Kicked` sub-ability that introduces the spell's
    /// only target (unkicked the spell targets nothing; kicked it chooses one
    /// target via `FaceDef::kicked_targets`).
    ///
    /// Returns `Some(effect_target)` when matched (having consumed `ValidTgts$`
    /// and any matching `Count$Kicked.1.0` `TargetMin$`/`TargetMax$`, and set
    /// `self.body.kicked_targets` while leaving `chain.target == None`).
    pub(super) fn try_kicked_only_target(
        &mut self,
        api: &str,
        p: &mut Params,
        chain: &Chain,
    ) -> Option<String> {
        if !self.on_a_spell
            || !self.has_single_kicker
            || chain.target.is_some()
            || self.body.kicked_targets.is_some()
            || p.peek("Condition") != Some("Kicked")
            || p.has("SubAbility")
            || p.has("TargetUnique")
            || p.has("DividedAsYouChoose")
        {
            return None;
        }
        let valid = p.peek("ValidTgts")?.to_string();
        match (p.peek("TargetMin"), p.peek("TargetMax")) {
            (None, None) => {}
            (Some(min), Some(max)) if min == max && self.kicked_pair(min) == Some((1, 0)) => {}
            _ => return None,
        }
        p.take("ValidTgts");
        p.take("TargetMin");
        p.take("TargetMax");
        let spec = self.resolve_target_spec(api, p, &valid, chain)?;
        let effect_target = Self::effect_target_of(&spec);
        self.body.kicked_targets = Some(format!("Some(TargetReq::one({spec}))"));
        Some(effect_target)
    }

    /// Single-line kicked target count changes on `A:SP$` (`FaceDef::kicked_targets`),
    /// and explicit refusal for target counts growing with `Count$TimesKicked`.
    ///
    /// Returns `Some(true)` if `TargetMin$`/`TargetMax$` were consumed as a
    /// kicked target count, `Some(false)` if not a kicked count (caller checks
    /// ordinary target count rules), or `None` if explicitly refused.
    pub(super) fn try_kicked_target_count(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        spec: &str,
    ) -> Option<bool> {
        // Any target count that scales with `Count$TimesKicked` (e.g.
        // `Count$TimesKicked/Plus.1` or `Count$TimesKicked`) cannot be
        // expressed in `TargetReq`. Refuse honestly by its SVar definition.
        for key in ["TargetMin", "TargetMax"] {
            if let Some(var) = p.peek(key)
                && let Some(def) = self.svars.get(var.trim())
                && def.trim().starts_with("Count$TimesKicked")
            {
                return self.deny(format!("a target count of `{}`", def.trim()));
            }
        }
        if !self.on_a_spell
            || !self.has_single_kicker
            || self.body.kicked_targets.is_some()
            || p.has("DividedAsYouChoose")
            || p.has("SubAbility")
        {
            return Some(false);
        }
        // Only effects whose resolver iterates every target in `res.targets`
        // (verified in `baylee-engine/src/resolve/{zones,counters}.rs`).
        let multi_target_safe = matches!(api, "Pump" | "Destroy")
            || (api == "ChangeZone"
                && matches!(
                    (p.peek("Origin"), p.peek("Destination")),
                    (Some("Graveyard" | "Battlefield"), Some("Hand"))
                        | (Some("Battlefield"), Some("Exile"))
                ));
        if !multi_target_safe {
            return Some(false);
        }
        let (Some(min_var), Some(max_var)) = (p.peek("TargetMin"), p.peek("TargetMax")) else {
            return Some(false);
        };
        // Shape 2a: `TargetMin$ X | TargetMax$ X` where `X = Count$Kicked.<k>.1` (k >= 2).
        if min_var == max_var
            && let Some((k, 1)) = self.kicked_pair(min_var)
            && k >= 2
            && let Ok(k) = u8::try_from(k)
        {
            p.take("TargetMin");
            p.take("TargetMax");
            self.body.kicked_targets = Some(format!("Some(TargetReq::exactly({spec}, {k}))"));
            return Some(true);
        }
        // Shape 2b: `TargetMin$ X | TargetMax$ Y` where `X = Count$Kicked.0.1`,
        // `Y = Count$Kicked.<Z>.1`, and `Z = Count$Valid <same valid>`.
        if self.kicked_pair(min_var) == Some((0, 1))
            && let Some(y_def) = self.svars.get(max_var.trim()).map(|s| s.trim())
            && let Some(rest) = y_def.strip_prefix("Count$Kicked.")
            && let Some((z_var, "1")) = rest.split_once('.')
            && let Some(z_def) = self.svars.get(z_var.trim()).map(|s| s.trim())
            && z_def.strip_prefix("Count$Valid ") == Some(valid)
        {
            p.take("TargetMin");
            p.take("TargetMax");
            self.body.kicked_targets = Some(format!("Some(TargetReq::up_to({spec}, u8::MAX))"));
            return Some(true);
        }
        Some(false)
    }

    /// Evaluates a line whose numeric parameter references a
    /// `Count$Kicked.<A>.<B>` SVar under both kicked (`Some(true)`) and
    /// unkicked (`Some(false)`) branches, wrapping the pair in
    /// `Effect::IfKicked { then, otherwise }`.
    ///
    /// Returns `Ok(Some(effects))` when split, `Ok(None)` when this line does
    /// not carry a splittable `Count$Kicked.<A>.<B>` parameter, and `Err(())`
    /// when a split was attempted and refused (with reason recorded).
    pub(super) fn try_split_kicked_amount(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        chain: &Chain,
    ) -> Result<Option<Vec<String>>, ()> {
        if !self.can_check_kicked()
            || self.kicked_branch.is_some()
            || chain.second_target.is_some()
            || p.has("Condition")
            || p.has("ConditionCheckSVar")
            || p.has("ConditionPresent")
            || p.has("ConditionDefined")
        {
            return Ok(None);
        }
        let has_kicked_pair = p.entries.iter().any(|(_, value)| {
            self.kicked_pair(value.trim().trim_start_matches(['+', '-']))
                .is_some()
        });
        if !has_kicked_pair {
            return Ok(None);
        }
        let mut p_then = p.clone();
        self.kicked_branch = Some(true);
        self.kicked_branch_used.set(false);
        let then_raw = self.effect_of(api, &mut p_then, target, targets_here, targets_a_player);
        let used_then = self.kicked_branch_used.get();

        let mut p_else = p.clone();
        self.kicked_branch = Some(false);
        self.kicked_branch_used.set(false);
        let else_raw = self.effect_of(api, &mut p_else, target, targets_here, targets_a_player);
        let used_else = self.kicked_branch_used.get();
        self.kicked_branch = None;

        if !used_then && !used_else {
            return Ok(None);
        }
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
        // A 0/0 empty-keyword `PumpTarget` from a `Count$Kicked.<N>.0` arm is
        // a no-op; drop it so the branch emits `otherwise: &[]` (or `then: &[]`).
        let is_noop_pump = |list: &[String]| {
            matches!(
                list,
                [one] if one.starts_with("Effect::PumpTarget { power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY,")
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
            "Effect::IfKicked {{ then: &[{}], otherwise: &[{}] }}",
            then_effects.join(", "),
            else_effects.join(", ")
        )]))
    }

    /// Reads kicker-related line conditions inside `wrap_line_condition`:
    /// - `Condition$ Kicked`
    /// - `ConditionCheckSVar$ <var> | ConditionSVarCompare$ <cmp>` where
    ///   `<var>` is `Count$Kicked.1.0` or `Count$TimesKicked`
    /// - `ConditionDefined$ Self | ConditionPresent$ Card.kicked` / `Card.!kicked`
    /// - `ConditionDefined$ Targeted | ConditionPresent$ Spell.kicked`
    ///
    /// Returns `Ok(Some(wrapped))` when handled, `Ok(None)` when the line's
    /// condition is not a kicker condition, and `Err(())` when a kicker
    /// condition was met and refused (with reason recorded).
    pub(super) fn wrap_kicked_condition(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        effects: &[String],
    ) -> Result<Option<Vec<String>>, ()> {
        // 1. `ConditionDefined$ Self | ConditionPresent$ Card.kicked` / `Card.!kicked`
        // Handled BEFORE `present_clause` so ETB triggers read `res.on_stack`
        // via `Effect::IfKicked` instead of `Condition::SourceMatches`.
        if p.peek("ConditionDefined") == Some("Self")
            && matches!(
                p.peek("ConditionPresent"),
                Some("Card.kicked" | "Card.!kicked")
            )
            && matches!(p.peek("ConditionCompare"), None | Some("GE1"))
        {
            if !self.can_check_kicked() {
                self.note("kicked condition on `Self` outside a spell or self-ETB trigger".into());
                return Err(());
            }
            let kicked = p.take("ConditionPresent").as_deref() == Some("Card.kicked");
            p.take("ConditionDefined");
            p.take("ConditionCompare");
            p.take("ConditionDescription");
            return self.finish_if_kicked(api, p, effects, kicked);
        }

        // 2. `ConditionDefined$ Targeted | ConditionPresent$ Spell.kicked`
        // ("Counter target spell if it was kicked.")
        if p.peek("ConditionDefined") == Some("Targeted")
            && p.peek("ConditionPresent") == Some("Spell.kicked")
            && matches!(p.peek("ConditionCompare"), None | Some("GE1"))
            && target.is_some_and(|t| t.starts_with("TargetSpec::Spell("))
        {
            p.take("ConditionDefined");
            p.take("ConditionPresent");
            p.take("ConditionCompare");
            p.take("ConditionDescription");
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
                return Err(());
            }
            return Ok(Some(vec![format!(
                "Effect::IfTargetMatches {{ filter: &Filter::Kicked, then: &[{}] }}",
                effects.join(", ")
            )]));
        }

        if p.has("ConditionPresent") {
            return Ok(None);
        }

        // 3. `Condition$ Kicked` (and explicit refusal for `Condition$ Kicked 1` / `2`).
        if let Some(cond) = p.peek("Condition") {
            if cond == "Kicked" {
                if !self.can_check_kicked() {
                    self.note("`Condition$ Kicked` outside a spell or self-ETB trigger".into());
                    return Err(());
                }
                p.take("Condition");
                p.take("ConditionDescription");
                return self.finish_if_kicked(api, p, effects, true);
            }
            if cond.starts_with("Kicked ") {
                self.note("a dual-kicker `Condition$`".into());
                return Err(());
            }
        }

        // 4. `ConditionCheckSVar$ <var> | ConditionSVarCompare$ <cmp>` over
        //    `Count$Kicked.1.0` or `Count$TimesKicked`.
        if let (Some(var), Some(cmp)) = (
            p.peek("ConditionCheckSVar").map(str::to_string),
            p.peek("ConditionSVarCompare").map(str::to_string),
        ) && let Some(def) = self.svars.get(var.trim()).map(|s| s.trim())
            && matches!(def, "Count$Kicked.1.0" | "Count$TimesKicked")
        {
            if !self.can_check_kicked() {
                self.note("kicked condition outside a spell or self-ETB trigger".into());
                return Err(());
            }
            let kicked = match cmp.trim() {
                "EQ0" => false,
                "GE1" => true,
                "EQ1" if self.has_single_kicker => true,
                other => {
                    self.note(format!("kicked comparison `{def}` `{other}`"));
                    return Err(());
                }
            };
            p.take("ConditionCheckSVar");
            p.take("ConditionSVarCompare");
            p.take("ConditionDescription");
            return self.finish_if_kicked(api, p, effects, kicked);
        }

        Ok(None)
    }

    fn finish_if_kicked(
        &self,
        api: &str,
        p: &Params,
        effects: &[String],
        kicked: bool,
    ) -> Result<Option<Vec<String>>, ()> {
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `{api}.{key}`"));
            return Err(());
        }
        let body = effects.join(", ");
        let expr = if kicked {
            format!("Effect::IfKicked {{ then: &[{body}], otherwise: &[] }}")
        } else {
            format!("Effect::IfKicked {{ then: &[], otherwise: &[{body}] }}")
        };
        Ok(Some(vec![expr]))
    }

    /// Appends `new_effects` to `effects`, merging two adjacent complementary
    /// one-armed `Effect::IfKicked` nodes (`then`-only + `otherwise`-only, in
    /// either order) into a single `Effect::IfKicked { then, otherwise }`.
    pub(super) fn push_effects_merging_kicked(effects: &mut Vec<String>, new_effects: Vec<String>) {
        if let ([incoming], Some(prev)) = (new_effects.as_slice(), effects.last_mut()) {
            fn split_only(s: &str) -> Option<(bool, &str)> {
                if let Some(inner) = s
                    .strip_prefix("Effect::IfKicked { then: &[], otherwise: &[")
                    .and_then(|r| r.strip_suffix("] }"))
                    && !inner.contains("], otherwise: &[")
                {
                    return Some((false, inner));
                }
                if let Some(inner) = s
                    .strip_prefix("Effect::IfKicked { then: &[")
                    .and_then(|r| r.strip_suffix("], otherwise: &[] }"))
                    && !inner.contains("], otherwise: &[")
                {
                    return Some((true, inner));
                }
                None
            }
            if let (Some((prev_kicked, prev_body)), Some((next_kicked, next_body))) =
                (split_only(prev), split_only(incoming))
                && prev_kicked != next_kicked
            {
                let (then_body, else_body) = if prev_kicked {
                    (prev_body, next_body)
                } else {
                    (next_body, prev_body)
                };
                *prev = format!(
                    "Effect::IfKicked {{ then: &[{then_body}], otherwise: &[{else_body}] }}"
                );
                return;
            }
        }
        effects.extend(new_effects);
    }
}
