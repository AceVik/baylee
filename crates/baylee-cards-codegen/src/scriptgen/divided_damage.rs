//! Announced damage divisions and an independent opponent life-gain target.

use super::{Params, Tx, amount};

impl Tx<'_> {
    /// Read the complete spell line before the ordinary one-target chain.
    pub(super) fn divided_damage_spell(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let valid = p.take("ValidTgts")?;
        let target = self.target_spec(&valid, "DealDamage")?;
        if matches!(
            target.as_str(),
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"
        ) {
            return self.deny("a player-only divided target requirement".into());
        }
        let total = self.divided_total(&p.take("NumDmg")?)?;
        if self.divided_total(&p.take("DividedAsYouChoose")?)? != total {
            return self.deny("damage and announced division have different totals".into());
        }
        if p.take("TargetMin").as_deref() != Some("0") {
            return self.deny("a damage division requiring a nonzero minimum target count".into());
        }
        let maximum = p.take("TargetMax")?;
        let maximum = self.divided_maximum(&maximum, &total, &valid)?;
        let next = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DealDamage.{key}`"));
        }
        let mut effects = vec![format!("Effect::DealDamageDivided {{ amount: {total} }}")];
        let second = if let Some(next) = next {
            let body = self.svars.get(&next)?;
            let (api, mut gain) = Params::parse(body)?;
            gain.drop_prose();
            if api != "GainLife"
                || gain.take("ValidTgts").as_deref() != Some("Opponent")
                || gain.take("Defined").is_some_and(|v| v != "Targeted")
                || gain.take("TargetMin").is_some_and(|v| v != "1")
                || gain.take("TargetMax").is_some_and(|v| v != "1")
            {
                return self.deny("an unsupported continuation after divided damage".into());
            }
            let n = amount(&gain.take("LifeAmount")?, self.svars, self.has_x)?;
            if let Some(key) = gain.first_key() {
                return self.deny(format!("unclaimed parameter `GainLife.{key}`"));
            }
            effects.push(format!(
                "Effect::GainLifeFor {{ amount: {n}, who: PlayerRel::SecondTargetPlayerOrController }}"
            ));
            ", second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"
        } else {
            ""
        };
        self.body.abilities.push(format!(
            "spell!(&[{}], targets = Some(TargetReq::up_to({target}, {maximum})){second})",
            effects.join(", ")
        ));
        Some(())
    }

    /// A fixed total or an announced X with one nonnegative offset.
    fn divided_total(&self, raw: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return Some(format!("DivisionAmount::Fixed({n})"));
        }
        let definition = self.svars.get(raw)?.trim();
        if let Ok(n) = definition.parse::<u32>() {
            return Some(format!("DivisionAmount::Fixed({n})"));
        }
        if !self.has_x {
            return None;
        }
        if definition == "Count$xPaid" {
            return Some("DivisionAmount::XPlus(0)".into());
        }
        let (base, offset) = definition.strip_prefix("SVar$")?.split_once("/Plus.")?;
        let offset = offset.parse::<u32>().ok()?;
        (self.svars.get(base).map(|v| v.trim()) == Some("Count$xPaid"))
            .then(|| format!("DivisionAmount::XPlus({offset})"))
    }

    /// A printed bound, the total itself, or a count covering every target.
    fn divided_maximum(&self, raw: &str, total: &str, valid: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u8>() {
            // 255 means unbounded in TargetReq; do not misread a literal cap.
            return (n < u8::MAX).then(|| n.to_string());
        }
        if self.divided_total(raw).as_deref() == Some(total) {
            return Some("u8::MAX".into());
        }
        // An all-players plus all-permanents count is only an upper bound;
        // the target menu and the positive division narrow it further.
        let counted = self
            .svars
            .get(raw)?
            .strip_prefix("PlayerCountPlayers$Amount/Plus.")?;
        (valid == "Any" && self.svars.get(counted).map(String::as_str) == Some("Count$Valid Any"))
            .then(|| "u8::MAX".into())
    }
}
