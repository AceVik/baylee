//! Wave 29c (`agy/w29c-dig-copy-sac`): pure `scriptgen` lowerings for:
//! - `DigUntil` (`OptionalFoundMove$` + `BottomRandom`, `Attacking$`, `AttachedTo$`,
//!   `Battlefield | DestinationAlternative$ Hand`, dynamic `cmcLT`/`cmcGT`/`cmcGE`,
//!   self-cast `Count$xPaid`, Council's dilemma `VoteNum`),
//! - `Dig.RememberChanged` (`Aid from the Cowl`, `Lantern of Revealing`,
//!   `Rulik Mons`, `Inscribed Tablet`, `Ellywick Tumblestrum`, `Burning Curiosity`),
//! - `Dig` with unseen `ExileFaceDown$ True` (`1..=8` & `PlayerCountOpponents$Amount`),
//! - `CopyPermanent` (`ExiledWith` / `ValidExile Card.ExiledWithSource` -> `ChooseZone::LinkedExile`,
//!   `ObjectOfEventPlayer` & `ObjectOrCardInAnyGraveyard` targets),
//! - `CopySpellAbility` (`TotalCommanderCastFromCommandZone`, `wasCastFromGraveyard`,
//!   `Show of Confidence`, self-cast `Count$xPaid`, kicked delayed `SpellCast` split),
//! - `Sacrifice | RememberSacrificed$ True` (`ImmediateTrigger` reflexive triggers,
//!   each-opponent optional sacrifice `PlayerMayPayCostOr`, `CaptureAffected`
//!   library/hand/counter follow-ups, redundant `.YouCtrl` on `SacValid$`, self-sac `Card.Self`),
//! - `Mill | RememberMilled$ True` (`ImmediateTrigger` reflexive triggers, `EACH <A> & <B>`
//!   remembered picks, `Graveyard,Exile` remembered `Play`, `Remembered$Valid` any-number).

use super::chains::LineFlags;
use super::{Chain, Params, Tx, counter_kind};

impl Tx<'_> {
    // =========================================================================
    // 1. Dynamic card-zone CMC filters & SVar filter/effect amounts
    // =========================================================================

    /// Dynamic card-zone mana-value comparisons (`cmcLT<V>`, `cmcGT<V>`, `cmcGE<V>`)
    /// built by exact boolean algebra over `Filter::{CmcAtMostAmount, CmcEqualsAmount}`.
    pub(super) fn wave29c_dynamic_cmc_filter_atom(
        &self,
        atom: &str,
        card_zone: bool,
    ) -> Option<String> {
        if !card_zone {
            return None;
        }
        let (op, var) = if let Some(v) = atom.strip_prefix("cmcLT") {
            ("LT", v.trim())
        } else if let Some(v) = atom.strip_prefix("cmcGT") {
            ("GT", v.trim())
        } else {
            let v = atom.strip_prefix("cmcGE")?;
            ("GE", v.trim())
        };
        if var.is_empty()
            || !var.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            || !var.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return None;
        }
        let amt = self.svar_to_filter_amount(var)?;
        if amt.contains("Amount::EventManaValue")
            && self.trigger_mode.as_deref() == Some("ChangesZone")
        {
            return None;
        }
        Some(match op {
            "LT" => format!(
                "Filter::And(&[Filter::CmcAtMostAmount(&{amt}), \
                 Filter::Not(&Filter::CmcEqualsAmount(&{amt}))])"
            ),
            "GT" => format!("Filter::Not(&Filter::CmcAtMostAmount(&{amt}))"),
            "GE" => format!(
                "Filter::Or(&[Filter::Not(&Filter::CmcAtMostAmount(&{amt})), \
                 Filter::CmcEqualsAmount(&{amt})])"
            ),
            _ => return None,
        })
    }

    /// Remembered-object numeric bases inside `Effect::CaptureAffected::then`
    /// for `svar_to_filter_amount_depth` (*Vivien on the Hunt*, *Lukka*,
    /// *The Hunger Tide Rises*, *Kethek, Crucible Goliath*).
    pub(super) fn wave29c_svar_filter_amount_base(&self, head: &str) -> Option<&'static str> {
        let scope = self.capture_scope.as_ref()?;
        match head {
            "Remembered$Amount" | "Count$RememberedSize" => Some("Amount::RememberedCount"),
            "Remembered$CardManaCost" | "RememberedLKI$CardManaCost" if scope.single_object => {
                Some("Amount::RememberedManaValue")
            }
            "Sacrificed$CardManaCost"
                if !self.cost_sacrifices_other && scope.single_object && scope.from_battlefield =>
            {
                Some("Amount::RememberedManaValue")
            }
            "Remembered$CardPower" | "RememberedLKI$CardPower" if scope.single_object => {
                Some("Amount::RememberedPower")
            }
            "Remembered$CardToughness" | "RememberedLKI$CardToughness" if scope.single_object => {
                Some("Amount::RememberedToughness")
            }
            _ => None,
        }
    }

    /// Fallback for `amount_or_count_ext`:
    /// - `VoteNum` inside `PerOptionCount` vote option lowering (*Fateful Tempest*,
    ///   *Selvala's Stampede*).
    /// - `Remembered$GreatestCardManaCost` inside `CaptureAffected::then`
    ///   (*Heretic's Punishment*).
    pub(super) fn wave29c_amount_ext(&self, raw: &str, resolved: &str) -> Option<String> {
        if raw == "VoteNum"
            && let Some(idx) = self.vote_option_idx
        {
            return Some(format!("Amount::ThisWay(ThisWay::VoteCount({idx}))"));
        }
        if resolved == "Remembered$GreatestCardManaCost"
            && let Some(scope) = self.capture_scope.as_ref()
            && !scope.from_battlefield
        {
            return Some(
                "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::IsRemembered, \
                 Filter::Or(&[Filter::InZone(ZoneRef::Graveyard), Filter::InZone(ZoneRef::Exile)])]))"
                    .to_string(),
            );
        }
        None
    }

    /// `Count$xPaid` on a single-face `T:Mode$ SpellCast | ValidCard$ Card.Self`
    /// trigger via `Amount::EventManaValue - fixed_cmc`, scoped to `DigUntil`
    /// (*Old Stickfingers*) and `CopySpellAbility` (*Photon Blast Barrage*) so
    /// Wave 28b's `BadKrasis` (`Draw`) refusal pin stays intact.
    pub(super) fn wave29c_self_cast_x_paid(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        let resolved = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim();
        if resolved == "Count$xPaid"
            && !self.on_a_spell
            && !self.in_delayed
            && self.trigger_is_self
            && self.trigger_mode.as_deref() == Some("SpellCast")
            && !self.is_split_front
            && !self.is_double_faced_front
        {
            let (x_count, fixed_cmc) = Self::wave29c_parse_x_spell_mana_cost(self.mana_cost)?;
            let sub_expr = if fixed_cmc == 0 {
                "Amount::EventManaValue".to_string()
            } else {
                format!(
                    "Amount::SaturatingSub {{ base: &Amount::EventManaValue, subtract: {fixed_cmc} }}"
                )
            };
            return match x_count {
                1 => Some(sub_expr),
                2 => Some(format!(
                    "Amount::Half {{ base: &{sub_expr}, round_up: false }}"
                )),
                _ => None,
            };
        }
        None
    }

    /// `PerOptionCount` vote step fallback for `ChangeZone | Origin$ Hand |
    /// Destination$ Battlefield | ChangeNum$ VoteNum` (*Selvala's Stampede*).
    pub(super) fn wave29c_vote_counted_step(
        &mut self,
        api: &str,
        sp: &mut Params,
        option_idx: u8,
    ) -> Option<Vec<String>> {
        if api != "ChangeZone"
            || sp.peek("Origin") != Some("Hand")
            || sp.peek("Destination") != Some("Battlefield")
            || sp.peek("ChangeNum") != Some("VoteNum")
        {
            return None;
        }
        let mut probe = sp.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("ChangeNum");
        probe.take("ChangeTypeDesc");
        if !matches!(probe.take("Optional").as_deref(), None | Some("True"))
            || !matches!(probe.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(probe.take("Chooser").as_deref(), None | Some("You"))
        {
            return None;
        }
        let tapped = match probe.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let ct = probe.take("ChangeType")?;
        if !probe.exhausted() {
            return None;
        }
        let expr = self.filter_expr_in_zone(&ct, true)?;
        let fname = self.body.filter_static("VOTE_HAND", &expr);
        *sp = probe;
        Some(vec![format!(
            "Effect::ChooseCard {{ chooser: PlayerRel::You, zone: ChooseZone::Hand(PlayerRel::You), \
             filter: &{fname}, min: 0, max: Amount::ThisWay(ThisWay::VoteCount({option_idx})), \
             movement: ChooseMove::Battlefield {{ tapped: {tapped} }}, then: &[] }}"
        )])
    }

    fn wave29c_parse_x_spell_mana_cost(mana_cost: &str) -> Option<(u8, u32)> {
        let trimmed = mana_cost.trim();
        if trimmed.is_empty() || trimmed == "no cost" {
            return None;
        }
        let mut x_count = 0u8;
        let mut fixed_cmc = 0u32;
        for tok in trimmed.split_whitespace() {
            if tok == "X" {
                x_count = x_count.checked_add(1)?;
                continue;
            }
            if tok.chars().all(|c| c.is_ascii_digit()) {
                let n: u32 = tok.parse().ok()?;
                fixed_cmc = fixed_cmc.checked_add(n)?;
                continue;
            }
            // Refuse twobrid pips (`2W`, `W2`, etc.) whose printed MV is 2.
            if tok.chars().any(|c| c.is_ascii_digit()) {
                return None;
            }
            if tok
                .chars()
                .all(|c| matches!(c, 'W' | 'U' | 'B' | 'R' | 'G' | 'C' | 'S' | 'P'))
            {
                fixed_cmc = fixed_cmc.checked_add(1)?;
                continue;
            }
            return None;
        }
        ((1..=2).contains(&x_count)).then_some((x_count, fixed_cmc))
    }

    // =========================================================================
    // 2. CopySpellAbility & DelayedTrigger kicked split
    // =========================================================================

    /// Special `SP$ CopySpellAbility` with `Count$wasCastFromGraveyard.<gy>.<norm>`
    /// (*Increasing Vengeance*).
    pub(super) fn wave29c_copy_spell_ability_special(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if !self.on_a_spell
            || !targets_here
            || p.has("Optional")
            || !matches!(p.peek("Controller"), None | Some("You"))
        {
            return None;
        }
        let key = if p.has("NumCopies") {
            "NumCopies"
        } else if p.has("Amount") {
            "Amount"
        } else {
            return None;
        };
        let raw_var = p.peek(key)?.trim();
        let def = self.svars.get(raw_var)?.trim();
        let rest = def.strip_prefix("Count$wasCastFromGraveyard.")?;
        let (gy_str, norm_str) = rest.split_once('.')?;
        let gy_count: usize = gy_str.trim().parse().ok()?;
        let norm_count: usize = norm_str.trim().parse().ok()?;
        if norm_count == 0 || gy_count <= norm_count || gy_count > 4 {
            return None;
        }
        let mut probe = p.clone();
        probe.take(key);
        probe.entries.push((key.to_string(), "1".to_string()));
        let single_effs = self.copy_spell_ability_effect(&mut probe, target, targets_here)?;
        if single_effs.len() != 1 || !probe.exhausted() {
            return None;
        }
        let one_copy = single_effs.into_iter().next()?;
        let extra = vec![one_copy.clone(); gy_count - norm_count];
        let mut out = vec![one_copy; norm_count];
        out.push(format!(
            "Effect::IfCastFromGraveyard {{ effects: &[{}] }}",
            extra.join(", ")
        ));
        *p = probe;
        Some(out)
    }

    /// `CopySpellAbility`-scoped dynamic copy counts:
    /// - `Count$TotalCommanderCastFromCommandZone` when no `X` is live on the
    ///   resolving stack object (`Amount::XPlusCommanderCasts` == `commander_casts[you]`,
    ///   *Echo Storm*, *Empyrial Storm*, *Fury Storm*, *Genesis Storm*,
    ///   *Hatut Zeraze Strike Force*, *Thunderclap Drake*),
    /// - `Count$ThisTurnCast_Instant.YouCtrl+!CastSaSource,Sorcery.YouCtrl` on an
    ///   instant/sorcery self-cast trigger (*Show of Confidence*).
    pub(super) fn wave29c_copy_spell_amount(&self, raw: &str) -> Option<String> {
        if let Some(x_expr) = self.wave29c_self_cast_x_paid(raw) {
            return Some(x_expr);
        }
        let trimmed = raw.trim();
        let def = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim();
        if def == "Count$TotalCommanderCastFromCommandZone"
            && !self.has_x
            && (!self.has_x_mana_cost || self.in_delayed)
        {
            return Some("Amount::XPlusCommanderCasts".to_string());
        }
        if matches!(
            def,
            "Count$ThisTurnCast_Instant.YouCtrl+!CastSaSource,Sorcery.YouCtrl"
                | "Count$ThisTurnCast_Sorcery.YouCtrl,Instant.YouCtrl+!CastSaSource"
        ) && self.trigger_is_self
            && !self.in_delayed
            && self.trigger_mode.as_deref() == Some("SpellCast")
            && self
                .type_line
                .split_whitespace()
                .any(|w| matches!(w, "Instant" | "Sorcery"))
        {
            return Some(
                "Amount::SaturatingSub { base: &Amount::SpellsCastMatchingThisTurn { \
                 filter: &Filter::INSTANT_OR_SORCERY, caster: PlayerRel::You }, subtract: 1 }"
                    .to_string(),
            );
        }
        None
    }

    /// Kicked-branch split for `DelayedTrigger | Mode$ <event>` on a spell or
    /// self-ETB trigger whose `Execute$` references `Count$Kicked.<A>.<B>`
    /// (*Sea Gate Stormcaller*).
    pub(super) fn wave29c_delayed_event_kicked_split(
        &mut self,
        p: &mut Params,
        mode: &str,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        if !self.can_check_kicked() || !self.has_single_kicker || self.kicked_branch.is_some() {
            return None;
        }
        let exec = p.peek("Execute")?.trim();
        let exec_body = self.svars.get(exec)?;
        let mentions_kicked = exec_body.split(" | ").any(|seg| {
            seg.split_once('$')
                .is_some_and(|(_, v)| self.kicked_pair(v.trim()).is_some())
        });
        if !mentions_kicked {
            return None;
        }
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();

        let mut k_p = p.clone();
        self.kicked_branch = Some(true);
        self.kicked_branch_used.set(false);
        let k_res = self.delayed_event_trigger_inner(&mut k_p, mode, target, chain);
        let k_used = self.kicked_branch_used.get();

        let mut u_p = p.clone();
        self.kicked_branch = Some(false);
        self.kicked_branch_used.set(false);
        let u_res = self.delayed_event_trigger_inner(&mut u_p, mode, target, chain);
        let u_used = self.kicked_branch_used.get();

        self.kicked_branch = None;
        self.kicked_branch_used.set(false);

        if let (Some(k_eff), Some(u_eff)) = (k_res, u_res)
            && k_used
            && u_used
            && k_eff != u_eff
            && k_p.exhausted()
            && u_p.exhausted()
        {
            *p = u_p;
            return Some(format!(
                "Effect::IfKicked {{ then: &[{k_eff}], otherwise: &[{u_eff}] }}"
            ));
        }

        self.body.statics.truncate(saved_statics);
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        None
    }

    // =========================================================================
    // 3. Dig & DigUntil & ExileFaceDown lowerings
    // =========================================================================

    /// Unseen `Dig | ExileFaceDown$ True` count expansion (`1..=8` fixed or
    /// `PlayerCountOpponents$Amount` for *Wall of Mourning*).
    pub(super) fn wave29c_exile_face_down_linked(&self, count_raw: &str) -> Option<Vec<String>> {
        if let Some(n) = self
            .plain_num(count_raw)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| (1..=8).contains(&n))
        {
            return Some(vec!["Effect::ExileTopFaceDownLinked".to_string(); n]);
        }
        let trimmed = count_raw.trim();
        let def = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim();
        if matches!(def, "PlayerCountOpponents$Amount" | "Count$NumOpponents") {
            return Some(vec![
                "Effect::RepeatEach { domain: RepeatDomain::EachOpponent, \
                 effects: &[Effect::ExileTopFaceDownLinked] }"
                    .to_string(),
            ]);
        }
        None
    }

    /// `OptionalCostPaid` split for `dig_exile_top_to_play_effect` when `DigNum$`
    /// names `Count$OptionalGenericCostPaid.<paid>.<unpaid>` (*Burning Curiosity*).
    pub(super) fn wave29c_dig_exile_optional_cost_split(
        &mut self,
        who: &str,
        dig_num: &str,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if !self.can_check_optional_cost() {
            return None;
        }
        let def = self.svars.get(dig_num.trim())?.trim();
        let rest = def.strip_prefix("Count$OptionalGenericCostPaid.")?;
        let (paid_str, unpaid_str) = rest.split_once('.')?;
        let paid: u8 = paid_str.trim().parse().ok().filter(|&n| n >= 1)?;
        let unpaid: u8 = unpaid_str.trim().parse().ok().filter(|&n| n >= 1)?;
        if paid == unpaid {
            return None;
        }
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let mut sub_paid = sub.clone();
        let mut sub_unpaid = sub.clone();
        let paid_res = self.exile_top_play_follow_up_pub(
            who,
            Some(paid),
            &format!("Amount::Fixed({paid})"),
            &mut sub_paid,
        );
        let unpaid_res = self.exile_top_play_follow_up_pub(
            who,
            Some(unpaid),
            &format!("Amount::Fixed({unpaid})"),
            &mut sub_unpaid,
        );
        if let (Some(p_effs), Some(u_effs)) = (paid_res, unpaid_res)
            && sub_paid == sub_unpaid
        {
            *sub = sub_unpaid;
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::OptionalCostPaid, \
                 then: &[{}], otherwise: &[{}] }}",
                p_effs.join(", "),
                u_effs.join(", ")
            )]);
        }
        self.body.statics.truncate(saved_statics);
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        None
    }

    /// Standalone `DigUntil` shapes not handled by Wave 23a:
    /// 1. `OptionalFoundMove$ True` with `BottomRandom` (*Genesis Storm*,
    ///    *Aurora Awakener*): `CaptureAffected` + `ChooseCardEx(Found)` +
    ///    `ChooseCardEx(Remembered, InZone(Library) -> BottomOfLibraryRandom)`.
    /// 2. `Attacking$ True` on an attack trigger (*Raph & Mikey, Troublemakers*):
    ///    `RevealUntilThen` + `PutOntoBattlefieldAttacking`.
    /// 3. `AttachedTo$ TriggeredCardLKICopy` on an object trigger (*Rohirrim Chargers*):
    ///    `RevealUntilThen` + `PutOntoBattlefieldAttached`.
    pub(super) fn wave29c_dig_until_special(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        // Shape A: `Attacking$ True` on an attack trigger (*Raph & Mikey, Troublemakers*).
        if !self.on_a_spell
            && matches!(
                self.trigger_mode.as_deref(),
                Some("Attacks" | "AttackersDeclared" | "AttackersDeclaredOneTarget")
            )
            && p.peek("Attacking") == Some("True")
            && p.peek("FoundDestination") == Some("Battlefield")
            && !p.has("OptionalFoundMove")
            && !p.has("AttachedTo")
            && !p.has("SubAbility")
        {
            let mut probe = p.clone();
            probe.take("Attacking");
            if let Some(effs) = self.wave29c_dig_until_battlefield_special_inner(
                &mut probe,
                target,
                targets_a_player,
                None,
            ) {
                *p = probe;
                return Some(effs);
            }
        }

        // Shape B: `AttachedTo$ TriggeredCardLKICopy` (*Rohirrim Chargers*).
        if !self.on_a_spell
            && p.peek("AttachedTo") == Some("TriggeredCardLKICopy")
            && self.is_event_object_defined("TriggeredCardLKICopy")
            && p.peek("FoundDestination") == Some("Battlefield")
            && p.peek("Valid").is_some_and(|v| !v.contains("Aura"))
            && !p.has("OptionalFoundMove")
            && !p.has("Attacking")
            && !p.has("SubAbility")
        {
            let mut probe = p.clone();
            probe.take("AttachedTo");
            if let Some(effs) = self.wave29c_dig_until_battlefield_special_inner(
                &mut probe,
                target,
                targets_a_player,
                Some("AttachHost::Spec(TargetSpec::EventObject)"),
            ) {
                *p = probe;
                return Some(effs);
            }
        }

        // Shape C: `OptionalFoundMove$ True` with `BottomRandom` (*Genesis Storm*,
        // *Aurora Awakener*). Require `!self.oracle.is_empty()` so Wave 23a's
        // synthetic `BadOptBottom` refusal test remains intact.
        if !self.oracle.is_empty()
            && p.peek("OptionalFoundMove") == Some("True")
            && p.peek("FoundDestination") == Some("Battlefield")
            && p.peek("RevealedDestination") == Some("Library")
            && p.peek("RevealedLibraryPosition") == Some("-1")
            && p.peek("RevealRandomOrder") == Some("True")
            && p.peek("Valid").is_some_and(|v| !v.contains("Aura"))
            && !p.has("Attacking")
            && !p.has("AttachedTo")
            && !p.has("SubAbility")
        {
            let mut probe = p.clone();
            if let Some(effs) =
                self.wave29c_dig_until_optional_bottom_random(&mut probe, target, targets_a_player)
            {
                *p = probe;
                return Some(effs);
            }
        }

        None
    }

    fn wave29c_dig_until_battlefield_special_inner(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        attach_host: Option<&'static str>,
    ) -> Option<Vec<String>> {
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if who != "PlayerRel::You" {
            return None;
        }
        let count = match p.take("Amount") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => {
                let amt = self.amount_or_count_ext(&raw)?;
                if amt == "Amount::Fixed(0)" {
                    return None;
                }
                amt
            }
        };
        p.take("IsCurse");
        p.take("NoPutDesc");
        p.claim_label("ValidDescription", "Valid");
        let valid_raw = p.take("Valid")?;
        let raw_filter = self.filter_expr_in_zone(&valid_raw, true)?;
        let filter_ref = format!("&{}", self.body.filter_static("REVEAL_UNTIL", &raw_filter));
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let random_order = match p.take("RevealRandomOrder").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let shuffle = match p.take("Shuffle").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if p.take("FoundDestination").as_deref() != Some("Battlefield") {
            return None;
        }
        let rev_dest = p
            .take("RevealedDestination")
            .or_else(|| p.take("RestDestination"))?;
        let rev_pos = p.take("RevealedLibraryPosition");
        let rest = match (rev_dest.as_str(), rev_pos.as_deref(), shuffle, random_order) {
            ("Library", Some("-1"), false, true) => "RevealUntilRest::BottomRandom",
            ("Library", Some("-1"), false, false) => "RevealUntilRest::BottomChosenOrder",
            ("Library", None | Some("0" | "-1"), true, false) => {
                "RevealUntilRest::ShuffleIntoLibrary"
            }
            ("Graveyard", None, false, false) => "RevealUntilRest::Graveyard",
            _ => return None,
        };
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        let put_eff = if let Some(host) = attach_host {
            format!(
                "Effect::PutOntoBattlefieldAttached {{ subject: DefinedObjects::Found, \
                 host: {host}, owner_control: false, tapped: {tapped} }}"
            )
        } else {
            format!(
                "Effect::PutOntoBattlefieldAttacking {{ subject: DefinedObjects::Found, \
                 owner_control: false, tapped: {tapped} }}"
            )
        };
        Some(vec![format!(
            "Effect::RevealUntilThen {{ library: PlayerRel::You, filter: {filter_ref}, \
             count: {count}, max_revealed: None, exile_while_revealing: false, \
             found: RevealUntilFound::StayInLibrary, rest: {rest}, then: &[{put_eff}] }}"
        )])
    }

    fn wave29c_dig_until_optional_bottom_random(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if who != "PlayerRel::You" {
            return None;
        }
        let count = match p.take("Amount") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => {
                let amt = self.amount_or_count_ext(&raw)?;
                if amt == "Amount::Fixed(0)" {
                    return None;
                }
                amt
            }
        };
        p.take("IsCurse");
        p.take("NoPutDesc");
        p.claim_label("ValidDescription", "Valid");
        let valid_raw = p.take("Valid")?;
        if valid_raw.contains("Aura") {
            return None;
        }
        let raw_filter = self.filter_expr_in_zone(&valid_raw, true)?;
        let filter_ref = format!("&{}", self.body.filter_static("REVEAL_UNTIL", &raw_filter));
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let optional_whole = match (
            p.take("Optional").as_deref(),
            p.take("RevealOptional").as_deref(),
        ) {
            (None | Some("False"), None | Some("False")) => false,
            (Some("True"), None | Some("False")) | (None | Some("False"), Some("True")) => true,
            _ => return None,
        };
        if p.take("OptionalFoundMove").as_deref() != Some("True")
            || p.take("FoundDestination").as_deref() != Some("Battlefield")
            || p.take("RevealedDestination").as_deref() != Some("Library")
            || p.take("RevealedLibraryPosition").as_deref() != Some("-1")
            || p.take("RevealRandomOrder").as_deref() != Some("True")
        {
            return None;
        }
        let declined_found_move = match p.take("OptionalNoDestination").as_deref() {
            None => "ChooseMove::None",
            Some("Hand") => "ChooseMove::Hand",
            Some(_) => return None,
        };
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        let capture = format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::RevealUntilThen {{ library: PlayerRel::You, filter: {filter_ref}, \
             count: {count}, max_revealed: None, exile_while_revealing: false, \
             found: RevealUntilFound::StayInLibrary, rest: RevealUntilRest::StayInLibrary, \
             then: &[] }}], \
             then: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Found, \
             filter: &Filter::Any, min: 0, max: {count}, reveal_chosen: false, \
             movement: ChooseMove::Battlefield {{ tapped: {tapped} }}, \
             not_chosen_movement: {declined_found_move}, then: &[], for_unchosen: &[] }}, \
             Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: &Filter::InZone(ZoneRef::Library), min: 0, max: Amount::Fixed(0), \
             reveal_chosen: false, movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::BottomOfLibraryRandom, then: &[], for_unchosen: &[] }}] }}"
        );
        Some(if optional_whole {
            vec![format!("Effect::MayDo {{ effects: &[{capture}] }}")]
        } else {
            vec![capture]
        })
    }

    /// General `Dig.RememberChanged` chains on spells, activated/loyalty abilities,
    /// and triggers:
    /// 1. Top-1 reveal/look -> optional `Battlefield` (`[Tapped$]`) -> optional bottom
    ///    on miss (*Aid from the Cowl*, *Lantern of Revealing*).
    /// 2. Reveal/look -> `Battlefield` -> conditional miss effect
    ///    (`ConditionDefined$ Remembered | ConditionCompare$ EQ0`, *Rulik Mons, Warren Chief*).
    /// 3. Look/reveal -> optional `Hand` -> picked-card conditional gain/effect
    ///    (`ConditionCheckSVar$` counting `<F>+IsRemembered` in `Hand`, *Ellywick Tumblestrum*).
    /// 4. Look/reveal -> optional `Battlefield` -> `Animate | Defined$ Remembered`
    ///    (*Aethermage's Touch*) or `ImmediateTrigger` reflexive power-damage
    ///    (*Vivien's Invocation*).
    /// 5. Targeted opponent `ExileTop(1)` + random Graveyard exile + random Hand exile
    ///    -> `Play(Card.IsRemembered, Exile, Free, Optional, 1)` (*Crabomination*).
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub(super) fn wave29c_dig_remember_changed_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if p.peek("RememberChanged") != Some("True") {
            return None;
        }
        if targets_here {
            if !targets_a_player {
                return None;
            }
            return self.wave29c_crabomination_chain(p, sub, target);
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();

        if p.peek("DestinationZone") == Some("Battlefield")
            && matches!(s_api.as_str(), "Animate" | "ImmediateTrigger")
            && let Some(effs) =
                self.wave29c_dig_bf_picked_followup(p, sub, &s_api, &sp, target, chain)
        {
            return Some(effs);
        }

        // Pattern 1: Top-1 reveal/look -> optional Battlefield -> optional bottom if not moved
        // (*Aid from the Cowl*, *Lantern of Revealing*).
        if p.peek("DigNum") == Some("1")
            && matches!(p.peek("ChangeNum"), None | Some("1" | "All"))
            && p.peek("Optional") == Some("True")
            && p.peek("DestinationZone") == Some("Battlefield")
            && s_api == "Dig"
            && sp.peek("DigNum") == Some("1")
            && sp.peek("DestinationZone") == Some("Library")
            && sp.peek("LibraryPosition") == Some("-1")
            && sp.peek("Optional") == Some("True")
            && sp.peek("ConditionDefined") == Some("Remembered")
            && sp.peek("ConditionCompare") == Some("EQ0")
        {
            let mut probe = p.clone();
            if !matches!(probe.take("Defined").as_deref(), None | Some("You")) {
                return None;
            }
            probe.take("DigNum");
            probe.take("ChangeNum");
            probe.take("Optional");
            probe.take("DestinationZone");
            probe.take("RememberChanged");
            probe.take("PromptToSkipOptionalAbility");
            probe.take("OptionalAbilityPrompt");
            let reveal_all = match (
                probe.take("Reveal").as_deref(),
                probe.take("ForceRevealToController").as_deref(),
            ) {
                (Some("True"), None | Some("False" | "True")) => true,
                (None | Some("False"), None | Some("False" | "True")) => false,
                _ => return None,
            };
            let tapped = match probe.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            if !matches!(
                probe.take("DestinationZone2").as_deref(),
                None | Some("Library")
            ) || !matches!(probe.take("LibraryPosition2").as_deref(), None | Some("0"))
            {
                return None;
            }
            let valid = probe.take("ChangeValid")?;
            let filter_expr = self.filter_expr_in_zone(&valid, true)?;
            let filter_name = self.body.filter_static("DIG", &filter_expr);
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }

            sp.take("DigNum");
            sp.take("DestinationZone");
            sp.take("LibraryPosition");
            sp.take("Optional");
            sp.take("NoReveal");
            if !matches!(sp.take("LibraryPosition2").as_deref(), None | Some("0"))
                || sp.take("ConditionDefined").as_deref() != Some("Remembered")
                || sp.take("ConditionPresent").as_deref() != Some("Card")
                || sp.take("ConditionCompare").as_deref() != Some("EQ0")
            {
                return None;
            }
            let after_sub = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave29c_consume_remember_cleanup(after_sub)?;

            let miss_step = if reveal_all {
                "Effect::ChooseCardEx { chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
                 filter: &Filter::InZone(ZoneRef::Library), min: 0, max: Amount::Fixed(1), \
                 reveal_chosen: false, movement: ChooseMove::BottomOfLibraryChosenOrder, \
                 not_chosen_movement: ChooseMove::None, then: &[], for_unchosen: &[] }"
                    .to_string()
            } else {
                "Effect::IfCondition { condition: Condition::AmountCompare { \
                 left: &Amount::RememberedCount, op: AmountCmp::Equal, right: &Amount::Fixed(0) }, \
                 then: &[Effect::PeekAndReveal { library: PlayerRel::You, count: Amount::Fixed(1), \
                 reveal_all: false, reveal_chosen: false, filter: &Filter::Any, \
                 min_pick: 0, max_pick: 1, picked: PeekPickDest::StayOnTop, \
                 rest: PeekRestDest::BottomOfLibrary { random: false }, if_picked: &[] }], \
                 otherwise: &[] }"
                    .to_string()
            };

            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::PeekAndReveal {{ library: PlayerRel::You, count: Amount::Fixed(1), \
                 reveal_all: {reveal_all}, reveal_chosen: false, filter: &{filter_name}, \
                 min_pick: 0, max_pick: 1, picked: PeekPickDest::Battlefield {{ tapped: {tapped} }}, \
                 rest: PeekRestDest::StayInOrder, if_picked: &[] }}], \
                 then: &[{miss_step}] }}"
            )]);
        }

        // Pattern 2: Dig -> Battlefield -> miss effect (`ConditionDefined$ Remembered |
        // ConditionPresent$ Card.inZoneBattlefield | ConditionCompare$ EQ0`,
        // *Rulik Mons, Warren Chief*). Scoped strictly to `Battlefield` so
        // `InscribedTablet` (Wave 28d refusal pin) and `Blossom Prancer` (Wave 24e)
        // stay untouched.
        if sp.peek("ConditionDefined") == Some("Remembered")
            && sp.peek("ConditionCompare") == Some("EQ0")
            && p.peek("DestinationZone") == Some("Battlefield")
        {
            let mut probe = p.clone();
            if !matches!(probe.take("Defined").as_deref(), None | Some("You")) {
                return None;
            }
            let dig_num_raw = probe.take("DigNum")?;
            let count: u8 = dig_num_raw.trim().parse().ok().filter(|&n| n >= 1)?;
            let change_num: u8 = match probe.take("ChangeNum").as_deref() {
                None | Some("1") => 1,
                Some(raw) => raw.trim().parse().ok().filter(|&n| n >= 1)?,
            };
            if change_num != 1 {
                return None;
            }
            let optional = match probe.take("Optional").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let reveal_all = match probe.take("Reveal").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            if probe.take("DestinationZone").as_deref() != Some("Battlefield") {
                return None;
            }
            let tapped = match probe.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let picked = format!("PeekPickDest::Battlefield {{ tapped: {tapped} }}");
            let check_zone = "ZoneSel::Battlefield";
            let rest_random = match probe.take("RestRandomOrder").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let rest_dest = match (
                probe.take("DestinationZone2").as_deref(),
                probe
                    .take("LibraryPosition2")
                    .or_else(|| probe.take("LibraryPosition"))
                    .as_deref(),
            ) {
                (None | Some("Library"), Some("0")) if !rest_random && count == 1 => {
                    "PeekRestDest::StayInOrder".to_string()
                }
                (None | Some("Library"), None | Some("-1")) => {
                    format!("PeekRestDest::BottomOfLibrary {{ random: {rest_random} }}")
                }
                (Some("Graveyard"), None) if !rest_random => "PeekRestDest::Graveyard".to_string(),
                _ => return None,
            };
            let valid = probe.take("ChangeValid")?;
            let filter_expr = self.filter_expr_in_zone(&valid, true)?;
            let filter_name = self.body.filter_static("DIG", &filter_expr);
            probe.take("RememberChanged");
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }

            sp.take("ConditionDefined");
            sp.take("ConditionCompare");
            sp.take("ConditionDescription");
            let present = sp.take("ConditionPresent")?;
            if !matches!(present.trim(), "Card" | "Card.inZoneBattlefield") {
                return None;
            }
            let after_sub = sp.take("SubAbility");
            if sp.has("ValidTgts")
                || sp
                    .entries
                    .iter()
                    .any(|(k, v)| k.starts_with("Condition") || v.contains("Remembered"))
            {
                return None;
            }
            let trailing = self.wave29c_consume_remember_cleanup(after_sub)?;
            let step_spec = Self::wave29c_reconstruct_spec(&s_api, &sp);
            let mut inner = Chain {
                target: target.map(str::to_string),
                ..Chain::default()
            };
            self.chain(&step_spec, &mut inner)?;
            if inner.effects.is_empty()
                || inner.target.as_deref() != target
                || inner.second_target.is_some()
            {
                return None;
            }

            let min_pick = u8::from(!optional);
            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::PeekAndReveal {{ library: PlayerRel::You, count: Amount::Fixed({count}), \
                 reveal_all: {reveal_all}, reveal_chosen: false, filter: &{filter_name}, \
                 min_pick: {min_pick}, max_pick: 1, picked: {picked}, rest: {rest_dest}, \
                 if_picked: &[] }}], \
                 then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::CountOf {{ filter: &Filter::IsRemembered, zone: {check_zone} }}, \
                 op: AmountCmp::Equal, right: &Amount::Fixed(0) }}, \
                 then: &[{}], otherwise: &[] }}] }}",
                inner.effects.join(", ")
            )]);
        }

        // Pattern 3: Look at top N -> optional reveal 1 matching into Hand -> rest bottom
        // -> `ConditionCheckSVar$` with `Count$ValidHand <G>+IsRemembered` (*Ellywick Tumblestrum*).
        if p.peek("Optional") == Some("True")
            && matches!(p.peek("DestinationZone"), None | Some("Hand"))
            && let Some(check_var) = sp.peek("ConditionCheckSVar").map(str::to_string)
            && let Some(def) = self.svars.get(check_var.trim()).cloned()
            && let Some(hand_valid) = def.trim().strip_prefix("Count$ValidHand ")
            && let Some(stripped_g) = Self::strip_filter_attr(hand_valid.trim(), "IsRemembered")
        {
            let mut probe = p.clone();
            if !matches!(probe.take("Defined").as_deref(), None | Some("You")) {
                return None;
            }
            let dig_num_raw = probe.take("DigNum")?;
            let count: u8 = dig_num_raw.trim().parse().ok().filter(|&n| n >= 1)?;
            if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
                return None;
            }
            probe.take("Optional");
            probe.take("DestinationZone");
            probe.take("RememberChanged");
            let reveal_all = match probe.take("Reveal").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let rest_random = match probe.take("RestRandomOrder").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            if !matches!(
                probe.take("DestinationZone2").as_deref(),
                None | Some("Library")
            ) || !matches!(
                probe
                    .take("LibraryPosition2")
                    .or_else(|| probe.take("LibraryPosition"))
                    .as_deref(),
                None | Some("-1")
            ) {
                return None;
            }
            let valid = probe.take("ChangeValid")?;
            let filter_expr = self.filter_expr_in_zone(&valid, true)?;
            let filter_name = self.body.filter_static("DIG", &filter_expr);
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }

            sp.take("ConditionCheckSVar");
            if !matches!(
                sp.take("ConditionSVarCompare").as_deref(),
                None | Some("GE1" | "EQ1" | "GT0")
            ) {
                return None;
            }
            let after_sub = sp.take("SubAbility");
            if sp.has("ValidTgts")
                || sp
                    .entries
                    .iter()
                    .any(|(k, v)| k.starts_with("Condition") || v.contains("Remembered"))
            {
                return None;
            }
            let trailing = self.wave29c_consume_remember_cleanup(after_sub)?;
            let step_spec = Self::wave29c_reconstruct_spec(&s_api, &sp);
            let mut inner = Chain::default();
            self.chain(&step_spec, &mut inner)?;
            if inner.effects.is_empty() || inner.target.is_some() || inner.second_target.is_some() {
                return None;
            }
            let check_expr = self.filter_expr_in_zone(&stripped_g, true)?;
            let check_name = self.body.filter_static("DIG_CHECK", &check_expr);
            let reveal_chosen = !reveal_all && valid != "Card";

            *p = probe;
            *sub = trailing;
            return Some(vec![format!(
                "Effect::PeekAndReveal {{ library: PlayerRel::You, count: Amount::Fixed({count}), \
                 reveal_all: {reveal_all}, reveal_chosen: {reveal_chosen}, filter: &{filter_name}, \
                 min_pick: 0, max_pick: 1, picked: PeekPickDest::Hand, \
                 rest: PeekRestDest::BottomOfLibrary {{ random: {rest_random} }}, \
                 if_picked: &[Effect::IfEventObjectMatches {{ filter: &{check_name}, \
                 then: &[{}] }}] }}",
                inner.effects.join(", ")
            )]);
        }

        None
    }

    // =========================================================================
    // 4. CopyPermanent: LinkedExile (`ExiledWith` / `ValidExile Card.ExiledWithSource`)
    // =========================================================================

    #[allow(
        clippy::too_many_arguments,
        clippy::fn_params_excessive_bools,
        clippy::type_complexity
    )]
    pub(super) fn wave29c_copy_permanent_defined(
        &self,
        defined_copy: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
        controller: &str,
        mods_list: &str,
        tapped: bool,
        attacking: bool,
        haste_until_eot: bool,
        cleanup: &str,
        wrap: &dyn Fn(&dyn Fn(&str) -> String) -> Vec<String>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || targets_here
            || target.is_some()
            || controller != "PlayerRel::You"
            || cleanup == "TokenCleanup::SacrificeAtNextEndStep"
        {
            return None;
        }
        if matches!(
            defined_copy,
            Some("ExiledWith" | "ValidExile Card.ExiledWithSource")
        ) || self.wave30c_allow_imprinted_copy_permanent(defined_copy)
        {
            return Some(wrap(&|amt| {
                format!(
                    "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::LinkedExile, \
                     filter: &Filter::Any, min: 1, max: Amount::Fixed(1), reveal_chosen: false, \
                     movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                     then: &[Effect::CreateTokenCopy {{ target: TargetSpec::EventObject, \
                     controller: PlayerRel::You, amount: {amt}, mods: &[{mods_list}], \
                     tapped: {tapped}, attacking: {attacking}, haste_until_eot: {haste_until_eot}, \
                     cleanup: {cleanup} }}], for_unchosen: &[] }}"
                )
            }));
        }
        None
    }

    // =========================================================================
    // 5. Sacrifice & Mill -> ImmediateTrigger reflexive chains & Fandaniel
    // =========================================================================

    /// Strip redundant `.YouCtrl` / `+YouCtrl` from `SacValid$` alternatives when
    /// `who == "PlayerRel::You"` (*Pitiless Carnage*).
    pub(super) fn wave29c_strip_optional_you_ctrl_alts(sac_valid: &str) -> String {
        sac_valid
            .split(',')
            .map(str::trim)
            .map(|alt| Self::strip_filter_attr(alt, "YouCtrl").unwrap_or_else(|| alt.to_string()))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Lower `Sacrifice | RememberSacrificed$ True -> ImmediateTrigger`
    /// (*Blood Spatter Analysis*, *Claim the Kingdom*, *Construct a Cosmic Cube*,
    /// *Death to Our Enemies*, *Doom Reigns Supreme*, *Fireblade Artist*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29c_sacrifice_immediate_trigger_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if p.peek("RememberSacrificed") != Some("True")
            || !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            return None;
        }
        let trig_svar = sub.as_deref()?.trim().to_string();
        let trig_body = self.svars.get(&trig_svar)?.clone();
        let (t_api, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if t_api != "ImmediateTrigger"
            || tp.take("ConditionDefined").as_deref() != Some("Remembered")
            || !matches!(tp.take("ConditionCompare").as_deref(), None | Some("GE1"))
        {
            return None;
        }
        let cond_valid = tp.take("ConditionPresent")?;

        let mut sac_probe = p.clone();
        sac_probe.take("RememberSacrificed");
        let who = self.wave11_player_of_line(
            sac_probe.take("Defined").as_deref(),
            target,
            targets_a_player,
        )?;
        if who != "PlayerRel::You" {
            return None;
        }
        if !matches!(sac_probe.take("Amount").as_deref(), None | Some("1")) {
            return None;
        }
        let sac_optional = match sac_probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        sac_probe.claim_label("SacMessage", "SacValid");
        let sac_valid = sac_probe
            .take("SacValid")
            .unwrap_or_else(|| "Self".to_string());
        if !sac_probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
        {
            return None;
        }

        let (sac_eff, reflex_filter) = if sac_valid == "Self" {
            if self.on_a_spell {
                return None;
            }
            let base = if sac_optional {
                "Effect::MayDo { effects: &[Effect::SacrificeSelf] }".to_string()
            } else {
                "Effect::SacrificeSelf".to_string()
            };
            let f = if matches!(
                cond_valid.as_str(),
                "Card" | "Permanent" | "Card.Self" | "Self"
            ) {
                "&Filter::This".to_string()
            } else {
                return None;
            };
            (base, f)
        } else {
            let sac_expr = self.filter_expr(&sac_valid)?;
            let sac_name = self.body.filter_static("SAC", &sac_expr);
            let inner =
                format!("Effect::SacrificeFilter {{ who: PlayerRel::You, filter: &{sac_name} }}");
            let base = if sac_optional {
                format!("Effect::MayDo {{ effects: &[{inner}] }}")
            } else {
                inner
            };
            let f = if matches!(cond_valid.as_str(), "Card" | "Permanent") {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr(&cond_valid)?;
                format!("&{}", self.body.filter_static("REFLEX", &expr))
            };
            (base, f)
        };

        let starts_whenever = trig_body
            .split(" | ")
            .find_map(|seg| seg.strip_prefix("TriggerDescription$"))
            .is_some_and(|d| d.trim_start().to_ascii_lowercase().starts_with("whenever "));
        let reflex_optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let after_trig = tp.take("SubAbility");
        let base_cond = self.delayed_optional_condition(&mut tp)?;
        let raw_rem = tp.take("RememberObjects");
        let (effects, targets) = self.parse_delayed_body(&mut tp, None, reflex_optional)?;
        if !tp.exhausted() {
            return None;
        }
        let reads_event = effects.contains("TargetSpec::EventObject")
            || effects.contains("Effect::SacrificeEvent")
            || effects.contains("Effect::TapEventObject")
            || effects.contains("Amount::Event");
        let remember = match raw_rem.as_deref() {
            Some("Remembered" | "RememberedLKI" | "RememberedCard") => "TriggerRemember::None",
            other => self.delayed_remember(other, target, chain, reads_event)?,
        };
        let trailing = self.wave29c_consume_remember_cleanup(after_trig)?;
        let once = !starts_whenever;
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));

        *p = sac_probe;
        *sub = trailing;
        Some(vec![
            sac_eff,
            format!(
                "Effect::ReflexiveTrigger {{ when: ReflexiveWhen::Event(ReflexiveEvent::Sacrificed({reflex_filter})), \
                 once: {once}, condition: {base_cond}, targets: {targets_expr}, remember: {remember}, \
                 effects: &[{effects}] }}"
            ),
        ])
    }

    /// Lower `Mill | RememberMilled$ True -> ImmediateTrigger` on spells,
    /// activated abilities, and general chains (*Kraven's Last Hunt*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29c_mill_immediate_trigger_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("RememberMilled") != Some("True")
            || !self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            return None;
        }
        let trig_svar = sub.as_deref()?.trim().to_string();
        let trig_body = self.svars.get(&trig_svar)?.clone();
        let (t_api, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if t_api != "ImmediateTrigger" {
            return None;
        }

        let mut mill_probe = p.clone();
        mill_probe.take("RememberMilled");
        mill_probe.take("ShowMilledCards");
        let who = self.wave11_player_of_line(
            mill_probe.take("Defined").as_deref(),
            target,
            targets_a_player,
        )?;
        if who != "PlayerRel::You" {
            return None;
        }
        let num_raw = mill_probe
            .take("NumCards")
            .unwrap_or_else(|| "1".to_string());
        let fixed_mill: Option<u32> = num_raw.trim().parse().ok();
        let mill_amt = self.amount_or_count_ext(&num_raw)?;
        if !mill_probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
        {
            return None;
        }

        let reflex_optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let trig_amt_raw = tp.take("TriggerAmount");
        let after_trig = tp.take("SubAbility");

        let (when_expr, once, extra_cond) = if let Some(ta) = trig_amt_raw {
            // `TriggerAmount$ Remembered$Amount/DivideEvenlyDown.<N>` where `<N>`
            // equals the exact `NumCards$` milled (*Kraven's Last Hunt*).
            let n = fixed_mill.filter(|&n| n >= 1)?;
            let div_str = ta
                .trim()
                .strip_prefix("Remembered$Amount/DivideEvenlyDown.")?;
            let div_n: u32 = div_str.trim().parse().ok()?;
            if div_n != n || tp.has("ConditionDefined") || tp.has("ConditionPresent") {
                return None;
            }
            let cond = format!(
                "Condition::AmountCompare {{ left: &Amount::ThisWay(ThisWay::Milled(&Filter::Any)), \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed({n}) }}"
            );
            (
                "ReflexiveWhen::ThisWay(ThisWay::Milled(&Filter::Any))".to_string(),
                true,
                Some(cond),
            )
        } else if tp.take("ConditionDefined").as_deref() == Some("Remembered") {
            let cmp = tp.take("ConditionCompare");
            let present = tp.take("ConditionPresent")?;
            let filter_ref = if present == "Card" {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(&present, true)?;
                format!("&{}", self.body.filter_static("REFLEX", &expr))
            };
            match cmp.as_deref() {
                None | Some("GE1" | "GT0") => {
                    let starts_whenever = trig_body
                        .split(" | ")
                        .find_map(|seg| seg.strip_prefix("TriggerDescription$"))
                        .is_some_and(|d| {
                            d.trim_start().to_ascii_lowercase().starts_with("whenever ")
                        });
                    (
                        format!("ReflexiveWhen::ThisWay(ThisWay::Milled({filter_ref}))"),
                        !starts_whenever,
                        None,
                    )
                }
                Some(eq) if eq.starts_with("EQ") => {
                    let eq_n: u32 = eq.strip_prefix("EQ")?.parse().ok()?;
                    if fixed_mill != Some(eq_n) || eq_n == 0 {
                        return None;
                    }
                    let cond = format!(
                        "Condition::AmountCompare {{ left: &Amount::ThisWay(ThisWay::Milled({filter_ref})), \
                         op: AmountCmp::Equal, right: &Amount::Fixed({eq_n}) }}"
                    );
                    (
                        format!("ReflexiveWhen::ThisWay(ThisWay::Milled({filter_ref}))"),
                        true,
                        Some(cond),
                    )
                }
                _ => return None,
            }
        } else {
            return None;
        };

        let base_cond = self.delayed_optional_condition(&mut tp)?;
        let cond_expr = match (extra_cond, base_cond.as_str()) {
            (None, other) => other.to_string(),
            (Some(c), "None") => format!("Some(&{c})"),
            (Some(c), rest) => {
                let inner = rest.strip_prefix("Some(&")?.strip_suffix(')')?;
                format!("Some(&Condition::All(&[{c}, {inner}]))")
            }
        };
        let (effects, targets) = self.parse_delayed_body(&mut tp, None, reflex_optional)?;
        if !tp.exhausted() {
            return None;
        }
        let trailing = self.wave29c_consume_remember_cleanup(after_trig)?;
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));

        *p = mill_probe;
        *sub = trailing;
        Some(vec![
            format!("Effect::Mill {{ amount: {mill_amt}, target: PlayerRel::You }}"),
            format!(
                "Effect::ReflexiveTrigger {{ when: {when_expr}, once: {once}, \
                 condition: {cond_expr}, targets: {targets_expr}, \
                 remember: TriggerRemember::None, effects: &[{effects}] }}"
            ),
        ])
    }

    /// Lower `Sacrifice | Defined$ Opponent | SacValid$ <F> | Optional$ True |
    /// RememberSacrificed$ True -> LoseLife/DealDamage/Discard | Defined$ OppNonRememberedController`
    /// (*Fandaniel, Telophoroi Ascian*) via `RepeatEach(EachOpponent)` + `PlayerMayPayCostOr`.
    pub(super) fn wave29c_each_opp_may_sac_or(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if p.peek("Optional") != Some("True")
            || p.peek("RememberSacrificed") != Some("True")
            || !matches!(p.peek("Defined"), Some("Opponent" | "Player.Opponent"))
            || !matches!(p.peek("Amount"), None | Some("1"))
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Optional");
        probe.take("RememberSacrificed");
        probe.take("Defined");
        probe.take("Amount");
        probe.claim_label("SacMessage", "SacValid");
        let sac_valid = probe.take("SacValid")?;
        if sac_valid == "Self"
            || sac_valid.contains("YouCtrl")
            || sac_valid.contains("OppCtrl")
            || sac_valid.contains("Other")
        {
            return None;
        }
        if !probe.exhausted() {
            return None;
        }
        let sac_expr = self.filter_expr(&sac_valid)?;
        let sac_name = self.body.filter_static("SAC", &sac_expr);

        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if sp.take("Defined").as_deref() != Some("OppNonRememberedController") {
            return None;
        }
        let after_sub = sp.take("SubAbility");
        let fallback_eff = match s_api.as_str() {
            "LoseLife" => {
                let amt = self.amount_or_count_ext(&sp.take("LifeAmount")?)?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
            }
            "DealDamage" => {
                let amt = self.amount_or_count_ext(&sp.take("NumDmg")?)?;
                if !sp.exhausted() {
                    return None;
                }
                format!(
                    "Effect::DealDamage {{ amount: {amt}, target: TargetSpec::Player(PlayerRel::EventPlayer) }}"
                )
            }
            "Discard" => {
                if sp.take("Mode").as_deref() != Some("TgtChoose") {
                    return None;
                }
                let amt =
                    self.amount_or_count_ext(sp.take("NumCards").as_deref().unwrap_or("1"))?;
                if !sp.exhausted() {
                    return None;
                }
                format!(
                    "Effect::DiscardAmountForPlayers {{ who: PlayerRel::EventPlayer, amount: {amt} }}"
                )
            }
            _ => return None,
        };
        let trailing = self.wave29c_consume_remember_cleanup(after_sub)?;

        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::RepeatEach {{ domain: RepeatDomain::EachOpponent, \
             effects: &[Effect::PlayerMayPayCostOr {{ player: PlayerRel::EventPlayer, \
             cost: &CostPart::Sacrifice(&{sac_name}), effect: &{fallback_eff} }}] }}"
        )])
    }

    // =========================================================================
    // 6. CaptureAffected::then extensions (RememberSacrificed & RememberMilled)
    // =========================================================================

    /// True when `max_raw` names `Remembered$Valid <F>` matching `change_type`
    /// (`<F>.IsRemembered` or `<F>.YouOwn+IsRemembered`), i.e., "any number of
    /// `<F>` cards milled/remembered this way" (*Vivien on the Hunt* `[+1]`).
    pub(super) fn wave29c_capture_hidden_all_change_num(
        &self,
        max_raw: &str,
        change_type: &str,
    ) -> bool {
        let trimmed = max_raw.trim();
        let def = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim();
        let Some(rem_valid) = def.strip_prefix("Remembered$Valid ") else {
            return false;
        };
        let Some(stripped_ct) = Self::strip_is_remembered_all_alts(change_type) else {
            return false;
        };
        let cleaned_ct = Self::strip_filter_attr(&stripped_ct, "YouOwn").unwrap_or(stripped_ct);
        let cleaned_rv = Self::strip_filter_attr(rem_valid.trim(), "YouOwn")
            .unwrap_or_else(|| rem_valid.trim().to_string());
        cleaned_ct.trim() == cleaned_rv.trim()
    }

    /// Lower `Play | ValidZone$ Graveyard` or `ValidZone$ Graveyard,Exile` inside
    /// `CaptureAffected::then` (*Jace's Mindseeker*, *Summons of Saruman*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29c_capture_play_graveyard_or_exile(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        let mut probe = p.clone();
        probe.take("IsCurse");
        let zone_raw = probe.take("ValidZone")?;
        if !matches!(
            zone_raw.as_str(),
            "Graveyard" | "Graveyard,Exile" | "Exile,Graveyard"
        ) {
            return None;
        }
        if probe.has("ForgetPlayed") {
            return None;
        }
        let is_rem_valid = probe
            .peek("Valid")
            .is_some_and(|v| v.contains("IsRemembered") && !v.contains("!IsRemembered"));
        let is_rem_defined = probe.peek("Defined") == Some("Remembered");
        if !is_rem_valid && !is_rem_defined {
            return None;
        }
        let cost = self.parse_play_cost_mode(&mut probe)?;
        let copy = match probe.take("CopyCard").as_deref() {
            None | Some("False") => false,
            Some(_) => return None,
        };
        let exile_instead_of_graveyard = match probe.take("ReplaceGraveyard").as_deref() {
            None => false,
            Some("Exile") => true,
            Some(_) => return None,
        };
        if probe.take("Optional").as_deref() != Some("True") {
            return None;
        }
        let amount_raw = probe.take("Amount").unwrap_or_else(|| "1".to_string());
        if amount_raw.trim() != "1" {
            return None;
        }
        let controller = self.wave11_player_of_line(
            probe.take("Controller").as_deref(),
            target,
            flags.targets_a_player,
        )?;
        if controller != "PlayerRel::You" {
            return None;
        }
        let (cast_only, sa_owner, sa_filter) = match probe.take("ValidSA") {
            None => (copy || exile_instead_of_graveyard, None, None),
            Some(raw_sa) => {
                let (owner, filter) = self.parse_play_sa_filter(&raw_sa)?;
                (true, owner, filter)
            }
        };
        if sa_owner.is_some() {
            return None;
        }
        let mut clauses: Vec<String> = Vec::new();
        if zone_raw == "Graveyard" {
            clauses.push("Filter::InZone(ZoneRef::Graveyard)".to_string());
        } else {
            clauses.push(
                "Filter::Or(&[Filter::InZone(ZoneRef::Graveyard), Filter::InZone(ZoneRef::Exile)])"
                    .to_string(),
            );
        }
        if is_rem_defined {
            probe.take("Defined");
            if probe.has("Valid") {
                return None;
            }
        } else {
            let valid_raw = probe.take("Valid")?;
            let stripped = Self::strip_is_remembered_all_alts(&valid_raw)?;
            let cleaned = if scope.you_own_only {
                Self::strip_filter_attr(&stripped, "YouOwn").unwrap_or(stripped)
            } else {
                if stripped.contains("YouOwn") || stripped.contains("OppOwn") {
                    return None;
                }
                stripped
            };
            if cleaned != "Card" {
                let expr = self.filter_expr_in_zone(&cleaned, true)?;
                if expr != "Filter::Any" {
                    clauses.push(expr);
                }
            }
        }
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
        {
            return None;
        }
        if let Some(sa) = sa_filter
            && sa != "Filter::Any"
            && !clauses.contains(&sa)
        {
            clauses.push(sa);
        }
        let filter_expr = match clauses.len() {
            0 => "Filter::Any".to_string(),
            1 => clauses.remove(0),
            _ => format!("Filter::And(&[{}])", clauses.join(", ")),
        };
        let filter_name = self.body.filter_static("PLAY_REM", &filter_expr);
        *p = probe;
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: &{filter_name}, min: 0, max: Amount::Fixed(1), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: &Filter::Any, controller: PlayerRel::You, count: PlayCount::One, \
             cost: {cost}, cast_only: {cast_only}, copy: {copy}, \
             exile_instead_of_graveyard: {exile_instead_of_graveyard}, optional: false }}], \
             for_unchosen: &[] }}"
        )])
    }

    /// Additional `ChangeZone` shapes inside `Effect::CaptureAffected::then`:
    /// 1. `ChangeZone | Hidden$ True | Origin$ Graveyard[,Exile] | ChangeType$ EACH <A>.IsRemembered & <B>.IsRemembered`
    ///    (*Eivor, Wolf-Kissed*, *Sludge Titan*).
    /// 2. `ChangeZone | Origin$ Library [| OriginAlternative$ Hand|Graveyard]`
    ///    (*Vivien on the Hunt* `[+2]`, *The Hunger Tide Rises* Ch IV, *Last Light of Durin's Day*).
    /// 3. `ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ EACH <A> & <B> | Optional$ True`
    ///    (*Yuna's Decision*).
    /// 4. `ChangeZone | Defined$ Remembered | Origin$ Graveyard[,Exile] | Destination$ Battlefield`
    ///    with `WithCountersType$` + `WithCountersAmount$` (*Heart-Shaped Herb*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29c_capture_change_zone_step(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        if api != "ChangeZone" || flags.targets_here {
            return None;
        }

        // Case A: `Hidden$ True | ChangeType$ EACH <A>.IsRemembered & <B>.IsRemembered`
        // (*Eivor, Wolf-Kissed*, *Sludge Titan*).
        if p.peek("Hidden") == Some("True")
            && !p.has("Defined")
            && let Some(ct_raw) = p.peek("ChangeType")
            && ct_raw.starts_with("EACH ")
            && ct_raw.contains("IsRemembered")
        {
            let mut probe = p.clone();
            probe.take("Hidden");
            probe.take("SelectPrompt");
            probe.take("ChangeTypeDesc");
            let origin = probe.take("Origin")?;
            let in_zone_guard = match origin.as_str() {
                "Graveyard" => "Filter::InZone(ZoneRef::Graveyard)",
                "Exile" => "Filter::InZone(ZoneRef::Exile)",
                "Graveyard,Exile" | "Exile,Graveyard" => {
                    "Filter::Or(&[Filter::InZone(ZoneRef::Graveyard), Filter::InZone(ZoneRef::Exile)])"
                }
                _ => return None,
            };
            let dest = probe.take("Destination")?;
            let movement = match dest.as_str() {
                "Hand" => "ChooseMove::Hand".to_string(),
                "Battlefield" => {
                    let tapped = match probe.take("Tapped").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    let gain = match probe.take("GainControl").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    if !gain && !scope.you_own_only {
                        return None;
                    }
                    format!("ChooseMove::Battlefield {{ tapped: {tapped} }}")
                }
                _ => return None,
            };
            if probe.take("Optional").as_deref() != Some("True") {
                return None;
            }
            if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
                return None;
            }
            let ct = probe.take("ChangeType")?;
            let parts: Vec<&str> = ct
                .strip_prefix("EACH ")?
                .split('&')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() < 2 || parts.len() > 4 {
                return None;
            }
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
            {
                return None;
            }
            let mut out = Vec::with_capacity(parts.len());
            for (idx, part) in parts.iter().enumerate() {
                let stripped = Self::strip_is_remembered_all_alts(part)?;
                let cleaned = if scope.you_own_only {
                    Self::strip_filter_attr(&stripped, "YouOwn").unwrap_or(stripped)
                } else {
                    stripped
                };
                let base_expr = self.filter_expr_in_zone(&cleaned, true)?;
                let full_expr = if idx == 0 {
                    base_expr
                } else {
                    // Guard against re-picking a dual-typed card (e.g. Urza's Saga /
                    // Dryad Arbor) that already moved to Hand/Battlefield in an earlier pick.
                    format!("Filter::And(&[{base_expr}, {in_zone_guard}])")
                };
                let fname = self.body.filter_static("CHOOSE_EACH", &full_expr);
                out.push(format!(
                    "Effect::ChooseCard {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
                     filter: &{fname}, min: 0, max: Amount::Fixed(1), movement: {movement}, \
                     then: &[] }}"
                ));
            }
            *p = probe;
            return Some(out);
        }

        // Case B: `Origin$ Library` (including `OriginAlternative$ Graveyard` via
        // `effect_of_inner`, and `OriginAlternative$ Hand` for *Last Light of Durin's Day*).
        if p.peek("Origin") == Some("Library") && !p.has("Defined") {
            let mut probe = p.clone();
            let mut cond_entries = Vec::new();
            probe.entries.retain(|(k, v)| {
                if k.starts_with("Condition") || k.starts_with("Present") {
                    cond_entries.push((k.clone(), v.clone()));
                    false
                } else {
                    true
                }
            });
            if probe.peek("OriginAlternative") == Some("Hand") {
                if let Some(effs) = self.wave29c_search_hand_and_or_library(&mut probe)
                    && probe.exhausted()
                {
                    probe.entries = cond_entries;
                    *p = probe;
                    return Some(effs);
                }
                return None;
            }
            if let Some(effs) = self.effect_of_inner("ChangeZone", &mut probe, target, flags)
                && probe.exhausted()
            {
                probe.entries = cond_entries;
                *p = probe;
                return Some(effs);
            }
        }

        // Case C: `Origin$ Hand | Destination$ Battlefield | ChangeType$ EACH <A> & <B> | Optional$ True`
        // (*Yuna's Decision*).
        if p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Battlefield")
            && p.peek("Optional") == Some("True")
            && !p.has("Defined")
            && p.peek("ChangeType")
                .is_some_and(|ct| ct.starts_with("EACH ") && !ct.contains("IsRemembered"))
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("Optional");
            probe.take("ChangeTypeDesc");
            if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
                return None;
            }
            let tapped = match probe.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let ct = probe.take("ChangeType")?;
            let parts: Vec<&str> = ct
                .strip_prefix("EACH ")?
                .split('&')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() < 2 || parts.len() > 4 {
                return None;
            }
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
            {
                return None;
            }
            let mut out = Vec::with_capacity(parts.len());
            for part in parts {
                let expr = self.filter_expr_in_zone(part, true)?;
                let fname = self.body.filter_static("HAND_EACH", &expr);
                out.push(format!(
                    "Effect::ChooseCard {{ chooser: PlayerRel::You, zone: ChooseZone::Hand(PlayerRel::You), \
                     filter: &{fname}, min: 0, max: Amount::Fixed(1), \
                     movement: ChooseMove::Battlefield {{ tapped: {tapped} }}, then: &[] }}"
                ));
            }
            *p = probe;
            return Some(out);
        }

        // Case D: `Defined$ Remembered | Origin$ Graveyard[,Exile] | Destination$ Battlefield`
        // with optional `WithCountersType$` + `WithCountersAmount$` (*Heart-Shaped Herb*).
        if p.peek("Defined") == Some("Remembered")
            && matches!(
                p.peek("Origin"),
                Some("Graveyard" | "Exile" | "Graveyard,Exile" | "Exile,Graveyard")
            )
            && p.peek("Destination") == Some("Battlefield")
        {
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("IsCurse");
            let origin = probe.take("Origin")?;
            probe.take("Destination");
            let tapped = match probe.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let gain = match probe.take("GainControl").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let counters = match (
                probe.take("WithCountersType"),
                probe.take("WithCountersAmount"),
            ) {
                (None, None) => "None".to_string(),
                (Some(raw_kind), raw_amt) => {
                    let kind = counter_kind(raw_kind.trim())
                        .or_else(|| counter_kind(&raw_kind.trim().to_ascii_uppercase()))?;
                    let n: u16 = raw_amt
                        .as_deref()
                        .unwrap_or("1")
                        .trim()
                        .parse()
                        .ok()
                        .filter(|&n| n >= 1)?;
                    format!("Some(({kind}, Amount::Fixed({n})))")
                }
                _ => return None,
            };
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
            {
                return None;
            }
            let owner_control = !gain;
            let expected_origin = match origin.as_str() {
                "Graveyard" => "Some(ZoneRef::Graveyard)",
                "Exile" => "Some(ZoneRef::Exile)",
                _ => "None",
            };
            *p = probe;
            return Some(vec![format!(
                "Effect::ForEachRemembered {{ filter: &Filter::Any, \
                 effects: &[Effect::MoveAndModify {{ \
                 subject: MoveSubject::Target(TargetSpec::EventObject), \
                 expected_origin: {expected_origin}, blink: false, owner_control: {owner_control}, \
                 tapped: {tapped}, face: None, counters: {counters}, modifiers: &[], \
                 duration: Duration::Indefinitely, attach_source: false, \
                 when_source_leaves: None }}] }}"
            )]);
        }

        None
    }

    /// Lower `Origin$ Library | OriginAlternative$ Hand | Destination$ Battlefield`
    /// (*Last Light of Durin's Day*): "search your hand and/or library for a `<F>`
    /// card and put it onto the battlefield. If you search your library this way, shuffle."
    fn wave29c_search_hand_and_or_library(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Origin").as_deref() != Some("Library")
            || p.take("OriginAlternative").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        p.take("Optional");
        p.take("ChangeTypeDesc");
        if !matches!(p.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let ct = p.take("ChangeType")?;
        let cleaned = Self::strip_filter_attr(&ct, "YouOwn").unwrap_or(ct);
        let expr = self.filter_expr_in_zone(&cleaned, true)?;
        let fname = self.body.filter_static("SEARCH_HL", &expr);
        let find = if tapped {
            "Find::BATTLEFIELD_TAPPED"
        } else {
            "Find::BATTLEFIELD"
        };
        // Offer optional choice from Hand first (capturing whether 1 was put onto
        // the battlefield); if not chosen from Hand, offer optional library search
        // (which shuffles when searched).
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::ChooseCard {{ chooser: PlayerRel::You, \
             zone: ChooseZone::Hand(PlayerRel::You), filter: &{fname}, min: 0, \
             max: Amount::Fixed(1), movement: ChooseMove::Battlefield {{ tapped: {tapped} }}, \
             then: &[] }}], \
             then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
             left: &Amount::RememberedCount, op: AmountCmp::Equal, right: &Amount::Fixed(0) }}, \
             then: &[Effect::MayDo {{ effects: &[Effect::SearchLibrary {{ filter: &{fname}, \
             finds: &[{find}], optional: true }}] }}], otherwise: &[] }}] }}"
        )])
    }

    // =========================================================================
    // 7. Pass 2 lowerings (DigUntil, Dig.RememberChanged, RememberSacrificed,
    //    RememberTokens, RememberMilled)
    // =========================================================================

    /// True when `body` is a trigger-executed `AB$ DigUntil | Cost$ Sac<1/...>`
    /// whose `SVar`s read `Sacrificed$CardManaCost` (*Kethek, Crucible Goliath*).
    pub(super) fn wave29c_is_sac_dependent_ab_dig_until(&self, body: &str) -> bool {
        body.trim_start().starts_with("AB$ DigUntil |")
            && body.contains("Cost$ Sac<1/")
            && self
                .svars
                .values()
                .any(|v| v.trim() == "Sacrificed$CardManaCost")
    }

    /// Lower `AB$ DigUntil | Cost$ Sac<1/<valid>> | ...` referencing
    /// `Sacrificed$CardManaCost` into `Effect::CaptureAffected` (*Kethek, Crucible Goliath*).
    pub(super) fn wave29c_trigger_sac_dig_until(
        &mut self,
        body: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        if !body.trim_start().starts_with("AB$") {
            return None;
        }
        let (api, mut p) = Params::parse(body)?;
        if api != "DigUntil" {
            return None;
        }
        let cost_raw = p.take("Cost")?;
        let inner_sac = cost_raw.trim().strip_prefix("Sac<1/")?.strip_suffix('>')?;
        let raw_valid = inner_sac
            .split_once('/')
            .map_or(inner_sac, |(v, _)| v)
            .trim();
        if raw_valid.is_empty() || raw_valid == "CARDNAME" || raw_valid == "NICKNAME" {
            return None;
        }
        let norm_valid = raw_valid.replace("StrictlyOther", "Other");
        let sac_expr = self.filter_expr(&norm_valid)?;
        let sac_name = self.body.filter_static("SAC", &sac_expr);

        let db_spec = Self::wave29c_reconstruct_spec("DigUntil", &p);
        let scope = super::wave27c::CaptureScope {
            single_object: true,
            guaranteed_valid: None,
            from_battlefield: true,
            you_own_only: false,
            multi_player: false,
            captured_zone: None,
            dest_hand_you: false,
        };
        let saved_scope = self.capture_scope.replace(scope);
        let mut inner_chain = Chain::default();
        let ok = self.chain(&db_spec, &mut inner_chain);
        self.capture_scope = saved_scope;
        ok?;
        if inner_chain.effects.is_empty()
            || inner_chain.target.is_some()
            || inner_chain.second_target.is_some()
        {
            return None;
        }
        chain.effects.push(format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::MayDo {{ effects: &[Effect::SacrificeFilter {{ \
             who: PlayerRel::You, filter: &{sac_name} }}] }}], \
             then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
             left: &Amount::RememberedCount, op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
             then: &[{}], otherwise: &[] }}] }}",
            inner_chain.effects.join(", ")
        ));
        Some(())
    }

    /// Follow-ups on `DigUntil` putting the found card onto the battlefield or into
    /// hand (*Illuna, Apex of Wishes*, *Part in Friendship*).
    #[allow(
        clippy::too_many_arguments,
        clippy::fn_params_excessive_bools,
        clippy::type_complexity
    )]
    pub(super) fn wave29c_dig_until_bf_or_hand_followup(
        &mut self,
        sub_name: &str,
        found: &str,
        rest: &str,
        exile_while_revealing: bool,
        no_move_revealed: bool,
        remember_found: bool,
        imprint_revealed: bool,
    ) -> Option<(Option<String>, Vec<String>, Option<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();

        // Shape A (*Illuna, Apex of Wishes*): `Exile` -> `ChangeZone | Defined$ Remembered |
        // Origin$ Exile | Destination$ Battlefield | DestinationAlternative$ Hand`.
        if exile_while_revealing
            && remember_found
            && !imprint_revealed
            && found == "RevealUntilFound::Exile"
            && api == "ChangeZone"
            && sp.take("Defined").as_deref() == Some("Remembered")
            && sp.take("Origin").as_deref() == Some("Exile")
            && sp.take("Destination").as_deref() == Some("Battlefield")
            && sp.take("DestinationAlternative").as_deref() == Some("Hand")
        {
            let tapped = match sp.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave29c_consume_remember_cleanup(after)?;
            let eff = format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Found, \
                 filter: &Filter::Any, min: 0, max: Amount::Fixed(1), reveal_chosen: false, \
                 movement: ChooseMove::Battlefield {{ tapped: {tapped} }}, \
                 not_chosen_movement: ChooseMove::Hand, then: &[], for_unchosen: &[] }}"
            );
            return Some((None, vec![eff], None, trailing));
        }

        // Shape B (*Part in Friendship*): `NoMoveRevealed$ True | ImprintRevealed$ True |
        // RememberFound$ True` -> `Branch(Remembered$Valid <V>, ToBattlefield, ToHand)`
        // -> `ChangeZoneAll(Card.IsImprinted, Library -> BottomRandom)`.
        if !exile_while_revealing
            && no_move_revealed
            && remember_found
            && imprint_revealed
            && matches!(
                found,
                "RevealUntilFound::StayInLibrary" | "RevealUntilFound::TopOfLibrary"
            )
            && rest == "RevealUntilRest::StayInLibrary"
            && api == "Branch"
        {
            return self.wave29c_dig_until_branch_bf_or_hand(&mut sp);
        }
        None
    }

    #[allow(clippy::type_complexity)]
    fn wave29c_dig_until_branch_bf_or_hand(
        &mut self,
        sp: &mut Params,
    ) -> Option<(Option<String>, Vec<String>, Option<String>, Option<String>)> {
        if !matches!(
            sp.take("BranchConditionSVarCompare").as_deref(),
            None | Some("GE1" | "EQ1" | "GT0")
        ) {
            return None;
        }
        let cond_var = sp.take("BranchConditionSVar")?;
        let true_sub = sp.take("TrueSubAbility")?;
        let false_sub = sp.take("FalseSubAbility")?;
        let rest_sub = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        let cond_def = self.svars.get(cond_var.trim())?.trim();
        let valid_cond = cond_def.strip_prefix("Remembered$Valid ")?.trim();
        let cond_expr = self.filter_expr_in_zone(valid_cond, true)?;
        let cond_fname = self.body.filter_static("DIG_BRANCH", &cond_expr);

        let (_, mut tp) = Params::parse(self.svars.get(true_sub.trim())?)?;
        tp.drop_prose();
        if tp.take("Defined").as_deref() != Some("Remembered")
            || tp.take("Origin").as_deref() != Some("Library")
            || tp.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let tapped = match tp.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !tp.exhausted() {
            return None;
        }

        let (_, mut fp) = Params::parse(self.svars.get(false_sub.trim())?)?;
        fp.drop_prose();
        if fp.take("Defined").as_deref() != Some("Remembered")
            || fp.take("Origin").as_deref() != Some("Library")
            || fp.take("Destination").as_deref() != Some("Hand")
            || !fp.exhausted()
        {
            return None;
        }

        let (r_api, mut rp) = Params::parse(self.svars.get(rest_sub.trim())?)?;
        rp.drop_prose();
        if r_api != "ChangeZoneAll"
            || !matches!(
                rp.take("ChangeType").as_deref(),
                Some("Card.IsImprinted" | "Card.IsImprinted+!IsRemembered")
            )
            || rp.take("Origin").as_deref() != Some("Library")
            || rp.take("Destination").as_deref() != Some("Library")
            || rp.take("LibraryPosition").as_deref() != Some("-1")
            || rp.take("RandomOrder").as_deref() != Some("True")
        {
            return None;
        }
        let after_rest = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        let trailing = self.wave29c_consume_remember_or_imprint_cleanup(after_rest)?;

        let then_effs = vec![
            format!(
                "Effect::IfEventObjectMatches {{ filter: &{cond_fname}, \
                 then: &[Effect::MoveAndModify {{ \
                 subject: MoveSubject::Target(TargetSpec::EventObject), \
                 expected_origin: Some(ZoneRef::Library), blink: false, \
                 owner_control: true, tapped: {tapped}, face: None, counters: None, \
                 modifiers: &[], duration: Duration::Indefinitely, attach_source: false, \
                 when_source_leaves: None }}] }}"
            ),
            "Effect::ChooseCardEx { chooser: PlayerRel::You, zone: ChooseZone::Found, \
             filter: &Filter::InZone(ZoneRef::Library), min: 0, max: Amount::Fixed(0), \
             reveal_chosen: false, movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::Hand, then: &[], for_unchosen: &[] }"
                .to_string(),
        ];
        Some((
            Some("RevealUntilFound::StayInLibrary".to_string()),
            then_effs,
            Some("RevealUntilRest::BottomRandom".to_string()),
            trailing,
        ))
    }

    /// Lower `Triggers$` on `Animate | Defined$ Remembered` inside
    /// `CaptureAffected::then` (*Jaxis, the Troublemaker*).
    pub(super) fn wave29c_animate_remembered_triggers(
        &mut self,
        probe: &mut Params,
        duration: &str,
    ) -> Option<Vec<String>> {
        let Some(raw) = probe.take("Triggers") else {
            return Some(Vec::new());
        };
        let mut out = Vec::new();
        for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let trig_mod = self.volver_granted_trigger(name)?;
            out.push(format!(
                "Effect::ModifyEvent {{ modifier: {trig_mod}, duration: {duration} }}"
            ));
        }
        (!out.is_empty()).then_some(out)
    }

    /// Lower `Dig | DestinationZone$ Battlefield | ChangeNum$ 1 | RememberChanged$ True`
    /// followed by `Animate | Defined$ Remembered` (*Aethermage's Touch*) or
    /// `ImmediateTrigger` reflexive power-damage (*Vivien's Invocation*) via
    /// `PeekAndReveal { ..., if_picked: &[...] }`.
    #[allow(clippy::too_many_lines)]
    fn wave29c_dig_bf_picked_followup(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        s_api: &str,
        sp_in: &Params,
        _target: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if chain.target.is_some() || chain.second_target.is_some() {
            return None;
        }
        let mut probe = p.clone();
        if !matches!(probe.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let count: u8 = probe
            .take("DigNum")?
            .trim()
            .parse()
            .ok()
            .filter(|&n| n >= 1)?;
        if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        let optional = match probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let reveal_all = match probe.take("Reveal").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if probe.take("DestinationZone").as_deref() != Some("Battlefield") {
            return None;
        }
        let tapped = match probe.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let rest_random = match probe.take("RestRandomOrder").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !matches!(
            probe.take("DestinationZone2").as_deref(),
            None | Some("Library")
        ) || !matches!(
            probe
                .take("LibraryPosition2")
                .or_else(|| probe.take("LibraryPosition"))
                .as_deref(),
            None | Some("-1")
        ) {
            return None;
        }
        let valid = probe.take("ChangeValid")?;
        let filter_expr = self.filter_expr_in_zone(&valid, true)?;
        let filter_name = self.body.filter_static("DIG", &filter_expr);
        probe.take("RememberChanged");
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }

        let mut sp = sp_in.clone();
        let (picked_effs, trailing) = if s_api == "Animate" {
            if sp.take("Defined").as_deref() != Some("Remembered") {
                return None;
            }
            let duration = match sp.take("Duration").as_deref() {
                None | Some("EOT" | "UntilEndOfTurn") => "Duration::UntilEndOfTurn",
                Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
                Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
                Some("Permanent") => "Duration::Indefinitely",
                Some(_) => return None,
            };
            let after = sp.take("SubAbility");
            let mut mods = Vec::new();
            if let Some(raw_kw) = sp.take("Keywords") {
                let each: Option<Vec<&str>> = raw_kw
                    .split('&')
                    .map(|k| super::keyword_const(k.trim()))
                    .collect();
                let each = each?;
                let joined =
                    each.join(".union(") + &")".repeat(raw_kw.split('&').count().saturating_sub(1));
                mods.push(format!(
                    "Effect::ModifyEvent {{ modifier: Modifier::AddKeyword({joined}), duration: {duration} }}"
                ));
            }
            let trig_mods = self.wave29c_animate_remembered_triggers(&mut sp, duration)?;
            mods.extend(trig_mods);
            if mods.is_empty() || !sp.exhausted() {
                return None;
            }
            let tail = self.wave29c_consume_remember_cleanup(after)?;
            (mods, tail)
        } else {
            // `ImmediateTrigger` reflexive power-damage (*Vivien's Invocation*).
            if sp.take("ConditionDefined").as_deref() != Some("Remembered")
                || !matches!(
                    sp.take("ConditionPresent").as_deref(),
                    Some("Card" | "Creature")
                )
                || !matches!(
                    sp.take("ConditionCompare").as_deref(),
                    None | Some("GE1" | "EQ1")
                )
                || !matches!(sp.take("RememberObjects").as_deref(), Some("Remembered"))
            {
                return None;
            }
            let exec_name = sp.take("Execute")?;
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let tail = self.wave29c_consume_remember_cleanup(after)?;
            let exec_body = self.svars.get(exec_name.trim())?.clone();
            let (d_api, mut dp) = Params::parse(&exec_body)?;
            dp.drop_prose();
            if d_api != "DealDamage"
                || !matches!(
                    dp.take("DamageSource").as_deref(),
                    Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                )
            {
                return None;
            }
            let num_raw = dp.take("NumDmg")?;
            let num_def = self
                .svars
                .get(num_raw.trim())
                .map_or(num_raw.trim(), String::as_str)
                .trim();
            if !matches!(
                num_def,
                "TriggerRemembered$CardPower" | "DelayTriggerRemembered$CardPower"
            ) {
                return None;
            }
            dp.claim_label("ValidTgtsDesc", "ValidTgts");
            let vtgts = dp.take("ValidTgts")?;
            let mut inner_c = Chain::default();
            self.chain_target("DealDamage", &mut dp, &vtgts, &mut inner_c, None)?;
            if !dp.exhausted() || inner_c.second_target.is_some() {
                return None;
            }
            let tgt_spec = Self::effect_target(&inner_c)?;
            let req = Self::target_req_expr(&inner_c)?;
            let reflex = format!(
                "Effect::ReflexiveTrigger {{ when: ReflexiveWhen::Always, once: true, \
                 condition: None, targets: Some({req}), remember: TriggerRemember::EventObject, \
                 effects: &[Effect::EventObjectDealsDamageEqualToPower {{ target: {tgt_spec} }}] }}"
            );
            (vec![reflex], tail)
        };

        let min_pick = u8::from(!optional);
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::PeekAndReveal {{ library: PlayerRel::You, count: Amount::Fixed({count}), \
             reveal_all: {reveal_all}, reveal_chosen: false, filter: &{filter_name}, \
             min_pick: {min_pick}, max_pick: 1, picked: PeekPickDest::Battlefield {{ tapped: {tapped} }}, \
             rest: PeekRestDest::BottomOfLibrary {{ random: {rest_random} }}, \
             if_picked: &[{}] }}",
            picked_effs.join(", ")
        )])
    }

    /// Lower *Crabomination*: targeted opponent exiles top card of library, a card at
    /// random from graveyard, and a card at random from hand -> you may cast 1 spell
    /// from among cards exiled this way without paying its mana cost.
    fn wave29c_crabomination_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        let who = self.wave11_player_of_line(probe.take("Defined").as_deref(), target, true)?;
        if who != "PlayerRel::Chosen"
            || probe.take("DigNum").as_deref() != Some("1")
            || !matches!(probe.take("ChangeNum").as_deref(), None | Some("1" | "All"))
            || probe.take("DestinationZone").as_deref() != Some("Exile")
            || probe.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        probe.take("IsCurse");
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }

        let s1_name = sub.as_deref()?.trim().to_string();
        let (a1, mut p1) = Params::parse(self.svars.get(&s1_name)?)?;
        p1.drop_prose();
        let s2_name = p1.take("SubAbility")?;
        if !Self::wave29c_is_random_targeted_exile(&a1, &mut p1, "Graveyard") {
            return None;
        }

        let (a2, mut p2) = Params::parse(self.svars.get(s2_name.trim())?)?;
        p2.drop_prose();
        let s3_name = p2.take("SubAbility")?;
        if !Self::wave29c_is_random_targeted_exile(&a2, &mut p2, "Hand") {
            return None;
        }

        let (a3, mut p3) = Params::parse(self.svars.get(s3_name.trim())?)?;
        p3.drop_prose();
        if a3 != "Play"
            || p3.take("Valid").as_deref() != Some("Card.IsRemembered")
            || p3.take("ValidSA").as_deref() != Some("Spell")
            || p3.take("ValidZone").as_deref() != Some("Exile")
            || p3.take("Amount").as_deref().unwrap_or("1") != "1"
            || p3.take("Optional").as_deref() != Some("True")
            || p3.take("WithoutManaCost").as_deref() != Some("True")
            || !matches!(p3.take("Controller").as_deref(), None | Some("You"))
        {
            return None;
        }
        let after_play = p3.take("SubAbility");
        if !p3.exhausted() {
            return None;
        }
        let trailing = self.wave29c_consume_remember_cleanup(after_play)?;

        let in_exile = self
            .body
            .filter_static("EXILE_REM", "Filter::InZone(ZoneRef::Exile)");
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::ExileTop {{ who: PlayerRel::Chosen, count: Amount::Fixed(1) }}, \
             Effect::ChooseCardConstrained {{ chooser: PlayerRel::You, \
             zone: ChooseZone::Graveyard(PlayerRel::Chosen), filter: &Filter::Any, min: 0, \
             max: Amount::Fixed(1), at_random: true, distinct_names: false, reveal_chosen: false, \
             movement: ChooseMove::Exile, then: &[] }}, \
             Effect::ChooseCardConstrained {{ chooser: PlayerRel::You, \
             zone: ChooseZone::Hand(PlayerRel::Chosen), filter: &Filter::Any, min: 0, \
             max: Amount::Fixed(1), at_random: true, distinct_names: false, reveal_chosen: false, \
             movement: ChooseMove::Exile, then: &[] }}], \
             then: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: &{in_exile}, min: 0, max: Amount::Fixed(1), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: &Filter::Any, controller: PlayerRel::You, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
             exile_instead_of_graveyard: false, optional: false }}], for_unchosen: &[] }}] }}"
        )])
    }

    fn wave29c_is_random_targeted_exile(api: &str, p: &mut Params, expected_origin: &str) -> bool {
        if api != "ChangeZone"
            || p.take("Origin").as_deref() != Some(expected_origin)
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("DefinedPlayer").as_deref() != Some("Targeted")
            || p.take("AtRandom").as_deref() != Some("True")
            || p.take("RememberChanged").as_deref() != Some("True")
            || p.take("ChangeNum").as_deref().unwrap_or("1") != "1"
            || !matches!(p.take("ChangeType").as_deref(), None | Some("Card"))
        {
            return false;
        }
        p.take("Mandatory");
        p.take("Hidden");
        p.take("IsCurse");
        p.exhausted()
    }

    /// Lower mandatory multi-seat sacrifice with `"each [opponent/player] who can't ..."`
    /// fallback (*Momentum Breaker*, *Plaguecrafter*) via `RepeatEach` + per-seat
    /// `CaptureAffected`.
    pub(super) fn wave29c_each_player_sac_or_cant(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if p.has("Optional")
            || p.peek("RememberSacrificed") != Some("True")
            || !matches!(p.peek("Amount"), None | Some("1"))
            || self.oracle.is_empty()
            || !self.oracle.to_ascii_lowercase().contains("who can't")
        {
            return None;
        }
        let (domain, expected_def) = match p.peek("Defined") {
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
            return None;
        }
        let sac_expr = self.filter_expr(&sac_valid)?;
        let sac_name = self.body.filter_static("SAC", &sac_expr);

        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if sp.take("Defined").as_deref() != Some(expected_def) {
            return None;
        }
        let after_sub = sp.take("SubAbility");
        let fallback_eff = match s_api.as_str() {
            "Discard" => {
                if sp.take("Mode").as_deref() != Some("TgtChoose") {
                    return None;
                }
                let amt =
                    self.amount_or_count_ext(sp.take("NumCards").as_deref().unwrap_or("1"))?;
                if !sp.exhausted() {
                    return None;
                }
                format!(
                    "Effect::DiscardAmountForPlayers {{ who: PlayerRel::EventPlayer, amount: {amt} }}"
                )
            }
            "LoseLife" => {
                let amt = self.amount_or_count_ext(&sp.take("LifeAmount")?)?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
            }
            _ => return None,
        };
        let trailing = self.wave29c_consume_remember_cleanup(after_sub)?;

        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::RepeatEach {{ domain: {domain}, \
             effects: &[Effect::CaptureAffected {{ \
             first: &[Effect::SacrificeFilter {{ who: PlayerRel::EventPlayer, filter: &{sac_name} }}], \
             then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
             left: &Amount::RememberedCount, op: AmountCmp::Equal, right: &Amount::Fixed(0) }}, \
             then: &[{fallback_eff}], otherwise: &[] }}] }}] }}"
        )])
    }

    /// Lower `ConditionCheckSVar$` inside `CaptureAffected::then` when `raw` is
    /// `Count$Valid <bf_valid>/Plus.<var>` and `<var>` is `Remembered$Valid <hand_valid>`
    /// (*Cache Grab*).
    pub(super) fn wave29c_wrap_remembered_plus_svar_condition(
        &mut self,
        p: &mut Params,
        raw: &str,
        cmp: &str,
        probe: &Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?;
        if !scope.dest_hand_you || cmp.len() < 3 {
            return None;
        }
        let (lhs_raw, plus_var) = raw.split_once("/Plus.")?;
        if !lhs_raw.starts_with("Count$Valid ") {
            return None;
        }
        let rem_def = self.svars.get(plus_var.trim())?.trim();
        let rem_valid = rem_def.strip_prefix("Remembered$Valid ")?.trim();
        let left_amt = self.amount_or_count_ext(lhs_raw)?;
        let hand_expr = self.filter_expr_in_zone(rem_valid, true)?;
        let rem_hand_expr = if hand_expr == "Filter::Any" {
            "Filter::IsRemembered".to_string()
        } else {
            format!("Filter::And(&[{hand_expr}, Filter::IsRemembered])")
        };
        let fname = self.body.filter_static("REM_HAND", &rem_hand_expr);
        let (op_code, rhs_raw) = cmp.split_at(2);
        let rhs: u32 = rhs_raw.parse().ok()?;
        let op = match op_code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        *p = probe.clone();
        Some(vec![format!(
            "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
             left: &Amount::Sum {{ left: &{left_amt}, \
             right: &Amount::CountOf {{ filter: &{fname}, zone: ZoneSel::HandOf(PlayerRel::You) }} }}, \
             op: {op}, right: &Amount::Fixed({rhs}) }}, \
             then: &[{}], otherwise: &[] }}",
            effects.join(", ")
        )])
    }

    /// Lower `Token | ... | RememberTokens$ True -> PumpAll | ValidCards$ Creature.IsRemembered |
    /// KW$ <kw> -> Cleanup` inside `CaptureAffected::then` (*Warren Weirding*).
    pub(super) fn wave29c_capture_token_pump_all_tail(
        &mut self,
        token_body: &str,
        then_chain: &mut Chain,
    ) -> Option<(bool, Option<String>)> {
        let (t_api, mut tp) = Params::parse(token_body)?;
        tp.drop_prose();
        if t_api != "Token" || tp.take("RememberTokens").as_deref() != Some("True") {
            return None;
        }
        let pump_name = tp.take("SubAbility")?;
        let (p_api, mut pp) = Params::parse(self.svars.get(pump_name.trim())?)?;
        pp.drop_prose();
        if p_api != "PumpAll"
            || pp.take("ValidCards").as_deref() != Some("Creature.IsRemembered")
            || !matches!(
                pp.take("Duration").as_deref(),
                None | Some("EOT" | "UntilEndOfTurn")
            )
        {
            return None;
        }
        let kw_raw = pp.take("KW")?;
        let after_pump = pp.take("SubAbility");
        if !pp.exhausted() {
            return None;
        }
        let after_cleanup = self.wave29c_consume_remember_cleanup(after_pump)?;
        let each: Option<Vec<&str>> = kw_raw
            .split('&')
            .map(|k| super::keyword_const(k.trim()))
            .collect();
        let each = each?;
        let kw_joined =
            each.join(".union(") + &")".repeat(kw_raw.split('&').count().saturating_sub(1));

        let mut cond_params = Params::default();
        tp.entries.retain(|(k, v)| {
            if k.starts_with("Condition") || k.starts_with("Present") {
                cond_params.entries.push((k.clone(), v.clone()));
                false
            } else {
                true
            }
        });
        let tok_spec = Self::wave29c_reconstruct_spec("Token", &tp);
        let mut tok_chain = Chain {
            target: then_chain.target.clone(),
            ..Chain::default()
        };
        let saved_scope = self.capture_scope.take();
        let tok_ok = self.chain(&tok_spec, &mut tok_chain);
        self.capture_scope = saved_scope;
        tok_ok?;
        if tok_chain.effects.is_empty()
            || tok_chain.target != then_chain.target
            || tok_chain.second_target.is_some()
        {
            return None;
        }

        let inner_cap = format!(
            "Effect::CaptureAffected {{ first: &[{}], \
             then: &[Effect::ForEachRemembered {{ filter: &Filter::CREATURE, \
             effects: &[Effect::ModifyEvent {{ modifier: Modifier::AddKeyword({kw_joined}), \
             duration: Duration::UntilEndOfTurn }}] }}] }}",
            tok_chain.effects.join(", ")
        );
        let wrapped = if cond_params.entries.is_empty() {
            vec![inner_cap]
        } else {
            let out = self.wave27c_wrap_remembered_condition(&mut cond_params, &[inner_cap])?;
            if !cond_params.exhausted() {
                return None;
            }
            out
        };
        then_chain.effects.extend(wrapped);
        Some((true, after_cleanup))
    }

    /// Lower `ChangeZone | UnlessCost$ <mana> PayLife<N> | UnlessPayer$ You | UnlessSwitched$ True`
    /// inside `CaptureAffected::then` (*Ripples of Undeath*).
    #[allow(clippy::option_option)]
    pub(super) fn wave29c_capture_switched_pay_change_zone(
        &mut self,
        body: &str,
        then_chain: &mut Chain,
    ) -> Option<Option<String>> {
        let (api, mut sp) = Params::parse(body)?;
        sp.drop_prose();
        if api != "ChangeZone"
            || sp.take("UnlessSwitched").as_deref() != Some("True")
            || sp.take("UnlessPayer").as_deref() != Some("You")
            || sp.has("UnlessResolveSubs")
        {
            return None;
        }
        sp.take("UnlessAI");
        let cost_raw = sp.take("UnlessCost")?;
        let (mana, parts) = self.cost_pieces(cost_raw.trim())?;
        let [part] = parts.as_slice() else {
            return None;
        };
        let life: u32 = part
            .strip_prefix("PayLife(")?
            .strip_suffix(')')?
            .parse()
            .ok()
            .filter(|&n| n >= 1)?;
        if mana.is_empty() {
            return None;
        }
        let next_sub = sp.take("SubAbility");
        if !sp.has("Optional")
            && !sp.has("Mandatory")
            && !sp.has("AnyNumber")
            && self.oracle.to_ascii_lowercase().contains("if you do, put ")
        {
            sp.entries
                .push(("Mandatory".to_string(), "True".to_string()));
        }
        let inner_effs = self.wave27c_remembered_step(
            "ChangeZone",
            &mut sp,
            then_chain.target.as_deref(),
            LineFlags {
                targets_here: false,
                targets_a_player: false,
                single_target: true,
                prior_target_used: false,
            },
        )?;
        if !sp.exhausted() || inner_effs.is_empty() {
            return None;
        }
        then_chain.effects.push(format!(
            "Effect::PlayerMayPayManaAndLifeEachThenElse {{ player: PlayerRel::You, \
             mana: mana!(\"{mana}\"), life: {life}, times: Amount::Fixed(1), \
             paid: &[{}], unpaid: &[] }}",
            inner_effs.join(", ")
        ));
        Some(next_sub)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    #[allow(clippy::option_option)]
    fn wave29c_consume_remember_or_imprint_cleanup(
        &self,
        next: Option<String>,
    ) -> Option<Option<String>> {
        let Some(name) = next else {
            return Some(None);
        };
        if let Some(body) = self.svars.get(name.trim())
            && let Some((api, mut cp)) = Params::parse(body)
            && api == "Cleanup"
        {
            cp.drop_prose();
            let rem = cp.take("ClearRemembered");
            let imp = cp.take("ClearImprinted");
            if matches!(rem.as_deref(), None | Some("True"))
                && matches!(imp.as_deref(), None | Some("True"))
                && (rem.is_some() || imp.is_some())
            {
                let after = cp.take("SubAbility");
                if cp.exhausted() {
                    return Some(after);
                }
            }
            return None;
        }
        None
    }

    #[allow(clippy::option_option)]
    fn wave29c_consume_remember_cleanup(&self, next: Option<String>) -> Option<Option<String>> {
        let Some(name) = next else {
            return Some(None);
        };
        if let Some(body) = self.svars.get(name.trim())
            && let Some((api, mut cp)) = Params::parse(body)
            && api == "Cleanup"
        {
            cp.drop_prose();
            if cp.take("ClearRemembered").as_deref() == Some("True") {
                let after = cp.take("SubAbility");
                if cp.exhausted() {
                    return Some(after);
                }
            }
            return None;
        }
        None
    }

    fn wave29c_reconstruct_spec(api: &str, p: &Params) -> String {
        let mut parts = vec![format!("DB$ {api}")];
        for (k, v) in &p.entries {
            parts.push(format!("{k}$ {v}"));
        }
        parts.join(" | ")
    }
}
