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
        let defined = p.take("Defined");
        let (filter, source_only) = match (defined.as_deref(), target) {
            (None | Some("Self"), None) if !self.on_a_spell => ("&Filter::This", true),
            (None | Some("Targeted" | "ParentTarget" | "ParentTargeted"), Some(t))
                if !t.starts_with("TargetSpec::Player(") =>
            {
                ("&Filter::This", false)
            }
            (Some("Enchanted" | "Equipped"), None) => ("&Filter::AttachedToBySource", false),
            (Some(def), None) if self.animate_triggered_subject(def) => {
                ("&Filter::Reference", false)
            }
            _ => {
                self.note("`Animate` of something other than the source".to_string());
                return None;
            }
        };
        self.animate_for_filter(p, filter, target.is_some(), source_only)
    }

    fn animate_triggered_subject(&self, def: &str) -> bool {
        match def {
            "TriggeredCardLKICopy" | "TriggeredCard" => {
                self.trigger_mode.as_deref() == Some("ChangesZone")
            }
            "TriggeredAttackerLKICopy" | "TriggeredAttacker" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                    && self
                        .block_line
                        .as_ref()
                        .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks))
            }
            "TriggeredBlockerLKICopy" | "TriggeredBlocker" => {
                self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked))
            }
            "TriggeredTargetLKICopy" | "TriggeredTarget" => {
                self.trigger_mode.as_deref() == Some("DamageDone")
            }
            "DelayTriggerRemembered" | "DelayTriggerRememberedLKI" => self.in_delayed,
            _ => false,
        }
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
        let mut out = Vec::new();
        self.animate_types_block(p, filter, duration, &mut out)?;
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

    fn take_true_flag(&mut self, p: &mut Params, key: &str) -> Option<bool> {
        match p.take(key).as_deref() {
            None => Some(false),
            Some("True") => Some(true),
            Some(other) => self.deny(format!("`{key}$ {other}`")),
        }
    }

    fn subtype_belongs_to_card_type(&self, word: &str, ct: &str) -> bool {
        let kinds: &[baylee_core::types::SubtypeKind] = match ct {
            "TypeSet::CREATURE" | "TypeSet::KINDRED" => {
                &[baylee_core::types::SubtypeKind::Creature]
            }
            "TypeSet::ARTIFACT" => &[baylee_core::types::SubtypeKind::Artifact],
            "TypeSet::ENCHANTMENT" => &[baylee_core::types::SubtypeKind::Enchantment],
            "TypeSet::LAND" => &[baylee_core::types::SubtypeKind::Land],
            "TypeSet::PLANESWALKER" => &[baylee_core::types::SubtypeKind::Planeswalker],
            "TypeSet::INSTANT" | "TypeSet::SORCERY" => &[baylee_core::types::SubtypeKind::Spell],
            "TypeSet::BATTLE" => &[baylee_core::types::SubtypeKind::Battle],
            _ => return false,
        };
        kinds
            .iter()
            .any(|&kind| self.cats.const_path_of(kind, word).is_some())
    }

    /// Lower Layer 4 type replacement (`RemoveCardTypes$ True`) for `Animate`
    /// and `Continuous` effects.
    pub(super) fn become_types_modifiers(
        &mut self,
        p: &mut Params,
        raw_types: &str,
        sep: &[char],
    ) -> Option<Vec<String>> {
        // `BecomeTypes` preserves existing supertypes per CR 205.1b.
        let _ = self.take_true_flag(p, "KeepSupertypes")?;
        if p.has("RemoveSupertypes") {
            return self.deny("`RemoveSupertypes$` with `RemoveCardTypes$`".to_string());
        }
        let remove_subtypes = self.take_true_flag(p, "RemoveSubTypes")?;
        let remove_creature = self.take_true_flag(p, "RemoveCreatureTypes")?;
        let remove_artifact = self.take_true_flag(p, "RemoveArtifactTypes")?;
        let remove_enchantment = self.take_true_flag(p, "RemoveEnchantmentTypes")?;
        let remove_land = self.take_true_flag(p, "RemoveLandTypes")?;

        let mut card_types: Vec<&'static str> = Vec::new();
        let mut supertypes: Vec<String> = Vec::new();
        let mut subtypes: Vec<(&str, String)> = Vec::new();
        for word in raw_types
            .split(sep)
            .map(str::trim)
            .filter(|w| !w.is_empty())
        {
            if let Some(ct) = card_type_const(word) {
                if !card_types.contains(&ct) {
                    card_types.push(ct);
                }
            } else if let Some(st) = supertype_const(word) {
                supertypes.push(st);
            } else {
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("type replacement into `{word}`"));
                    return None;
                };
                subtypes.push((word, path));
            }
        }
        if card_types.is_empty() {
            return self.deny("`RemoveCardTypes$` with no card type".to_string());
        }
        if !remove_subtypes && card_types.len() > 1 {
            let covered = card_types.iter().all(|&ct| match ct {
                "TypeSet::CREATURE" => remove_creature,
                "TypeSet::ARTIFACT" => remove_artifact,
                "TypeSet::ENCHANTMENT" => remove_enchantment,
                "TypeSet::LAND" => remove_land,
                _ => false,
            });
            if !covered {
                return self.deny("`RemoveCardTypes` without matching subtype removal".to_string());
            }
        }
        for (word, _) in &subtypes {
            if !card_types
                .iter()
                .any(|&ct| self.subtype_belongs_to_card_type(word, ct))
            {
                return self.deny(format!(
                    "subtype `{word}` without its card type under `RemoveCardTypes$`"
                ));
            }
        }
        let types_expr = card_types
            .into_iter()
            .map(str::to_string)
            .reduce(|acc, ct| format!("{acc}.union({ct})"))?;
        let subtypes_expr = if subtypes.is_empty() {
            "&[]".to_string()
        } else {
            let paths: Vec<&str> = subtypes.iter().map(|(_, p)| p.as_str()).collect();
            format!("&[{}]", paths.join(", "))
        };
        let mut out = vec![format!(
            "Modifier::BecomeTypes {{ types: {types_expr}, subtypes: {subtypes_expr} }}"
        )];
        for st in supertypes {
            out.push(format!("Modifier::AddSupertype({st})"));
        }
        Some(out)
    }

    fn animate_types_block(
        &mut self,
        p: &mut Params,
        filter: &str,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if self.take_true_flag(p, "RemoveCardTypes")? {
            let types = p.take("Types").unwrap_or_default();
            for modifier in self.become_types_modifiers(p, &types, &[',', '&'])? {
                out.push(Self::animate_expr_for(filter, &modifier, duration));
            }
            if !p.has("Keywords")
                && !p.has("Abilities")
                && !p.has("Triggers")
                && !p.has("StaticAbilities")
                && !p.has("DamagedThisWayCantBeRegenerated")
                && self.take_true_flag(p, "RemoveAllAbilities")?
            {
                out.push(Self::animate_expr_for(
                    filter,
                    "Modifier::LoseAllAbilities",
                    duration,
                ));
            }
            return Some(());
        }
        let replace_creature_types = self.take_true_flag(p, "RemoveCreatureTypes")?;
        let set_land_type = self.take_true_flag(p, "RemoveLandTypes")?;
        let types = p.take("Types").unwrap_or_default();
        self.animate_types_for(
            &types,
            (replace_creature_types, set_land_type),
            filter,
            duration,
            out,
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
        Some(())
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
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let super::statics::GrantedActivated::Modifier(modifier) =
                    self.build_grant_activated_modifier(name, "Abilities$", false)?
                else {
                    unreachable!("for_static_this is false");
                };
                out.push(Self::animate_expr_for(filter, &modifier, duration));
            }
        }
        if let Some(raw) = p.take("Triggers") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let modifier = self.build_grant_triggered_modifier(name, "Triggers$")?;
                out.push(Self::animate_expr_for(filter, &modifier, duration));
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
