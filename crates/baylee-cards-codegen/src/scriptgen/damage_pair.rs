//! Reads one damage instruction naming a permanent and its controller.

use super::{BlockRole, Chain, Params, Tx};

enum PairCondition {
    Unconditional,
    If(String),
}

impl Tx<'_> {
    pub(super) fn prints_simultaneous_damage_to_controller(&self) -> bool {
        self.oracle.to_ascii_lowercase().split('.').any(|sentence| {
            let Some((verb, recipients)) = sentence.split_once(" damage to ") else {
                return false;
            };
            let Some((_, followup)) = recipients.split_once(" and ") else {
                return false;
            };
            let Some((amount, recipient)) = followup.split_once(" damage to ") else {
                return false;
            };
            verb.contains("deals ")
                && !amount
                    .split_whitespace()
                    .any(|word| matches!(word, "deal" | "deals" | "then"))
                && permanent_controller_phrase(recipient)
        })
    }

    /// Forge can give the second recipient its own damage line. Bind both
    /// recipients to the same permanent and gather the two amounts into a
    /// single engine event (CR 120.4b, 614.9); leave later instructions in
    /// the chain to be read in order.
    pub(super) fn damage_pair(
        &mut self,
        mut permanent: Params,
        next: Option<&str>,
        chain: &Chain,
        targets_here: bool,
    ) -> Option<(Vec<String>, Option<String>)> {
        let (api, mut controller) = Params::parse(self.svars.get(next?)?)?;
        if api != "DealDamage"
            || chain.second_target.is_some()
            || chain.target_is_x
            || chain.target_up_to.is_some_and(|n| n > 1)
            || chain.target_exact.is_some_and(|n| n != 1)
            || chain.target_range.is_some()
        {
            return None;
        }
        // With several references in the same face, its Oracle does not
        // identify which script links belong to the split instruction.
        if self
            .oracle
            .to_ascii_lowercase()
            .split(" damage to ")
            .skip(1)
            .filter(|phrase| permanent_controller_phrase(phrase))
            .count()
            != 1
        {
            return None;
        }
        controller.drop_prose();
        let target = self.damage_pair_subject(
            permanent.take("Defined").as_deref(),
            controller.take("Defined").as_deref(),
            chain,
            targets_here,
        )?;

        let condition = self.damage_pair_condition(&mut controller, targets_here)?;
        let after_damage = controller.take("SubAbility");
        let exile = permanent.take("ReplaceDyingDefined");
        let permanent_raw = original_source_damage(&mut permanent)?;
        let controller_raw = original_source_damage(&mut controller)?;
        if !permanent.exhausted() || !controller.exhausted() {
            return None;
        }

        let permanent_amount = self.damage_pair_amount(&permanent_raw)?;
        let controller_amount = self.damage_pair_amount(&controller_raw)?;
        let pair = format!(
            "Effect::DamagePermanentAndController {{ target: {target}, permanent_amount: {permanent_amount}, controller_amount: {controller_amount} }}"
        );
        let damage = match condition {
            PairCondition::If(cond) => format!(
                "Effect::IfCondition {{ condition: {cond}, then: &[{pair}], otherwise: &[Effect::DealDamage {{ amount: {permanent_amount}, target: {target} }}] }}"
            ),
            PairCondition::Unconditional => pair,
        };
        let mut effects = vec![damage];
        if let Some(defined) = exile {
            effects.push(self.exile_if_dies(&defined, Some(&target))?);
        }
        Some((effects, after_damage))
    }

    fn damage_pair_subject(
        &self,
        permanent_defined: Option<&str>,
        controller_defined: Option<&str>,
        chain: &Chain,
        targets_here: bool,
    ) -> Option<String> {
        let block = self.block_line.as_ref().map(|(role, ..)| *role);
        match (permanent_defined, controller_defined, block) {
            (None | Some("Targeted"), Some("TargetedController"), _) if targets_here => {
                Self::effect_target(chain)
                    .filter(|target| target.starts_with("TargetSpec::Object("))
            }
            (
                Some("ParentTarget" | "ParentTargeted"),
                Some("TargetedController" | "ParentTargetController"),
                _,
            ) if chain.target.is_some() => Self::effect_target(chain)
                .filter(|target| target.starts_with("TargetSpec::Object(")),
            (
                Some("TriggeredCard" | "TriggeredCardLKICopy"),
                Some("TriggeredCardController"),
                _,
            ) if chain.target.is_none()
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZone" | "Attacks" | "Blocks" | "Taps" | "BecomesTarget")
                ) =>
            {
                Some("TargetSpec::EventObject".to_string())
            }
            (
                Some("TriggeredAttacker" | "TriggeredAttackerLKICopy"),
                Some("TriggeredAttackerController"),
                Some(BlockRole::Blocks),
            )
            | (
                Some("TriggeredBlocker" | "TriggeredBlockerLKICopy"),
                Some("TriggeredBlockerController"),
                Some(BlockRole::Blocked),
            ) if chain.target.is_none() => Some("TargetSpec::EventObject".to_string()),
            (
                Some("TriggeredAttacker" | "TriggeredAttackerLKICopy"),
                Some("TriggeredAttackerController"),
                None,
            ) if chain.target.is_none()
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) =>
            {
                Some("TargetSpec::EventObject".to_string())
            }
            (
                Some("TriggeredBlocker" | "TriggeredBlockerLKICopy"),
                Some("TriggeredBlockerController"),
                None,
            ) if chain.target.is_none() && self.trigger_mode.as_deref() == Some("Blocks") => {
                Some("TargetSpec::EventObject".to_string())
            }
            _ => None,
        }
    }

    fn damage_pair_condition(
        &self,
        controller: &mut Params,
        targets_here: bool,
    ) -> Option<PairCondition> {
        if let Some(cond) = controller.take("Condition") {
            if !targets_here
                || controller.has("ConditionPresent")
                || controller.has("ConditionDefined")
                || controller.has("ConditionZone")
                || controller.has("ConditionCompare")
                || controller.has("ConditionCheckSVar")
            {
                return None;
            }
            let expr =
                Self::named_condition(&cond).or_else(|| Self::wave7_named_condition(&cond))?;
            controller.take("ConditionDescription");
            return Some(PairCondition::If(expr.to_string()));
        }
        if !controller.has("ConditionPresent") {
            return Some(PairCondition::Unconditional);
        }
        if !targets_here
            || controller.has("ConditionDefined")
            || controller.has("ConditionCheckSVar")
        {
            return None;
        }
        let present = controller.take("ConditionPresent")?;
        let zone = controller.take("ConditionZone");
        let cmp = controller.take("ConditionCompare");
        if zone.as_deref() == Some("Stack")
            && present == "Card.Self+PromisedGift"
            && cmp.as_deref() == Some("EQ1")
        {
            controller.take("ConditionDescription");
            return Some(PairCondition::If("Condition::GiftWasPromised".to_string()));
        }
        if zone.is_some() {
            return None;
        }
        let count: u8 = match cmp.as_deref() {
            None | Some("GT0" | "NE0") => 1,
            Some(raw) => raw
                .strip_prefix("GE")
                .and_then(|n| n.parse::<u8>().ok())
                .filter(|&n| n >= 1)?,
        };
        let stripped = strip_you_ctrl_from_valid(&present)?;
        let filter = self.filter_expr(&stripped)?;
        controller.take("ConditionDescription");
        Some(PairCondition::If(format!(
            "Condition::ControlCount(&{filter}, {count})"
        )))
    }

    fn damage_pair_amount(&mut self, raw: &str) -> Option<String> {
        if let Some(amount) = self.amount_or_count_ext(raw) {
            return Some(amount);
        }
        let def = self.svars.get(raw.trim()).map(String::as_str)?.trim();
        match def {
            "Count$Party" | "Count$PartySize" => {
                Some("Amount::PartySize(PlayerRel::You)".to_string())
            }
            "Count$YourLifeTotal" => Some("Amount::LifeTotal(PlayerRel::You)".to_string()),
            "Count$ValidHand Card.YouOwn" => Some("Amount::HandSize(PlayerRel::You)".to_string()),
            _ => def
                .strip_prefix("Count$CardCounters.")
                .and_then(super::counter_kind)
                .map(|kind| format!("Amount::CountersOnSource({kind})")),
        }
    }
}

fn strip_you_ctrl_from_valid(valid: &str) -> Option<String> {
    let mut out = Vec::new();
    for alt in valid.split(',').map(str::trim) {
        if alt.is_empty() {
            return None;
        }
        let mut head = alt;
        let mut tail = "";
        if let Some((h, t)) = alt.split_once('.') {
            head = h.trim();
            tail = t.trim();
        }
        let mut has_you_ctrl = false;
        let mut kept = Vec::new();
        if !tail.is_empty() {
            for atom in tail.split('+').map(str::trim) {
                if atom == "YouCtrl" {
                    has_you_ctrl = true;
                } else if atom.contains("Ctrl") || atom.contains("Own") {
                    return None;
                } else if !atom.is_empty() {
                    kept.push(atom);
                }
            }
        }
        if !has_you_ctrl {
            return None;
        }
        if kept.is_empty() {
            out.push(head.to_string());
        } else {
            out.push(format!("{head}.{}", kept.join("+")));
        }
    }
    (!out.is_empty()).then(|| out.join(","))
}

fn original_source_damage(params: &mut Params) -> Option<String> {
    if !matches!(params.take("DamageSource").as_deref(), None | Some("Self"))
        || !matches!(params.take("DamageMap").as_deref(), None | Some("True"))
    {
        return None;
    }
    params.take("NumDmg")
}

fn permanent_controller_phrase(text: &str) -> bool {
    [
        "its controller",
        "that artifact's controller",
        "that battle's controller",
        "that creature's controller",
        "that enchantment's controller",
        "that land's controller",
        "that permanent's controller",
        "that planeswalker's controller",
        "that vehicle's controller",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
}
