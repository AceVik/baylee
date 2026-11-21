//! Reads one damage instruction naming a permanent and its controller.

use super::{BlockRole, Chain, Params, Tx};

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
        let block = self.block_line.as_ref().map(|(role, ..)| *role);
        let target = match (
            permanent.take("Defined").as_deref(),
            controller.take("Defined").as_deref(),
            block,
        ) {
            (None | Some("Targeted"), Some("TargetedController"), _) if targets_here => {
                Self::effect_target(chain)
                    .filter(|target| target.starts_with("TargetSpec::Object("))?
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
            ) if chain.target.is_none() => "TargetSpec::EventObject".to_string(),
            _ => return None,
        };

        let optional_threshold = match controller.take("Condition").as_deref() {
            None => false,
            Some("Threshold") if targets_here => true,
            Some(_) => return None,
        };
        let after_damage = controller.take("SubAbility");
        let exile = permanent.take("ReplaceDyingDefined");
        let permanent_raw = original_source_damage(&mut permanent)?;
        let controller_raw = original_source_damage(&mut controller)?;
        if !permanent.exhausted() || !controller.exhausted() {
            return None;
        }

        let permanent_amount = self.amount_or_count_ext(&permanent_raw)?;
        let controller_amount = self.amount_or_count_ext(&controller_raw)?;
        let pair = format!(
            "Effect::DamagePermanentAndController {{ target: {target}, permanent_amount: {permanent_amount}, controller_amount: {controller_amount} }}"
        );
        let damage = if optional_threshold {
            format!(
                "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{pair}], otherwise: &[Effect::DealDamage {{ amount: {permanent_amount}, target: {target} }}] }}"
            )
        } else {
            pair
        };
        let mut effects = vec![damage];
        if let Some(defined) = exile {
            effects.push(self.exile_if_dies(&defined, Some(&target))?);
        }
        Some((effects, after_damage))
    }
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
        "that creature's controller",
        "that permanent's controller",
        "that planeswalker's controller",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
}
