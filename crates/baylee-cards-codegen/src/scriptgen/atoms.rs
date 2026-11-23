//! The small pieces a script line is built from: amounts, colours, stats,
//! counts, and the atoms and token stems a card's script names.

use super::{BTreeMap, CLUE_TOKEN_SCRIPT, CardScript, PROSE_KEYS, Params, keyword_const};

/// A whole number, or an `SVar` that resolves to one.
pub(super) fn amount(raw: &str, svars: &BTreeMap<String, String>, has_x: bool) -> Option<String> {
    let raw = raw.trim().trim_start_matches('+');
    if let Ok(n) = raw.parse::<i64>() {
        return Some(format!("Amount::Fixed({n})"));
    }
    // `X` is the number the player announced, and `Amount::X` is how the
    // engine reads it back — off the spell for a cast and off
    // `Engine::activation_x` for an activation. Two things have to be true
    // before that is the right reading, and both are checked because
    // neither is visible at the use site.
    //
    // The corpus's `X` is only *sometimes* that number: of the 356 scripts
    // that write `TokenAmount$ X`, 58 define `SVar:X:Count$xPaid` and the
    // rest count something — damage dealt, opponents, creatures in a
    // graveyard. So the definition is demanded rather than assumed.
    //
    // And `has_x` is the other half: a **triggered** ability announces no
    // number, so `Amount::X` would evaluate to `x.unwrap_or(0)` — a card
    // that compiles, claims `Implemented` and makes nothing at all, which
    // is exactly the outcome the honest-stub rule exists to prevent.
    if raw == "PreventedDamage" {
        return Some("Amount::ThisWay(ThisWay::DamagePrevented)".to_string());
    }
    if raw.starts_with("Count$")
        && let Some(expr) = static_wave14_amount_depth(raw, svars, 0)
    {
        return Some(expr);
    }
    // "For each creature that died this turn" (Scavenging Ghoul), counted as
    // the effect applies. Asked before the `X` rule, because `X` is the
    // letter the reference writes it under.
    if svars.get(raw).map(|def| def.trim())
        == Some("Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature")
    {
        return Some("Amount::CreaturesDiedThisTurn".to_string());
    }
    // "The damage dealt to you this turn" (Simulacrum, Discordant Spirit),
    // under the same letter and asked before it for the same reason.
    if svars.get(raw).map(|def| def.trim()) == Some("PlayerCountPropertyYou$DamageThisTurn") {
        return Some("Amount::DamageDealtToYouThisTurn".to_string());
    }
    // "For each time it was kicked" (CR 702.33d), including fixed multiples
    // (`/Twice`, `/Thrice`, `/Times.<n>`), which the reference writes under
    // the same letter.
    if let Some(def) = svars.get(raw).map(|def| def.trim()) {
        let (head, suffix) = match def.split_once('/') {
            Some((h, s)) => (h.trim(), Some(s.trim())),
            None => (def, None),
        };
        let base = match head {
            "Count$TimesKicked" => Some("Amount::TimesKicked"),
            "Count$CastTotalManaSpent Snow" => Some("Amount::SnowManaSpentToCast"),
            "Count$Devoured Card" => Some("Amount::EntrySacrifices"),
            _ => None,
        };
        if let Some(base) = base {
            return scaled_svar_amount(base, suffix);
        }
    }
    if let Some(kind) = svars
        .get(raw)
        .and_then(|def| def.trim().strip_prefix("Count$CardCounters."))
    {
        return super::counter_kind(kind).map(|kind| format!("Amount::CountersOnSource({kind})"));
    }
    if svars.get(raw).map(|def| def.trim()) == Some("Count$ValidHand Card.YouOwn") {
        return Some("Amount::HandSize(PlayerRel::You)".to_string());
    }
    if svars.get(raw).map(|def| def.trim()) == Some("TriggeredPlayer$CardsInHand/NMinus.3") {
        return Some(
            "Amount::Difference { left: &Amount::Fixed(3), right: &Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::HandActivePlayer } }"
                .to_string(),
        );
    }
    if svars.get(raw).map(|def| def.trim()) == Some("TriggeredPlayer$CardsInHand/Minus.4") {
        return Some(
            "Amount::Difference { left: &Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::HandActivePlayer }, right: &Amount::Fixed(4) }"
                .to_string(),
        );
    }
    if svars.get(raw).map(|def| def.trim()) == Some("Exiled$CardManaCost/Plus.1") {
        return Some("Amount::Plus { base: &Amount::SacrificedManaValue, offset: 1 }".to_string());
    }
    if svars.get(raw).map(|def| def.trim()) == Some("Sacrificed$CardManaCost") {
        return Some("Amount::SacrificedManaValue".to_string());
    }
    if svars.get(raw).map(|def| def.trim()) == Some("Targeted$CardManaCost") {
        return Some("Amount::TargetCmc".to_string());
    }
    if let Some(def) = svars.get(raw).map(|def| def.trim())
        && let Some(expr) = wave14_svar_amount_depth(def, svars, has_x, 0)
    {
        return Some(expr);
    }
    if raw == "X" {
        return (has_x && svars.get("X").map(String::as_str) == Some("Count$xPaid"))
            .then(|| "Amount::X".to_string());
    }
    let resolved = svars.get(raw)?;
    let n = resolved.trim().parse::<i64>().ok()?;
    Some(format!("Amount::Fixed({n})"))
}

fn scaled_svar_amount(base: &str, suffix: Option<&str>) -> Option<String> {
    let Some(rest) = suffix else {
        return Some(base.to_string());
    };
    let factor: u32 = match rest {
        "Twice" => 2,
        "Thrice" => 3,
        _ => rest.strip_prefix("Times.").and_then(|n| n.parse().ok())?,
    };
    if factor == 0 {
        return None;
    }
    Some(if factor == 1 {
        base.to_string()
    } else {
        format!("Amount::Times {{ base: &{base}, factor: {factor} }}")
    })
}

fn apply_wave14_suffix(base: &str, suffix: Option<&str>) -> Option<String> {
    let Some(rest) = suffix else {
        return Some(base.to_string());
    };
    match rest {
        "Twice" => Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}")),
        "Thrice" => Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}")),
        "HalfDown" => Some(format!("Amount::Half {{ base: &{base}, round_up: false }}")),
        "HalfUp" => Some(format!("Amount::Half {{ base: &{base}, round_up: true }}")),
        _ => {
            if let Some(n) = rest
                .strip_prefix("Times.")
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                return (n > 0).then(|| {
                    if n == 1 {
                        base.to_string()
                    } else {
                        format!("Amount::Times {{ base: &{base}, factor: {n} }}")
                    }
                });
            }
            if let Some(n) = rest
                .strip_prefix("Plus.")
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                return Some(if n == 0 {
                    base.to_string()
                } else {
                    format!("Amount::Plus {{ base: &{base}, offset: {n} }}")
                });
            }
            if let Some(n) = rest
                .strip_prefix("Minus.")
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                return Some(if n == 0 {
                    base.to_string()
                } else {
                    format!("Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}")
                });
            }
            if let Some(n) = rest
                .strip_prefix("Div.")
                .or_else(|| rest.strip_prefix("DivideEvenlyDown."))
                .and_then(|s| s.trim().parse::<u32>().ok())
                .filter(|&n| n >= 2)
            {
                return Some(format!("Amount::divided(&{base}, {n}, false)"));
            }
            if let Some(n) = rest
                .strip_prefix("DivideEvenlyUp.")
                .and_then(|s| s.trim().parse::<u32>().ok())
                .filter(|&n| n >= 2)
            {
                return Some(format!("Amount::divided(&{base}, {n}, true)"));
            }
            None
        }
    }
}

impl super::Tx<'_> {
    pub(super) fn resolution_chosen_amount(&self, raw: &str) -> Option<String> {
        if !self.has_resolution_chosen_number || self.in_delayed {
            return None;
        }
        let raw = raw.trim();
        let definition = self.svars.get(raw).map_or(raw, String::as_str);
        self.resolution_chosen_definition(definition, 0)
    }

    fn resolution_chosen_definition(&self, raw: &str, depth: u8) -> Option<String> {
        if depth >= 4 {
            return None;
        }
        let raw = raw.trim();
        let (head, suffix) = raw
            .rsplit_once('/')
            .map_or((raw, None), |(h, s)| (h.trim(), Some(s.trim())));
        if head == "Count$ChosenNumber" {
            return apply_wave14_suffix("Amount::ResolutionChosenNumber", suffix);
        }
        let definition = self.svars.get(head.strip_prefix("SVar$")?.trim())?;
        let base = self.resolution_chosen_definition(definition, depth + 1)?;
        apply_wave14_suffix(&base, suffix)
    }
}

fn greatest_power_filter(valid: &str) -> Option<&'static str> {
    Some(match valid.trim() {
        "Creature.YouCtrl" => "Filter::YOUR_CREATURE",
        "Creature.YouCtrl+Other" | "Creature.Other+YouCtrl" => {
            "Filter::ANOTHER_CREATURE_YOU_CONTROL"
        }
        "Creature.OppCtrl" => "Filter::OPPONENT_CREATURE",
        "Creature.YouDontCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ControlledByYou)])"
        }
        "Creature" => "Filter::CREATURE",
        "Creature.Other" => "Filter::ANOTHER_CREATURE",
        "Creature.attacking" => "Filter::ATTACKING_CREATURE",
        "Creature.attacking+YouCtrl" | "Creature.YouCtrl+attacking" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Attacking])"
        }
        "Creature.YouCtrl+Other+attacking" | "Creature.Other+YouCtrl+attacking" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Another, Filter::Attacking])"
        }
        "Creature.YouCtrl+nonToken"
        | "Creature.nonToken+YouCtrl"
        | "Creature.YouCtrl+!token"
        | "Creature.!token+YouCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Not(&Filter::IsToken)])"
        }
        "Creature.YouCtrl+ThisTurnEntered" | "Creature.ThisTurnEntered+YouCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::EnteredThisTurn])"
        }
        "Creature.tapped+OppCtrl" | "Creature.OppCtrl+tapped" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent, Filter::Tapped])"
        }
        "Creature.Legendary+YouCtrl" | "Creature.YouCtrl+Legendary" => {
            "Filter::And(&[Filter::LEGENDARY_CREATURE, Filter::ControlledByYou])"
        }
        "Creature.YouCtrl+withFlying" | "Creature.withFlying+YouCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::HasKeyword(KeywordSet::FLYING)])"
        }
        _ => return None,
    })
}

fn greatest_mana_value_filter(valid: &str) -> Option<&'static str> {
    Some(match valid.trim() {
        "Permanent.YouCtrl" | "Card.YouCtrl" => "Filter::ControlledByYou",
        "Permanent.YouCtrl+Other" | "Permanent.Other+YouCtrl" => {
            "Filter::And(&[Filter::ControlledByYou, Filter::Another])"
        }
        "Permanent.nonLand+YouCtrl"
        | "Permanent.YouCtrl+nonLand"
        | "Card.nonLand+YouCtrl"
        | "Card.YouCtrl+nonLand" => "Filter::And(&[Filter::NONLAND, Filter::ControlledByYou])",
        "Artifact.YouCtrl" => "Filter::YOUR_ARTIFACT",
        "Artifact.Other+YouCtrl" | "Artifact.YouCtrl+Other" => {
            "Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou, Filter::Another])"
        }
        "Creature.YouCtrl" => "Filter::YOUR_CREATURE",
        "Creature.Other+YouCtrl" | "Creature.YouCtrl+Other" => {
            "Filter::ANOTHER_CREATURE_YOU_CONTROL"
        }
        "Enchantment.YouCtrl" => "Filter::And(&[Filter::ENCHANTMENT, Filter::ControlledByYou])",
        "Permanent.Historic+YouCtrl" | "Permanent.YouCtrl+Historic" => {
            "Filter::And(&[Filter::Historic, Filter::ControlledByYou])"
        }
        "Permanent.IsCommander+YouCtrl"
        | "Permanent.YouCtrl+IsCommander"
        | "Card.IsCommander+YouCtrl"
        | "Card.YouCtrl+IsCommander" => "Filter::COMMANDER_YOU_CONTROL",
        "Permanent" | "Card" => "Filter::Any",
        "Permanent.nonLand" | "Card.nonLand" => "Filter::NONLAND",
        "Creature" => "Filter::CREATURE",
        "Artifact" => "Filter::ARTIFACT",
        "Enchantment" => "Filter::ENCHANTMENT",
        "Permanent.OppCtrl" | "Card.OppCtrl" => "Filter::ControlledByOpponent",
        "Creature.OppCtrl" => "Filter::OPPONENT_CREATURE",
        _ => return None,
    })
}

fn total_power_filter(valid: &str) -> Option<&'static str> {
    Some(match valid.trim() {
        "Creature.YouCtrl" => "Filter::YOUR_CREATURE",
        "Creature.attacking" => "Filter::ATTACKING_CREATURE",
        "Creature.attacking+YouCtrl" | "Creature.YouCtrl+attacking" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Attacking])"
        }
        "Creature.YouCtrl+Other" | "Creature.Other+YouCtrl" => {
            "Filter::ANOTHER_CREATURE_YOU_CONTROL"
        }
        "Creature.OppCtrl" => "Filter::OPPONENT_CREATURE",
        "Creature.YouDontCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ControlledByYou)])"
        }
        "Creature.YouCtrl+withFlying" | "Creature.withFlying+YouCtrl" => {
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::HasKeyword(KeywordSet::FLYING)])"
        }
        "Creature" => "Filter::CREATURE",
        "Card.IsCommander+YouCtrl"
        | "Card.YouCtrl+IsCommander"
        | "Creature.IsCommander+YouCtrl"
        | "Creature.YouCtrl+IsCommander" => "Filter::COMMANDER_YOU_CONTROL",
        "Creature.YouCtrl+IsRingbearer" | "Creature.IsRingbearer+YouCtrl" => {
            "Filter::YOUR_RING_BEARER"
        }
        _ => return None,
    })
}

fn static_wave14_base(head: &str) -> Option<String> {
    match head {
        "Count$Party" | "Count$PartySize" => {
            return Some("Amount::PartySize(PlayerRel::You)".to_string());
        }
        "Count$ThisTurnCast_Card.YouCtrl" => {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::You)".to_string());
        }
        "Count$ThisTurnCast_Card.OppCtrl" => {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::Opponent)".to_string());
        }
        "Count$ThisTurnCast_Card" => {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::EachPlayer)".to_string());
        }
        "Count$YouDrewThisTurn" => {
            return Some("Amount::CardsDrawnThisTurn(PlayerRel::You)".to_string());
        }
        "PlayerCountOpponents$CardsDrawn" => {
            return Some("Amount::CardsDrawnThisTurn(PlayerRel::Opponent)".to_string());
        }
        "Count$ValidGraveyard Creature.YouOwn$GreatestCardPower" => {
            return Some(
                "Amount::GreatestPowerAmong(&Filter::And(&[Filter::CREATURE, Filter::OwnedByYou, Filter::InZone(ZoneRef::Graveyard)]))"
                    .to_string(),
            );
        }
        "Count$ValidGraveyard Creature$GreatestCardPower" => {
            return Some(
                "Amount::GreatestPowerAmong(&Filter::And(&[Filter::CREATURE, Filter::InZone(ZoneRef::Graveyard)]))"
                    .to_string(),
            );
        }
        "Count$ValidGraveyard Card.YouOwn$GreatestCardManaCost" => {
            return Some(
                "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::Graveyard)]))"
                    .to_string(),
            );
        }
        "Count$ValidGraveyard Creature.YouOwn$GreatestCardManaCost" => {
            return Some(
                "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::CREATURE, Filter::OwnedByYou, Filter::InZone(ZoneRef::Graveyard)]))"
                    .to_string(),
            );
        }
        "Count$ValidBattlefield,Command Card.IsCommander+YouOwn$GreatestCardManaCost"
        | "Count$ValidBattlefield,Command Card.YouOwn+IsCommander$GreatestCardManaCost" => {
            return Some(
                "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::IsCommander, Filter::OwnedByYou, Filter::InZone(ZoneRef::BattlefieldOrCommand)]))"
                    .to_string(),
            );
        }
        _ => {}
    }
    if let Some(rest) = head.strip_prefix("Count$Valid ") {
        if let Some(valid) = rest.strip_suffix("$GreatestCardPower") {
            let filter = greatest_power_filter(valid)?;
            return Some(format!("Amount::GreatestPowerAmong(&{filter})"));
        }
        if let Some(valid) = rest.strip_suffix("$GreatestCardManaCost") {
            let filter = greatest_mana_value_filter(valid)?;
            return Some(format!("Amount::GreatestManaValueAmong(&{filter})"));
        }
        if let Some(valid) = rest.strip_suffix("$CardPower") {
            let filter = total_power_filter(valid)?;
            return Some(format!("Amount::TotalPowerAmong(&{filter})"));
        }
    }
    if let Some(valid) = head.strip_prefix("Count$TotalPower ") {
        let filter = total_power_filter(valid)?;
        return Some(format!("Amount::TotalPowerAmong(&{filter})"));
    }
    None
}

/// Static-safe Wave 14 `Amount::` `SVar` reader usable from activation limits,
/// activation cost reductions, and non-resolution conditions.
pub(super) fn static_wave14_amount(def: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    static_wave14_amount_depth(def, svars, 0)
}

fn static_wave14_amount_depth(
    def: &str,
    svars: &BTreeMap<String, String>,
    depth: u8,
) -> Option<String> {
    if depth >= 4 {
        return None;
    }
    let def = def.trim();
    let (head, suffix) = match def.rsplit_once('/') {
        Some((h, s)) => (h.trim(), Some(s.trim())),
        None => (def, None),
    };
    let base = if let Some(var) = head.strip_prefix("SVar$") {
        let inner = svars.get(var.trim())?;
        static_wave14_amount_depth(inner, svars, depth + 1)?
    } else {
        static_wave14_base(head)?
    };
    apply_wave14_suffix(&base, suffix)
}

fn wave14_svar_amount_depth(
    def: &str,
    svars: &BTreeMap<String, String>,
    has_x: bool,
    depth: u8,
) -> Option<String> {
    if depth >= 4 {
        return None;
    }
    let def = def.trim();
    let (head, suffix) = match def.rsplit_once('/') {
        Some((h, s)) => (h.trim(), Some(s.trim())),
        None => (def, None),
    };
    let base = if let Some(var) = head.strip_prefix("SVar$") {
        let inner = svars.get(var.trim())?;
        wave14_svar_amount_depth(inner, svars, has_x, depth + 1)?
    } else if let Some(static_base) = static_wave14_base(head) {
        static_base
    } else {
        match head {
            "TargetedPlayer$CardsDrawn" | "ThisTargetedPlayer$CardsDrawn" => {
                "Amount::CardsDrawnThisTurn(PlayerRel::Chosen)".to_string()
            }
            "Sacrificed$TotalCardPower" => "Amount::SacrificedTotalPower".to_string(),
            "Sacrificed$TotalCardToughness" => "Amount::SacrificedTotalToughness".to_string(),
            "Sacrificed$TotalCardManaCost" => "Amount::SacrificedTotalManaValue".to_string(),
            "Sacrificed$CardManaCost" if suffix.is_some() => {
                "Amount::SacrificedManaValue".to_string()
            }
            "Sacrificed$CardNumColors" | "Sacrificed$Colors" | "Sacrificed$TotalCardColors" => {
                "Amount::SacrificedDistinctColors".to_string()
            }
            "PreventedDamage" | "Count$PreventedDamage" => {
                "Amount::ThisWay(ThisWay::DamagePrevented)".to_string()
            }
            "Count$SacrificedThisWay" | "Count$SacrificedCount" => {
                "Amount::ThisWay(ThisWay::SacrificedCount)".to_string()
            }
            // `Sacrificed$Amount` maps to `ThisWay::SacrificedCount` only for
            // resolution sacrifices (not `Cost$ ... Sac<X/...>` where `X` is
            // `Count$xPaid`, since `ThisWay::SacrificedCount` counts `Cause::Effect`).
            "Sacrificed$Amount"
                if !(has_x && svars.get("X").map(String::as_str) == Some("Count$xPaid")) =>
            {
                "Amount::ThisWay(ThisWay::SacrificedCount)".to_string()
            }
            _ => return None,
        }
    };
    apply_wave14_suffix(&base, suffix)
}

/// `W` as `ManaColor::White`, for a reader that is not inside a closure.
pub(super) fn mana_color_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "W" => "ManaColor::White",
        "U" => "ManaColor::Blue",
        "B" => "ManaColor::Black",
        "R" => "ManaColor::Red",
        "G" => "ManaColor::Green",
        "C" => "ManaColor::Colorless",
        _ => return None,
    })
}

/// `UR` as `{U/R}`, or `None` when the token is not a hybrid pair.
///
/// The reference runs the two letters together and the printed card puts a
/// slash between them; `ColorPair` keeps the printed order — `{G/U}`, not
/// `{U/G}` — so the pair is written the way it is read.
pub(super) fn hybrid_pair(token: &str) -> Option<String> {
    let mut letters = token.chars();
    let (Some(a), Some(b), None) = (letters.next(), letters.next(), letters.next()) else {
        return None;
    };
    let colored = |c: char| "WUBRG".contains(c);
    (colored(a) && colored(b) && a != b).then(|| format!("{{{a}/{b}}}"))
}

/// A `NumAtt`/`NumDef` value as an `Amount`.
///
/// Separate from [`amount`] because a pump is the one place a *negative*
/// constant is ordinary, and `Amount::Fixed` holds a `u32` — the sign
/// lives in the variant, not in the number. Everything else it asks is the
/// same, and it asked none of it until a batch of six hundred cards walked
/// seven finished pumps into the pool that pump by nothing at all.
pub(super) fn pump_amount(
    raw: &str,
    svars: &BTreeMap<String, String>,
    has_x: bool,
) -> Option<String> {
    let raw = raw.trim();
    // `+X/+X` is the second commonest pump printed, and the two questions
    // [`amount`] asks of an `X` are asked here for the same two reasons.
    //
    // The letter is not the number. 207 scripts in the reference pump by
    // `X`, every one of them defines `SVar:X`, and only **44** define it as
    // `Count$xPaid` — the X the player announced, which `Amount::X` reads
    // straight back off the spell. The other 163 are *counts*:
    // `Count$Domain`, `Count$Valid Artifact.YouCtrl`, the greatest mana
    // value among permanents you control, and the DSL has no way to say any
    // of them. Written as `Amount::X` regardless, Gaea's Might came out
    // claiming `Implemented` and giving +0/+0, and six more with it.
    //
    // And `has_x` is the other half, because a triggered ability announces
    // no number at all: there `Amount::X` is `x.unwrap_or(0)`.
    match raw {
        "X" | "+X" | "-X" => {
            if has_x && svars.get("X").map(String::as_str) == Some("Count$xPaid") {
                return Some(
                    if raw == "-X" {
                        "Amount::NegX"
                    } else {
                        "Amount::X"
                    }
                    .to_string(),
                );
            }
            if let Some(def) = svars.get("X")
                && let Some(inner) = wave14_svar_amount_depth(def, svars, has_x, 0)
            {
                return Some(if raw == "-X" {
                    format!("Amount::Negated(&{inner})")
                } else {
                    inner
                });
            }
            return None;
        }
        _ => {}
    }
    if let Ok(n) = raw.parse::<i64>() {
        return Some(if n < 0 {
            format!("Amount::NegXFixed({})", n.unsigned_abs())
        } else {
            format!("Amount::Fixed({n})")
        });
    }
    let (neg, var) = if let Some(rest) = raw.strip_prefix('-') {
        (true, rest.trim())
    } else if let Some(rest) = raw.strip_prefix('+') {
        (false, rest.trim())
    } else {
        (false, raw)
    };
    if var == "PreventedDamage" {
        let inner = "Amount::ThisWay(ThisWay::DamagePrevented)";
        return Some(if neg {
            format!("Amount::Negated(&{inner})")
        } else {
            inner.to_string()
        });
    }
    if !neg && let Some(n) = svars.get(var).and_then(|s| s.trim().parse::<i64>().ok()) {
        return Some(if n < 0 {
            format!("Amount::NegXFixed({})", n.unsigned_abs())
        } else {
            format!("Amount::Fixed({n})")
        });
    }
    let resolved = svars.get(var)?;
    let inner = wave14_svar_amount_depth(resolved, svars, has_x, 0)?;
    Some(if neg {
        format!("Amount::Negated(&{inner})")
    } else {
        inner
    })
}

const PRINTED_KEYWORDS: &[&str] = &[
    "Flying",
    "First Strike",
    "Double Strike",
    "Deathtouch",
    "Haste",
    "Hexproof",
    "Indestructible",
    "Lifelink",
    "Menace",
    "Reach",
    "Trample",
    "Vigilance",
    "Defender",
    "Flash",
    "Shroud",
    "Shadow",
    "Fear",
    "Intimidate",
    "Infect",
    "Flanking",
    "Wither",
    "Horsemanship",
    "Skulk",
    "Enlist",
    "Persist",
    "Undying",
    "Phasing",
    "Rebound",
    "Prowess",
    "Devoid",
    "Changeling",
    "Banding",
    "Storm",
    "Battle Cry",
    "Dethrone",
    "Melee",
    "Mentor",
    "Training",
    "Myriad",
    "Extort",
    "Soulbond",
    "Ascend",
    "Daybound",
    "Nightbound",
    "Split Second",
    "Read Ahead",
    "Exploit",
    "Evolve",
    "Riot",
    "Unleash",
    "Umbra Armor",
];

/// `withFlying` / `hasKeywordFlying` as `Filter::HasKeyword(KeywordSet::FLYING)`,
/// `withoutFlying` / `!withFlying` / `!hasKeywordFlying` as its negation.
/// Matches both spaced (`withFirst Strike`) and unspaced (`withFirstStrike`)
/// spellings against every engine-enforced keyword bit.
pub(super) fn keyword_atom(atom: &str) -> Option<String> {
    if let Some(saddled) = super::wave27d::saddled_filter_atom(atom) {
        return Some(saddled);
    }
    // CR 700.9: modified creature predicate (`Creature.modified`,
    // `Card.Self+modified`, `Creature.isModified`, `Creature.!modified`).
    match atom {
        "modified" | "isModified" | "IsModified" => return Some("Filter::Modified".to_string()),
        "!modified" | "nonModified" | "!isModified" | "!IsModified" => {
            return Some("Filter::Not(&Filter::Modified)".to_string());
        }
        _ => {}
    }
    let (negated, name) = if let Some(rest) = atom
        .strip_prefix("without")
        .or_else(|| atom.strip_prefix("!with"))
        .or_else(|| atom.strip_prefix("!hasKeyword"))
    {
        (true, rest)
    } else {
        let rest = atom
            .strip_prefix("with")
            .or_else(|| atom.strip_prefix("hasKeyword"))?;
        (false, rest)
    };
    if matches!(name, "Cumulative upkeep" | "CumulativeUpkeep") {
        let has = "Filter::HasCumulativeUpkeep";
        return Some(if negated {
            format!("Filter::Not(&{has})")
        } else {
            has.to_string()
        });
    }
    let direct_kw = match name {
        "Modular" => Some("KeywordSet::MODULAR"),
        "Renown" => Some("KeywordSet::RENOWN"),
        "Partner" => Some("KeywordSet::PARTNER"),
        "Retrace" => Some("KeywordSet::RETRACE"),
        "Warp" => Some("KeywordSet::WARP"),
        "Bloodthirst" => Some("KeywordSet::BLOODTHIRST"),
        "Devour" => Some("KeywordSet::DEVOUR"),
        "Spree" => Some("KeywordSet::SPREE"),
        "Provoke" => Some("KeywordSet::PROVOKE"),
        "Mobilize" => Some("KeywordSet::MOBILIZE"),
        "Transmute" => Some("KeywordSet::TRANSMUTE"),
        "Scavenge" => Some("KeywordSet::SCAVENGE"),
        "Firebending" => Some("KeywordSet::FIREBENDING"),
        "Freerunning" => Some("KeywordSet::FREERUNNING"),
        "Sneak" => Some("KeywordSet::SNEAK"),
        "Sunburst" => Some("KeywordSet::SUNBURST"),
        "Ravenous" => Some("KeywordSet::RAVENOUS"),
        "Graft" => Some("KeywordSet::GRAFT"),
        "Outlast" => Some("KeywordSet::OUTLAST"),
        "Afterlife" => Some("KeywordSet::AFTERLIFE"),
        "Afflict" => Some("KeywordSet::AFFLICT"),
        "Reinforce" => Some("KeywordSet::REINFORCE"),
        "Splice" => Some("KeywordSet::SPLICE"),
        "Awaken" => Some("KeywordSet::AWAKEN"),
        "Conspire" => Some("KeywordSet::CONSPIRE"),
        "Cipher" => Some("KeywordSet::CIPHER"),
        "Champion" => Some("KeywordSet::CHAMPION"),
        "Tribute" => Some("KeywordSet::TRIBUTE"),
        "Plainswalk" | "Landwalk:Plains" => Some("KeywordSet::PLAINSWALK"),
        "Islandwalk" | "Landwalk:Island" => Some("KeywordSet::ISLANDWALK"),
        "Swampwalk" | "Landwalk:Swamp" => Some("KeywordSet::SWAMPWALK"),
        "Mountainwalk" | "Landwalk:Mountain" => Some("KeywordSet::MOUNTAINWALK"),
        "Forestwalk" | "Landwalk:Forest" => Some("KeywordSet::FORESTWALK"),
        _ => None,
    };
    let kw_bit = if let Some(bit) = direct_kw.or_else(|| keyword_const(name)) {
        bit
    } else {
        let printed = PRINTED_KEYWORDS.iter().copied().find(|p| {
            p.eq_ignore_ascii_case(name) || p.replace(' ', "").eq_ignore_ascii_case(name)
        })?;
        keyword_const(printed)?
    };
    let has = format!("Filter::HasKeyword({kw_bit})");
    Some(if negated {
        format!("Filter::Not(&{has})")
    } else {
        has
    })
}

/// "Creatures named Plague Rats": the name in a `named…` atom. A name is a
/// characteristic (CR 201.2), compared as the object carries it now. Only a
/// plain name: the reference also writes counts and limits after one
/// (`namedHedron Alignment/LimitMax`).
pub(super) fn named_atom(atom: &str) -> Option<&str> {
    atom.strip_prefix("named").filter(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || " '-".contains(c))
    })
}

/// A colour word in a valid-string, and whether it is negated: `Black` is
/// `(false, "Black")` and `nonBlack` / `!Black` is `(true, "Black")`, the
/// second half being the `Color` variant's name.
pub(super) fn color_atom(atom: &str) -> Option<(bool, &'static str)> {
    let (negated, word) = atom
        .strip_prefix("non")
        .or_else(|| atom.strip_prefix('!'))
        .map_or((false, atom), |rest| (true, rest));
    let color = match word {
        "White" => "White",
        "Blue" => "Blue",
        "Black" => "Black",
        "Red" => "Red",
        "Green" => "Green",
        _ => return None,
    };
    Some((negated, color))
}

/// A power or toughness compared with a fixed number (`powerLE2`,
/// `toughnessGE4`), as the `Filter` that asks it. The strict comparisons
/// become the inclusive ones a step over, because the filters compare
/// inclusively and a power is a whole number.
pub(super) fn stat_atom(atom: &str) -> Option<String> {
    let lower = atom.to_ascii_lowercase();
    let (stat, rest) = if let Some(rest) = lower.strip_prefix("power") {
        ("Power", rest)
    } else {
        ("Toughness", lower.strip_prefix("toughness")?)
    };
    let (cmp, number) = rest.split_at_checked(2)?;
    let n: i16 = number.parse().ok()?;
    let (bound, n) = match cmp {
        "le" => ("AtMost", n),
        "lt" => ("AtMost", n.checked_sub(1)?),
        "ge" => ("AtLeast", n),
        "gt" => ("AtLeast", n.checked_add(1)?),
        "eq" => {
            return Some(format!(
                "Filter::And(&[Filter::{stat}AtMost({n}), Filter::{stat}AtLeast({n})])"
            ));
        }
        _ => return None,
    };
    Some(format!("Filter::{stat}{bound}({n})"))
}

pub(super) fn plain_number(raw: &str, svars: &BTreeMap<String, String>) -> Option<i64> {
    let raw = raw.trim().trim_start_matches('+');
    raw.parse::<i64>()
        .ok()
        .or_else(|| svars.get(raw)?.trim().parse::<i64>().ok())
}

/// A `PresentCompare$` as the bound a count condition says: `GE2` is at least
/// two, `EQ0` none ("if no creatures are on the battlefield"), `LT3` at most
/// two. An exact count above zero is two bounds at once, which one
/// `Condition` cannot say.
pub(super) fn count_bound(compare: &str) -> Option<Bound> {
    let (cmp, number) = compare.split_at_checked(2)?;
    let n: u16 = number.parse().ok()?;
    let fits = |n: u16| u8::try_from(n).is_ok().then_some(n);
    Some(match cmp {
        "GE" => Bound::AtLeast(fits(n)?),
        "GT" => Bound::AtLeast(fits(n.checked_add(1)?)?),
        "LE" => Bound::AtMost(fits(n)?),
        "LT" => Bound::AtMost(fits(n.checked_sub(1)?)?),
        "EQ" if n == 0 => Bound::AtMost(0),
        _ => return None,
    })
}

/// Which side of a count an enters-tapped clause is on, once the reference's
/// comparator has been read.
///
/// The reference states when the land comes down **tapped** and the card
/// prints when it does not, so every comparator [`Tx::enters_tapped_unless`]
/// sees is already the opposite of what a player reads. Naming the two
/// directions keeps that inversion in one place: the arithmetic happens where
/// the comparator is parsed, and the emitter picks its variant from a word
/// instead of re-deriving a direction from a number it was handed.
#[derive(Clone, Copy)]
pub(super) enum Bound {
    /// "unless you control N or more" — a slow land, a battle land, and at
    /// one a checkland.
    AtLeast(u16),
    /// "unless you control N or fewer" — a fast land, and the same predicate
    /// the manlands print as "if you control N+1 or more, it enters tapped".
    AtMost(u16),
}

/// Every token script this card names, by the stem its `TokenScript$` gives.
///
/// Read off the raw text of the rules lines and the `SVar` bodies rather than
/// from a parsed effect, because it is asked **before** the card is
/// transcoded: the ledger has to have given the token an id before a card may
/// name the constant it sits at, and a card is refused for a dozen reasons
/// that have nothing to do with its token. Naming a stem here is therefore
/// not a claim that the card will be read — it is a claim that *this token*
/// is one a card in this pool reaches for.
///
/// Every stem in the corpus is `[A-Za-z0-9_]`, so the scan ends at the first
/// character outside that set and needs no `|` splitting.
#[must_use]
pub fn token_stems(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    collect_token_stems(script, &mut out);
    out
}

fn collect_token_stems(script: &CardScript, out: &mut Vec<String>) {
    let bodies = script
        .rules
        .iter()
        .map(|(_, body)| body.as_str())
        .chain(script.svars.values().map(String::as_str));
    for body in bodies {
        for at in body.match_indices("TokenScript$").map(|(i, _)| i) {
            let rest = body[at + "TokenScript$".len()..].trim_start();
            let stem: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !stem.is_empty() && !out.contains(&stem) {
                out.push(stem);
            }
        }
        // The token a line names by not naming it. `Investigate` writes no
        // `TokenScript$` at all — CR 701.16a supplies which token it is —
        // so a scan for that key alone tells the ledger the card needs no
        // token, while [`Tx::investigate_effect`] is about to emit the
        // constant the ledger was never asked to assign. The emitter and
        // the ledger have to name the same tokens or one of them is wrong,
        // and today it survives only because another card happens to name
        // the Clue outright.
        //
        // Read through [`Params::parse`] and never for the word: every one
        // of these lines also *prints* it, and `SpellDescription$
        // Investigate. (Create a Clue token…)` is on 21 of the corpus's
        // 112.
        if let Some((api, mut p)) = Params::parse(body) {
            if api == "Investigate" && !out.iter().any(|stem| stem == CLUE_TOKEN_SCRIPT) {
                out.push(CLUE_TOKEN_SCRIPT.to_string());
            } else if api == "Amass"
                && let Some(raw_type) = p.take("Type")
            {
                let sub = raw_type.trim().to_ascii_lowercase();
                if !sub.is_empty() && sub.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    let stem = format!("b_0_0_{sub}_army");
                    if !out.contains(&stem) {
                        out.push(stem);
                    }
                }
            }
        }
    }
    for line in &script.keywords {
        let trimmed = line.trim();
        let stem = match trimmed {
            "Living Weapon" | "Living weapon" => Some("b_0_0_phyrexian_germ"),
            "For Mirrodin" | "For Mirrodin!" => Some("r_2_2_rebel"),
            "Job select" => Some("c_1_1_hero"),
            _ if trimmed.starts_with("Fabricate:") => Some("c_1_1_a_servo"),
            _ => None,
        };
        if let Some(stem) = stem
            && !out.iter().any(|existing| existing == stem)
        {
            out.push(stem.to_string());
        }
    }
    if let Some(back) = script.alternate.as_deref() {
        collect_token_stems(back, out);
    }
}

fn collect_atoms(script: &CardScript, out: &mut Vec<String>) {
    if let Some(mode) = &script.alternate_mode {
        out.push(format!("alt:{mode}"));
    }
    for line in &script.keywords {
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        out.push(format!("kw:{head}"));
    }
    for line in &script.unknown_lines {
        let head = line.split(':').next().unwrap_or(line);
        out.push(format!("line:{head}"));
    }
    for (kind, spec) in &script.rules {
        if matches!(kind, 'S' | 'R') {
            out.push(format!("line:{kind}"));
        }
        let mut queue = vec![spec.clone()];
        let mut seen = 0usize;
        while let Some(spec) = queue.pop() {
            seen += 1;
            if seen > 32 {
                break; // a malformed chain must not spin here
            }
            let Some((api, params)) = Params::parse(&spec) else {
                continue;
            };
            out.push(format!("api:{api}"));
            for (key, _) in &params.entries {
                if !PROSE_KEYS.contains(&key.as_str()) {
                    out.push(format!("param:{api}.{key}"));
                }
            }
            for part in spec.split(" | ") {
                for prefix in ["SubAbility$ ", "Execute$ "] {
                    if let Some(name) = part.strip_prefix(prefix)
                        && let Some(body) = script.svars.get(name.trim())
                    {
                        queue.push(body.clone());
                    }
                }
            }
        }
    }
    if let Some(back) = script.alternate.as_deref() {
        collect_atoms(back, out);
    }
}

/// Every distinct mechanic a script touches, as flat strings.
///
/// This is the unit a coverage plan is built out of: `api:Token`,
/// `param:Pump.Duration`, `kw:Equip`, `line:S`. It is deliberately *not*
/// a list of what the transcoder refused — it names what the script
/// *uses*, so that atoms already appearing in scripts the transcoder reads
/// in full can be subtracted as known. That subtraction is what keeps the
/// plan honest without restating each rule's parameter list here, where
/// the copy would rot the first time a rule learned a new key.
#[must_use]
pub fn atoms(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    collect_atoms(script, &mut out);
    out.sort();
    out.dedup();
    out
}
