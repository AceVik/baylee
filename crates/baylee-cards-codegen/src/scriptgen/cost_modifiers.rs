//! Cost-modifier lowers for post-engine Wave 19 spell, ability, plot, morph,
//! commander-tax, and target-dependent discounts/taxes (`CR 601.2f`, `CR 602.2b`).

use super::statics::{StaticEffectZone, simple_const_filter};
use super::wave17c::activator_player_rel;
use super::{Params, Tx, counter_kind, printed_mana};

#[derive(Debug, Default)]
pub(super) struct CommanderTaxLifePair {
    pub reduce_seen: bool,
    pub life_per_tax: Option<u32>,
}

impl Tx<'_> {
    /// Parses `Modifier::PlotActionCostsLess` (*Doc Aurlock*),
    /// `Modifier::ActivatedAbilitiesWithKeywordCostLess` (*Dragonkin Berserker*, *Silver-Fur Master*),
    /// `Modifier::FirstActivatedAbilityEachTurnCostLess` (*Professor Hojo*, *Tezzeret*),
    /// `Modifier::SpellsCostLessPerSharedCardTypeWithExiled` (*Cemetery Prowler*),
    /// `Modifier::FirstSpellEachTurnCostAdjustByAmount` (*Zimone, Infinite Analyst*),
    /// `Modifier::ReduceCostIfTargetsCheck` (*Lullmage's Domination*, *Not of This World*),
    /// and the `ReduceCost` half of `Modifier::CommanderTaxReplacedWithLife` (*Liesa*).
    ///
    /// Returns `None` when `p` does not match any of these discriminators so the
    /// caller continues normal `mode_reduce_cost` lowering.
    pub(super) fn try_reduce_cost_modifiers(&mut self, p: &Params) -> Option<Result<(), ()>> {
        if p.peek("ValidSpell") == Some("Static.Plotting") {
            return Some(self.reduce_cost_plot_action(p.clone()).ok_or(()));
        }
        if matches!(
            p.peek("ValidSpell"),
            Some("Activated.Boast" | "Activated.Ninjutsu")
        ) {
            return Some(self.reduce_cost_activated_keyword(p.clone()).ok_or(()));
        }
        if p.peek("ValidCard") == Some("Card.Self+wasCastFromCommand")
            && p.peek("EffectZone") == Some("Stack")
        {
            return Some(self.record_commander_tax_reduce(p.clone()).ok_or(()));
        }
        if p.peek("ValidSpell") == Some("Activated.YouCtrl")
            && p.has("CheckSVar")
            && p.peek("SVarCompare") == Some("LE1")
        {
            return Some(
                self.reduce_cost_first_activated_ability(p.clone())
                    .ok_or(()),
            );
        }
        if p.peek("ValidCard") == Some("Card")
            && p.peek("Type") == Some("Spell")
            && !p.has("ValidTarget")
            && p.peek("Amount").is_some_and(|raw| {
                let t = raw.trim();
                t == "Count$TypesSharedWith ValidExile Card.ExiledWithSource"
                    || self.svars.get(t).is_some_and(|d| {
                        d.trim() == "Count$TypesSharedWith ValidExile Card.ExiledWithSource"
                    })
            })
        {
            return Some(
                self.reduce_cost_shared_card_types_with_exiled(p.clone())
                    .ok_or(()),
            );
        }
        if matches!(p.peek("EffectZone"), Some("All" | "Stack"))
            && p.peek("ValidCard") == Some("Card.Self")
            && p.peek("Relative") == Some("True")
            && !p.has("ValidTarget")
            && p.peek("Amount").is_some_and(|raw| {
                self.svars
                    .get(raw.trim())
                    .is_some_and(|d| d.trim().starts_with("Count$Compare "))
            })
        {
            return Some(self.reduce_cost_if_targets_check(p.clone()).ok_or(()));
        }
        if self.is_first_spell_this_turn_check(p) {
            return Some(
                self.first_spell_cost_adjust_by_amount(p.clone(), false)
                    .ok_or(()),
            );
        }
        None
    }

    /// Parses `Modifier::MorphTurnFaceUpCostsMore` (*Exiled Doomsayer*),
    /// `Modifier::ActivatedAbilitiesCostAdditional` (*Brutal Suppression*),
    /// `Modifier::FirstSpellEachTurnCostAdjustByAmount` taxes,
    /// `Modifier::AdjustSpellCostBeyondFirstTarget` (*Fireball*, *Officious Interrogation*),
    /// and the `RaiseCost` half of `Modifier::CommanderTaxReplacedWithLife` (*Liesa*).
    pub(super) fn try_raise_cost_modifiers(&mut self, p: &Params) -> Option<Result<(), ()>> {
        if p.peek("ValidSpell") == Some("Static.MorphUp") {
            return Some(self.raise_cost_morph_face_up(p.clone()).ok_or(()));
        }
        if p.peek("ValidCard") == Some("Card.Self+wasCastFromCommand")
            && p.peek("EffectZone") == Some("Stack")
            && p.peek("Secondary") == Some("True")
        {
            return Some(self.record_commander_tax_raise(p.clone()).ok_or(()));
        }
        if p.peek("Type") == Some("Ability")
            && p.has("Cost")
            && !p.has("ForEachShard")
            && matches!(p.peek("ValidSpell"), None | Some("Activated"))
            && p.peek("ValidCard") != Some("Card.Self")
        {
            return Some(self.raise_cost_activated_additional(p.clone()).ok_or(()));
        }
        if matches!(p.peek("EffectZone"), Some("All" | "Stack"))
            && p.peek("ValidCard") == Some("Card.Self")
            && p.peek("Relative") == Some("True")
            && !p.has("ValidTarget")
            && p.peek("Amount").is_some_and(|raw| {
                let t = raw.trim();
                t == "TargetedObjects$Amount/Minus.1"
                    || self
                        .svars
                        .get(t)
                        .is_some_and(|d| d.trim() == "TargetedObjects$Amount/Minus.1")
            })
        {
            return Some(self.raise_cost_beyond_first_target(p.clone()).ok_or(()));
        }
        if self.is_first_spell_this_turn_check(p) {
            return Some(
                self.first_spell_cost_adjust_by_amount(p.clone(), true)
                    .ok_or(()),
            );
        }
        None
    }

    fn is_first_spell_this_turn_check(&self, p: &Params) -> bool {
        matches!(p.peek("Type"), Some("Spell"))
            && matches!(p.peek("SVarCompare"), Some("EQ0" | "LE0" | "LT1"))
            && !p.has("ValidTarget")
            && !p.has("OnlyFirstSpell")
            && p.peek("Amount")
                .is_some_and(|a| a.trim().parse::<u32>().is_err())
            && p.peek("CheckSVar").is_some_and(|var| {
                self.svars
                    .get(var.trim())
                    .is_some_and(|d| d.trim().starts_with("Count$ThisTurnCast_"))
            })
    }

    fn reduce_cost_plot_action(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidSpell");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        }
        if !matches!(p.take("AffectedZone").as_deref(), None | Some("Hand"))
            || !matches!(p.take("Type").as_deref(), None | Some("Static"))
            || !matches!(p.take("ValidCard").as_deref(), None | Some("Card"))
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<u8>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::PlotActionCostsLess {{ who: {who}, amount: {amount} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn raise_cost_morph_face_up(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidSpell");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        }
        if !matches!(
            p.take("ValidCard").as_deref(),
            None | Some("Creature.faceDown" | "Card.faceDown")
        ) || !matches!(p.take("Activator").as_deref(), None | Some("Player"))
        {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<u8>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::MorphTurnFaceUpCostsMore {{ amount: {amount} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn raise_cost_activated_additional(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("Type");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        }
        if !matches!(p.take("ValidSpell").as_deref(), None | Some("Activated"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
        {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let (mana, parts) = self.cost_pieces(&cost_raw)?;
        if !mana.is_empty() || parts.len() != 1 {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let part = &parts[0];
        if part.contains("Self") || part.contains("Grantor") || part.contains('X') {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let card_expr = if raw_card == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr(&raw_card)?
        };
        let filter_ref = if simple_const_filter(&card_expr) {
            format!("&{card_expr}")
        } else {
            format!("&{}", self.body.filter_static("AB_TAX", &card_expr))
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::ActivatedAbilitiesCostAdditional {{ who: {who}, filter: {filter_ref}, cost: CostPart::{part} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_shared_card_types_with_exiled(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("Type");
        p.take("ValidCard");
        p.take("Amount");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::SpellsCostLessPerSharedCardTypeWithExiled {{ who: {who} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn first_spell_cost_adjust_by_amount(&mut self, mut p: Params, increase: bool) -> Option<()> {
        let mode_label = if increase {
            "static ability `S: Mode$ RaiseCost`"
        } else {
            "static ability `S: Mode$ ReduceCost`"
        };
        p.drop_prose();
        p.take("Type");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny(mode_label.into()),
        }
        if !matches!(p.take("ValidSpell").as_deref(), None | Some("Spell")) {
            return self.deny(mode_label.into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny(mode_label.into());
        };
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny(mode_label.into());
        };
        let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        let check_var = p.take("CheckSVar")?;
        p.take("SVarCompare");
        let cast_def = self.svars.get(check_var.trim())?.trim().to_string();
        let cast_raw = cast_def.strip_prefix("Count$ThisTurnCast_")?;
        let normalized_cast = match who {
            "PlayerRel::You" => strip_scope_atom(cast_raw, "YouCtrl")?,
            "PlayerRel::EachOpponent" => strip_scope_atom(cast_raw, "OppCtrl")?,
            "PlayerRel::EachPlayer" if !cast_raw.contains("Ctrl") => cast_raw.to_string(),
            _ => return self.deny(mode_label.into()),
        };
        if normalized_cast != raw_card {
            return self.deny(mode_label.into());
        }
        let amt_raw = p.take("Amount")?;
        let amt_def = if amt_raw.starts_with("Count$") || amt_raw.starts_with("SVar$") {
            amt_raw
        } else {
            self.svars.get(amt_raw.trim())?.clone()
        };
        let amt_expr = self
            .wave14_static_amount(&amt_def)
            .or_else(|| self.static_count_expr_ext(&amt_def))?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny(mode_label.into());
        }
        let expr = self.filter_expr_in_zone(&raw_card, true)?;
        let inner_filter = if simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("FIRST_SPELL_COST", &expr))
        };
        let modifier = format!(
            "Modifier::FirstSpellEachTurnCostAdjustByAmount {{ who: {who}, filter: {inner_filter}, amount: {amt_expr}, increase: {increase} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_first_activated_ability(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidSpell");
        p.take("SVarCompare");
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        }
        if !matches!(p.take("Type").as_deref(), None | Some("Ability"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let your_turn_only = match p.take("Activator").as_deref() {
            None | Some("You") => false,
            Some("You.Active") => true,
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let Some(raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let raw_target = p.take("ValidTarget");
        let check_var = p.take("CheckSVar")?;
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<u32>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let act_def = self.svars.get(check_var.trim())?.trim().to_string();
        let act_rest = act_def.strip_prefix("Count$ThisTurnActivated_Activated")?;
        let mut act_atoms = act_rest
            .trim_start_matches('.')
            .split('+')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect::<Vec<_>>();
        if !act_atoms.contains(&"YouCtrl") {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        act_atoms.retain(|a| *a != "YouCtrl");

        // Verify the source scope matches `ValidCard$`.
        let src_stripped = strip_in_zone_battlefield(&raw_card);
        if src_stripped == "Card" {
            if act_atoms
                .iter()
                .any(|a| !a.starts_with("IsTargeting Valid "))
            {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
        } else if !act_atoms.contains(&src_stripped.as_str()) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }

        // Verify optional target filter matches `IsTargeting Valid ...`.
        let target_filter = match raw_target.as_deref() {
            None => "&Filter::Any".to_string(),
            Some(tgt) => {
                let Some(is_tgt_atom) = act_atoms
                    .iter()
                    .find(|a| a.starts_with("IsTargeting Valid "))
                else {
                    return self.deny("static ability `S: Mode$ ReduceCost`".into());
                };
                let hist_tgt = is_tgt_atom["IsTargeting Valid ".len()..].replace('~', "+");
                if strip_in_zone_battlefield(&hist_tgt) != strip_in_zone_battlefield(tgt) {
                    return self.deny("static ability `S: Mode$ ReduceCost`".into());
                }
                self.parse_target_cost_filter(tgt, false)?
            }
        };
        let src_expr = if src_stripped == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr(&src_stripped)?
        };
        let source_filter = if simple_const_filter(&src_expr) {
            format!("&{src_expr}")
        } else {
            format!("&{}", self.body.filter_static("ACT_COST_SRC", &src_expr))
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::FirstActivatedAbilityEachTurnCostLess {{ who: PlayerRel::You, source_filter: {source_filter}, target_filter: {target_filter}, amount: {amount}, your_turn_only: {your_turn_only} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_activated_keyword(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let kind = match p.take("ValidSpell").as_deref() {
            Some("Activated.Boast") => "ActivatedAbilityKind::Boast",
            Some("Activated.Ninjutsu") => "ActivatedAbilityKind::Ninjutsu",
            _ => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        }
        if !matches!(p.take("Type").as_deref(), None | Some("Ability"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(p.take("ValidCard").as_deref(), None | Some("Card"))
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let amt_raw = p.take("Amount")?;
        let amount_expr = if let Ok(n) = amt_raw.trim().parse::<u32>() {
            if n == 0 {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            format!("Amount::Fixed({n})")
        } else {
            let def = if amt_raw.starts_with("Count$") || amt_raw.starts_with("SVar$") {
                amt_raw
            } else {
                self.svars.get(amt_raw.trim())?.clone()
            };
            self.wave14_static_amount(def.trim())
                .or_else(|| self.static_count_expr_ext(def.trim()))?
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::ActivatedAbilitiesWithKeywordCostLess {{ who: {who}, keyword: {kind}, amount: {amount_expr} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    fn record_commander_tax_reduce(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidCard");
        p.take("EffectZone");
        if p.take("Type").as_deref() != Some("Spell") || self.commander_tax_pair.reduce_seen {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let amt_raw = p.take("Amount")?;
        let def = self.svars.get(amt_raw.trim())?.trim().to_string();
        if def != "Count$CommanderCastFromCommandZone/Times.2"
            || !matches!(p.take("Activator").as_deref(), None | Some("You"))
            || !p.exhausted()
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        self.commander_tax_pair.reduce_seen = true;
        self.maybe_emit_commander_tax_life();
        Some(())
    }

    fn record_commander_tax_raise(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidCard");
        p.take("EffectZone");
        p.take("Secondary");
        if p.take("Type").as_deref() != Some("Spell")
            || self.commander_tax_pair.life_per_tax.is_some()
        {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let cost_raw = p.take("Cost")?;
        let var = cost_raw
            .trim()
            .strip_prefix("PayLife<")
            .and_then(|s| s.strip_suffix('>'))?
            .trim();
        let def = self.svars.get(var)?.trim().to_string();
        let life_per_tax = if def == "Count$CommanderCastFromCommandZone" {
            1
        } else if let Some(n_str) = def.strip_prefix("Count$CommanderCastFromCommandZone/Times.") {
            n_str.trim().parse::<u32>().ok().filter(|&n| n > 0)?
        } else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        if !matches!(p.take("Activator").as_deref(), None | Some("You")) || !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.commander_tax_pair.life_per_tax = Some(life_per_tax);
        self.maybe_emit_commander_tax_life();
        Some(())
    }

    fn maybe_emit_commander_tax_life(&mut self) {
        if self.commander_tax_pair.reduce_seen
            && let Some(life_per_tax) = self.commander_tax_pair.life_per_tax
        {
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::CommanderTaxReplacedWithLife {{ life_per_tax: {life_per_tax} }}, zone = ZoneRef::Command)"
            ));
        }
    }

    /// Ensures paired commander-tax replacement statics (`ReduceCost` + `RaiseCost`)
    /// never succeed with only one half (*Liesa, Shroud of Dusk*).
    pub(super) fn commander_tax_pair_complete(&mut self) -> Option<()> {
        if self.commander_tax_pair.reduce_seen != self.commander_tax_pair.life_per_tax.is_some() {
            return self.deny("unpaired commander tax replacement static".to_string());
        }
        Some(())
    }

    /// Lowers *Monastery Siege* (`Dragons`) `ValidTarget$ You,<perm_you_ctrl>`
    /// into `Modifier::RaiseCostIfTargetsYouOrPermanent`.
    pub(super) fn try_raise_cost_you_or_permanent(
        &mut self,
        raw_target: &str,
        who: &str,
        spell_filter: &str,
        extra: (&str, u32),
        zone: StaticEffectZone,
        cond: &str,
    ) -> Option<()> {
        let (extra_mana, extra_life) = extra;
        let mut saw_you = false;
        let mut perm_parts = Vec::new();
        for raw_alt in raw_target.split(',') {
            let alt = raw_alt.trim();
            if alt == "You" {
                if saw_you {
                    return self.deny("static ability `S: Mode$ RaiseCost`".into());
                }
                saw_you = true;
            } else {
                perm_parts.push(alt);
            }
        }
        if !saw_you || perm_parts.is_empty() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let joined = perm_parts.join(",");
        let perm_expr = self.filter_expr(&joined)?;
        let perm_ref = if simple_const_filter(&perm_expr) {
            format!("&{perm_expr}")
        } else {
            format!("&{}", self.body.filter_static("TARGET_COST", &perm_expr))
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::RaiseCostIfTargetsYouOrPermanent {{ who: {who}, spell_filter: {spell_filter}, permanent_filter: {perm_ref}, extra_mana: {extra_mana}, extra_life: {extra_life} }}"
            ),
            zone,
            cond,
        ));
        Some(())
    }

    fn raise_cost_beyond_first_target(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidCard");
        p.take("EffectZone");
        p.take("Relative");
        p.take("Amount");
        if !matches!(p.take("Type").as_deref(), None | Some("Spell"))
            || !matches!(p.take("ValidSpell").as_deref(), None | Some("Spell"))
            || !matches!(p.take("Activator").as_deref(), None | Some("You"))
        {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let mana_expr = match p.take("Cost") {
            None => "mana!(\"{1}\")".to_string(),
            Some(cost_raw) => {
                let mana = printed_mana(cost_raw.trim())?;
                format!("mana!(\"{mana}\")")
            }
        };
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::AdjustSpellCostBeyondFirstTarget {{ who: PlayerRel::You, spell_filter: &Filter::This, mana_per_extra_target: {mana_expr} }}, zone = ZoneRef::Stack)"
        ));
        Some(())
    }

    fn reduce_cost_if_targets_check(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidCard");
        p.take("EffectZone");
        p.take("Relative");
        if !matches!(p.take("Type").as_deref(), None | Some("Spell"))
            || !matches!(p.take("ValidSpell").as_deref(), None | Some("Spell"))
            || !matches!(p.take("Activator").as_deref(), None | Some("You"))
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let amt_raw = p.take("Amount")?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let def = self.svars.get(amt_raw.trim())?.trim().to_string();
        let compare = def.strip_prefix("Count$Compare ")?;
        let (lhs_part, else_str) = compare.rsplit_once('.')?;
        if else_str.trim() != "0" {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let (cond_spec, then_str) = lhs_part.rsplit_once('.')?;
        let amount = then_str.trim().parse::<u32>().ok().filter(|&n| n > 0)?;
        let (var, cmp) = cond_spec.trim().rsplit_once(' ')?;
        let var_def = self.svars.get(var.trim())?.trim().to_string();
        let check = if var_def == "TargetedController$CardsInGraveyard" {
            let n = cmp
                .trim()
                .strip_prefix("GE")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&v| v > 0)?;
            format!("TargetCostCheck::TargetControllerGraveyardAtLeast({n})")
        } else if let Some(valid) = var_def.strip_prefix("TargetedByTarget$Valid ") {
            if !matches!(cmp.trim(), "GE1" | "GT0") {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let stripped = strip_scope_atom(valid.trim(), "YouCtrl")?;
            let expr = self.filter_expr(&stripped)?;
            let filter_ref = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("TGT_STACK", &expr))
            };
            format!("TargetCostCheck::TargetedStackTargetsYouControl({filter_ref})")
        } else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::ReduceCostIfTargetsCheck {{ who: PlayerRel::You, spell_filter: &Filter::This, reduction: mana!(\"{{{amount}}}\"), check: {check} }}, zone = ZoneRef::Stack)"
        ));
        Some(())
    }

    /// Lowers `K:Equip:...ReduceCost$ X` when `X` names `Targeted$CardPower`
    /// (*Belt of Giant Strength*), `Targeted$CardNumColors` (*Dragonfire Blade*),
    /// or `Targeted$CardCounters.<kind>` (*Warrior's Blades*).
    pub(super) fn try_equip_target_metric(&mut self, raw: &str) -> Option<()> {
        let trimmed = raw.trim();
        let def = if trimmed.starts_with("Targeted$") {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.trim().to_string()
        };
        let metric = if def == "Targeted$CardPower" {
            "EquipTargetMetric::Power".to_string()
        } else if def == "Targeted$CardNumColors" {
            "EquipTargetMetric::ColorCount".to_string()
        } else {
            let kind_raw = def.strip_prefix("Targeted$CardCounters.")?;
            let ck = counter_kind(kind_raw.trim())?;
            format!("EquipTargetMetric::Counters({ck})")
        };
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!(
                "Modifier::EquipCostsLessByTargetAmount {{ who: PlayerRel::You, metric: {metric} }}"
            ),
        ));
        Some(())
    }

    /// Lowers `ReduceCost$ <mana> | ReduceAmount$ <svar>` (*Flying Drone*, *Kami of Jealous Thirst*)
    /// into `CostReduction::IfMana`, and `ReduceCost$ X` targeting `AllTargeted$Valid ...`
    /// (*Raft Security Officer*, *Wayta, Trainer Prodigy*) into `CostReduction::IfTargetsMatch`.
    pub(super) fn try_activated_if_mana_or_targets(
        &mut self,
        raw_cost: &str,
        reduce_amount: Option<&str>,
    ) -> Option<String> {
        if let Some(amt_var) = reduce_amount {
            let mana = printed_mana(raw_cost.trim())?;
            let def = if amt_var.trim().starts_with("Count$") {
                amt_var.trim().to_string()
            } else {
                self.svars.get(amt_var.trim())?.trim().to_string()
            };
            let compare = def.strip_prefix("Count$Compare ")?;
            let (lhs_part, else_str) = compare.rsplit_once('.')?;
            if else_str.trim() != "0" {
                return None;
            }
            let (cond_spec, then_str) = lhs_part.rsplit_once('.')?;
            if then_str.trim() != "1" {
                return None;
            }
            let (var, cmp) = cond_spec.trim().rsplit_once(' ')?;
            let cond = self.svar_condition_clause(var.trim(), cmp.trim())?;
            if cond.contains("Condition::XAtLeast") {
                return None;
            }
            return Some(format!(
                ", cost_reduction = Some(CostReduction::IfMana {{ condition: {cond}, mana: mana!(\"{mana}\") }})"
            ));
        }

        let def = if raw_cost.trim().starts_with("AllTargeted$")
            || raw_cost.trim().starts_with("Count$")
        {
            raw_cost.trim().to_string()
        } else {
            self.svars.get(raw_cost.trim())?.trim().to_string()
        };
        let (valid_spec, min_count, amount) =
            if let Some(valid) = def.strip_prefix("AllTargeted$Valid ") {
                (valid.trim().to_string(), 1u8, 1u32)
            } else {
                let compare = def.strip_prefix("Count$Compare ")?;
                let (lhs_part, else_str) = compare.rsplit_once('.')?;
                if else_str.trim() != "0" {
                    return None;
                }
                let (cond_spec, then_str) = lhs_part.rsplit_once('.')?;
                let amt = then_str.trim().parse::<u32>().ok().filter(|&n| n > 0)?;
                let (var, cmp) = cond_spec.trim().rsplit_once(' ')?;
                let var_def = self.svars.get(var.trim())?.trim().to_string();
                let valid = var_def
                    .strip_prefix("AllTargeted$Valid ")?
                    .trim()
                    .to_string();
                let min_n = cmp
                    .trim()
                    .strip_prefix("GE")
                    .or_else(|| cmp.trim().strip_prefix("EQ"))
                    .and_then(|s| s.parse::<u8>().ok())
                    .filter(|&n| n > 0)?;
                (valid, min_n, amt)
            };

        let expr = self.filter_expr(&valid_spec)?;
        let filter_ref = if simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("TGT_RED", &expr))
        };
        Some(format!(
            ", cost_reduction = Some(CostReduction::IfTargetsMatch {{ filter: {filter_ref}, min_count: {min_count}, amount: {amount} }})"
        ))
    }
}

fn strip_in_zone_battlefield(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(base) = raw.strip_suffix(".inZoneBattlefield") {
        return base.to_string();
    }
    if let Some(base) = raw.strip_suffix("+inZoneBattlefield") {
        return base.to_string();
    }
    raw.replacen(".inZoneBattlefield+", ".", 1)
        .replacen("+inZoneBattlefield+", "+", 1)
}

fn strip_scope_atom(raw: &str, scope: &str) -> Option<String> {
    let mut alts = Vec::new();
    for raw_alt in raw.split(',') {
        let alt = raw_alt.trim();
        let (base, qual) = alt.split_once('.')?;
        let atoms = qual
            .split('+')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect::<Vec<_>>();
        if !atoms.contains(&scope) {
            return None;
        }
        let rest = atoms
            .into_iter()
            .filter(|a| *a != scope)
            .collect::<Vec<_>>();
        if rest.is_empty() {
            alts.push(base.trim().to_string());
        } else {
            alts.push(format!("{}.{}", base.trim(), rest.join("+")));
        }
    }
    Some(alts.join(","))
}
