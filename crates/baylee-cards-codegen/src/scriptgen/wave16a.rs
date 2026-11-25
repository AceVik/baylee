//! Wave 16a `scriptgen` lowering — Trigger/Cast Context Amounts & Turn-History Conditions:
//!
//! - Cast mana & convoke amounts (`Amount::ManaColorSpentToCast`, `Amount::ColorsOfManaSpentToCast`,
//!   `Amount::ConvokedCount`, `Amount::EventManaSpentToCast`, `Amount::TargetManaSpentToCast`):
//!   - `Adamant$ <Color>` / `Adamant$ Any` (*Ardenvale Paladin*, *Henge Walker*)
//!   - `CastSA>Count$Adamant_2.<Color>.2.0` (*Catharsis* Elemental Incarnation cycle)
//!   - `Count$EachSpentToCast.<C>` / `Count$EachSpentToCast <C>` (*Bladecoil Serpent*, *Clay Champion*)
//!   - `Count$Converge` (*Crystalline Crawler*, *Woodland Wanderer*)
//!   - `Convoked$Amount` (*Ancient Imperiosaur*)
//!   - `TriggeredCard$CastTotalManaSpent` / `TriggeredSpellAbility$ManaSpent` (*Aetherflux Conduit*, *Tellah, Great Sage*)
//!   - `Targeted$CastTotalManaSpent` (*Unravel*)
//! - Turn-history amounts & conditions (`Phase.CheckSVar`, `ChangesZone.CheckSVar`, `Draw.ConditionCheckSVar`):
//!   - `Count$ThisTurnEntered_Hand_from_Battlefield_...` & `Library_from_Battlefield_...` (*Barrin, Tolarian Archmage*)
//!   - `Count$ThisTurnEntered_Battlefield_...` with `tokenCreated` (*Bennie Bracks, Zoologist*)
//!   - `Count$LeftGraveyardThisTurn <Valid>` (*Gau, Feral Youth*, *Syrix*)
//!   - `Count$Void.1.0` / `Count$Void` (*Elegy Acolyte*, *Hymn of the Faller*)
//!   - `Count$ResolvedThisTurn` (`Effect::IfResolvedTimesThisTurn`; *Harvestrite Host*)
//!   - `Count$LifeYouLostThisTurn`, `HighestLifeLostThisTurn` (*Luminarch Ascension*, *Bloodchief Ascension*, *Knight of the Ebon Legion*)
//!   - `CardsDiscardedThisTurn` (*The Raven Man*, *Recalibrate*)
//!   - `SacrificedThisTurn <Valid>` (*Phoenix Fleet Airship*)
//! - Trigger-context objects & conditional amounts:
//!   - `TriggeredCard$Valid <Filter>[/Plus.<N>]` & `Targeted$Valid <Filter>[/Plus.<N>]` (*Akoum Hellkite*)
//!   - `PutCounter | Defined$ TriggeredSourceLKICopy` on player-damage triggers (*Aveline de Grandpré*)
//!   - `DamageAll | ValidPlayers$ OppNonTriggeredTarget` (`PlayerRel::OtherOpponents`; *Amarant Coral*, *Hydra Omnivore*)

use super::{Chain, Params, Tx, counter_kind, keyword_const, mana_color_const};
use std::collections::BTreeSet;

fn parse_mana_color(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "W" | "White" => Some("ManaColor::White"),
        "U" | "Blue" => Some("ManaColor::Blue"),
        "B" | "Black" => Some("ManaColor::Black"),
        "R" | "Red" => Some("ManaColor::Red"),
        "G" | "Green" => Some("ManaColor::Green"),
        "C" | "Colorless" => Some("ManaColor::Colorless"),
        _ => mana_color_const(raw.trim()),
    }
}

fn parse_amount_cmp(code: &str) -> Option<&'static str> {
    Some(match code {
        "GE" => "AmountCmp::AtLeast",
        "GT" => "AmountCmp::GreaterThan",
        "LE" => "AmountCmp::AtMost",
        "LT" => "AmountCmp::LessThan",
        "EQ" => "AmountCmp::Equal",
        "NE" => "AmountCmp::NotEqual",
        _ => return None,
    })
}

fn apply_slash_op(base: &str, op: &str) -> Option<String> {
    let op = op.trim();
    if op == "Twice" {
        return Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"));
    }
    if op == "HalfDown" {
        return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
    }
    if op == "HalfUp" {
        return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
    }
    if let Some(n_str) = op.strip_prefix("Times.") {
        let n = n_str.trim().parse::<u32>().ok().filter(|&v| v >= 1)?;
        return Some(format!("Amount::Times {{ base: &{base}, factor: {n} }}"));
    }
    if let Some(n_str) = op.strip_prefix("Plus.") {
        let n = n_str.trim().parse::<u32>().ok()?;
        return Some(format!("Amount::Plus {{ base: &{base}, offset: {n} }}"));
    }
    if let Some(n_str) = op.strip_prefix("Minus.") {
        let n = n_str.trim().parse::<u32>().ok()?;
        return Some(format!(
            "Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}"
        ));
    }
    if let Some(n_str) = op.strip_prefix("Div.") {
        let n = n_str.trim().parse::<u32>().ok().filter(|&v| v >= 2)?;
        return Some(format!("Amount::divided(&{base}, {n}, false)"));
    }
    None
}

fn split_valid_alt(alt: &str) -> (&str, Vec<&str>) {
    let alt = alt.trim();
    match alt.split_once('.') {
        Some((base, rest)) => (
            base.trim(),
            rest.split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .collect(),
        ),
        None => (alt, Vec::new()),
    }
}

fn rebuild_valid_alt(base: &str, atoms: &[&str]) -> String {
    if atoms.is_empty() {
        base.to_string()
    } else {
        format!("{base}.{}", atoms.join("+"))
    }
}

fn has_scope_atom(atoms: &[&str]) -> bool {
    atoms.iter().any(|a| {
        a.contains("Ctrl")
            || a.contains("Own")
            || a.starts_with("ControlledBy ")
            || a.starts_with("OwnedBy ")
    })
}

fn strip_uniform_atom(valid: &str, target_atom: &str, reject_other_scopes: bool) -> Option<String> {
    let mut out = Vec::new();
    for alt in valid.split(',') {
        let (base, atoms) = split_valid_alt(alt);
        if base.is_empty() {
            return None;
        }
        let mut found = false;
        let mut kept = Vec::new();
        for atom in atoms {
            if atom == target_atom {
                found = true;
            } else if reject_other_scopes
                && (atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom.starts_with("ControlledBy ")
                    || atom.starts_with("OwnedBy "))
            {
                return None;
            } else {
                kept.push(atom);
            }
        }
        if !found {
            return None;
        }
        out.push(rebuild_valid_alt(base, &kept));
    }
    (!out.is_empty()).then(|| out.join(","))
}

fn has_unmodelled_turn_atom(valid: &str, allow_token_created: bool) -> bool {
    valid.contains(['$', '/', ';'])
        || valid.split(',').any(|alt| {
            let (base, atoms) = split_valid_alt(alt);
            base.is_empty()
                || atoms.iter().any(|a| {
                    (!allow_token_created && *a == "tokenCreated")
                        || matches!(
                            *a,
                            "Self"
                                | "EffectSource"
                                | "IsRemembered"
                                | "!IsRemembered"
                                | "TriggeredCard"
                                | "!TriggeredCard"
                                | "CastSaSource"
                                | "!CastSaSource"
                                | "hasXCost"
                                | "wasCastFromExile"
                                | "wasCastFromGraveyard"
                                | "RememberedPlayerCtrl"
                                | "ChosenType"
                                | "ChosenColor"
                                | "NamedCard"
                        )
                        || a.starts_with("CastSa ")
                        || a.starts_with("sharesNameWith ")
                        || a.contains("TriggeredActivator")
                })
        })
}

fn is_player_damage_trigger(trigger: &str) -> bool {
    trigger.starts_with("Trigger::DealsCombatDamageToPlayer(")
        || trigger.starts_with("Trigger::DealsCombatDamageToOpponent(")
        || trigger.starts_with("Trigger::DealsDamageToPlayer ")
        || trigger.starts_with("Trigger::DealsDamageToOpponent(")
        || trigger.starts_with("Trigger::OneOrMoreDealCombatDamageToPlayer(")
        || trigger.starts_with("Trigger::DealsCombatDamageToYou(")
        || trigger.starts_with("Trigger::DealsCombatDamage(")
        || trigger.starts_with("Trigger::DealsCombatDamageToPlayerOrPlaneswalker(")
}

impl Tx<'_> {
    fn wave16a_filter_ref(&mut self, expr: &str) -> String {
        match expr {
            "Filter::Any" => "&Filter::Any".to_string(),
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
            "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
            "Filter::LAND" => "&Filter::LAND".to_string(),
            "Filter::NONLAND" => "&Filter::NONLAND".to_string(),
            "Filter::NONCREATURE" => "&Filter::NONCREATURE".to_string(),
            "Filter::INSTANT" => "&Filter::INSTANT".to_string(),
            "Filter::SORCERY" => "&Filter::SORCERY".to_string(),
            "Filter::PLANESWALKER" => "&Filter::PLANESWALKER".to_string(),
            "Filter::YOUR_CREATURE" => "&Filter::YOUR_CREATURE".to_string(),
            "Filter::YOUR_ARTIFACT" => "&Filter::YOUR_ARTIFACT".to_string(),
            "Filter::YOUR_LAND" => "&Filter::YOUR_LAND".to_string(),
            "Filter::OPPONENT_CREATURE" => "&Filter::OPPONENT_CREATURE".to_string(),
            "Filter::CastFromHand" => "&Filter::CastFromHand".to_string(),
            "Filter::Historic" => "&Filter::Historic".to_string(),
            "Filter::IsToken" => "&Filter::IsToken".to_string(),
            _ => format!("&{}", self.body.filter_static("COUNT", expr)),
        }
    }

    /// Filter-free Wave 16a `Amount` expression reader usable from `&self`
    /// (`Tx::amount_num` in `mod.rs`).
    pub(super) fn wave16a_amount_num(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim().trim_start_matches('+');
        if trimmed.is_empty() {
            return None;
        }
        let def = if trimmed.contains('$') {
            trimmed
        } else {
            self.svars.get(trimmed)?.trim()
        };
        self.wave16a_pure_amount_expr(def, 0)
    }

    fn wave16a_pure_amount_expr(&self, def: &str, depth: u8) -> Option<String> {
        if depth >= 4 {
            return None;
        }
        let def = def.trim();
        if let Some((lhs, rhs)) = def.rsplit_once('/') {
            let lhs_base = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
                let inner = self.svars.get(var.trim())?.trim();
                self.wave16a_pure_amount_expr(inner, depth + 1)?
            } else {
                self.wave16a_pure_amount_expr(lhs.trim(), depth + 1)?
            };
            return apply_slash_op(&lhs_base, rhs);
        }
        if let Some(var) = def.strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.trim();
            return self.wave16a_pure_amount_expr(inner, depth + 1);
        }
        if matches!(
            def,
            "TriggeredCard$CastTotalManaSpent" | "TriggeredSpellAbility$ManaSpent"
        ) {
            return (self.trigger_mode.as_deref() == Some("SpellCast") && !self.in_delayed)
                .then(|| "Amount::EventManaSpentToCast".to_string());
        }
        if def == "Targeted$CastTotalManaSpent" {
            return (!self.in_delayed).then(|| "Amount::TargetManaSpentToCast".to_string());
        }
        if def == "Convoked$Amount" {
            return (self.body.convoke && !self.in_delayed)
                .then(|| "Amount::ConvokedCount".to_string());
        }
        if def == "Count$Converge" {
            return (!self.in_delayed).then(|| "Amount::ColorsOfManaSpentToCast".to_string());
        }
        if let Some(color_raw) = def
            .strip_prefix("Count$EachSpentToCast.")
            .or_else(|| def.strip_prefix("Count$EachSpentToCast "))
        {
            if self.in_delayed {
                return None;
            }
            let color = parse_mana_color(color_raw)?;
            return Some(format!("Amount::ManaColorSpentToCast({color})"));
        }
        Self::wave16a_turn_scalar_amount(def).map(str::to_string)
    }

    fn wave16a_turn_scalar_amount(def: &str) -> Option<&'static str> {
        Some(match def.trim() {
            "Count$LifeYouLostThisTurn" | "PlayerCountPropertyYou$LifeLostThisTurn" => {
                "Amount::LifeLostThisTurn(PlayerRel::You)"
            }
            "PlayerCountRegisteredOpponents$HighestLifeLostThisTurn"
            | "PlayerCountOpponents$HighestLifeLostThisTurn" => {
                "Amount::HighestLifeLostThisTurn(PlayerRel::EachOpponent)"
            }
            "PlayerCountDefinedRegistered$HighestLifeLostThisTurn"
            | "PlayerCountPlayers$HighestLifeLostThisTurn" => {
                "Amount::HighestLifeLostThisTurn(PlayerRel::EachPlayer)"
            }
            "Count$CardsDiscardedThisTurn" | "PlayerCountPropertyYou$CardsDiscardedThisTurn" => {
                "Amount::CardsDiscardedThisTurn(PlayerRel::You)"
            }
            "PlayerCountOpponents$CardsDiscardedThisTurn"
            | "PlayerCountRegisteredOpponents$CardsDiscardedThisTurn" => {
                "Amount::CardsDiscardedThisTurn(PlayerRel::EachOpponent)"
            }
            "PlayerCountPlayers$CardsDiscardedThisTurn"
            | "PlayerCountDefinedRegistered$CardsDiscardedThisTurn" => {
                "Amount::CardsDiscardedThisTurn(PlayerRel::EachPlayer)"
            }
            _ => return None,
        })
    }

    /// `Adamant$ <Color|Any>` condition reader (`CR 207.2c`: at least three mana
    /// of the named color — or of any single color for `Any` — spent to cast).
    pub(super) fn wave16a_adamant_condition(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed == "Any" {
            let arms: Vec<String> = [
                "ManaColor::White",
                "ManaColor::Blue",
                "ManaColor::Black",
                "ManaColor::Red",
                "ManaColor::Green",
            ]
            .into_iter()
            .map(|c| format!("Condition::AmountAtLeast(Amount::ManaColorSpentToCast({c}), 3)"))
            .collect();
            return Some(format!("Condition::Any(&[{}])", arms.join(", ")));
        }
        let color = parse_mana_color(trimmed)?;
        Some(format!(
            "Condition::AmountAtLeast(Amount::ManaColorSpentToCast({color}), 3)"
        ))
    }

    /// `K:etbCounter:` dynamic amount reader hooked into `keywords::etb_wave6_expr`.
    pub(super) fn wave16a_etb_amount(&mut self, def: &str) -> Option<String> {
        let def = def.trim();
        if let Some(pure) = self.wave16a_pure_amount_expr(def, 0) {
            if matches!(
                pure.as_str(),
                "Amount::EventManaSpentToCast" | "Amount::TargetManaSpentToCast"
            ) {
                return None;
            }
            return Some(pure);
        }
        self.wave16a_turn_filtered_amount(def)
    }

    fn wave16a_turn_filtered_amount(&mut self, def: &str) -> Option<String> {
        let def = def.trim();
        if let Some(rest) = def.strip_prefix("Count$ThisTurnEntered_Battlefield_")
            && rest.contains("tokenCreated")
        {
            return self.wave16a_tokens_created_amount(rest.trim());
        }
        if let Some(rest) = def.strip_prefix("Count$ThisTurnEntered_Hand_from_Battlefield_") {
            return self.wave16a_public_departure_entered("ZoneRef::Hand", rest.trim());
        }
        if let Some(rest) = def.strip_prefix("Count$ThisTurnEntered_Library_from_Battlefield_") {
            return self.wave16a_public_departure_entered("ZoneRef::Library", rest.trim());
        }
        if let Some(valid) = def.strip_prefix("Count$LeftGraveyardThisTurn ") {
            return self.wave16a_left_graveyard_amount(valid.trim());
        }
        if let Some(valid) = def
            .strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ")
            .or_else(|| def.strip_prefix("Count$SacrificedThisTurn "))
        {
            return self.wave16a_sacrificed_this_turn_amount(valid.trim());
        }
        None
    }

    fn wave16a_tokens_created_amount(&mut self, valid_raw: &str) -> Option<String> {
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw, true) {
            return None;
        }
        let without_tok = strip_uniform_atom(valid_raw, "tokenCreated", false)?;
        let no_scope = without_tok
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (creator, cleaned) = if let Some(s) = strip_uniform_atom(&without_tok, "YouOwn", true)
            .or_else(|| strip_uniform_atom(&without_tok, "YouCtrl", true))
        {
            ("PlayerRel::You", s)
        } else if let Some(s) = strip_uniform_atom(&without_tok, "OppOwn", true)
            .or_else(|| strip_uniform_atom(&without_tok, "OppCtrl", true))
        {
            ("PlayerRel::EachOpponent", s)
        } else if no_scope {
            ("PlayerRel::EachPlayer", without_tok)
        } else {
            return None;
        };
        let filter_expr = self.filter_expr(&cleaned)?;
        let f_ref = self.wave16a_filter_ref(&filter_expr);
        Some(format!(
            "Amount::TokensCreatedThisTurn {{ filter: {f_ref}, creator: {creator} }}"
        ))
    }

    fn wave16a_public_departure_entered(
        &mut self,
        to_ref: &'static str,
        valid_raw: &str,
    ) -> Option<String> {
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw, false) {
            return None;
        }
        let no_scope = valid_raw
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (owner_or_ctrl, cleaned) =
            if let Some(s) = strip_uniform_atom(valid_raw, "YouOwn", true) {
                ("PlayerRel::You", s)
            } else if let Some(s) = strip_uniform_atom(valid_raw, "OppOwn", true) {
                ("PlayerRel::EachOpponent", s)
            } else if no_scope {
                ("PlayerRel::EachPlayer", valid_raw.to_string())
            } else {
                return None;
            };
        let filter_expr = self.filter_expr(&cleaned)?;
        let f_ref = self.wave16a_filter_ref(&filter_expr);
        Some(format!(
            "Amount::EnteredThisTurn {{ filter: {f_ref}, from: Some(ZoneRef::Battlefield), to: {to_ref}, owner_or_ctrl: {owner_or_ctrl}, exclude_source: false }}"
        ))
    }

    fn wave16a_left_graveyard_amount(&mut self, valid_raw: &str) -> Option<String> {
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw, false) {
            return None;
        }
        let no_scope = valid_raw
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (owner, cleaned) = if let Some(s) = strip_uniform_atom(valid_raw, "YouOwn", true) {
            ("PlayerRel::You", s)
        } else if let Some(s) = strip_uniform_atom(valid_raw, "OppOwn", true) {
            ("PlayerRel::EachOpponent", s)
        } else if no_scope {
            ("PlayerRel::EachPlayer", valid_raw.to_string())
        } else {
            return None;
        };
        let filter_expr = self.filter_expr_in_zone(&cleaned, true)?;
        let f_ref = self.wave16a_filter_ref(&filter_expr);
        Some(format!(
            "Amount::LeftZoneThisTurn {{ filter: {f_ref}, from: ZoneRef::Graveyard, owner_or_ctrl: {owner} }}"
        ))
    }

    fn wave16a_sacrificed_this_turn_amount(&mut self, valid_raw: &str) -> Option<String> {
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw, false) {
            return None;
        }
        let no_scope = valid_raw
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (who, cleaned) = if let Some(s) = strip_uniform_atom(valid_raw, "YouCtrl", true) {
            ("PlayerRel::You", s)
        } else if let Some(s) = strip_uniform_atom(valid_raw, "OppCtrl", true) {
            ("PlayerRel::EachOpponent", s)
        } else if no_scope {
            ("PlayerRel::You", valid_raw.to_string())
        } else {
            return None;
        };
        let filter_expr = self.filter_expr(&cleaned)?;
        let f_ref = self.wave16a_filter_ref(&filter_expr);
        Some(format!(
            "Amount::PermanentsSacrificedThisTurn {{ who: {who}, filter: {f_ref} }}"
        ))
    }

    fn wave16a_void_condition(raw: &str, cmp: &str) -> Option<String> {
        let inverted = match raw.trim() {
            "Count$Void.1.0" | "Count$Void" => false,
            "Count$Void.0.1" => true,
            _ => return None,
        };
        let want_true = match cmp.trim() {
            "GE1" | "GT0" | "EQ1" | "NE0" => !inverted,
            "EQ0" | "LE0" | "LT1" => inverted,
            _ => return None,
        };
        let void_cond = "Condition::Any(&[\
            Condition::AmountAtLeast(Amount::LeftZoneThisTurn { \
                filter: &Filter::NONLAND, from: ZoneRef::Battlefield, owner_or_ctrl: PlayerRel::EachPlayer \
            }, 1), \
            Condition::SpellWarpedThisTurn(None)])";
        Some(if want_true {
            void_cond.to_string()
        } else {
            format!("Condition::Not(&{void_cond})")
        })
    }

    /// Wave 16a `CheckSVar$` / `ConditionCheckSVar$` lowering over a resolved
    /// `SVar` body (`raw`) and comparator (`cmp`), hooked into
    /// `triggers::wave15_trigger_svar_condition`.
    pub(super) fn wave16a_trigger_svar_condition(
        &mut self,
        raw: &str,
        cmp: &str,
    ) -> Option<String> {
        let raw = raw.trim();
        let cmp = cmp.trim();
        if let Some(cond) = Self::wave16a_void_condition(raw, cmp) {
            return Some(cond);
        }
        if let Some(adamant_raw) = raw.strip_prefix("Adamant$") {
            let base = Self::wave16a_adamant_condition(adamant_raw)?;
            return match cmp {
                "GE1" | "GT0" | "EQ1" | "NE0" => Some(base),
                "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{base})")),
                _ => None,
            };
        }
        if let Some(rest) = raw.strip_prefix("CastSA>Count$Adamant_2.")
            && let Some(color_raw) = rest.strip_suffix(".2.0")
        {
            let color = parse_mana_color(color_raw)?;
            let base =
                format!("Condition::AmountAtLeast(Amount::ManaColorSpentToCast({color}), 2)");
            return match cmp {
                "GE1" | "GT0" | "GE2" | "EQ2" | "NE0" => Some(base),
                "EQ0" | "LE0" | "LT1" | "LT2" => Some(format!("Condition::Not(&{base})")),
                _ => None,
            };
        }
        if let Some(base_raw) = raw.strip_suffix("/LimitMax.1")
            && matches!(cmp, "GE1" | "GT0" | "EQ1" | "NE0" | "EQ0" | "LE0" | "LT1")
        {
            let lhs = self.wave16a_etb_amount(base_raw.trim())?;
            let op = match cmp {
                "GE1" | "GT0" | "EQ1" | "NE0" => "AmountCmp::AtLeast",
                _ => "AmountCmp::Equal",
            };
            let n = usize::from(op == "AmountCmp::AtLeast");
            return Some(format!(
                "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &Amount::Fixed({n}) }}"
            ));
        }
        let lhs = self.wave16a_etb_amount(raw)?;
        self.wave16a_compare_lhs_against_cmp(&lhs, cmp, false)
    }

    fn wave16a_compare_lhs_against_cmp(
        &mut self,
        lhs: &str,
        cmp: &str,
        allow_resolution_rhs: bool,
    ) -> Option<String> {
        let cmp = cmp.trim();
        if let Some(min_str) = cmp.strip_prefix("GE")
            && let Ok(min) = min_str.trim().parse::<u32>()
            && min >= 1
        {
            return Some(format!("Condition::AmountAtLeast({lhs}, {min})"));
        }
        if let Some(gt_str) = cmp.strip_prefix("GT")
            && let Ok(gt) = gt_str.trim().parse::<u32>()
            && let Some(min) = gt.checked_add(1)
        {
            return Some(format!("Condition::AmountAtLeast({lhs}, {min})"));
        }
        if matches!(cmp, "EQ0" | "LE0" | "LT1") {
            return Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({lhs}, 1))"
            ));
        }
        if cmp == "NE0" {
            return Some(format!("Condition::AmountAtLeast({lhs}, 1)"));
        }
        let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
        let op = parse_amount_cmp(op_code)?;
        let rhs = self.wave16a_rhs_amount(rhs_raw.trim(), allow_resolution_rhs)?;
        if lhs == rhs {
            return None;
        }
        Some(format!(
            "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &{rhs} }}"
        ))
    }

    fn wave16a_rhs_amount(&mut self, rhs_raw: &str, allow_resolution: bool) -> Option<String> {
        if rhs_raw.is_empty() {
            return None;
        }
        if let Ok(n) = rhs_raw.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        let resolved = self
            .svars
            .get(rhs_raw)
            .map_or_else(|| rhs_raw.to_string(), |s| s.trim().to_string());
        if let Some(n_str) = resolved.strip_prefix("Number$")
            && let Ok(n) = n_str.trim().parse::<u32>()
        {
            return Some(format!("Amount::Fixed({n})"));
        }
        if allow_resolution {
            if matches!(
                resolved.as_str(),
                "Targeted$CardManaCost" | "Targeted$ManaCost"
            ) {
                return Some("Amount::TargetCmc".to_string());
            }
            if resolved == "Targeted$CardPower" {
                return Some("Amount::TargetPower".to_string());
            }
            if resolved == "Targeted$CardToughness" {
                return Some("Amount::TargetToughness".to_string());
            }
            if let Some(pure) = self.wave16a_pure_amount_expr(&resolved, 0) {
                return Some(pure);
            }
        }
        self.wave16a_etb_amount(&resolved)
            .or_else(|| self.static_count_expr(&resolved))
    }

    /// Resolution-time condition reader for `ConditionCheckSVar$` on `DB$` steps
    /// inside triggers and spells.
    fn wave16a_resolution_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = if var.contains('$') {
            var.trim().to_string()
        } else {
            self.svars.get(var.trim())?.trim().to_string()
        };
        if let Some(cond) = self.wave16a_trigger_svar_condition(&raw, cmp) {
            return Some(cond);
        }
        let lhs = match raw.as_str() {
            "TriggeredCard$CastTotalManaSpent" | "TriggeredSpellAbility$ManaSpent"
                if self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                "Amount::EventManaSpentToCast".to_string()
            }
            "TriggeredSpellAbility$CardPower"
                if self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                "Amount::EventPower".to_string()
            }
            "TriggeredAttacker$CardPower" | "TriggeredCard$CardPower"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "ChangesZone" | "AttackerUnblocked")
                ) =>
            {
                "Amount::EventPower".to_string()
            }
            "Targeted$CastTotalManaSpent" => "Amount::TargetManaSpentToCast".to_string(),
            _ => return None,
        };
        self.wave16a_compare_lhs_against_cmp(&lhs, cmp, true)
    }

    /// Lowers `TriggeredCard$Valid <Filter>[/Plus.<N>]` (*Akoum Hellkite*) or
    /// `Targeted$Valid <Filter>[/Plus.<N>]` (*Elder Cathar*) to `Amount::IfCondition`
    /// over `Condition::ObjectMatches`.
    fn wave16a_object_valid_amount(&mut self, raw: &str, has_target: bool) -> Option<String> {
        let trimmed = raw.trim();
        let def = if trimmed.contains('$') {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.trim().to_string()
        };
        let (head, offset) = match def.rsplit_once("/Plus.") {
            Some((lhs, n_str)) => (lhs.trim(), n_str.trim().parse::<u32>().ok()?),
            None => (def.as_str(), 0),
        };
        let (what_spec, valid) = if let Some(v) = head.strip_prefix("TriggeredCard$Valid ") {
            if matches!(self.trigger_mode.as_deref(), None | Some("Phase")) {
                return None;
            }
            ("&TargetSpec::EventObject", v.trim())
        } else {
            let v = head.strip_prefix("Targeted$Valid ")?;
            if !has_target {
                return None;
            }
            ("&TargetSpec::Object(&Filter::Any)", v.trim())
        };
        if valid.is_empty() || valid.contains(['$', '/']) {
            return None;
        }
        let filter_expr = self.filter_expr(valid)?;
        let f_ref = self.wave16a_filter_ref(&filter_expr);
        let cond_expr =
            format!("Condition::ObjectMatches {{ what: {what_spec}, filter: {f_ref} }}");
        let cond_name = self.body.condition_static("MATCH", &cond_expr);
        let then_val = 1_u32.checked_add(offset)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond_name}, then: &Amount::Fixed({then_val}), otherwise: &Amount::Fixed({offset}) }}"
        ))
    }

    /// Fallback trigger `Execute$` chain reader hooked into `triggers.rs`.
    pub(super) fn wave16a_trigger_chain(
        &mut self,
        trigger: &str,
        body: &str,
        out: &mut Chain,
    ) -> Option<()> {
        self.wave16a_walk_chain(Some(trigger), body, out)
    }

    /// Fallback `A:` spell reader hooked into `keywords::rule` (`'A'`).
    pub(super) fn wave16a_spell_or_activated(&mut self, spec: &str) -> Option<()> {
        if !spec.trim_start().starts_with("SP$") {
            return None;
        }
        let (_, probe) = Params::parse(spec)?;
        if probe.has("Cost")
            || probe.has("ActivationPhases")
            || probe.has("ActivationZone")
            || probe.has("Activator")
            || probe.has("CheckSVar")
            || probe.has("Condition")
            || probe.has("IsPresent")
        {
            return None;
        }
        let mut chain = Chain::default();
        self.wave16a_walk_chain(None, spec, &mut chain)?;
        if chain.effects.is_empty() || chain.second_target.is_some() || chain.third_target.is_some()
        {
            return None;
        }
        let targets =
            Self::target_req_expr(&chain).map_or_default(|req| format!(", targets = Some({req})"));
        let effects = chain.effects.join(", ");
        self.body
            .abilities
            .push(format!("spell!(&[{effects}]{targets})"));
        Some(())
    }

    fn wave16a_walk_chain(
        &mut self,
        trigger: Option<&str>,
        first_spec: &str,
        out: &mut Chain,
    ) -> Option<()> {
        let mut visited = BTreeSet::new();
        let mut current = Some(first_spec.to_string());
        let mut used_wave16a_extension = false;
        while let Some(spec) = current.take() {
            let (head_prefix, _) = spec.split_once('$')?;
            let head_prefix = head_prefix.trim();
            if !matches!(head_prefix, "SP" | "DB") {
                return None;
            }
            let (api, mut p) = Params::parse(&spec)?;
            let next_sub = p.take("SubAbility").map(|s| s.trim().to_string());
            if let Some(ref sub_name) = next_sub
                && (sub_name.is_empty() || !visited.insert(sub_name.clone()))
            {
                return None;
            }
            let rebuilt = Self::wave16a_rebuild_spec(head_prefix, &api, &p);
            let (step_effects, step_used_ext) =
                self.wave16a_lower_single_step(trigger, head_prefix, &api, &p, &rebuilt, out)?;
            used_wave16a_extension |= step_used_ext;
            out.effects.extend(step_effects);
            if let Some(sub_name) = next_sub {
                current = Some(self.svars.get(&sub_name)?.clone());
            }
        }
        used_wave16a_extension.then_some(())
    }

    fn wave16a_rebuild_spec(prefix: &str, api: &str, p: &Params) -> String {
        let mut parts = Vec::with_capacity(1 + p.entries.len());
        parts.push(format!("{prefix}$ {api}"));
        for (k, v) in &p.entries {
            parts.push(format!("{k}$ {v}"));
        }
        parts.join(" | ")
    }

    fn wave16a_lower_single_step(
        &mut self,
        trigger: Option<&str>,
        head_prefix: &str,
        api: &str,
        orig_p: &Params,
        rebuilt: &str,
        chain: &mut Chain,
    ) -> Option<(Vec<String>, bool)> {
        // 1. Check `ConditionCheckSVar$` wrapper (`Count$ResolvedThisTurn` or `wave16a` conditions).
        if let Some(cond_var) = orig_p.peek("ConditionCheckSVar") {
            let cmp = orig_p.peek("ConditionSVarCompare").unwrap_or("GE1").trim();
            let mut stripped_p = orig_p.clone();
            stripped_p.take("ConditionCheckSVar");
            stripped_p.take("ConditionSVarCompare");
            stripped_p.take("ConditionDescription");
            if stripped_p.has("ValidTgts") || stripped_p.has("TargetType") {
                return None;
            }
            let inner_rebuilt = Self::wave16a_rebuild_spec(head_prefix, api, &stripped_p);
            let resolved_raw = if cond_var.contains('$') {
                Some(cond_var.trim().to_string())
            } else {
                self.svars
                    .get(cond_var.trim())
                    .map(|s| s.trim().to_string())
            };
            if resolved_raw.as_deref() == Some("Count$ResolvedThisTurn") {
                let times = cmp
                    .strip_prefix("EQ")
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .filter(|&n| n >= 1)?;
                let (inner_effs, _) = self.wave16a_lower_unconditional_step(
                    trigger,
                    api,
                    &stripped_p,
                    &inner_rebuilt,
                    chain,
                )?;
                return Some((
                    vec![format!(
                        "Effect::IfResolvedTimesThisTurn {{ times: {times}, then: &[{}] }}",
                        inner_effs.join(", ")
                    )],
                    true,
                ));
            }
            if let Some(cond) = self.wave16a_resolution_svar_condition(cond_var, cmp) {
                let (inner_effs, _) = self.wave16a_lower_unconditional_step(
                    trigger,
                    api,
                    &stripped_p,
                    &inner_rebuilt,
                    chain,
                )?;
                return Some((
                    vec![format!(
                        "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
                        inner_effs.join(", ")
                    )],
                    true,
                ));
            }
        }
        self.wave16a_lower_unconditional_step(trigger, api, orig_p, rebuilt, chain)
    }

    fn wave16a_lower_unconditional_step(
        &mut self,
        trigger: Option<&str>,
        api: &str,
        orig_p: &Params,
        rebuilt: &str,
        chain: &mut Chain,
    ) -> Option<(Vec<String>, bool)> {
        // Custom Wave 16a step forms:
        if let Some(effs) = self.wave16a_custom_step(trigger, api, orig_p.clone(), chain) {
            return Some((effs, true));
        }
        // Delegate standard single step to `self.chain`:
        // - If `chain.target` is empty, run directly on a fresh `step_chain` and merge target state.
        // - If `chain.target` is already bound by an earlier step (e.g. `SP$ Counter | ValidTgts$ Card`
        //   followed by `DB$ Draw`), ensure this step adds no new targets and run cleanly.
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let mut step_chain = Chain::default();
        if chain.target.is_some()
            && (orig_p.has("ValidTgts") || orig_p.has("TargetType") || orig_p.has("Defined"))
            && orig_p
                .peek("Defined")
                .is_some_and(|d| d.contains("Targeted"))
        {
            // Reject targeted follow-ups that depend on `chain.target` when `step_chain` is empty,
            // unless handled by `wave16a_custom_step` above.
            return None;
        }
        if self.chain(rebuilt, &mut step_chain).is_none() {
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        }
        if step_chain.target.is_some() {
            if chain.target.is_some() {
                self.body.statics.truncate(saved_statics);
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                return None;
            }
            chain.target = step_chain.target;
            chain.target_is_x = step_chain.target_is_x;
            chain.target_up_to = step_chain.target_up_to;
            chain.target_exact = step_chain.target_exact;
            chain.target_unique = step_chain.target_unique;
            chain.target_constraint = step_chain.target_constraint;
            chain.second_target = step_chain.second_target;
            chain.second_target_up_to = step_chain.second_target_up_to;
            chain.second_target_constraint = step_chain.second_target_constraint;
            chain.third_target = step_chain.third_target;
            chain.third_target_up_to = step_chain.third_target_up_to;
            chain.third_target_constraint = step_chain.third_target_constraint;
        }
        chain.could_add_mana |= step_chain.could_add_mana;
        chain.moves_library |= step_chain.moves_library;
        Some((step_chain.effects, false))
    }

    #[allow(clippy::too_many_lines)] // flat dispatch over Wave 16a custom step shapes
    fn wave16a_custom_step(
        &mut self,
        trigger: Option<&str>,
        api: &str,
        mut p: Params,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        p.drop_prose();
        match api {
            // 1. Source-centric player-damage trigger `PutCounter | Defined$ TriggeredSourceLKICopy`
            // (*Aveline de Grandpré*).
            "PutCounter"
                if trigger.is_some_and(is_player_damage_trigger)
                    && chain.target.is_none()
                    && matches!(
                        p.peek("Defined"),
                        Some("TriggeredSourceLKICopy" | "TriggeredSource")
                    ) =>
            {
                p.take("Defined");
                let kind = counter_kind(p.take("CounterType")?.trim())?;
                let num_raw = p.take("CounterNum").unwrap_or_else(|| "1".to_string());
                let amount = self.amount_or_count_ext(num_raw.trim())?;
                if !p.exhausted() {
                    return None;
                }
                Some(vec![format!(
                    "Effect::AddCounterFilter {{ filter: &Filter::Reference, kind: {kind}, amount: {amount} }}"
                )])
            }
            // 2. `PutCounter` with `TriggeredCard$Valid` / `Targeted$Valid` amount (*Elder Cathar*).
            "PutCounter" if p.peek("CounterNum").is_some() => {
                let has_own_tgt = p.has("ValidTgts");
                let num_raw = p.take("CounterNum")?;
                let saved_statics = self.body.statics.len();
                let Some(amount) = self.wave16a_object_valid_amount(
                    num_raw.trim(),
                    has_own_tgt || chain.target.is_some(),
                ) else {
                    self.body.statics.truncate(saved_statics);
                    return None;
                };
                let kind = counter_kind(p.take("CounterType")?.trim())?;
                if has_own_tgt {
                    if chain.target.is_some() || p.has("TargetMin") || p.has("TargetMax") {
                        self.body.statics.truncate(saved_statics);
                        return None;
                    }
                    let valid = p.take("ValidTgts")?;
                    let filter_expr = self.filter_expr(valid.trim())?;
                    let f_ref = self.wave16a_filter_ref(&filter_expr);
                    if !p.exhausted() {
                        self.body.statics.truncate(saved_statics);
                        return None;
                    }
                    chain.target = Some(format!("TargetSpec::Object({f_ref})"));
                    return Some(vec![format!(
                        "Effect::AddCounter {{ kind: {kind}, amount: {amount} }}"
                    )]);
                }
                if p.take("Defined").as_deref() == Some("Self") && p.exhausted() {
                    return Some(vec![format!(
                        "Effect::AddCounterSource {{ kind: {kind}, amount: {amount} }}"
                    )]);
                }
                self.body.statics.truncate(saved_statics);
                None
            }
            // 3. `DealDamage` with `TriggeredCard$Valid <Filter>[/Plus.<N>]` (*Akoum Hellkite*).
            "DealDamage" if p.peek("NumDmg").is_some() => {
                let has_own_tgt = p.has("ValidTgts");
                let dmg_raw = p.take("NumDmg")?;
                let saved_statics = self.body.statics.len();
                let Some(amount) = self.wave16a_object_valid_amount(
                    dmg_raw.trim(),
                    has_own_tgt || chain.target.is_some(),
                ) else {
                    self.body.statics.truncate(saved_statics);
                    return None;
                };
                if has_own_tgt {
                    if chain.target.is_some() || p.has("TargetMin") || p.has("TargetMax") {
                        self.body.statics.truncate(saved_statics);
                        return None;
                    }
                    let valid = p.take("ValidTgts")?;
                    let target_spec = match valid.trim() {
                        "Any" => "TargetSpec::AnyTarget".to_string(),
                        "Player" => "TargetSpec::AnyPlayer".to_string(),
                        "Opponent" | "Player.Opponent" => "TargetSpec::Opponent".to_string(),
                        other => {
                            let filter_expr = self.filter_expr(other)?;
                            let f_ref = self.wave16a_filter_ref(&filter_expr);
                            format!("TargetSpec::Object({f_ref})")
                        }
                    };
                    if !p.exhausted() {
                        self.body.statics.truncate(saved_statics);
                        return None;
                    }
                    chain.target = Some(target_spec.clone());
                    return Some(vec![format!(
                        "Effect::DealDamage {{ amount: {amount}, target: {target_spec} }}"
                    )]);
                }
                let defined = p.take("Defined")?;
                let who = self.player_rel(Some(defined.trim()))?;
                if !p.exhausted() {
                    self.body.statics.truncate(saved_statics);
                    return None;
                }
                Some(vec![format!(
                    "Effect::DealDamage {{ amount: {amount}, target: TargetSpec::Player({who}) }}"
                )])
            }
            // 4. `DamageAll | ValidPlayers$ OppNonTriggeredTarget` (*Amarant Coral*, *Hydra Omnivore*, *Grenzo's Ruffians*).
            "DamageAll"
                if trigger.is_some_and(is_player_damage_trigger)
                    && p.peek("ValidPlayers") == Some("OppNonTriggeredTarget") =>
            {
                p.take("ValidPlayers");
                p.take("ValidDescription");
                let raw_dmg = p.take("NumDmg")?;
                let amount = self.amount_or_count_ext(raw_dmg.trim())?;
                let event_source = match p.take("DamageSource").as_deref() {
                    None => false,
                    Some("TriggeredSource" | "TriggeredSourceLKICopy") => {
                        // Guard against scripts like Kediss where Forge wrote `ValidTarget$ Player`
                        // while Oracle specifies "deals combat damage to an opponent".
                        let lower_oracle = self.oracle.to_ascii_lowercase();
                        if lower_oracle.contains("damage to an opponent")
                            && trigger.is_some_and(|t| t.contains("ToPlayer("))
                        {
                            return None;
                        }
                        true
                    }
                    Some(_) => return None,
                };
                if !p.exhausted() {
                    return None;
                }
                Some(vec![if event_source {
                    format!(
                        "Effect::EventObjectDealsDamage {{ amount: {amount}, target: TargetSpec::Player(PlayerRel::OtherOpponents) }}"
                    )
                } else {
                    format!(
                        "Effect::DealDamage {{ amount: {amount}, target: TargetSpec::Player(PlayerRel::OtherOpponents) }}"
                    )
                }])
            }
            // 5. Self `Pump` on a permanent's trigger with dynamic Wave 16a `Amount` (`+RR` on *Bladecoil Serpent*).
            "Pump"
                if trigger.is_some()
                    && self.is_permanent_card_pub()
                    && !p.has("ValidTgts")
                    && matches!(p.peek("Defined"), None | Some("Self")) =>
            {
                p.take("Defined");
                let mut used_dynamic = false;
                let att = match p.take("NumAtt") {
                    None => "Amount::Fixed(0)".to_string(),
                    Some(raw) => {
                        let expr = self.wave16a_amount_num(&raw)?;
                        used_dynamic = true;
                        expr
                    }
                };
                let def = match p.take("NumDef") {
                    None => "Amount::Fixed(0)".to_string(),
                    Some(raw) => {
                        let expr = self.wave16a_amount_num(&raw)?;
                        used_dynamic = true;
                        expr
                    }
                };
                if !used_dynamic {
                    return None;
                }
                let kw_expr = match p.take("KW") {
                    None => "KeywordSet::EMPTY".to_string(),
                    Some(kws) => {
                        let mut parts = Vec::new();
                        for word in kws.split('&') {
                            parts.push(keyword_const(word.trim())?.to_string());
                        }
                        match parts.as_slice() {
                            [] => "KeywordSet::EMPTY".to_string(),
                            [single] => single.clone(),
                            many => {
                                let mut expr = many[0].clone();
                                for next in &many[1..] {
                                    expr = format!("{expr}.union({next})");
                                }
                                expr
                            }
                        }
                    }
                };
                let duration = match p.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => return None,
                };
                if !p.exhausted() {
                    return None;
                }
                Some(vec![format!(
                    "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, power: {att}, toughness: {def}, keywords: {kw_expr}, duration: {duration} }}"
                )])
            }
            _ => None,
        }
    }
}
