//! Codegen Wave 30 Lane C:
//!
//! 1. `CopyPermanent | Defined$ Imprinted.ExiledWithSource` + activated `{X}`
//!    defined by `Imprinted$CardManaCost` (*Prototype Portal*, *Soul Foundry*).
//! 2. `DigUntil | Defined$ TriggeredDefendingPlayer | RememberRevealed$ True`
//!    -> `Pump | Defined$ TriggeredAttackerLKICopy` (*Trepanation Blade*).
//! 3. `Dig | RememberChanged$ True` follow-ups into `CaptureAffected`:
//!    - combat damage exile top -> optional permanent to battlefield or Fish token
//!      (*Fishing Gear*),
//!    - targeted opponent exile top -> Treasure + free cast if `cmc < X` else
//!      cast this turn (*Rashmi and Ragavan*),
//!    - `Defined$ TargetedOwner` exile top -> land to owner's battlefield else
//!      owner casts free (*Audacious Swap*).
//! 4. `Sacrifice | RememberSacrificed$ True` follow-ups:
//!    - `OppNonRememberedController` `LoseLife` `HalfUp`/`HalfDown` (*Skull Storm*),
//!    - `SacX` + `RememberedLKI$CardPower` -> exile top X & cast instant/sorcery free
//!      (*Kylox, Visionary Inventor*),
//!    - optional sac -> each opponent may sac sharing card type or lose 2 & you draw 1
//!      (*Braids, Arisen Nightmare*),
//!    - each player sacrifices -> if you sacrificed, return another permanent card
//!      from your graveyard to hand or battlefield (*Deadly Brew*, *Rise of the Witch-king*).

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Allow `CopyPermanent | Defined$ Imprinted.ExiledWithSource` when the card
    /// has no unlinked exile producer so `ChooseZone::LinkedExile` stays exact.
    pub(super) fn wave30c_allow_imprinted_copy_permanent(
        &self,
        defined_copy: Option<&str>,
    ) -> bool {
        defined_copy == Some("Imprinted.ExiledWithSource")
            && !self.card_has_unlinked_exile_producer()
    }

    /// Allow `{X}` in activated `Cost$` when the ability targets `LinkedExile`
    /// and `SVar:X:Imprinted$CardManaCost` defines it (*Prototype Portal*,
    /// *Soul Foundry*).
    pub(super) fn wave30c_allows_defined_cost_x(
        &self,
        effects: &[String],
        non_battlefield_zone: bool,
    ) -> bool {
        !non_battlefield_zone
            && !self.card_has_unlinked_exile_producer()
            && effects
                .iter()
                .any(|e| e.contains("ChooseZone::LinkedExile"))
            && self
                .svars
                .get("X")
                .is_some_and(|v| v.trim() == "Imprinted$CardManaCost")
    }

    /// Format `, x_value = ...` for `build_activated_parts`.
    pub(super) fn wave30c_activated_x_or_at_least(
        &self,
        x_min: u32,
        cost: &str,
        effects: &[String],
        non_battlefield_zone: bool,
    ) -> String {
        if x_min > 0 {
            return format!(", x_value = ActivationX::AtLeast({x_min})");
        }
        if Self::cost_has_variable_x(cost)
            && self.wave30c_allows_defined_cost_x(effects, non_battlefield_zone)
        {
            ", x_value = ActivationX::Defined(Amount::TotalManaValueInZone { zone: ZoneSel::LinkedExile, filter: &Filter::Any })".to_string()
        } else {
            String::new()
        }
    }

    /// Lower `PlayerCountRemembered$LifeTotal/HalfUp|HalfDown` (*Skull Storm*).
    pub(super) fn wave30c_player_count_remembered_life(&self, raw: &str) -> Option<String> {
        let def = self.svars.get(raw.trim()).map_or(raw.trim(), |s| s.trim());
        match def {
            "PlayerCountRemembered$LifeTotal/HalfUp" => Some(
                "Amount::Half { base: &Amount::LifeTotal(PlayerRel::EventPlayer), round_up: true }"
                    .to_string(),
            ),
            "PlayerCountRemembered$LifeTotal/HalfDown" => Some(
                "Amount::Half { base: &Amount::LifeTotal(PlayerRel::EventPlayer), round_up: false }"
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn wave30c_pump_remembered_delta(&self, raw: Option<String>) -> Option<&'static str> {
        let Some(raw) = raw else {
            return Some("Amount::Fixed(0)");
        };
        let var = raw.trim().strip_prefix('+')?;
        (self
            .svars
            .get(var.trim())
            .map(String::as_str)
            .map(str::trim)
            == Some("Remembered$Amount"))
        .then_some("Amount::RememberedCount")
    }

    /// Lower `DigUntil | RememberRevealed$ True -> Pump | Defined$ TriggeredAttackerLKICopy`
    /// (*Trepanation Blade*) via `CaptureAffected` + `RepeatEach(Times(1))` so
    /// `RevealUntilThen` preserves the attacking creature on `EventObject`.
    pub(super) fn wave30c_dig_until_remember_revealed(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if self.on_a_spell || !chain.effects.is_empty() {
            return None;
        }
        let saved_p = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();

        let restore = |this: &mut Self, p_ref: &mut Params| {
            *p_ref = saved_p;
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            this.body.statics.truncate(saved_statics);
        };

        let Some(parsed) = self.parse_dig_until_core(p, target, targets_a_player, true) else {
            restore(self, p);
            return None;
        };
        if !parsed.remember_revealed
            || parsed.remember_found
            || parsed.imprint_found
            || parsed.imprint_revealed
            || parsed.exile_while_revealing
            || parsed.multi_seat_domain.is_some()
            || parsed.optional_whole
            || parsed.optional_found_move
            || parsed.trailing_shuffle
            || parsed.found != "RevealUntilFound::SameAsRest"
            || parsed.rest != "RevealUntilRest::Graveyard"
            || !p.entries.iter().all(|(k, _)| k.starts_with("Condition"))
        {
            restore(self, p);
            return None;
        }

        let Some(sub_name) = sub.clone() else {
            restore(self, p);
            return None;
        };
        let Some(body) = self.svars.get(sub_name.trim()).cloned() else {
            restore(self, p);
            return None;
        };
        let Some((api, mut sp)) = Params::parse(&body) else {
            restore(self, p);
            return None;
        };
        sp.drop_prose();
        if api != "Pump"
            || sp.has("ValidTgts")
            || sp.has("KW")
            || sp.has("RememberPumped")
            || !matches!(
                sp.take("Duration").as_deref(),
                None | Some("UntilEndOfTurn")
            )
            || !matches!(
                sp.take("Defined").as_deref(),
                Some(def @ ("TriggeredAttackerLKICopy" | "TriggeredAttacker"))
                    if self.is_event_object_defined(def)
            )
        {
            restore(self, p);
            return None;
        }
        let Some(power) = self.wave30c_pump_remembered_delta(sp.take("NumAtt")) else {
            restore(self, p);
            return None;
        };
        let Some(toughness) = self.wave30c_pump_remembered_delta(sp.take("NumDef")) else {
            restore(self, p);
            return None;
        };
        if power == "Amount::Fixed(0)" && toughness == "Amount::Fixed(0)" {
            restore(self, p);
            return None;
        }
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            restore(self, p);
            return None;
        }
        let Some(tail) = self.consume_dig_cleanup_tail(next, false) else {
            restore(self, p);
            return None;
        };

        let reveal = Self::format_reveal_until_then(&parsed, &parsed.found, &parsed.rest, &[]);
        *sub = tail.into_option();
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::RepeatEach {{ domain: RepeatDomain::Times(Amount::Fixed(1)), \
             effects: &[{reveal}] }}], \
             then: &[Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }}] }}"
        )])
    }

    /// Lower `Dig | RememberChanged$ True` follow-up patterns into `CaptureAffected`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave30c_dig_remember_changed_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("DigNum") != Some("1")
            || p.peek("ChangeNum") != Some("All")
            || p.peek("DestinationZone") != Some("Exile")
            || p.peek("RememberChanged") != Some("True")
            || p.has("Optional")
            || p.has("Reveal")
            || p.has("ExileFaceDown")
        {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        let restore_statics = |this: &mut Self| {
            (*this.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
            this.body.statics.truncate(saved_statics);
        };

        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s1_api, mut sp1) = Params::parse(&sub_body)?;
        sp1.drop_prose();

        // Pattern A (*Fishing Gear*): combat-damage exile top 1 -> optional permanent
        // to battlefield under your control else create a token.
        if !targets_here
            && s1_api == "ChangeZone"
            && sp1.peek("ConditionDefined") == Some("Remembered")
            && sp1.peek("ConditionPresent") == Some("Permanent")
            && sp1.peek("Defined") == Some("Remembered")
            && sp1.peek("Origin") == Some("Exile")
            && sp1.peek("Destination") == Some("Battlefield")
            && sp1.peek("Optional") == Some("True")
            && sp1.peek("Imprint") == Some("True")
            && sp1.peek("GainControl") == Some("True")
        {
            let mut probe = p.clone();
            probe.take("DigNum");
            probe.take("ChangeNum");
            probe.take("DestinationZone");
            probe.take("RememberChanged");
            let defined = probe.take("Defined");
            let Some(who) = self.player_rel(defined.as_deref()).or_else(|| {
                self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)
            }) else {
                restore_statics(self);
                return None;
            };
            if !Self::is_single_player_rel(who)
                || who == "PlayerRel::You"
                || !probe
                    .entries
                    .iter()
                    .all(|(k, _)| k.starts_with("Condition"))
            {
                restore_statics(self);
                return None;
            }
            sp1.take("ConditionDefined");
            sp1.take("ConditionPresent");
            if !matches!(
                sp1.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1" | "GT0")
            ) {
                restore_statics(self);
                return None;
            }
            sp1.take("Defined");
            sp1.take("Origin");
            sp1.take("Destination");
            sp1.take("Optional");
            sp1.take("Imprint");
            sp1.take("GainControl");
            let tok_sub = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(tok_name) = tok_sub else {
                restore_statics(self);
                return None;
            };
            let Some(tok_body) = self.svars.get(tok_name.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&tok_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            if s2_api != "Token"
                || sp2.take("ConditionDefined").as_deref() != Some("Imprinted")
                || sp2.take("ConditionPresent").as_deref() != Some("Card")
                || sp2.take("ConditionCompare").as_deref() != Some("EQ0")
                || !matches!(sp2.peek("TokenOwner"), None | Some("You"))
            {
                restore_statics(self);
                return None;
            }
            let after_tok = sp2.take("SubAbility");
            let Some(trailing) = self.wave30c_consume_cleanup(after_tok, true) else {
                restore_statics(self);
                return None;
            };
            let mut tok_chain = Chain::default();
            let tok_spec = Self::wave30c_reconstruct_spec("Token", &sp2);
            if self.chain(&tok_spec, &mut tok_chain).is_none()
                || tok_chain.effects.len() != 1
                || tok_chain.target.is_some()
                || tok_chain.second_target.is_some()
            {
                restore_statics(self);
                return None;
            }
            let tok_eff = &tok_chain.effects[0];
            let perm_expr = self.filter_expr_in_zone("Permanent", true)?;
            let perm_only = self.body.filter_static("DIG", &perm_expr);
            let rem_perm = self.body.filter_static(
                "DIG_PERM",
                &format!("Filter::And(&[Filter::IsRemembered, {perm_expr}])"),
            );
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::ExileTop {{ who: {who}, count: Amount::Fixed(1) }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::CountOf {{ filter: &{rem_perm}, zone: ZoneSel::ExileAll }}, \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
                 then: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
                 zone: ChooseZone::Remembered, filter: &{perm_only}, min: 0, \
                 max: Amount::Fixed(1), reveal_chosen: false, \
                 movement: ChooseMove::Battlefield {{ tapped: false }}, \
                 not_chosen_movement: ChooseMove::None, then: &[], \
                 for_unchosen: &[{tok_eff}] }}], otherwise: &[{tok_eff}] }}] }}"
            )]);
        }

        // Pattern B (*Rashmi and Ragavan*): targeted opponent exile top 1 -> token
        // -> optional free cast if `Spell.cmcLT<var>` -> if not cast, cast this turn.
        if targets_here
            && targets_a_player
            && s1_api == "Token"
            && !sp1.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            let mut probe = p.clone();
            probe.take("DigNum");
            probe.take("ChangeNum");
            probe.take("DestinationZone");
            probe.take("RememberChanged");
            let defined = probe.take("Defined");
            let Some(who) = self.player_rel(defined.as_deref()).or_else(|| {
                self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)
            }) else {
                restore_statics(self);
                return None;
            };
            if !Self::is_single_player_rel(who)
                || !probe
                    .entries
                    .iter()
                    .all(|(k, _)| k.starts_with("Condition"))
            {
                restore_statics(self);
                return None;
            }
            let play_sub = sp1.take("SubAbility");
            let mut tok_chain = Chain::default();
            let tok_spec = Self::wave30c_reconstruct_spec("Token", &sp1);
            if self.chain(&tok_spec, &mut tok_chain).is_none()
                || tok_chain.effects.len() != 1
                || tok_chain.target.is_some()
                || tok_chain.second_target.is_some()
            {
                restore_statics(self);
                return None;
            }
            let Some(play_name) = play_sub else {
                restore_statics(self);
                return None;
            };
            let Some(play_body) = self.svars.get(play_name.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&play_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            if s2_api != "Play"
                || sp2.take("Valid").as_deref() != Some("Card.IsRemembered")
                || sp2.take("ValidZone").as_deref() != Some("Exile")
                || sp2.take("ImprintPlayed").as_deref() != Some("True")
                || !matches!(sp2.take("Controller").as_deref(), None | Some("You"))
                || sp2.take("WithoutManaCost").as_deref() != Some("True")
                || sp2.take("Optional").as_deref() != Some("True")
                || !matches!(sp2.take("Amount").as_deref(), None | Some("1" | "All"))
            {
                restore_statics(self);
                return None;
            }
            let Some(valid_sa) = sp2.take("ValidSA") else {
                restore_statics(self);
                return None;
            };
            let Some(cmc_var) = valid_sa.trim().strip_prefix("Spell.cmcLT") else {
                restore_statics(self);
                return None;
            };
            let Some(cmc_amt) = self.dynamic_filter_amount(cmc_var, 0) else {
                restore_statics(self);
                return None;
            };
            let eff_sub = sp2.take("SubAbility");
            if !sp2.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(eff_name) = eff_sub else {
                restore_statics(self);
                return None;
            };
            let Some(eff_body) = self.svars.get(eff_name.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s3_api, mut sp3)) = Params::parse(&eff_body) else {
                restore_statics(self);
                return None;
            };
            sp3.drop_prose();
            if s3_api != "Effect"
                || sp3.take("ConditionDefined").as_deref() != Some("Imprinted")
                || sp3.take("ConditionPresent").as_deref() != Some("Card")
                || sp3.take("ConditionCompare").as_deref() != Some("EQ0")
                || sp3.take("RememberObjects").as_deref() != Some("RememberedCard")
                || sp3.take("ForgetOnMoved").as_deref() != Some("Exile")
                || !matches!(sp3.take("Duration").as_deref(), None | Some("EndOfTurn"))
            {
                restore_statics(self);
                return None;
            }
            let Some(st_var) = sp3.take("StaticAbilities") else {
                restore_statics(self);
                return None;
            };
            let after_eff = sp3.take("SubAbility");
            if !sp3.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(st_body) = self.svars.get(st_var.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((st_api, mut stp)) = Params::parse(&st_body) else {
                restore_statics(self);
                return None;
            };
            stp.drop_prose();
            if st_api != "Continuous"
                || stp.take("Affected").as_deref() != Some("Card.IsRemembered+nonLand")
                || stp.take("MayPlay").as_deref() != Some("True")
                || stp.take("AffectedZone").as_deref() != Some("Exile")
                || !stp.exhausted()
            {
                restore_statics(self);
                return None;
            }
            let Some(trailing) = self.wave30c_consume_cleanup(after_eff, true) else {
                restore_statics(self);
                return None;
            };

            let play_filter = self.body.filter_static(
                "PLAY",
                &format!(
                    "Filter::And(&[Filter::CmcAtMostAmount(&{cmc_amt}), \
                     Filter::Not(&Filter::CmcEqualsAmount(&{cmc_amt}))])"
                ),
            );
            let rem_nonland = self.body.filter_static(
                "DIG",
                "Filter::And(&[Filter::IsRemembered, Filter::NONLAND])",
            );
            let tok_eff = &tok_chain.effects[0];
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::ExileTop {{ who: {who}, count: Amount::Fixed(1) }}], \
                 then: &[{tok_eff}, Effect::ForEachRemembered {{ filter: &Filter::Any, \
                 effects: &[Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
                 filter: &{play_filter}, controller: PlayerRel::You, count: PlayCount::One, \
                 cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
                 exile_instead_of_graveyard: false, optional: true }}, \
                 Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::CountOf {{ filter: &{rem_nonland}, zone: ZoneSel::ExileAll }}, \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
                 then: &[Effect::ExileWithPermission {{ target: TargetSpec::EventObject, \
                 until: None, permission: ExilePermission::cast(PlayWindow::ThisTurn) }}], \
                 otherwise: &[] }}] }}] }}"
            )]);
        }

        // Pattern C (*Audacious Swap*): `Defined$ TargetedOwner` exile top 1 ->
        // if land, owner puts onto battlefield; else owner may cast free.
        if !targets_here
            && !targets_a_player
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("Defined") == Some("TargetedOwner")
            && s1_api == "ChangeZone"
            && sp1.peek("ConditionDefined") == Some("Remembered")
            && sp1.peek("ConditionPresent") == Some("Land")
            && sp1.peek("Defined") == Some("Remembered")
            && sp1.peek("Origin") == Some("Exile")
            && sp1.peek("Destination") == Some("Battlefield")
            && sp1.peek("ForgetChanged") == Some("True")
        {
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("DigNum");
            probe.take("ChangeNum");
            probe.take("DestinationZone");
            probe.take("RememberChanged");
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                restore_statics(self);
                return None;
            }
            sp1.take("ConditionDefined");
            sp1.take("ConditionPresent");
            if !matches!(
                sp1.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1" | "GT0")
            ) {
                restore_statics(self);
                return None;
            }
            sp1.take("Defined");
            sp1.take("DefinedDesc");
            sp1.take("Origin");
            sp1.take("Destination");
            sp1.take("ForgetChanged");
            let cast_sub = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(cast_name) = cast_sub else {
                restore_statics(self);
                return None;
            };
            let Some(cast_body) = self.svars.get(cast_name.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&cast_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            sp2.take("DefinedDesc");
            if s2_api != "Play"
                || sp2.take("ConditionDefined").as_deref() != Some("Remembered")
                || sp2.take("ConditionPresent").as_deref() != Some("Card")
                || !matches!(
                    sp2.take("ConditionCompare").as_deref(),
                    None | Some("GE1" | "EQ1" | "GT0")
                )
                || sp2.take("Defined").as_deref() != Some("Remembered")
                || sp2.take("ValidSA").as_deref() != Some("Spell")
                || sp2.take("Controller").as_deref() != Some("RememberedOwner")
                || sp2.take("WithoutManaCost").as_deref() != Some("True")
                || sp2.take("Optional").as_deref() != Some("True")
                || !matches!(sp2.take("Amount").as_deref(), None | Some("1" | "All"))
            {
                restore_statics(self);
                return None;
            }
            let after_cast = sp2.take("SubAbility");
            if !sp2.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(trailing) = self.wave30c_consume_cleanup(after_cast, false) else {
                restore_statics(self);
                return None;
            };
            let rem_land = self
                .body
                .filter_static("DIG", "Filter::And(&[Filter::IsRemembered, Filter::LAND])");
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::ExileTop {{ who: PlayerRel::OwnerOfTarget, \
                 count: Amount::Fixed(1) }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::CountOf {{ filter: &{rem_land}, zone: ZoneSel::ExileAll }}, \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
                 then: &[Effect::ForEachRemembered {{ filter: &Filter::Any, \
                 effects: &[Effect::MoveAndModify {{ \
                 subject: MoveSubject::Target(TargetSpec::EventObject), \
                 expected_origin: Some(ZoneRef::Exile), blink: false, owner_control: true, \
                 tapped: false, face: None, counters: None, modifiers: &[], \
                 duration: Duration::Indefinitely, attach_source: false, \
                 when_source_leaves: None }}] }}], \
                 otherwise: &[Effect::ForEachRemembered {{ filter: &Filter::Any, \
                 effects: &[Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
                 filter: &Filter::Any, controller: PlayerRel::OwnerOfTarget, \
                 count: PlayCount::One, cost: PlayCostMode::WithoutPayingManaCost, \
                 cast_only: true, copy: false, exile_instead_of_graveyard: false, \
                 optional: true }}] }}] }}] }}"
            )]);
        }

        None
    }

    /// Lower `Sacrifice | RememberSacrificed$ True` follow-up patterns into
    /// `CaptureAffected`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave30c_sac_remember_sacrificed_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here || p.peek("RememberSacrificed") != Some("True") {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        let restore_statics = |this: &mut Self| {
            (*this.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
            this.body.statics.truncate(saved_statics);
        };

        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s1_api, mut sp1) = Params::parse(&sub_body)?;
        sp1.drop_prose();

        // Pattern A (*Kylox, Visionary Inventor*): optional sacrifice any number ->
        // exile top `RememberedLKI$CardPower` -> cast any number of instant/sorcery free.
        if p.peek("Optional") == Some("True")
            && matches!(p.peek("Defined"), None | Some("You"))
            && s1_api == "Dig"
            && sp1.peek("DestinationZone") == Some("Exile")
            && sp1.peek("ChangeNum") == Some("All")
            && sp1.peek("Imprint") == Some("True")
        {
            let mut probe = p.clone();
            probe.take("Optional");
            probe.take("Defined");
            probe.take("RememberSacrificed");
            probe.claim_label("SacMessage", "SacValid");
            let amt_var = probe.take("Amount")?;
            let sac_valid = probe.take("SacValid")?;
            if sac_valid == "Self"
                || sac_valid.contains("YouCtrl")
                || sac_valid.contains("OppCtrl")
                || !probe.exhausted()
            {
                restore_statics(self);
                return None;
            }
            let norm_valid = sac_valid.replace("StrictlyOther", "Other");
            let expected_cnt = format!("Count$Valid {sac_valid}");
            let expected_norm = format!("Count$Valid {norm_valid}");
            if !self.svars.get(amt_var.trim()).is_some_and(|v| {
                let t = v.trim();
                t == expected_cnt || t == expected_norm
            }) {
                restore_statics(self);
                return None;
            }
            let Some(sac_expr) = self.filter_expr(&norm_valid) else {
                restore_statics(self);
                return None;
            };
            let sac_name = self.body.filter_static("SAC", &sac_expr);

            if !matches!(sp1.take("Defined").as_deref(), None | Some("You")) {
                restore_statics(self);
                return None;
            }
            sp1.take("DestinationZone");
            sp1.take("ChangeNum");
            sp1.take("Imprint");
            let dig_num_var = sp1.take("DigNum")?;
            if self
                .svars
                .get(dig_num_var.trim())
                .map(String::as_str)
                .map(str::trim)
                != Some("RememberedLKI$CardPower")
            {
                restore_statics(self);
                return None;
            }
            let play_sub = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(play_name) = play_sub else {
                restore_statics(self);
                return None;
            };
            let Some(play_body) = self.svars.get(play_name.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&play_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            if s2_api != "Play"
                || !matches!(
                    sp2.take("Valid").as_deref(),
                    Some("Card.IsImprinted+YouOwn" | "Card.IsImprinted")
                )
                || sp2.take("ValidZone").as_deref() != Some("Exile")
                || !matches!(sp2.take("Controller").as_deref(), None | Some("You"))
                || sp2.take("WithoutManaCost").as_deref() != Some("True")
                || sp2.take("Optional").as_deref() != Some("True")
                || sp2.take("Amount").as_deref() != Some("All")
            {
                restore_statics(self);
                return None;
            }
            let Some(valid_sa) = sp2.take("ValidSA") else {
                restore_statics(self);
                return None;
            };
            let Some(sa_filter) = self.wave30c_instant_sorcery_sa_filter(&valid_sa) else {
                restore_statics(self);
                return None;
            };
            let after_play = sp2.take("SubAbility");
            if !sp2.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(trailing) = self.wave30c_consume_cleanup(after_play, true) else {
                restore_statics(self);
                return None;
            };
            let play_name = self.body.filter_static(
                "PLAY",
                &format!("Filter::And(&[Filter::IsRemembered, {sa_filter}])"),
            );
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::SacrificeAnyNumber {{ who: PlayerRel::You, \
                 filter: &{sac_name} }}], \
                 then: &[Effect::CaptureAffected {{ \
                 first: &[Effect::ExileTop {{ who: PlayerRel::You, \
                 count: Amount::RememberedPower }}], \
                 then: &[Effect::Play {{ source: PlaySource::Zone {{ zone: ZoneRef::Exile, \
                 owner: PlayerRel::You }}, filter: &{play_name}, controller: PlayerRel::You, \
                 count: PlayCount::All, cost: PlayCostMode::WithoutPayingManaCost, \
                 cast_only: true, copy: false, exile_instead_of_graveyard: false, \
                 optional: true }}] }}] }}"
            )]);
        }

        // Pattern B (*Braids, Arisen Nightmare*): optional sacrifice 1 -> each opponent
        // may sacrifice a permanent sharing a card type with it, else lose life & you draw.
        if p.peek("Optional") == Some("True")
            && matches!(p.peek("Defined"), None | Some("You"))
            && matches!(p.peek("Amount"), None | Some("1"))
            && s1_api == "RepeatEach"
            && sp1.peek("ConditionDefined") == Some("Remembered")
            && sp1.peek("ConditionPresent") == Some("Card")
            && matches!(
                sp1.peek("RepeatPlayers"),
                Some("Player.Opponent" | "Opponent")
            )
        {
            let mut probe = p.clone();
            probe.take("Optional");
            probe.take("Defined");
            probe.take("Amount");
            probe.take("RememberSacrificed");
            probe.claim_label("SacMessage", "SacValid");
            let sac_valid = probe.take("SacValid")?;
            if sac_valid == "Self"
                || sac_valid.contains("YouCtrl")
                || sac_valid.contains("OppCtrl")
                || sac_valid.contains("Other")
                || !probe.exhausted()
            {
                restore_statics(self);
                return None;
            }
            let Some(sac_expr) = self.filter_expr(&sac_valid) else {
                restore_statics(self);
                return None;
            };
            let sac_name = self.body.filter_static("SAC", &sac_expr);

            sp1.take("ConditionDefined");
            sp1.take("ConditionPresent");
            if !matches!(
                sp1.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1" | "GT0")
            ) {
                restore_statics(self);
                return None;
            }
            sp1.take("RepeatPlayers");
            let rep_sub = sp1.take("RepeatSubAbility")?;
            let after_rep = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(may_sac_body) = self.svars.get(rep_sub.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&may_sac_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            if s2_api != "Sacrifice"
                || sp2.take("Defined").as_deref() != Some("Player.IsRemembered")
                || sp2.take("SacValid").as_deref()
                    != Some("Permanent.sharesCardTypeWith RememberedCard")
                || sp2.take("RememberSacrificed").as_deref() != Some("True")
                || sp2.take("Optional").as_deref() != Some("True")
                || !matches!(sp2.take("Amount").as_deref(), None | Some("1"))
            {
                restore_statics(self);
                return None;
            }
            let fallback_sub = sp2.take("SubAbility");
            if !sp2.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(fallback_effs) = self.wave30c_braids_fallback_chain(fallback_sub) else {
                restore_statics(self);
                return None;
            };
            let Some(trailing) = self.wave30c_consume_cleanup(after_rep, false) else {
                restore_statics(self);
                return None;
            };
            let fallback_expr = match fallback_effs.as_slice() {
                [only] => only.clone(),
                many => format!("Effect::Sequence(&[{}])", many.join(", ")),
            };
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::MayDo {{ effects: &[Effect::SacrificeFilter {{ \
                 who: PlayerRel::You, filter: &{sac_name} }}] }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::RememberedCount, op: AmountCmp::AtLeast, \
                 right: &Amount::Fixed(1) }}, \
                 then: &[Effect::RepeatEach {{ domain: RepeatDomain::EachOpponent, \
                 effects: &[Effect::PlayerMayPayCostOr {{ player: PlayerRel::EventPlayer, \
                 cost: &CostPart::Sacrifice(&Filter::SharesCardTypeWithRemembered), \
                 effect: &{fallback_expr} }}] }}], otherwise: &[] }}] }}"
            )]);
        }

        // Pattern C (*Deadly Brew*, *Rise of the Witch-king*): each player sacrifices
        // -> if you sacrificed a permanent/creature this way, you may return another
        // non-Aura permanent card from your graveyard to hand or battlefield.
        if !p.has("Optional")
            && p.peek("Defined") == Some("Player")
            && matches!(p.peek("Amount"), None | Some("1"))
            && s1_api == "ChangeZone"
            && sp1.peek("ConditionDefined") == Some("Remembered")
            && matches!(
                sp1.peek("ConditionPresent"),
                Some("Card.Permanent+YouCtrl" | "Permanent.YouCtrl")
            )
            && matches!(sp1.peek("ConditionCompare"), Some("GE1" | "GT0"))
            && sp1.peek("Origin") == Some("Graveyard")
            && matches!(sp1.peek("Destination"), Some("Hand" | "Battlefield"))
            && sp1.peek("Hidden") == Some("True")
        {
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("Amount");
            probe.take("RememberSacrificed");
            probe.claim_label("SacMessage", "SacValid");
            let sac_valid = probe.take("SacValid")?;
            if sac_valid == "Self"
                || sac_valid.contains("YouCtrl")
                || sac_valid.contains("OppCtrl")
                || sac_valid.contains("Other")
                || !probe.exhausted()
            {
                restore_statics(self);
                return None;
            }
            let Some(sac_expr) = self.filter_expr(&sac_valid) else {
                restore_statics(self);
                return None;
            };
            let sac_name = self.body.filter_static("SAC", &sac_expr);

            sp1.take("ConditionDefined");
            sp1.take("ConditionPresent");
            sp1.take("ConditionCompare");
            sp1.take("Origin");
            let dest = sp1.take("Destination")?;
            sp1.take("Hidden");
            let opt = sp1.take("Optional").as_deref() == Some("True")
                || (!self.oracle.is_empty()
                    && self.oracle.to_ascii_lowercase().contains("you may"));
            if !opt
                || !matches!(sp1.take("ChangeNum").as_deref(), None | Some("1"))
                || !matches!(sp1.take("DefinedPlayer").as_deref(), None | Some("You"))
                || !matches!(sp1.take("Chooser").as_deref(), None | Some("You"))
            {
                restore_statics(self);
                return None;
            }
            let ct = sp1.take("ChangeType")?;
            let after_cz = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(ct_no_rem) = Self::wave30c_strip_filter_attr(&ct, "!IsRemembered") else {
                restore_statics(self);
                return None;
            };
            let Some(base_ct) = Self::wave30c_strip_filter_attr(&ct_no_rem, "YouCtrl") else {
                restore_statics(self);
                return None;
            };
            if base_ct.contains("Aura") || base_ct.contains("Remembered") {
                restore_statics(self);
                return None;
            }
            let Some(base_expr) = self.filter_expr_in_zone(&base_ct, true) else {
                restore_statics(self);
                return None;
            };
            let Some(trailing) = self.wave30c_consume_cleanup(after_cz, false) else {
                restore_statics(self);
                return None;
            };
            let ret_name = self.body.filter_static(
                "SAC_RET",
                &format!("Filter::And(&[{base_expr}, Filter::NotRemembered])"),
            );
            let movement = match dest.as_str() {
                "Hand" => "ChooseMove::Hand",
                "Battlefield" => "ChooseMove::Battlefield { tapped: false }",
                _ => {
                    restore_statics(self);
                    return None;
                }
            };
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::RepeatEach {{ domain: RepeatDomain::EachPlayer, \
                 effects: &[Effect::SacrificeFilter {{ who: PlayerRel::EventPlayer, \
                 filter: &{sac_name} }}] }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::ThisWayBy {{ what: ThisWay::SacrificedCount, who: PlayerRel::You }}, \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
                 then: &[Effect::ChooseCard {{ chooser: PlayerRel::You, \
                 zone: ChooseZone::Graveyard(PlayerRel::You), filter: &{ret_name}, \
                 min: 0, max: Amount::Fixed(1), movement: {movement}, then: &[] }}], \
                 otherwise: &[] }}] }}"
            )]);
        }

        // Pattern D (*Skull Storm*): each player/opponent sacrifices a creature ->
        // `RepeatEach | RepeatPlayers$ OppNonRememberedController|NonRememberedController`
        // -> `LoseLife | Defined$ Remembered | LifeAmount$ PlayerCountRemembered$LifeTotal/HalfUp|HalfDown`.
        if !p.has("Optional")
            && matches!(p.peek("Amount"), None | Some("1"))
            && !self.oracle.is_empty()
            && self.oracle.to_ascii_lowercase().contains("who can't")
            && s1_api == "RepeatEach"
            && !sp1.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            let (domain, expected_rep) = match p.peek("Defined") {
                Some("Opponent" | "Player.Opponent") => {
                    ("RepeatDomain::EachOpponent", "OppNonRememberedController")
                }
                Some("Player") => ("RepeatDomain::EachPlayer", "NonRememberedController"),
                _ => return None,
            };
            let mut probe = p.clone();
            probe.take("RememberSacrificed");
            probe.take("Defined");
            probe.take("Amount");
            probe.claim_label("SacMessage", "SacValid");
            let sac_valid = probe.take("SacValid")?;
            if sac_valid == "Self"
                || sac_valid.contains("YouCtrl")
                || sac_valid.contains("OppCtrl")
                || sac_valid.contains("Other")
                || !probe.exhausted()
            {
                restore_statics(self);
                return None;
            }
            if sp1.take("RepeatPlayers").as_deref() != Some(expected_rep) {
                restore_statics(self);
                return None;
            }
            let Some(rep_sub) = sp1.take("RepeatSubAbility") else {
                restore_statics(self);
                return None;
            };
            let after_rep = sp1.take("SubAbility");
            if !sp1.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(lose_body) = self.svars.get(rep_sub.trim()).cloned() else {
                restore_statics(self);
                return None;
            };
            let Some((s2_api, mut sp2)) = Params::parse(&lose_body) else {
                restore_statics(self);
                return None;
            };
            sp2.drop_prose();
            if s2_api != "LoseLife" || sp2.take("Defined").as_deref() != Some("Remembered") {
                restore_statics(self);
                return None;
            }
            let Some(raw_amt) = sp2.take("LifeAmount") else {
                restore_statics(self);
                return None;
            };
            let Some(amt) = self
                .wave30c_player_count_remembered_life(&raw_amt)
                .or_else(|| self.amount_or_count_ext(&raw_amt))
            else {
                restore_statics(self);
                return None;
            };
            if !sp2.exhausted() {
                restore_statics(self);
                return None;
            }
            let Some(trailing) = self.wave30c_consume_cleanup(after_rep, false) else {
                restore_statics(self);
                return None;
            };
            let Some(sac_expr) = self.filter_expr(&sac_valid) else {
                restore_statics(self);
                return None;
            };
            let sac_name = self.body.filter_static("SAC", &sac_expr);
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::RepeatEach {{ domain: {domain}, \
                 effects: &[Effect::CaptureAffected {{ \
                 first: &[Effect::SacrificeFilter {{ who: PlayerRel::EventPlayer, \
                 filter: &{sac_name} }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::RememberedCount, op: AmountCmp::Equal, \
                 right: &Amount::Fixed(0) }}, \
                 then: &[Effect::LoseLife {{ amount: {amt}, target: PlayerRel::EventPlayer }}], \
                 otherwise: &[] }}] }}] }}"
            )]);
        }

        None
    }

    fn wave30c_instant_sorcery_sa_filter(&self, raw: &str) -> Option<String> {
        let mut alts = Vec::new();
        for part in raw.split(',').map(str::trim) {
            let expr = match part {
                "Instant" => "Filter::INSTANT".to_string(),
                "Sorcery" => "Filter::SORCERY".to_string(),
                _ => {
                    let suffix = part.strip_prefix("Spell.")?;
                    self.filter_expr_in_zone(&format!("Card.{suffix}"), true)?
                }
            };
            alts.push(expr);
        }
        match alts.len() {
            0 => None,
            1 => Some(alts.remove(0)),
            _ => Some(format!("Filter::Or(&[{}])", alts.join(", "))),
        }
    }

    fn wave30c_braids_fallback_chain(&mut self, mut next: Option<String>) -> Option<Vec<String>> {
        let mut effs = Vec::new();
        while let Some(name) = next {
            let body = self.svars.get(name.trim())?.clone();
            let (api, mut fp) = Params::parse(&body)?;
            fp.drop_prose();
            if fp.take("ConditionSVarCompare").as_deref() != Some("EQ0") {
                return None;
            }
            let var = fp.take("ConditionCheckSVar")?;
            if self
                .svars
                .get(var.trim())
                .map(String::as_str)
                .map(str::trim)
                != Some("Remembered$Valid Card.RememberedPlayerCtrl")
            {
                return None;
            }
            next = fp.take("SubAbility");
            let eff = match api.as_str() {
                "LoseLife" => {
                    if fp.take("Defined").as_deref() != Some("Player.IsRemembered") {
                        return None;
                    }
                    let amt = self.amount_or_count_ext(&fp.take("LifeAmount")?)?;
                    if !fp.exhausted() {
                        return None;
                    }
                    format!("Effect::LoseLife {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
                }
                "Draw" => {
                    if !matches!(fp.take("Defined").as_deref(), None | Some("You")) {
                        return None;
                    }
                    let amt =
                        self.amount_or_count_ext(fp.take("NumCards").as_deref().unwrap_or("1"))?;
                    if !fp.exhausted() {
                        return None;
                    }
                    format!("Effect::DrawCards {{ amount: {amt} }}")
                }
                _ => return None,
            };
            effs.push(eff);
        }
        (!effs.is_empty()).then_some(effs)
    }

    #[allow(clippy::option_option)]
    fn wave30c_consume_cleanup(
        &self,
        next: Option<String>,
        allow_imprinted: bool,
    ) -> Option<Option<String>> {
        let Some(name) = next else {
            return Some(None);
        };
        let body = self.svars.get(name.trim())?;
        let (api, mut cp) = Params::parse(body)?;
        cp.drop_prose();
        if api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
            return None;
        }
        if allow_imprinted && !matches!(cp.take("ClearImprinted").as_deref(), None | Some("True")) {
            return None;
        }
        let after = cp.take("SubAbility");
        cp.exhausted().then_some(after)
    }

    fn wave30c_reconstruct_spec(api: &str, p: &Params) -> String {
        let mut parts = vec![format!("DB$ {api}")];
        for (k, v) in &p.entries {
            parts.push(format!("{k}$ {v}"));
        }
        parts.join(" | ")
    }

    fn wave30c_strip_filter_attr(filter: &str, attr: &str) -> Option<String> {
        let trimmed = filter.trim();
        let (base, attrs) = trimmed.split_once('.')?;
        let mut kept: Vec<&str> = Vec::new();
        let mut found = false;
        for part in attrs.split('+') {
            if part == attr {
                found = true;
            } else {
                kept.push(part);
            }
        }
        if !found {
            return None;
        }
        Some(if kept.is_empty() {
            base.to_string()
        } else {
            format!("{base}.{}", kept.join("+"))
        })
    }
}
