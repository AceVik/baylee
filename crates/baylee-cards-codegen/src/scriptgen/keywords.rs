//! Reading keywords, replacement effects and how a land enters.

use super::{Bound, Params, Tx, keyword_const, keyword_enter_modifier, keyword_static};

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
        if line == "Start your engines!" {
            self.body
                .abilities
                .push("AbilityDef::StartYourEngines".to_string());
            return Some(());
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
        if let super::saga_ward_convoke::ExtraOutcome::Handled(res) = self.try_extra_keyword(line) {
            if res.is_some() {
                return res;
            }
            if let Some(rest) = line.strip_prefix("etbCounter:") {
                return self.etb_counter_wave6(rest);
            }
            return None;
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
        if line == "Jump-start" {
            return self.jump_start();
        }
        if line == "Aftermath" {
            return self.aftermath();
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
        let all_supported = parts.iter().all(|part| {
            part.starts_with("PayLife(")
                || part.starts_with("ExileFromHand(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("TapN {")
        });
        if !all_supported || (mana.is_empty() && parts.is_empty()) {
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
                let mut chain = super::Chain::default();
                self.chain(&body, &mut chain)?;
                if chain.effects.is_empty() || chain.target_is_x {
                    return self.deny(format!("unsupported chapter `{name}`"));
                }
                let targets = Self::target_req_expr(&chain)
                    .map_or_default(|req| format!(", targets = Some({req})"));
                let second_targets = Self::second_target_req_expr(&chain)
                    .map_or_default(|req| format!(", second_targets = Some({req})"));
                let effects = chain.effects.join(", ");
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

    /// `K:etbCounter:<KIND>:<n>:CheckSVar$ <var>[:<desc>]` for a single-kicker
    /// permanent whose `<var>` is `Count$Kicked.1.0` and whose `<n>` is a
    /// positive whole number: "If this creature was kicked, it enters with N
    /// <kind> counters on it."
    ///
    /// On a single-kicker spell `times_kicked` is strictly `0` or `1` (and `0`
    /// when not cast off the stack), so `Amount::TimesKicked` (or `Times` by
    /// `n`) places the exact counters as the permanent enters. Multi-kicker,
    /// dual-kicker, non-kicker cards, variable `<n>`, and any other `SVar`
    /// definition are refused.
    pub(super) fn kicked_etb_counter(&self, line: &str) -> Option<String> {
        if !self.has_single_kicker {
            return None;
        }
        let mut fields = line.strip_prefix("etbCounter:")?.split(':');
        let kind = super::counter_kind(fields.next()?.trim())?;
        let n: u32 = fields.next()?.trim().parse().ok()?;
        if n == 0 {
            return None;
        }
        let cond = fields.next()?.trim();
        let var = cond.strip_prefix("CheckSVar$")?.trim();
        let def = self.svars.get(var).map(|s| s.trim());
        if !matches!(def, Some("Count$Kicked.1.0" | "Count$TimesKicked")) {
            return None;
        }
        let amount = if n == 1 {
            "Amount::TimesKicked".to_string()
        } else {
            format!("Amount::Times {{ base: &Amount::TimesKicked, factor: {n} }}")
        };
        Some(format!(
            "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
        ))
    }

    /// Additional Wave-6 `K:etbCounter:` shapes not handled by `saga_ward_convoke`:
    /// empty 3rd condition field (`::desc`), `Count$TotalManaSpent` /
    /// `Count$TimesKicked`, and static board/turn/life/domain/distinct-color
    /// `Amount`s via `static_count_expr`.
    fn etb_counter_wave6(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(kind_raw), Some(amount_raw)) = (fields.next(), fields.next()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        let cond_raw = fields.next();
        let _desc = fields.next();
        if fields.next().is_some() {
            return self.deny("keyword `etbCounter`".to_string());
        }
        if let Some(cond) = cond_raw {
            let trimmed = cond.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("no condition") {
                return self.deny("keyword `etbCounter`".to_string());
            }
        }
        let Some(kind) = super::counter_kind(kind_raw.trim()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        let Some(amount) = self.etb_wave6_amount(amount_raw.trim()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        self.body.enter_modifiers.push(format!(
            "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
        ));
        Some(())
    }

    fn etb_wave6_amount(&mut self, raw: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return (n > 0).then(|| format!("Amount::Fixed({n})"));
        }
        let def = self.svars.get(raw)?.trim().to_string();
        if matches!(
            def.as_str(),
            "Count$CastTotalManaSpent" | "Count$TotalManaSpent"
        ) {
            return Some("Amount::ManaSpentToCast".to_string());
        }
        if def == "Count$TimesKicked" {
            return Some("Amount::TimesKicked".to_string());
        }
        if def == "Count$xPaid" && self.has_x_mana_cost {
            return Some("Amount::X".to_string());
        }
        let normalized = if let Some(spec) = def.strip_prefix("Count$LastStateBattlefield ") {
            format!("Count$Valid {spec}")
        } else {
            def
        };
        let expr = self.static_count_expr(&normalized)?;
        if expr.contains("SourcePower") || expr.contains("CountersOnSource") {
            return None;
        }
        Some(expr)
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
            || !parts.iter().all(|part| {
                part.starts_with("PayLife(")
                    || part.starts_with("ExileFromHand(")
                    || part.starts_with("Sacrifice(")
                    || part.starts_with("ReturnToHand(")
                    || part.starts_with("TapN {")
            })
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
            'S' => match self.try_extra_static(spec) {
                super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                super::saga_ward_convoke::ExtraOutcome::Pass => self.static_ability(spec),
            },
            'R' => match self.try_extra_replacement(spec) {
                super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                super::saga_ward_convoke::ExtraOutcome::Pass => self.replacement(spec),
            },
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
            // plain "enters tapped" (optionally chaining ETB counters onto
            // itself, as on the Vivid lands or Sphere of the Suns).
            (None, None) => match body.take("ConditionCheckSVar") {
                None => {
                    if let Some(sub) = body.take("SubAbility") {
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
                    }
                    "EnterModifier::Tapped".to_string()
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
