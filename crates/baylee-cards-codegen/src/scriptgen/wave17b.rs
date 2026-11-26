//! Wave 17b scriptgen readers mapped onto Wave 17b paper-MTG DSL primitives:
//!
//! 1. `Effect::Empower` (`Empower Jace N`, CR 701.71a) across `SP$`/`AB$`/`DB$`
//!    chains (`wave17b_empower_effect`, plus excess-damage and remembered-exile
//!    hooks in `chains.rs`).
//! 2. `Amount::DistinctPowersAmong` (`Count$Valid <valid>$DifferentCardPower`,
//!    Coven `>= 3` conditions, dynamic spell/mana/ETB counts) and
//!    `Effect::ChooseDistinctPowersOnBattlefield` (`ChooseCard | WithDifferentPowers$ True`,
//!    *Sigarda's Vanguard*, *Sigardian Zealot*).
//! 3. Dynamic `K:etbCounter` / `Count$` expressions:
//!    - `Count$TotalCommanderCastFromCommandZone` -> `Amount::CommanderCasts(PlayerRel::You)`
//!    - `PlayerCountPlayers$HighestValid <valid>` / `PlayerCountOpponents$HighestValid <valid>`
//!      -> `Amount::HighestControlledCount { among, filter }`
//!    - `PlayerCountOpponents$HighestCardsDrawn` / `PlayerCountPlayers$HighestCardsDrawn`
//!      -> `Amount::HighestCardsDrawnThisTurn(who)`
//!    - `Count$ValidGraveyard <valid>$CardManaCost` -> `Amount::TotalManaValueInZone`
//!    - `Count$ExactManaCostGraveyard <valid>` -> `Amount::DistinctExactManaCostsInZone`
//! 4. `CounterType$ EachFromSource` without `CounterNum$` (`Effect::PutAllCountersFrom`
//!    and `Effect::PutAllCountersFromIntoFound`, CR 122.8).
//! 5. `K:Mayhem:<cost>` (`FaceDef::mayhem` + `KeywordSet::MAYHEM`, CR 702.187).

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// `K:Mayhem:<cost>` on a non-land face -> `FaceDef::mayhem = Some(mana!(...))`
    /// plus `KeywordSet::MAYHEM` (`CR 702.187a`).
    ///
    /// Costless `K:Mayhem` is land-only (*Oscorp Industries*); costed `K:Mayhem:<cost>`
    /// on a land stays refused.
    pub(super) fn wave17b_mayhem(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty()
            || self.is_land_card_type()
            || self.body.mayhem.is_some()
            || rest.contains(':')
        {
            return self.deny(format!("keyword `Mayhem:{rest}`"));
        }
        let rest = rest.trim();
        if rest.is_empty() {
            return self.deny("keyword `Mayhem:`".to_string());
        }
        let previous_has_x = std::mem::replace(&mut self.has_x, true);
        let cost = self.cost_pieces(rest);
        self.has_x = previous_has_x;
        let Some((mana, _)) = cost.filter(|(m, parts)| !m.is_empty() && parts.is_empty()) else {
            return self.deny(format!("keyword `Mayhem:{rest}`"));
        };
        self.body.mayhem = Some(format!("Some(mana!(\"{mana}\"))"));
        if !self.body.keywords.iter().any(|k| k == "KeywordSet::MAYHEM") {
            self.body.keywords.push("KeywordSet::MAYHEM".to_string());
        }
        self.body.notes.push("mayhem".to_string());
        Some(())
    }

    /// `DB$/AB$/SP$ Empower | Type$ Jace | Num$ <N>` (`CR 701.71a`).
    pub(super) fn wave17b_empower_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if who != "PlayerRel::You" {
            return self.deny("`Empower` for somebody other than you".to_string());
        }
        let raw_type = p.take("Type")?;
        if raw_type.trim() != "Jace" {
            return self.deny(format!("`Empower` type `{}`", raw_type.trim()));
        }
        let raw_num = p.take("Num").or_else(|| p.take("Amount"))?;
        let amount = self.amount_or_count_ext(&raw_num)?;
        Some(vec![format!(
            "Effect::Empower {{ token: &generated_tokens::JACE_PLANESWALKER_BLUE, amount: {amount} }}"
        )])
    }

    /// Dynamic `Amount` reader shared by `mana::count_expr_depth`,
    /// `wave27a::wave27a_svar_condition`, and spell/ability/trigger counts.
    pub(super) fn wave17b_count_expr(
        &mut self,
        def: &str,
        _depth: u8,
        _static_only: bool,
    ) -> Option<String> {
        let def = def.trim();
        if let Some(rest) = def.strip_prefix("Count$Valid ")
            && let Some(valid) = rest.strip_suffix("$DifferentCardPower")
        {
            return self.wave17b_distinct_powers_amount(valid);
        }
        self.wave17b_common_zone_and_player_amount(def)
    }

    /// Dynamic `Amount` reader for `K:etbCounter:` (`etb_wave6_expr`).
    pub(super) fn wave17b_etb_amount(&mut self, def: &str) -> Option<String> {
        let def = def.trim();
        if let Some(rest) = def
            .strip_prefix("Count$LastStateBattlefield ")
            .or_else(|| def.strip_prefix("Count$Valid "))
            && let Some(valid) = rest.strip_suffix("$DifferentCardPower")
        {
            let stripped = Self::strip_etb_other_atoms(valid.trim());
            return self.wave17b_distinct_powers_amount(&stripped);
        }
        if let Some(rest) = def.strip_prefix("Count$LastStateGraveyard ") {
            let normalized = format!("Count$ValidGraveyard {}", rest.trim());
            return self.wave17b_common_zone_and_player_amount(&normalized);
        }
        if let Some(valid) = def.strip_prefix("PlayerCountOpponents$HighestValid ") {
            return self.wave17b_highest_controlled_amount("PlayerRel::EachOpponent", valid);
        }
        self.wave17b_common_zone_and_player_amount(def)
    }

    /// Dynamic `Amount` reader for `statics::wave14_amount_leaf`.
    pub(super) fn wave17b_static_amount(
        &mut self,
        trimmed: &str,
        allow_power_scan: bool,
        allow_turn_counts: bool,
    ) -> Option<String> {
        if matches!(
            trimmed,
            "PlayerCountOpponents$HighestCardsDrawn" | "PlayerCountPlayers$HighestCardsDrawn"
        ) && (!allow_turn_counts || !allow_power_scan)
        {
            return None;
        }
        if (trimmed.starts_with("PlayerCountPlayers$HighestValid ")
            || trimmed.starts_with("PlayerCountOpponents$HighestValid "))
            && !allow_power_scan
        {
            return None;
        }
        self.wave17b_common_zone_and_player_amount(trimmed)
    }

    fn wave17b_distinct_powers_amount(&mut self, valid: &str) -> Option<String> {
        let valid = valid.trim();
        if valid.is_empty()
            || valid.contains(['$', '/'])
            || valid.contains("inZone")
            || valid.contains("Self")
        {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        let filter_ref = Self::wave17b_filter_ref(&mut self.body, "COUNT", &expr);
        Some(format!("Amount::DistinctPowersAmong({filter_ref})"))
    }

    fn wave17b_common_zone_and_player_amount(&mut self, def: &str) -> Option<String> {
        if def == "Count$TotalCommanderCastFromCommandZone" {
            return Some("Amount::CommanderCasts(PlayerRel::You)".to_string());
        }
        if def == "PlayerCountOpponents$HighestCardsDrawn" {
            return Some("Amount::HighestCardsDrawnThisTurn(PlayerRel::EachOpponent)".to_string());
        }
        if def == "PlayerCountPlayers$HighestCardsDrawn" {
            return Some("Amount::HighestCardsDrawnThisTurn(PlayerRel::EachPlayer)".to_string());
        }
        if let Some(valid) = def.strip_prefix("PlayerCountPlayers$HighestValid ") {
            return self.wave17b_highest_controlled_amount("PlayerRel::EachPlayer", valid);
        }
        if let Some(rest) = def.strip_prefix("Count$ValidGraveyard ")
            && let Some(valid) = rest.strip_suffix("$CardManaCost")
        {
            let (zone, stripped) = Self::wave17b_graveyard_zone_and_filter(valid)?;
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_ref = Self::wave17b_filter_ref(&mut self.body, "COUNT", &expr);
            return Some(format!(
                "Amount::TotalManaValueInZone {{ zone: {zone}, filter: {filter_ref} }}"
            ));
        }
        if let Some(valid) = def.strip_prefix("Count$ExactManaCostGraveyard ") {
            let (zone, stripped) = Self::wave17b_graveyard_zone_and_filter(valid)?;
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_ref = Self::wave17b_filter_ref(&mut self.body, "COUNT", &expr);
            return Some(format!(
                "Amount::DistinctExactManaCostsInZone {{ zone: {zone}, filter: {filter_ref} }}"
            ));
        }
        None
    }

    fn wave17b_highest_controlled_amount(
        &mut self,
        among: &'static str,
        valid: &str,
    ) -> Option<String> {
        let valid = valid.trim();
        if valid.is_empty()
            || valid.contains(['$', '/'])
            || valid.contains("inZone")
            || valid.contains("Self")
            || valid.contains("Other")
        {
            return None;
        }
        // In Forge's `PlayerCount*$HighestValid <Type>.YouCtrl`, `YouCtrl` binds
        // to the seat currently being evaluated by `PlayerCount*`. Require
        // uniform `YouCtrl` across every branch (or no controller/owner atoms)
        // and strip it so `Amount::HighestControlledCount` checks `o.controller == seat`.
        let mut saw_you_ctrl = false;
        let mut saw_unscoped = false;
        let mut stripped_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut alt_you = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == "YouCtrl" {
                    alt_you = true;
                } else if atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom.contains("ControlledBy")
                    || atom.contains("OwnedBy")
                {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if alt_you {
                if saw_unscoped {
                    return None;
                }
                saw_you_ctrl = true;
            } else {
                if saw_you_ctrl {
                    return None;
                }
                saw_unscoped = true;
            }
            if kept.is_empty() {
                stripped_alts.push(base.trim().to_string());
            } else {
                stripped_alts.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        let stripped = stripped_alts.join(",");
        let expr = self.filter_expr(&stripped)?;
        let filter_ref = Self::wave17b_filter_ref(&mut self.body, "COUNT", &expr);
        Some(format!(
            "Amount::HighestControlledCount {{ among: {among}, filter: {filter_ref} }}"
        ))
    }

    fn wave17b_graveyard_zone_and_filter(valid: &str) -> Option<(&'static str, String)> {
        let valid = valid.trim();
        if valid.is_empty() || valid.contains(['$', '/']) || valid.contains("Self") {
            return None;
        }
        let mut zone: Option<&'static str> = None;
        let mut saw_unowned = false;
        let mut stripped_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut alt_zone: Option<&'static str> = None;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                let z = match atom {
                    "YouOwn" | "YouCtrl" => Some("ZoneSel::GraveyardYou"),
                    "OppOwn" | "OppCtrl" | "YouDontOwn" => {
                        Some("ZoneSel::GraveyardOf(PlayerRel::EachOpponent)")
                    }
                    _ if atom.contains("Own")
                        || atom.contains("Ctrl")
                        || atom.contains("ControlledBy")
                        || atom.contains("OwnedBy") =>
                    {
                        return None;
                    }
                    _ => None,
                };
                if let Some(found) = z {
                    if alt_zone.is_some_and(|prev| prev != found) {
                        return None;
                    }
                    alt_zone = Some(found);
                } else {
                    kept.push(atom);
                }
            }
            if let Some(found) = alt_zone {
                if saw_unowned || zone.is_some_and(|prev| prev != found) {
                    return None;
                }
                zone = Some(found);
            } else {
                if zone.is_some() {
                    return None;
                }
                saw_unowned = true;
            }
            if kept.is_empty() {
                stripped_alts.push(base.trim().to_string());
            } else {
                stripped_alts.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        let resolved_zone = zone.unwrap_or("ZoneSel::GraveyardAll");
        Some((resolved_zone, stripped_alts.join(",")))
    }

    fn strip_etb_other_atoms(spec: &str) -> String {
        if spec.contains('$') || spec.contains("Self") {
            return spec.to_string();
        }
        spec.split(',')
            .map(|alt| {
                let alt = alt.trim();
                let Some((base, atoms)) = alt.split_once('.') else {
                    return alt.to_string();
                };
                let kept: Vec<&str> = atoms
                    .split('+')
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && *a != "Other" && *a != "StrictlyOther")
                    .collect();
                if kept.is_empty() {
                    base.trim().to_string()
                } else {
                    format!("{}.{}", base.trim(), kept.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn wave17b_filter_ref(body: &mut crate::body::CardBody, prefix: &str, expr: &str) -> String {
        if expr.contains(['(', '[', ',', ' ']) {
            format!("&{}", body.filter_static(prefix, expr))
        } else {
            format!("&{expr}")
        }
    }

    /// `ChooseCard | WithDifferentPowers$ True` ->
    /// `Effect::ChooseDistinctPowersOnBattlefield` (*Sigarda's Vanguard*,
    /// *Sigardian Zealot*).
    pub(super) fn wave17b_choose_distinct_powers_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        p.take("ChoiceTitle");
        p.take("ChoiceDesc");
        p.take("IsCurse");
        if p.take("WithDifferentPowers").as_deref() != Some("True") {
            return None;
        }
        if p.has("AtRandom")
            || p.has("DifferentNames")
            || p.has("EachBasicType")
            || p.has("Reveal")
            || p.has("ControlledByPlayer")
            || p.has("RememberChosen")
            || p.has("ChosenMap")
            || p.has("LockInText")
        {
            return self.deny("`ChooseCard` `WithDifferentPowers` with unsupported flags".into());
        }
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        if !matches!(p.take("ChoiceZone").as_deref(), None | Some("Battlefield")) {
            return self.deny("`ChooseCard` `WithDifferentPowers` outside battlefield".into());
        }
        let chooser =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(chooser) {
            return self.deny(format!(
                "`ChooseCard` `WithDifferentPowers` for multi-seat `{chooser}`"
            ));
        }
        let choices_raw = p.take("Choices")?;
        if choices_raw.contains("Remembered")
            || choices_raw.contains("IsImprinted")
            || choices_raw.contains("Targeted")
            || choices_raw.contains("ExiledWith")
        {
            return self.deny(format!("`ChooseCard` with `Choices$ {choices_raw}`"));
        }
        let filter_expr = self.filter_expr(&choices_raw)?;
        let filter_ref = Self::wave17b_filter_ref(&mut self.body, "CHOOSE", &filter_expr);
        let mandatory = match p.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChooseCard` with `Mandatory$ {other}`")),
        };
        let raw_amount = p.take("Amount");
        let max_expr = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(raw)?,
        };
        if max_expr == "Amount::Fixed(0)" {
            return None;
        }
        let min: u8 = if let Some(raw_min) = p.take("MinAmount") {
            raw_min.trim().parse::<u8>().ok()?
        } else if mandatory {
            match raw_amount.as_deref() {
                None => 1,
                Some(raw) => raw.trim().parse::<u8>().unwrap_or(u8::MAX),
            }
        } else {
            0
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseCard.{key}`"));
        }
        let sub_name = sub?;
        let then_effects = self.compile_chosen_object_chain(sub_name)?;
        let then_joined = then_effects.join(", ");
        let choose = format!(
            "Effect::ChooseDistinctPowersOnBattlefield {{ chooser: {chooser}, \
             filter: {filter_ref}, min: {min}, max: {max_expr}, then: &[{then_joined}] }}"
        );
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{choose}], otherwise: &[] }}"
            ),
            None => choose,
        });
        Some(())
    }

    /// `DB$/AB$ PutCounter | CounterType$ EachFromSource` without `CounterNum$`
    /// -> `Effect::PutAllCountersFrom { donor, target }` (`CR 122.8`).
    ///
    /// Copies *all* counters of each kind from `donor` (`Self` on activated
    /// abilities / non-self triggers like *Zack Fair* and *Denry Klin*, or
    /// `TriggeredCardLKICopy` on `ChangesZone` triggers like *Star Pupil* and
    /// *The Ozolith*) onto `target`.
    pub(super) fn wave17b_put_counters_from_source(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.take("CounterType").as_deref() != Some("EachFromSource")
            || p.has("CounterNum")
            || p.has("ETB")
        {
            return self.deny("counter `EachFromSource`".to_string());
        }
        let is_self_zone_trigger =
            self.trigger_mode.as_deref() == Some("ChangesZone") && self.trigger_is_self;
        let donor = match p.take("EachFromSource").as_deref() {
            Some("Self") if !(self.on_a_spell || self.in_delayed || is_self_zone_trigger) => {
                "CounterDonor::Source"
            }
            Some("TriggeredCardLKICopy") if self.trigger_mode.as_deref() == Some("ChangesZone") => {
                "CounterDonor::Event"
            }
            _ => return self.deny("counter `EachFromSource`".to_string()),
        };
        p.take("IsCurse");
        let spec = match p.take("Defined").as_deref() {
            None if !targets_here && target.is_none() => "TargetSpec::ThisObject".to_string(),
            Some("Self") if !targets_here && !self.on_a_spell && !self.in_delayed => {
                "TargetSpec::ThisObject".to_string()
            }
            Some(
                "TriggeredCardLKICopy"
                | "TriggeredCard"
                | "TriggeredNewCardLKICopy"
                | "TriggeredNewCard",
            ) if target.is_none()
                && donor != "CounterDonor::Event"
                && matches!(
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
                "TargetSpec::EventObject".to_string()
            }
            None if targets_here
                && !targets_a_player
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                target?.to_string()
            }
            Some("Targeted" | "ParentTarget")
                if !targets_here
                    && !targets_a_player
                    && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                target?.to_string()
            }
            _ => return self.deny("counter `EachFromSource`".to_string()),
        };
        Some(vec![format!(
            "Effect::PutAllCountersFrom {{ donor: {donor}, target: {spec} }}"
        )])
    }

    /// `DB$ Token | RememberTokens$ True | SubAbility$ DBPutCounter` with
    /// `DB$ PutCounter | Defined$ Remembered | CounterType$ EachFromSource |
    /// EachFromSource$ TriggeredCardLKICopy` (*Ambitious Augmenter*).
    pub(super) fn wave17b_token_found_put_all_counters(
        &mut self,
        sp: &mut Params,
    ) -> Option<String> {
        if self.trigger_mode.as_deref() != Some("ChangesZone")
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("CounterType").as_deref() != Some("EachFromSource")
            || sp.take("EachFromSource").as_deref() != Some("TriggeredCardLKICopy")
            || sp.has("CounterNum")
            || sp.has("ETB")
        {
            return None;
        }
        sp.take("IsCurse");
        if !sp.exhausted() {
            return None;
        }
        Some("Effect::PutAllCountersFromIntoFound { donor: CounterDonor::Event }".to_string())
    }
}
