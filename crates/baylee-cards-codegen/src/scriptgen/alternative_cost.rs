//! Reading `S: Mode$ AlternativeCost` onto `FaceDef::alternative_costs`.

use super::{Bound, Params, Tx, count_bound};

impl Tx<'_> {
    /// Reads a self-spell `S: Mode$ AlternativeCost` static into an
    /// `AlternativeCost` entry on the front face (CR 118.9, 601.2b).
    ///
    /// Only costs whose parts `cast_wizard::paid_as_an_alternative_cost`
    /// actually collects and conditions with a one-to-one `AltCondition`
    /// reading are accepted; everything else refuses with a specific reason.
    pub(super) fn alternative_cost(&mut self, mut p: Params) -> Option<()> {
        if self.has_evoke {
            return self.deny("an `AlternativeCost` beside `Evoke`".to_string());
        }
        let prints_zero = p
            .peek("Description")
            .unwrap_or(self.oracle)
            .to_ascii_lowercase()
            .contains("pay {0}");
        p.drop_prose();
        match p.take("ValidSA").as_deref() {
            Some("Spell.Self") => {
                if let Some(card) = p.take("ValidCard")
                    && card != "Card.Self"
                {
                    return self.deny(format!("alternative cost for `ValidCard$ {card}`"));
                }
            }
            Some("Spell") => match p.take("ValidCard").as_deref() {
                Some("Card.Self") => {}
                Some(other) => {
                    return self.deny(format!("alternative cost for `ValidCard$ {other}`"));
                }
                None => return self.deny("alternative cost for other spells".to_string()),
            },
            Some(other) => return self.deny(format!("alternative cost for `ValidSA$ {other}`")),
            None => return self.deny("alternative cost with no `ValidSA$`".to_string()),
        }
        match p.take("ValidPlayer").as_deref() {
            None | Some("You") => {}
            Some(other) => {
                return self.deny(format!("alternative cost for `ValidPlayer$ {other}`"));
            }
        }
        match p.take("EffectZone").as_deref() {
            Some("All") => {}
            Some(zone) => {
                return self.deny(format!("alternative cost from `EffectZone$ {zone}`"));
            }
            None => return self.deny("alternative cost from the battlefield".to_string()),
        }
        let Some(raw_cost) = p.take("Cost") else {
            return self.deny("alternative cost with no `Cost$`".to_string());
        };
        let prev = std::mem::replace(&mut self.in_alternative_cost, true);
        let pieces = self.cost_pieces(&raw_cost);
        self.in_alternative_cost = prev;
        let (mana, parts) = pieces?;
        let paid = |part: &String| {
            part.starts_with("PayLife(")
                || part.starts_with("ExileFromHand(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("TapN {")
        };
        if (mana.is_empty() && parts.is_empty()) || !parts.iter().all(paid) {
            return self.deny(format!(
                "an `AlternativeCost` cost `{raw_cost}` the cast cannot pay"
            ));
        }
        let cost = if mana == "{0}" && parts.is_empty() && !prints_zero {
            "Cost::FREE".to_string()
        } else {
            crate::body::cost_literal(&mana, &parts)
        };
        let condition = self.alt_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `AlternativeCost.{key}`"));
        }
        self.body.alternative_costs.push(format!(
            "AlternativeCost {{ cost: {cost}, condition: {condition} }}"
        ));
        Some(())
    }

    fn alt_condition(&self, p: &mut Params) -> Option<String> {
        if matches!(
            p.peek("IsPresent"),
            Some("Card.IsCommander+YouCtrl" | "Card.YouCtrl+IsCommander")
        ) && !p.has("Condition")
            && !p.has("LifeTotal")
            && !p.has("CheckSVar")
            && !p.has("CheckSecondSVar")
            && !p.has("PresentPlayer")
            && !p.has("PresentDefined")
            && !p.has("NoResolvingCheck")
            && !p.has("IsPresent2")
        {
            p.take("IsPresent");
            match p.take("PresentZone").as_deref() {
                None | Some("Battlefield") => {}
                Some(zone) => return self.deny(format!("`PresentZone$ {zone}`")),
            }
            match p.take("PresentCompare").as_deref() {
                None | Some("GE1") => {}
                Some(cmp) => return self.deny(format!("`PresentCompare$ {cmp}`")),
            }
            return Some("AltCondition::CommanderControlled".to_string());
        }
        if p.peek("Condition") == Some("NotPlayerTurn")
            && !p.has("IsPresent")
            && !p.has("LifeTotal")
            && !p.has("CheckSVar")
            && !p.has("CheckSecondSVar")
        {
            p.take("Condition");
            return Some("AltCondition::NotYourTurn".to_string());
        }
        let mut clauses = Vec::new();
        if let Some(cond) = p.take("Condition") {
            clauses.push(match cond.as_str() {
                "NotPlayerTurn" => "Condition::Not(&Condition::YourTurn)".to_string(),
                "PlayerTurn" => "Condition::YourTurn".to_string(),
                "Threshold" => "Condition::GraveyardCountAtLeast(7)".to_string(),
                other => return self.deny(format!("`AlternativeCost` condition `{other}`")),
            });
        }
        if let Some(who) = p.take("LifeTotal") {
            if who != "You" {
                return self.deny(format!("`LifeTotal$ {who}`"));
            }
            let life_amount = p.take("LifeAmount").unwrap_or_else(|| "GE1".to_string());
            let clause = match count_bound(&life_amount) {
                Some(Bound::AtLeast(n)) if n >= 1 => {
                    format!("Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n})")
                }
                Some(Bound::AtMost(n)) => format!("Condition::LifeAtMost({n})"),
                _ => return self.deny(format!("`LifeAmount$ {life_amount}`")),
            };
            clauses.push(clause);
        }
        if let Some(valid) = p.take("IsPresent") {
            clauses.push(self.alt_present_clause(p, &valid)?);
        }
        if let Some(var) = p.take("CheckSVar") {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            clauses.push(self.alt_svar_clause(&var, &cmp)?);
            if let Some(var2) = p.take("CheckSecondSVar") {
                let cmp2 = p
                    .take("SecondSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                clauses.push(self.alt_svar_clause(&var2, &cmp2)?);
            }
        }
        Some(match clauses.as_slice() {
            [] => "AltCondition::Always".to_string(),
            [one] => format!("AltCondition::If(&{one})"),
            many => format!("AltCondition::If(&Condition::All(&[{}]))", many.join(", ")),
        })
    }

    fn alt_present_clause(&self, p: &mut Params, valid: &str) -> Option<String> {
        if p.take("NoResolvingCheck").is_some() {
            return self.deny("`NoResolvingCheck$`, a clause checked once".to_string());
        }
        if p.take("IsPresent2").is_some() {
            return self.deny("a second `IsPresent2$` clause".to_string());
        }
        if p.peek("PresentZone") == Some("Graveyard") && p.peek("PresentPlayer") == Some("You") {
            p.take("PresentZone");
            p.take("PresentPlayer");
            if let Some(defined) = p.take("PresentDefined") {
                return self.deny(format!("`PresentDefined$ {defined}`"));
            }
            if has_relative_atom(valid)
                || valid
                    .split(',')
                    .flat_map(|alt| alt.split(['.', '+']))
                    .any(|a| {
                        let a = a.trim();
                        (a.contains("Ctrl") || a.contains("Own"))
                            && !matches!(a, "YouCtrl" | "YouOwn")
                    })
            {
                return self.deny(format!("`IsPresent$ {valid}` in graveyard"));
            }
            let compare = p
                .take("PresentCompare")
                .unwrap_or_else(|| "GE1".to_string());
            let Some(Bound::AtLeast(n)) = count_bound(&compare).filter(|b| match b {
                Bound::AtLeast(n) => *n >= 1,
                Bound::AtMost(_) => false,
            }) else {
                return self.deny(format!("`PresentCompare$ {compare}`"));
            };
            let stripped = strip_atoms(valid, &["YouCtrl", "YouOwn"]);
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            return Some(if expr == "Filter::Any" {
                format!("Condition::GraveyardCountAtLeast({n})")
            } else {
                format!(
                    "Condition::AmountAtLeast(Amount::CountOf {{ filter: &{expr}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}, {n})"
                )
            });
        }
        if let Some(who) = p.take("PresentPlayer") {
            return self.deny(format!("`PresentPlayer$ {who}`"));
        }
        match p.take("PresentZone").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => return self.deny(format!("`PresentZone$ {zone}`")),
        }
        if let Some(defined) = p.take("PresentDefined") {
            return self.deny(format!("`PresentDefined$ {defined}`"));
        }
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        if has_relative_atom(valid) {
            return self.deny(format!(
                "`IsPresent$ {valid}`, a count relative to this card"
            ));
        }
        let Some(bound) = count_bound(&compare) else {
            return self.deny(format!("`PresentCompare$ {compare}`"));
        };
        self.battlefield_count_condition(valid, bound)
            .or_else(|| self.deny(format!("`IsPresent$ {valid}` compared `{compare}`")))
    }

    fn alt_svar_clause(&self, var: &str, compare: &str) -> Option<String> {
        let Some(raw) = self.svars.get(var.trim()) else {
            return self.deny(format!("`CheckSVar$ {}` naming no SVar", var.trim()));
        };
        let def = raw.trim();
        let deny_def = || {
            if compare == "GE1" {
                self.deny(format!("alternative-cost condition `{def}`"))
            } else {
                self.deny(format!(
                    "alternative-cost condition `{def}` compared `{compare}`"
                ))
            }
        };
        if def == "Count$AttackersDeclared" {
            if let Some(Bound::AtLeast(n)) = count_bound(compare)
                && n >= 1
            {
                return Some(format!("Condition::AttackedWithAtLeast({n})"));
            }
            return deny_def();
        }
        if let Some(valid) = def.strip_prefix("Count$Valid ") {
            if valid.contains('/') || has_relative_atom(valid) {
                return deny_def();
            }
            let Some(bound) = count_bound(compare) else {
                return deny_def();
            };
            return self
                .battlefield_count_condition(valid, bound)
                .or_else(deny_def);
        }
        deny_def()
    }

    fn battlefield_count_condition(&self, valid: &str, bound: Bound) -> Option<String> {
        let alternatives: Vec<&str> = valid.split(',').collect();
        let yours = alternatives.iter().all(|alt| {
            let mut has_you = false;
            for atom in alt.split(['.', '+']).map(str::trim) {
                if atom == "YouCtrl" {
                    has_you = true;
                } else if atom.contains("Ctrl") || atom.contains("Own") {
                    return false;
                }
            }
            has_you
        });
        let opps = alternatives.iter().all(|alt| {
            let mut has_opp = false;
            for atom in alt.split(['.', '+']).map(str::trim) {
                if atom == "OppCtrl" {
                    has_opp = true;
                } else if atom.contains("Ctrl") || atom.contains("Own") {
                    return false;
                }
            }
            has_opp
        });
        let nobodys = !alternatives
            .iter()
            .flat_map(|alt| alt.split(['.', '+']))
            .any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
        match (yours, opps, nobodys, bound) {
            (true, _, _, Bound::AtLeast(n)) if n >= 1 => {
                let stripped = strip_atoms(valid, &["YouCtrl"]);
                let expr = self.filter_expr(&stripped)?;
                Some(format!("Condition::ControlCount(&{expr}, {n})"))
            }
            (true, _, _, Bound::AtMost(n)) => {
                let expr = self.filter_expr(valid)?;
                Some(format!("Condition::ControlCountAtMost(&{expr}, {n})"))
            }
            (_, true, _, Bound::AtLeast(1)) => {
                let expr = self.filter_expr(valid)?;
                Some(format!("Condition::OpponentControlCount(&{expr}, 1)"))
            }
            (_, true, _, Bound::AtMost(0)) => {
                let expr = self.filter_expr(valid)?;
                Some(format!("Condition::BattlefieldCountAtMost(&{expr}, 0)"))
            }
            (false, false, true, Bound::AtLeast(n)) if n >= 1 => {
                let expr = self.filter_expr(valid)?;
                Some(format!("Condition::BattlefieldCount(&{expr}, {n})"))
            }
            (false, false, true, Bound::AtMost(n)) => {
                let expr = self.filter_expr(valid)?;
                Some(format!("Condition::BattlefieldCountAtMost(&{expr}, {n})"))
            }
            _ => None,
        }
    }
}

fn has_relative_atom(valid: &str) -> bool {
    valid.split(',').any(|alt| {
        alt.split(['.', '+']).any(|a| {
            matches!(
                a.trim(),
                "Self"
                    | "Other"
                    | "StrictlyOther"
                    | "EnchantedBy"
                    | "EquippedBy"
                    | "AttachedBy"
                    | "Attached"
            )
        })
    })
}

fn strip_atoms(valid: &str, drop: &[&str]) -> String {
    valid
        .split(',')
        .map(|alt| {
            let mut parts = alt.split('.');
            let base = parts.next().unwrap_or_default().trim();
            let rest: Vec<&str> = parts
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty() && !drop.contains(a))
                .collect();
            if rest.is_empty() {
                base.to_string()
            } else {
                format!("{base}.{}", rest.join("+"))
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
