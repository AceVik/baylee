//! Reading keywords, replacement effects and how a land enters.

use super::{Bound, Chain, Params, Tx, keyword_const, keyword_enter_modifier, keyword_static};

impl Tx<'_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn keyword(&mut self, line: &str) -> Option<()> {
        if line == "Mayhem" {
            // `transcode` requires a land type line for this permission.
            self.body.keywords.push("KeywordSet::MAYHEM".to_string());
            return Some(());
        }
        if line == "Partner" {
            // Bare Partner (CR 702.124a) is emitted via `CardDef::partner` from
            // Scryfall oracle text, which `stubgen::transcode_faces` verifies.
            self.body.has_partner = true;
            return Some(());
        }
        if matches!(line, "Start your engines!" | "Start your engines") {
            self.body
                .abilities
                .push("AbilityDef::StartYourEngines".to_string());
            return Some(());
        }
        if let Some(stem) = match line.trim() {
            "Living Weapon" | "Living weapon" => Some("b_0_0_phyrexian_germ"),
            "For Mirrodin" | "For Mirrodin!" => Some("r_2_2_rebel"),
            "Job select" => Some("c_1_1_hero"),
            _ => None,
        } {
            return self.attach_token_etb(line.trim(), stem);
        }
        if line == "Convoke" {
            self.body.convoke = true;
            return Some(());
        }
        if line == "Delve" {
            self.body.delve = true;
            return Some(());
        }
        if line == "Improvise" {
            self.body.improvise = true;
            return Some(());
        }
        if line == "Exalted" {
            self.body.abilities.push(
                "triggered!(Trigger::AttacksAlone(&Filter::YOUR_CREATURE), \
                 &[Effect::PumpEventObject { power: Amount::Fixed(1), toughness: Amount::Fixed(1), \
                 keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
                    .to_string(),
            );
            return Some(());
        }
        if line == "Cascade" {
            self.body.abilities.push(
                "triggered!(Trigger::SpellCast(&Filter::This), &[Effect::Cascade])".to_string(),
            );
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("Replicate:") {
            return self.replicate(rest);
        }
        if let Some(rest) = line.strip_prefix("Disguise:") {
            return self.disguise(rest);
        }
        if let Some(rest) = line.strip_prefix("Rampage:") {
            return self.rampage(rest);
        }
        if let Some(rest) = line.strip_prefix("Evoke:") {
            return self.evoke(rest);
        }
        if let Some(rest) = line.strip_prefix("Chapter:") {
            return self.chapter(rest);
        }
        if let Some(rest) = line.strip_prefix("Ward:") {
            return match self.try_extra_ward(rest) {
                super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                super::saga_ward_convoke::ExtraOutcome::Pass => self.ward(rest),
            };
        }
        if let Some(rest) = line.strip_prefix("Crew:") {
            return self.crew(rest);
        }
        if let Some(rest) = line.strip_prefix("Saddle:") {
            return self.saddle(rest);
        }
        if let Some(rest) = line.strip_prefix("Toxic:") {
            return self.toxic(rest);
        }
        if let Some(modifier) = keyword_static(line) {
            self.body
                .abilities
                .push(Self::static_expr("Filter::This", modifier));
            return Some(());
        }
        if let Some(entry) = keyword_enter_modifier(line, self.svars) {
            self.body.enter_modifiers.push(entry);
            return Some(());
        }
        if let Some(entry) = self.kicked_etb_counter(line) {
            self.body.enter_modifiers.push(entry);
            return Some(());
        }
        let prev_unclaimed = line
            .starts_with("etbCounter:")
            .then(|| self.unclaimed.clone());
        if let super::saga_ward_convoke::ExtraOutcome::Handled(res) = self.try_extra_keyword(line) {
            if res.is_some() {
                return res;
            }
            if let Some(rest) = line.strip_prefix("etbCounter:") {
                if let Some(prev) = prev_unclaimed {
                    self.unclaimed = prev;
                }
                return self.etb_counter_wave6(rest);
            }
            return None;
        }
        if line == "Gift" {
            return self.gift_keyword();
        }
        if let Some(rest) = line.strip_prefix("Impending:") {
            return self.impending_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("ETBReplacement:") {
            if self.try_etb_prepared(rest) {
                return Some(());
            }
            return self.etb_replacement(rest);
        }
        if let Some(rest) = line.strip_prefix("Equip:") {
            return self.equip(rest);
        }
        if let Some(rest) = line.strip_prefix("Ninjutsu:") {
            return self.ninjutsu(rest, false);
        }
        if let Some(rest) = line.strip_prefix("CommanderNinjutsu:") {
            return self.ninjutsu(rest, true);
        }
        if let Some(rest) = line.strip_prefix("Cycling:") {
            return self.cycling(rest);
        }
        if let Some(rest) = line.strip_prefix("TypeCycling:") {
            return self.typecycling(rest);
        }
        if let Some(rest) = line.strip_prefix("Dredge:") {
            return self.dredge(rest);
        }
        if let Some(rest) = line.strip_prefix("Enchant:") {
            return self.enchant(rest);
        }
        if let Some(rest) = line.strip_prefix("Cumulative upkeep:") {
            return self.cumulative_upkeep(rest);
        }
        if let Some(rest) = line.strip_prefix("Echo:") {
            return self.echo(rest);
        }
        if let Some(rest) = line.strip_prefix("Fading:") {
            return self.fading(rest);
        }
        if let Some(rest) = line
            .strip_prefix("Vanishing:")
            .or_else(|| (line == "Vanishing").then_some(""))
        {
            return self.vanishing(rest);
        }
        if let Some(rest) = line.strip_prefix("Amplify:") {
            return self.amplify(rest);
        }
        if let Some(rest) = line.strip_prefix("Flashback:") {
            return self.flashback(rest);
        }
        if let Some(rest) = line.strip_prefix("Madness:") {
            return self.madness(rest);
        }
        if let Some(rest) = line.strip_prefix("Foretell:") {
            return self.foretell(rest);
        }
        if let Some(rest) = line.strip_prefix("Miracle:") {
            return self.miracle(rest);
        }
        if let Some(rest) = line.strip_prefix("Dash:") {
            return self.dash(rest);
        }
        if let Some(rest) = line.strip_prefix("Blitz:") {
            return self.blitz(rest);
        }
        if let Some(rest) = line.strip_prefix("Spectacle:") {
            return self.keyword_alternative_cost("Spectacle", "spectacle", rest, false);
        }
        if let Some(rest) = line.strip_prefix("Surge:") {
            return self.keyword_alternative_cost("Surge", "surge", rest, true);
        }
        if let Some(rest) = line.strip_prefix("Prowl:") {
            return self.keyword_alternative_cost("Prowl", "prowl", rest, false);
        }
        if line == "Jump-start" {
            return self.jump_start();
        }
        if line.trim() == "Retrace" {
            return self.retrace();
        }
        if line == "Aftermath" {
            return self.aftermath();
        }
        if line == "Fuse" {
            return self.fuse();
        }
        if let Some(rest) = line.strip_prefix("Disturb:") {
            return self.disturb(rest);
        }
        if let Some(rest) = line.strip_prefix("Escape:") {
            return self.escape(rest);
        }
        if let Some(rest) = line.strip_prefix("Squad:") {
            let expr =
                self.optional_permanent_cost_keyword("Squad", rest, self.body.squad.is_some())?;
            self.body.squad = Some(expr);
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("Offspring:") {
            let expr = self.optional_permanent_cost_keyword(
                "Offspring",
                rest,
                self.body.offspring.is_some(),
            )?;
            self.body.offspring = Some(expr);
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("MayFlashCost:") {
            return self.may_flash_cost(rest);
        }
        if line == "CARDNAME can be your commander."
            && (self.oracle.is_empty()
                || self
                    .oracle
                    .to_ascii_lowercase()
                    .contains("can be your commander"))
        {
            return Some(());
        }
        if line == "CARDNAME can't attack." {
            self.body
                .keywords
                .push("KeywordSet::CANT_ATTACK".to_string());
            return Some(());
        }
        if line == "CARDNAME can't block." {
            self.body
                .keywords
                .push("KeywordSet::CANT_BLOCK".to_string());
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("AlternateAdditionalCost:") {
            return self.alternate_additional_cost(rest);
        }
        if let Some(rest) = line.strip_prefix("Mutate:") {
            return self.mutate(rest);
        }
        if let Some(rest) = line.strip_prefix("Plot:") {
            return self.plot(rest);
        }
        if let Some(rest) = line.strip_prefix("Casualty:") {
            return self.casualty(rest);
        }
        if let Some(rest) = line.strip_prefix("Backup:") {
            return self.backup(rest);
        }
        if let Some(rest) = line.strip_prefix("Unearth:") {
            return self.unearth(rest);
        }
        if let Some(rest) = line.strip_prefix("Encore:") {
            return self.encore(rest);
        }
        if let Some(rest) = line.strip_prefix("Embalm:") {
            return self.embalm(rest);
        }
        if let Some(rest) = line.strip_prefix("Eternalize:") {
            return self.eternalize(rest);
        }
        if let Some(rest) = line.strip_prefix("Bestow:") {
            return self.bestow(rest);
        }
        if line == "Bargain" {
            return self.bargain();
        }
        if line.starts_with("Bargain:") {
            return self.deny(format!("keyword `{line}`"));
        }
        if let Some(rest) = line.strip_prefix("Emerge:") {
            return self.emerge(rest);
        }
        if let Some(rest) = line.strip_prefix("Prototype:") {
            return self.prototype(rest);
        }
        if let Some(rest) = line
            .strip_prefix("Megamorph:")
            .or_else(|| (line == "Megamorph").then_some(""))
        {
            return self.face_down_keyword(rest, true);
        }
        if let Some(rest) = line
            .strip_prefix("Morph:")
            .or_else(|| (line == "Morph").then_some(""))
        {
            return self.face_down_keyword(rest, false);
        }
        if let Some(rest) = line.strip_prefix("Entwine:") {
            return self.entwine(rest);
        }
        if let Some(rest) = line.strip_prefix("Buyback:") {
            return self.buyback(rest);
        }
        if let Some(rest) = line.strip_prefix("Kicker:") {
            return self.kicker(rest);
        }
        if let Some(rest) = line.strip_prefix("Multikicker:") {
            return self.multikicker(rest);
        }
        if let Some(rest) = line.strip_prefix("Affinity:") {
            return self.affinity(rest);
        }
        if let Some(rest) = line.strip_prefix("Modular:") {
            return self.modular_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Renown:") {
            return self.renown_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Bushido:") {
            return self.bushido_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Soulshift:") {
            return self.soulshift_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Reconfigure:") {
            return self.reconfigure_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Class:") {
            return self.class_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Overload:") {
            return self.overload_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Warp:") {
            return self.warp_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Devour:") {
            return self.devour_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Bloodthirst:") {
            return self.bloodthirst_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Craft:") {
            return self.craft_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Fabricate:") {
            return self.fabricate_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Hideaway:") {
            return self.hideaway_keyword(rest);
        }
        if line.trim() == "Spree" {
            return self.spree_keyword();
        }
        if let Some(rest) = line.strip_prefix("Strive:") {
            return self.strive_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Transmute:") {
            return self.transmute_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Scavenge:") {
            return self.scavenge_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Outlast:") {
            return self.outlast_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Reinforce:") {
            return self.reinforce_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Freerunning:") {
            return self.freerunning_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Sneak:") {
            return self.sneak_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Splice:") {
            return self.splice_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Awaken:") {
            return self.awaken_keyword(rest);
        }
        if line.trim() == "Conspire" {
            return self.conspire_keyword();
        }
        if line.starts_with("Conspire:") {
            return self.deny(format!("keyword `{line}`"));
        }
        if let Some(rest) = line.strip_prefix("Mobilize:") {
            return self.mobilize_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Afterlife:") {
            return self.afterlife_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Afflict:") {
            return self.afflict_keyword(rest);
        }
        if line.trim() == "Sunburst" {
            return self.sunburst_keyword();
        }
        if line.trim() == "Ravenous" {
            return self.ravenous_keyword();
        }
        if let Some(rest) = line.strip_prefix("Graft:") {
            return self.graft_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Tribute:") {
            return self.tribute_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("Champion:") {
            return self.champion_keyword(rest);
        }
        if line.trim() == "Cipher" {
            return self.cipher_keyword();
        }
        if let Some(rest) = line.strip_prefix("Firebending:") {
            return self.firebending_keyword(rest);
        }
        if let Some(rest) = line.strip_prefix("MayEffectFromOpeningHand:") {
            return self.may_effect_from_opening_hand(rest);
        }
        if matches!(line.trim(), "Umbra armor" | "Umbra Armor") {
            self.require_aura_permanent("Umbra armor")?;
        }
        if line.trim() == "Exploit" {
            self.require_permanent_keyword("Exploit")?;
        }
        if line.trim() == "Evolve" {
            self.require_creature_permanent("Evolve")?;
        }
        if matches!(line.trim(), "Riot" | "Unleash") {
            self.require_permanent_keyword(line.trim())?;
        }
        if let Some(bit) = keyword_const(line) {
            if bit == "KeywordSet::PROWESS" && self.body.keywords.iter().any(|k| k == bit) {
                let expr = self.filter_expr("Card.nonCreature+YouCtrl")?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                self.body.abilities.push(format!(
                    "triggered!(Trigger::SpellCast(&{filter}), &[Effect::PumpFilter {{ \
                     filter: &Filter::This, controlled_by: None, power: Amount::Fixed(1), \
                     toughness: Amount::Fixed(1), keywords: KeywordSet::EMPTY, \
                     duration: Duration::UntilEndOfTurn }}])"
                ));
            } else if bit == "KeywordSet::STORM" && self.body.keywords.iter().any(|k| k == bit) {
                // CR 702.40b: duplicate Storm instances on a card keep separate instances on the stack.
                self.body.abilities.push(
                    "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::STORM), \
                     zone = ZoneRef::Stack)"
                        .to_string(),
                );
            } else if matches!(
                bit,
                "KeywordSet::MYRIAD"
                    | "KeywordSet::EXTORT"
                    | "KeywordSet::ENLIST"
                    | "KeywordSet::SOULBOND"
                    | "KeywordSet::EXPLOIT"
                    | "KeywordSet::EVOLVE"
                    | "KeywordSet::RIOT"
                    | "KeywordSet::UNLEASH"
            ) && self.body.keywords.iter().any(|k| k == bit)
            {
                // CR 702.116b / CR 702.101b / CR 702.154c / CR 702.95d /
                // CR 702.98b / CR 702.136b: duplicate instances register
                // separately for `layers::keyword_instances`.
                self.body.abilities.push(Self::static_expr(
                    "Filter::This",
                    &format!("Modifier::AddKeyword({bit})"),
                ));
            } else {
                self.body.keywords.push(bit.to_string());
            }
            return Some(());
        }
        if let Some(from) = self.protection_filter(line) {
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                &format!("Modifier::ProtectionFrom(&{from})"),
            ));
            return Some(());
        }
        if let Some(land) = self.landwalk_filter(line) {
            let name = self.body.filter_static("LANDWALK", &land);
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                &format!("Modifier::LandwalkMatching(&{name})"),
            ));
            return Some(());
        }
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        self.deny(format!("keyword `{head}`"))
    }

    /// What a protection keyword protects from, as the `Filter` asked of a
    /// source (CR 702.16a: "protection from [quality]").
    ///
    /// Two spellings. `Protection from black` names a colour, the commonest
    /// shape by far (153 of the corpus's keyword lines). The other is
    /// `Protection:<valid>[:<prose>[:<exception>]]`, where the valid-string
    /// is the quality ("Artifact", "Creature") and is read by
    /// [`Self::filter_expr`] like any other. The one exception understood is
    /// the host card itself, which is how the Alpha Wards print "This effect
    /// doesn't remove this Aura": protection from white, except from the
    /// white Aura that grants it, so the Aura stays attached (CR 702.16c
    /// would otherwise put it into the graveyard). `Filter::This` names that
    /// Aura, the static's source. Any other exception is refused.
    pub(super) fn protection_filter(&self, word: &str) -> Option<String> {
        if let Some(color) = word.trim().strip_prefix("Protection from ") {
            let color = match color {
                "white" => "White",
                "blue" => "Blue",
                "black" => "Black",
                "red" => "Red",
                "green" => "Green",
                _ => return None,
            };
            return Some(format!(
                "Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))"
            ));
        }
        let rest = word.trim().strip_prefix("Protection:")?;
        let mut fields = rest.split(':');
        let quality = self.filter_expr(fields.next()?)?;
        let _prose = fields.next();
        let filter = match fields.next() {
            None => quality,
            Some("Card.CardUID_HostCardUID") => {
                format!("Filter::And(&[{quality}, Filter::Not(&Filter::This)])")
            }
            Some(other) => {
                return self.deny(format!("a protection that excepts `{other}`"));
            }
        };
        if fields.next().is_some() {
            return self.deny("a protection keyword with a fifth field".to_string());
        }
        Some(filter)
    }

    /// `K:Landwalk:<valid>[:<prose>]` (CR 702.14a) for a non-basic landwalk
    /// quality (snow swampwalk, snow forestwalk, legendary landwalk, nonbasic
    /// landwalk). The five basic landwalks are [`KeywordSet`] bits and are
    /// matched earlier by [`keyword_const`].
    pub(super) fn landwalk_filter(&self, word: &str) -> Option<String> {
        let trimmed = word.trim();
        if keyword_const(trimmed).is_some() {
            return None;
        }
        let rest = trimmed.strip_prefix("Landwalk:")?;
        let mut fields = rest.split(':');
        let quality = self.filter_expr(fields.next()?)?;
        let _prose = fields.next();
        if fields.next().is_some() {
            return self.deny("a landwalk keyword with a fourth field".to_string());
        }
        Some(quality)
    }

    /// `K:Cumulative upkeep:<cost>[:<prose>]` (CR 702.24a): at the beginning of
    /// your upkeep, if this permanent is on the battlefield, put an age counter
    /// on it, then sacrifice it unless you pay `<cost>` for each age counter on
    /// it.
    pub(super) fn cumulative_upkeep(&mut self, rest: &str) -> Option<()> {
        let payment = self.cumulative_upkeep_payment(rest)?;
        self.body.abilities.push(format!(
            "triggered!(Trigger::StepBegin {{ step: StepKind::Upkeep, whose: PlayerRel::You }}, \
             &[Effect::AddCounter {{ kind: counters::AGE, amount: Amount::Fixed(1) }}, {payment}], \
             condition = Some(Condition::SourceMatches(&Filter::InZone(ZoneRef::Battlefield))))"
        ));
        Some(())
    }

    /// The per-age-counter payment effect of [`Self::cumulative_upkeep`], also
    /// used when a static ability grants cumulative upkeep (`AddKeyword$
    /// Cumulative upkeep:<cost>`).
    pub(super) fn cumulative_upkeep_payment(&mut self, rest: &str) -> Option<String> {
        let mut fields = rest.split(':');
        let raw = fields.next()?.trim();
        let _prose = fields.next();
        if fields.next().is_some() {
            return self.deny(format!("cumulative upkeep `{rest}`"));
        }
        if let Ok(n) = raw.parse::<u32>()
            && n > 0
        {
            let mana = if n == 1 {
                "Amount::CountersOnSource(counters::AGE)".to_string()
            } else {
                format!(
                    "Amount::Times {{ base: &Amount::CountersOnSource(counters::AGE), factor: {n} }}"
                )
            };
            return Some(format!(
                "Effect::PlayerMayPayOr {{ player: PlayerRel::You, mana: {mana}, \
                 effect: &Effect::SacrificeSelf }}"
            ));
        }
        if let Some(life_str) = raw
            .strip_prefix("PayLife<")
            .and_then(|s| s.strip_suffix('>'))
            && let Ok(n) = life_str.parse::<u32>()
            && n > 0
        {
            let life = if n == 1 {
                "Amount::CountersOnSource(counters::AGE)".to_string()
            } else {
                format!(
                    "Amount::Times {{ base: &Amount::CountersOnSource(counters::AGE), factor: {n} }}"
                )
            };
            return Some(format!(
                "Effect::PlayerMayPayLifeOr {{ player: PlayerRel::You, life: {life}, \
                 effect: &Effect::SacrificeSelf }}"
            ));
        }
        let tokens: Vec<String> = super::cost_parts(raw);
        if !tokens.is_empty()
            && tokens.iter().all(|t| {
                t.chars().all(|c| c.is_ascii_digit())
                    || matches!(t.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "S")
            })
        {
            let (mana, parts) = self.cost_pieces(raw)?;
            if parts.is_empty() && !mana.is_empty() {
                return Some(format!(
                    "Effect::PlayerMayPayManaEachThenElse {{ player: PlayerRel::You, \
                     each: mana!(\"{mana}\"), times: Amount::CountersOnSource(counters::AGE), \
                     paid: &[], unpaid: &[Effect::SacrificeSelf] }}"
                ));
            }
        }
        self.deny(format!("cumulative upkeep `{raw}`"))
    }

    /// `K:Echo:<cost>` (CR 702.30a): "At the beginning of your upkeep, if this
    /// permanent came under your control since the beginning of your last
    /// upkeep, sacrifice it unless you pay its echo cost."
    pub(super) fn echo(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let raw = fields.next()?.trim();
        let _prose = fields.next();
        if fields.next().is_some() {
            return self.deny(format!("echo `{rest}`"));
        }
        let tokens: Vec<String> = super::cost_parts(raw);
        if !tokens.is_empty()
            && tokens.iter().all(|t| {
                t.chars().all(|c| c.is_ascii_digit())
                    || matches!(t.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "S")
            })
        {
            let (mana, parts) = self.cost_pieces(raw)?;
            if parts.is_empty() && !mana.is_empty() {
                self.body
                    .abilities
                    .push(format!("AbilityDef::Echo {{ cost: mana!(\"{mana}\") }}"));
                return Some(());
            }
        }
        self.deny(format!("echo `{raw}`"))
    }

    /// `K:Fading:<n>` (CR 702.32a): "This permanent enters with N fade counters
    /// on it" and "At the beginning of your upkeep, remove a fade counter from
    /// this permanent. If you can't, sacrifice the permanent."
    pub(super) fn fading(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let raw = fields.next()?.trim();
        let _prose = fields.next();
        if fields.next().is_some() {
            return self.deny(format!("fading `{rest}`"));
        }
        let Ok(n) = raw.parse::<i32>() else {
            return self.deny(format!("fading `{raw}`"));
        };
        if n < 0 {
            return self.deny(format!("fading `{raw}`"));
        }
        self.body.enter_modifiers.push(format!(
            "EnterModifier::WithCounters {{ kind: counters::FADE, amount: Amount::Fixed({n}) }}"
        ));
        self.body.abilities.push(
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, \
             &[Effect::IfCondition { condition: Condition::CountersOnSelf(counters::FADE, 1), \
             then: &[Effect::RemoveCounterSelf { kind: counters::FADE, n: 1 }], \
             otherwise: &[Effect::SacrificeSelf] }])"
                .to_string(),
        );
        Some(())
    }

    /// `K:Vanishing[:<n>]` (CR 702.63a–b): optional `n` time counters on entry,
    /// upkeep time-counter removal with intervening-if check, and sacrifice on
    /// `Trigger::LastCounterRemoved`.
    pub(super) fn vanishing(&mut self, rest: &str) -> Option<()> {
        if !rest.is_empty() {
            let mut fields = rest.split(':');
            let raw = fields.next()?.trim();
            let _prose = fields.next();
            if fields.next().is_some() {
                return self.deny(format!("vanishing `{rest}`"));
            }
            let Ok(n) = raw.parse::<u32>() else {
                return self.deny(format!("vanishing `{raw}`"));
            };
            if n == 0 {
                return self.deny("vanishing `0`".to_string());
            }
            self.body.enter_modifiers.push(format!(
                "EnterModifier::WithCounters {{ kind: CounterKind::Time, amount: Amount::Fixed({n}) }}"
            ));
        }
        self.body.abilities.push(
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, \
             &[Effect::RemoveCounterSelf { kind: CounterKind::Time, n: 1 }], \
             condition = Some(Condition::CountersOnSelf(CounterKind::Time, 1)))"
                .to_string(),
        );
        self.body.abilities.push(
            "triggered!(Trigger::LastCounterRemoved { kind: CounterKind::Time, by: None }, \
             &[Effect::SacrificeSelf])"
                .to_string(),
        );
        Some(())
    }

    /// `K:Amplify:<n>` (CR 702.38a, CR 702.38b): as this object enters, reveal
    /// any number of cards from your hand that share a creature type with it;
    /// it enters with `n` +1/+1 counters on it for each card revealed. Each
    /// keyword line emits its own `EnterModifier::Amplify(n)` so multiple
    /// instances work separately.
    pub(super) fn amplify(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let raw = fields.next()?.trim();
        if fields.next().is_some() {
            return self.deny(format!("amplify `{rest}`"));
        }
        let Ok(n) = raw.parse::<u8>() else {
            return self.deny(format!("amplify `{raw}`"));
        };
        if n == 0 {
            return self.deny("amplify `0`".to_string());
        }
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::Amplify({n})"));
        Some(())
    }

    /// `K:Ward:<n>` (CR 702.21a) for plain generic mana ward `1..=10`
    /// (`baylee-engine` supports up to `WARD_CEILING = 10`). Non-generic ward
    /// costs stay refused.
    pub(super) fn ward(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("ward `{rest}`"));
        }
        let Ok(mana) = rest.trim().parse::<u16>() else {
            return self.deny(format!("ward `{rest}`"));
        };
        if !(1..=10).contains(&mana) {
            return self.deny(format!("ward `{mana}`"));
        }
        self.body
            .abilities
            .push(format!("AbilityDef::Ward {{ mana: {mana} }}"));
        Some(())
    }

    /// `K:Crew:<n>` (CR 702.122a).
    pub(super) fn crew(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("crew `{rest}`"));
        }
        let Ok(n) = rest.trim().parse::<u8>() else {
            return self.deny(format!("crew `{rest}`"));
        };
        self.body.abilities.push(format!("crew!({n})"));
        Some(())
    }

    /// `K:Saddle:<n>` (CR 702.171a).
    pub(super) fn saddle(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("saddle `{rest}`"));
        }
        let Ok(n) = rest.trim().parse::<u8>() else {
            return self.deny(format!("saddle `{rest}`"));
        };
        self.body.abilities.push(format!("saddle!({n})"));
        Some(())
    }

    /// `K:Toxic:<n>` (CR 702.164a).
    pub(super) fn toxic(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("toxic `{rest}`"));
        }
        let Ok(poison) = rest.trim().parse::<u8>() else {
            return self.deny(format!("toxic `{rest}`"));
        };
        if poison == 0 {
            return self.deny("toxic `0`".to_string());
        }
        self.body
            .abilities
            .push(format!("AbilityDef::Toxic {{ poison: {poison} }}"));
        Some(())
    }

    /// `K:Replicate:<mana>` (CR 702.56a).
    pub(super) fn replicate(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("replicate `{rest}`"));
        }
        let Some(mana) = super::printed_mana(rest) else {
            return self.deny(format!("non-mana replicate `{rest}`"));
        };
        self.body.replicate = Some(format!("Some(mana!(\"{mana}\"))"));
        Some(())
    }

    /// `K:Disguise:<mana>` (CR 702.168a).
    pub(super) fn disguise(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("disguise `{rest}`"));
        }
        let Some(mana) = super::printed_mana(rest) else {
            return self.deny(format!("non-mana disguise `{rest}`"));
        };
        self.body.disguise = Some(format!("Some(mana!(\"{mana}\"))"));
        Some(())
    }

    /// `K:Rampage:<n>` (CR 702.23a).
    pub(super) fn rampage(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("rampage `{rest}`"));
        }
        let Ok(n) = rest.trim().parse::<u32>() else {
            return self.deny(format!("rampage `{rest}`"));
        };
        if n == 0 {
            return self.deny("rampage `0`".to_string());
        }
        let amount = if n == 1 {
            "Amount::BlockersBeyondFirst".to_string()
        } else {
            format!("Amount::Times {{ base: &Amount::BlockersBeyondFirst, factor: {n} }}")
        };
        self.body.abilities.push(format!(
            "triggered!(Trigger::Rampage, &[Effect::PumpFilter {{ filter: &Filter::This, \
             controlled_by: None, power: {amount}, toughness: {amount}, \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }}])"
        ));
        Some(())
    }

    /// `K:Evoke:<cost>` (CR 702.74a): alternative cast cost plus an ETB
    /// sacrifice trigger gated on `Trigger::EntersBattlefieldEvoked`.
    pub(super) fn evoke(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') || self.has_evoke || !self.body.alternative_costs.is_empty() {
            return self.deny(format!("evoke `{rest}`"));
        }
        let prev_alt = self.in_alternative_cost;
        self.in_alternative_cost = true;
        let parsed = self.cost_pieces(rest.trim());
        self.in_alternative_cost = prev_alt;
        let (mana, parts) = parsed?;
        if !parts.iter().all(|part| is_paid_alternative_cost_part(part))
            || (mana.is_empty() && parts.is_empty())
        {
            return self.deny(format!("evoke `{rest}`"));
        }
        let cost = crate::body::cost_literal(&mana, &parts);
        self.has_evoke = true;
        self.body.alternative_costs.push(format!(
            "AlternativeCost {{ cost: {cost}, condition: AltCondition::Always }}"
        ));
        self.body.abilities.push(
            "triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf])".to_string(),
        );
        Some(())
    }

    /// `K:Chapter:<max>:<svar1>,<svar2>,...` (CR 715): Saga chapter triggers.
    ///
    /// Consecutive or non-adjacent chapters naming the same `SVar` share one
    /// multi-chapter `chapters!(&[...], ...)` ability (CR 715.2c).
    pub(super) fn chapter(&mut self, rest: &str) -> Option<()> {
        let Some((max_raw, list)) = rest.split_once(':') else {
            return self.deny(format!("chapter `{rest}`"));
        };
        if list.contains(':') {
            return self.deny(format!("chapter `{rest}`"));
        }
        let Ok(max) = max_raw.trim().parse::<u8>() else {
            return self.deny(format!("chapter count `{max_raw}`"));
        };
        let names: Vec<&str> = list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if max == 0 || names.len() != usize::from(max) {
            return self.deny(format!("chapter count mismatch `{rest}`"));
        }
        let mut groups: Vec<(&str, Vec<u8>)> = Vec::new();
        for (idx, name) in names.iter().enumerate() {
            let Ok(chapter) = u8::try_from(idx + 1) else {
                return self.deny(format!("chapter `{rest}`"));
            };
            if let Some((_, chapters)) = groups.iter_mut().find(|(seen, _)| *seen == *name) {
                chapters.push(chapter);
            } else {
                groups.push((name, vec![chapter]));
            }
        }
        let prev_has_x = self.has_x;
        let prev_spell = self.on_a_spell;
        let prev_self = self.trigger_is_self;
        self.has_x = false;
        self.on_a_spell = false;
        self.trigger_is_self = true;
        let mut emitted = Vec::with_capacity(groups.len());
        let ok = (|| {
            for (name, chapters) in groups {
                let Some(body) = self.svars.get(name).cloned() else {
                    return self.deny(format!("chapter SVar `{name}` missing"));
                };
                let (chain, effects) = self.wave28b_read_chapter_chain(&body, name)?;
                let targets = Self::target_req_expr(&chain)
                    .map_or_default(|req| format!(", targets = Some({req})"));
                let second_targets = Self::second_target_req_expr(&chain)
                    .map_or_default(|req| format!(", second_targets = Some({req})"));
                match chapters.as_slice() {
                    [chapter] => {
                        if !second_targets.is_empty() && *chapter >= 16 {
                            return self.deny(format!("unsupported chapter `{name}`"));
                        }
                        emitted.push(format!(
                            "chapter!({chapter}, &[{effects}]{targets}{second_targets})"
                        ));
                    }
                    many => {
                        let nums = many
                            .iter()
                            .map(u8::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        emitted.push(format!(
                            "chapters!(&[{nums}], &[{effects}]{targets}{second_targets})"
                        ));
                    }
                }
            }
            Some(())
        })();
        self.has_x = prev_has_x;
        self.on_a_spell = prev_spell;
        self.trigger_is_self = prev_self;
        ok?;
        self.body.abilities.extend(emitted);
        Some(())
    }

    /// `K:Equip:<cost>[:<valid>:<desc>][:<mod>...]` (CR 702.6a).
    ///
    /// Every lowered Equip ability enforces CR 702.6a (`ActivationTiming::SorcerySpeed`,
    /// never `CostPart::TapSelf`, single target `min == max == 1`, and
    /// `Filter::ControlledByYou` + `Filter::CREATURE` on narrowed creature targets).
    /// Non-creature Equip targets (such as `Planeswalker.YouCtrl`) and
    /// target-dependent reductions stay honestly refused.
    pub(super) fn equip(&mut self, rest: &str) -> Option<()> {
        let fields: Vec<&str> = rest.split(':').collect();
        let cost_raw = fields.first()?.trim();
        let cost = self.cost_expr(cost_raw)?;
        if cost == "Cost::TAP" || cost.contains("TapSelf") {
            return self.deny("an `Equip` ability that taps itself".to_string());
        }
        if fields.len() == 1 || (fields.len() == 2 && fields[1].trim().starts_with("Flavor ")) {
            self.body.abilities.push(format!("equip!({cost})"));
            return Some(());
        }
        let target_raw = fields[1].trim();
        let (target_filter, tail_start) = if target_raw.is_empty() {
            let start = if fields.get(2).is_some_and(|f| f.trim().is_empty()) {
                3
            } else {
                2
            };
            ("&Filter::YOUR_CREATURE".to_string(), start)
        } else {
            if fields.len() < 3
                || target_raw.contains('$')
                || target_raw.starts_with("Flavor ")
                || fields[2].contains('$')
            {
                return self.deny("an `Equip` that narrows what it may attach to".to_string());
            }
            let Some(filter) = self.equip_target_filter(target_raw) else {
                return self.deny("an `Equip` that narrows what it may attach to".to_string());
            };
            (filter, 3)
        };
        let (limit, reduction, alt_cost) =
            self.parse_equip_modifiers(&fields[tail_start..], !target_raw.is_empty())?;
        if let Some(alt) = alt_cost {
            self.body.abilities.push(format!("equip!({cost})"));
            self.body.abilities.push(format!("equip!({alt})"));
            return Some(());
        }
        if target_raw.is_empty() && limit.is_none() && reduction.is_none() {
            return self.deny("an `Equip` with unexpected trailing fields".to_string());
        }
        let limit_clause = limit.map_or_else(String::new, |n| {
            format!(", limit = ActivationLimit::PerTurn({n})")
        });
        let red_clause = reduction.unwrap_or_default();
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::AttachSelf {{ target: TargetSpec::Object({target_filter}) }}], \
             timing = ActivationTiming::SorcerySpeed, \
             target = Some(TargetSpec::Object({target_filter})){limit_clause}{red_clause})"
        ));
        Some(())
    }

    fn parse_equip_modifiers(
        &mut self,
        tail: &[&str],
        narrowed: bool,
    ) -> Option<(Option<u8>, Option<String>, Option<String>)> {
        let mut limit: Option<u8> = None;
        let mut reduction: Option<String> = None;
        let mut alt_cost: Option<String> = None;
        for field in tail {
            let f = field.trim();
            if f.is_empty() || f.starts_with("Flavor ") {
                continue;
            }
            if let Some(lim_raw) = f.strip_prefix("ActivationLimit$ ") {
                let Ok(n) = lim_raw.trim().parse::<u8>() else {
                    return self.deny(format!("invalid `Equip` `ActivationLimit$ {lim_raw}`"));
                };
                if n == 0 || limit.is_some() || alt_cost.is_some() {
                    return self.deny(format!("invalid `Equip` modifier `{f}`"));
                }
                limit = Some(n);
            } else if let Some(red_raw) = f.strip_prefix("ReduceCost$ ") {
                if reduction.is_some() || alt_cost.is_some() {
                    return self.deny(format!("invalid `Equip` modifier `{f}`"));
                }
                reduction = Some(self.equip_cost_reduction(red_raw.trim())?);
            } else if let Some(alt_raw) = f.strip_prefix("AlternateCost$ ") {
                if narrowed || alt_cost.is_some() || limit.is_some() || reduction.is_some() {
                    return self.deny(format!("unsupported `Equip` `AlternateCost$ {alt_raw}`"));
                }
                let alt = self.cost_expr(alt_raw.trim())?;
                if alt == "Cost::TAP" || alt.contains("TapSelf") {
                    return self.deny("an `Equip` alternate cost that taps itself".to_string());
                }
                alt_cost = Some(alt);
            } else if f.contains('$') {
                return self.deny(format!("unsupported `Equip` modifier `{f}`"));
            } else if f.starts_with("Activate only") {
                if limit.is_none() {
                    return self.deny(format!("unclaimed `Equip` restriction `{f}`"));
                }
            } else if f.starts_with("This ability costs ") {
                if reduction.is_none() {
                    return self.deny(format!("unclaimed `Equip` reduction prose `{f}`"));
                }
            } else {
                return self.deny(format!("unclaimed `Equip` field `{f}`"));
            }
        }
        Some((limit, reduction, alt_cost))
    }

    fn equip_target_filter(&mut self, valid: &str) -> Option<String> {
        if valid.contains(['$', '/']) {
            return None;
        }
        let mut normalized = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.')?;
            let base = base.trim();
            let mut saw_you = false;
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == "YouCtrl" {
                    saw_you = true;
                } else if atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom.contains("ControlledBy")
                    || atom == "Self"
                    || atom == "StrictlySelf"
                {
                    return None;
                }
            }
            if !saw_you {
                return None;
            }
            if base == "Creature" {
                normalized.push(alt.to_string());
            } else if self
                .cats
                .const_path_of(baylee_core::types::SubtypeKind::Creature, base)
                .is_some()
            {
                // CR 702.6a: "Equip [quality]" always targets a creature you
                // control, never a non-creature Kindred permanent of that subtype.
                normalized.push(format!("Creature.{base}+{atoms_raw}"));
            } else {
                return None;
            }
        }
        if normalized.is_empty() {
            return None;
        }
        let joined = normalized.join(",");
        let expr = self.filter_expr(&joined)?;
        if expr != "Filter::YOUR_CREATURE" && !expr.contains("Filter::ControlledByYou") {
            return None;
        }
        let filter = self.body.filter_static("EQUIP", &expr);
        Some(format!("&{filter}"))
    }

    fn equip_cost_reduction(&mut self, raw: &str) -> Option<String> {
        let def = if raw.starts_with("Count$") || raw.starts_with("PlayerCount") {
            raw.to_string()
        } else if let Some(resolved) = self.svars.get(raw).cloned() {
            resolved.trim().to_string()
        } else {
            return self.deny(format!("`Equip` `ReduceCost$ {raw}` naming no `SVar`"));
        };
        if def == "PlayerCountPropertyYou$HasPropertyActive" {
            return Some(
                ", cost_reduction = Some(CostReduction::If { condition: Condition::YourTurn, amount: 1 })"
                    .to_string(),
            );
        }
        if let Some(cond_red) = Self::equip_flag_reduction(&def) {
            return Some(cond_red);
        }
        if let Some(compare) = def.strip_prefix("Count$Compare ") {
            if let Some((lhs_part, else_str)) = compare.rsplit_once('.')
                && else_str.trim() == "0"
                && let Some((cond_spec, then_str)) = lhs_part.rsplit_once('.')
                && let Ok(amount) = then_str.trim().parse::<u32>()
                && amount > 0
                && let Some((var, cmp)) = cond_spec.trim().rsplit_once(' ')
                && let Some(cond) = self
                    .svar_condition_clause(var.trim(), cmp.trim())
                    .or_else(|| self.extra_svar_condition_clause(var.trim(), cmp.trim()))
                    .or_else(|| self.wave15_trigger_svar_condition(var.trim(), cmp.trim()))
                && !cond.contains("Condition::XAtLeast")
            {
                return Some(format!(
                    ", cost_reduction = Some(CostReduction::If {{ condition: {cond}, amount: {amount} }})"
                ));
            }
            return self.deny(format!("unsupported `Equip` `ReduceCost$ {raw}`"));
        }
        let Some(amt) = self.static_count_expr(&def) else {
            return self.deny(format!("unsupported `Equip` `ReduceCost$ {raw}`"));
        };
        if amt.starts_with("Amount::Fixed(") || amt.contains("Target") {
            return self.deny(format!("unsupported `Equip` `ReduceCost$ {raw}`"));
        }
        Some(format!(
            ", cost_reduction = Some(CostReduction::PerCount {{ amount: {amt}, each: 1 }})"
        ))
    }

    fn equip_flag_reduction(def: &str) -> Option<String> {
        let rest = def.strip_prefix("Count$")?;
        let mut parts = rest.split('.');
        let flag = parts.next()?.trim();
        let then_str = parts.next()?.trim();
        let else_str = parts.next()?.trim();
        if parts.next().is_some() || else_str != "0" {
            return None;
        }
        let amount = then_str.parse::<u32>().ok().filter(|&n| n > 0)?;
        let cond = match flag {
            "Monarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" => "Condition::CompletedADungeon(PlayerRel::You)",
            "Revolt" => "Condition::Revolt",
            "Delirium" => "Condition::GraveyardTypesAtLeast(4)",
            "Threshold" => "Condition::GraveyardCountAtLeast(7)",
            "Metalcraft" => "Condition::ControlCount(&Filter::ARTIFACT, 3)",
            "Hellbent" => "Condition::HandSizeAtMost(0)",
            "FatefulHour" => "Condition::LifeAtMost(5)",
            _ => return None,
        };
        Some(format!(
            ", cost_reduction = Some(CostReduction::If {{ condition: {cond}, amount: {amount} }})"
        ))
    }

    /// `K:Ninjutsu:<cost>[:Commander]` and `K:CommanderNinjutsu:<cost>`
    /// (CR 702.49a–d): hand- or hand/command-zone activated ability that
    /// returns an unblocked attacker you control and puts this card onto the
    /// battlefield tapped and attacking that defender.
    pub(super) fn ninjutsu(&mut self, rest: &str, force_commander: bool) -> Option<()> {
        let is_permanent = self.type_line.split_whitespace().any(|w| {
            matches!(
                w,
                "Creature" | "Artifact" | "Enchantment" | "Planeswalker" | "Land" | "Battle"
            )
        });
        if !is_permanent {
            return self.deny("`Ninjutsu` on a non-permanent card".to_string());
        }
        let mut fields = rest.split(':');
        let cost_raw = fields.next()?.trim();
        let commander = match (force_commander, fields.next().map(str::trim)) {
            (false, None) => false,
            (true, None) | (false, Some("Commander")) => true,
            (_, Some(_)) => {
                return self.deny(format!("`Ninjutsu` with unexpected fields `{rest}`"));
            }
        };
        if fields.next().is_some() {
            return self.deny(format!("`Ninjutsu` with unexpected fields `{rest}`"));
        }
        if cost_raw.is_empty() {
            return self.deny("an empty `Ninjutsu` cost".to_string());
        }
        let (mana, mut parts) = self.cost_pieces(cost_raw)?;
        if (mana.is_empty() && parts.is_empty())
            || parts.iter().any(|p| {
                matches!(
                    p.as_str(),
                    "TapSelf" | "UntapSelf" | "ExertSelf" | "SacrificeSelf" | "SacrificeGrantor"
                )
            })
        {
            return self.deny(format!("unsupported `Ninjutsu` cost `{cost_raw}`"));
        }
        parts.push("ReturnUnblockedAttacker".to_string());
        let cost = crate::body::cost_literal(&mana, &parts);
        let zone = if commander {
            "ActivationZone::HandOrCommand"
        } else {
            "ActivationZone::Hand"
        };
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::PutNinjutsuOntoBattlefield], \
             zone = {zone}, condition = Some(Condition::InCombatAfterBlockers))"
        ));
        self.body.notes.push(
            if commander {
                "commander ninjutsu"
            } else {
                "ninjutsu"
            }
            .to_string(),
        );
        Some(())
    }

    /// `K:Cycling:<cost>` as the activated ability the keyword is.
    ///
    /// CR 702.29a in full: "Cycling [cost]" means "[cost], Discard this card:
    /// Draw a card", activated from the hand. Nothing here is new to the DSL
    /// — [`crate::landgen`] has written that sentence from the printed text
    /// since the cycling lands — and this emits the same string through the
    /// same [`crate::body::cost_literal`], so a Desert comes out of either
    /// reader byte for byte the same.
    ///
    /// The cost is read by [`Self::cost_pieces`] rather than as mana, which
    /// is the `cost 'Sac'` lesson once more: the reference writes 57 distinct
    /// costs across these 306 lines and two of them are not mana at all
    /// (`PayLife<2>`, `Sac<1/Land>`). One reader answers every spelling, and
    /// `1 U` is the same sentence as `{1}{U}` to it.
    ///
    /// A fourth field refuses **by name**. All 306 lines carry three fields
    /// today, so a fourth is a sentence nobody here has read — and cycling is
    /// exactly where a guess would be invisible, because the keyword supplies
    /// everything the card does not print. `K:TypeCycling` is a different
    /// sentence with its own arm, [`Self::typecycling`].
    pub(super) fn cycling(&mut self, rest: &str) -> Option<()> {
        if let Some((_, extra)) = rest.split_once(':') {
            return self.deny(format!("a `Cycling` with a fourth field `{extra}`"));
        }
        let (mana, mut parts) = self.cost_pieces(rest)?;
        parts.push("DiscardSelf".to_string());
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::draw(1)], zone = ActivationZone::Hand)"
        ));
        self.body.notes.push("cycling".to_string());
        Some(())
    }

    /// `K:TypeCycling:<type>:<cost>[:<label>]` as the activated ability the
    /// keyword is: "[Type]cycling [cost]" means "[Cost], Discard this card:
    /// Search your library for a [type] card, reveal it, and put it into
    /// your hand. Then shuffle." (CR 702.29e). The search reveals and
    /// shuffles by itself, as Ash Barrens' hand-written basic landcycling
    /// does, and `AbilityDef::is_cycling` reads this shape as cycling
    /// (702.29f), so "when you cycle" triggers see it.
    ///
    /// The type is a valid-string read as a card in a library: `Basic` is
    /// basic landcycling ("a basic land card"), `Plains` a Plains card,
    /// `Wizard` a Wizard card, `Land.Artifact` an artifact land card. A
    /// fifth field is the words the type is printed with (`artifact land`)
    /// and says nothing the type does not.
    pub(super) fn typecycling(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(kind), Some(cost)) = (fields.next(), fields.next()) else {
            return self.deny("a `TypeCycling` without a type and a cost".to_string());
        };
        fields.next();
        if let Some(extra) = fields.next() {
            return self.deny(format!("a `TypeCycling` with a fifth field `{extra}`"));
        }
        let valid = match kind.trim() {
            "Basic" => "Land.Basic",
            other => other,
        };
        let Some(filter) = self.filter_expr_in_zone(valid, true) else {
            return self.deny(format!("a `TypeCycling` type `{kind}`"));
        };
        let name = self.body.filter_static("SEARCH", &filter);
        let (mana, mut parts) = self.cost_pieces(cost)?;
        parts.push("DiscardSelf".to_string());
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::SearchLibrary {{ filter: &{name}, finds: &[Find::HAND], optional: false }}], zone = ActivationZone::Hand)"
        ));
        self.body.notes.push("typecycling".to_string());
        Some(())
    }

    /// `K:Dredge:<N>`: "As long as you have at least N cards in your library,
    /// if you would draw a card, you may instead mill N cards and return this
    /// card from your graveyard to your hand" (CR 702.52a), the replacement
    /// Golgari Thug's hand-written dredge 4 already is. The engine applies it
    /// from the graveyard and asks it only of a library of N or more
    /// (702.52b).
    pub(super) fn dredge(&mut self, rest: &str) -> Option<()> {
        let Some(n) = rest.trim().parse::<u8>().ok().filter(|&n| n > 0) else {
            return self.deny(format!("a `Dredge` count `{rest}`"));
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::Dredge({n}))"
        ));
        self.body.notes.push("dredge".to_string());
        Some(())
    }

    /// `K:Flashback:<cost>` for mana and the non-mana parts the casting
    /// wizard already pays as an alternative cost (CR 702.34a).
    pub(super) fn flashback(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Flashback` with extra fields `{rest}`"));
        }
        let (mana, raw_parts) = self.cost_pieces(rest)?;
        // `cast_wizard` pays `TapN { .. }` (not `TapOther`) for alternative
        // / flashback spell casts (`paid_as_an_alternative_cost`), so promote
        // single-permanent `tapXType<1/...>` from `TapOther` to `TapN { .. }`.
        let parts: Vec<String> = raw_parts
            .into_iter()
            .map(|part| {
                if let Some(filter) = part
                    .strip_prefix("TapOther(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    format!("TapN {{ filter: {filter}, count: 1 }}")
                } else {
                    part
                }
            })
            .collect();
        // `cost_pieces` produces these supported alternative-cost parts;
        // its other object and self costs need different payment rules.
        let paid = |part: &String| {
            part.starts_with("PayLife(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("TapN {")
        };
        if (mana.is_empty() && parts.is_empty()) || !parts.iter().all(paid) {
            return self.deny(format!("a `Flashback` cost `{rest}` the cast cannot pay"));
        }
        self.body.flashback = Some(if mana.is_empty() {
            "Some(ManaCost::ZERO)".to_string()
        } else {
            format!("Some(mana!(\"{mana}\"))")
        });
        self.body.flashback_parts = parts;
        self.body.notes.push("flashback".to_string());
        Some(())
    }

    /// `K:Madness:<cost>` as the alternative cost of the card's exile trigger
    /// (CR 702.35). An empty cost, extra fields or a non-mana part the wizard
    /// cannot pay refuses the whole card rather than quietly changing its price.
    pub(super) fn madness(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Madness` with extra fields `{rest}`"));
        }
        // Keywords are read before the spell's `A:` line sets `has_x`, but
        // Madness's X is likewise announced and paid during its cast. Keep
        // `cost_pieces`' requirement for `SVar:X:Count$xPaid`: the scope here
        // supplies the announcement, never a guess at what X counts.
        let previous_has_x = std::mem::replace(&mut self.has_x, true);
        let cost = self.cost_pieces(rest);
        self.has_x = previous_has_x;
        let (mana, parts) = cost?;
        if (mana.is_empty() && parts.is_empty())
            || !parts.iter().all(|part| part.starts_with("PayLife("))
        {
            return self.deny(format!("a `Madness` cost `{rest}` the cast cannot pay"));
        }
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.madness = Some(format!("Some({cost})"));
        self.body.notes.push("madness".to_string());
        Some(())
    }

    /// `K:Buyback:<cost>` (CR 702.27a), mana or parts: "Sacrifice a land",
    /// "Pay 3 life, Discard a card at random". Only parts the cast pays are
    /// read: another one written here would be cast for free.
    pub(super) fn buyback(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Buyback` with extra fields `{rest}`"));
        }
        let (mana, parts) = self.cost_pieces(rest)?;
        let paid = |part: &String| {
            part.starts_with("PayLife(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("Discard(")
                || part == "DiscardRandom"
        };
        if !parts.iter().all(paid) {
            return self.deny(format!("a `Buyback` cost `{rest}` the cast cannot pay"));
        }
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.buyback = Some(format!("Some({cost})"));
        self.body.notes.push("buyback".to_string());
        Some(())
    }

    /// `K:Kicker:<mana>` or `K:Kicker:<mana1>:<mana2>` (CR 702.33a–b) for
    /// single or dual mana-only kicker costs.
    ///
    /// Three-or-more-cost kickers, variable-X kicker costs, and non-mana
    /// kicker costs remain refused. For dual kickers, the emitted
    /// `FaceDef::additional_costs` slice is aligned to the Oracle text's
    /// `"Kicker <A> and/or <B>"` order (CR 702.33f), recording the 1-based
    /// script-slot remap in `self.dual_kicker_remap`.
    /// `K:Kicker:<cost>` or `K:Kicker:<cost1>:<cost2>` (CR 702.33a), including
    /// variable `{X}` kickers, non-mana `CostPart`s, and mixed mana + non-mana
    /// kickers, preserving Oracle-order dual-kicker alignment.
    pub(super) fn kicker(&mut self, rest: &str) -> Option<()> {
        self.lower_kicker_line(rest)
    }

    fn try_etb_prepared(&mut self, rest: &str) -> bool {
        let mut fields = rest.split(':').map(str::trim);
        let (Some("Other"), Some(svar), None) = (fields.next(), fields.next(), fields.next())
        else {
            return false;
        };
        let Some(body) = self.svars.get(svar) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        if api != "AlterAttribute" {
            return false;
        }
        p.drop_prose();
        let defined_ok = matches!(p.take("Defined").as_deref(), None | Some("Self"));
        let attr_ok = p.take("Attributes").as_deref() == Some("Prepared");
        if !(defined_ok && attr_ok && p.exhausted()) {
            return false;
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::Prepared".to_string());
        self.body.notes.push("prepared".to_string());
        true
    }

    /// `K:etbCounter:<KIND>:<n>:CheckSVar$ <var>[:<desc>]` for a kicked
    /// permanent:
    /// - Single-kicker (`Count$Kicked.1.0` | `Count$TimesKicked`): lowers to
    ///   `EnterModifier::WithCounters` scaled by `Amount::TimesKicked`.
    /// - Dual-kicker (`P1P1`, positive `u8`, `Count$Kicked.1.0`): lowers to
    ///   `EnterModifier::IfKicked { index: None, counters: n, modifiers: &[] }`.
    pub(super) fn kicked_etb_counter(&mut self, line: &str) -> Option<String> {
        let mut fields = line.strip_prefix("etbCounter:")?.split(':');
        let kind = super::counter_kind(fields.next()?.trim())?;
        let amount_raw = fields.next()?.trim();
        let cond = fields.next()?.trim();
        let var = cond.strip_prefix("CheckSVar$")?.trim();
        let def = self.svars.get(var).map(|s| s.trim());
        if (self.has_single_kicker
            || self.dual_kicker_remap.is_some()
            || self.body.multikicker.is_some())
            && var == amount_raw
            && def.is_some_and(|d| d.starts_with("Count$TimesKicked"))
        {
            let amount = self.etb_wave6_amount(amount_raw)?;
            return Some(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
            ));
        }
        if let Some(n) = amount_raw.parse::<u32>().ok().filter(|&n| n > 0) {
            if self.has_single_kicker {
                if !matches!(def, Some("Count$Kicked.1.0" | "Count$TimesKicked")) {
                    return None;
                }
                let amount = if n == 1 {
                    "Amount::TimesKicked".to_string()
                } else {
                    format!("Amount::Times {{ base: &Amount::TimesKicked, factor: {n} }}")
                };
                return Some(format!(
                    "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
                ));
            }
            if self.dual_kicker_remap.is_some()
                && def == Some("Count$Kicked.1.0")
                && kind == "CounterKind::P1P1"
            {
                let counters = u8::try_from(n).ok()?;
                return Some(format!(
                    "EnterModifier::IfKicked {{ index: None, counters: {counters}, modifiers: &[] }}"
                ));
            }
            return None;
        }
        if self.has_single_kicker
            && !self.has_x_mana_cost
            && def == Some("Count$Kicked.1.0")
            && self.body.additional_costs.iter().any(|c| c.contains("{X}"))
            && self.svars.get(amount_raw).map(|s| s.trim()) == Some("Count$xPaid")
        {
            return Some(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: Amount::X }}"
            ));
        }
        None
    }

    /// Additional Wave-6/Wave-7 `K:etbCounter:` shapes not handled by `saga_ward_convoke`:
    /// empty 3rd condition field (`::desc`), `Count$TotalManaSpent` /
    /// `Count$TimesKicked` / `Count$xPaid`, `LastState*` zone counts, arithmetic
    /// suffixes (`Twice`, `Times.`, `Plus.`, `Minus.`, `Half*`, `Div.`), and
    /// static board/turn/life/domain/distinct-color `Amount`s via `static_count_expr`.
    fn etb_counter_wave6(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(kind_raw), Some(amount_raw)) = (fields.next(), fields.next()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        let cond_raw = fields.next();
        let desc_raw = fields.next();
        if fields.next().is_some() {
            return self.deny("keyword `etbCounter`".to_string());
        }
        let cond = match cond_raw.map(str::trim) {
            None | Some("") => None,
            Some(c) if c.eq_ignore_ascii_case("no condition") => None,
            Some(c) => {
                // Keep 4-field `etbCounter:KIND:amt:Prose...` (with omitted condition
                // colon slot) refused instead of parsing prose as parameters.
                if desc_raw.is_none() && !c.contains('$') {
                    return self.deny("keyword `etbCounter`".to_string());
                }
                let Some(cond_expr) = self.etb_counter_condition(c) else {
                    return self.deny("keyword `etbCounter`".to_string());
                };
                Some(cond_expr)
            }
        };
        let Some(kind) = super::counter_kind(kind_raw.trim()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        if cond.is_none() && self.push_etb_conditional_amount(&kind, amount_raw.trim()) {
            return Some(());
        }
        let Some(amount) = self.etb_wave6_amount(amount_raw.trim()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        self.body.enter_modifiers.push(match cond {
            None => format!("EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"),
            Some(condition) => format!(
                "EnterModifier::WithCountersIf {{ kind: {kind}, count: {amount}, condition: {condition} }}"
            ),
        });
        Some(())
    }

    fn push_etb_conditional_amount(&mut self, kind: &str, amount_raw: &str) -> bool {
        let def = if amount_raw.starts_with("Count$") {
            amount_raw.to_string()
        } else {
            let Some(v) = self.svars.get(amount_raw) else {
                return false;
            };
            v.trim().to_string()
        };
        let arms = if let Some(rest) = def.strip_prefix("Count$Escaped.") {
            let Some((escaped_arm, normal_arm)) = rest.split_once('.') else {
                return false;
            };
            Some((
                "Condition::Escaped".to_string(),
                escaped_arm.trim(),
                normal_arm.trim(),
            ))
        } else if let Some((cond, rest)) = def
            .strip_prefix("Count$Monarch.")
            .map(|r| ("Condition::IsMonarch(PlayerRel::You)", r))
            .or_else(|| {
                def.strip_prefix("Count$Initiative.")
                    .map(|r| ("Condition::HasInitiative(PlayerRel::You)", r))
            })
            .or_else(|| {
                def.strip_prefix("Count$CompletedDungeon.")
                    .map(|r| ("Condition::CompletedADungeon(PlayerRel::You)", r))
            })
            .or_else(|| {
                def.strip_prefix("Count$Bargained.")
                    .map(|r| ("Condition::Bargained", r))
            })
            .or_else(|| {
                def.strip_prefix("Count$Prototyped.")
                    .map(|r| ("Condition::Prototyped", r))
            })
            .or_else(|| {
                def.strip_prefix("Count$Warped.")
                    .or_else(|| def.strip_prefix("Count$Warp."))
                    .map(|r| ("Condition::WarpCostPaid", r))
            })
        {
            let Some((then_arm, else_arm)) = rest.split_once('.') else {
                return false;
            };
            Some((cond.to_string(), then_arm.trim(), else_arm.trim()))
        } else if let Some(rest) = def.strip_prefix("Count$Compare ") {
            let Some((lhs, else_arm)) = rest.rsplit_once('.') else {
                return false;
            };
            let Some((cond_spec, then_arm)) = lhs.rsplit_once('.') else {
                return false;
            };
            let Some((inner_var, inner_cmp)) = cond_spec.trim().rsplit_once(' ') else {
                return false;
            };
            let Some(cond) = self.etb_check_svar_condition_at(inner_var, Some(inner_cmp), 0) else {
                return false;
            };
            if cond.contains("CountersOnSelf") || cond.contains("CountersOnSource") {
                return false;
            }
            Some((cond, then_arm.trim(), else_arm.trim()))
        } else {
            None
        };
        let Some((then_cond, then_raw, else_raw)) = arms else {
            return false;
        };
        let Some(then_amount) = self.etb_optional_wave6_amount(then_raw) else {
            return false;
        };
        let Some(else_amount) = self.etb_optional_wave6_amount(else_raw) else {
            return false;
        };
        if then_amount.is_empty() && else_amount.is_empty() {
            return false;
        }
        let else_cond = self.negate_etb_condition(&then_cond);
        for (condition, count) in [(then_cond, then_amount), (else_cond, else_amount)] {
            if !count.is_empty() {
                self.body.enter_modifiers.push(format!(
                    "EnterModifier::WithCountersIf {{ kind: {kind}, count: {count}, condition: {condition} }}"
                ));
            }
        }
        true
    }

    fn etb_optional_wave6_amount(&mut self, raw: &str) -> Option<String> {
        if raw.trim() == "0" {
            Some(String::new())
        } else {
            self.etb_wave6_amount(raw.trim())
        }
    }

    fn negate_etb_condition(&mut self, inner: &str) -> String {
        if let Some(unwrapped) = inner
            .strip_prefix("Condition::Not(&")
            .and_then(|s| s.strip_suffix(')'))
        {
            unwrapped.to_string()
        } else {
            let name = self.body.condition_static("CHECK", inner);
            format!("Condition::Not(&{name})")
        }
    }

    fn etb_counter_condition(&mut self, raw: &str) -> Option<String> {
        let spec = format!("Stub$ _ | {}", raw.trim());
        let (_, mut p) = Params::parse(&spec)?;
        let cond = if let Some(valid) = p.take("ValidCard") {
            match valid.trim() {
                "Card.Self+escaped" | "Card.escaped+Self" => "Condition::Escaped".to_string(),
                "Card.Self+bargained" | "Card.bargained+Self" => "Condition::Bargained".to_string(),
                "Card.Self+prototyped" | "Card.prototyped+Self" => {
                    "Condition::Prototyped".to_string()
                }
                "Card.Self+PromisedGift" | "Card.PromisedGift+Self" => {
                    "Condition::GiftWasPromised".to_string()
                }
                "Card.Self+Warped"
                | "Card.Warped+Self"
                | "Card.Self+CastSa Spell.Warp"
                | "Card.CastSa Spell.Warp+Self" => "Condition::WarpCostPaid".to_string(),
                "Card.Self+wasCastByYou" | "Card.wasCastByYou+Self" => {
                    "Condition::SourceWasCastByYou".to_string()
                }
                "Card.Self+!wasCastFromYourHandByYou" | "Card.!wasCastFromYourHandByYou+Self" => {
                    "Condition::Not(&Condition::All(&[Condition::SourceWasCastByYou, \
                     Condition::SourceMatches(&Filter::CastFromHand)]))"
                        .to_string()
                }
                "Card.Self+wasCastFromYourHandByYou" | "Card.wasCastFromYourHandByYou+Self" => {
                    "Condition::All(&[Condition::SourceWasCastByYou, \
                     Condition::SourceMatches(&Filter::CastFromHand)])"
                        .to_string()
                }
                "Card.Self+!token" | "Card.!token+Self" => {
                    "Condition::SourceMatches(&Filter::Not(&Filter::IsToken))".to_string()
                }
                _ => return None,
            }
        } else if let Some(cond_raw) = p.take("Condition") {
            Self::named_condition(&cond_raw)
                .or_else(|| Self::wave7_named_condition(&cond_raw))
                .or_else(|| Self::wave21c_named_condition(&cond_raw))?
                .to_string()
        } else if let Some(clause) = Self::take_check_defined_player(&mut p) {
            clause.to_string()
        } else if p.peek("Revolt").is_some()
            || p.peek("Delirium").is_some()
            || p.peek("Threshold").is_some()
            || p.peek("Metalcraft").is_some()
            || p.peek("Hellbent").is_some()
            || p.peek("FatefulHour").is_some()
        {
            let clauses = Self::take_trigger_bool_conditions(&mut p)?;
            match clauses.as_slice() {
                [only] => (*only).to_string(),
                _ => return None,
            }
        } else if let Some(adamant) = p.take("Adamant") {
            Self::wave16a_adamant_condition(&adamant)?
        } else if let Some(valid) = p.take("IsPresent") {
            self.present_clause(&mut p, &valid)?
        } else {
            let var = p.take("CheckSVar")?;
            let explicit_cmp = p.take("SVarCompare");
            self.etb_check_svar_condition(&var, explicit_cmp.as_deref())?
        };
        if !p.exhausted()
            || cond.contains("CountersOnSelf")
            || cond.contains("CountersOnSource")
            || cond.contains("SourcePower")
        {
            return None;
        }
        Some(cond)
    }

    fn etb_check_svar_condition(
        &mut self,
        var: &str,
        explicit_cmp: Option<&str>,
    ) -> Option<String> {
        self.etb_check_svar_condition_at(var, explicit_cmp, 0)
    }

    fn etb_check_svar_condition_at(
        &mut self,
        var: &str,
        explicit_cmp: Option<&str>,
        depth: u8,
    ) -> Option<String> {
        if depth >= 2 {
            return None;
        }
        let trimmed_var = var.trim();
        let cmp = explicit_cmp.unwrap_or("GE1").trim();
        let def = self
            .svars
            .get(trimmed_var)
            .map_or_else(|| trimmed_var.to_string(), |s| s.trim().to_string());
        // Preserve existing contract: plain `Count$Valid <single-clause>` without
        // explicit `SVarCompare$` (e.g. `Creature.YouCtrl` in `CondEtb`) and
        // unhandled `Count$Kicked*` shapes stay refused, while qualified
        // `Count$Valid <filter>+<pred>` checks (*Frontier Mastodon*, *Heir of the
        // Ancient Fang*) lower via `inline_valid_count_condition`.
        if def.starts_with("Count$Kicked")
            || (explicit_cmp.is_none()
                && def
                    .strip_prefix("Count$Valid ")
                    .is_some_and(|rest| !rest.contains('$') && !rest.contains('+')))
        {
            return None;
        }
        if let Some(cond) = self.etb_compare_condition(&def, cmp, depth) {
            return Some(cond);
        }
        if let Some(flag_true) = match def.as_str() {
            "Count$wasCastFromYourHandByYou.1.0" => Some(true),
            "Count$wasCastFromYourHandByYou.0.1" => Some(false),
            _ => None,
        } {
            let want_true = match cmp {
                "GE1" | "GT0" | "EQ1" => true,
                "EQ0" | "LE0" | "LT1" => false,
                _ => return None,
            };
            let base = "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceMatches(&Filter::CastFromHand)])";
            return Some(if flag_true == want_true {
                base.to_string()
            } else {
                self.negate_etb_condition(base)
            });
        }
        if def == "Count$xPaid" && self.has_x_mana_cost {
            return match super::count_bound(cmp) {
                Some(super::Bound::AtLeast(n)) => Some(format!("Condition::XAtLeast({n})")),
                Some(super::Bound::AtMost(n)) => n
                    .checked_add(1)
                    .map(|k| format!("Condition::Not(&Condition::XAtLeast({k}))")),
                None => None,
            };
        }
        if def == "Count$TimesKicked" {
            return Self::wave15_amount_cmp("Amount::TimesKicked", cmp);
        }
        if def == "Count$TimesKicked/Twice"
            || def
                .strip_prefix("Count$TimesKicked/Times.")
                .and_then(|s| s.trim().parse::<u32>().ok())
                .is_some_and(|n| n >= 1)
        {
            return match cmp {
                "GE1" | "GT0" | "NE0" => {
                    Some("Condition::AmountAtLeast(Amount::TimesKicked, 1)".to_string())
                }
                "EQ0" | "LE0" | "LT1" => Some(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::TimesKicked, 1))".to_string(),
                ),
                _ => None,
            };
        }
        if matches!(
            def.as_str(),
            "Count$CastTotalManaSpent" | "Count$TotalManaSpent"
        ) {
            return Self::mana_spent_condition(cmp);
        }
        self.svar_condition(trimmed_var, cmp)
            .or_else(|| self.extra_svar_condition_clause(trimmed_var, cmp))
            .or_else(|| self.extra_svar_condition_wave8(trimmed_var, cmp))
            .or_else(|| self.wave15_trigger_svar_condition(trimmed_var, cmp))
            .or_else(|| self.wave28b_etb_svar_condition(trimmed_var, cmp))
            .or_else(|| {
                (explicit_cmp.is_some()
                    || def
                        .strip_prefix("Count$Valid ")
                        .is_some_and(|rest| rest.contains('+') && !rest.contains('$')))
                .then(|| self.inline_valid_count_condition(trimmed_var, cmp))
                .flatten()
            })
    }

    fn etb_compare_condition(&mut self, def: &str, cmp: &str, depth: u8) -> Option<String> {
        let rest = def.strip_prefix("Count$Compare ")?;
        let (lhs, else_arm) = rest.rsplit_once('.')?;
        let (cond_spec, then_arm) = lhs.rsplit_once('.')?;
        let flag_true = match (then_arm.trim(), else_arm.trim()) {
            ("1", "0") => true,
            ("0", "1") => false,
            _ => return None,
        };
        let want_true = match cmp {
            "GE1" | "GT0" | "EQ1" => true,
            "EQ0" | "LE0" | "LT1" => false,
            _ => return None,
        };
        let (inner_var, inner_cmp) = cond_spec.trim().rsplit_once(' ')?;
        let inner = self.etb_check_svar_condition_at(inner_var, Some(inner_cmp), depth + 1)?;
        if flag_true == want_true {
            Some(inner)
        } else {
            Some(self.negate_etb_condition(&inner))
        }
    }

    fn mana_spent_condition(cmp: &str) -> Option<String> {
        match cmp {
            "EQ0" | "LE0" | "LT1" => Some(
                "Condition::Not(&Condition::AmountAtLeast(Amount::ManaSpentToCast, 1))".to_string(),
            ),
            "NE0" | "GE1" | "GT0" => {
                Some("Condition::AmountAtLeast(Amount::ManaSpentToCast, 1)".to_string())
            }
            _ => {
                let min =
                    if let Some(n) = cmp.strip_prefix("GE").and_then(|v| v.parse::<u32>().ok()) {
                        (n >= 1).then_some(n)
                    } else {
                        cmp.strip_prefix("GT")
                            .and_then(|v| v.parse::<u32>().ok())
                            .and_then(|v| v.checked_add(1))
                    }?;
                Some(format!(
                    "Condition::AmountAtLeast(Amount::ManaSpentToCast, {min})"
                ))
            }
        }
    }

    /// `K:Impending:<N>:<cost>` (`CR 702.176a`): mana-only alternative cost on
    /// a creature permanent that enters with `N` time counters and strips
    /// creature characteristics while any remain.
    fn impending_keyword(&mut self, rest: &str) -> Option<()> {
        if self.body.impending.is_some() {
            return self.deny("a second `Impending` cost on one card".to_string());
        }
        let is_creature = self
            .type_line
            .split_whitespace()
            .any(|word| word == "Creature");
        if !is_creature || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny("`Impending` on a non-creature card".to_string());
        }
        let Some((count_raw, cost_raw)) = rest.split_once(':') else {
            return self.deny("keyword `Impending`".to_string());
        };
        if cost_raw.contains(':') {
            return self.deny("keyword `Impending`".to_string());
        }
        let Ok(n) = count_raw.trim().parse::<u8>() else {
            return self.deny("keyword `Impending`".to_string());
        };
        if n == 0 {
            return self.deny("`Impending 0`".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Impending", cost_raw.trim(), false, true)?;
        self.body.impending = Some(format!("Some(({n}, mana!(\"{mana}\")))"));
        self.body.notes.push("impending".to_string());
        Some(())
    }

    /// `K:Gift` (`CR 702.174a`): optional additional cost promising the named
    /// gift from `SVar:GiftAbility` to a chosen opponent.
    fn gift_keyword(&mut self) -> Option<()> {
        if self.body.gift.is_some() {
            return self.deny("a second `Gift` keyword on one card".to_string());
        }
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Gift` on a land or untyped card".to_string());
        }
        let Some(raw) = self.svars.get("GiftAbility").map(|s| s.trim().to_string()) else {
            return self.deny("`Gift` without `SVar:GiftAbility`".to_string());
        };
        let Some((api, mut gp)) = Params::parse(&raw) else {
            return self.deny("malformed `SVar:GiftAbility`".to_string());
        };
        let desc = gp.take("GiftDescription");
        let kind = match api.as_str() {
            "Draw" => {
                if gp.take("Defined").as_deref() != Some("Promised")
                    || !matches!(gp.take("NumCards").as_deref(), None | Some("1"))
                    || !matches!(desc.as_deref(), None | Some("a card"))
                {
                    return self.deny("unsupported `Gift` draw ability".to_string());
                }
                "GiftKind::Card"
            }
            "AddTurn" => {
                if gp.take("Defined").as_deref() != Some("Promised")
                    || gp.take("NumTurns").as_deref() != Some("1")
                    || !matches!(desc.as_deref(), None | Some("an extra turn"))
                {
                    return self.deny("unsupported `Gift` extra turn ability".to_string());
                }
                "GiftKind::ExtraTurn"
            }
            "Token" => {
                if gp.take("TokenOwner").as_deref() != Some("Promised")
                    || !matches!(gp.take("TokenAmount").as_deref(), None | Some("1"))
                    || !matches!(gp.take("LockTokenScript").as_deref(), None | Some("True"))
                {
                    return self.deny("unsupported `Gift` token ability".to_string());
                }
                let script = gp.take("TokenScript");
                let tapped = gp.take("TokenTapped");
                match (script.as_deref(), tapped.as_deref(), desc.as_deref()) {
                    (Some("c_a_food_sac"), None | Some("False"), None | Some("a Food")) => {
                        "GiftKind::Food"
                    }
                    (Some("c_a_treasure_sac"), None | Some("False"), None | Some("a Treasure")) => {
                        "GiftKind::Treasure"
                    }
                    (Some("u_1_1_fish"), Some("True"), None | Some("a tapped Fish")) => {
                        "GiftKind::TappedFish"
                    }
                    (Some("u_8_8_octopus"), None | Some("False"), None | Some("an Octopus")) => {
                        "GiftKind::Octopus"
                    }
                    _ => return self.deny("unsupported `Gift` token kind".to_string()),
                }
            }
            _ => return self.deny(format!("unsupported `Gift` ability `{api}`")),
        };
        if let Some(leftover) = gp.first_key() {
            return self.deny(format!("unclaimed `GiftAbility` parameter `{leftover}`"));
        }
        self.body.gift = Some(format!("Some({kind})"));
        self.body.notes.push("gift".to_string());
        Some(())
    }

    fn etb_wave6_amount(&mut self, raw: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return (n > 0).then(|| format!("Amount::Fixed({n})"));
        }
        let def = if raw.starts_with("Count$") {
            raw.trim().to_string()
        } else {
            self.svars.get(raw)?.trim().to_string()
        };
        self.etb_wave6_expr(&def, 0)
    }

    fn etb_wave6_expr(&mut self, def: &str, depth: u8) -> Option<String> {
        if depth >= 4 {
            return None;
        }
        let def = def.trim();
        if let Some(var) = def.strip_prefix("SVar$")
            && !var.contains('/')
        {
            let inner = self.svars.get(var.trim())?.clone();
            return self.etb_wave6_expr(&inner, depth + 1);
        }
        if matches!(def, "Count$CastTotalManaSpent" | "Count$TotalManaSpent") {
            return Some("Amount::ManaSpentToCast".to_string());
        }
        if def == "Count$CastTotalManaSpent Snow" {
            return Some("Amount::SnowManaSpentToCast".to_string());
        }
        if def == "Count$Devoured Card" {
            return Some("Amount::EntrySacrifices".to_string());
        }
        if def == "Count$TimesKicked" {
            return Some("Amount::TimesKicked".to_string());
        }
        if matches!(def, "Count$Party" | "Count$PartySize") {
            return Some("Amount::PartySize(PlayerRel::You)".to_string());
        }
        if def == "Count$ThisTurnCast_Card.YouCtrl" {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::You)".to_string());
        }
        if def == "Count$ThisTurnCast_Card.OppCtrl" {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::EachOpponent)".to_string());
        }
        if def == "Count$ThisTurnCast_Card" {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::EachPlayer)".to_string());
        }
        if def == "Count$YouDrewThisTurn" {
            return Some("Amount::CardsDrawnThisTurn(PlayerRel::You)".to_string());
        }
        if let Some(expr) = Self::wave26f_devotion_amount(def) {
            return Some(expr);
        }
        if let Some(expr) = self.wave26f_greatest_or_total_amount(def, false, true) {
            return Some(expr);
        }
        if let Some(expr) = self.wave27e_etb_amount(def) {
            return Some(expr);
        }
        if let Some(expr) = self.wave16a_etb_amount(def) {
            return Some(expr);
        }
        if def == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" {
            return Some("Amount::CreaturesDiedThisTurn".to_string());
        }
        if def == "Count$ValidHand Card" {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::HandOf(PlayerRel::EachPlayer) }"
                    .to_string(),
            );
        }
        if def == "Count$xPaid" && self.has_x_mana_cost {
            return Some("Amount::X".to_string());
        }
        if let Some((lhs, rhs)) = def.split_once('/')
            && let Some(expr) = self.etb_wave6_slash_expr(lhs, rhs.trim(), depth)
        {
            return Some(expr);
        }
        let normalized = if let Some(spec) = def
            .strip_prefix("Count$LastStateBattlefield ")
            .or_else(|| def.strip_prefix("Count$Valid "))
        {
            format!("Count$Valid {}", Self::strip_etb_other_valid(spec.trim()))
        } else if let Some(spec) = def.strip_prefix("Count$LastStateGraveyard ") {
            format!("Count$ValidGraveyard {}", spec.trim())
        } else {
            def.to_string()
        };
        let expr = self.static_count_expr(&normalized)?;
        if expr.contains("SourcePower") || expr.contains("CountersOnSource") {
            return None;
        }
        Some(expr)
    }

    fn etb_wave6_slash_expr(&mut self, lhs: &str, rhs: &str, depth: u8) -> Option<String> {
        if rhs == "Twice" {
            let base = self.etb_wave6_expr(lhs, depth + 1)?;
            return Some(if base == "Amount::X" {
                "Amount::DoubleX".to_string()
            } else {
                format!("Amount::Times {{ base: &{base}, factor: 2 }}")
            });
        }
        if let Some(arg) = rhs.strip_prefix("Plus.")
            && arg.trim().parse::<u32>().is_err()
        {
            let base = self.etb_wave6_expr(lhs, depth + 1)?;
            let other = self.etb_wave6_operand(arg.trim(), depth + 1)?;
            return Some(format!("Amount::Sum {{ left: &{base}, right: &{other} }}"));
        }
        if let Some(arg) = rhs.strip_prefix("Minus.")
            && arg.trim().parse::<u32>().is_err()
        {
            let base = self.etb_wave6_expr(lhs, depth + 1)?;
            let other = self.etb_wave6_operand(arg.trim(), depth + 1)?;
            return Some(format!(
                "Amount::Difference {{ left: &{base}, right: &{other} }}"
            ));
        }
        let (kind, arg) = if let Some(n) = rhs.strip_prefix("Times.") {
            (0u8, n.trim().parse::<u32>().ok().filter(|&v| v >= 1)?)
        } else if let Some(n) = rhs.strip_prefix("Plus.") {
            (1u8, n.trim().parse::<u32>().ok()?)
        } else if let Some(n) = rhs.strip_prefix("Minus.") {
            (2u8, n.trim().parse::<u32>().ok()?)
        } else if rhs == "HalfDown" {
            (3u8, 0)
        } else if rhs == "HalfUp" {
            (4u8, 0)
        } else {
            let n = rhs.strip_prefix("Div.")?;
            (5u8, n.trim().parse::<u32>().ok().filter(|&v| v >= 2)?)
        };
        let base = self.etb_wave6_expr(lhs, depth + 1)?;
        Some(match kind {
            0 => format!("Amount::Times {{ base: &{base}, factor: {arg} }}"),
            1 => format!("Amount::Plus {{ base: &{base}, offset: {arg} }}"),
            2 => format!("Amount::SaturatingSub {{ base: &{base}, subtract: {arg} }}"),
            3 => format!("Amount::Half {{ base: &{base}, round_up: false }}"),
            4 => format!("Amount::Half {{ base: &{base}, round_up: true }}"),
            5 => format!("Amount::divided(&{base}, {arg}, false)"),
            _ => unreachable!(),
        })
    }

    fn etb_wave6_operand(&mut self, raw: &str, depth: u8) -> Option<String> {
        if depth >= 4 || raw.is_empty() {
            return None;
        }
        if let Ok(n) = raw.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        let inner = if raw.starts_with("Count$") {
            raw.to_string()
        } else {
            self.svars.get(raw)?.trim().to_string()
        };
        self.etb_wave6_expr(&inner, depth + 1)
    }

    /// CR 614.12 / CR 603.6b: an "as this enters" counter replacement is
    /// evaluated before the entering permanent is on the battlefield, so
    /// stripping `Other` / `StrictlyOther` from plain battlefield counts is
    /// exact.
    fn strip_etb_other_valid(spec: &str) -> String {
        if spec.contains('$') || spec.contains("Self") || spec.contains("StrictlySelf") {
            return spec.to_string();
        }
        spec.split(',')
            .map(|alt| {
                let alt = alt.trim();
                let Some((base, atoms)) = alt.split_once('.') else {
                    return alt.to_string();
                };
                let kept: Vec<&str> = atoms
                    .split('+')
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && *a != "Other" && *a != "StrictlyOther")
                    .collect();
                if kept.is_empty() {
                    base.trim().to_string()
                } else {
                    format!("{}.{}", base.trim(), kept.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    /// `K:Multikicker:<cost>` (CR 702.33c). Every printed one is mana.
    pub(super) fn multikicker(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Multikicker` with extra fields `{rest}`"));
        }
        if !self.body.additional_costs.is_empty() || self.body.multikicker.is_some() {
            return self.deny("a second `Kicker` cost".to_string());
        }
        let (mana, parts) = self.cost_pieces(rest)?;
        if mana.is_empty() || !parts.is_empty() {
            return self.deny(format!("a non-mana `Multikicker` cost `{rest}`"));
        }
        self.body.multikicker = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("multikicker".to_string());
        Some(())
    }

    /// `K:Foretell:<cost>` for a mana-only later cast cost (CR 702.143a).
    /// The {2} special action is supplied by the engine, not by this field.
    pub(super) fn foretell(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Foretell` with extra fields `{rest}`"));
        }
        let (mana, parts) = self.cost_pieces(rest)?;
        if mana.is_empty() || !parts.is_empty() {
            return self.deny(format!("a non-mana `Foretell` cost `{rest}`"));
        }
        self.body.foretell = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("foretell".to_string());
        Some(())
    }

    fn is_land_card_type(&self) -> bool {
        self.type_line.split_whitespace().any(|word| word == "Land")
    }

    fn parse_cost_pieces_with(
        &mut self,
        keyword: &str,
        rest: &str,
        allow_x: bool,
        mana_only: bool,
        part_ok: fn(&str) -> bool,
    ) -> Option<(String, Vec<String>)> {
        if rest.contains(':') {
            return self.deny(format!("a `{keyword}` with extra fields `{rest}`"));
        }
        if super::cost_parts(rest).is_empty() {
            return self.deny(format!("an empty `{keyword}` cost"));
        }
        let prev_x = std::mem::replace(&mut self.has_x, allow_x);
        let prev_alt = std::mem::replace(&mut self.in_alternative_cost, true);
        let pieces = self.cost_pieces(rest);
        self.has_x = prev_x;
        self.in_alternative_cost = prev_alt;
        let (mana, parts) = pieces?;
        if mana_only && (mana.is_empty() || !parts.is_empty()) {
            return self.deny(format!("a non-mana `{keyword}` cost `{rest}`"));
        }
        if !allow_x && mana.contains('X') {
            return self.deny(format!("variable `X` in `{keyword}` cost `{rest}`"));
        }
        if (mana.is_empty() && parts.is_empty()) || !parts.iter().all(|part| part_ok(part)) {
            return self.deny(format!("a `{keyword}` cost `{rest}` the cast cannot pay"));
        }
        if !mana.is_empty() && baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!(
                "a `{keyword}` cost `{rest}` that is not valid mana"
            ));
        }
        Some((mana, parts))
    }

    fn parse_alt_cost_pieces(
        &mut self,
        keyword: &str,
        rest: &str,
        allow_x: bool,
        mana_only: bool,
    ) -> Option<(String, Vec<String>)> {
        self.parse_cost_pieces_with(
            keyword,
            rest,
            allow_x,
            mana_only,
            is_paid_alternative_cost_part,
        )
    }

    pub(super) fn optional_permanent_cost_keyword(
        &mut self,
        kw: &str,
        rest: &str,
        already_set: bool,
    ) -> Option<String> {
        if self.type_line.is_empty() || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny(format!("`{kw}` on a non-permanent card"));
        }
        if already_set {
            return self.deny(format!("a second `{kw}` on one card"));
        }
        let (mana, parts) =
            self.parse_cost_pieces_with(kw, rest, false, false, is_paid_additional_cost_part)?;
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.notes.push(kw.to_ascii_lowercase());
        Some(format!("Some({cost})"))
    }

    /// `K:MayFlashCost:<cost>`: an optional additional mana cost that also
    /// grants flash (CR 702.8a) on the ordinary cast from hand.
    fn may_flash_cost(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`MayFlashCost` on a land or untyped card".to_string());
        }
        if self.body.optional_cost.is_some() || self.body.optional_cost_grants_flash {
            return self.deny("a second optional additional cost on one face".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("MayFlashCost", rest, false, true)?;
        let cost = crate::body::cost_literal(&mana, &[]);
        self.body.optional_cost = Some(format!("Some({cost})"));
        self.body.optional_cost_grants_flash = true;
        self.body.notes.push("optional flash cost".to_string());
        Some(())
    }

    /// `K:Miracle:<cost>` (`CR 702.94a`): mana-only alternative cast cost when
    /// revealed as the first card drawn this turn (with optional `{X}`).
    pub(super) fn miracle(&mut self, rest: &str) -> Option<()> {
        if self.body.miracle.is_some() {
            return self.deny("a second `Miracle` cost on one card".to_string());
        }
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Miracle` on a land or untyped card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Miracle", rest, true, true)?;
        self.body.miracle = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("miracle".to_string());
        Some(())
    }

    /// `K:Dash:<cost>` (`CR 702.109a`): mana-only alternative cast cost for
    /// permanent cards (gaining haste and end-step return in the engine).
    pub(super) fn dash(&mut self, rest: &str) -> Option<()> {
        if self.body.dash.is_some() {
            return self.deny("a second `Dash` cost on one card".to_string());
        }
        if self.type_line.is_empty() || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny("`Dash` on a non-permanent card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Dash", rest, false, true)?;
        self.body.dash = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("dash".to_string());
        Some(())
    }

    /// `K:Blitz:<cost>` (`CR 702.152a`): alternative cast cost for permanent
    /// cards (`mana` + `paid_as_an_alternative_cost` parts).
    pub(super) fn blitz(&mut self, rest: &str) -> Option<()> {
        if self.body.blitz.is_some() {
            return self.deny("a second `Blitz` cost on one card".to_string());
        }
        if self.type_line.is_empty() || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny("`Blitz` on a non-permanent card".to_string());
        }
        let (mana, parts) = self.parse_alt_cost_pieces("Blitz", rest, false, false)?;
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.blitz = Some(format!("Some({cost})"));
        self.body.notes.push("blitz".to_string());
        Some(())
    }

    /// `K:Spectacle:<cost>` (`CR 702.137a`), `K:Surge:<cost>` (`CR 702.117a`),
    /// and `K:Prowl:<cost>` (`CR 702.76a`) via `AlternativeCost::<ctor>(cost)`.
    fn keyword_alternative_cost(
        &mut self,
        keyword: &str,
        ctor: &str,
        rest: &str,
        allow_x: bool,
    ) -> Option<()> {
        if self.has_evoke || !self.body.alternative_costs.is_empty() {
            return self.deny(format!("a conflicting `{keyword}` alternative cost"));
        }
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny(format!("`{keyword}` on a land or untyped card"));
        }
        let (mana, parts) = self.parse_alt_cost_pieces(keyword, rest, allow_x, false)?;
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body
            .alternative_costs
            .push(format!("AlternativeCost::{ctor}({cost})"));
        self.body.notes.push(ctor.to_string());
        Some(())
    }

    fn is_spell_card_type(&self) -> bool {
        self.type_line
            .split_whitespace()
            .any(|word| matches!(word, "Instant" | "Sorcery"))
    }

    pub(super) fn jump_start(&mut self) -> Option<()> {
        if !self.is_spell_card_type() {
            return self.deny("`Jump-start` on a non-instant/non-sorcery card".to_string());
        }
        if self.body.jump_start {
            return self.deny("a second `Jump-start` on one card".to_string());
        }
        self.body.jump_start = true;
        self.body.notes.push("jump-start".to_string());
        Some(())
    }

    /// `K:Retrace` (`CR 702.81a`): non-land card castable from the graveyard by
    /// discarding a land card in addition to paying its other costs.
    fn retrace(&mut self) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Retrace` on a land or typeless card".to_string());
        }
        if self
            .body
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::RETRACE")
        {
            return self.deny("a second `Retrace` on one card".to_string());
        }
        self.body.keywords.push("KeywordSet::RETRACE".to_string());
        self.body.notes.push("retrace".to_string());
        Some(())
    }

    pub(super) fn aftermath(&mut self) -> Option<()> {
        if !self.is_spell_card_type() {
            return self.deny("`Aftermath` on a non-instant/non-sorcery card".to_string());
        }
        if self.body.aftermath {
            return self.deny("a second `Aftermath` on one card".to_string());
        }
        self.body.aftermath = true;
        self.body.notes.push("aftermath".to_string());
        Some(())
    }

    pub(super) fn fuse(&mut self) -> Option<()> {
        if !self.is_split_front {
            return self.deny("`Fuse` outside the front half of a `Split` card".to_string());
        }
        if !super::is_pure_spell_face(self.type_line) {
            return self.deny("`Fuse` on a non-instant/non-sorcery face".to_string());
        }
        if self.body.fuse {
            return self.deny("a second `Fuse` on one card".to_string());
        }
        self.body.fuse = true;
        self.body.notes.push("fuse".to_string());
        Some(())
    }

    pub(super) fn disturb(&mut self, rest: &str) -> Option<()> {
        if !self.is_double_faced_front {
            return self
                .deny("`Disturb` outside the front face of a `DoubleFaced` card".to_string());
        }
        if self.type_line.is_empty() || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny("`Disturb` on a non-permanent front face".to_string());
        }
        if self.body.disturb.is_some() {
            return self.deny("a second `Disturb` on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Disturb", rest, false, true)?;
        self.body.disturb = Some(mana);
        self.body.notes.push("disturb".to_string());
        Some(())
    }

    pub(super) fn escape(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Escape` on a land or untyped card".to_string());
        }
        if self.body.escape.is_some() {
            return self.deny("a second `Escape` on one card".to_string());
        }
        if rest.contains(':') {
            return self.deny(format!("an `Escape` with extra fields `{rest}`"));
        }
        let mut mana_tokens = Vec::new();
        let mut exile_count: Option<u8> = None;
        for token in super::cost_parts(rest) {
            if let Some(inner) = token
                .strip_prefix("ExileFromGrave<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let mut fields = inner.splitn(3, '/');
                let (Some(count_str), Some(filter_str)) = (fields.next(), fields.next()) else {
                    return self.deny(format!("malformed `Escape` exile cost `{token}`"));
                };
                if filter_str != "Card.Other" {
                    return self.deny(format!("`Escape` exile filter `{filter_str}`"));
                }
                let Ok(n) = count_str.parse::<u8>() else {
                    return self.deny(format!(
                        "variable or invalid `Escape` exile count `{count_str}`"
                    ));
                };
                if n == 0 || exile_count.is_some() {
                    return self.deny(format!("invalid `Escape` exile count `{rest}`"));
                }
                exile_count = Some(n);
            } else {
                mana_tokens.push(token);
            }
        }
        let Some(exile) = exile_count else {
            return self.deny(format!("`Escape` without `ExileFromGrave` `{rest}`"));
        };
        let mana_raw = mana_tokens.join(" ");
        let (mana, _) = self.parse_alt_cost_pieces("Escape", &mana_raw, false, true)?;
        self.body.escape = Some(format!(
            "Some(Escape {{ cost: mana!(\"{mana}\"), exile: {exile} }})"
        ));
        self.body.notes.push("escape".to_string());
        Some(())
    }

    pub(super) fn alternate_additional_cost(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`AlternateAdditionalCost` on a land or untyped card".to_string());
        }
        let Some((lhs, rhs)) = split_two_top_level_colons(rest) else {
            return self.deny("keyword `AlternateAdditionalCost`".to_string());
        };
        if let Some(inner) = lhs
            .trim()
            .strip_prefix("Behold<")
            .and_then(|s| s.strip_suffix('>'))
        {
            if self.body.behold_or_pay.is_some() {
                return self.deny("a second `BeholdOrPay` cost on one card".to_string());
            }
            let mut fields = inner.splitn(3, '/');
            let (Some(count_str), Some(valid)) = (fields.next(), fields.next()) else {
                return self.deny(format!("malformed `Behold` cost `{lhs}`"));
            };
            if count_str.trim() != "1" {
                return self.deny(format!("`Behold` count `{count_str}`"));
            }
            let expr = self.filter_expr(valid.trim())?;
            let (mana, _) =
                self.parse_alt_cost_pieces("AlternateAdditionalCost", rhs, false, true)?;
            let filter_name = self.body.filter_static("BEHOLD", &expr);
            self.body.behold_or_pay = Some(format!(
                "Some(BeholdOrPay {{ filter: &{filter_name}, cost: mana!(\"{mana}\") }})"
            ));
            self.body.notes.push("behold or pay".to_string());
            return Some(());
        }
        if self
            .body
            .mandatory_additional_costs
            .iter()
            .any(|part| part.starts_with("CostPart::Either("))
        {
            return self.deny("a second `AlternateAdditionalCost` on one card".to_string());
        }
        let (mana_a, parts_a) = self.parse_cost_pieces_with(
            "AlternateAdditionalCost",
            lhs.trim(),
            false,
            false,
            is_spell_either_cost_part,
        )?;
        let (mana_b, parts_b) = self.parse_cost_pieces_with(
            "AlternateAdditionalCost",
            rhs.trim(),
            false,
            false,
            is_spell_either_cost_part,
        )?;
        let cost_a = crate::body::cost_literal(&mana_a, &parts_a);
        let cost_b = crate::body::cost_literal(&mana_b, &parts_b);
        self.body
            .mandatory_additional_costs
            .push(format!("CostPart::Either(&{cost_a}, &{cost_b})"));
        self.body
            .notes
            .push("alternate additional cost".to_string());
        Some(())
    }

    pub(super) fn mutate(&mut self, rest: &str) -> Option<()> {
        if !self.type_line.split_whitespace().any(|w| w == "Creature")
            || self.is_land_card_type()
            || self.is_spell_card_type()
        {
            return self.deny("`Mutate` on a non-creature card".to_string());
        }
        if self.body.mutate.is_some() {
            return self.deny("a second `Mutate` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Mutate", rest, false, true)?;
        self.body.mutate = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("mutate".to_string());
        Some(())
    }

    pub(super) fn plot(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Plot` on a land or untyped card".to_string());
        }
        if self.body.plot.is_some() {
            return self.deny("a second `Plot` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Plot", rest, false, true)?;
        self.body.plot = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("plot".to_string());
        Some(())
    }

    pub(super) fn casualty(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Casualty` on a land or untyped card".to_string());
        }
        if self.body.casualty.is_some() {
            return self.deny("a second `Casualty` keyword on one card".to_string());
        }
        let trimmed = rest.trim();
        if trimmed.contains(':') {
            return self.deny(format!("`Casualty:{rest}` with extra modifiers"));
        }
        let Ok(n) = trimmed
            .parse::<u8>()
            .map_err(|_| ())
            .and_then(|n| if n >= 1 { Ok(n) } else { Err(()) })
        else {
            return self.deny(format!("invalid `Casualty` power threshold `{trimmed}`"));
        };
        self.body.casualty = Some(format!("Some({n})"));
        self.body.notes.push("casualty".to_string());
        Some(())
    }

    fn require_creature_permanent(&mut self, kw: &str) -> Option<()> {
        if !self.type_line.split_whitespace().any(|w| w == "Creature")
            || self.is_land_card_type()
            || self.is_spell_card_type()
        {
            return self.deny(format!("`{kw}` on a non-creature card"));
        }
        Some(())
    }

    /// `K:Warp:<cost>` (`CR 702.185a`): alternative cast cost for non-land permanent
    /// cards (`mana` + `paid_as_an_alternative_cost` parts + optional `{X}`).
    fn warp_keyword(&mut self, rest: &str) -> Option<()> {
        if self.body.warp.is_some() {
            return self.deny("a second `Warp` cost on one card".to_string());
        }
        self.require_permanent_keyword("Warp")?;
        let (mana, parts) = self.parse_alt_cost_pieces("Warp", rest, true, false)?;
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.warp = Some(format!("Some({cost})"));
        self.body.keywords.push("KeywordSet::WARP".to_string());
        self.body.notes.push("warp".to_string());
        Some(())
    }

    /// `K:Bloodthirst:<N|X>` (`CR 702.54a–c`): creature entry replacement + keyword bit.
    fn bloodthirst_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Bloodthirst")?;
        let trimmed = rest.trim();
        if trimmed.contains(':') || trimmed.is_empty() {
            return self.deny(format!("keyword `Bloodthirst:{rest}`"));
        }
        let amount = if trimmed == "X" {
            "Amount::DamageDealtToOpponentsThisTurn".to_string()
        } else if let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n > 0) {
            format!("Amount::Fixed({n})")
        } else {
            return self.deny(format!("keyword `Bloodthirst:{rest}`"));
        };
        if !self
            .body
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::BLOODTHIRST")
        {
            self.body
                .keywords
                .push("KeywordSet::BLOODTHIRST".to_string());
        }
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::Bloodthirst({amount})"));
        Some(())
    }

    /// `K:Devour:<N|X>[:quality[:prose]]` (`CR 702.82a–c`): sacrifice fodder as it enters.
    fn devour_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Devour")?;
        let mut fields = rest.split(':');
        let factor_raw = fields.next()?.trim();
        let quality = fields.next().map_or("", str::trim);
        let _prose = fields.next();
        if fields.next().is_some() {
            return self.deny(format!("keyword `Devour:{rest}`"));
        }
        let factor: u8 = if factor_raw == "X" {
            0
        } else {
            match factor_raw.parse::<u8>() {
                Ok(n) if n >= 1 => n,
                _ => return self.deny(format!("keyword `Devour:{rest}`")),
            }
        };
        let entry = if quality.is_empty() {
            format!("EnterModifier::devour({factor})")
        } else {
            if quality.contains("Ctrl")
                || quality.contains("Own")
                || quality.contains("Self")
                || quality.contains("Other")
            {
                return self.deny(format!("relative `Devour` quality `{quality}`"));
            }
            let normalized = quality.replace(';', ",");
            let expr = self.filter_expr(&normalized)?;
            let filter = self.body.filter_static("DEVOUR", &expr);
            format!("EnterModifier::Devour {{ filter: &{filter}, factor: {factor} }}")
        };
        if !self.body.keywords.iter().any(|k| k == "KeywordSet::DEVOUR") {
            self.body.keywords.push("KeywordSet::DEVOUR".to_string());
        }
        self.body.enter_modifiers.push(entry);
        Some(())
    }

    /// `K:Spree` (`CR 702.172a`): modal spell keyword on an instant or sorcery.
    fn spree_keyword(&mut self) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Spree` on a non-instant/non-sorcery card".to_string());
        }
        if self.body.keywords.iter().any(|k| k == "KeywordSet::SPREE") {
            return self.deny("a second `Spree` keyword on one card".to_string());
        }
        self.body.keywords.push("KeywordSet::SPREE".to_string());
        self.body.notes.push("spree".to_string());
        Some(())
    }

    /// `K:Fabricate:<N>` (`CR 702.114a–b`): creature ETB choice of counters or Servos.
    fn fabricate_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Fabricate")?;
        if rest.contains(':') {
            return self.deny(format!("keyword `Fabricate:{rest}`"));
        }
        let Some(count) = rest.trim().parse::<u16>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Fabricate:{rest}`"));
        };
        let Some(tokens) = self.tokens else {
            return self.deny(
                "`Fabricate` with no token scripts to read `c_1_1_a_servo` against".to_string(),
            );
        };
        let Some(tok) = tokens.body("c_1_1_a_servo", self.cats) else {
            return self.deny("token script `c_1_1_a_servo` for `Fabricate`".to_string());
        };
        let token = format!("&generated_tokens::{}", tok.constant);
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::Fabricate {{ count: {count}, token: {token} }}])"
        ));
        Some(())
    }

    /// `K:Hideaway:<N>` (`CR 702.75a–b`): permanent ETB hidden exile from top N cards.
    fn hideaway_keyword(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_spell_card_type() {
            return self.deny("`Hideaway` on a non-permanent card".to_string());
        }
        if rest.contains(':') {
            return self.deny(format!("keyword `Hideaway:{rest}`"));
        }
        let Some(n) = rest.trim().parse::<u8>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Hideaway:{rest}`"));
        };
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::Hideaway({n})])"
        ));
        Some(())
    }

    /// `K:Craft:<cost>` (`CR 702.167a–c`): front-face sorcery-speed exile-and-return-transformed.
    fn craft_keyword(&mut self, rest: &str) -> Option<()> {
        if !self.is_double_faced_front || self.is_land_card_type() || self.is_spell_card_type() {
            return self.deny(
                "`Craft` outside the front permanent face of a `DoubleFaced` card".to_string(),
            );
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.contains("ReturnCraftedTransformed"))
        {
            return self.deny("a second `Craft` ability on one card".to_string());
        }
        let Some(main_spec) = Self::split_craft_spec(rest) else {
            return self.deny(format!("malformed `Craft` specification `{rest}`"));
        };
        let tokens = super::cost_parts(main_spec);
        if tokens.is_empty() {
            return self.deny("an empty `Craft` cost".to_string());
        }
        let (mana, xmin, material_tokens) = self.split_craft_tokens(&tokens, rest)?;
        let mut parts = vec!["ExileSelf".to_string()];
        let mut used_xmin = false;
        for tok in material_tokens {
            let (part, uses_x) = self.parse_craft_material(tok, xmin)?;
            if uses_x {
                if used_xmin {
                    return self.deny(format!(
                        "multiple variable-count `Craft` materials `{rest}`"
                    ));
                }
                used_xmin = true;
            }
            parts.push(part);
        }
        if xmin.is_some() && !used_xmin {
            return self.deny(format!("unused `XMin` in `Craft:{rest}`"));
        }
        if parts.len() < 2 {
            return self.deny(format!("`Craft` without material costs `{rest}`"));
        }
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::ReturnCraftedTransformed], timing = ActivationTiming::SorcerySpeed)"
        ));
        self.body.notes.push("craft".to_string());
        Some(())
    }

    fn split_craft_spec(rest: &str) -> Option<&str> {
        let mut depth = 0usize;
        let mut splits = Vec::new();
        for (i, ch) in rest.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                ':' if depth == 0 => splits.push(i),
                _ => {}
            }
        }
        if splits.len() > 2 {
            return None;
        }
        Some(match splits.first().copied() {
            Some(idx) => rest[..idx].trim(),
            None => rest.trim(),
        })
    }

    fn split_craft_tokens<'b>(
        &self,
        tokens: &'b [String],
        full: &str,
    ) -> Option<(String, Option<u8>, Vec<&'b str>)> {
        let mut mana = String::new();
        let mut xmin: Option<u8> = None;
        let mut materials = Vec::new();
        for token in tokens {
            let t = token.as_str();
            if let Some(n_str) = t.strip_prefix("XMin") {
                if xmin.is_some() {
                    return self.deny(format!("duplicate `XMin` in `Craft:{full}`"));
                }
                let Some(k) = n_str.parse::<u8>().ok().filter(|&k| k >= 1) else {
                    return self.deny(format!("invalid `XMin` in `Craft:{full}`"));
                };
                xmin = Some(k);
            } else if t.chars().all(|c| c.is_ascii_digit())
                || matches!(t, "W" | "U" | "B" | "R" | "G" | "C" | "S")
            {
                mana.push('{');
                mana.push_str(t);
                mana.push('}');
            } else if let Some(pair) = super::hybrid_pair(t) {
                mana.push_str(&pair);
            } else if t.starts_with("ExileCtrlOrGrave<") || t.starts_with("ExileFromGrave<") {
                materials.push(t);
            } else {
                return self.deny(format!("unsupported `Craft` cost token `{t}`"));
            }
        }
        if !mana.is_empty() && baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("invalid mana in `Craft:{full}`"));
        }
        Some((mana, xmin, materials))
    }

    fn parse_craft_material(&mut self, token: &str, xmin: Option<u8>) -> Option<(String, bool)> {
        let (grave_only, body) = if let Some(b) = token
            .strip_prefix("ExileCtrlOrGrave<")
            .and_then(|s| s.strip_suffix('>'))
        {
            (false, b)
        } else if let Some(b) = token
            .strip_prefix("ExileFromGrave<")
            .and_then(|s| s.strip_suffix('>'))
        {
            (true, b)
        } else {
            return self.deny(format!("`Craft` material `{token}`"));
        };
        let Some((count_str, rest_body)) = body.split_once('/') else {
            return self.deny(format!("malformed `Craft` material `{token}`"));
        };
        let valid_raw = rest_body.split('/').next().unwrap_or("").trim();
        if valid_raw.is_empty() {
            return self.deny(format!("empty `Craft` material filter `{token}`"));
        }
        let (min, max, uses_x) = if count_str == "X" {
            let Some(k) = xmin else {
                return self.deny(format!("variable `Craft` without `XMin` `{token}`"));
            };
            (k, u8::MAX, true)
        } else {
            let Some(n) = count_str.parse::<u8>().ok().filter(|&n| n >= 1) else {
                return self.deny(format!("invalid `Craft` material count `{token}`"));
            };
            (n, n, false)
        };
        let normalized = valid_raw.replace(';', ",");
        let mut expr = self.filter_expr_in_zone(&normalized, true)?;
        if grave_only {
            expr = match expr
                .strip_prefix("Filter::And(&[")
                .and_then(|s| s.strip_suffix("])"))
            {
                Some(clauses) => {
                    format!("Filter::And(&[Filter::InZone(ZoneRef::Graveyard), {clauses}])")
                }
                None => format!("Filter::And(&[Filter::InZone(ZoneRef::Graveyard), {expr}])"),
            };
        }
        let filter_ref = self.body.filter_static("COST", &expr);
        Some((
            format!("CraftMaterials {{ filter: &{filter_ref}, min: {min}, max: {max} }}"),
            uses_x,
        ))
    }

    pub(super) fn backup(&mut self, rest: &str) -> Option<()> {
        if !self.type_line.split_whitespace().any(|w| w == "Creature")
            || self.is_land_card_type()
            || self.is_spell_card_type()
        {
            return self.deny("`Backup` on a non-creature card".to_string());
        }
        let Some((count_raw, svar_raw)) = rest.split_once(':') else {
            return self.deny(format!("malformed `Backup:{rest}`"));
        };
        let svar_name = svar_raw.trim();
        if svar_name.is_empty() || svar_name.contains(':') {
            return self.deny(format!("malformed `Backup:{rest}`"));
        }
        let Some(count) = count_raw.trim().parse::<u8>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("invalid `Backup` count `{count_raw}`"));
        };
        let (keywords, grant_abilities) = self.backup_grant_spec(svar_name)?;
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::Backup {{ count: {count}, keywords: {keywords}, \
             grant_abilities: {grant_abilities} }}], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ));
        self.body.notes.push("backup".to_string());
        Some(())
    }

    fn backup_grant_spec(&mut self, svar_name: &str) -> Option<(String, String)> {
        if let Some(cached) = self.backup_grants.get(svar_name) {
            return Some(cached.clone());
        }
        let Some(svar_body) = self.svars.get(svar_name).cloned() else {
            return self.deny(format!("`Backup` naming missing SVar `{svar_name}`"));
        };
        let Some((api, mut p)) = Params::parse(&svar_body) else {
            return self.deny(format!("malformed `Backup` SVar `{svar_name}`"));
        };
        p.drop_prose();
        let prev_abilities = self.body.abilities.len();
        let prev_statics = self.body.statics.len();
        let res = match api.as_str() {
            "Pump" => {
                let Some(raw_kw) = p.take("KW") else {
                    return self.deny("`Backup` Pump without `KW$`".to_string());
                };
                if let Some(key) = p.first_key() {
                    return self.deny(format!("unclaimed parameter `Backup.Pump.{key}`"));
                }
                let keywords = self.join_keyword_union("Backup", &raw_kw)?;
                Some((keywords, "&[]".to_string()))
            }
            "Animate" => self.backup_animate_grants(&mut p),
            other => self.deny(format!("`Backup` API `{other}`")),
        };
        let Some(spec) = res else {
            self.body.abilities.truncate(prev_abilities);
            self.body.statics.truncate(prev_statics);
            return None;
        };
        self.backup_grants
            .insert(svar_name.to_string(), spec.clone());
        Some(spec)
    }

    fn join_keyword_union(&mut self, ctx: &str, raw: &str) -> Option<String> {
        let mut bits = Vec::new();
        for part in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            let Some(bit) = keyword_const(part) else {
                return self.deny(format!("`{ctx}` granting `{part}`"));
            };
            bits.push(bit);
        }
        if bits.is_empty() {
            return self.deny(format!("empty `{ctx}` keyword list"));
        }
        let mut expr = bits[0].to_string();
        for bit in &bits[1..] {
            expr = format!("{expr}.union({bit})");
        }
        Some(expr)
    }

    fn backup_animate_grants(&mut self, p: &mut Params) -> Option<(String, String)> {
        if let Some(defined) = p.take("Defined")
            && defined != "Self"
        {
            return self.deny(format!("`Backup` Animate with `Defined$ {defined}`"));
        }
        if let Some(dur) = p.take("Duration") {
            return self.deny(format!("`Backup` Animate with `Duration$ {dur}`"));
        }
        let keywords = match p.take("Keywords") {
            Some(raw) => self.join_keyword_union("Backup", &raw)?,
            None => "KeywordSet::EMPTY".to_string(),
        };
        let mut grant_exprs = Vec::new();
        if let Some(raw) = p.take("Abilities") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let Some(ab_line) = self.svars.get(name).cloned() else {
                    return self.deny(format!("`Backup` `Abilities$ {name}` naming no SVar"));
                };
                if !ab_line.trim_start().starts_with("AB$") {
                    return self.deny("`Backup` granted ability that is not activated".to_string());
                }
                let parsed = self.parse_activated_spec(&ab_line, false, false)?;
                if parsed.non_battlefield_zone || parsed.alternate_ability_expr.is_some() {
                    return self.deny(
                        "`Backup` granted activated ability outside battlefield".to_string(),
                    );
                }
                grant_exprs.push(parsed.ability_expr);
            }
        }
        if let Some(raw) = p.take("Triggers") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                grant_exprs.push(self.backup_single_trigger(name)?);
            }
        }
        for key in ["staticAbilities", "StaticAbilities"] {
            if let Some(raw) = p.take(key) {
                for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    grant_exprs.push(self.backup_single_static(name)?);
                }
            }
        }
        if let Some(raw) = p.take("sVars") {
            for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                if self.svars.get(name).map(String::as_str) != Some("SVar:HasAttackEffect:TRUE") {
                    return self.deny(format!("`Backup` `sVars$ {name}`"));
                }
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Backup.Animate.{key}`"));
        }
        if keywords == "KeywordSet::EMPTY" && grant_exprs.is_empty() {
            return self.deny("`Backup` granting nothing".to_string());
        }
        let slice_ref = if grant_exprs.is_empty() {
            "&[]".to_string()
        } else {
            let name = self
                .body
                .ability_slice_static("BACKUP_GRANTED", &grant_exprs);
            format!("&{name}")
        };
        Some((keywords, slice_ref))
    }

    fn backup_single_trigger(&mut self, name: &str) -> Option<String> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`Backup` `Triggers$ {name}` naming no SVar"));
        };
        if !line.trim_start().starts_with("Mode$") {
            return self.deny("`Backup` granted trigger without `Mode$`".to_string());
        }
        let prev_abilities = self.body.abilities.len();
        let prev_trigger_mode = self.trigger_mode.take();
        let prev_block_line = self.block_line.take();
        let ok = self.triggered(&line);
        self.trigger_mode = prev_trigger_mode;
        let ended_block = self.block_line.is_some();
        self.block_line = prev_block_line;
        if ok.is_none() || ended_block || self.body.abilities.len() != prev_abilities + 1 {
            return None;
        }
        let expr = self.body.abilities.pop()?;
        if (!expr.starts_with("triggered!(") && !expr.starts_with("modal_triggered!("))
            || expr.contains("zone = TriggerZone::")
        {
            return self.deny("`Backup` granted trigger outside the battlefield".to_string());
        }
        Some(expr)
    }

    fn backup_single_static(&mut self, name: &str) -> Option<String> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`Backup` `staticAbilities$ {name}` naming no SVar"));
        };
        if !line.trim_start().starts_with("Mode$") {
            return self.deny("`Backup` granted static without `Mode$`".to_string());
        }
        let prev_abilities = self.body.abilities.len();
        self.rule('S', &line)?;
        if self.body.abilities.len() != prev_abilities + 1 {
            return self.deny("`Backup` granted static that is not one ability".to_string());
        }
        let expr = self.body.abilities.pop()?;
        if !expr.starts_with("static_ability!(") || expr.contains("zone = ZoneRef::") {
            return self.deny("`Backup` granted static outside the battlefield".to_string());
        }
        Some(expr)
    }

    fn graveyard_keyword_cost(
        &mut self,
        keyword: &str,
        rest: &str,
    ) -> Option<(String, Vec<String>)> {
        if rest.contains(':') {
            return self.deny(format!("a `{keyword}` with extra fields `{rest}`"));
        }
        if super::cost_parts(rest).is_empty() {
            return self.deny(format!("an empty `{keyword}` cost"));
        }
        let prev_x = std::mem::replace(&mut self.has_x, false);
        let prev_alt = std::mem::replace(&mut self.in_alternative_cost, false);
        let pieces = self.cost_pieces(rest);
        self.has_x = prev_x;
        self.in_alternative_cost = prev_alt;
        let (mana, parts) = pieces?;
        if (mana.is_empty() && parts.is_empty())
            || mana.contains('X')
            || !parts.iter().all(|part| is_graveyard_activation_part(part))
        {
            return self.deny(format!("an unsupported `{keyword}` cost `{rest}`"));
        }
        Some((mana, parts))
    }

    pub(super) fn unearth(&mut self, rest: &str) -> Option<()> {
        let (mana, parts) = self.graveyard_keyword_cost("Unearth", rest)?;
        let expr = if parts.is_empty() {
            format!("unearth!(\"{mana}\")")
        } else {
            format!("unearth!({})", crate::body::cost_literal(&mana, &parts))
        };
        self.body.abilities.push(expr);
        self.body.notes.push("unearth".to_string());
        Some(())
    }

    pub(super) fn encore(&mut self, rest: &str) -> Option<()> {
        let (mana, parts) = self.graveyard_keyword_cost("Encore", rest)?;
        let expr = if parts.is_empty() {
            format!("encore!(\"{mana}\")")
        } else {
            format!("encore!({})", crate::body::cost_literal(&mana, &parts))
        };
        self.body.abilities.push(expr);
        self.body.notes.push("encore".to_string());
        Some(())
    }

    pub(super) fn embalm(&mut self, rest: &str) -> Option<()> {
        self.token_copy_graveyard_keyword("Embalm", "embalm", "EMBALM_MODS", rest)
    }

    pub(super) fn eternalize(&mut self, rest: &str) -> Option<()> {
        self.token_copy_graveyard_keyword("Eternalize", "eternalize", "ETERNALIZE_MODS", rest)
    }

    fn token_copy_graveyard_keyword(
        &mut self,
        keyword: &str,
        macro_name: &str,
        mods_const: &str,
        rest: &str,
    ) -> Option<()> {
        let (mana, mut parts) = self.graveyard_keyword_cost(keyword, rest)?;
        let expr = if parts.is_empty() {
            format!("{macro_name}!(\"{mana}\")")
        } else {
            parts.push("ExileSelf".to_string());
            let cost = crate::body::cost_literal(&mana, &parts);
            format!(
                "activated!({cost}, &[Effect::CreateTokenCopyOfSource {{ mods: {mods_const} }}], \
                 zone = ActivationZone::Graveyard, timing = ActivationTiming::SorcerySpeed)"
            )
        };
        self.body.abilities.push(expr);
        self.body.notes.push(macro_name.to_string());
        Some(())
    }

    pub(super) fn bestow(&mut self, rest: &str) -> Option<()> {
        if self.body.bestow.is_some() {
            return self.deny("a second `Bestow` cost on one card".to_string());
        }
        let has_enchantment = self
            .type_line
            .split_whitespace()
            .any(|w| w == "Enchantment");
        let has_creature = self.type_line.split_whitespace().any(|w| w == "Creature");
        if !has_enchantment || !has_creature {
            return self.deny("a `Bestow` on a non-enchantment-creature card".to_string());
        }
        let cost_raw = match rest.split_once(':') {
            None => rest,
            Some((cost_part, "GainControl")) => cost_part,
            Some(_) => return self.deny(format!("a `Bestow` with extra fields `{rest}`")),
        };
        if super::cost_parts(cost_raw).is_empty() {
            return self.deny("an empty `Bestow` cost".to_string());
        }
        let prev_x = std::mem::replace(&mut self.has_x, true);
        let prev_alt = std::mem::replace(&mut self.in_alternative_cost, true);
        let pieces = self.cost_pieces(cost_raw);
        self.has_x = prev_x;
        self.in_alternative_cost = prev_alt;
        let (mana, parts) = pieces?;
        if (mana.is_empty() && parts.is_empty())
            || !parts.iter().all(|part| is_paid_alternative_cost_part(part))
        {
            return self.deny(format!("a `Bestow` cost `{rest}` the cast cannot pay"));
        }
        self.body.bestow = Some(format!(
            "Some({})",
            crate::body::cost_literal(&mana, &parts)
        ));
        self.body.notes.push("bestow".to_string());
        Some(())
    }

    /// `K:Morph:<cost>` and `K:Megamorph:<cost>` (CR 702.37, CR 702.37b):
    /// cast face down as a 2/2 creature for {3} and turn face up any time you
    /// have priority for the face-up mana cost (with optional `{X}` and
    /// non-mana `morph_parts`).
    pub(super) fn face_down_keyword(&mut self, rest: &str, is_megamorph: bool) -> Option<()> {
        let kw = if is_megamorph { "Megamorph" } else { "Morph" };
        if (is_megamorph && self.body.megamorph.is_some())
            || (!is_megamorph && self.body.morph.is_some())
        {
            return self.deny(format!("a second `{kw}` cost on one card"));
        }
        if self.body.morph.is_some()
            || self.body.megamorph.is_some()
            || self.body.disguise.is_some()
        {
            return self.deny(format!("a conflicting `{kw}` cost on one card"));
        }
        if rest.contains(':') {
            return self.deny(format!("a `{kw}` with extra fields `{rest}`"));
        }
        if super::cost_parts(rest).is_empty() {
            return self.deny(format!("an empty `{kw}` cost"));
        }
        let prev_x = std::mem::replace(&mut self.has_x, true);
        let prev_alt = std::mem::replace(&mut self.in_alternative_cost, true);
        let pieces = self.cost_pieces(rest);
        self.has_x = prev_x;
        self.in_alternative_cost = prev_alt;
        let (mana, parts) = pieces?;
        if (mana.is_empty() && parts.is_empty())
            || !parts.iter().all(|part| is_morph_cost_part(part))
        {
            return self.deny(format!("a non-mana `{kw}` cost `{rest}`"));
        }
        if !mana.is_empty() && baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("a `{kw}` cost `{rest}` that is not valid mana"));
        }
        let mana_expr = if mana.is_empty() {
            "Some(ManaCost::ZERO)".to_string()
        } else {
            format!("Some(mana!(\"{mana}\"))")
        };
        if is_megamorph {
            self.body.megamorph = Some(mana_expr);
            self.body.notes.push("megamorph".to_string());
        } else {
            self.body.morph = Some(mana_expr);
            self.body.notes.push("morph".to_string());
        }
        self.body.morph_parts = parts;
        Some(())
    }

    pub(super) fn entwine(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("an `Entwine` with extra fields `{rest}`"));
        }
        let cost = self.cost_expr(rest)?;
        self.body.entwine = Some(format!("Some({cost})"));
        self.body.notes.push("entwine".to_string());
        Some(())
    }

    /// `K:Affinity:<valid>` (CR 702.41a): "This spell costs {1} less to cast
    /// for each [text] you control."
    pub(super) fn affinity(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("an `Affinity` with extra fields `{rest}`"));
        }
        let you_ctrl = rest
            .split([',', ';'])
            .map(|alternative| {
                let alternative = alternative.trim();
                if alternative.contains('.') {
                    format!("{alternative}+YouCtrl")
                } else {
                    format!("{alternative}.YouCtrl")
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let expr = self.filter_expr(&you_ctrl)?;
        let filter = self.body.filter_static("AFFINITY", &expr);
        self.body.cost_reduction = Some(format!(
            "Some(CostReduction::PerCount {{ amount: Amount::CountOf {{ filter: &{filter}, zone: ZoneSel::Battlefield }}, each: 1 }})"
        ));
        self.body.notes.push("affinity".to_string());
        Some(())
    }

    /// `K:Bargain` (CR 702.166a).
    pub(super) fn bargain(&mut self) -> Option<()> {
        if self.body.bargain {
            return self.deny("a second `Bargain` keyword on one card".to_string());
        }
        self.body.bargain = true;
        self.body.notes.push("bargain".to_string());
        Some(())
    }

    /// `K:Emerge:<cost>[:<valid>]` (CR 702.119a–b).
    pub(super) fn emerge(&mut self, rest: &str) -> Option<()> {
        if self.body.emerge.is_some() {
            return self.deny("a second `Emerge` cost on one card".to_string());
        }
        let mut fields = rest.split(':');
        let cost_raw = fields.next()?;
        let valid_raw = fields.next();
        if fields.next().is_some() {
            return self.deny(format!("an `Emerge` with extra fields `{rest}`"));
        }
        let Some(mana) = super::printed_mana(cost_raw) else {
            return self.deny(format!("non-mana `Emerge` cost `{cost_raw}`"));
        };
        if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("invalid `Emerge` mana `{cost_raw}`"));
        }
        let expr = if let Some(valid) = valid_raw {
            let trimmed = valid.trim();
            if trimmed.is_empty() {
                return self.deny(format!("an empty `Emerge` filter `{rest}`"));
            }
            let filter_expr = self.filter_expr(trimmed)?;
            let filter_ref = self.body.filter_static("EMERGE", &filter_expr);
            format!("Some(Emerge::from(mana!(\"{mana}\"), &{filter_ref}))")
        } else {
            format!("Some(Emerge::creature(mana!(\"{mana}\")))")
        };
        self.body.emerge = Some(expr);
        self.body.notes.push("emerge".to_string());
        Some(())
    }

    /// `K:Prototype:<cost>:<power>:<toughness>` (CR 702.160, CR 719.3).
    pub(super) fn prototype(&mut self, rest: &str) -> Option<()> {
        if self.body.prototype.is_some() {
            return self.deny("a second `Prototype` on one card".to_string());
        }
        let mut fields = rest.split(':');
        let (Some(cost_raw), Some(power_raw), Some(toughness_raw), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return self.deny(format!("prototype `{rest}`"));
        };
        let Some(mana) = super::printed_mana(cost_raw) else {
            return self.deny(format!("non-mana prototype cost `{cost_raw}`"));
        };
        if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("invalid prototype mana `{cost_raw}`"));
        }
        let Ok(power) = power_raw.trim().parse::<i16>() else {
            return self.deny(format!("prototype `{rest}`"));
        };
        let Ok(toughness) = toughness_raw.trim().parse::<i16>() else {
            return self.deny(format!("prototype `{rest}`"));
        };
        self.body.prototype = Some(format!(
            "Some(Prototype::new(mana!(\"{mana}\"), {power}, {toughness}))"
        ));
        self.body.notes.push("prototype".to_string());
        Some(())
    }

    /// `K:Enchant:<valid>` as the spell an Aura card is.
    ///
    /// Enchant is a static ability of the *spell* (CR 702.5b): it says what
    /// the Aura targets as it is cast (CR 303.4a), and the Aura arrives on
    /// the battlefield already attached to that permanent. One `spell!`
    /// carrying `Effect::AttachSelf` is all of that, and it is the shape
    /// the pool's hand-written Auras already have.
    ///
    /// The target is named twice — once as what the spell may aim at and
    /// once as what the effect attaches to — which is one `static` in the
    /// generated file, because `filter_static` gives a filter written twice
    /// in a card a single name.
    ///
    /// **An Aura on a player is refused.** `Effect::AttachSelf` reads the
    /// resolution's first target as an object, so `K:Enchant:Player` would
    /// generate a card that resolves, attaches to nothing, and is put into
    /// its owner's graveyard by the next state-based action (CR 704.5m).
    /// The corpus prints it 50 times, and each is a card this engine cannot
    /// yet say rather than one it may guess at.
    ///
    /// A third field is the printed wording — "creature you control" — and
    /// this side takes the printed wording from Scryfall, so it is prose.
    pub(super) fn enchant(&mut self, rest: &str) -> Option<()> {
        let valid = rest.split(':').next().unwrap_or(rest).trim();
        let target_spec = match valid {
            "Player" => "TargetSpec::AnyPlayer".to_string(),
            "Opponent" | "Player.Opponent" | "Player.OppCtrl" => {
                "TargetSpec::AnyOpponent".to_string()
            }
            "Player.You" | "You" => "TargetSpec::Player(PlayerRel::You)".to_string(),
            _ if valid.split(',').any(|alt| {
                let head = alt.split(['.', '+']).next().unwrap_or(alt).trim();
                matches!(head, "Player" | "Opponent" | "You")
            }) =>
            {
                return self.deny(format!("an `Enchant {valid}` player qualifier"));
            }
            _ => {
                let expr = self.filter_expr(valid)?;
                let name = self.body.filter_static("ENCHANT", &expr);
                format!("TargetSpec::Object(&{name})")
            }
        };
        self.body.abilities.push(format!(
            "spell!(&[Effect::AttachSelf {{ target: {target_spec} }}], \
             targets = Some(TargetReq::one({target_spec})))"
        ));
        Some(())
    }

    fn should_bypass_extra_static(spec: &str) -> bool {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return false;
        };
        p.drop_prose();
        let cmd_zone = matches!(
            p.peek("EffectZone"),
            Some("Command" | "Battlefield,Command" | "Command,Battlefield")
        );
        match mode.as_str() {
            "CantBeCast" => {
                !p.has("NumLimitEachTurn")
                    && p.peek("ValidCard").is_some_and(|v| v != "Card.Self")
                    && matches!(
                        p.peek("EffectZone"),
                        None | Some(
                            "Battlefield"
                                | "Command"
                                | "Battlefield,Command"
                                | "Command,Battlefield"
                        )
                    )
                    && (p.peek("Caster") == Some("You")
                        || (p.peek("Caster") == Some("Opponent") && cmd_zone))
                    && p.entries
                        .iter()
                        .all(|(k, _)| matches!(k.as_str(), "ValidCard" | "Caster" | "EffectZone"))
            }
            "Continuous" => {
                (p.has("CheckSVar")
                    && (p.has("AddAbility") || p.has("AddTrigger") || p.has("Goad")))
                    || (cmd_zone
                        && !p.has("AffectedZone")
                        && !matches!(
                            p.peek("Affected"),
                            Some(
                                "You" | "Player" | "Opponent" | "Player.Opponent" | "Player.Chosen"
                            )
                        ))
            }
            "CantAttack"
            | "CantAttack,CantBlock"
            | "CantAttack,CantBlock,CantBeActivated"
            | "CantBeActivated"
            | "CantBlock"
            | "CantBlockBy"
            | "MustAttack" => {
                cmd_zone
                    || p.peek("CheckSVar")
                        .is_some_and(|v| v.starts_with("Count$") || v.starts_with("PlayerCount"))
            }
            _ => false,
        }
    }

    pub(super) fn rule(&mut self, kind: char, spec: &str) -> Option<()> {
        self.has_x = kind == 'A';
        self.on_a_spell = kind == 'A' && spec.trim_start().starts_with("SP$");
        self.trigger_mode = None;
        self.block_line = None;
        match kind {
            'A' => {
                let saved_statics = self.body.statics.len();
                let saved_abilities = self.body.abilities.len();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                if self.activated_or_spell(spec).is_some() {
                    Some(())
                } else {
                    let first_unclaimed = self.unclaimed.borrow().clone();
                    self.body.statics.truncate(saved_statics);
                    self.body.abilities.truncate(saved_abilities);
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    if self.wave16a_spell_or_activated(spec).is_some() {
                        Some(())
                    } else {
                        self.body.statics.truncate(saved_statics);
                        self.body.abilities.truncate(saved_abilities);
                        *self.unclaimed.borrow_mut() = first_unclaimed;
                        None
                    }
                }
            }
            'T' => self.triggered(spec),
            'S' if self.is_ai_only_static(spec) => Some(()),
            'S' if Self::should_bypass_extra_static(spec) => self.static_ability(spec),
            'S' => match self.try_extra_static(spec) {
                super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                super::saga_ward_convoke::ExtraOutcome::Pass => self.static_ability(spec),
            },
            'R' => {
                let saved_statics = self.body.statics.len();
                let saved_enter_mods = self.body.enter_modifiers.len();
                let saved_abilities = self.body.abilities.len();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                match self.try_extra_replacement(spec) {
                    super::saga_ward_convoke::ExtraOutcome::Handled(Some(())) => Some(()),
                    super::saga_ward_convoke::ExtraOutcome::Handled(None) => {
                        let extra_unclaimed = self.unclaimed.borrow().clone();
                        self.body.statics.truncate(saved_statics);
                        self.body.enter_modifiers.truncate(saved_enter_mods);
                        self.body.abilities.truncate(saved_abilities);
                        *self.unclaimed.borrow_mut() = saved_unclaimed;
                        if self
                            .wave24e_self_etb_moved_replacement(spec)
                            .or_else(|| self.wave26f_moved_replacement(spec))
                            .or_else(|| self.wave27e_replacement(spec))
                            .is_some()
                        {
                            return Some(());
                        }
                        self.body.statics.truncate(saved_statics);
                        self.body.enter_modifiers.truncate(saved_enter_mods);
                        self.body.abilities.truncate(saved_abilities);
                        *self.unclaimed.borrow_mut() = extra_unclaimed;
                        None
                    }
                    super::saga_ward_convoke::ExtraOutcome::Pass => self.replacement(spec),
                }
            }
            other => self.deny(format!("rules line kind `{other}:`")),
        }
    }

    /// An `R:` replacement, for the one shape the engine models as data.
    ///
    /// "Enters tapped" is a replacement effect in the corpus and an
    /// `EnterModifier` here, and the difference matters: a modifier is read
    /// *as the permanent enters*, which is what CR 614.1c describes and what
    /// stops the land from being tapped a moment after it arrives untapped.
    /// Every other `Moved` replacement is a rule of its own and refuses.
    #[allow(clippy::too_many_lines)]
    pub(super) fn replacement(&mut self, spec: &str) -> Option<()> {
        let Some((event, mut p)) = Params::parse(spec) else {
            return self.deny("an `R:` line with no `$` in it".to_string());
        };
        if matches!(event.as_str(), "GameLoss" | "GameWin") {
            p.drop_prose();
            let loss = event == "GameLoss";
            if p.take("ActiveZones").as_deref() != Some("Battlefield")
                || p.take("Layer").as_deref() != Some("CantHappen")
                || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            {
                return self.deny(format!("replacement `R: Event$ {event}`"));
            }
            let who = match p.take("ValidPlayer").as_deref() {
                Some("You") => "PlayerRel::You",
                Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                Some("Player") => "PlayerRel::EachPlayer",
                _ => return self.deny(format!("replacement `R: Event$ {event}`")),
            };
            let zero_life = match (loss, p.take("ValidLoseReason").as_deref()) {
                (_, None) => false,
                (true, Some("LifeReachedZero")) => true,
                _ => return self.deny(format!("replacement `R: Event$ {event}`")),
            };
            let condition = self.condition(&mut p)?;
            if !p.exhausted() {
                return self.deny(format!("replacement `R: Event$ {event}`"));
            }
            let modifier = match (loss, zero_life, who) {
                (true, true, _) => format!("Modifier::NoLossForZeroLife {{ who: {who} }}"),
                (true, false, "PlayerRel::EachPlayer") => "Modifier::PlayersCantLose".to_string(),
                (true, false, _) => format!("Modifier::NamedPlayersCantLose({who})"),
                (false, _, _) => format!("Modifier::PlayersCantWin({who})"),
            };
            let mut expr = Self::static_expr("Filter::Any", &modifier);
            if !condition.is_empty()
                && let Some(open) = expr.strip_suffix(')')
            {
                expr = format!("{open}{condition})");
            }
            self.body.abilities.push(expr);
            return Some(());
        }
        if event == "DamageDone" && p.peek("ValidCause") == Some("Spell.IsTargeting Self") {
            return self.bronze_horse_prevention(p);
        }
        if event == "Moved" && p.peek("ValidCard") == Some("Gate.YouCtrl") {
            return self.gond_entry(p);
        }
        if event == "DamageDone" && p.peek("ValidSource") == Some("Creature.blockedBySource") {
            return self.wall_blocked_damage_prevention(p);
        }
        if event == "DamageDone" && p.peek("ValidSource") == Some("Creature.EnchantedBy") {
            return self.enchanted_source_damage_prevention(p);
        }
        if event == "DamageDone" && p.peek("ValidTarget") == Some("Creature.EnchantedBy") {
            return self.enchanted_target_damage_prevention(p);
        }
        if event == "DamageDone"
            && p.peek("IsCombat") == Some("True")
            && (p.peek("ValidTarget") == Some("Creature.YouCtrl")
                || p.peek("ValidSource") == Some("Creature.YouCtrl"))
        {
            return self.combat_damage_your_creatures_prevention(p);
        }
        if event == "DamageDone" {
            return self.spell_damage_prevention(p);
        }
        if event == "Untap" {
            return self.does_not_untap(&mut p);
        }
        if event == "BeginPhase" {
            return self.skip_untap_steps(&mut p);
        }
        if event == "RollDice" {
            return self.roll_dice_replacement(p);
        }
        if event == "Counter" {
            p.drop_prose();
            if p.peek("ValidCard") == Some("Card.Self")
                && p.peek("ValidSA") == Some("Spell")
                && p.peek("Layer") == Some("CantHappen")
            {
                let mut self_p = p.clone();
                self_p.take("ValidCard");
                self_p.take("ValidSA");
                self_p.take("Layer");
                if self_p.exhausted() {
                    self.body
                        .keywords
                        .push("KeywordSet::UNCOUNTERABLE".to_string());
                    return Some(());
                }
            }
            if self.wave26f_counter_replacement(p).is_some() {
                return Some(());
            }
            self.note("replacement `R: Event$ Counter` this rule cannot read".to_string());
            return None;
        }
        if event != "Moved" {
            self.note(format!("replacement `R: Event$ {event}`"));
            return None;
        }
        if matches!(
            p.peek("ValidCard"),
            Some("Land.OppCtrl+nonBasic" | "Artifact.OppCtrl,Creature.OppCtrl,Land.OppCtrl")
        ) {
            return self.external_enter_tapped(p);
        }
        p.drop_prose();
        // Anything but the card itself entering the battlefield is a
        // different effect ("whenever another creature enters…").
        let about_self = p.take("ValidCard").as_deref() == Some("Card.Self");
        let entering = p.take("Destination").as_deref() == Some("Battlefield");
        // `Updated` means the event still happens, changed. `Prevented` and
        // the rest replace it with something else entirely.
        let updated = p.take("ReplacementResult").as_deref() == Some("Updated");
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` with no `ReplaceWith$`".to_string());
        };
        if !about_self || !entering || !updated || !p.exhausted() {
            if self
                .wave24e_self_etb_moved_replacement(spec)
                .or_else(|| self.wave26f_moved_replacement(spec))
                .is_some()
            {
                return Some(());
            }
            self.note("replacement `Moved` this rule cannot read".to_string());
            return None;
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny(format!("`ReplaceWith$ {with}` names no readable SVar"));
        };
        body.drop_prose();
        // `DB$ Tap | Defined$ Self | ETB$ True`: the tap has to be of this
        // card, as it enters, or it is not this modifier. `Defined$ ReplacedCard`
        // also names this entering permanent when a card reuses its `ETBTapped`
        // SVar across a self-entry and external-entry replacement (*False Floor*).
        if api != "Tap"
            || !matches!(
                body.take("Defined").as_deref(),
                Some("Self" | "ReplacedCard")
            )
            || body.take("ETB").as_deref() != Some("True")
        {
            if self
                .wave24e_self_etb_moved_replacement(spec)
                .or_else(|| self.wave26f_moved_replacement(spec))
                .is_some()
            {
                return Some(());
            }
            self.note(format!("replacement `Moved` replacing with `{api}`"));
            return None;
        }
        // A land prints one of three sentences about coming down tapped, and
        // the reference writes all three on this one line: unconditionally,
        // *unless you control* enough of something, or *unless you pay*. They
        // are three `EnterModifier` variants, so they are read as three
        // shapes rather than one with options, and a line claiming both a
        // count and a cost is neither of them.
        let modifier = match (body.take("ConditionPresent"), body.take("UnlessCost")) {
            (Some(_), Some(_)) => {
                self.note("replacement `Moved` both counting and charging".to_string());
                return None;
            }
            (None, Some(cost)) => self.enters_tapped_or_pays(&mut body, &cost)?,
            (Some(present), None) => self.enters_tapped_unless(&mut body, &present)?,
            // A fourth sentence, and it arrives through a different key
            // because it counts something no `IsPresent$` filter reaches: a
            // number of *players*. Without one of those the line is the
            // plain "enters tapped" (optionally chaining ETB counters onto
            // itself, as on the Vivid lands or Sphere of the Suns, or gating
            // on `ConditionPlayerTurn$` as on *Horned Loch-Whale*).
            (None, None) => match body.take("ConditionCheckSVar") {
                None => {
                    if let Some(turn_flag) = body.take("ConditionPlayerTurn") {
                        match turn_flag.as_str() {
                            "False" => {
                                "EnterModifier::TappedUnlessCondition(Condition::YourTurn)"
                                    .to_string()
                            }
                            "True" => {
                                "EnterModifier::TappedUnlessCondition(Condition::Not(&Condition::YourTurn))"
                                    .to_string()
                            }
                            _ => {
                                self.note(format!(
                                    "replacement `Moved` with `ConditionPlayerTurn$ {turn_flag}`"
                                ));
                                return None;
                            }
                        }
                    } else if let Some(cond_raw) = body.take("Condition") {
                        let Some(tap_cond) = Self::named_condition(&cond_raw)
                            .or_else(|| Self::wave7_named_condition(&cond_raw))
                            .or_else(|| Self::wave21c_named_condition(&cond_raw))
                        else {
                            self.note(format!(
                                "replacement `Moved` tapping with `Condition$ {cond_raw}`"
                            ));
                            return None;
                        };
                        let unless = self.negate_etb_condition(tap_cond);
                        format!("EnterModifier::TappedUnlessCondition({unless})")
                    } else if let Some(sub) = body.take("SubAbility") {
                        if !body.exhausted() {
                            self.note(format!(
                                "replacement `Moved` tapping with `{}`",
                                body.first_key().unwrap_or_default()
                            ));
                            return None;
                        }
                        let extras = self.self_etb_counter_chain(&sub)?;
                        self.body
                            .enter_modifiers
                            .push("EnterModifier::Tapped".to_string());
                        self.body.enter_modifiers.extend(extras);
                        return Some(());
                    } else {
                        "EnterModifier::Tapped".to_string()
                    }
                }
                Some(svar) => self.enters_tapped_unless_players(&mut body, &svar)?,
            },
        };
        if !body.exhausted() {
            self.note(format!(
                "replacement `Moved` tapping with `{}`",
                body.first_key().unwrap_or_default()
            ));
            return None;
        }
        self.body.enter_modifiers.push(modifier);
        Some(())
    }

    /// Wave 24e self-ETB `R:Event$ Moved` replacement shapes mapping cleanly
    /// to existing `EnterModifier` (and `Trigger::ETB` for escaped-ETB) DSL variants:
    /// - `ChooseType` (`Basic Land`) -> `Tap | UnlessCost$ PayLife<N>` (*Multiversal Passage*)
    /// - `PutCounter` (`Number$C/Minus.X`) -> `Tap` (`X <= M`) (*Slumbering Trudge*)
    /// - `ChooseCard` (GY X) -> `Exile` -> `MoveToYard` -> `PutCounter` (`SplitAmount$ True`)
    ///   (*Frankenstein's Monster*)
    /// - `Sacrifice` (optional any number) -> `PutCounter` (`P1P1`) (*Shimatsu*, *Devouring Hellion*)
    /// - `Card.Self+escaped` -> `PutCounter` -> `ImmediateTrigger` (*Pharika's Spawn*)
    #[allow(clippy::too_many_lines)]
    fn wave24e_self_etb_moved_replacement(&mut self, spec: &str) -> Option<()> {
        let (event, mut p) = Params::parse(spec)?;
        if event != "Moved" {
            return None;
        }
        p.drop_prose();
        let valid_card = p.take("ValidCard")?;
        if p.take("Destination").as_deref() != Some("Battlefield") {
            return None;
        }
        let result_kind = p.take("ReplacementResult");
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }
        let body_str = self.svars.get(with.trim())?.clone();
        let (api, mut body) = Params::parse(&body_str)?;
        body.drop_prose();

        // Shape 1 (*Multiversal Passage*): `ChooseType` (`Basic Land`) -> `Tap` (`PayLife<N>`).
        if valid_card == "Card.Self"
            && result_kind.as_deref() == Some("Updated")
            && api == "ChooseType"
            && body.take("ETB").as_deref() == Some("True")
            && body.take("Type").as_deref() == Some("Basic Land")
            && let Some(sub) = body.take("SubAbility")
            && body.exhausted()
        {
            let tap_str = self.svars.get(sub.trim())?.clone();
            let (t_api, mut tp) = Params::parse(&tap_str)?;
            tp.drop_prose();
            if t_api == "Tap"
                && tp.take("ETB").as_deref() == Some("True")
                && tp.take("Defined").as_deref() == Some("Self")
                && tp.take("UnlessPayer").as_deref() == Some("You")
                && let Some(cost) = tp.take("UnlessCost")
                && tp.exhausted()
                && let Some(life) = cost
                    .strip_prefix("PayLife<")
                    .and_then(|s| s.strip_suffix('>'))
                    .and_then(|n| n.parse::<u16>().ok())
            {
                self.body
                    .enter_modifiers
                    .push("EnterModifier::ChooseBasicLandType".to_string());
                self.body
                    .enter_modifiers
                    .push(format!("EnterModifier::TappedOrPayLife({life})"));
                return Some(());
            }
            return None;
        }

        // Shape 2 (*Slumbering Trudge*): `PutCounter` (`Number$C/Minus.X`) -> `Tap` (`X <= M`).
        if valid_card == "Card.Self"
            && result_kind.as_deref() == Some("Updated")
            && self.has_x_mana_cost
            && api == "PutCounter"
            && body.take("ETB").as_deref() == Some("True")
            && matches!(body.take("Defined").as_deref(), None | Some("Self"))
            && let Some(kind_raw) = body.take("CounterType")
            && let Some(num_raw) = body.take("CounterNum")
            && let Some(sub) = body.take("SubAbility")
            && body.exhausted()
            && let Some(kind) = super::counter_kind(&kind_raw)
        {
            let num_def = self.svars.get(num_raw.trim())?.trim();
            let (lhs, rhs) = num_def.split_once("/Minus.")?;
            let constant: u32 = lhs.strip_prefix("Number$")?.trim().parse().ok()?;
            if self.svars.get(rhs.trim()).map(|s| s.trim()) != Some("Count$xPaid") {
                return None;
            }
            let tap_str = self.svars.get(sub.trim())?.clone();
            let (t_api, mut tp) = Params::parse(&tap_str)?;
            tp.drop_prose();
            if t_api != "Tap"
                || tp.take("ETB").as_deref() != Some("True")
                || !matches!(tp.take("Defined").as_deref(), None | Some("Self"))
            {
                return None;
            }
            let chk = tp.take("ConditionCheckSVar")?;
            let cmp = tp.take("ConditionSVarCompare")?;
            if !tp.exhausted()
                || self.svars.get(chk.trim()).map(|s| s.trim()) != Some("Count$xPaid")
            {
                return None;
            }
            let threshold: u32 = if let Some(m) = cmp.strip_prefix("LE") {
                m.parse::<u32>().ok()?.checked_add(1)?
            } else {
                let m = cmp.strip_prefix("LT")?;
                m.parse::<u32>().ok().filter(|&v| v >= 1)?
            };
            self.body.enter_modifiers.push(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: Amount::ConstantMinus {{ \
                 constant: {constant}, subtract: &Amount::X }} }}"
            ));
            self.body.enter_modifiers.push(format!(
                "EnterModifier::TappedUnlessCondition(Condition::XAtLeast({threshold}))"
            ));
            return Some(());
        }

        // Shape 3 (*Frankenstein's Monster*): exile X creatures from GY or go to GY instead,
        // then choose one counter per exiled card from a printed counter list.
        if valid_card == "Card.Self"
            && result_kind.is_none()
            && self.has_x_mana_cost
            && api == "ChooseCard"
        {
            return self.try_wave24e_frankenstein_entry(body);
        }

        // Shape 4 (*Shimatsu the Bloodcloaked*, *Devouring Hellion*): optional sacrifice of
        // any number of matching permanents -> `EnterModifier::Devour { filter, factor }`.
        if valid_card == "Card.Self"
            && result_kind.as_deref() == Some("Updated")
            && api == "Sacrifice"
        {
            return self.try_wave24e_devour_replacement(body);
        }

        // Shape 5 (*Pharika's Spawn*): `Card.Self+escaped` -> `PutCounter` -> `ImmediateTrigger`.
        if matches!(
            valid_card.as_str(),
            "Card.Self+escaped" | "Card.escaped+Self"
        ) && result_kind.as_deref() == Some("Updated")
            && api == "PutCounter"
            && body.take("ETB").as_deref() == Some("True")
            && matches!(body.take("Defined").as_deref(), None | Some("Self"))
            && let Some(kind_raw) = body.take("CounterType")
            && let Some(num_raw) = body.take("CounterNum")
            && let Some(sub) = body.take("SubAbility")
            && body.exhausted()
            && let Some(kind) = super::counter_kind(&kind_raw)
            && let Some(amount) = self.etb_wave6_amount(num_raw.trim())
        {
            let trig_str = self.svars.get(sub.trim())?.clone();
            let (t_api, mut tp) = Params::parse(&trig_str)?;
            tp.drop_prose();
            if t_api != "ImmediateTrigger" {
                return None;
            }
            let exec_name = tp.take("Execute")?;
            if !tp.exhausted() {
                return None;
            }
            let exec_body = self.svars.get(exec_name.trim())?.clone();
            let mut chain = Chain::default();
            let saved_statics = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let ok = self.chain(&exec_body, &mut chain);
            if ok.is_none() || chain.effects.is_empty() || chain.second_target.is_some() {
                self.body.statics.truncate(saved_statics);
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                return None;
            }
            let targets = Self::target_req_expr(&chain)
                .map_or_default(|req| format!(", targets = Some({req})"));
            self.body.enter_modifiers.push(format!(
                "EnterModifier::WithCountersIf {{ kind: {kind}, count: {amount}, \
                 condition: Condition::Escaped }}"
            ));
            self.body.abilities.push(format!(
                "triggered!(Trigger::ETB, &[{}]{targets}, condition = Some(Condition::Escaped))",
                chain.effects.join(", ")
            ));
            return Some(());
        }

        None
    }

    /// Wave 14d `R:Event$ Counter | Layer$ CantHappen` static uncounterability
    /// -> `Modifier::SpellsCantBeCountered { who, filter }` (*Prowling Serpopard*,
    /// *Destiny Spinner*, *Allosaurus Shepherd*, *Lier*, *Rhythm of the Wild*,
    /// *Taigam, Ojutai Master*, *Spellbreaker Behemoth*, *Gaea's Herald*,
    /// *Root Sliver*, *Chimil*, *Leyline of Lifeforce*).
    fn wave26f_counter_replacement(&mut self, mut p: Params) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = (|| {
            let zone_arg = match p.take("ActiveZones").as_deref() {
                Some("Battlefield") => "",
                Some("Command") => ", zone = ZoneRef::Command",
                _ => return None,
            };
            if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
                return None;
            }
            let (who, filter_ref) = self.spells_cant_be_countered_parts(&mut p)?;
            let cond = self.condition(&mut p)?;
            if !p.exhausted() {
                return None;
            }
            self.body.abilities.push(format!(
                "static_ability!(Filter::Any, Modifier::SpellsCantBeCountered {{ \
                 who: {who}, filter: {filter_ref} }}{zone_arg}{cond})"
            ));
            Some(())
        })();
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    /// Wave 26f `R:Event$ Moved` replacement unlocks with atomic rollback:
    /// 1. Self-ETB `AB$ StoreSVar | Cost$ Mandatory PayLife<X>` -> `EnterModifier::PayLifeUpTo`
    ///    (*Minion of the Wastes*, *Phyrexian Processor*, *Nameless Race*).
    /// 2. Self-ETB `DB$ AlterAttribute | Attributes$ Suspected` -> `EnterModifier::Suspected`.
    /// 3. Self-ETB `DB$ PutCounter` (standalone, conditional, or chained with `PutCounter`/`Tap`).
    /// 4. Self-ETB `DB$ Tap` gated on `R:`-header `Condition$`/`IsPresent$`/`CheckSVar$`.
    /// 5. External `Other` `EnterTapped` / `EnterUntapped` (`ReplacementRule::EnterTapped` /
    ///    `ReplacementRule::EnterUntapped`).
    fn wave26f_moved_replacement(&mut self, spec: &str) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_enter_mods = self.body.enter_modifiers.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave26f_moved_replacement_inner(spec);
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.enter_modifiers.truncate(saved_enter_mods);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    #[allow(clippy::too_many_lines)]
    fn wave26f_moved_replacement_inner(&mut self, spec: &str) -> Option<()> {
        let (event, mut p) = Params::parse(spec)?;
        if event != "Moved" {
            return None;
        }
        p.drop_prose();
        let valid_card = p.take("ValidCard")?;
        if p.take("Destination").as_deref() != Some("Battlefield") {
            return None;
        }
        let active_zones = p.take("ActiveZones");
        let origin = p.take("Origin");
        let secondary = p.take("Secondary");
        let result_kind = p.take("ReplacementResult");
        let with = p.take("ReplaceWith")?;
        let body_str = self.svars.get(with.trim())?.clone();
        let (api, mut body) = Params::parse(&body_str)?;
        body.drop_prose();

        // Shape 5: external `Other` `EnterTapped` / `EnterUntapped`.
        if valid_card.contains("Other")
            && !valid_card.contains("Self")
            && active_zones.as_deref() == Some("Battlefield")
            && origin.is_none()
            && result_kind.as_deref() == Some("Updated")
            && matches!(secondary.as_deref(), None | Some("True"))
            && p.exhausted()
            && body.take("ETB").as_deref() == Some("True")
            && body.take("Defined").as_deref() == Some("ReplacedCard")
            && body.exhausted()
        {
            let rule = match api.as_str() {
                "Tap" => "EnterTapped",
                "Untap"
                    if valid_card
                        .split(',')
                        .all(|alt| alt.split(['.', '+']).any(|a| a.trim() == "YouCtrl")) =>
                {
                    "EnterUntapped"
                }
                _ => return None,
            };
            let expr = self.filter_expr(&valid_card)?;
            let filter = self.body.filter_static("ENTER", &expr);
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::{rule}(&{filter}))"
            ));
            return Some(());
        }

        if secondary.is_some()
            || !matches!(origin.as_deref(), None | Some("Any"))
            || !matches!(active_zones.as_deref(), None | Some("All"))
        {
            return None;
        }

        // Shape 1: `PayLifeUpTo` (*Minion of the Wastes*, *Phyrexian Processor*, *Nameless Race*).
        if valid_card == "Card.Self"
            && result_kind.as_deref() == Some("Updated")
            && p.exhausted()
            && api == "StoreSVar"
            && body.take("SVar").as_deref() == Some("LifePaidOnETB")
            && body.take("Type").as_deref() == Some("Calculate")
            && let Some(cost_raw) = body.take("Cost")
            && let Some(expr_var) = body.take("Expression")
        {
            let x_var = cost_raw
                .trim()
                .strip_prefix("Mandatory PayLife<")
                .and_then(|s| s.strip_suffix('>'))?
                .trim();
            if x_var != expr_var.trim()
                || self.svars.get(x_var).map(|s| s.trim()) != Some("Count$xPaid")
            {
                return None;
            }
            let cap = match body.take("XMax") {
                None => "Amount::LifeTotal(PlayerRel::You)".to_string(),
                Some(limit) => self.etb_wave6_amount(limit.trim())?,
            };
            if !body.exhausted() {
                return None;
            }
            self.body
                .enter_modifiers
                .push(format!("EnterModifier::PayLifeUpTo({cap})"));
            return Some(());
        }

        // Shape 2: `EnterModifier::Suspected`.
        if valid_card == "Card.Self"
            && matches!(result_kind.as_deref(), None | Some("Updated"))
            && p.exhausted()
            && api == "AlterAttribute"
            && matches!(body.take("Defined").as_deref(), None | Some("Self"))
            && body.take("Attributes").as_deref() == Some("Suspected")
            && matches!(body.take("Activate").as_deref(), None | Some("True"))
            && matches!(body.take("ETB").as_deref(), None | Some("True"))
            && body.exhausted()
        {
            self.body
                .enter_modifiers
                .push("EnterModifier::Suspected".to_string());
            return Some(());
        }

        // Parse optional `R:`-header condition for self-ETB Shapes 3 and 4.
        let header_cond = if valid_card == "Card.Self" {
            self.wave26f_take_etb_header_condition(&mut p).ok()?
        } else {
            if !p.exhausted() {
                return None;
            }
            Some(self.etb_counter_condition(&format!("ValidCard$ {valid_card}"))?)
        };
        if !p.exhausted() {
            return None;
        }

        // Shape 4: self-ETB `DB$ Tap` gated on an `R:`-header condition.
        if api == "Tap"
            && result_kind.as_deref() == Some("Updated")
            && let Some(tap_cond) = header_cond.as_deref()
            && matches!(
                body.take("Defined").as_deref(),
                Some("Self" | "ReplacedCard")
            )
            && body.take("ETB").as_deref() == Some("True")
            && body.exhausted()
        {
            let unless = self.negate_etb_condition(tap_cond);
            self.body
                .enter_modifiers
                .push(format!("EnterModifier::TappedUnlessCondition({unless})"));
            return Some(());
        }

        // Shape 3: self-ETB `DB$ PutCounter` (standalone, conditional, or chained).
        if api == "PutCounter"
            && matches!(result_kind.as_deref(), None | Some("Updated"))
            && body.take("ETB").as_deref() == Some("True")
            && matches!(
                body.take("Defined").as_deref(),
                None | Some("Self" | "ReplacedCard")
            )
            && let Some(kind_raw) = body.take("CounterType")
            && let Some(num_raw) = body.take("CounterNum")
            && let Some(kind) = super::counter_kind(kind_raw.trim())
            && let Some(amount) = self.etb_wave6_amount(num_raw.trim())
        {
            let sub = body.take("SubAbility");
            let body_cond = self.wave26f_take_etb_body_condition(&mut body).ok()?;
            if !body.exhausted() || (header_cond.is_some() && body_cond.is_some()) {
                return None;
            }
            let cond = header_cond.or(body_cond);
            if cond.is_some() && sub.is_some() {
                return None;
            }
            let mut mods = Vec::new();
            let first = match cond {
                None => {
                    format!("EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}")
                }
                Some(c) => format!(
                    "EnterModifier::WithCountersIf {{ kind: {kind}, count: {amount}, \
                     condition: {c} }}"
                ),
            };
            mods.push(first);
            let mut next = sub;
            let mut depth = 0usize;
            while let Some(sub_name) = next.take() {
                depth += 1;
                if depth > 4 {
                    return None;
                }
                let sub_str = self.svars.get(sub_name.trim())?.clone();
                let (s_api, mut sp) = Params::parse(&sub_str)?;
                sp.drop_prose();
                next = sp.take("SubAbility");
                match s_api.as_str() {
                    "PutCounter"
                        if sp.take("ETB").as_deref() == Some("True")
                            && matches!(
                                sp.take("Defined").as_deref(),
                                None | Some("Self" | "ReplacedCard")
                            ) =>
                    {
                        let sk = super::counter_kind(sp.take("CounterType")?.trim())?;
                        let sa = self.etb_wave6_amount(sp.take("CounterNum")?.trim())?;
                        if !sp.exhausted() {
                            return None;
                        }
                        mods.push(format!(
                            "EnterModifier::WithCounters {{ kind: {sk}, amount: {sa} }}"
                        ));
                    }
                    "Tap"
                        if sp.take("ETB").as_deref() == Some("True")
                            && matches!(
                                sp.take("Defined").as_deref(),
                                None | Some("Self" | "ReplacedCard")
                            )
                            && sp.exhausted() =>
                    {
                        mods.push("EnterModifier::Tapped".to_string());
                    }
                    _ => return None,
                }
            }
            self.body.enter_modifiers.extend(mods);
            return Some(());
        }

        None
    }

    fn wave26f_take_etb_header_condition(&mut self, p: &mut Params) -> Result<Option<String>, ()> {
        if let Some(c) = p.take("Condition") {
            return self
                .etb_counter_condition(&format!("Condition$ {c}"))
                .map(Some)
                .ok_or(());
        }
        if let Some(present) = p.take("IsPresent") {
            let mut spec = format!("IsPresent$ {present}");
            if let Some(cmp) = p.take("PresentCompare") {
                spec.push_str(" | PresentCompare$ ");
                spec.push_str(&cmp);
            }
            if let Some(def) = p.take("PresentDefined") {
                spec.push_str(" | PresentDefined$ ");
                spec.push_str(&def);
            }
            return self.etb_counter_condition(&spec).map(Some).ok_or(());
        }
        if let Some(var) = p.take("CheckSVar") {
            let mut spec = format!("CheckSVar$ {var}");
            if let Some(cmp) = p.take("SVarCompare") {
                spec.push_str(" | SVarCompare$ ");
                spec.push_str(&cmp);
            }
            return self.etb_counter_condition(&spec).map(Some).ok_or(());
        }
        Ok(None)
    }

    fn wave26f_take_etb_body_condition(&mut self, p: &mut Params) -> Result<Option<String>, ()> {
        if let Some(c) = p.take("Condition") {
            return self
                .etb_counter_condition(&format!("Condition$ {c}"))
                .map(Some)
                .ok_or(());
        }
        if let Some(present) = p.take("ConditionPresent") {
            let mut spec = format!("IsPresent$ {present}");
            if let Some(cmp) = p.take("ConditionCompare") {
                spec.push_str(" | PresentCompare$ ");
                spec.push_str(&cmp);
            }
            if let Some(def) = p.take("ConditionDefined") {
                spec.push_str(" | PresentDefined$ ");
                spec.push_str(&def);
            }
            return self.etb_counter_condition(&spec).map(Some).ok_or(());
        }
        if let Some(var) = p.take("ConditionCheckSVar") {
            let mut spec = format!("CheckSVar$ {var}");
            if let Some(cmp) = p.take("ConditionSVarCompare") {
                spec.push_str(" | SVarCompare$ ");
                spec.push_str(&cmp);
            }
            return self.etb_counter_condition(&spec).map(Some).ok_or(());
        }
        Ok(None)
    }

    #[allow(clippy::too_many_lines)]
    fn try_wave24e_frankenstein_entry(&mut self, mut p: Params) -> Option<()> {
        if p.take("ETB").as_deref() != Some("True")
            || p.take("Choices").as_deref()
                != Some("Creature.YouOwn+NotDefinedReplacedSimultaneousETB")
            || p.take("ChoiceZone").as_deref() != Some("Graveyard")
            || p.take("Mandatory").as_deref() != Some("True")
        {
            return None;
        }
        let x_var = p.take("Amount")?;
        let chk_yard = p.take("ConditionCheckSVar")?;
        let chk_yard_cmp = p.take("ConditionSVarCompare")?;
        let exile_sub = p.take("SubAbility")?;
        if !p.exhausted()
            || self.svars.get(x_var.trim()).map(|s| s.trim()) != Some("Count$xPaid")
            || chk_yard_cmp != format!("GE{x_var}")
            || self.svars.get(chk_yard.trim()).map(|s| s.trim())
                != Some("Count$ValidGraveyard Creature.YouOwn+NotDefinedReplacedSimultaneousETB")
        {
            return None;
        }
        let (e_api, mut ep) = Params::parse(self.svars.get(exile_sub.trim())?)?;
        ep.drop_prose();
        if e_api != "ChangeZone"
            || ep.take("Defined").as_deref() != Some("ChosenCard")
            || ep.take("Origin").as_deref() != Some("Graveyard")
            || ep.take("Destination").as_deref() != Some("Exile")
            || ep.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        let yard_sub = ep.take("SubAbility")?;
        if !ep.exhausted() {
            return None;
        }
        let (y_api, mut yp) = Params::parse(self.svars.get(yard_sub.trim())?)?;
        yp.drop_prose();
        if y_api != "ChangeZone"
            || yp.take("Hidden").as_deref() != Some("True")
            || yp.take("Origin").as_deref() != Some("All")
            || yp.take("Destination").as_deref() != Some("Graveyard")
            || yp.take("Defined").as_deref() != Some("Self")
            || yp.take("Imprint").as_deref() != Some("True")
            || yp.take("ETB").as_deref() != Some("True")
        {
            return None;
        }
        let chk_ex = yp.take("ConditionCheckSVar")?;
        let chk_ex_cmp = yp.take("ConditionSVarCompare")?;
        let ctr_sub = yp.take("SubAbility")?;
        if !yp.exhausted()
            || chk_ex_cmp != format!("LT{x_var}")
            || self.svars.get(chk_ex.trim()).map(|s| s.trim())
                != Some("Count$ValidExile Card.IsRemembered")
        {
            return None;
        }
        let (c_api, mut cp) = Params::parse(self.svars.get(ctr_sub.trim())?)?;
        cp.drop_prose();
        if c_api != "PutCounter"
            || cp.take("CounterNum").as_deref() != Some(x_var.as_str())
            || cp.take("SplitAmount").as_deref() != Some("True")
            || cp.take("ETB").as_deref() != Some("True")
            || cp.take("ConditionDefined").as_deref() != Some("Imprinted")
            || cp.take("ConditionPresent").as_deref() != Some("Card")
            || cp.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let kinds_raw = cp.take("CounterTypes")?;
        let upd_sub = cp.take("SubAbility")?;
        if !cp.exhausted() {
            return None;
        }
        let mut kinds = Vec::new();
        for tok in kinds_raw.split(',').map(str::trim) {
            kinds.push(super::counter_kind(tok)?);
        }
        if kinds.len() < 2 {
            return None;
        }
        let (u_api, mut up) = Params::parse(self.svars.get(upd_sub.trim())?)?;
        up.drop_prose();
        if u_api != "ReplaceEffect"
            || up.take("ConditionDefined").as_deref() != Some("Imprinted")
            || up.take("ConditionPresent").as_deref() != Some("Card")
            || up.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let clean_sub = up.take("SubAbility")?;
        if !up.exhausted() {
            return None;
        }
        let (cl_api, mut clp) = Params::parse(self.svars.get(clean_sub.trim())?)?;
        clp.drop_prose();
        if cl_api != "Cleanup"
            || clp.take("ClearRemembered").as_deref() != Some("True")
            || clp.take("ClearImprinted").as_deref() != Some("True")
            || !clp.exhausted()
        {
            return None;
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::ExileFromGraveyardXOrGraveyard(&Filter::CREATURE)".to_string());
        self.body.enter_modifiers.push(format!(
            "EnterModifier::ChooseCounterPerEntryCount(&[{}])",
            kinds.join(", ")
        ));
        Some(())
    }

    fn try_wave24e_devour_replacement(&mut self, mut p: Params) -> Option<()> {
        if !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || p.take("RememberSacrificed").as_deref() != Some("True")
            || p.take("Optional").as_deref() != Some("True")
        {
            return None;
        }
        p.claim_label("SacMessage", "SacValid");
        let sac_valid = p.take("SacValid")?;
        let amount_var = p.take("Amount")?;
        let put_sub = p.take("SubAbility")?;
        if !p.exhausted() {
            return None;
        }
        // Verify `Amount$` is "any number of `<SacValid>` you control" so
        // `EnterModifier::Devour`'s unconstrained-count selection is exact.
        let amount_def = self.svars.get(amount_var.trim())?.trim();
        let amount_valid = amount_def.strip_prefix("Count$Valid ")?.trim();
        let normalize_sac = |raw: &str, require_you_ctrl: bool| -> Option<Vec<String>> {
            let mut out = Vec::new();
            for alt in raw.split(',').map(str::trim) {
                let (noun, attrs) = alt.split_once('.').map_or((alt, ""), |(n, a)| (n, a));
                let mut has_you_ctrl = false;
                let mut kept = Vec::new();
                for piece in attrs.split('+').map(str::trim).filter(|s| !s.is_empty()) {
                    if piece == "YouCtrl" {
                        has_you_ctrl = true;
                    } else if piece != "Other" {
                        kept.push(piece);
                    }
                }
                if require_you_ctrl && !has_you_ctrl {
                    return None;
                }
                if kept.is_empty() {
                    out.push(noun.to_string());
                } else {
                    out.push(format!("{noun}.{}", kept.join("+")));
                }
            }
            (!out.is_empty()).then_some(out)
        };
        let norm_sac = normalize_sac(&sac_valid, false)?;
        let norm_amt = normalize_sac(amount_valid, true)?;
        if norm_sac != norm_amt {
            return None;
        }

        let put_str = self.svars.get(put_sub.trim())?.clone();
        let (p_api, mut pp) = Params::parse(&put_str)?;
        pp.drop_prose();
        if p_api != "PutCounter"
            || pp.take("ETB").as_deref() != Some("True")
            || !matches!(pp.take("Defined").as_deref(), None | Some("Self"))
            || pp.take("CounterType").as_deref() != Some("P1P1")
        {
            return None;
        }
        let num_var = pp.take("CounterNum")?;
        let clean_sub = pp.take("SubAbility")?;
        if !pp.exhausted() {
            return None;
        }
        let (cl_api, mut clp) = Params::parse(self.svars.get(clean_sub.trim())?)?;
        clp.drop_prose();
        if cl_api != "Cleanup"
            || clp.take("ClearRemembered").as_deref() != Some("True")
            || !clp.exhausted()
        {
            return None;
        }
        let num_def = self.svars.get(num_var.trim())?.trim();
        let factor: u8 = match num_def {
            "Remembered$Amount" | "Count$RememberedSize" => 1,
            "Remembered$Amount/Twice" | "Count$RememberedSize/Twice" => 2,
            "Remembered$Amount/Thrice" | "Count$RememberedSize/Thrice" => 3,
            other => {
                let mult = other
                    .strip_prefix("Remembered$Amount/Times.")
                    .or_else(|| other.strip_prefix("Count$RememberedSize/Times."))?;
                mult.trim().parse::<u8>().ok().filter(|&n| n >= 1)?
            }
        };
        let filter_expr = self.filter_expr(&norm_sac.join(","))?;
        let filter_ref = match filter_expr.as_str() {
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::PERMANENT" => "&Filter::PERMANENT".to_string(),
            _ => format!("&{}", self.body.filter_static("DEVOUR", &filter_expr)),
        };
        self.body.enter_modifiers.push(format!(
            "EnterModifier::Devour {{ filter: {filter_ref}, factor: {factor} }}"
        ));
        Some(())
    }

    /// "You may pay N life. If you don’t, this enters tapped" (shocklands,
    /// `EnterModifier::TappedOrPayLife`) and "As this enters, you may reveal a
    /// [filter] card from your hand. If you don't, this enters tapped" (Lorwyn
    /// reveal lands and Shadows/M21 snarls, `EnterModifier::TappedUnlessReveal`).
    pub(super) fn enters_tapped_or_pays(
        &mut self,
        body: &mut Params,
        cost: &str,
    ) -> Option<String> {
        match body.take("UnlessPayer").as_deref() {
            Some("You") => {}
            other => {
                self.note(format!(
                    "replacement `Moved` charging `{}`",
                    other.unwrap_or("nobody")
                ));
                return None;
            }
        }
        if let Some(life) = cost
            .strip_prefix("PayLife<")
            .and_then(|rest| rest.strip_suffix('>'))
            .and_then(|n| n.parse::<u16>().ok())
        {
            return Some(format!("EnterModifier::TappedOrPayLife({life})"));
        }
        if let Some(reveal) = cost
            .strip_prefix("Reveal<")
            .and_then(|rest| rest.strip_suffix('>'))
            && let Some(modifier) = self.enters_tapped_unless_reveal(reveal)
        {
            return Some(modifier);
        }
        self.note(format!("replacement `Moved` charging `{cost}`"));
        None
    }

    /// "This enters tapped unless you control N or more …".
    ///
    /// **The comparison is on the tap, and the card prints the opposite.**
    /// The reference says when the land comes down *tapped* — Rockfall Vale
    /// is `ConditionCompare$ LT2` over `Land.YouCtrl` and prints "enters
    /// tapped unless you control two or more other lands" — so `LT n` is
    /// `at_least = n` and `LE n` is `at_least = n + 1`. Canopy Vista settles
    /// that the arithmetic is right rather than plausible: its script writes
    /// `LE1` and its hand-written card in this pool writes `at_least: 2`.
    ///
    /// "Other" needs no clause. `controls_at_least` skips the entering
    /// object itself, so `Land.YouCtrl` counts the other lands whether or
    /// not the script spells `+Other` — and the ten that do spell it emit a
    /// redundant `Filter::Another` rather than a wrong count.
    ///
    /// `GT` and `GE` are the **upper** bound and come out as
    /// [`EnterModifier::TappedUnlessAtMost`]: `GT n` is `at_most = n` and
    /// `GE n` is `n - 1`. One predicate, and the corpus writes it from both
    /// ends — a fast land prints the bound ("unless you control two or fewer
    /// other lands", `GT2`, ten scripts) and the Forgotten Realms manlands
    /// print the complement ("if you control two or more other lands, this
    /// land enters tapped", `GE2` on three of them and `GT1` on the other
    /// two). Fifteen scripts, three spellings, one `at_most`.
    ///
    /// `GE0` is refused rather than read as `at_most` underflowing: a land
    /// that is tapped whatever the board is not this sentence, and the
    /// corpus writes it nowhere.
    pub(super) fn enters_tapped_unless(
        &mut self,
        body: &mut Params,
        present: &str,
    ) -> Option<String> {
        let Some(cmp) = body.take("ConditionCompare") else {
            self.note("replacement `Moved` counting with no `ConditionCompare$`".to_string());
            return None;
        };
        let lt = cmp.strip_prefix("LT").and_then(|n| n.parse::<u16>().ok());
        let le = cmp
            .strip_prefix("LE")
            .and_then(|n| n.parse::<u16>().ok())
            .and_then(|n| n.checked_add(1));
        let gt = cmp.strip_prefix("GT").and_then(|n| n.parse::<u16>().ok());
        let ge = cmp
            .strip_prefix("GE")
            .and_then(|n| n.parse::<u16>().ok())
            .and_then(|n| n.checked_sub(1));
        let bound = match (cmp.as_str(), lt, le, gt, ge) {
            ("EQ0", ..) => Bound::AtLeast(1),
            (_, Some(n), _, _, _) | (_, None, Some(n), _, _) if n >= 1 => Bound::AtLeast(n),
            (_, _, _, Some(n), _) | (_, _, _, None, Some(n)) => Bound::AtMost(n),
            _ => {
                self.note(format!(
                    "an enter-tapped condition `{cmp}` this rule cannot read"
                ));
                return None;
            }
        };
        let expr = self.filter_expr(present)?;
        let name = self.body.filter_static("CHECK", &expr);
        Some(match bound {
            // One spelling for one sentence: "unless you control at least
            // one" is what a checkland prints, and `TappedUnless` is the
            // variant that says it. Emitting `TappedUnlessCount { at_least:
            // 1 }` beside it would give that sentence a second form nothing
            // but a hash could tell from the first.
            Bound::AtLeast(1) => format!("EnterModifier::TappedUnless(&{name})"),
            Bound::AtLeast(n) => {
                format!("EnterModifier::TappedUnlessCount {{ filter: &{name}, at_least: {n} }}")
            }
            Bound::AtMost(n) => {
                format!("EnterModifier::TappedUnlessAtMost {{ filter: &{name}, at_most: {n} }}")
            }
        })
    }

    /// The two enters-tapped sentences whose condition counts **players**.
    ///
    /// `ConditionCheckSVar$` points at an `SVar` and `ConditionSVarCompare$`
    /// says when the *tap* happens, so both readings are the card's sentence
    /// turned around — the same inversion [`Self::enters_tapped_unless`]
    /// documents, and the reason each direction is spelled out here rather
    /// than shared:
    ///
    /// * `PlayerCountOpponents$Amount` with `LT2` taps while you have fewer
    ///   than two opponents, which is "unless you have two or more
    ///   opponents": `LT n` is `at_least = n`, `LE n` is `n + 1`.
    /// * `PlayerCountPlayers$LowestLifeTotal` with `GT13` taps while the
    ///   *lowest* life total at the table is above thirteen, which is
    ///   "unless a player has 13 or less life": `GT n` is `life = n`, `GE n`
    ///   is `n - 1`.
    ///
    /// The two take **opposite** comparators, and that is used rather than
    /// tolerated: a count accepts only `LT`/`LE` and a life total only
    /// `GT`/`GE`, so a pairing this rule has never seen is refused instead
    /// of being read with the direction silently flipped. Together they are
    /// 20 of the reference's 27 conditional enters-tapped lines; the other
    /// seven count something else and are refused by the definition not
    /// matching.
    pub(super) fn enters_tapped_unless_players(
        &mut self,
        body: &mut Params,
        svar: &str,
    ) -> Option<String> {
        let Some(defn) = self.svars.get(svar).cloned() else {
            self.note(format!("`ConditionCheckSVar$ {svar}` names no SVar"));
            return None;
        };
        let Some(cmp) = body.take("ConditionSVarCompare") else {
            self.note("replacement `Moved` counting an SVar with no comparison".to_string());
            return None;
        };
        let value = |prefix: &str| cmp.strip_prefix(prefix).and_then(|n| n.parse::<i64>().ok());
        let modifier = match defn.trim() {
            "PlayerCountOpponents$Amount" => {
                let at_least = match (value("LT"), value("LE")) {
                    (Some(n), _) => n,
                    (None, Some(n)) => n + 1,
                    (None, None) => return self.unread_enter_condition(&defn, &cmp),
                };
                let Ok(at_least) = u8::try_from(at_least) else {
                    return self.unread_enter_condition(&defn, &cmp);
                };
                format!("EnterModifier::TappedUnlessOpponents {{ at_least: {at_least} }}")
            }
            "PlayerCountPlayers$LowestLifeTotal" => {
                let life = match (value("GT"), value("GE")) {
                    (Some(n), _) => n,
                    (None, Some(n)) => n - 1,
                    (None, None) => return self.unread_enter_condition(&defn, &cmp),
                };
                let Ok(life) = i32::try_from(life) else {
                    return self.unread_enter_condition(&defn, &cmp);
                };
                format!("EnterModifier::TappedUnlessSomeoneAtOrBelow {{ life: {life} }}")
            }
            _ => return self.unread_enter_condition(&defn, &cmp),
        };
        Some(modifier)
    }

    /// One refusal for both halves above, so the report names the *count*
    /// rather than the letter the corpus happened to write.
    pub(super) fn unread_enter_condition(&mut self, defn: &str, cmp: &str) -> Option<String> {
        self.note(format!(
            "an enter-tapped condition counting `{defn}` `{cmp}`"
        ));
        None
    }

    fn require_permanent_keyword(&mut self, kw: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_spell_card_type() || self.is_land_card_type() {
            return self.deny(format!("`{kw}` on a non-permanent card"));
        }
        Some(())
    }

    fn modular_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Modular")?;
        let Ok(n) = rest.trim().parse::<u32>() else {
            return self.deny(format!("keyword `Modular:{rest}`"));
        };
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::modular({n})"));
        if self
            .body
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::MODULAR")
        {
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                "Modifier::AddKeyword(KeywordSet::MODULAR)",
            ));
        } else {
            self.body.keywords.push("KeywordSet::MODULAR".to_string());
        }
        Some(())
    }

    fn renown_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Renown")?;
        let Some(n) = rest.trim().parse::<u8>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Renown:{rest}`"));
        };
        if !self.body.keywords.iter().any(|k| k == "KeywordSet::RENOWN") {
            self.body.keywords.push("KeywordSet::RENOWN".to_string());
        }
        self.body.abilities.push(format!(
            "triggered!(Trigger::DealsCombatDamageToPlayer(&Filter::This), \
             &[Effect::BecomeRenowned({n})], condition = Some(Condition::Not(&Condition::IsRenowned)))"
        ));
        Some(())
    }

    fn bushido_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Bushido")?;
        let Some(n) = rest.trim().parse::<u16>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Bushido:{rest}`"));
        };
        self.body.abilities.push(format!(
            "triggered!(Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE), \
             &[Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
             power: Amount::Fixed({n}), toughness: Amount::Fixed({n}), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }}])"
        ));
        Some(())
    }

    fn soulshift_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Soulshift")?;
        let Ok(n) = rest.trim().parse::<u32>() else {
            return self.deny(format!("keyword `Soulshift:{rest}`"));
        };
        let filter_expr = format!(
            "Filter::And(&[Filter::HasSubtype(subtypes::creature::SPIRIT), Filter::CmcAtMost({n})])"
        );
        let filter_name = self.body.filter_static("TARGET", &filter_expr);
        self.body.abilities.push(format!(
            "triggered!(Trigger::Dies(&Filter::This), &[Effect::MayDo {{ \
             effects: &[Effect::GraveyardToHand {{ \
             target: TargetSpec::CardInGraveyard(&{filter_name}, PlayerRel::You) }}] }}], \
             targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&{filter_name}, PlayerRel::You))))"
        ));
        Some(())
    }

    fn attach_token_etb(&mut self, kw: &str, stem: &str) -> Option<()> {
        self.require_permanent_keyword(kw)?;
        let Some(tokens) = self.tokens else {
            return self.deny(format!(
                "`{kw}` with no token scripts to read `{stem}` against"
            ));
        };
        let Some(tok) = tokens.body(stem, self.cats) else {
            return self.deny(format!("token script `{stem}` for `{kw}`"));
        };
        let token = format!("&generated_tokens::{}", tok.constant);
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::CreateTokenThenAttachSelf {{ token: {token} }}])"
        ));
        Some(())
    }

    fn reconfigure_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Reconfigure")?;
        if rest.trim().is_empty() {
            return self.deny("an empty `Reconfigure` cost".to_string());
        }
        let mut parsed_costs = Vec::new();
        for arm in rest.split(':') {
            let trimmed = arm.trim();
            if trimmed.is_empty() {
                return self.deny(format!("keyword `Reconfigure:{rest}`"));
            }
            parsed_costs.push(self.parse_activated_cost(trimmed, None)?);
        }
        let remove_type = "static_ability!(Filter::This, Modifier::RemoveType(TypeSet::CREATURE), \
             condition = Some(Condition::SourceMatches(&Filter::IsAttached)))";
        if !self.body.abilities.iter().any(|a| a == remove_type) {
            self.body.abilities.push(remove_type.to_string());
        }
        for cost in parsed_costs {
            self.body.abilities.push(format!(
                "activated!({cost}, &[Effect::ReconfigureAttach], \
                 target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)), \
                 timing = ActivationTiming::SorcerySpeed)"
            ));
            self.body.abilities.push(format!(
                "activated!({cost}, &[Effect::DetachSource], \
                 timing = ActivationTiming::SorcerySpeed, \
                 condition = Some(Condition::SourceMatches(&Filter::IsAttached)))"
            ));
        }
        Some(())
    }

    fn class_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Class")?;
        if !self.type_line.split_whitespace().any(|w| w == "Class") {
            return self.deny("`Class` keyword without `Class` subtype".to_string());
        }
        let mut parts = rest.splitn(3, ':');
        let (Some(lvl_raw), Some(cost_raw), Some(actions_raw)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return self.deny(format!("keyword `Class:{rest}`"));
        };
        let Ok(level) = lvl_raw.trim().parse::<u8>() else {
            return self.deny(format!("Class level `{lvl_raw}`"));
        };
        if level != self.class_next_level || level < 2 {
            return self.deny(format!("non-sequential Class level `{level}`"));
        }
        let cost_raw = cost_raw.trim();
        if cost_raw.is_empty() {
            return self.deny(format!("Class level {level} with an empty cost"));
        }
        let cost = self.parse_activated_cost(cost_raw, None)?;
        self.class_next_level = level.saturating_add(1);
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::SetClassLevel({level})], \
             timing = ActivationTiming::SorcerySpeed, \
             condition = Some(Condition::ClassLevelExactly({})))",
            level - 1
        ));
        let mut any_action = false;
        for clause in actions_raw
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            any_action = true;
            self.class_action(level, clause)?;
        }
        if !any_action {
            return self.deny(format!("Class level {level} with no actions"));
        }
        Some(())
    }

    fn class_action(&mut self, level: u8, clause: &str) -> Option<()> {
        let Some((key, val)) = clause.split_once('$') else {
            return self.deny(format!("Class action `{clause}`"));
        };
        let key = key.trim();
        let val = val.trim();
        if val.is_empty() {
            return self.deny(format!("Class action `{clause}` naming no SVar"));
        }
        let names: Vec<&str> = val
            .split('&')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if names.is_empty() {
            return self.deny(format!("Class action `{clause}` naming no SVar"));
        }
        let cond = format!("Condition::ClassLevelAtLeast({level})");
        match key {
            "AddStaticAbility" => {
                for name in names {
                    self.class_static_action(name, &cond)?;
                }
                Some(())
            }
            "AddTrigger" => {
                for name in names {
                    self.class_trigger_action(level, name, &cond)?;
                }
                Some(())
            }
            "AddReplacementEffect" => {
                for name in names {
                    self.class_replacement_action(name, &cond)?;
                }
                Some(())
            }
            _ => self.deny(format!("Class action `{clause}`")),
        }
    }

    fn class_static_action(&mut self, name: &str, cond: &str) -> Option<()> {
        let Some(svar) = self.svars.get(name).cloned() else {
            return self.deny(format!("`AddStaticAbility$ {name}` naming no `SVar`"));
        };
        let before_ab = self.body.abilities.len();
        let before_en = self.body.enter_modifiers.len();
        let before_kw = self.body.keywords.len();
        let prev_x = std::mem::replace(&mut self.has_x, false);
        let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
        let ok = self.static_ability(&svar);
        self.has_x = prev_x;
        self.on_a_spell = prev_spell;
        ok?;
        if self.body.enter_modifiers.len() != before_en
            || self.body.keywords.len() != before_kw
            || self.body.abilities.len() == before_ab
        {
            return self.deny(format!("Class `AddStaticAbility$ {name}` side effect"));
        }
        for ability in &mut self.body.abilities[before_ab..] {
            *ability = append_ability_condition(ability, cond)?;
        }
        Some(())
    }

    fn class_replacement_action(&mut self, name: &str, cond: &str) -> Option<()> {
        let Some(svar) = self.svars.get(name).cloned() else {
            return self.deny(format!("`AddReplacementEffect$ {name}` naming no `SVar`"));
        };
        let before_ab = self.body.abilities.len();
        let before_en = self.body.enter_modifiers.len();
        let before_kw = self.body.keywords.len();
        let prev_x = std::mem::replace(&mut self.has_x, false);
        let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
        let ok = self.replacement(&svar);
        self.has_x = prev_x;
        self.on_a_spell = prev_spell;
        ok?;
        if self.body.enter_modifiers.len() != before_en
            || self.body.keywords.len() != before_kw
            || self.body.abilities.len() == before_ab
        {
            return self.deny(format!("Class `AddReplacementEffect$ {name}` side effect"));
        }
        for ability in &mut self.body.abilities[before_ab..] {
            *ability = append_ability_condition(ability, cond)?;
        }
        Some(())
    }

    fn class_trigger_action(&mut self, level: u8, name: &str, cond: &str) -> Option<()> {
        let Some(svar) = self.svars.get(name).cloned() else {
            return self.deny(format!("Class `AddTrigger$ {name}` naming no `SVar`"));
        };
        if let Some((mode, p)) = Params::parse(&svar)
            && mode == "ClassLevelGained"
        {
            return self.class_level_gained_trigger(level, p);
        }
        let modifier = self.build_grant_triggered_modifier(name, "AddTrigger$")?;
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, {modifier}, condition = Some({cond}))"
        ));
        Some(())
    }

    fn class_level_gained_trigger(&mut self, level: u8, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => {
                return self.deny(format!("`ClassLevelGained` with `Secondary$ {other}`"));
            }
        }
        if p.take("ValidCard").as_deref() != Some("Card.Self") {
            return self.deny("`ClassLevelGained` for a non-self card".to_string());
        }
        if !matches!(
            p.take("TriggerZones").as_deref(),
            None | Some("Battlefield")
        ) {
            return self.deny("`ClassLevelGained` outside the battlefield".to_string());
        }
        let expected_level = p
            .take("ClassLevel")
            .and_then(|s| s.trim().parse::<u8>().ok());
        if expected_level != Some(level) {
            return self.deny(format!(
                "`ClassLevelGained` level mismatch for level {level}"
            ));
        }
        let Some(execute) = p.take("Execute") else {
            return self.deny("`ClassLevelGained` trigger with no `Execute$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ClassLevelGained.{key}`"));
        }
        let Some(exec_body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no `SVar`"));
        };
        if self.optional_trigger_price(&exec_body).is_some() {
            return self.deny("`ClassLevelGained` with `Cost$` on its executed line".to_string());
        }
        let prev_x = std::mem::replace(&mut self.has_x, false);
        let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
        let prev_self = std::mem::replace(&mut self.trigger_is_self, true);
        let mut chain = Chain::default();
        let ok = self.chain(&exec_body, &mut chain);
        self.has_x = prev_x;
        self.on_a_spell = prev_spell;
        self.trigger_is_self = prev_self;
        ok?;
        if chain.effects.is_empty()
            || chain.target_is_x
            || chain.target_unique
            || chain.second_target.is_some()
        {
            return self.deny("unsupported `ClassLevelGained` execution chain".to_string());
        }
        let targets = chain.target.as_deref().map_or_default(|t| {
            if let Some(max) = chain.target_up_to {
                if max == 1 {
                    format!(", targets = Some(TargetReq::up_to_one({t}))")
                } else {
                    format!(", targets = Some(TargetReq::up_to({t}, {max}))")
                }
            } else if let Some(n) = chain.target_exact {
                format!(", targets = Some(TargetReq::exactly({t}, {n}))")
            } else if let Some((min, max)) = chain.target_range {
                format!(
                    ", targets = Some({})",
                    Self::wave28a_format_range_req(t, min, max)
                )
            } else {
                format!(", targets = Some(TargetReq::one({t}))")
            }
        });
        self.body.abilities.push(format!(
            "triggered!(Trigger::ClassLevelGained({level}), &[{}]{targets})",
            chain.effects.join(", ")
        ));
        Some(())
    }

    fn overload_keyword(&mut self, rest: &str) -> Option<()> {
        if !self.is_spell_card_type() {
            return self.deny("`Overload` on a non-instant/non-sorcery card".to_string());
        }
        if self.overload_cost.is_some() {
            return self.deny("a second `Overload` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Overload", rest, true, true)?;
        if mana.is_empty() {
            return self.deny(format!("empty `Overload` mana cost `{rest}`"));
        }
        self.overload_cost = Some(mana);
        self.body.notes.push("overload".to_string());
        Some(())
    }

    pub(super) fn apply_overload(&mut self) -> Option<()> {
        let has_cipher = self.body.keywords.iter().any(|k| k == "KeywordSet::CIPHER");
        if let Some(overload_mana) = self.overload_cost.take() {
            if has_cipher {
                return self.deny("`Overload` beside `Cipher`".to_string());
            }
            if !self.has_x_mana_cost && overload_mana.contains("{X}") {
                return self.deny("`Overload` with `{X}` on a non-X spell".to_string());
            }
            if self.body.abilities.len() != 1
                || !self.body.alternative_costs.is_empty()
                || !self.body.additional_costs.is_empty()
                || !self.body.mandatory_additional_costs.is_empty()
                || !self.body.flashback_parts.is_empty()
                || !self.body.morph_parts.is_empty()
            {
                return self
                    .deny("`Overload` beside other spell costs or multiple abilities".to_string());
            }
            let Some((effect_raw, filter_ref)) = parse_single_target_spell(&self.body.abilities[0])
            else {
                return self.deny("`Overload` on an unsupported spell shape".to_string());
            };
            let Some(overloaded_effect) = map_overload_effect(effect_raw, filter_ref) else {
                return self.deny("`Overload` on an unsupported spell effect".to_string());
            };
            self.body.abilities[0] = format!(
                "AbilityDef::ModalSpell {{ choose: ModeCount::ONE, modes: &[\
                 mode!(&[{effect_raw}], targets = Some(TargetReq::one(TargetSpec::Object({filter_ref})))), \
                 mode!(&[{overloaded_effect}], cost_override = Some(mana!(\"{overload_mana}\")))] }}"
            );
        }
        if has_cipher {
            self.apply_cipher()?;
        }
        Some(())
    }

    fn apply_cipher(&mut self) -> Option<()> {
        if self.body.abilities.len() != 1 {
            return self.deny("`Cipher` on a spell without a single spell ability".to_string());
        }
        let Some(updated) = append_effect_to_spell_macro(&self.body.abilities[0], "Effect::Cipher")
        else {
            return self.deny("`Cipher` on an unsupported spell shape".to_string());
        };
        self.body.abilities[0] = updated;
        Some(())
    }

    fn push_keyword_once(&mut self, bit: &str) {
        if !self.body.keywords.iter().any(|k| k == bit) {
            self.body.keywords.push(bit.to_string());
        }
    }

    fn require_aura_permanent(&mut self, kw: &str) -> Option<()> {
        self.require_permanent_keyword(kw)?;
        if !self.type_line.split_whitespace().any(|w| w == "Aura") {
            return self.deny(format!("`{kw}` on a non-Aura card"));
        }
        Some(())
    }

    /// `K:Strive:<mana>` (`CR 207.2c` / `CR 601.2f`): per-extra-target mana cost.
    fn strive_keyword(&mut self, rest: &str) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Strive` on a non-instant/non-sorcery card".to_string());
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.starts_with("AbilityDef::strive("))
        {
            return self.deny("a second `Strive` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Strive", rest, false, true)?;
        self.body
            .abilities
            .push(format!("AbilityDef::strive(mana!(\"{mana}\"))"));
        self.body.notes.push("strive".to_string());
        Some(())
    }

    /// `K:Transmute:<mana>` (`CR 702.53a`): sorcery-speed hand discard tutor.
    fn transmute_keyword(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Transmute` on a land or untyped card".to_string());
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.starts_with("AbilityDef::transmute"))
        {
            return self.deny("a second `Transmute` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Transmute", rest, false, true)?;
        self.push_keyword_once("KeywordSet::TRANSMUTE");
        self.body
            .abilities
            .push(format!("AbilityDef::transmute(mana!(\"{mana}\"))"));
        self.body.notes.push("transmute".to_string());
        Some(())
    }

    /// `K:Scavenge:<mana>` (`CR 702.97a`): sorcery-speed graveyard exile-self counter ability.
    fn scavenge_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Scavenge")?;
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.starts_with("AbilityDef::scavenge"))
        {
            return self.deny("a second `Scavenge` cost on one card".to_string());
        }
        let (mana, _) = self.parse_alt_cost_pieces("Scavenge", rest, false, true)?;
        self.push_keyword_once("KeywordSet::SCAVENGE");
        self.body
            .abilities
            .push(format!("AbilityDef::scavenge(mana!(\"{mana}\"))"));
        self.body.notes.push("scavenge".to_string());
        Some(())
    }

    /// `K:Outlast:<mana>` (`CR 702.107a`): sorcery-speed `{cost}, {T}: Put a +1/+1 counter on this creature.`
    fn outlast_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Outlast")?;
        let (mana, _) = self.parse_alt_cost_pieces("Outlast", rest, false, true)?;
        let cost = crate::body::cost_literal(&mana, &["TapSelf".to_string()]);
        self.push_keyword_once("KeywordSet::OUTLAST");
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::AddCounterSource {{ kind: CounterKind::P1P1, \
             amount: Amount::Fixed(1) }}], timing = ActivationTiming::SorcerySpeed)"
        ));
        self.body.notes.push("outlast".to_string());
        Some(())
    }

    /// `K:Reinforce:<N|X>:<mana>` (`CR 702.77a`): hand discard ability putting N/X +1/+1 counters on target creature.
    fn reinforce_keyword(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Reinforce` on a land or untyped card".to_string());
        }
        let Some((count_raw, cost_raw)) = split_two_top_level_colons(rest) else {
            return self.deny(format!("keyword `Reinforce:{rest}`"));
        };
        let count_raw = count_raw.trim();
        let (amount, mana) = if count_raw == "X" {
            let mana = self.parse_x_mana_only("Reinforce", cost_raw.trim())?;
            ("Amount::X".to_string(), mana)
        } else if let Some(n) = count_raw.parse::<u32>().ok().filter(|&n| n >= 1) {
            let (mana, _) =
                self.parse_alt_cost_pieces("Reinforce", cost_raw.trim(), false, true)?;
            (format!("Amount::Fixed({n})"), mana)
        } else {
            return self.deny(format!("keyword `Reinforce:{rest}`"));
        };
        let cost = crate::body::cost_literal(&mana, &["DiscardSelf".to_string()]);
        self.push_keyword_once("KeywordSet::REINFORCE");
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::AddCounter {{ kind: CounterKind::P1P1, \
             amount: {amount} }}], target = Some(TargetSpec::Object(&Filter::CREATURE)), \
             zone = ActivationZone::Hand)"
        ));
        self.body.notes.push("reinforce".to_string());
        Some(())
    }

    /// `K:Freerunning:<cost>` (`CR 702.173a`): alternative cast cost if an Assassin or commander dealt combat damage this turn.
    fn freerunning_keyword(&mut self, rest: &str) -> Option<()> {
        self.keyword_alternative_cost("Freerunning", "freerunning", rest, true)
    }

    /// `K:Sneak:<cost>` (`CR 702.190a–b`): static alternative casting rule.
    fn sneak_keyword(&mut self, rest: &str) -> Option<()> {
        if self.type_line.is_empty() || self.is_land_card_type() {
            return self.deny("`Sneak` on a land or untyped card".to_string());
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.starts_with("AbilityDef::sneak("))
        {
            return self.deny("a second `Sneak` cost on one card".to_string());
        }
        let (mana, parts) = self.parse_alt_cost_pieces("Sneak", rest, false, false)?;
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body
            .abilities
            .push(format!("AbilityDef::sneak({cost})"));
        self.body.notes.push("sneak".to_string());
        Some(())
    }

    /// `K:Splice:<onto>:<cost>` (`CR 702.47a`): splice onto Arcane or spell filter.
    fn splice_keyword(&mut self, rest: &str) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Splice` on a non-instant/non-sorcery card".to_string());
        }
        let Some((onto_raw, cost_raw)) = split_two_top_level_colons(rest) else {
            return self.deny(format!("keyword `Splice:{rest}`"));
        };
        let onto_raw = onto_raw.trim();
        let onto_expr = if onto_raw == "Arcane" {
            "Filter::HasSubtype(subtypes::spell::ARCANE)".to_string()
        } else {
            self.filter_expr_in_zone(onto_raw, true)?
        };
        let filter_name = self.body.filter_static("SPLICE", &onto_expr);
        let (mana, parts) = self.parse_cost_pieces_with(
            "Splice",
            cost_raw.trim(),
            false,
            false,
            is_spell_either_cost_part,
        )?;
        self.push_keyword_once("KeywordSet::SPLICE");
        let ability = if parts.is_empty() {
            format!("AbilityDef::splice_mana(&{filter_name}, mana!(\"{mana}\"))")
        } else {
            let cost = crate::body::cost_literal(&mana, &parts);
            format!("AbilityDef::splice(&{filter_name}, {cost})")
        };
        self.body.abilities.push(ability);
        self.body.notes.push("splice".to_string());
        Some(())
    }

    /// `K:Awaken:<N>:<cost>` (`CR 702.113a`): alternative spell cost animating a land with N +1/+1 counters.
    fn awaken_keyword(&mut self, rest: &str) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Awaken` on a non-instant/non-sorcery card".to_string());
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.starts_with("AbilityDef::awaken"))
        {
            return self.deny("a second `Awaken` cost on one card".to_string());
        }
        let Some((count_raw, cost_raw)) = split_two_top_level_colons(rest) else {
            return self.deny(format!("keyword `Awaken:{rest}`"));
        };
        let Some(n) = count_raw.trim().parse::<u16>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Awaken:{rest}`"));
        };
        let (mana, parts) = self.parse_alt_cost_pieces("Awaken", cost_raw.trim(), false, false)?;
        self.push_keyword_once("KeywordSet::AWAKEN");
        let ability = if parts.is_empty() {
            format!("AbilityDef::awaken({n}, mana!(\"{mana}\"))")
        } else {
            let cost = crate::body::cost_literal(&mana, &parts);
            format!("AbilityDef::awaken_cost({n}, {cost})")
        };
        self.body.abilities.push(ability);
        self.body.notes.push("awaken".to_string());
        Some(())
    }

    /// `K:Conspire` (`CR 702.78a–b`): optional spell tap-2 copy keyword.
    fn conspire_keyword(&mut self) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Conspire` on a non-instant/non-sorcery card".to_string());
        }
        if self
            .body
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::CONSPIRE")
        {
            self.body.abilities.push(
                "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CONSPIRE), \
                 zone = ZoneRef::Stack)"
                    .to_string(),
            );
        } else {
            self.body.keywords.push("KeywordSet::CONSPIRE".to_string());
        }
        self.body.notes.push("conspire".to_string());
        Some(())
    }

    /// `K:Mobilize:<N|X>` (`CR 702.181a`): attack trigger creating N/X tapped-and-attacking 1/1 red Warriors.
    fn mobilize_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Mobilize")?;
        let trimmed = rest.trim();
        if trimmed.is_empty() || trimmed.contains(':') {
            return self.deny(format!("keyword `Mobilize:{rest}`"));
        }
        let effect = if let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n >= 1) {
            format!("Effect::mobilize({n})")
        } else if trimmed == "X" {
            let Some(def) = self.svars.get("X").cloned() else {
                return self.deny("`Mobilize:X` without `SVar:X`".to_string());
            };
            let Some(amt) = self.static_count_expr(def.trim()) else {
                return self.deny(format!("unsupported `Mobilize:X` count `{def}`"));
            };
            format!("Effect::Mobilize({amt})")
        } else {
            return self.deny(format!("keyword `Mobilize:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::MOBILIZE");
        self.body.abilities.push(format!(
            "triggered!(Trigger::Attacks(&Filter::This), &[{effect}])"
        ));
        Some(())
    }

    /// `K:Afterlife:<N>` (`CR 702.135a`): dies trigger creating N 1/1 W/B flying Spirit tokens.
    fn afterlife_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Afterlife")?;
        let trimmed = rest.trim();
        if trimmed.contains(':') {
            return self.deny(format!("keyword `Afterlife:{rest}`"));
        }
        let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Afterlife:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::AFTERLIFE");
        self.body.abilities.push(format!(
            "triggered!(Trigger::Dies(&Filter::This), &[Effect::CreateTokenN {{ \
             token: &generated_tokens::SPIRIT_1_1_WHITE_BLACK_FLYING, amount: Amount::Fixed({n}) }}])"
        ));
        Some(())
    }

    /// `K:Afflict:<N>` (`CR 702.130a`): becomes-blocked trigger making defending player lose N life.
    fn afflict_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Afflict")?;
        let trimmed = rest.trim();
        if trimmed.contains(':') {
            return self.deny(format!("keyword `Afflict:{rest}`"));
        }
        let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Afflict:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::AFFLICT");
        self.body.abilities.push(format!(
            "triggered!(Trigger::BecomesBlocked, &[Effect::LoseLife {{ \
             amount: Amount::Fixed({n}), target: PlayerRel::DefendingPlayer }}])"
        ));
        Some(())
    }

    /// `K:Sunburst` (`CR 702.44a–b`): enters with counters per color of mana spent (`KeywordSet::SUNBURST` derived).
    fn sunburst_keyword(&mut self) -> Option<()> {
        self.require_permanent_keyword("Sunburst")?;
        self.body
            .enter_modifiers
            .push("EnterModifier::Sunburst".to_string());
        Some(())
    }

    /// `K:Ravenous` (`CR 702.156a`): enters with X +1/+1 counters and draws a card on ETB if X >= 5.
    fn ravenous_keyword(&mut self) -> Option<()> {
        self.require_permanent_keyword("Ravenous")?;
        if !self.has_x_mana_cost {
            return self.deny("`Ravenous` on a card without `{X}` in its mana cost".to_string());
        }
        if self
            .body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ravenous()")
        {
            return self.deny("a second `Ravenous` keyword on one card".to_string());
        }
        self.push_keyword_once("KeywordSet::RAVENOUS");
        self.body
            .enter_modifiers
            .push("EnterModifier::ravenous()".to_string());
        self.body
            .abilities
            .push("AbilityDef::ravenous_draw()".to_string());
        Some(())
    }

    /// `K:Graft:<N>` (`CR 702.58a`): enters with N +1/+1 counters and may move one onto entering creatures.
    fn graft_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Graft")?;
        let trimmed = rest.trim();
        if trimmed.contains(':') {
            return self.deny(format!("keyword `Graft:{rest}`"));
        }
        let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Graft:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::GRAFT");
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::graft({n})"));
        self.body
            .abilities
            .push("AbilityDef::graft_trigger()".to_string());
        Some(())
    }

    /// `K:Tribute:<N>` (`CR 702.104a–b`): opponent choice of N +1/+1 counters on entry + linked `TrigNotTribute` ETB trigger.
    fn tribute_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Tribute")?;
        let trimmed = rest.trim();
        if trimmed.contains(':') {
            return self.deny(format!("keyword `Tribute:{rest}`"));
        }
        if self
            .body
            .enter_modifiers
            .iter()
            .any(|m| m.starts_with("EnterModifier::tribute("))
        {
            return self.deny("a second `Tribute` keyword on one card".to_string());
        }
        let Some(n) = trimmed.parse::<u32>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("keyword `Tribute:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::TRIBUTE");
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::tribute({n})"));
        if let Some(exec_body) = self.svars.get("TrigNotTribute").cloned() {
            if self.optional_trigger_price(&exec_body).is_some() {
                return self.deny("`Tribute` with `Cost$` on `TrigNotTribute`".to_string());
            }
            let prev_x = std::mem::replace(&mut self.has_x, false);
            let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
            let prev_self = std::mem::replace(&mut self.trigger_is_self, true);
            let prev_etb = std::mem::replace(&mut self.is_etb_self_trigger, true);
            let mut chain = Chain::default();
            let ok = self.chain(&exec_body, &mut chain);
            self.has_x = prev_x;
            self.on_a_spell = prev_spell;
            self.trigger_is_self = prev_self;
            self.is_etb_self_trigger = prev_etb;
            ok?;
            if chain.effects.is_empty() {
                return self.deny("`Tribute` `TrigNotTribute` with no effects".to_string());
            }
            let targets = Self::target_req_expr(&chain)
                .map_or_default(|req| format!(", targets = Some({req})"));
            let second = Self::second_target_req_expr(&chain)
                .map_or_default(|req| format!(", second_targets = Some({req})"));
            self.body.abilities.push(format!(
                "triggered!(Trigger::ETB, &[{}]{targets}{second}, \
                 condition = Some(Condition::TributeWasNotPaid))",
                chain.effects.join(", ")
            ));
        } else if self
            .oracle
            .to_ascii_lowercase()
            .contains("tribute wasn't paid")
        {
            return self.deny("`Tribute` missing `SVar:TrigNotTribute`".to_string());
        }
        Some(())
    }

    /// `K:Champion:<valid>` (`CR 702.72a–b`): exile another matching permanent on ETB and return on LTB.
    fn champion_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_permanent_keyword("Champion")?;
        let trimmed = rest.trim();
        if trimmed.is_empty() || trimmed.contains(':') {
            return self.deny(format!("keyword `Champion:{rest}`"));
        }
        let expr = self.filter_expr(trimmed)?;
        let filter = self.body.filter_static("CHAMPION", &expr);
        self.push_keyword_once("KeywordSet::CHAMPION");
        self.body
            .abilities
            .push(format!("AbilityDef::champion(&{filter})"));
        Some(())
    }

    /// `K:Cipher` (`CR 702.99a`): marks spell for `Effect::Cipher` post-pass in `apply_overload`.
    fn cipher_keyword(&mut self) -> Option<()> {
        if !self.is_spell_card_type() || self.type_line.is_empty() {
            return self.deny("`Cipher` on a non-instant/non-sorcery card".to_string());
        }
        if self.body.keywords.iter().any(|k| k == "KeywordSet::CIPHER") {
            return self.deny("a second `Cipher` keyword on one card".to_string());
        }
        self.body.keywords.push("KeywordSet::CIPHER".to_string());
        self.body.notes.push("cipher".to_string());
        Some(())
    }

    /// `K:Firebending:<N|X>[:<prose>]` (`CR 702.189a`): attack trigger adding N/X `{R}` until end of combat.
    fn firebending_keyword(&mut self, rest: &str) -> Option<()> {
        self.require_creature_permanent("Firebending")?;
        let mut fields = rest.split(':');
        let amt_raw = fields.next()?.trim();
        let _prose = fields.next();
        if fields.next().is_some() || amt_raw.is_empty() {
            return self.deny(format!("keyword `Firebending:{rest}`"));
        }
        let amount = if let Some(n) = amt_raw.parse::<u32>().ok().filter(|&n| n >= 1) {
            format!("Amount::Fixed({n})")
        } else if amt_raw == "X" {
            let Some(def) = self.svars.get("X").cloned() else {
                return self.deny("`Firebending:X` without `SVar:X`".to_string());
            };
            let trimmed = def.trim();
            if trimmed == "Count$CardPower" {
                "Amount::SourcePower".to_string()
            } else if let Some(expr) = self.static_count_expr(trimmed) {
                expr
            } else {
                return self.deny(format!("unsupported `Firebending:X` count `{trimmed}`"));
            }
        } else {
            return self.deny(format!("keyword `Firebending:{rest}`"));
        };
        self.push_keyword_once("KeywordSet::FIREBENDING");
        self.body.abilities.push(format!(
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::Firebend({amount})])"
        ));
        Some(())
    }

    /// `K:MayEffectFromOpeningHand:<svar>[:!PlayFirst]` (`CR 103.6`):
    /// - Leyline opening-hand onto battlefield (`AbilityDef::OPENING_HAND_ON_BATTLEFIELD`)
    /// - Gemstone Caverns (`!PlayFirst` + luck counter + exile hand card)
    /// - Reveal from opening hand for a one-off trigger at the beginning of the first upkeep
    ///   (`AbilityDef::opening_hand_reveal_at_first_upkeep`)
    fn may_effect_from_opening_hand(&mut self, rest: &str) -> Option<()> {
        if self.body.abilities.iter().any(|a| {
            a.contains("OPENING_HAND_") || a.contains("OpeningHand") || a.contains("opening_hand_")
        }) {
            return self.deny("a second `MayEffectFromOpeningHand` on one card".to_string());
        }
        let mut fields = rest.split(':');
        let svar = fields.next()?.trim();
        let not_first = match fields.next().map(str::trim) {
            None => false,
            Some("!PlayFirst") => true,
            Some(other) => {
                return self.deny(format!("`MayEffectFromOpeningHand` condition `{other}`"));
            }
        };
        if svar.is_empty() || fields.next().is_some() {
            return self.deny(format!("keyword `MayEffectFromOpeningHand:{rest}`"));
        }
        let Some(raw) = self.svars.get(svar).cloned() else {
            return self.deny(format!(
                "`MayEffectFromOpeningHand` naming missing SVar `{svar}`"
            ));
        };
        let Some((api, mut p)) = Params::parse(&raw) else {
            return self.deny(format!("malformed opening-hand SVar `{svar}`"));
        };
        p.drop_prose();
        match (api.as_str(), not_first) {
            ("ChangeZone", false) => self.opening_hand_leyline(svar, p),
            ("ChangeZone", true) => self.opening_hand_gemstone_caverns(svar, p),
            ("Reveal", false) => self.opening_hand_reveal_first_upkeep(svar, p),
            _ => self.deny(format!("unsupported `MayEffectFromOpeningHand:{rest}`")),
        }
    }

    fn opening_hand_leyline(&mut self, svar: &str, mut p: Params) -> Option<()> {
        if self.type_line.is_empty() || self.is_spell_card_type() {
            return self.deny("opening-hand battlefield entry on a non-permanent".to_string());
        }
        if p.take("Defined").as_deref() != Some("Self")
            || p.take("Origin").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || !p.exhausted()
        {
            return self.deny(format!("unsupported opening-hand `ChangeZone` `{svar}`"));
        }
        self.body
            .abilities
            .push("AbilityDef::OPENING_HAND_ON_BATTLEFIELD".to_string());
        Some(())
    }

    fn opening_hand_gemstone_caverns(&mut self, svar: &str, mut p: Params) -> Option<()> {
        if self.type_line.is_empty() || self.is_spell_card_type() {
            return self.deny("opening-hand battlefield entry on a non-permanent".to_string());
        }
        if p.take("Defined").as_deref() != Some("Self")
            || p.take("Origin").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Battlefield")
        {
            return self.deny(format!("unsupported `!PlayFirst` opening-hand `{svar}`"));
        }
        let Some(sub1_name) = p.take("SubAbility") else {
            return self.deny(format!("unsupported `!PlayFirst` opening-hand `{svar}`"));
        };
        if !p.exhausted() {
            return self.deny(format!("unsupported `!PlayFirst` opening-hand `{svar}`"));
        }
        let Some(sub1_raw) = self.svars.get(sub1_name.trim()).cloned() else {
            return self.deny(format!("missing opening-hand subability `{sub1_name}`"));
        };
        let Some((api1, mut p1)) = Params::parse(&sub1_raw) else {
            return self.deny(format!("malformed opening-hand subability `{sub1_name}`"));
        };
        p1.drop_prose();
        if api1 != "PutCounter"
            || p1.take("CounterType").as_deref() != Some("LUCK")
            || p1.take("CounterNum").as_deref() != Some("1")
            || p1.take("Defined").as_deref() != Some("Self")
        {
            return self.deny(format!(
                "unsupported `!PlayFirst` counter step `{sub1_name}`"
            ));
        }
        let Some(sub2_name) = p1.take("SubAbility") else {
            return self.deny(format!(
                "unsupported `!PlayFirst` counter step `{sub1_name}`"
            ));
        };
        if !p1.exhausted() {
            return self.deny(format!(
                "unsupported `!PlayFirst` counter step `{sub1_name}`"
            ));
        }
        let Some(sub2_raw) = self.svars.get(sub2_name.trim()).cloned() else {
            return self.deny(format!("missing opening-hand subability `{sub2_name}`"));
        };
        let Some((api2, mut p2)) = Params::parse(&sub2_raw) else {
            return self.deny(format!("malformed opening-hand subability `{sub2_name}`"));
        };
        p2.drop_prose();
        if api2 != "ChangeZone"
            || p2.take("Origin").as_deref() != Some("Hand")
            || p2.take("Destination").as_deref() != Some("Exile")
            || p2.take("ChangeType").as_deref() != Some("Card")
            || p2.take("ChangeNum").as_deref() != Some("1")
            || p2.take("Mandatory").as_deref() != Some("True")
            || !p2.exhausted()
        {
            return self.deny(format!("unsupported `!PlayFirst` exile step `{sub2_name}`"));
        }
        self.body.abilities.push(
            "AbilityDef::Replacement(\
             ReplacementRule::OpeningHandBattlefieldWithLuckCounterExilingHandCard)"
                .to_string(),
        );
        Some(())
    }

    fn opening_hand_reveal_first_upkeep(&mut self, svar: &str, mut p: Params) -> Option<()> {
        if p.take("RevealDefined").as_deref() != Some("Self") {
            return self.deny(format!("unsupported opening-hand `Reveal` `{svar}`"));
        }
        let Some(eff_name) = p.take("SubAbility") else {
            return self.deny(format!(
                "opening-hand `Reveal` without `SubAbility$` `{svar}`"
            ));
        };
        if !p.exhausted() {
            return self.deny(format!("unsupported opening-hand `Reveal` `{svar}`"));
        }
        let Some(eff_raw) = self.svars.get(eff_name.trim()).cloned() else {
            return self.deny(format!("missing opening-hand effect `{eff_name}`"));
        };
        let Some((eff_api, mut ep)) = Params::parse(&eff_raw) else {
            return self.deny(format!("malformed opening-hand effect `{eff_name}`"));
        };
        ep.drop_prose();
        if eff_api != "Effect" {
            return self.deny(format!("unsupported opening-hand effect `{eff_api}`"));
        }
        let Some(trig_name) = ep.take("Triggers") else {
            return self.deny(format!(
                "opening-hand `Effect` without `Triggers$` `{eff_name}`"
            ));
        };
        if !matches!(ep.take("Duration").as_deref(), None | Some("Permanent")) {
            return self.deny(format!("unsupported opening-hand `Effect` `{eff_name}`"));
        }
        if trig_name.contains(',') || !ep.exhausted() {
            return self.deny(format!("unsupported opening-hand `Effect` `{eff_name}`"));
        }
        let Some(trig_raw) = self.svars.get(trig_name.trim()).cloned() else {
            return self.deny(format!("missing opening-hand trigger `{trig_name}`"));
        };
        let Some((mode, mut tp)) = Params::parse(&trig_raw) else {
            return self.deny(format!("malformed opening-hand trigger `{trig_name}`"));
        };
        tp.drop_prose();
        // `OpeningHandRevealAtFirstUpkeep` fires on `DelayedWhen::NextUpkeepOfAnyone`
        // ("at the beginning of the first upkeep"), so `ValidPlayer$ You` ("your first
        // upkeep") and non-Upkeep phases stay honestly refused.
        if mode != "Phase"
            || tp.take("Phase").as_deref() != Some("Upkeep")
            || tp.take("OneOff").as_deref() != Some("True")
            || !matches!(tp.take("ValidPlayer").as_deref(), None | Some("Player"))
        {
            return self.deny(format!("unsupported opening-hand trigger `{trig_name}`"));
        }
        let Some(exec_name) = tp.take("Execute") else {
            return self.deny(format!(
                "opening-hand trigger without `Execute$` `{trig_name}`"
            ));
        };
        if !tp.exhausted() {
            return self.deny(format!("unsupported opening-hand trigger `{trig_name}`"));
        }
        let Some(exec_raw) = self.svars.get(exec_name.trim()).cloned() else {
            return self.deny(format!("missing opening-hand execute `{exec_name}`"));
        };
        let prev_x = std::mem::replace(&mut self.has_x, false);
        let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
        let prev_self = std::mem::replace(&mut self.trigger_is_self, false);
        let mut chain = Chain::default();
        let ok = self.chain(&exec_raw, &mut chain);
        self.has_x = prev_x;
        self.on_a_spell = prev_spell;
        self.trigger_is_self = prev_self;
        ok?;
        if chain.effects.is_empty() || chain.target.is_some() || chain.second_target.is_some() {
            return self.deny(format!(
                "unsupported opening-hand execute chain `{exec_name}`"
            ));
        }
        self.body.abilities.push(format!(
            "AbilityDef::opening_hand_reveal_at_first_upkeep(&[{}])",
            chain.effects.join(", ")
        ));
        Some(())
    }

    /// Parses a mana-only cost containing `{X}` without requiring a card-level `SVar:X:Count$xPaid`.
    fn parse_x_mana_only(&self, keyword: &str, raw: &str) -> Option<String> {
        let tokens = super::cost_parts(raw);
        if tokens.is_empty() {
            return self.deny(format!("an empty `{keyword}` cost"));
        }
        let mut mana = String::new();
        for token in tokens {
            let t = token.as_str();
            if t == "X"
                || t.chars().all(|c| c.is_ascii_digit())
                || matches!(t, "W" | "U" | "B" | "R" | "G" | "C" | "S")
            {
                mana.push('{');
                mana.push_str(t);
                mana.push('}');
            } else if let Some(pair) = super::hybrid_pair(t) {
                mana.push_str(&pair);
            } else {
                return self.deny(format!("a non-mana `{keyword}` cost `{raw}`"));
            }
        }
        if !mana.contains("{X}") || baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("invalid `{keyword}` mana cost `{raw}`"));
        }
        Some(mana)
    }
}

fn split_two_top_level_colons(raw: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut split_at: Option<usize> = None;
    for (idx, ch) in raw.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                if split_at.is_some() {
                    return None;
                }
                split_at = Some(idx);
            }
            _ => {}
        }
    }
    let idx = split_at?;
    Some((&raw[..idx], &raw[idx + 1..]))
}

fn append_effect_to_spell_macro(expr: &str, extra_effect: &str) -> Option<String> {
    let prefix = "spell!(&[";
    if !expr.starts_with(prefix) || !expr.ends_with(')') {
        return None;
    }
    let bytes = expr.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut close_idx = None;
    let mut i = prefix.len();
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'[' => depth += 1,
                b']' => {
                    if depth == 0 {
                        close_idx = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    let close_idx = close_idx?;
    if expr[prefix.len()..close_idx].trim().is_empty() {
        return None;
    }
    let mut out = String::with_capacity(expr.len() + extra_effect.len() + 2);
    out.push_str(&expr[..close_idx]);
    out.push_str(", ");
    out.push_str(extra_effect);
    out.push_str(&expr[close_idx..]);
    Some(out)
}

fn is_spell_either_cost_part(part: &str) -> bool {
    part.starts_with("PayLife(")
        || part.starts_with("PayEnergy(")
        || part.starts_with("Sacrifice(")
        || part.starts_with("ExilePermanent(")
        || part.starts_with("ExileFromHand(")
        || part.starts_with("ExileFromGraveyard(")
        || part.starts_with("Discard(")
        || part.starts_with("ReturnToHand(")
        || part.starts_with("TapOther(")
        || part.starts_with("TapN {")
        || part == "Forage"
}

fn append_ability_condition(expr: &str, cond: &str) -> Option<String> {
    if !(expr.starts_with("static_ability!(")
        || expr.starts_with("activated!(")
        || expr.starts_with("mana_ability!("))
    {
        return None;
    }
    let without_close = expr.strip_suffix(')')?;
    if let Some(idx) = without_close.rfind(", condition = Some(")
        && let Some(existing) = without_close[idx + ", condition = Some(".len()..].strip_suffix(')')
    {
        Some(format!(
            "{}, condition = Some(Condition::All(&[{existing}, {cond}])))",
            &without_close[..idx]
        ))
    } else {
        Some(format!("{without_close}, condition = Some({cond}))"))
    }
}

fn parse_single_target_spell(expr: &str) -> Option<(&str, &str)> {
    let inner = expr.strip_prefix("spell!(&[")?.strip_suffix(')')?;
    let (effect_raw, tail) =
        inner.split_once("], targets = Some(TargetReq::one(TargetSpec::Object(")?;
    let filter_ref = tail.strip_suffix(")))")?;
    if filter_ref.contains(", ") || !is_single_rust_expr(effect_raw) {
        return None;
    }
    Some((effect_raw, filter_ref))
}

fn is_single_rust_expr(slice_body: &str) -> bool {
    let mut depth = 0usize;
    for b in slice_body.bytes() {
        match b {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => return false,
            _ => {}
        }
    }
    depth == 0 && !slice_body.is_empty()
}

fn map_overload_effect(effect_raw: &str, filter_ref: &str) -> Option<String> {
    if effect_raw == "Effect::TapTarget" {
        return Some(format!("Effect::TapAll {{ filter: {filter_ref} }}"));
    }
    if effect_raw == format!("Effect::destroy(TargetSpec::Object({filter_ref}))") {
        return Some(format!("Effect::destroy_all({filter_ref})"));
    }
    if effect_raw == format!("Effect::destroy_no_regen(TargetSpec::Object({filter_ref}))") {
        return Some(format!("Effect::destroy_all_no_regen({filter_ref})"));
    }
    if effect_raw == format!("Effect::bounce(TargetSpec::Object({filter_ref}))") {
        return Some(format!(
            "Effect::ReturnAllToHand {{ filter: {filter_ref}, opponents_only: false }}"
        ));
    }
    if let Some(amt) = effect_raw
        .strip_prefix("Effect::DealDamage { amount: ")
        .and_then(|s| s.strip_suffix(&format!(", target: TargetSpec::Object({filter_ref}) }}")))
    {
        return Some(format!(
            "Effect::DealDamageEach {{ amount: {amt}, filter: {filter_ref} }}"
        ));
    }
    if let Some(rest) = effect_raw.strip_prefix("Effect::PumpTarget { ")
        && rest.ends_with(" }")
    {
        return Some(format!(
            "Effect::PumpFilter {{ filter: {filter_ref}, controlled_by: None, {rest}"
        ));
    }
    None
}

fn is_paid_alternative_cost_part(part: &str) -> bool {
    part.starts_with("PayLife(")
        || part.starts_with("ExileFromHand(")
        || part.starts_with("Sacrifice(")
        || part.starts_with("ReturnToHand(")
        || part.starts_with("TapN {")
}

fn is_paid_additional_cost_part(part: &str) -> bool {
    is_graveyard_activation_part(part)
        || part.starts_with("ExileFromHand(")
        || part.starts_with("TapN {")
}

fn is_graveyard_activation_part(part: &str) -> bool {
    part.starts_with("PayLife(")
        || part.starts_with("PayEnergy(")
        || part.starts_with("Discard(")
        || part == "DiscardRandom"
        || part == "DiscardHand"
        || part.starts_with("Sacrifice(")
        || part.starts_with("ReturnToHand(")
        || part.starts_with("TapOther(")
        || part.starts_with("ExileFromGraveyard(")
        || part.starts_with("ExileTopOfLibrary(")
        || part.starts_with("ExilePermanent(")
}

fn is_morph_cost_part(part: &str) -> bool {
    part.starts_with("PayLife(")
        || part.starts_with("PayEnergy(")
        || part == "PayEnergyX"
        || part.starts_with("Discard(")
        || part == "DiscardRandom"
        || part == "DiscardHand"
        || part.starts_with("Sacrifice(")
        || part.starts_with("SacrificeX(")
        || part.starts_with("ReturnToHand(")
        || part.starts_with("ExileFromHand(")
        || part.starts_with("ExileFromGraveyard(")
        || part.starts_with("ExileTopOfLibrary(")
        || part.starts_with("ExilePermanent(")
        || part.starts_with("TapOther(")
        || part.starts_with("TapX(")
        || part.starts_with("TapN {")
}
