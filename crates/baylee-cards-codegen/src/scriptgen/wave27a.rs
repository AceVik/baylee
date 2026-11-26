//! Wave 27a `SVar` / `Count$` / `CheckSVar$` / `ConditionCheckSVar$` readers
//! backed by Wave 15a DSL primitives:
//!
//! - `Amount::EnteredThisTurn` (`Count$ThisTurnEntered_...`)
//! - `Amount::SpellsCastMatchingThisTurn` (`Count$ThisTurnCast_...`)
//! - `Amount::DistinctCardTypesCastThisTurn` (`Count$ThisTurnCast_Card...$CardTypes`)
//! - `Amount::CreaturesAttackedThisTurn` (`Count$CreaturesAttackedThisTurn ...`,
//!   `PlayerCountPlayers$AttackersDeclared`)
//! - `Amount::DistinctManaValuesInZone` (`Count$Valid...$DifferentCardManaCost`,
//!   `ExiledWith$DifferentCardManaCost`)
//! - `Amount::DistinctCounterKindsOnSource` (`Count$DifferentCounterKinds_Card.Self`)
//! - `Amount::HighestOpponentHandSize` (`PlayerCountOpponents$HighestCardsInHand`)
//! - `Amount::IfCondition` (`Count$Compare <var> <cmp>.<then>.<else>` in resolution)
//! - `Condition::CountersPlacedOnSourceThisTurn` (`Count$CountersAddedThisTurn <kind> <who> Card.Self`)
//! - `Condition::AmountCompare` (`CheckSVar$` / `ConditionCheckSVar$` comparing dynamic amounts)

use super::{Tx, counter_kind};

/// Filter-free, static-safe Wave 15a `Amount` atoms usable from both
/// `atoms::amount` (including inline `Count$...` values) and `Tx::count_expr`.
pub(super) fn static_amount_atom(head: &str) -> Option<&'static str> {
    Some(match head.trim() {
        "PlayerCountOpponents$HighestCardsInHand" => "Amount::HighestOpponentHandSize",
        "Count$DifferentCounterKinds_Card.Self" | "Count$DifferentCounterKinds_Self" => {
            "Amount::DistinctCounterKindsOnSource"
        }
        "Count$ThisTurnCast_Card.YouCtrl$CardTypes" => {
            "Amount::DistinctCardTypesCastThisTurn(PlayerRel::You)"
        }
        "Count$ThisTurnCast_Card.OppCtrl$CardTypes" => {
            "Amount::DistinctCardTypesCastThisTurn(PlayerRel::EachOpponent)"
        }
        "Count$ThisTurnCast_Card$CardTypes" => {
            "Amount::DistinctCardTypesCastThisTurn(PlayerRel::EachPlayer)"
        }
        "PlayerCountPlayers$AttackersDeclared" => {
            "Amount::CreaturesAttackedThisTurn { filter: &Filter::Any, controller: PlayerRel::EachPlayer }"
        }
        _ => return None,
    })
}

/// Parses a two-letter Forge `SVarCompare$` operator prefix into an `AmountCmp`.
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

/// Maps a Forge `Count$CountersAddedThisTurn <kind> <who> <target>` expression
/// onto `Condition::CountersPlacedOnSourceThisTurn { kind, by_you }`.
///
/// Refuses honestly when:
/// - `<kind>` is `Any` or an unmodelled counter kind (`CountersPlacedOnSourceThisTurn`
///   requires a specific `CounterKind`),
/// - `<who>` is neither `You` nor `Player`,
/// - `<target>` is not `Card.Self` (e.g. `Creature.YouCtrl` or `Card.EffectSource`), or
/// - `cmp` asks for a numeric count other than zero vs at-least-one (`GE2`, `EQ1`, etc.).
fn counters_placed_on_source_condition(raw: &str, cmp: &str) -> Option<String> {
    let rest = raw.trim().strip_prefix("Count$CountersAddedThisTurn ")?;
    let mut parts = rest.split_whitespace();
    let (Some(kind_raw), Some(who_raw), Some(target_raw), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if kind_raw.eq_ignore_ascii_case("Any") {
        return None;
    }
    let kind = counter_kind(kind_raw)?;
    let by_you = match who_raw {
        "You" => true,
        "Player" => false,
        _ => return None,
    };
    if target_raw != "Card.Self" {
        return None;
    }
    let cond =
        format!("Condition::CountersPlacedOnSourceThisTurn {{ kind: {kind}, by_you: {by_you} }}");
    match cmp.trim() {
        "GE1" | "GT0" | "NE0" => Some(cond),
        "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{cond})")),
        _ => None,
    }
}

/// Checks whether a Forge valid-clause contains an exact atom (`+` or `.` delimited).
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

/// Strips `target_atom` from every comma-separated alternative if every
/// alternative carries it and carries no conflicting scope atom.
fn strip_uniform_scope(valid: &str, target_atom: &str) -> Option<String> {
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
            } else if atom.contains("Ctrl")
                || atom.contains("Own")
                || atom.starts_with("ControlledBy ")
                || atom.starts_with("OwnedBy ")
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

/// Strips `Other` / `StrictlyOther` if present on every comma-separated alternative.
/// Returns `None` if mixed (present on some alternatives but not others).
fn extract_uniform_other(valid: &str) -> Option<(String, bool)> {
    let mut any_other = false;
    let mut all_other = true;
    let mut out = Vec::new();
    for alt in valid.split(',') {
        let (base, atoms) = split_valid_alt(alt);
        if base.is_empty() {
            return None;
        }
        let mut found = false;
        let mut kept = Vec::new();
        for atom in atoms {
            if matches!(atom, "Other" | "StrictlyOther") {
                found = true;
            } else {
                kept.push(atom);
            }
        }
        any_other |= found;
        all_other &= found;
        out.push(rebuild_valid_alt(base, &kept));
    }
    if any_other && !all_other {
        return None;
    }
    Some((out.join(","), all_other))
}

fn has_unmodelled_turn_atom(valid: &str) -> bool {
    valid.contains(['$', '/', ';'])
        || valid.split(',').any(|alt| {
            let (base, atoms) = split_valid_alt(alt);
            base.is_empty()
                || atoms.iter().any(|a| {
                    matches!(
                        *a,
                        "tokenCreated"
                            | "Self"
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
                    ) || a.starts_with("CastSa ")
                        || a.starts_with("sharesNameWith ")
                        || a.contains("TriggeredActivator")
                })
        })
}

fn parse_from_zone(zone: &str) -> Option<&'static str> {
    Some(match zone {
        "Battlefield" => "ZoneRef::Battlefield",
        "Graveyard" => "ZoneRef::Graveyard",
        "Hand" => "ZoneRef::Hand",
        "Library" => "ZoneRef::Library",
        "Exile" => "ZoneRef::Exile",
        "Stack" => "ZoneRef::Stack",
        "Command" => "ZoneRef::Command",
        _ => return None,
    })
}

impl Tx<'_> {
    fn wave27a_filter_ref(&mut self, expr: &str) -> String {
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
            "Filter::Suspected" => "&Filter::Suspected".to_string(),
            _ => format!("&{}", self.body.filter_static("COUNT", expr)),
        }
    }

    /// Wave 27a `Amount` reader plugged into `mana::count_expr_depth`.
    pub(super) fn wave27a_count_expr(
        &mut self,
        def: &str,
        depth: u8,
        static_only: bool,
    ) -> Option<String> {
        let def = def.trim();
        if !static_only
            && depth < 3
            && let Some((lhs, k_str)) = def.rsplit_once("/LimitMax.")
            && let Ok(k) = k_str.trim().parse::<u32>()
            && k >= 1
        {
            let base = self.count_expr_depth(lhs.trim(), depth + 1, false)?;
            if base.starts_with("Amount::Fixed(") || base.contains("Neg") {
                return None;
            }
            let otherwise = if k == 1 {
                "Amount::Fixed(0)".to_string()
            } else {
                base.clone()
            };
            return Some(format!(
                "Amount::IfCondition {{ condition: &Condition::AmountCompare {{ left: &{base}, op: AmountCmp::AtLeast, right: &Amount::Fixed({k}) }}, then: &Amount::Fixed({k}), otherwise: &{otherwise} }}"
            ));
        }
        if let Some(atom) = static_amount_atom(def) {
            return Some(atom.to_string());
        }
        if def == "ExiledWith$DifferentCardManaCost" && !self.on_a_spell {
            return Some(
                "Amount::DistinctManaValuesInZone { zone: ZoneSel::LinkedExile, filter: &Filter::Any }"
                    .to_string(),
            );
        }
        if def.ends_with("$DifferentCardManaCost") {
            return self.wave27a_distinct_mana_values(def);
        }
        if let Some(rest) = def.strip_prefix("Count$ThisTurnEntered_") {
            return self.wave27a_entered_this_turn(rest);
        }
        if let Some(rest) = def.strip_prefix("Count$ThisTurnCast_") {
            return self.wave27a_spells_cast_matching(rest);
        }
        if let Some(valid) = def.strip_prefix("Count$CreaturesAttackedThisTurn ") {
            return self.wave27a_creatures_attacked_this_turn(valid.trim());
        }
        if !static_only
            && depth < 3
            && let Some(rest) = def.strip_prefix("Count$Compare ")
        {
            return self.wave27a_compare_amount(rest, depth);
        }
        None
    }

    /// Static-safe subset of `wave27a_count_expr` usable from `statics::wave14_amount_depth`.
    /// Rejects filters that scan P/T when `allow_power_scan` is false (Layer 7c).
    pub(super) fn wave27a_static_amount(
        &mut self,
        def: &str,
        allow_power_scan: bool,
        allow_turn_counts: bool,
    ) -> Option<String> {
        let def = def.trim();
        if !allow_turn_counts
            && (def.starts_with("Count$ThisTurnEntered_")
                || def.starts_with("Count$ThisTurnCast_")
                || def.starts_with("Count$CreaturesAttackedThisTurn ")
                || def == "PlayerCountPlayers$AttackersDeclared")
        {
            return None;
        }
        let expr = self.wave27a_count_expr(def, 0, true)?;
        if !allow_power_scan && (expr.contains("Power") || expr.contains("Toughness")) {
            return None;
        }
        Some(expr)
    }

    fn wave27a_distinct_mana_values(&mut self, def: &str) -> Option<String> {
        let body = def.strip_suffix("$DifferentCardManaCost")?.trim();
        let (zone, expr) = if let Some(valid) = body.strip_prefix("Count$ValidGraveyard ") {
            self.wave27a_graveyard_mv_filter(valid.trim())?
        } else if let Some(valid) = body.strip_prefix("Count$Valid ") {
            let valid = valid.trim();
            if valid.is_empty() || valid.contains(['$', '/']) {
                return None;
            }
            ("ZoneSel::Battlefield", self.filter_expr(valid)?)
        } else if let Some(valid) = body.strip_prefix("Count$ValidExile ") {
            self.wave27a_exile_mv_filter(valid.trim())?
        } else if let Some(valid) = body.strip_prefix("Count$ValidHand ") {
            let valid = valid.trim();
            if valid.is_empty() || valid.contains(['$', '/']) {
                return None;
            }
            let stripped = strip_uniform_scope(valid, "YouOwn")?;
            (
                "ZoneSel::HandYou",
                self.filter_expr_in_zone(&stripped, true)?,
            )
        } else {
            let valid = body.strip_prefix("Count$ValidLibrary ")?.trim();
            if valid.is_empty() || valid.contains(['$', '/']) {
                return None;
            }
            let stripped = strip_uniform_scope(valid, "YouOwn")?;
            (
                "ZoneSel::LibraryYou",
                self.filter_expr_in_zone(&stripped, true)?,
            )
        };
        let f_ref = self.wave27a_filter_ref(&expr);
        Some(format!(
            "Amount::DistinctManaValuesInZone {{ zone: {zone}, filter: {f_ref} }}"
        ))
    }

    fn wave27a_graveyard_mv_filter(&self, valid: &str) -> Option<(&'static str, String)> {
        if valid.is_empty() || valid.contains(['$', '/']) {
            return None;
        }
        if !valid.contains("Ctrl") && !valid.contains("Own") {
            return Some((
                "ZoneSel::GraveyardAll",
                self.filter_expr_in_zone(valid, true)?,
            ));
        }
        if let Some(stripped) = strip_uniform_scope(valid, "YouOwn") {
            return Some((
                "ZoneSel::GraveyardOf(PlayerRel::You)",
                self.filter_expr_in_zone(&stripped, true)?,
            ));
        }
        let stripped = strip_uniform_scope(valid, "OppOwn")?;
        let lower = self.oracle.to_ascii_lowercase();
        if lower.contains("target opponent's graveyard")
            || lower.contains("target player's graveyard")
        {
            return None;
        }
        Some((
            "ZoneSel::GraveyardOpponents",
            self.filter_expr_in_zone(&stripped, true)?,
        ))
    }

    fn wave27a_exile_mv_filter(&self, valid: &str) -> Option<(&'static str, String)> {
        if valid.is_empty() || valid.contains(['$', '/']) {
            return None;
        }
        if valid == "Card.ExiledWithSource" {
            return Some(("ZoneSel::LinkedExile", "Filter::Any".to_string()));
        }
        if let Some(stripped) = valid
            .strip_suffix("+ExiledWithSource")
            .or_else(|| valid.strip_suffix(".ExiledWithSource"))
        {
            return Some((
                "ZoneSel::LinkedExile",
                self.filter_expr_in_zone(stripped, true)?,
            ));
        }
        if !valid.contains("ExiledWithSource") {
            return Some(("ZoneSel::ExileAll", self.filter_expr_in_zone(valid, true)?));
        }
        None
    }

    fn wave27a_entered_this_turn(&mut self, rest: &str) -> Option<String> {
        // Only public destination zones are recorded in `PerTurn::zone_changes`
        // (`Battlefield`, `Graveyard`, `Exile`, `Stack`, `Command`). Hidden
        // destinations (`Hand`, `Library`) are deliberately not tracked and
        // must refuse honestly.
        let (to_ref, to_is_bf, tail) = if let Some(t) = rest.strip_prefix("Battlefield_") {
            ("ZoneRef::Battlefield", true, t)
        } else if let Some(t) = rest.strip_prefix("Graveyard_") {
            ("ZoneRef::Graveyard", false, t)
        } else if let Some(t) = rest.strip_prefix("Exile_") {
            ("ZoneRef::Exile", false, t)
        } else if let Some(t) = rest.strip_prefix("Stack_") {
            ("ZoneRef::Stack", false, t)
        } else {
            let t = rest.strip_prefix("Command_")?;
            ("ZoneRef::Command", false, t)
        };
        let (from_ref, valid_raw) = if let Some(after_from) = tail.strip_prefix("from_") {
            let (zone_word, valid) = after_from.split_once('_')?;
            (Some(parse_from_zone(zone_word)?), valid.trim())
        } else {
            (None, tail.trim())
        };
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw) {
            return None;
        }
        let from_is_bf = from_ref == Some("ZoneRef::Battlefield");
        let (valid_no_other, exclude_source) = if to_is_bf {
            extract_uniform_other(valid_raw)?
        } else {
            (valid_raw.to_string(), false)
        };
        let no_scope = valid_no_other
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (owner_or_ctrl, cleaned) = if to_is_bf {
            if let Some(s) = strip_uniform_scope(&valid_no_other, "YouCtrl")
                .or_else(|| strip_uniform_scope(&valid_no_other, "ControlledBy CardController"))
            {
                ("PlayerRel::You", s)
            } else if let Some(s) = strip_uniform_scope(&valid_no_other, "OppCtrl") {
                ("PlayerRel::EachOpponent", s)
            } else if no_scope {
                ("PlayerRel::EachPlayer", valid_no_other)
            } else {
                return None;
            }
        } else if let Some(s) = strip_uniform_scope(&valid_no_other, "YouOwn") {
            ("PlayerRel::You", s)
        } else if let Some(s) = strip_uniform_scope(&valid_no_other, "OppOwn") {
            ("PlayerRel::EachOpponent", s)
        } else if let Some(s) = strip_uniform_scope(&valid_no_other, "EnchantedPlayerOwn") {
            ("PlayerRel::EnchantedPlayer", s)
        } else if no_scope {
            ("PlayerRel::EachPlayer", valid_no_other)
        } else if from_is_bf {
            // For departures from the battlefield (`Graveyard_from_Battlefield_Creature.YouCtrl`,
            // `Exile_from_Battlefield_Creature.OppCtrl`), `owner_or_ctrl` checks the
            // destination zone's owner (`record.owner`), whereas `filter` runs against
            // the departed permanent's frozen battlefield snapshot (preserving its
            // last controller). Keep `ControlledByYou` / `ControlledByOpponent` inside
            // `filter` with `owner_or_ctrl: PlayerRel::EachPlayer`.
            if !valid_no_other.split(',').all(|alt| {
                split_valid_alt(alt).1.iter().all(|a| {
                    matches!(*a, "YouCtrl" | "OppCtrl" | "YouOwn" | "OppOwn")
                        || (!a.contains("Ctrl")
                            && !a.contains("Own")
                            && !a.starts_with("ControlledBy ")
                            && !a.starts_with("OwnedBy "))
                })
            }) {
                return None;
            }
            ("PlayerRel::EachPlayer", valid_no_other)
        } else {
            return None;
        };
        let filter_expr = if to_is_bf || from_is_bf {
            self.filter_expr(&cleaned)?
        } else {
            self.filter_expr_in_zone(&cleaned, true)?
        };
        let f_ref = self.wave27a_filter_ref(&filter_expr);
        let from_expr = match from_ref {
            Some(z) => format!("Some({z})"),
            None => "None".to_string(),
        };
        Some(format!(
            "Amount::EnteredThisTurn {{ filter: {f_ref}, from: {from_expr}, to: {to_ref}, owner_or_ctrl: {owner_or_ctrl}, exclude_source: {exclude_source} }}"
        ))
    }

    fn wave27a_spells_cast_matching(&mut self, rest: &str) -> Option<String> {
        let valid_raw = rest.trim();
        if valid_raw.is_empty() || has_unmodelled_turn_atom(valid_raw) {
            return None;
        }
        // `Card.wasCastFromYourHandByYou` combines `caster: PlayerRel::You` and
        // `Filter::CastFromHand` in a single Forge atom.
        if valid_raw == "Card.wasCastFromYourHandByYou" {
            return Some(
                "Amount::SpellsCastMatchingThisTurn { filter: &Filter::CastFromHand, caster: PlayerRel::You }"
                    .to_string(),
            );
        }
        let no_scope = valid_raw
            .split(',')
            .all(|alt| !has_scope_atom(&split_valid_alt(alt).1));
        let (caster, stripped) = if let Some(s) = strip_uniform_scope(valid_raw, "YouCtrl") {
            ("PlayerRel::You", s)
        } else if let Some(s) = strip_uniform_scope(valid_raw, "OppCtrl") {
            ("PlayerRel::EachOpponent", s)
        } else if let Some(s) = strip_uniform_scope(valid_raw, "ControlledBy Player.Active") {
            ("PlayerRel::ActivePlayer", s)
        } else if self.trigger_mode.as_deref() == Some("Phase")
            && let Some(s) = strip_uniform_scope(valid_raw, "ControlledBy TriggeredPlayer")
        {
            ("PlayerRel::ActivePlayer", s)
        } else if no_scope {
            ("PlayerRel::EachPlayer", valid_raw.to_string())
        } else {
            return None;
        };
        // Rewrite `wasCastFromYourHand` / `!wasCastFromYourHand` to the zone-neutral
        // `wasCastFromHand` / `!wasCastFromHand` atoms when the caster is already `You`.
        let normalized = if caster == "PlayerRel::You" {
            let mut alts = Vec::new();
            for alt in stripped.split(',') {
                let (base, atoms) = split_valid_alt(alt);
                let mapped: Vec<&str> = atoms
                    .into_iter()
                    .map(|a| match a {
                        "wasCastFromYourHand" => "wasCastFromHand",
                        "!wasCastFromYourHand" => "!wasCastFromHand",
                        other => other,
                    })
                    .collect();
                alts.push(rebuild_valid_alt(base, &mapped));
            }
            alts.join(",")
        } else {
            stripped
        };
        let expr = self.filter_expr_in_zone(&normalized, true)?;
        // Unfiltered spell counts (`Count$ThisTurnCast_Card*`) belong to
        // `Amount::SpellsCastThisTurn` so existing readers keep emitting the
        // lightweight counter and preserve their refusal guards.
        if expr == "Filter::Any" {
            return None;
        }
        let f_ref = self.wave27a_filter_ref(&expr);
        Some(format!(
            "Amount::SpellsCastMatchingThisTurn {{ filter: {f_ref}, caster: {caster} }}"
        ))
    }

    fn wave27a_creatures_attacked_this_turn(&mut self, valid: &str) -> Option<String> {
        if valid.is_empty() || has_unmodelled_turn_atom(valid) {
            return None;
        }
        // In Forge (`AbilityUtils.java`), `Count$CreaturesAttackedThisTurn <valid>`
        // always queries `player.getCreaturesAttackedThisTurn()` (the controller
        // of the ability). Reject conflicting opponent/owner scopes honestly.
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',') {
            let (base, atoms) = split_valid_alt(alt);
            if base.is_empty() {
                return None;
            }
            let mut kept = Vec::new();
            for atom in atoms {
                if atom == "YouCtrl" {
                    continue;
                }
                if atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom.starts_with("ControlledBy ")
                    || atom.starts_with("OwnedBy ")
                {
                    return None;
                }
                kept.push(atom);
            }
            cleaned_alts.push(rebuild_valid_alt(base, &kept));
        }
        let cleaned = cleaned_alts.join(",");
        let expr = self.filter_expr(&cleaned)?;
        let f_ref = self.wave27a_filter_ref(&expr);
        Some(format!(
            "Amount::CreaturesAttackedThisTurn {{ filter: {f_ref}, controller: PlayerRel::You }}"
        ))
    }

    fn wave27a_compare_amount(&mut self, rest: &str, depth: u8) -> Option<String> {
        let (check_part, arms) = rest.split_once('.')?;
        let (then_raw, else_raw) = arms.split_once('.')?;
        if else_raw.contains('.') {
            return None;
        }
        let (lhs_var, cmp) = check_part.trim().split_once(' ')?;
        let cond = self.wave27a_compare_condition(lhs_var.trim(), cmp.trim())?;
        let then_amt = self.wave27a_compare_arm(then_raw.trim(), depth + 1)?;
        let else_amt = self.wave27a_compare_arm(else_raw.trim(), depth + 1)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }

    fn wave27a_compare_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        if let Some(cond) = self.svar_condition_clause(var, cmp) {
            return Some(cond);
        }
        let raw = self.resolve_svar_or_inline(var)?;
        if let Some(cond) = self.wave27a_trigger_svar_condition(&raw, cmp) {
            return Some(cond);
        }
        if self.cost_sacrifices_other
            && let Some(valid) = raw.strip_prefix("Sacrificed$Valid ")
            && !valid.contains(['$', '/'])
        {
            let expr = self.filter_expr(valid.trim())?;
            let f_ref = self.wave27a_filter_ref(&expr);
            let base = format!("Condition::SacrificedForCost({f_ref})");
            return match cmp {
                "GE1" | "EQ1" | "GT0" | "NE0" => Some(base),
                "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{base})")),
                _ => None,
            };
        }
        self.wave28b_compare_condition(var, cmp)
    }

    fn wave27a_compare_arm(&mut self, raw: &str, depth: u8) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        let def = self.svars.get(raw)?.clone();
        let amt = self.count_expr_depth(&def, depth, false)?;
        (!amt.contains("Neg")).then_some(amt)
    }

    /// Wave 27a `CheckSVar$` / `ConditionCheckSVar$` lowering shared by
    /// `saga_ward_convoke::extra_svar_condition_clause` (statics, activated
    /// abilities, effect `ConditionCheckSVar$`) and `triggers::wave15_trigger_svar_condition`.
    pub(super) fn wave27a_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.resolve_svar_or_inline(var)?;
        self.wave27a_trigger_svar_condition(&raw, cmp)
    }

    /// Lowering over an already-resolved `SVar` body (`raw`) and comparator (`cmp`).
    pub(super) fn wave27a_trigger_svar_condition(
        &mut self,
        raw: &str,
        cmp: &str,
    ) -> Option<String> {
        let raw = raw.trim();
        let cmp = cmp.trim();
        if let Some(cond) = counters_placed_on_source_condition(raw, cmp) {
            return Some(cond);
        }
        if let Some(cond) = self.wave27a_limit_max_sum_condition(raw, cmp) {
            return Some(cond);
        }
        // `LimitMax.1` on a non-negative count preserves boolean zero-vs-nonzero tests.
        if let Some(base_raw) = raw.strip_suffix("/LimitMax.1")
            && matches!(cmp, "GE1" | "GT0" | "EQ1" | "NE0" | "EQ0" | "LE0" | "LT1")
        {
            let lhs = self.static_count_expr(base_raw.trim())?;
            if lhs.starts_with("Amount::Fixed(") || lhs.contains("Neg") {
                return None;
            }
            let op = match cmp {
                "GE1" | "GT0" | "EQ1" | "NE0" => "AmountCmp::AtLeast",
                "EQ0" | "LE0" | "LT1" => "AmountCmp::Equal",
                _ => return None,
            };
            let n = usize::from(op == "AmountCmp::AtLeast");
            return Some(format!(
                "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &Amount::Fixed({n}) }}"
            ));
        }
        let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
        let op = parse_amount_cmp(op_code)?;
        let lhs = self.static_count_expr(raw)?;
        if lhs.starts_with("Amount::Fixed(") || lhs.contains("Neg") {
            return None;
        }
        let rhs = self.wave27a_rhs_amount(rhs_raw.trim())?;
        if lhs == rhs || rhs.contains("Neg") {
            return None;
        }
        Some(format!(
            "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &{rhs} }}"
        ))
    }

    fn wave27a_rhs_amount(&mut self, rhs_raw: &str) -> Option<String> {
        if rhs_raw.is_empty() {
            return None;
        }
        if let Ok(n) = rhs_raw.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        if let Some(def) = self.svars.get(rhs_raw).cloned() {
            let trimmed = def.trim();
            if let Some(n_str) = trimmed.strip_prefix("Number$")
                && let Ok(n) = n_str.trim().parse::<u32>()
            {
                return Some(format!("Amount::Fixed({n})"));
            }
            return self.static_count_expr(trimmed);
        }
        if rhs_raw.starts_with("Count$") || rhs_raw.starts_with("PlayerCount") {
            return self.static_count_expr(rhs_raw);
        }
        None
    }

    /// Lowers `SVar$<A>/Plus.<B>` where both `<A>` and `<B>` are `<count>/LimitMax.1`
    /// (e.g. *Eshki Dragonclaw*, *Fae Offering*, *Raka Sanctuary* cycle).
    /// Because each summand is `0` or `1`:
    /// - `GE2` / `EQ2` / `GT1` holds iff both base counts are `>= 1` (`Condition::All`),
    /// - `GE1` / `GT0` / `NE0` holds iff either base count is `>= 1` (`Condition::Any`),
    /// - `EQ0` / `LE0` / `LT1` holds iff both base counts are `== 0` (`Condition::All`).
    ///
    /// Any other comparator (e.g. `GE3`, `EQ1`) refuses honestly.
    fn wave27a_limit_max_sum_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        let (lhs_raw, rhs_var) = raw.split_once("/Plus.")?;
        let lhs_var = lhs_raw.trim().strip_prefix("SVar$")?.trim();
        let rhs_var = rhs_var.trim();
        let lhs_def = self.svars.get(lhs_var)?.trim().to_string();
        let rhs_def = self.svars.get(rhs_var)?.trim().to_string();
        let lhs_base = lhs_def.strip_suffix("/LimitMax.1")?.trim();
        let rhs_base = rhs_def.strip_suffix("/LimitMax.1")?.trim();
        let left = self.static_count_expr(lhs_base)?;
        let right = self.static_count_expr(rhs_base)?;
        if left.starts_with("Amount::Fixed(")
            || right.starts_with("Amount::Fixed(")
            || left.contains("Neg")
            || right.contains("Neg")
        {
            return None;
        }
        let ge1 = |a: &str| {
            format!(
                "Condition::AmountCompare {{ left: &{a}, op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}"
            )
        };
        let eq0 = |a: &str| {
            format!(
                "Condition::AmountCompare {{ left: &{a}, op: AmountCmp::Equal, right: &Amount::Fixed(0) }}"
            )
        };
        match cmp {
            "GE2" | "EQ2" | "GT1" => Some(format!(
                "Condition::All(&[{}, {}])",
                ge1(&left),
                ge1(&right)
            )),
            "GE1" | "GT0" | "NE0" => Some(format!(
                "Condition::Any(&[{}, {}])",
                ge1(&left),
                ge1(&right)
            )),
            "EQ0" | "LE0" | "LT1" => Some(format!(
                "Condition::All(&[{}, {}])",
                eq0(&left),
                eq0(&right)
            )),
            _ => None,
        }
    }
}
