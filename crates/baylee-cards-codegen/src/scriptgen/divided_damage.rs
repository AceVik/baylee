//! Announced damage and counter divisions (`CR 601.2d`), plus the independent
//! opponent life-gain target paired with divided damage.

use super::{Chain, Params, Tx, counter_kind};

impl Tx<'_> {
    /// Read the complete spell line before the ordinary one-target chain.
    pub(super) fn divided_damage_spell(&mut self, p: Params) -> Option<()> {
        let (effects, _target, target_req, second_req) =
            self.wave28a_read_divided_damage(p, false)?;
        let second = second_req.map_or_default(|req| format!(", second_targets = Some({req})"));
        self.body.abilities.push(format!(
            "spell!(&[{}], targets = Some({target_req}){second})",
            effects.join(", ")
        ));
        Some(())
    }

    /// A fixed total, an announced X with one nonnegative offset, or
    /// `DivisionAmount::XDoubleFrom` (`Shatterskull Smashing`).
    pub(super) fn divided_total(&self, raw: &str) -> Option<String> {
        let raw = raw.trim();
        if let Ok(n) = raw.parse::<u32>() {
            return Some(format!("DivisionAmount::Fixed({n})"));
        }
        let definition = self.svars.get(raw)?.trim();
        if let Ok(n) = definition.parse::<u32>() {
            return Some(format!("DivisionAmount::Fixed({n})"));
        }
        if let Some(n_str) = definition.strip_prefix("Number$")
            && let Ok(n) = n_str.trim().parse::<u32>()
        {
            return Some(format!("DivisionAmount::Fixed({n})"));
        }
        if !self.has_x {
            return None;
        }
        if definition == "Count$xPaid" {
            return Some("DivisionAmount::XPlus(0)".into());
        }
        if let Some(offset) = definition
            .strip_prefix("Count$xPaid/Plus.")
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            return Some(format!("DivisionAmount::XPlus({offset})"));
        }
        if let Some((base, offset)) = definition
            .strip_prefix("SVar$")
            .and_then(|s| s.split_once("/Plus."))
            && let Ok(offset) = offset.trim().parse::<u32>()
            && self.svars.get(base.trim()).map(|v| v.trim()) == Some("Count$xPaid")
        {
            return Some(format!("DivisionAmount::XPlus({offset})"));
        }
        self.divided_x_double_from(definition)
    }

    fn divided_x_double_from(&self, definition: &str) -> Option<String> {
        let rest = definition.strip_prefix("Count$Compare ")?;
        let (lhs, else_arm) = rest.rsplit_once('.')?;
        let (cond, then_arm) = lhs.rsplit_once('.')?;
        let (var, cmp) = cond.trim().rsplit_once(' ')?;
        let is_x = |name: &str| {
            let n = name.trim();
            n == "Count$xPaid" || self.svars.get(n).map(|s| s.trim()) == Some("Count$xPaid")
        };
        let is_2x = |name: &str| {
            let n = name.trim();
            let def = self.svars.get(n).map_or(n, |s| s.trim());
            matches!(def, "Count$xPaid/Twice" | "Count$xPaid/Times.2")
                || def
                    .strip_prefix("SVar$")
                    .and_then(|s| {
                        s.strip_suffix("/Twice")
                            .or_else(|| s.strip_suffix("/Times.2"))
                    })
                    .is_some_and(is_x)
        };
        if !is_x(var) {
            return None;
        }
        if let Some(k_str) = cmp.strip_prefix("LT")
            && let Ok(k) = k_str.parse::<u32>()
            && k >= 1
            && is_x(then_arm)
            && is_2x(else_arm)
        {
            return Some(format!("DivisionAmount::XDoubleFrom({k})"));
        }
        if let Some(k_str) = cmp.strip_prefix("GE")
            && let Ok(k) = k_str.parse::<u32>()
            && k >= 1
            && is_2x(then_arm)
            && is_x(else_arm)
        {
            return Some(format!("DivisionAmount::XDoubleFrom({k})"));
        }
        None
    }

    /// A printed bound, the total itself, or a count covering every target.
    pub(super) fn divided_maximum(&self, raw: &str, total: &str, valid: &str) -> Option<String> {
        let raw = raw.trim();
        if let Ok(n) = raw.parse::<u8>() {
            // 255 means unbounded in TargetReq; do not misread a literal cap.
            return (n < u8::MAX).then(|| n.to_string());
        }
        if self.divided_total(raw).as_deref() == Some(total) {
            return Some("u8::MAX".into());
        }
        let def = self.svars.get(raw)?.trim();
        if def.strip_prefix("Count$Valid ").map(str::trim) == Some(valid.trim()) {
            return Some("u8::MAX".into());
        }
        // An all-players plus all-permanents count is only an upper bound;
        // the target menu and the positive division narrow it further.
        let counted = def.strip_prefix("PlayerCountPlayers$Amount/Plus.")?;
        (valid == "Any" && self.svars.get(counted).map(String::as_str) == Some("Count$Valid Any"))
            .then(|| "u8::MAX".into())
    }

    /// Reads `PutCounter` with `DividedAsYouChoose$` (`Effect::PutCountersDivided`)
    /// for spells, choose-one modal modes, activated abilities, and loyalty
    /// abilities (`CR 601.2d`, `CR 602.2b`).
    ///
    /// Returns `(effects, target_spec_expr, target_req_expr)`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn read_divided_counters(
        &mut self,
        mut p: Params,
    ) -> Option<(Vec<String>, String, String)> {
        p.drop_prose();
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
        let Some(amount) = self.divided_counter_amount(&raw_num) else {
            return self.deny(format!("divided counter count `{raw_num}`"));
        };
        if self.divided_counter_amount(&raw_div).as_deref() != Some(amount.as_str()) {
            return self.deny("counter count and announced division have different totals".into());
        }
        let fixed_total = amount
            .strip_prefix("Amount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u32>().ok());
        if fixed_total == Some(0) {
            return self.deny("divided counter total of zero".into());
        }
        let min: u8 = match p.take("TargetMin").as_deref() {
            Some("0") => 0,
            Some("1") => 1,
            Some(other) => {
                return self.deny(format!(
                    "divided counter distribution with `TargetMin$ {other}`"
                ));
            }
            None => {
                return self.deny("divided counter distribution without `TargetMin$`".into());
            }
        };
        if let Some(total) = fixed_total
            && u32::from(min) > total
        {
            return self.deny("divided counter minimum targets exceeds counter total".into());
        }
        let Some(raw_max) = p.take("TargetMax") else {
            return self.deny("divided counter distribution without `TargetMax$`".into());
        };
        let max_expr = if let Ok(n) = raw_max.trim().parse::<u8>() {
            if n == 0 || n == u8::MAX || n < min {
                return self.deny(format!(
                    "divided counter distribution with `TargetMax$ {raw_max}`"
                ));
            }
            if let Some(total) = fixed_total
                && u32::from(n) > total
            {
                return self.deny("divided counter maximum targets exceeds counter total".into());
            }
            n.to_string()
        } else if raw_max.trim() == raw_num.trim()
            || raw_max.trim() == raw_div.trim()
            || self
                .svars
                .get(raw_max.trim())
                .and_then(|s| s.trim().strip_prefix("Count$Valid "))
                .map(str::trim)
                == Some(valid.trim())
        {
            "u8::MAX".to_string()
        } else {
            return self.deny(format!(
                "divided counter distribution with `TargetMax$ {raw_max}`"
            ));
        };
        p.take("IsCurse");
        let next = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `PutCounter.{key}`"));
        }
        let target_req = match (min, max_expr.as_str()) {
            (0, "1") => format!("TargetReq::up_to_one({spec})"),
            (0, max) => format!("TargetReq::up_to({spec}, {max})"),
            (1, "1") => format!("TargetReq::one({spec})"),
            (1, max) => format!("TargetReq {{ min: 1, ..TargetReq::up_to({spec}, {max}) }}"),
            _ => unreachable!(),
        };
        let mut effects = vec![format!(
            "Effect::PutCountersDivided {{ kind: {kind}, amount: {amount} }}"
        )];
        if let Some(next_name) = next {
            let Some(body) = self.svars.get(next_name.trim()).cloned() else {
                return self.deny(format!("`SubAbility$ {next_name}` names no SVar"));
            };
            let initial_up_to = match max_expr.as_str() {
                "1" => None,
                "u8::MAX" => Some(u8::MAX),
                num => num.parse::<u8>().ok(),
            };
            let mut follow = Chain {
                effects: effects.clone(),
                target: Some(spec.clone()),
                target_up_to: initial_up_to,
                ..Chain::default()
            };
            self.chain(&body, &mut follow)?;
            if follow.target.as_deref() != Some(spec.as_str())
                || follow.second_target.is_some()
                || follow.target_is_x
                || follow.target_up_to != initial_up_to
                || follow.target_exact.is_some()
                || follow.target_range.is_some()
                || follow.could_add_mana
                || follow
                    .effects
                    .iter()
                    .skip(1)
                    .any(|e| e.contains("DealDamageDivided") || e.contains("PutCountersDivided"))
            {
                return self.deny("an unsupported continuation after divided counters".into());
            }
            effects = follow.effects;
        }
        Some((effects, spec, target_req))
    }

    /// Evaluates a `PutCountersDivided` total known at cast/activation
    /// announcement time (`CR 601.2d`), refusing resolution-only amounts.
    fn divided_counter_amount(&mut self, raw: &str) -> Option<String> {
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
        if let Some(def) = self.svars.get(trimmed).map(|s| s.trim().to_string()) {
            if let Some(valid) = def.strip_prefix("Count$LastStateBattlefield ")
                && !valid.contains('$')
                && !valid.contains('/')
            {
                let expr = self.filter_expr(valid.trim())?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}"
                ));
            }
            if let Some(valid) = def.strip_prefix("Count$LastStateGraveyardWithFallback ") {
                let valid = valid.trim();
                if !valid.contains("Ctrl") && !valid.contains("Own") {
                    let expr = self.filter_expr_in_zone(valid, true)?;
                    let name = self.body.filter_static("COUNT", &expr);
                    return Some(format!(
                        "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardAll }}"
                    ));
                }
                if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
                    let expr = self.filter_expr_in_zone(&stripped, true)?;
                    let name = self.body.filter_static("COUNT", &expr);
                    return Some(format!(
                        "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardYou }}"
                    ));
                }
                if let Some(stripped) = Self::strip_opp_graveyard_owner(valid) {
                    let expr = self.filter_expr_in_zone(&stripped, true)?;
                    let name = self.body.filter_static("COUNT", &expr);
                    return Some(format!(
                        "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOpponents }}"
                    ));
                }
                return None;
            }
        }
        let expr = self.amount_or_count_ext(trimmed)?;
        if RESOLUTION_ONLY.iter().any(|bad| expr.contains(bad)) {
            return None;
        }
        Some(expr)
    }
}
