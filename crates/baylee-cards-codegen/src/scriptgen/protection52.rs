//! Narrow player protection/life-lock readers and Wave 5 static/replacement
//! extensions, retaining strict unknown-key refusal.

use super::saga_ward_convoke::ExtraOutcome;
use super::{Params, Tx, card_type_const};

impl Tx<'_> {
    pub(super) fn everything_protection(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("You")
            || p.take("Duration").as_deref() != Some("UntilYourNextTurn")
            || p.take("KW").as_deref() != Some("Protection from everything")
        {
            return self
                .deny("player protection outside the exact self/until-next-turn shape".into());
        }
        Some(vec!["Effect::continuous(&Filter::Any, Modifier::PlayerProtectionFromEverything { who: PlayerRel::You }, Duration::UntilYourNextTurn)".into()])
    }
    pub(super) fn life_total_lock(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Duration").as_deref() != Some("UntilYourNextTurn") {
            return self.deny("life lock with an unsupported duration".into());
        }
        let name = p.take("StaticAbilities")?;
        let (mode, mut child) = self.svars.get(&name).and_then(|s| Params::parse(s))?;
        child.drop_prose();
        if mode != "CantChangeLife"
            || child.take("ValidPlayer").as_deref() != Some("You")
            || !child.exhausted()
        {
            return self.deny("life lock outside the exact self-player static".into());
        }
        Some(vec!["Effect::continuous(&Filter::Any, Modifier::LifeTotalCantChange { who: PlayerRel::You }, Duration::UntilYourNextTurn)".into()])
    }
    pub(super) fn phase_your_permanents(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("Valid Permanent.YouCtrl") {
            return self
                .deny("phase-out other than the simultaneous controlled permanent set".into());
        }
        Some(vec![
            "Effect::PhaseOutFilter(&Filter::ControlledByYou)".into(),
        ])
    }
    pub(super) fn cast_self_protection_trigger(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if mode != "ChangesZone"
            || p.take("ValidCard").as_deref() != Some("Card.wasCastByYou+Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
        {
            return self.deny("unsupported cast-self entry trigger".into());
        }
        if let Some(origin) = p.take("Origin")
            && origin != "Any"
        {
            return self.deny("cast-self entry from a restricted origin".into());
        }
        if let Some(zone) = p.take("TriggerZones")
            && zone != "Battlefield"
        {
            return self.deny("cast-self trigger outside battlefield".into());
        }
        let name = p.take("Execute")?;
        if !p.exhausted() {
            return self.deny("unclaimed cast-self entry parameter".into());
        }
        let (api, mut child) = self.svars.get(&name).and_then(|s| Params::parse(s))?;
        child.drop_prose();
        if api != "Pump" {
            return self.deny("cast-self entry child is not protection".into());
        }
        let effects = self.everything_protection(&mut child)?;
        if !child.exhausted() {
            return self.deny("unclaimed cast-self protection child parameter".into());
        }
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[{}], condition = Some(Condition::SourceWasCastByYou))",
            effects.join(", ")
        ));
        Some(())
    }

    /// `K:Bands with other:<valid>[:<desc>]` (CR 702.22k).
    pub(super) fn bands_with_other(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let valid = fields.next()?.trim();
        let _desc = fields.next();
        if valid.is_empty() || fields.next().is_some() {
            return self.deny(format!("keyword `Bands with other:{rest}`"));
        }
        let expr = self.filter_expr(valid)?;
        let filter = self.body.filter_static("BANDS", &expr);
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!("Modifier::BandsWithOther(&{filter})"),
        ));
        Some(())
    }

    /// `K:ETBReplacement:Other:<svar>` where `<svar>` is `DB$ NameCard`.
    pub(super) fn etb_name_card(&mut self, svar: &str) -> Option<()> {
        let body = self.svars.get(svar)?.clone();
        let (api, mut p) = Params::parse(&body)?;
        if api != "NameCard" {
            return None;
        }
        p.drop_prose();
        p.take("ValidDescription");
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return self.deny("as-enters `NameCard` for another player".into());
        }
        let valid_cards = p.take("ValidCards");
        if !p.exhausted() {
            return self.deny(format!(
                "as-enters `NameCard` with `{}`",
                p.first_key().unwrap_or_default()
            ));
        }
        let modifier = match valid_cards {
            None => "EnterModifier::ChooseCardName".to_string(),
            Some(vc) => {
                let Some(excluded) = excluded_card_types(&vc) else {
                    return self.deny(format!("as-enters `NameCard` filter `{vc}`"));
                };
                format!("EnterModifier::ChooseCardNameExcept({excluded})")
            }
        };
        self.body.enter_modifiers.push(modifier);
        Some(())
    }

    pub(super) fn has_chosen_opponent(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ChooseOpponent")
    }

    /// `K:ETBReplacement:Other:<svar>` where `<svar>` is `DB$ ChoosePlayer`
    /// choosing an opponent (`EnterModifier::ChooseOpponent`).
    pub(super) fn etb_choose_opponent(&mut self, svar: &str) -> Option<()> {
        let body = self.svars.get(svar)?.clone();
        let (api, mut p) = Params::parse(&body)?;
        if api != "ChoosePlayer" {
            return None;
        }
        p.drop_prose();
        if !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || !matches!(
                p.take("Choices").as_deref(),
                Some("Player.Opponent" | "Opponent")
            )
            || !p.exhausted()
        {
            return self.deny("as-enters `ChoosePlayer` outside choosing an opponent".into());
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::ChooseOpponent".to_string());
        Some(())
    }

    /// Player-targeted `S: Mode$ Continuous` rules: `Modifier::NoMaxHandSize`,
    /// `Modifier::ChosenPlayerMaxHandSize`, `Modifier::ExtraLandDrops`, and
    /// `Modifier::PlayerHexproof`.
    pub(super) fn continuous_player_rule(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || p.has("AffectedZone")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("player-targeted `Continuous` outside battlefield".into());
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("a continuous static with no `Affected$`".into());
        };
        let mut mods = Vec::new();
        if let Some(max_hand) = p.take("SetMaxHandSize") {
            if affected == "You" && max_hand == "Unlimited" {
                mods.push("Modifier::NoMaxHandSize".to_string());
            } else if affected == "Player.Chosen"
                && self.has_chosen_opponent()
                && let Ok(n) = max_hand.trim().parse::<u8>()
            {
                mods.push(format!("Modifier::ChosenPlayerMaxHandSize({n})"));
            } else {
                return self.deny(format!("`Continuous.SetMaxHandSize` `{max_hand}`"));
            }
        }
        if let Some(no_hand) = p.take("NoHandSize") {
            if affected == "You" && no_hand == "True" {
                mods.push("Modifier::NoMaxHandSize".to_string());
            } else {
                return self.deny("unclaimed parameter `Continuous.NoHandSize`".into());
            }
        }
        if let Some(lands) = p.take("AdjustLandPlays") {
            if affected == "You"
                && let Ok(n) = lands.trim().parse::<u8>()
                && n > 0
            {
                mods.push(format!("Modifier::ExtraLandDrops({n})"));
            } else {
                return self.deny(format!("`Continuous.AdjustLandPlays` `{lands}`"));
            }
        }
        if let Some(kw) = p.take("AddKeyword") {
            let who = match affected.as_str() {
                "You" => Some("PlayerRel::You"),
                "Player" => Some("PlayerRel::EachPlayer"),
                "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
                _ => None,
            };
            if kw == "Hexproof"
                && let Some(who) = who
            {
                mods.push(format!("Modifier::PlayerHexproof {{ who: {who} }}"));
            } else {
                return self.deny(format!("player static keyword `{kw}`"));
            }
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() || mods.is_empty() {
            return self.deny("unsupported player-targeted `Continuous` static".into());
        }
        for modifier in mods {
            self.push_static_with_cond("Filter::Any", &modifier, &cond);
        }
        Some(())
    }

    pub(super) fn push_static_with_cond(&mut self, filter: &str, modifier: &str, cond: &str) {
        self.body
            .abilities
            .push(format!("static_ability!({filter}, {modifier}{cond})"));
    }

    /// Reads `Condition$` / `LifeTotal$` / `IsPresent$`, or falls back to
    /// `CheckSVar$` (+ optional `CheckSecondSVar$`) using both `svar_condition`
    /// and `extra_svar_condition`.
    pub(super) fn condition_any(&mut self, p: &mut Params) -> Option<String> {
        let cond = self.condition(p)?;
        if !cond.is_empty() {
            return Some(cond);
        }
        let Some(var) = p.take("CheckSVar") else {
            return Some(String::new());
        };
        let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
        let first = self
            .svar_condition(&var, &cmp)
            .or_else(|| self.extra_svar_condition_clause(&var, &cmp))?;
        if let Some(var2) = p.take("CheckSecondSVar") {
            let cmp2 = p
                .take("SecondSVarCompare")
                .unwrap_or_else(|| "GE1".to_string());
            let second = self
                .svar_condition(&var2, &cmp2)
                .or_else(|| self.extra_svar_condition_clause(&var2, &cmp2))?;
            return Some(format!(
                ", condition = Some(Condition::All(&[{first}, {second}]))"
            ));
        }
        Some(format!(", condition = Some({first})"))
    }

    /// Battlefield-source `AffectedZone$ Library` continuous statics:
    /// `LookAtLibraryTop`, `RevealLibraryTop`, `RevealAllLibraryTops`,
    /// `PlayLandsFromLibraryTop`, `PlayFromLibraryTopForManaValueLife`.
    pub(super) fn continuous_library_top(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Library")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("static ability reaching `AffectedZone$ Library`".into());
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Library`".into());
        };
        let may_look = p.take("MayLookAt");
        let may_play = p.take("MayPlay");
        let alt_cost = p.take("MayPlayAltManaCost");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Library`".into());
        }
        let modifier = match (
            may_look.as_deref(),
            may_play.as_deref(),
            alt_cost.as_deref(),
        ) {
            (Some("You"), None, None) if is_top_library_yours(&affected, "Card") => {
                "Modifier::LookAtLibraryTop"
            }
            (Some("Player"), None, None) if is_top_library_yours(&affected, "Card") => {
                "Modifier::RevealLibraryTop"
            }
            (Some("Player"), None, None) if affected.trim() == "Card.TopLibrary" => {
                "Modifier::RevealAllLibraryTops"
            }
            (None, Some("True"), None) if is_top_library_yours(&affected, "Land") => {
                "Modifier::PlayLandsFromLibraryTop"
            }
            (None, Some("True"), Some("PayLife<ConvertedManaCost>"))
                if is_top_library_yours(&affected, "Card") =>
            {
                "Modifier::PlayFromLibraryTopForManaValueLife"
            }
            _ => return self.deny("static ability reaching `AffectedZone$ Library`".into()),
        };
        self.push_static_with_cond("Filter::Any", modifier, &cond);
        Some(())
    }

    /// Battlefield-source `AffectedZone$ Graveyard | MayPlay$ True` statics:
    /// `PlayLandsFromGraveyard`, `CastPermanentSpellsFromGraveyard`,
    /// `CastSpellsFromGraveyard`, and `GraveyardInstantSorceryOnce`.
    pub(super) fn continuous_graveyard_play(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.take("MayPlay").as_deref() != Some("True")
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        if p.peek("MayPlayLimit") == Some("1")
            && p.peek("ReplaceGraveyard") == Some("Exile")
            && p.peek("Condition") == Some("PlayerTurn")
            && matches!(
                affected.trim(),
                "Instant.YouCtrl,Sorcery.YouCtrl" | "Instant.YouOwn,Sorcery.YouOwn"
            )
        {
            p.take("MayPlayLimit");
            p.take("ReplaceGraveyard");
            p.take("Condition");
            if !p.exhausted() {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
            }
            self.push_static_with_cond("Filter::Any", "Modifier::GraveyardInstantSorceryOnce", "");
            return Some(());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        match affected.trim() {
            "Land.YouOwn" | "Land.YouCtrl" => {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::PlayLandsFromGraveyard",
                    &cond,
                );
                Some(())
            }
            "Card.YouOwn" | "Card.YouCtrl" => {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::PlayLandsFromGraveyard",
                    &cond,
                );
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::CastSpellsFromGraveyard",
                    &cond,
                );
                Some(())
            }
            "Permanent.nonLand+YouOwn"
            | "Permanent.YouOwn+nonLand"
            | "Permanent.nonLand+YouCtrl"
            | "Permanent.YouCtrl+nonLand" => {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::CastPermanentSpellsFromGraveyard",
                    &cond,
                );
                Some(())
            }
            "Permanent.YouOwn" | "Permanent.YouCtrl" => {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::PlayLandsFromGraveyard",
                    &cond,
                );
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::CastPermanentSpellsFromGraveyard",
                    &cond,
                );
                Some(())
            }
            _ => self.deny("static ability reaching `AffectedZone$ Graveyard`".into()),
        }
    }

    /// Battlefield-source `AffectedZone$ Hand | Affected$ Card.YouOwn | MayLookAt$ Player`
    /// (`Modifier::RevealYourHand`).
    pub(super) fn continuous_reveal_your_hand(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Hand")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.take("MayLookAt").as_deref() != Some("Player")
            || !matches!(
                p.take("Affected").as_deref(),
                Some("Card.YouOwn" | "Card.YouCtrl")
            )
        {
            return self.deny("static ability reaching `AffectedZone$ Hand`".into());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Hand`".into());
        }
        self.push_static_with_cond("Filter::Any", "Modifier::RevealYourHand", &cond);
        Some(())
    }

    /// Battlefield-only `S: Mode$ Continuous` handling `RemoveAllAbilities$ True`
    /// (without same-line Layer 6 grants), `AddAllCreatureTypes$ True`,
    /// `AddType$ AllBasicLandType`, `SetColor$ Colorless`, `BecomeType`, and
    /// `ReplaceCreatureTypes`.
    pub(super) fn continuous_wave5_characteristic(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.has("CharacteristicDefining")
        {
            return None;
        }
        let affected = self.resolve_continuous_affected(&mut p)?;
        let filter = self.filter_expr(&affected)?;
        let cond = self.condition_any(&mut p)?;
        let before = self.body.abilities.len();
        let mut out = Vec::new();
        let ok = self.lower_wave5_continuous_clauses(&mut p, &filter, &mut out);
        if ok.is_none() || !p.exhausted() || out.is_empty() {
            self.body.abilities.truncate(before);
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `Continuous.{key}`"));
            }
            return None;
        }
        for mut expr in out {
            if !cond.is_empty()
                && let Some(open) = expr.strip_suffix(')')
            {
                expr = format!("{open}{cond})");
            }
            self.body.abilities.push(expr);
        }
        Some(())
    }

    fn lower_wave5_continuous_clauses(
        &mut self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let remove_all_abilities = match p.take("RemoveAllAbilities").as_deref() {
            None => false,
            Some("True") => {
                // Same-timestamp Layer 6 grants would be wiped by `abilities_lost <= lost`.
                if p.has("AddKeyword")
                    || p.has("AddAbility")
                    || p.has("AddTrigger")
                    || p.has("AddStaticAbility")
                {
                    return self.deny(
                        "continuous `RemoveAllAbilities$ True` paired with a Layer 6 grant".into(),
                    );
                }
                true
            }
            Some(_) => {
                return self.deny("unclaimed parameter `Continuous.RemoveAllAbilities`".into());
            }
        };
        if let Some(all_creatures) = p.take("AddAllCreatureTypes") {
            if all_creatures != "True" {
                return self.deny("unclaimed parameter `Continuous.AddAllCreatureTypes`".into());
            }
            out.push(Self::static_expr(filter, "Modifier::AllCreatureTypes"));
        }
        if p.peek("AddType") == Some("AllBasicLandType") {
            p.take("AddType");
            out.push(Self::static_expr(filter, "Modifier::AllBasicLandTypes"));
        }
        if p.peek("SetColor") == Some("Colorless") {
            p.take("SetColor");
            out.push(Self::static_expr(
                filter,
                "Modifier::SetColor(ColorSet::EMPTY)",
            ));
        }
        self.lower_become_or_replace_types(p, filter, out)?;
        self.lower_bands_with_other_kw(p, filter, out)?;
        self.pt_modifiers(p, filter, out)?;
        self.keyword_modifiers(p, filter, out)?;
        self.type_modifiers(p, filter, out)?;
        self.color_modifiers(p, filter, out)?;
        if let Some(ctrl) = p.take("GainControl") {
            if ctrl != "You" {
                return self.deny(format!("control of a static given to `{ctrl}`"));
            }
            out.push(Self::static_expr(filter, "Modifier::GainControl"));
        }
        if remove_all_abilities {
            out.push(Self::static_expr(filter, "Modifier::LoseAllAbilities"));
        }
        Some(())
    }

    fn lower_become_or_replace_types(
        &self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let remove_card = p.peek("RemoveCardTypes") == Some("True");
        let remove_creature = p.peek("RemoveCreatureTypes") == Some("True");
        let remove_artifact = p.peek("RemoveArtifactTypes") == Some("True");
        if !remove_card && !remove_creature && !remove_artifact {
            return Some(());
        }
        let raw_add = p.take("AddType")?;
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in raw_add.split_whitespace().filter(|&w| w != "&") {
            if let Some(ty) = card_type_const(word) {
                types.push(ty);
            } else {
                subtypes.push(self.cats.const_path(word)?);
            }
        }
        // `Modifier::BecomeType` replaces card types and clears all subtypes
        // to the single named subtype (layers.rs), matching CR 205.1a + 205.3d
        // only when the single resulting card type's own subtypes are also
        // being replaced (pure Creature or pure Artifact).
        if remove_card && subtypes.len() == 1 {
            let creature_become = remove_creature
                && !remove_artifact
                && types.as_slice() == ["TypeSet::CREATURE"]
                && subtypes[0].starts_with("subtypes::creature::");
            let artifact_become = remove_artifact
                && !remove_creature
                && types.as_slice() == ["TypeSet::ARTIFACT"]
                && subtypes[0].starts_with("subtypes::artifact::");
            if creature_become || artifact_become {
                p.take("RemoveCardTypes");
                if remove_creature {
                    p.take("RemoveCreatureTypes");
                }
                if remove_artifact {
                    p.take("RemoveArtifactTypes");
                }
                out.push(Self::static_expr(
                    filter,
                    &format!(
                        "Modifier::BecomeType {{ types: {}, subtype: {} }}",
                        types[0], subtypes[0]
                    ),
                ));
                return Some(());
            }
            return self.deny("`RemoveCardTypes` with unsupported `AddType$`".into());
        }
        if !remove_card && remove_creature && !remove_artifact {
            p.take("RemoveCreatureTypes");
            if subtypes.len() != 1 || !subtypes[0].starts_with("subtypes::creature::") {
                return self.deny("`RemoveCreatureTypes` with unsupported `AddType$`".into());
            }
            if !types.is_empty() {
                let joined = join_const_union(&types);
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::AddType({joined})"),
                ));
            }
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::ReplaceCreatureTypes({})", subtypes[0]),
            ));
            return Some(());
        }
        self.deny("`RemoveCardTypes` without matching subtype removal".into())
    }

    fn lower_bands_with_other_kw(
        &mut self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let Some(raw) = p.peek("AddKeyword") else {
            return Some(());
        };
        if !raw.starts_with("Bands with other:") || raw.contains('&') {
            return Some(());
        }
        let raw = p.take("AddKeyword")?;
        let rest = raw.strip_prefix("Bands with other:")?;
        let mut fields = rest.split(':');
        let valid = fields.next()?.trim();
        let _desc = fields.next();
        if valid.is_empty() || fields.next().is_some() {
            return self.deny(format!("static ability granting keyword `{raw}`"));
        }
        let expr = self.filter_expr(valid)?;
        let bands = self.body.filter_static("BANDS", &expr);
        out.push(Self::static_expr(
            filter,
            &format!("Modifier::BandsWithOther(&{bands})"),
        ));
        Some(())
    }

    fn resolve_continuous_affected(&self, p: &mut Params) -> Option<String> {
        let affected = p.take("Affected");
        let Some(defined) = p.take("AffectedDefined") else {
            return affected
                .or_else(|| self.deny("a continuous static with no `Affected$`".into()));
        };
        let relation = match defined.as_str() {
            "Self" => "Self",
            "Enchanted" => "EnchantedBy",
            "Equipped" => "EquippedBy",
            "AttachedBy Self" => "AttachedBy",
            other => return self.deny(format!("continuous `AffectedDefined$ {other}`")),
        };
        let valid = affected.as_deref().unwrap_or("Card");
        Some(
            valid
                .split(',')
                .map(|alt| {
                    let (base, tail) = alt.trim().split_once('.').unwrap_or((alt.trim(), ""));
                    if tail.is_empty() {
                        format!("{base}.{relation}")
                    } else {
                        format!("{base}.{relation}+{tail}")
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// `S: Mode$ MinMaxBlocker | ValidCard$ ... | Min$ <n>` (`Modifier::MinimumBlockers`).
    /// `Max$` stays refused because `MaxBlockersPerCombat` is global, not per-attacker.
    pub(super) fn min_max_blocker(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || p.has("Max")
        {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        };
        let Some(min_raw) = p.take("Min") else {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        };
        let Ok(min) = min_raw.trim().parse::<u8>() else {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        };
        if min < 2 {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let filter = self.filter_expr(&valid)?;
        self.push_static_with_cond(&filter, &format!("Modifier::MinimumBlockers({min})"), &cond);
        Some(())
    }

    /// `S: Mode$ IgnoreLegendRule | ValidCard$ Permanent.YouCtrl` -> `Modifier::LegendRuleOff`.
    /// Global or subtype-restricted forms stay refused because `LegendRuleOff`
    /// disables the legend rule for all permanents its controller controls.
    pub(super) fn ignore_legend_rule(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(
                p.take("ValidCard").as_deref(),
                Some("Permanent.YouCtrl" | "Card.YouCtrl")
            )
        {
            return self.deny("static ability `S: Mode$ IgnoreLegendRule`".into());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ IgnoreLegendRule`".into());
        }
        self.push_static_with_cond("Filter::Any", "Modifier::LegendRuleOff", &cond);
        Some(())
    }

    /// Global `S: Mode$ AttackRestrict | MaxAttackers$ <n>` and
    /// `S: Mode$ BlockRestrict | MaxBlockers$ <n>` (`MaxAttackersPerCombat` /
    /// `MaxBlockersPerCombat`). Per-defender limits (`ValidDefender$`) stay refused.
    pub(super) fn combat_restrict(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        let (key, variant) = match mode {
            "AttackRestrict" => ("MaxAttackers", "MaxAttackersPerCombat"),
            "BlockRestrict" => ("MaxBlockers", "MaxBlockersPerCombat"),
            _ => return self.deny(format!("static ability `S: Mode$ {mode}`")),
        };
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let Some(raw) = p.take(key) else {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        };
        let Ok(n) = raw.trim().parse::<u8>() else {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        self.push_static_with_cond("Filter::Any", &format!("Modifier::{variant}({n})"), &cond);
        Some(())
    }

    /// `S: Mode$ CantDraw` (`DrawLimitPerTurn { who, limit }`, with `limit: 0`
    /// when `DrawLimit$` is omitted).
    pub(super) fn cant_draw_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ CantDraw`".into());
        }
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("You") => "PlayerRel::You",
            Some(_) => return self.deny("static ability `S: Mode$ CantDraw`".into()),
        };
        let limit: u8 = match p.take("DrawLimit") {
            None => 0,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) => n,
                Err(_) => return self.deny("static ability `S: Mode$ CantDraw`".into()),
            },
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantDraw`".into());
        }
        self.push_static_with_cond(
            "Filter::Any",
            &format!("Modifier::DrawLimitPerTurn {{ who: {who}, limit: {limit} }}"),
            &cond,
        );
        Some(())
    }

    /// `S: Mode$ IgnoreHexproof | ValidEntity$ <permanent_filter>` (all-player
    /// permanent-scoped `Modifier::CanBeTargetedAsThoughNoHexproof`).
    pub(super) fn ignore_hexproof_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        }
        let Some(entity) = p.take("ValidEntity") else {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() || entity.contains("Opponent") || entity.contains("Player") {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        }
        let filter = self.filter_expr(&entity)?;
        self.push_static_with_cond(&filter, "Modifier::CanBeTargetedAsThoughNoHexproof", &cond);
        Some(())
    }

    /// `S: Mode$ ManaConvert` (`ManaIsAnyColor`, `SpendManaAs`,
    /// `CreatureAbilityManaAnyColor`).
    pub(super) fn mana_convert_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ ManaConvert`".into());
        }
        let conv = p.take("ManaConversion")?;
        let player = p.take("ValidPlayer");
        let card = p.take("ValidCard");
        let sa = p.take("ValidSA");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ ManaConvert`".into());
        }
        if player.is_none() && card.is_none() && sa.is_none() && conv.trim() == "AnyType->AnyColor"
        {
            self.push_static_with_cond("Filter::Any", "Modifier::ManaIsAnyColor", &cond);
            return Some(());
        }
        if player.as_deref() == Some("You")
            && card.as_deref() == Some("Creature.YouCtrl+inZoneBattlefield")
            && sa.as_deref() == Some("Activated")
            && conv.trim() == "AnyType->AnyColor"
        {
            self.push_static_with_cond(
                "Filter::Any",
                "Modifier::CreatureAbilityManaAnyColor",
                &cond,
            );
            return Some(());
        }
        if player.as_deref() == Some("You")
            && card.is_none()
            && sa.is_none()
            && let Some((lhs, rhs)) = conv.trim().split_once("->")
            && let (Some(from), Some(to)) =
                (mana_color_const(lhs.trim()), mana_color_const(rhs.trim()))
        {
            self.push_static_with_cond(
                "Filter::Any",
                &format!("Modifier::SpendManaAs {{ from: {from}, to: {to} }}"),
                &cond,
            );
            return Some(());
        }
        self.deny("static ability `S: Mode$ ManaConvert`".into())
    }

    /// Extended `S: Mode$ CantBeActivated`: `ChosenNameCantActivate`,
    /// `CantActivateNonMana`, `CantActivateArtifacts`, and battlefield
    /// `CantActivate` shapes not covered by `statics.rs::cant_be_activated`.
    pub(super) fn cant_be_activated_extra(&mut self, mut p: Params) -> ExtraOutcome {
        let valid_sa = p.peek("ValidSA");
        // Leave plain `ValidSA$ Activated` with no `AffectedZone$` / `Activator$` /
        // `Secondary$` on `statics.rs::cant_be_activated`.
        if valid_sa == Some("Activated")
            && !p.has("AffectedZone")
            && !p.has("Activator")
            && !p.has("Secondary")
        {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.cant_be_activated_wave5(&mut p))
    }

    fn cant_be_activated_wave5(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        let affected_zone = p.take("AffectedZone");
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(affected_zone.as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ CantBeActivated`".into());
        }
        let valid_sa = p.take("ValidSA");
        let activator = p.take("Activator");
        let phases = p.take("Phases");
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("a `CantBeActivated` with no `ValidCard$`".into());
        };
        let mut cond = self.condition_any(p)?;
        if let Some(ph) = phases {
            if ph != "BeginCombat->EndCombat" || !cond.is_empty() {
                return self.deny("static ability `S: Mode$ CantBeActivated`".into());
            }
            cond = ", condition = Some(Condition::DuringCombat)".to_string();
        }
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantBeActivated`".into());
        }
        // `ChosenNameCantActivate` checks sources in all zones (CR 602.2), so
        // require `AffectedZone$` to be omitted and refuse battlefield-only
        // `Card.NamedCard` activations.
        if valid_card.trim() == "Card.NamedCard" {
            if valid_sa.as_deref() == Some("Activated.!ManaAbility")
                && matches!(activator.as_deref(), None | Some("Player"))
                && affected_zone.is_none()
            {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::ChosenNameCantActivate",
                    &cond,
                );
                return Some(());
            }
            return self.deny("static ability `S: Mode$ CantBeActivated`".into());
        }
        // `CantActivateArtifacts` checks `Artifact` + opponent controller
        // (`restrictions.rs`), matching `ValidCard$ Artifact.OppCtrl`.
        if valid_card.trim() == "Artifact.OppCtrl"
            && matches!(valid_sa.as_deref(), None | Some("Activated"))
            && matches!(activator.as_deref(), None | Some("Player"))
        {
            self.push_static_with_cond("Filter::Any", "Modifier::CantActivateArtifacts", &cond);
            return Some(());
        }
        if !matches!(activator.as_deref(), None | Some("Player")) {
            return self.deny("static ability `S: Mode$ CantBeActivated`".into());
        }
        // `CantActivate` / `CantActivateNonMana` evaluate via `effects::applies_to`,
        // which reaches only battlefield permanents unless `Filter::InZone` is
        // present. Refuse unscoped `Card` / `Card.All` without `AffectedZone$ Battlefield`
        // so hand/graveyard abilities (e.g. Cycling) are never silently missed.
        if affected_zone.is_none() && matches!(valid_card.trim(), "Card" | "Card.All") {
            return self
                .deny("all-zone `CantBeActivated` without `AffectedZone$ Battlefield`".into());
        }
        let modifier = match valid_sa.as_deref() {
            None | Some("Activated") => "Modifier::CantActivate",
            Some("Activated.!ManaAbility") => "Modifier::CantActivateNonMana",
            Some(_) => return self.deny("static ability `S: Mode$ CantBeActivated`".into()),
        };
        let filter = self.filter_expr(&valid_card)?;
        self.push_static_with_cond(&filter, modifier, &cond);
        Some(())
    }

    /// Extended `S: Mode$ CantBeCast`: `OpponentsCastAsSorcery`, conditional /
    /// phased / origin-scoped `OpponentsCantCast`, and `PlayersCantCast`.
    /// Always passes `NumLimitEachTurn$` and existing bare forms to `statics.rs`.
    pub(super) fn cant_be_cast_extra(&mut self, p: Params) -> ExtraOutcome {
        if p.has("NumLimitEachTurn") {
            return ExtraOutcome::Pass;
        }
        if p.peek("ValidCard") == Some("Creature")
            && p.peek("Caster") == Some("You")
            && !p.has("Condition")
            && !p.has("IsPresent")
            && !p.has("CheckSVar")
            && !p.has("Origin")
            && !p.has("Phases")
            && !p.has("OnlySorcerySpeed")
        {
            return ExtraOutcome::Pass;
        }
        if p.peek("Caster") == Some("Opponent")
            && p.has("ValidCard")
            && !p.peek("ValidCard").is_some_and(|v| v.contains("NamedCard"))
            && !p.has("Condition")
            && !p.has("IsPresent")
            && !p.has("CheckSVar")
            && !p.has("Origin")
            && !p.has("Phases")
            && !p.has("OnlySorcerySpeed")
            && !p.has("Secondary")
        {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.cant_be_cast_wave5(p))
    }

    fn cant_be_cast_wave5(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.peek("ValidCard") == Some("Card.Self") && p.peek("EffectZone") == Some("All") {
            return self.cant_be_cast_self_permanent(p);
        }
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let caster = p.take("Caster");
        let valid_card = p.take("ValidCard");
        let origin = p.take("Origin");
        let sorcery_only = p.take("OnlySorcerySpeed");
        let phases = p.take("Phases");
        let mut cond = self.condition_any(&mut p)?;
        if let Some(ph) = phases {
            if ph != "BeginCombat->EndCombat" || !cond.is_empty() {
                return self.deny("static ability `S: Mode$ CantBeCast`".into());
            }
            cond = ", condition = Some(Condition::DuringCombat)".to_string();
        }
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        if sorcery_only.as_deref() == Some("True") {
            if matches!(caster.as_deref(), Some("Opponent" | "Player.Opponent"))
                && matches!(valid_card.as_deref(), None | Some("Card"))
                && origin.is_none()
            {
                self.push_static_with_cond(
                    "Filter::Any",
                    "Modifier::OpponentsCastAsSorcery",
                    &cond,
                );
                return Some(());
            }
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let spell_filter = self.cant_be_cast_filter(valid_card.as_deref(), origin.as_deref())?;
        let filter_ref = if spell_filter.starts_with("Filter::") && !spell_filter.contains('(') {
            format!("&{spell_filter}")
        } else {
            format!("&{}", self.body.filter_static("CANT_CAST", &spell_filter))
        };
        let modifier = match caster.as_deref() {
            Some("Opponent" | "Player.Opponent") => {
                format!("Modifier::OpponentsCantCast({filter_ref})")
            }
            None | Some("Player") => format!(
                "Modifier::PlayersCantCast {{ who: PlayerRel::EachPlayer, filter: {filter_ref} }}"
            ),
            Some("You") => {
                format!("Modifier::PlayersCantCast {{ who: PlayerRel::You, filter: {filter_ref} }}")
            }
            Some("Player.EnchantedController") => format!(
                "Modifier::PlayersCantCast {{ who: PlayerRel::ControllerOfAttached, filter: {filter_ref} }}"
            ),
            Some(_) => return self.deny("static ability `S: Mode$ CantBeCast`".into()),
        };
        self.push_static_with_cond("Filter::Any", &modifier, &cond);
        Some(())
    }

    /// Self-spell casting restrictions on permanent cards
    /// (`S: Mode$ CantBeCast | ValidCard$ Card.Self | EffectZone$ All`), emitted
    /// as `spell!(&[], condition = Some(<allowed>))` (Serra Avenger pattern).
    fn cant_be_cast_self_permanent(&mut self, mut p: Params) -> Option<()> {
        if !self.is_permanent_card_pub() {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        p.take("ValidCard");
        p.take("EffectZone");
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let caster = p.take("Caster");
        if caster.as_deref() == Some("Player.Active")
            && p.peek("CheckSVar") == Some("Count$YourTurns")
        {
            p.take("CheckSVar");
            let cmp = p.take("SVarCompare")?;
            let n: u32 = cmp
                .strip_prefix("LE")
                .and_then(|s| s.parse().ok())
                .or_else(|| {
                    cmp.strip_prefix("LT")
                        .and_then(|s| s.parse::<u32>().ok())
                        .and_then(|v| v.checked_sub(1))
                })?;
            if n == 0 || !p.exhausted() {
                return self.deny("static ability `S: Mode$ CantBeCast`".into());
            }
            self.body.abilities.push(format!(
                "spell!(&[], condition = Some(Condition::Not(&Condition::DuringYourFirstTurns({n}))))"
            ));
            return Some(());
        }
        if !matches!(caster.as_deref(), None | Some("You" | "Player")) {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let forbidden_raw = self.condition_any(&mut p)?;
        if forbidden_raw.is_empty() || !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let forbidden = forbidden_raw
            .strip_prefix(", condition = Some(")?
            .strip_suffix(')')?;
        let allowed = negate_condition_expr(forbidden);
        self.body
            .abilities
            .push(format!("spell!(&[], condition = Some({allowed}))"));
        Some(())
    }

    /// `S: Mode$ CantBlockBy` carrying `CheckSVar$` (not handled by `statics.rs::cant_block_by`).
    pub(super) fn cant_block_by_with_check_svar(
        &mut self,
        spec: &str,
        mut p: Params,
    ) -> ExtraOutcome {
        if !p.has("CheckSVar") || p.has("IsPresent") || p.has("Condition") {
            return ExtraOutcome::Pass;
        }
        let Some(var) = p.take("CheckSVar") else {
            return ExtraOutcome::Pass;
        };
        let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
        let Some(cond) = self
            .svar_condition(&var, &cmp)
            .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
        else {
            return ExtraOutcome::Pass;
        };
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| {
                let key = part.split_once('$').map_or("", |(k, _)| k.trim());
                !matches!(key, "CheckSVar" | "SVarCompare")
            })
            .collect();
        let clean = stripped.join(" | ");
        let Some((_, clean_p)) = Params::parse(&clean) else {
            return ExtraOutcome::Pass;
        };
        let before = self.body.abilities.len();
        if self.cant_block_by(clean_p).is_none() || self.body.abilities.len() != before + 1 {
            self.body.abilities.truncate(before);
            return ExtraOutcome::Handled(None);
        }
        let item = self.body.abilities.pop().unwrap_or_default();
        let Some(open) = item.strip_suffix(')') else {
            return ExtraOutcome::Handled(None);
        };
        self.body
            .abilities
            .push(format!("{open}, condition = Some({cond}))"));
        ExtraOutcome::Handled(Some(()))
    }

    /// `S: Mode$ CantAttack,CantBlock,CantBeActivated` with `ValidSA$ Activated`
    /// or `Activated.!ManaAbility` (extending `statics.rs::cant_attack_block_activate`).
    pub(super) fn cant_attack_block_activate_extra(&mut self, mut p: Params) -> ExtraOutcome {
        if !p.has("ValidSA") {
            return ExtraOutcome::Pass;
        }
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return ExtraOutcome::Handled(None);
        }
        let act_mod = match p.take("ValidSA").as_deref() {
            Some("Activated") => "Modifier::CantActivate",
            Some("Activated.!ManaAbility") => "Modifier::CantActivateNonMana",
            _ => {
                return ExtraOutcome::Handled(self.deny(
                    "unclaimed parameter `CantAttack,CantBlock,CantBeActivated.ValidSA`".into(),
                ));
            }
        };
        let Some(valid) = p.take("ValidCard") else {
            return ExtraOutcome::Handled(None);
        };
        let Some(cond) = self.condition_any(&mut p) else {
            return ExtraOutcome::Handled(None);
        };
        if !p.exhausted() {
            return ExtraOutcome::Handled(None);
        }
        let Some(filter) = self.filter_expr(&valid) else {
            return ExtraOutcome::Handled(None);
        };
        self.push_static_with_cond(
            &filter,
            "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))",
            &cond,
        );
        self.push_static_with_cond(&filter, act_mod, &cond);
        ExtraOutcome::Handled(Some(()))
    }

    fn cant_be_cast_filter(
        &mut self,
        valid_card: Option<&str>,
        origin: Option<&str>,
    ) -> Option<String> {
        let base = match valid_card.map(str::trim) {
            None | Some("Card") => None,
            // Lands cannot be cast (CR 305.9), so `Card.NamedCard` and
            // `Card.NamedCard+nonLand` restrict the exact same spell set.
            Some("Card.NamedCard" | "Card.NamedCard+nonLand" | "Card.nonLand+NamedCard") => {
                Some("Filter::NamedBySource".to_string())
            }
            Some(v) if v.contains("Self") || v.contains("NamedCard") => return None,
            Some(v) => Some(self.filter_expr_in_zone(v, true)?),
        };
        let zone = match origin {
            None => None,
            Some(raw) => {
                let mut zones = Vec::new();
                for word in raw.split(',').map(str::trim) {
                    let expr = match word {
                        "Hand" => "Filter::InZone(ZoneRef::Hand)",
                        "Graveyard" => "Filter::InZone(ZoneRef::Graveyard)",
                        "Library" => "Filter::InZone(ZoneRef::Library)",
                        "Exile" => "Filter::InZone(ZoneRef::Exile)",
                        _ => return None,
                    };
                    zones.push(expr);
                }
                if zones.is_empty() {
                    return None;
                }
                Some(if zones.len() == 1 {
                    zones[0].to_string()
                } else {
                    format!("Filter::AnyOf(&[{}])", zones.join(", "))
                })
            }
        };
        match (base, zone) {
            (None, None) => Some("Filter::Any".to_string()),
            (Some(b), None) => Some(b),
            (None, Some(z)) => Some(z),
            (Some(b), Some(z)) => Some(format!("Filter::And(&[{b}, {z}])")),
        }
    }

    /// Extended `S: Mode$ CantAttackUnless` / `CantBlockUnless` beyond the
    /// per-counter case handled by `statics.rs::combat_mana_tax_per_counter`.
    pub(super) fn combat_unless_extra(&mut self, mode: &str, p: Params) -> ExtraOutcome {
        if p.peek("ValidCard") == Some("Card.Self")
            && !p.has("Target")
            && !p.has("Attacker")
            && p.peek("Cost")
                .and_then(|c| self.svars.get(c.trim()))
                .is_some_and(|def| def.starts_with("Count$CardCounters."))
        {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.combat_unless_wave5(mode, p))
    }

    fn combat_unless_wave5(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        };
        let Some(cost) = p.take("Cost") else {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        };
        let target = p.take("Target");
        let attacker = p.take("Attacker");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let filter = self.filter_expr(&valid_card)?;
        match mode {
            "CantAttackUnless" => self.cant_attack_unless_wave5(
                &valid_card,
                &filter,
                cost.trim(),
                target.as_deref(),
                attacker.as_deref(),
                &cond,
            ),
            "CantBlockUnless" => self.cant_block_unless_wave5(
                &filter,
                cost.trim(),
                target.as_deref(),
                attacker.as_deref(),
                &cond,
            ),
            _ => self.deny(format!("static ability `S: Mode$ {mode}`")),
        }
    }

    fn cant_attack_unless_wave5(
        &mut self,
        valid_card: &str,
        filter: &str,
        cost: &str,
        target: Option<&str>,
        attacker: Option<&str>,
        cond: &str,
    ) -> Option<()> {
        if let Some(tgt) = target {
            if !is_you_and_your_planeswalkers(tgt) {
                return self.deny("static ability `S: Mode$ CantAttackUnless`".into());
            }
            if let Ok(mana) = cost.parse::<u32>()
                && mana > 0
            {
                self.push_static_with_cond(
                    filter,
                    &format!("Modifier::AttackTaxMana {{ mana: {mana} }}"),
                    cond,
                );
                return Some(());
            }
            if let Some(life) = parse_pay_life_u16(cost) {
                self.push_static_with_cond(
                    filter,
                    &format!("Modifier::AttackTaxLife {{ life: {life} }}"),
                    cond,
                );
                return Some(());
            }
            return self.deny("static ability `S: Mode$ CantAttackUnless`".into());
        }
        if attacker.is_some() {
            return self.deny("static ability `S: Mode$ CantAttackUnless`".into());
        }
        if let Ok(mana) = cost.parse::<u32>()
            && mana > 0
        {
            self.push_static_with_cond(
                filter,
                &format!("Modifier::CantAttackUnlessPayMana({mana})"),
                cond,
            );
            return Some(());
        }
        if matches!(valid_card.trim(), "Card.Self" | "Creature.Self")
            && let Some(sac) = cost.strip_prefix("Sac<").and_then(|s| s.strip_suffix('>'))
            && let Some((count, sac_filter)) = self.parse_attack_sac(sac)
        {
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::CantAttackUnlessYouSacrifice {{ count: {count}, filter: &{sac_filter} }}"
                ),
                cond,
            );
            return Some(());
        }
        self.deny("static ability `S: Mode$ CantAttackUnless`".into())
    }

    fn cant_block_unless_wave5(
        &mut self,
        filter: &str,
        cost: &str,
        target: Option<&str>,
        attacker: Option<&str>,
        cond: &str,
    ) -> Option<()> {
        if target.is_some() {
            return self.deny("static ability `S: Mode$ CantBlockUnless`".into());
        }
        let Ok(mana) = cost.parse::<u32>() else {
            return self.deny("static ability `S: Mode$ CantBlockUnless`".into());
        };
        if mana == 0 {
            return self.deny("static ability `S: Mode$ CantBlockUnless`".into());
        }
        if let Some(att_raw) = attacker {
            let att_expr = self.filter_expr(att_raw)?;
            let att_name = self.body.filter_static("ATTACKER", &att_expr);
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::CantBlockMatchingUnlessPayMana {{ attacker: &{att_name}, mana: {mana} }}"
                ),
                cond,
            );
            return Some(());
        }
        self.push_static_with_cond(
            filter,
            &format!("Modifier::CantBlockUnlessPayMana({mana})"),
            cond,
        );
        Some(())
    }

    fn parse_attack_sac(&mut self, body: &str) -> Option<(u8, String)> {
        let mut parts = body.split('/');
        let count: u8 = parts.next()?.trim().parse().ok()?;
        let valid = parts.next()?.trim();
        let _desc = parts.next();
        if count == 0 || parts.next().is_some() || valid.is_empty() || valid.contains("CARDNAME") {
            return None;
        }
        let normalized = valid.replace(';', ",");
        let expr = self.filter_expr(&normalized)?;
        Some((count, self.body.filter_static("ATTACK_SAC", &expr)))
    }

    /// Conditional `S: Mode$ CantAttack | ValidCard$ Card.Self | <condition>`.
    /// Bare `ValidCard$ Card.Self` without a condition stays refused (pinned in
    /// `tests/reading.rs`).
    pub(super) fn cant_attack_self_conditional(&mut self, mut p: Params) -> ExtraOutcome {
        if p.peek("ValidCard") == Some("Card.Self+attackedThisTurn") {
            return ExtraOutcome::Handled(
                self.deny("`CantAttack` with `Card.Self+attackedThisTurn`".into()),
            );
        }
        if !matches!(p.peek("ValidCard"), Some("Card.Self" | "Creature.Self"))
            || p.has("UnlessDefender")
            || p.has("Target")
            || (!p.has("IsPresent") && !p.has("CheckSVar") && !p.has("Condition"))
        {
            return ExtraOutcome::Pass;
        }
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return ExtraOutcome::Handled(
                self.deny("`CantAttack` with unsupported `Secondary$`".into()),
            );
        }
        p.take("ValidCard");
        let Some(cond) = self.condition_any(&mut p) else {
            return ExtraOutcome::Handled(None);
        };
        if cond.is_empty() || !p.exhausted() {
            return ExtraOutcome::Handled(
                self.deny("`CantAttack` with no `UnlessDefender$`".into()),
            );
        }
        self.push_static_with_cond(
            "Filter::This",
            "Modifier::AddKeyword(KeywordSet::CANT_ATTACK)",
            &cond,
        );
        ExtraOutcome::Handled(Some(()))
    }

    /// `R: Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Draw`:
    /// `Skip$ True` -> `Modifier::SkipDrawStep { who }`, and optional
    /// `ReplaceWith$ DB$ GainLife` -> `ReplacementRule::MaySkipDrawStepGainLife(n)`.
    pub(super) fn replace_begin_phase_draw(&mut self, mut p: Params) -> ExtraOutcome {
        if p.peek("Phase") != Some("Draw") {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.replace_begin_phase_draw_inner(&mut p))
    }

    fn replace_begin_phase_draw_inner(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Phase").as_deref() != Some("Draw")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("`BeginPhase` of `Draw` outside battlefield".into());
        }
        let who = match p.take("ValidPlayer").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            _ => return self.deny("`BeginPhase` of `Draw` without `ValidPlayer$`".into()),
        };
        if p.take("Skip").as_deref() == Some("True") {
            let cond = self.condition_any(p)?;
            if !p.exhausted() {
                return self.deny("`BeginPhase` of `Draw` with extra clauses".into());
            }
            self.push_static_with_cond(
                "Filter::Any",
                &format!("Modifier::SkipDrawStep {{ who: {who} }}"),
                &cond,
            );
            return Some(());
        }
        if who == "PlayerRel::You"
            && p.take("Optional").as_deref() == Some("True")
            && matches!(p.take("Layer").as_deref(), None | Some("Other"))
            && let Some(with) = p.take("ReplaceWith")
            && p.exhausted()
            && let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s))
        {
            body.drop_prose();
            if api == "GainLife"
                && matches!(body.take("Defined").as_deref(), None | Some("You"))
                && let Some(raw) = body.take("LifeAmount")
                && let Ok(n) = raw.trim().parse::<u16>()
                && n > 0
                && body.exhausted()
            {
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepGainLife({n}))"
                ));
                return Some(());
            }
        }
        self.deny("replacement `R: Event$ BeginPhase`".into())
    }

    /// `R: Event$ BeginTurn` for Time Vault (`ReplacementRule::SkipTurnToUntapSelf`).
    pub(super) fn replace_begin_turn(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ValidPlayer").as_deref() != Some("You")
            || p.take("IsPresent").as_deref() != Some("Card.Self+tapped")
            || p.take("Optional").as_deref() != Some("True")
        {
            return self.deny("replacement `R: Event$ BeginTurn`".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ BeginTurn`".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ BeginTurn`".into());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `R: Event$ BeginTurn`".into());
        };
        body.drop_prose();
        if api != "Untap" || body.take("Defined").as_deref() != Some("Self") || !body.exhausted() {
            return self.deny("replacement `R: Event$ BeginTurn`".into());
        }
        self.body
            .abilities
            .push("AbilityDef::Replacement(ReplacementRule::SkipTurnToUntapSelf)".to_string());
        Some(())
    }

    /// `R: Event$ Draw` replacements backed by `baylee-engine/src/state/draw_offer.rs`:
    /// `DiscardInsteadOfLaterDraws`, `OpponentsExtraDrawsCreateToken`,
    /// `MaySkipDrawStepDraw`, `RevealDrawPutMatchingInGraveyard`,
    /// `RevealDrawOthersMayPayLife`.
    pub(super) fn replace_draw_extra(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") {
            return self.deny("replacement `R: Event$ Draw`".into());
        }
        let valid_player = p.take("ValidPlayer");
        let with = p.take("ReplaceWith")?;
        if p.take("NotFirstCardInDrawStep").as_deref() == Some("True") {
            if valid_player.as_deref() == Some("Player")
                && p.exhausted()
                && self.is_chains_discard_draw_mill(&with)
            {
                self.body.abilities.push(
                    "AbilityDef::Replacement(ReplacementRule::DiscardInsteadOfLaterDraws)"
                        .to_string(),
                );
                return Some(());
            }
            if matches!(
                valid_player.as_deref(),
                Some("Opponent" | "Player.Opponent")
            ) && p.exhausted()
                && let Some(token_const) = self.parse_hullbreacher_token(&with)
            {
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::OpponentsExtraDrawsCreateToken(\
                     &generated_tokens::{token_const}))"
                ));
                return Some(());
            }
            return self.deny("replacement `R: Event$ Draw`".into());
        }
        if valid_player.as_deref() == Some("You")
            && p.take("ActivePhases").as_deref() == Some("Draw")
            && p.take("PlayerTurn").as_deref() == Some("True")
            && p.take("Optional").as_deref() == Some("True")
            && p.exhausted()
            && self.is_island_sanctuary_effect(&with)
        {
            self.body
                .abilities
                .push("AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw)".to_string());
            return Some(());
        }
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".into());
        }
        if valid_player.as_deref() == Some("You")
            && let Some(filter) = self.parse_enduring_renewal_draw(&with)
        {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::RevealDrawPutMatchingInGraveyard(&{filter}))"
            ));
            return Some(());
        }
        if valid_player.as_deref() == Some("Player")
            && let Some(life) = self.parse_zurs_weirding_draw(&with)
        {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::RevealDrawOthersMayPayLife({life}))"
            ));
            return Some(());
        }
        self.deny("replacement `R: Event$ Draw`".into())
    }

    fn is_chains_discard_draw_mill(&self, start: &str) -> bool {
        let Some((api1, mut p1)) = self.svars.get(start).and_then(|s| Params::parse(s)) else {
            return false;
        };
        p1.drop_prose();
        let Some(sub1) = p1.take("SubAbility") else {
            return false;
        };
        if api1 != "Discard"
            || p1.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p1.take("Mandatory").as_deref() != Some("True")
            || p1.take("NumCards").as_deref() != Some("1")
            || p1.take("Mode").as_deref() != Some("TgtChoose")
            || p1.take("RememberDiscarded").as_deref() != Some("True")
            || !p1.exhausted()
        {
            return false;
        }
        let Some((api2, mut p2)) = self.svars.get(&sub1).and_then(|s| Params::parse(s)) else {
            return false;
        };
        p2.drop_prose();
        let Some(sub2) = p2.take("SubAbility") else {
            return false;
        };
        if api2 != "Draw"
            || p2.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p2.take("ConditionDefined").as_deref() != Some("Remembered")
            || p2.take("ConditionPresent").as_deref() != Some("Card")
            || !matches!(p2.take("ConditionCompare").as_deref(), None | Some("GE1"))
            || !matches!(p2.take("NumCards").as_deref(), None | Some("1"))
            || !p2.exhausted()
        {
            return false;
        }
        let Some((api3, mut p3)) = self.svars.get(&sub2).and_then(|s| Params::parse(s)) else {
            return false;
        };
        p3.drop_prose();
        let clean = p3.take("SubAbility");
        if api3 != "Mill"
            || p3.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p3.take("NumCards").as_deref() != Some("1")
            || p3.take("ConditionDefined").as_deref() != Some("Remembered")
            || p3.take("ConditionPresent").as_deref() != Some("Card")
            || p3.take("ConditionCompare").as_deref() != Some("EQ0")
            || !p3.exhausted()
        {
            return false;
        }
        self.is_optional_clear_remembered(clean.as_deref())
    }

    fn parse_hullbreacher_token(&self, start: &str) -> Option<String> {
        let (api, mut p) = self.svars.get(start).and_then(|s| Params::parse(s))?;
        p.drop_prose();
        if api != "Token"
            || !matches!(p.take("TokenOwner").as_deref(), None | Some("You"))
            || !matches!(p.take("TokenAmount").as_deref(), None | Some("1"))
        {
            return None;
        }
        let stem = p.take("TokenScript")?;
        if !p.exhausted() {
            return None;
        }
        let tokens = self.tokens?;
        let body = tokens.body(stem.trim(), self.cats)?;
        Some(body.constant)
    }

    /// `R: Event$ LifeReduced | ActiveZones$ Battlefield | ValidPlayer$ You.lifeGE<n> |
    /// Result$ LT<n> | IsDamage$ True` -> `Modifier::DamageLifeLossFloor` (`Worship`,
    /// `Ali from Cairo`, `Sustaining Spirit`, `Fortune Thief`, `Elderscale Wurm`).
    pub(super) fn replace_life_reduced(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("IsDamage").as_deref() != Some("True")
        {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        }
        let Some(min_player) = p
            .take("ValidPlayer")
            .and_then(|v| {
                v.strip_prefix("You.lifeGE")
                    .and_then(|n| n.parse::<i32>().ok())
            })
            .filter(|&n| n >= 1)
        else {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        };
        let Some(min_result) = p
            .take("Result")
            .and_then(|v| v.strip_prefix("LT").and_then(|n| n.parse::<i32>().ok()))
        else {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        };
        if min_player != min_result {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        };
        let extra_cond = self.condition_any(&mut p)?;
        if !p.exhausted() || !self.is_damage_life_floor_replace(&with, min_player) {
            return self.deny("replacement `R: Event$ LifeReduced`".into());
        }
        let floor_cond =
            format!("Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {min_player})");
        let combined = if let Some(inner) = extra_cond
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            format!(", condition = Some(Condition::All(&[{floor_cond}, {inner}]))")
        } else {
            format!(", condition = Some({floor_cond})")
        };
        self.push_static_with_cond(
            "Filter::This",
            &format!(
                "Modifier::DamageLifeLossFloor {{ players: PlayerRel::You, minimum: {min_player} }}"
            ),
            &combined,
        );
        Some(())
    }

    fn is_damage_life_floor_replace(&self, svar: &str, minimum: i32) -> bool {
        let Some((api, mut rep)) = self.svars.get(svar.trim()).and_then(|s| Params::parse(s))
        else {
            return false;
        };
        rep.drop_prose();
        if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("Amount") {
            return false;
        }
        let Some(val_var) = rep.take("VarValue") else {
            return false;
        };
        if !rep.exhausted() {
            return false;
        }
        let Some(diff_var) = self
            .svars
            .get(val_var.trim())
            .and_then(|s| s.trim().strip_prefix("ReplaceCount$Amount/LimitMax."))
        else {
            return false;
        };
        let expected = format!("Count$YourLifeTotal/Minus.{minimum}");
        self.svars.get(diff_var.trim()).map(|s| s.trim()) == Some(expected.as_str())
    }

    fn is_island_sanctuary_effect(&self, start: &str) -> bool {
        let Some((api, mut p)) = self.svars.get(start).and_then(|s| Params::parse(s)) else {
            return false;
        };
        p.drop_prose();
        if api != "Effect" || p.take("Duration").as_deref() != Some("UntilYourNextTurn") {
            return false;
        }
        let Some(sa) = p.take("StaticAbilities") else {
            return false;
        };
        if !p.exhausted() {
            return false;
        }
        let Some((mode, mut sp)) = self.svars.get(&sa).and_then(|s| Params::parse(s)) else {
            return false;
        };
        sp.drop_prose();
        mode == "CantAttack"
            && sp.take("ValidCard").as_deref()
                == Some("Creature.withoutFlying+!hasKeywordLandwalk:Island")
            && sp.take("Target").as_deref() == Some("You")
            && sp.exhausted()
    }

    fn parse_enduring_renewal_draw(&mut self, start: &str) -> Option<String> {
        let (api1, mut p1) = self.svars.get(start).and_then(|s| Params::parse(s))?;
        p1.drop_prose();
        let sub1 = p1.take("SubAbility")?;
        if api1 != "PeekAndReveal"
            || p1.take("NoPeek").as_deref() != Some("True")
            || p1.take("RememberRevealed").as_deref() != Some("True")
            || !p1.exhausted()
        {
            return None;
        }
        let (api2, mut p2) = self.svars.get(&sub1).and_then(|s| Params::parse(s))?;
        p2.drop_prose();
        let sub2 = p2.take("SubAbility")?;
        let present = p2.take("ConditionPresent")?;
        if api2 != "ChangeZone"
            || p2.take("Defined").as_deref() != Some("Remembered")
            || p2.take("ConditionDefined").as_deref() != Some("Remembered")
            || p2.take("Origin").as_deref() != Some("Library")
            || p2.take("Destination").as_deref() != Some("Graveyard")
            || !p2.exhausted()
        {
            return None;
        }
        let (api3, mut p3) = self.svars.get(&sub2).and_then(|s| Params::parse(s))?;
        p3.drop_prose();
        let clean = p3.take("SubAbility");
        if api3 != "Draw"
            || p3.take("ConditionDefined").as_deref() != Some("Remembered")
            || p3.take("ConditionPresent").as_deref() != Some(present.as_str())
            || p3.take("ConditionCompare").as_deref() != Some("EQ0")
            || !matches!(p3.take("Defined").as_deref(), None | Some("You"))
            || !matches!(p3.take("NumCards").as_deref(), None | Some("1"))
            || !p3.exhausted()
            || !self.is_optional_clear_remembered(clean.as_deref())
        {
            return None;
        }
        let expr = self.filter_expr_in_zone(&present, true)?;
        Some(self.body.filter_static("REVEAL_DRAW", &expr))
    }

    fn parse_zurs_weirding_draw(&self, start: &str) -> Option<u8> {
        let (api1, mut p1) = self.svars.get(start).and_then(|s| Params::parse(s))?;
        p1.drop_prose();
        let sub1 = p1.take("SubAbility")?;
        if api1 != "PeekAndReveal"
            || p1.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p1.take("NoPeek").as_deref() != Some("True")
            || !p1.exhausted()
        {
            return None;
        }
        let (api2, mut p2) = self.svars.get(&sub1).and_then(|s| Params::parse(s))?;
        p2.drop_prose();
        let sub2 = p2.take("SubAbility")?;
        let life = parse_pay_life_u16(p2.take("UnlessCost")?.trim())?;
        let life = u8::try_from(life).ok()?;
        if api2 != "Dig"
            || p2.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p2.take("DestinationZone").as_deref() != Some("Graveyard")
            || p2.take("DigNum").as_deref() != Some("1")
            || p2.take("ChangeNum").as_deref() != Some("All")
            || p2.take("UnlessPayer").as_deref() != Some("NonReplacedPlayer")
            || p2.take("UnlessSwitched").as_deref() != Some("True")
            || p2.take("UnlessResolveSubs").as_deref() != Some("WhenNotPaid")
            || !p2.exhausted()
        {
            return None;
        }
        let (api3, mut p3) = self.svars.get(&sub2).and_then(|s| Params::parse(s))?;
        p3.drop_prose();
        if api3 != "Draw"
            || p3.take("Defined").as_deref() != Some("ReplacedPlayer")
            || p3.take("NumCards").as_deref() != Some("1")
            || !p3.exhausted()
        {
            return None;
        }
        Some(life)
    }

    fn is_optional_clear_remembered(&self, name: Option<&str>) -> bool {
        let Some(name) = name else {
            return true;
        };
        let Some((api, mut p)) = self.svars.get(name).and_then(|s| Params::parse(s)) else {
            return false;
        };
        p.drop_prose();
        api == "Cleanup" && p.take("ClearRemembered").as_deref() == Some("True") && p.exhausted()
    }

    /// `R: Event$ Moved | Discard$ True | EffectOnly$ True | Optional$ True`
    /// (`ReplacementRule::MayDiscardToLibraryTop`, Library of Leng).
    pub(super) fn moved_discard_to_library(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || !matches!(
                p.take("ValidCard").as_deref(),
                Some("Card.YouCtrl" | "Card.YouOwn")
            )
            || p.take("Discard").as_deref() != Some("True")
            || p.take("EffectOnly").as_deref() != Some("True")
            || p.take("Optional").as_deref() != Some("True")
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || body.take("Origin").as_deref() != Some("All")
            || body.take("Destination").as_deref() != Some("Library")
            || body.take("NoShuffle").as_deref() != Some("True")
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
            || !body.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        self.body
            .abilities
            .push("AbilityDef::Replacement(ReplacementRule::MayDiscardToLibraryTop)".to_string());
        Some(())
    }

    /// `R: Event$ Moved` for Firestorm Phoenix
    /// (`ReplacementRule::ReturnSelfToHandRevealedCantPlayUntilOwnersNextTurn`).
    pub(super) fn moved_firestorm_phoenix(&mut self, p: &Params) -> bool {
        if p.peek("ActiveZones") != Some("Battlefield")
            || p.peek("Origin") != Some("Battlefield")
            || p.peek("Destination") != Some("Graveyard")
            || p.peek("ValidCard") != Some("Card.Self")
        {
            return false;
        }
        let Some(with) = p.peek("ReplaceWith") else {
            return false;
        };
        let Some((api, mut body)) = self.svars.get(with).and_then(|s| Params::parse(s)) else {
            return false;
        };
        body.drop_prose();
        let Some(eff_var) = body.take("SubAbility") else {
            return false;
        };
        if api != "ChangeZone"
            || body.take("Origin").as_deref() != Some("Battlefield")
            || body.take("Destination").as_deref() != Some("Hand")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || !body.exhausted()
        {
            return false;
        }
        let Some((e_api, mut ep)) = self.svars.get(&eff_var).and_then(|s| Params::parse(s)) else {
            return false;
        };
        ep.drop_prose();
        let Some(trig_name) = ep.take("Triggers") else {
            return false;
        };
        let Some(statics) = ep.take("StaticAbilities") else {
            return false;
        };
        if e_api != "Effect"
            || ep.take("RememberObjects").as_deref() != Some("ReplacedCard")
            || ep.take("Duration").as_deref() != Some("UntilYourNextTurn")
            || !ep.exhausted()
            || !self.is_firestorm_cleanup_trigger(&trig_name)
        {
            return false;
        }
        let parts: Vec<&str> = statics.split(',').map(str::trim).collect();
        if parts.len() != 2 {
            return false;
        }
        let mut saw_cant_cast = false;
        let mut saw_reveal = false;
        for name in parts {
            let Some((m, mut sp)) = self.svars.get(name).and_then(|s| Params::parse(s)) else {
                return false;
            };
            sp.drop_prose();
            if m == "CantBeCast"
                && sp.take("ValidCard").as_deref() == Some("Card.IsRemembered")
                && sp.take("Caster").as_deref() == Some("You")
                && sp.exhausted()
            {
                saw_cant_cast = true;
            } else if m == "Continuous"
                && sp.take("AffectedZone").as_deref() == Some("Hand")
                && sp.take("Affected").as_deref() == Some("Card.IsRemembered")
                && sp.take("MayLookAt").as_deref() == Some("Player")
                && sp.exhausted()
            {
                saw_reveal = true;
            }
        }
        saw_cant_cast && saw_reveal
    }

    fn is_firestorm_cleanup_trigger(&self, name: &str) -> bool {
        let Some((mode, mut tp)) = self.svars.get(name.trim()).and_then(|s| Params::parse(s))
        else {
            return false;
        };
        tp.drop_prose();
        let Some(exec) = tp.take("Execute") else {
            return false;
        };
        if mode != "ChangesZone"
            || tp.take("Origin").as_deref() != Some("Hand")
            || tp.take("Destination").as_deref() != Some("Any")
            || tp.take("ValidCard").as_deref() != Some("Card.IsRemembered")
            || tp.take("TriggerZones").as_deref() != Some("Command")
            || !matches!(
                tp.take("TriggerController").as_deref(),
                None | Some("TriggeredCardController")
            )
            || !matches!(tp.take("Static").as_deref(), None | Some("True"))
            || !tp.exhausted()
        {
            return false;
        }
        let Some((e_api, mut xp)) = self.svars.get(&exec).and_then(|s| Params::parse(s)) else {
            return false;
        };
        xp.drop_prose();
        e_api == "ChangeZone"
            && xp.take("Origin").as_deref() == Some("Command")
            && xp.take("Destination").as_deref() == Some("Exile")
            && xp.take("Defined").as_deref() == Some("Self")
            && xp.exhausted()
    }
}

fn is_top_library_yours(affected: &str, base: &str) -> bool {
    let Some((head, tail)) = affected.trim().split_once('.') else {
        return false;
    };
    if head != base {
        return false;
    }
    let atoms: Vec<&str> = tail.split('+').map(str::trim).collect();
    atoms.len() == 2
        && atoms.contains(&"TopLibrary")
        && (atoms.contains(&"YouCtrl") || atoms.contains(&"YouOwn"))
}

fn is_you_and_your_planeswalkers(target: &str) -> bool {
    let parts: Vec<&str> = target.split(',').map(str::trim).collect();
    parts.len() == 2 && parts.contains(&"You") && parts.contains(&"Planeswalker.YouCtrl")
}

fn parse_pay_life_u16(raw: &str) -> Option<u16> {
    let inner = raw.strip_prefix("PayLife<")?.strip_suffix('>')?;
    let n: u16 = inner.trim().parse().ok()?;
    (n > 0).then_some(n)
}

fn excluded_card_types(valid_cards: &str) -> Option<String> {
    let tail = valid_cards.trim().strip_prefix("Card.")?;
    let mut types = Vec::new();
    for atom in tail.split('+') {
        let word = atom.trim().strip_prefix("non")?;
        let ty = card_type_const(word)?;
        if !types.contains(&ty) {
            types.push(ty);
        }
    }
    (!types.is_empty()).then(|| join_const_union(&types))
}

fn join_const_union(items: &[&str]) -> String {
    items
        .iter()
        .skip(1)
        .fold(items[0].to_string(), |mut acc, t| {
            acc.push_str(".union(");
            acc.push_str(t);
            acc.push(')');
            acc
        })
}

fn mana_color_const(word: &str) -> Option<&'static str> {
    match word {
        "White" | "W" => Some("ManaColor::White"),
        "Blue" | "U" => Some("ManaColor::Blue"),
        "Black" | "B" => Some("ManaColor::Black"),
        "Red" | "R" => Some("ManaColor::Red"),
        "Green" | "G" => Some("ManaColor::Green"),
        "Colorless" | "C" => Some("ManaColor::Colorless"),
        _ => None,
    }
}

fn negate_condition_expr(cond: &str) -> String {
    if let Some(inner) = cond
        .strip_prefix("Condition::Not(&")
        .and_then(|s| s.strip_suffix(')'))
    {
        return inner.to_string();
    }
    format!("Condition::Not(&{cond})")
}
