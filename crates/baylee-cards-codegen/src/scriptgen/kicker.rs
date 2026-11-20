//! Reading kicker branches, kicked-only targets, and kicked target replacements.

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Whether a line happens only if this spell was kicked, in any of the
    /// reference's spellings: `Condition$ Kicked` (or a dual kicker's
    /// `Kicked 1`), a `ConditionCheckSVar$` over `Count$Kicked…` or
    /// `Count$TimesKicked…` compared other than `EQ0`, or
    /// `ConditionDefined$ Self | ConditionPresent$ Card.kicked`.
    ///
    /// Only the kicked side: a line that happens only when the spell was not
    /// kicked keeps its target, which the kicked half then reads as "that
    /// permanent" (Rona's Vortex), and a condition on another object's being
    /// kicked (Ertai's Trickery's "if it was kicked", on its target) is no
    /// gate on this spell at all.
    pub(super) fn line_is_gated_on_kicking(&self, p: &Params) -> bool {
        if p.peek("Condition")
            .is_some_and(|cond| cond == "Kicked" || cond.starts_with("Kicked "))
        {
            return true;
        }
        if let Some(var) = p.peek("ConditionCheckSVar")
            && let Some(def) = self.svars.get(var.trim()).map(|s| s.trim())
            && (def.starts_with("Count$Kicked") || def.starts_with("Count$TimesKicked"))
        {
            return !matches!(
                p.peek("ConditionSVarCompare").map(str::trim),
                Some("EQ0" | "LT1" | "LE0")
            );
        }
        p.peek("ConditionDefined") == Some("Self")
            && p.peek("ConditionPresent").is_some_and(|present| {
                present
                    .split(['.', '+'])
                    .any(|atom| atom.trim() == "kicked")
            })
    }

    /// Two consecutive lines of the same API whose `TargetMin$`/`TargetMax$`
    /// flip between `Count$Kicked.0.1` and `Count$Kicked.1.0` and whose other
    /// non-prose parameters are identical: a spell whose kicker replaces its
    /// target requirement (`FaceDef::kicked_targets`, CR 702.33g).
    ///
    /// Returns `Some(child_sub)` when matched (having consumed `ValidTgts$`,
    /// `TargetMin$`, `TargetMax$`, `TgtZone$`, and `SubAbility$` on both lines,
    /// and having populated `chain.target` and `self.body.kicked_targets`).
    /// Returns `None` without modifying `p` when the shape does not apply.
    /// The inner option is the second line's own `SubAbility$`, which the
    /// chain continues with when there is one: two different absences.
    #[allow(clippy::option_option)]
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
        for key in [
            "ValidTgts",
            "ValidTgtsDesc",
            "TargetMin",
            "TargetMax",
            "SubAbility",
        ] {
            p_rest.take(key);
        }
        let mut cp_rest = cp.clone();
        for key in [
            "ValidTgts",
            "ValidTgtsDesc",
            "TargetMin",
            "TargetMax",
            "SubAbility",
        ] {
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
        p.claim_label("ValidTgtsDesc", "ValidTgts");
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
        p.claim_label("ValidTgtsDesc", "ValidTgts");
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
    /// `Count$Kicked.<A>.<B>` `SVar` under both kicked (`Some(true)`) and
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

        // 3. `Condition$ Kicked` (single kicker) and `Condition$ Kicked <1|2>`
        //    (dual kicker, CR 702.33a–b, remapped to Oracle order).
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
            if let Some(slot) = cond.strip_prefix("Kicked ") {
                if let Some(idx) = self.dual_kicker_index(slot) {
                    if !self.can_check_kicked() {
                        self.note("`Condition$ Kicked` outside a spell or self-ETB trigger".into());
                        return Err(());
                    }
                    p.take("Condition");
                    p.take("ConditionDescription");
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                        return Err(());
                    }
                    let body = effects.join(", ");
                    return Ok(Some(vec![format!(
                        "Effect::IfCondition {{ condition: Condition::KickedWith({idx}), then: &[{body}], otherwise: &[] }}"
                    )]));
                }
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

    /// Maps a 1-based script dual-kicker slot (`"1"` or `"2"`) to its 0-based
    /// Oracle-ordered `FaceDef::additional_costs` index, or `None` when this
    /// card is not a dual-kicker card or the slot is unrecognized.
    pub(super) fn dual_kicker_index(&self, slot: &str) -> Option<u8> {
        let remap = self.dual_kicker_remap?;
        match slot.trim() {
            "1" => Some(remap[0]),
            "2" => Some(remap[1]),
            _ => None,
        }
    }

    /// When a dual-kicker script lists its two kicker costs in the reverse of
    /// Oracle order (`dual_kicker_remap == Some([1, 0])`), reorders emitted
    /// `Condition::KickedWith` triggers and `EnterModifier::IfKicked` entries
    /// so they match Oracle paragraph order (CR 702.33f, `lines::map`).
    pub(super) fn align_dual_kicker_order(&mut self) {
        if self.dual_kicker_remap != Some([1, 0]) {
            return;
        }
        Self::reorder_by_key(&mut self.body.abilities, |expr| {
            let marker = "condition = Some(Condition::KickedWith(";
            let start = expr.find(marker)? + marker.len();
            let rest = &expr[start..];
            let end = rest.find(')')?;
            rest[..end].parse::<u8>().ok()
        });
        Self::reorder_by_key(&mut self.body.enter_modifiers, |expr| {
            let marker = "EnterModifier::IfKicked { index: Some(";
            let start = expr.find(marker)? + marker.len();
            let rest = &expr[start..];
            let end = rest.find(')')?;
            rest[..end].parse::<u8>().ok()
        });
    }

    fn reorder_by_key(items: &mut [String], key: impl Fn(&str) -> Option<u8>) {
        let mut slots: Vec<(usize, u8)> = items
            .iter()
            .enumerate()
            .filter_map(|(pos, item)| key(item).map(|k| (pos, k)))
            .collect();
        if slots.len() < 2 {
            return;
        }
        let positions: Vec<usize> = slots.iter().map(|(pos, _)| *pos).collect();
        slots.sort_by_key(|&(_, k)| k);
        let reordered: Vec<String> = slots.iter().map(|&(pos, _)| items[pos].clone()).collect();
        for (pos, value) in positions.into_iter().zip(reordered) {
            items[pos] = value;
        }
    }

    /// `K:ETBReplacement:Other:<svar>:Mandatory::Card.Self+kicked <1|2>`
    /// lowered into `EnterModifier::IfKicked` (CR 614.1c, CR 611.2e, CR 702.33e–f).
    ///
    /// Reads the `DB$ PutCounter` (P1P1 counters on `Self` with `ETB$ True`)
    /// and its optional permanent `DB$ Animate` sub-ability granting keywords,
    /// activated abilities, or triggered abilities.
    pub(super) fn volver_etb_replacement(&mut self, svar: &str, index: Option<u8>) -> Option<()> {
        let Some(body) = self.svars.get(svar).cloned() else {
            return self.deny(format!("an `ETBReplacement` naming the missing `{svar}`"));
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return self.deny("an `ETBReplacement` ability with no `$` in it".to_string());
        };
        p.drop_prose();
        if api != "PutCounter"
            || p.take("Defined").as_deref() != Some("Self")
            || p.take("CounterType").as_deref() != Some("P1P1")
            || p.take("ETB").as_deref() != Some("True")
        {
            return self.deny(format!("kicked as-enters effect `{api}`"));
        }
        let Some(num_raw) = p.take("CounterNum") else {
            return self.deny("kicked `PutCounter` with no `CounterNum$`".to_string());
        };
        let Ok(counters) = num_raw.trim().parse::<u8>() else {
            return self.deny(format!("kicked `PutCounter` count `{num_raw}`"));
        };
        let sub = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `PutCounter.{key}`"));
        }
        let modifiers = match sub {
            Some(sub_name) => self.volver_animate_modifiers(&sub_name)?,
            None => Vec::new(),
        };
        if counters == 0 && modifiers.is_empty() {
            return self.deny("a kicked `ETBReplacement` that does nothing".to_string());
        }
        let index_expr = index.map_or_else(|| "None".to_string(), |i| format!("Some({i})"));
        self.body.enter_modifiers.push(format!(
            "EnterModifier::IfKicked {{ index: {index_expr}, counters: {counters}, modifiers: &[{}] }}",
            modifiers.join(", ")
        ));
        Some(())
    }

    fn volver_animate_modifiers(&mut self, svar: &str) -> Option<Vec<String>> {
        let Some(body) = self.svars.get(svar).cloned() else {
            return self.deny(format!("`SubAbility$ {svar}` names no SVar"));
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return self.deny("an `ETBReplacement` sub-ability with no `$` in it".to_string());
        };
        p.drop_prose();
        if api != "Animate"
            || p.take("Defined").as_deref() != Some("Self")
            || p.take("Duration").as_deref() != Some("Permanent")
        {
            return self.deny(format!("kicked as-enters sub-ability `{api}`"));
        }
        let mut modifiers = Vec::new();
        if let Some(raw) = p.take("Keywords") {
            let each: Option<Vec<&str>> = raw
                .split('&')
                .map(|k| super::keyword_const(k.trim()))
                .collect();
            let Some(each) = each else {
                return self.deny(format!("kicked `Animate` granting `{raw}`"));
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            modifiers.push(format!("Modifier::AddKeyword({joined})"));
        }
        if let Some(raw) = p.take("Abilities") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                modifiers.push(self.volver_granted_activated(name)?);
            }
        }
        if let Some(raw) = p.take("Triggers") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                modifiers.push(self.volver_granted_trigger(name)?);
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Animate.{key}`"));
        }
        if modifiers.is_empty() {
            return self.deny("kicked `Animate` granting nothing".to_string());
        }
        Some(modifiers)
    }

    fn volver_granted_activated(&mut self, name: &str) -> Option<String> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`Abilities$ {name}` naming no `SVar`"));
        };
        if !line.trim_start().starts_with("AB$") {
            return self.deny("a granted ability that is not an activated one".to_string());
        }
        let Some((_, mut probe)) = Params::parse(&line) else {
            return self.deny("a granted ability with no `$` in it".to_string());
        };
        let Some(cost) = probe.take("Cost") else {
            return self.deny("a granted ability with no `Cost$`".to_string());
        };
        let cost = self.cost_expr(&cost)?;
        let stripped: Vec<&str> = line
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$"))
            .collect();
        let mut chain = Chain::default();
        self.chain(&stripped.join(" | "), &mut chain)?;
        if chain.target.is_some() {
            return self.deny("a granted ability with a target".to_string());
        }
        if chain.effects.is_empty() {
            return self.deny("a granted ability that reads as no effect at all".to_string());
        }
        let mana = chain.could_add_mana && !chain.moves_library;
        Some(format!(
            "Modifier::GrantActivated {{ cost: {cost}, effects: &[{}], mana_ability: {mana} }}",
            chain.effects.join(", ")
        ))
    }

    fn volver_granted_trigger(&mut self, name: &str) -> Option<String> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`Triggers$ {name}` naming no `SVar`"));
        };
        let Some((mode, mut tp)) = Params::parse(&line) else {
            return self.deny("a granted trigger with no `$` in it".to_string());
        };
        tp.drop_prose();
        let trigger = self.trigger_expr(&mut tp, &mode)?;
        if !self.condition(&mut tp)?.is_empty() {
            return self.deny("a granted trigger with an intervening condition".to_string());
        }
        let Some(execute) = tp.take("Execute") else {
            return self.deny(format!("a granted `{mode}` trigger with no `Execute$`"));
        };
        if let Some(key) = tp.first_key() {
            return self.deny(format!("unclaimed parameter `{mode}.{key}`"));
        }
        let Some(exec_body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        let prev_mode = self.trigger_mode.replace(mode);
        let prev_self = std::mem::replace(
            &mut self.trigger_is_self,
            trigger == "Trigger::ETB" || trigger.contains("&Filter::This"),
        );
        let mut chain = Chain::default();
        let ok = self.chain(&exec_body, &mut chain);
        self.trigger_mode = prev_mode;
        self.trigger_is_self = prev_self;
        ok?;
        if chain.effects.is_empty()
            || chain.second_target.is_some()
            || chain.target_up_to.is_some()
            || chain.target_exact.is_some()
            || chain.target_is_x
        {
            return self.deny("a granted trigger with unsupported effects or targets".to_string());
        }
        let target = chain
            .target
            .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
        Some(format!(
            "Modifier::GrantTriggered {{ trigger: {trigger}, effects: &[{}], target: {target} }}",
            chain.effects.join(", ")
        ))
    }
}
