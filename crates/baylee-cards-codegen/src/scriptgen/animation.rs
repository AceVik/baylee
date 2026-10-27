//! Reading multi-layer animation effects.

use super::{Params, Tx, card_type_const, color_const, keyword_const};

impl Tx<'_> {
    /// `Animate`: the manland sentence — "until end of turn, this land
    /// becomes a 4/4 white and blue Elemental creature with flying and
    /// vigilance. It's still a land."
    ///
    /// One printed sentence, four continuous effects, because CR 613.1
    /// applies type, colour, ability and power/toughness in that order and
    /// each is its own layer. "It's still a land" is why the types are
    /// *added* rather than set — an animated Colonnade that stopped being a
    /// land would stop making mana.
    ///
    /// Two objects are read, each the one `Filter::This` binds to: the
    /// source (`Defined$ Self`) on a chain that targets nothing, and the
    /// target on a line that names no `Defined$` — the Laces' "target spell
    /// or permanent becomes red". `Filter::This` binds to the first target
    /// when there is one and to the source when there is not, so a chain
    /// with both would animate the wrong permanent, and is refused.
    ///
    /// `Duration$ Permanent` is "for the rest of the game"
    /// (`Duration::Indefinitely`), `UntilEndOfCombat` is until end of combat;
    /// without one it is until end of turn.
    ///
    /// `RemoveCreatureTypes$ True` is CR 205.1b's "becomes a [creature type]
    /// artifact creature": the first creature type named replaces the ones it
    /// had (`Modifier::ReplaceCreatureTypes`) and every other type is kept.
    ///
    /// `RemoveLandTypes$ True` is "target land becomes a Forest" (Gaea's
    /// Liege): CR 305.7's setting of a land's subtype, `Modifier::SetLandType`,
    /// for the one basic land type named; a second land type, or none, is
    /// refused. `Duration$ UntilHostLeavesPlay` is "until [this] leaves the
    /// battlefield", `Duration::WhileSourceOnBattlefield`.
    /// `AnimateAll`: "All lands become 2/2 creatures until end of turn.
    /// They're still lands." (Natural Affinity).
    pub(super) fn animate_all_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() || p.take("Defined").is_some() {
            return None;
        }
        let valid = p.take("ValidCards")?;
        let expr = self.filter_expr(&valid)?;
        let filter = format!("&{}", self.body.filter_static("ANIMATE_ALL", &expr));
        self.animate_for_filter(p, &filter, false, false)
    }

    pub(super) fn animate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let source_only = match (p.take("Defined").as_deref(), target) {
            (Some("Self"), None) => true,
            (None | Some("Targeted"), Some(_)) => false,
            _ => {
                self.note("`Animate` of something other than the source".to_string());
                return None;
            }
        };
        self.animate_for_filter(p, "&Filter::This", target.is_some(), source_only)
    }

    fn animate_for_filter(
        &mut self,
        p: &mut Params,
        filter: &str,
        has_target: bool,
        source_only: bool,
    ) -> Option<Vec<String>> {
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilHostLeavesPlay") => "Duration::WhileSourceOnBattlefield",
            Some(other) => {
                self.note(format!("`Animate` lasting `{other}`"));
                return None;
            }
        };
        let replace_creature_types = match p.take("RemoveCreatureTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveCreatureTypes$ {other}`")),
        };
        let set_land_type = match p.take("RemoveLandTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveLandTypes$ {other}`")),
        };
        let mut out = Vec::new();
        let types = p.take("Types").unwrap_or_default();
        self.animate_types_for(
            &types,
            (replace_creature_types, set_land_type),
            filter,
            duration,
            &mut out,
        )?;
        if let Some(removed) = p.take("RemoveTypes") {
            for word in removed.split(',') {
                let word = word.trim();
                let Some(supertype) = supertype_const(word) else {
                    return self.deny(format!("`Animate` removing type `{word}`"));
                };
                out.push(Self::animate_expr_for(
                    filter,
                    &format!("Modifier::RemoveSupertype({supertype})"),
                    duration,
                ));
            }
        }
        out.extend(self.animate_regeneration_grant(p, source_only, duration)?);
        self.animate_colors_and_keywords(p, filter, duration, &mut out)?;
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness))
                if power == "X"
                    && toughness == "X"
                    && has_target
                    && self
                        .svars
                        .get("X")
                        .is_some_and(|s| s.trim() == "Targeted$CardManaCost") =>
            {
                out.push(format!(
                    "Effect::SetPTFilter {{ filter: {filter}, power: Amount::TargetCmc, toughness: Amount::TargetCmc, duration: {duration} }}"
                ));
            }
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr_for(
                    filter,
                    &format!("Modifier::SetPT({power}, {toughness})"),
                    duration,
                ));
            }
            (None, None) => {}
            _ => {
                self.note("`Animate` setting only one of power and toughness".to_string());
                return None;
            }
        }
        if out.is_empty() {
            self.note("`Animate` that changes nothing".to_string());
            return None;
        }
        Some(out)
    }

    /// Fixed color changes and keyword grants retain their distinct layers.
    fn animate_colors_and_keywords(
        &mut self,
        p: &mut Params,
        filter: &str,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 105.3).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            if raw.trim() == "Colorless" && overwrite {
                out.push(Self::animate_expr_for(
                    filter,
                    "Modifier::SetColor(ColorSet::EMPTY)",
                    duration,
                ));
            } else {
                let colors: Option<Vec<&str>> =
                    raw.split(',').map(|c| color_const(c.trim())).collect();
                let Some(colors) = colors else {
                    self.note(format!("`Animate` into colours `{raw}`"));
                    return None;
                };
                let which = if overwrite { "SetColor" } else { "AddColor" };
                out.push(Self::animate_expr_for(
                    filter,
                    &format!(
                        "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                        colors.join(", ")
                    ),
                    duration,
                ));
            }
        }
        // Layer 6: keywords it gains.
        if let Some(raw) = p.take("Keywords") {
            let each: Option<Vec<&str>> = raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let Some(each) = each else {
                self.note(format!("`Animate` granting `{raw}`"));
                return None;
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            out.push(Self::animate_expr_for(
                filter,
                &format!("Modifier::AddKeyword({joined})"),
                duration,
            ));
        }
        if let Some(raw) = p.take("RemoveKeywords") {
            let each: Option<Vec<&str>> = raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let Some(each) = each else {
                self.note(format!("`Animate` removing `{raw}`"));
                return None;
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            out.push(Self::animate_expr_for(
                filter,
                &format!("Modifier::RemoveKeyword({joined})"),
                duration,
            ));
        }
        if let Some(raw) = p.take("Abilities") {
            for name in raw.split(',').map(str::trim) {
                let Some(line) = self.svars.get(name).cloned() else {
                    return self.deny(format!("`Abilities$ {name}` naming no `SVar`"));
                };
                if !line.trim_start().starts_with("AB$") {
                    return self.deny("a granted ability that is not an activated one".to_string());
                }
                let Some((_, mut probe)) = Params::parse(&line) else {
                    return self.deny("a granted ability with no `$` in it".to_string());
                };
                let Some(cost) = probe.take("Cost") else {
                    return self.deny("a granted ability with no `Cost$`".to_string());
                };
                let cost = self.cost_expr(&cost)?;
                let stripped: Vec<&str> = line
                    .split(" | ")
                    .filter(|part| !part.starts_with("Cost$"))
                    .collect();
                let mut chain = super::Chain::default();
                self.chain(&stripped.join(" | "), &mut chain)?;
                if chain.target.is_some() {
                    return self.deny("a granted ability with a target".to_string());
                }
                if chain.effects.is_empty() {
                    return self
                        .deny("a granted ability that reads as no effect at all".to_string());
                }
                let mana = chain.could_add_mana && !chain.moves_library;
                out.push(Self::animate_expr_for(
                    filter,
                    &format!(
                        "Modifier::GrantActivated {{ cost: {cost}, effects: &[{}], mana_ability: {mana} }}",
                        chain.effects.join(", ")
                    ),
                    duration,
                ));
            }
        }
        Some(())
    }

    /// Layer 4 of an [`Self::animate_effect`]: the types it becomes. A word
    /// is either a card type or a subtype, and the corpus writes both in one
    /// list. `remove` is (`RemoveCreatureTypes$ True`, `RemoveLandTypes$
    /// True`), each of which needs the one word it replaces with.
    fn animate_types_for(
        &mut self,
        types: &str,
        remove: (bool, bool),
        filter: &str,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let (replace_creature_types, set_land_type) = remove;
        let mut set = false;
        let mut replaced = false;
        for word in types.split(',').filter(|w| !w.trim().is_empty()) {
            let word = word.trim();
            let modifier = if let Some(types) = card_type_const(word) {
                format!("Modifier::AddType({types})")
            } else if let Some(supertype) = supertype_const(word) {
                format!("Modifier::AddSupertype({supertype})")
            } else if set_land_type && let Some(path) = self.basic_land_type(word) {
                if set {
                    return self.deny("`RemoveLandTypes$` naming two land types".to_string());
                }
                set = true;
                format!("Modifier::SetLandType({path})")
            } else {
                if set_land_type
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Land, word)
                        .is_some()
                {
                    return self.deny(format!("`RemoveLandTypes$` into `{word}`"));
                }
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("`Animate` into `{word}`"));
                    return None;
                };
                let creature_type = self
                    .cats
                    .const_path_of(baylee_core::types::SubtypeKind::Creature, word)
                    .is_some();
                if replace_creature_types && creature_type && !replaced {
                    replaced = true;
                    format!("Modifier::ReplaceCreatureTypes({path})")
                } else {
                    format!("Modifier::AddSubtype({path})")
                }
            };
            out.push(Self::animate_expr_for(filter, &modifier, duration));
        }
        if replace_creature_types && !replaced {
            return self.deny("`RemoveCreatureTypes$` naming no creature type".to_string());
        }
        if set_land_type && !set {
            return self.deny("`RemoveLandTypes$` naming no basic land type".to_string());
        }
        Some(())
    }

    /// One layer of an [`Self::animate_effect`], as the `Effect` expression.
    ///
    /// No layer is passed in because none is written out: `Effect::continuous`
    /// derives it from the modifier the way CR 613.1 does, so the emitter
    /// cannot name a layer that disagrees with what it is applying.
    pub(super) fn animate_expr(modifier: &str, duration: &str) -> String {
        Self::animate_expr_for("&Filter::This", modifier, duration)
    }

    pub(super) fn animate_expr_for(filter: &str, modifier: &str, duration: &str) -> String {
        format!("Effect::continuous({filter}, {modifier}, {duration})")
    }
}

/// A printed supertype's existing core constant, not a subtype catalog entry.
pub(super) fn supertype_const(word: &str) -> Option<String> {
    baylee_core::types::SupertypeSet::from_word(word)
        // A temporary removal also regains World when it ends. The engine
        // does not track continuous World tenure, so neither direction reads.
        .filter(|supertype| *supertype != baylee_core::types::SupertypeSet::WORLD)
        .map(|_| format!("SupertypeSet::{}", word.to_ascii_uppercase()))
}
