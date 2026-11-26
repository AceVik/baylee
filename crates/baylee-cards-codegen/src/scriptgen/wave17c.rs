//! Wave 17c `scriptgen` lowerings:
//! 1. `S: Mode$ ReduceCost` & `S: Mode$ ForetellAnyTurn`:
//!    - `ValidSpell$ Static.Foretelling` / `Type$ Foretell` (`FirstForetell$ True`) ->
//!      `Modifier::ForetellActionCostsLess` (*Cosmos Charger*, *Ranar the Ever-Watchful*)
//!    - `S: Mode$ ForetellAnyTurn` -> `Modifier::MayForetellAnyTurn` (*Cosmos Charger*)
//!    - `ValidSpell$ Static.Unlock` -> `Modifier::UnlockCostsLess` (*Inquisitive Glimmer*)
//!    - `ValidSpell$ Activated.Equip | ValidTarget$ ...` ->
//!      `Modifier::EquipCostsLessIfTargets` (*Fervent Champion*, *Cloud*, *Strong Back*, …)
//!    - `ValidSpell$ Spell.Bargain` -> `SpellCastMetric::Bargained`
//!      (*Hamlet Glutton*, *Ice Out*, *Johann's Stopgap*)
//!    - `Color$ <C> | Amount$ <dynamic>` -> `Modifier::SpellsCostLessManaByAmount`
//!      (*Demilich*, *Khalni Hydra*)
//!    - `TargetedObjectsDistinct$Valid <Filter>` ->
//!      `SpellCastMetric::DistinctTargetsMatching` (*Battlefield Thaumaturge*)
//! 2. `S: Mode$ RaiseCost`:
//!    - `BeholdExile<1/<Valid>>` -> `CostPart::BeholdAndExile` (*Champion of the Clachan* cycle)
//!    - `ValidSpell$ Activated.Loyalty` with `AddCounter`/`SubCounter` ->
//!      `Modifier::AdjustLoyaltyCost` (*Carth the Lion*)
//!    - `ValidSpell$ Activated.Loyalty` with `Amount$ N` ->
//!      `Modifier::LoyaltyAbilitiesCostMore` (*Eidolon of Obstruction*)
//!    - `ValidSpell$ Activated.!ManaAbility` / `Spell,Activated.!ManaAbility` ->
//!      `Modifier::NonManaAbilitiesCostMore` (*Suppression Field*, *Anointed Peacekeeper*, *Tithe Taker*)
//!    - `ValidTarget$ ...` on `Type$ Spell` / `Type$ Ability` ->
//!      `Modifier::RaiseCostIfTargets` (*Dragon's Prey*, *Vanish into Eternity*, *Kopala*, *Terror of the Peaks*)
//! 3. `AffectedZone$ Graveyard` static grants & permissions:
//!    - `Unearth:<cost>`, `Scavenge:CardManaCost`, `Dredge:<N>`, `Sneak:<cost>`
//!      (*Sedris*, *Dregscape Sliver*, *Mishra*, *Solemn Doomguide*, *Varolz*, *Young Deathclaws*, *The Necrobloom*, *Ninja Teen*)
//!    - `RemoveAllAbilities$ True` -> `Modifier::LoseAllAbilities` (*Yixlid Jailer*)
//!    - `MayPlay$ True | ValidSA$ Spell.(Mutate|Blitz|Warp|Sneak)` ->
//!      `Modifier::MayCastFromGraveyardUsingAlt` (*Brokkos*, *Tenacious Underdog*, *Timeline Culler*, *Ninja Teen*)
//!    - `Affected$ Card.surveilledThisTurn` -> `Filter::SurveilledByYouThisTurn` (*Eye of Duskmantle*)
//! 4. `T: Mode$ DiscardedAll` -> `Trigger::OneOrMoreCardsDiscarded` +
//!    `ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card.TriggeredCards` (*Toluz*)
//! 5. `DB$/AB$/SP$ Pump` & `PumpAll` with `NumAtt$ Double` / `NumDef$ Double` ->
//!    `Effect::DoublePT` (*Casey Jones*, *Devilish Valet*, *God-Eternal Rhonas*, *Junk Jet*, *Unnatural Growth*, *Wolverine*, …)

use super::{
    Params, Tx, printed_mana,
    statics::{StaticEffectZone, simple_const_filter},
};

fn activator_player_rel(raw: Option<&str>) -> Option<&'static str> {
    Some(match raw {
        None | Some("Player") => "PlayerRel::EachPlayer",
        Some("You") => "PlayerRel::You",
        Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
        Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
            "PlayerRel::EnchantedPlayer"
        }
        Some(_) => return None,
    })
}

impl Tx<'_> {
    // =========================================================================
    // 1. ReduceCost & ForetellAnyTurn
    // =========================================================================

    /// `S: Mode$ ForetellAnyTurn | ValidPlayer$ You` (*Cosmos Charger*).
    pub(super) fn wave17c_foretell_any_turn(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("static ability `S: Mode$ ForetellAnyTurn`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ForetellAnyTurn`".into());
        };
        let Some(who) = activator_player_rel(p.take("ValidPlayer").as_deref()) else {
            return self.deny("static ability `S: Mode$ ForetellAnyTurn`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ForetellAnyTurn`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::MayForetellAnyTurn {{ who: {who} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost` for the `{2}` foretell special action (`ValidSpell$ Static.Foretelling`
    /// on *Cosmos Charger*, `Type$ Foretell | FirstForetell$ True` on *Ranar the Ever-Watchful*).
    pub(super) fn wave17c_reduce_cost_foretell(&mut self, mut p: Params) -> Option<()> {
        let valid_spell = p.take("ValidSpell");
        let ty = p.take("Type");
        if !matches!(
            (ty.as_deref(), valid_spell.as_deref()),
            (None, Some("Static.Foretelling"))
                | (Some("Foretell"), None | Some("Static.Foretelling"))
        ) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if !matches!(p.take("ValidCard").as_deref(), None | Some("Card")) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let first_each_turn_only = match p.take("FirstForetell").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
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
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::ForetellActionCostsLess {{ who: {who}, amount: {amount}, first_each_turn_only: {first_each_turn_only} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost | ValidSpell$ Static.Unlock | Activator$ You | Amount$ N`
    /// (*Inquisitive Glimmer*, `CR 709.5e`).
    pub(super) fn wave17c_reduce_cost_unlock(&mut self, mut p: Params) -> Option<()> {
        if p.take("ValidSpell").as_deref() != Some("Static.Unlock")
            || !matches!(p.take("Type").as_deref(), None | Some("Unlock"))
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
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::UnlockCostsLess {{ who: {who}, amount: {amount} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost | ValidSpell$ Activated.Equip | ValidTarget$ <T> | Amount$ N`
    /// (*Fervent Champion*, *Cloud, Planet's Champion*, *Dwarven Mauler*, *Helitrooper*, *Strong Back*).
    pub(super) fn wave17c_reduce_cost_equip_if_targets(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if !matches!(
            p.take("AffectedZone").as_deref(),
            None | Some("Battlefield")
        ) || !matches!(p.take("Type").as_deref(), None | Some("Ability"))
            || !matches!(
                p.take("ValidCard").as_deref(),
                None | Some("Card" | "Equipment")
            )
            || !matches!(p.take("MinMana").as_deref(), None | Some("0"))
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
            return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
        };
        let raw_target = p.take("ValidTarget")?;
        let target_filter = self.parse_target_cost_filter(&raw_target, false)?;
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<u8>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::EquipCostsLessIfTargets {{ who: {who}, target_filter: {target_filter}, amount: {amount} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost | ValidSpell$ Spell.Bargain | ... | Amount$ N`
    /// (*Hamlet Glutton*, *Ice Out*, *Johann's Stopgap*, `CR 702.166a`).
    pub(super) fn wave17c_reduce_cost_bargain(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        if !matches!(p.take("Type").as_deref(), None | Some("Spell")) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let (is_self_zone, zone) = match p.take("EffectZone").as_deref() {
            Some("All" | "Stack") => (true, StaticEffectZone::Explicit("ZoneRef::Stack")),
            raw => match Self::static_effect_zone(raw, false) {
                Some(z) => (false, z),
                None => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            },
        };
        let (outer_filter, who, inner_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You"))
                || p.take("ValidCard").as_deref() != Some("Card.Self")
            {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let expr = self.filter_expr_in_zone(&raw_card, true)?;
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<i8>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let delta = -amount;
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &format!(
                "Modifier::AdjustSpellCostByCast {{ who: {who}, filter: {inner_filter}, delta_per_unit: {delta}, metric: SpellCastMetric::Bargained }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost | Color$ <C> | Amount$ <SVar/Count>`
    /// (*Demilich*, *Khalni Hydra*, `CR 118.7b–c`).
    pub(super) fn wave17c_reduce_cost_colored_by_amount(
        &mut self,
        mut p: Params,
        spell_type: Option<&str>,
        valid_spell: Option<&str>,
        is_self_zone: bool,
        zone: StaticEffectZone,
    ) -> Option<()> {
        if !matches!(spell_type, None | Some("Spell"))
            || !matches!(valid_spell, None | Some("Spell"))
            || p.peek("Relative") == Some("True")
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let (outer_filter, who, inner_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You"))
                || p.take("ValidCard").as_deref() != Some("Card.Self")
            {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let expr = self.filter_expr_in_zone(&raw_card, true)?;
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let color_raw = p.take("Color")?;
        let Some(mana) = printed_mana(&color_raw) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let colored_only = match p.take("IgnoreGeneric").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let amt_raw = p.take("Amount")?;
        let amt_def = if amt_raw.starts_with("Count$") || amt_raw.starts_with("SVar$") {
            amt_raw
        } else {
            let Some(def) = self.svars.get(amt_raw.trim()).cloned() else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            def
        };
        let Some(amt_expr) = self
            .wave14_static_amount(&amt_def)
            .or_else(|| self.static_count_expr_ext(&amt_def))
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &format!(
                "Modifier::SpellsCostLessManaByAmount {{ who: {who}, filter: {inner_filter}, unit: mana!(\"{mana}\"), colored_only: {colored_only}, amount: &{amt_expr} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    /// Strips `.YouCtrl` / `+YouCtrl` when every comma-separated branch of
    /// `raw_card` requires `YouCtrl` and carries no other `Ctrl`/`Own` atom
    /// (*Battlefield Thaumaturge*: `ValidCard$ Instant.YouCtrl,Sorcery.YouCtrl`).
    pub(super) fn wave17c_strip_all_you_ctrl(raw_card: &str) -> Option<String> {
        let mut out = Vec::new();
        for raw_alt in raw_card.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return None;
            }
            let (base, qual) = alt.split_once('.')?;
            let base = base.trim();
            if base.is_empty() || base.contains("Ctrl") || base.contains("Own") {
                return None;
            }
            let mut saw_you = false;
            let mut kept = Vec::new();
            for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if atom == "YouCtrl" {
                    saw_you = true;
                } else if atom.contains("Ctrl") || atom.contains("Own") {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if !saw_you {
                return None;
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    /// `TargetedObjectsDistinct$Valid <Filter>` -> `SpellCastMetric::DistinctTargetsMatching(&...)`
    /// (*Battlefield Thaumaturge*).
    pub(super) fn wave17c_distinct_targets_matching_metric(
        &mut self,
        expr: &str,
        relative: bool,
    ) -> Option<String> {
        if !relative {
            return None;
        }
        let valid = expr.strip_prefix("TargetedObjectsDistinct$Valid ")?.trim();
        if valid.is_empty() || valid.contains('$') || valid.contains('/') {
            return None;
        }
        let f_ref = self.parse_target_cost_filter(valid, false)?;
        Some(format!("SpellCastMetric::DistinctTargetsMatching({f_ref})"))
    }

    // =========================================================================
    // 2. RaiseCost & BeholdExile
    // =========================================================================

    /// Parses `BeholdExile<1/<valid>[/<desc>]>` into `BeholdAndExile(&<filter>)`
    /// (*Champion of the Clachan* cycle, `CR 701.4a`, `CR 607.2a`).
    pub(super) fn wave17c_parse_behold_exile_cost_token(&mut self, token: &str) -> Option<String> {
        let Some(inner) = token
            .strip_prefix("BeholdExile<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let mut fields = inner.splitn(3, '/');
        let (Some(count_str), Some(valid)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        if count_str.trim() != "1" {
            return self.deny(format!("cost `{token}`"));
        }
        let valid = valid.trim();
        if valid.is_empty()
            || valid.split(',').any(|alt| {
                alt.split(['.', '+']).any(|atom| {
                    let atom = atom.trim();
                    matches!(
                        atom,
                        "CARDNAME" | "NICKNAME" | "Self" | "Other" | "StrictlyOther"
                    ) || atom.contains("Ctrl")
                        || atom.contains("Own")
                })
            })
        {
            return self.deny(format!("cost `{token}`"));
        }
        let expr = self.filter_expr_in_zone(valid, true)?;
        let filter_name = self.body.filter_static("COST", &expr);
        Some(format!("BeholdAndExile(&{filter_name})"))
    }

    /// Dispatches Wave 17c `S: Mode$ RaiseCost` shapes (`Activated.Loyalty`,
    /// `Activated.!ManaAbility`, `Spell,Activated.!ManaAbility`, and `ValidTarget$`).
    /// Returns `None` when `p` is not one of those shapes so caller fallbacks continue.
    pub(super) fn wave17c_try_raise_cost_extra(&mut self, mut p: Params) -> Option<Result<(), ()>> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return None;
        }
        if p.peek("ValidSpell") == Some("Activated.Loyalty") {
            return Some(self.wave17c_raise_cost_loyalty(p).ok_or(()));
        }
        if matches!(
            p.peek("ValidSpell"),
            Some(
                "Activated.!ManaAbility"
                    | "Spell,Activated.!ManaAbility"
                    | "Activated.!ManaAbility,Spell"
            )
        ) {
            return Some(self.wave17c_raise_cost_non_mana_ability(p).ok_or(()));
        }
        if p.has("ValidTarget")
            && matches!(p.peek("Type"), Some("Spell" | "Ability"))
            && p.peek("OnlyFirstSpell") != Some("True")
        {
            return Some(self.wave17c_raise_cost_if_targets(p).ok_or(()));
        }
        None
    }

    fn wave17c_raise_cost_loyalty(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        if !matches!(p.take("Type").as_deref(), None | Some("Ability"))
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
        let raw_card = p
            .take("ValidCard")
            .unwrap_or_else(|| "Planeswalker".to_string());
        let filter_expr = self.filter_expr(&raw_card)?;
        let filter_ref = if simple_const_filter(&filter_expr) {
            format!("&{filter_expr}")
        } else {
            format!("&{}", self.body.filter_static("LOYALTY_TAX", &filter_expr))
        };
        let amount = p.take("Amount");
        let cost = p.take("Cost");
        let modifier = match (amount.as_deref(), cost.as_deref()) {
            (Some(amt_raw), None) => {
                let Some(n) = amt_raw.trim().parse::<u8>().ok().filter(|&n| n > 0) else {
                    return self.deny("static ability `S: Mode$ RaiseCost`".into());
                };
                format!(
                    "Modifier::LoyaltyAbilitiesCostMore {{ who: {who}, filter: {filter_ref}, amount: {n} }}"
                )
            }
            (None, Some(cost_raw)) => {
                let trimmed = cost_raw.trim();
                let delta = if let Some(n_str) = trimmed
                    .strip_prefix("AddCounter<")
                    .and_then(|s| s.strip_suffix("/LOYALTY>"))
                {
                    n_str.trim().parse::<i8>().ok().filter(|&n| n > 0)?
                } else if let Some(n_str) = trimmed
                    .strip_prefix("SubCounter<")
                    .and_then(|s| s.strip_suffix("/LOYALTY>"))
                {
                    let n = n_str.trim().parse::<i8>().ok().filter(|&n| n > 0)?;
                    -n
                } else {
                    return self.deny("static ability `S: Mode$ RaiseCost`".into());
                };
                format!(
                    "Modifier::AdjustLoyaltyCost {{ who: {who}, filter: {filter_ref}, delta: {delta} }}"
                )
            }
            _ => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn wave17c_raise_cost_non_mana_ability(&mut self, mut p: Params) -> Option<()> {
        let valid_spell = p.take("ValidSpell")?;
        let also_spells = matches!(
            valid_spell.as_str(),
            "Spell,Activated.!ManaAbility" | "Activated.!ManaAbility,Spell"
        );
        if !matches!(p.take("Type").as_deref(), None | Some("Ability")) || p.has("AffectedZone") {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let raw_act = p.take("Activator");
        let Some(who) = activator_player_rel(raw_act.as_deref()) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
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
        if also_spells {
            let Some(spell_valid) = Self::apply_reduce_activator(&raw_card, raw_act.as_deref())
            else {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            };
            let spell_filter = self.filter_expr(&spell_valid)?;
            self.body.abilities.push(Self::format_static_expr(
                &spell_filter,
                &format!("Modifier::SpellsCostMore({amount})"),
                zone,
                &cond,
            ));
        }
        let ab_expr = if raw_card == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr_in_zone(&raw_card, true)?
        };
        let ab_ref = if simple_const_filter(&ab_expr) {
            format!("&{ab_expr}")
        } else {
            format!("&{}", self.body.filter_static("AB_TAX", &ab_expr))
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!(
                "Modifier::NonManaAbilitiesCostMore {{ who: {who}, filter: {ab_ref}, amount: {amount} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    #[allow(clippy::too_many_lines)]
    fn wave17c_raise_cost_if_targets(&mut self, mut p: Params) -> Option<()> {
        let abilities = match p.take("Type").as_deref() {
            Some("Spell") => false,
            Some("Ability") => true,
            _ => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        };
        if abilities {
            if !matches!(p.take("ValidSpell").as_deref(), None | Some("Activated"))
                || !matches!(
                    p.take("AffectedZone").as_deref(),
                    None | Some("Battlefield")
                )
            {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            }
        } else if !matches!(p.take("ValidSpell").as_deref(), None | Some("Spell")) {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let (is_self_zone, zone) = match p.take("EffectZone").as_deref() {
            Some("All" | "Stack") if !abilities => {
                (true, StaticEffectZone::Explicit("ZoneRef::Stack"))
            }
            raw => match Self::static_effect_zone(raw, false) {
                Some(z) => (false, z),
                None => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
            },
        };
        let raw_target = p.take("ValidTarget")?;
        let (outer_filter, who, spell_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You"))
                || p.take("ValidCard").as_deref() != Some("Card.Self")
            {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let Some(who) = activator_player_rel(p.take("Activator").as_deref()) else {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            };
            let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            }
            let expr = if abilities {
                if raw_card == "Card" {
                    "Filter::Any".to_string()
                } else {
                    self.filter_expr_in_zone(&raw_card, true)?
                }
            } else {
                self.filter_expr_in_zone(&raw_card, true)?
            };
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let target_filter = self.parse_target_cost_filter(&raw_target, is_self_zone)?;
        let (extra_mana, extra_life) = match (p.take("Amount"), p.take("Cost")) {
            (Some(amt_raw), None) => {
                let Some(n) = amt_raw.trim().parse::<u32>().ok().filter(|&n| n > 0) else {
                    return self.deny("static ability `S: Mode$ RaiseCost`".into());
                };
                (format!("mana!(\"{{{n}}}\")"), 0_u32)
            }
            (None, Some(cost_raw)) => {
                let trimmed = cost_raw.trim();
                if let Some(life_str) = trimmed
                    .strip_prefix("PayLife<")
                    .and_then(|s| s.strip_suffix('>'))
                {
                    let Some(life) = life_str.trim().parse::<u32>().ok().filter(|&n| n > 0) else {
                        return self.deny("static ability `S: Mode$ RaiseCost`".into());
                    };
                    ("ManaCost::ZERO".to_string(), life)
                } else {
                    let Some(mana) = printed_mana(trimmed) else {
                        return self.deny("static ability `S: Mode$ RaiseCost`".into());
                    };
                    (format!("mana!(\"{mana}\")"), 0_u32)
                }
            }
            _ => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &format!(
                "Modifier::RaiseCostIfTargets {{ who: {who}, spell_filter: {spell_filter}, target_filter: {target_filter}, extra_mana: {extra_mana}, extra_life: {extra_life}, abilities: {abilities} }}"
            ),
            zone,
            &cond,
        ));
        Some(())
    }

    // =========================================================================
    // 3. AffectedZone$ Graveyard grants, LoseAllAbilities, & ValidSA$ alt-cast
    // =========================================================================

    /// Wave 17c graveyard grant keywords (`Unearth:<cost>`, `Scavenge:CardManaCost`,
    /// `Dredge:<N>`, `Sneak:<cost>`).
    pub(super) fn wave17c_graveyard_grant_modifier(
        kw: &str,
        allow_offboard_static_kw: bool,
    ) -> Option<(String, bool)> {
        let kw = kw.trim();
        if let Some(cost_raw) = kw.strip_prefix("Unearth:") {
            let cost_raw = cost_raw.trim();
            let mana_expr = if cost_raw == "0" {
                "ManaCost::ZERO".to_string()
            } else {
                let mana = printed_mana(cost_raw)?;
                if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
                    return None;
                }
                format!("mana!(\"{mana}\")")
            };
            return Some((format!("Modifier::GrantsUnearth({mana_expr})"), false));
        }
        if kw == "Scavenge:CardManaCost" {
            return Some(("Modifier::GrantsScavengeManaCost".to_string(), false));
        }
        if kw == "Harmonize" {
            return Some(("Modifier::GrantsHarmonizeManaCost".to_string(), false));
        }
        if matches!(kw, "Mayhem:CardManaCost" | "Mayhem:ManaCost") {
            return Some(("Modifier::GrantsMayhemManaCost".to_string(), false));
        }
        if kw == "Beam me up" {
            return Some((
                "Modifier::GrantsBeamMeUpManaCost { cant_return_source: false }".to_string(),
                false,
            ));
        }
        if allow_offboard_static_kw {
            if let Some(n_raw) = kw.strip_prefix("Dredge:") {
                let n = n_raw.trim().parse::<u8>().ok().filter(|&n| n > 0)?;
                return Some((format!("Modifier::GrantsDredge({n})"), true));
            }
            if let Some(cost_raw) = kw.strip_prefix("Sneak:") {
                let cost_raw = cost_raw.trim();
                let cost_expr = if cost_raw == "0" {
                    "Cost::FREE".to_string()
                } else {
                    let mana = printed_mana(cost_raw)?;
                    if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
                        return None;
                    }
                    format!("cost!(\"{mana}\")")
                };
                return Some((format!("Modifier::Sneak({cost_expr})"), true));
            }
        }
        None
    }

    /// `S: Mode$ Continuous | AffectedZone$ Graveyard | RemoveAllAbilities$ True`
    /// (*Yixlid Jailer*).
    pub(super) fn wave17c_graveyard_lose_all_abilities(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || p.take("RemoveAllAbilities").as_deref() != Some("True")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let base_filter = self.filter_expr_in_zone(&affected, true)?;
        let filter = if base_filter == "Filter::Any" {
            "Filter::InZone(ZoneRef::Graveyard)".to_string()
        } else {
            format!("Filter::And(&[{base_filter}, Filter::InZone(ZoneRef::Graveyard)])")
        };
        let mut expr = Self::static_expr(&filter, "Modifier::LoseAllAbilities");
        if !cond.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{cond})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    /// `S: Mode$ Continuous | AffectedZone$ Graveyard | MayPlay$ True | ValidSA$ Spell.(Mutate|Blitz|Warp|Sneak)`
    /// (*Brokkos, Apex of Forever*, *Tenacious Underdog*, *Timeline Culler*, *Ninja Teen*).
    pub(super) fn wave17c_graveyard_alt_play(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || p.take("MayPlay").as_deref() != Some("True")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let kind = match p.take("ValidSA").as_deref() {
            Some("Spell.Mutate") => "GraveyardAltKind::Mutate",
            Some("Spell.Blitz") => "GraveyardAltKind::Blitz",
            Some("Spell.Warp") => "GraveyardAltKind::Warp",
            Some("Spell.Sneak") => "GraveyardAltKind::Sneak",
            _ => return self.deny("static ability reaching `AffectedZone$ Graveyard`".into()),
        };
        let effect_zone = p.take("EffectZone");
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        if effect_zone.as_deref() == Some("Graveyard") {
            if affected != "Card.Self" || self.is_land_card_type() {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
            }
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::MayCastFromGraveyardUsingAlt({kind}), \
                 zone = ZoneRef::Graveyard{cond})"
            ));
            return Some(());
        }
        if !matches!(effect_zone.as_deref(), None | Some("Battlefield")) {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let mut rewritten = Vec::new();
        for alt in affected.split(',').map(str::trim) {
            if alt.is_empty() || alt.contains("Opp") || alt.contains("Dont") {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
            }
            if !alt.contains("YouCtrl") && !alt.contains("YouOwn") {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
            }
            rewritten.push(alt.replace("YouCtrl", "YouOwn"));
        }
        let base_filter = self.filter_expr_in_zone(&rewritten.join(","), true)?;
        let filter = format!("Filter::And(&[{base_filter}, Filter::InZone(ZoneRef::Graveyard)])");
        let mut expr = Self::static_expr(
            &filter,
            &format!("Modifier::MayCastFromGraveyardUsingAlt({kind})"),
        );
        if !cond.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{cond})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    // =========================================================================
    // 4. T: Mode$ DiscardedAll & Card.TriggeredCards graveyard exile
    // =========================================================================

    /// `T: Mode$ DiscardedAll` -> `Trigger::OneOrMoreCardsDiscarded`
    /// (*Rielle*, *Veronica*, *Toluz*, *Cryptcaller Chariot*, *Hostile Investigator*, *Tinybones*, …).
    pub(super) fn wave17c_discarded_all_trigger(
        &mut self,
        p: &mut Params,
        zoned: bool,
    ) -> Option<String> {
        if !zoned {
            return self.deny("a `DiscardedAll` trigger with no `TriggerZones$`".to_string());
        }
        if p.has("Random") {
            return self.deny("a `DiscardedAll` trigger with `Random$`".to_string());
        }
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let first_this_turn = match p.take("FirstTime").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`FirstTime$ {other}`")),
        };
        let oracle_lower = self.oracle.to_ascii_lowercase();
        let (who, per_player) = match p.take("ValidPlayer").as_deref() {
            Some("You") => ("PlayerRel::You", true),
            Some("Opponent" | "Player.Opponent") => {
                let batch_all = oracle_lower.contains("one or more opponents");
                ("PlayerRel::EachOpponent", !batch_all)
            }
            None | Some("Player") => {
                let batch_all = oracle_lower.contains("one or more players")
                    || oracle_lower.contains("you or another player");
                ("PlayerRel::EachPlayer", !batch_all)
            }
            Some(other) => return self.deny(format!("a `DiscardedAll` trigger by `{other}`")),
        };
        let valid = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        if valid.split(['.', '+', ',']).any(|atom| atom == "Self")
            || valid.contains("Ctrl")
            || valid.contains("Own")
        {
            return self.deny(format!("a `DiscardedAll` trigger on `{valid}`"));
        }
        let filter = if valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&valid, true)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!(
            "Trigger::OneOrMoreCardsDiscarded {{ who: {who}, filter: {filter}, \
             first_this_turn: {first_this_turn}, per_player: {per_player} }}"
        ))
    }

    /// `DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card.TriggeredCards`
    /// under `T: Mode$ DiscardedAll` (*Toluz, Clever Conductor*).
    pub(super) fn wave17c_exile_triggered_discarded_cards(
        &mut self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        if p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("ChangeType").as_deref() != Some("Card.TriggeredCards")
            || p.take("Defined").is_some()
        {
            return None;
        }
        p.take("IsCurse");
        if !p.exhausted() {
            return None;
        }
        let move_kind = if self.has_linked_exile_reader() {
            "ChooseMove::ExileLinked"
        } else {
            "ChooseMove::Exile"
        };
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Found, \
             filter: &Filter::Any, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: {move_kind}, \
             then: &[], for_unchosen: &[] }}"
        )])
    }

    // =========================================================================
    // 5. Pump & PumpAll with NumAtt$ Double / NumDef$ Double
    // =========================================================================

    /// `DB$/AB$/SP$ Pump` with `NumAtt$ Double` and/or `NumDef$ Double` ->
    /// `Effect::DoublePT` (`CR 701.10b–c`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave17c_pump_double_effect(
        &mut self,
        p: &mut Params,
        target: &str,
        second: bool,
    ) -> Option<Vec<String>> {
        if second || p.has("KWChoice") || p.peek("Duration") == Some("Perpetual") {
            return self.deny("pump amount `Double`".to_string());
        }
        let double_power = match p.take("NumAtt").as_deref().map(str::trim) {
            Some("Double") => true,
            None | Some("0" | "+0") => false,
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        let double_toughness = match p.take("NumDef").as_deref().map(str::trim) {
            Some("Double") => true,
            None | Some("0" | "+0") => false,
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        if !double_power && !double_toughness {
            return None;
        }
        let (keywords, prevent_mods) = match p.take("KW") {
            None => ("KeywordSet::EMPTY".to_string(), Vec::new()),
            Some(kw) => self.pump_kw_and_prevent(&kw, p.opponents_only_lure)?,
        };
        if !prevent_mods.is_empty() {
            return self.deny("pump amount `Double`".to_string());
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        p.take("IsCurse");
        let defined = match p.take("Defined") {
            None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell => {
                Some("Self".to_string())
            }
            other => other,
        };
        let subject = match defined.as_deref() {
            Some("Self") if !self.on_a_spell => "DoublePtSubject::Source".to_string(),
            Some("Equipped" | "Enchanted")
                if !self.on_a_spell && target == "TargetSpec::AnyPlayer" =>
            {
                "DoublePtSubject::Attached".to_string()
            }
            Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                    && self
                        .block_line
                        .as_ref()
                        .is_some_and(|(role, _, _)| *role == super::BlockRole::Blocks)) =>
            {
                "DoublePtSubject::EventObject".to_string()
            }
            Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                if self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(role, _, _)| *role == super::BlockRole::Blocked)) =>
            {
                "DoublePtSubject::EventObject".to_string()
            }
            Some(
                "TriggeredCardLKICopy"
                | "TriggeredCard"
                | "TriggeredNewCardLKICopy"
                | "TriggeredNewCard",
            ) if matches!(
                self.trigger_mode.as_deref(),
                Some(
                    "ChangesZone"
                        | "Exerted"
                        | "Enlisted"
                        | "Taps"
                        | "Untaps"
                        | "TurnFaceUp"
                        | "Transformed"
                )
            ) =>
            {
                "DoublePtSubject::EventObject".to_string()
            }
            Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                if self.in_delayed && target == "TargetSpec::AnyPlayer" =>
            {
                "DoublePtSubject::EventObject".to_string()
            }
            None | Some("Targeted" | "ParentTarget")
                if target.starts_with("TargetSpec::Object(") =>
            {
                "DoublePtSubject::Targets".to_string()
            }
            Some(raw) if target == "TargetSpec::AnyPlayer" && raw.starts_with("Valid ") => {
                let expr = self.filter_expr(raw["Valid ".len()..].trim())?;
                let name = self.body.filter_static("DOUBLE_PT", &expr);
                format!("DoublePtSubject::Filter(&{name})")
            }
            _ => return self.deny("pump amount `Double`".to_string()),
        };
        Some(vec![format!(
            "Effect::DoublePT {{ subject: {subject}, double_power: {double_power}, \
             double_toughness: {double_toughness}, keywords: {keywords}, duration: {duration} }}"
        )])
    }

    /// `DB$/AB$/SP$ PumpAll` with `NumAtt$ Double` and/or `NumDef$ Double` ->
    /// `Effect::DoublePT { subject: DoublePtSubject::Filter(&...), ... }`.
    pub(super) fn wave17c_pump_all_double_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.peek("Duration") == Some("Perpetual") {
            return self.deny("pump amount `Double`".to_string());
        }
        let valid = p.take("ValidCards")?;
        let double_power = match p.take("NumAtt").as_deref().map(str::trim) {
            Some("Double") => true,
            None | Some("0" | "+0") => false,
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        let double_toughness = match p.take("NumDef").as_deref().map(str::trim) {
            Some("Double") => true,
            None | Some("0" | "+0") => false,
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        if !double_power && !double_toughness {
            return None;
        }
        let (keywords, extra_mods) = match p.take("KW") {
            None => ("KeywordSet::EMPTY".to_string(), Vec::new()),
            Some(kw) => self.pump_kw_and_prevent(&kw, p.opponents_only_lure)?,
        };
        if !extra_mods.is_empty() {
            return self.deny("pump amount `Double`".to_string());
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return self.deny("pump amount `Double`".to_string()),
        };
        p.take("IsCurse");
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("DOUBLE_PT", &expr);
        Some(vec![format!(
            "Effect::DoublePT {{ subject: DoublePtSubject::Filter(&{filter}), \
             double_power: {double_power}, double_toughness: {double_toughness}, \
             keywords: {keywords}, duration: {duration} }}"
        )])
    }
}
