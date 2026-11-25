//! Wave 28d — pure `scriptgen` lowerings for `ChangeZoneAll`, `DigUntil`,
//! `PeekAndReveal`, `Discard`, `Mill`, `RememberSacrificed`, `CopyPermanent`,
//! `Pump`, and targeted/loop filter atoms backed by existing `baylee-cards-dsl`
//! & `baylee-engine` primitives.

use super::{Chain, Params, Tx, chains::LineFlags, wave27c::CaptureScope};

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
        } else if optional {
            (0, self.amount_or_count_ext(raw_num.as_deref()?)?)
        } else {
            return None;
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
                format!("Effect::PlayerMayDo {{ who: {who}, effects: &[{inner}] }}")
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
        let n = self.amount_or_count_ext(&p.take("NumCards")?)?;
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
            format!("Effect::PlayerMayDo {{ who: {who}, effects: &[{mill}] }}")
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
        let origin = p.peek("Origin")?;
        let dest = p.peek("Destination")?;
        if !matches!(
            (origin, dest),
            ("Graveyard", "Exile" | "Hand")
                | ("Hand", "Exile" | "Graveyard" | "Battlefield")
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

        let (movement, prepend_reveal) = match (origin, dest) {
            ("Graveyard", "Exile") => ("ChooseMove::Exile".to_string(), false),
            ("Graveyard", "Hand") => ("ChooseMove::Hand".to_string(), false),
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
        let choose_zone = match origin {
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

    /// Lowers `DigUntil | FoundDestination$ Exile | RevealedDestination$ Exile |
    /// RememberFound$ True | RememberRevealed$ True` -> `Play | Defined$ Remembered |
    /// WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True` ->
    /// `ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile |
    /// Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True` -> `Cleanup`.
    ///
    /// Wraps `RevealUntilThen(exile_while_revealing: true, found: Exile, rest: Exile, then: &[Play])`
    /// in `Effect::CaptureAffected`, then runs `ChooseCardEx(Remembered, InZone(Exile),
    /// BottomOfLibraryRandom)` so all exiled cards that weren't cast (both non-found cards AND
    /// the found card if declined) are shuffled together onto the bottom of that library.
    pub(super) fn wave28d_dig_until_exile_play_bottom(
        &mut self,
        sub_name: &str,
        library: &str,
        filter_ref: &str,
        count: &str,
        max_revealed: Option<&str>,
    ) -> Option<(Vec<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Play"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("ForgetPlayed").as_deref() != Some("True")
            || sp.take("Optional").as_deref() != Some("True")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
        {
            return None;
        }
        // Optional redundant markers on Dazzling Sphinx / Gríma.
        if let Some(vz) = sp.take("ValidZone")
            && vz != "Exile"
        {
            return None;
        }
        if let Some(rem) = sp.take("RememberObjects")
            && rem != "Remembered"
        {
            return None;
        }
        let valid_sa = sp.take("ValidSA");
        let valid_card = sp.take("Valid");
        let play_filter =
            self.wave28d_dig_play_filter(valid_sa.as_deref(), valid_card.as_deref())?;
        let rest_sub = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }

        // Next step MUST be `ChangeZoneAll | Origin$ Exile | Destination$ Library |
        // LibraryPosition$ -1 | RandomOrder$ True | ChangeType$ Card.IsRemembered`.
        let rest_body = self.svars.get(rest_sub.trim())?.clone();
        let (r_api, mut rp) = Params::parse(&rest_body)?;
        rp.drop_prose();
        if r_api != "ChangeZoneAll"
            || rp.take("Origin").as_deref() != Some("Exile")
            || rp.take("Destination").as_deref() != Some("Library")
            || rp.take("LibraryPosition").as_deref() != Some("-1")
            || rp.take("RandomOrder").as_deref() != Some("True")
            || rp.take("ChangeType").as_deref() != Some("Card.IsRemembered")
        {
            return None;
        }
        let after_rest = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        let trailing = match after_rest {
            None => {
                if !self.on_a_spell {
                    return None;
                }
                None
            }
            Some(ref cname) => {
                let cbody = self.svars.get(cname.trim())?.clone();
                let (c_api, mut cp) = Params::parse(&cbody)?;
                cp.drop_prose();
                if c_api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
                    return None;
                }
                let next = cp.take("SubAbility");
                if !cp.exhausted() {
                    return None;
                }
                next
            }
        };

        let max_rev_expr = match max_revealed {
            None => "None".to_string(),
            Some(m) => format!("Some({m})"),
        };
        let play_eff = format!(
            "Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: {play_filter}, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, optional: true }}"
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

    fn wave28d_dig_play_filter(
        &mut self,
        valid_sa: Option<&str>,
        valid_card: Option<&str>,
    ) -> Option<String> {
        let mut clauses: Vec<String> = Vec::new();
        if let Some(sa) = valid_sa {
            let expr = match sa.trim() {
                "Spell" => "Filter::NONLAND".to_string(),
                "Spell.Instant,Spell.Sorcery" | "Spell.Sorcery,Spell.Instant" => {
                    "Filter::INSTANT_OR_SORCERY".to_string()
                }
                "Spell.Instant" => "Filter::INSTANT".to_string(),
                "Spell.Sorcery" => "Filter::SORCERY".to_string(),
                _ => return None,
            };
            clauses.push(expr);
        }
        if let Some(vc) = valid_card {
            let stripped = Self::strip_is_remembered_all_alts(vc)?;
            let expr = self.filter_expr_in_zone(&stripped, true)?;
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
        if has_sub || p.peek("RememberTokens") != Some("True") {
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
        if qual.is_empty() || !target.starts_with("TargetSpec::Object(") {
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
        if card_zone || !self.current_object_target {
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

    fn wave28d_only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }
}
