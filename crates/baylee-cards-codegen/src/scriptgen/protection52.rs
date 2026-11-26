//! Narrow player protection/life-lock readers and Wave 5 static/replacement
//! extensions, retaining strict unknown-key refusal.

use super::saga_ward_convoke::ExtraOutcome;
use super::statics::StaticEffectZone;
use super::{Chain, Params, Tx, card_type_const};

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
        if mode != "ChangesZone" || p.take("Destination").as_deref() != Some("Battlefield") {
            return self.deny("unsupported cast-self entry trigger".into());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("unsupported cast-self entry trigger".into());
        };
        let Some(mut cond_parts) = self.wave17a_cast_self_etb_clauses(&valid_card) else {
            return self.deny("unsupported cast-self entry trigger".into());
        };
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
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("unclaimed cast-self entry parameter".into());
        }
        let optional_flag = match p.take("Optional").as_deref() {
            None => None,
            Some("Yes" | "True") => Some(true),
            Some("No" | "False") => Some(false),
            Some(_) => return self.deny("unclaimed cast-self entry parameter".into()),
        };
        let may = match (p.take("OptionalDecider").as_deref(), optional_flag) {
            (None, None | Some(false)) => false,
            (None, Some(true)) | (Some("You"), None | Some(true)) => true,
            _ => return self.deny("unclaimed cast-self entry parameter".into()),
        };
        let extra_cond = self.condition_any(&mut p)?;
        let name = p.take("Execute")?;
        if !p.exhausted() {
            return self.deny("unclaimed cast-self entry parameter".into());
        }
        if let Some(inner) = extra_cond
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            cond_parts.push(inner.to_string());
        }
        let cond_expr = if cond_parts.len() == 1 {
            cond_parts.remove(0)
        } else {
            format!("Condition::All(&[{}])", cond_parts.join(", "))
        };
        let body = self.svars.get(&name)?.clone();
        self.emit_cast_self_body(&body, &cond_expr, may)
    }

    fn emit_cast_self_body(&mut self, body: &str, cond_expr: &str, may: bool) -> Option<()> {
        if !may
            && let Some((api, mut child)) = Params::parse(body)
            && api == "Pump"
            && child.peek("KW") == Some("Protection from everything")
        {
            child.drop_prose();
            let effects = self.everything_protection(&mut child)?;
            if !child.exhausted() {
                return self.deny("unclaimed cast-self protection child parameter".into());
            }
            self.body.abilities.push(format!(
                "triggered!(Trigger::ETB, &[{}], condition = Some({cond_expr}))",
                effects.join(", ")
            ));
            return Some(());
        }
        if !body.trim_start().starts_with("DB$") {
            return self.deny("unsupported cast-self entry trigger".into());
        }
        let prev_mode = self.trigger_mode.replace("ChangesZone".to_string());
        let prev_etb = std::mem::replace(&mut self.is_etb_self_trigger, true);
        let prev_self = std::mem::replace(&mut self.trigger_is_self, true);
        let mut chain = Chain::default();
        let ok = self.chain(body, &mut chain);
        self.trigger_mode = prev_mode;
        self.is_etb_self_trigger = prev_etb;
        self.trigger_is_self = prev_self;
        ok?;
        if chain.effects.is_empty() {
            return self.deny("unsupported cast-self entry trigger".into());
        }
        let targets =
            Self::target_req_expr(&chain).map_or_default(|req| format!(", targets = Some({req})"));
        let second = Self::second_target_req_expr(&chain)
            .map_or_default(|req| format!(", second_targets = Some({req})"));
        let joined = chain.effects.join(", ");
        let effects = if may {
            format!("Effect::MayDo {{ effects: &[{joined}] }}")
        } else {
            joined
        };
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[{effects}]{targets}{second}, condition = Some({cond_expr}))"
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

    fn has_chosen_card_name(&self) -> bool {
        self.body.enter_modifiers.iter().any(|m| {
            m == "EnterModifier::ChooseCardName"
                || m.starts_with("EnterModifier::ChooseCardNameExcept(")
        })
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

    /// Player-targeted (and mixed `You,<permanent>`) `S: Mode$ Continuous` rules:
    /// `NoMaxHandSize`, `ChosenPlayerMaxHandSize`, `ExtraLandDrops`,
    /// `PlayerHexproof`, `PlayerProtectionFrom`, `PlayerProtectionFromOpponents`,
    /// `OpponentsCantSearch`, and `SearchTakeover`.
    pub(super) fn continuous_player_rule(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.has("AffectedZone") || !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("player-targeted `Continuous` outside battlefield".into());
        }
        let zone = self.take_static_effect_zone(&mut p, "Continuous")?;
        let Some(affected) = p.take("Affected") else {
            return self.deny("a continuous static with no `Affected$`".into());
        };
        let parts: Vec<&str> = affected
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() > 1 {
            return self.continuous_mixed_you_and_permanents(&parts, zone, p);
        }
        let mut mods: Vec<(&str, String)> = Vec::new();
        if let Some(max_hand) = p.take("SetMaxHandSize") {
            if affected == "You" && max_hand == "Unlimited" {
                mods.push(("Filter::Any", "Modifier::NoMaxHandSize".to_string()));
            } else if affected == "Player.Chosen"
                && self.has_chosen_opponent()
                && let Ok(n) = max_hand.trim().parse::<u8>()
            {
                mods.push((
                    "Filter::Any",
                    format!("Modifier::ChosenPlayerMaxHandSize({n})"),
                ));
            } else {
                return self.deny(format!("`Continuous.SetMaxHandSize` `{max_hand}`"));
            }
        }
        if let Some(no_hand) = p.take("NoHandSize") {
            if affected == "You" && no_hand == "True" {
                mods.push(("Filter::Any", "Modifier::NoMaxHandSize".to_string()));
            } else {
                return self.deny("unclaimed parameter `Continuous.NoHandSize`".into());
            }
        }
        if let Some(lands) = p.take("AdjustLandPlays") {
            if affected == "You"
                && let Ok(n) = lands.trim().parse::<u8>()
                && n > 0
            {
                mods.push(("Filter::Any", format!("Modifier::ExtraLandDrops({n})")));
            } else {
                return self.deny(format!("`Continuous.AdjustLandPlays` `{lands}`"));
            }
        }
        if let Some(ctrl_search) = p.take("ControlOpponentsSearchingLibrary") {
            if matches!(affected.as_str(), "Opponent" | "Player.Opponent") && ctrl_search == "You" {
                mods.push(("Filter::Any", "Modifier::SearchTakeover".to_string()));
            } else {
                return self.deny(
                    "unclaimed parameter `Continuous.ControlOpponentsSearchingLibrary`".into(),
                );
            }
        }
        if let Some(kw) = p.take("AddKeyword") {
            self.push_player_keyword_modifier(&affected, &kw, &mut mods)?;
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() || mods.is_empty() {
            return self.deny("unsupported player-targeted `Continuous` static".into());
        }
        for (filter, modifier) in mods {
            self.body
                .abilities
                .push(Self::format_static_expr(filter, &modifier, zone, &cond));
        }
        Some(())
    }

    fn push_player_keyword_modifier(
        &mut self,
        affected: &str,
        kw: &str,
        mods: &mut Vec<(&'static str, String)>,
    ) -> Option<()> {
        let who = match affected {
            "You" => Some("PlayerRel::You"),
            "Player" => Some("PlayerRel::EachPlayer"),
            "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
            "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                Some("PlayerRel::EnchantedPlayer")
            }
            _ => None,
        };
        if kw == "Hexproof"
            && let Some(who) = who
        {
            mods.push((
                "Filter::Any",
                format!("Modifier::PlayerHexproof {{ who: {who} }}"),
            ));
            return Some(());
        }
        if kw == "CantSearchLibrary" && matches!(affected, "Opponent" | "Player.Opponent") {
            mods.push(("Filter::Any", "Modifier::OpponentsCantSearch".to_string()));
            return Some(());
        }
        if kw == "LimitSearchLibrary" && matches!(affected, "Opponent" | "Player.Opponent") {
            mods.push(("Filter::Any", "Modifier::OpponentsSearchTop(4)".to_string()));
            return Some(());
        }
        if affected == "You" && kw.starts_with("Protection:Player.Opponent") {
            let tail = kw.trim_start_matches("Protection:Player.Opponent");
            if tail.is_empty() || (tail.starts_with(':') && !tail[1..].contains(':')) {
                mods.push((
                    "Filter::This",
                    "Modifier::PlayerProtectionFromOpponents { who: PlayerRel::You }".to_string(),
                ));
                return Some(());
            }
        }
        if let Some(prot_who) = match affected {
            "You" => Some("PlayerRel::You"),
            "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                Some("PlayerRel::EnchantedPlayer")
            }
            _ => None,
        } && let Some(from_ref) = self.resolve_player_protection_filter(kw)
        {
            mods.push((
                "Filter::This",
                format!("Modifier::PlayerProtectionFrom {{ who: {prot_who}, from: &{from_ref} }}"),
            ));
            return Some(());
        }
        self.deny(format!("player static keyword `{kw}`"))
    }

    fn continuous_mixed_you_and_permanents(
        &mut self,
        parts: &[&str],
        zone: StaticEffectZone,
        mut p: Params,
    ) -> Option<()> {
        let mut saw_you = false;
        let mut perm_parts = Vec::new();
        for &part in parts {
            if part == "You" {
                saw_you = true;
            } else if matches!(
                part,
                "Player" | "Opponent" | "Player.Opponent" | "Player.Chosen"
            ) {
                return self.deny("unsupported mixed player-targeted `Continuous` static".into());
            } else {
                perm_parts.push(part);
            }
        }
        if !saw_you || perm_parts.is_empty() {
            return self.deny("unsupported mixed player-targeted `Continuous` static".into());
        }
        let Some(kw) = p.take("AddKeyword") else {
            return self.deny("unsupported mixed player-targeted `Continuous` static".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("unsupported mixed player-targeted `Continuous` static".into());
        }
        let perm_filter = self.filter_expr(&perm_parts.join(","))?;
        if kw == "Hexproof" {
            self.body.abilities.push(Self::format_static_expr(
                "Filter::Any",
                "Modifier::PlayerHexproof { who: PlayerRel::You }",
                zone,
                &cond,
            ));
            self.body.abilities.push(Self::format_static_expr(
                &perm_filter,
                "Modifier::AddKeyword(KeywordSet::HEXPROOF)",
                zone,
                &cond,
            ));
            return Some(());
        }
        if let Some(from_ref) = self.resolve_player_protection_filter(&kw) {
            self.body.abilities.push(Self::format_static_expr(
                "Filter::This",
                &format!(
                    "Modifier::PlayerProtectionFrom {{ who: PlayerRel::You, from: &{from_ref} }}"
                ),
                zone,
                &cond,
            ));
            self.body.abilities.push(Self::format_static_expr(
                &perm_filter,
                &format!("Modifier::ProtectionFrom(&{from_ref})"),
                zone,
                &cond,
            ));
            return Some(());
        }
        self.deny(format!("player static keyword `{kw}`"))
    }

    fn resolve_player_protection_filter(&mut self, kw: &str) -> Option<String> {
        let trimmed = kw.trim();
        if let Some(rest) = trimmed.strip_prefix("Protection:") {
            let mut fields = rest.split(':');
            let quality = fields.next()?.trim();
            let _prose = fields.next();
            if fields.next().is_some() {
                return None;
            }
            if quality == "ChosenName" {
                return self
                    .has_chosen_card_name()
                    .then(|| "Filter::NamedBySource".to_string());
            }
            if quality == "ChosenType" {
                if self.has_chosen_card_type() {
                    return Some("Filter::OfSourceChosenCardType".to_string());
                }
                if self.has_chosen_creature_type() {
                    return Some("Filter::MatchesChosenTypeOfSource".to_string());
                }
                return None;
            }
            let expr = self.filter_expr(quality)?;
            return Some(self.body.filter_static("PROT", &expr));
        }
        if trimmed.starts_with("Protection from ") {
            let expr = self.protection_filter(trimmed)?;
            return Some(self.body.filter_static("PROT", &expr));
        }
        None
    }

    pub(super) fn push_static_with_cond(&mut self, filter: &str, modifier: &str, cond: &str) {
        self.body
            .abilities
            .push(format!("static_ability!({filter}, {modifier}{cond})"));
    }

    /// Reads `Condition$` / `LifeTotal$` / `IsPresent$`, or falls back to
    /// `CheckSVar$` (+ optional `CheckSecondSVar$`) using both `svar_condition`
    /// and `extra_svar_condition`. Supports `Condition$` combined with
    /// `IsPresent$` and/or `CheckSVar$` on the same static line.
    pub(super) fn take_ability_word_flags(p: &mut Params, clauses: &mut Vec<String>) -> Option<()> {
        for (key, cond) in [
            ("Revolt", "Condition::Revolt"),
            ("Hellbent", "Condition::HandSizeAtMost(0)"),
            ("Threshold", "Condition::GraveyardCountAtLeast(7)"),
            ("Delirium", "Condition::GraveyardTypesAtLeast(4)"),
            (
                "Metalcraft",
                "Condition::ControlCount(&Filter::ARTIFACT, 3)",
            ),
            ("FatefulHour", "Condition::LifeAtMost(5)"),
            ("Blessing", "Condition::CitysBlessing"),
        ] {
            if let Some(val) = p.take(key) {
                if val.trim() != "True" {
                    return None;
                }
                clauses.push(cond.to_string());
            }
        }
        Some(())
    }

    pub(super) fn condition_any(&mut self, p: &mut Params) -> Option<String> {
        let mut clauses: Vec<String> = Vec::new();
        Self::take_ability_word_flags(p, &mut clauses)?;
        if let Some(cond_raw) = p.peek("Condition")
            && p.has("IsPresent")
            && let Some(named) = Self::named_condition(cond_raw)
        {
            p.take("Condition");
            clauses.push(named.to_string());
        }
        if let Some(state_cond) = self.take_self_state_present(p).ok()? {
            clauses.push(state_cond);
        }
        let base = self.condition(p)?;
        if let Some(existing) = base
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(existing.to_string());
        }
        if let Some(var) = p.take("CheckSVar") {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            let first = self
                .svar_condition(&var, &cmp)
                .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                .or_else(|| self.promised_gift_svar_condition(&var, &cmp))?;
            clauses.push(first);
            if let Some(var2) = p.take("CheckSecondSVar") {
                let cmp2 = p
                    .take("SecondSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let second = self
                    .svar_condition(&var2, &cmp2)
                    .or_else(|| self.extra_svar_condition_clause(&var2, &cmp2))
                    .or_else(|| self.promised_gift_svar_condition(&var2, &cmp2))?;
                clauses.push(second);
            }
        }
        match clauses.len() {
            0 => Some(String::new()),
            1 => Some(format!(", condition = Some({})", clauses.remove(0))),
            _ => Some(format!(
                ", condition = Some(Condition::All(&[{}]))",
                clauses.join(", ")
            )),
        }
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

    /// Self-cast-from-graveyard static (`EffectZone$ Graveyard | AffectedZone$ Graveyard |
    /// Affected$ Card.Self | MayPlay$ True`) -> `Modifier::CastSpellsFromGraveyard`
    /// with `zone = ZoneRef::Graveyard` and `Filter::This`.
    pub(super) fn continuous_graveyard_self_play(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || p.take("EffectZone").as_deref() != Some("Graveyard")
            || p.take("Affected").as_deref() != Some("Card.Self")
            || p.take("MayPlay").as_deref() != Some("True")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || self.type_line.split_whitespace().any(|w| w == "Land")
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CastSpellsFromGraveyard, \
             zone = ZoneRef::Graveyard{cond})"
        ));
        Some(())
    }

    /// Graveyard keyword-grant statics (`AffectedZone$ Graveyard | AddKeyword$ ...`):
    /// - bare `Flashback` (cost equals mana cost) -> `Modifier::GrantsFlashback`
    /// - `Escape:CardManaCost ExileFromGrave<N/Card.Other/other>` -> `Modifier::GrantsEscape(N)`
    pub(super) fn continuous_graveyard_keyword(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let zone_arg = match p.take("EffectZone").as_deref() {
            None | Some("Battlefield") => "",
            Some("Command") => ", zone = ZoneRef::Command",
            Some("Battlefield,Command" | "Command,Battlefield") => {
                ", zone = ZoneRef::BattlefieldOrCommand"
            }
            Some(_) => {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
            }
        };
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        let Some(kw) = p.take("AddKeyword") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        let Some((mut modifier, needs_graveyard_zone)) =
            Self::wave16b_graveyard_grant_modifier(&kw, true)
        else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        if modifier == "Modifier::GrantsBeamMeUpManaCost { cant_return_source: false }"
            && self.has_cant_be_beamed_up_for_current_rule()
        {
            modifier = "Modifier::GrantsBeamMeUpManaCost { cant_return_source: true }".to_string();
        }
        let Ok(phase_cond) = super::zone_grants::has_phases_cond(&mut p) else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        };
        let mut cond = self.condition_any(&mut p)?;
        if let Some(pc) = phase_cond {
            cond = match cond
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                Some(existing) => {
                    format!(", condition = Some(Condition::All(&[{pc}, {existing}]))")
                }
                None => format!(", condition = Some({pc})"),
            };
        }
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        }
        let base_filter = self.filter_expr_in_zone(&affected, true)?;
        let filter = if needs_graveyard_zone {
            conjoin_filter_expr(&base_filter, "Filter::InZone(ZoneRef::Graveyard)")
        } else {
            base_filter
        };
        let mut expr = Self::static_expr(&filter, &modifier);
        if (!zone_arg.is_empty() || !cond.is_empty())
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{zone_arg}{cond})");
        }
        self.body.abilities.push(expr);
        Some(())
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

    /// `S: Mode$ MinMaxBlocker | ValidCard$ ... | Min$ <n> | Max$ <m>`:
    /// per-attacker minimum (`Modifier::MinimumBlockers`, CR 509.1b) and/or
    /// maximum (`Modifier::MaxBlockers`, CR 509.1b) blocker limits.
    pub(super) fn min_max_blocker(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        };
        let min = match p.take("Min") {
            None => None,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n >= 2 => Some(n),
                _ => return self.deny("static ability `S: Mode$ MinMaxBlocker`".into()),
            },
        };
        let max = match p.take("Max") {
            None => None,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n >= 1 => Some(n),
                _ => return self.deny("static ability `S: Mode$ MinMaxBlocker`".into()),
            },
        };
        if min.is_none() && max.is_none() {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ MinMaxBlocker`".into());
        }
        let filter = self.filter_expr(&valid)?;
        if let Some(n) = min {
            self.push_static_with_cond(&filter, &format!("Modifier::MinimumBlockers({n})"), &cond);
        }
        if let Some(n) = max {
            self.push_static_with_cond(&filter, &format!("Modifier::MaxBlockers({n})"), &cond);
        }
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
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
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

    /// `S: Mode$ IgnoreHexproof | ValidEntity$ ...`:
    /// - Without `Activator$`, all-player permanent-scoped
    ///   `Modifier::CanBeTargetedAsThoughNoHexproof` (*Nowhere to Run*).
    /// - With `Activator$ You`, `Modifier::IgnoreOpponentsHexproofMatching`
    ///   (*Glaring Spotlight*, *Kaya, Bane of the Dead*).
    pub(super) fn ignore_hexproof_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        }
        let activator = p.take("Activator");
        let Some(entity) = p.take("ValidEntity") else {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
        }
        match activator.as_deref() {
            None => {
                if entity.contains("Opponent") || entity.contains("Player") {
                    return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
                }
                let filter = self.filter_expr(&entity)?;
                self.push_static_with_cond(
                    &filter,
                    "Modifier::CanBeTargetedAsThoughNoHexproof",
                    &cond,
                );
                Some(())
            }
            Some("You") => {
                let mut players = false;
                let mut perm_parts = Vec::new();
                for part in entity.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    if part == "Opponent" {
                        if players {
                            return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
                        }
                        players = true;
                        continue;
                    }
                    if part.contains("Player") || part.contains("Opponent") {
                        return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
                    }
                    let stripped = part
                        .strip_suffix(".OppCtrl")
                        .map(str::to_string)
                        .or_else(|| part.strip_suffix("+OppCtrl").map(str::to_string))
                        .or_else(|| {
                            part.contains(".OppCtrl+")
                                .then(|| part.replacen(".OppCtrl+", ".", 1))
                        });
                    let Some(perm_spec) = stripped else {
                        return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
                    };
                    perm_parts.push(perm_spec);
                }
                if perm_parts.is_empty() {
                    return self.deny("static ability `S: Mode$ IgnoreHexproof`".into());
                }
                let joined = perm_parts.join(",");
                let perm_expr = self.filter_expr(&joined)?;
                let perm_ref = if perm_expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else if perm_expr == "Filter::CREATURE" {
                    "&Filter::CREATURE".to_string()
                } else {
                    format!("&{}", self.body.filter_static("IGNORE_HEX", &perm_expr))
                };
                self.push_static_with_cond(
                    "Filter::Any",
                    &format!(
                        "Modifier::IgnoreOpponentsHexproofMatching {{ permanent_filter: {perm_ref}, players: {players} }}"
                    ),
                    &cond,
                );
                Some(())
            }
            Some(_) => self.deny("static ability `S: Mode$ IgnoreHexproof`".into()),
        }
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

    /// Extended `S: Mode$ CantBeCast`: filtered & player-scoped spell-cast
    /// limits (`MatchingSpellCastLimitPerTurn` / `SpellCastLimitPerTurn`),
    /// `OpponentsCastAsSorcery`, conditional / phased / origin-scoped
    /// `OpponentsCantCast`, and `PlayersCantCast`.
    pub(super) fn cant_be_cast_extra(&mut self, p: Params) -> ExtraOutcome {
        if p.has("NumLimitEachTurn") {
            return self.cant_be_cast_limit(p);
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

    /// Per-turn spell-cast limits (`NumLimitEachTurn$`): handles filtered
    /// limits (`Modifier::MatchingSpellCastLimitPerTurn`, e.g. Deafening
    /// Silence, Ethersworn Canonist, Phyrexian Censor) and player-scoped
    /// quotas (Curse of Exhaustion, Fires of Invention, You, Command emblems),
    /// while leaving `archon_kismet.rs`'s exact tested shapes on `Pass`.
    fn cant_be_cast_limit(&mut self, mut p: Params) -> ExtraOutcome {
        let valid_peek = p.peek("ValidCard");
        let caster_peek = p.peek("Caster");
        let has_extra_clauses = p.has("Condition")
            || p.has("IsPresent")
            || p.has("CheckSVar")
            || p.has("EffectZone")
            || p.has("Secondary");
        if valid_peek == Some("Card") && caster_peek == Some("Player") && !has_extra_clauses {
            return ExtraOutcome::Pass;
        }
        if valid_peek == Some("Card")
            && caster_peek == Some("Opponent")
            && matches!(p.peek("EffectZone"), None | Some("Battlefield"))
        {
            return ExtraOutcome::Pass;
        }
        if caster_peek == Some("Player")
            && valid_peek.is_some_and(|v| v != "Card" && !v.starts_with("Card."))
        {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.cant_be_cast_limit_inner(&mut p))
    }

    fn cant_be_cast_limit_inner(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        };
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let Some(limit) = p
            .take("NumLimitEachTurn")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|&n| n >= 1)
        else {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        };
        let who = match p.take("Caster").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::Opponent",
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
            Some("Player.EnchantedController") => "PlayerRel::ControllerOfAttached",
            Some(_) => return self.deny("static ability `S: Mode$ CantBeCast`".into()),
        };
        let valid_card = p.take("ValidCard");
        let cond = self.condition_any(p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantBeCast`".into());
        }
        let modifier = match valid_card.as_deref() {
            None | Some("Card") => {
                format!("Modifier::SpellCastLimitPerTurn {{ who: {who}, limit: {limit} }}")
            }
            Some(valid) => {
                let expr = self.filter_expr(valid)?;
                let filter_ref = if expr.starts_with("Filter::") && !expr.contains('(') {
                    format!("&{expr}")
                } else {
                    format!("&{}", self.body.filter_static("CANT_CAST", &expr))
                };
                format!(
                    "Modifier::MatchingSpellCastLimitPerTurn {{ who: {who}, filter: {filter_ref}, limit: {limit} }}"
                )
            }
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
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
            Some("You.!Active") => {
                cond = combine_static_cond(&cond, "Condition::Not(&Condition::YourTurn)");
                format!("Modifier::PlayersCantCast {{ who: PlayerRel::You, filter: {filter_ref} }}")
            }
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => format!(
                "Modifier::PlayersCantCast {{ who: PlayerRel::EnchantedPlayer, filter: {filter_ref} }}"
            ),
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
            && p.peek("CheckSVar").is_some_and(|v| {
                v == "Count$YourTurns"
                    || self.svars.get(v.trim()).map(|s| s.trim()) == Some("Count$YourTurns")
            })
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
                    format!("Filter::Or(&[{}])", zones.join(", "))
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
        if let Some(sac) = cost.strip_prefix("Sac<").and_then(|s| s.strip_suffix('>'))
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
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
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
    /// (`ReplacementRule::MayDiscardToLibraryTop`, Library of Leng; and with
    /// `ValidCause$ SpellAbility.OppCtrl` + reveal,
    /// `ReplacementRule::MayRevealOpponentDiscardToLibraryTop`, Nephalia Academy).
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
        let opp_cause = match p.take("ValidCause").as_deref() {
            None => false,
            Some("SpellAbility.OppCtrl") => true,
            Some(_) => return self.deny("replacement `Moved` this rule cannot read".into()),
        };
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
        if opp_cause {
            let Some(sub_name) = body.take("SubAbility") else {
                return self.deny("replacement `Moved` this rule cannot read".into());
            };
            if api != "Reveal"
                || !matches!(body.take("Defined").as_deref(), None | Some("You"))
                || body.take("RevealDefined").as_deref() != Some("ReplacedCard")
                || !body.exhausted()
            {
                return self.deny("replacement `Moved` this rule cannot read".into());
            }
            let Some((sub_api, mut sub)) = self
                .svars
                .get(sub_name.trim())
                .and_then(|s| Params::parse(s))
            else {
                return self.deny("replacement `Moved` this rule cannot read".into());
            };
            sub.drop_prose();
            if sub_api != "ChangeZone"
                || sub.take("Defined").as_deref() != Some("ReplacedCard")
                || sub.take("Origin").as_deref() != Some("All")
                || sub.take("Destination").as_deref() != Some("Library")
                || sub.take("NoShuffle").as_deref() != Some("True")
                || !matches!(sub.take("Hidden").as_deref(), None | Some("True"))
                || !sub.exhausted()
            {
                return self.deny("replacement `Moved` this rule cannot read".into());
            }
            self.body.abilities.push(
                "AbilityDef::Replacement(ReplacementRule::MayRevealOpponentDiscardToLibraryTop)"
                    .to_string(),
            );
            return Some(());
        }
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

    /// `R: Event$ Moved | ActiveZones$ Battlefield | Destination$ Battlefield |
    /// ValidCard$ Land.OppCtrl+ControllerControlsAtLeastAsManyLand`
    /// (`ReplacementRule::LandEquilibrium`).
    pub(super) fn moved_land_equilibrium(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ValidCard").as_deref()
                != Some("Land.OppCtrl+ControllerControlsAtLeastAsManyLand")
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((api, mut mv)) = self.svars.get(with.trim()).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        mv.drop_prose();
        let Some(sac_name) = mv.take("SubAbility") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if api != "ChangeZone"
            || mv.take("Origin").as_deref() != Some("All")
            || mv.take("Destination").as_deref() != Some("Battlefield")
            || mv.take("Defined").as_deref() != Some("ReplacedCard")
            || !mv.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((sac_api, mut sac)) = self
            .svars
            .get(sac_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        sac.drop_prose();
        if sac_api != "Sacrifice"
            || sac.take("SacValid").as_deref() != Some("Land")
            || sac.take("Defined").as_deref() != Some("ReplacedCardController")
            || !sac.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        self.body
            .abilities
            .push("AbilityDef::Replacement(ReplacementRule::LandEquilibrium)".to_string());
        Some(())
    }

    /// `R: Event$ Explore | ActiveZones$ Battlefield | ValidExplorer$ Creature.YouCtrl`
    /// (`ReplacementRule::ExploreScry1First`, Twists and Turns).
    pub(super) fn replace_explore(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ValidExplorer").as_deref() != Some("Creature.YouCtrl")
        {
            return self.deny("replacement `R: Event$ Explore`".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ Explore`".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ Explore`".into());
        }
        let Some((api, mut scry)) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ Explore`".into());
        };
        scry.drop_prose();
        let Some(sub_name) = scry.take("SubAbility") else {
            return self.deny("replacement `R: Event$ Explore`".into());
        };
        if api != "Scry"
            || !matches!(scry.take("Defined").as_deref(), None | Some("You"))
            || !matches!(scry.take("ScryNum").as_deref(), None | Some("1"))
            || !scry.exhausted()
        {
            return self.deny("replacement `R: Event$ Explore`".into());
        }
        let Some((sub_api, mut exp)) = self
            .svars
            .get(sub_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ Explore`".into());
        };
        exp.drop_prose();
        if sub_api != "Explore"
            || exp.take("Defined").as_deref() != Some("ReplacedCard")
            || !exp.exhausted()
        {
            return self.deny("replacement `R: Event$ Explore`".into());
        }
        self.body
            .abilities
            .push("AbilityDef::Replacement(ReplacementRule::ExploreScry1First)".to_string());
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

    /// Extended `S: Mode$ CantTarget`: battlefield targeting prohibitions beyond
    /// `wall52.rs`'s bare `Card.Self` + `SourceCanOnlyTarget$/ValidSource$` forms
    /// (`Modifier::CantBeTargetedBy` and `Modifier::CantBeTargetedByAbilitiesFrom`).
    pub(super) fn cant_target_extra(&mut self, p: Params) -> ExtraOutcome {
        if p.peek("ValidTarget") == Some("Card.Self")
            && !p.has("ValidSA")
            && !p.has("Activator")
            && !p.has("Secondary")
            && !p.has("Condition")
            && !p.has("IsPresent")
            && !p.has("CheckSVar")
            && !p.has("EffectZone")
            && !p.has("AffectedZone")
            && (p.has("SourceCanOnlyTarget") || p.has("ValidSource"))
        {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.cant_target_wave7(p))
    }

    fn cant_target_wave7(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.has("SourceCanOnlyTarget")
        {
            return self.deny("static ability `S: Mode$ CantTarget`".into());
        }
        let affected_zone = p.take("AffectedZone");
        let zone_filter = match affected_zone.as_deref() {
            None | Some("Battlefield") => None,
            Some("Graveyard") => Some("Filter::InZone(ZoneRef::Graveyard)"),
            Some("Battlefield,Graveyard" | "Graveyard,Battlefield") => Some(
                "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::Graveyard)])",
            ),
            Some(_) => return self.deny("static ability `S: Mode$ CantTarget`".into()),
        };
        let valid_target = match (p.take("ValidTarget"), affected_zone.as_deref()) {
            (Some(vt), _) => vt,
            (None, Some("Graveyard")) => "Card".to_string(),
            (None, _) => return self.deny("static ability `S: Mode$ CantTarget`".into()),
        };
        let valid_source = p.take("ValidSource");
        let valid_sa = p.take("ValidSA");
        let activator = p.take("Activator");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantTarget`".into());
        }
        let (stack_only, abilities_only) = match valid_sa.as_deref() {
            None => {
                let lower = self.oracle.to_ascii_lowercase();
                let abilities_from_oracle = lower.contains("target of abilities")
                    && !lower.contains("spells or abilities")
                    && !lower.contains("spells and abilities");
                (false, abilities_from_oracle)
            }
            Some("Spell,Activated,Triggered") => (false, false),
            Some("Spell") => (true, false),
            Some("Activated,Triggered" | "Triggered,Activated") => (false, true),
            Some(_) => return self.deny("static ability `S: Mode$ CantTarget`".into()),
        };
        let base_target = if zone_filter.is_some() {
            self.filter_expr_in_zone(&valid_target, true)?
        } else {
            self.filter_expr(&valid_target)?
        };
        let target_filter = zone_filter.map_or(base_target.clone(), |zf| {
            conjoin_filter_expr(&base_target, zf)
        });
        let src_ref =
            self.cant_target_source_ref(stack_only, valid_source.as_deref(), activator.as_deref())?;
        let modifier = if abilities_only {
            format!("Modifier::CantBeTargetedByAbilitiesFrom(&{src_ref})")
        } else {
            format!("Modifier::CantBeTargetedBy(&{src_ref})")
        };
        self.push_static_with_cond(&target_filter, &modifier, &cond);
        Some(())
    }

    fn cant_target_source_ref(
        &mut self,
        stack_only: bool,
        valid_source: Option<&str>,
        activator: Option<&str>,
    ) -> Option<String> {
        let mut src_parts = Vec::new();
        if stack_only {
            src_parts.push("Filter::InZone(ZoneRef::Stack)".to_string());
        }
        if let Some(src) = valid_source {
            let expr = self.filter_expr(src)?;
            if expr != "Filter::Any" {
                src_parts.push(expr);
            }
        }
        match activator {
            None | Some("Player") => {}
            Some("Opponent" | "Player.Opponent") => {
                src_parts.push("Filter::ControlledByOpponent".to_string());
            }
            Some("You") => src_parts.push("Filter::ControlledByYou".to_string()),
            Some(_) => return self.deny("static ability `S: Mode$ CantTarget`".into()),
        }
        let raw_src = match src_parts.len() {
            0 => "Filter::Any".to_string(),
            1 => src_parts.remove(0),
            _ => format!("Filter::And(&[{}])", src_parts.join(", ")),
        };
        let normalized = Self::named_constant(&raw_src);
        Some(self.body.filter_static("SOURCE", &normalized))
    }

    /// `R: Event$ RollDice | ActiveZones$ Battlefield | ValidPlayer$ You`
    /// rolling one extra die and ignoring the lowest (`Modifier::RollAdditionalDieIgnoreLowest`).
    pub(super) fn replace_roll_dice(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ValidPlayer").as_deref() != Some("You")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `R: Event$ RollDice`".into());
        }
        let Some(plus_name) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ RollDice`".into());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ RollDice`".into());
        }
        let Some((api1, mut rep1)) = self
            .svars
            .get(plus_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ RollDice`".into());
        };
        rep1.drop_prose();
        let Some(ign_name) = rep1.take("SubAbility") else {
            return self.deny("replacement `R: Event$ RollDice`".into());
        };
        if api1 != "ReplaceEffect"
            || rep1.take("VarName").as_deref() != Some("Number")
            || rep1.take("VarValue").as_deref() != Some("ReplaceCount$Number/Plus.1")
            || !rep1.exhausted()
        {
            return self.deny("replacement `R: Event$ RollDice`".into());
        }
        let Some((api2, mut rep2)) = self
            .svars
            .get(ign_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ RollDice`".into());
        };
        rep2.drop_prose();
        if api2 != "ReplaceEffect"
            || rep2.take("VarName").as_deref() != Some("Ignore")
            || rep2.take("VarValue").as_deref() != Some("ReplaceCount$Ignore/Plus.1")
            || !rep2.exhausted()
        {
            return self.deny("replacement `R: Event$ RollDice`".into());
        }
        self.push_static_with_cond(
            "Filter::Any",
            "Modifier::RollAdditionalDieIgnoreLowest",
            &cond,
        );
        Some(())
    }

    /// `R: Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self`
    /// pay-before-entry chain (`EnterModifier::DiscardOrGraveyard` and
    /// `EnterModifier::SacrificeOrGraveyard`).
    pub(super) fn moved_pay_before_etb(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || !matches!(
                p.take("ActiveZones").as_deref(),
                None | Some("Battlefield" | "All")
            )
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((api, mut step1)) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        step1.drop_prose();
        step1.take("AILogic");
        if api == "Discard" {
            let valid = step1.take("DiscardValid")?;
            let to_gy = step1.take("SubAbility")?;
            if step1.take("Mode").as_deref() != Some("TgtChoose")
                || step1.take("RememberDiscarded").as_deref() != Some("True")
                || step1.take("Optional").as_deref() != Some("True")
                || !matches!(step1.take("Defined").as_deref(), None | Some("You"))
                || !matches!(step1.take("NumCards").as_deref(), None | Some("1"))
                || !step1.exhausted()
                || !self.verify_pay_before_etb_tail(&to_gy, 1)
            {
                return self.deny("replacement `Moved` this rule cannot read".into());
            }
            let expr = self.filter_expr_in_zone(&valid, true)?;
            let filter = self.body.filter_static("ENTRY_PAY", &expr);
            self.body
                .enter_modifiers
                .push(format!("EnterModifier::DiscardOrGraveyard(&{filter})"));
            return Some(());
        }
        if api == "Sacrifice" {
            let valid = step1.take("SacValid")?;
            let to_gy = step1.take("SubAbility")?;
            let amount_raw = step1.take("Amount");
            let strict = step1.take("StrictAmount");
            let count: u8 = match amount_raw {
                None => 1,
                Some(raw) => raw.trim().parse::<u8>().ok().filter(|&n| n >= 1)?,
            };
            if (count > 1 && strict.as_deref() != Some("True"))
                || (count == 1 && !matches!(strict.as_deref(), None | Some("True")))
                || step1.take("Defined").as_deref() != Some("You")
                || step1.take("RememberSacrificed").as_deref() != Some("True")
                || !step1.exhausted()
                || !self.verify_pay_before_etb_tail(&to_gy, count)
            {
                return self.deny("replacement `Moved` this rule cannot read".into());
            }
            let scoped = scope_controlled_by_you(&valid);
            let expr = self.filter_expr(&scoped)?;
            let filter = self.body.filter_static("ENTRY_PAY", &expr);
            self.body.enter_modifiers.push(format!(
                "EnterModifier::SacrificeOrGraveyard {{ filter: &{filter}, count: {count} }}"
            ));
            return Some(());
        }
        self.deny("replacement `Moved` this rule cannot read".into())
    }

    fn verify_pay_before_etb_tail(&self, to_gy_name: &str, count: u8) -> bool {
        let Some((api_gy, mut gy)) = self
            .svars
            .get(to_gy_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return false;
        };
        gy.drop_prose();
        let Some(var) = gy.take("ConditionCheckSVar") else {
            return false;
        };
        let Some(cmp_fail) = gy.take("ConditionSVarCompare") else {
            return false;
        };
        let Some(to_bf_name) = gy.take("SubAbility") else {
            return false;
        };
        let fail_ok = (count == 1 && cmp_fail == "EQ0") || cmp_fail == format!("LT{count}");
        if api_gy != "ChangeZone"
            || gy.take("Origin").as_deref() != Some("All")
            || gy.take("Destination").as_deref() != Some("Graveyard")
            || gy.take("Defined").as_deref() != Some("ReplacedCard")
            || !fail_ok
            || !gy.exhausted()
            || self.svars.get(var.trim()).map(|s| s.trim()) != Some("Remembered$Amount")
        {
            return false;
        }
        let Some((api_bf, mut bf)) = self
            .svars
            .get(to_bf_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return false;
        };
        bf.drop_prose();
        let Some(cmp_ok) = bf.take("ConditionSVarCompare") else {
            return false;
        };
        let cleanup = bf.take("SubAbility");
        let ok_cmp = (count == 1 && matches!(cmp_ok.as_str(), "EQ1" | "GE1"))
            || cmp_ok == format!("GE{count}")
            || cmp_ok == format!("EQ{count}");
        api_bf == "ChangeZone"
            && bf.take("Origin").as_deref() == Some("All")
            && bf.take("Destination").as_deref() == Some("Battlefield")
            && bf.take("Defined").as_deref() == Some("ReplacedCard")
            && bf.take("ConditionCheckSVar").as_deref() == Some(var.as_str())
            && ok_cmp
            && bf.exhausted()
            && self.is_optional_clear_remembered(cleanup.as_deref())
    }

    /// Characteristic-defining P/T counting cards in your or opponents'
    /// graveyards (`PtCount::InAllGraveyards` with `Filter::OwnedByYou` or
    /// `Filter::Not(&Filter::OwnedByYou)`).
    pub(super) fn has_owner_graveyard_cda(&self, p: &Params) -> bool {
        if p.peek("Affected").is_some() || p.peek("CharacteristicDefining") != Some("True") {
            return false;
        }
        [p.peek("SetPower"), p.peek("SetToughness")]
            .into_iter()
            .flatten()
            .any(|var| self.svar_is_owner_graveyard_count(var))
    }

    fn svar_is_owner_graveyard_count(&self, var: &str) -> bool {
        let Some(raw) = self.svars.get(var.trim()).map(|s| s.trim()) else {
            return false;
        };
        let base = raw.rsplit_once("/Plus.").map_or(raw, |(l, _)| l.trim());
        let resolved = base
            .strip_prefix("SVar$")
            .and_then(|inner| self.svars.get(inner.trim()).map(|s| s.trim()))
            .unwrap_or(base);
        let Some(valid) = resolved.strip_prefix("Count$ValidGraveyard ") else {
            return false;
        };
        !valid.contains('/')
            && !valid.contains('$')
            && (valid.contains("YouOwn") || valid.contains("OppOwn"))
    }

    pub(super) fn characteristic_pt_graveyard_owner(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("CharacteristicDefining").as_deref() != Some("True")
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(
                p.take("EffectZone").as_deref(),
                None | Some("Battlefield" | "All")
            )
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("a characteristic-defining ability with unsupported scope".into());
        }
        let power_var = p.take("SetPower");
        let tough_var = p.take("SetToughness");
        if power_var.is_none() && tough_var.is_none() {
            return self.deny("a characteristic-defining ability with no P/T".into());
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        match (power_var, tough_var) {
            (Some(power), Some(toughness)) if power == toughness => {
                let count = self.pt_count_graveyard_svar(&power)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                     count: {count}, toughness_plus: 0 }})"
                ));
                Some(())
            }
            (Some(power), Some(toughness)) => {
                if let Some(t_def) = self.svars.get(&toughness).cloned()
                    && let Some(rest) = t_def.trim().strip_prefix(&format!("SVar${power}/Plus."))
                    && let Ok(plus) = rest.trim().parse::<i8>()
                {
                    let count = self.pt_count_graveyard_svar(&power)?;
                    self.body.abilities.push(format!(
                        "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                         count: {count}, toughness_plus: {plus} }})"
                    ));
                    return Some(());
                }
                let p_count = self.pt_count_graveyard_svar(&power)?;
                let t_count = self.pt_count_graveyard_svar(&toughness)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: {p_count}, toughness: {t_count} }})"
                ));
                Some(())
            }
            (Some(power), None) => {
                let (_, fixed_t) = self.pt.and_then(|pt| pt.split_once('/'))?;
                let Ok(t_num) = fixed_t.trim().parse::<u16>() else {
                    return self.deny("a characteristic-defining ability with no P/T".into());
                };
                let p_count = self.pt_count_graveyard_svar(&power)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: {p_count}, toughness: PtCount::Fixed({t_num}) }})"
                ));
                Some(())
            }
            (None, Some(toughness)) => {
                let (fixed_p, _) = self.pt.and_then(|pt| pt.split_once('/'))?;
                let Ok(p_num) = fixed_p.trim().parse::<u16>() else {
                    return self.deny("a characteristic-defining ability with no P/T".into());
                };
                let t_count = self.pt_count_graveyard_svar(&toughness)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: PtCount::Fixed({p_num}), toughness: {t_count} }})"
                ));
                Some(())
            }
            (None, None) => unreachable!(),
        }
    }

    fn pt_count_graveyard_svar(&mut self, var: &str) -> Option<String> {
        let Some(raw) = self.svars.get(var.trim()).cloned() else {
            return self.deny(format!("`{var}` names no SVar"));
        };
        let raw = raw.trim();
        if let Some((lhs, rhs)) = raw.rsplit_once("/Plus.")
            && let Ok(offset) = rhs.trim().parse::<u16>()
        {
            let base = if let Some(inner_var) = lhs.trim().strip_prefix("SVar$") {
                let inner = self.svars.get(inner_var.trim())?.clone();
                self.pt_count_graveyard_base(inner.trim())?
            } else {
                self.pt_count_graveyard_base(lhs.trim())?
            };
            return Some(format!(
                "PtCount::Plus {{ base: &{base}, offset: {offset} }}"
            ));
        }
        self.pt_count_graveyard_base(raw)
    }

    fn pt_count_graveyard_base(&mut self, count: &str) -> Option<String> {
        let Some(valid) = count.strip_prefix("Count$ValidGraveyard ") else {
            return self.deny(format!("count `{count}`"));
        };
        if valid.contains('/') || valid.contains('$') {
            return self.deny(format!("count `{count}`"));
        }
        let filter_expr = if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
            let base = self.filter_expr_in_zone(&stripped, true)?;
            conjoin_filter_expr(&base, "Filter::OwnedByYou")
        } else if let Some(stripped) = Self::strip_opp_graveyard_owner(valid) {
            let base = self.filter_expr_in_zone(&stripped, true)?;
            conjoin_filter_expr(&base, "Filter::Not(&Filter::OwnedByYou)")
        } else if !valid.contains("Ctrl") && !valid.contains("Own") {
            self.filter_expr_in_zone(valid, true)?
        } else {
            return self.deny(format!("count `{count}`"));
        };
        let name = self.body.filter_static("COUNTED", &filter_expr);
        Some(format!("PtCount::InAllGraveyards(&{name})"))
    }

    /// Wave 8/9 `SVar` condition clauses: `StartingPlayer`, `ControlYourCommander`,
    /// `AnyLibraryAtMost`, boolean `Count$<Keyword>.1.0` / `.0.1` (`Kicked`,
    /// `wasCastFromYourHandByYou`, `Delirium`, `Threshold`, `Metalcraft`,
    /// `Hellbent`, `FatefulHour`, `Blessing`, `Revolt`, `Raid`), and
    /// opponent-control count/comparison clauses (`OpponentControlCount`,
    /// `OpponentControlCountAtMost`, `OpponentControlsMore`,
    /// `OpponentControlsMoreBy`).
    pub(super) fn extra_svar_condition_wave8(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.resolve_svar_or_inline(var)?;
        let cmp = cmp.trim();
        if let Some(cond) = binary_flag_svar_condition(&raw, cmp) {
            return Some(cond);
        }
        if matches!(
            raw.as_str(),
            "Count$Valid Card.IsCommander+YouOwn+YouCtrl"
                | "Count$Valid Card.YouOwn+IsCommander+YouCtrl"
                | "Count$Valid Card.IsCommander+YouCtrl+YouOwn"
                | "Count$Valid Permanent.IsCommander+YouOwn+YouCtrl"
                | "Count$Valid Permanent.YouOwn+IsCommander+YouCtrl"
                | "Count$Valid Permanent.IsCommander+YouCtrl+YouOwn"
        ) {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::ControlYourCommander".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::ControlYourCommander)".to_string())
                }
                _ => None,
            };
        }
        if let Some(cond) = self.wave16b_svar_condition(&raw, cmp) {
            return Some(cond);
        }
        if let Some(cond) = self.wave17b_svar_condition(&raw, cmp) {
            return Some(cond);
        }
        if raw == "PlayerCountPlayers$LowestValidLibrary Card.YouOwn"
            && let Some(n) = super::saga_ward_convoke::parse_at_most_u16(cmp)
        {
            return Some(format!("Condition::AnyLibraryAtMost({n})"));
        }
        if matches!(
            raw.as_str(),
            "PlayerCountPlayers$LowestCardsInHand"
                | "PlayerCountPlayers$LowestValidHand Card.YouOwn"
        ) {
            if let Some(n) =
                super::saga_ward_convoke::parse_at_most_u16(cmp).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!("Condition::APlayerHandSizeAtMost({n})"));
            }
            if let Some(min) = super::saga_ward_convoke::parse_at_least_u16(cmp)
                && let Some(max) = min.checked_sub(1).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::APlayerHandSizeAtMost({max}))"
                ));
            }
        }
        self.opponent_count_svar_condition(&raw, cmp)
            .or_else(|| self.wave17b_amount_compare(&raw, cmp))
    }

    fn wave17b_svar_condition(&self, raw: &str, cmp: &str) -> Option<String> {
        if raw == "Count$ThisTurnCast_Card.YouCtrl" {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::Not(&Condition::YouCastNoSpellThisTurn)".into()),
                "EQ0" | "LT1" | "LE0" => Some("Condition::YouCastNoSpellThisTurn".into()),
                _ => None,
            };
        }
        if matches!(
            raw,
            "PlayerCountRegisteredOpponents$HighestLifeLostThisTurn"
                | "PlayerCountOpponents$HighestLifeLostThisTurn"
        ) {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::OpponentLostLifeThisTurn".into()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".into())
                }
                _ => None,
            };
        }
        if raw == "Count$ValidHand Card.YouOwn"
            && let Some(n) = super::saga_ward_convoke::parse_at_least_u16(cmp).filter(|&n| n >= 1)
        {
            return Some(format!(
                "Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), {n})"
            ));
        }
        if raw == "Count$YourLifeTotal" {
            if let Some(n) = super::saga_ward_convoke::parse_at_least_u16(cmp).filter(|&n| n >= 1) {
                return Some(format!(
                    "Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n})"
                ));
            }
            if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp) {
                return Some(format!("Condition::LifeAtMost({max})"));
            }
        }
        if raw == "Count$CardPower" && !self.on_a_spell && !self.in_delayed {
            return self_power_condition(cmp);
        }
        if let Some(sub_raw) = raw
            .strip_prefix("Count$ThisTurnEntered_Battlefield_")
            .and_then(|s| s.strip_suffix(".YouCtrl"))
            && super::card_type_const(sub_raw).is_none()
            && sub_raw != "Permanent"
            && let Some(sub) = self.cats.const_path(sub_raw)
        {
            return match cmp {
                "GE1" | "GT0" => Some(format!(
                    "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{sub}])"
                )),
                "EQ0" | "LT1" | "LE0" => Some(format!(
                    "Condition::Not(&Condition::EnteredSubtypeUnderYourControlThisTurn(&[{sub}]))"
                )),
                _ => None,
            };
        }
        None
    }

    fn wave17b_amount_compare(&mut self, raw: &str, cmp: &str) -> Option<String> {
        if raw == "Count$CardPower" {
            return None;
        }
        let (op_raw, rhs_raw) = cmp.split_at_checked(2)?;
        if rhs_raw.is_empty() {
            return None;
        }
        // Leave `GE`/`GT`/`LE`/`LT` two-SVar pairs to `extra_svar_condition_clause`'s
        // existing `Amount::Difference` lowering; handle `EQ`/`NE` comparisons here.
        let op = match op_raw {
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        let (right, rhs_positive) = if let Ok(n) = rhs_raw.parse::<u32>() {
            (format!("Amount::Fixed({n})"), n >= 1)
        } else {
            let rhs_def = self.svars.get(rhs_raw.trim())?.trim().to_string();
            if rhs_def == "Count$CardPower" {
                return None;
            }
            let pos = rhs_def.contains("StartingLife")
                || rhs_def
                    .rsplit_once("/Plus.")
                    .and_then(|(_, n)| n.trim().parse::<u32>().ok())
                    .is_some_and(|n| n >= 1);
            (self.static_count_expr(&rhs_def)?, pos)
        };
        if raw == "Count$YourLifeTotal" && !rhs_positive {
            return None;
        }
        let left = self.static_count_expr(raw)?;
        Some(format!(
            "Condition::AmountCompare {{ left: &{left}, op: {op}, right: &{right} }}"
        ))
    }

    fn wave16b_svar_condition(&self, raw: &str, cmp: &str) -> Option<String> {
        if raw == "Count$xPaid" && self.has_x {
            return x_paid_condition(cmp);
        }
        if matches!(
            raw,
            "Count$CastTotalManaSpent Snow" | "Count$TotalManaSpent Snow"
        ) && !self.in_delayed
            && (self.on_a_spell || self.trigger_is_self || self.trigger_mode.is_none())
        {
            return snow_spent_condition(cmp);
        }
        if matches!(
            raw,
            "PlayerCountRegisteredOpponents$HasPropertywasDealtDamageThisTurn"
                | "PlayerCountOpponents$HasPropertywasDealtDamageThisTurn"
        ) {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::OpponentDealtDamageThisTurn".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::OpponentDealtDamageThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if !self.in_delayed
            && let Some(cond) = warp_cost_paid_svar_condition(raw, cmp)
        {
            return Some(cond);
        }
        spell_warped_this_turn_condition(raw, cmp)
    }

    fn opponent_count_svar_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        if let Some(valid) = raw.strip_prefix("PlayerCountOpponents$HighestValid ") {
            let base_valid = strip_you_ctrl_only(valid)?;
            if let Some(n) =
                super::saga_ward_convoke::parse_at_least_u16(cmp).and_then(|v| u8::try_from(v).ok())
            {
                let opp_expr = self.filter_expr(&scope_controlled_by_opp(&base_valid))?;
                let name = self.body.filter_static("CHECK", &opp_expr);
                return Some(format!("Condition::OpponentControlCount(&{name}, {n})"));
            }
            if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp)
                && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
            {
                let opp_expr = self.filter_expr(&scope_controlled_by_opp(&base_valid))?;
                let name = self.body.filter_static("CHECK", &opp_expr);
                return Some(format!(
                    "Condition::Not(&Condition::OpponentControlCount(&{name}, {min}))"
                ));
            }
            return self.opponent_more_svar_pair(&base_valid, cmp, true, 0);
        }
        if let Some(valid) = raw.strip_prefix("PlayerCountOpponents$LowestValid ") {
            let base_valid = strip_you_ctrl_only(valid)?;
            if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp) {
                let expr = self.filter_expr(&base_valid)?;
                let name = self.body.filter_static("CHECK", &expr);
                return Some(format!(
                    "Condition::OpponentControlCountAtMost(&{name}, {max})"
                ));
            }
            if let Some(min) = super::saga_ward_convoke::parse_at_least_u16(cmp)
                && let Some(max) = min.checked_sub(1)
            {
                let expr = self.filter_expr(&base_valid)?;
                let name = self.body.filter_static("CHECK", &expr);
                return Some(format!(
                    "Condition::Not(&Condition::OpponentControlCountAtMost(&{name}, {max}))"
                ));
            }
            return None;
        }
        if let Some((base_valid, plus)) = parse_your_valid_plus(raw) {
            return self.opponent_more_svar_pair(&base_valid, cmp, false, plus);
        }
        None
    }

    fn opponent_more_svar_pair(
        &mut self,
        lhs_base: &str,
        cmp: &str,
        lhs_is_opp: bool,
        lhs_plus: u32,
    ) -> Option<String> {
        let (op, rhs_var) = cmp.split_at_checked(2)?;
        if rhs_var.is_empty() || rhs_var.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let rhs_def = self.svars.get(rhs_var.trim())?.trim().to_string();
        let (min_diff, negated) = if lhs_is_opp {
            let (rhs_base, rhs_plus) = parse_your_valid_plus(&rhs_def)?;
            if rhs_base != lhs_base {
                return None;
            }
            match (op, rhs_plus) {
                ("GT", k) => (k.checked_add(1)?, false),
                ("GE", k) if k >= 1 => (k, false),
                ("LE", 0) => (1, true),
                ("LT", k) if k >= 1 => (k, true),
                _ => return None,
            }
        } else {
            let rhs_valid = rhs_def.strip_prefix("PlayerCountOpponents$HighestValid ")?;
            let rhs_base = strip_you_ctrl_only(rhs_valid)?;
            if rhs_base != lhs_base {
                return None;
            }
            match (op, lhs_plus) {
                ("LT", k) => (k.checked_add(1)?, false),
                ("LE", k) if k >= 1 => (k, false),
                ("GE", 0) => (1, true),
                ("GT", k) if k >= 1 => (k, true),
                _ => return None,
            }
        };
        let expr = self.filter_expr(lhs_base)?;
        let name = self.body.filter_static("CHECK", &expr);
        let core = if min_diff == 1 {
            format!("Condition::OpponentControlsMore(&{name})")
        } else {
            format!("Condition::OpponentControlsMoreBy(&{name}, {min_diff})")
        };
        Some(if negated {
            format!("Condition::Not(&{core})")
        } else {
            core
        })
    }

    /// `R: Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | Layer$ Copy`
    /// with `DB$ GenericChoice` of `DB$ Clone | Defined$ Self` shape branches
    /// (`EnterModifier::ChooseShape`, e.g. Primal Clay).
    pub(super) fn moved_choose_shape(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("Layer").as_deref() != Some("Copy")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(choice_name) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((api, mut choice)) = self
            .svars
            .get(choice_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        choice.drop_prose();
        if api != "GenericChoice" || choice.take("Defined").as_deref() != Some("You") {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(choices_raw) = choice.take("Choices") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !choice.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let mut shapes = Vec::new();
        for branch_name in choices_raw.split(',').map(str::trim) {
            let Some(mods) = self.parse_shape_clone_branch(branch_name) else {
                return self.deny("replacement `Moved` this rule cannot read".into());
            };
            shapes.push(format!("&[{}]", mods.join(", ")));
        }
        if shapes.len() < 2 {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        self.body.enter_modifiers.push(format!(
            "EnterModifier::ChooseShape(&[{}])",
            shapes.join(", ")
        ));
        Some(())
    }

    fn parse_shape_clone_branch(&self, name: &str) -> Option<Vec<String>> {
        let (api, mut b) = self.svars.get(name).and_then(|s| Params::parse(s))?;
        b.drop_prose();
        if api != "Clone" || b.take("Defined").as_deref() != Some("Self") {
            return None;
        }
        let mut mods = Vec::new();
        if let (Some(pw), Some(th)) = (b.take("SetPower"), b.take("SetToughness")) {
            let p_num: i16 = pw.trim().parse().ok()?;
            let t_num: i16 = th.trim().parse().ok()?;
            mods.push(format!("CopyMod::SetPT({p_num}, {t_num})"));
        } else if b.has("SetPower") || b.has("SetToughness") {
            return None;
        }
        if let Some(types_raw) = b.take("AddTypes") {
            let mut card_types = Vec::new();
            for word in types_raw
                .split(['&', ' '])
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                if let Some(ty) = card_type_const(word) {
                    card_types.push(ty);
                } else {
                    let sub = self.cats.const_path(word)?;
                    mods.push(format!("CopyMod::AddSubtype({sub})"));
                }
            }
            if !card_types.is_empty() {
                mods.push(format!(
                    "CopyMod::AddType({})",
                    join_const_union(&card_types)
                ));
            }
        }
        if let Some(kws_raw) = b.take("AddKeywords") {
            for kw in kws_raw.split('&').map(str::trim).filter(|k| !k.is_empty()) {
                let bit = super::keyword_const_of(kw)?;
                mods.push(format!("CopyMod::AddKeyword({bit})"));
            }
        }
        (b.exhausted() && !mods.is_empty()).then_some(mods)
    }

    /// `R: Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self`
    /// with `DB$ SacrificeAll | ValidCards$ ... | SubAbility$ ChangeZone(Battlefield)`
    /// (`EnterModifier::SacrificeOthers`, e.g. Sheltered Valley).
    pub(super) fn moved_sacrifice_others(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || !matches!(
                p.take("ActiveZones").as_deref(),
                None | Some("Battlefield" | "All")
            )
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((api, mut sac)) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        sac.drop_prose();
        let Some(valid) = sac.take("ValidCards") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        let Some(sub_name) = sac.take("SubAbility") else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        if api != "SacrificeAll" || !sac.exhausted() || strip_you_ctrl_only(&valid).is_none() {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let Some((sub_api, mut bf)) = self
            .svars
            .get(sub_name.trim())
            .and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".into());
        };
        bf.drop_prose();
        if sub_api != "ChangeZone"
            || bf.take("Origin").as_deref() != Some("All")
            || bf.take("Destination").as_deref() != Some("Battlefield")
            || bf.take("Defined").as_deref() != Some("ReplacedCard")
            || !bf.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".into());
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("ENTRY_SAC", &expr);
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::SacrificeOthers(&{filter})"));
        Some(())
    }

    /// `R: Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplacementResult$ Updated`
    /// with `DB$ Tap | Defined$ Self | ETB$ True` gated on `DuringYourFirstTurns`
    /// (Starting Town) or `NotStartingPlayer` (`EnterModifier::TappedUnlessCondition`).
    pub(super) fn moved_tapped_unless_condition(&mut self, p: Params) -> ExtraOutcome {
        if p.peek("ReplacementResult") != Some("Updated") {
            return ExtraOutcome::Pass;
        }
        let Some(with) = p.peek("ReplaceWith") else {
            return ExtraOutcome::Pass;
        };
        let Some((api, mut tap)) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))
        else {
            return ExtraOutcome::Pass;
        };
        tap.drop_prose();
        if api != "Tap"
            || tap.take("Defined").as_deref() != Some("Self")
            || tap.take("ETB").as_deref() != Some("True")
        {
            return ExtraOutcome::Pass;
        }
        let Some(var) = tap.take("ConditionCheckSVar") else {
            return ExtraOutcome::Pass;
        };
        let cmp = tap
            .take("ConditionSVarCompare")
            .unwrap_or_else(|| "GE1".to_string());
        if !tap.exhausted() {
            return ExtraOutcome::Pass;
        }
        let Some(unless_cond) = self.tapped_unless_svar_condition(var.trim(), cmp.trim()) else {
            return ExtraOutcome::Pass;
        };
        let mut check = p;
        check.drop_prose();
        if check.take("ValidCard").as_deref() != Some("Card.Self")
            || check.take("Destination").as_deref() != Some("Battlefield")
            || check.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(check.take("Secondary").as_deref(), None | Some("True"))
        {
            return ExtraOutcome::Pass;
        }
        check.take("ReplaceWith");
        if !check.exhausted() {
            return ExtraOutcome::Pass;
        }
        self.body.enter_modifiers.push(format!(
            "EnterModifier::TappedUnlessCondition({unless_cond})"
        ));
        ExtraOutcome::Handled(Some(()))
    }

    fn tapped_unless_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.svars.get(var)?.trim();
        if (raw == "Count$StartingPlayer.1.0" && matches!(cmp, "GE1" | "GT0" | "EQ1"))
            || (raw == "Count$StartingPlayer.0.1" && matches!(cmp, "EQ0" | "LT1"))
        {
            return Some("Condition::NotStartingPlayer".to_string());
        }
        if let Some(rest) = raw.strip_prefix("Count$Compare ")
            && let Some(gt_str) = cmp.strip_prefix("GT")
            && let Ok(n) = gt_str.parse::<u32>()
            && n >= 1
        {
            let (active_var, tail) = rest.split_once(" GE1.")?;
            let (turns_var, fallback_str) = tail.split_once('.')?;
            let fallback: u32 = fallback_str.trim().parse().ok()?;
            if fallback > n
                && self.svars.get(active_var.trim()).map(|s| s.trim())
                    == Some("PlayerCountPropertyYou$HasPropertyActive")
                && self.svars.get(turns_var.trim()).map(|s| s.trim()) == Some("Count$YourTurns")
            {
                return Some(format!("Condition::DuringYourFirstTurns({n})"));
            }
        }
        None
    }
}

fn binary_flag_svar_condition(raw: &str, cmp: &str) -> Option<String> {
    let want_one = match cmp {
        "GE1" | "GT0" | "EQ1" => true,
        "EQ0" | "LT1" | "LE0" => false,
        _ => return None,
    };
    let (expr, one_is_positive) = match raw {
        "Count$StartingPlayer.0.1" => ("Condition::NotStartingPlayer", true),
        "Count$StartingPlayer.1.0" => ("Condition::NotStartingPlayer", false),
        "Count$Kicked.0.1" => ("Condition::AmountAtLeast(Amount::TimesKicked, 1)", false),
        "Count$wasCastFromYourHandByYou.1.0" => (
            "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceMatches(&Filter::CastFromHand)])",
            true,
        ),
        "Count$wasCastFromYourHandByYou.0.1" => (
            "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceMatches(&Filter::CastFromHand)])",
            false,
        ),
        "Count$Delirium.1.0" => ("Condition::GraveyardTypesAtLeast(4)", true),
        "Count$Delirium.0.1" => ("Condition::GraveyardTypesAtLeast(4)", false),
        "Count$Threshold.1.0" => ("Condition::GraveyardCountAtLeast(7)", true),
        "Count$Threshold.0.1" => ("Condition::GraveyardCountAtLeast(7)", false),
        "Count$Metalcraft.1.0" => ("Condition::ControlCount(&Filter::ARTIFACT, 3)", true),
        "Count$Metalcraft.0.1" => ("Condition::ControlCount(&Filter::ARTIFACT, 3)", false),
        "Count$Hellbent.1.0" => ("Condition::HandSizeAtMost(0)", true),
        "Count$Hellbent.0.1" => ("Condition::HandSizeAtMost(0)", false),
        "Count$FatefulHour.1.0" => ("Condition::LifeAtMost(5)", true),
        "Count$FatefulHour.0.1" => ("Condition::LifeAtMost(5)", false),
        "Count$Blessing.1.0" => ("Condition::CitysBlessing", true),
        "Count$Blessing.0.1" => ("Condition::CitysBlessing", false),
        "Count$Revolt.0.1" => ("Condition::Revolt", false),
        "Count$Raid.0.1" => ("Condition::AttackedWithAtLeast(1)", false),
        "Count$AltCost.1.0" => ("Condition::AltCostPaid", true),
        "Count$AltCost.0.1" => ("Condition::AltCostPaid", false),
        "Count$Monstrous"
        | "Count$Monstrous.1.0"
        | "Count$ValidSelf Card.IsMonstrous"
        | "Count$Valid Card.Self+IsMonstrous"
        | "Count$Valid Creature.Self+IsMonstrous"
        | "Count$ValidSelf Card.monstrous"
        | "Count$Valid Card.Self+monstrous" => ("Condition::IsMonstrous", true),
        "Count$Monstrous.0.1" => ("Condition::IsMonstrous", false),
        "Count$Suspected"
        | "Count$Suspected.1.0"
        | "Count$ValidSelf Card.IsSuspected"
        | "Count$Valid Card.Self+IsSuspected"
        | "Count$Valid Creature.Self+IsSuspected"
        | "Count$ValidSelf Card.suspected"
        | "Count$Valid Card.Self+suspected" => ("Condition::IsSuspected", true),
        "Count$Suspected.0.1" => ("Condition::IsSuspected", false),
        _ => return None,
    };
    Some(if want_one == one_is_positive {
        expr.to_string()
    } else {
        negate_condition_expr(expr)
    })
}

fn self_power_condition(cmp: &str) -> Option<String> {
    if let Some(n) = cmp.strip_prefix("GE").and_then(|s| s.parse::<i16>().ok()) {
        return Some(format!(
            "Condition::SourceMatches(&Filter::PowerAtLeast({n}))"
        ));
    }
    if let Some(n) = cmp
        .strip_prefix("GT")
        .and_then(|s| s.parse::<i16>().ok())
        .and_then(|v| v.checked_add(1))
    {
        return Some(format!(
            "Condition::SourceMatches(&Filter::PowerAtLeast({n}))"
        ));
    }
    if let Some(n) = cmp.strip_prefix("LE").and_then(|s| s.parse::<i16>().ok()) {
        return Some(format!(
            "Condition::SourceMatches(&Filter::PowerAtMost({n}))"
        ));
    }
    if let Some(n) = cmp
        .strip_prefix("LT")
        .and_then(|s| s.parse::<i16>().ok())
        .and_then(|v| v.checked_sub(1))
    {
        return Some(format!(
            "Condition::SourceMatches(&Filter::PowerAtMost({n}))"
        ));
    }
    if let Some(n) = cmp.strip_prefix("EQ").and_then(|s| s.parse::<i16>().ok()) {
        return Some(format!(
            "Condition::SourceMatches(&Filter::And(&[Filter::PowerAtLeast({n}), Filter::PowerAtMost({n})]))"
        ));
    }
    None
}

fn x_paid_condition(cmp: &str) -> Option<String> {
    if let Some(n) = super::saga_ward_convoke::parse_at_least_u16(cmp).filter(|&n| n >= 1) {
        return Some(format!("Condition::XAtLeast({n})"));
    }
    if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp) {
        let above = u32::from(max).checked_add(1)?;
        return Some(format!("Condition::Not(&Condition::XAtLeast({above}))"));
    }
    if let Some(eq) = cmp.strip_prefix("EQ").and_then(|s| s.parse::<u32>().ok()) {
        return Some(if eq == 0 {
            "Condition::Not(&Condition::XAtLeast(1))".to_string()
        } else {
            let above = eq.checked_add(1)?;
            format!(
                "Condition::All(&[Condition::XAtLeast({eq}), \
                 Condition::Not(&Condition::XAtLeast({above}))])"
            )
        });
    }
    None
}

fn snow_spent_condition(cmp: &str) -> Option<String> {
    match cmp {
        "GE1" | "GT0" => return Some("Condition::SnowManaWasSpent".to_string()),
        "EQ0" | "LT1" | "LE0" => {
            return Some("Condition::Not(&Condition::SnowManaWasSpent)".to_string());
        }
        _ => {}
    }
    if let Some(n) = super::saga_ward_convoke::parse_at_least_u16(cmp).filter(|&n| n >= 2) {
        return Some(format!(
            "Condition::AmountAtLeast(Amount::SnowManaSpentToCast, {n})"
        ));
    }
    if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp).filter(|&n| n >= 1) {
        let above = u32::from(max).checked_add(1)?;
        return Some(format!(
            "Condition::Not(&Condition::AmountAtLeast(Amount::SnowManaSpentToCast, {above}))"
        ));
    }
    if let Some(eq) = cmp.strip_prefix("EQ").and_then(|s| s.parse::<u32>().ok())
        && eq >= 1
    {
        let above = eq.checked_add(1)?;
        return Some(format!(
            "Condition::All(&[Condition::AmountAtLeast(Amount::SnowManaSpentToCast, {eq}), \
             Condition::Not(&Condition::AmountAtLeast(Amount::SnowManaSpentToCast, {above}))])"
        ));
    }
    None
}

fn warp_cost_paid_svar_condition(raw: &str, cmp: &str) -> Option<String> {
    let one_is_positive = match raw {
        "Count$Warped.1.0" | "Count$WarpCostPaid.1.0" | "Count$CastByWarp.1.0" => true,
        "Count$Warped.0.1" | "Count$WarpCostPaid.0.1" | "Count$CastByWarp.0.1" => false,
        _ => return None,
    };
    let want_one = match cmp {
        "GE1" | "GT0" | "EQ1" => true,
        "EQ0" | "LT1" | "LE0" => false,
        _ => return None,
    };
    Some(if want_one == one_is_positive {
        "Condition::WarpCostPaid".to_string()
    } else {
        "Condition::Not(&Condition::WarpCostPaid)".to_string()
    })
}

fn spell_warped_this_turn_condition(raw: &str, cmp: &str) -> Option<String> {
    let (cond_arg, amt_rel) = match raw {
        "Count$ThisTurnWarped"
        | "Count$WarpedThisTurn"
        | "Count$SpellsWarpedThisTurn"
        | "Count$ThisTurnCast_Card.CastSa Spell.Warp" => ("None", "PlayerRel::EachPlayer"),
        "Count$ThisTurnWarped_You"
        | "Count$WarpedThisTurn_You"
        | "Count$ThisTurnCast_Card.YouCtrl+CastSa Spell.Warp" => {
            ("Some(PlayerRel::You)", "PlayerRel::You")
        }
        "Count$ThisTurnWarped_Opponent"
        | "Count$WarpedThisTurn_Opponent"
        | "Count$ThisTurnCast_Card.OppCtrl+CastSa Spell.Warp" => {
            ("Some(PlayerRel::Opponent)", "PlayerRel::Opponent")
        }
        _ => return None,
    };
    match cmp {
        "GE1" | "GT0" => return Some(format!("Condition::SpellWarpedThisTurn({cond_arg})")),
        "EQ0" | "LT1" | "LE0" => {
            return Some(format!(
                "Condition::Not(&Condition::SpellWarpedThisTurn({cond_arg}))"
            ));
        }
        _ => {}
    }
    if let Some(n) = super::saga_ward_convoke::parse_at_least_u16(cmp).filter(|&n| n >= 2) {
        return Some(format!(
            "Condition::AmountAtLeast(Amount::SpellsWarpedThisTurn({amt_rel}), {n})"
        ));
    }
    if let Some(max) = super::saga_ward_convoke::parse_at_most_u16(cmp).filter(|&n| n >= 1) {
        let above = u32::from(max).checked_add(1)?;
        return Some(format!(
            "Condition::Not(&Condition::AmountAtLeast(Amount::SpellsWarpedThisTurn({amt_rel}), {above}))"
        ));
    }
    None
}

fn strip_you_ctrl_only(valid: &str) -> Option<String> {
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
            if atom == "YouCtrl" {
                has_you = true;
            } else if atom.contains("Ctrl") || atom.contains("Own") {
                return None;
            } else {
                kept.push(atom);
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

fn scope_controlled_by_opp(valid: &str) -> String {
    valid
        .split(',')
        .map(|alt| {
            let alt = alt.trim();
            if alt.contains('.') {
                format!("{alt}+OppCtrl")
            } else {
                format!("{alt}.OppCtrl")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_your_valid_plus(raw: &str) -> Option<(String, u32)> {
    let body = raw.strip_prefix("Count$Valid ")?;
    let (valid, plus) = match body.rsplit_once("/Plus.") {
        Some((lhs, rhs)) => (lhs.trim(), rhs.trim().parse::<u32>().ok()?),
        None => (body.trim(), 0),
    };
    if valid.contains('/') || valid.contains('$') {
        return None;
    }
    Some((strip_you_ctrl_only(valid)?, plus))
}

fn scope_controlled_by_you(valid: &str) -> String {
    valid
        .split([',', ';'])
        .map(|alt| {
            let alt = alt.trim();
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
}

pub(super) fn conjoin_filter_expr(expr: &str, extra: &str) -> String {
    if expr == "Filter::Any" {
        return extra.to_string();
    }
    if let Some(inner) = expr
        .strip_prefix("Filter::And(&[")
        .and_then(|s| s.strip_suffix("])"))
    {
        return format!("Filter::And(&[{inner}, {extra}])");
    }
    format!("Filter::And(&[{expr}, {extra}])")
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

fn combine_static_cond(existing: &str, extra: &str) -> String {
    if let Some(prev) = existing
        .strip_prefix(", condition = Some(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return format!(", condition = Some(Condition::All(&[{prev}, {extra}]))");
    }
    format!(", condition = Some({extra})")
}
