//! Semantic domain lowerings for `Draw`, `Discard`, `RevealHand`, `PeekAndReveal`,
//! and `ChooseCard` scripts that compose existing `baylee-cards-dsl` and
//! `baylee-engine` primitives cleanly.

use std::collections::BTreeSet;

use super::{Chain, Params, Tx, chains::LineFlags, mana_color_const};

/// Outcome of inspecting an optional trailing `SubAbility$` step.
enum TrailingSub {
    /// No further `SubAbility$` remains (or only a trailing `Cleanup` was consumed).
    Done,
    /// Continue chaining from the named `SVar`.
    Next(String),
}

impl TrailingSub {
    fn from_opt(sub: Option<String>) -> Self {
        match sub {
            Some(s) => Self::Next(s),
            None => Self::Done,
        }
    }

    fn into_opt(self) -> Option<String> {
        match self {
            Self::Done => None,
            Self::Next(s) => Some(s),
        }
    }
}

impl Tx<'_> {
    // -------------------------------------------------------------------------
    // 1. Draw helpers
    // -------------------------------------------------------------------------

    /// Lower a trailing Slot-2 `Draw | Defined$ TargetedPlayer` (or `Targeted`)
    /// line after a 2-slot target chain (e.g. *Cease // Desist*) by wrapping
    /// it in `Effect::WithSecondTarget`.
    pub(super) fn hand_draw_second_target_step(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: Option<&String>,
        chain: &Chain,
        had_second_target: bool,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if !had_second_target
            || targets_here
            || api != "Draw"
            || chain.third_target.is_some()
            || self.later_line_targets(sub.map(String::as_str))
        {
            return None;
        }
        let second = chain.second_target.as_deref()?;
        if !matches!(second, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            && !second.starts_with("TargetSpec::Player(")
        {
            return None;
        }
        if !matches!(
            p.peek("Defined"),
            None | Some("Targeted" | "TargetedPlayer")
        ) {
            return None;
        }
        let second_spec = Self::effect_target_of(second);
        let mut probe = p.clone();
        probe.take("Defined");
        probe
            .entries
            .push(("Defined".to_string(), "Targeted".to_string()));
        let inner = self.effect_of_inner(
            "Draw",
            &mut probe,
            Some(&second_spec),
            LineFlags {
                targets_here: true,
                targets_a_player: true,
                single_target: true,
                prior_target_used: false,
            },
        )?;
        if !probe.exhausted() || inner.is_empty() {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::WithSecondTarget(&[{}])",
            inner.join(", ")
        )])
    }

    // -------------------------------------------------------------------------
    // 2. Discard (`Mode$ RevealYouChoose` with `RevealNumber$`)
    // -------------------------------------------------------------------------

    /// Lower `Discard | Mode$ RevealYouChoose | RevealNumber$ <R>` (e.g.
    /// *Blackmail*, *Noggin Whack*, *Acquisitions Expert*, *Hollow Specter*,
    /// *Mire's Toll*, *Disciple of Phenax*):
    /// `CaptureAffected([RevealFromHand(Filter::Any, min: u8::MAX, max: R)],
    ///   [ChooseCardEx(Remembered, filter, min, max, None -> DiscardDefined)])`.
    pub(super) fn discard_reveal_subset_you_choose(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Mode") != Some("RevealYouChoose") || !p.has("RevealNumber") || p.has("Cost") {
            return None;
        }
        let oracle_lower = self.oracle.to_ascii_lowercase();
        if oracle_lower.contains("look at") && !oracle_lower.contains("reveal") {
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
        if !Self::is_single_seat_rel(who) {
            return None;
        }
        let optional = match probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_reveal = probe.take("RevealNumber")?;
        let reveal_amt = self.amount_or_count_ext(&raw_reveal)?;
        let raw_num = probe.take("NumCards");
        let valid = probe.take("DiscardValid");
        if probe.take("DiscardValidDesc").is_some()
            && !matches!(valid.as_deref(), None | Some("Card"))
        {
            return None;
        }
        if !Self::discard_only_condition_keys_left(&probe) {
            return None;
        }
        let filter_ref = match valid.as_deref() {
            None | Some("Card") => "&Filter::Any".to_string(),
            Some(v) => {
                let expr = self.filter_expr_in_zone(v, true)?;
                format!("&{}", self.body.filter_static("DISCARD", &expr))
            }
        };
        let plain_num = match raw_num.as_deref() {
            None => Some(1u8),
            Some(r) => self.plain_num(r).and_then(|n| u8::try_from(n).ok()),
        };
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
        *p = probe;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::RevealFromHand {{ who: {who}, filter: &Filter::Any, min: u8::MAX, max: {reveal_amt}, if_revealed: &[] }}], \
             then: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
             filter: {filter_ref}, min: {min}, max: {max_expr}, reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], for_unchosen: &[] }}] }}"
        )])
    }

    // -------------------------------------------------------------------------
    // 3. RevealHand remembered / imprint chains
    // -------------------------------------------------------------------------

    /// Chain handler for `RevealHand | RememberRevealed$ True` (or
    /// `ImprintRevealed$ True`) followed by:
    /// - `DealDamage` counting `Remembered$Valid <F>` (*Inquisition*);
    /// - `ChooseCard` from `Hand` discarding (+ optional draw gated on `ChosenCard`,
    ///   e.g. *Concealing Curtains // Revealing Eye*);
    /// - `ChooseCard` from `Hand` exiling with optional `ChosenCard` filter gate
    ///   exiling a card from your hand (*Specter's Shriek*);
    /// - `ChooseCard` -> `Bookkeeping(Imprint)` -> `GainLife(Imprinted$CardToughness)`
    ///   -> `Discard(Defined$ Imprinted)` (*Talara's Bane*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn reveal_hand_remembered_chain(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if !(p.has("RememberRevealed") || p.has("ImprintRevealed")) {
            return None;
        }
        let mut reveal = p.clone();
        reveal.take("IsCurse");
        if reveal.peek("Look") == Some("True") || reveal.has("Optional") {
            return None;
        }
        let remembered = match reveal.take("RememberRevealed").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let imprinted = match reveal.take("ImprintRevealed").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !remembered && !imprinted {
            return None;
        }
        let _remember_targets = match reveal.take("RememberTargets").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let lower = self.oracle.to_ascii_lowercase();
        if !(lower.contains("reveals their hand")
            || lower.contains("reveal your hand")
            || lower.contains("reveals his or her hand")
            || lower.contains("reveal their hands")
            || lower.contains("reveal their hand"))
            || lower.contains("look at")
        {
            return None;
        }
        let who = self.wave11_player_of_line(
            reveal.take("Defined").as_deref(),
            target,
            targets_a_player,
        )?;
        if !Self::is_single_seat_rel(who) || !reveal.exhausted() {
            return None;
        }
        let sub_name = sub?.trim();
        let sub_body = self.svars.get(sub_name)?.clone();
        let (sub_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();

        // A. Direct count-based damage (*Inquisition*).
        if sub_api == "DealDamage" && remembered && !imprinted {
            if sp.take("DamageSource").is_some() {
                return None;
            }
            let damaged = self.wave11_player_of_line(
                sp.take("Defined").as_deref(),
                chain.target.as_deref(),
                false,
            )?;
            if damaged != who {
                return None;
            }
            let raw_amount = sp.take("NumDmg")?;
            let trailing = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let definition = self.svars.get(raw_amount.trim())?.trim().to_string();
            let (head, suffix) = definition
                .split_once('/')
                .map_or((definition.as_str(), ""), |(h, s)| (h, s));
            let valid = head.strip_prefix("Remembered$Valid ")?.trim();
            let expr = self.filter_expr_in_zone(valid, true)?;
            let filter = self.body.filter_static("HAND_COUNT", &expr);
            let amount = Self::wrap_amount_suffix(
                format!("Amount::CountOf {{ filter: &{filter}, zone: ZoneSel::HandOf({who}) }}"),
                suffix,
            )?;
            chain
                .effects
                .push(format!("Effect::RevealHand {{ who: {who} }}"));
            chain.effects.push(format!(
                "Effect::DealDamage {{ amount: {amount}, target: TargetSpec::Player({who}) }}"
            ));
            return self.chain_sub_ability(trailing, chain);
        }

        // B. Single `ChooseCard | ChoiceZone$ Hand` from the revealed hand.
        if sub_api != "ChooseCard"
            || sp.take("ChoiceZone").as_deref() != Some("Hand")
            || !matches!(sp.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        sp.take("ChoiceTitle");
        sp.take("ChoiceDesc");
        sp.take("IsCurse");
        let raw_choices = sp.take("Choices")?;
        let marker = if remembered {
            "IsRemembered"
        } else {
            "IsImprinted"
        };
        let stripped_choices = Self::strip_named_filter_atom(&raw_choices, marker)?;
        if stripped_choices.contains("IsRemembered")
            || stripped_choices.contains("IsImprinted")
            || stripped_choices.contains("Remembered")
            || stripped_choices.contains("Targeted")
        {
            return None;
        }
        let filter_ref = if stripped_choices == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&stripped_choices, true)?;
            format!("&{}", self.body.filter_static("CHOOSE", &expr))
        };
        let mandatory = match sp.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_amt = sp.take("Amount").unwrap_or_else(|| "1".to_string());
        let max_num: u8 = self.plain_num(&raw_amt)?.try_into().ok()?;
        if max_num == 0 {
            return None;
        }
        let min_num = match sp.take("MinAmount").as_deref() {
            None => {
                if mandatory {
                    max_num
                } else {
                    0
                }
            }
            Some(raw) => {
                let n: u8 = self.plain_num(raw)?.try_into().ok()?;
                if mandatory && n != max_num {
                    return None;
                }
                n
            }
        };
        let choose_sub = sp.take("SubAbility")?;
        if !sp.exhausted() {
            return None;
        }
        let body2 = self.svars.get(choose_sub.trim())?.clone();
        let (api2, mut p2) = Params::parse(&body2)?;
        p2.drop_prose();

        // B1. Discard chosen card (+ optional ChosenCard-gated draw/etc., e.g. *Revealing Eye*).
        if api2 == "Discard"
            && p2.take("Mode").as_deref() == Some("Defined")
            && p2.take("DefinedCards").as_deref() == Some("ChosenCard")
        {
            let def_player = self.wave11_player_of_line(
                p2.take("Defined").as_deref(),
                chain.target.as_deref(),
                false,
            )?;
            if def_player != who {
                return None;
            }
            if let Some(cond_def) = p2.take("ConditionDefined")
                && (cond_def != "ChosenCard"
                    || !matches!(p2.take("ConditionPresent").as_deref(), None | Some("Card"))
                    || !matches!(
                        p2.take("ConditionCompare").as_deref(),
                        None | Some("EQ1" | "GE1")
                    ))
            {
                return None;
            }
            let after2 = p2.take("SubAbility");
            if !p2.exhausted() {
                return None;
            }
            let mut then_effs =
                vec!["Effect::DiscardDefined { cards: DefinedObjects::Found }".to_string()];
            let trailing = self
                .consume_chosen_card_gated_draw(after2, who, &mut then_effs)?
                .into_opt();
            chain
                .effects
                .push(format!("Effect::RevealHand {{ who: {who} }}"));
            chain.effects.push(format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Hand({who}), \
                 filter: {filter_ref}, min: {min_num}, max: Amount::Fixed({max_num}), \
                 reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[{}], for_unchosen: &[] }}",
                then_effs.join(", ")
            ));
            return self.chain_sub_ability(trailing, chain);
        }

        // B2. Exile chosen card + optional ChosenCard-conditioned self-hand exile (*Specter's Shriek*).
        if api2 == "ChangeZone"
            && p2.take("Origin").as_deref() == Some("Hand")
            && p2.take("Destination").as_deref() == Some("Exile")
            && p2.take("Defined").as_deref() == Some("ChosenCard")
        {
            match p2.take("Mandatory").as_deref() {
                None | Some("True" | "False") => {}
                Some(_) => return None,
            }
            let after2 = p2.take("SubAbility");
            if !p2.exhausted() {
                return None;
            }
            let (cond_exile_then, trailing_tail) =
                self.consume_chosen_exile_hand_followup(after2)?;
            let trailing = trailing_tail.into_opt();
            chain
                .effects
                .push(format!("Effect::RevealHand {{ who: {who} }}"));
            if let Some(then_fx) = cond_exile_then {
                chain.effects.push(format!(
                    "Effect::CaptureAffected {{ \
                     first: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Hand({who}), \
                     filter: {filter_ref}, min: {min_num}, max: Amount::Fixed({max_num}), \
                     reveal_chosen: false, movement: ChooseMove::Exile, not_chosen_movement: ChooseMove::None, \
                     then: &[], for_unchosen: &[] }}], \
                     then: &[{then_fx}] }}"
                ));
            } else {
                chain.effects.push(format!(
                    "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Hand({who}), \
                     filter: {filter_ref}, min: {min_num}, max: Amount::Fixed({max_num}), \
                     reveal_chosen: false, movement: ChooseMove::Exile, not_chosen_movement: ChooseMove::None, \
                     then: &[], for_unchosen: &[] }}"
                ));
            }
            return self.chain_sub_ability(trailing, chain);
        }

        // B3. `Bookkeeping(Imprint)` -> `GainLife(Imprinted$CardToughness)` -> `Discard(Imprinted)` (*Talara's Bane*).
        if api2 == "Bookkeeping"
            && max_num == 1
            && mandatory
            && p2.take("Defined").as_deref() == Some("ChosenCard")
            && p2.take("ImprintCards").as_deref() == Some("ChosenCard")
        {
            let after2 = p2.take("SubAbility")?;
            if !p2.exhausted() {
                return None;
            }
            let trailing = self
                .consume_talara_imprint_gain_discard(&after2, who)?
                .into_opt();
            chain
                .effects
                .push(format!("Effect::RevealHand {{ who: {who} }}"));
            chain.effects.push(format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::Hand({who}), \
                 filter: {filter_ref}, min: 1, max: Amount::Fixed(1), \
                 reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[], for_unchosen: &[] }}], \
                 then: &[Effect::ForEachRemembered {{ effects: &[ \
                 Effect::GainLife {{ amount: Amount::RememberedToughness }}, \
                 Effect::DiscardDefined {{ cards: DefinedObjects::Spec(TargetSpec::EventObject) }}] }}] }}"
            ));
            return self.chain_sub_ability(trailing, chain);
        }

        None
    }

    fn consume_chosen_card_gated_draw(
        &mut self,
        after: Option<String>,
        who: &str,
        then_effs: &mut Vec<String>,
    ) -> Option<TrailingSub> {
        let Some(name) = after else {
            return Some(TrailingSub::Done);
        };
        let trimmed = name.trim();
        if self.is_reveal_hand_cleanup_svar(trimmed) {
            return Some(TrailingSub::Done);
        }
        let body = self.svars.get(trimmed)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api == "Draw" && sp.peek("ConditionDefined") == Some("ChosenCard") {
            sp.take("ConditionDefined");
            if !matches!(sp.take("ConditionPresent").as_deref(), None | Some("Card"))
                || !matches!(
                    sp.take("ConditionCompare").as_deref(),
                    None | Some("EQ1" | "GE1")
                )
            {
                return None;
            }
            let next = sp.take("SubAbility");
            let drawer = self.wave11_player_of_line(
                sp.take("Defined").as_deref(),
                Some(&format!("TargetSpec::Player({who})")),
                who == "PlayerRel::Chosen",
            )?;
            let amt = self.amount_or_count_ext(sp.take("NumCards").as_deref().unwrap_or("1"))?;
            if !sp.exhausted() {
                return None;
            }
            let draw_fx = if drawer == "PlayerRel::You" {
                format!("Effect::DrawCards {{ amount: {amt} }}")
            } else {
                format!("Effect::DrawCardsFor {{ amount: {amt}, who: {drawer} }}")
            };
            then_effs.push(draw_fx);
            return Some(TrailingSub::from_opt(
                self.strip_reveal_hand_cleanup_tail(next),
            ));
        }
        Some(TrailingSub::Next(trimmed.to_string()))
    }

    fn consume_chosen_exile_hand_followup(
        &mut self,
        after: Option<String>,
    ) -> Option<(Option<String>, TrailingSub)> {
        let Some(name) = after else {
            return Some((None, TrailingSub::Done));
        };
        let trimmed = name.trim();
        if self.is_reveal_hand_cleanup_svar(trimmed) {
            return Some((None, TrailingSub::Done));
        }
        let body = self.svars.get(trimmed)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api == "ChangeZone" && sp.peek("ConditionDefined") == Some("ChosenCard") {
            sp.take("ConditionDefined");
            let present = sp.take("ConditionPresent")?;
            if !matches!(
                sp.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            ) {
                return None;
            }
            if sp.take("Origin").as_deref() != Some("Hand")
                || sp.take("Destination").as_deref() != Some("Exile")
                || !matches!(sp.take("Defined").as_deref(), None | Some("You"))
                || !matches!(sp.take("Mandatory").as_deref(), None | Some("True"))
                || sp.take("ChangeType").as_deref() != Some("Card")
            {
                return None;
            }
            let next = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let cond_expr = self.filter_expr_in_zone(&present, true)?;
            let cond_filter = self.body.filter_static(
                "EXILED",
                &format!("Filter::And(&[Filter::InZone(ZoneRef::Exile), {cond_expr}])"),
            );
            let then_fx = format!(
                "Effect::ForEachRemembered {{ effects: &[Effect::IfEventObjectMatches {{ \
                 filter: &{cond_filter}, \
                 then: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
                 zone: ChooseZone::Hand(PlayerRel::You), filter: &Filter::Any, min: 1, \
                 max: Amount::Fixed(1), reveal_chosen: false, movement: ChooseMove::Exile, \
                 not_chosen_movement: ChooseMove::None, then: &[], for_unchosen: &[] }}] }}] }}"
            );
            return Some((
                Some(then_fx),
                TrailingSub::from_opt(self.strip_reveal_hand_cleanup_tail(next)),
            ));
        }
        Some((None, TrailingSub::Next(trimmed.to_string())))
    }

    fn consume_talara_imprint_gain_discard(
        &mut self,
        start: &str,
        who: &str,
    ) -> Option<TrailingSub> {
        let body3 = self.svars.get(start.trim())?.clone();
        let (api3, mut p3) = Params::parse(&body3)?;
        p3.drop_prose();
        if api3 != "GainLife"
            || !matches!(p3.take("Defined").as_deref(), None | Some("You"))
            || p3.take("ConditionDefined").as_deref() != Some("Imprinted")
            || !matches!(p3.take("ConditionPresent").as_deref(), None | Some("Card"))
            || !matches!(
                p3.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            )
        {
            return None;
        }
        let life_raw = p3.take("LifeAmount")?;
        if self.svars.get(life_raw.trim()).map(String::as_str) != Some("Imprinted$CardToughness") {
            return None;
        }
        let after3 = p3.take("SubAbility")?;
        if !p3.exhausted() {
            return None;
        }
        let body4 = self.svars.get(after3.trim())?.clone();
        let (api4, mut p4) = Params::parse(&body4)?;
        p4.drop_prose();
        if api4 != "Discard"
            || p4.take("Mode").as_deref() != Some("Defined")
            || p4.take("DefinedCards").as_deref() != Some("Imprinted")
            || p4.take("ConditionDefined").as_deref() != Some("Imprinted")
            || !matches!(p4.take("ConditionPresent").as_deref(), None | Some("Card"))
            || !matches!(
                p4.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            )
        {
            return None;
        }
        let discarder = self.wave11_player_of_line(
            p4.take("Defined").as_deref(),
            Some(&format!("TargetSpec::Player({who})")),
            who == "PlayerRel::Chosen",
        )?;
        if discarder != who {
            return None;
        }
        let after4 = p4.take("SubAbility");
        if !p4.exhausted() {
            return None;
        }
        Some(TrailingSub::from_opt(
            self.strip_reveal_hand_cleanup_tail(after4),
        ))
    }

    fn strip_named_filter_atom(raw: &str, expected_atom: &str) -> Option<String> {
        let mut alts = Vec::new();
        for alt in raw.split(',') {
            let trimmed = alt.trim();
            let (noun, attrs) = trimmed.split_once('.')?;
            let mut kept = Vec::new();
            let mut found = false;
            for atom in attrs.split('+').map(str::trim) {
                if atom == expected_atom {
                    found = true;
                } else if !atom.is_empty() {
                    kept.push(atom);
                }
            }
            if !found {
                return None;
            }
            if kept.is_empty() {
                alts.push(noun.trim().to_string());
            } else {
                alts.push(format!("{}.{}", noun.trim(), kept.join("+")));
            }
        }
        Some(alts.join(","))
    }

    fn is_reveal_hand_cleanup_svar(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        if api != "Cleanup" {
            return false;
        }
        p.drop_prose();
        for key in ["ClearRemembered", "ClearChosenCard", "ClearImprinted"] {
            if let Some(v) = p.take(key)
                && v != "True"
            {
                return false;
            }
        }
        !p.has("SubAbility") && p.exhausted()
    }

    fn strip_reveal_hand_cleanup_tail(&self, sub: Option<String>) -> Option<String> {
        match sub {
            Some(name) if self.is_reveal_hand_cleanup_svar(&name) => None,
            other => other,
        }
    }

    // -------------------------------------------------------------------------
    // 4. PeekAndReveal helpers (Parley Mana, Prophecy, Cerebral Eruption,
    //    Thran Tome, Getaway Barrel)
    // -------------------------------------------------------------------------

    /// Parley `DB$ Mana | Produced$ <C> | Amount$ <Remembered$Valid Card.nonLand>`.
    pub(super) fn parley_mana_step(&mut self, sp: &mut Params, reveal_who: &str) -> Option<String> {
        let produced = sp.take("Produced")?;
        let color = mana_color_const(&produced)?;
        let raw_amt = sp.take("Amount")?;
        let amt = self.parley_valid_amount(&raw_amt, reveal_who)?;
        if !matches!(sp.take("Defined").as_deref(), None | Some("You")) || !sp.exhausted() {
            return None;
        }
        Some(format!(
            "Effect::AddManaAmountFor {{ who: PlayerRel::You, color: {color}, amount: {amt} }}"
        ))
    }

    /// True if `start_svar` or any step in its `SubAbility$` chain mentions
    /// `Remembered` / `Imprinted` / `ChosenCard` state.
    pub(super) fn svar_chain_references_remembered(&self, start_svar: &str) -> bool {
        let mut cursor = Some(start_svar.trim().to_string());
        let mut seen = BTreeSet::new();
        while let Some(name) = cursor.take() {
            if !seen.insert(name.clone()) {
                return true;
            }
            if self.is_reveal_hand_cleanup_svar(&name) {
                return false;
            }
            let Some(body) = self.svars.get(&name) else {
                return true;
            };
            if body.contains("Remembered")
                || body.contains("Imprinted")
                || body.contains("ChosenCard")
            {
                return true;
            }
            let Some((_, sp)) = Params::parse(body) else {
                return true;
            };
            cursor = sp.peek("SubAbility").map(|s| s.trim().to_string());
        }
        false
    }

    /// Public `PeekAndReveal` chains backed by `CaptureAffected([RevealTop])`:
    /// 1. *Cerebral Eruption* (`DamageAll` with `Remembered$CardManaCost` +
    ///    optional `ChangeZone(Parent, Stack -> Hand)` on `Remembered` filter).
    /// 2. *Thran Tome* (top N revealed -> `ChooseCard | ValidTgts$ Opponent |
    ///    Choices$ Card.IsRemembered` -> `ChangeZone(Library -> Dest)` ->
    ///    non-remembered trailing effects).
    /// 3. *Getaway Barrel* (top N revealed -> `ChooseCard | AtRandom$ True |
    ///    Choices$ Card.Creature+IsRemembered` -> `ChangeZone(Battlefield)` ->
    ///    `ChangeZoneAll(Card.IsRemembered, Library -> Bottom random)`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn peek_capture_affected_chain(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        let mut probe = p.clone();
        let outer_cond = self.take_wave8_line_condition(&mut probe).ok()?;
        let library =
            self.player_of_line(probe.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_seat_rel(library) {
            return None;
        }
        let count = match probe.take("PeekAmount") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        if matches!(probe.take("NoReveal").as_deref(), Some("True"))
            || matches!(probe.take("RevealOptional").as_deref(), Some("True"))
            || matches!(probe.take("RememberPeeked").as_deref(), Some("True"))
            || probe.take("RevealValid").is_some()
            || probe.take("RememberRevealed").as_deref() != Some("True")
        {
            return None;
        }
        probe.take("NoPeek");
        if !probe.exhausted() {
            return None;
        }
        let sub_name = sub?.trim();
        let body = self.svars.get(sub_name)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();

        // 1. *Cerebral Eruption*: RevealTop(1) -> DamageAll(Remembered$CardManaCost)
        //    -> optional ReturnToHand(ThisObject) if ConditionDefined$ Remembered.
        if api == "DamageAll" && count == "Amount::Fixed(1)" && outer_cond.is_none() {
            sp.take("IsCurse");
            sp.take("ValidDescription");
            let raw_dmg = sp.take("NumDmg")?;
            if self.svars.get(raw_dmg.trim()).map(String::as_str) != Some("Remembered$CardManaCost")
            {
                return None;
            }
            let valid_cards = sp.take("ValidCards");
            let valid_players = sp.take("ValidPlayers");
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let mut then_effs = Vec::new();
            if let Some(vc) = valid_cards {
                let expr = if library == "PlayerRel::Chosen"
                    && let Some((ctrl_rel, stripped)) =
                        self.extract_battlefield_ctrl_atom(&vc, true)
                {
                    let base = self.filter_expr(&stripped)?;
                    if base == "Filter::Any" {
                        format!("Filter::ControlledBy({ctrl_rel})")
                    } else {
                        format!("Filter::And(&[{base}, Filter::ControlledBy({ctrl_rel})])")
                    }
                } else {
                    self.filter_expr(&vc)?
                };
                let name = self.body.filter_static("EACH", &expr);
                then_effs.push(format!(
                    "Effect::DealDamageEach {{ amount: Amount::RememberedManaValue, filter: &{name} }}"
                ));
            }
            if let Some(vp) = valid_players {
                let who = self.wave11_player_of_line(Some(&vp), chain.target.as_deref(), false)?;
                then_effs.push(format!(
                    "Effect::DealDamage {{ amount: Amount::RememberedManaValue, target: TargetSpec::Player({who}) }}"
                ));
            }
            if then_effs.is_empty() {
                return None;
            }
            let trailing = self
                .consume_revealed_parent_return(after, &mut then_effs)?
                .into_opt();
            chain.effects.push(format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::RevealTop {{ player: {library}, count: {count}, \
                 filter: &Filter::Any, if_matches: &[], otherwise: &[] }}], \
                 then: &[{}] }}",
                then_effs.join(", ")
            ));
            return self.chain_sub_ability(trailing, chain);
        }

        // 2 & 3. RevealTop(N) -> `ChooseCard | Choices$ <F>+IsRemembered | ChoiceZone$ Library`.
        if api == "ChooseCard"
            && outer_cond.is_none()
            && sp.take("ChoiceZone").as_deref() == Some("Library")
        {
            sp.take("ChoiceTitle");
            sp.take("ChoiceDesc");
            sp.take("IsCurse");
            let at_random = match sp.take("AtRandom").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let mandatory = match sp.take("Mandatory").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let raw_choices = sp.take("Choices")?;
            let stripped = Self::strip_named_filter_atom(&raw_choices, "IsRemembered")?;
            if stripped.contains("IsRemembered")
                || stripped.contains("Remembered")
                || stripped.contains("IsImprinted")
            {
                return None;
            }
            let filter_ref = if stripped == "Card" {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                format!("&{}", self.body.filter_static("PEEK", &expr))
            };
            let raw_amt = sp.take("Amount").unwrap_or_else(|| "1".to_string());
            let max_num: u8 = self.plain_num(&raw_amt)?.try_into().ok()?;
            if max_num != 1 {
                return None;
            }
            let chooser = if let Some(vt) = sp.take("ValidTgts") {
                if chain.target.is_some() || sp.has("Defined") || at_random || !mandatory {
                    return None;
                }
                self.chain_target("ChooseCard", &mut sp, &vt, chain, None)?;
                if chain.target.as_deref() != Some("TargetSpec::AnyOpponent")
                    && chain.target.as_deref() != Some("TargetSpec::AnyPlayer")
                {
                    return None;
                }
                "PlayerRel::Chosen"
            } else {
                self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    chain.target.as_deref(),
                    false,
                )?
            };
            if !Self::is_single_seat_rel(chooser) {
                return None;
            }
            let choose_sub = sp.take("SubAbility")?;
            if !sp.exhausted() {
                return None;
            }
            let body2 = self.svars.get(choose_sub.trim())?.clone();
            let (api2, mut p2) = Params::parse(&body2)?;
            p2.drop_prose();
            if api2 != "ChangeZone"
                || p2.take("Origin").as_deref() != Some("Library")
                || p2.take("Defined").as_deref() != Some("ChosenCard")
            {
                return None;
            }
            if let Some(shuf) = p2.take("Shuffle")
                && shuf != "False"
            {
                return None;
            }
            let tapped = match p2.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let dest = p2.take("Destination")?;
            let after2 = p2.take("SubAbility");
            if !p2.exhausted() {
                return None;
            }
            let movement = match dest.as_str() {
                "Graveyard" if !tapped => "ChooseMove::Graveyard".to_string(),
                "Exile" if !tapped => "ChooseMove::Exile".to_string(),
                "Hand" if !tapped && library == "PlayerRel::You" => "ChooseMove::Hand".to_string(),
                "Battlefield" => format!("ChooseMove::Battlefield {{ tapped: {tapped} }}"),
                _ => return None,
            };

            let min = u8::from(mandatory || at_random);
            let pick_fx = if at_random {
                format!(
                    "Effect::ChooseCardConstrained {{ chooser: {chooser}, zone: ChooseZone::Remembered, \
                     filter: {filter_ref}, min: {min}, max: Amount::Fixed(1), at_random: true, \
                     distinct_names: false, reveal_chosen: false, movement: {movement}, then: &[] }}"
                )
            } else {
                format!(
                    "Effect::ChooseCardEx {{ chooser: {chooser}, zone: ChooseZone::Remembered, \
                     filter: {filter_ref}, min: {min}, max: Amount::Fixed(1), reveal_chosen: false, \
                     movement: {movement}, not_chosen_movement: ChooseMove::None, then: &[], for_unchosen: &[] }}"
                )
            };

            let mut then_effs = vec![pick_fx];
            let trailing = self
                .consume_library_rest_random_bottom(after2, &mut then_effs)?
                .into_opt();
            if let Some(ref rem_sub) = trailing
                && self.svar_chain_references_remembered(rem_sub)
            {
                return None;
            }
            chain.effects.push(format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::RevealTop {{ player: {library}, count: {count}, \
                 filter: &Filter::Any, if_matches: &[], otherwise: &[] }}], \
                 then: &[{}] }}",
                then_effs.join(", ")
            ));
            return self.chain_sub_ability(trailing, chain);
        }

        None
    }

    fn consume_revealed_parent_return(
        &mut self,
        after: Option<String>,
        then_effs: &mut Vec<String>,
    ) -> Option<TrailingSub> {
        let Some(name) = after else {
            return Some(TrailingSub::Done);
        };
        let trimmed = name.trim();
        if self.is_reveal_hand_cleanup_svar(trimmed) {
            return Some(TrailingSub::Done);
        }
        let body = self.svars.get(trimmed)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api == "ChangeZone"
            && sp.peek("ConditionDefined") == Some("Remembered")
            && sp.peek("Defined") == Some("Parent")
            && sp.peek("Origin") == Some("Stack")
            && sp.peek("Destination") == Some("Hand")
        {
            sp.take("ConditionDefined");
            sp.take("Defined");
            sp.take("Origin");
            sp.take("Destination");
            let present = sp.take("ConditionPresent")?;
            if !matches!(
                sp.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            ) {
                return None;
            }
            let next = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let expr = self.filter_expr_in_zone(&present, true)?;
            let filter = self.body.filter_static("REVEAL", &expr);
            then_effs.push(format!(
                "Effect::ForEachRemembered {{ effects: &[Effect::IfEventObjectMatches {{ \
                 filter: &{filter}, then: &[Effect::ReturnToHand {{ target: TargetSpec::ThisObject }}] }}] }}"
            ));
            return Some(TrailingSub::from_opt(
                self.strip_reveal_hand_cleanup_tail(next),
            ));
        }
        if self.svar_chain_references_remembered(trimmed) {
            return None;
        }
        Some(TrailingSub::Next(trimmed.to_string()))
    }

    fn consume_library_rest_random_bottom(
        &self,
        after: Option<String>,
        then_effs: &mut Vec<String>,
    ) -> Option<TrailingSub> {
        let Some(name) = after else {
            return Some(TrailingSub::Done);
        };
        let trimmed = name.trim();
        if self.is_reveal_hand_cleanup_svar(trimmed) {
            return Some(TrailingSub::Done);
        }
        let body = self.svars.get(trimmed)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api == "ChangeZoneAll"
            && sp.peek("Origin") == Some("Library")
            && sp.peek("Destination") == Some("Library")
            && sp.peek("ChangeType") == Some("Card.IsRemembered")
            && sp.peek("LibraryPosition") == Some("-1")
            && sp.peek("RandomOrder") == Some("True")
        {
            sp.take("Origin");
            sp.take("Destination");
            sp.take("ChangeType");
            sp.take("LibraryPosition");
            sp.take("RandomOrder");
            let next = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            then_effs.push(
                "Effect::ChooseCardEx { chooser: PlayerRel::You, zone: ChooseZone::Remembered, \
                 filter: &Filter::Any, min: u8::MAX, max: Amount::Fixed(u8::MAX as u32), \
                 reveal_chosen: false, movement: ChooseMove::BottomOfLibraryRandom, \
                 not_chosen_movement: ChooseMove::None, then: &[], for_unchosen: &[] }"
                    .to_string(),
            );
            return Some(TrailingSub::from_opt(
                self.strip_reveal_hand_cleanup_tail(next),
            ));
        }
        Some(TrailingSub::Next(trimmed.to_string()))
    }

    // -------------------------------------------------------------------------
    // 5. ChooseCard helpers (no Choices$ target-aliases, Watchers of the Dead,
    //    and RememberChosen -> ChangeZone(Hand) -> Pump(Self, Remembered$CardManaCost))
    // -------------------------------------------------------------------------

    /// `ChooseCard` with `ValidTgts$` and no `Choices$` (*Cryptek*, *Together Forever*,
    /// *Mandate of Abaddon*, *Rushed Rebirth*, *Last Night Together*, etc.) acts solely
    /// as a target-declaring wrapper over `SubAbility$`.
    pub(super) fn choose_card_target_alias_chain(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if !targets_here || p.has("Choices") || p.has("ChoiceZone") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ChoiceTitle");
        probe.take("ChoiceDesc");
        probe.take("IsCurse");
        probe.take("Mandatory");
        if probe.take("Optional").as_deref() == Some("True") {
            return None;
        }
        if let Some(def) = probe.take("Defined")
            && def != "You"
        {
            return None;
        }
        if let Some(dc) = probe.take("DefinedCards")
            && dc != "Targeted"
        {
            return None;
        }
        if let Some(amt) = probe.take("Amount") {
            let n: u32 = self.plain_num(&amt)?.try_into().ok()?;
            let expected_ok = match (chain.target_range, chain.target_up_to) {
                (Some((_, hi)), _) | (None, Some(hi)) => n == u32::from(hi),
                (None, None) => n == 1,
            };
            if !expected_ok || target.is_none() {
                return None;
            }
        }
        if !probe.exhausted() {
            return None;
        }
        let sub_name = sub?.trim();
        self.chain_sub_ability(Some(sub_name.to_string()), chain)
    }

    /// `RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ ChooseTwoGrave |
    /// SubAbility$ ExileAllOthers` (*Watchers of the Dead*): each opponent chooses N
    /// cards in their graveyard and exiles the rest (`ChooseCardEx(..., not_chosen_movement: Exile)`).
    pub(super) fn repeat_choose_exile_rest_graveyard(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        let mut probe = p.clone();
        if probe.take("ConditionDefined").is_some()
            || probe.take("ConditionPresent").is_some()
            || probe.take("ConditionCheckSVar").is_some()
        {
            return None;
        }
        let repeat_players = probe.take("RepeatPlayers")?;
        let domain = match repeat_players.as_str() {
            "Opponent" | "Player.Opponent" => "RepeatDomain::EachOpponent",
            "Player" => "RepeatDomain::EachPlayer",
            _ => return None,
        };
        let repeat_sub = probe.take("RepeatSubAbility")?;
        if !probe.exhausted() {
            return None;
        }
        let sub_name = sub?.trim();

        let rbody = self.svars.get(repeat_sub.trim())?.clone();
        let (rapi, mut rp) = Params::parse(&rbody)?;
        rp.drop_prose();
        if rapi != "ChooseCard"
            || rp.take("Defined").as_deref() != Some("Remembered")
            || rp.take("ChoiceZone").as_deref() != Some("Graveyard")
            || rp.take("Mandatory").as_deref() != Some("True")
            || rp.take("RememberChosen").as_deref() != Some("True")
        {
            return None;
        }
        rp.take("ChoiceTitle");
        rp.take("ChoiceDesc");
        let raw_choices = rp.take("Choices")?;
        let stripped = Self::strip_named_filter_atom(&raw_choices, "RememberedPlayerCtrl")
            .or_else(|| Self::strip_named_filter_atom(&raw_choices, "RememberedPlayerOwn"))?;
        if stripped.contains("Remembered") {
            return None;
        }
        let choose_filter_ref = if stripped == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            format!("&{}", self.body.filter_static("CHOOSE", &expr))
        };
        let raw_amt = rp.take("Amount").unwrap_or_else(|| "1".to_string());
        let max_num: u8 = self.plain_num(&raw_amt)?.try_into().ok()?;
        if max_num == 0 || !rp.exhausted() {
            return None;
        }

        let sbody = self.svars.get(sub_name)?.clone();
        let (sapi, mut sp) = Params::parse(&sbody)?;
        sp.drop_prose();
        if sapi != "ChangeZoneAll"
            || sp.take("Origin").as_deref() != Some("Graveyard")
            || sp.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let change_type = sp.take("ChangeType")?;
        if change_type != "Card.!IsRemembered" {
            return None;
        }
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        chain.effects.push(format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::ChooseCardEx {{ \
             chooser: PlayerRel::EventPlayer, zone: ChooseZone::Graveyard(PlayerRel::EventPlayer), \
             filter: {choose_filter_ref}, min: {max_num}, max: Amount::Fixed({max_num}), \
             reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::Exile, \
             then: &[], for_unchosen: &[] }}] }}"
        ));
        let trailing = self.strip_reveal_hand_cleanup_tail(next);
        self.chain_sub_ability(trailing, chain)
    }

    /// `ChooseCard | RememberChosen$ True` -> `ChangeZone(ChosenCard, Hand)` ->
    /// `Pump(Self, NumAtt: +Remembered$CardManaCost)` (*O-Kagachi Made Manifest*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn choose_card_remember_chosen_chain(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if p.peek("RememberChosen") != Some("True") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("RememberChosen");
        probe.take("ChoiceTitle");
        probe.take("ChoiceDesc");
        probe.take("IsCurse");
        let chooser =
            self.wave11_player_of_line(probe.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_seat_rel(chooser) {
            return None;
        }
        let origin_name = match probe.take("ChoiceZone").as_deref() {
            None | Some("Battlefield") => "Battlefield",
            Some("Graveyard") => "Graveyard",
            Some("Exile") => "Exile",
            Some("Hand") => "Hand",
            Some(_) => return None,
        };
        let mandatory = match probe.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !mandatory || !matches!(probe.take("Amount").as_deref(), None | Some("1")) {
            return None;
        }
        let raw_choices = probe.take("Choices")?;
        if !probe.exhausted() {
            return None;
        }
        let (owner, stripped) = if origin_name == "Graveyard" {
            let s = Self::strip_named_filter_atom(&raw_choices, "YouOwn")
                .or_else(|| Self::strip_named_filter_atom(&raw_choices, "YouCtrl"))?;
            ("PlayerRel::You", s)
        } else {
            return None;
        };
        let filter_expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter_name = self.body.filter_static("CHOOSE", &filter_expr);

        let sub1 = sub?.trim();
        let b1 = self.svars.get(sub1)?.clone();
        let (api1, mut p1) = Params::parse(&b1)?;
        p1.drop_prose();
        if api1 != "ChangeZone"
            || p1.take("Defined").as_deref() != Some("ChosenCard")
            || p1.take("Origin").as_deref() != Some(origin_name)
            || p1.take("Destination").as_deref() != Some("Hand")
            || owner != "PlayerRel::You"
        {
            return None;
        }
        let sub2 = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }

        let b2 = self.svars.get(sub2.trim())?.clone();
        let (api2, mut p2) = Params::parse(&b2)?;
        p2.drop_prose();
        if api2 != "Pump"
            || self.on_a_spell
            || !matches!(p2.take("Defined").as_deref(), None | Some("Self"))
            || p2.has("Duration")
            || p2.has("KW")
            || p2.has("NumDef")
        {
            return None;
        }
        let num_att = p2.take("NumAtt")?;
        let svar_name = num_att.strip_prefix('+')?;
        if self.svars.get(svar_name.trim()).map(String::as_str) != Some("Remembered$CardManaCost") {
            return None;
        }
        let after = p2.take("SubAbility");
        if !p2.exhausted() {
            return None;
        }
        let trailing = self.strip_reveal_hand_cleanup_tail(after);

        chain.effects.push(format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::ChooseCardEx {{ chooser: {chooser}, zone: ChooseZone::{origin_name}({owner}), \
             filter: &{filter_name}, min: 1, max: Amount::Fixed(1), reveal_chosen: false, \
             movement: ChooseMove::Hand, not_chosen_movement: ChooseMove::None, \
             then: &[], for_unchosen: &[] }}], \
             then: &[Effect::ForEachRemembered {{ effects: &[Effect::PumpTargetSlot {{ \
             slot: TargetSlot::This, power: Amount::RememberedManaValue, \
             toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, \
             duration: Duration::UntilEndOfTurn }}] }}] }}"
        ));
        self.chain_sub_ability(trailing, chain)
    }

    fn discard_only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }
}
