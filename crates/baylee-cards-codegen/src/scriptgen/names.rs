//! Semantic domain lowerings for `NameCard` effects and name-based hand/library/graveyard chains.

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Parses `ValidCards$` restrictions on `DB$ NameCard` into
    /// `(types, excluded_types, excluded_supertypes)` expressions for `Effect::ChooseCardName`.
    pub(super) fn parse_name_card_filter(
        &self,
        valid: Option<&str>,
    ) -> Option<(&'static str, &'static str, &'static str)> {
        match valid {
            None => Some(("TypeSet::EMPTY", "TypeSet::EMPTY", "SupertypeSet::EMPTY")),
            Some("Card.nonBasic") => {
                Some(("TypeSet::EMPTY", "TypeSet::EMPTY", "SupertypeSet::BASIC"))
            }
            Some("Card.nonLand") => {
                Some(("TypeSet::EMPTY", "TypeSet::LAND", "SupertypeSet::EMPTY"))
            }
            Some("Card.nonLand+nonArtifact" | "Card.nonArtifact+nonLand") => Some((
                "TypeSet::EMPTY",
                "TypeSet::LAND.union(TypeSet::ARTIFACT)",
                "SupertypeSet::EMPTY",
            )),
            Some("Creature" | "Card.Creature") => {
                Some(("TypeSet::CREATURE", "TypeSet::EMPTY", "SupertypeSet::EMPTY"))
            }
            Some("Artifact" | "Card.Artifact") => {
                Some(("TypeSet::ARTIFACT", "TypeSet::EMPTY", "SupertypeSet::EMPTY"))
            }
            Some("Land" | "Card.Land") => {
                Some(("TypeSet::LAND", "TypeSet::EMPTY", "SupertypeSet::EMPTY"))
            }
            Some(other) => self.deny(format!("a `NameCard` filter `{other}`")),
        }
    }

    /// Lowers a standalone `DB$ NameCard` step (`Effect::ChooseCardName`).
    pub(super) fn name_card_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let targets_a_player =
            targets_here && target == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let player =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(player) {
            return None;
        }
        p.claim_label("ValidDescription", "ValidCards");
        let (types, excluded_types, excluded_supertypes) =
            self.parse_name_card_filter(p.take("ValidCards").as_deref())?;
        let remember = !self.on_a_spell && player == "PlayerRel::You" && self.has_named_card_static;
        Some(vec![format!(
            "Effect::ChooseCardName {{ player: {player}, types: {types}, \
             excluded_types: {excluded_types}, excluded_supertypes: {excluded_supertypes}, \
             remember: {remember} }}"
        )])
    }

    /// Lowers multi-step `NameCard` ability chains that cannot be expressed line-by-line
    /// because downstream steps query `Remembered`, `RevealHand`, or targeted library/hand/graveyard exile.
    pub(super) fn try_name_card_chain(
        &mut self,
        p: &Params,
        sub: Option<&str>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if chain.second_target.is_some() {
            return None;
        }
        let sub_name = sub?.trim();
        let (chooser_rel, types, excluded_types, excluded_supertypes, head_target) =
            self.parse_name_card_head(p, targets_here, chain)?;

        // 1. Petra Sphinx: `NameCard(ValidTgts$ Player) -> Dig(1, Defined$ Targeted,
        //    ChangeNum$ All, ChangeValid$ Card.NamedCard, DestinationZone2$ Graveyard, Reveal$ True)`
        if let Some(tgts) = head_target.clone()
            && self
                .try_petra_sphinx_chain(
                    sub_name,
                    chooser_rel,
                    types,
                    excluded_types,
                    excluded_supertypes,
                    &tgts,
                    chain,
                )
                .is_some()
        {
            return Some(());
        }

        if head_target.is_some() || chooser_rel != "PlayerRel::You" {
            return None;
        }

        // 2. Foreshadow: `NameCard -> Mill(Opponent, RememberMilled) -> Draw(Card.NamedCard) [-> Slowtrip]`
        if self
            .try_foreshadow_chain(sub_name, types, excluded_types, excluded_supertypes, chain)
            .is_some()
        {
            return Some(());
        }

        // 3. Brain Pry: `NameCard -> RevealHand(ValidTgts$ Player, RememberRevealed) ->
        //    Discard(Targeted, 1, TgtChoose, Card.NamedCard) -> Draw(EQ0)`
        if self
            .try_brain_pry_chain(sub_name, types, excluded_types, excluded_supertypes, chain)
            .is_some()
        {
            return Some(());
        }

        // 4. Lobotomy suite: `NameCard -> search target player or opponent's graveyard, hand,
        //    and library for cards with that name and exile them [+ Lost Legacy draw]`.
        self.try_lobotomy_chain(sub_name, types, excluded_types, excluded_supertypes, chain)
    }

    fn parse_name_card_head(
        &mut self,
        p: &Params,
        targets_here: bool,
        chain: &Chain,
    ) -> Option<(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Option<String>,
    )> {
        let mut np = p.clone();
        np.drop_prose();
        np.take("AILogic");
        np.claim_label("ValidDescription", "ValidCards");
        let valid_cards = np.take("ValidCards");
        let defined = np.take("Defined");
        let head_target = if targets_here {
            if chain.target_exact.is_some()
                || chain.target_range.is_some()
                || chain.target_is_x
                || chain.target_up_to.is_some()
                || chain.target_up_to_x
                || chain.target_constraint.is_some()
            {
                return None;
            }
            let tgt = chain.target.clone()?;
            if tgt != "TargetSpec::AnyPlayer" {
                return None;
            }
            Some(tgt)
        } else {
            None
        };
        if !np.exhausted() {
            return None;
        }
        let chooser_rel = match (defined.as_deref(), head_target.is_some()) {
            (None | Some("You"), false) => "PlayerRel::You",
            (None, true) => "PlayerRel::Chosen",
            _ => return None,
        };
        let (types, excluded_types, excluded_supertypes) =
            self.parse_name_card_filter(valid_cards.as_deref())?;
        Some((
            chooser_rel,
            types,
            excluded_types,
            excluded_supertypes,
            head_target,
        ))
    }

    fn parse_name_card_target_player(p: &mut Params, chain: &Chain) -> Option<String> {
        if chain.target.is_some()
            || p.has("TargetMin")
            || p.has("TargetMax")
            || p.has("TargetsForEachPlayer")
        {
            return None;
        }
        p.take("TgtPrompt");
        match p.take("ValidTgts").as_deref() {
            Some("Player") => Some("TargetSpec::AnyPlayer".to_string()),
            Some("Opponent" | "Player.Opponent") => Some("TargetSpec::AnyOpponent".to_string()),
            _ => None,
        }
    }

    fn consume_name_cleanup(&self, sub: Option<String>, clear_named: bool) -> Option<()> {
        let Some(name) = sub else {
            return Some(());
        };
        let body = self.svars.get(name.trim())?;
        let (api, mut cp) = Params::parse(body)?;
        cp.drop_prose();
        if api != "Cleanup" {
            return None;
        }
        let rem = cp.take("ClearRemembered");
        let named = cp.take("ClearNamedCard");
        if rem.as_deref().is_some_and(|v| v != "True")
            || named.as_deref().is_some_and(|v| v != "True")
        {
            return None;
        }
        if rem.as_deref() != Some("True") && (!clear_named || named.as_deref() != Some("True")) {
            return None;
        }
        cp.exhausted().then_some(())
    }

    fn choose_card_name_expr(
        player: &str,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
    ) -> String {
        format!(
            "Effect::ChooseCardName {{ player: {player}, types: {types}, \
             excluded_types: {excluded_types}, excluded_supertypes: {excluded_supertypes}, \
             remember: false }}"
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_petra_sphinx_chain(
        &self,
        sub_name: &str,
        chooser_rel: &str,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
        head_tgt: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let dig_body = self.svars.get(sub_name)?.clone();
        let (api, mut dp) = Params::parse(&dig_body)?;
        dp.drop_prose();
        if api != "Dig"
            || dp.take("DigNum").as_deref() != Some("1")
            || dp.take("Defined").as_deref() != Some("Targeted")
            || dp.take("ChangeNum").as_deref() != Some("All")
            || !matches!(
                dp.take("ChangeValid").as_deref(),
                Some("Card.NamedCard" | "NamedCard")
            )
            || !matches!(dp.take("DestinationZone").as_deref(), None | Some("Hand"))
            || dp.take("DestinationZone2").as_deref() != Some("Graveyard")
            || dp.take("Reveal").as_deref() != Some("True")
        {
            return None;
        }
        let cleanup = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        self.consume_name_cleanup(cleanup, true)?;
        chain.target = Some(head_tgt.to_string());
        chain.effects.push(Self::choose_card_name_expr(
            chooser_rel,
            types,
            excluded_types,
            excluded_supertypes,
        ));
        chain.effects.push(format!(
            "Effect::RevealTopByName {{ player: {chooser_rel}, \
             matched: SearchDest::Hand, otherwise: SearchDest::Graveyard }}"
        ));
        Some(())
    }

    fn try_foreshadow_chain(
        &mut self,
        sub_name: &str,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let mill_body = self.svars.get(sub_name)?.clone();
        let (api1, mut mp) = Params::parse(&mill_body)?;
        mp.drop_prose();
        if api1 != "Mill" || mp.take("RememberMilled").as_deref() != Some("True") {
            return None;
        }
        let tgt = Self::parse_name_card_target_player(&mut mp, chain)?;
        let raw_num = mp.take("NumCards").unwrap_or_else(|| "1".to_string());
        let mill_amt = self.amount_or_count_ext(&raw_num)?;
        let draw_name = mp.take("SubAbility")?;
        if !mp.exhausted() {
            return None;
        }

        let draw_body = self.svars.get(draw_name.trim())?.clone();
        let (api2, mut dp) = Params::parse(&draw_body)?;
        dp.drop_prose();
        if api2 != "Draw"
            || !matches!(dp.take("Defined").as_deref(), None | Some("You"))
            || dp.take("ConditionDefined").as_deref() != Some("Remembered")
            || !matches!(
                dp.take("ConditionPresent").as_deref(),
                Some("Card.NamedCard" | "NamedCard")
            )
            || !matches!(dp.take("ConditionCompare").as_deref(), None | Some("GE1"))
        {
            return None;
        }
        let draw_num = self.plain_num(dp.take("NumCards").as_deref().unwrap_or("1"))?;
        if draw_num == 0 {
            return None;
        }
        let trailing_sub = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }

        let filter_name = self.body.filter_static("NAMED", "Filter::NamedBySource");
        chain.target = Some(tgt);
        chain.effects.push(Self::choose_card_name_expr(
            "PlayerRel::You",
            types,
            excluded_types,
            excluded_supertypes,
        ));
        chain.effects.push(format!(
            "Effect::Mill {{ amount: {mill_amt}, target: PlayerRel::Chosen }}"
        ));
        chain.effects.push(format!(
            "Effect::IfCondition {{ condition: Condition::AmountAtLeast(\
             Amount::ThisWay(ThisWay::Milled(&{filter_name})), 1), \
             then: &[Effect::draw({draw_num})], otherwise: &[] }}"
        ));
        self.chain_sub_ability(trailing_sub, chain)
    }

    fn try_brain_pry_chain(
        &mut self,
        sub_name: &str,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let rev_body = self.svars.get(sub_name)?.clone();
        let (api1, mut rp) = Params::parse(&rev_body)?;
        rp.drop_prose();
        if api1 != "RevealHand" || rp.take("RememberRevealed").as_deref() != Some("True") {
            return None;
        }
        let tgt = Self::parse_name_card_target_player(&mut rp, chain)?;
        let disc_name = rp.take("SubAbility")?;
        if !rp.exhausted() {
            return None;
        }

        let disc_body = self.svars.get(disc_name.trim())?.clone();
        let (api2, mut dp) = Params::parse(&disc_body)?;
        dp.drop_prose();
        if api2 != "Discard"
            || dp.take("Defined").as_deref() != Some("Targeted")
            || !matches!(dp.take("NumCards").as_deref(), None | Some("1"))
            || dp.take("Mode").as_deref() != Some("TgtChoose")
            || !matches!(
                dp.take("DiscardValid").as_deref(),
                Some("Card.NamedCard" | "NamedCard")
            )
        {
            return None;
        }
        let draw_name = dp.take("SubAbility")?;
        if !dp.exhausted() {
            return None;
        }

        let draw_body = self.svars.get(draw_name.trim())?.clone();
        let (api3, mut drp) = Params::parse(&draw_body)?;
        drp.drop_prose();
        if api3 != "Draw"
            || !matches!(drp.take("Defined").as_deref(), None | Some("You"))
            || drp.take("ConditionDefined").as_deref() != Some("Remembered")
            || !matches!(
                drp.take("ConditionPresent").as_deref(),
                Some("Card.NamedCard" | "NamedCard")
            )
            || drp.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let draw_num = self.plain_num(drp.take("NumCards").as_deref().unwrap_or("1"))?;
        if draw_num == 0 {
            return None;
        }
        let cleanup = drp.take("SubAbility");
        if !drp.exhausted() {
            return None;
        }
        self.consume_name_cleanup(cleanup, true)?;

        let filter_name = self.body.filter_static("NAMED", "Filter::NamedBySource");
        chain.target = Some(tgt);
        chain.effects.push(Self::choose_card_name_expr(
            "PlayerRel::You",
            types,
            excluded_types,
            excluded_supertypes,
        ));
        chain
            .effects
            .push("Effect::RevealHand { who: PlayerRel::Chosen }".to_string());
        chain.effects.push(format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::Chosen, \
             zone: ChooseZone::Hand(PlayerRel::Chosen), filter: &{filter_name}, \
             min: 1, max: Amount::Fixed(1), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], \
             for_unchosen: &[] }}"
        ));
        chain.effects.push(format!(
            "Effect::IfCondition {{ condition: Condition::Not(&Condition::AmountAtLeast(\
             Amount::ThisWay(ThisWay::Discarded), 1)), \
             then: &[Effect::draw({draw_num})], otherwise: &[] }}"
        ));
        Some(())
    }

    fn try_lobotomy_chain(
        &mut self,
        sub_name: &str,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        let body1 = self.svars.get(sub_name)?.clone();
        let (api1, mut p1) = Params::parse(&body1)?;
        p1.drop_prose();

        if api1 == "ChangeZone" && p1.peek("Origin") == Some("Graveyard,Hand,Library") {
            let tgt = self.try_lobotomy_single_zone(&mut p1, chain)?;
            self.emit_lobotomy(
                types,
                excluded_types,
                excluded_supertypes,
                &tgt,
                false,
                false,
                chain,
            );
            return Some(());
        }

        let (tgt, draw_for_exiled_hand) = self.try_lobotomy_three_step(sub_name, chain)?;
        let any_number = self.oracle.to_ascii_lowercase().contains("any number");
        self.emit_lobotomy(
            types,
            excluded_types,
            excluded_supertypes,
            &tgt,
            any_number,
            draw_for_exiled_hand,
            chain,
        );
        Some(())
    }

    fn try_lobotomy_single_zone(&self, p1: &mut Params, chain: &Chain) -> Option<String> {
        if p1.take("Origin").as_deref() != Some("Graveyard,Hand,Library")
            || p1.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let tgt = Self::parse_name_card_target_player(p1, chain)?;
        if !self.lobotomy_target_matches_oracle(&tgt)
            || !matches!(
                p1.take("ChangeType").as_deref(),
                Some("Card.NamedCard" | "NamedCard")
            )
            || !matches!(
                p1.take("DefinedPlayer").as_deref(),
                Some("Targeted" | "TargetedPlayer")
            )
            || p1.take("Chooser").as_deref() != Some("You")
            || !matches!(p1.take("Hidden").as_deref(), None | Some("True"))
        {
            return None;
        }
        let num_svar = p1.take("ChangeNum")?;
        if self.svars.get(num_svar.trim()).map(String::as_str)
            != Some("Count$ValidGraveyard,Hand,Library Card.TargetedPlayerOwn")
        {
            return None;
        }
        let cleanup = p1.take("SubAbility");
        if !p1.exhausted() {
            return None;
        }
        self.consume_name_cleanup(cleanup, true)?;
        Some(tgt)
    }

    fn try_lobotomy_three_step(&self, sub_name: &str, chain: &Chain) -> Option<(String, bool)> {
        let mut seen = [false; 3];
        let mut target: Option<String> = None;
        let mut draw_for_exiled_hand = false;
        let mut cur_opt = Some(sub_name.to_string());

        for step_idx in 0..3 {
            let step_name = cur_opt.take()?;
            let body = self.svars.get(step_name.trim())?;
            let (api, mut sp) = Params::parse(body)?;
            sp.drop_prose();
            if !matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                || sp.take("Destination").as_deref() != Some("Exile")
                || !matches!(
                    sp.take("ChangeType").as_deref(),
                    Some("Card.NamedCard" | "NamedCard")
                )
            {
                return None;
            }
            if step_idx == 0 {
                let tgt = Self::parse_name_card_target_player(&mut sp, chain)?;
                if !self.lobotomy_target_matches_oracle(&tgt) {
                    return None;
                }
                target = Some(tgt);
                if let Some(dp) = sp.take("DefinedPlayer")
                    && !matches!(dp.as_str(), "Targeted" | "TargetedPlayer")
                {
                    return None;
                }
            } else if !matches!(
                sp.take("DefinedPlayer")
                    .or_else(|| sp.take("Defined"))
                    .as_deref(),
                Some("Targeted" | "TargetedPlayer")
            ) {
                return None;
            }
            if sp.take("Hidden").is_some_and(|h| h != "True") {
                return None;
            }
            let origin = sp.take("Origin")?;
            let idx = match origin.as_str() {
                "Graveyard" => 0,
                "Hand" => 1,
                "Library" => 2,
                _ => return None,
            };
            if seen[idx] {
                return None;
            }
            seen[idx] = true;
            if idx != 0 || api == "ChangeZone" {
                if sp.take("Chooser").as_deref() != Some("You") {
                    return None;
                }
                let expected = [
                    "TargetedPlayer$CardsInGraveyard",
                    "TargetedPlayer$CardsInHand",
                    "TargetedPlayer$CardsInLibrary",
                ][idx];
                let num = sp.take("ChangeNum")?;
                if !self.is_lobotomy_zone_count(&num, expected) {
                    return None;
                }
                if idx == 1 && sp.take("RememberChanged").as_deref() == Some("True") {
                    draw_for_exiled_hand = true;
                }
                if idx == 2 && sp.take("Shuffle").as_deref() != Some("True") {
                    return None;
                }
            }
            cur_opt = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
        }

        if draw_for_exiled_hand {
            self.consume_lobotomy_draw_step(&cur_opt.take()?)?;
        } else if cur_opt.is_some() {
            self.consume_name_cleanup(cur_opt, false)?;
        }

        Some((target?, draw_for_exiled_hand))
    }

    fn consume_lobotomy_draw_step(&self, draw_name: &str) -> Option<()> {
        let draw_body = self.svars.get(draw_name.trim())?;
        let (d_api, mut dp) = Params::parse(draw_body)?;
        dp.drop_prose();
        if d_api != "Draw"
            || !matches!(
                dp.take("Defined").as_deref(),
                Some("Targeted" | "TargetedPlayer")
            )
        {
            return None;
        }
        let num_var = dp.take("NumCards")?;
        if self.svars.get(num_var.trim()).map(String::as_str) != Some("Count$RememberedSize") {
            return None;
        }
        let cleanup = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        self.consume_name_cleanup(cleanup, false)
    }

    fn lobotomy_target_matches_oracle(&self, tgt: &str) -> bool {
        !(tgt == "TargetSpec::AnyPlayer"
            && self.oracle.to_ascii_lowercase().contains("target opponent"))
    }

    fn is_lobotomy_zone_count(&self, token: &str, expected: &str) -> bool {
        self.svars
            .get(token.trim())
            .is_some_and(|s| s.trim() == expected)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_lobotomy(
        &mut self,
        types: &str,
        excluded_types: &str,
        excluded_supertypes: &str,
        tgt: &str,
        any_number: bool,
        draw_for_exiled_hand: bool,
        chain: &mut Chain,
    ) {
        let filter_name = self.body.filter_static("NAMED", "Filter::NamedBySource");
        chain.target = Some(tgt.to_string());
        chain.effects.push(Self::choose_card_name_expr(
            "PlayerRel::You",
            types,
            excluded_types,
            excluded_supertypes,
        ));
        chain.effects.push("Effect::LookAtChosenHand".to_string());

        let (zone_max, zone_mov, zone_unchosen) = if any_number {
            (
                "Amount::Fixed(u8::MAX as u32)",
                "ChooseMove::Exile",
                "ChooseMove::None",
            )
        } else {
            ("Amount::Fixed(0)", "ChooseMove::None", "ChooseMove::Exile")
        };
        let zone_exile = |zone: &str| {
            format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
                 zone: {zone}, filter: &{filter_name}, \
                 min: 0, max: {zone_max}, reveal_chosen: false, movement: {zone_mov}, \
                 not_chosen_movement: {zone_unchosen}, then: &[], for_unchosen: &[] }}"
            )
        };
        let gy_exile = zone_exile("ChooseZone::Graveyard(PlayerRel::Chosen)");
        let hand_exile = zone_exile("ChooseZone::Hand(PlayerRel::Chosen)");
        let lib_search = format!(
            "Effect::SearchLibraryOf {{ library: PlayerRel::Chosen, owner_searches: false, \
             filter: &{filter_name}, mana_value: None, finds: &[Find::EXILE.any_number()], \
             optional: true }}"
        );

        chain.effects.push(gy_exile);
        if draw_for_exiled_hand {
            chain.effects.push(format!(
                "Effect::CaptureAffected {{ first: &[{hand_exile}], then: &[{lib_search}, \
                 Effect::DrawCardsFor {{ who: PlayerRel::Chosen, amount: Amount::RememberedCount }}] }}"
            ));
        } else {
            chain.effects.push(hand_exile);
            chain.effects.push(lib_search);
        }
    }
}
