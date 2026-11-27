//! Narrow readers for Thran Portal and Quarum Trench Gnomes.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn reduce_cost(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Type").as_deref() != Some("Spell")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let valid = match p.take("Activator").as_deref() {
            None | Some("Player") => valid,
            Some("You") => append_cost_atom(&valid, "YouCtrl"),
            Some("Opponent" | "Player.Opponent") => append_cost_atom(&valid, "OppCtrl"),
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                append_cost_atom(&valid, "EnchantedPlayerCtrl")
            }
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let Some(amount) = p.take("Amount").and_then(|a| a.parse::<u8>().ok()) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let filter = self.filter_expr(&valid)?;
        let modifier = format!("Modifier::SpellsCostLess({amount})");
        self.body
            .abilities
            .push(Self::static_expr(&filter, &modifier));
        Some(())
    }

    /// A self mana-ability surcharge, only from the ordinary battlefield zone.
    pub(super) fn mana_life_tax(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.peek("Type") == Some("Spell")
            && p.peek("Activator") == Some("Player")
            && p.peek("Amount") == Some("X")
            && p.peek("Relative") == Some("True")
        {
            p.take("Type");
            p.take("Activator");
            p.take("Amount");
            p.take("Relative");
            if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield")) {
                return None;
            }
            let valid = p.take("ValidCard")?;
            let counted = self
                .svars
                .get("X")
                .and_then(|definition| definition.strip_prefix("Count$Valid "))?;
            let counted = counted.strip_suffix(".YouCtrl").unwrap_or(counted);
            if !p.exhausted() {
                return None;
            }
            let spell_filter = self.filter_expr(&valid)?;
            let count_filter = self
                .body
                .filter_static("TAX_COUNT", &self.filter_expr(counted)?);
            self.body.abilities.push(Self::static_expr(
                &spell_filter,
                &format!("Modifier::SpellsCostMorePerControlled(&{count_filter})"),
            ));
            return Some(());
        }
        if p.peek("Type") == Some("Spell") {
            return self.raise_spell_cost(p);
        }
        if p.peek("Type") == Some("Ability")
            && !p.has("ValidSpell")
            && p.peek("AffectedZone") == Some("Battlefield")
            && p.has("Amount")
            && !p.has("Cost")
        {
            return self.ability_cost_tax(p);
        }
        if p.take("Type").as_deref() != Some("Ability")
            || p.take("ValidSpell").as_deref() != Some("Activated.ManaAbility")
            || p.take("ValidCard").as_deref() != Some("Card.Self")
            || !matches!(
                p.take("EffectZone").as_deref(),
                None | Some("Battlefield" | "All")
            )
        {
            return None;
        }
        let cost = p.take("Cost")?;
        if cost != "PayLife<1>" {
            return None;
        }
        let amount = 1;
        if !p.exhausted() {
            return None;
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!("Modifier::ManaAbilitiesCostLife({amount})"),
        ));
        Some(())
    }

    fn raise_spell_cost(&mut self, mut p: Params) -> Option<()> {
        p.take("Type");
        let effect_zone = p.take("EffectZone");
        let valid_target = p.take("ValidTarget");
        let stack_self_target = effect_zone.as_deref() == Some("Stack")
            && valid_target.as_deref() == Some("Spell.Self")
            && !p.has("ValidCard");
        if !stack_self_target && !matches!(effect_zone.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        let target_filter: Option<&'static str> = match valid_target.as_deref() {
            None => None,
            Some("Spell.Self") if stack_self_target => Some("Filter::SpellTargetingAffected"),
            Some("Card.Self" | "Creature.Self" | "Permanent.Self") if !stack_self_target => {
                Some("Filter::SpellTargetingAffected")
            }
            Some(
                "Permanent.YouCtrl"
                | "Permanent.YouCtrl+inZoneBattlefield"
                | "Card.YouCtrl+inZoneBattlefield",
            ) if !stack_self_target => Some("Filter::TargetsPermanentYouControl"),
            Some(_) => return None,
        };
        let valid = match p.take("ValidCard") {
            Some(v) => v,
            None if target_filter.is_some() => "Card".to_string(),
            None => return None,
        };
        let valid = match p.take("Activator").as_deref() {
            None | Some("Player") => valid,
            Some("You") => append_cost_atom(&valid, "YouCtrl"),
            Some("Opponent" | "Player.Opponent") => append_cost_atom(&valid, "OppCtrl"),
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                append_cost_atom(&valid, "EnchantedPlayerCtrl")
            }
            Some(_) => return None,
        };
        let modifier = match (p.take("Amount"), p.take("Cost")) {
            (Some(amount), None) => {
                let amount: u8 = amount.parse().ok()?;
                format!("Modifier::SpellsCostMore({amount})")
            }
            (None, Some(cost)) => {
                let mana = super::printed_mana(&cost)?;
                if let Some(amount) = super::generic_mana(&mana) {
                    format!("Modifier::SpellsCostMore({amount})")
                } else {
                    format!("Modifier::SpellsCostMoreMana(mana!(\"{mana}\"))")
                }
            }
            _ => return None,
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let base_filter = self.filter_expr(&valid)?;
        let filter = match target_filter {
            None => base_filter,
            Some(tf) if base_filter == "Filter::Any" => tf.to_string(),
            Some(tf) => format!("Filter::And(&[{base_filter}, {tf}])"),
        };
        if stack_self_target {
            self.body.abilities.push(format!(
                "static_ability!({filter}, {modifier}, zone = ZoneRef::Stack{cond})"
            ));
        } else {
            self.push_static_with_cond(&filter, &modifier, &cond);
        }
        Some(())
    }

    fn ability_cost_tax(&mut self, mut p: Params) -> Option<()> {
        p.take("Type");
        p.take("AffectedZone");
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Activator").as_deref(), None | Some("Player"))
        {
            return None;
        }
        let valid = p.take("ValidCard")?;
        let amount: u32 = p.take("Amount")?.trim().parse().ok()?;
        if amount == 0 {
            return None;
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let filter = self.filter_expr(&valid)?;
        self.push_static_with_cond(
            &filter,
            &format!("Modifier::AbilitiesCostMore({amount})"),
            &cond,
        );
        Some(())
    }
    /// Indefinite replacement bound to the target's present incarnation.
    pub(super) fn replace_target_white_mana(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        target.filter(|t| t.starts_with("TargetSpec::Object("))?;
        if p.take("IsCurse").as_deref() != Some("True")
            || p.take("RememberObjects").as_deref() != Some("Targeted")
            || p.take("ForgetOnMoved").as_deref() != Some("Battlefield")
            || p.take("Duration").as_deref() != Some("Permanent")
        {
            return None;
        }
        let name = p.take("ReplacementEffects")?;
        let (event, mut r) = Params::parse(self.svars.get(&name)?)?;
        r.drop_prose();
        if event != "ProduceMana" || r.take("ValidCard").as_deref() != Some("Card.IsRemembered") {
            return None;
        }
        let name = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let (api, mut child) = Params::parse(self.svars.get(&name)?)?;
        child.drop_prose();
        if api != "ReplaceMana"
            || child.take("ReplaceColor").as_deref() != Some("C")
            || child.take("ReplaceOnly").as_deref() != Some("W")
            || !child.exhausted()
        {
            return None;
        }
        Some(vec!["Effect::ModifyTargets { modifier: Modifier::ReplaceTappedMana { from: ManaColor::White, to: ManaColor::Colorless }, duration: Duration::Indefinitely }".into()])
    }
}

fn append_cost_atom(valid: &str, atom: &str) -> String {
    valid
        .split(',')
        .map(|alt| {
            let alt = alt.trim();
            if alt.split(['.', '+']).any(|part| part == atom) {
                alt.to_string()
            } else if alt.contains('.') {
                format!("{alt}+{atom}")
            } else {
                format!("{alt}.{atom}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
