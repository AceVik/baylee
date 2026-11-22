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
        if p.take("Defined").is_some() {
            return None;
        }
        let controlled_by = match target {
            None => None,
            Some("TargetSpec::Player(PlayerRel::Chosen)") => Some("PlayerRel::Chosen"),
            Some(_) => return None,
        };
        let valid_raw = p.take("ValidCards")?;
        let valid = if controlled_by.is_some() {
            strip_targeted_player_ctrl(&valid_raw)?
        } else {
            valid_raw
        };
        let expr = self.filter_expr(&valid)?;
        let filter = format!("&{}", self.body.filter_static("ANIMATE_ALL", &expr));
        self.animate_for_filter(p, &filter, controlled_by, false, false, true)
    }

    pub(super) fn animate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        if defined.is_none()
            && target.is_some()
            && self.trigger_is_self
            && self.trigger_mode.as_deref() == Some("SpellCast")
            && p.peek("Duration") == Some("Permanent")
            && p.has("Triggers")
        {
            return self.deny(
                "a cast trigger grants an indefinite triggered ability without specifying a subject"
                    .into(),
            );
        }
        if let Some(def) = defined.as_deref() {
            if let Some(valid) = def.strip_prefix("Valid ")
                && target.is_none()
            {
                let expr = self.filter_expr(valid.trim())?;
                let filter = format!("&{}", self.body.filter_static("ANIMATE_ALL", &expr));
                return self.animate_for_filter(p, &filter, None, false, false, false);
            }
            if let Some((prefix, qual)) = def.split_once('.') {
                match (prefix, target) {
                    ("Targeted" | "ParentTarget" | "ParentTargeted", Some(t))
                        if !t.starts_with("TargetSpec::Player(") =>
                    {
                        let cond_filter = self.animate_subject_guard_filter(qual)?;
                        let _ = self.take_true_flag(p, "IfDesc")?;
                        let inner =
                            self.animate_for_filter(p, "&Filter::This", None, true, false, false)?;
                        return Some(vec![format!(
                            "Effect::IfTargetMatches {{ filter: {cond_filter}, then: &[{}] }}",
                            inner.join(", ")
                        )]);
                    }
                    ("Self", None) if !self.on_a_spell => {
                        let cond_filter = self.animate_subject_guard_filter(qual)?;
                        let _ = self.take_true_flag(p, "IfDesc")?;
                        let inner =
                            self.animate_for_filter(p, "&Filter::This", None, false, true, true)?;
                        return Some(vec![format!(
                            "Effect::IfCondition {{ condition: Condition::SourceMatches({cond_filter}), then: &[{}], otherwise: &[] }}",
                            inner.join(", ")
                        )]);
                    }
                    _ => {}
                }
            }
        }
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
        self.animate_for_filter(p, filter, None, target.is_some(), source_only, source_only)
    }

    fn animate_subject_guard_filter(&mut self, qual: &str) -> Option<String> {
        let qual = qual.trim();
        if qual.is_empty() || qual.contains(',') {
            return None;
        }
        let bad = qual.split('+').map(str::trim).any(|atom| {
            matches!(
                atom,
                "Self"
                    | "Other"
                    | "StrictlyOther"
                    | "EnchantedBy"
                    | "EquippedBy"
                    | "AttachedBy"
                    | "Attached"
                    | "IsRemembered"
                    | "ExiledWithSource"
                    | "ExiledWith"
                    | "IsImprinted"
                    | "TopLibrary"
                    | "TargetedPlayerCtrl"
            ) || atom.starts_with("inZone")
        });
        if bad {
            return None;
        }
        let expr = self.filter_expr(&format!("Card.{qual}"))?;
        Some(if super::statics::simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("ANIMATE_CHECK", &expr))
        })
    }

    fn animate_triggered_subject(&self, def: &str) -> bool {
        match def {
            "TriggeredCardLKICopy" | "TriggeredCard" => matches!(
                self.trigger_mode.as_deref(),
                Some(
                    "ChangesZone"
                        | "Taps"
                        | "Untaps"
                        | "TurnFaceUp"
                        | "Transformed"
                        | "Exerted"
                        | "Enlisted"
                )
            ),
            "TriggeredNewCardLKICopy" | "TriggeredNewCard" => {
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

    #[allow(clippy::fn_params_excessive_bools)]
    fn animate_for_filter(
        &mut self,
        p: &mut Params,
        filter: &str,
        controlled_by: Option<&str>,
        has_target: bool,
        source_only: bool,
        allow_general_static: bool,
    ) -> Option<Vec<String>> {
        let _ = self.take_true_flag(p, "IsCurse")?;
        let optional = self.take_true_flag(p, "Optional")?;
        if optional {
            p.take("OptionQuestion");
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("UntilYourNextUpkeep") => "Duration::UntilYourNextUpkeep",
            Some("UntilTheEndOfYourNextUpkeep") => "Duration::UntilEndOfYourNextUpkeep",
            Some("UntilHostLeavesPlay" | "AsLongAsInPlay") => "Duration::WhileSourceOnBattlefield",
            Some("AsLongAsControl") => "Duration::WhileYouControlSource",
            Some("UntilUntaps") => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            Some(other) => {
                self.note(format!("`Animate` lasting `{other}`"));
                return None;
            }
        };
        let mut out = Vec::new();
        self.animate_types_block(p, filter, controlled_by, duration, &mut out)?;
        // Emit `LoseAllAbilities` before granted statics/keywords/abilities/triggers so
        // each resolving instruction gets a strictly later timestamp (CR 613.7).
        if self.take_true_flag(p, "RemoveAllAbilities")? {
            if p.has("RemoveKeywords") || p.has("DamagedThisWayCantBeRegenerated") {
                return self.deny(
                    "`RemoveAllAbilities$` combined with another ability-removal flag".to_string(),
                );
            }
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
                "Modifier::LoseAllAbilities",
                duration,
            ));
        }
        if controlled_by.is_none() {
            self.animate_static_abilities(
                p,
                filter,
                source_only,
                allow_general_static,
                duration,
                &mut out,
            )?;
        }
        self.animate_colors_and_keywords(p, filter, controlled_by, duration, &mut out)?;
        self.animate_pt_block(p, filter, controlled_by, has_target, duration, &mut out)?;
        if out.is_empty() {
            self.note("`Animate` that changes nothing".to_string());
            return None;
        }
        if optional {
            return Some(vec![format!(
                "Effect::MayDo {{ effects: &[{}] }}",
                out.join(", ")
            )]);
        }
        Some(out)
    }

    fn animate_pt_block(
        &mut self,
        p: &mut Params,
        filter: &str,
        controlled_by: Option<&str>,
        has_target: bool,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        // Layer 7b: the printed P/T it takes on. Fixed single-sided P or T maps
        // cleanly to `Modifier::SetPower` / `Modifier::SetToughness`; dynamic
        // two-sided values lock on resolution via `Effect::SetPTFilter` (CR 608.2h).
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                if let (Ok(p_val), Ok(t_val)) =
                    (power.trim().parse::<i16>(), toughness.trim().parse::<i16>())
                {
                    out.push(Self::animate_modifier_expr(
                        filter,
                        controlled_by,
                        &format!("Modifier::SetPT({p_val}, {t_val})"),
                        duration,
                    ));
                } else if controlled_by.is_none()
                    && let (Some(p_amt), Some(t_amt)) = (
                        self.animate_pt_amount(&power, has_target),
                        self.animate_pt_amount(&toughness, has_target),
                    )
                {
                    out.push(format!(
                        "Effect::SetPTFilter {{ filter: {filter}, power: {p_amt}, toughness: {t_amt}, duration: {duration} }}"
                    ));
                } else {
                    return None;
                }
            }
            (Some(power), None) => {
                let Ok(p_val) = power.trim().parse::<i16>() else {
                    self.note("`Animate` setting only one of power and toughness".to_string());
                    return None;
                };
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    &format!("Modifier::SetPower({p_val})"),
                    duration,
                ));
            }
            (None, Some(toughness)) => {
                let Ok(t_val) = toughness.trim().parse::<i16>() else {
                    self.note("`Animate` setting only one of power and toughness".to_string());
                    return None;
                };
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    &format!("Modifier::SetToughness({t_val})"),
                    duration,
                ));
            }
            (None, None) => {}
        }
        Some(())
    }

    fn animate_static_abilities(
        &mut self,
        p: &mut Params,
        filter: &str,
        source_only: bool,
        allow_general_static: bool,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let static_count = p
            .entries
            .iter()
            .filter(|(k, _)| k == "staticAbilities" || k == "StaticAbilities")
            .count();
        if static_count == 0 {
            return Some(());
        }
        if static_count > 1 {
            return self.deny(
                "a static animation grant with multiple references or another subject".into(),
            );
        }
        let saved_p = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(grants) = self.animate_regeneration_grant(p, source_only, duration) {
            out.extend(grants);
            return Some(());
        }
        let regen_unclaimed = self.unclaimed.borrow().clone();
        *p = saved_p;
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics_len);
        if allow_general_static {
            let name = p
                .take("staticAbilities")
                .or_else(|| p.take("StaticAbilities"))?;
            if let Some(modifiers) = self.grant_static_modifiers_for_animate(&name) {
                for modifier in modifiers {
                    out.push(Self::animate_expr_for(filter, &modifier, duration));
                }
                return Some(());
            }
        }
        *self.unclaimed.borrow_mut() = regen_unclaimed;
        None
    }

    fn animate_pt_amount(&mut self, raw: &str, has_target: bool) -> Option<String> {
        self.animate_pt_amount_depth(raw, has_target, 0)
    }

    #[allow(clippy::too_many_lines)]
    fn animate_pt_amount_depth(
        &mut self,
        raw: &str,
        has_target: bool,
        depth: u8,
    ) -> Option<String> {
        if depth > 4 {
            return None;
        }
        let trimmed = raw.trim();
        if let Ok(n) = trimmed.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        if trimmed == "Count$LastDieRoll" {
            return Some("Amount::LastDieRoll".to_string());
        }
        let def = if trimmed.starts_with("Count$") {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.trim().to_string()
        };
        let can_read_x =
            self.has_x || (self.is_etb_self_trigger && self.has_x_mana_cost && !self.in_delayed);
        if let Some((lhs, rhs)) = def.rsplit_once("/Plus.")
            && let Ok(offset) = rhs.trim().parse::<u32>()
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!(
                "Amount::Plus {{ base: &{base}, offset: {offset} }}"
            ));
        }
        if let Some((lhs, rhs)) = def.rsplit_once("/Minus.")
            && let Ok(subtract) = rhs.trim().parse::<u32>()
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!(
                "Amount::SaturatingSub {{ base: &{base}, subtract: {subtract} }}"
            ));
        }
        if let Some((lhs, rhs)) = def.rsplit_once("/Times.")
            && let Ok(factor) = rhs.trim().parse::<u32>()
            && factor >= 1
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!(
                "Amount::Times {{ base: &{base}, factor: {factor} }}"
            ));
        }
        if let Some(lhs) = def.strip_suffix("/Twice")
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"));
        }
        if let Some(lhs) = def.strip_suffix("/HalfDown")
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if let Some(lhs) = def.strip_suffix("/HalfUp")
            && let Some(var) = lhs.trim().strip_prefix("SVar$")
            && let Some(base) = self.animate_pt_amount_depth(var.trim(), has_target, depth + 1)
        {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
        let event_is_attacker = matches!(
            self.trigger_mode.as_deref(),
            Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
        ) || (self.trigger_mode.as_deref()
            == Some("AttackerBlockedByCreature")
            && self
                .block_line
                .as_ref()
                .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks));
        let event_is_blocker = self.trigger_mode.as_deref() == Some("Blocks")
            || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                && self
                    .block_line
                    .as_ref()
                    .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked));
        if let Some(kind_raw) = def.strip_prefix("Count$CardCounters.")
            && let Some(kind) = super::counter_kind(kind_raw.trim())
        {
            return Some(format!("Amount::CountersOnSource({kind})"));
        }
        if let Some(valid_body) = def.strip_prefix("Count$Valid ")
            && let Some((raw_filter, metric)) = valid_body.rsplit_once('$')
        {
            let variant = match metric.trim() {
                "GreatestCardPower" => Some("Amount::GreatestPowerAmong"),
                "GreatestCardManaCost" => Some("Amount::GreatestManaValueAmong"),
                "TotalCardPower" => Some("Amount::TotalPowerAmong"),
                _ => None,
            };
            if let Some(variant) = variant {
                let expr = self.filter_expr(raw_filter.trim())?;
                let filter_static = self.body.filter_static("ANIMATE_PT", &expr);
                return Some(format!("{variant}(&{filter_static})"));
            }
        }
        match def.as_str() {
            "Count$xPaid" if can_read_x => Some("Amount::X".to_string()),
            "Count$xPaid/Twice" | "Count$xPaid/Times.2" if can_read_x => {
                Some("Amount::DoubleX".to_string())
            }
            "Count$LastDieRoll" => Some("Amount::LastDieRoll".to_string()),
            "Count$CardPower" => Some("Amount::SourcePower".to_string()),
            "Count$Party" => Some("Amount::PartySize(PlayerRel::You)".to_string()),
            "Count$CreaturesDiedThisTurn" => Some("Amount::CreaturesDiedThisTurn".to_string()),
            "Count$TotalManaSpent" => Some("Amount::ManaSpentToCast".to_string()),
            "Count$ValidAll Card.IsCommander+YouOwn$GreatestCardManaCost" => {
                Some("Amount::GreatestCommanderManaValue".to_string())
            }
            "Targeted$CardManaCost" if has_target => Some("Amount::TargetCmc".to_string()),
            "Targeted$CardPower" if has_target => Some("Amount::TargetPower".to_string()),
            "Targeted$CardToughness" if has_target => Some("Amount::TargetToughness".to_string()),
            "TriggeredCard$CardPower" | "TriggeredCardLKICopy$CardPower"
                if self.trigger_mode.as_deref() == Some("ChangesZone") =>
            {
                Some("Amount::EventPower".to_string())
            }
            "TriggeredCard$CardToughness" | "TriggeredCardLKICopy$CardToughness"
                if self.trigger_mode.as_deref() == Some("ChangesZone") =>
            {
                Some("Amount::EventLastToughness".to_string())
            }
            "TriggeredCard$CardManaCost" | "TriggeredCardLKICopy$CardManaCost"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZone" | "SpellCast")
                ) =>
            {
                Some("Amount::EventManaValue".to_string())
            }
            "TriggeredAttacker$CardPower" | "TriggeredAttackerLKICopy$CardPower"
                if event_is_attacker =>
            {
                Some("Amount::EventPower".to_string())
            }
            "TriggeredAttacker$CardToughness" | "TriggeredAttackerLKICopy$CardToughness"
                if event_is_attacker =>
            {
                Some("Amount::EventLastToughness".to_string())
            }
            "TriggeredBlocker$CardPower" | "TriggeredBlockerLKICopy$CardPower"
                if event_is_blocker =>
            {
                Some("Amount::EventPower".to_string())
            }
            "TriggeredBlocker$CardToughness" | "TriggeredBlockerLKICopy$CardToughness"
                if event_is_blocker =>
            {
                Some("Amount::EventLastToughness".to_string())
            }
            "TriggeredSpellAbility$CardManaCostLKI" | "TriggeredSpellAbility$CardManaCost"
                if self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                Some("Amount::EventManaValue".to_string())
            }
            "Sacrificed$CardManaCost" => Some("Amount::SacrificedManaValue".to_string()),
            "Sacrificed$CardPower" => Some("Amount::SacrificedPower".to_string()),
            "Sacrificed$CardToughness" => Some("Amount::SacrificedToughness".to_string()),
            "Sacrificed$TotalCardPower" => Some("Amount::SacrificedTotalPower".to_string()),
            "Sacrificed$TotalCardToughness" => Some("Amount::SacrificedTotalToughness".to_string()),
            "Sacrificed$TotalCardManaCost" => Some("Amount::SacrificedTotalManaValue".to_string()),
            _ => None,
        }
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
        controlled_by: Option<&str>,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if self.take_true_flag(p, "RemoveCardTypes")? {
            let types = p.take("Types").unwrap_or_default();
            for modifier in self.become_types_modifiers(p, &types, &[',', '&'])? {
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    &modifier,
                    duration,
                ));
            }
            if self.take_true_flag(p, "AddAllCreatureTypes")? {
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    "Modifier::AllCreatureTypes",
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
            controlled_by,
            duration,
            out,
        )?;
        if self.take_true_flag(p, "AddAllCreatureTypes")? {
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
                "Modifier::AllCreatureTypes",
                duration,
            ));
        }
        if let Some(removed) = p.take("RemoveTypes") {
            for word in removed.split(',') {
                let word = word.trim();
                let Some(supertype) = supertype_const(word) else {
                    return self.deny(format!("`Animate` removing type `{word}`"));
                };
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
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
        controlled_by: Option<&str>,
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 105.3).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            if raw.trim() == "Colorless" && overwrite {
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
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
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
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
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
                &format!("Modifier::AddKeyword({joined})"),
                duration,
            ));
        }
        if let Some(raw) = p.take("HiddenKeywords") {
            let modifier = self.animate_hidden_keywords(&raw)?;
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
                &modifier,
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
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
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
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    &modifier,
                    duration,
                ));
            }
        }
        if let Some(raw) = p.take("Triggers") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let modifier = self.build_grant_triggered_modifier(name, "Triggers$")?;
                out.push(Self::animate_modifier_expr(
                    filter,
                    controlled_by,
                    &modifier,
                    duration,
                ));
            }
        }
        Some(())
    }

    fn animate_hidden_keywords(&self, raw: &str) -> Option<String> {
        let mut bits = Vec::new();
        for clause in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            let bit = match clause {
                "CARDNAME can't block." => "KeywordSet::CANT_BLOCK",
                "CARDNAME can't attack." => "KeywordSet::CANT_ATTACK",
                "CARDNAME can't attack or block." => {
                    "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)"
                }
                _ => {
                    self.note(format!("`Animate` granting hidden keyword `{clause}`"));
                    return None;
                }
            };
            bits.push(bit);
        }
        if bits.is_empty() {
            self.note("`Animate` granting empty hidden keywords".to_string());
            return None;
        }
        let joined = bits
            .iter()
            .skip(1)
            .fold(bits[0].to_string(), |acc, b| format!("{acc}.union({b})"));
        Some(format!("Modifier::AddKeyword({joined})"))
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
        controlled_by: Option<&str>,
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
            out.push(Self::animate_modifier_expr(
                filter,
                controlled_by,
                &modifier,
                duration,
            ));
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

    fn animate_modifier_expr(
        filter: &str,
        controlled_by: Option<&str>,
        modifier: &str,
        duration: &str,
    ) -> String {
        match controlled_by {
            None => Self::animate_expr_for(filter, modifier, duration),
            Some(who) => format!(
                "Effect::ModifyFilter {{ filter: {filter}, controlled_by: Some({who}), modifier: {modifier}, duration: {duration} }}"
            ),
        }
    }
}

/// Strips `TargetedPlayerCtrl` from every comma-separated alternative of a
/// player-targeted `AnimateAll` filter, or accepts a controller-agnostic filter
/// when the `AnimateAll` line itself declares `ValidTgts$ Player`/`Opponent`.
fn strip_targeted_player_ctrl(valid: &str) -> Option<String> {
    let mut saw_targeted = false;
    let mut saw_plain = false;
    let mut out = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Ctrl") || base.contains("Own") {
            return None;
        }
        let mut has_targeted = false;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            if atom == "TargetedPlayerCtrl" {
                has_targeted = true;
            } else if atom.contains("Ctrl") || atom.contains("Own") {
                return None;
            } else {
                kept.push(atom);
            }
        }
        if has_targeted {
            saw_targeted = true;
        } else {
            saw_plain = true;
        }
        if kept.is_empty() {
            out.push(base.trim().to_string());
        } else {
            out.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    if saw_targeted && saw_plain {
        return None;
    }
    (!out.is_empty()).then(|| out.join(","))
}

/// A printed supertype's existing core constant, not a subtype catalog entry.
pub(super) fn supertype_const(word: &str) -> Option<String> {
    baylee_core::types::SupertypeSet::from_word(word)
        // A temporary removal also regains World when it ends. The engine
        // does not track continuous World tenure, so neither direction reads.
        .filter(|supertype| *supertype != baylee_core::types::SupertypeSet::WORLD)
        .map(|_| format!("SupertypeSet::{}", word.to_ascii_uppercase()))
}
