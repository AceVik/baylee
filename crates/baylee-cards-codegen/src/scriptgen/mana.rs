//! Reading mana, amounts and costs.

use super::{
    Params, Tx, cost_parts, counter_kind, hybrid_pair, keyword_atom, mana_color_const, named_atom,
    object_cost, plain_number, stat_atom,
};

type SpendRestriction = (String, Option<&'static str>, Option<String>);

enum ManaSpendRider {
    None,
    RestrictUncounterable,
    WhenSpent { filter: String, rider: String },
}

impl ManaSpendRider {
    fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl Tx<'_> {
    /// `Produced$ Combo W U | Amount$ 2` and friends.
    pub(super) fn mana_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        // "Its controller adds an additional {R}" (Gauntlet of Might): mana
        // in another player's pool.
        let defined = p.take("Defined");
        if let Some(who) = defined.as_deref().filter(|d| *d != "You") {
            let who = self.player_rel(Some(who))?;
            return self.mana_for(p, who);
        }
        let Some(produced) = p.take("Produced") else {
            return self.deny("`Mana` with no `Produced$`".to_string());
        };
        let persistent = match p.take("PersistentMana").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Mana.PersistentMana$ {other}`")),
        };
        let restrict = match p.take("RestrictValid") {
            None => None,
            Some(valid) => Some(self.spend_restriction(&valid)?),
        };
        let spend_rider = self.take_mana_spend_rider(p, restrict.is_some())?;
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        let fixed = plain_number(&raw, self.svars).and_then(|n| u32::try_from(n).ok());
        let has_modifier = restrict.is_some() || spend_rider.is_some();
        let effects = if let Some(valid) = produced.strip_prefix("Special EachColorAmong_Valid ") {
            if fixed != Some(1) || has_modifier || persistent {
                return self.deny("mana source `Special`".to_string());
            }
            let expr = self.filter_expr(valid.trim())?;
            let name = self.body.filter_static("MANA", &expr);
            return Some(vec![format!("Effect::ManaForColorsAmong(&{name})")]);
        } else {
            self.produced_mana_effects(&produced, &raw, fixed, has_modifier)?
        };
        self.apply_mana_modifiers(effects, restrict, spend_rider, persistent)
    }

    fn produced_mana_effects(
        &mut self,
        produced: &str,
        raw: &str,
        fixed: Option<u32>,
        fold_identical: bool,
    ) -> Option<Vec<String>> {
        let color = |c: &str| {
            Some(match c {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                _ => return None,
            })
        };
        if produced == "Any" {
            return Some(match fixed {
                Some(1) => vec!["Effect::mana_of_any_color()".to_string()],
                _ => vec![format!(
                    "Effect::mana_choice_dynamic(ALL_MANA_COLORS, {})",
                    self.counted_amount(raw)?
                )],
            });
        }
        if produced == "Chosen" || produced == "ChosenColor" {
            return Some(if fixed == Some(1) {
                vec!["Effect::mana_chosen()".to_string()]
            } else {
                let amount = if self.svars.get(raw.trim()).map(String::as_str)
                    == Some("Count$Devotion.Chosen")
                {
                    "Amount::DevotionToManaColor".to_string()
                } else {
                    self.counted_amount(raw)?
                };
                vec![format!(
                    "Effect::AddMana {{ source: ManaSource::Chosen, amount: {amount}, distribution: ManaDistribution::OneColor, restriction: None }}"
                )]
            });
        }
        if let Some(list) = produced.strip_prefix("Combo ") {
            return self.combo_mana(list, raw, fixed);
        }
        let Some(colors) = produced
            .split_whitespace()
            .map(color)
            .collect::<Option<Vec<_>>>()
        else {
            let head = produced.split_whitespace().next().unwrap_or(produced);
            return self.deny(format!("mana source `{head}`"));
        };
        if let Some(n) = fixed {
            if fold_identical && colors.len() > 1 && colors.iter().all(|c| *c == colors[0]) {
                let total = n.saturating_mul(u32::try_from(colors.len()).unwrap_or(1));
                return Some(vec![format!("Effect::mana({}, {total})", colors[0])]);
            }
            return Some(
                colors
                    .into_iter()
                    .map(|c| format!("Effect::mana({c}, {n})"))
                    .collect(),
            );
        }
        let [only] = &colors[..] else {
            return self.deny("a counted amount of more than one colour".to_string());
        };
        let amount = self
            .mono_devotion_mana_amount(only, raw)
            .map(str::to_string)
            .or_else(|| self.counted_amount(raw))?;
        Some(vec![format!("Effect::mana_dynamic({only}, {amount})")])
    }

    fn apply_mana_modifiers(
        &mut self,
        effects: Vec<String>,
        restrict: Option<SpendRestriction>,
        spend_rider: ManaSpendRider,
        persistent: bool,
    ) -> Option<Vec<String>> {
        let mut out = if let Some((filter, uses, ability_filter)) = restrict {
            if effects.is_empty() {
                return self.deny("`Mana.RestrictValid` with no mana produced".to_string());
            }
            let rider = match spend_rider {
                ManaSpendRider::RestrictUncounterable => "SpendRider::Uncounterable",
                ManaSpendRider::None => "SpendRider::None",
                ManaSpendRider::WhenSpent { .. } => {
                    return self.deny("a mana spend rider beside `RestrictValid$`".to_string());
                }
            };
            let name = self.body.filter_static("SPEND", &filter);
            let ab_ref = ability_filter.map(|ab_filter| {
                if ab_filter == "Filter::Any" {
                    "Filter::Any".to_string()
                } else {
                    self.body.filter_static("SPEND_ABILITY", &ab_filter)
                }
            });
            effects
                .into_iter()
                .map(|only| {
                    let mut expr = match uses {
                        None => format!("{only}.restricted(&{name}, {rider})"),
                        Some(uses) => format!("{only}.restricted_for(&{name}, {rider}, {uses})"),
                    };
                    if let Some(ref ab) = ab_ref {
                        expr = format!("{expr}.for_abilities_matching(&{ab})");
                    }
                    expr
                })
                .collect()
        } else if let ManaSpendRider::WhenSpent { filter, rider } = spend_rider {
            if effects.len() != 1 {
                return self.deny("a mana spend rider on multi-effect mana".to_string());
            }
            let only = effects.into_iter().next().expect("len == 1");
            let name = if filter == "Filter::Any" {
                "Filter::Any".to_string()
            } else {
                self.body.filter_static("SPEND", &filter)
            };
            vec![format!("{only}.when_spent(&{name}, {rider})")]
        } else {
            effects
        };
        if persistent {
            for effect in &mut out {
                effect.push_str(".mana_until_end_of_turn()");
            }
        }
        Some(out)
    }

    fn take_mana_spend_rider(&self, p: &mut Params, has_restrict: bool) -> Option<ManaSpendRider> {
        let adds_no_counter = p.take("AddsNoCounter");
        let adds_keywords = p.take("AddsKeywords");
        let adds_counters = p.take("AddsCounters");
        let triggers_when_spent = p.take("TriggersWhenSpent");
        let count = usize::from(adds_no_counter.is_some())
            + usize::from(adds_keywords.is_some())
            + usize::from(adds_counters.is_some())
            + usize::from(triggers_when_spent.is_some());
        if count > 1 {
            return self.deny("multiple mana spend riders on one ability".to_string());
        }
        if let Some(raw) = adds_no_counter {
            if raw != "True" || !has_restrict {
                return self.deny(format!("`Mana.AddsNoCounter$ {raw}`"));
            }
            return Some(ManaSpendRider::RestrictUncounterable);
        }
        if let Some(kw) = adds_keywords {
            return self.parse_adds_keywords_rider(p, &kw, has_restrict);
        }
        if let Some(raw) = adds_counters {
            return self.parse_adds_counters_rider(&raw, has_restrict);
        }
        if let Some(trig) = triggers_when_spent {
            return self.parse_triggers_when_spent(&trig, has_restrict);
        }
        Some(ManaSpendRider::None)
    }

    fn parse_adds_keywords_rider(
        &self,
        p: &mut Params,
        kw: &str,
        has_restrict: bool,
    ) -> Option<ManaSpendRider> {
        if has_restrict || kw != "Haste" {
            return self.deny(format!("`Mana.AddsKeywords$ {kw}`"));
        }
        let Some(valid) = p.take("AddsKeywordsValid") else {
            return self.deny("`Mana.AddsKeywords$` without `AddsKeywordsValid$`".to_string());
        };
        let (filter, uses, ab_filter) = self.spend_restriction(&valid)?;
        if uses.is_some() || ab_filter.is_some() {
            return self.deny(format!("`Mana.AddsKeywordsValid$ {valid}`"));
        }
        let rider = match p.take("AddsKeywordsUntil").as_deref() {
            None => "SpendRider::Haste".to_string(),
            Some("UntilEOT") => "SpendRider::HasteUntilEndOfTurn".to_string(),
            Some(other) => return self.deny(format!("`Mana.AddsKeywordsUntil$ {other}`")),
        };
        Some(ManaSpendRider::WhenSpent { filter, rider })
    }

    fn parse_adds_counters_rider(&self, raw: &str, has_restrict: bool) -> Option<ManaSpendRider> {
        if has_restrict {
            return self.deny("`Mana.AddsCounters$` beside `RestrictValid$`".to_string());
        }
        let parts: Vec<&str> = raw.split('_').collect();
        let [valid_raw, "P1P1", count_raw] = parts.as_slice() else {
            return self.deny(format!("`Mana.AddsCounters$ {raw}`"));
        };
        if matches!(
            *valid_raw,
            "Card.YouOwn+IsCommander" | "Card.IsCommander+YouOwn"
        ) && *count_raw == "ManaAddsCounterNum"
        {
            return Some(ManaSpendRider::WhenSpent {
                filter: "Filter::And(&[Filter::IsCommander, Filter::OwnedByYou])".to_string(),
                rider: "SpendRider::CommanderEnterCounters".to_string(),
            });
        }
        let Some(n) = count_raw.parse::<u32>().ok().filter(|&n| n > 0) else {
            return self.deny(format!("`Mana.AddsCounters$ {raw}`"));
        };
        if has_unscoped_spend_atom(valid_raw) {
            return self.deny(format!("`Mana.AddsCounters$ {raw}`"));
        }
        let Some(filter) = self.filter_expr_in_zone(valid_raw, true) else {
            return self.deny(format!("`Mana.AddsCounters$ {raw}`"));
        };
        Some(ManaSpendRider::WhenSpent {
            filter,
            rider: format!("SpendRider::EnterCounters({n})"),
        })
    }

    fn parse_triggers_when_spent(
        &self,
        trig_name: &str,
        has_restrict: bool,
    ) -> Option<ManaSpendRider> {
        let deny = || self.deny(format!("`Mana.TriggersWhenSpent$ {trig_name}`"));
        if has_restrict {
            return deny();
        }
        let Some(trig_body) = self.svars.get(trig_name.trim()) else {
            return deny();
        };
        let Some((mode, mut tp)) = Params::parse(trig_body) else {
            return deny();
        };
        tp.drop_prose();
        if mode != "SpellCast"
            || !matches!(
                tp.take("ValidActivatingPlayer").as_deref(),
                None | Some("You")
            )
        {
            return deny();
        }
        let Some(exec_name) = tp.take("Execute") else {
            return deny();
        };
        let mut is_your_commander = false;
        let filter = match tp.take("ValidCard").as_deref() {
            None => "Filter::Any".to_string(),
            Some("Card.IsCommander+YouOwn" | "Card.YouOwn+IsCommander") => {
                is_your_commander = true;
                "Filter::And(&[Filter::IsCommander, Filter::OwnedByYou])".to_string()
            }
            Some("Creature.sharesCreatureTypeWith Commander") => {
                "Filter::And(&[Filter::CREATURE, Filter::SharesSubtypeWithCommander])".to_string()
            }
            Some(valid) if !has_unscoped_spend_atom(valid) => {
                let Some(expr) = self.filter_expr_in_zone(valid, true) else {
                    return deny();
                };
                expr
            }
            Some(_) => return deny(),
        };
        if tp.first_key().is_some() {
            return deny();
        }
        let Some(exec_body) = self.svars.get(exec_name.trim()) else {
            return deny();
        };
        let Some((exec_api, mut ep)) = Params::parse(exec_body) else {
            return deny();
        };
        ep.drop_prose();
        let rider = match exec_api.as_str() {
            "Scry" => {
                let Some(num) = ep.take("ScryNum") else {
                    return deny();
                };
                if ep.first_key().is_some() {
                    return deny();
                }
                if let Ok(n) = num.trim().parse::<u8>()
                    && n > 0
                {
                    format!("SpendRider::Scry({n})")
                } else if is_your_commander
                    && self.svars.get(num.trim()).map(String::as_str)
                        == Some("TriggeredCard$CommanderCastFromCommandZone")
                {
                    "SpendRider::CommanderScry".to_string()
                } else {
                    return deny();
                }
            }
            "CopySpellAbility" => {
                if ep.take("Defined").as_deref() != Some("TriggeredSpellAbility")
                    || ep.take("MayChooseTarget").as_deref() != Some("True")
                    || ep.first_key().is_some()
                {
                    return deny();
                }
                "SpendRider::Copy".to_string()
            }
            _ => return deny(),
        };
        Some(ManaSpendRider::WhenSpent { filter, rider })
    }

    fn mono_devotion_mana_amount(&self, color_const: &str, raw: &str) -> Option<&'static str> {
        let def = self.svars.get(raw.trim())?.trim();
        let expected = match color_const {
            "ManaColor::White" => "Count$Devotion.White",
            "ManaColor::Blue" => "Count$Devotion.Blue",
            "ManaColor::Black" => "Count$Devotion.Black",
            "ManaColor::Red" => "Count$Devotion.Red",
            "ManaColor::Green" => "Count$Devotion.Green",
            _ => return None,
        };
        (def == expected).then_some("Amount::DevotionToManaColor")
    }

    /// [`Self::mana_effect`] for a pool other than the controller's:
    /// `Effect::AddManaFor` for fixed amounts, or `Effect::AddManaAmountFor`
    /// for a dynamic single-color amount.
    pub(super) fn mana_for(&mut self, p: &mut Params, who: &str) -> Option<Vec<String>> {
        let Some(produced) = p.take("Produced") else {
            return self.deny("`Mana` for another player with no `Produced$`".to_string());
        };
        if p.peek("RestrictValid").is_some() {
            return self.deny("restricted mana for another player".to_string());
        }
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        let color_of = |symbol: &str| match symbol {
            "W" => Some("ManaColor::White"),
            "U" => Some("ManaColor::Blue"),
            "B" => Some("ManaColor::Black"),
            "R" => Some("ManaColor::Red"),
            "G" => Some("ManaColor::Green"),
            "C" => Some("ManaColor::Colorless"),
            _ => None,
        };
        if let Some(amount) = plain_number(&raw, self.svars).and_then(|n| u16::try_from(n).ok()) {
            let mut out = Vec::new();
            for symbol in produced.split_whitespace() {
                let Some(color) = color_of(symbol) else {
                    return self.deny(format!("mana for another player of `{symbol}`"));
                };
                out.push(format!(
                    "Effect::AddManaFor {{ who: {who}, color: {color}, amount: {amount} }}"
                ));
            }
            return Some(out);
        }
        let symbols: Vec<&str> = produced.split_whitespace().collect();
        let [only] = symbols.as_slice() else {
            return self.deny(format!("mana for another player of `Amount$ {raw}`"));
        };
        let Some(color) = color_of(only) else {
            return self.deny(format!("mana for another player of `{only}`"));
        };
        let saved = self.unclaimed.borrow().clone();
        let Some(amount) = self.counted_amount(&raw) else {
            *self.unclaimed.borrow_mut() = saved;
            return self.deny(format!("mana for another player of `Amount$ {raw}`"));
        };
        Some(vec![format!(
            "Effect::AddManaAmountFor {{ who: {who}, color: {color}, amount: {amount} }}"
        )])
    }

    /// `A:AB$ ManaReflected`: delegates `ReflectProperty$ Produce` to
    /// [`Self::gond_mana_reflected`] (plus two clean extensions) and lowers
    /// `ReflectProperty$ Is` (`ColorsAmong` / `ExiledWithSource`).
    pub(super) fn activated_mana_reflected(&mut self, mut p: Params) -> Option<()> {
        if p.peek("ReflectProperty") == Some("Is") {
            return self.mana_reflected_is(p);
        }
        if p.peek("ReflectProperty") == Some("Produce")
            && matches!(
                p.peek("Valid"),
                Some("Gate.YouCtrl" | "Basic.YouCtrl" | "Land.YouCtrl" | "Land.OppCtrl")
            )
            && !(p.peek("Cost") == Some("T")
                && matches!(p.peek("Valid"), Some("Land.YouCtrl" | "Land.OppCtrl")))
            && !(p.peek("Valid") == Some("Gate.YouCtrl") && p.peek("Cost") != Some("T"))
        {
            p.drop_prose();
            let cost_raw = p.take("Cost")?;
            p.take("ReflectProperty");
            let any_type = match p.take("ColorOrType").as_deref() {
                Some("Color") => false,
                Some("Type") => true,
                _ => {
                    return self
                        .deny("reflected mana outside the exact your-Gate colours grammar".into());
                }
            };
            let effect = match p.take("Valid").as_deref() {
                Some("Gate.YouCtrl") => format!(
                    "Effect::mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, {any_type})"
                ),
                Some("Basic.YouCtrl") => {
                    format!("Effect::mana_from_lands(&Filter::BASIC_LAND, true, {any_type})")
                }
                Some("Land.YouCtrl") => {
                    format!("Effect::mana_from_lands(&Filter::ControlledByYou, true, {any_type})")
                }
                Some("Land.OppCtrl") => format!(
                    "Effect::mana_from_lands(&Filter::ControlledByOpponent, false, {any_type})"
                ),
                _ => unreachable!(),
            };
            if let Some(amount) = p.take("Amount")
                && amount != "1"
            {
                return self.deny("reflected Gate amount other than one".into());
            }
            let persistent = match p.take("PersistentMana").as_deref() {
                None => false,
                Some("True") => true,
                Some(other) => {
                    return self.deny(format!("`ManaReflected.PersistentMana$ {other}`"));
                }
            };
            let restrict = match p.take("RestrictValid") {
                None => None,
                Some(valid) => Some(self.spend_restriction(&valid)?),
            };
            if !p.exhausted() {
                return self.deny("unclaimed reflected Gate mana parameter".into());
            }
            let effects = self.apply_mana_modifiers(
                vec![effect],
                restrict,
                ManaSpendRider::None,
                persistent,
            )?;
            let cost = self.parse_activated_cost(&cost_raw, None)?;
            let ability = if cost == "Cost::TAP" {
                format!("mana_ability!(&[{}])", effects.join(", "))
            } else {
                format!("mana_ability!({cost}, &[{}])", effects.join(", "))
            };
            self.body.abilities.push(ability);
            return Some(());
        }
        self.gond_mana_reflected(p)
    }

    fn mana_reflected_is(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ReflectProperty");
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("`ManaReflected` with no `Cost$`".to_string());
        };
        if p.take("ColorOrType").as_deref() != Some("Color") {
            return self.deny("`ManaReflected` with non-`Color` `ReflectProperty$ Is`".to_string());
        }
        if let Some(amount) = p.take("Amount")
            && amount != "1"
        {
            return self.deny(format!("`ManaReflected` with `Amount$ {amount}`"));
        }
        let persistent = match p.take("PersistentMana").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ManaReflected.PersistentMana$ {other}`")),
        };
        let restrict = match p.take("RestrictValid") {
            None => None,
            Some(valid) => Some(self.spend_restriction(&valid)?),
        };
        let Some(valid) = p.take("Valid") else {
            return self.deny("`ManaReflected` with no `Valid$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ManaReflected.{key}`"));
        }
        let base_effect = match valid.as_str() {
            "Defined.ExiledWith" | "Defined.Imprinted" => "Effect::mana_exiled_colors()".to_string(),
            "Defined.Self" => "Effect::AddMana { source: ManaSource::ColorsAmong { filter: &Filter::This, zone: ZoneSel::Battlefield }, amount: Amount::Fixed(1), distribution: ManaDistribution::OneColor, restriction: None }".to_string(),
            other if other.starts_with("Defined.ValidGraveyard ") => {
                let raw = other.trim_start_matches("Defined.ValidGraveyard ").trim();
                let Some(cleaned) = strip_player_valid_scope(raw, "YouOwn") else {
                    return self.deny(format!("`ManaReflected.Valid$ {valid}`"));
                };
                let expr = self.filter_expr_in_zone(&cleaned, true)?;
                let filter_ref = if expr == "Filter::Any" {
                    "Filter::Any".to_string()
                } else {
                    self.body.filter_static("MANA", &expr)
                };
                format!(
                    "Effect::AddMana {{ source: ManaSource::ColorsAmong {{ filter: &{filter_ref}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}, amount: Amount::Fixed(1), distribution: ManaDistribution::OneColor, restriction: None }}"
                )
            }
            other if other.starts_with("Defined.") => {
                return self.deny(format!("`ManaReflected.Valid$ {valid}`"));
            }
            other => {
                let expr = self.filter_expr(other)?;
                let filter_ref = self.body.filter_static("MANA", &expr);
                format!(
                    "Effect::AddMana {{ source: ManaSource::ColorsAmong {{ filter: &{filter_ref}, zone: ZoneSel::Battlefield }}, amount: Amount::Fixed(1), distribution: ManaDistribution::OneColor, restriction: None }}"
                )
            }
        };
        let effects = self.apply_mana_modifiers(
            vec![base_effect],
            restrict,
            ManaSpendRider::None,
            persistent,
        )?;
        let cost = self.parse_activated_cost(&cost_raw, None)?;
        let ability = if cost == "Cost::TAP" {
            format!("mana_ability!(&[{}])", effects.join(", "))
        } else {
            format!("mana_ability!({cost}, &[{}])", effects.join(", "))
        };
        self.body.abilities.push(ability);
        Some(())
    }

    /// `Produced$ Combo …` — a choice among the colours listed, and how many.
    ///
    /// `Combo` is the corpus's word for "one of these", and the amount is
    /// what decides which of two different sentences it is: one mana picked
    /// from a list, or *n* mana each picked from it. "Add {G}{G}, {G}{W}, or
    /// {W}{W}" is the second — the filter cycle's whole point — and
    /// `combination: true` is where it lives (CR 608.2d: the choices are made
    /// as the effect is applied, and nothing in the rules numbers them).
    pub(super) fn combo_mana(
        &mut self,
        list: &str,
        raw: &str,
        fixed: Option<u32>,
    ) -> Option<Vec<String>> {
        // `ColorIdentity` is a source of its own rather than a colour list:
        // what a Command Tower makes is read off its controller's commanders
        // (CR 903.4), so no card can name the colours.
        if list.trim() == "ColorIdentity" {
            return Some(if fixed == Some(1) {
                vec!["Effect::mana_commander_identity()".to_string()]
            } else {
                vec![format!(
                    "Effect::AddMana {{ source: ManaSource::CommanderIdentity, amount: {}, distribution: ManaDistribution::AnyCombination, restriction: None }}",
                    self.counted_amount(raw)?
                )]
            });
        }
        // The chosen colour is one of the options rather than a colour of
        // its own: "Add {W} or one mana of the chosen color."
        let (chosen, rest): (Vec<&str>, Vec<&str>) = list
            .split_whitespace()
            .partition(|w| *w == "Chosen" || *w == "ChosenColor");
        // `Combo AnyDifferent`: fixed 2–5 distinct colors of the five colors
        // ("Add two mana of different colors", Interplanar Beacon et al.).
        if rest == ["AnyDifferent"] {
            if !chosen.is_empty() {
                return self.deny("`Mana` combination over `AnyDifferent`".to_string());
            }
            return match fixed {
                Some(n) if (2..=5).contains(&n) => Some(vec![format!(
                    "Effect::mana_different_colors(ALL_MANA_COLORS, Amount::Fixed({n}))"
                )]),
                _ => self.deny("`Mana` combination over `AnyDifferent`".to_string()),
            };
        }
        // `Combo Any` is the five colours written as one word, and here it is
        // a *list* rather than a source — which is what makes "add four mana
        // in any combination of colors" the same rule as the filter lands'
        // two.
        let colors: Vec<String> = if rest == ["Any"] {
            Vec::new()
        } else {
            let mut out = Vec::new();
            for word in rest {
                let Some(color) = mana_color_const(word) else {
                    return self.deny(format!("`Mana` combination over `{word}`"));
                };
                out.push(color.to_string());
            }
            out
        };
        let listed = if colors.is_empty() {
            "ALL_MANA_COLORS".to_string()
        } else {
            format!("&[{}]", colors.join(", "))
        };
        if !chosen.is_empty() {
            return if fixed == Some(1) {
                Some(vec![format!("Effect::mana_chosen_or({listed})")])
            } else {
                self.deny(format!("`Mana` chosen-or combination with `Amount$ {raw}`"))
            };
        }
        Some(match fixed {
            Some(1) if colors.is_empty() => vec!["Effect::mana_of_any_color()".to_string()],
            Some(1) => vec![format!("Effect::mana_choice({listed})")],
            // `combination: true`: one pick per mana, and the answers may
            // differ. One pick for the whole amount is `Produced$ Any`, a
            // sentence away and a different card.
            _ => vec![format!(
                "Effect::mana_combination({listed}, {})",
                self.counted_amount(raw)?
            )],
        })
    }

    /// An `Amount$` value as an `Amount`, counts included.
    ///
    /// [`amount`] reads what a number can be without a board: a literal, an
    /// `SVar` that resolves to one, and the `X` a player announced. A mana
    /// line is where the corpus's *counted* amounts are commonest — "Add {B}
    /// for each Swamp you control" — and the DSL has been able to say that
    /// since `Amount::CountOf`; Gaea's Cradle is written with it by hand.
    /// Nothing read it.
    ///
    /// Two differences from [`Self::filter_expr`]'s other counting caller are
    /// deliberate and pull the opposite way from
    /// `a_clause_that_counts_needs_the_filter_to_say_whose`. That rule refuses
    /// a filter naming no controller because `Condition::ControlCount` means
    /// "you control" and would silently narrow it; `CountOf` over
    /// `ZoneSel::Battlefield` counts everything, which is what Cloudpost
    /// prints ("each Locus on the battlefield"). And it refuses `Other`
    /// because a condition has no room for the card asking; `eval::amount`
    /// hands `matches` the source object, so Baldur's Gate's "each **other**
    /// Gate you control" is `Filter::Another` and says what it means.
    pub(super) fn counted_amount(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = self.amount_num(raw) {
            return Some(n);
        }
        let Some(def) = self.svars.get(raw.trim()).cloned() else {
            return self.deny(format!("mana amount `{}`", raw.trim()));
        };
        match self.count_expr(&def) {
            Some(expr) => Some(expr),
            // Named by what it resolves *through* and never by its own
            // spelling: `Amount$ X` is one letter standing for thirty
            // different questions, and the definition is the one of them this
            // card is asking.
            None => self.deny(format!("count `{}`", def.trim())),
        }
    }

    /// A `Count$Valid …` definition as an [`Amount::CountOf`] expression.
    ///
    /// Silent on a definition it cannot read, because its two callers refuse
    /// in different words and a refusal is a worklist entry.
    /// [`Self::counted_amount`] is reading a mana line and [`Self::pump_side`]
    /// a pump; a pump filed under "mana amount" sends whoever reads the
    /// report to a rule the card never touched, and a wrong reason travels
    /// further than a wrong reading because nothing downstream can check it.
    ///
    /// The definition is passed in rather than the `SVar` name so that the
    /// caller decides what an *undefined* name is called as well.
    pub(super) fn count_expr(&mut self, def: &str) -> Option<String> {
        self.count_expr_depth(def, 0, false)
    }

    pub(super) fn opponents_attacked_amount(&self) -> &'static str {
        // The reference property is shared by two different counts. Read
        // the printed scope before using it for a live combat amount.
        let oracle = self.oracle.to_ascii_lowercase();
        if oracle.contains("for each opponent you're attacking")
            || oracle.contains("for each opponent you’re attacking")
            || oracle.contains("for each player being attacked")
        {
            "Amount::OpponentsYouAreAttacking"
        } else {
            "Amount::OpponentsYouAttackedThisCombat"
        }
    }

    /// Static-safe subset of [`Self::count_expr`]: only amounts that evaluate
    /// without a stack resolution (safe inside static abilities and
    /// `Condition::AmountAtLeast`).
    pub(super) fn static_count_expr(&mut self, def: &str) -> Option<String> {
        self.count_expr_depth(def, 0, true)
    }

    #[allow(clippy::too_many_lines)]
    fn count_expr_depth(&mut self, def: &str, depth: u8, static_only: bool) -> Option<String> {
        let def = def.trim();
        if let Some(n_str) = def.strip_prefix("Number$")
            && let Ok(n) = n_str.trim().parse::<u32>()
        {
            return Some(format!("Amount::Fixed({n})"));
        }
        if depth < 4
            && let Some(var) = def.strip_prefix("SVar$")
            && !var.contains('/')
            && let Some(inner) = self.svars.get(var.trim()).cloned()
        {
            return self.count_expr_depth(&inner, depth + 1, static_only);
        }
        let can_read_x =
            self.has_x || (self.is_etb_self_trigger && self.has_x_mana_cost && !self.in_delayed);
        if !static_only && def == "Count$xPaid" && can_read_x {
            return Some("Amount::X".to_string());
        }
        if !static_only && matches!(def, "Count$xPaid/Times.2" | "Count$xPaid/Twice") && can_read_x
        {
            return Some("Amount::DoubleX".to_string());
        }
        if depth < 4
            && let Some((lhs, rhs)) = def.rsplit_once('/')
        {
            let lhs = lhs.trim();
            if rhs.trim() == "Twice" {
                let base = if let Some(var) = lhs.strip_prefix("SVar$") {
                    let inner = self.svars.get(var.trim())?.clone();
                    self.count_expr_depth(&inner, depth + 1, static_only)?
                } else if !static_only && lhs == "Count$xPaid" && can_read_x {
                    return Some("Amount::DoubleX".to_string());
                } else {
                    self.count_expr_depth(lhs, depth + 1, static_only)?
                };
                return Some(if base == "Amount::X" {
                    "Amount::DoubleX".to_string()
                } else {
                    format!("Amount::Times {{ base: &{base}, factor: 2 }}")
                });
            }
            if let Some(sub_var) = rhs.strip_prefix("Minus.")
                && let Some(const_n) = lhs
                    .strip_prefix("Number$")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                && let Some(inner) = self.svars.get(sub_var.trim()).cloned()
            {
                let sub = self.count_expr_depth(&inner, depth + 1, static_only)?;
                return Some(format!(
                    "Amount::ConstantMinus {{ constant: {const_n}, subtract: &{sub} }}"
                ));
            }
            if let Some(rhs_var) = rhs.strip_prefix("Plus.")
                && rhs_var.trim().parse::<u32>().is_err()
                && let Some(inner_rhs) = self.svars.get(rhs_var.trim()).cloned()
            {
                let left = if let Some(var) = lhs.strip_prefix("SVar$") {
                    let inner = self.svars.get(var.trim())?.clone();
                    self.count_expr_depth(&inner, depth + 1, static_only)?
                } else if !static_only && lhs == "Count$xPaid" && can_read_x {
                    "Amount::X".to_string()
                } else {
                    self.count_expr_depth(lhs, depth + 1, static_only)?
                };
                let right = self.count_expr_depth(&inner_rhs, depth + 1, static_only)?;
                return Some(format!("Amount::Sum {{ left: &{left}, right: &{right} }}"));
            }
            if let Some(rhs_var) = rhs.strip_prefix("Minus.")
                && rhs_var.trim().parse::<u32>().is_err()
                && let Some(inner_rhs) = self.svars.get(rhs_var.trim()).cloned()
            {
                let left = if let Some(var) = lhs.strip_prefix("SVar$") {
                    let inner = self.svars.get(var.trim())?.clone();
                    self.count_expr_depth(&inner, depth + 1, static_only)?
                } else if !static_only && lhs == "Count$xPaid" && can_read_x {
                    "Amount::X".to_string()
                } else {
                    self.count_expr_depth(lhs, depth + 1, static_only)?
                };
                let right = self.count_expr_depth(&inner_rhs, depth + 1, static_only)?;
                return Some(format!(
                    "Amount::Difference {{ left: &{left}, right: &{right} }}"
                ));
            }
            let op = if let Some(n) = rhs.strip_prefix("Times.") {
                n.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|&v| v >= 1)
                    .map(|v| (0u8, v))
            } else if let Some(n) = rhs.strip_prefix("Plus.") {
                n.trim().parse::<u32>().ok().map(|v| (1u8, v))
            } else if let Some(n) = rhs.strip_prefix("Minus.") {
                n.trim().parse::<u32>().ok().map(|v| (2u8, v))
            } else if rhs == "Thrice" {
                Some((0u8, 3))
            } else if rhs == "HalfDown" {
                Some((3u8, 0))
            } else if rhs == "HalfUp" {
                Some((4u8, 0))
            } else if let Some(n) = rhs
                .strip_prefix("Div.")
                .or_else(|| rhs.strip_prefix("DivideEvenlyDown."))
            {
                n.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|&v| v >= 2)
                    .map(|v| (5u8, v))
            } else if let Some(n) = rhs.strip_prefix("DivideEvenlyUp.") {
                n.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|&v| v >= 2)
                    .map(|v| (6u8, v))
            } else {
                None
            };
            if let Some((kind, arg)) = op {
                let base = if let Some(var) = lhs.strip_prefix("SVar$") {
                    let inner = self.svars.get(var.trim())?.clone();
                    self.count_expr_depth(&inner, depth + 1, static_only)?
                } else if !static_only && lhs == "Count$xPaid" && can_read_x {
                    "Amount::X".to_string()
                } else {
                    self.count_expr_depth(lhs, depth + 1, static_only)?
                };
                return Some(match kind {
                    0 => format!("Amount::Times {{ base: &{base}, factor: {arg} }}"),
                    1 => format!("Amount::Plus {{ base: &{base}, offset: {arg} }}"),
                    2 => format!("Amount::SaturatingSub {{ base: &{base}, subtract: {arg} }}"),
                    3 => format!("Amount::Half {{ base: &{base}, round_up: false }}"),
                    4 => format!("Amount::Half {{ base: &{base}, round_up: true }}"),
                    5 => format!("Amount::divided(&{base}, {arg}, false)"),
                    6 => format!("Amount::divided(&{base}, {arg}, true)"),
                    _ => unreachable!(),
                });
            }
        }
        if !static_only {
            if self.in_delayed {
                match def {
                    "Spawner>Sacrificed$CardPower"
                    | "DelayTriggerRemembered$CardPower"
                    | "DelayTriggerRememberedLKI$CardPower" => {
                        return Some("Amount::EventPower".to_string());
                    }
                    "Spawner>Sacrificed$CardToughness"
                    | "DelayTriggerRemembered$CardToughness"
                    | "DelayTriggerRememberedLKI$CardToughness" => {
                        return Some("Amount::EventLastToughness".to_string());
                    }
                    "Spawner>Sacrificed$CardManaCost"
                    | "DelayTriggerRemembered$CardManaCost"
                    | "DelayTriggerRememberedLKI$CardManaCost" => {
                        return Some("Amount::EventManaValue".to_string());
                    }
                    _ => {}
                }
            }
            if def == "TriggerCount$DamageAmount"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce" | "DamageDealtOnce")
                )
            {
                return Some("Amount::EventAmount".to_string());
            }
            if def == "TriggerCount$Result"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("RolledDie" | "RolledDieOnce")
                )
            {
                return Some("Amount::EventAmount".to_string());
            }
            if def == "Count$LastDieRoll" {
                return Some("Amount::LastDieRoll".to_string());
            }
            if def == "Count$CoinsWonThisResolution" {
                return Some("Amount::CoinsWonThisResolution".to_string());
            }
            if def == "TriggerCount$LifeAmount"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("LifeGained" | "LifeLost")
                )
            {
                return Some("Amount::EventAmount".to_string());
            }
            if def == "TriggerCount$Amount"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZoneAll" | "AttackersDeclared" | "AttackersDeclaredOneTarget")
                )
            {
                return Some("Amount::EventAmount".to_string());
            }
            if (matches!(
                def,
                "TriggeredCard$CardPower" | "TriggeredCardLKICopy$CardPower"
            ) && matches!(
                self.trigger_mode.as_deref(),
                Some(
                    "ChangesZone" | "Sacrificed" | "Taps" | "Untaps" | "TapsForMana" | "TurnFaceUp"
                )
            )) || (matches!(
                def,
                "TriggeredAttacker$CardPower" | "TriggeredAttackerLKICopy$CardPower"
            ) && matches!(
                self.trigger_mode.as_deref(),
                Some("Attacks" | "AttackerUnblocked")
            )) || (matches!(
                def,
                "TriggeredBlocker$CardPower" | "TriggeredBlockerLKICopy$CardPower"
            ) && (self.trigger_mode.as_deref() == Some("Blocks")
                || (self.trigger_is_self
                    && matches!(
                        self.trigger_mode.as_deref(),
                        Some("AttackerBlocked" | "AttackerBlockedByCreature")
                    ))))
            {
                return Some("Amount::EventPower".to_string());
            }
            if (matches!(
                def,
                "TriggeredCard$CardToughness"
                    | "TriggeredCardLKICopy$CardToughness"
                    | "TriggeredCard$Toughness"
                    | "TriggeredCardLKICopy$Toughness"
            ) && matches!(
                self.trigger_mode.as_deref(),
                Some(
                    "ChangesZone" | "Sacrificed" | "Taps" | "Untaps" | "TapsForMana" | "TurnFaceUp"
                )
            )) || (matches!(
                def,
                "TriggeredAttacker$CardToughness" | "TriggeredAttackerLKICopy$CardToughness"
            ) && matches!(
                self.trigger_mode.as_deref(),
                Some("Attacks" | "AttackerUnblocked")
            )) || (matches!(
                def,
                "TriggeredBlocker$CardToughness" | "TriggeredBlockerLKICopy$CardToughness"
            ) && (self.trigger_mode.as_deref() == Some("Blocks")
                || (self.trigger_is_self
                    && matches!(
                        self.trigger_mode.as_deref(),
                        Some("AttackerBlocked" | "AttackerBlockedByCreature")
                    ))))
            {
                return Some("Amount::EventLastToughness".to_string());
            }
            if (matches!(
                def,
                "TriggeredCard$CardManaCost"
                    | "TriggeredCard$CardManaCostLKI"
                    | "TriggeredSpellAbility$CardManaCostLKI"
                    | "TriggeredSpellAbility$CardManaCost"
            ) && self.trigger_mode.as_deref() == Some("SpellCast"))
                || (matches!(
                    def,
                    "TriggeredCard$CardManaCost"
                        | "TriggeredCard$CardManaCostLKI"
                        | "TriggeredCardLKICopy$CardManaCost"
                ) && matches!(
                    self.trigger_mode.as_deref(),
                    Some(
                        "ChangesZone"
                            | "Sacrificed"
                            | "Discarded"
                            | "Cycled"
                            | "Taps"
                            | "Untaps"
                            | "TapsForMana"
                            | "TurnFaceUp"
                    )
                ))
                || (matches!(
                    def,
                    "TriggeredAttacker$CardManaCost" | "TriggeredAttackerLKICopy$CardManaCost"
                ) && matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked")
                ))
                || (matches!(
                    def,
                    "TriggeredBlocker$CardManaCost" | "TriggeredBlockerLKICopy$CardManaCost"
                ) && (self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_is_self
                        && matches!(
                            self.trigger_mode.as_deref(),
                            Some("AttackerBlocked" | "AttackerBlockedByCreature")
                        ))))
            {
                return Some("Amount::EventManaValue".to_string());
            }
            if matches!(
                def,
                "Targeted$CardToughness" | "ParentTargeted$CardToughness"
            ) {
                return Some("Amount::TargetToughness".to_string());
            }
            if matches!(def, "Targeted$CardPower" | "ParentTargeted$CardPower") {
                return Some("Amount::TargetPower".to_string());
            }
            if matches!(def, "Targeted$CardManaCost" | "ParentTargeted$CardManaCost") {
                return Some("Amount::TargetCmc".to_string());
            }
            if matches!(def, "Targeted$CardSumPT" | "ParentTargeted$CardSumPT") {
                return Some(
                    "Amount::Sum { left: &Amount::TargetPower, right: &Amount::TargetToughness }"
                        .to_string(),
                );
            }
            if def == "Sacrificed$CardPower" && self.cost_sacrifices_self && !self.on_a_spell {
                return Some("Amount::SourcePower".to_string());
            }
            if def == "Sacrificed$CardPower" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedPower".to_string());
            }
            if def == "Sacrificed$CardToughness" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedToughness".to_string());
            }
            if def == "Sacrificed$CardManaCost" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedManaValue".to_string());
            }
            if def == "Sacrificed$TotalCardPower" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedTotalPower".to_string());
            }
            if def == "Sacrificed$TotalCardToughness" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedTotalToughness".to_string());
            }
            if def == "Sacrificed$TotalCardManaCost" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedTotalManaValue".to_string());
            }
            if def == "Sacrificed$Colors" && self.cost_sacrifices_other {
                return Some("Amount::SacrificedDistinctColors".to_string());
            }
            if (matches!(def, "Count$TotalManaSpent" | "Count$CastTotalManaSpent")
                && (self.on_a_spell || self.is_etb_self_trigger))
                || (def == "TriggeredCard$CastTotalManaSpent" && self.is_etb_self_trigger)
            {
                return Some("Amount::ManaSpentToCast".to_string());
            }
            if def == "Tapped$CardPower" && !self.on_a_spell {
                return Some("Amount::TappedPower".to_string());
            }
            if matches!(
                def,
                "TargetedPlayer$CardsInHand" | "ThisTargetedPlayer$CardsInHand"
            ) {
                return Some("Amount::HandSize(PlayerRel::Chosen)".to_string());
            }
            if matches!(
                def,
                "TargetedPlayer$CardsInLibrary" | "ThisTargetedPlayer$CardsInLibrary"
            ) {
                return Some("Amount::LibrarySize(PlayerRel::Chosen)".to_string());
            }
            if matches!(
                def,
                "TargetedPlayer$LifeTotal" | "ThisTargetedPlayer$LifeTotal"
            ) {
                return Some("Amount::LifeTotal(PlayerRel::Chosen)".to_string());
            }
            if def == "TargetedController$CardsInHand" {
                return Some("Amount::HandSize(PlayerRel::ControllerOfTarget)".to_string());
            }
            if def == "TargetedController$CardsInLibrary" {
                return Some("Amount::LibrarySize(PlayerRel::ControllerOfTarget)".to_string());
            }
            if def == "TargetedController$LifeTotal" {
                return Some("Amount::LifeTotal(PlayerRel::ControllerOfTarget)".to_string());
            }
            if let Some(valid) = def
                .strip_prefix("TargetedPlayer$Valid ")
                .or_else(|| def.strip_prefix("ThisTargetedPlayer$Valid "))
                && let Some(cleaned) = strip_player_valid_scope(valid, "YouCtrl")
            {
                let expr = self.filter_expr(&cleaned)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::ControlledCount {{ who: PlayerRel::Chosen, filter: &{name} }}"
                ));
            }
            if let Some(valid) = def.strip_prefix("TriggeredPlayer$Valid ")
                && self.trigger_mode.as_deref() == Some("Phase")
                && let Some(cleaned) = strip_player_valid_scope(valid, "YouCtrl")
            {
                let expr = self.filter_expr(&cleaned)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::ControlledCount {{ who: PlayerRel::ActivePlayer, filter: &{name} }}"
                ));
            }
            if let Some(expr) = self.player_scoped_amount(def) {
                return Some(expr);
            }
            if matches!(def, "Count$KickedTimes" | "Count$TimesKicked") {
                return Some("Amount::TimesKicked".to_string());
            }
            if matches!(
                def,
                "Count$CastTotalManaSpent Snow" | "Count$TotalManaSpent Snow"
            ) && !self.in_delayed
                && (self.on_a_spell || self.trigger_is_self || self.trigger_mode.is_none())
            {
                return Some("Amount::SnowManaSpentToCast".to_string());
            }
            if def == "Count$Devoured Card" && !self.on_a_spell && !self.in_delayed {
                return Some("Amount::EntrySacrifices".to_string());
            }
            if let Some(warped) = Self::warped_this_turn_amount(def) {
                return Some(warped.to_string());
            }
            if self.on_a_spell
                && self.has_x
                && let Some(other) = def.strip_prefix("Count$Foretold.X.")
                && let Ok(n) = other.trim().parse::<u32>()
            {
                return Some(format!("Amount::XIfSourceWasForetold {{ otherwise: {n} }}"));
            }
        }
        if def == "PlayerCountPropertyYou$OpponentsAttackedThisCombat" {
            return Some(self.opponents_attacked_amount().to_string());
        }
        if let Some(expr) = self.enchanted_player_amount(def) {
            return Some(expr);
        }
        if matches!(
            def,
            "Count$ValidAll Card.IsCommander+YouOwn$GreatestCardManaCost"
                | "Count$ValidAll Card.YouOwn+IsCommander$GreatestCardManaCost"
        ) {
            return Some("Amount::GreatestCommanderManaValue".to_string());
        }
        if def == "TriggeredPlayer$CardsInHand" && self.trigger_mode.as_deref() == Some("Phase") {
            return Some("Amount::HandSize(PlayerRel::ActivePlayer)".to_string());
        }
        if def == "TriggeredPlayer$CardsInLibrary" && self.trigger_mode.as_deref() == Some("Phase")
        {
            return Some("Amount::LibrarySize(PlayerRel::ActivePlayer)".to_string());
        }
        if def == "TriggeredPlayer$LifeTotal" && self.trigger_mode.as_deref() == Some("Phase") {
            return Some("Amount::LifeTotal(PlayerRel::ActivePlayer)".to_string());
        }
        if def == "TriggeredPlayer$CardsInGraveyard"
            && self.trigger_mode.as_deref() == Some("Phase")
        {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::ActivePlayer) }"
                    .to_string(),
            );
        }
        if matches!(def, "Count$CardsInYourHand" | "Count$ValidHand Card.YouOwn") {
            return Some("Amount::HandSize(PlayerRel::You)".to_string());
        }
        if matches!(
            def,
            "Count$CardsInYourLibrary" | "Count$ValidLibrary Card.YouOwn"
        ) {
            return Some("Amount::LibrarySize(PlayerRel::You)".to_string());
        }
        if matches!(
            def,
            "Count$StartingLife" | "Count$StartingLifeTotal" | "Count$YourStartingLife"
        ) {
            return Some("Amount::StartingLifeTotal(PlayerRel::You)".to_string());
        }
        if def == "Count$LifeYouGainedThisTurn" {
            return Some("Amount::LifeGainedThisTurn(PlayerRel::You)".to_string());
        }
        if def == "Count$LifeOppsLostThisTurn" {
            return Some("Amount::OpponentsLifeLostThisTurn".to_string());
        }
        if matches!(def, "Count$YourPoisonCounters" | "Count$YourCountersPoison") {
            return Some("Amount::CountersOnYou(CounterKind::Poison)".to_string());
        }
        if matches!(def, "Count$YourEnergyCounters" | "Count$YourCountersEnergy") {
            return Some("Amount::CountersOnYou(CounterKind::Energy)".to_string());
        }
        if matches!(def, "Count$YourRadCounters" | "Count$YourCountersRAD") {
            return Some("Amount::CountersOnYou(CounterKind::Rad)".to_string());
        }
        if def == "Count$DamageYouTookThisTurn" {
            return Some("Amount::DamageDealtToYouThisTurn".to_string());
        }
        if def == "Count$YourLifeTotal" {
            return Some("Amount::LifeTotal(PlayerRel::You)".to_string());
        }
        if def == "Count$Domain" {
            return Some("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)".to_string());
        }
        if def == "Count$Party" {
            return Some("Amount::PartySize(PlayerRel::You)".to_string());
        }
        if def == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" {
            return Some("Amount::CreaturesDiedThisTurn".to_string());
        }
        if def == "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" {
            return Some("Amount::CreaturesEnteredUnderYourControlThisTurn".to_string());
        }
        if def == "PlayerCountOpponents$Amount" {
            return Some("Amount::Players(PlayerRel::Opponent)".to_string());
        }
        if def == "PlayerCountPlayers$Amount" {
            return Some("Amount::Players(PlayerRel::EachPlayer)".to_string());
        }
        if def == "Count$CardPower" && !self.on_a_spell {
            return Some("Amount::SourcePower".to_string());
        }
        if let Some(kind) = def.strip_prefix("Count$CardCounters.") {
            return super::counter_kind(kind)
                .map(|kind| format!("Amount::CountersOnSource({kind})"));
        }
        if let Some(valid) = def
            .strip_prefix("Count$ValidBattlefield,Graveyard ")
            .or_else(|| def.strip_prefix("Count$ValidGraveyard,Battlefield "))
            && !valid.contains('$')
            && !valid.contains('/')
        {
            if !valid.contains("Ctrl") && !valid.contains("Own") {
                let bf_expr = self.filter_expr(valid)?;
                let gy_expr = self.filter_expr_in_zone(valid, true)?;
                let bf_name = self.body.filter_static("COUNT", &bf_expr);
                let gy_name = self.body.filter_static("COUNT", &gy_expr);
                return Some(format!(
                    "Amount::Sum {{ left: &Amount::CountOf {{ filter: &{bf_name}, zone: ZoneSel::Battlefield }}, \
                     right: &Amount::CountOf {{ filter: &{gy_name}, zone: ZoneSel::GraveyardAll }} }}"
                ));
            }
            if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
                let bf_expr = self.filter_expr(valid)?;
                let gy_expr = self.filter_expr_in_zone(&stripped, true)?;
                let bf_name = self.body.filter_static("COUNT", &bf_expr);
                let gy_name = self.body.filter_static("COUNT", &gy_expr);
                return Some(format!(
                    "Amount::Sum {{ left: &Amount::CountOf {{ filter: &{bf_name}, zone: ZoneSel::Battlefield }}, \
                     right: &Amount::CountOf {{ filter: &{gy_name}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }} }}"
                ));
            }
        }
        if def == "Count$ValidGraveyard Card.YouOwn" {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::You) }"
                    .to_string(),
            );
        }
        if let Some(valid) = def.strip_prefix("Count$ValidGraveyard ") {
            let valid_owned = normalize_graveyard_entered(valid);
            let valid = valid_owned.as_str();
            if !valid.contains("Ctrl") && !valid.contains("Own") {
                let expr = self.filter_expr_in_zone(valid, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardAll }}"
                ));
            }
            if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}"
                ));
            }
            if let Some(stripped) = Self::strip_opp_graveyard_owner(valid) {
                let lower = self.oracle.to_ascii_lowercase();
                if lower.contains("target opponent's graveyard")
                    || lower.contains("target player's graveyard")
                {
                    return None;
                }
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOpponents }}"
                ));
            }
            if let Some((stripped, who)) = self.strip_named_graveyard_owner(valid, static_only) {
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOf({who}) }}"
                ));
            }
            return None;
        }
        if let Some(valid) = def.strip_prefix("Count$ValidHand ") {
            let stripped = Self::strip_your_graveyard_owner(valid)?;
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!(
                "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::HandYou }}"
            ));
        }
        if let Some(valid) = def.strip_prefix("Count$ValidExile ") {
            if valid == "Card.ExiledWithSource" {
                return Some(
                    "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::LinkedExile }"
                        .to_string(),
                );
            }
            if let Some(stripped) = valid
                .strip_suffix("+ExiledWithSource")
                .or_else(|| valid.strip_suffix(".ExiledWithSource"))
            {
                let expr = self.filter_expr_in_zone(stripped, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::LinkedExile }}"
                ));
            }
            return None;
        }
        if let Some(valid) = def.strip_prefix("Count$ValidLibrary ") {
            let stripped = Self::strip_your_graveyard_owner(valid)?;
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!(
                "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::LibraryYou }}"
            ));
        }
        let valid = def.strip_prefix("Count$Valid ")?;
        if let Some(color_valid) = valid.strip_suffix("$Colors")
            && !color_valid.contains('/')
            && !color_valid.contains('$')
        {
            let expr = self.filter_expr(color_valid)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!("Amount::DistinctColorsAmong(&{name})"));
        }
        if let Some(basic_valid) = valid.strip_suffix("$BasicLandTypes")
            && !basic_valid.contains('/')
            && !basic_valid.contains('$')
        {
            let expr = self.filter_expr(basic_valid)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!("Amount::BasicLandTypesAmong(&{name})"));
        }
        if let Some(mv_valid) = valid.strip_suffix("$GreatestCardManaCost")
            && !mv_valid.contains('/')
            && !mv_valid.contains('$')
        {
            let expr = self.filter_expr(mv_valid)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!("Amount::GreatestManaValueAmong(&{name})"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}"
        ))
    }

    fn warped_this_turn_amount(def: &str) -> Option<&'static str> {
        match def {
            "Count$ThisTurnWarped_You"
            | "Count$WarpedThisTurn_You"
            | "Count$ThisTurnCast_Card.YouCtrl+CastSa Spell.Warp" => {
                Some("Amount::SpellsWarpedThisTurn(PlayerRel::You)")
            }
            "Count$ThisTurnWarped_Opponent"
            | "Count$WarpedThisTurn_Opponent"
            | "Count$ThisTurnCast_Card.OppCtrl+CastSa Spell.Warp" => {
                Some("Amount::SpellsWarpedThisTurn(PlayerRel::Opponent)")
            }
            "Count$ThisTurnWarped"
            | "Count$WarpedThisTurn"
            | "Count$SpellsWarpedThisTurn"
            | "Count$ThisTurnCast_Card.CastSa Spell.Warp" => {
                Some("Amount::SpellsWarpedThisTurn(PlayerRel::EachPlayer)")
            }
            _ => None,
        }
    }

    fn player_scoped_amount(&mut self, def: &str) -> Option<String> {
        let is_damage = matches!(
            self.trigger_mode.as_deref(),
            Some("DamageDone" | "DamageDoneOnce")
        );
        let is_phase = self.trigger_mode.as_deref() == Some("Phase");
        if matches!(
            def,
            "TargetedPlayer$CardsInGraveyard" | "ThisTargetedPlayer$CardsInGraveyard"
        ) {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::Chosen) }"
                    .to_string(),
            );
        }
        if def == "TargetedController$CardsInGraveyard" {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::ControllerOfTarget) }"
                    .to_string(),
            );
        }
        if is_damage {
            match def {
                "TriggeredTarget$LifeTotal" => {
                    return Some("Amount::LifeTotal(PlayerRel::DamagedPlayer)".to_string());
                }
                "TriggeredTarget$CardsInHand" => {
                    return Some("Amount::HandSize(PlayerRel::DamagedPlayer)".to_string());
                }
                "TriggeredTarget$CardsInLibrary" => {
                    return Some("Amount::LibrarySize(PlayerRel::DamagedPlayer)".to_string());
                }
                "TriggeredTarget$CardsInGraveyard" => {
                    return Some(
                        "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::DamagedPlayer) }"
                            .to_string(),
                    );
                }
                _ => {}
            }
        }
        let controlled = [
            (
                "TargetedController$Valid ",
                "PlayerRel::ControllerOfTarget",
                true,
            ),
            (
                "TriggeredTarget$Valid ",
                "PlayerRel::DamagedPlayer",
                is_damage,
            ),
        ];
        for (prefix, who, ok) in controlled {
            if ok
                && let Some(valid) = def.strip_prefix(prefix)
                && let Some(cleaned) = strip_player_valid_scope(valid, "YouCtrl")
            {
                let expr = self.filter_expr(&cleaned)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::ControlledCount {{ who: {who}, filter: &{name} }}"
                ));
            }
        }
        self.player_graveyard_amount(def, is_phase, is_damage)
    }

    fn player_graveyard_amount(
        &mut self,
        def: &str,
        is_phase: bool,
        is_damage: bool,
    ) -> Option<String> {
        let graveyards = [
            ("TargetedPlayer$ValidGraveyard ", "PlayerRel::Chosen", true),
            (
                "ThisTargetedPlayer$ValidGraveyard ",
                "PlayerRel::Chosen",
                true,
            ),
            (
                "TargetedController$ValidGraveyard ",
                "PlayerRel::ControllerOfTarget",
                true,
            ),
            (
                "TriggeredPlayer$ValidGraveyard ",
                "PlayerRel::ActivePlayer",
                is_phase,
            ),
            (
                "TriggeredTarget$ValidGraveyard ",
                "PlayerRel::DamagedPlayer",
                is_damage,
            ),
        ];
        for (prefix, who, ok) in graveyards {
            if ok
                && let Some(valid) = def.strip_prefix(prefix)
                && let Some(cleaned) = strip_player_valid_scope(valid, "YouOwn")
            {
                let normalized = normalize_graveyard_entered(&cleaned);
                let expr = self.filter_expr_in_zone(&normalized, true)?;
                let name = self.body.filter_static("COUNT", &expr);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOf({who}) }}"
                ));
            }
        }
        None
    }

    fn enchanted_player_amount(&mut self, def: &str) -> Option<String> {
        match def {
            "Count$EnchantedControllerCreatures" => {
                return Some(
                    "Amount::ControlledCount { who: PlayerRel::ControllerOfAttached, filter: &Filter::CREATURE }"
                        .to_string(),
                );
            }
            "EnchantedPlayer$CardsInHand" => {
                return Some("Amount::HandSize(PlayerRel::EnchantedPlayer)".to_string());
            }
            "EnchantedPlayer$CardsInLibrary" => {
                return Some("Amount::LibrarySize(PlayerRel::EnchantedPlayer)".to_string());
            }
            "EnchantedPlayer$LifeTotal" => {
                return Some("Amount::LifeTotal(PlayerRel::EnchantedPlayer)".to_string());
            }
            "EnchantedPlayer$CardsInGraveyard" => {
                return Some(
                    "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::EnchantedPlayer) }"
                        .to_string(),
                );
            }
            _ => {}
        }
        if let Some(valid) = def.strip_prefix("EnchantedPlayer$Valid ")
            && let Some(cleaned) = strip_player_valid_scope(valid, "YouCtrl")
        {
            let expr = self.filter_expr(&cleaned)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!(
                "Amount::ControlledCount {{ who: PlayerRel::EnchantedPlayer, filter: &{name} }}"
            ));
        }
        if let Some(valid) = def.strip_prefix("EnchantedPlayer$ValidGraveyard ")
            && let Some(cleaned) = strip_player_valid_scope(valid, "YouOwn")
        {
            let normalized = normalize_graveyard_entered(&cleaned);
            let expr = self.filter_expr_in_zone(&normalized, true)?;
            let name = self.body.filter_static("COUNT", &expr);
            return Some(format!(
                "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOf(PlayerRel::EnchantedPlayer) }}"
            ));
        }
        None
    }

    fn strip_named_graveyard_owner(
        &self,
        valid: &str,
        static_only: bool,
    ) -> Option<(String, &'static str)> {
        let is_damage = matches!(
            self.trigger_mode.as_deref(),
            Some("DamageDone" | "DamageDoneOnce")
        );
        let is_phase = self.trigger_mode.as_deref() == Some("Phase");
        let candidates = [
            ("OwnedBy TargetedPlayer", "PlayerRel::Chosen", !static_only),
            (
                "OwnedBy TargetedController",
                "PlayerRel::ControllerOfTarget",
                !static_only,
            ),
            (
                "OwnedBy EnchantedController",
                "PlayerRel::ControllerOfAttached",
                true,
            ),
            (
                "EnchantedControllerOwn",
                "PlayerRel::ControllerOfAttached",
                true,
            ),
            (
                "EnchantedControllerCtrl",
                "PlayerRel::ControllerOfAttached",
                true,
            ),
            ("EnchantedPlayerOwn", "PlayerRel::EnchantedPlayer", true),
            (
                "OwnedBy EnchantedPlayer",
                "PlayerRel::EnchantedPlayer",
                true,
            ),
            (
                "OwnedBy Player.EnchantedBy",
                "PlayerRel::EnchantedPlayer",
                true,
            ),
            (
                "OwnedBy TriggeredPlayer",
                "PlayerRel::ActivePlayer",
                is_phase,
            ),
            (
                "OwnedBy TriggeredTarget",
                "PlayerRel::DamagedPlayer",
                !static_only && is_damage,
            ),
        ];
        for (owner_atom, who, ok) in candidates {
            if !ok {
                continue;
            }
            let mut cleaned_alts = Vec::new();
            let mut matched = true;
            for alt in valid.split(',') {
                let mut parts = alt
                    .split('.')
                    .flat_map(|s| s.split('+'))
                    .map(str::trim)
                    .filter(|a| !a.is_empty());
                let Some(base) = parts.next() else {
                    matched = false;
                    break;
                };
                let mut saw = false;
                let mut kept = Vec::new();
                for atom in parts {
                    if atom == owner_atom {
                        saw = true;
                    } else if atom.contains("Ctrl") || atom.contains("Own") {
                        matched = false;
                        break;
                    } else {
                        kept.push(atom);
                    }
                }
                if !matched || !saw {
                    matched = false;
                    break;
                }
                if kept.is_empty() {
                    cleaned_alts.push(base.to_string());
                } else {
                    cleaned_alts.push(format!("{base}.{}", kept.join("+")));
                }
            }
            if matched && !cleaned_alts.is_empty() {
                return Some((cleaned_alts.join(","), who));
            }
        }
        None
    }

    /// The atoms that are one word of printed text each, and the filter each
    /// is:
    ///
    /// - a name ("creatures named Plague Rats", [`named_atom`]);
    /// - a number the printed words compare against ("power 2 or less",
    ///   [`stat_atom`]), or "less than this creature's power" where `X` is
    ///   the source's power ([`Self::source_power_atom`]);
    /// - a mana value against a number or the announced X
    ///   ([`Self::cmc_atom`]);
    /// - "each creature with flying", "each creature without flying"
    ///   (Hurricane, Earthquake): a keyword the engine has a bit for
    ///   ([`keyword_atom`]). A keyword it has none for is a rule, not a flag,
    ///   and stays refused.
    pub(super) fn worded_atom(&self, atom: &str) -> Option<String> {
        if let Some(name) = named_atom(atom) {
            return Some(format!("Filter::Named({name:?})"));
        }
        if let Some(raw) = atom.strip_prefix("named")
            && !raw.is_empty()
            && raw.contains(';')
            && raw
                .chars()
                .all(|c| c.is_alphanumeric() || " '-;".contains(c))
        {
            let name = raw.replace(';', ",");
            return Some(format!("Filter::Named({name:?})"));
        }
        stat_atom(atom)
            .or_else(|| self.source_power_atom(atom))
            .or_else(|| self.cmc_atom(atom))
            .or_else(|| self.dynamic_power_atom(atom))
            .or_else(|| keyword_atom(atom))
    }

    /// `powerLT<var>`, `powerLE<var>`, and `toughnessLT<var>` where `<var>` is
    /// `Count$CardPower`, the source's own power.
    pub(super) fn source_power_atom(&self, atom: &str) -> Option<String> {
        let (kind, var) = if let Some(var) = atom.strip_prefix("powerLT") {
            (0u8, var)
        } else if let Some(var) = atom.strip_prefix("powerLE") {
            (1u8, var)
        } else {
            let var = atom.strip_prefix("toughnessLT")?;
            (2u8, var)
        };
        if var.is_empty()
            || !var.chars().all(|c| c.is_ascii_alphabetic())
            || self.svars.get(var).map(|x| x.trim()) != Some("Count$CardPower")
        {
            return None;
        }
        Some(
            match kind {
                0 => "Filter::PowerLessThanSourcePower",
                1 => "Filter::PowerAtMostSourcePower",
                _ => "Filter::ToughnessLessThanSourcePower",
            }
            .to_string(),
        )
    }

    /// `cmcLE2`, `cmcGE4`, `cmcEQX`, or dynamic Wave 14a `SVar` comparisons
    /// (`Filter::CmcAtMostAmount`, `Filter::CmcEqualsAmount`).
    pub(super) fn cmc_atom(&self, atom: &str) -> Option<String> {
        let (cmp, number) = atom.strip_prefix("cmc")?.split_at_checked(2)?;
        if !number.is_empty() && number.chars().all(|c| c.is_ascii_alphabetic()) {
            let def = self.svars.get(number).map(|s| s.trim());
            if number == "X" {
                let announced =
                    self.has_x && matches!(def, Some("Count$xPaid" | "Targeted$CardManaCost"));
                match cmp {
                    "LE" if announced => return Some("Filter::CmcAtMostX".to_string()),
                    "EQ" if announced => return Some("Filter::CmcExactlyX".to_string()),
                    _ => {}
                }
            }
            if cmp == "LE" && self.on_a_spell && def == Some("Count$Converge") {
                return Some("Filter::CmcAtMostColorsSpent".to_string());
            }
            if cmp == "LE"
                && !self.in_delayed
                && (self.on_a_spell || self.trigger_is_self || self.trigger_mode.is_none())
                && matches!(
                    def,
                    Some("Count$CastTotalManaSpent Snow" | "Count$TotalManaSpent Snow")
                )
            {
                return Some("Filter::CmcAtMostSnowManaSpent".to_string());
            }
            if cmp == "EQ" && def == Some("Count$ManaPool:All") {
                return Some("Filter::CmcEqualsYourUnspentMana".to_string());
            }
            if cmp == "EQ"
                && (self.trigger_mode.is_some() || !self.cost_sacrifices_self)
                && let Some(kind_code) = def.and_then(|d| d.strip_prefix("Count$CardCounters."))
            {
                let kind = counter_kind(kind_code)?;
                return Some(format!("Filter::CmcEqualsCountersOnSource({kind})"));
            }
            let amt = self.dynamic_filter_amount(number, 0)?;
            return match cmp {
                "LE" => Some(format!("Filter::CmcAtMostAmount(&{amt})")),
                "EQ" => Some(format!("Filter::CmcEqualsAmount(&{amt})")),
                "GT" => Some(format!("Filter::Not(&Filter::CmcAtMostAmount(&{amt}))")),
                "GE" => Some(format!(
                    "Filter::Or(&[Filter::CmcEqualsAmount(&{amt}), Filter::Not(&Filter::CmcAtMostAmount(&{amt}))])"
                )),
                _ => None,
            };
        }
        let n: u32 = number.parse().ok()?;
        Some(match cmp {
            "LE" => format!("Filter::CmcAtMost({n})"),
            "LT" => format!("Filter::CmcAtMost({})", n.checked_sub(1)?),
            "GE" => format!("Filter::CmcAtLeast({n})"),
            "GT" => format!("Filter::CmcAtLeast({})", n.checked_add(1)?),
            "EQ" => format!("Filter::And(&[Filter::CmcAtMost({n}), Filter::CmcAtLeast({n})])"),
            _ => return None,
        })
    }

    /// Wave 14a dynamic `SVar` power atoms (`powerLE<var>`, `powerGE<var>`,
    /// `powerGT<var>`, `powerEQ<var>` -> `Filter::PowerAtMostAmount` /
    /// `Filter::PowerAtLeastAmount`).
    fn dynamic_power_atom(&self, atom: &str) -> Option<String> {
        let (cmp, var) = atom.strip_prefix("power")?.split_at_checked(2)?;
        if var.is_empty()
            || !var.chars().all(|c| c.is_ascii_alphabetic())
            || matches!(var, "toughness" | "basePower" | "Chosen")
        {
            return None;
        }
        let amt = self.dynamic_filter_amount(var, 0)?;
        match cmp {
            "LE" => Some(format!("Filter::PowerAtMostAmount(&{amt})")),
            "GE" => Some(format!("Filter::PowerAtLeastAmount(&{amt})")),
            "GT" => Some(format!(
                "Filter::PowerAtLeastAmount(&Amount::Plus {{ base: &{amt}, offset: 1 }})"
            )),
            "EQ" => Some(format!(
                "Filter::And(&[Filter::PowerAtLeastAmount(&{amt}), Filter::PowerAtMostAmount(&{amt})])"
            )),
            _ => None,
        }
    }

    /// Resolves an alphabetic `SVar` name to a const-promotable inline `Amount`
    /// expression for `Filter::{CmcAtMostAmount, CmcEqualsAmount,
    /// PowerAtMostAmount, PowerAtLeastAmount}`. Strictly pure Paper-MTG only.
    fn dynamic_filter_amount(&self, var: &str, depth: u8) -> Option<String> {
        if depth >= 3 {
            return None;
        }
        let def = self.svars.get(var.trim())?.trim();
        self.dynamic_filter_amount_def(def, depth)
    }

    fn dynamic_filter_amount_def(&self, def: &str, depth: u8) -> Option<String> {
        let def = def.trim();
        if depth >= 3
            || def.contains("Intensity")
            || def.contains("Sticker")
            || def.contains("Attraction")
            || def.contains("Contraption")
        {
            return None;
        }
        if let Some(n_str) = def.strip_prefix("Number$")
            && let Ok(n) = n_str.trim().parse::<u32>()
        {
            return Some(format!("Amount::Fixed({n})"));
        }
        if let Some(inner_var) = def.strip_prefix("SVar$")
            && !inner_var.contains('/')
        {
            return self.dynamic_filter_amount(inner_var.trim(), depth + 1);
        }
        if let Some((lhs, rhs)) = def.rsplit_once('/') {
            return self.dynamic_filter_arith(lhs.trim(), rhs.trim(), depth);
        }
        self.dynamic_filter_atom(def, depth)
    }

    fn dynamic_filter_arith(&self, lhs: &str, rhs: &str, depth: u8) -> Option<String> {
        let base = if let Some(var) = lhs.strip_prefix("SVar$") {
            self.dynamic_filter_amount(var.trim(), depth + 1)?
        } else {
            self.dynamic_filter_amount_def(lhs, depth + 1)?
        };
        if rhs == "Twice" {
            return Some(if base == "Amount::X" {
                "Amount::DoubleX".to_string()
            } else {
                format!("Amount::Times {{ base: &{base}, factor: 2 }}")
            });
        }
        if rhs == "Thrice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}"));
        }
        if rhs == "HalfDown" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if rhs == "HalfUp" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
        if let Some(n_str) = rhs.strip_prefix("Plus.") {
            if let Ok(offset) = n_str.trim().parse::<u32>() {
                return Some(format!(
                    "Amount::Plus {{ base: &{base}, offset: {offset} }}"
                ));
            }
            let right = self.dynamic_filter_amount(n_str.trim(), depth + 1)?;
            return Some(format!("Amount::Sum {{ left: &{base}, right: &{right} }}"));
        }
        if let Some(n_str) = rhs.strip_prefix("Minus.") {
            if let Ok(subtract) = n_str.trim().parse::<u32>() {
                return Some(format!(
                    "Amount::SaturatingSub {{ base: &{base}, subtract: {subtract} }}"
                ));
            }
            let right = self.dynamic_filter_amount(n_str.trim(), depth + 1)?;
            return Some(format!(
                "Amount::Difference {{ left: &{base}, right: &{right} }}"
            ));
        }
        if let Some(n_str) = rhs.strip_prefix("Times.")
            && let Ok(factor) = n_str.trim().parse::<u32>()
            && factor >= 1
        {
            return Some(format!(
                "Amount::Times {{ base: &{base}, factor: {factor} }}"
            ));
        }
        None
    }

    fn dynamic_filter_atom(&self, def: &str, depth: u8) -> Option<String> {
        let can_read_x =
            self.has_x || (self.is_etb_self_trigger && self.has_x_mana_cost && !self.in_delayed);
        if def == "Count$xPaid" && can_read_x {
            return Some("Amount::X".to_string());
        }
        if (matches!(def, "Count$TotalManaSpent" | "Count$CastTotalManaSpent")
            && (self.on_a_spell || self.is_etb_self_trigger))
            || (def == "TriggeredCard$CastTotalManaSpent" && self.is_etb_self_trigger)
        {
            return Some("Amount::ManaSpentToCast".to_string());
        }
        if def == "Count$CardPower" && !self.on_a_spell {
            return Some("Amount::SourcePower".to_string());
        }
        if let Some(kind_code) = def.strip_prefix("Count$CardCounters.")
            && (self.trigger_mode.is_some() || !self.cost_sacrifices_self)
        {
            return counter_kind(kind_code).map(|k| format!("Amount::CountersOnSource({k})"));
        }
        if def == "Count$ChosenNumber"
            && self.on_a_spell
            && !self.in_delayed
            && !self
                .body
                .enter_modifiers
                .iter()
                .any(|m| m.contains("ChooseNumber"))
        {
            return Some("Amount::ResolutionChosenNumber".to_string());
        }
        if let Some(event) = self.dynamic_filter_event_or_target_atom(def) {
            return Some(event);
        }
        if let Some(scalar) = dynamic_filter_scalar_atom(def) {
            return Some(scalar);
        }
        if depth == 0 {
            return self.dynamic_filter_counted_atom(def);
        }
        None
    }

    fn dynamic_filter_event_or_target_atom(&self, def: &str) -> Option<String> {
        if (def == "TriggerCount$DamageAmount"
            && matches!(
                self.trigger_mode.as_deref(),
                Some("DamageDone" | "DamageDoneOnce" | "DamageDealtOnce")
            ))
            || (def == "TriggerCount$LifeAmount"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("LifeGained" | "LifeLost")
                ))
            || (def == "TriggerCount$Amount"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZoneAll" | "AttackersDeclared" | "AttackersDeclaredOneTarget")
                ))
        {
            return Some("Amount::EventAmount".to_string());
        }
        if (matches!(
            def,
            "TriggeredCard$CardManaCost"
                | "TriggeredCard$CardManaCostLKI"
                | "TriggeredSpellAbility$CardManaCostLKI"
                | "TriggeredSpellAbility$CardManaCost"
        ) && self.trigger_mode.as_deref() == Some("SpellCast"))
            || (matches!(
                def,
                "TriggeredCard$CardManaCost"
                    | "TriggeredCard$CardManaCostLKI"
                    | "TriggeredCardLKICopy$CardManaCost"
            ) && matches!(
                self.trigger_mode.as_deref(),
                Some(
                    "ChangesZone"
                        | "Sacrificed"
                        | "Discarded"
                        | "Cycled"
                        | "Taps"
                        | "Untaps"
                        | "TapsForMana"
                        | "TurnFaceUp"
                )
            ))
        {
            return Some("Amount::EventManaValue".to_string());
        }
        if (matches!(
            def,
            "TriggeredCard$CardPower" | "TriggeredCardLKICopy$CardPower"
        ) && matches!(
            self.trigger_mode.as_deref(),
            Some("ChangesZone" | "Sacrificed" | "Taps" | "Untaps" | "TapsForMana" | "TurnFaceUp")
        )) || (matches!(
            def,
            "TriggeredAttacker$CardPower" | "TriggeredAttackerLKICopy$CardPower"
        ) && matches!(
            self.trigger_mode.as_deref(),
            Some("Attacks" | "AttackerUnblocked")
        )) {
            return Some("Amount::EventPower".to_string());
        }
        match def {
            "Targeted$CardPower" | "ParentTargeted$CardPower" => {
                Some("Amount::TargetPower".to_string())
            }
            "Targeted$CardToughness" | "ParentTargeted$CardToughness" => {
                Some("Amount::TargetToughness".to_string())
            }
            "Targeted$CardManaCost" | "ParentTargeted$CardManaCost" => {
                Some("Amount::TargetCmc".to_string())
            }
            "Sacrificed$CardPower" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedPower".to_string())
            }
            "Sacrificed$CardToughness" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedToughness".to_string())
            }
            "Sacrificed$CardManaCost" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedManaValue".to_string())
            }
            "Sacrificed$TotalCardPower" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedTotalPower".to_string())
            }
            "Sacrificed$TotalCardToughness" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedTotalToughness".to_string())
            }
            "Sacrificed$TotalCardManaCost" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedTotalManaValue".to_string())
            }
            "Sacrificed$Colors" if self.cost_sacrifices_other => {
                Some("Amount::SacrificedDistinctColors".to_string())
            }
            _ => None,
        }
    }

    fn dynamic_filter_counted_atom(&self, def: &str) -> Option<String> {
        let has_dynamic_stat = |s: &str| {
            s.split([',', '.', '+']).any(|atom| {
                atom.strip_prefix("cmc")
                    .or_else(|| atom.strip_prefix("power"))
                    .or_else(|| atom.strip_prefix("toughness"))
                    .and_then(|rest| rest.get(2..))
                    .is_some_and(|tail| {
                        !tail.is_empty() && tail.chars().all(|c| c.is_ascii_alphabetic())
                    })
            })
        };
        if let Some(valid) = def.strip_prefix("Count$TotalPower ")
            && !valid.contains('$')
            && !valid.contains('/')
            && !has_dynamic_stat(valid)
        {
            let expr = self.filter_expr(valid)?;
            return Some(format!("Amount::TotalPowerAmong(&{expr})"));
        }
        if let Some(valid) = def.strip_prefix("Count$ValidGraveyard ")
            && !valid.contains('$')
            && !valid.contains('/')
            && !has_dynamic_stat(valid)
        {
            let valid_owned = normalize_graveyard_entered(valid);
            let valid = valid_owned.as_str();
            if !valid.contains("Ctrl") && !valid.contains("Own") {
                let expr = self.filter_expr_in_zone(valid, true)?;
                return Some(format!(
                    "Amount::CountOf {{ filter: &{expr}, zone: ZoneSel::GraveyardAll }}"
                ));
            }
            if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                return Some(format!(
                    "Amount::CountOf {{ filter: &{expr}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}"
                ));
            }
            return None;
        }
        let valid = def.strip_prefix("Count$Valid ")?;
        if let Some(sub) = valid.strip_suffix("$GreatestCardPower")
            && !sub.contains('$')
            && !sub.contains('/')
            && !has_dynamic_stat(sub)
        {
            let expr = self.filter_expr(sub)?;
            return Some(format!("Amount::GreatestPowerAmong(&{expr})"));
        }
        if let Some(sub) = valid.strip_suffix("$GreatestCardManaCost")
            && !sub.contains('$')
            && !sub.contains('/')
            && !has_dynamic_stat(sub)
        {
            let expr = self.filter_expr(sub)?;
            return Some(format!("Amount::GreatestManaValueAmong(&{expr})"));
        }
        if let Some(sub) = valid.strip_suffix("$CardPower")
            && !sub.contains('$')
            && !sub.contains('/')
            && !has_dynamic_stat(sub)
        {
            let expr = self.filter_expr(sub)?;
            return Some(format!("Amount::TotalPowerAmong(&{expr})"));
        }
        if let Some(sub) = valid.strip_suffix("$Colors")
            && !sub.contains('$')
            && !sub.contains('/')
            && !has_dynamic_stat(sub)
        {
            let expr = self.filter_expr(sub)?;
            return Some(format!("Amount::DistinctColorsAmong(&{expr})"));
        }
        if let Some(sub) = valid.strip_suffix("$BasicLandTypes")
            && !sub.contains('$')
            && !sub.contains('/')
            && !has_dynamic_stat(sub)
        {
            let expr = self.filter_expr(sub)?;
            return Some(format!("Amount::BasicLandTypesAmong(&{expr})"));
        }
        if valid.contains('$') || valid.contains('/') || has_dynamic_stat(valid) {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        Some(format!(
            "Amount::CountOf {{ filter: &{expr}, zone: ZoneSel::Battlefield }}"
        ))
    }

    /// [`amount`], or else a count of permanents ([`Self::count_expr`]):
    /// Karma's "damage equal to the number of Swamps they control". A count
    /// is read as the ability resolves, like every other `Amount`.
    /// Strips `.YouOwn` / `+YouOwn` / `.YouCtrl` / `+YouCtrl` from every
    /// comma-separated alternative in a graveyard filter; returns `None` if
    /// any alternative does not scope to you or carries another player atom.
    pub(super) fn strip_your_graveyard_owner(valid: &str) -> Option<String> {
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut has_you = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "YouOwn" | "YouCtrl" => has_you = true,
                    other if other.contains("Ctrl") || other.contains("Own") => return None,
                    other => kept.push(other),
                }
            }
            if !has_you {
                return None;
            }
            if kept.is_empty() {
                cleaned_alts.push(base.to_string());
            } else {
                cleaned_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!cleaned_alts.is_empty()).then(|| cleaned_alts.join(","))
    }

    pub(super) fn strip_opp_graveyard_owner(valid: &str) -> Option<String> {
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut has_opp = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "OppOwn" | "OppCtrl" => has_opp = true,
                    other if other.contains("Ctrl") || other.contains("Own") => return None,
                    other => kept.push(other),
                }
            }
            if !has_opp {
                return None;
            }
            if kept.is_empty() {
                cleaned_alts.push(base.to_string());
            } else {
                cleaned_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!cleaned_alts.is_empty()).then(|| cleaned_alts.join(","))
    }

    pub(super) fn amount_or_count(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = self.amount_num(raw) {
            return Some(n);
        }
        let trimmed = raw.trim();
        let def = match self.svars.get(trimmed) {
            Some(def) => def.clone(),
            None if trimmed.contains('$') => trimmed.to_string(),
            None => return None,
        };
        if matches!(def.trim(), "Remembered$Amount" | "Count$RememberedSize") {
            let mut removals = self.svars.values().filter_map(|body| {
                let (api, mut p) = Params::parse(body)?;
                if api != "RemoveCounter"
                    || p.take("CounterNum").as_deref() != Some("All")
                    || p.take("RememberRemoved").as_deref() != Some("True")
                {
                    return None;
                }
                counter_kind(&p.take("CounterType")?)
            });
            if let Some(kind) = removals.next() {
                if removals.next().is_some() {
                    return self.deny("more than one remembered all-counter removal".to_string());
                }
                return Some(format!("Amount::ThisWay(ThisWay::CountersRemoved({kind}))"));
            }
            if let Some(valid) = self
                .svars
                .get("X")
                .and_then(|definition| definition.strip_prefix("Count$ValidLibrary "))
            {
                let expr = self.filter_expr(valid)?;
                let filter = self.body.filter_static("EXILED", &expr);
                return Some(format!(
                    "Amount::ThisWay(ThisWay::ExiledFromLibrary(&{filter}))"
                ));
            }
            return None;
        }
        self.count_expr(&def)
    }

    /// `RestrictValid$` — what produced mana may be spent on.
    ///
    /// Returns `(filter_expr, optional_mana_use)`: `None` uses the concise
    /// `.restricted(&filter, SpendRider::None)` (`ManaUse::Spells`), while
    /// `Some(uses)` emits `.restricted_for(&filter, SpendRider::None, uses)`
    /// (plus `.for_abilities_matching(...)` when a separate ability filter is
    /// returned in the third slot).
    pub(super) fn spend_restriction(
        &self,
        valid: &str,
    ) -> Option<(String, Option<&'static str>, Option<String>)> {
        let trimmed = valid.trim();
        match trimmed {
            "CantCastNonArtifactSpells" => {
                return Some((
                    "Filter::ARTIFACT".to_string(),
                    Some("ManaUse::NonSpellPaymentsOrMatchingSpells"),
                    None,
                ));
            }
            "CumulativeUpkeep" => {
                return Some((
                    "Filter::Any".to_string(),
                    Some("ManaUse::CumulativeUpkeep"),
                    None,
                ));
            }
            "Activated" => {
                return Some(("Filter::Any".to_string(), Some("ManaUse::Abilities"), None));
            }
            "Spell" => return Some(("Filter::Any".to_string(), None, None)),
            "nonSpell" => {
                return Some((
                    "Filter::Not(&Filter::Any)".to_string(),
                    Some("ManaUse::NonSpellPaymentsOrMatchingSpells"),
                    None,
                ));
            }
            _ => {}
        }
        let mut spell_any = false;
        let mut act_any = false;
        let mut spell_parts: Vec<String> = Vec::new();
        let mut act_parts: Vec<String> = Vec::new();
        for alt in trimmed.split(',').map(str::trim) {
            if alt == "Spell" {
                spell_any = true;
            } else if let Some(rest) = alt.strip_prefix("Spell.") {
                spell_parts.push(normalize_spend_valid(rest));
            } else if alt == "Activated" {
                act_any = true;
            } else if let Some(rest) = alt.strip_prefix("Activated.") {
                let cleaned = rest.strip_suffix("+inZoneBattlefield").unwrap_or(rest);
                act_parts.push(normalize_spend_valid(cleaned));
            } else {
                self.note("`Mana.RestrictValid` beyond a spell".to_string());
                return None;
            }
        }
        let has_spell = spell_any || !spell_parts.is_empty();
        let has_act = act_any || !act_parts.is_empty();
        if has_spell && !has_act {
            return Some((self.parse_spend_side(spell_any, &spell_parts)?, None, None));
        }
        let saved = self.unclaimed.take();
        let out = if has_act && !has_spell {
            self.parse_spend_side(act_any, &act_parts)
                .map(|expr| (expr, Some("ManaUse::Abilities"), None))
        } else if has_spell && has_act {
            let se = self.parse_spend_side(spell_any, &spell_parts);
            let ae = self.parse_spend_side(act_any, &act_parts);
            match (se, ae) {
                (Some(se), Some(ae)) if se == ae => {
                    Some((se, Some("ManaUse::SpellsOrAbilities"), None))
                }
                (Some(se), Some(ae)) => Some((se, Some("ManaUse::SpellsOrAbilities"), Some(ae))),
                _ => None,
            }
        } else {
            None
        };
        self.unclaimed.replace(saved);
        if out.is_none() {
            self.note("`Mana.RestrictValid` beyond a spell".to_string());
        }
        out
    }

    fn parse_spend_side(&self, any: bool, parts: &[String]) -> Option<String> {
        if any {
            Some("Filter::Any".to_string())
        } else {
            self.filter_expr(&parts.join(","))
        }
    }

    /// A `Cost$` value as a `Cost` expression, plus whether it taps.
    ///
    /// An unreadable token names itself: a cost is where the widest variety
    /// of the corpus's syntax shows up — `Discard<…>`, `Exile<…>`,
    /// `tapXType<…>` — and a report saying "a cost" would send the reader
    /// back to the script to find out which.
    ///
    /// `&mut self` because one part carries a filter, and a filter is
    /// declared as a `static` above the card the way a target's is.
    pub(super) fn cost_expr(&mut self, raw: &str) -> Option<String> {
        let (mana, parts) = self.cost_pieces(raw)?;
        Some(crate::body::cost_literal(&mana, &parts))
    }

    /// The same reading, before [`crate::body::cost_literal`] joins it.
    ///
    /// Split out because one caller wants a *part* rather than a cost:
    /// "unless you return an untapped Plains" is priced by a single
    /// [`CostPart`], and re-reading that syntax beside this one would be two
    /// places for `Return<1/Plains>` to mean two things.
    pub(super) fn cost_pieces(&mut self, raw: &str) -> Option<(String, Vec<String>)> {
        let mut mana = String::new();
        let mut parts: Vec<String> = Vec::new();
        for token in cost_parts(raw) {
            let token = token.as_str();
            if token == "T" {
                parts.push("TapSelf".to_string());
            } else if token == "Q" {
                parts.push("UntapSelf".to_string());
            } else if token == "Sac<1/CARDNAME>"
                || token.starts_with("Sac<1/CARDNAME/")
                || token == "Sac<1/NICKNAME>"
                || token.starts_with("Sac<1/NICKNAME/")
            {
                parts.push("SacrificeSelf".to_string());
            } else if (token == "Sac<1/OriginalHost>" || token.starts_with("Sac<1/OriginalHost/"))
                && !self.on_a_spell
                && !self.in_alternative_cost
            {
                parts.push("SacrificeGrantor".to_string());
            } else if token == "Exile<1/CARDNAME>"
                || token.starts_with("Exile<1/CARDNAME/")
                || token == "Exile<1/NICKNAME>"
                || token.starts_with("Exile<1/NICKNAME/")
            {
                parts.push("ExileSelf".to_string());
            } else if (token == "Exert<1/CARDNAME>"
                || token.starts_with("Exert<1/CARDNAME/")
                || token == "Exert<1/NICKNAME>"
                || token.starts_with("Exert<1/NICKNAME/"))
                && !self.on_a_spell
                && !self.in_alternative_cost
            {
                parts.push("ExertSelf".to_string());
            } else if token == "Discard<1/Hand>"
                || token == "Discard<0/Hand>"
                || token.starts_with("Discard<1/Hand/")
                || token.starts_with("Discard<0/Hand/")
            {
                parts.push("DiscardHand".to_string());
            } else if let Some(inner) = token
                .strip_prefix("AddCounter<")
                .and_then(|t| t.strip_suffix('>'))
            {
                parts.push(self.add_counter_cost(inner, token)?);
            } else if let Some(inner) = token
                .strip_prefix("ExileSameGrave<")
                .and_then(|t| t.strip_suffix('>'))
            {
                parts.push(self.exile_same_grave_cost(inner, token)?);
            } else if let Some(inner) = token
                .strip_prefix("RemoveAnyCounter<")
                .and_then(|t| t.strip_suffix('>'))
            {
                parts.push(self.remove_any_counter_cost(inner, token)?);
            } else if let Some((n, kind)) = token
                .strip_prefix("SubCounter<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.split_once('/'))
            {
                parts.push(self.sub_counter_cost(n, kind, token)?);
            } else if let Some((kind, body)) = object_cost(token) {
                parts.extend(self.object_cost_part(kind, body, token)?);
            } else if token == "Forage" {
                parts.push("Forage".to_string());
            } else if token.starts_with("PayLife<")
                || token.starts_with("PayEnergy<")
                || token.starts_with("ExileFromTop<")
                || token.starts_with("DamageYou<")
            {
                parts.push(self.energy_or_exile_top_cost(token)?);
            } else if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token, "W" | "U" | "B" | "R" | "G" | "C" | "S")
                || (token == "X"
                    && self.has_x
                    && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
                    && !self.oracle.to_ascii_lowercase().contains("x is "))
            {
                mana.push('{');
                mana.push_str(token);
                mana.push('}');
            } else if let Some(pair) = hybrid_pair(token).or_else(|| special_mana_symbol(token)) {
                // `Cost$ UR T` -> `{U/R}`, `2W` -> `{2/W}`, `WP` -> `{W/P}`,
                // `URP` -> `{U/R/P}` (CR 107.4e–f). Doubled pairs like `WW` or
                // `WWP` and non-color symbols like `2C`/`CP` return `None` and
                // keep refusing by name so `ColorPair::new` can never panic.
                mana.push_str(&pair);
            } else {
                let head = token.split('<').next().unwrap_or(token);
                return self.deny(format!("cost `{head}`"));
            }
        }
        Some((mana, parts))
    }

    fn add_counter_cost(&mut self, inner: &str, token: &str) -> Option<String> {
        let mut fields = inner.splitn(4, '/');
        let (Some(n_raw), Some(kind_raw)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        let target_opt = fields.next();
        if kind_raw == "LOYALTY" {
            // Loyalty costs belong exclusively on `Planeswalker$ True` loyalty
            // abilities (CR 606.3–4), never on ordinary counter costs.
            return self.deny("a loyalty cost".to_string());
        }
        let Ok(n) = n_raw.parse::<u16>() else {
            return self.deny(format!("counter count `{n_raw}`"));
        };
        let Some(kind) = counter_kind(kind_raw) else {
            return self.deny(format!("counter `{kind_raw}`"));
        };
        match target_opt {
            None | Some("CARDNAME" | "NICKNAME") => {
                Some(format!("PutCounterSelf {{ kind: {kind}, n: {n} }}"))
            }
            Some(valid) => {
                if self.on_a_spell || self.in_alternative_cost || n == 0 {
                    return self.deny(format!("cost `{token}`"));
                }
                let controlled_by_you = valid.split([',', ';']).all(|alt| {
                    let mut saw_you = false;
                    for atom in alt.split(['.', '+']).map(str::trim) {
                        if atom == "YouCtrl" {
                            saw_you = true;
                        } else if atom.contains("Ctrl")
                            || atom.contains("Own")
                            || matches!(atom, "CARDNAME" | "Self" | "targetedBy")
                        {
                            return false;
                        }
                    }
                    saw_you
                });
                if !controlled_by_you {
                    return self.deny(format!("a counter cost on `{valid}`"));
                }
                let normalized = normalize_object_cost_spec(valid, "PutCounterOnOther", true);
                let expr = self.filter_expr(&normalized)?;
                let name = self.body.filter_static("COST", &expr);
                Some(format!(
                    "PutCounterOnOther {{ filter: &{name}, kind: {kind}, n: {n} }}"
                ))
            }
        }
    }

    fn exile_same_grave_cost(&mut self, inner: &str, token: &str) -> Option<String> {
        if self.on_a_spell || self.in_alternative_cost || self.trigger_mode.is_some() {
            return self.deny(format!("cost `{token}`"));
        }
        let mut fields = inner.splitn(3, '/');
        let (Some(n_raw), Some(spec)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        let Some(count) = n_raw.parse::<u8>().ok().filter(|&c| c > 0) else {
            return self.deny(format!("cost `{token}`"));
        };
        if spec == "CARDNAME" || spec.contains("Ctrl") || spec.contains("Own") {
            return self.deny(format!("cost `{token}`"));
        }
        let normalized = spec.replace(';', ",");
        let expr = self.filter_expr_in_zone(&normalized, true)?;
        let name = self.body.filter_static("COST", &expr);
        Some(format!(
            "ExileFromSingleGraveyard {{ filter: &{name}, count: {count} }}"
        ))
    }

    pub(super) fn remove_any_counter_cost(&mut self, inner: &str, token: &str) -> Option<String> {
        let fields: Vec<&str> = inner.split('/').collect();
        if fields.len() < 3 || fields.len() > 5 {
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(zone) = fields.get(4).copied().map(str::trim)
            && !zone.is_empty()
            && zone != "Battlefield"
        {
            return self.deny(format!("cost `{token}` outside the battlefield"));
        }
        let count_raw = fields[0].trim();
        let kind_raw = fields[1].trim();
        let spec = fields[2].trim();
        let kind = if kind_raw.eq_ignore_ascii_case("any") {
            None
        } else {
            let Some(k) = counter_kind(kind_raw) else {
                return self.deny(format!("counter `{kind_raw}`"));
            };
            Some(k)
        };
        self.parse_remove_any_counter(count_raw, kind.as_deref(), spec, token)
    }

    pub(super) fn sub_counter_spell_cost(
        &mut self,
        n: &str,
        rest: &str,
        token: &str,
    ) -> Option<String> {
        let fields: Vec<&str> = rest.split('/').collect();
        if fields.is_empty() || fields.len() > 4 {
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(zone) = fields.get(3).copied().map(str::trim)
            && !zone.is_empty()
            && zone != "Battlefield"
        {
            return self.deny(format!("cost `{token}` outside the battlefield"));
        }
        let kind_raw = fields[0].trim();
        let spec_opt = fields.get(1).copied().map(str::trim);
        if kind_raw.eq_ignore_ascii_case("any") {
            let spec = spec_opt.unwrap_or("CARDNAME");
            return self.parse_remove_any_counter(n, None, spec, token);
        }
        let Some(spec) = spec_opt else {
            return self.deny(format!("cost `{token}` naming itself"));
        };
        let Some(kind) = counter_kind(kind_raw) else {
            return self.deny(format!("counter `{kind_raw}`"));
        };
        self.parse_remove_any_counter(n, Some(&kind), spec, token)
    }

    fn parse_remove_any_counter(
        &mut self,
        count_raw: &str,
        kind: Option<&str>,
        spec: &str,
        token: &str,
    ) -> Option<String> {
        let disallow_self = self.on_a_spell
            || self.in_alternative_cost
            || self.in_graveyard_activation
            || self.in_hand_activation;
        let filter = if matches!(spec.trim(), "CARDNAME" | "NICKNAME") {
            if disallow_self {
                return self.deny(format!("cost `{token}` naming itself"));
            }
            if kind == Some("CounterKind::Loyalty") {
                return self.deny("a loyalty cost".to_string());
            }
            "Filter::This".to_string()
        } else {
            let Some(cleaned) = normalize_remove_any_counter_spec(spec, disallow_self) else {
                return self.deny(format!("cost `{token}`"));
            };
            let Some(expr) = self.filter_expr(&cleaned) else {
                if self.unclaimed.borrow().is_none() {
                    self.note(format!("cost `{token}`"));
                }
                return None;
            };
            if kind == Some("CounterKind::Loyalty") && expr == "Filter::This" {
                return self.deny("a loyalty cost".to_string());
            }
            self.body.filter_static("COST", &expr)
        };
        let kind_expr = kind.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
        if count_raw == "X" {
            if self.trigger_mode.is_none()
                && self.has_x
                && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
                && !self.oracle.to_ascii_lowercase().contains("x is ")
            {
                return Some(format!(
                    "RemoveAnyCounterX {{ kind: {kind_expr}, filter: &{filter} }}"
                ));
            }
            return self.deny(format!("cost `{token}` without announced X"));
        }
        let Some(count) = count_raw.parse::<u8>().ok().filter(|&c| c > 0) else {
            return self.deny(format!("counter count `{count_raw}`"));
        };
        Some(format!(
            "RemoveAnyCounter {{ count: {count}, kind: {kind_expr}, filter: &{filter} }}"
        ))
    }

    fn sub_counter_cost(&mut self, n: &str, kind: &str, token: &str) -> Option<String> {
        // "Remove a counter from ..." as a cost:
        // - `SubCounter<N|X/KIND>` or `SubCounter<N|X/KIND/CARDNAME[/desc]>`
        //   removes from the source via `RemoveCounterSelf{,X}` (battlefield
        //   only; loyalty costs CR 606.3-4 stay refused).
        // - `SubCounter<N|X/Any/...>` and non-self `SubCounter<N|X/KIND/Filter[/desc][/Battlefield]>`
        //   lower via `RemoveAnyCounter{,X}`, refusing opponent permanents,
        //   non-battlefield zones, zero/overflow counts, and unsupported filters.
        let fields: Vec<&str> = kind.split('/').collect();
        if fields.is_empty() || fields.len() > 4 {
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(zone) = fields.get(3).copied().map(str::trim)
            && !zone.is_empty()
            && zone != "Battlefield"
        {
            return self.deny(format!("cost `{token}` outside the battlefield"));
        }
        let kind_raw = fields[0].trim();
        let spec_opt = fields.get(1).copied().map(str::trim);
        if kind_raw.eq_ignore_ascii_case("any") {
            let spec = spec_opt.unwrap_or("CARDNAME");
            return self.parse_remove_any_counter(n, None, spec, token);
        }
        if let Some(target) = spec_opt
            && !matches!(target, "CARDNAME" | "NICKNAME")
        {
            let Some(kind_expr) = counter_kind(kind_raw) else {
                return self.deny(format!("counter `{kind_raw}`"));
            };
            return self.parse_remove_any_counter(n, Some(&kind_expr), target, token);
        }
        if self.on_a_spell
            || self.in_alternative_cost
            || self.in_graveyard_activation
            || self.in_hand_activation
        {
            return self.deny(format!("cost `{token}` naming itself"));
        }
        if kind_raw == "LOYALTY" {
            return self.deny("a loyalty cost".to_string());
        }
        if n == "X" {
            if !self.has_x || self.svars.get("X").map(String::as_str) != Some("Count$xPaid") {
                return self.deny("counter count `X`".to_string());
            }
            let Some(kind) = counter_kind(kind_raw) else {
                return self.deny(format!("counter `{kind_raw}`"));
            };
            return Some(format!("RemoveCounterSelfX {{ kind: {kind} }}"));
        }
        let Ok(n) = n.parse::<u16>() else {
            return self.deny(format!("counter count `{n}`"));
        };
        let Some(kind) = counter_kind(kind_raw) else {
            return self.deny(format!("counter `{kind_raw}`"));
        };
        Some(format!("RemoveCounterSelf {{ kind: {kind}, n: {n} }}"))
    }

    fn energy_or_exile_top_cost(&self, token: &str) -> Option<String> {
        if let Some(life) = token
            .strip_prefix("PayLife<")
            .and_then(|t| t.strip_suffix('>'))
        {
            if life == "X"
                && self.in_alternative_cost
                && self.has_x
                && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
                && !self.oracle.to_ascii_lowercase().contains("x is ")
            {
                return Some("PayLifeX".to_string());
            }
            if let Ok(n) = life.parse::<u16>() {
                return Some(format!("PayLife({n})"));
            }
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(energy) = token
            .strip_prefix("PayEnergy<")
            .and_then(|t| t.strip_suffix('>'))
        {
            if energy == "X"
                && self.has_x
                && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
                && !self.oracle.to_ascii_lowercase().contains("x is ")
            {
                return Some("PayEnergyX".to_string());
            }
            if let Ok(n) = energy.parse::<u32>()
                && n > 0
            {
                return Some(format!("PayEnergy({n})"));
            }
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(dmg) = token
            .strip_prefix("DamageYou<")
            .and_then(|t| t.strip_suffix('>'))
        {
            if let Ok(n) = dmg.parse::<u16>()
                && n > 0
            {
                return Some(format!("DealDamageToYou({n})"));
            }
            return self.deny(format!("cost `{token}`"));
        }
        if let Some(body) = token
            .strip_prefix("ExileFromTop<")
            .and_then(|t| t.strip_suffix('>'))
        {
            let mut fields = body.splitn(3, '/');
            let Some(n) = fields
                .next()
                .and_then(|s| s.parse::<u16>().ok())
                .filter(|&n| n > 0)
            else {
                return self.deny(format!("cost `{token}`"));
            };
            if !matches!(fields.next(), Some("Card" | "Card.YouOwn")) {
                return self.deny(format!("cost `{token}`"));
            }
            return Some(format!("ExileTopOfLibrary({n})"));
        }
        self.deny(format!("cost `{token}`"))
    }

    /// One cost part the player pays by naming an object.
    ///
    /// Five spellings of one shape — `Sac<1/…>`, `Discard<1/…>`,
    /// `tapXType<1/…>`, `Return<1/…>`, `ExileFromGrave<1/…>` — and they were
    /// worth writing once rather than five times because what differs
    /// between them is two facts: which `CostPart` they are, and whether the
    /// object has to be one the payer controls.
    ///
    /// **One object per part**, which is what `CostPart` carries: 97 of the
    /// corpus's costs sacrifice two, three or X, and paying one of them
    /// would be a discount rather than the cost the card prints.
    ///
    /// "You control" is added where the script does not say it, which is the
    /// one place this reader writes a clause it did not read — and only for
    /// the three that take a permanent. CR 701.21a lets a player sacrifice
    /// only what they control and CR 118.3 says the same of tapping one to
    /// pay, so a filter without it would be the card and
    /// `cost_wizard::options` offering two different menus for one cost. A
    /// **discard** takes none of it: a card in a hand has no controller at
    /// all, and the engine reads that zone by whose hand it is. An exile
    /// from the graveyard is the same case one pile over, and
    /// `ExileFromGrave<1/CARDNAME>` — eternalize's "exile this card from
    /// your graveyard" — is refused by name, because it is paid from a zone
    /// no ability is activated from yet.
    pub(super) fn object_cost_part(
        &mut self,
        kind: &str,
        body: &str,
        token: &str,
    ) -> Option<Vec<String>> {
        if kind == "ExileFromHand" && !self.in_alternative_cost {
            return self.deny("cost `ExileFromHand`".to_string());
        }
        let mut fields = body.splitn(3, '/');
        let (Some(n), Some(spec)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        let x_paid = n == "X"
            && self.has_x
            && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
            && !self.oracle.to_ascii_lowercase().contains("x is ");
        let sac_x = kind == "Sac" && x_paid;
        let sac_all = kind == "Sac"
            && n == "All"
            && spec != "CARDNAME"
            && !self.on_a_spell
            && !self.in_alternative_cost;
        let tap_x = kind == "tapXType" && x_paid && spec != "CARDNAME";
        let discard_x =
            kind == "Discard" && x_paid && !matches!(spec, "CARDNAME" | "Random" | "Hand");
        let exile_grave_x =
            kind == "ExileFromGrave" && x_paid && spec != "CARDNAME" && !spec.contains("Ctrl");
        let tap_n = if kind == "tapXType"
            && spec != "CARDNAME"
            && !tap_x
            && (self.in_alternative_cost || (self.trigger_mode.is_none() && n != "1"))
        {
            match n
                .parse::<u32>()
                .ok()
                .filter(|count| (1..=10).contains(count))
            {
                Some(count) => Some(count),
                None => return self.deny(format!("a cost naming `{n}` objects")),
            }
        } else {
            None
        };
        let is_self_name = matches!(spec, "CARDNAME" | "NICKNAME");
        let multi_parts = (matches!(
            kind,
            "Sac" | "Discard" | "ExileFromGrave" | "Return" | "Exile"
        ) || (self.in_alternative_cost && kind == "ExileFromHand"))
            && matches!(n, "2" | "3")
            && !is_self_name;
        let special = sac_x || sac_all || tap_x || discard_x || exile_grave_x || tap_n.is_some();
        if n != "1" && !special && !multi_parts {
            return self.deny(format!("a cost naming `{n}` objects"));
        }
        let part_count = if multi_parts {
            n.parse::<usize>().unwrap_or(1)
        } else {
            1
        };
        let (variant, controlled) = match kind {
            "Sac" => ("Sacrifice", true),
            "Discard" => ("Discard", false),
            "tapXType" => ("TapOther", true),
            "ExileFromHand" => ("ExileFromHand", false),
            "ExileFromGrave" => ("ExileFromGraveyard", false),
            "Exile" => ("ExilePermanent", true),
            _ => ("ReturnToHand", true),
        };
        if is_self_name {
            // The source pays for itself, which is a part with no filter and
            // no question. `Sac<1/CARDNAME>` never reaches here (it is
            // matched one branch up), so this is the return's own case.
            return Some(match variant {
                "ReturnToHand" => vec!["ReturnSelfToHand".to_string()],
                _ => return self.deny(format!("cost `{token}` naming itself")),
            });
        }
        if variant == "Discard" && spec == "Random" {
            return Some(vec!["DiscardRandom".to_string(); part_count]);
        }
        let spec = normalize_object_cost_spec(spec, variant, controlled);
        let expr = if variant == "ExileFromHand" || discard_x || exile_grave_x {
            self.filter_expr_in_zone(&spec, true)?
        } else {
            self.filter_expr(&spec)?
        };
        let name = self.body.filter_static("COST", &expr);
        Some(if sac_x {
            vec![format!("SacrificeX(&{name})")]
        } else if sac_all {
            vec![format!("SacrificeAll(&{name})")]
        } else if tap_x {
            vec![format!("TapX(&{name})")]
        } else if discard_x {
            vec![format!("DiscardX(&{name})")]
        } else if exile_grave_x {
            vec![format!("ExileFromGraveyardX(&{name})")]
        } else if let Some(count) = tap_n {
            vec![format!("TapN {{ filter: &{name}, count: {count} }}")]
        } else {
            vec![format!("{variant}(&{name})"); part_count]
        })
    }
}

fn normalize_object_cost_spec(spec: &str, variant: &str, controlled: bool) -> String {
    if controlled {
        // Appended per alternative, because a valid-string's commas are
        // `Or` and a clause glued to the end would narrow only the last
        // branch.
        spec.split([',', ';'])
            .map(|alt| {
                if alt.contains("YouCtrl") {
                    alt.to_string()
                } else if alt.contains('.') {
                    format!("{alt}+YouCtrl")
                } else {
                    format!("{alt}.YouCtrl")
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    } else if variant == "ExileFromHand" {
        // `casting::pitchable` already excludes the spell being cast from
        // the hand candidates, so a pitch script's `Other` is redundant.
        spec.split([',', ';'])
            .map(|alt| {
                let mut parts = alt.split('.');
                let base = parts.next().unwrap_or_default().trim();
                let rest: Vec<&str> = parts
                    .flat_map(|s| s.split('+'))
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && !matches!(*a, "Other" | "StrictlyOther"))
                    .collect();
                if rest.is_empty() {
                    base.to_string()
                } else {
                    format!("{base}.{}", rest.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    } else {
        spec.replace(';', ",")
    }
}

fn dynamic_filter_scalar_atom(def: &str) -> Option<String> {
    if let Some(rest) = def
        .strip_prefix("Count$Devotion.")
        .or_else(|| def.strip_prefix("Count$DevotionDual."))
    {
        let mut colors = Vec::new();
        for part in rest.split('.') {
            let color = match part.trim() {
                "White" => "Color::White",
                "Blue" => "Color::Blue",
                "Black" => "Color::Black",
                "Red" => "Color::Red",
                "Green" => "Color::Green",
                _ => return None,
            };
            if !colors.contains(&color) {
                colors.push(color);
            }
        }
        if colors.is_empty() {
            return None;
        }
        return Some(format!(
            "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[{}]) }}",
            colors.join(", ")
        ));
    }
    Some(
        match def {
            "Count$LifeYouGainedThisTurn" => "Amount::LifeGainedThisTurn(PlayerRel::You)",
            "Count$LifeOppsLostThisTurn" => "Amount::OpponentsLifeLostThisTurn",
            "Count$YourLifeTotal" => "Amount::LifeTotal(PlayerRel::You)",
            "Count$CardsInYourHand" | "Count$ValidHand Card.YouOwn" => {
                "Amount::HandSize(PlayerRel::You)"
            }
            "Count$CardsInYourLibrary" | "Count$ValidLibrary Card.YouOwn" => {
                "Amount::LibrarySize(PlayerRel::You)"
            }
            "Count$Domain" => "Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)",
            "Count$Party" => "Amount::PartySize(PlayerRel::You)",
            "Count$YouDrewThisTurn" | "Count$CardsDrawnThisTurn" => {
                "Amount::CardsDrawnThisTurn(PlayerRel::You)"
            }
            "Count$ThisTurnCast_Card.YouCtrl" => "Amount::SpellsCastThisTurn(PlayerRel::You)",
            "Count$ThisTurnCast_Card.OppCtrl" => {
                "Amount::SpellsCastThisTurn(PlayerRel::EachOpponent)"
            }
            "Count$ThisTurnCast_Card" => "Amount::SpellsCastThisTurn(PlayerRel::EachPlayer)",
            "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" => {
                "Amount::CreaturesDiedThisTurn"
            }
            "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" => {
                "Amount::CreaturesEnteredUnderYourControlThisTurn"
            }
            "Count$TimesKicked" | "Count$KickedTimes" => "Amount::TimesKicked",
            "Count$YourPoisonCounters" | "Count$YourCountersPoison" => {
                "Amount::CountersOnYou(CounterKind::Poison)"
            }
            "Count$YourEnergyCounters" | "Count$YourCountersEnergy" => {
                "Amount::CountersOnYou(CounterKind::Energy)"
            }
            "Count$YourRadCounters" | "Count$YourCountersRAD" => {
                "Amount::CountersOnYou(CounterKind::Rad)"
            }
            "Count$DamageYouTookThisTurn" => "Amount::DamageDealtToYouThisTurn",
            "PlayerCountOpponents$Amount" => "Amount::Players(PlayerRel::Opponent)",
            "PlayerCountPlayers$Amount" => "Amount::Players(PlayerRel::EachPlayer)",
            "Count$ValidExile Card.ExiledWithSource" => {
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::LinkedExile }"
            }
            _ => return None,
        }
        .to_string(),
    )
}

fn normalize_spend_valid(raw: &str) -> String {
    let s = raw.trim();
    let s = if s == "Kicked" { "Card.kicked" } else { s };
    if let Some(plus) = s.find('+')
        && !s[..plus].contains('.')
    {
        let mut out = s.to_string();
        out.replace_range(plus..=plus, ".");
        out
    } else {
        s.to_string()
    }
}

/// Strips `scope_atom` (`"YouCtrl"` on player-scoped `Valid`, `"YouOwn"` on
/// player-scoped `ValidGraveyard`) from every comma-alternative, refusing if
/// any other controller/owner qualifier remains.
fn strip_player_valid_scope(valid: &str, scope_atom: &str) -> Option<String> {
    let mut cleaned = Vec::new();
    for alt in valid.split(',') {
        let mut parts = alt
            .split('.')
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let base = parts.next()?;
        let mut kept = Vec::new();
        for atom in parts {
            if atom == scope_atom {
                continue;
            }
            if atom.contains("Ctrl") || atom.contains("Own") {
                return None;
            }
            kept.push(atom);
        }
        if kept.is_empty() {
            cleaned.push(base.to_string());
        } else {
            cleaned.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!cleaned.is_empty()).then(|| cleaned.join(","))
}

/// Rewrites exact graveyard `ThisTurnEntered` / `!ThisTurnEntered` atoms into
/// the graveyard-only marker `PutIntoGraveyardThisTurn` without touching
/// origin-restricted forms like `ThisTurnEnteredFrom_Battlefield`.
fn normalize_graveyard_entered(valid: &str) -> String {
    valid
        .split(',')
        .map(|alt| {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let Some(base) = parts.next() else {
                return alt.to_string();
            };
            let kept: Vec<&str> = parts
                .map(|atom| match atom {
                    "ThisTurnEntered" => "PutIntoGraveyardThisTurn",
                    "!ThisTurnEntered" => "!PutIntoGraveyardThisTurn",
                    other => other,
                })
                .collect();
            if kept.is_empty() {
                base.to_string()
            } else {
                format!("{base}.{}", kept.join("+"))
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Forge shorthand for printed non-hybrid special mana symbols understood by
/// `ManaCost::try_parse`: two-or-color (`2W` -> `{2/W}`), Phyrexian
/// (`WP` -> `{W/P}`), and hybrid Phyrexian (`WUP` -> `{W/U/P}`, distinct colors).
pub(super) fn special_mana_symbol(token: &str) -> Option<String> {
    let is_color = |b: u8| matches!(b, b'W' | b'U' | b'B' | b'R' | b'G');
    match token.as_bytes() {
        [b'2', c] if is_color(*c) => Some(format!("{{2/{}}}", *c as char)),
        [c, b'P'] if is_color(*c) => Some(format!("{{{}/P}}", *c as char)),
        [a, b, b'P'] if is_color(*a) && is_color(*b) && a != b => {
            Some(format!("{{{}/{}/P}}", *a as char, *b as char))
        }
        _ => None,
    }
}

fn normalize_remove_any_counter_spec(spec: &str, on_spell_or_alt: bool) -> Option<String> {
    let mut cleaned_alts = Vec::new();
    for raw_alt in spec.split([',', ';']) {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let mut parts = alt.split('.');
        let base = parts.next()?.trim();
        if base.is_empty()
            || matches!(
                base,
                "CARDNAME" | "NICKNAME" | "Self" | "OriginalHost" | "Enchanted" | "Equipped"
            )
            || base.contains("Ctrl")
            || base.contains("Own")
        {
            return None;
        }
        let mut kept = Vec::new();
        for atom in parts
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty())
        {
            if atom == "YouCtrl" || atom == "inZoneBattlefield" {
                continue;
            }
            if atom.contains("Ctrl")
                || atom.contains("Own")
                || atom.starts_with("inZone")
                || matches!(
                    atom,
                    "suspended"
                        | "OriginalHost"
                        | "Enchanted"
                        | "Equipped"
                        | "EnchantedBy"
                        | "EquippedBy"
                        | "AttachedBy"
                        | "Attached"
                        | "CARDNAME"
                        | "NICKNAME"
                )
            {
                return None;
            }
            if on_spell_or_alt && matches!(atom, "Self" | "Other" | "StrictlyOther") {
                return None;
            }
            kept.push(atom);
        }
        if kept.is_empty() {
            cleaned_alts.push(base.to_string());
        } else {
            cleaned_alts.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!cleaned_alts.is_empty()).then(|| cleaned_alts.join(","))
}

fn has_unscoped_spend_atom(valid: &str) -> bool {
    valid.split(',').any(|alt| {
        alt.split(['.', '+']).any(|atom| {
            let a = atom.trim();
            a.contains("Ctrl")
                || a.contains("Own")
                || a.contains("shares")
                || matches!(a, "Self" | "Other" | "StrictlyOther")
        })
    })
}
