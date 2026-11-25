//! Wave 28a pure-`scriptgen` rules:
//!
//! 1. **Target count ranges (`TargetMin$`..`TargetMax$`) & divided damage**:
//!    - Nonzero target ranges `1 <= min < max < 255` (and whole-pool `SVar` maxes)
//!      on object-targeting `multi_target_ok` chains (Slots 1, 2, and 3) ->
//!      `TargetReq { min, ..TargetReq::up_to(spec, max) }` / `TargetReq::exactly`.
//!    - Generalized `DealDamage | DividedAsYouChoose$` across spells (allowing
//!      `TargetMin$ >= 1` and general Slot-2 continuations), activated abilities,
//!      loyalty abilities, choose-one charm modes, and object-only triggered
//!      abilities (strictly refusing player-including targets on triggers because
//!      `ask_trigger_division` reads only `obj.targets`).
//! 2. **`ChangeZone` & `RememberChanged` gaps**:
//!    - Self-dies shuffle into library (`Defined$ TriggeredNewCardLKICopy`,
//!      `Destination$ Library | Shuffle$ True`) ->
//!      `Effect::ShuffleGraveyardMatchingIntoLibrary { who: OwnerOfSource, filter: &Filter::This }`.
//!    - Hand-to-library with `Shuffle$ True` (*See Beyond*, *Credit Voucher*) ->
//!      `Effect::ChooseCardEx { movement: TopOfLibrary }` + `Effect::ShuffleLibrary`.
//!    - Target opponent chooses & exiles a permanent they control
//!      (`Origin$ Battlefield | Destination$ Exile | Hidden$ True | DefinedPlayer$ Targeted | Chooser$ Targeted`).
//!    - Conditional library search destination (*Loyal Inventor*) ->
//!      `Effect::IfCondition` selecting `Find::HAND` vs `Find::TOP_OF_LIBRARY`.
//!    - `CaptureAffected` extensions: full `Animate | Defined$ Remembered`
//!      (types, colors, keywords, P/T, `LeaveBattlefield$ Exile`,
//!      `AtEOT$ YourExile`/`YourSacrifice`/`YourHand`), `Attach | Defined$ Remembered`
//!      (*Deathrender*), `CopyPermanent | Defined$ Remembered` (*God-Pharaoh's Gift*,
//!      *Hour of Eternity*, *Nexus of Becoming*, *Offspring's Revenge*),
//!      `ConditionCheckSVar$` over `Remembered$Amount` / `Count$RememberedSize` /
//!      `Remembered$Valid <F>` (*Mistbreath Elder*, *Oriq Loremage*), and leading
//!      no-op `Cleanup | ClearRemembered$ True` before a `CaptureAffected` step.

use super::animation::supertype_const;
use super::chains::LineFlags;
use super::{Chain, Params, Tx, amount, card_type_const, color_const, keyword_const};

impl Tx<'_> {
    // =========================================================================
    // Part 1: TargetMin..TargetMax range parsing & formatting
    // =========================================================================

    /// Format a `TargetReq` for a nonzero target range `(min, max)` with
    /// `1 <= min <= max` and `max > 1`.
    pub(super) fn wave28a_format_range_req(spec: &str, min: u8, max: u8) -> String {
        if min == max {
            format!("TargetReq::exactly({spec}, {min})")
        } else {
            format!("TargetReq {{ min: {min}, ..TargetReq::up_to({spec}, {max}) }}")
        }
    }

    /// True if `max_var` names an `SVar` counting the entire target pool for
    /// `(api, p, valid, spec)` (so `TargetMax$ <max_var>` is effectively `u8::MAX`).
    pub(super) fn wave28a_is_whole_pool_max_svar(
        &self,
        api: &str,
        p: &Params,
        valid: &str,
        spec: &str,
        max_var: &str,
    ) -> bool {
        let is_obj_spec = spec.starts_with("TargetSpec::Object(");
        let raw = max_var.trim();
        let def = if let Some(s) = self.svars.get(raw) {
            s.trim()
        } else if raw.starts_with("Count$Valid") {
            raw
        } else {
            return false;
        };
        def == format!("Count$Valid {valid}")
            || def
                .strip_prefix("Count$Valid ")
                .is_some_and(|cv| Self::same_valid_atoms(cv, valid))
            || (is_obj_spec && matches!(def, "Count$Valid Card" | "Count$Valid Permanent"))
            || (api == "ChangeZone"
                && p.peek("Origin") == Some("Graveyard")
                && def.strip_prefix("Count$ValidGraveyard ").is_some_and(|cv| {
                    cv == valid
                        || cv == "Card"
                        || (matches!(cv, "Card.YouOwn" | "Card.YouCtrl")
                            && (valid.contains("YouOwn") || valid.contains("YouCtrl")))
                        || Self::same_valid_atoms(
                            &cv.replace("YouOwn", "YouCtrl"),
                            &valid.replace("YouOwn", "YouCtrl"),
                        )
                }))
    }

    /// Claim a nonzero target range `TargetMin$ m | TargetMax$ n` (`1 <= m < n < 255`,
    /// or `n` is a whole-pool `SVar` -> `u8::MAX`) for Slot 1.
    ///
    /// Strictly refuses player-only specs (`AnyPlayer` / `AnyOpponent`) because
    /// `PlayerRel::Chosen` holds only one seat.
    pub(super) fn wave28a_take_first_target_range(
        &self,
        api: &str,
        p: &mut Params,
        valid: &str,
        spec: &str,
        multi_target_ok: bool,
    ) -> Option<(u8, u8)> {
        if !multi_target_ok || matches!(spec, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") {
            return None;
        }
        let min = p.peek("TargetMin")?.trim().parse::<u8>().ok()?;
        if min == 0 || min == u8::MAX {
            return None;
        }
        let raw_max = p.peek("TargetMax")?;
        let max = if let Ok(n) = raw_max.trim().parse::<u8>() {
            if n <= min || n == u8::MAX {
                return None;
            }
            n
        } else if self.wave28a_is_whole_pool_max_svar(api, p, valid, spec, raw_max) {
            u8::MAX
        } else {
            return None;
        };
        p.take("TargetMin");
        p.take("TargetMax");
        Some((min, max))
    }

    /// Claim a nonzero exact count (`m == n > 1`) or range (`1 <= m < n`) for
    /// Slot 2 or Slot 3. Strictly refuses player-only specs.
    pub(super) fn wave28a_take_extra_slot_range(
        &self,
        api: &str,
        p: &mut Params,
        valid: &str,
        spec: &str,
        allow_multi: bool,
    ) -> Option<(u8, u8)> {
        if !allow_multi || matches!(spec, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") {
            return None;
        }
        let min = p.peek("TargetMin")?.trim().parse::<u8>().ok()?;
        if min == 0 || min == u8::MAX {
            return None;
        }
        let raw_max = p.peek("TargetMax")?;
        let max = if let Ok(n) = raw_max.trim().parse::<u8>() {
            if n < min || n <= 1 || n == u8::MAX {
                return None;
            }
            n
        } else if self.wave28a_is_whole_pool_max_svar(api, p, valid, spec, raw_max) {
            u8::MAX
        } else {
            return None;
        };
        p.take("TargetMin");
        p.take("TargetMax");
        Some((min, max))
    }

    // =========================================================================
    // Part 2: Generalized `DealDamage | DividedAsYouChoose$`
    // =========================================================================

    /// Parse `DealDamage | DividedAsYouChoose$` for spells, activated abilities,
    /// loyalty abilities, choose-one charm modes, and object-only triggers.
    ///
    /// Returns `(effects, target_spec, target_req_expr, second_target_req_expr)`.
    ///
    /// When `trigger_only` is `true`, strictly requires `TargetSpec::Object(..)`
    /// because the engine's `ask_trigger_division` reads only `obj.targets` and
    /// ignores `obj.target_players`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28a_read_divided_damage(
        &mut self,
        mut p: Params,
        trigger_only: bool,
    ) -> Option<(Vec<String>, String, String, Option<String>)> {
        p.drop_prose();
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let target = self.target_spec(&valid, "DealDamage")?;
        if matches!(
            target.as_str(),
            "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"
        ) {
            return self.deny("a player-only divided target requirement".into());
        }
        if trigger_only && !target.starts_with("TargetSpec::Object(") {
            return self.deny("a triggered damage division targeting players".into());
        }
        let total = self.divided_total(&p.take("NumDmg")?)?;
        if self.divided_total(&p.take("DividedAsYouChoose")?)? != total {
            return self.deny("damage and announced division have different totals".into());
        }
        let fixed_total = total
            .strip_prefix("DivisionAmount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u32>().ok());
        if fixed_total == Some(0) {
            return self.deny("divided damage total of zero".into());
        }
        let min_raw = p.take("TargetMin")?;
        let Ok(min) = min_raw.trim().parse::<u8>() else {
            return self.deny(format!("divided damage with `TargetMin$ {min_raw}`"));
        };
        if min > 1 {
            return self.deny("a damage division requiring more than one minimum target".into());
        }
        if let Some(t) = fixed_total
            && u32::from(min) > t
        {
            return self.deny("divided damage minimum targets exceeds damage total".into());
        }
        let max_raw = p.take("TargetMax")?;
        let maximum = self.divided_maximum(&max_raw, &total, &valid)?;
        if let Ok(max_num) = maximum.parse::<u8>() {
            if max_num == 0 || max_num < min {
                return self.deny(format!("divided damage with `TargetMax$ {max_raw}`"));
            }
            if let Some(t) = fixed_total
                && u32::from(max_num) > t
            {
                return self.deny("divided damage maximum targets exceeds damage total".into());
            }
        }
        p.take("IsCurse");
        let next = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DealDamage.{key}`"));
        }
        let target_req = match (min, maximum.as_str()) {
            (0, max) => format!("TargetReq::up_to({target}, {max})"),
            (1, "1") => format!("TargetReq::one({target})"),
            (1, max) => format!("TargetReq {{ min: 1, ..TargetReq::up_to({target}, {max}) }}"),
            _ => unreachable!(),
        };
        let mut effects = vec![format!("Effect::DealDamageDivided {{ amount: {total} }}")];
        let mut second_req: Option<String> = None;
        if let Some(next) = next {
            let body = self.svars.get(&next)?.clone();
            let (api, mut gain) = Params::parse(&body)?;
            gain.drop_prose();
            if !trigger_only
                && api == "GainLife"
                && gain.has("ValidTgts")
                && !gain.has("SubAbility")
            {
                gain.claim_label("ValidTgtsDesc", "ValidTgts");
                if gain.take("ValidTgts").as_deref() != Some("Opponent")
                    || gain.take("Defined").is_some_and(|v| v != "Targeted")
                    || gain.take("TargetMin").is_some_and(|v| v != "1")
                    || gain.take("TargetMax").is_some_and(|v| v != "1")
                {
                    return self.deny("an unsupported continuation after divided damage".into());
                }
                let n = amount(&gain.take("LifeAmount")?, self.svars, self.has_x)?;
                if let Some(key) = gain.first_key() {
                    return self.deny(format!("unclaimed parameter `GainLife.{key}`"));
                }
                effects.push(format!(
                    "Effect::GainLifeFor {{ amount: {n}, who: PlayerRel::SecondTargetPlayerOrController }}"
                ));
                second_req = Some("TargetReq::one(TargetSpec::AnyOpponent)".to_string());
            } else {
                let (initial_up_to, initial_range) = match (min, maximum.as_str()) {
                    (0, "u8::MAX") => (Some(u8::MAX), None),
                    (0, num) => (num.parse::<u8>().ok(), None),
                    (1, "1") => (None, None),
                    (1, "u8::MAX") => (None, Some((1, u8::MAX))),
                    (1, num) => (None, num.parse::<u8>().ok().map(|n| (1, n))),
                    _ => unreachable!(),
                };
                let mut follow = Chain {
                    effects: effects.clone(),
                    target: Some(target.clone()),
                    target_up_to: initial_up_to,
                    target_range: initial_range,
                    ..Chain::default()
                };
                self.chain(&body, &mut follow)?;
                if follow.target.as_deref() != Some(target.as_str())
                    || follow.third_target.is_some()
                    || (trigger_only && follow.second_target.is_some())
                    || follow.target_is_x
                    || follow.target_up_to != initial_up_to
                    || follow.target_range != initial_range
                    || follow.target_exact.is_some()
                    || follow.could_add_mana
                    || follow.effects.iter().skip(1).any(|e| {
                        e.contains("DealDamageDivided") || e.contains("PutCountersDivided")
                    })
                {
                    return self.deny("an unsupported continuation after divided damage".into());
                }
                second_req = Self::second_target_req_expr(&follow);
                effects = follow.effects;
            }
        }
        Some((effects, target, target_req, second_req))
    }

    /// Lower an object-only triggered `DealDamage | DividedAsYouChoose$` chain
    /// (*Dragonlord Atarka*) directly into `Chain` so `triggers.rs` formats it
    /// cleanly via `target_req_expr`.
    pub(super) fn wave28a_divided_damage_trigger_chain(
        &mut self,
        body: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let (api, p) = Params::parse(body)?;
        if api != "DealDamage" || !p.has("DividedAsYouChoose") {
            return None;
        }
        let min_raw = p.peek("TargetMin")?.trim().to_string();
        let max_raw = p.peek("TargetMax")?.trim().to_string();
        let total = self.divided_total(p.peek("NumDmg")?)?;
        let valid = p.peek("ValidTgts")?.to_string();
        let maximum = self.divided_maximum(&max_raw, &total, &valid)?;
        let (effects, target_spec, _req, second_req) = self.wave28a_read_divided_damage(p, true)?;
        if second_req.is_some() {
            return None;
        }
        let min: u8 = min_raw.parse().ok()?;
        let max_u8: u8 = if maximum == "u8::MAX" {
            u8::MAX
        } else {
            maximum.parse().ok()?
        };
        chain.effects = effects;
        chain.target = Some(target_spec);
        match (min, max_u8) {
            (0, m) => chain.target_up_to = Some(m),
            (1, 1) => {}
            (1, m) => chain.target_range = Some((1, m)),
            _ => return None,
        }
        Some(())
    }

    // =========================================================================
    // Part 3: `ChangeZone` gaps (self-dies shuffle, hand->library shuffle,
    //         targeted opponent exile permanent, conditional library search)
    // =========================================================================

    /// Self-dies trigger shuffling this card from its owner's graveyard into its
    /// owner's library (*Alabaster Dragon*, *Angel of Fury*, *Guan Yu*, *Lembas*,
    /// *Cavalier of Gales*):
    /// `Defined$ TriggeredNewCardLKICopy | [Origin$ Graveyard] | Destination$ Library | Shuffle$ True`
    /// -> `Effect::ShuffleGraveyardMatchingIntoLibrary { who: PlayerRel::OwnerOfSource, filter: &Filter::This }`.
    pub(super) fn wave28a_self_dies_shuffle_into_library(
        &mut self,
        p: &mut Params,
        defined: &str,
        origin: Option<&str>,
        destination: &str,
    ) -> Option<Vec<String>> {
        if self.in_delayed
            || !self.trigger_is_self
            || self.trigger_mode.as_deref() != Some("ChangesZone")
            || destination != "Library"
            || p.peek("Shuffle") != Some("True")
            || !matches!(
                defined,
                "TriggeredNewCardLKICopy"
                    | "TriggeredNewCard"
                    | "TriggeredCardLKICopy"
                    | "TriggeredCard"
            )
        {
            return None;
        }
        let origin_ok = match origin {
            Some("Graveyard") => true,
            None => {
                let lower = self.oracle.to_ascii_lowercase();
                (lower.contains(" dies") || lower.contains("put into a graveyard"))
                    && lower.contains("shuffle")
            }
            Some(_) => false,
        };
        if !origin_ok {
            return None;
        }
        let mut probe = p.clone();
        if origin.is_none() {
            probe.take("Destination");
            probe.take("Defined");
        }
        probe.take("Shuffle");
        probe.take("IsCurse");
        if let Some(num) = probe.take("ChangeNum")
            && num != "1"
        {
            return None;
        }
        if let Some(mand) = probe.take("Mandatory")
            && mand != "True"
        {
            return None;
        }
        if let Some(pos) = probe.take("LibraryPosition")
            && !matches!(pos.as_str(), "0" | "-1")
        {
            return None;
        }
        if !Self::wave28a_only_condition_keys_left(&probe) {
            return None;
        }
        *p = probe;
        Some(vec![
            "Effect::ShuffleGraveyardMatchingIntoLibrary { who: PlayerRel::OwnerOfSource, filter: &Filter::This }"
                .to_string(),
        ])
    }

    /// `Origin$ Hand | Destination$ Library | Shuffle$ True` (*See Beyond*,
    /// *Credit Voucher*): moves chosen card(s) from your hand to the top of your
    /// library and then shuffles your library (CR 701.20: shuffles even if 0
    /// cards are moved).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28a_hand_to_library_shuffle(
        &mut self,
        p: &mut Params,
        target: &str,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Shuffle") != Some("True") {
            return None;
        }
        let targets_a_player = targets_here && target == "TargetSpec::Player(PlayerRel::Chosen)";
        let mut probe = p.clone();
        probe.take("Shuffle");
        if let Some(hidden) = probe.take("Hidden")
            && hidden != "True"
        {
            return None;
        }
        probe.take("SelectPrompt");
        probe.take("IsCurse");
        if let Some(pos) = probe.take("LibraryPosition")
            && !matches!(pos.as_str(), "0" | "-1")
        {
            return None;
        }
        let defined_player = probe.take("DefinedPlayer");
        let chooser_raw = probe.take("Chooser");
        let player = match defined_player.as_deref() {
            None if targets_a_player => "PlayerRel::Chosen",
            None | Some("You") => "PlayerRel::You",
            Some(who) => {
                let rel = self.wave11_player_of_line(Some(who), Some(target), targets_a_player)?;
                if !Self::is_single_player_rel(rel) {
                    return None;
                }
                rel
            }
        };
        if let Some(c) = chooser_raw.as_deref() {
            let c_rel = self.wave11_player_of_line(Some(c), Some(target), targets_a_player)?;
            if c_rel != player {
                return None;
            }
        }
        let num_raw = probe.take("ChangeNum").unwrap_or_else(|| "1".to_string());
        let ct_raw = probe.take("ChangeType");
        let cleaned = match ct_raw.as_deref() {
            None | Some("Card") => "Card".to_string(),
            Some(valid) => {
                if player == "PlayerRel::You" {
                    Self::strip_your_graveyard_owner(valid)
                        .or_else(|| Self::strip_hand_owner_ctrl(valid, "YouOwn", "YouCtrl"))
                        .unwrap_or_else(|| valid.to_string())
                } else {
                    valid.to_string()
                }
            }
        };
        let is_any_number_svar = self.svars.get(num_raw.trim()).is_some_and(|def| {
            def.trim()
                .strip_prefix("Count$ValidHand ")
                .is_some_and(|v| {
                    let v_clean = if player == "PlayerRel::You" {
                        Self::strip_your_graveyard_owner(v)
                            .or_else(|| Self::strip_hand_owner_ctrl(v, "YouOwn", "YouCtrl"))
                            .unwrap_or_else(|| v.to_string())
                    } else {
                        v.to_string()
                    };
                    v_clean == cleaned
                })
        });
        let oracle_lower = self.oracle.to_ascii_lowercase();
        let optional = match (
            probe.take("Optional").as_deref(),
            probe.take("Mandatory").as_deref(),
        ) {
            (Some("True"), None | Some("False")) => true,
            (Some("You"), None | Some("False")) if player == "PlayerRel::You" => true,
            (None | Some("False"), Some("True")) => false,
            (None, None) if is_any_number_svar && oracle_lower.contains("any number") => true,
            _ => return None,
        };
        let (min, max_expr) = if is_any_number_svar && optional {
            (0u8, "Amount::Fixed(255)".to_string())
        } else if optional {
            let max = self.amount_or_count_ext(&num_raw)?;
            if max == "Amount::Fixed(0)" {
                return None;
            }
            (0u8, max)
        } else {
            let n: u8 = self.plain_num(&num_raw)?.try_into().ok()?;
            if n == 0 {
                return None;
            }
            (n, format!("Amount::Fixed({n})"))
        };
        if !Self::wave28a_only_condition_keys_left(&probe) {
            return None;
        }
        let filter = if cleaned == "Card" {
            "Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&cleaned, true)?;
            self.body.filter_static("SHUFFLE_HAND", &expr)
        };
        *p = probe;
        Some(vec![
            format!(
                "Effect::ChooseCardEx {{ chooser: {player}, zone: ChooseZone::Hand({player}), \
                 filter: &{filter}, min: {min}, max: {max_expr}, reveal_chosen: false, \
                 movement: ChooseMove::TopOfLibrary, not_chosen_movement: ChooseMove::None, \
                 then: &[], for_unchosen: &[] }}"
            ),
            format!("Effect::ShuffleLibrary {{ who: {player} }}"),
        ])
    }

    /// Target opponent chooses & exiles a permanent they control (*Debt to the
    /// Kami*, *Doomfall* mode 1, *Early Winter* mode 2):
    /// `Origin$ Battlefield | Destination$ Exile | Hidden$ True | ValidTgts$ Opponent/Player | DefinedPlayer$ Targeted | Chooser$ Targeted | Mandatory$ True`.
    pub(super) fn wave28a_targeted_player_exile_bf(
        &mut self,
        p: &mut Params,
        target: &str,
    ) -> Option<Vec<String>> {
        if target != "TargetSpec::Player(PlayerRel::Chosen)" || self.has_linked_exile_reader() {
            return None;
        }
        let mut probe = p.clone();
        if probe.take("Hidden").as_deref() != Some("True") {
            return None;
        }
        probe.take("SelectPrompt");
        probe.take("IsCurse");
        probe.claim_label("ChangeTypeDesc", "ChangeType");
        if probe.take("Mandatory").as_deref() != Some("True") {
            return None;
        }
        if !matches!(
            probe.take("DefinedPlayer").as_deref(),
            Some("Targeted" | "TargetedPlayer")
        ) || !matches!(
            probe.take("Chooser").as_deref(),
            Some("Targeted" | "TargetedPlayer")
        ) {
            return None;
        }
        let n: u8 = self
            .plain_num(probe.take("ChangeNum").as_deref().unwrap_or("1"))?
            .try_into()
            .ok()?;
        if n == 0 {
            return None;
        }
        let change_type = probe.take("ChangeType")?;
        if change_type.contains("Ctrl") || change_type.contains("Own") {
            return None;
        }
        if !Self::wave28a_only_condition_keys_left(&probe) {
            return None;
        }
        let base = self.filter_expr(&change_type)?;
        let scoped = if base == "Filter::Any" {
            "Filter::ControlledBy(PlayerRel::Chosen)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::ControlledBy(PlayerRel::Chosen)])")
        };
        let name = self.body.filter_static("CHOOSE_BF", &scoped);
        *p = probe;
        Some(vec![format!(
            "Effect::ChooseCard {{ chooser: PlayerRel::Chosen, zone: ChooseZone::Battlefield, \
             filter: &{name}, min: {n}, max: Amount::Fixed({n}), \
             movement: ChooseMove::Exile, then: &[] }}"
        )])
    }

    /// Conditional library search destination (*Loyal Inventor*):
    /// `ChangeZone | Origin$ Library | Destination$ Library | RememberChanged$ True | SubAbility$ S1`
    /// where `S1` moves `Defined$ Remembered` to `Hand` under `ConditionCheckSVar$ X | GE1`
    /// and `S2` moves `Defined$ Remembered` to `Library | LibraryPosition$ 0` under `X | LE0/EQ0`,
    /// followed by `Cleanup | ClearRemembered$ True`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28a_conditional_search_dest_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || target.is_some()
            || p.peek("Origin") != Some("Library")
            || p.peek("Destination") != Some("Library")
            || p.peek("RememberChanged") != Some("True")
        {
            return None;
        }
        let s1_name = sub.as_deref()?.trim().to_string();
        let s1_body = self.svars.get(&s1_name)?.clone();
        let (s1_api, mut s1) = Params::parse(&s1_body)?;
        s1.drop_prose();
        if s1_api != "ChangeZone"
            || s1.take("Defined").as_deref() != Some("Remembered")
            || s1.take("Origin").as_deref() != Some("Library")
            || s1.take("Destination").as_deref() != Some("Hand")
        {
            return None;
        }
        let var1 = s1.take("ConditionCheckSVar")?;
        let cmp1 = s1
            .take("ConditionSVarCompare")
            .unwrap_or_else(|| "GE1".to_string());
        s1.take("ConditionDescription");
        let s2_name = s1.take("SubAbility")?;
        if !s1.exhausted() {
            return None;
        }
        // Condition must be evaluated independently of the search result.
        if self
            .svars
            .get(var1.trim())
            .is_some_and(|d| d.contains("Remembered"))
        {
            return None;
        }
        let s2_body = self.svars.get(s2_name.trim())?.clone();
        let (s2_api, mut s2) = Params::parse(&s2_body)?;
        s2.drop_prose();
        if s2_api != "ChangeZone"
            || s2.take("Defined").as_deref() != Some("Remembered")
            || s2.take("Origin").as_deref() != Some("Library")
            || s2.take("Destination").as_deref() != Some("Library")
            || !matches!(s2.take("LibraryPosition").as_deref(), None | Some("0"))
        {
            return None;
        }
        let var2 = s2.take("ConditionCheckSVar")?;
        let cmp2 = s2.take("ConditionSVarCompare")?;
        s2.take("ConditionDescription");
        let s3_name = s2.take("SubAbility");
        if !s2.exhausted() || var2.trim() != var1.trim() {
            return None;
        }
        // Verify exact complementary comparison (`GE1`/`GT0` vs `EQ0`/`LE0`/`LT1`).
        let complementary =
            matches!(cmp1.trim(), "GE1" | "GT0") && matches!(cmp2.trim(), "EQ0" | "LE0" | "LT1");
        if !complementary {
            return None;
        }
        let after_cleanup = if let Some(clean_name) = s3_name {
            let clean_body = self.svars.get(clean_name.trim())?.clone();
            let (c_api, mut cp) = Params::parse(&clean_body)?;
            cp.drop_prose();
            if c_api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
                return None;
            }
            let after = cp.take("SubAbility");
            if !cp.exhausted() {
                return None;
            }
            after
        } else if self.on_a_spell {
            None
        } else {
            return None;
        };
        let cond = self
            .svar_condition(&var1, &cmp1)
            .or_else(|| self.extra_svar_condition_clause(&var1, &cmp1))
            .or_else(|| self.wave7_svar_condition(&var1, &cmp1))
            .or_else(|| self.wave11_amount_compare_condition(&var1, &cmp1, true))?;
        // Lower both search branches (`Hand` and `Library` top).
        let mut p_hand = p.clone();
        p_hand.take("Origin");
        p_hand.take("Destination");
        p_hand.take("RememberChanged");
        p_hand.take("LibraryPosition");
        let hand_fx = self.search_library(&mut p_hand, "Hand", None, false)?;
        if !p_hand.exhausted() {
            return None;
        }
        let mut p_top = p.clone();
        p_top.take("Origin");
        p_top.take("Destination");
        p_top.take("RememberChanged");
        let top_fx = self.search_library(&mut p_top, "Library", None, false)?;
        if !p_top.exhausted() {
            return None;
        }
        p.entries.clear();
        *sub = after_cleanup;
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
            hand_fx.join(", "),
            top_fx.join(", ")
        )])
    }

    // =========================================================================
    // Part 4: `CaptureAffected` / `RememberChanged` extensions
    // =========================================================================

    fn wave28a_only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }

    /// True if `sub` (or any downstream `SubAbility$` reachable from `sub`)
    /// begins a resolution-scoped `CaptureAffected` (`RememberChanged$`,
    /// `RememberTargets$`, `RememberLKI$`, `RememberSacrificed$`, etc.).
    ///
    /// Used to allow a leading `DB$ Cleanup | ClearRemembered$ True` right
    /// before a `CaptureAffected` chain (*Deathrender*) without ever dropping a
    /// standalone `Cleanup` that clears persistent state.
    pub(super) fn wave28a_sub_leads_to_capture(&self, sub: Option<&str>) -> bool {
        let mut seen = std::collections::BTreeSet::new();
        let mut cursor = sub.map(|s| s.trim().to_string());
        while let Some(name) = cursor.take() {
            if !seen.insert(name.clone()) {
                return false;
            }
            let Some(body) = self.svars.get(&name) else {
                return false;
            };
            let Some((api, sp)) = Params::parse(body) else {
                return false;
            };
            if Self::wave27c_capture_flag(&api, &sp).is_some()
                || (api == "ChangeZone" && sp.peek("RememberTargets") == Some("True"))
            {
                return true;
            }
            cursor = sp.peek("SubAbility").map(|s| s.trim().to_string());
        }
        false
    }

    /// Extended `Defined$ Remembered` step handler called from `wave27c_remembered_step`:
    /// 1. Full `Animate | Defined$ Remembered` (types, colors, keywords, P/T,
    ///    `RemoveAllAbilities$`, `LeaveBattlefield$ Exile`, `AtEOT$ YourExile`/
    ///    `YourSacrifice`/`YourHand`/`Exile`/`Sacrifice`/`Hand`).
    /// 2. `Attach | Defined$ Remembered` (*Deathrender*) -> `Effect::AttachSelf`.
    /// 3. `CopyPermanent | Defined$ Remembered` (*God-Pharaoh's Gift*, *Hour of
    ///    Eternity*, *Nexus of Becoming*, *Offspring's Revenge*) ->
    ///    `Effect::ForEachRemembered { effects: &[Effect::CreateTokenCopy { target: EventObject, .. }] }`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28a_remembered_step(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        if self.capture_scope.is_none()
            || flags.targets_here
            || p.peek("Defined") != Some("Remembered")
        {
            return None;
        }
        match api {
            "Animate" => self.wave28a_animate_remembered(p),
            "Attach" => {
                if self.on_a_spell {
                    return None;
                }
                let mut probe = p.clone();
                probe.take("Defined");
                if !matches!(probe.take("Object").as_deref(), None | Some("Self")) {
                    return None;
                }
                if !Self::wave28a_only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::ForEachRemembered { filter: &Filter::Any, \
                     effects: &[Effect::AttachSelf { target: TargetSpec::EventObject }] }"
                        .to_string(),
                ])
            }
            "CopyPermanent" => {
                // Battlefield -> Exile -> CopyPermanent would copy printed values
                // in exile rather than battlefield LKI (CR 707.2, CR 608.2h).
                if self
                    .capture_scope
                    .as_ref()
                    .is_none_or(|s| s.from_battlefield)
                {
                    return None;
                }
                let mut probe = p.clone();
                probe.take("Defined");
                probe
                    .entries
                    .push(("Defined".to_string(), "DelayTriggerRemembered".to_string()));
                let saved_in_delayed = std::mem::replace(&mut self.in_delayed, true);
                let copy_target = if flags.targets_a_player { target } else { None };
                let effs = self.copy_permanent_effect(
                    &mut probe,
                    copy_target,
                    false,
                    flags.targets_a_player,
                );
                self.in_delayed = saved_in_delayed;
                let effs = effs?;
                if effs.is_empty() || !Self::wave28a_only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![format!(
                    "Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[{}] }}",
                    effs.join(", ")
                )])
            }
            "Play" => {
                // Single-object `CaptureAffected` -> `Play | Defined$ Remembered`
                // (*Bring to Light*, *Wondrous Crucible*, *Nashi, Moon's Legacy*,
                // *Unexpected Results*, *Founding the Third Path*). Restricting
                // to `single_object` preserves CR 608.2g casting-order choice
                // for multi-card effects like *Spelltwine*.
                if self.overload_cost.is_some()
                    || self.capture_scope.as_ref().is_none_or(|s| !s.single_object)
                    || !matches!(p.peek("Amount"), None | Some("1" | "All"))
                    || p.has("AllowRepeats")
                    || p.has("Valid")
                {
                    return None;
                }
                let mut probe = p.clone();
                probe.take("Defined");
                probe
                    .entries
                    .push(("Defined".to_string(), "DelayTriggerRemembered".to_string()));
                let saved_in_delayed = std::mem::replace(&mut self.in_delayed, true);
                let play_target = if flags.targets_a_player { target } else { None };
                let effs = self.play_effect(&mut probe, play_target, false, flags.targets_a_player);
                self.in_delayed = saved_in_delayed;
                let effs = effs?;
                if effs.is_empty()
                    || !effs
                        .iter()
                        .all(|e| e.contains("PlaySource::Subject(TargetSpec::EventObject)"))
                    || !Self::wave28a_only_condition_keys_left(&probe)
                {
                    return None;
                }
                *p = probe;
                Some(vec![format!(
                    "Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[{}] }}",
                    effs.join(", ")
                )])
            }
            _ => None,
        }
    }

    /// Full `Animate | Defined$ Remembered` inside `CaptureAffected::then`.
    #[allow(clippy::too_many_lines)]
    fn wave28a_animate_remembered(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("IsCurse");
        let duration = match probe.take("Duration").as_deref() {
            None | Some("EOT" | "UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        let mut effects: Vec<String> = Vec::new();
        let mut push_mod = |modifier: String, dur: &str| {
            effects.push(format!(
                "Effect::ModifyEvent {{ modifier: {modifier}, duration: {dur} }}"
            ));
        };

        // Layer 4: types / subtypes / supertypes.
        let remove_card_types = match probe.take("RemoveCardTypes").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if remove_card_types {
            let types = probe.take("Types").unwrap_or_default();
            for modifier in self.become_types_modifiers(&mut probe, &types, &[',', '&'])? {
                push_mod(modifier, duration);
            }
        } else {
            let remove_creature_types = match probe.take("RemoveCreatureTypes").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let remove_land_types = match probe.take("RemoveLandTypes").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            if let Some(raw_types) = probe.take("Types") {
                let mut replaced_creature = false;
                let mut set_land = false;
                for word in raw_types
                    .split([',', '&'])
                    .map(str::trim)
                    .filter(|w| !w.is_empty())
                {
                    if let Some(ct) = card_type_const(word) {
                        push_mod(format!("Modifier::AddType({ct})"), duration);
                    } else if let Some(st) = supertype_const(word) {
                        push_mod(format!("Modifier::AddSupertype({st})"), duration);
                    } else {
                        let path = self.cats.const_path(word)?;
                        if remove_land_types
                            && !set_land
                            && matches!(word, "Plains" | "Island" | "Swamp" | "Mountain" | "Forest")
                        {
                            push_mod(format!("Modifier::SetLandType({path})"), duration);
                            set_land = true;
                        } else if remove_creature_types
                            && !replaced_creature
                            && self
                                .cats
                                .const_path_of(baylee_core::types::SubtypeKind::Creature, word)
                                .is_some()
                        {
                            push_mod(format!("Modifier::ReplaceCreatureTypes({path})"), duration);
                            replaced_creature = true;
                        } else {
                            push_mod(format!("Modifier::AddSubtype({path})"), duration);
                        }
                    }
                }
                if (remove_creature_types && !replaced_creature) || (remove_land_types && !set_land)
                {
                    return None;
                }
            } else if remove_creature_types || remove_land_types {
                return None;
            }
        }
        if probe.take("AddAllCreatureTypes").as_deref() == Some("True") {
            push_mod("Modifier::AllCreatureTypes".to_string(), duration);
        }

        // Layer 5: colors.
        if let Some(raw) = probe.take("Colors") {
            let overwrite = match probe.take("OverwriteColors").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            if raw.trim() == "Colorless" && overwrite {
                push_mod("Modifier::SetColor(ColorSet::EMPTY)".to_string(), duration);
            } else {
                let colors: Option<Vec<&str>> =
                    raw.split(',').map(|c| color_const(c.trim())).collect();
                let colors = colors?;
                let which = if overwrite { "SetColor" } else { "AddColor" };
                push_mod(
                    format!(
                        "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                        colors.join(", ")
                    ),
                    duration,
                );
            }
        } else if probe.has("OverwriteColors") {
            return None;
        }

        // Layer 6: RemoveAllAbilities (first) + Keywords + LeaveBattlefield$ Exile.
        match probe.take("RemoveAllAbilities").as_deref() {
            None | Some("False") => {}
            Some("True") => push_mod("Modifier::LoseAllAbilities".to_string(), duration),
            Some(_) => return None,
        }
        if let Some(raw_kw) = probe.take("Keywords") {
            let each: Option<Vec<&str>> =
                raw_kw.split('&').map(|k| keyword_const(k.trim())).collect();
            let each = each?;
            let joined =
                each.join(".union(") + &")".repeat(raw_kw.split('&').count().saturating_sub(1));
            push_mod(format!("Modifier::AddKeyword({joined})"), duration);
        }
        if let Some(leave) = probe.take("LeaveBattlefield") {
            if leave != "Exile" {
                return None;
            }
            push_mod(
                "Modifier::ExileInsteadOfLeavingBattlefield".to_string(),
                "Duration::Indefinitely",
            );
        }

        // Layer 7b: fixed Power / Toughness.
        match (probe.take("Power"), probe.take("Toughness")) {
            (Some(pow), Some(tou)) => {
                let p_val = pow.trim().parse::<i16>().ok()?;
                let t_val = tou.trim().parse::<i16>().ok()?;
                push_mod(format!("Modifier::SetPT({p_val}, {t_val})"), duration);
            }
            (Some(pow), None) => {
                let p_val = pow.trim().parse::<i16>().ok()?;
                push_mod(format!("Modifier::SetPower({p_val})"), duration);
            }
            (None, Some(tou)) => {
                let t_val = tou.trim().parse::<i16>().ok()?;
                push_mod(format!("Modifier::SetToughness({t_val})"), duration);
            }
            (None, None) => {}
        }

        // Optional delayed end-step cleanup (`AtEOT$`).
        if let Some(at_eot) = probe.take("AtEOT") {
            let (whose, delayed) = match at_eot.as_str() {
                "Hand" => (
                    "None",
                    "Effect::ReturnToHand { target: TargetSpec::EventObject }",
                ),
                "Sacrifice" | "SacrificeCtrl" => ("None", "Effect::SacrificeEvent"),
                "Exile" => ("None", "Effect::exile(TargetSpec::EventObject)"),
                "YourHand" => (
                    "Some(PlayerRel::You)",
                    "Effect::ReturnToHand { target: TargetSpec::EventObject }",
                ),
                "YourSacrifice" => ("Some(PlayerRel::You)", "Effect::SacrificeEvent"),
                "YourExile" => (
                    "Some(PlayerRel::You)",
                    "Effect::exile(TargetSpec::EventObject)",
                ),
                _ => return None,
            };
            effects.push(format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ \
                 step: DelayedStep::EndStep, whose: {whose}, this_turn_only: false, \
                 recurring: false }}, condition: None, targets: None, \
                 remember: TriggerRemember::EventObject, effects: &[{delayed}] }}"
            ));
        }

        // Forge AI-only `sVars$` hint (e.g. *Whip of Erebos*'s `SVar:MustAttack:True`,
        // which is purely an AI attack hint rather than a `Mode$ MustAttack` rules static).
        if let Some(svar_name) = probe.take("sVars")
            && self
                .svars
                .get(svar_name.trim())
                .map(String::as_str)
                .map(str::trim)
                != Some("SVar:MustAttack:True")
        {
            return None;
        }

        if effects.is_empty() || !Self::wave28a_only_condition_keys_left(&probe) {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[{}] }}",
            effects.join(", ")
        )])
    }

    /// Lower `ConditionCheckSVar$` reading `Remembered$Amount`,
    /// `Count$RememberedSize`, or `Remembered$Valid <F>` inside
    /// `Effect::CaptureAffected::then`.
    pub(super) fn wave28a_wrap_remembered_svar_condition(
        &mut self,
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        if p.has("ConditionPresent") || p.has("ConditionDefined") || !p.has("ConditionCheckSVar") {
            return None;
        }
        let mut probe = p.clone();
        let var = probe.take("ConditionCheckSVar")?;
        let cmp = probe
            .take("ConditionSVarCompare")
            .unwrap_or_else(|| "GE1".to_string());
        probe.take("ConditionDescription");
        if !probe.exhausted() {
            return None;
        }
        let raw = self
            .svars
            .get(var.trim())
            .map_or_else(|| var.trim().to_string(), |s| s.trim().to_string());
        let cmp = cmp.trim();

        if matches!(raw.as_str(), "Remembered$Amount" | "Count$RememberedSize") {
            if cmp.len() < 3 {
                return None;
            }
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
            *p = probe;
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::RememberedCount, op: {op}, right: &Amount::Fixed({rhs}) }}, \
                 then: &[{}], otherwise: &[] }}",
                effects.join(", ")
            )]);
        }

        if let Some(valid) = raw.strip_prefix("Remembered$Valid ")
            && scope.single_object
            && matches!(cmp, "GE1" | "EQ1" | "GT0")
        {
            let filter_expr = self.filter_expr_in_zone(valid.trim(), !scope.from_battlefield)?;
            let filter_name = self.body.filter_static("REM_COND", &filter_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ForEachRemembered {{ filter: &{filter_name}, effects: &[{}] }}",
                effects.join(", ")
            )]);
        }

        None
    }
}
