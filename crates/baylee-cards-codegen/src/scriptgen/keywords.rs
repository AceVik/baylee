//! Reading keywords, replacement effects and how a land enters.

use super::{Bound, Params, Tx, keyword_const, keyword_enter_modifier, keyword_static};

impl Tx<'_> {
    pub(super) fn keyword(&mut self, line: &str) -> Option<()> {
        if line == "Mayhem" {
            // `transcode` requires a land type line for this permission.
            self.body.keywords.push("KeywordSet::MAYHEM".to_string());
            return Some(());
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
        if let Some(rest) = line.strip_prefix("ETBReplacement:") {
            return self.etb_replacement(rest);
        }
        if let Some(rest) = line.strip_prefix("Equip:") {
            return self.equip(rest);
        }
        if let Some(rest) = line.strip_prefix("Cycling:") {
            return self.cycling(rest);
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
        if let Some(rest) = line.strip_prefix("Flashback:") {
            return self.flashback(rest);
        }
        if let Some(rest) = line.strip_prefix("Foretell:") {
            return self.foretell(rest);
        }
        if let Some(rest) = line.strip_prefix("Entwine:") {
            if rest.contains(':') {
                return self.deny(format!("an `Entwine` with extra fields `{rest}`"));
            }
            let cost = self.cost_expr(rest)?;
            self.body.entwine = Some(format!("Some({cost})"));
            self.body.notes.push("entwine".to_string());
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("Affinity:") {
            return self.affinity(rest);
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
        let rest = word.trim().strip_prefix("Landwalk:")?;
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
                    || matches!(t.as_str(), "W" | "U" | "B" | "R" | "G" | "C")
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
                    || matches!(t.as_str(), "W" | "U" | "B" | "R" | "G" | "C")
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

    /// `K:Equip:<cost>` as the activated ability the keyword is.
    ///
    /// Equip prints one thing and the rules supply the rest (CR 702.6a):
    /// sorcery speed, "target creature you control", and attaching this
    /// permanent to it. `equip!` is that sentence, so the cost is all there
    /// is to read — and reading it through [`Self::cost_expr`] rather than
    /// as mana means the eight scripts that equip for a sacrifice, a
    /// discard or three life are the same rule as the 543 that equip for
    /// mana.
    ///
    /// A fourth field refuses. `K:Equip:1:Creature.Legendary+YouCtrl` is
    /// "equip legendary creature", which narrows the target the keyword
    /// otherwise defines — and `equip!` has no room for it precisely
    /// because the rules fill that room. Writing the card without the
    /// restriction would let it move onto anything.
    pub(super) fn equip(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny("an `Equip` that narrows what it may attach to".to_string());
        }
        let cost = self.cost_expr(rest)?;
        self.body.abilities.push(format!("equip!({cost})"));
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
    /// sentence with its own arm to come (101 lines, four fields).
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

    /// `K:Flashback:<cost>` for mana-only flashback costs (CR 702.34a).
    pub(super) fn flashback(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("a `Flashback` with extra fields `{rest}`"));
        }
        let (mana, parts) = self.cost_pieces(rest)?;
        if mana.is_empty() || !parts.is_empty() {
            return self.deny(format!("a non-mana `Flashback` cost `{rest}`"));
        }
        self.body.flashback = Some(format!("Some(mana!(\"{mana}\"))"));
        self.body.notes.push("flashback".to_string());
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
        if matches!(valid, "Player" | "Opponent") {
            return self.deny(format!("an `Enchant {valid}`, which attaches to no object"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("ENCHANT", &expr);
        self.body.abilities.push(format!(
            "spell!(&[Effect::AttachSelf {{ target: TargetSpec::Object(&{name}) }}], \
             targets = Some(TargetReq::one(TargetSpec::Object(&{name}))))"
        ));
        Some(())
    }

    pub(super) fn rule(&mut self, kind: char, spec: &str) -> Option<()> {
        self.has_x = kind == 'A';
        self.on_a_spell = kind == 'A' && spec.trim_start().starts_with("SP$");
        self.trigger_mode = None;
        self.block_line = None;
        match kind {
            'A' => self.activated_or_spell(spec),
            'T' => self.triggered(spec),
            'S' if self.is_ai_only_static(spec) => Some(()),
            'S' => self.static_ability(spec),
            'R' => self.replacement(spec),
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
            let expected_player = if loss { "You" } else { "Opponent" };
            if p.take("ActiveZones").as_deref() != Some("Battlefield")
                || p.take("ValidPlayer").as_deref() != Some(expected_player)
                || p.take("Layer").as_deref() != Some("CantHappen")
                || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
                || !p.exhausted()
            {
                return self.deny(format!("replacement `R: Event$ {event}`"));
            }
            let modifier = if loss {
                "Modifier::NamedPlayersCantLose(PlayerRel::You)"
            } else {
                "Modifier::PlayersCantWin(PlayerRel::EachOpponent)"
            };
            self.body
                .abilities
                .push(Self::static_expr("Filter::Any", modifier));
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
        if event == "Counter" {
            p.drop_prose();
            if p.take("ValidCard").as_deref() == Some("Card.Self")
                && p.take("ValidSA").as_deref() == Some("Spell")
                && p.take("Layer").as_deref() == Some("CantHappen")
                && p.exhausted()
            {
                self.body
                    .keywords
                    .push("KeywordSet::UNCOUNTERABLE".to_string());
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
            self.note("replacement `Moved` this rule cannot read".to_string());
            return None;
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny(format!("`ReplaceWith$ {with}` names no readable SVar"));
        };
        body.drop_prose();
        // `DB$ Tap | Defined$ Self | ETB$ True`: the tap has to be of this
        // card, as it enters, or it is not this modifier.
        if api != "Tap"
            || body.take("Defined").as_deref() != Some("Self")
            || body.take("ETB").as_deref() != Some("True")
        {
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
            // plain "enters tapped".
            (None, None) => match body.take("ConditionCheckSVar") {
                None => "EnterModifier::Tapped".to_string(),
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

    /// "You may pay N life. If you don’t, this enters tapped" — the
    /// shocklands, 26 of the corpus’s `Moved` replacements, and
    /// [`EnterModifier::TappedOrPayLife`] says it exactly. Steam Vents is
    /// hand-written in this pool as `TappedOrPayLife(2)` against the
    /// script’s `PayLife<2>`, which is the reading checked against a
    /// printed card rather than argued from the key’s name.
    ///
    /// `PayLife<N>` and nothing else: the other five `UnlessCost$` values on
    /// these lines reveal a card instead (Rustic Clachan’s Kithkin),
    /// which this modifier has nowhere to carry. And the payer has to be the
    /// land’s own controller, because that is the only player
    /// `TappedOrPayLife` can ask.
    pub(super) fn enters_tapped_or_pays(
        &mut self,
        body: &mut Params,
        cost: &str,
    ) -> Option<String> {
        let life = cost
            .strip_prefix("PayLife<")
            .and_then(|rest| rest.strip_suffix('>'))
            .and_then(|n| n.parse::<u16>().ok());
        let Some(life) = life else {
            self.note(format!("replacement `Moved` charging `{cost}`"));
            return None;
        };
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
        Some(format!("EnterModifier::TappedOrPayLife({life})"))
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
}
