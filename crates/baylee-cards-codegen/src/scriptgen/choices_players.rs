//! Choices, player relations, untap-on-other-steps, combat tax/restriction,
//! secret/random numbers, join forces, and turn-history / counter conditions.

use super::Chain;
use super::atoms::{Bound, amount, count_bound, static_wave14_amount};
use super::saga_ward_convoke::ExtraOutcome;
use super::{Params, Tx, counter_kind};

impl Tx<'_> {
    /// Returns `true` for *Territorial Hellkite*'s Forge-internal `Static$ True`
    /// triggers tracking last-combat defending players, which the engine already
    /// maintains for `PlayerFilter::OpponentNotAttackedBySourceLastCombat`.
    pub(super) fn is_hellkite_combat_memory_trigger(&self, mode: &str, p: &Params) -> bool {
        if p.peek("Static") != Some("True")
            || !self
                .body
                .abilities
                .iter()
                .any(|a| a.contains("OpponentNotAttackedBySourceLastCombat"))
        {
            return false;
        }
        let Some((api, mut child)) = p
            .peek("Execute")
            .and_then(|name| self.svars.get(name.trim()))
            .and_then(|body| Params::parse(body))
        else {
            return false;
        };
        if api != "Cleanup" {
            return false;
        }
        match mode {
            "Attacks" => {
                p.peek("ValidCard") == Some("Card.Self")
                    && p.entries
                        .iter()
                        .all(|(k, _)| matches!(k.as_str(), "ValidCard" | "Execute" | "Static"))
                    && child.take("ClearRemembered").as_deref() == Some("True")
                    && child
                        .take("SubAbility")
                        .and_then(|s| self.svars.get(s.trim()))
                        .and_then(|b| Params::parse(b))
                        .is_some_and(|(s_api, mut sp)| {
                            s_api == "Bookkeeping"
                                && sp.take("RememberObjects").as_deref()
                                    == Some("TriggeredDefendingPlayer")
                                && sp.exhausted()
                        })
                    && child.exhausted()
            }
            "Phase" => {
                p.peek("Phase") == Some("EndCombat")
                    && p.peek("ValidPlayer") == Some("You")
                    && p.peek("TriggerZones") == Some("Battlefield")
                    && p.peek("IsPresent") == Some("Card.!attackedThisTurn+Self")
                    && p.entries.iter().all(|(k, _)| {
                        matches!(
                            k.as_str(),
                            "Phase"
                                | "ValidPlayer"
                                | "TriggerZones"
                                | "IsPresent"
                                | "Execute"
                                | "Static"
                        )
                    })
                    && child.take("ClearChosenPlayer").as_deref() == Some("True")
                    && child.take("ClearRemembered").as_deref() == Some("True")
                    && child.exhausted()
            }
            _ => false,
        }
    }

    pub(super) fn has_secret_chosen_opponent(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::SecretlyChooseOpponent")
    }

    pub(super) fn is_reveal_chosen_player_cost(&self, token: &str) -> bool {
        token == "RevealChosen<Player>"
            && !self.on_a_spell
            && !self.in_alternative_cost
            && self.has_secret_chosen_opponent()
    }

    // -------------------------------------------------------------------------
    // 1. `S: Mode$ UntapOtherPlayer` -> `Modifier::UntapDuringUntapStep`
    // -------------------------------------------------------------------------

    /// Lowers `Mode$ UntapOtherPlayer` into
    /// `Modifier::UntapDuringUntapStep { who }`.
    pub(super) fn statics_untap_other_player(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("static ability `S: Mode$ UntapOtherPlayer`".into());
        }
        let zone = self.take_static_effect_zone(&mut p, "UntapOtherPlayer")?;
        if !zone.is_battlefield() {
            return self.deny("static ability `S: Mode$ UntapOtherPlayer`".into());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("`UntapOtherPlayer` with no `ValidCard$`".into());
        };
        let who = match p.take("ValidPlayer").as_deref() {
            None => "PlayerRel::EachOtherPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            Some(other) => {
                return self.deny(format!("`UntapOtherPlayer.ValidPlayer$ {other}`"));
            }
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny(format!(
                "unclaimed parameter `UntapOtherPlayer.{}`",
                p.first_key().unwrap_or_default()
            ));
        }
        let filter = self.filter_expr(valid_card.trim())?;
        self.push_static_with_cond(
            &filter,
            &format!("Modifier::UntapDuringUntapStep {{ who: {who} }}"),
            &cond,
        );
        Some(())
    }

    /// Resolves `UntapOtherPlayer` inside `Effect | StaticAbilities$` (e.g.
    /// `EffectSubject::Unbound` temporary static or emblem static).
    pub(super) fn choices_static_effect_line(
        &mut self,
        mode: &str,
        st: &mut Params,
        subject: super::effects::EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if mode != "UntapOtherPlayer" || subject != super::effects::EffectSubject::Unbound {
            return None;
        }
        st.drop_prose();
        if !matches!(st.take("Secondary").as_deref(), None | Some("True")) {
            return None;
        }
        let valid_card = st.take("ValidCard")?;
        let who = match st.take("ValidPlayer").as_deref() {
            None => "PlayerRel::EachOtherPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            Some(_) => return None,
        };
        if !st.exhausted() {
            return None;
        }
        let filter_expr = self.filter_expr(valid_card.trim())?;
        let filter_ref = self.body.filter_static("EFFECT", &filter_expr);
        Some(vec![format!(
            "Effect::continuous(&{filter_ref}, Modifier::UntapDuringUntapStep {{ who: {who} }}, {duration})"
        )])
    }

    // -------------------------------------------------------------------------
    // 2. `S: Mode$ CantAttackUnless` / `CantBlockUnless`
    // -------------------------------------------------------------------------

    /// Extended `Mode$ CantAttackUnless` / `CantBlockUnless` lowering:
    /// - `AttackTaxTarget::{You, PlaneswalkersYouControl, YouOrYourPlaneswalkers, AnyDefender}`
    /// - `Modifier::AttackManaTax`, `Modifier::AttackPhyrexianTax`, `Modifier::BlockManaTax`
    /// - `Modifier::CantAttackUnlessYouReturn`, `Modifier::CantAttackUnlessYouTap`,
    ///   `Modifier::CantBlockUnlessYouTap`
    pub(super) fn choices_combat_unless(&mut self, mode: &str, mut p: Params) -> Option<()> {
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
        let remembering_attacker = match p.take("RememberingAttacker").as_deref() {
            None => false,
            Some("True") if mode == "CantAttackUnless" => true,
            Some(_) => return self.deny(format!("static ability `S: Mode$ {mode}`")),
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let filter = self.filter_expr(&valid_card)?;
        match mode {
            "CantAttackUnless" => self.choices_cant_attack_unless(
                &filter,
                cost.trim(),
                target.as_deref(),
                attacker.as_deref(),
                remembering_attacker,
                &cond,
            ),
            "CantBlockUnless" if !remembering_attacker => self.choices_cant_block_unless(
                &filter,
                cost.trim(),
                target.as_deref(),
                attacker.as_deref(),
                &cond,
            ),
            _ => self.deny(format!("static ability `S: Mode$ {mode}`")),
        }
    }

    fn attack_tax_target(raw: &str) -> Option<&'static str> {
        let mut parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        parts.sort_unstable();
        match parts.as_slice() {
            ["You"] => Some("AttackTaxTarget::You"),
            ["Planeswalker.YouCtrl"] => Some("AttackTaxTarget::PlaneswalkersYouControl"),
            ["Planeswalker.YouCtrl", "You"] => Some("AttackTaxTarget::YouOrYourPlaneswalkers"),
            _ => None,
        }
    }

    fn phyrexian_tax_color(cost: &str) -> Option<&'static str> {
        match cost {
            "WP" => Some("ManaColor::White"),
            "UP" => Some("ManaColor::Blue"),
            "BP" => Some("ManaColor::Black"),
            "RP" => Some("ManaColor::Red"),
            "GP" => Some("ManaColor::Green"),
            _ => None,
        }
    }

    fn parse_tap_cost_filter(&mut self, raw: &str, combat_flag: &str) -> Option<(u8, String)> {
        let (count_raw, filter_raw) = raw.split_once('/')?;
        let count: u8 = count_raw.trim().parse().ok().filter(|&n| n > 0)?;
        let stripped = Self::strip_combat_restriction_atom(filter_raw.trim(), combat_flag)?;
        let filter_expr = self.filter_expr(&stripped)?;
        let filter_ref = if Self::is_simple_filter_const(&filter_expr) {
            filter_expr
        } else {
            self.body.filter_static("COMBAT_TAX", &filter_expr)
        };
        Some((count, filter_ref))
    }

    fn strip_combat_restriction_atom(raw: &str, combat_flag: &str) -> Option<String> {
        if raw.contains(',') {
            return None;
        }
        let (base, qual_raw) = raw.split_once('.').unwrap_or((raw, ""));
        let mut kept: Vec<&str> = Vec::new();
        let mut removed = false;
        for atom in qual_raw.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            if atom == combat_flag {
                removed = true;
            } else {
                kept.push(atom);
            }
        }
        if !removed {
            return None;
        }
        if kept.is_empty() {
            Some(base.trim().to_string())
        } else {
            Some(format!("{}.{}", base.trim(), kept.join("+")))
        }
    }

    fn is_simple_filter_const(expr: &str) -> bool {
        matches!(
            expr,
            "Filter::Any"
                | "Filter::This"
                | "Filter::CREATURE"
                | "Filter::ARTIFACT"
                | "Filter::ENCHANTMENT"
                | "Filter::LAND"
                | "Filter::PLANESWALKER"
                | "Filter::YOUR_CREATURE"
                | "Filter::OPPONENT_CREATURE"
                | "Filter::YOUR_LAND"
                | "Filter::YOUR_BASIC_LAND"
                | "Filter::YOUR_ARTIFACT"
                | "Filter::ATTACKING_CREATURE"
                | "Filter::BLOCKING_CREATURE"
        )
    }

    /// Lowers a variable mana tax `Amount` (`Sphere of Safety`, `Collective
    /// Restraint`, `Cowed by Wisdom`, `Whipgrass Entangler`, `Nils`).
    /// Never admits `Count$ChosenNumber` / `SetChosenNumber$` (`War Tax` / `War Cadence`).
    fn choices_static_tax_amount(
        &mut self,
        cost: &str,
        remembering_attacker: bool,
    ) -> Option<String> {
        let def = self
            .svars
            .get(cost.trim())
            .map_or(cost.trim(), String::as_str)
            .trim()
            .to_string();
        if def.contains("ChosenNumber") || def == "Count$xPaid" {
            return None;
        }
        if remembering_attacker {
            return (def == "Remembered$CardCounters.ALL")
                .then(|| "Amount::TotalCountersOnSubject".to_string());
        }
        if def == "Count$Domain" {
            return Some("Amount::Domain(PlayerRel::You)".to_string());
        }
        if let Some(expr) = self.choices_pure_amount_expr(&def, 0) {
            return Some(expr);
        }
        if let Some(expr) = static_wave14_amount(&def, self.svars) {
            return Some(expr);
        }
        self.wave14_static_amount(cost)
            .or_else(|| self.wave27a_static_amount(cost, false, false))
            .or_else(|| self.counted_amount(cost))
    }

    fn choices_targeted_attack_tax(
        &mut self,
        filter: &str,
        cost: &str,
        tgt_raw: &str,
        remembering_attacker: bool,
        cond: &str,
    ) -> Option<()> {
        let Some(tgt) = Self::attack_tax_target(tgt_raw) else {
            return self.deny("static ability `S: Mode$ CantAttackUnless`".into());
        };
        if let Ok(mana) = cost.parse::<u32>()
            && mana > 0
            && !remembering_attacker
        {
            let modifier = if tgt == "AttackTaxTarget::YouOrYourPlaneswalkers" {
                format!("Modifier::AttackTaxMana {{ mana: {mana} }}")
            } else {
                format!(
                    "Modifier::AttackManaTax {{ target: {tgt}, amount: Amount::Fixed({mana}) }}"
                )
            };
            self.push_static_with_cond(filter, &modifier, cond);
            return Some(());
        }
        if !remembering_attacker
            && tgt == "AttackTaxTarget::YouOrYourPlaneswalkers"
            && let Some(life) = super::protection52::parse_pay_life_u16(cost)
        {
            self.push_static_with_cond(
                filter,
                &format!("Modifier::AttackTaxLife {{ life: {life} }}"),
                cond,
            );
            return Some(());
        }
        if !remembering_attacker && let Some(color) = Self::phyrexian_tax_color(cost) {
            self.push_static_with_cond(
                filter,
                &format!("Modifier::AttackPhyrexianTax {{ target: {tgt}, color: {color} }}"),
                cond,
            );
            return Some(());
        }
        if let Some(amt) = self.choices_static_tax_amount(cost, remembering_attacker) {
            self.push_static_with_cond(
                filter,
                &format!("Modifier::AttackManaTax {{ target: {tgt}, amount: {amt} }}"),
                cond,
            );
            return Some(());
        }
        self.deny("static ability `S: Mode$ CantAttackUnless`".into())
    }

    fn choices_cant_attack_unless(
        &mut self,
        filter: &str,
        cost: &str,
        target: Option<&str>,
        attacker: Option<&str>,
        remembering_attacker: bool,
        cond: &str,
    ) -> Option<()> {
        if attacker.is_some() {
            return self.deny("static ability `S: Mode$ CantAttackUnless`".into());
        }
        if let Some(tgt_raw) = target {
            return self.choices_targeted_attack_tax(
                filter,
                cost,
                tgt_raw,
                remembering_attacker,
                cond,
            );
        }
        if remembering_attacker {
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
        if let Some(ret) = cost
            .strip_prefix("Return<")
            .and_then(|s| s.strip_suffix('>'))
            && let Some((count, ret_filter)) = self.parse_attack_sac(ret)
        {
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::CantAttackUnlessYouReturn {{ count: {count}, filter: &{ret_filter} }}"
                ),
                cond,
            );
            return Some(());
        }
        if let Some(tap) = cost
            .strip_prefix("tapXType<")
            .and_then(|s| s.strip_suffix('>'))
            && let Some((count, tap_filter)) = self.parse_tap_cost_filter(tap, "!attacking")
        {
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::CantAttackUnlessYouTap {{ count: {count}, filter: &{tap_filter} }}"
                ),
                cond,
            );
            return Some(());
        }
        if let Some(amt) = self.choices_static_tax_amount(cost, false) {
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::AttackManaTax {{ target: AttackTaxTarget::AnyDefender, amount: {amt} }}"
                ),
                cond,
            );
            return Some(());
        }
        self.deny("static ability `S: Mode$ CantAttackUnless`".into())
    }

    fn choices_cant_block_unless(
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
        if let Ok(mana) = cost.parse::<u32>() {
            if mana == 0 {
                return self.deny("static ability `S: Mode$ CantBlockUnless`".into());
            }
            if let Some(att_raw) = attacker {
                let att_expr = self.filter_expr(att_raw)?;
                let att_ref = self.body.filter_static("ATTACKER", &att_expr);
                self.push_static_with_cond(
                    filter,
                    &format!(
                        "Modifier::CantBlockMatchingUnlessPayMana {{ attacker: &{att_ref}, mana: {mana} }}"
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
            return Some(());
        }
        if attacker.is_some() {
            return self.deny("static ability `S: Mode$ CantBlockUnless`".into());
        }
        if let Some(tap) = cost
            .strip_prefix("tapXType<")
            .and_then(|s| s.strip_suffix('>'))
            && let Some((count, tap_filter)) = self.parse_tap_cost_filter(tap, "!blocking")
        {
            self.push_static_with_cond(
                filter,
                &format!(
                    "Modifier::CantBlockUnlessYouTap {{ count: {count}, filter: &{tap_filter} }}"
                ),
                cond,
            );
            return Some(());
        }
        if let Some(amt) = self.choices_static_tax_amount(cost, false) {
            self.push_static_with_cond(
                filter,
                &format!("Modifier::BlockManaTax {{ amount: {amt} }}"),
                cond,
            );
            return Some(());
        }
        self.deny("static ability `S: Mode$ CantBlockUnless`".into())
    }

    // -------------------------------------------------------------------------
    // 3. `K:ETBReplacement:Other:*` entry choices
    // -------------------------------------------------------------------------

    /// Handles `EnterModifier::{ChooseTwoPlayers, SecretlyChooseOpponent,
    /// OpponentChoosesCreatureType, NoteMostPrevalentCreatureTypeInOpponentLibrary}`.
    pub(super) fn choices_etb_other(&mut self, svar: &str) -> ExtraOutcome {
        let Some(body) = self.svars.get(svar).cloned() else {
            return ExtraOutcome::Pass;
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return ExtraOutcome::Pass;
        };
        if api != "ChoosePlayer" {
            return ExtraOutcome::Pass;
        }
        // `SecretlyChooseOpponent` (*Guardian Archon*, *Emissary of Grudges*)
        if p.peek("Secretly") == Some("True") {
            p.drop_prose();
            p.take("Secretly");
            p.take("ChoiceTitle");
            if !matches!(p.take("Defined").as_deref(), None | Some("You"))
                || !matches!(
                    p.take("Choices").as_deref(),
                    Some("Opponent" | "Player.Opponent")
                )
                || !p.exhausted()
            {
                return ExtraOutcome::Handled(self.deny(
                    "as-enters `ChoosePlayer` with `Secretly$ True` outside opponent choice".into(),
                ));
            }
            self.body
                .enter_modifiers
                .push("EnterModifier::SecretlyChooseOpponent".to_string());
            return ExtraOutcome::Handled(Some(()));
        }

        // `ChooseTwoPlayers` (*Bitter Feud*, *Sower of Discord*)
        if p.peek("Choices") == Some("Player")
            && p.peek("RememberChosen") == Some("True")
            && p.has("SubAbility")
        {
            p.drop_prose();
            p.take("Defined");
            p.take("Choices");
            p.take("RememberChosen");
            p.take("ChoiceTitle");
            let sub2 = p.take("SubAbility").unwrap_or_default();
            if !p.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("as-enters `ChooseTwoPlayers` with extra parameters".into()),
                );
            }
            let Some(body2) = self.svars.get(sub2.trim()).cloned() else {
                return ExtraOutcome::Handled(None);
            };
            let Some((api2, mut p2)) = Params::parse(&body2) else {
                return ExtraOutcome::Handled(None);
            };
            p2.drop_prose();
            if api2 != "ChoosePlayer"
                || !matches!(p2.take("Defined").as_deref(), None | Some("You"))
                || p2.take("Choices").as_deref() != Some("NonChosenPlayer")
                || !p2.exhausted()
            {
                return ExtraOutcome::Handled(
                    self.deny("as-enters `ChooseTwoPlayers` invalid second choice".into()),
                );
            }
            self.body
                .enter_modifiers
                .push("EnterModifier::ChooseTwoPlayers".to_string());
            return ExtraOutcome::Handled(Some(()));
        }

        // `OpponentChoosesCreatureType` (*Callous Oppressor*) and
        // `NoteMostPrevalentCreatureTypeInOpponentLibrary` (*Favored Enemy*)
        if matches!(p.peek("Choices"), Some("Opponent" | "Player.Opponent")) && p.has("SubAbility")
        {
            return self.choices_etb_opponent_type(&mut p);
        }

        ExtraOutcome::Pass
    }

    fn choices_etb_opponent_type(&mut self, p: &mut Params) -> ExtraOutcome {
        let Some(sub2_name) = p.peek("SubAbility").map(str::to_string) else {
            return ExtraOutcome::Pass;
        };
        let Some(body2) = self.svars.get(sub2_name.trim()).cloned() else {
            return ExtraOutcome::Pass;
        };
        let Some((api2, mut p2)) = Params::parse(&body2) else {
            return ExtraOutcome::Pass;
        };
        if api2 != "ChooseType" {
            return ExtraOutcome::Pass;
        }
        p.drop_prose();
        p.take("Defined");
        p.take("Choices");
        p.take("ChoiceTitle");
        p.take("SubAbility");
        if !p.exhausted() {
            return ExtraOutcome::Handled(
                self.deny("as-enters opponent creature type choice extra parameters".into()),
            );
        }
        p2.drop_prose();
        if p2.take("Type").as_deref() != Some("Creature") {
            return ExtraOutcome::Handled(
                self.deny("as-enters `ChooseType` outside creature type".into()),
            );
        }
        if matches!(p2.peek("Defined"), Some("Player.Chosen" | "ChosenPlayer")) {
            p2.take("Defined");
            if !p2.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("as-enters `OpponentChoosesCreatureType` extra params".into()),
                );
            }
            self.body
                .enter_modifiers
                .push("EnterModifier::OpponentChoosesCreatureType".to_string());
            return ExtraOutcome::Handled(Some(()));
        }
        if p2.take("MostPrevalentInDefinedZone").as_deref() == Some("ChosenPlayer_Library")
            && p2.take("Note").as_deref() == Some("True")
        {
            if let Some(clean_name) = p2.take("SubAbility") {
                let Some(clean_body) = self.svars.get(clean_name.trim()).cloned() else {
                    return ExtraOutcome::Handled(None);
                };
                let Some((c_api, mut cp)) = Params::parse(&clean_body) else {
                    return ExtraOutcome::Handled(None);
                };
                cp.drop_prose();
                if c_api != "Cleanup"
                    || cp.take("ClearChosenPlayer").as_deref() != Some("True")
                    || !cp.exhausted()
                {
                    return ExtraOutcome::Handled(
                        self.deny("as-enters noted library creature type bad cleanup".into()),
                    );
                }
            }
            if !p2.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("as-enters noted library creature type extra params".into()),
                );
            }
            self.body
                .enter_modifiers
                .push("EnterModifier::NoteMostPrevalentCreatureTypeInOpponentLibrary".to_string());
            return ExtraOutcome::Handled(Some(()));
        }
        ExtraOutcome::Handled(self.deny("as-enters opponent `ChooseType` unsupported shape".into()))
    }

    // -------------------------------------------------------------------------
    // 4. `Amount` & `Condition` additions
    // -------------------------------------------------------------------------

    /// Pure `Amount` atoms introduced in Wave 19 (`eng-choices-players`):
    /// - `Amount::Domain(PlayerRel::{TargetPlayer, EventPlayer})`
    /// - `Amount::RingTemptedCount(PlayerRel::You)`
    /// - `Amount::PlayerCounters { player, kind }` (`Poison`, `Rad`, `Energy`)
    /// - `Amount::ResolutionAccumulatedNumber` (Join forces accumulator)
    pub(super) fn choices_amount_num(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim().trim_start_matches('+');
        if trimmed.is_empty() {
            return None;
        }
        if let Some(acc_var) = self.join_forces_acc_var.as_deref()
            && trimmed == acc_var.trim()
        {
            return Some("Amount::ResolutionAccumulatedNumber".to_string());
        }
        if let Some((lo, hi)) = &self.random_number_range
            && self.has_resolution_chosen_number
        {
            let resolved = self
                .svars
                .get(trimmed)
                .map_or(trimmed, String::as_str)
                .trim();
            if resolved == format!("Count$Random.{lo}.{hi}") {
                return Some("Amount::ResolutionChosenNumber".to_string());
            }
        }
        let def = if trimmed.contains('$') {
            trimmed
        } else {
            self.svars.get(trimmed)?.trim()
        };
        self.choices_pure_amount_expr(def, 0)
    }

    pub(super) fn choices_pure_amount_expr(&self, def: &str, depth: u8) -> Option<String> {
        if depth >= 4 {
            return None;
        }
        let def = def.trim();
        if let Some(var) = def.strip_prefix("SVar$") {
            let trimmed_var = var.trim();
            if let Some(acc_var) = self.join_forces_acc_var.as_deref()
                && trimmed_var == acc_var.trim()
            {
                return Some("Amount::ResolutionAccumulatedNumber".to_string());
            }
            let inner = self.svars.get(trimmed_var)?.trim();
            return self.choices_pure_amount_expr(inner, depth + 1);
        }
        if let Some((lhs, rhs)) = def.rsplit_once("/Minus.") {
            let base = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
                self.choices_pure_amount_expr(self.svars.get(var.trim())?, depth + 1)?
            } else {
                self.choices_pure_amount_expr(lhs.trim(), depth + 1)?
            };
            let sub = if let Ok(n) = rhs.trim().parse::<u32>() {
                format!("Amount::Fixed({n})")
            } else {
                let rhs_def = self.svars.get(rhs.trim())?.trim();
                self.choices_pure_amount_expr(rhs_def, depth + 1)
                    .or_else(|| static_wave14_amount(rhs_def, self.svars))
                    .or_else(|| self.fallback_valid_count(rhs_def))?
            };
            return Some(format!(
                "Amount::Subtract {{ base: &{base}, subtract: &{sub} }}"
            ));
        }
        if def == "PlayerCountPropertyYou$RingTemptedYou" {
            return Some("Amount::RingTemptedCount(PlayerRel::You)".to_string());
        }
        if let Some((prefix, kind_raw)) = Self::split_player_counters(def) {
            if kind_raw.eq_ignore_ascii_case("TICKET") {
                return None;
            }
            let kind = match kind_raw {
                "Poison" | "POISON" => "CounterKind::Poison".to_string(),
                "Energy" | "ENERGY" => "CounterKind::Energy".to_string(),
                "Rad" | "RAD" => "CounterKind::Rad".to_string(),
                _ => counter_kind(kind_raw)?,
            };
            let player = match prefix {
                "TargetedController" => "PlayerRel::TargetController",
                "TargetedPlayer" => "PlayerRel::TargetPlayer",
                "TriggeredTarget" | "TriggeredPlayer" => "PlayerRel::EventPlayer",
                "PlayerCountRemembered" if self.in_repeat_player => "PlayerRel::EventPlayer",
                _ => return None,
            };
            return Some(format!(
                "Amount::PlayerCounters {{ player: {player}, kind: {kind} }}"
            ));
        }
        if self.in_chosen_player
            && let Some(expr) = self.fallback_valid_count(def)
        {
            return Some(expr);
        }
        None
    }

    fn fallback_valid_count(&self, def: &str) -> Option<String> {
        let valid = def.trim().strip_prefix("Count$Valid ")?;
        if valid.contains('$') {
            return None;
        }
        if valid == "Land.YouCtrl" {
            return Some(
                "Amount::CountOf { filter: &Filter::YOUR_LAND, zone: ZoneSel::Battlefield }"
                    .to_string(),
            );
        }
        if valid == "Land.ChosenCtrl" && self.in_chosen_player {
            return Some(
                "Amount::CountOf { filter: &Filter::And(&[Filter::LAND, Filter::ControlledBy(PlayerRel::Chosen)]), zone: ZoneSel::Battlefield }"
                    .to_string(),
            );
        }
        None
    }

    fn split_player_counters(def: &str) -> Option<(&str, &str)> {
        let (prefix, kind) = def.split_once("$Counters.")?;
        if kind.contains(['$', '/', ' ']) {
            return None;
        }
        Some((prefix.trim(), kind.trim()))
    }

    /// Evaluates Wave 19 `CheckSVar$` / `ConditionCheckSVar$` / `BranchConditionSVar$` conditions:
    /// - `Count$Foretold.1.0` -> `Condition::WasForetold`
    /// - `Count$wasSneaked` -> `Condition::WasSneaked`
    /// - `Count$TotalManaSpent Treasure` / `Count$CastTotalManaSpent Treasure` -> `Condition::TreasureManaWasSpent`
    /// - `PlayerCountOpponents$LowestLifeTotal` (`LE<n>` / `LT<n>`) -> `Condition::OpponentLifeAtMost`
    /// - `PlayerCountDefinedTriggeredPlayer$HasPropertyattackedYouTheirCurrentTurn` -> `Condition::PlayerAttackedYouThisTurn(PlayerRel::EventPlayer)`
    /// - `Amount::PlayerCounters` / `Amount::RingTemptedCount` / `Amount::Domain` bounds
    pub(super) fn choices_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let raw = self
            .svars
            .get(var.trim())
            .map_or(var.trim(), String::as_str)
            .trim();
        let cmp = cmp.trim();
        let eq_num = cmp.strip_prefix("EQ").and_then(|n| n.parse::<u16>().ok());
        let bound = count_bound(cmp);
        let bool_pos = match cmp {
            "GE1" | "EQ1" | "GT0" => Some(true),
            "EQ0" | "LT1" | "LE0" => Some(false),
            _ => None,
        };

        if raw == "Count$Foretold.1.0"
            && let Some(pos) = bool_pos
        {
            return Some(if pos {
                "Condition::WasForetold".to_string()
            } else {
                "Condition::Not(&Condition::WasForetold)".to_string()
            });
        }
        if raw == "Count$wasSneaked"
            && let Some(pos) = bool_pos
        {
            return Some(if pos {
                "Condition::WasSneaked".to_string()
            } else {
                "Condition::Not(&Condition::WasSneaked)".to_string()
            });
        }
        if matches!(
            raw,
            "Count$TotalManaSpent Treasure" | "Count$CastTotalManaSpent Treasure"
        ) && let Some(pos) = bool_pos
        {
            return Some(if pos {
                "Condition::TreasureManaWasSpent".to_string()
            } else {
                "Condition::Not(&Condition::TreasureManaWasSpent)".to_string()
            });
        }
        if raw == "PlayerCountDefinedTriggeredPlayer$HasPropertyattackedYouTheirCurrentTurn"
            && let Some(pos) = bool_pos
        {
            let base = "Condition::PlayerAttackedYouThisTurn(PlayerRel::EventPlayer)";
            return Some(if pos {
                base.to_string()
            } else {
                format!("Condition::Not(&{base})")
            });
        }
        if raw == "PlayerCountOpponents$LowestLifeTotal" {
            if let Some(n_raw) = cmp.strip_prefix("LE")
                && let Ok(n) = n_raw.parse::<i32>()
            {
                return Some(format!("Condition::OpponentLifeAtMost({n})"));
            }
            if let Some(n_raw) = cmp.strip_prefix("LT")
                && let Ok(n) = n_raw.parse::<i32>()
            {
                return Some(format!(
                    "Condition::OpponentLifeAtMost({})",
                    n.saturating_sub(1)
                ));
            }
            if let Some(n_raw) = cmp.strip_prefix("GE")
                && let Ok(n) = n_raw.parse::<i32>()
                && n >= 1
            {
                return Some(format!(
                    "Condition::Not(&Condition::OpponentLifeAtMost({}))",
                    n - 1
                ));
            }
            if let Some(n_raw) = cmp.strip_prefix("GT")
                && let Ok(n) = n_raw.parse::<i32>()
                && n >= 0
            {
                return Some(format!(
                    "Condition::Not(&Condition::OpponentLifeAtMost({n}))"
                ));
            }
        }
        if let Some(amt) = self.choices_pure_amount_expr(raw, 0) {
            return Self::choices_amount_bound_condition(&amt, bound, eq_num);
        }
        None
    }

    fn choices_amount_bound_condition(
        amt: &str,
        bound: Option<Bound>,
        eq_num: Option<u16>,
    ) -> Option<String> {
        match (bound, eq_num) {
            (Some(Bound::AtLeast(0)), _) => Some("Condition::Always".to_string()),
            (Some(Bound::AtLeast(n)), _) => Some(format!("Condition::AmountAtLeast({amt}, {n})")),
            (Some(Bound::AtMost(n)), _) => {
                let next = n.checked_add(1)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({amt}, {next}))"
                ))
            }
            (None, Some(0)) => Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({amt}, 1))"
            )),
            (None, Some(n)) => {
                let next = n.checked_add(1)?;
                Some(format!(
                    "Condition::All(&[Condition::AmountAtLeast({amt}, {n}), \
                     Condition::Not(&Condition::AmountAtLeast({amt}, {next}))])"
                ))
            }
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // 5. `Effect::ChoosePlayerEx`, `ChooseNumberAtRandom`, `SecretChooseNumbers`,
    //    `SecretNumberGuess`, Join Forces & `RepeatEachPlayerPayUntilNone`
    // -------------------------------------------------------------------------

    /// Lowers `ChoosePlayer` chains using `Effect::ChoosePlayerEx`:
    /// - `Choices$ Player.Opponent+withMoreLandsThanYou` (*Boreas Charger*)
    /// - `Choices$ Player.Opponent+lifeEQX` (*The Master, Gallifrey's End*, *The Mighty Will Fall*)
    /// - `Choices$ Player.!IsRemembered` with `RememberChosen$ True` (*Gluntch*)
    /// - `Choices$ Player` / `Opponent` / `Player.Other` with `Random$ True` or
    ///   `CantChooseSubAbility$` or `RememberChosen$ True` or `AddManaChoiceFor`
    pub(super) fn choices_choose_player_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if targets_here || p.has("ValidTgts") || p.has("Secretly") {
            return None;
        }
        let chooser = match p.take("Defined").as_deref() {
            None | Some("You") => "PlayerRel::You",
            _ => return None,
        };
        let random = match p.take("Random").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let remember = match p.take("RememberChosen").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let filter = self.choices_player_filter(p.take("Choices").as_deref())?;
        let cant_sub = p.take("CantChooseSubAbility");
        let choose_sub = p.take("ChooseSubAbility");
        let regular_sub = sub.take();
        let then_sub = match (choose_sub, regular_sub) {
            (Some(cs), None) => cs,
            (None, Some(rs)) => rs,
            _ => return None,
        };
        p.take("ChoiceTitle");
        if !p.exhausted() {
            return None;
        }

        let if_none = if let Some(none_name) = cant_sub {
            let body = self.svars.get(none_name.trim())?.clone();
            let mut none_chain = Chain {
                target: chain.target.clone(),
                second_target: chain.second_target.clone(),
                ..Chain::default()
            };
            self.chain(&body, &mut none_chain)?;
            if none_chain.effects.is_empty()
                || none_chain.target != chain.target
                || none_chain.second_target != chain.second_target
            {
                return None;
            }
            none_chain.effects
        } else {
            Vec::new()
        };

        let saved_in_chosen = self.in_chosen_player;
        self.in_chosen_player = true;
        let then_res = self.lower_choices_subchain(&then_sub, chain, true);
        self.in_chosen_player = saved_in_chosen;
        let (then_effects, trailing) = then_res?;
        if then_effects.is_empty() {
            return None;
        }
        *sub = trailing;
        Some(vec![format!(
            "Effect::ChoosePlayerEx {{ chooser: {chooser}, filter: {filter}, random: {random}, \
             remember: {remember}, if_none: &[{}], then: &[{}] }}",
            if_none.join(", "),
            then_effects.join(", "),
        )])
    }

    fn choices_player_filter(&self, raw: Option<&str>) -> Option<&'static str> {
        match raw {
            None | Some("Player") => Some("PlayerFilter::Among(PlayerRel::EachPlayer)"),
            Some("Opponent" | "Player.Opponent") => {
                Some("PlayerFilter::Among(PlayerRel::EachOpponent)")
            }
            Some("Player.Other") => Some("PlayerFilter::Among(PlayerRel::EachOtherPlayer)"),
            Some("Player.Opponent+withMoreLandsThanYou") => {
                Some("PlayerFilter::OpponentWithMoreLandsThanYou")
            }
            Some("Player.!IsRemembered") => Some("PlayerFilter::NotYetRemembered"),
            Some("Player.Opponent+!IsRemembered")
                if self
                    .svars
                    .values()
                    .any(|v| v.contains("RememberObjects$ TriggeredDefendingPlayer")) =>
            {
                Some("PlayerFilter::OpponentNotAttackedBySourceLastCombat")
            }
            Some("Player.Opponent+lifeEQX")
                if self.svars.get("X").map(String::as_str)
                    == Some("Count$OppGreatestLifeTotal") =>
            {
                Some("PlayerFilter::OpponentWithMostLife")
            }
            Some(_) => None,
        }
    }

    fn lower_choices_subchain(
        &mut self,
        start: &str,
        outer: &Chain,
        consume_chosen_steps: bool,
    ) -> Option<(Vec<String>, Option<String>)> {
        let mut cursor = Some(start.to_string());
        let mut out = Vec::new();
        let mut consumed = false;
        let mut steps = 0usize;
        while let Some(name) = cursor {
            steps += 1;
            if steps > 8 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (api, mut p) = Params::parse(&body)?;
            p.drop_prose();
            if api == "Cleanup" {
                let cp = p.take("ClearChosenPlayer");
                let rem = p.take("ClearRemembered");
                let next = p.take("SubAbility");
                if cp.as_deref().is_some_and(|v| v != "True")
                    || rem.as_deref().is_some_and(|v| v != "True")
                    || (cp.is_none() && rem.is_none())
                    || !p.exhausted()
                {
                    return None;
                }
                cursor = next;
                break;
            }
            let next_sub = p.take("SubAbility");
            if matches!(api.as_str(), "Branch" | "ChoosePlayer" | "VillainousChoice") {
                if !body.contains("ChosenPlayer")
                    && !body.contains("Player.Chosen")
                    && !body.contains("ChosenCtrl")
                {
                    return None;
                }
                let mut single_chain = Chain {
                    target: outer.target.clone(),
                    second_target: outer.second_target.clone(),
                    ..Chain::default()
                };
                self.chain(&body, &mut single_chain)?;
                if single_chain.effects.is_empty() || single_chain.target != outer.target {
                    return None;
                }
                consumed = true;
                out.extend(single_chain.effects);
                cursor = None;
                break;
            }

            let mut target_hint = outer.target.as_deref();
            if consume_chosen_steps {
                if api == "DealDamage"
                    && p.peek("Defined") == Some("ChosenPlayer")
                    && self
                        .oracle
                        .to_ascii_lowercase()
                        .contains("that player or a planeswalker")
                {
                    return None;
                }
                let mut rewrote = false;
                for (k, v) in &mut p.entries {
                    if matches!(k.as_str(), "Defined" | "DefinedPlayer" | "TokenOwner")
                        && matches!(v.as_str(), "ChosenPlayer" | "Player.Chosen")
                    {
                        *v = "Targeted".to_string();
                        rewrote = true;
                    }
                }
                if rewrote {
                    target_hint = Some("TargetSpec::Player(PlayerRel::Chosen)");
                    consumed = true;
                } else if api == "Effect"
                    && p.peek("StaticAbilities")
                        .and_then(|sa| self.svars.get(sa.trim()))
                        .is_some_and(|b| b.contains("ChosenPlayer"))
                {
                    consumed = true;
                } else if !consumed
                    || p.entries.iter().any(|(_, v)| {
                        v.contains("ChosenPlayer")
                            || v.contains("Player.Chosen")
                            || v.contains("ChosenCtrl")
                    })
                {
                    return None;
                } else {
                    cursor = Some(name);
                    break;
                }
            }
            let step_effs = self.effect_of(&api, &mut p, target_hint, false, false)?;
            if !p.exhausted() || step_effs.is_empty() {
                return None;
            }
            out.extend(step_effs);
            cursor = next_sub;
        }
        consumed.then_some((out, cursor))
    }

    /// Lowers `ChooseNumber` variants not covered by `wave16c_choose_number_chain`:
    /// - `Effect::ChooseNumberAtRandom` (`Random$ True`, `Min$..Max$`)
    /// - `Effect::SecretChooseNumbers` (`Secretly$ True`, `Defined$ Player | TargetedAndYou`)
    /// - `Effect::SecretNumberGuess` (`Secretly$ True`, `Guesser$ Opponent`)
    pub(super) fn choices_choose_number_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if targets_here {
            return None;
        }
        if p.peek("Random") == Some("True") && !p.has("Secretly") {
            return self.choices_choose_number_random(p, sub, chain);
        }
        if p.peek("Secretly") == Some("True") {
            if p.has("Guesser") {
                return self.choices_secret_number_guess(p, sub, chain);
            }
            return self.choices_secret_choose_numbers(p, sub, chain);
        }
        None
    }

    fn choices_choose_number_random(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<()> {
        p.take("Random");
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let min_raw = p.take("Min")?;
        let max_raw = p.take("Max")?;
        let min = amount(&min_raw, self.svars, self.has_x)?;
        let max = amount(&max_raw, self.svars, self.has_x)?;
        if let (Ok(lo), Ok(hi)) = (min_raw.trim().parse::<u32>(), max_raw.trim().parse::<u32>())
            && lo > hi
        {
            return None;
        }
        p.take("ListTitle");
        if !p.exhausted() {
            return None;
        }
        let follow_name = sub.take()?;
        let follow_body = self.svars.get(follow_name.trim())?.clone();
        let saved_num = self.has_resolution_chosen_number;
        let saved_rng = self
            .random_number_range
            .replace((min_raw.trim().to_string(), max_raw.trim().to_string()));
        self.has_resolution_chosen_number = true;
        let mut inner = Chain {
            target: chain.target.clone(),
            target_is_x: chain.target_is_x,
            target_up_to_x: chain.target_up_to_x,
            target_up_to: chain.target_up_to,
            target_exact: chain.target_exact,
            target_range: chain.target_range,
            target_unique: chain.target_unique,
            target_distinct_mana_values: chain.target_distinct_mana_values,
            target_constraint: chain.target_constraint,
            second_target: chain.second_target.clone(),
            second_target_up_to: chain.second_target_up_to,
            second_target_range: chain.second_target_range,
            ..Chain::default()
        };
        let res = self.chain(&follow_body, &mut inner);
        self.has_resolution_chosen_number = saved_num;
        self.random_number_range = saved_rng;
        res?;
        if inner.effects.is_empty()
            || inner.target != chain.target
            || inner.second_target != chain.second_target
        {
            return None;
        }
        chain.could_add_mana |= inner.could_add_mana;
        chain.moves_library |= inner.moves_library;
        chain.effects.push(format!(
            "Effect::ChooseNumberAtRandom {{ min: {min}, max: {max}, then: &[{}] }}",
            inner.effects.join(", ")
        ));
        Some(())
    }

    fn choices_secret_choose_numbers(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<()> {
        p.take("Secretly");
        let players = match p.take("Defined").as_deref() {
            Some("Player") => "PlayerRel::EachPlayer",
            Some("TargetedAndYou")
                if matches!(chain.target.as_deref(), Some("TargetSpec::AnyOpponent")) =>
            {
                "PlayerRel::YouAndTargetOpponent"
            }
            _ => return None,
        };
        let min = match p.take("Min") {
            None => "Amount::Fixed(0)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let max = match p.take("Max") {
            None => "Amount::Fixed(u32::MAX)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let remember_highest = match p.take("RememberHighest").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let each_sub = p.take("ChooseNumberSubAbility");
        let high_sub = p.take("Highest");
        let low_sub = p.take("Lowest");
        let not_low_sub = p.take("NotLowest");
        let matched_sub = p.take("MatchedAbility");
        let unmatched_sub = p.take("UnmatchedAbility");
        if each_sub.is_none()
            && high_sub.is_none()
            && low_sub.is_none()
            && not_low_sub.is_none()
            && matched_sub.is_none()
            && unmatched_sub.is_none()
        {
            return None;
        }
        if !p.exhausted() {
            return None;
        }

        let mut lower_arm = |arm: Option<String>, bind_event_player: bool| -> Option<Vec<String>> {
            let Some(name) = arm else {
                return Some(Vec::new());
            };
            let body = self.svars.get(name.trim())?.clone();
            let saved_num = self.has_resolution_chosen_number;
            let saved_rep = self.in_repeat_player;
            self.has_resolution_chosen_number = true;
            if bind_event_player {
                self.in_repeat_player = true;
            }
            let mut inner = Chain {
                target: chain.target.clone(),
                ..Chain::default()
            };
            let r = self.chain(&body, &mut inner);
            self.has_resolution_chosen_number = saved_num;
            self.in_repeat_player = saved_rep;
            r?;
            if inner.effects.is_empty() || inner.target != chain.target {
                return None;
            }
            Some(inner.effects)
        };

        let on_each = lower_arm(each_sub, true)?;
        let on_highest = lower_arm(high_sub, true)?;
        let on_lowest = lower_arm(low_sub, true)?;
        let on_not_lowest = lower_arm(not_low_sub, true)?;
        let on_matched = lower_arm(matched_sub, false)?;
        let on_unmatched = lower_arm(unmatched_sub, false)?;

        chain.effects.push(format!(
            "Effect::SecretChooseNumbers(SecretNumberChoice {{ \
             players: {players}, min: {min}, max: {max}, remember_highest: {remember_highest}, \
             on_each: &[{}], on_highest: &[{}], on_lowest: &[{}], \
             on_not_lowest: &[{}], on_matched: &[{}], on_unmatched: &[{}] }})",
            on_each.join(", "),
            on_highest.join(", "),
            on_lowest.join(", "),
            on_not_lowest.join(", "),
            on_matched.join(", "),
            on_unmatched.join(", "),
        ));

        // Optional post-reveal conditional step when `RememberHighest$ True`
        // (e.g. *Menacing Ogre*: `ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You` + Cleanup).
        if remember_highest && let Some(next_name) = sub.take() {
            let next_body = self.svars.get(next_name.trim())?.clone();
            let saved_rem = self.in_secret_remembered_player;
            self.in_secret_remembered_player = true;
            let mut post_chain = Chain {
                target: chain.target.clone(),
                ..Chain::default()
            };
            let res = self.chain(&next_body, &mut post_chain);
            self.in_secret_remembered_player = saved_rem;
            res?;
            if post_chain.target != chain.target {
                return None;
            }
            chain.effects.extend(post_chain.effects);
        }
        Some(())
    }

    fn choices_secret_number_guess(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<()> {
        if sub.is_some() {
            return None;
        }
        p.take("Secretly");
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let min: u8 = p.take("Min")?.trim().parse().ok()?;
        let max: u8 = p.take("Max")?.trim().parse().ok()?;
        if min > max {
            return None;
        }
        let track_used_on_source = match (
            p.take("RemoveChoices").as_deref(),
            p.take("RememberChosen").as_deref(),
        ) {
            (Some("Remembered"), Some("True")) => true,
            (None, None) => false,
            _ => return None,
        };
        let guesser = match p.take("Guesser").as_deref() {
            Some("Opponent" | "Player.Opponent") => "PlayerRel::Opponent",
            _ => return None,
        };
        let wrong_sub = p.take("GuessWrong");
        let correct_sub = p.take("GuessCorrect");
        if (wrong_sub.is_none() && correct_sub.is_none()) || !p.exhausted() {
            return None;
        }
        let mut lower_guess_arm = |arm: Option<String>| -> Option<Vec<String>> {
            let Some(name) = arm else {
                return Some(Vec::new());
            };
            let body = self.svars.get(name.trim())?.clone();
            let saved_num = self.has_resolution_chosen_number;
            let saved_guesser = self.in_secret_guesser_ctx;
            self.has_resolution_chosen_number = true;
            self.in_secret_guesser_ctx = true;
            let mut inner = Chain {
                target: chain.target.clone(),
                ..Chain::default()
            };
            let r = self.chain(&body, &mut inner);
            self.has_resolution_chosen_number = saved_num;
            self.in_secret_guesser_ctx = saved_guesser;
            r?;
            if inner.effects.is_empty() || inner.target != chain.target {
                return None;
            }
            Some(inner.effects)
        };
        let on_correct = lower_guess_arm(correct_sub)?;
        let on_wrong = lower_guess_arm(wrong_sub)?;
        chain.effects.push(format!(
            "Effect::SecretNumberGuess {{ min: {min}, max: {max}, \
             track_used_on_source: {track_used_on_source}, guesser: {guesser}, \
             on_correct: &[{}], on_wrong: &[{}] }}",
            on_correct.join(", "),
            on_wrong.join(", "),
        ));
        Some(())
    }

    // -------------------------------------------------------------------------
    // 6. Join forces (`RepeatEach | StartingWith$ You`) & `Plague of Vermin`
    // -------------------------------------------------------------------------

    /// Lowers Join forces:
    /// `RepeatEach | RepeatPlayers$ Player | StartingWith$ You | RepeatSubAbility$ DBPay | SubAbility$ <follow>`
    /// into:
    /// `[Effect::RepeatEach { domain: RepeatDomain::EachPlayerStartingWithYou, effects: &[ChooseNumberEx(ManaPayment -> AccumulateChosenNumber)] }, <follow(ResolutionAccumulatedNumber)>]`
    pub(super) fn choices_join_forces(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if p.peek("StartingWith") != Some("You") || p.peek("RepeatPlayers") != Some("Player") {
            return None;
        }
        let mut rp = p.clone();
        rp.drop_prose();
        rp.take("RepeatPlayers");
        rp.take("StartingWith");
        let pay_name = rp.take("RepeatSubAbility")?;
        if !rp.exhausted() {
            return None;
        }
        let follow_name = sub?;

        // 1. Validate `DBPay: ChooseNumber | Defined$ Player.IsRemembered | ChooseAnyNumber$ True | SubAbility$ DBStore`
        let pay_body = self.svars.get(pay_name.trim())?.clone();
        let (pay_api, mut pp) = Params::parse(&pay_body)?;
        pp.drop_prose();
        if pay_api != "ChooseNumber"
            || pp.take("Defined").as_deref() != Some("Player.IsRemembered")
            || pp.take("ChooseAnyNumber").as_deref() != Some("True")
        {
            return None;
        }
        pp.take("ListTitle");
        let store_name = pp.take("SubAbility")?;
        if !pp.exhausted() {
            return None;
        }

        // 2. Validate `DBStore: StoreSVar | SVar$ <acc> | Type$ CountSVar | Expression$ <acc>/Plus.<num> | UnlessCost$ <num> | UnlessPayer$ Player.IsRemembered | UnlessSwitched$ True`
        let store_body = self.svars.get(store_name.trim())?.clone();
        let (store_api, mut sp) = Params::parse(&store_body)?;
        sp.drop_prose();
        if store_api != "StoreSVar" || sp.take("Type").as_deref() != Some("CountSVar") {
            return None;
        }
        let acc_var = sp.take("SVar")?;
        if self.svars.get(acc_var.trim()).map(String::as_str) != Some("Number$0") {
            return None;
        }
        let expr = sp.take("Expression")?;
        let (lhs, rhs) = expr.split_once("/Plus.")?;
        if lhs.trim() != acc_var.trim() {
            return None;
        }
        let num_var = rhs.trim();
        if self.svars.get(num_var).map(String::as_str) != Some("Count$ChosenNumber") {
            return None;
        }
        if sp.take("UnlessCost").as_deref() != Some(num_var)
            || sp.take("UnlessPayer").as_deref() != Some("Player.IsRemembered")
            || sp.take("UnlessSwitched").as_deref() != Some("True")
            || !sp.exhausted()
        {
            return None;
        }

        // 3. Lower `<follow>` with `join_forces_acc_var = Some(acc_var)` and allow trailing reset `StoreSVar`!
        let follow_body = self.svars.get(follow_name.trim())?.clone();
        let saved_jf = self.join_forces_acc_var.replace(acc_var);
        let mut inner = Chain {
            target: chain.target.clone(),
            ..Chain::default()
        };
        let res = self.chain(&follow_body, &mut inner);
        self.join_forces_acc_var = saved_jf;
        res?;
        if inner.effects.is_empty() || inner.target != chain.target {
            return None;
        }
        chain.could_add_mana |= inner.could_add_mana;
        chain.moves_library |= inner.moves_library;

        let mut out = vec![
            "Effect::RepeatEach { domain: RepeatDomain::EachPlayerStartingWithYou, effects: &[Effect::ChooseNumberEx { chooser: PlayerRel::EventPlayer, min: Amount::Fixed(0), max: Amount::Fixed(u32::MAX), prompt: NumberChoiceKind::ManaPayment, pay_cost: Some(VariablePayKind::GenericMana), then: &[Effect::AccumulateChosenNumber] }] }".to_string(),
        ];
        out.extend(inner.effects);
        Some(out)
    }

    /// Lowers *Plague of Vermin*:
    /// `Repeat | RepeatSubAbility$ DBResetCheck | RepeatCheckSVar$ <count> | RepeatSVarCompare$ LTTotalPlayer | SubAbility$ DBRepeatToken`
    pub(super) fn choices_plague_of_vermin(
        &mut self,
        p: &Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut rp = p.clone();
        rp.drop_prose();
        rp.take("StackDescription");
        let reset_check_name = rp.take("RepeatSubAbility")?;
        let check_var = rp.take("RepeatCheckSVar")?;
        if rp.take("RepeatSVarCompare").as_deref() != Some("LTTotalPlayer") || !rp.exhausted() {
            return None;
        }
        let post_name = sub?;
        self.validate_plague_loop(&reset_check_name, &check_var)?;
        let tok_ref = self.parse_plague_post_token(post_name)?;
        Some(vec![format!(
            "Effect::RepeatEachPlayerPayUntilNone {{ kind: VariablePayKind::Life, \
             max_per_player: Amount::LifeTotal(PlayerRel::EventPlayer), \
             then_per_player: &[Effect::CreateTokensFor {{ player: PlayerRel::EventPlayer, \
             token: {tok_ref}, amount: Amount::ResolutionChosenNumber }}] }}"
        )])
    }

    fn validate_plague_loop(&self, reset_check_name: &str, check_var: &str) -> Option<()> {
        let reset_body = self.svars.get(reset_check_name.trim())?;
        let (r_api, mut r_p) = Params::parse(reset_body)?;
        r_p.drop_prose();
        if r_api != "StoreSVar"
            || r_p.take("SVar").as_deref() != Some(check_var.trim())
            || r_p.take("Type").as_deref() != Some("Number")
            || r_p.take("Expression").as_deref() != Some("0")
        {
            return None;
        }
        let each_name = r_p.take("SubAbility")?;
        if !r_p.exhausted() {
            return None;
        }
        let each_body = self.svars.get(each_name.trim())?;
        let (e_api, mut ep) = Params::parse(each_body)?;
        ep.drop_prose();
        if e_api != "RepeatEach"
            || ep.take("StartingWith").as_deref() != Some("You")
            || ep.take("RepeatPlayers").as_deref() != Some("Player")
        {
            return None;
        }
        let choice_name = ep.take("RepeatSubAbility")?;
        if !ep.exhausted() {
            return None;
        }
        let choice_body = self.svars.get(choice_name.trim())?;
        let (c_api, mut cp) = Params::parse(choice_body)?;
        cp.drop_prose();
        if c_api != "ChooseNumber"
            || cp.take("Defined").as_deref() != Some("Remembered")
            || cp.take("Max").as_deref() != Some("LifeTotal")
        {
            return None;
        }
        cp.take("ListTitle");
        let chk_paid_name = cp.take("SubAbility")?;
        if !cp.exhausted() {
            return None;
        }
        let chk_body = self.svars.get(chk_paid_name.trim())?;
        let (cp_api, mut cpp) = Params::parse(chk_body)?;
        cpp.drop_prose();
        if cp_api != "StoreSVar"
            || cpp.take("SVar").as_deref() != Some(check_var.trim())
            || cpp.take("Type").as_deref() != Some("CountSVar")
            || cpp.take("ConditionCheckSVar").is_none()
            || cpp.take("ConditionSVarCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let store_each_name = cpp.take("SubAbility")?;
        cpp.take("Expression");
        if !cpp.exhausted() {
            return None;
        }
        let store_body = self.svars.get(store_each_name.trim())?;
        let (st_api, mut stp) = Params::parse(store_body)?;
        stp.drop_prose();
        (st_api == "StoreSVar"
            && stp.take("Type").as_deref() == Some("AdditiveForEach")
            && stp.take("UnlessPayer").as_deref() == Some("Remembered")
            && stp.take("UnlessSwitched").as_deref() == Some("True")
            && stp.take("UnlessCost")?.starts_with("PayLife<"))
        .then_some(())
    }

    fn parse_plague_post_token(&self, post_name: &str) -> Option<String> {
        let post_body = self.svars.get(post_name.trim())?;
        let (po_api, mut pop) = Params::parse(post_body)?;
        pop.drop_prose();
        if po_api != "RepeatEach"
            || pop.take("RepeatPlayers").as_deref() != Some("Player")
            || !matches!(pop.take("ChangeZoneTable").as_deref(), None | Some("True"))
        {
            return None;
        }
        let tok_name = pop.take("RepeatSubAbility")?;
        let _final_reset = pop.take("SubAbility");
        if !pop.exhausted() {
            return None;
        }
        let tok_body = self.svars.get(tok_name.trim())?.clone();
        let (t_api, mut tp) = Params::parse(&tok_body)?;
        tp.drop_prose();
        if t_api != "Token" || tp.take("TokenOwner").as_deref() != Some("Remembered") {
            return None;
        }
        let raw_amt = tp.take("TokenAmount")?;
        if self.svars.get(raw_amt.trim()).map(String::as_str)
            != Some("PlayerCountRemembered$SVarAmount")
        {
            return None;
        }
        let stem = tp.take("TokenScript")?;
        if !tp.exhausted() {
            return None;
        }
        let tokens = self.tokens?;
        let tok_def = tokens.body_fixed(&stem, self.cats)?;
        Some(format!("&generated_tokens::{}", tok_def.constant))
    }
}
