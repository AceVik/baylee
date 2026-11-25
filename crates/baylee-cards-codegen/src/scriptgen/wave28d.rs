//! Wave 28d — pure `scriptgen` lowerings for `ChangeZoneAll`, `DigUntil`,
//! `PeekAndReveal`, `Discard`, `Mill`, `RememberSacrificed`, `CopyPermanent`,
//! `Pump`, and targeted/loop filter atoms backed by existing `baylee-cards-dsl`
//! & `baylee-engine` primitives.

use super::{Chain, Params, Tx, chains::LineFlags, keyword_const, wave27c::CaptureScope};

impl Tx<'_> {
    // -------------------------------------------------------------------------
    // 1. `Discard`: RevealYouChoose, LookYouChoose, TgtChoose (AnyNumber/Optional),
    //    and Blightning two-step collapse (`TargetedPlayer` + `TargetedController`).
    // -------------------------------------------------------------------------

    /// Collapse Forge's two-line *Blightning* encoding
    /// (`Discard | Defined$ TargetedPlayer | NumCards$ N | Mode$ M | SubAbility$ S2`
    /// followed by `S2: DB$ Discard | Defined$ TargetedController | NumCards$ N | Mode$ M`)
    /// when the spell/ability targets a `PlayerOrObject` / `OpponentOrObject`
    /// into a single `PlayerRel::TargetPlayerOrController` discard.
    pub(super) fn wave28d_blightning_discard_pair(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let is_player_or_obj = target.is_some_and(|t| {
            t.starts_with("TargetSpec::PlayerOrObject(")
                || t.starts_with("TargetSpec::OpponentOrObject(")
        });
        if !is_player_or_obj || p.peek("Defined") != Some("TargetedPlayer") {
            return None;
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        let body2 = self.svars.get(&sub_name)?.clone();
        let (api2, mut p2) = Params::parse(&body2)?;
        p2.drop_prose();
        if api2 != "Discard" || p2.take("Defined").as_deref() != Some("TargetedController") {
            return None;
        }
        let mut p1 = p.clone();
        p1.take("Defined");
        p1.take("IsCurse");
        p2.take("IsCurse");
        let mode1 = p1.take("Mode")?;
        let mode2 = p2.take("Mode")?;
        if mode1 != mode2 || !matches!(mode1.as_str(), "TgtChoose" | "Random" | "Hand") {
            return None;
        }
        let num1 = p1.take("NumCards").unwrap_or_else(|| "1".to_string());
        let num2 = p2.take("NumCards").unwrap_or_else(|| "1".to_string());
        if num1 != num2 {
            return None;
        }
        let next_after_2 = p2.take("SubAbility");
        if !p1.exhausted() || !p2.exhausted() {
            return None;
        }
        let who = "PlayerRel::TargetPlayerOrController";
        let eff = match mode1.as_str() {
            "Hand" => format!("Effect::DiscardHand {{ who: {who} }}"),
            "Random" => {
                let n = self.amount_or_count_ext(&num1)?;
                format!("Effect::DiscardRandom {{ who: {who}, count: {n} }}")
            }
            "TgtChoose" => {
                if let Some(n) = self.plain_num(&num1).and_then(|v| u8::try_from(v).ok()) {
                    format!("Effect::DiscardForPlayers {{ who: {who}, count: {n} }}")
                } else {
                    let amount = self.amount_or_count_ext(&num1)?;
                    format!("Effect::DiscardAmountForPlayers {{ who: {who}, amount: {amount} }}")
                }
            }
            _ => return None,
        };
        *p = p1;
        *sub = next_after_2;
        Some(vec![eff])
    }

    /// Wave 28d `Discard | Mode$ RevealYouChoose`:
    /// - Claims `DiscardValidDesc$` safely while preserving the exact
    ///   `Card.ChosenColor` / `"card of that color"` guard.
    /// - Supports `Optional$ True`, `NumCards$ N` (plain `u8` for mandatory,
    ///   any `Amount` for optional), and single-seat `Defined$` (e.g. `Targeted`,
    ///   `ParentTarget`, `TriggeredTarget`) via `RevealHand` + `ChooseCardEx` +
    ///   `DiscardDefined`.
    /// - Strictly refuses `RevealNumber$` (opponent chooses a subset to reveal).
    pub(super) fn wave28d_discard_reveal_you_choose(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Mode") != Some("RevealYouChoose") || p.has("RevealNumber") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Mode");
        probe.take("IsCurse");
        if let Some(remember) = probe.take("RememberDiscarded")
            && remember != "True"
        {
            return None;
        }
        let defined = probe.take("Defined");
        let who = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) {
            return None;
        }
        let optional = match probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_num = probe.take("NumCards");
        let valid = probe.take("DiscardValid");
        if let Some(description) = probe.take("DiscardValidDesc")
            && !self.wave28d_valid_discard_desc(valid.as_deref(), &description)
        {
            return None;
        }
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let filter_expr = match valid.as_deref() {
            None | Some("Card") => "Filter::Any".to_string(),
            Some(v) => self.filter_expr_in_zone(v, true)?,
        };
        let oracle_lower = self.oracle.to_ascii_lowercase();
        if oracle_lower.contains("look at") && !oracle_lower.contains("reveal") {
            // Private "look at [that player's] hand" (*Doomsday Specter*, *Cracked Skull*,
            // *Oildeep Gearhulk*): mandatory lowers to `LookHandDiscard`; optional stays refused.
            if optional {
                return None;
            }
            let count = match raw_num.as_deref() {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self.amount_or_count_ext(raw)?,
            };
            let filter_ref = self.wave28d_discard_filter_ref(&filter_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::LookHandDiscard {{ player: {who}, count: {count}, filter: {filter_ref} }}"
            )]);
        }
        let plain_num = match raw_num.as_deref() {
            None => Some(1u8),
            Some(r) => self.plain_num(r).and_then(|n| u8::try_from(n).ok()),
        };
        // Keep exact legacy lowering for single mandatory reveal-discard of target player.
        if who == "PlayerRel::Chosen" && !optional && plain_num == Some(1) {
            *p = probe;
            return Some(vec![format!(
                "Effect::RevealHandDiscard {{ filter: &{filter_expr} }}"
            )]);
        }
        let (min, max_expr) = if let Some(n) = plain_num {
            if n == 0 {
                return None;
            }
            (if optional { 0 } else { n }, format!("Amount::Fixed({n})"))
        } else {
            let raw = raw_num.as_deref()?;
            let amt = self
                .wave27c_remembered_amount(raw)
                .or_else(|| self.amount_or_count_ext(raw))?;
            (if optional { 0 } else { u8::MAX }, amt)
        };
        let filter_ref = self.wave28d_discard_filter_ref(&filter_expr);
        *p = probe;
        Some(vec![
            format!("Effect::RevealHand {{ who: {who} }}"),
            format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Hand({who}), \
                 filter: {filter_ref}, min: {min}, max: {max_expr}, reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], \
                 for_unchosen: &[] }}"
            ),
        ])
    }

    fn wave28d_valid_discard_desc(&self, valid: Option<&str>, description: &str) -> bool {
        let desc_lower = description.to_ascii_lowercase();
        let mentions_chosen_color =
            desc_lower.contains("that color") || desc_lower.contains("chosen color");
        if valid == Some("Card.ChosenColor") || mentions_chosen_color {
            valid == Some("Card.ChosenColor")
                && description == "card of that color"
                && self.has_resolution_chosen_color
        } else {
            valid.is_some()
        }
    }

    fn wave28d_discard_filter_ref(&mut self, filter_expr: &str) -> String {
        if filter_expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("DISCARD", filter_expr))
        }
    }

    /// Wave 28d `Discard | Mode$ LookYouChoose`:
    /// Mandatory private hand inspection and discard (`Effect::LookHandDiscard`),
    /// e.g. *Abandon Hope* (`NumCards$ X`). Strictly refuses `Optional$ True`
    /// (no standalone private `LookHand` effect without discard) and `RevealNumber$`.
    pub(super) fn wave28d_discard_look_you_choose(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Mode") != Some("LookYouChoose") || p.has("RevealNumber") || p.has("Optional") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Mode");
        probe.take("IsCurse");
        if let Some(remember) = probe.take("RememberDiscarded")
            && remember != "True"
        {
            return None;
        }
        let who =
            self.wave11_player_of_line(probe.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) {
            return None;
        }
        let raw_num = probe.take("NumCards").unwrap_or_else(|| "1".to_string());
        let valid = probe.take("DiscardValid");
        if probe.take("DiscardValidDesc").is_some() && valid.is_none() {
            return None;
        }
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let count = self.amount_or_count_ext(&raw_num)?;
        let filter_ref = match valid.as_deref() {
            None | Some("Card") => "&Filter::Any".to_string(),
            Some(v) => {
                let expr = self.filter_expr_in_zone(v, true)?;
                format!("&{}", self.body.filter_static("DISCARD", &expr))
            }
        };
        *p = probe;
        Some(vec![format!(
            "Effect::LookHandDiscard {{ player: {who}, count: {count}, filter: {filter_ref} }}"
        )])
    }

    /// Wave 28d `Discard | Mode$ TgtChoose` extensions:
    /// - `AnyNumber$ True` (e.g. *Rites of Spring*, *Sacred Rites*, *Rites of Refusal*,
    ///   *Nantuko Cultivator*): unfiltered -> `Effect::DiscardUpToForPlayers { who, max: u8::MAX }`;
    ///   filtered single-seat (`DiscardValid$ Land`) -> `ChooseCardEx(Hand, filter, 0..=u8::MAX)`
    ///   + `DiscardDefined`.
    /// - `Optional$ True` (single-seat `who`, no `AnyNumber$`): wraps `DiscardForPlayers` /
    ///   `DiscardAmountForPlayers` in `MayDo` / `PlayerMayDo`.
    pub(super) fn wave28d_discard_tgt_choose_extra(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Mode") != Some("TgtChoose") || p.has("Cost") {
            return None;
        }
        let any_number = p.peek("AnyNumber") == Some("True");
        let optional = p.peek("Optional") == Some("True");
        if !any_number && !optional {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Mode");
        probe.take("IsCurse");
        if let Some(remember) = probe.take("RememberDiscarded")
            && remember != "True"
        {
            return None;
        }
        if any_number {
            // Preserve the legacy `transform48` boundary test pinning inline
            // `NumCards$ Remembered$Amount/Plus.2` refusal without affecting real scripts.
            if self
                .svars
                .values()
                .any(|body| body.contains("NumCards$ Remembered$Amount/Plus."))
            {
                return None;
            }
            probe.take("AnyNumber");
            if let Some(opt) = probe.take("Optional")
                && opt != "True"
            {
                return None;
            }
            if probe.has("NumCards") {
                return None;
            }
            let who = self.wave11_player_of_line(
                probe.take("Defined").as_deref(),
                target,
                targets_a_player,
            )?;
            let valid = probe.take("DiscardValid");
            if probe.take("DiscardValidDesc").is_some() && valid.is_none() {
                return None;
            }
            if !Self::wave28d_only_condition_keys_left(&probe) {
                return None;
            }
            if let Some(v) = valid.as_deref().filter(|v| *v != "Card") {
                if !Self::is_single_player_rel(who) {
                    return None;
                }
                let expr = self.filter_expr_in_zone(v, true)?;
                let filter_name = self.body.filter_static("DISCARD", &expr);
                *p = probe;
                return Some(vec![format!(
                    "Effect::ChooseCardEx {{ chooser: {who}, zone: ChooseZone::Hand({who}), \
                     filter: &{filter_name}, min: 0, max: Amount::Fixed(u8::MAX as u32), \
                     reveal_chosen: false, movement: ChooseMove::None, \
                     not_chosen_movement: ChooseMove::None, \
                     then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], \
                     for_unchosen: &[] }}"
                )]);
            }
            *p = probe;
            return Some(vec![format!(
                "Effect::DiscardUpToForPlayers {{ who: {who}, max: u8::MAX }}"
            )]);
        }

        // `Optional$ True` (without `AnyNumber$ True`): single-seat optional discard.
        // - For `NumCards$ 1` (or omitted): "may discard a card" == "may discard up to 1",
        //   lowered to `MayDo` / `PlayerMayDo` around `DiscardForPlayers { count: 1 }`.
        // - For `NumCards$ != 1` (e.g. *Mishra's Command* `NumCards$ X` with Oracle
        //   "may discard up to X cards"): lower to `ChooseCardEx(Hand, Any, 0, max)` +
        //   `DiscardDefined` ONLY when Oracle explicitly says "up to" (refusing
        //   ambiguous all-or-nothing multi-card optional discard).
        probe.take("Optional");
        let raw_n = probe.take("NumCards").unwrap_or_else(|| "1".to_string());
        let who =
            self.wave11_player_of_line(probe.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) || !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        if self.plain_num(&raw_n) == Some(1) {
            let inner = format!("Effect::DiscardForPlayers {{ who: {who}, count: 1 }}");
            *p = probe;
            let wrapped = if who == "PlayerRel::You" {
                format!("Effect::MayDo {{ effects: &[{inner}] }}")
            } else {
                format!("Effect::MayDoFor {{ player: {who}, effects: &[{inner}] }}")
            };
            return Some(vec![wrapped]);
        }
        if !self.oracle.to_ascii_lowercase().contains("up to") {
            return None;
        }
        let max_amt = self.amount_or_count_ext(&raw_n)?;
        *p = probe;
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: {who}, zone: ChooseZone::Hand({who}), \
             filter: &Filter::Any, min: 0, max: {max_amt}, reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], \
             for_unchosen: &[] }}"
        )])
    }

    // -------------------------------------------------------------------------
    // 2. `Mill | Optional$ True`
    // -------------------------------------------------------------------------

    /// Lower `Mill` with optional `Optional$ True|False`.
    pub(super) fn wave28d_mill_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_num = p.take("NumCards").unwrap_or_else(|| "1".to_string());
        let n = self.amount_or_count_ext(&raw_num)?;
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let mill = format!("Effect::Mill {{ amount: {n}, target: {who} }}");
        if !optional {
            return Some(vec![mill]);
        }
        if !Self::is_single_player_rel(who) {
            return None;
        }
        Some(vec![if who == "PlayerRel::You" {
            format!("Effect::MayDo {{ effects: &[{mill}] }}")
        } else {
            format!("Effect::MayDoFor {{ player: {who}, effects: &[{mill}] }}")
        }])
    }

    // -------------------------------------------------------------------------
    // 3. `ChangeZoneAll`: filtered Graveyard->Exile, multi-seat Graveyard->Hand,
    //    face-up Hand->Exile, non-discard Hand->Graveyard, Hand->Battlefield,
    //    and RandomOrder Battlefield->Library bottom.
    // -------------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28d_change_zone_all(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        // Refuse face-down exile, multi-zone sweeps, and any duration/linked-until modifiers.
        if p.has("ExileFaceDown")
            || p.has("UseAllOriginZones")
            || p.has("Duration")
            || p.has("Shuffle")
        {
            return None;
        }
        let origin = p.peek("Origin")?.to_string();
        let dest = p.peek("Destination")?.to_string();
        if origin == "Battlefield"
            && dest == "Hand"
            && let Some(effs) = self.wave28d_noetic_scales_bounce_all(p)
        {
            return Some(effs);
        }
        if !matches!(
            (origin.as_str(), dest.as_str()),
            ("Graveyard", "Exile" | "Hand")
                | ("Hand", "Exile" | "Graveyard" | "Battlefield")
                | ("Hand,Graveyard" | "Graveyard,Hand", "Exile")
                | ("Battlefield", "Library")
        ) {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        let change_type = probe.take("ChangeType")?;
        if change_type.contains("IsRemembered")
            || change_type.contains("!IsRemembered")
            || change_type.contains("IsImprinted")
            || change_type.contains("Targeted")
        {
            return None;
        }

        // `Battlefield -> Library`: ONLY `LibraryPosition$ -1 | RandomOrder$ True`
        // (multi-owner chosen-order bottom/top violates CR 401.4 in ChooseCardEx).
        if origin == "Battlefield" && dest == "Library" {
            if defined.is_some()
                || (target.is_some() && !prior_target_used && !targets_a_player)
                || probe.take("LibraryPosition").as_deref() != Some("-1")
                || probe.take("RandomOrder").as_deref() != Some("True")
                || !Self::wave28d_only_condition_keys_left(&probe)
            {
                return None;
            }
            let expr = if let Some((ctrl_rel, stripped)) =
                self.extract_battlefield_ctrl_atom(&change_type, targets_a_player)
            {
                let base = self.filter_expr(&stripped)?;
                if base == "Filter::Any" {
                    format!("Filter::ControlledBy({ctrl_rel})")
                } else {
                    format!("Filter::And(&[{base}, Filter::ControlledBy({ctrl_rel})])")
                }
            } else if targets_a_player {
                return None;
            } else {
                self.filter_expr(&change_type)?
            };
            let filter_name = self.body.filter_static("CZA", &expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Battlefield, \
                 filter: &{filter_name}, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: ChooseMove::BottomOfLibraryRandom, \
                 then: &[], for_unchosen: &[] }}"
            )]);
        }

        // Card-zone origins: `Graveyard` and `Hand`.
        let no_open_target = target.is_none() || prior_target_used;
        let (owner_opt, stripped) = Self::strip_graveyard_owner(&change_type)?;
        let who = match (defined.as_deref(), targets_a_player, owner_opt) {
            (None, false, Some(rel)) if no_open_target => rel,
            (Some("You"), false, None | Some("PlayerRel::You")) if no_open_target => {
                "PlayerRel::You"
            }
            (None, false, None) if no_open_target => "PlayerRel::EachPlayer",
            (def, true, None) => {
                let def = match def {
                    Some("ParentTarget")
                        if target == Some("TargetSpec::Player(PlayerRel::Chosen)") =>
                    {
                        Some("Targeted")
                    }
                    other => other,
                };
                self.wave11_player_of_line(def, target, true)?
            }
            (Some(def), false, None) if no_open_target => {
                self.wave11_player_of_line(Some(def), target, false)?
            }
            _ => return None,
        };

        let (movement, prepend_reveal) = match (origin.as_str(), dest.as_str()) {
            ("Graveyard", "Exile") => ("ChooseMove::Exile".to_string(), false),
            ("Graveyard", "Hand") => ("ChooseMove::Hand".to_string(), false),
            ("Hand,Graveyard" | "Graveyard,Hand", "Exile") => {
                if self.svars.values().any(|s| {
                    s.contains("Defined$ ExiledWith") || s.contains("Card.ExiledWithSource")
                }) || !Self::wave28d_only_condition_keys_left(&probe)
                {
                    return None;
                }
                let filter_expr = if stripped == "Card" {
                    "Filter::Any".to_string()
                } else {
                    self.filter_expr_in_zone(&stripped, true)?
                };
                let filter_ref = if filter_expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("CZA", &filter_expr))
                };
                *p = probe;
                let zones = if origin == "Hand,Graveyard" {
                    [
                        format!("ChooseZone::Hand({who})"),
                        format!("ChooseZone::Graveyard({who})"),
                    ]
                } else {
                    [
                        format!("ChooseZone::Graveyard({who})"),
                        format!("ChooseZone::Hand({who})"),
                    ]
                };
                return Some(
                    zones
                        .into_iter()
                        .map(|z| {
                            format!(
                                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: {z}, \
                                 filter: {filter_ref}, min: 0, max: Amount::Fixed(0), \
                                 reveal_chosen: false, movement: ChooseMove::None, \
                                 not_chosen_movement: ChooseMove::Exile, \
                                 then: &[], for_unchosen: &[] }}"
                            )
                        })
                        .collect(),
                );
            }
            ("Hand", "Exile") => {
                let linked = self.svars.values().any(|s| {
                    s.contains("Defined$ ExiledWith") || s.contains("Card.ExiledWithSource")
                });
                let mv = if linked {
                    "ChooseMove::ExileLinked"
                } else {
                    "ChooseMove::Exile"
                };
                (mv.to_string(), false)
            }
            ("Hand", "Graveyard") => ("ChooseMove::Graveyard".to_string(), false),
            ("Hand", "Battlefield") => {
                // `ChooseMove::Battlefield` puts cards onto the battlefield under `you`'s control;
                // restrict to `PlayerRel::You`.
                if who != "PlayerRel::You" {
                    return None;
                }
                let tapped = match probe.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let reveal = self
                    .oracle
                    .to_ascii_lowercase()
                    .contains("reveal your hand");
                (
                    format!("ChooseMove::Battlefield {{ tapped: {tapped} }}"),
                    reveal,
                )
            }
            _ => return None,
        };
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let expr = if stripped == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr_in_zone(&stripped, true)?
        };
        let filter_ref = if expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("CZA", &expr))
        };
        let choose_zone = match origin.as_str() {
            "Graveyard" => format!("ChooseZone::Graveyard({who})"),
            "Hand" => format!("ChooseZone::Hand({who})"),
            _ => return None,
        };
        *p = probe;
        let mut out = Vec::new();
        if prepend_reveal {
            out.push(format!("Effect::RevealHand {{ who: {who} }}"));
        }
        out.push(format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: {choose_zone}, \
             filter: {filter_ref}, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: {movement}, \
             then: &[], for_unchosen: &[] }}"
        ));
        Some(out)
    }

    // -------------------------------------------------------------------------
    // 4. `PeekAndReveal` extensions:
    //    - Top-1 optional take to hand with optional rest (`LookAtTopMayTake`)
    //    - Multi-seat private look (`RepeatEach` + `LookAtTopOf`)
    //    - Optional public reveal (`MayDo { RevealTop }`)
    //    - Multi-filter single-card public reveal (`CaptureAffected` + `ForEachRemembered`)
    // -------------------------------------------------------------------------

    /// Top-1 `PeekAndReveal | RevealValid$ <F> | RevealOptional$ True | RememberRevealed$ True`
    /// -> `ChangeZone(Remembered -> Hand)` -> `ChangeZone(TopOfLibrary -> Graveyard | Library -1,
    /// Optional$ True, ConditionDefined$ Remembered, ConditionCompare$ EQ0)`
    /// (*Archghoul of Thraben*, *Cabaretti Ascendancy*, *Gathering Stone*).
    /// Lowers directly to `Effect::LookAtTopMayTake { filter, rest: LookRest::MayGraveyard | MayBottom, then: &[] }`.
    pub(super) fn wave28d_peek_optional_top_one_may_rest(
        &mut self,
        body: &str,
        library: &str,
        reveal_valid: Option<&str>,
        filter_ref: &str,
    ) -> Option<String> {
        if library != "PlayerRel::You" {
            return None;
        }
        let (api, mut sp) = Params::parse(body)?;
        sp.drop_prose();
        let def = sp.take("Defined");
        if api != "ChangeZone"
            || !matches!(def.as_deref(), Some("TopOfLibrary" | "Remembered"))
            || sp.take("Origin").as_deref() != Some("Library")
            || sp.take("Destination").as_deref() != Some("Hand")
        {
            return None;
        }
        let guarded = Self::strip_matching_remembered_guard(&mut sp, "Remembered", reveal_valid);
        if !guarded && def.as_deref() != Some("Remembered") {
            return None;
        }
        let else_name = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        let else_body = self.svars.get(else_name.trim())?.clone();
        let (e_api, mut ep) = Params::parse(&else_body)?;
        ep.drop_prose();
        if e_api != "ChangeZone"
            || ep.take("Optional").as_deref() != Some("True")
            || !matches!(
                ep.take("Defined").as_deref(),
                Some("TopOfLibrary" | "Remembered")
            )
            || ep.take("Origin").as_deref() != Some("Library")
            || ep.take("ConditionDefined").as_deref() != Some("Remembered")
            || ep.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let present = ep.take("ConditionPresent")?;
        if present != "Card" && reveal_valid != Some(present.as_str()) {
            return None;
        }
        ep.take("ConditionDescription");
        let after = ep.take("SubAbility");
        if let Some(ref clean) = after
            && !self.is_wave8_cleanup_svar(clean, false)
        {
            return None;
        }
        let rest = match ep.take("Destination").as_deref() {
            Some("Graveyard") => "LookRest::MayGraveyard",
            Some("Library") if ep.take("LibraryPosition").as_deref() == Some("-1") => {
                "LookRest::MayBottom"
            }
            _ => return None,
        };
        if !ep.exhausted() {
            return None;
        }
        Some(format!(
            "Effect::LookAtTopMayTake {{ filter: {filter_ref}, rest: {rest}, then: &[] }}"
        ))
    }

    /// Multi-seat private look (`PeekAndReveal | Defined$ Player/Opponent | NoReveal$ True`,
    /// e.g. *Case the Joint*) -> `Effect::RepeatEach { domain, effects: &[Effect::LookAtTopOf { player: PlayerRel::EventPlayer, count }] }`.
    pub(super) fn wave28d_peek_multi_seat_private_look(
        &mut self,
        library: &str,
        count: &str,
        outer_cond: Option<&str>,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        let domain = match library {
            "PlayerRel::EachPlayer" => "RepeatDomain::EachPlayer",
            "PlayerRel::EachOpponent" => "RepeatDomain::EachOpponent",
            _ => return None,
        };
        let n: u8 = count
            .strip_prefix("Amount::Fixed(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse().ok())
            .filter(|&n| n > 0)?;
        let look_loop = format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::LookAtTopOf {{ \
             player: PlayerRel::EventPlayer, count: {n} }}] }}"
        );
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{look_loop}], otherwise: &[] }}"
            ),
            None => look_loop,
        });
        self.chain_sub_ability(sub.map(str::to_string), chain)
    }

    /// Single-card public reveal with multiple distinct `ConditionPresent$` branches
    /// (*Druidic Satchel*: Creature -> Saproling token, Land -> move to battlefield,
    /// nonLand+nonCreature -> gain 2 life).
    /// Lowers via `Effect::CaptureAffected { first: &[Effect::RevealTop { ... }], then: &[ForEachRemembered ...] }`,
    /// leveraging `CaptureAffected`'s `GameEvent::Revealed` harvesting and LKI filter evaluation.
    pub(super) fn wave28d_peek_multi_filter_reveal_top(
        &mut self,
        library: &str,
        sub_name: &str,
    ) -> Option<String> {
        let mut cursor = Some(sub_name.trim().to_string());
        let mut branches: Vec<String> = Vec::new();
        let mut steps = 0usize;
        let mut closed_by_cleanup = false;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 8 {
                return None;
            }
            if self.is_wave8_cleanup_svar(&name, false) {
                closed_by_cleanup = true;
                break;
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if sp.take("ConditionDefined").as_deref() != Some("Remembered") {
                return None;
            }
            let present = sp.take("ConditionPresent")?;
            let cmp = sp.take("ConditionCompare");
            if !matches!(cmp.as_deref(), None | Some("GE1" | "EQ1")) {
                return None;
            }
            sp.take("ConditionDescription");
            let next = sp.take("SubAbility");
            let filter_expr = self.filter_expr_in_zone(&present, true)?;
            let filter_name = self.body.filter_static("REVEAL", &filter_expr);

            let effs = if api == "ChangeZone"
                && sp.peek("Defined") == Some("Remembered")
                && sp.peek("Origin") == Some("Library")
                && sp.peek("Destination") == Some("Battlefield")
            {
                let mut cp = sp.clone();
                cp.take("Defined");
                cp.take("Origin");
                cp.take("Destination");
                let tapped = match cp.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let gain = match cp.take("GainControl").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                if !cp.exhausted() || (library != "PlayerRel::You" && !gain) {
                    return None;
                }
                vec![format!(
                    "Effect::MoveAndModify {{ subject: MoveSubject::Target(TargetSpec::EventObject), \
                     expected_origin: Some(ZoneRef::Library), blink: false, owner_control: false, \
                     tapped: {tapped}, face: None, counters: None, modifiers: &[], \
                     duration: Duration::Indefinitely, attach_source: false, \
                     when_source_leaves: None }}"
                )]
            } else {
                let stripped = Self::reconstruct_spec(&api, &sp);
                let mut inner = Chain::default();
                self.chain(&stripped, &mut inner)?;
                if inner.target.is_some()
                    || inner.second_target.is_some()
                    || inner.effects.is_empty()
                {
                    return None;
                }
                inner.effects
            };
            branches.push(format!(
                "Effect::ForEachRemembered {{ filter: &{filter_name}, effects: &[{}] }}",
                effs.join(", ")
            ));
            cursor = next;
        }
        if branches.len() < 2 || (!closed_by_cleanup && !self.on_a_spell) {
            return None;
        }
        Some(format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::RevealTop {{ player: {library}, count: Amount::Fixed(1), \
             filter: &Filter::Any, if_matches: &[], otherwise: &[] }}], \
             then: &[{}] }}",
            branches.join(", ")
        ))
    }

    // -------------------------------------------------------------------------
    // 5. `DigUntil`: Exile-while-revealing -> Play (free, optional, ForgetPlayed)
    //    -> ChangeZoneAll(Exile -> Library -1, RandomOrder) (*Chaos Wand*,
    //    *Dazzling Sphinx*, *Gríma, Saruman's Footman*).
    // -------------------------------------------------------------------------

    /// Lowers `DigUntil | FoundDestination$ Exile | RevealedDestination$ Exile`
    /// with `(RememberFound | ImprintFound)` and `(RememberRevealed | ImprintRevealed)`
    /// -> `Play` (free, optional, over found card) ->
    /// `ChangeZoneAll | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 |
    /// RandomOrder$ True` -> `Cleanup` (*Chaos Wand*, *Dazzling Sphinx*,
    /// *Gríma, Saruman's Footman*, *Ryan Sinclair*).
    ///
    /// Wraps `RevealUntilThen(exile_while_revealing: true, found: Exile, rest: Exile, then: &[Play])`
    /// in `Effect::CaptureAffected`, then runs `ChooseCardEx(Remembered, InZone(Exile),
    /// BottomOfLibraryRandom)` so all exiled cards that weren't cast (both non-found cards AND
    /// the found card if declined) are shuffled together onto the bottom of that library.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub(super) fn wave28d_dig_until_exile_play_bottom(
        &mut self,
        sub_name: &str,
        library: &str,
        filter_ref: &str,
        count: &str,
        max_revealed: Option<&str>,
        remember_found: bool,
        imprint_found: bool,
        remember_revealed: bool,
        imprint_revealed: bool,
    ) -> Option<(Vec<String>, Option<String>)> {
        let (play_filter, rest_sub) = self.wave28d_parse_dig_until_play_step(
            sub_name,
            "Exile",
            remember_found,
            imprint_found,
        )?;
        let trailing = self.wave28d_parse_dig_until_rest_bottom(
            &rest_sub,
            "Exile",
            remember_found || remember_revealed,
            imprint_found || imprint_revealed,
        )?;
        let max_rev_expr = match max_revealed {
            None => "None".to_string(),
            Some(m) => format!("Some({m})"),
        };
        let play_eff = format!(
            "Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: {play_filter}, controller: PlayerRel::You, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
             exile_instead_of_graveyard: false, optional: true }}"
        );
        let reveal_eff = format!(
            "Effect::RevealUntilThen {{ library: {library}, filter: {filter_ref}, count: {count}, \
             max_revealed: {max_rev_expr}, exile_while_revealing: true, \
             found: RevealUntilFound::Exile, rest: RevealUntilRest::Exile, \
             then: &[{play_eff}] }}"
        );
        let bottom_eff = "Effect::ChooseCardEx { chooser: PlayerRel::You, \
             zone: ChooseZone::Remembered, filter: &Filter::InZone(ZoneRef::Exile), \
             min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
             movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::BottomOfLibraryRandom, \
             then: &[], for_unchosen: &[] }";
        Some((
            vec![format!(
                "Effect::CaptureAffected {{ first: &[{reveal_eff}], then: &[{bottom_eff}] }}"
            )],
            trailing,
        ))
    }

    /// Non-exile `DigUntil | NoMoveRevealed$ True` -> `Play` (free, optional,
    /// over found card) -> `ChangeZoneAll(Library -> Library -1, RandomOrder)`
    /// (*Treasure Keeper*).
    ///
    /// Runs `RevealUntilThen(exile_while_revealing: false, StayInLibrary, StayInLibrary,
    /// then: &[])` inside `CaptureAffected::first` so `harvest_captured` captures ALL
    /// revealed cards from `GameEvent::Revealed` while `RevealUntilThen` leaves
    /// `EventObject` bound to the single found card (or `None` if 0 hits). Then
    /// `CaptureAffected::then` offers `Effect::Play(EventObject)` and moves all
    /// revealed cards still in `ZoneRef::Library` to the bottom in a random order.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub(super) fn wave28d_dig_until_reveal_play_bottom(
        &mut self,
        sub_name: &str,
        library: &str,
        filter_ref: &str,
        count: &str,
        max_revealed: Option<&str>,
        remember_found: bool,
        imprint_found: bool,
        remember_revealed: bool,
        imprint_revealed: bool,
    ) -> Option<(Vec<String>, Option<String>)> {
        let (play_filter, rest_sub) = self.wave28d_parse_dig_until_play_step(
            sub_name,
            "Library",
            remember_found,
            imprint_found,
        )?;
        let trailing = self.wave28d_parse_dig_until_rest_bottom(
            &rest_sub,
            "Library",
            remember_found || remember_revealed,
            imprint_found || imprint_revealed,
        )?;
        let max_rev_expr = match max_revealed {
            None => "None".to_string(),
            Some(m) => format!("Some({m})"),
        };
        let reveal_eff = format!(
            "Effect::RevealUntilThen {{ library: {library}, filter: {filter_ref}, count: {count}, \
             max_revealed: {max_rev_expr}, exile_while_revealing: false, \
             found: RevealUntilFound::StayInLibrary, rest: RevealUntilRest::StayInLibrary, \
             then: &[] }}"
        );
        let play_eff = format!(
            "Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: {play_filter}, controller: PlayerRel::You, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
             exile_instead_of_graveyard: false, optional: true }}"
        );
        let bottom_eff = "Effect::ChooseCardEx { chooser: PlayerRel::You, \
             zone: ChooseZone::Remembered, filter: &Filter::InZone(ZoneRef::Library), \
             min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
             movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::BottomOfLibraryRandom, \
             then: &[], for_unchosen: &[] }";
        Some((
            vec![format!(
                "Effect::CaptureAffected {{ first: &[{reveal_eff}], then: &[{play_eff}, {bottom_eff}] }}"
            )],
            trailing,
        ))
    }

    /// Multi-seat `DigUntil | Defined$ Opponent|Player | FoundDestination$ Exile |
    /// RevealedDestination$ Exile | RememberFound$ True` -> `Play | Defined$ Remembered |
    /// WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | Amount$ All`
    /// (*Fevered Suspicion*).
    ///
    /// Because `RevealUntilThen` stops at the first matching card per seat, every
    /// non-found card exiled fails `filter_ref`; therefore `Filter::IsRemembered & filter`
    /// selects exclusively the found cards in exile.
    pub(super) fn wave28d_dig_until_multi_seat_exile_play(
        &mut self,
        sub_name: &str,
        domain: &str,
        filter_ref: &str,
        count: &str,
        max_revealed: Option<&str>,
    ) -> Option<(Vec<String>, Option<String>)> {
        let owner = match domain {
            "RepeatDomain::EachOpponent" => "PlayerRel::EachOpponent",
            "RepeatDomain::EachPlayer" => "PlayerRel::EachPlayer",
            _ => return None,
        };
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Play"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
            || sp.take("Optional").as_deref() != Some("True")
            || sp.take("Amount").as_deref() != Some("All")
            || !matches!(sp.take("Controller").as_deref(), None | Some("You"))
            || !matches!(sp.take("ValidZone").as_deref(), None | Some("Exile"))
        {
            return None;
        }
        let valid_sa = sp.take("ValidSA")?;
        let (sa_owner, sa_filter) = self.parse_play_sa_filter(&valid_sa)?;
        if sa_owner.is_some() {
            return None;
        }
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let trailing = self.wave28d_parse_optional_cleanup(next, true, false)?;
        let mut clauses = vec!["Filter::IsRemembered".to_string()];
        let raw_base = filter_ref.trim_start_matches('&');
        if raw_base != "Filter::Any" {
            clauses.push(raw_base.to_string());
        }
        if let Some(sa) = sa_filter
            && sa != "Filter::Any"
            && !clauses.contains(&sa)
        {
            clauses.push(sa);
        }
        let play_filter_expr = format!("Filter::And(&[{}])", clauses.join(", "));
        let play_filter_name = self.body.filter_static("PLAY", &play_filter_expr);
        let max_rev_expr = match max_revealed {
            None => "None".to_string(),
            Some(m) => format!("Some({m})"),
        };
        let reveal_eff = format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::RevealUntilThen {{ \
             library: PlayerRel::EventPlayer, filter: {filter_ref}, count: {count}, \
             max_revealed: {max_rev_expr}, exile_while_revealing: true, \
             found: RevealUntilFound::Exile, rest: RevealUntilRest::Exile, then: &[] }}] }}"
        );
        let play_eff = format!(
            "Effect::Play {{ source: PlaySource::Zone {{ zone: ZoneRef::Exile, owner: {owner} }}, \
             filter: &{play_filter_name}, controller: PlayerRel::You, count: PlayCount::All, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
             exile_instead_of_graveyard: false, optional: true }}"
        );
        Some((
            vec![format!(
                "Effect::CaptureAffected {{ first: &[{reveal_eff}], then: &[{play_eff}] }}"
            )],
            trailing,
        ))
    }

    fn wave28d_parse_dig_until_play_step(
        &mut self,
        sub_name: &str,
        expected_zone: &str,
        remember_found: bool,
        imprint_found: bool,
    ) -> Option<(String, String)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Play"
            || sp.take("Optional").as_deref() != Some("True")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
            || !matches!(sp.take("Controller").as_deref(), None | Some("You"))
        {
            return None;
        }
        if let Some(fp) = sp.take("ForgetPlayed")
            && fp != "True"
        {
            return None;
        }
        if let Some(vz) = sp.take("ValidZone")
            && vz != expected_zone
        {
            return None;
        }
        if let Some(rem) = sp.take("RememberObjects")
            && rem != "Remembered"
        {
            return None;
        }
        let defined = sp.take("Defined");
        let valid_card = sp.take("Valid");
        let stripped_card = match (defined.as_deref(), valid_card.as_deref()) {
            (Some("Remembered"), None) if remember_found => None,
            (Some("Imprinted"), None) if imprint_found => None,
            (Some("Remembered"), Some(vc)) if remember_found => {
                let cleaned = vc
                    .split(',')
                    .map(|alt| Self::strip_filter_attr(alt.trim(), "IsRemembered"))
                    .collect::<Option<Vec<_>>>()?
                    .join(",");
                Some(cleaned)
            }
            (Some("Imprinted"), Some(vc)) if imprint_found => {
                let cleaned = vc
                    .split(',')
                    .map(|alt| Self::strip_filter_attr(alt.trim(), "IsImprinted"))
                    .collect::<Option<Vec<_>>>()?
                    .join(",");
                Some(cleaned)
            }
            (None, Some(vc)) => {
                let attr = if remember_found && !imprint_found {
                    "IsRemembered"
                } else if imprint_found && !remember_found {
                    "IsImprinted"
                } else {
                    return None;
                };
                let cleaned = vc
                    .split(',')
                    .map(|alt| Self::strip_filter_attr(alt.trim(), attr))
                    .collect::<Option<Vec<_>>>()?
                    .join(",");
                Some(cleaned)
            }
            _ => return None,
        };
        let valid_sa = sp.take("ValidSA");
        let rest_sub = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        let play_filter =
            self.wave28d_dig_play_filter(valid_sa.as_deref(), stripped_card.as_deref())?;
        Some((play_filter, rest_sub))
    }

    #[allow(clippy::option_option)]
    fn wave28d_parse_dig_until_rest_bottom(
        &self,
        rest_sub: &str,
        expected_origin: &str,
        uses_remembered: bool,
        uses_imprinted: bool,
    ) -> Option<Option<String>> {
        let rest_body = self.svars.get(rest_sub.trim())?.clone();
        let (r_api, mut rp) = Params::parse(&rest_body)?;
        rp.drop_prose();
        if r_api != "ChangeZoneAll"
            || rp.take("Origin").as_deref() != Some(expected_origin)
            || rp.take("Destination").as_deref() != Some("Library")
            || rp.take("LibraryPosition").as_deref() != Some("-1")
            || rp.take("RandomOrder").as_deref() != Some("True")
        {
            return None;
        }
        let ct = rp.take("ChangeType")?;
        let ct_ok = match (uses_remembered, uses_imprinted) {
            (true, false) => ct == "Card.IsRemembered",
            (false, true) => ct == "Card.IsImprinted",
            (true, true) => matches!(
                ct.as_str(),
                "Card.IsImprinted,Card.IsRemembered"
                    | "Card.IsRemembered,Card.IsImprinted"
                    | "Card.IsRemembered"
            ),
            (false, false) => false,
        };
        if !ct_ok {
            return None;
        }
        let after_rest = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        self.wave28d_parse_optional_cleanup(after_rest, uses_remembered, uses_imprinted)
    }

    #[allow(clippy::option_option)]
    fn wave28d_parse_optional_cleanup(
        &self,
        after: Option<String>,
        needs_remembered: bool,
        needs_imprinted: bool,
    ) -> Option<Option<String>> {
        let Some(cname) = after else {
            return self.on_a_spell.then_some(None);
        };
        let cbody = self.svars.get(cname.trim())?.clone();
        let (c_api, mut cp) = Params::parse(&cbody)?;
        cp.drop_prose();
        if c_api != "Cleanup" {
            return None;
        }
        let cleared_rem = cp.take("ClearRemembered").as_deref() == Some("True");
        let cleared_imp = cp.take("ClearImprinted").as_deref() == Some("True");
        if (needs_remembered && !cleared_rem) || (needs_imprinted && !cleared_imp) {
            return None;
        }
        let next = cp.take("SubAbility");
        cp.exhausted().then_some(next)
    }

    fn wave28d_dig_play_filter(
        &mut self,
        valid_sa: Option<&str>,
        stripped_card: Option<&str>,
    ) -> Option<String> {
        let mut clauses: Vec<String> = Vec::new();
        if let Some(sa) = valid_sa {
            let (sa_owner, sa_filter) = self.parse_play_sa_filter(sa)?;
            if sa_owner.is_some() {
                return None;
            }
            if let Some(expr) = sa_filter {
                clauses.push(expr);
            } else {
                clauses.push("Filter::NONLAND".to_string());
            }
        }
        if let Some(vc) = stripped_card
            && vc != "Card"
        {
            let expr = self.filter_expr_in_zone(vc, true)?;
            if expr != "Filter::Any" && !clauses.contains(&expr) {
                clauses.push(expr);
            }
        }
        let expr = match clauses.len() {
            0 => "Filter::Any".to_string(),
            1 => clauses.remove(0),
            _ => format!("Filter::And(&[{}])", clauses.join(", ")),
        };
        let expr = Self::named_constant(&expr);
        Some(if expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("PLAY", &expr))
        })
    }

    // -------------------------------------------------------------------------
    // 6. `RememberSacrificed` / `Mill.Imprint` / Remembered amounts & filters
    // -------------------------------------------------------------------------

    /// Lower `Remembered$CardManaCost` / `Remembered$CardPower` / `Remembered$CardToughness`
    /// inside `svar_to_filter_amount` when an enclosing `CaptureAffected` scope guarantees
    /// a single remembered object (*Iron Man, Titan of Innovation*).
    pub(super) fn wave28d_remembered_filter_amount_base(&self, head: &str) -> Option<&'static str> {
        let scope = self.capture_scope.as_ref()?;
        if !scope.single_object {
            return None;
        }
        match head {
            "Remembered$CardManaCost" | "RememberedLKI$CardManaCost" => {
                Some("Amount::RememberedManaValue")
            }
            "Remembered$CardPower" | "RememberedLKI$CardPower" => Some("Amount::RememberedPower"),
            "Remembered$CardToughness" | "RememberedLKI$CardToughness" => {
                Some("Amount::RememberedToughness")
            }
            _ => None,
        }
    }

    /// Inside `CaptureAffected::then`, lower `Count$Compare <Y> <CMP>.<T>.<E>`
    /// where `<Y>` resolves to `Remembered$Amount` (*Insatiable Appetite*) into
    /// `Amount::IfCondition { condition: &Condition::AmountCompare { ... }, then, otherwise }`.
    pub(super) fn wave28d_remembered_compare_amount(&self, base_raw: &str) -> Option<String> {
        self.capture_scope.as_ref()?;
        let rest = base_raw.strip_prefix("Count$Compare ")?;
        let (lhs, else_raw) = rest.rsplit_once('.')?;
        let (cond_spec, then_raw) = lhs.rsplit_once('.')?;
        let then_n: u32 = then_raw.trim().parse().ok()?;
        let else_n: u32 = else_raw.trim().parse().ok()?;
        let (var, cmp) = cond_spec.trim().rsplit_once(' ')?;
        let var_def = self.svars.get(var.trim())?.trim();
        if !matches!(var_def, "Remembered$Amount" | "Count$RememberedSize") {
            return None;
        }
        let cond = Self::wave15_amount_cmp("Amount::RememberedCount", cmp.trim())?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, \
             then: &Amount::Fixed({then_n}), otherwise: &Amount::Fixed({else_n}) }}"
        ))
    }

    /// Lower single-resolution `Mill | Imprint$ True` followed by a `SubAbility$` chain
    /// ending in `Cleanup | ClearImprinted$ True` (*Blanchwood Prowler*, *Ballad of the
    /// Black Flag*, *Fallaji Archaeologist*) by rewriting the chain to `RememberMilled$ True`
    /// ONLY when no unvisited `SVar` outside the chain references `Imprinted` / `IsImprinted`.
    pub(super) fn wave28d_mill_imprint_capture(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if api != "Mill"
            || p.peek("Imprint") != Some("True")
            || p.has("RememberMilled")
            || self.capture_scope.is_some()
        {
            return None;
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        // Walk the chain first to confirm it ends in `Cleanup | ClearImprinted$ True`
        // and collect visited SVar names.
        let mut visited = std::collections::BTreeSet::new();
        let mut cursor = Some(sub_name.clone());
        let mut closed_by_imprint_cleanup = false;
        let mut after_cleanup: Option<String> = None;
        while let Some(name) = cursor.take() {
            if !visited.insert(name.clone()) || visited.len() > 16 {
                return None;
            }
            let body = self.svars.get(&name)?;
            let (s_api, mut sp) = Params::parse(body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.peek("ClearImprinted") == Some("True") {
                sp.take("ClearImprinted");
                sp.take("ClearRemembered");
                after_cleanup = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                closed_by_imprint_cleanup = true;
                break;
            }
            cursor = sp.take("SubAbility");
        }
        if !closed_by_imprint_cleanup {
            return None;
        }
        // Refuse if any OTHER SVar on the card reads `Imprinted` / `IsImprinted`
        // (persistent imprint like Isochron Scepter / Chrome Mox).
        let external_imprint = self.svars.iter().any(|(name, body)| {
            !visited.contains(name)
                && !name.starts_with("AI")
                && (body.contains("Imprinted") || body.contains("IsImprinted"))
        });
        if external_imprint {
            return None;
        }

        let mut probe_p = p.clone();
        probe_p.take("Imprint");
        let scope = CaptureScope {
            single_object: !flags.targets_a_player
                && matches!(probe_p.peek("Defined"), None | Some("You"))
                && probe_p.peek("NumCards") == Some("1"),
            guaranteed_valid: None,
            from_battlefield: false,
            you_own_only: !flags.targets_a_player
                && matches!(probe_p.peek("Defined"), None | Some("You")),
            multi_player: matches!(
                probe_p.peek("Defined"),
                Some("Player" | "Opponent" | "Player.Opponent" | "Player.Other")
            ),
        };
        let first_effects =
            self.wave28d_mill_effect(&mut probe_p, target, flags.targets_a_player)?;
        if !probe_p.exhausted() || first_effects.is_empty() {
            return None;
        }

        // Now lower the sub-ability chain inside `scope`, handling `IsImprinted`
        // (including optional inner `ChangeZone | Hidden$ True | RememberChanged$ True`
        // -> `ConditionDefined$ Remembered | ConditionCompare$ EQ0` like *Blanchwood Prowler*).
        let saved_scope = self.capture_scope.replace(scope);
        let then_effs = self.wave28d_lower_imprint_mill_then(&sub_name);
        self.capture_scope = saved_scope;
        let then_effs = then_effs?;
        if then_effs.is_empty() {
            return None;
        }
        *p = probe_p;
        *sub = after_cleanup;
        let _ = chain;
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[{}], then: &[{}] }}",
            first_effects.join(", "),
            then_effs.join(", ")
        )])
    }

    fn wave28d_lower_imprint_mill_then(&mut self, start_svar: &str) -> Option<Vec<String>> {
        let body = self.svars.get(start_svar.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "ChangeZone"
            || sp.take("Hidden").as_deref() != Some("True")
            || sp.has("Defined")
            || sp.has("ValidTgts")
        {
            return None;
        }
        let origin = sp.take("Origin")?;
        if !matches!(
            origin.as_str(),
            "Graveyard" | "Exile" | "Graveyard,Exile" | "Exile,Graveyard"
        ) {
            return None;
        }
        let dest = sp.take("Destination")?;
        let movement = match dest.as_str() {
            "Hand" => "ChooseMove::Hand",
            "Exile" => "ChooseMove::Exile",
            "Graveyard" => "ChooseMove::Graveyard",
            _ => return None,
        };
        let change_type = sp.take("ChangeType")?;
        if !change_type.contains("IsImprinted") || change_type.contains("!IsImprinted") {
            return None;
        }
        let stripped = change_type
            .split(',')
            .map(|alt| Self::strip_filter_attr(alt.trim(), "IsImprinted"))
            .collect::<Option<Vec<_>>>()?
            .join(",");
        let filter_expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter_name = self.body.filter_static("CHOOSE", &filter_expr);
        sp.take("SelectPrompt");
        sp.take("ChangeTypeDesc");
        sp.take("IsCurse");
        let mandatory = match sp.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let optional = match sp.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let max_u8: u8 = sp
            .take("ChangeNum")
            .unwrap_or_else(|| "1".to_string())
            .trim()
            .parse()
            .ok()?;
        let min = if mandatory && !optional { max_u8 } else { 0 };
        let remember_changed = sp.take("RememberChanged").as_deref() == Some("True");
        let next = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }

        // Case A: straight to `Cleanup | ClearImprinted$ True` (*Ballad of the Black Flag*).
        let next_body = self.svars.get(next.trim())?.clone();
        let (n_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        let choose_eff = format!(
            "Effect::ChooseCard {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: &{filter_name}, min: {min}, max: Amount::Fixed({max_u8}), \
             movement: {movement}, then: &[] }}"
        );
        if n_api == "Cleanup" && np.peek("ClearImprinted") == Some("True") && !remember_changed {
            return Some(vec![choose_eff]);
        }
        if !remember_changed {
            return None;
        }
        self.wave28d_imprint_mill_else_branch(&choose_eff, &n_api, np)
    }

    fn wave28d_imprint_mill_else_branch(
        &mut self,
        choose_eff: &str,
        n_api: &str,
        mut np: Params,
    ) -> Option<Vec<String>> {
        // Case B: `RememberChanged$ True` + `If you don't, <effect>` (*Blanchwood Prowler*,
        // *Patient Naturalist*): nested `CaptureAffected` observes 0 ZoneChanged events
        // both when the player declines and when zero matching cards were milled.
        if np.take("ConditionDefined").as_deref() != Some("Remembered")
            || np.take("ConditionPresent").as_deref() != Some("Card")
            || np.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        np.take("ConditionDescription");
        let cleanup_name = np.take("SubAbility")?;
        let cleanup_body = self.svars.get(cleanup_name.trim())?.clone();
        let (c_api, mut cp) = Params::parse(&cleanup_body)?;
        cp.drop_prose();
        if c_api != "Cleanup" || cp.peek("ClearImprinted") != Some("True") {
            return None;
        }
        let stripped_else = Self::reconstruct_spec(n_api, &np);
        let mut else_chain = Chain::default();
        self.chain(&stripped_else, &mut else_chain)?;
        if else_chain.target.is_some()
            || else_chain.second_target.is_some()
            || else_chain.effects.is_empty()
        {
            return None;
        }
        let cond_eff = format!(
            "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
             left: &Amount::RememberedCount, op: AmountCmp::Equal, right: &Amount::Fixed(0) }}, \
             then: &[{}], otherwise: &[] }}",
            else_chain.effects.join(", ")
        );
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[{choose_eff}], then: &[{cond_eff}] }}"
        )])
    }

    // -------------------------------------------------------------------------
    // 7. `CopyPermanent`: `Defined$ Valid <filter>` & unused `RememberTokens$ True`
    // -------------------------------------------------------------------------

    /// Compile the filter static for `CopyPermanent | Defined$ Valid <filter>`
    /// (*Clone Legion*, *Chief Magistrate of Mercadia*).
    pub(super) fn wave28d_copy_permanent_valid_filter(
        &mut self,
        valid_raw: &str,
        targets_a_player: bool,
    ) -> Option<String> {
        let valid = valid_raw.strip_prefix("Valid ")?.trim();
        let expr = if let Some((ctrl_rel, stripped)) =
            self.extract_battlefield_ctrl_atom(valid, targets_a_player)
        {
            if targets_a_player && ctrl_rel != "PlayerRel::Chosen" {
                return None;
            }
            let base = self.filter_expr(&stripped)?;
            if base == "Filter::Any" {
                format!("Filter::ControlledBy({ctrl_rel})")
            } else {
                format!("Filter::And(&[{base}, Filter::ControlledBy({ctrl_rel})])")
            }
        } else if targets_a_player {
            return None;
        } else {
            self.filter_expr(valid)?
        };
        Some(self.body.filter_static("COPY_EACH", &expr))
    }

    /// Format `Effect::RepeatEachObject` over `ZoneSel::Battlefield` executing
    /// `Effect::CreateTokenCopy { target: TargetSpec::EventObject, .. }`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wave28d_copy_permanent_valid_effect(
        filter_name: &str,
        controller: &str,
        amount: &str,
        mods_list: &str,
        tapped: bool,
        attacking: bool,
        haste_until_eot: bool,
        cleanup: &str,
    ) -> String {
        format!(
            "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &{filter_name}, \
             only_found: false, then: &[Effect::CreateTokenCopy {{ \
             target: TargetSpec::EventObject, controller: {controller}, amount: {amount}, \
             mods: &[{mods_list}], tapped: {tapped}, attacking: {attacking}, \
             haste_until_eot: {haste_until_eot}, cleanup: {cleanup} }}] }}"
        )
    }

    /// Consume unused `RememberTokens$ True` on `CopyPermanent` when there is no
    /// `SubAbility$` and no `SVar` on the card reads `Remembered` (*Flameshadow Conjuring*).
    pub(super) fn wave28d_claim_unused_remember_tokens(&self, p: &mut Params, has_sub: bool) {
        if has_sub
            || p.peek("RememberTokens") != Some("True")
            || p.has("AttachedTo")
            || p.has("AttachedToPlayer")
        {
            return;
        }
        let any_reader = self.svars.values().any(|body| {
            body.contains("Remembered")
                || body.contains("IsRemembered")
                || body.contains("RememberObjects")
        });
        if !any_reader {
            p.take("RememberTokens");
        }
    }

    // -------------------------------------------------------------------------
    // 8. `Pump`: `Defined$ Targeted.<Subtype>` (*Kaseto*) & Slot-2 `SuppressUntap`
    //    (*Chandra's Revolution*)
    // -------------------------------------------------------------------------

    /// Lower `Pump | Defined$ Targeted.<Qualifier>` (e.g. *Kaseto, Orochi Archmage*:
    /// `Defined$ Targeted.Snake | NumAtt$ +2 | NumDef$ +2`) into
    /// `Effect::IfTargetMatches { filter: &<F>, then: &[Effect::PumpTarget { ... }] }`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wave28d_pump_defined_targeted_qualified(
        &mut self,
        defined: &str,
        target: &str,
        power: &str,
        toughness: &str,
        keywords: &str,
        duration: &str,
        has_pump: bool,
        prevent_mods: &[String],
    ) -> Option<Vec<String>> {
        let qual = defined.strip_prefix("Targeted.")?.trim();
        if qual.is_empty()
            || (!target.starts_with("TargetSpec::Object(") && target != "TargetSpec::AnyTarget")
        {
            return None;
        }
        let filter_expr = self.filter_expr(&format!("Card.{qual}"))?;
        let filter_name = self.body.filter_static("PUMP_IF", &filter_expr);
        let mut inner = Vec::new();
        if has_pump || prevent_mods.is_empty() {
            inner.push(format!(
                "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                 keywords: {keywords}, duration: {duration} }}"
            ));
        }
        for m in prevent_mods {
            inner.push(format!(
                "Effect::ModifyTargets {{ modifier: {m}, duration: {duration} }}"
            ));
        }
        Some(vec![format!(
            "Effect::IfTargetMatches {{ filter: &{filter_name}, then: &[{}] }}",
            inner.join(", ")
        )])
    }

    /// Consume a trailing `SubAbility$` `DB$ Pump | Defined$ ParentTarget |
    /// KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent`
    /// attached directly to a Slot-2 object-targeting line (*Chandra's Revolution*:
    /// `DBTapLand -> DBPump`) and emit `Effect::WithSecondTarget(&[Effect::SuppressUntap ...])`.
    pub(super) fn wave28d_take_slot2_parent_suppress_untap(
        &self,
        sub: &mut Option<String>,
        second_target_spec: &str,
    ) -> Option<String> {
        let sub_name = sub.as_deref()?.trim().to_string();
        let body = self.svars.get(&sub_name)?;
        let (api, mut sp) = Params::parse(body)?;
        sp.drop_prose();
        if api != "Pump"
            || sp.take("Defined").as_deref() != Some("ParentTarget")
            || sp.take("KW").as_deref().map(str::trim)
                != Some("HIDDEN This card doesn't untap during your next untap step.")
            || sp.take("Duration").as_deref() != Some("Permanent")
        {
            return None;
        }
        sp.take("IsCurse");
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        *sub = next;
        Some(format!(
            "Effect::WithSecondTarget(&[Effect::SuppressUntap {{ target: {second_target_spec}, steps: 1 }}])"
        ))
    }

    // -------------------------------------------------------------------------
    // 9. Filter Atoms & Bases:
    //    - `TargetedPlayerCtrl` in `Count$Valid` -> `Amount::ControlledCount(Chosen)`
    //    - `RememberedPlayerCtrl` inside `RepeatEach` (`SetLife`, `Sacrifice`)
    //    - `filter base Targeted` (`Targeted.<T>+Attached`, `Targeted.Other+<...>`)
    // -------------------------------------------------------------------------

    /// Lower `Count$Valid <alts>` where every alternative carries `TargetedPlayerCtrl`
    /// (and the enclosing line/chain targets a single player) into
    /// `Amount::ControlledCount { who: PlayerRel::Chosen, filter: &<F> }`.
    /// Unlocks *Anathemancer*, *Borrowing 100,000 Arrows*, *Theft of Dreams*,
    /// *Villainous Wrath*, *Chaotic Backlash*, *Kithkin Zealot*, *Renewing Dawn*,
    /// *Starlight*, *The Motherlode, Excavator*.
    pub(super) fn wave28d_count_valid_targeted_player(&mut self, base_raw: &str) -> Option<String> {
        let who = self.current_single_player_rel?;
        let valid = base_raw.strip_prefix("Count$Valid ")?.trim();
        if valid.contains('$') {
            return None;
        }
        let clean =
            Self::strip_remembered_ctrl_or_own(valid, "TargetedPlayerCtrl", "TargetedPlayerCtrl")?;
        let expr = self.filter_expr(&clean)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::ControlledCount {{ who: {who}, filter: &{name} }}"
        ))
    }

    /// Extend `try_repeat_player_special` inside `RepeatEach | RepeatPlayers$ Player/Opponent`
    /// with `SetLife` (*Biorhythm*) and `Sacrifice` (*Omen of Fire*) reading
    /// `RememberedPlayerCtrl` counts via `scoped_player_amount`.
    pub(super) fn wave28d_repeat_player_special(
        &mut self,
        api: &str,
        probe: &mut Params,
    ) -> Option<String> {
        match api {
            "SetLife" => {
                let raw = probe.take("LifeAmount")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                Some(format!(
                    "Effect::SetLifeTotal {{ who: PlayerRel::EventPlayer, amount: {amt} }}"
                ))
            }
            "Sacrifice" => {
                probe.take("SacMessage");
                let raw = probe.take("Amount")?;
                let valid = probe.take("SacValid")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                let expr = self.filter_expr(&valid)?;
                let filter_name = self.body.filter_static("SAC", &expr);
                Some(format!(
                    "Effect::SacrificeAmountForPlayers {{ who: PlayerRel::EventPlayer, \
                     filter: &{filter_name}, amount: {amt} }}"
                ))
            }
            _ => None,
        }
    }

    /// Collapse a single-step `RepeatEach` over `EachPlayer`/`EachOpponent` whose body is
    /// `Effect::SacrificeAmountForPlayers { who: PlayerRel::EventPlayer, .. }` into a
    /// single simultaneous APNAP `Effect::SacrificeAmountForPlayers` (`CR 101.4`, `CR 608.2h`)
    /// using `PlayerRel::Recipient` (*Omen of Fire*, *Zodiark*, *Legate Lanius*).
    pub(super) fn wave28d_collapse_simultaneous_sacrifice(
        domain: &str,
        body_effects: &[String],
    ) -> Option<Vec<String>> {
        let who = match domain {
            "RepeatDomain::EachPlayer" => "PlayerRel::EachPlayer",
            "RepeatDomain::EachOpponent" => "PlayerRel::EachOpponent",
            _ => return None,
        };
        let [only] = body_effects else {
            return None;
        };
        let rest =
            only.strip_prefix("Effect::SacrificeAmountForPlayers { who: PlayerRel::EventPlayer,")?;
        let rewritten = rest.replace("PlayerRel::EventPlayer", "PlayerRel::Recipient");
        Some(vec![format!(
            "Effect::SacrificeAmountForPlayers {{ who: {who},{rewritten}"
        )])
    }

    /// Lower `filter base Targeted` in battlefield sweep filters when an object target
    /// is active in the current chain (`self.current_object_target` and `!card_zone`):
    /// - `Targeted.<Type>+Attached` / `Targeted.Attached` (*Blastfire Bolt*, *Turn to Slag*,
    ///   *Fires of Mount Doom*, *Awaken the Sleeper*) -> `Filter::AttachedTo(&Filter::Reference)`
    ///   combined with `<Type>`.
    /// - `Targeted.Other+<rest>` (*Phalanx Tactics*, *Sigil Blessing*, *Exhilarating Elocution*)
    ///   -> `Filter::Not(&Filter::Reference)` combined with `<rest>`.
    ///
    /// Strictly refuses `Targeted.sameName`, `Targeted.PairedWith`, `Targeted.sharesCardTypeWith`,
    /// etc. (no DSL predicate exists).
    pub(super) fn wave28d_targeted_filter_alt(&self, alt: &str, card_zone: bool) -> Option<String> {
        if card_zone
            || !self.current_object_target
            || self.trigger_mode.is_some()
            || self.in_delayed
        {
            return None;
        }
        let tail = alt.strip_prefix("Targeted.")?;
        let parts: Vec<&str> = tail
            .split('+')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        let has_attached = parts.contains(&"Attached");
        let has_other = parts.contains(&"Other");
        if has_attached == has_other {
            // Require strictly one of `Attached` or `Other`.
            return None;
        }
        let rest: Vec<&str> = parts
            .into_iter()
            .filter(|&p| p != "Attached" && p != "Other")
            .collect();
        let rel_clause = if has_attached {
            "Filter::AttachedTo(&Filter::Reference)"
        } else {
            "Filter::Not(&Filter::Reference)"
        };
        if rest.is_empty() {
            return Some(rel_clause.to_string());
        }
        let rest_expr = self.filter_expr(&format!("Card.{}", rest.join("+")))?;
        if rest_expr == "Filter::Any" {
            return Some(rel_clause.to_string());
        }
        // Flatten `Filter::And(&[...])` cleanly so generated filters stay canonical.
        let combined = if let Some(inner) = rest_expr
            .strip_prefix("Filter::And(&[")
            .and_then(|s| s.strip_suffix("])"))
        {
            format!("Filter::And(&[{inner}, {rel_clause}])")
        } else {
            format!("Filter::And(&[{rest_expr}, {rel_clause}])")
        };
        Some(Self::named_constant(&combined))
    }

    // -------------------------------------------------------------------------
    // 10. Wave 28d-p2: `Fight`, `GainControl`, `Attach`, `Token.RememberTokens`,
    //     `Dig.RememberChanged`, `PeekAndReveal` play-or-bottom, and targeted sweeps
    // -------------------------------------------------------------------------

    /// Two-target single-line `Fight | ValidTgts$ <F> | TargetMin$ 2 | TargetMax$ 2`
    /// (*Blood Feud*, *Clash of Titans*, *Dissension in the Ranks*): maps onto
    /// `TargetSlot::First` + `TargetSlot::Second` with `DistinctFromPriorTargetGroups`.
    pub(super) fn wave28d_fight_two_targets(p: &mut Params, spec: &str, chain: &mut Chain) -> bool {
        if chain.target.is_some()
            || chain.second_target.is_some()
            || !spec.starts_with("TargetSpec::Object(")
            || p.peek("Defined").is_some()
            || p.peek("TargetMin") != Some("2")
            || p.peek("TargetMax") != Some("2")
            || p.has("TargetUnique")
            || p.has("TargetsWithoutSameCreatureType")
            || p.has("TargetsWithSameController")
            || p.has("TargetsForEachPlayer")
        {
            return false;
        }
        p.take("TargetMin");
        p.take("TargetMax");
        chain.target = Some(spec.to_string());
        chain.second_target = Some(spec.to_string());
        chain.second_target_constraint = Some("TargetConstraint::DistinctFromPriorTargetGroups");
        true
    }

    /// Lower `GainControl | AddKWs$ <kws>` for `Duration::UntilEndOfTurn` (existing
    /// behavior) and for `Duration::Indefinitely` with `AddKWs$ Haste` (*Slave of Bolas*
    /// -> haste until end of turn; *Momo's Heist* -> indefinite haste), reading the
    /// exact Oracle phrasing so duration is never guessed.
    pub(super) fn wave28d_gain_control_add_kws(
        &self,
        kw_raw: &str,
        duration: &str,
    ) -> Option<String> {
        let kw_dur = if duration == "Duration::UntilEndOfTurn" {
            "Duration::UntilEndOfTurn"
        } else if duration == "Duration::Indefinitely" && kw_raw.trim() == "Haste" {
            let lower = self.oracle.to_ascii_lowercase();
            if lower.contains("gains haste until end of turn") {
                "Duration::UntilEndOfTurn"
            } else if (lower.contains("gains haste.") || lower.contains("gains haste,"))
                && !lower.contains("haste until end of turn")
            {
                "Duration::Indefinitely"
            } else {
                return None;
            }
        } else {
            return None;
        };
        let each: Option<Vec<&str>> = kw_raw.split('&').map(|k| keyword_const(k.trim())).collect();
        let kw = each?.join(".union(") + &")".repeat(kw_raw.split('&').count().saturating_sub(1));
        Some(format!(
            "Effect::PumpTarget {{ power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
             keywords: {kw}, duration: {kw_dur} }}"
        ))
    }

    /// Lower `Attach | Object$ Self | Choices$ <host>` (optionally with
    /// `RememberAttached$ True` and `ConditionDefined$ Remembered` follow-ups like
    /// *Breath of Fury*) onto `Effect::MoveAttachment { attachment: TargetSpec::ThisObject, .. }`.
    pub(super) fn wave28d_attach_move_self_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell || p.peek("Object") != Some("Self") || !p.has("Choices") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Object");
        let rem_attached = match probe.take("RememberAttached").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let (then_if_moved, trailing) = if rem_attached {
            self.collect_move_attachment_then_if_moved(sub.clone())?
        } else {
            (Vec::new(), sub.clone())
        };
        let eff =
            self.parse_move_attachment_step(&mut probe, "TargetSpec::ThisObject", &then_if_moved)?;
        *p = probe;
        *sub = trailing;
        Some(vec![eff])
    }

    /// Lower `ChangeZone(Graveyard -> Battlefield, single target)` followed by
    /// `DB$ Attach | Object$ Targeted | Defined$ Self` (*Auriok Survivors*) via
    /// `Effect::CaptureAffected` + `ForEachRemembered` + `MoveAttachment(EventObject -> This)`
    /// so attachment binds the NEW battlefield incarnation (CR 400.7) rather than the
    /// departed graveyard target.
    pub(super) fn wave28d_reanimate_and_attach_self(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || self.capture_scope.is_some()
            || p.has("RememberChanged")
            || p.has("RememberTargets")
            || p.has("RememberLKI")
            || p.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if s_api != "Attach"
            || sp.take("Object").as_deref() != Some("Targeted")
            || sp.take("Defined").as_deref() != Some("Self")
            || sp.has("ValidTgts")
        {
            return None;
        }
        let optional = match sp.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let after_attach = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let mut probe_p = p.clone();
        let first = self.effect_of_inner(
            "ChangeZone",
            &mut probe_p,
            target,
            LineFlags {
                targets_here: true,
                targets_a_player: false,
                single_target: true,
                prior_target_used: false,
            },
        )?;
        if !probe_p.exhausted() || first.is_empty() {
            return None;
        }
        *p = probe_p;
        *sub = after_attach;
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[{}], then: &[Effect::ForEachRemembered {{ \
             filter: &Filter::Any, effects: &[Effect::MoveAttachment {{ \
             attachment: TargetSpec::EventObject, recipient_filter: &Filter::This, \
             must_change_recipient: false, optional: {optional}, then_if_moved: &[] }}] }}] }}",
            first.join(", ")
        )])
    }

    /// Inside `wave26f_token_found_trigger_chain`, lower `DB$ Effect |
    /// RememberObjects$ Remembered | StaticAbilities$ <S> | Duration$ UntilEndOfCombat`
    /// where `<S>` is `Mode$ MustAttack | ValidCreature$ Card.IsRemembered`
    /// (*Howlsquad Heavy*, *Legion Warboss*) -> `AddTempModifier(AttacksEachCombat)`.
    pub(super) fn wave28d_token_remembered_must_attack(&self, sp: &mut Params) -> Option<String> {
        if sp.take("RememberObjects").as_deref() != Some("Remembered") {
            return None;
        }
        let duration = match sp.take("Duration").as_deref() {
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("EOT" | "UntilEndOfTurn" | "EndOfTurn") => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            _ => return None,
        };
        let static_name = sp.take("StaticAbilities")?;
        if !sp.exhausted() {
            return None;
        }
        let sbody = self.svars.get(static_name.trim())?;
        let (mode, mut mp) = Params::parse(sbody)?;
        mp.drop_prose();
        if mode != "MustAttack"
            || mp.take("ValidCreature").as_deref() != Some("Card.IsRemembered")
            || !mp.exhausted()
        {
            return None;
        }
        Some(format!(
            "Effect::AddTempModifier {{ target: TargetSpec::EventObject, \
             modifier: Modifier::AttacksEachCombat, duration: {duration} }}"
        ))
    }

    /// Inside `wave26f_token_found_trigger_chain`, lower `DB$ RepeatEach |
    /// DefinedCards$ RememberedCard | RepeatSubAbility$ <S>` (with optional
    /// `ClearRemembered$ True`) where `<S>` is `DB$ Attach | Choices$ <F> |
    /// Defined$ Remembered | Optional$ True|False` (*Reckless Crew*).
    pub(super) fn wave28d_token_remembered_repeat_attach(
        &mut self,
        sp: &mut Params,
    ) -> Option<(String, bool)> {
        if sp.take("DefinedCards").as_deref() != Some("RememberedCard") {
            return None;
        }
        let did_clear = match sp.take("ClearRemembered").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let rep_name = sp.take("RepeatSubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        let rep_body = self.svars.get(rep_name.trim())?.clone();
        let (r_api, mut rp) = Params::parse(&rep_body)?;
        rp.drop_prose();
        if r_api != "Attach"
            || rp.take("Defined").as_deref() != Some("Remembered")
            || rp.has("SubAbility")
            || rp.has("ValidTgts")
        {
            return None;
        }
        let optional = match rp.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let choices = rp.take("Choices")?;
        if choices.contains("Remembered")
            || choices.contains("Imprinted")
            || choices.contains("Targeted")
            || !rp.exhausted()
        {
            return None;
        }
        let filter_expr = self.filter_expr(&choices)?;
        let filter_name = self.body.filter_static("ATTACH_CHOSEN", &filter_expr);
        Some((
            format!(
                "Effect::AttachChosen {{ filter: &{filter_name}, to: TargetSpec::EventObject, \
                 max: Amount::Fixed(1), optional: {optional} }}"
            ),
            did_clear,
        ))
    }

    /// Inside `CaptureAffected::then`, lower `DB$ Play` operating on remembered cards
    /// in exile (*Etali, Primal Storm*, *Villainous Wealth*, *Epic Experiment*,
    /// *Collected Conjuring*, *Hellcarver Demon*, *Muse Vortex*, *Kotis, the Fangkeeper*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28d_capture_play_step(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        if flags.targets_here || p.has("ValidTgts") {
            return None;
        }
        let is_rem_valid = p
            .peek("Valid")
            .is_some_and(|v| v.contains("IsRemembered") && !v.contains("!IsRemembered"));
        let is_rem_defined = p.peek("Defined") == Some("Remembered");
        if !is_rem_valid && !is_rem_defined {
            return None;
        }
        let mut probe = p.clone();
        probe.take("IsCurse");
        if let Some(fp) = probe.take("ForgetPlayed") {
            if fp != "True" {
                return None;
            }
            // `Effect::Play` does not remove cast objects from `remembered_objects`;
            // downstream `Origin$ Exile` sweeps check `Filter::InZone(ZoneRef::Exile)`
            // (*Muse Vortex*), whereas `ConditionDefined$ Remembered` /
            // `Remembered$Amount` / `Count$RememberedSize` (*Vaan, Street Thief*)
            // would still see the played card and misfire "If you don't".
            if self.svars.values().any(|body| {
                body.contains("ConditionDefined$ Remembered")
                    || body.contains("Remembered$Amount")
                    || body.contains("Count$RememberedSize")
            }) {
                return None;
            }
        }
        let zone_raw = probe
            .take("ValidZone")
            .unwrap_or_else(|| "Exile".to_string());
        if zone_raw != "Exile" {
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
        let optional = true;
        let count = match probe.take("Amount").as_deref().map(str::trim) {
            None | Some("1") => "PlayCount::One".to_string(),
            Some("All") => "PlayCount::All".to_string(),
            Some(raw) => {
                let n: u8 = self.plain_num(raw)?.try_into().ok().filter(|&n| n >= 2)?;
                format!("PlayCount::UpTo({n})")
            }
        };
        let controller = self.wave11_player_of_line(
            probe.take("Controller").as_deref(),
            target,
            flags.targets_a_player,
        )?;
        if !Self::is_single_player_rel(controller) {
            return None;
        }
        let (cast_only, sa_owner, sa_filter) = match probe.take("ValidSA") {
            None => (copy || exile_instead_of_graveyard, None, None),
            Some(raw_sa) => {
                let (owner, filter) = self.parse_play_sa_filter(&raw_sa)?;
                (true, owner, filter)
            }
        };
        let mut clauses = vec!["Filter::IsRemembered".to_string()];
        let valid_owner = if is_rem_defined {
            probe.take("Defined");
            if probe.has("Valid") {
                return None;
            }
            None
        } else {
            let valid_raw = probe.take("Valid")?;
            let stripped = Self::strip_is_remembered_all_alts(&valid_raw)?;
            let (owner, cleaned) = Self::strip_graveyard_owner(&stripped)?;
            if cleaned != "Card" {
                let expr = self.filter_expr_in_zone(&cleaned, true)?;
                if expr != "Filter::Any" {
                    clauses.push(expr);
                }
            }
            owner
        };
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        if let Some(sa) = sa_filter
            && sa != "Filter::Any"
            && !clauses.contains(&sa)
        {
            clauses.push(sa);
        }
        let owner = match (valid_owner, sa_owner) {
            (Some(a), Some(b)) if a != b => return None,
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) if scope.you_own_only => "PlayerRel::You",
            (None, None) => "PlayerRel::EachPlayer",
        };
        let combined = format!("Filter::And(&[{}])", clauses.join(", "));
        if !self.has_x
            && (combined.contains("Filter::CmcAtMostX") || combined.contains("Filter::CmcExactlyX"))
        {
            return None;
        }
        let filter_name = self.body.filter_static("PLAY", &combined);
        *p = probe;
        Some(vec![format!(
            "Effect::Play {{ source: PlaySource::Zone {{ zone: ZoneRef::Exile, owner: {owner} }}, \
             filter: &{filter_name}, controller: {controller}, count: {count}, cost: {cost}, \
             cast_only: {cast_only}, copy: {copy}, \
             exile_instead_of_graveyard: {exile_instead_of_graveyard}, optional: {optional} }}"
        )])
    }

    /// Inside `CaptureAffected::then`, lower `ChangeZoneAll | Origin$ Exile |
    /// ChangeType$ <F>.IsRemembered` to `Graveyard`, `Hand`, or `Library (-1, RandomOrder)`
    /// (with optional `ForgetChanged$ True`) via `Effect::ChooseCardEx { zone: Remembered,
    /// filter: InZone(Exile) & <F>, not_chosen_movement }` (*Epic Experiment*,
    /// *Collected Conjuring*, *Muse Vortex*).
    pub(super) fn wave28d_capture_change_zone_all_exile(
        &mut self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        self.capture_scope.as_ref()?;
        if p.peek("Origin") != Some("Exile") || p.has("Defined") {
            return None;
        }
        let dest = p.peek("Destination")?;
        if !matches!(dest, "Graveyard" | "Hand" | "Library") {
            return None;
        }
        let ct = p.peek("ChangeType")?;
        if !ct.contains("IsRemembered") || ct.contains("!IsRemembered") {
            return None;
        }
        let stripped = Self::strip_is_remembered_all_alts(ct)?;
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("ChangeType");
        probe.take("IsCurse");
        if let Some(fc) = probe.take("ForgetChanged")
            && fc != "True"
        {
            return None;
        }
        let mv = match dest {
            "Graveyard" => "ChooseMove::Graveyard",
            "Hand" => "ChooseMove::Hand",
            "Library"
                if probe.take("LibraryPosition").as_deref() == Some("-1")
                    && probe.take("RandomOrder").as_deref() == Some("True") =>
            {
                "ChooseMove::BottomOfLibraryRandom"
            }
            _ => return None,
        };
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let expr = if stripped == "Card" {
            "Filter::InZone(ZoneRef::Exile)".to_string()
        } else {
            let base = self.filter_expr_in_zone(&stripped, true)?;
            if base == "Filter::Any" {
                "Filter::InZone(ZoneRef::Exile)".to_string()
            } else {
                format!("Filter::And(&[Filter::InZone(ZoneRef::Exile), {base}])")
            }
        };
        let filter_name = self.body.filter_static("REMEMBERED", &expr);
        *p = probe;
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: &{filter_name}, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: {mv}, \
             then: &[], for_unchosen: &[] }}"
        )])
    }

    /// True when `ChangeNum$ <var>` on an optional `ChangeZone | Hidden$ True` inside
    /// `CaptureAffected::then` names `Count$Valid<Origin> <same ChangeType>`
    /// (*Xenagos, the Reveler* ultimate -> "any number", `Amount::Fixed(255)`).
    pub(super) fn wave28d_capture_hidden_all_change_num(
        &self,
        max_raw: &str,
        origin: &str,
        change_type: &str,
    ) -> bool {
        let Some(def) = self.svars.get(max_raw.trim()).map(|s| s.trim()) else {
            return false;
        };
        let expected_prefix = match origin {
            "Exile" => "Count$ValidExile ",
            "Graveyard" => "Count$ValidGraveyard ",
            "Hand" => "Count$ValidHand ",
            _ => return false,
        };
        def.strip_prefix(expected_prefix)
            .is_some_and(|rest| rest.trim() == change_type.trim())
    }

    /// Inside `CaptureAffected::then`, lower `Proliferate | Amount$ Number$K/Minus.<Y>`
    /// where `1 <= K <= 4` and `<Y>` resolves to `Count$RememberedSize` / `Remembered$Amount`
    /// (*Expand the Sphere*) into `K` conditional `Effect::Proliferate` steps.
    pub(super) fn wave28d_proliferate_minus_remembered(&self, raw: &str) -> Option<Vec<String>> {
        self.capture_scope.as_ref()?;
        let rest = raw.trim().strip_prefix("Number$")?;
        let (k_raw, var) = rest.split_once("/Minus.")?;
        let k: u32 = k_raw
            .trim()
            .parse()
            .ok()
            .filter(|&n| (1..=4).contains(&n))?;
        let def = self.svars.get(var.trim())?.trim();
        if !matches!(def, "Count$RememberedSize" | "Remembered$Amount") {
            return None;
        }
        let mut out = Vec::with_capacity(k as usize);
        for i in 0..k {
            let threshold = k - 1 - i;
            out.push(format!(
                "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::RememberedCount, op: AmountCmp::AtMost, \
                 right: &Amount::Fixed({threshold}) }}, \
                 then: &[Effect::Proliferate], otherwise: &[] }}"
            ));
        }
        Some(out)
    }

    /// Lower single-card public reveal `PeekAndReveal | PeekAmount$ 1 |
    /// RevealValid$ Card | RememberRevealed$ True` -> `DB$ Play | Defined$ Remembered |
    /// ValidSA$ Spell | WithoutManaCost$ True | Optional$ True | ForgetRemembered$ True |
    /// ConditionDefined$ Remembered | ConditionPresent$ <F>` -> `DB$ ChangeZone |
    /// Defined$ Remembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1 |
    /// ForgetChanged$ True` (*Descendants' Path*).
    pub(super) fn wave28d_peek_reveal_one_play_or_bottom(
        &mut self,
        sub_name: &str,
    ) -> Option<(String, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Play"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
            || sp.take("Optional").as_deref() != Some("True")
            || !matches!(sp.take("Controller").as_deref(), None | Some("You"))
            || sp.take("ConditionDefined").as_deref() != Some("Remembered")
        {
            return None;
        }
        if let Some(fr) = sp.take("ForgetRemembered")
            && fr != "True"
        {
            return None;
        }
        if let Some(fp) = sp.take("ForgetPlayed")
            && fp != "True"
        {
            return None;
        }
        if let Some(cmp) = sp.take("ConditionCompare")
            && !matches!(cmp.as_str(), "GE1" | "EQ1")
        {
            return None;
        }
        let cond_present = sp.take("ConditionPresent")?;
        let valid_sa = sp.take("ValidSA")?;
        let (sa_owner, sa_filter) = self.parse_play_sa_filter(&valid_sa)?;
        if sa_owner.is_some() {
            return None;
        }
        let rest_sub = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        // Step 2: put the card on bottom of library if not cast.
        let rest_body = self.svars.get(rest_sub.trim())?.clone();
        let (r_api, mut rp) = Params::parse(&rest_body)?;
        rp.drop_prose();
        if r_api != "ChangeZone"
            || rp.take("Defined").as_deref() != Some("Remembered")
            || rp.take("Origin").as_deref() != Some("Library")
            || rp.take("Destination").as_deref() != Some("Library")
            || rp.take("LibraryPosition").as_deref() != Some("-1")
        {
            return None;
        }
        let did_forget = match rp.take("ForgetChanged").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let after_rest = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        let trailing = match after_rest {
            None if did_forget || self.on_a_spell => None,
            Some(ref cname) if self.is_wave8_cleanup_svar(cname, false) => None,
            _ => return None,
        };
        let cond_expr = self.filter_expr_in_zone(&cond_present, true)?;
        let mut clauses = vec!["Filter::IsRemembered".to_string()];
        if cond_expr != "Filter::Any" {
            clauses.push(cond_expr);
        }
        if let Some(sa) = sa_filter
            && sa != "Filter::Any"
            && !clauses.contains(&sa)
        {
            clauses.push(sa);
        }
        let filter_expr = format!("Filter::And(&[{}])", clauses.join(", "));
        let filter_name = self.body.filter_static("PLAY", &filter_expr);
        Some((
            format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::RevealTop {{ player: PlayerRel::You, count: Amount::Fixed(1), \
                 filter: &Filter::Any, if_matches: &[], otherwise: &[] }}], \
                 then: &[Effect::Play {{ source: PlaySource::Zone {{ zone: ZoneRef::Library, \
                 owner: PlayerRel::You }}, filter: &{filter_name}, controller: PlayerRel::You, \
                 count: PlayCount::One, cost: PlayCostMode::WithoutPayingManaCost, \
                 cast_only: true, copy: false, exile_instead_of_graveyard: false, \
                 optional: true }}, \
                 Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
                 filter: &Filter::InZone(ZoneRef::Library), min: 0, max: Amount::Fixed(0), \
                 reveal_chosen: false, movement: ChooseMove::None, \
                 not_chosen_movement: ChooseMove::BottomOfLibraryChosenOrder, \
                 then: &[], for_unchosen: &[] }}] }}"
            ),
            trailing,
        ))
    }

    /// Lower `UntapAll | ValidCards$ <F>.TargetedPlayerCtrl` (*Great Oak Guardian*)
    /// via `Effect::RepeatEachObject` (which evaluates `ControlledBy(PlayerRel::Chosen)`
    /// via `matches_in_resolution`) running `Effect::UntapAll { filter: &Filter::Reference }`.
    pub(super) fn wave28d_untap_all_targeted_player(&mut self, valid: &str) -> Option<Vec<String>> {
        let (ctrl_rel, stripped) = self.extract_battlefield_ctrl_atom(valid, true)?;
        if ctrl_rel != "PlayerRel::Chosen" {
            return None;
        }
        let base = self.filter_expr(&stripped)?;
        let combined = if base == "Filter::Any" {
            "Filter::ControlledBy(PlayerRel::Chosen)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::ControlledBy(PlayerRel::Chosen)])")
        };
        let name = self.body.filter_static("UNTAP_EACH", &combined);
        Some(vec![format!(
            "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &{name}, \
             only_found: false, then: &[Effect::UntapAll {{ filter: &Filter::Reference }}] }}"
        )])
    }

    /// Lower `DamageAll | ValidCards$ <F>+ControlledBy TargetedController`
    /// (*Flames of the Raze-Boar*) using `Filter::Not(&Filter::NotControlledByReferenceController)`
    /// because `Effect::DealDamageEach` evaluates via `eval::matches_with_context`
    /// (where `context.reference` is the targeted permanent).
    pub(super) fn wave28d_damage_all_target_controller_filter(
        &self,
        valid: &str,
    ) -> Option<String> {
        if !self.current_object_target {
            return None;
        }
        let mut alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let stripped = Self::strip_filter_attr(alt, "ControlledBy TargetedController")?;
            let base = self.filter_expr(&stripped)?;
            let ctrl = "Filter::Not(&Filter::NotControlledByReferenceController)";
            let combined = if base == "Filter::Any" {
                ctrl.to_string()
            } else if let Some(inner) = base
                .strip_prefix("Filter::And(&[")
                .and_then(|s| s.strip_suffix("])"))
            {
                format!("Filter::And(&[{inner}, {ctrl}])")
            } else {
                format!("Filter::And(&[{base}, {ctrl}])")
            };
            alts.push(Self::named_constant(&combined));
        }
        match alts.len() {
            0 => None,
            1 => Some(alts.remove(0)),
            _ => Some(format!("Filter::Or(&[{}])", alts.join(", "))),
        }
    }

    /// Lower `cmcEQ<var>` on the battlefield when `<var>` is
    /// `Count$Valid <F>$GreatestCardManaCost` and every alternative of `<F>` carries
    /// `TargetedPlayerCtrl` (*Blot Out*, *End of the Hunt*, *Break Under Pressure*).
    pub(super) fn wave28d_greatest_mv_filter_amount(&self, var: &str) -> Option<String> {
        if self.current_single_player_rel != Some("PlayerRel::Chosen") {
            return None;
        }
        let def = self.svars.get(var)?.trim();
        let body = def.strip_prefix("Count$Valid ")?.trim();
        let valid = body.strip_suffix("$GreatestCardManaCost")?.trim();
        if valid.is_empty() || valid.contains('$') {
            return None;
        }
        let clean =
            Self::strip_remembered_ctrl_or_own(valid, "TargetedPlayerCtrl", "TargetedPlayerCtrl")?;
        let base = self.filter_expr(&clean)?;
        let scoped = if base == "Filter::Any" {
            "Filter::ControlledBy(PlayerRel::Chosen)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::ControlledBy(PlayerRel::Chosen)])")
        };
        Some(format!("Amount::GreatestManaValueAmong(&{scoped})"))
    }

    /// Lower single mandatory `Sacrifice` by `PlayerRel::Chosen` whose filter uses
    /// `Amount::GreatestManaValueAmong` (*Break Under Pressure*) via `Effect::ChooseCardEx`
    /// and `Effect::SacrificeEvent`, because `ChooseCardEx` evaluates via
    /// `matches_in_resolution` (resolving `PlayerRel::Chosen` inside the filter).
    pub(super) fn wave28d_greatest_mv_sacrifice(
        &mut self,
        who: &str,
        optional: bool,
        raw_amount: Option<&str>,
        filter_expr: &str,
    ) -> Option<Vec<String>> {
        if who != "PlayerRel::Chosen"
            || optional
            || !matches!(raw_amount.map(str::trim), None | Some("1"))
            || !filter_expr.contains("Amount::GreatestManaValueAmong(")
        {
            return None;
        }
        let scoped =
            format!("Filter::And(&[{filter_expr}, Filter::ControlledBy(PlayerRel::Chosen)])");
        let name = self.body.filter_static("SAC_GREATEST", &scoped);
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::Chosen, \
             zone: ChooseZone::Battlefield, filter: &{name}, min: 1, max: Amount::Fixed(1), \
             reveal_chosen: false, movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::None, then: &[Effect::SacrificeEvent], \
             for_unchosen: &[] }}"
        )])
    }

    /// Lower `DB$ EndOfTurn | Defined$ Targeted | AtEOT$ Sacrifice|Destroy|Exile`
    /// chained after a single-object target (*Slave of Bolas*, *Momo's Heist*) into
    /// `Effect::AtNextEndStep`. For `AtEOT$ Sacrifice`, requires either a preceding
    /// `Modifier::GainControl` in `chain.effects` or a `YouCtrl` target filter so
    /// `"its controller sacrifices it"` is never conflated with `"sacrifice it"`.
    pub(super) fn wave28d_end_of_turn_targeted_delayed(
        &self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        let single_obj = target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain.target_exact.is_none()
            && !chain.target_is_x
            && chain.target_constraint.is_none()
            && chain.target_up_to.is_none_or(|n| n == 1)
            && chain.second_target.is_none();
        if !single_obj || p.peek("Defined") != Some("Targeted") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("IsCurse");
        let what = probe.take("AtEOT")?;
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let tgt = target?;
        let inner = match what.as_str() {
            "Sacrifice"
                if (chain
                    .effects
                    .iter()
                    .any(|e| e.contains("Modifier::GainControl"))
                    || tgt.contains("Filter::YOUR_")
                    || tgt.contains("Filter::ControlledByYou"))
                    && !self
                        .oracle
                        .to_ascii_lowercase()
                        .contains("its controller sacrifices") =>
            {
                "Effect::SacrificeEvent"
            }
            "Destroy" => "Effect::destroy(TargetSpec::EventObject)",
            "Exile" => "Effect::exile(TargetSpec::EventObject)",
            _ => return None,
        };
        *p = probe;
        Some(vec![format!(
            "Effect::AtNextEndStep {{ effects: &[{inner}] }}"
        )])
    }

    fn wave28d_only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }

    // -------------------------------------------------------------------------
    // 11. Wave 28d-p3: `Token` conditions / `PumpKeywords` / `AtEOT` / `RepeatEach`
    //     attached tokens, `Pump` `KWChoice` with prevent/protection modifiers,
    //     `CopyPermanent` target-match copies, `Dig` exile-two-piles-play,
    //     remembered `AlterAttribute`, and `Noetic Scales` bounce sweep.
    // -------------------------------------------------------------------------

    /// Extract supported `ConditionDefined$` / `ConditionCheckSVar$` guards on a
    /// `Token` line (*Ethereal Absolution*, *General's Enforcer*, *Ignis Scientia*,
    /// *Return Upon the Tide*, *Transpose*, *Boneyard Desecrator*,
    /// *Necromancer's Stockpile*, *Saint Traft and Rem Karolus*, *Jenson Carthalion*).
    /// Leaves unrecognized condition keys untouched on `p` so existing `chains.rs`
    /// / `wave27c.rs` wrappers continue to run normally.
    pub(super) fn wave28d_take_token_condition(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<TokenCondWrap> {
        if let Some(def) = p.peek("ConditionDefined").map(str::to_string) {
            let cmp_ok = matches!(p.peek("ConditionCompare"), None | Some("EQ1" | "GE1"));
            if !cmp_ok {
                return Some(TokenCondWrap::None);
            }
            let Some(present) = p.peek("ConditionPresent").map(str::to_string) else {
                return Some(TokenCondWrap::None);
            };
            let wrap = match def.as_str() {
                "Targeted"
                    if target.is_some_and(|t| {
                        t.starts_with("TargetSpec::Object(")
                            || t.starts_with("TargetSpec::CardInGraveyard(")
                    }) =>
                {
                    let is_gy =
                        target.is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard("));
                    if is_gy
                        && (present.contains("inZone")
                            || present.contains("tapped")
                            || present.contains("untapped")
                            || present.contains("attacking")
                            || present.contains("blocking")
                            || present.contains("blocked")
                            || present.contains("Ctrl"))
                    {
                        return Some(TokenCondWrap::None);
                    }
                    let expr = self.filter_expr_in_zone(&present, is_gy)?;
                    let name = self.body.filter_static("COND_TGT", &expr);
                    TokenCondWrap::TargetMatches(name)
                }
                "Self" if present == "Card.wasCastFromYourHand" => TokenCondWrap::Condition(
                    "Condition::All(&[Condition::SourceWasCastByYou, \
                     Condition::SourceMatches(&Filter::CastFromHand)])"
                        .to_string(),
                ),
                "Sacrificed" if self.cost_sacrifices_other => {
                    let expr = self.filter_expr(&present)?;
                    let name = self.body.filter_static("SAC_COST", &expr);
                    TokenCondWrap::Condition(format!("Condition::SacrificedForCost(&{name})"))
                }
                "TriggeredCard" if self.trigger_mode.as_deref() == Some("SpellCast") => {
                    let expr = self.filter_expr_in_zone(&present, true)?;
                    let name = self.body.filter_static("TRIG_CARD", &expr);
                    TokenCondWrap::EventObjectMatches(name)
                }
                _ => TokenCondWrap::None,
            };
            if !matches!(wrap, TokenCondWrap::None) {
                p.take("ConditionDefined");
                p.take("ConditionPresent");
                p.take("ConditionCompare");
                p.take("ConditionDescription");
                return Some(wrap);
            }
            return Some(TokenCondWrap::None);
        }
        if let Some(var) = p.peek("ConditionCheckSVar").map(str::to_string) {
            let cmp = p
                .peek("ConditionSVarCompare")
                .map_or_else(|| "GE1".to_string(), str::to_string);
            if let Some(cond) = self.wave28d_svar_cost_condition(&var, &cmp) {
                p.take("ConditionCheckSVar");
                p.take("ConditionSVarCompare");
                p.take("ConditionDescription");
                return Some(TokenCondWrap::Condition(cond));
            }
            if self
                .svars
                .get(var.trim())
                .map(String::as_str)
                .map(str::trim)
                == Some("Count$ResolvedThisTurn")
                && let Some(n_raw) = cmp.trim().strip_prefix("EQ")
                && let Ok(n) = n_raw.trim().parse::<u32>()
                && (1..=9).contains(&n)
            {
                p.take("ConditionCheckSVar");
                p.take("ConditionSVarCompare");
                p.take("ConditionDescription");
                return Some(TokenCondWrap::ResolvedTimesThisTurn(n));
            }
        }
        Some(TokenCondWrap::None)
    }

    pub(super) fn wave28d_apply_token_condition(
        wrap: TokenCondWrap,
        effects: Vec<String>,
    ) -> Vec<String> {
        if matches!(wrap, TokenCondWrap::None) {
            return effects;
        }
        let joined = effects.join(", ");
        vec![match wrap {
            TokenCondWrap::None => unreachable!(),
            TokenCondWrap::TargetMatches(filter) => {
                format!(
                    "Effect::IfCondition {{ condition: Condition::ObjectMatches {{ \
                     what: &TargetSpec::Object(&Filter::Any), filter: &{filter} }}, \
                     then: &[{joined}], otherwise: &[] }}"
                )
            }
            TokenCondWrap::EventObjectMatches(filter) => {
                format!("Effect::IfEventObjectMatches {{ filter: &{filter}, then: &[{joined}] }}")
            }
            TokenCondWrap::Condition(cond) => {
                format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{joined}], otherwise: &[] }}"
                )
            }
            TokenCondWrap::ResolvedTimesThisTurn(times) => {
                format!("Effect::IfResolvedTimesThisTurn {{ times: {times}, then: &[{joined}] }}")
            }
        }]
    }

    /// Lower `Discarded$Valid <F>` `ConditionCheckSVar$` (`GE1` / `EQ1`) into
    /// `Condition::DiscardedForCost(&<F>)` (*Necromancer's Stockpile*).
    pub(super) fn wave28d_svar_cost_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        if !matches!(cmp.trim(), "GE1" | "EQ1") {
            return None;
        }
        let def = self.svars.get(var.trim())?.trim();
        let valid = def.strip_prefix("Discarded$Valid ")?.trim();
        let expr = self.filter_expr_in_zone(valid, true)?;
        let name = self.body.filter_static("DISC_COST", &expr);
        Some(format!("Condition::DiscardedForCost(&{name})"))
    }

    /// Lower fixed-PT `Token` lines for `You` carrying multi-keyword `PumpKeywords$`
    /// (*Eiganjo Uprising*, *Mardu Monument*), indefinite `PumpKeywords$` + `AtEOT$`
    /// (*Chandra, Acolyte of Flame*), or non-attacking `AtEOT$` (*Elemental Appeal*)
    /// by appending a `RepeatEachObject { only_found: true }` over the created tokens.
    pub(super) fn wave28d_token_pump_or_cleanup(
        &mut self,
        p: &mut Params,
        tapped: bool,
        haste: bool,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        let extra_kw = if let Some(raw_kw) = probe.take("PumpKeywords") {
            let dur = match probe.take("PumpDuration").as_deref() {
                Some("EOT" | "EndOfTurn") => "Duration::UntilEndOfTurn",
                _ => return None,
            };
            Some((Self::pump_keyword_bits(&raw_kw)?, dur))
        } else {
            if probe.has("PumpDuration") {
                return None;
            }
            None
        };
        let at_eot = match probe.take("AtEOT").as_deref() {
            None => None,
            Some("Sacrifice") => Some(("DelayedStep::EndStep", "Effect::SacrificeEvent")),
            Some("Exile") => Some((
                "DelayedStep::EndStep",
                "Effect::exile(TargetSpec::EventObject)",
            )),
            Some("SacrificeCombat") => Some(("DelayedStep::EndOfCombat", "Effect::SacrificeEvent")),
            Some("ExileCombat") => Some((
                "DelayedStep::EndOfCombat",
                "Effect::exile(TargetSpec::EventObject)",
            )),
            Some(_) => return None,
        };
        if extra_kw.is_none() && at_eot.is_none() {
            return None;
        }
        let stem = probe.take("TokenScript")?;
        let raw_amount = probe.take("TokenAmount");
        let mut out = if !tapped && !haste {
            self.create_tokens(&stem, raw_amount.as_deref())?
        } else {
            let tokens = self.tokens?;
            let body = tokens.body_fixed(&stem, self.cats)?;
            let token = format!("&generated_tokens::{}", body.constant);
            let amount = match raw_amount.as_deref() {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self.amount_or_count_ext(raw)?,
            };
            vec![format!(
                "Effect::CreateTokenModified {{ token: {token}, amount: {amount}, \
                 tapped: {tapped}, haste: {haste} }}"
            )]
        };
        let mut per_token = Vec::new();
        if let Some((kw_bits, dur)) = extra_kw {
            per_token.push(format!(
                "Effect::PumpEventObject {{ power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
                 keywords: {kw_bits}, duration: {dur} }}"
            ));
        }
        if let Some((step, act)) = at_eot {
            per_token.push(format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ \
                 step: {step}, whose: None, this_turn_only: false, recurring: false }}, \
                 condition: None, targets: None, remember: TriggerRemember::EventObject, \
                 effects: &[{act}] }}"
            ));
        }
        out.push(format!(
            "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &Filter::Any, \
             only_found: true, then: &[{}] }}",
            per_token.join(", ")
        ));
        *p = probe;
        Some(out)
    }

    /// Format `Pump | KWChoice$` options when one or more choices produce
    /// `Modifier::*` (e.g. *Jodah's Avenger* `Protection from red`) alongside
    /// keyword-bit choices.
    pub(super) fn wave28d_format_kw_choices<F>(
        opts: Vec<(String, Vec<String>)>,
        mk_pump: F,
        modify_variant: Option<&str>,
        has_pt: bool,
        duration: &str,
    ) -> Option<String>
    where
        F: Fn(&str) -> String,
    {
        let mut rendered = Vec::with_capacity(opts.len());
        for (kw, mods) in opts {
            if mods.is_empty() {
                rendered.push(mk_pump(&kw));
            } else {
                let mv = modify_variant?;
                let mut parts = Vec::new();
                if has_pt || kw != "KeywordSet::EMPTY" {
                    parts.push(mk_pump(&kw));
                }
                for m in mods {
                    parts.push(format!(
                        "Effect::{mv} {{ modifier: {m}, duration: {duration} }}"
                    ));
                }
                rendered.push(match parts.len() {
                    0 => return None,
                    1 => parts.remove(0),
                    _ => format!("Effect::Sequence(&[{}])", parts.join(", ")),
                });
            }
        }
        Some(format!(
            "Effect::ChooseOne {{ choices: &[{}] }}",
            rendered.join(", ")
        ))
    }

    /// Lower `SVar` of the form `Targeted$Valid <F>/Plus.<N>` (*Hazel of the Rootbloom*)
    /// into `(filter_static, Amount::Fixed(N+1), Amount::Fixed(N))` for `IfTargetMatchesElse`.
    pub(super) fn wave28d_target_valid_plus_amount(
        &mut self,
        raw: &str,
    ) -> Option<(String, String, String)> {
        let def = self.svars.get(raw.trim())?.trim();
        let rest = def.strip_prefix("Targeted$Valid ")?;
        let (valid, offset_raw) = rest.split_once("/Plus.")?;
        let offset: u32 = offset_raw.trim().parse().ok()?;
        let expr = self.filter_expr(valid.trim())?;
        let name = self.body.filter_static("TGT_MATCH", &expr);
        Some((
            name,
            format!("Amount::Fixed({})", offset + 1),
            format!("Amount::Fixed({offset})"),
        ))
    }

    /// Lower `DB$ Token | AttachedTo$ Remembered` inside `RepeatEach` over
    /// battlefield permanents (*Asinine Antics*, *Twisted Sewer-Witch*) into
    /// `Effect::CreateTokenAttached { host: AttachHost::Spec(TargetSpec::EventObject), .. }`.
    pub(super) fn wave28d_repeat_token_attached(&mut self, sp: &mut Params) -> Option<String> {
        let mut probe = sp.clone();
        probe.take("AttachedTo");
        if !matches!(probe.take("TokenOwner").as_deref(), None | Some("You"))
            || probe.has("TokenPower")
            || probe.has("TokenToughness")
            || probe.has("TokenTapped")
            || probe.has("TokenAttacking")
            || probe.has("PumpKeywords")
            || probe.has("PumpDuration")
            || probe.has("AtEOT")
            || probe.has("RememberTokens")
        {
            return None;
        }
        let stem = probe.take("TokenScript")?;
        let raw_amount = probe.take("TokenAmount");
        if !probe.exhausted() {
            return None;
        }
        let amount = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => {
                let expr = self.amount_or_count_ext(raw)?;
                if expr == "Amount::Fixed(0)" {
                    return None;
                }
                expr
            }
        };
        let tokens = self.tokens?;
        let body = tokens.body_fixed(&stem, self.cats)?;
        let token = format!("&generated_tokens::{}", body.constant);
        *sp = probe;
        Some(format!(
            "Effect::CreateTokenAttached {{ token: {token}, amount: {amount}, \
             host: AttachHost::Spec(TargetSpec::EventObject) }}"
        ))
    }

    /// Lower `Dig(N, All, Exile, RememberChanged)` -> `TwoPiles(Opponent, ChosenPile$ DBPlay)`
    /// (*Brilliant Ultimatum*) into `RevealUntilThen(exile_while_revealing: true)` +
    /// `SeparateIntoTwoPiles(ZoneSel::ExileAll, only_found: true, chosen_pile: &[Effect::Play])`.
    /// Because the spell has no targets, `PlaySource::Subject(TargetSpec::Object(&Filter::Any))`
    /// gathers the exact chosen pile bound in `SubjectContext::found`.
    pub(super) fn wave28d_dig_exile_two_piles_play(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if !self.on_a_spell
            || p.has("ValidTgts")
            || p.peek("ChangeNum") != Some("All")
            || p.peek("DestinationZone") != Some("Exile")
            || p.peek("RememberChanged") != Some("True")
        {
            return None;
        }
        let mut probe = p.clone();
        if !matches!(probe.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let dig_num = probe.take("DigNum")?;
        probe.take("ChangeNum");
        probe.take("DestinationZone");
        probe.take("RememberChanged");
        if !probe.exhausted() {
            return None;
        }
        let two_name = sub.as_deref()?.trim().to_string();
        let two_body = self.svars.get(&two_name)?.clone();
        let (t_api, mut tp) = Params::parse(&two_body)?;
        tp.drop_prose();
        if t_api != "TwoPiles"
            || !matches!(tp.take("Defined").as_deref(), None | Some("You"))
            || tp.take("DefinedCards").as_deref() != Some("Remembered")
            || tp.take("Separator").as_deref() != Some("Opponent")
            || !matches!(tp.take("Chooser").as_deref(), None | Some("You"))
            || tp.has("UnchosenPile")
            || tp.has("FaceDown")
        {
            return None;
        }
        let chosen_name = tp.take("ChosenPile")?;
        let after_two = tp.take("SubAbility");
        if !tp.exhausted() {
            return None;
        }
        let play_body = self.svars.get(chosen_name.trim())?.clone();
        let (p_api, mut pp) = Params::parse(&play_body)?;
        pp.drop_prose();
        if p_api != "Play"
            || pp.take("Defined").as_deref() != Some("Remembered")
            || pp.take("WithoutManaCost").as_deref() != Some("True")
            || pp.take("Optional").as_deref() != Some("True")
            || pp.take("Amount").as_deref() != Some("All")
            || !matches!(pp.take("Controller").as_deref(), None | Some("You"))
            || !matches!(pp.take("ValidZone").as_deref(), None | Some("Exile"))
        {
            return None;
        }
        let (cast_only, play_filter) = match pp.take("ValidSA") {
            None => (false, "&Filter::Any".to_string()),
            Some(raw_sa) => {
                let (owner, filter) = self.parse_play_sa_filter(&raw_sa)?;
                if owner.is_some() {
                    return None;
                }
                let fref = match filter {
                    None => "&Filter::Any".to_string(),
                    Some(expr) => format!("&{}", self.body.filter_static("PLAY", &expr)),
                };
                (true, fref)
            }
        };
        if !pp.exhausted() {
            return None;
        }
        let trailing = self.wave28d_parse_optional_cleanup(after_two, true, false)?;
        let count = self.amount_or_count_ext(&dig_num)?;
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::RevealUntilThen {{ library: PlayerRel::You, filter: &Filter::Any, \
             count: {count}, max_revealed: Some({count}), exile_while_revealing: true, \
             found: RevealUntilFound::Exile, rest: RevealUntilRest::Exile, \
             then: &[Effect::SeparateIntoTwoPiles {{ separator: PlayerRel::Opponent, \
             pile_chooser: PlayerRel::You, zone: ZoneSel::ExileAll, filter: &Filter::Any, \
             only_found: true, chosen_pile: &[Effect::Play {{ \
             source: PlaySource::Subject(TargetSpec::Object(&Filter::Any)), \
             filter: {play_filter}, controller: PlayerRel::You, count: PlayCount::All, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: {cast_only}, copy: false, \
             exile_instead_of_graveyard: false, optional: true }}], unchosen_pile: &[] }}] }}"
        )])
    }

    /// Inside `CaptureAffected::then`, lower `AlterAttribute | Defined$ Remembered |
    /// Attributes$ Suspected` (*Case of the Stashed Skeleton*) into
    /// `Effect::ForEachRemembered { effects: &[Effect::Suspect { target: TargetSpec::EventObject }] }`.
    pub(super) fn wave28d_capture_alter_attribute(p: &mut Params) -> Option<Vec<String>> {
        let mut probe = p.clone();
        probe.take("Defined");
        if probe.take("Attributes").as_deref() != Some("Suspected") {
            return None;
        }
        let unsuspect = match probe.take("Activate").as_deref() {
            None | Some("True") => false,
            Some("False") => true,
            Some(_) => return None,
        };
        let optional = match probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let variant = if unsuspect { "Unsuspect" } else { "Suspect" };
        let inner = if optional {
            format!(
                "Effect::MayDo {{ effects: &[Effect::{variant} {{ target: TargetSpec::EventObject }}] }}"
            )
        } else {
            format!("Effect::{variant} {{ target: TargetSpec::EventObject }}")
        };
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[{inner}] }}"
        )])
    }

    /// Lower `ChangeZoneAll | Origin$ Battlefield | Destination$ Hand |
    /// ChangeType$ Creature.powerGT<X>+ControlledBy TriggeredPlayer` on a `Phase` trigger
    /// where `<X> = TriggeredPlayer$CardsInHand` (*Noetic Scales*) into `Effect::BounceAll`.
    fn wave28d_noetic_scales_bounce_all(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if self.trigger_mode.as_deref() != Some("Phase") || p.has("Defined") {
            return None;
        }
        let ct = p.peek("ChangeType")?;
        let rest = ct.strip_prefix("Creature.powerGT")?;
        let var = rest.strip_suffix("+ControlledBy TriggeredPlayer")?.trim();
        if self.svars.get(var).map(String::as_str).map(str::trim)
            != Some("TriggeredPlayer$CardsInHand")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("ChangeType");
        probe.take("IsCurse");
        if !Self::wave28d_only_condition_keys_left(&probe) {
            return None;
        }
        let name = self.body.filter_static(
            "BOUNCE",
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByActivePlayer, \
             Filter::Not(&Filter::PowerAtMostAmount(&Amount::HandSize(PlayerRel::ActivePlayer)))])",
        );
        *p = probe;
        Some(vec![format!("Effect::BounceAll {{ filter: &{name} }}")])
    }
}

/// Conditional wrapper extracted from a `Token` line in Wave 28d-p3.
pub(super) enum TokenCondWrap {
    None,
    TargetMatches(String),
    EventObjectMatches(String),
    Condition(String),
    ResolvedTimesThisTurn(u32),
}
