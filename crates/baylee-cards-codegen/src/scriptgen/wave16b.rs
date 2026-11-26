//! Wave 16b `scriptgen` lowerings:
//! - `A:SP$ PermanentCreature` and `A:SP$ PermanentNoncreature` spell lines
//!   carrying mandatory non-mana additional costs (`CR 118.8`, `CR 601.2b/f`).
//! - `Blight<N>` cost tokens (`CostPart::PutCounterOnOther`, `CR 118.3`,
//!   `CR 122.1`) and `DB$ Blight` / `AB$ Blight` APNAP choice effects
//!   (`Effect::PutCountersOnChosenControlled`, `CR 101.4`).
//! - `Reveal<N/Valid>`, `Reveal<X/Valid>`, and `Reveal<1/CARDNAME>` (`Forecast`,
//!   `CR 702.57a–b`) cost tokens (`CostPart::RevealFromHand`,
//!   `CostPart::RevealFromHandX`, `CostPart::RevealSelfFromHand`), plus
//!   `Revealed$CardPower` / `RevealedCards$CardPower` / `Revealed$CardManaCost`
//!   (`Amount::BeheldOrRevealedPower`, `Amount::BeheldOrRevealedManaValue`,
//!   `CR 608.2h`) and Tarkir `Count$Presence_<Subtype>.1.0`
//!   (`Condition::ControlledSubtypeOnCast`, `CR 601.2h`).
//! - `S: Mode$ OptionalCost` extensions: `Cost$ Waterbend<N>` (`CR 701.67a`),
//!   `SpellCastMetric::OptionalCostPaid` (`ReduceCost` on *Bite Down on Crime*),
//!   and 2-mode `CharmNum$ Count$OptionalGenericCostPaid.2.1` (`entwine` on
//!   *Pyrrhic Strike*, `CR 702.42a`).
//! - Graveyard target & static grants (`TgtZone$ Graveyard | PumpZone$ Graveyard`,
//!   `PumpAll | PumpZone$ Graveyard`, `AffectedZone$ Graveyard`):
//!   `Modifier::GrantsFlashbackCost`, `Modifier::GrantsJumpStart`,
//!   `Modifier::GrantsEncoreManaCost`, `Modifier::GrantsEmbalmManaCost`, and
//!   `Modifier::GrantsEscape` (`CR 702.34a`, `CR 702.128a`, `CR 702.133a`,
//!   `CR 702.138a`, `CR 702.141a`).

use super::{Params, Tx, cost_parts, printed_mana};

impl Tx<'_> {
    /// Whether `raw` cost string contains a `Behold<`, `Reveal<` (other than
    /// `Reveal<1/CARDNAME>`/`NICKNAME`), or `RevealOrChoose<` token.
    pub(super) fn cost_raw_beholds_or_reveals(raw: Option<&str>) -> bool {
        raw.is_some_and(|c| {
            cost_parts(c).iter().any(|t| {
                t.starts_with("Behold<")
                    || t.starts_with("BeholdExile<")
                    || t.starts_with("RevealOrChoose<")
                    || (t.starts_with("Reveal<")
                        && !t.starts_with("Reveal<1/CARDNAME")
                        && !t.starts_with("Reveal<1/NICKNAME"))
            })
        })
    }

    /// Whether this spell's `optional_cost`, `mandatory_additional_costs`, or
    /// active `A:SP$` `Cost$` beholds a permanent or reveals a card from hand.
    pub(super) fn has_behold_or_reveal_spell_cost(&self) -> bool {
        self.cost_beholds_or_reveals
            || self.body.optional_cost.as_deref().is_some_and(|c| {
                c.contains("Behold(")
                    || c.contains("BeholdAndExile(")
                    || c.contains("RevealFromHand(")
            })
            || self.body.mandatory_additional_costs.iter().any(|c| {
                c.contains("Behold(")
                    || c.contains("BeholdAndExile(")
                    || c.contains("RevealFromHand(")
            })
    }

    /// `A:SP$ PermanentCreature` and `A:SP$ PermanentNoncreature`: permanent
    /// spell lines declaring mandatory additional costs (`CR 118.8`, `CR 601.2b`).
    pub(super) fn wave16b_permanent_spell(&mut self, spec: &str) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        if self.type_line.is_empty() || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny(format!("`{api}` on a land or non-permanent card"));
        }
        let is_creature = self.type_line.split_whitespace().any(|w| w == "Creature");
        if api == "PermanentCreature" && !is_creature {
            return self.deny("`PermanentCreature` on a non-creature face".to_string());
        }
        if api == "PermanentNoncreature" && is_creature {
            return self.deny("`PermanentNoncreature` on a creature face".to_string());
        }
        p.drop_prose();
        p.take("AdditionalDesc");
        p.take("PrecostDesc");
        p.take("CostDesc");
        p.take("AILogic");
        let raw_cost = p.take("Cost");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `{api}.{key}`"));
        }
        if let Some(raw) = raw_cost {
            self.spell_mana_cost(&raw)?;
        }
        Some(())
    }

    /// Parses `Blight<N>` (`1..=65535`) into `PutCounterOnOther` placing `N`
    /// `-1/-1` counters on a creature you control (`CR 118.3`, `CR 122.1`).
    pub(super) fn parse_blight_cost_token(&self, token: &str) -> Option<String> {
        let Some(raw_n) = token
            .strip_prefix("Blight<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let Some(n) = raw_n.trim().parse::<u16>().ok().filter(|&n| n > 0) else {
            return self.deny(format!("cost `{token}`"));
        };
        Some(format!(
            "PutCounterOnOther {{ filter: &Filter::YOUR_CREATURE, kind: CounterKind::M1M1, n: {n} }}"
        ))
    }

    /// Parses `Reveal<<count>/<valid>[/<desc>]>` into:
    /// - `RevealSelfFromHand` when `count == "1"`, `valid` is `CARDNAME`/`NICKNAME`,
    ///   and `self.in_hand_activation` holds (`CR 702.57b` Forecast).
    /// - `RevealFromHandX(&COSTn)` when `count == "X"` and `allow_x` holds
    ///   (`CR 107.3a`, `CR 701.20a`; *Martyr* cycle).
    /// - `count` (`1..=3`) copies of `RevealFromHand(&COSTn)` otherwise.
    pub(super) fn parse_reveal_cost_token(
        &mut self,
        token: &str,
        allow_x: bool,
    ) -> Option<Vec<String>> {
        let Some(inner) = token
            .strip_prefix("Reveal<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let mut fields = inner.splitn(3, '/');
        let (Some(count_raw), Some(valid_raw)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        let count_str = count_raw.trim();
        let valid = valid_raw.trim();
        if count_str == "1" && matches!(valid, "CARDNAME" | "NICKNAME") {
            if self.in_forecast_activation {
                return Some(vec!["RevealSelfFromHand".to_string()]);
            }
            return self.deny(format!("cost `{token}` naming itself"));
        }
        if valid.is_empty()
            || valid.split([',', ';']).any(|alt| {
                alt.split(['.', '+']).any(|atom| {
                    let atom = atom.trim();
                    matches!(
                        atom,
                        "CARDNAME"
                            | "NICKNAME"
                            | "Self"
                            | "Other"
                            | "StrictlyOther"
                            | "SameColor"
                            | "DifferentColor"
                            | "SameName"
                            | "DifferentNames"
                            | "ChosenColor"
                            | "ChosenType"
                    ) || atom.contains("Ctrl")
                        || atom.contains("Own")
                })
            })
        {
            return self.deny(format!("cost `{token}`"));
        }
        let normalized = valid.replace(';', ",");
        let expr = self.filter_expr_in_zone(&normalized, true)?;
        let filter_name = self.body.filter_static("COST", &expr);
        if count_str == "X" {
            if !allow_x
                || self.in_alternative_cost
                || self.trigger_mode.is_some()
                || !self.has_x
                || self.svars.get("X").map(String::as_str) != Some("Count$xPaid")
                || self.oracle.to_ascii_lowercase().contains("x is ")
            {
                return self.deny(format!("cost `{token}`"));
            }
            return Some(vec![format!("RevealFromHandX(&{filter_name})")]);
        }
        let Some(count) = count_str
            .parse::<usize>()
            .ok()
            .filter(|&c| (1..=3).contains(&c))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        Some(vec![format!("RevealFromHand(&{filter_name})"); count])
    }

    /// `DB$ Blight` / `AB$ Blight`: each matching player chooses a creature
    /// they control in APNAP order (`CR 101.4`) and puts `count` `-1/-1`
    /// counters on it (`CR 122.1`).
    pub(super) fn wave16b_blight_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let raw_who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let who = if raw_who == "PlayerRel::Opponent" {
            "PlayerRel::EachOpponent"
        } else {
            raw_who
        };
        let raw_num = p
            .take("Num")
            .or_else(|| p.take("Amount"))
            .unwrap_or_else(|| "1".to_string());
        let count = self.amount_or_count_ext(&raw_num)?;
        Some(vec![format!(
            "Effect::PutCountersOnChosenControlled {{ who: {who}, filter: &Filter::CREATURE, \
             kind: CounterKind::M1M1, count: {count} }}"
        )])
    }

    /// Parses `Waterbend<N>` (`1..=65535`) on `S: Mode$ OptionalCost` (`CR 701.67a`).
    pub(super) fn wave16b_try_waterbend_optional_cost(
        &mut self,
        cost_raw: &str,
    ) -> Option<Result<(), ()>> {
        let inner = cost_raw
            .trim()
            .strip_prefix("Waterbend<")?
            .strip_suffix('>')?;
        let Some(n) = inner.trim().parse::<u16>().ok().filter(|&n| n > 0) else {
            self.note(format!("cost `{}`", cost_raw.trim()));
            return Some(Err(()));
        };
        self.body.optional_cost = Some(format!("Some(cost!(\"{{{n}}}\"))"));
        self.body.waterbend = true;
        self.body.notes.push("optional additional cost".to_string());
        Some(Ok(()))
    }

    /// Resolves `RevealedCards$CardPower` / `Revealed$CardPower` /
    /// `Revealed$CardManaCost` on spells whose cost beholds a permanent or
    /// reveals a card from hand (`CR 608.2h`).
    pub(super) fn wave16b_revealed_cost_amount(&self, base_raw: &str) -> Option<String> {
        if !self.on_a_spell
            || self.in_delayed
            || self.trigger_mode.is_some()
            || !self.has_behold_or_reveal_spell_cost()
        {
            return None;
        }
        match base_raw.trim() {
            "RevealedCards$CardPower" | "Revealed$CardPower" => {
                Some("Amount::BeheldOrRevealedPower".to_string())
            }
            "RevealedCards$CardManaCost" | "Revealed$CardManaCost" => {
                Some("Amount::BeheldOrRevealedManaValue".to_string())
            }
            _ => None,
        }
    }

    /// Resolves Tarkir `Count$Presence_<Subtype>.1.0` into
    /// `Condition::Any(&[Condition::OptionalCostPaid, Condition::ControlledSubtypeOnCast(...)])`
    /// (`CR 601.2h`) when `FaceDef::optional_cost` is `RevealFromHand` for that
    /// exact creature subtype.
    pub(super) fn wave16b_presence_condition(&self, def: &str, cmp: &str) -> Option<String> {
        if self.in_delayed || !self.can_check_optional_cost() {
            return None;
        }
        let subtype = def
            .trim()
            .strip_prefix("Count$Presence_")?
            .strip_suffix(".1.0")?;
        let sub_path = self.cats.const_path(subtype)?;
        if !sub_path.starts_with("subtypes::creature::") {
            return None;
        }
        let opt = self.body.optional_cost.as_deref()?;
        let filter_name = opt
            .strip_prefix("Some(cost!(RevealFromHand(&")?
            .strip_suffix(")))")?;
        let expected_static =
            format!("static {filter_name}: Filter = Filter::HasSubtype({sub_path});");
        if !self
            .body
            .statics
            .lines()
            .any(|l| l.trim() == expected_static)
        {
            return None;
        }
        let pos = format!(
            "Condition::Any(&[Condition::OptionalCostPaid, Condition::ControlledSubtypeOnCast({sub_path})])"
        );
        match cmp.trim() {
            "GE1" | "EQ1" | "GT0" | "NE0" => Some(pos),
            "EQ0" | "LE0" | "LT1" | "NE1" => Some(format!("Condition::Not(&{pos})")),
            _ => None,
        }
    }

    /// `R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen | CheckSVar$ ...`
    /// (*Dragonlord's Prerogative*): conditional self-uncounterability on the stack.
    pub(super) fn wave16b_conditional_self_uncounterable(&mut self, mut p: Params) -> Option<()> {
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("ValidSA").as_deref() != Some("Spell")
            || p.take("Layer").as_deref() != Some("CantHappen")
        {
            return None;
        }
        let var = p.take("CheckSVar")?;
        let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
        if !p.exhausted() {
            return None;
        }
        let def = self
            .svars
            .get(var.trim())
            .map_or(var.trim(), String::as_str)
            .trim()
            .to_string();
        let cond = self
            .wave16b_presence_condition(&def, &cmp)
            .or_else(|| self.svar_condition_clause(&var, &cmp))?;
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNCOUNTERABLE), \
             zone = ZoneRef::Stack, condition = Some({cond}))"
        ));
        Some(())
    }

    /// Checks whether a 2-mode `SP$ Charm` uses `CharmNum$ X` with
    /// `SVar:X:Count$OptionalGenericCostPaid.2.1` (*Pyrrhic Strike*, `CR 702.42a`).
    pub(super) fn wave16b_is_optional_cost_entwine_charm(&self, raw: &str) -> bool {
        if self.body.optional_cost.is_none() || self.body.entwine.is_some() {
            return false;
        }
        let def = self
            .svars
            .get(raw.trim())
            .map_or(raw.trim(), String::as_str)
            .trim();
        def == "Count$OptionalGenericCostPaid.2.1"
    }

    /// Claims `AdditionalDescription$` on `SP$ Charm` when it restates
    /// `"If this spell's additional cost was paid, choose both instead."`.
    pub(super) fn wave16b_claim_entwine_charm_desc(p: &mut Params) {
        let Some(raw) = p.peek("AdditionalDescription") else {
            return;
        };
        let norm = raw
            .trim()
            .trim_start_matches('.')
            .trim()
            .trim_end_matches('.')
            .trim()
            .to_ascii_lowercase();
        if norm == "if this spell's additional cost was paid, choose both instead" {
            p.take("AdditionalDescription");
        }
    }

    /// Parses graveyard grant keywords shared by `AffectedZone$ Graveyard`
    /// statics, `DB$ Pump | PumpZone$ Graveyard`, and `DB$ PumpAll | PumpZone$ Graveyard`:
    /// - `"Flashback"` -> `Modifier::GrantsFlashback`
    /// - `"Flashback:<mana>"` -> `Modifier::GrantsFlashbackCost(...)`
    /// - `"Jump-start"` -> `Modifier::GrantsJumpStart`
    /// - `"Encore:CardManaCost"` -> `Modifier::GrantsEncoreManaCost`
    /// - `"Embalm:CardManaCost"` -> `Modifier::GrantsEmbalmManaCost`
    /// - `"Escape:CardManaCost ExileFromGrave<N/Card.Other/other>"` -> `Modifier::GrantsEscape(N)`
    /// - `"Retrace"` (when `allow_retrace`) -> `Modifier::AddKeyword(KeywordSet::RETRACE)`
    pub(super) fn wave16b_graveyard_grant_modifier(
        kw: &str,
        allow_retrace: bool,
    ) -> Option<(String, bool)> {
        let kw = kw.trim();
        if kw == "Flashback" {
            return Some(("Modifier::GrantsFlashback".to_string(), false));
        }
        if let Some(cost_raw) = kw.strip_prefix("Flashback:") {
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
            return Some((format!("Modifier::GrantsFlashbackCost({mana_expr})"), false));
        }
        if kw == "Jump-start" {
            return Some(("Modifier::GrantsJumpStart".to_string(), false));
        }
        if kw == "Encore:CardManaCost" {
            return Some(("Modifier::GrantsEncoreManaCost".to_string(), false));
        }
        if kw == "Embalm:CardManaCost" {
            return Some(("Modifier::GrantsEmbalmManaCost".to_string(), false));
        }
        if allow_retrace && kw == "Retrace" {
            return Some((
                "Modifier::AddKeyword(KeywordSet::RETRACE)".to_string(),
                true,
            ));
        }
        if let Some(rest) = kw.strip_prefix("Escape:CardManaCost ExileFromGrave<")
            && let Some(n_str) = rest
                .strip_suffix("/Card.Other/other>")
                .or_else(|| rest.strip_suffix("/Card.!Self/other>"))
                .or_else(|| rest.strip_suffix("/Card.Other>"))
            && let Ok(n) = n_str.trim().parse::<u8>()
            && n > 0
        {
            return Some((format!("Modifier::GrantsEscape({n})"), false));
        }
        Self::wave17c_graveyard_grant_modifier(kw, allow_retrace)
    }

    /// Single-target `DB$ Pump | PumpZone$ Graveyard | KW$ ...` ->
    /// `Effect::GrantFlashback` (preserving canonical bare `Flashback` output)
    /// or `Effect::ModifyTargets { modifier, duration: Duration::UntilEndOfTurn }`.
    pub(super) fn wave16b_pump_graveyard_grant(
        p: &mut Params,
        target: &str,
    ) -> Option<Vec<String>> {
        if !target.starts_with("TargetSpec::CardInGraveyard(") {
            return None;
        }
        let kw_peek = p.peek("KW")?;
        let (modifier, _) = Self::wave16b_graveyard_grant_modifier(kw_peek, false)?;
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
            || !matches!(
                p.take("Defined").as_deref(),
                None | Some("Targeted" | "ParentTarget")
            )
        {
            return None;
        }
        p.take("PumpZone");
        p.take("KW");
        p.take("IsCurse");
        if modifier == "Modifier::GrantsFlashback" {
            Some(vec!["Effect::GrantFlashback".to_string()])
        } else {
            Some(vec![format!(
                "Effect::ModifyTargets {{ modifier: {modifier}, duration: Duration::UntilEndOfTurn }}"
            )])
        }
    }

    /// `DB$ PumpAll | PumpZone$ Graveyard | KW$ ...` ->
    /// `Effect::continuous(&FILTER, Modifier::..., Duration::UntilEndOfTurn)`.
    pub(super) fn wave16b_pump_all_graveyard_grant(
        &mut self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        let kw_peek = p.peek("KW")?;
        let (modifier, _) = Self::wave16b_graveyard_grant_modifier(kw_peek, false)?;
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        p.take("PumpZone");
        p.take("KW");
        p.take("IsCurse");
        let valid = p.take("ValidCards")?;
        let mut rewritten = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() || alt.contains("OppCtrl") || alt.contains("OppOwn") {
                return None;
            }
            if !alt.contains("YouCtrl") && !alt.contains("YouOwn") {
                return None;
            }
            rewritten.push(alt.replace("YouCtrl", "YouOwn"));
        }
        let base = self.filter_expr_in_zone(&rewritten.join(","), true)?;
        let expr = format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::Graveyard)])");
        let prefix = if modifier == "Modifier::GrantsFlashback" {
            "FLASHBACK"
        } else {
            "GRAVE_GRANT"
        };
        let filter = self.body.filter_static(prefix, &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, {modifier}, Duration::UntilEndOfTurn)"
        )])
    }
}
