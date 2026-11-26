//! Wave 29d pure-`scriptgen` lowerings for statics, replacements, prevention
//! shields, unless-costs, `Reveal` at random, `ChooseNumber`, and `--stubs`.
//!
//! Every rule here lowers strictly onto existing, engine-verified
//! `baylee-cards-dsl` variants while preserving honest refusal boundaries.

use super::{Chain, Params, Tx, counter_kind, keyword_const};

impl Tx<'_> {
    /// `R: Event$ Moved | ActiveZones$ Battlefield | Origin$ <zones> |
    /// Destination$ Battlefield | ValidLKI$ <valid> | Prevent$ True |
    /// Layer$ CantHappen` (*Grafdigger's Cage*, *Kunoros, Hound of Athreos*,
    /// *Soulless Jailer*, *Weathered Runestone*).
    ///
    /// Verified in `baylee-engine/src/state/moves.rs`: `CantEnterBattlefield`
    /// runs before the card leaves `from_zone`, so `Filter::InZone(ZoneRef::…)`
    /// matches the card's origin zone directly.
    pub(super) fn wave29d_moved_cant_enter_from_zone(&mut self, p: &Params) -> Option<()> {
        let mut mp = p.clone();
        mp.drop_prose();
        if mp.take("ActiveZones").as_deref() != Some("Battlefield")
            || mp.take("Destination").as_deref() != Some("Battlefield")
            || mp.take("Prevent").as_deref() != Some("True")
            || mp.take("Layer").as_deref() != Some("CantHappen")
        {
            return None;
        }
        let origin = mp.take("Origin")?;
        let valid_raw = mp.take("ValidLKI")?;
        if !mp.exhausted() {
            return None;
        }
        let mut zone_exprs = Vec::new();
        for z in origin.split(',').map(str::trim) {
            let z_ref = match z {
                "Graveyard" => "Filter::InZone(ZoneRef::Graveyard)",
                "Library" => "Filter::InZone(ZoneRef::Library)",
                "Exile" => "Filter::InZone(ZoneRef::Exile)",
                "Hand" => "Filter::InZone(ZoneRef::Hand)",
                _ => return None,
            };
            if !zone_exprs.contains(&z_ref) {
                zone_exprs.push(z_ref);
            }
        }
        if zone_exprs.is_empty() {
            return None;
        }
        let zone_filter = match zone_exprs.as_slice() {
            [only] => (*only).to_string(),
            many => format!("Filter::Or(&[{}])", many.join(", ")),
        };

        // While the replacement's host is on the battlefield (`ActiveZones$ Battlefield`),
        // cards in graveyards/libraries/exile/hand are inherently distinct from `Self`,
        // so `.Other` / `+Other` on `ValidLKI$` (`Creature.Other`) is stripped before
        // off-battlefield card-characteristic parsing.
        let cleaned = strip_other_atom_all_alts(&valid_raw)?;
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        let card_expr = if cleaned == "Card" {
            "Filter::Any".to_string()
        } else {
            let Some(expr) = self.filter_expr_in_zone(&cleaned, true) else {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                return None;
            };
            expr
        };
        let combined = if card_expr == "Filter::Any" {
            zone_filter
        } else {
            format!("Filter::And(&[{zone_filter}, {card_expr}])")
        };
        let filter_name = self.body.filter_static("CANT_ENTER", &combined);
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!("Modifier::CantEnterBattlefield(&{filter_name})"),
        ));
        Some(())
    }

    /// Additional `static_pt_amount` atoms (`Greven, Predator Captain`
    /// `Count$LifeYouLostThisTurn`, `Count$CardCounters.<KIND>`,
    /// `Kelsien, the Plague` `Count$YourCountersExperience`).
    pub(super) fn wave29d_static_pt_amount(body: &str) -> Option<String> {
        let trimmed = body.trim();
        match trimmed {
            "Count$LifeYouLostThisTurn" | "PlayerCountPropertyYou$LifeLostThisTurn" => {
                Some("Amount::LifeLostThisTurn(PlayerRel::You)".to_string())
            }
            "Count$LifeYouGainedThisTurn" | "PlayerCountPropertyYou$LifeGainedThisTurn" => {
                Some("Amount::LifeGainedThisTurn(PlayerRel::You)".to_string())
            }
            _ if trimmed.starts_with("Count$YourCounters") => {
                let kind_raw = trimmed.strip_prefix("Count$YourCounters")?.trim();
                if kind_raw.is_empty() || kind_raw.contains(['/', '$', ' ']) {
                    return None;
                }
                let kind = counter_kind(kind_raw)
                    .or_else(|| counter_kind(&kind_raw.to_ascii_uppercase()))?;
                Some(format!("Amount::CountersOnYou({kind})"))
            }
            _ => {
                let kind_raw = trimmed.strip_prefix("Count$CardCounters.")?;
                if kind_raw.contains(['/', '$', ' ']) {
                    return None;
                }
                let kind = counter_kind(kind_raw.trim())?;
                Some(format!("Amount::CountersOnSource({kind})"))
            }
        }
    }

    pub(super) fn wave29d_is_second_basic_land_choice(&self, sub_name: &str) -> bool {
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        api == "ChooseType"
            && matches!(p.take("Defined").as_deref(), None | Some("You"))
            && p.take("Type").as_deref() == Some("Basic Land")
            && p.take("ChooseType2").as_deref() == Some("True")
            && p.exhausted()
    }

    /// Additional `CheckSVar$` conditions for `Phase` / general triggers:
    /// - `Lunar Convocation`: `PlayerCountPropertyYou$LifeLostThisTurn/Times.<var>`
    ///   where `<var>` is `Count$LifeYouGainedThisTurn` ("if you gained and lost
    ///   life this turn").
    /// - `Prairie Dog`: `Count$ThisTurnCast_...` / `Count$ThisTurnEntered_...`
    ///   via `wave27a_count_expr`.
    pub(super) fn wave29d_svar_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        let raw = raw.trim();
        if let Some(rhs_var) = raw
            .strip_prefix("PlayerCountPropertyYou$LifeLostThisTurn/Times.")
            .or_else(|| raw.strip_prefix("Count$LifeYouLostThisTurn/Times."))
            && matches!(
                self.svars.get(rhs_var.trim()).map(|s| s.trim()),
                Some("Count$LifeYouGainedThisTurn" | "PlayerCountPropertyYou$LifeGainedThisTurn")
            )
        {
            let both = "Condition::All(&[\
                        Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 1), \
                        Condition::AmountAtLeast(Amount::LifeLostThisTurn(PlayerRel::You), 1)])";
            return match cmp {
                "GE1" | "GT0" | "NE0" => Some(both.to_string()),
                "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{both})")),
                _ => None,
            };
        }
        if (raw.starts_with("Count$ThisTurnCast_") || raw.starts_with("Count$ThisTurnEntered_"))
            && let Some(amt) = self.wave27a_count_expr(raw, 0, false)
        {
            return Self::wave15_amount_cmp(&amt, cmp);
        }
        None
    }

    /// `ChooseCard | Choices$ <filter> | Mandatory$ True -> DB$ Effect |
    /// ReplacementEffects$ <R> | RememberObjects$ ChosenCard` (*Ajani's Aid*).
    pub(super) fn wave29d_choose_card_prevent_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        if chain.target.is_some() || chain.second_target.is_some() {
            return None;
        }
        p.drop_prose();
        if p.take("Mandatory").as_deref() != Some("True")
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || !matches!(p.take("Amount").as_deref(), None | Some("1"))
        {
            return None;
        }
        let choices = p.take("Choices")?;
        if !p.exhausted() {
            return None;
        }
        let sub_name = sub?.trim();
        let eff_body = self.svars.get(sub_name)?.clone();
        let (eff_api, mut eff) = Params::parse(&eff_body)?;
        eff.drop_prose();
        if eff_api != "Effect"
            || eff.take("RememberObjects").as_deref() != Some("ChosenCard")
            || !matches!(
                eff.take("Duration").as_deref(),
                None | Some("UntilEndOfTurn")
            )
            || !matches!(
                eff.take("ExileOnMoved").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(
                eff.take("ForgetOnMoved").as_deref(),
                None | Some("Battlefield")
            )
        {
            return None;
        }
        let rep_name = eff.take("ReplacementEffects")?;
        let trailing = match eff.take("SubAbility") {
            None => None,
            Some(ref clean) if self.is_wave8_cleanup_svar(clean, false) => None,
            Some(other) => Some(other),
        };
        if !eff.exhausted() {
            return None;
        }
        let rep_body = self.svars.get(rep_name.trim())?.clone();
        let (event, mut shield) = Params::parse(&rep_body)?;
        shield.drop_prose();
        if event != "DamageDone"
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
            || !matches!(
                shield.take("ValidSource").as_deref(),
                Some("Card.IsRemembered" | "Creature.IsRemembered" | "Permanent.IsRemembered")
            )
        {
            return None;
        }
        let target_expr = match shield.take("ValidTarget").as_deref() {
            None => "None",
            Some("You") => "Some(TargetSpec::Player(PlayerRel::You))",
            _ => return None,
        };
        let combat_only = match shield.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let next_only = if shield.take("Prevent").as_deref() == Some("True") {
            false
        } else if shield.take("PreventionEffect").as_deref() == Some("True")
            && shield
                .take("ReplaceWith")
                .is_some_and(|rw| self.exiles_itself(&rw))
        {
            true
        } else {
            return None;
        };
        if !shield.exhausted() {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        let Some(filter_expr) = self.filter_expr(&choices) else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        let src_ref = self.body.filter_static("SOURCE", &filter_expr);
        chain.effects.push(format!(
            "Effect::PreventFromChosenSource {{ target: {target_expr}, sources: &{src_ref}, \
             next_only: {next_only}, combat_only: {combat_only}, all_but: 0, gain_life: false }}"
        ));
        self.chain_sub_ability(trailing, chain)
    }

    /// `NameCard | Defined$ You -> Reveal | Random$ True | RememberRevealed$ True ->
    /// Discard | DiscardValid$ Card.NamedCard+IsRemembered -> Cleanup`
    /// (*Dementia Sliver*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29d_try_name_card_reveal_random(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        if chain.second_target.is_some() {
            return None;
        }
        let mut np = p.clone();
        np.drop_prose();
        if np.take("Defined").as_deref() != Some("You") || np.has("ValidCards") || !np.exhausted() {
            return None;
        }
        let rev_name = sub?.trim();
        let rev_body = self.svars.get(rev_name)?.clone();
        let (rev_api, mut rp) = Params::parse(&rev_body)?;
        rp.drop_prose();
        if rev_api != "Reveal"
            || rp.take("Random").as_deref() != Some("True")
            || rp.take("RememberRevealed").as_deref() != Some("True")
        {
            return None;
        }
        let saved_target = chain.target.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        rp.claim_label("ValidTgtsDesc", "ValidTgts");
        let rev_targets_here = if let Some(valid) = rp.take("ValidTgts") {
            if self
                .chain_target("Reveal", &mut rp, &valid, chain, None)
                .is_none()
            {
                chain.target = saved_target;
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                return None;
            }
            true
        } else {
            false
        };
        let eff_target = Self::effect_target(chain);
        let rev_targets_player = rev_targets_here
            && eff_target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let Some(who) = self.wave11_player_of_line(
            rp.take("Defined").as_deref(),
            eff_target.as_deref(),
            rev_targets_player,
        ) else {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        let count = match rp.take("NumCards") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => {
                if let Some(c) = self.amount_or_count_ext(&raw) {
                    c
                } else {
                    chain.target = saved_target;
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                }
            }
        };
        let Some(next_name) = rp.take("SubAbility") else {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        if !rp.exhausted() {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        }
        let Some(disc_body) = self.svars.get(next_name.trim()).cloned() else {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        let Some((disc_api, mut dp)) = Params::parse(&disc_body) else {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        dp.drop_prose();
        let valid_discard = dp.take("DiscardValid");
        let mode_ok = matches!(
            dp.take("Mode").as_deref(),
            Some("TgtChoose" | "RevealYouChoose")
        );
        let num_ok = matches!(dp.take("NumCards").as_deref(), None | Some("1"));
        let disc_who =
            self.wave11_player_of_line(dp.take("Defined").as_deref(), eff_target.as_deref(), false);
        let cleanup_ok = match dp.take("SubAbility") {
            None => true,
            Some(clean) => self.wave29d_is_cleanup_only(&clean),
        };
        if disc_api != "Discard"
            || !matches!(
                valid_discard.as_deref(),
                Some("Card.NamedCard+IsRemembered" | "Card.IsRemembered+NamedCard")
            )
            || !mode_ok
            || !num_ok
            || disc_who != Some(who)
            || !cleanup_ok
            || !dp.exhausted()
        {
            chain.target = saved_target;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        }
        chain.effects.push(format!(
            "Effect::RevealRandomDiscardNamed {{ player: {who}, count: {count} }}"
        ));
        Some(())
    }

    fn wave29d_is_cleanup_only(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut cp)) = Params::parse(body) else {
            return false;
        };
        cp.drop_prose();
        if api != "Cleanup" {
            return false;
        }
        cp.take("ClearRemembered");
        cp.take("ClearNamedCard");
        cp.exhausted()
    }

    /// `Reveal | Random$ True` -> `Effect::RevealRandomFromHand` (*Planeswalker's
    /// Favor*, *Planeswalker's Fury*, *Planeswalker's Mirth*, *Planeswalker's
    /// Scorn*, *Ignite Memories*, *Singe-Mind Ogre*, *Friendly Fire*, *Hired
    /// Torturer*, *Merfolk Spy*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29d_reveal_random_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        if p.take("Random").as_deref() != Some("True") {
            return self.deny("`Reveal` with unsupported `Random$`".to_string());
        }
        let mut who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if p.take("AnyNumber").is_some_and(|v| v != "False") {
            return self.deny("`Reveal` with both `Random$` and `AnyNumber$`".to_string());
        }
        if p.take("Optional").is_some_and(|v| v != "False") {
            return self.deny("`Reveal` with both `Random$` and `Optional$`".to_string());
        }
        let remember = match p.take("RememberRevealed").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Reveal` with `RememberRevealed$ {other}`")),
        };
        let count = match p.take("NumCards") {
            None => "Amount::Fixed(1)".to_string(),
            Some(r) => self.amount_or_count_ext(&r)?,
        };
        if let Some(valid) = p.take("RevealValid") {
            let cleaned = if who == "PlayerRel::You" {
                valid
                    .trim()
                    .strip_suffix(".YouOwn")
                    .or_else(|| valid.trim().strip_suffix(".YouCtrl"))
                    .unwrap_or(valid.trim())
            } else {
                valid.trim()
            };
            if cleaned != "Card" {
                return self.deny(format!("filtered random `Reveal` `{valid}`"));
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Reveal.{key}`"));
        }

        if !remember {
            if who == "PlayerRel::You" {
                return self.deny("self random `Reveal` without `RememberRevealed$`".to_string());
            }
            let reveal_eff =
                format!("Effect::RevealRandomFromHand {{ who: {who}, count: {count} }}");
            chain.effects.push(match outer_cond {
                Some(c) => format!(
                    "Effect::IfCondition {{ condition: {c}, then: &[{reveal_eff}], otherwise: &[] }}"
                ),
                None => reveal_eff,
            });
            return self.chain_sub_ability(sub.map(str::to_string), chain);
        }

        let Some(sub_name) = sub else {
            return self.deny("`Reveal` `RememberRevealed` with no `SubAbility$`".to_string());
        };
        if count != "Amount::Fixed(1)" {
            return self.deny("multi-card random `Reveal` with `RememberRevealed$`".to_string());
        }
        let (followup_effs, trailing) = self.wave29d_reveal_random_mv_followup(sub_name, chain)?;
        if chain.second_target.is_some() && who == "PlayerRel::Chosen" {
            who = "PlayerRel::FirstTargetPlayerOrController";
        }
        let reveal_eff = format!("Effect::RevealRandomFromHand {{ who: {who}, count: {count} }}");
        if let Some(c) = outer_cond {
            let mut all = vec![reveal_eff];
            all.extend(followup_effs);
            chain.effects.push(format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{}], otherwise: &[] }}",
                all.join(", ")
            ));
        } else {
            chain.effects.push(reveal_eff);
            chain.effects.extend(followup_effs);
        }
        self.chain_sub_ability(trailing, chain)
    }

    fn wave29d_revealed_mv_amount(&self, raw: &str) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        if let Ok(n) = key.parse::<u32>() {
            return Some((format!("Amount::Fixed({n})"), neg));
        }
        let def = self.svars.get(key)?.trim();
        let (head, suffix) = def.split_once('/').map_or((def, ""), |(h, s)| (h, s));
        let base = match head {
            "Remembered$CardManaCost"
            | "RememberedLKI$CardManaCost"
            | "Remembered$TotalManaCost" => {
                "Amount::ThisWay(ThisWay::RevealedManaValue)".to_string()
            }
            _ => return None,
        };
        let mag = Self::wrap_amount_suffix(base, suffix)?;
        Some((mag, neg))
    }

    #[allow(clippy::too_many_lines)]
    fn wave29d_reveal_random_mv_followup(
        &mut self,
        sub_name: &str,
        chain: &mut Chain,
    ) -> Option<(Vec<String>, Option<String>)> {
        let Some(body) = self.svars.get(sub_name.trim()).cloned() else {
            return self.deny(format!("`SubAbility$ {sub_name}` names no SVar"));
        };
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        let had_first_target = chain.target.is_some();
        let had_second_target = chain.second_target.is_some();
        let saved_target = chain.target.clone();
        let saved_second = chain.second_target.clone();

        let cond_remembered = if sp.peek("ConditionDefined") == Some("Remembered")
            && !sp.has("ConditionPresent")
            && !sp.has("ConditionCheckSVar")
        {
            sp.take("ConditionDefined");
            true
        } else {
            false
        };

        let next = sp.take("SubAbility");
        let trailing = match next {
            None => None,
            Some(ref clean) if self.wave29d_is_cleanup_only(clean) => None,
            other => other,
        };

        sp.claim_label("ValidTgtsDesc", "ValidTgts");
        let targets_here = if let Some(valid) = sp.take("ValidTgts") {
            if self
                .chain_target(&api, &mut sp, &valid, chain, None)
                .is_none()
            {
                chain.target = saved_target;
                chain.second_target = saved_second;
                return None;
            }
            true
        } else {
            false
        };
        let second_target_here =
            had_first_target && !had_second_target && chain.second_target.is_some();
        let eff_target = Self::effect_target(chain);
        let targets_a_player =
            targets_here && eff_target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");

        let mut effs = match api.as_str() {
            "DealDamage" => {
                let raw = sp.take("NumDmg")?;
                let (amt, neg) = self.wave29d_revealed_mv_amount(&raw)?;
                let defined = sp.take("Defined");
                if neg || !sp.exhausted() {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self
                        .deny("unreadable random `Reveal` `DealDamage` follow-up".to_string());
                }
                let dmg_target = match defined.as_deref() {
                    None if targets_here => eff_target.clone()?,
                    Some("Targeted" | "ParentTarget") => eff_target.clone()?,
                    Some(other) => {
                        let rel = self.wave11_player_of_line(
                            Some(other),
                            eff_target.as_deref(),
                            targets_a_player,
                        )?;
                        format!("TargetSpec::Player({rel})")
                    }
                    None => {
                        chain.target = saved_target;
                        chain.second_target = saved_second;
                        return self
                            .deny("random `Reveal` `DealDamage` with no recipient".to_string());
                    }
                };
                vec![format!(
                    "Effect::DealDamage {{ amount: {amt}, target: {dmg_target} }}"
                )]
            }
            "DamageAll" => {
                // Friendly Fire: `ValidPlayers$ TargetedController | ValidCards$ Targeted.Self`
                let raw = sp.take("NumDmg")?;
                let (amt, neg) = self.wave29d_revealed_mv_amount(&raw)?;
                let valid_players = sp.take("ValidPlayers");
                let valid_cards = sp.take("ValidCards");
                if neg
                    || valid_players.as_deref() != Some("TargetedController")
                    || !matches!(valid_cards.as_deref(), Some("Targeted" | "Targeted.Self"))
                    || !eff_target
                        .as_deref()
                        .is_some_and(|t| t.starts_with("TargetSpec::Object("))
                    || !sp.exhausted()
                {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self
                        .deny("unreadable random `Reveal` `DamageAll` follow-up".to_string());
                }
                let obj_tgt = eff_target.clone()?;
                vec![
                    format!("Effect::DealDamage {{ amount: {amt}, target: {obj_tgt} }}"),
                    format!(
                        "Effect::DealDamage {{ amount: {amt}, \
                         target: TargetSpec::Player(PlayerRel::ControllerOfTarget) }}"
                    ),
                ]
            }
            "GainLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, neg) = self.wave29d_revealed_mv_amount(&raw)?;
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    eff_target.as_deref(),
                    targets_a_player,
                )?;
                if neg || !sp.exhausted() {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self
                        .deny("unreadable random `Reveal` `GainLife` follow-up".to_string());
                }
                if who == "PlayerRel::You" {
                    vec![format!("Effect::GainLife {{ amount: {amt} }}")]
                } else {
                    vec![format!(
                        "Effect::GainLifeFor {{ amount: {amt}, who: {who} }}"
                    )]
                }
            }
            "LoseLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, neg) = self.wave29d_revealed_mv_amount(&raw)?;
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    eff_target.as_deref(),
                    targets_a_player,
                )?;
                if neg || !sp.exhausted() {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self
                        .deny("unreadable random `Reveal` `LoseLife` follow-up".to_string());
                }
                vec![format!(
                    "Effect::LoseLife {{ amount: {amt}, target: {who} }}"
                )]
            }
            "Pump" => {
                let att_raw = sp.take("NumAtt");
                let def_raw = sp.take("NumDef");
                sp.take("IsCurse");
                let kw_expr = match sp.take("KW") {
                    None => "KeywordSet::EMPTY".to_string(),
                    Some(raw_kw) => {
                        let mut bits = Vec::new();
                        for part in raw_kw.split('&').map(str::trim) {
                            let Some(bit) = keyword_const(part) else {
                                chain.target = saved_target;
                                chain.second_target = saved_second;
                                return self
                                    .deny(format!("random `Reveal` `Pump` keyword `{part}`"));
                            };
                            bits.push(bit.to_string());
                        }
                        bits.split_first().map_or_else(
                            || "KeywordSet::EMPTY".to_string(),
                            |(h, tail)| {
                                tail.iter()
                                    .fold(h.clone(), |acc, b| format!("{acc}.union({b})"))
                            },
                        )
                    }
                };
                let duration = match sp.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => {
                        chain.target = saved_target;
                        chain.second_target = saved_second;
                        return self.deny("random `Reveal` `Pump` duration".to_string());
                    }
                };
                let parse_pt = |opt: Option<String>| -> Option<String> {
                    let Some(s) = opt else {
                        return Some("Amount::Fixed(0)".to_string());
                    };
                    let (mag, neg) = self.wave29d_revealed_mv_amount(&s)?;
                    Some(if neg {
                        if let Some(n) = mag
                            .strip_prefix("Amount::Fixed(")
                            .and_then(|r| r.strip_suffix(')'))
                        {
                            format!("Amount::NegXFixed({n})")
                        } else {
                            format!("Amount::Negated(&{mag})")
                        }
                    } else {
                        mag
                    })
                };
                let Some(power) = parse_pt(att_raw) else {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self.deny("random `Reveal` `Pump` `NumAtt$`".to_string());
                };
                let Some(toughness) = parse_pt(def_raw) else {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self.deny("random `Reveal` `Pump` `NumDef$`".to_string());
                };
                let defined = sp.take("Defined");
                if !sp.exhausted() {
                    chain.target = saved_target;
                    chain.second_target = saved_second;
                    return self.deny("unclaimed parameter in random `Reveal` `Pump`".to_string());
                }
                match (defined.as_deref(), eff_target.as_deref()) {
                    (None | Some("Targeted"), Some(t)) if t.starts_with("TargetSpec::Object(") => {
                        vec![format!(
                            "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                             keywords: {kw_expr}, duration: {duration} }}"
                        )]
                    }
                    (None | Some("Self"), None) if !self.on_a_spell => vec![format!(
                        "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                         power: {power}, toughness: {toughness}, keywords: {kw_expr}, \
                         duration: {duration} }}"
                    )],
                    _ => {
                        chain.target = saved_target;
                        chain.second_target = saved_second;
                        return self.deny("random `Reveal` `Pump` recipient".to_string());
                    }
                }
            }
            other => {
                chain.target = saved_target;
                chain.second_target = saved_second;
                return self.deny(format!("random `Reveal` follow-up `{other}`"));
            }
        };

        if second_target_here {
            effs = vec![format!("Effect::WithSecondTarget(&[{}])", effs.join(", "))];
        }
        if cond_remembered {
            effs = vec![format!(
                "Effect::IfCondition {{ condition: Condition::AmountAtLeast(\
                 Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), 1), \
                 then: &[{}], otherwise: &[] }}",
                effs.join(", ")
            )];
        }
        Some((effs, trailing))
    }

    /// `T: Mode$ ChangesZone | ... | OptionalDecider$ TriggeredCardController`
    /// where the executed line has `TargetingPlayer$ TriggeredCardController`
    /// (*Death Match*, *Pandemonium*).
    pub(super) fn wave29d_event_controller_may_trigger(
        &mut self,
        mode: &str,
        trigger: &str,
        execute: &str,
        zone: &str,
        once_per_turn: &str,
        condition: &str,
    ) -> Option<()> {
        if mode != "ChangesZone" || !trigger.starts_with("Trigger::EntersBattlefield(") {
            return None;
        }
        let body = self.svars.get(execute)?.clone();
        let (api, mut p) = Params::parse(&body)?;
        p.drop_prose();
        if p.take("TargetingPlayer").as_deref() != Some("TriggeredCardController") {
            return None;
        }
        let valid_tgts = p.take("ValidTgts")?;
        if p.has("TargetMin") || p.has("TargetMax") || p.has("SubAbility") {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        let Some(spec) = self.target_spec(&valid_tgts, &api) else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            return None;
        };
        let inner_eff = match api.as_str() {
            "Pump" => {
                if !spec.starts_with("TargetSpec::Object(") {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                }
                let mut probe = p.clone();
                let Some(effs) = self.pump_effect(&mut probe, &spec, true) else {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                };
                if !probe.exhausted() || effs.len() != 1 {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                }
                effs.into_iter().next()?
            }
            "DealDamage" => {
                let num_dmg = p.take("NumDmg")?;
                let dmg_src = p.take("DamageSource")?;
                if !matches!(dmg_src.as_str(), "TriggeredCard" | "TriggeredCardLKICopy")
                    || !p.exhausted()
                {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                }
                let dmg_def = self
                    .svars
                    .get(num_dmg.trim())
                    .map_or(num_dmg.trim(), |s| s.trim());
                if matches!(
                    dmg_def,
                    "TriggeredCard$CardPower" | "TriggeredCardLKICopy$CardPower"
                ) {
                    let eff_tgt = Self::effect_target_of(&spec);
                    format!("Effect::EventObjectDealsDamageEqualToPower {{ target: {eff_tgt} }}")
                } else if let Ok(n) = dmg_def.parse::<u32>() {
                    let eff_tgt = Self::effect_target_of(&spec);
                    format!(
                        "Effect::EventObjectDealsDamage {{ amount: Amount::Fixed({n}), \
                         target: {eff_tgt} }}"
                    )
                } else {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics_len);
                    return None;
                }
            }
            _ => {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics_len);
                return None;
            }
        };
        self.body.abilities.push(format!(
            "triggered!({trigger}, &[Effect::MayDoFor {{ player: PlayerRel::ControllerOfEvent, \
             effects: &[{inner_eff}] }}]{zone}{once_per_turn}, \
             targets = Some(TargetReq::one({spec}).chosen_by_event_player()){condition})"
        ));
        Some(())
    }

    /// `Oath of Lieges`: `T: Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player`
    /// -> `DB$ ChangeZone | TargetingPlayer$ TriggeredPlayer | ValidTgts$
    /// Player.OpponentOf TriggeredPlayer+withMoreLandsThanTriggeredPlayer |
    /// Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic |
    /// Optional$ True`.
    pub(super) fn wave29d_oath_of_lieges_trigger(
        &mut self,
        mode: &str,
        trigger: &str,
        body: &str,
        zone: &str,
        once_per_turn: &str,
        condition: &str,
    ) -> Option<()> {
        if mode != "Phase"
            || trigger
                != "Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }"
        {
            return None;
        }
        let (api, mut p) = Params::parse(body)?;
        p.drop_prose();
        p.claim_label("ChangeTypeDesc", "ChangeType");
        if api != "ChangeZone"
            || p.take("TargetingPlayer").as_deref() != Some("TriggeredPlayer")
            || p.take("ValidTgts").as_deref()
                != Some("Player.OpponentOf TriggeredPlayer+withMoreLandsThanTriggeredPlayer")
            || p.take("DefinedPlayer").as_deref() != Some("TriggeredPlayer")
            || !matches!(p.take("Chooser").as_deref(), None | Some("TriggeredPlayer"))
            || p.take("Origin").as_deref() != Some("Library")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ChangeType").as_deref() != Some("Land.Basic")
            || p.take("Optional").as_deref() != Some("True")
            || !matches!(
                p.take("ShuffleNonMandatory").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !p.exhausted() {
            return None;
        }
        let eff = if tapped {
            "Effect::OptionalBasicLandSearchFor { player: PlayerRel::EventPlayer }"
        } else {
            "Effect::SearchLibraryOf { library: PlayerRel::EventPlayer, owner_searches: true, filter: &Filter::BASIC_LAND, mana_value: None, finds: &[Find::BATTLEFIELD], optional: true }"
        };
        self.body.abilities.push(format!(
            "triggered!({trigger}, &[{eff}]{zone}{once_per_turn}, \
             targets = Some(TargetReq::one(\
             TargetSpec::OpponentWithMoreThanActivePlayer(&Filter::LAND))\
             .chosen_by_event_player()){condition})"
        ));
        Some(())
    }

    /// `Counter | ... | TargetValidTargeting$ <Permanent>.YouCtrl[+inRealZoneBattlefield]`
    /// (*Hydromorph Guardian*, *Hydromorph Gull*, etc.) -> `Filter::TargetsYouControl(&...)`
    /// or `Filter::TargetsPermanentYouControl`.
    pub(super) fn wave29d_counter_target_valid_targeting(
        &mut self,
        valid: &str,
        p: &mut Params,
    ) -> Option<String> {
        if p.has("TargetMin")
            || p.has("TargetMax")
            || p.has("TargetUnique")
            || p.has("DestroyPermanent")
        {
            return None;
        }
        let raw_tv = p.peek("TargetValidTargeting")?.to_string();
        let cleaned_tv = strip_battlefield_you_ctrl_alts(&raw_tv)?;
        let (wrapper_kind, tt_ctrl) = match p.peek("TargetType") {
            None | Some("Spell") => ("Spell", None),
            Some("Spell.OppCtrl") => ("Spell", Some("Filter::ControlledByOpponent")),
            Some("Spell.YouDontCtrl") => ("Spell", Some("Filter::Not(&Filter::ControlledByYou)")),
            Some("Spell,Activated,Triggered") => ("SpellOrAbility", None),
            Some("Spell.OppCtrl,Activated.OppCtrl,Triggered.OppCtrl") => {
                ("SpellOrAbility", Some("Filter::ControlledByOpponent"))
            }
            Some("Spell.YouDontCtrl,Activated.YouDontCtrl,Triggered.YouDontCtrl") => (
                "SpellOrAbility",
                Some("Filter::Not(&Filter::ControlledByYou)"),
            ),
            Some("Activated,Triggered") => ("AbilityOnStack", None),
            Some("Activated.OppCtrl,Triggered.OppCtrl") => {
                ("AbilityOnStack", Some("Filter::ControlledByOpponent"))
            }
            Some("Activated.YouDontCtrl,Triggered.YouDontCtrl") => (
                "AbilityOnStack",
                Some("Filter::Not(&Filter::ControlledByYou)"),
            ),
            _ => return None,
        };
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        let rollback = |tx: &mut Self| {
            (*tx.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
            tx.body.statics.truncate(saved_statics_len);
        };
        let valid_clause = match valid.trim() {
            "Card" | "Card,Emblem" | "Emblem,Card" => None,
            "Card.OppCtrl" => Some("Filter::ControlledByOpponent".to_string()),
            "Card.YouDontCtrl" => Some("Filter::Not(&Filter::ControlledByYou)".to_string()),
            other
                if wrapper_kind == "Spell" && !other.contains("Ctrl") && !other.contains("Own") =>
            {
                let Some(e) = self.filter_expr_in_zone(other, true) else {
                    rollback(self);
                    return None;
                };
                Some(e)
            }
            _ => return None,
        };
        let targets_clause = if matches!(cleaned_tv.as_str(), "Permanent" | "Card") {
            "Filter::TargetsPermanentYouControl".to_string()
        } else {
            let Some(expr) = self.filter_expr(&cleaned_tv) else {
                rollback(self);
                return None;
            };
            if expr.starts_with("Filter::") && !expr.contains([' ', '(', '&']) {
                format!("Filter::TargetsYouControl(&{expr})")
            } else {
                let inner_name = self.body.filter_static("TARGET_PERM", &expr);
                format!("Filter::TargetsYouControl(&{inner_name})")
            }
        };
        let mut clauses = Vec::new();
        if let Some(vc) = valid_clause {
            clauses.push(vc);
        }
        if let Some(tc) = tt_ctrl
            && !clauses.iter().any(|c| c == tc)
        {
            clauses.push(tc.to_string());
        }
        clauses.push(targets_clause);
        let combined = match clauses.as_slice() {
            [only] => only.clone(),
            many => format!("Filter::And(&[{}])", many.join(", ")),
        };
        p.take("TargetValidTargeting");
        let name = self.body.filter_static("TARGET", &combined);
        Some(format!("TargetSpec::{wrapper_kind}(&{name})"))
    }
}

fn strip_battlefield_you_ctrl_alts(raw: &str) -> Option<String> {
    let mut out = Vec::new();
    for alt in raw.split(',').map(str::trim) {
        if alt.is_empty() {
            return None;
        }
        let (base, rest) = alt.split_once('.').unwrap_or((alt, ""));
        if base.is_empty()
            || matches!(
                base,
                "You" | "Player" | "Opponent" | "Any" | "Self" | "Other"
            )
        {
            return None;
        }
        let mut saw_you_ctrl = false;
        let mut kept = Vec::new();
        for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            match atom {
                "YouCtrl" => saw_you_ctrl = true,
                "inRealZoneBattlefield" | "inZoneBattlefield" => {}
                a if a == "Self"
                    || a == "Other"
                    || a.contains("Remembered")
                    || a.contains("Ctrl")
                    || a.contains("Own")
                    || a.starts_with("inZone")
                    || a.starts_with("inRealZone") =>
                {
                    return None;
                }
                other => kept.push(other),
            }
        }
        if !saw_you_ctrl {
            return None;
        }
        if kept.is_empty() {
            out.push(base.to_string());
        } else {
            out.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!out.is_empty()).then(|| out.join(","))
}

fn strip_other_atom_all_alts(valid: &str) -> Option<String> {
    let mut out = Vec::new();
    for alt in valid.split(',').map(str::trim) {
        if alt.is_empty() {
            return None;
        }
        let (base, rest) = alt.split_once('.').unwrap_or((alt, ""));
        if base.is_empty() || base == "Self" || base == "Other" {
            return None;
        }
        let mut kept = Vec::new();
        for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            if atom == "Other" {
                continue;
            }
            if atom == "Self" || atom == "IsRemembered" || atom == "!IsRemembered" {
                return None;
            }
            kept.push(atom);
        }
        if kept.is_empty() {
            out.push(base.trim().to_string());
        } else {
            out.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!out.is_empty()).then(|| out.join(","))
}
