//! High-leverage keyword, entry-counter, static, and replacement rules backed
//! by existing `baylee-cards-dsl` and `baylee-engine` primitives.
//!
//! Keeps the honest-stub invariant: every accepted form maps one-to-one to a
//! verified engine rule, and every unsupported cost, zone, condition, or card
//! type stays refused with an explicit reason.

use super::{Params, Tx, card_type_const, cost_parts, counter_kind, hybrid_pair};

/// Dispatch outcome for an extension hook in this module.
pub(super) enum ExtraOutcome {
    /// This module claimed the line and returned its transcode result.
    Handled(Option<()>),
    /// The line belongs to the base dispatch in `keywords.rs`.
    Pass,
}

impl Tx<'_> {
    pub(super) fn is_permanent_card_pub(&self) -> bool {
        self.is_permanent_card()
    }

    fn is_permanent_card(&self) -> bool {
        let mut has_permanent = false;
        for word in self.type_line.split_whitespace() {
            if matches!(word, "Instant" | "Sorcery") {
                return false;
            }
            if matches!(
                word,
                "Artifact" | "Creature" | "Enchantment" | "Land" | "Planeswalker" | "Battle"
            ) {
                has_permanent = true;
            }
        }
        has_permanent
    }

    fn is_creature_card(&self) -> bool {
        self.type_line
            .split_whitespace()
            .any(|word| word == "Creature")
    }

    fn is_instant_card(&self) -> bool {
        self.type_line
            .split_whitespace()
            .any(|word| word == "Instant")
    }

    fn is_instant_or_sorcery(&self) -> bool {
        self.type_line
            .split_whitespace()
            .any(|word| matches!(word, "Instant" | "Sorcery"))
    }

    /// Additional `K:` keyword handlers run after `keyword_enter_modifier` and
    /// `kicked_etb_counter` and before `keyword_const`.
    pub(super) fn try_extra_keyword(&mut self, line: &str) -> ExtraOutcome {
        if let ExtraOutcome::Handled(res) = self.try_extra_prefixed_keyword(line) {
            return ExtraOutcome::Handled(res);
        }
        match line.trim() {
            "Station" => return ExtraOutcome::Handled(self.station_keyword()),
            // Deckbuilding-only commander/partner rules (CR 702.124g–i, CR 903.3)
            // are already emitted onto `CardDef::partner` / `CardDef::commander`
            // by `stubgen` from Scryfall oracle text.
            "Choose a Background" => {
                self.body.notes.push("choose a background".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            "Doctor's companion" => {
                self.body.notes.push("doctor's companion".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            "Partner:Friends forever" => {
                self.body.notes.push("friends forever".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            "CARDNAME can be your commander." => {
                self.body.notes.push("can be your commander".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            "CARDNAME can't attack or block." => {
                self.body
                    .keywords
                    .push("KeywordSet::CANT_ATTACK".to_string());
                self.body
                    .keywords
                    .push("KeywordSet::CANT_BLOCK".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            "CARDNAME can only attack alone." => {
                self.body.abilities.push(Self::static_expr(
                    "Filter::This",
                    "Modifier::AttacksAloneOnly",
                ));
                return ExtraOutcome::Handled(Some(()));
            }
            // CR 702.195: Storied is checked continuously for permanents on the
            // battlefield (`GameState::award_enduring_stories`).
            "Storied" => {
                if !self.is_permanent_card() {
                    return ExtraOutcome::Handled(
                        self.deny("keyword `Storied` on a non-permanent".to_string()),
                    );
                }
                self.body.keywords.push("KeywordSet::STORIED".to_string());
                return ExtraOutcome::Handled(Some(()));
            }
            // CR 702.131b: `GameState::award_citys_blessings` checks battlefield
            // permanents only; non-permanent spells with Ascend must stay stubs.
            "Ascend" if self.is_instant_or_sorcery() => {
                return ExtraOutcome::Handled(
                    self.deny("keyword `Ascend` on a non-permanent spell".to_string()),
                );
            }
            // CR 702.39a–b: Provoke; multiple printed instances trigger separately.
            "Provoke" => {
                if self
                    .body
                    .keywords
                    .iter()
                    .any(|k| k == "KeywordSet::PROVOKE")
                {
                    self.body.abilities.push(Self::static_expr(
                        "Filter::This",
                        "Modifier::AddKeyword(KeywordSet::PROVOKE)",
                    ));
                } else {
                    self.body.keywords.push("KeywordSet::PROVOKE".to_string());
                }
                return ExtraOutcome::Handled(Some(()));
            }
            // CR 702.25b: multiple printed instances of flanking trigger separately.
            "Flanking"
                if self
                    .body
                    .keywords
                    .iter()
                    .any(|k| k == "KeywordSet::FLANKING") =>
            {
                self.body.abilities.push(Self::static_expr(
                    "Filter::This",
                    "Modifier::AddKeyword(KeywordSet::FLANKING)",
                ));
                return ExtraOutcome::Handled(Some(()));
            }
            _ => {}
        }
        ExtraOutcome::Pass
    }

    fn try_extra_prefixed_keyword(&mut self, line: &str) -> ExtraOutcome {
        if let Some(rest) = line.strip_prefix("Suspend:") {
            return ExtraOutcome::Handled(self.suspend(rest));
        }
        if let Some(rest) = line.strip_prefix("Partner with:") {
            return ExtraOutcome::Handled(self.partner_with(rest));
        }
        if let Some(rest) = line.strip_prefix("etbCounter:") {
            return ExtraOutcome::Handled(self.etb_counter_extra(rest));
        }
        if let Some(rest) = line.strip_prefix("Level up:") {
            return ExtraOutcome::Handled(self.level_up(rest));
        }
        if let Some(rest) = line.strip_prefix("Bands with other:") {
            return ExtraOutcome::Handled(self.bands_with_other(rest));
        }
        if let Some(rest) = line.strip_prefix("Annihilator:") {
            return ExtraOutcome::Handled(self.annihilator(rest));
        }
        if let Some(svar) = line.strip_prefix("ETBReplacement:Other:")
            && !svar.contains(':')
            && self
                .svars
                .get(svar.trim())
                .is_some_and(|s| s.trim_start().starts_with("DB$ NameCard"))
        {
            return ExtraOutcome::Handled(self.etb_name_card(svar.trim()));
        }
        ExtraOutcome::Pass
    }

    /// `K:Annihilator:<n>` (CR 702.86a): "Whenever this creature attacks,
    /// defending player sacrifices N permanents." Each printed instance emits
    /// its own trigger (CR 702.86b).
    fn annihilator(&mut self, rest: &str) -> Option<()> {
        if !self.is_permanent_card() {
            return self.deny("keyword `Annihilator` on a non-permanent".to_string());
        }
        let Ok(n) = rest.trim().parse::<u16>() else {
            return self.deny(format!("keyword `Annihilator:{rest}`"));
        };
        if n == 0 {
            return self.deny("keyword `Annihilator:0`".to_string());
        }
        self.body.abilities.push(format!(
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::SacrificeAmountForPlayers {{ \
             who: PlayerRel::DefendingPlayer, filter: &Filter::Any, amount: Amount::Fixed({n}) }}])"
        ));
        Some(())
    }

    /// `K:Suspend:<n>:<cost>` (CR 702.62a) for fixed counter counts and fixed
    /// mana costs on sorcery-speed non-creature cards.
    ///
    /// Creature spells stay refused because `CastFromExileWithoutPaying` does
    /// not yet grant CR 702.62a's post-suspend haste; instants and flash cards
    /// stay refused because `offers.rs` currently offers suspend only at sorcery
    /// timing.
    fn suspend(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(count_raw), Some(cost_raw), None) = (fields.next(), fields.next(), fields.next())
        else {
            return self.deny(format!("suspend `{rest}`"));
        };
        let count_str = count_raw.trim();
        if count_str == "X" {
            return self.deny("a variable-X `Suspend` count".to_string());
        }
        let Ok(counters) = count_str.parse::<u8>() else {
            return self.deny(format!("suspend count `{count_str}`"));
        };
        if counters == 0 {
            return self.deny("suspend count `0`".to_string());
        }
        if self.type_line.is_empty() {
            return self.deny("keyword `Suspend` without a type line".to_string());
        }
        if self.is_creature_card() {
            return self.deny("keyword `Suspend` on a creature".to_string());
        }
        if self.is_instant_card() || self.has_flash {
            return self.deny("keyword `Suspend` on an instant or flash card".to_string());
        }
        let mana = self.parse_fixed_mana_cost(cost_raw, "Suspend")?;
        self.body.abilities.push(format!(
            "AbilityDef::Suspend {{ counters: {counters}, cost: mana!(\"{mana}\") }}"
        ));
        self.body.notes.push("suspend".to_string());
        Some(())
    }

    fn parse_fixed_mana_cost(&self, raw: &str, kind: &str) -> Option<String> {
        let tokens = cost_parts(raw);
        if tokens.is_empty() {
            return self.deny(format!("an empty `{kind}` cost"));
        }
        let mut mana = String::new();
        for token in &tokens {
            let token = token.as_str();
            if token == "X" || token.starts_with("XMin") {
                return self.deny(format!("a `{kind}` cost `{raw}` with `X`"));
            }
            if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token, "W" | "U" | "B" | "R" | "G" | "C")
            {
                mana.push('{');
                mana.push_str(token);
                mana.push('}');
            } else if let Some(pair) = hybrid_pair(token) {
                mana.push_str(&pair);
            } else {
                return self.deny(format!("a non-mana `{kind}` cost `{raw}`"));
            }
        }
        if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("a `{kind}` cost `{raw}` that is not valid mana"));
        }
        Some(mana)
    }

    /// Non-numeric `K:Ward:<cost>` forms (CR 702.21a).
    ///
    /// Plain integer costs return [`ExtraOutcome::Pass`] so `keywords.rs::ward`
    /// handles generic `AbilityDef::Ward { mana }` (`1..=10`).
    pub(super) fn try_extra_ward(&mut self, rest: &str) -> ExtraOutcome {
        let trimmed = rest.trim();
        if trimmed.parse::<u16>().is_ok() {
            return ExtraOutcome::Pass;
        }
        ExtraOutcome::Handled(self.ward_extra(trimmed))
    }

    fn ward_extra(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny(format!("ward `{rest}`"));
        }
        if let Some(body) = rest
            .strip_prefix("PayLife<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return self.ward_pay_life(body, rest);
        }
        if let Some(body) = rest
            .strip_prefix("AddCounterYou<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return self.ward_poison(body, rest);
        }
        if let Some(body) = rest
            .strip_prefix("Discard<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return self.ward_discard(body, rest);
        }
        if let Some(body) = rest.strip_prefix("Sac<").and_then(|s| s.strip_suffix('>')) {
            return self.ward_sacrifice(body, rest);
        }
        if !rest.contains('<') {
            let mana = self.parse_fixed_mana_cost(rest, "Ward")?;
            self.body.abilities.push(format!(
                "triggered!(Trigger::Ward, &[Effect::PlayerMayPayManaOr {{ \
                 player: PlayerRel::ControllerOfTarget, cost: mana!(\"{mana}\"), \
                 effect: &Effect::CounterTargetSpellOrAbility }}])"
            ));
            return Some(());
        }
        self.deny(format!("ward `{rest}`"))
    }

    fn ward_pay_life(&mut self, body: &str, full: &str) -> Option<()> {
        let mut parts = body.split('/');
        let raw = parts.next()?.trim();
        let _desc = parts.next();
        if parts.next().is_some() {
            return self.deny(format!("ward `{full}`"));
        }
        let life = if let Ok(n) = raw.parse::<u32>()
            && n > 0
        {
            format!("Amount::Fixed({n})")
        } else if self.svars.get(raw).map(|s| s.trim()) == Some("Count$CardPower") {
            "Amount::SourcePower".to_string()
        } else {
            return self.deny(format!("ward `{full}`"));
        };
        self.body.abilities.push(format!(
            "triggered!(Trigger::Ward, &[Effect::PlayerMayPayLifeOr {{ \
             player: PlayerRel::ControllerOfTarget, life: {life}, \
             effect: &Effect::CounterTargetSpellOrAbility }}])"
        ));
        Some(())
    }

    fn ward_poison(&mut self, body: &str, full: &str) -> Option<()> {
        let mut parts = body.split('/');
        let (Some(count_str), Some(kind_str)) = (parts.next(), parts.next()) else {
            return self.deny(format!("ward `{full}`"));
        };
        let _desc = parts.next();
        if parts.next().is_some() || kind_str.trim() != "POISON" {
            return self.deny(format!("ward `{full}`"));
        }
        let Ok(n) = count_str.trim().parse::<u32>() else {
            return self.deny(format!("ward `{full}`"));
        };
        if n == 0 {
            return self.deny(format!("ward `{full}`"));
        }
        self.body.abilities.push(format!(
            "triggered!(Trigger::Ward, &[Effect::PlayerMayGetPoisonOr {{ \
             player: PlayerRel::ControllerOfTarget, poison: Amount::Fixed({n}), \
             effect: &Effect::CounterTargetSpellOrAbility }}])"
        ));
        Some(())
    }

    fn ward_discard(&mut self, body: &str, full: &str) -> Option<()> {
        let mut parts = body.split('/');
        let (Some(count_str), Some(valid_str)) = (parts.next(), parts.next()) else {
            return self.deny(format!("ward `{full}`"));
        };
        let _desc = parts.next();
        if parts.next().is_some() || count_str.trim() != "1" {
            return self.deny(format!("ward `{full}`"));
        }
        let valid = valid_str.trim();
        if matches!(valid, "Random" | "Hand" | "LastDrawn" | "CARDNAME")
            || valid.contains("Ctrl")
            || valid.contains("Own")
            || valid.contains("Self")
            || valid.contains("Other")
            || valid.contains(',')
        {
            return self.deny(format!("ward `{full}`"));
        }
        let normalized = valid.replace(';', ",");
        let expr = self.filter_expr_in_zone(&normalized, true)?;
        let filter = self.body.filter_static("WARD", &expr);
        self.body.abilities.push(format!(
            "triggered!(Trigger::Ward, &[Effect::PlayerMayPayCostOr {{ \
             player: PlayerRel::ControllerOfTarget, cost: &CostPart::Discard(&{filter}), \
             effect: &Effect::CounterTargetSpellOrAbility }}])"
        ));
        Some(())
    }

    fn ward_sacrifice(&mut self, body: &str, full: &str) -> Option<()> {
        let mut parts = body.split('/');
        let (Some(count_str), Some(valid_str)) = (parts.next(), parts.next()) else {
            return self.deny(format!("ward `{full}`"));
        };
        let _desc = parts.next();
        if parts.next().is_some() {
            return self.deny(format!("ward `{full}`"));
        }
        let Ok(count) = count_str.trim().parse::<u8>() else {
            return self.deny(format!("ward `{full}`"));
        };
        if count == 0 {
            return self.deny(format!("ward `{full}`"));
        }
        let valid = valid_str.trim();
        if valid == "CARDNAME"
            || valid.contains("Ctrl")
            || valid.contains("Own")
            || valid.contains("Self")
            || valid.contains("Other")
            || valid.contains(',')
        {
            return self.deny(format!("ward `{full}`"));
        }
        let normalized = valid.replace(';', ",");
        let expr = self.filter_expr(&normalized)?;
        let filter = self.body.filter_static("WARD", &expr);
        let effect = if count == 1 {
            format!(
                "Effect::PlayerMayPayCostOr {{ player: PlayerRel::ControllerOfTarget, \
                 cost: &CostPart::Sacrifice(&{filter}), \
                 effect: &Effect::CounterTargetSpellOrAbility }}"
            )
        } else {
            format!(
                "Effect::PlayerMaySacrificeNOr {{ player: PlayerRel::ControllerOfTarget, \
                 filter: &{filter}, count: {count}, \
                 effect: &Effect::CounterTargetSpellOrAbility }}"
            )
        };
        self.body
            .abilities
            .push(format!("triggered!(Trigger::Ward, &[{effect}])"));
        Some(())
    }

    /// `K:Partner with:<name>[:<short>]` (CR 702.124f).
    ///
    /// Deckbuilding partner identity is already emitted onto `CardDef::partner`
    /// by `stubgen` from Scryfall oracle text; this emits the enters-the-
    /// battlefield triggered ability that lets target player search their
    /// library for the named partner card and put it into their hand.
    fn partner_with(&mut self, rest: &str) -> Option<()> {
        if !self.is_permanent_card() {
            return self.deny("keyword `Partner with` on a non-permanent".to_string());
        }
        let mut fields = rest.split(':');
        let name = fields.next()?.trim();
        let _short = fields.next();
        if name.is_empty() || name.contains("//") || fields.next().is_some() {
            return self.deny(format!("partner with `{rest}`"));
        }
        let filter = self
            .body
            .filter_static("PARTNER", &format!("Filter::Named({name:?})"));
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::SearchLibraryOf {{ \
             library: PlayerRel::Chosen, owner_searches: true, filter: &{filter}, \
             mana_value: None, finds: &[Find::HAND], optional: true }}], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ));
        self.body.notes.push("partner with".to_string());
        Some(())
    }

    /// Extended `K:etbCounter:` forms whose amounts map cleanly onto
    /// `EnterModifier::WithCounters` (`eval::amount` on entry) or dual-kicker
    /// `EnterModifier::IfKicked`.
    fn etb_counter_extra(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(kind_raw), Some(amount_raw)) = (fields.next(), fields.next()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        let cond_raw = fields.next();
        let _desc = fields.next();
        if fields.next().is_some() {
            return self.deny("keyword `etbCounter`".to_string());
        }
        if let Some(cond) = cond_raw
            && !cond.trim().eq_ignore_ascii_case("no condition")
        {
            if let Some(entry) =
                self.dual_kicker_etb_counter(kind_raw.trim(), amount_raw.trim(), cond.trim())
            {
                self.body.enter_modifiers.push(entry);
                return Some(());
            }
            return self.deny("keyword `etbCounter`".to_string());
        }
        let kind_trimmed = kind_raw.trim();
        if let Some(list) = kind_trimmed.strip_prefix("EACH ") {
            let Ok(n) = amount_raw.trim().parse::<u32>() else {
                return self.deny("keyword `etbCounter`".to_string());
            };
            if n == 0 {
                return self.deny("keyword `etbCounter`".to_string());
            }
            let kinds: Option<Vec<String>> = list
                .split(',')
                .map(|code| counter_kind(code.trim()))
                .collect();
            let Some(kinds) = kinds.filter(|ks| !ks.is_empty()) else {
                return self.deny("keyword `etbCounter`".to_string());
            };
            for kind in kinds {
                self.body.enter_modifiers.push(format!(
                    "EnterModifier::WithCounters {{ kind: {kind}, amount: Amount::Fixed({n}) }}"
                ));
            }
            return Some(());
        }
        let Some(kind) = counter_kind(kind_trimmed) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        let Some(amount) = self.etb_counter_amount(amount_raw.trim()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        self.body.enter_modifiers.push(format!(
            "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
        ));
        Some(())
    }

    fn dual_kicker_etb_counter(&self, kind: &str, amount: &str, cond: &str) -> Option<String> {
        let remap = self.dual_kicker_remap?;
        if kind != "P1P1" {
            return None;
        }
        let n: u8 = amount.parse().ok()?;
        if n == 0 {
            return None;
        }
        let var = cond.strip_prefix("CheckSVar$")?.trim();
        let slot = match self.svars.get(var).map(|s| s.trim()) {
            Some("Count$Kicked1.1.0") => 0,
            Some("Count$Kicked2.1.0") => 1,
            _ => return None,
        };
        let index = remap[slot];
        Some(format!(
            "EnterModifier::IfKicked {{ index: Some({index}), counters: {n}, modifiers: &[] }}"
        ))
    }

    /// Walks a chain of `DB$ PutCounter | Defined$ Self | ETB$ True` sub-abilities
    /// attached to an unconditional self-enter-tapped replacement (e.g. Vivid lands,
    /// Sphere of the Suns).
    pub(super) fn self_etb_counter_chain(&mut self, start: &str) -> Option<Vec<String>> {
        let mut out = Vec::new();
        let mut current = Some(start.trim().to_string());
        for _ in 0..8 {
            let Some(name) = current else {
                return (!out.is_empty()).then_some(out);
            };
            let Some((api, mut p)) = self.svars.get(&name).and_then(|s| Params::parse(s)) else {
                return self.deny(format!("`SubAbility$ {name}` names no readable SVar"));
            };
            p.drop_prose();
            if api != "PutCounter"
                || p.take("Defined").as_deref() != Some("Self")
                || p.take("ETB").as_deref() != Some("True")
            {
                return self.deny(format!("replacement `Moved` chaining `{api}`"));
            }
            let kind_raw = p.take("CounterType")?;
            let Some(kind) = counter_kind(kind_raw.trim()) else {
                return self.deny(format!("unknown counter `{kind_raw}`"));
            };
            let num_raw = p.take("CounterNum").unwrap_or_else(|| "1".to_string());
            let amount = self.etb_put_counter_amount(num_raw.trim())?;
            current = p.take("SubAbility").map(|s| s.trim().to_string());
            if !p.exhausted() {
                return self.deny(format!(
                    "replacement `Moved` counter with `{}`",
                    p.first_key().unwrap_or_default()
                ));
            }
            out.push(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
            ));
        }
        self.deny("cyclic `Moved` counter sub-ability chain".to_string())
    }

    fn etb_put_counter_amount(&mut self, raw: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>()
            && n > 0
        {
            return Some(format!("Amount::Fixed({n})"));
        }
        if self.has_x_mana_cost && self.svars.get(raw).map(|s| s.trim()) == Some("Count$xPaid") {
            return Some("Amount::X".to_string());
        }
        self.etb_counter_amount(raw)
    }

    /// Reads `Reveal<1/<valid>[/<desc>]>` inside `UnlessCost$` on a self-ETB tap
    /// replacement (`EnterModifier::TappedUnlessReveal`). Restores `unclaimed`
    /// on failure so callers keep their existing refusal message on unknown subtypes.
    pub(super) fn enters_tapped_unless_reveal(&mut self, body: &str) -> Option<String> {
        let mut parts = body.split('/');
        let (Some(count_str), Some(valid_str)) = (parts.next(), parts.next()) else {
            return None;
        };
        let _desc = parts.next();
        if parts.next().is_some() || count_str.trim() != "1" {
            return None;
        }
        let valid = valid_str.trim();
        if valid.is_empty()
            || valid.contains("Ctrl")
            || valid.contains("Own")
            || valid.contains("Self")
            || valid.contains("Other")
            || valid.contains(',')
        {
            return None;
        }
        let normalized = valid.replace(';', ",");
        let prev_unclaimed = self.unclaimed.borrow().clone();
        let Some(expr) = self.filter_expr_in_zone(&normalized, true) else {
            *self.unclaimed.borrow_mut() = prev_unclaimed;
            return None;
        };
        let name = self.body.filter_static("REVEAL", &expr);
        Some(format!("EnterModifier::TappedUnlessReveal(&{name})"))
    }

    fn etb_counter_amount(&mut self, raw: &str) -> Option<String> {
        let def = self.svars.get(raw)?.trim().to_string();
        if let Some(rest) = def.strip_prefix("SVar$") {
            let (var, suffix) = rest.split_once('/')?;
            let inner = self.etb_counter_base_amount(var.trim())?;
            return apply_etb_amount_suffix(inner, suffix);
        }
        let (head, suffix) = match def.split_once('/') {
            None => (def.as_str(), None),
            Some((h, s)) => (h, Some(s)),
        };
        if let Some(base) = self.etb_counter_scalar_head(head) {
            return match suffix {
                None => Some(base),
                Some(s) => apply_etb_amount_suffix(base, s),
            };
        }
        if let Some(spec) = head.strip_prefix("Count$ValidGraveyard ") {
            if suffix.is_some() {
                return None;
            }
            return self.etb_graveyard_count(spec);
        }
        let spec = head
            .strip_prefix("Count$Valid ")
            .or_else(|| head.strip_prefix("Count$LastStateBattlefield "))?;
        if spec.contains('$') {
            return None;
        }
        let expr = self.filter_expr(spec.trim())?;
        let name = self.body.filter_static("COUNT", &expr);
        let base = format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}");
        match suffix {
            None => Some(base),
            Some(s) => apply_etb_amount_suffix(base, s),
        }
    }

    fn etb_counter_base_amount(&self, var: &str) -> Option<String> {
        let def = self.svars.get(var)?.trim();
        if def == "Count$xPaid" && self.has_x_mana_cost {
            return Some("Amount::X".to_string());
        }
        self.etb_counter_scalar_head(def)
    }

    fn etb_counter_scalar_head(&self, head: &str) -> Option<String> {
        match head {
            "Count$CastTotalManaSpent" => Some("Amount::ManaSpentToCast".to_string()),
            "Count$YourLifeTotal" => Some("Amount::LifeTotal(PlayerRel::You)".to_string()),
            "Count$ValidHand Card.YouOwn" | "Count$ValidHand Card.YouCtrl" => {
                Some("Amount::HandSize(PlayerRel::You)".to_string())
            }
            "Count$TimesKicked" => Some("Amount::TimesKicked".to_string()),
            "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" => {
                Some("Amount::CreaturesDiedThisTurn".to_string())
            }
            "Count$xPaid" if self.has_x_mana_cost => Some("Amount::X".to_string()),
            _ => None,
        }
    }

    fn etb_graveyard_count(&mut self, spec: &str) -> Option<String> {
        if spec.contains('$') || spec.contains('/') {
            return None;
        }
        let mut saw_you = false;
        let mut saw_opp = false;
        let mut saw_unscoped = false;
        let mut cleaned = Vec::new();
        for alt in spec.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut alt_you = false;
            let mut alt_opp = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "YouCtrl" | "YouOwn" => alt_you = true,
                    "OppCtrl" | "OppOwn" => alt_opp = true,
                    other => kept.push(other),
                }
            }
            if alt_you && alt_opp {
                return None;
            }
            if alt_you {
                saw_you = true;
            } else if alt_opp {
                saw_opp = true;
            } else {
                saw_unscoped = true;
            }
            if kept.is_empty() {
                cleaned.push(base.to_string());
            } else {
                cleaned.push(format!("{base}.{}", kept.join("+")));
            }
        }
        let zone = match (saw_you, saw_opp, saw_unscoped) {
            (false, false, true) => "ZoneSel::GraveyardAll",
            (true, false, false) => "ZoneSel::GraveyardYou",
            (false, true, false) => "ZoneSel::GraveyardOpponents",
            _ => return None,
        };
        let expr = self.filter_expr_in_zone(&cleaned.join(","), true)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::CountOf {{ filter: &{name}, zone: {zone} }}"
        ))
    }

    /// Additional `S:` static ability handlers (`CantAttack,CantBlock`,
    /// `CastWithFlash`, `Panharmonicon`, `DisableTriggers`,
    /// `CantPreventDamage`, `CantLoseLife`, `CantChangeLife`,
    /// `CantSearchLibrary`) not modeled by `statics.rs`.
    pub(super) fn try_extra_static(&mut self, spec: &str) -> ExtraOutcome {
        let Some((mode, p)) = Params::parse(spec) else {
            return ExtraOutcome::Pass;
        };
        match mode.as_str() {
            "CantAttack,CantBlock" => ExtraOutcome::Handled(self.cant_attack_or_block(p)),
            "CastWithFlash" => ExtraOutcome::Handled(self.cast_with_flash(p)),
            "Panharmonicon" => ExtraOutcome::Handled(self.panharmonicon_static(p)),
            "DisableTriggers" => ExtraOutcome::Handled(self.disable_triggers_static(p)),
            "CantPreventDamage" => ExtraOutcome::Handled(self.cant_prevent_combat_damage(p)),
            "CantLoseLife" => {
                ExtraOutcome::Handled(self.player_life_rule(p, "CantLoseLife", "CantLoseLife"))
            }
            "CantChangeLife" => ExtraOutcome::Handled(self.player_life_rule(
                p,
                "CantChangeLife",
                "LifeTotalCantChange",
            )),
            "CantSearchLibrary" => ExtraOutcome::Handled(self.cant_search_library(p)),
            "CantAttach" if !p.has("ValidCardToTarget") => {
                ExtraOutcome::Handled(self.cant_attach_static(p))
            }
            "TapPowerValue" => ExtraOutcome::Handled(self.tap_power_value_static(p)),
            "MinMaxBlocker" => ExtraOutcome::Handled(self.min_max_blocker(p)),
            "IgnoreLegendRule" => ExtraOutcome::Handled(self.ignore_legend_rule(p)),
            "AttackRestrict" | "BlockRestrict" => {
                ExtraOutcome::Handled(self.combat_restrict(&mode, p))
            }
            "CantDraw" => ExtraOutcome::Handled(self.cant_draw_static(p)),
            "IgnoreHexproof" => ExtraOutcome::Handled(self.ignore_hexproof_static(p)),
            "ManaConvert" => ExtraOutcome::Handled(self.mana_convert_static(p)),
            "CantBeActivated" => self.cant_be_activated_extra(p),
            "CantBeCast" => self.cant_be_cast_extra(p),
            "CantBlockBy" => self.cant_block_by_with_check_svar(spec, p),
            "CantAttack,CantBlock,CantBeActivated" => self.cant_attack_block_activate_extra(p),
            "CantAttackUnless" | "CantBlockUnless" => self.combat_unless_extra(&mode, p),
            "CantAttack" => self.cant_attack_self_conditional(p),
            "Continuous" => self.try_extra_continuous(spec, p),
            _ => ExtraOutcome::Pass,
        }
    }

    fn cant_attack_or_block(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny(format!("`CantAttack,CantBlock.Secondary$ {sec}`"));
        }
        if let Some(zone) = p.take("AffectedZone")
            && zone != "Battlefield"
        {
            return self.deny(format!("`CantAttack,CantBlock.AffectedZone$ {zone}`"));
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CantAttack,CantBlock` with no `ValidCard$`".to_string());
        };
        let filter = self.filter_expr(&valid)?;
        let condition = self.condition_any(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `CantAttack,CantBlock.{key}`"));
        }
        let mut expr = Self::static_expr(
            &filter,
            "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))",
        );
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn cast_with_flash(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Caster").as_deref() != Some("You")
            || p.take("ValidSA").as_deref() != Some("Spell")
        {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        };
        let mut condition = self.condition(&mut p)?;
        if condition.is_empty()
            && let Some(var) = p.take("CheckSVar")
        {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            let Some(clause) = self
                .svar_condition(&var, &cmp)
                .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
            else {
                return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
            };
            condition = format!(", condition = Some({clause})");
        }
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        let modifier = match valid_card.trim() {
            "Card" => "Modifier::SpellsHaveFlash(None)".to_string(),
            "Sorcery" => "Modifier::SorceriesHaveFlash".to_string(),
            other => {
                let mut types = Vec::new();
                for alt in other.split(',') {
                    let word = alt.trim();
                    let Some(ty) = card_type_const(word) else {
                        return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
                    };
                    if !types.contains(&ty) {
                        types.push(ty);
                    }
                }
                if types.is_empty() {
                    return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
                }
                let joined = types
                    .iter()
                    .skip(1)
                    .fold(types[0].to_string(), |mut acc, t| {
                        acc.push_str(".union(");
                        acc.push_str(t);
                        acc.push(')');
                        acc
                    });
                format!("Modifier::SpellsHaveFlash(Some({joined}))")
            }
        };
        let mut expr = Self::static_expr("Filter::Any", &modifier);
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn panharmonicon_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ Panharmonicon`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ Panharmonicon`".to_string());
        };
        let event = match p.take("ValidMode").as_deref() {
            None => "TriggerEventKind::Any",
            Some("ChangesZone" | "ChangesZoneAll" | "ChangesZone,ChangesZoneAll") => {
                if p.take("Destination").as_deref() != Some("Battlefield")
                    || p.take("ValidCause").as_deref() != Some("Permanent")
                {
                    return self.deny("static ability `S: Mode$ Panharmonicon`".to_string());
                }
                "TriggerEventKind::EntersBattlefield"
            }
            Some(_) => return self.deny("static ability `S: Mode$ Panharmonicon`".to_string()),
        };
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ Panharmonicon`".to_string());
        }
        let expr = self.filter_expr(&valid_card)?;
        let filter = self.body.filter_static("TRIGGER_MULT", &expr);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::TriggerMultiplier {{ \
             source_filter: &{filter}, event: {event} }})"
        ));
        Some(())
    }

    fn disable_triggers_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ DisableTriggers`".to_string());
        }
        if p.peek("ValidTrigger") == Some("Triggered.Ward") {
            p.take("ValidTrigger");
            let valid_card = p.take("ValidCard");
            let cond = self.condition_any(&mut p)?;
            if !p.exhausted() {
                return self.deny("static ability `S: Mode$ DisableTriggers`".to_string());
            }
            let filter = match valid_card {
                None => "Filter::Any".to_string(),
                Some(raw) => {
                    let cleaned = strip_battlefield_zone(&raw).unwrap_or(raw);
                    self.filter_expr(&cleaned)?
                }
            };
            self.push_static_with_cond(&filter, "Modifier::WardDoesntTrigger", &cond);
            return Some(());
        }
        let event = match p.take("ValidMode").as_deref() {
            Some("ChangesZone" | "ChangesZoneAll" | "ChangesZone,ChangesZoneAll") => {
                if p.take("Destination").as_deref() != Some("Battlefield")
                    || p.take("ValidCause").as_deref() != Some("Permanent")
                {
                    return self.deny("static ability `S: Mode$ DisableTriggers`".to_string());
                }
                "TriggerEventKind::EntersBattlefield"
            }
            _ => return self.deny("static ability `S: Mode$ DisableTriggers`".to_string()),
        };
        let valid_card = p.take("ValidCard");
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ DisableTriggers`".to_string());
        }
        let filter = match valid_card {
            None => "Filter::Any".to_string(),
            Some(raw) => {
                let cleaned = strip_battlefield_zone(&raw).unwrap_or(raw);
                let expr = self.filter_expr(&cleaned)?;
                self.body.filter_static("TRIGGER_SUPPRESS", &expr)
            }
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::TriggerSuppress {{ \
             source_filter: &{filter}, event: {event} }})"
        ));
        Some(())
    }

    fn cant_prevent_combat_damage(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("IsCombat").as_deref() != Some("True") {
            return self.deny("static ability `S: Mode$ CantPreventDamage`".to_string());
        }
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CantPreventDamage`".to_string());
        }
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            return self.deny("static ability `S: Mode$ CantPreventDamage`".to_string());
        }
        let valid_source = p.take("ValidSource");
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantPreventDamage`".to_string());
        }
        let filter = match valid_source {
            None => "Filter::Any".to_string(),
            Some(valid) => self.filter_expr(&valid)?,
        };
        let mut expr = Self::static_expr(&filter, "Modifier::CombatDamageCantBePrevented");
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn player_life_rule(&mut self, mut p: Params, mode: &str, variant: &str) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let who = match p.take("ValidPlayer").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player") => "PlayerRel::EachPlayer",
            _ => return self.deny(format!("static ability `S: Mode$ {mode}`")),
        };
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        let mut expr = Self::static_expr(
            "Filter::Any",
            &format!("Modifier::{variant} {{ who: {who} }}"),
        );
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn cant_search_library(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CantSearchLibrary`".to_string());
        }
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            return self.deny("static ability `S: Mode$ CantSearchLibrary`".to_string());
        }
        if !matches!(
            p.take("ValidPlayer").as_deref(),
            Some("Opponent" | "Player.Opponent")
        ) {
            return self.deny("static ability `S: Mode$ CantSearchLibrary`".to_string());
        }
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantSearchLibrary`".to_string());
        }
        let mut expr = Self::static_expr("Filter::Any", "Modifier::OpponentsCantSearch");
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    /// `K:Station` (CR 702.184a): sorcery-speed tap another untapped creature
    /// you control to put charge counters equal to its power on this permanent.
    fn station_keyword(&mut self) -> Option<()> {
        if !self.is_permanent_card() {
            return self.deny("keyword `Station` on a non-permanent".to_string());
        }
        self.body.abilities.push(
            "activated!(cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)), \
             &[Effect::AddCounter { kind: CounterKind::Charge, \
             amount: Amount::TappedPower }], timing = ActivationTiming::SorcerySpeed)"
                .to_string(),
        );
        Some(())
    }

    /// `K:Level up:<cost>` (CR 702.87a): sorcery-speed activated ability that
    /// puts a level counter on this permanent.
    fn level_up(&mut self, rest: &str) -> Option<()> {
        if !self.is_permanent_card() || rest.contains(':') {
            return self.deny("keyword `Level`".to_string());
        }
        let Some(mana) = super::printed_mana(rest.trim()) else {
            return self.deny("keyword `Level`".to_string());
        };
        self.body.abilities.push(format!(
            "activated!(cost!({mana:?}), &[Effect::AddCounter {{ kind: CounterKind::Level, \
             amount: Amount::Fixed(1) }}], timing = ActivationTiming::SorcerySpeed)"
        ));
        Some(())
    }

    fn cant_attach_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CantAttach`".to_string());
        }
        let Some(target_raw) = p.take("Target") else {
            return self.deny("static ability `S: Mode$ CantAttach`".to_string());
        };
        let modifier = match p.take("ValidCard").as_deref() {
            Some("Card.Aura" | "Aura") => "Modifier::CantBeEnchanted",
            Some("Aura.Other") if target_raw.contains("EnchantedBy") => {
                "Modifier::CantBeEnchantedExceptSource"
            }
            _ => return self.deny("static ability `S: Mode$ CantAttach`".to_string()),
        };
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantAttach`".to_string());
        }
        let filter = self.filter_expr(&target_raw)?;
        let mut expr = Self::static_expr(&filter, modifier);
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn tap_power_value_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        }
        let Some(valid_sa) = p.take("ValidSA") else {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        };
        let parts: Vec<&str> = valid_sa.split(',').map(str::trim).collect();
        let is_crew_and_saddle = parts.len() == 2
            && parts.contains(&"Activated.Crew+Vehicle")
            && parts.contains(&"Activated.Saddle+Mount");
        if !is_crew_and_saddle {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        };
        let Some(val_raw) = p.take("Value") else {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        };
        let Ok(bonus) = val_raw.trim().parse::<i16>() else {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        };
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ TapPowerValue`".to_string());
        }
        let filter = self.filter_expr(&valid_card)?;
        let mut expr =
            Self::static_expr(&filter, &format!("Modifier::CrewSaddlePowerBonus({bonus})"));
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    /// Extended `S: Mode$ Continuous` handlers for Station thresholds, Leveler
    /// bands, Library/Graveyard/Hand rules, Layer 4/5/6 characteristic rules,
    /// and extra `CheckSVar$` conditions not yet in `activated::svar_condition`.
    fn try_extra_continuous(&mut self, spec: &str, p: Params) -> ExtraOutcome {
        if p.peek("AffectedDefined") == Some("Self") {
            if let Some(min) = p.peek("Affected").and_then(parse_station_threshold) {
                return ExtraOutcome::Handled(self.station_threshold_continuous(p, min));
            }
            if let Some(cond) = p.peek("IsPresent").and_then(parse_level_band) {
                return ExtraOutcome::Handled(self.level_band_continuous(p, &cond));
            }
        }
        if p.peek("AffectedZone") == Some("Library") {
            return ExtraOutcome::Handled(self.continuous_library_top(p));
        }
        if p.peek("AffectedZone") == Some("Graveyard")
            && matches!(p.peek("EffectZone"), None | Some("Battlefield"))
            && p.peek("MayPlay") == Some("True")
        {
            return ExtraOutcome::Handled(self.continuous_graveyard_play(p));
        }
        if p.peek("AffectedZone") == Some("Hand")
            && p.peek("MayLookAt") == Some("Player")
            && matches!(p.peek("Affected"), Some("Card.YouOwn" | "Card.YouCtrl"))
        {
            return ExtraOutcome::Handled(self.continuous_reveal_your_hand(p));
        }
        if matches!(p.peek("EffectZone"), None | Some("Battlefield"))
            && matches!(p.peek("AffectedZone"), None | Some("Battlefield"))
            && !p.has("CharacteristicDefining")
            && (p.has("RemoveAllAbilities")
                || p.has("AddAllCreatureTypes")
                || p.peek("AddType") == Some("AllBasicLandType")
                || p.peek("SetColor") == Some("Colorless")
                || p.has("RemoveCardTypes")
                || p.has("RemoveCreatureTypes")
                || p.peek("AddKeyword")
                    .is_some_and(|k| k.starts_with("Bands with other:")))
        {
            return ExtraOutcome::Handled(self.continuous_wave5_characteristic(p));
        }
        if p.has("CheckSVar")
            && !p.has("IsPresent")
            && !p.has("Condition")
            && !p.has("PlayerTurn")
            && let Some(var) = p.peek("CheckSVar")
        {
            let cmp = p.peek("SVarCompare").unwrap_or("GE1");
            if let Some(first) = self
                .svar_condition(var, cmp)
                .or_else(|| self.extra_svar_condition_clause(var, cmp))
            {
                let cond = if let Some(var2) = p.peek("CheckSecondSVar") {
                    let cmp2 = p.peek("SecondSVarCompare").unwrap_or("GE1");
                    let Some(second) = self
                        .svar_condition(var2, cmp2)
                        .or_else(|| self.extra_svar_condition_clause(var2, cmp2))
                    else {
                        return ExtraOutcome::Pass;
                    };
                    format!("Condition::All(&[{first}, {second}])")
                } else {
                    first
                };
                return ExtraOutcome::Handled(self.continuous_with_extra_condition(spec, &cond));
            }
        }
        ExtraOutcome::Pass
    }

    fn station_threshold_continuous(&mut self, mut p: Params, min: u8) -> Option<()> {
        p.drop_prose();
        p.take("AffectedDefined");
        p.take("Affected");
        let cond = format!("Condition::Station({min})");
        self.threshold_continuous_body(p, Some(min), None, &cond)
    }

    fn level_band_continuous(&mut self, mut p: Params, cond: &str) -> Option<()> {
        p.drop_prose();
        p.take("AffectedDefined");
        let raw_present = p.take("IsPresent");
        if !matches!(
            p.take("PresentCompare").as_deref(),
            None | Some("EQ1" | "GE1")
        ) {
            return self.deny("leveler band static with unsupported `PresentCompare$`".to_string());
        }
        self.threshold_continuous_body(p, None, raw_present.as_deref(), cond)
    }

    fn threshold_continuous_body(
        &mut self,
        mut p: Params,
        station_min: Option<u8>,
        level_present: Option<&str>,
        cond: &str,
    ) -> Option<()> {
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("threshold static with unexpected `Secondary$`".to_string());
        }
        if let Some(svars) = p.take("AddSVar") {
            for name in svars
                .split([',', '&'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if !self.svars.contains_key(name) {
                    return self.deny(format!("threshold static naming missing `SVar:{name}`"));
                }
            }
        }
        let before = self.body.abilities.len();
        let res = self.lower_threshold_clauses(&mut p, station_min, level_present, cond);
        if res.is_none() || !p.exhausted() || self.body.abilities.len() == before {
            self.body.abilities.truncate(before);
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `Continuous.{key}`"));
            }
            return None;
        }
        Some(())
    }

    fn lower_threshold_clauses(
        &mut self,
        p: &mut Params,
        station_min: Option<u8>,
        level_present: Option<&str>,
        cond: &str,
    ) -> Option<()> {
        if let Some(types_raw) = p.take("AddType") {
            let types = join_card_types(&types_raw)?;
            if let Some(at_least) = station_min {
                self.body.abilities.push(Self::static_expr(
                    "Filter::This",
                    &format!(
                        "Modifier::AddTypeIfCountersAtLeast {{ kind: CounterKind::Charge, \
                         at_least: {at_least}, types: {types} }}"
                    ),
                ));
            } else {
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::AddType({types}), \
                     condition = Some({cond}))"
                ));
            }
        }
        if p.has("SetPower") || p.has("SetToughness") {
            let pow: i16 = p.take("SetPower")?.trim().parse().ok()?;
            let tou: i16 = p.take("SetToughness")?.trim().parse().ok()?;
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::SetPT({pow}, {tou}), \
                 condition = Some({cond}))"
            ));
        }
        if p.has("AddPower") || p.has("AddToughness") {
            let pow: i16 = p
                .take("AddPower")
                .map_or(Ok(0), |s| s.trim().trim_start_matches('+').parse())
                .ok()?;
            let tou: i16 = p
                .take("AddToughness")
                .map_or(Ok(0), |s| s.trim().trim_start_matches('+').parse())
                .ok()?;
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::ModifyPT({pow}, {tou}), \
                 condition = Some({cond}))"
            ));
        }
        if let Some(kws) = p.take("AddKeyword") {
            self.lower_threshold_keywords(&kws, station_min, cond)?;
        }
        if let Some(statics) = p.take("AddStaticAbility") {
            self.lower_threshold_granted(
                &statics,
                'S',
                level_present,
                cond,
                station_min.is_some(),
            )?;
        }
        if let Some(abilities) = p.take("AddAbility") {
            self.lower_threshold_granted(
                &abilities,
                'A',
                level_present,
                cond,
                station_min.is_some(),
            )?;
        }
        if let Some(triggers) = p.take("AddTrigger") {
            self.lower_threshold_granted(
                &triggers,
                'T',
                level_present,
                cond,
                station_min.is_some(),
            )?;
        }
        Some(())
    }

    fn lower_threshold_keywords(
        &mut self,
        raw: &str,
        station_min: Option<u8>,
        cond: &str,
    ) -> Option<()> {
        let mut bits: Vec<&'static str> = Vec::new();
        for kw in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(bit) = super::keyword_const_of(kw) {
                if !bits.contains(&bit) {
                    bits.push(bit);
                }
            } else if kw == "Provoke" {
                if !bits.contains(&"KeywordSet::PROVOKE") {
                    bits.push("KeywordSet::PROVOKE");
                }
            } else {
                let from = self.protection_filter(kw)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::ProtectionFrom(&{from}), \
                     condition = Some({cond}))"
                ));
            }
        }
        if !bits.is_empty() {
            let joined = bits.iter().skip(1).fold(bits[0].to_string(), |mut acc, b| {
                acc.push_str(".union(");
                acc.push_str(b);
                acc.push(')');
                acc
            });
            if let Some(at_least) = station_min {
                self.body.abilities.push(Self::static_expr(
                    "Filter::This",
                    &format!(
                        "Modifier::AddKeywordIfCountersAtLeast {{ kind: CounterKind::Charge, \
                         at_least: {at_least}, keywords: {joined} }}"
                    ),
                ));
            } else {
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::AddKeyword({joined}), \
                     condition = Some({cond}))"
                ));
            }
        }
        Some(())
    }

    fn lower_threshold_granted(
        &mut self,
        names: &str,
        kind: char,
        level_present: Option<&str>,
        cond: &str,
        is_station: bool,
    ) -> Option<()> {
        for name in names
            .split([',', '&'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let mut body = self.svars.get(name)?.clone();
            if kind == 'S'
                && let Some(expected) = level_present
            {
                body = strip_matching_level_present(&body, expected);
            }
            let start = self.body.abilities.len();
            self.rule(kind, &body)?;
            if self.body.abilities.len() == start {
                return None;
            }
            for item in &mut self.body.abilities[start..] {
                let updated = attach_condition_to_ability(item, cond, is_station)?;
                *item = updated;
            }
        }
        Some(())
    }

    fn continuous_with_extra_condition(&mut self, spec: &str, cond: &str) -> Option<()> {
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| {
                let key = part.split_once('$').map_or("", |(k, _)| k.trim());
                !matches!(
                    key,
                    "CheckSVar" | "SVarCompare" | "CheckSecondSVar" | "SecondSVarCompare"
                )
            })
            .collect();
        let clean_spec = stripped.join(" | ");
        let start = self.body.abilities.len();
        if self.static_ability(&clean_spec).is_none() || self.body.abilities.len() == start {
            self.body.abilities.truncate(start);
            return None;
        }
        for item in &mut self.body.abilities[start..] {
            if !item.starts_with("static_ability!(") {
                self.body.abilities.truncate(start);
                return None;
            }
            let Some(updated) = attach_condition_to_ability(item, cond, false) else {
                self.body.abilities.truncate(start);
                return None;
            };
            *item = updated;
        }
        Some(())
    }

    /// Additional `CheckSVar$` + `SVarCompare$` conditions mapped 1:1 to existing
    /// `baylee-cards-dsl::Condition` variants not yet handled by `activated::svar_condition`.
    pub(super) fn extra_svar_condition_clause(&self, var: &str, cmp: &str) -> Option<String> {
        self.extra_svar_condition(var, cmp)
            .or_else(|| self.extra_svar_condition_wave5(var, cmp))
    }

    fn extra_svar_condition_wave5(&self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.svars.get(var.trim())?.trim();
        let cmp = cmp.trim();
        if raw == "Count$ThisTurnCast_Card.YouCtrl" && matches!(cmp, "EQ0" | "LE0") {
            return Some("Condition::YouCastNoSpellThisTurn".to_string());
        }
        if raw == "Count$Morbid.1.0" && matches!(cmp, "GE1" | "GT0") {
            return Some("Condition::CreaturesDiedThisTurnAtLeast(1)".to_string());
        }
        if raw == "Count$Morbid.0.1" && matches!(cmp, "GE1" | "GT0") {
            return Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string());
        }
        if raw == "Count$ThisTurnCast_Creature.YouCtrl" {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::CastCreatureThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::CastCreatureThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if matches!(
            raw,
            "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl"
                | "Count$ThisTurnCast_Sorcery.YouCtrl,Instant.YouCtrl"
        ) {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::CastInstantOrSorceryThisTurn".to_string()),
                "EQ0" | "LT1" => {
                    Some("Condition::Not(&Condition::CastInstantOrSorceryThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if raw == "PlayerCountPropertyYou$DamageToOppsThisTurn" {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!("Condition::OpponentWasDealtAtLeast({n})"));
            }
            if matches!(cmp, "EQ0" | "LT1") {
                return Some("Condition::Not(&Condition::OpponentWasDealtAtLeast(1))".to_string());
            }
        }
        if raw == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
            && let Some(n) = parse_at_least_u16(cmp)
        {
            return Some(format!("Condition::CreaturesDiedThisTurnAtLeast({n})"));
        }
        if raw == "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl"
            && let Some(n) = parse_at_least_u16(cmp)
        {
            return Some(format!(
                "Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, {n})"
            ));
        }
        if raw == "Count$YourLifeTotal" {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!(
                    "Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n})"
                ));
            }
            if let Some(n) = parse_at_most_u16(cmp) {
                return Some(format!("Condition::LifeAtMost({n})"));
            }
        }
        if raw == "Count$TotalPower Creature.YouCtrl"
            && let Some(n) = parse_at_least_u16(cmp)
        {
            return Some(format!("Condition::ControlledCreaturePowerAtLeast({n})"));
        }
        if raw == "Count$ValidHand Card.YouOwn" {
            if let Some(n_str) = cmp.strip_prefix("EQ")
                && let Ok(n) = n_str.parse::<u8>()
            {
                return Some(if n == 0 {
                    "Condition::HandSizeAtMost(0)".to_string()
                } else {
                    format!("Condition::HandSizeExactly({n})")
                });
            }
            if let Some(n) = parse_at_most_u16(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::HandSizeAtMost({n})"));
            }
        }
        if raw == "Count$ValidGraveyard Card.YouOwn" {
            if let Some(n) = parse_at_least_u16(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::GraveyardCountAtLeast({n})"));
            }
            if let Some(max) = parse_at_most_u16(cmp)
                && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::GraveyardCountAtLeast({min}))"
                ));
            }
        }
        if raw == "PlayerCount$LowestCardsInLibrary"
            && let Some(n) = parse_at_most_u16(cmp)
        {
            return Some(format!("Condition::AnyLibraryAtMost({n})"));
        }
        None
    }

    fn extra_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.svars.get(var.trim())?.trim();
        let cmp = cmp.trim();
        if raw == "PlayerCountOpponents$HighestCounters.Poison" {
            let n = parse_at_least_u16(cmp)?;
            return Some(format!("Condition::OpponentPoisonAtLeast({n})"));
        }
        if matches!(raw, "Count$ValidHand Card.All" | "Count$ValidHand Card") && cmp == "EQ0" {
            return Some("Condition::EveryHandEmpty".to_string());
        }
        if raw == "Count$YourLifeTotal"
            && let Some(rhs) = cmp.strip_prefix("LT")
            && self.svars.get(rhs.trim()).map(|s| s.trim()) == Some("Count$StartingLife")
        {
            return Some("Condition::LifeBelowStarting".to_string());
        }
        if raw == "Count$TypeInYourYard" {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            return Some(format!("Condition::GraveyardTypesAtLeast({n})"));
        }
        if matches!(
            raw,
            "Count$ValidGraveyard Card.OppOwn" | "PlayerCountOpponents$HighestCardsInGraveyard"
        ) {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            return Some(format!("Condition::OpponentGraveyardCountAtLeast({n})"));
        }
        if raw == "PlayerCount$LowestCardsInHand" {
            let n = u8::try_from(parse_at_most_u16(cmp)?).ok()?;
            return Some(format!("Condition::APlayerHandSizeAtMost({n})"));
        }
        if raw == "Count$LandsPlayed" {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            return Some(format!("Condition::LandsPlayedThisTurnAtLeast({n})"));
        }
        if raw == "Count$OpponentsLifeLostThisTurn" {
            let n = u32::from(parse_at_least_u16(cmp)?);
            return Some(format!(
                "Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, {n})"
            ));
        }
        if raw == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Permanent.YouOwn+!token"
            && cmp == "GE1"
        {
            return Some("Condition::DescendedThisTurn".to_string());
        }
        if raw == "Count$CardCounters.ALL/Mod.2" && cmp == "EQ0" {
            return Some("Condition::EvenCountersOnSelf".to_string());
        }
        None
    }

    /// Additional `R:` replacement handlers (`BeginPhase`, `BeginTurn`, `Draw`,
    /// `CreateToken`, `AddCounter`, `GainLife`, and extended `Moved`).
    pub(super) fn try_extra_replacement(&mut self, spec: &str) -> ExtraOutcome {
        let Some((event, p)) = Params::parse(spec) else {
            return ExtraOutcome::Pass;
        };
        match event.as_str() {
            "BeginPhase" => self.replace_begin_phase_draw(p),
            "BeginTurn" => ExtraOutcome::Handled(self.replace_begin_turn(p)),
            "Draw" => ExtraOutcome::Handled(self.replace_draw_extra(p)),
            "CreateToken" => ExtraOutcome::Handled(self.replace_create_token(p)),
            "AddCounter" => ExtraOutcome::Handled(self.replace_add_counter(p)),
            "GainLife" => ExtraOutcome::Handled(self.replace_gain_life(p)),
            "Moved" => self.try_extra_moved(p),
            _ => ExtraOutcome::Pass,
        }
    }

    fn replace_gain_life(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        }
        if p.peek("Prevent") == Some("True") {
            p.take("Prevent");
            let who = match p.take("ValidPlayer").as_deref() {
                None | Some("Player") => "PlayerRel::EachPlayer",
                Some("You") => "PlayerRel::You",
                Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                Some(_) => return self.deny("replacement `R: Event$ GainLife`".to_string()),
            };
            let cond = self.condition_any(&mut p)?;
            if !p.exhausted() {
                return self.deny("replacement `R: Event$ GainLife`".to_string());
            }
            self.push_static_with_cond(
                "Filter::Any",
                &format!("Modifier::LifeGainNoLifeInstead {{ who: {who} }}"),
                &cond,
            );
            return Some(());
        }
        if p.take("ValidPlayer").as_deref() != Some("You") {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        }
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&replace_with).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        };
        body.drop_prose();
        if api != "Draw" || body.take("Defined").as_deref() != Some("You") {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        }
        let num_var = body.take("NumCards")?;
        if !body.exhausted()
            || self.svars.get(num_var.trim()).map(|s| s.trim()) != Some("ReplaceCount$LifeGained")
        {
            return self.deny("replacement `R: Event$ GainLife`".to_string());
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::Any",
            "Modifier::LifeGainDrawsInstead { who: PlayerRel::You }",
        ));
        Some(())
    }

    /// Extended `R: Event$ Moved` handlers that leave `keywords.rs::replacement`,
    /// `gond_entry`, and `external_enter_tapped` untouched for their existing shapes.
    fn try_extra_moved(&mut self, p: Params) -> ExtraOutcome {
        let valid_card = p.peek("ValidCard");
        let dest = p.peek("Destination");
        // Leave `gond_entry` and `external_enter_tapped` on their exact existing paths.
        if matches!(
            valid_card,
            Some(
                "Gate.YouCtrl"
                    | "Land.OppCtrl+nonBasic"
                    | "Artifact.OppCtrl,Creature.OppCtrl,Land.OppCtrl"
            )
        ) {
            return ExtraOutcome::Pass;
        }
        // Discard-to-library replacement (`ReplacementRule::MayDiscardToLibraryTop`).
        if p.peek("Discard") == Some("True") {
            return ExtraOutcome::Handled(self.moved_discard_to_library(p));
        }
        // Prevent entering the battlefield (`Modifier::CantEnterBattlefield`).
        if p.peek("Prevent") == Some("True") && dest == Some("Battlefield") {
            return ExtraOutcome::Handled(self.moved_cant_enter(p));
        }
        // Departure exile replacement (`Modifier::ExileInsteadOfLeavingBattlefield`).
        if p.peek("ExcludeDestination") == Some("Exile") {
            return ExtraOutcome::Handled(self.moved_exile_on_leave(p));
        }
        // Graveyard-to-exile replacements (`ExileOpponentsGraveyard`,
        // `ExileInsteadOfYourGraveyard`) and Firestorm Phoenix.
        if dest == Some("Graveyard") {
            if self.moved_firestorm_phoenix(&p) {
                let mut check = p;
                check.drop_prose();
                check.take("ActiveZones");
                check.take("Origin");
                check.take("Destination");
                check.take("ValidCard");
                check.take("ReplaceWith");
                if check.exhausted() {
                    self.body.abilities.push(
                        "AbilityDef::Replacement(ReplacementRule::ReturnSelfToHandRevealedCantPlayUntilOwnersNextTurn)"
                            .to_string(),
                    );
                    return ExtraOutcome::Handled(Some(()));
                }
                return ExtraOutcome::Handled(
                    self.deny("replacement `Moved` this rule cannot read".to_string()),
                );
            }
            return ExtraOutcome::Handled(self.moved_exile_instead_of_graveyard(p));
        }
        // Self-ETB replacements whose `ReplaceWith$` is NOT `DB$ Tap`.
        if valid_card == Some("Card.Self") && dest == Some("Battlefield") {
            if let Some(with) = p.peek("ReplaceWith")
                && let Some((api, _)) = self.svars.get(with).and_then(|s| Params::parse(s))
            {
                if api == "Reveal" {
                    return ExtraOutcome::Handled(self.moved_reveal_or_control_tap(p));
                }
                if api == "LoseLife" {
                    return ExtraOutcome::Handled(self.moved_lose_life_equal_to_life(p));
                }
            }
            return ExtraOutcome::Pass;
        }
        // External battlefield enter-tapped / enter-untapped replacements.
        if dest == Some("Battlefield")
            && p.peek("ActiveZones") == Some("Battlefield")
            && p.peek("ReplacementResult") == Some("Updated")
            && valid_card.is_some_and(|v| !v.starts_with("Card.Self"))
        {
            return ExtraOutcome::Handled(self.moved_external_enter_orientation(p));
        }
        ExtraOutcome::Pass
    }

    fn moved_cant_enter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("Prevent").as_deref() != Some("True")
            || p.take("Layer").as_deref() != Some("CantHappen")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("CANT_ENTER", &expr);
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!("Modifier::CantEnterBattlefield(&{filter})"),
        ));
        Some(())
    }

    fn moved_exile_on_leave(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Origin").as_deref() != Some("Battlefield")
            || p.take("ExcludeDestination").as_deref() != Some("Exile")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || !matches!(body.take("Origin").as_deref(), Some("Battlefield" | "All"))
            || body.take("Destination").as_deref() != Some("Exile")
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
            || !body.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let filter = if valid == "Card.Self" {
            "Filter::This".to_string()
        } else {
            self.filter_expr(&valid)?
        };
        self.body.abilities.push(Self::static_expr(
            &filter,
            "Modifier::ExileInsteadOfLeavingBattlefield",
        ));
        Some(())
    }

    fn moved_exile_instead_of_graveyard(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Graveyard")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let origin = p.take("Origin");
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || body.take("Destination").as_deref() != Some("Exile")
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let body_origin = body.take("Origin");
        let counter_raw = body.take("WithCountersType");
        if !body.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        if origin.is_none() && body_origin.as_deref() == Some("All") {
            if valid == "Card.!token+OppOwn" {
                let counter = match counter_raw {
                    None => "None".to_string(),
                    Some(code) => format!("Some({})", counter_kind(code.trim())?),
                };
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::ExileOpponentsGraveyard {{ \
                     counter: {counter} }})"
                ));
                return Some(());
            }
            if valid == "Card.!token+YouOwn" && counter_raw.is_none() {
                self.body.abilities.push(Self::static_expr(
                    "Filter::Any",
                    "Modifier::ExileInsteadOfYourGraveyard",
                ));
                return Some(());
            }
        }
        self.deny("replacement `Moved` this rule cannot read".to_string())
    }

    fn moved_reveal_or_control_tap(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api, mut rev)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        rev.drop_prose();
        let valid = rev.take("RevealValid")?;
        let tap_var = rev.take("SubAbility")?;
        if api != "Reveal"
            || rev.take("Optional").as_deref() != Some("True")
            || rev.take("RememberRevealed").as_deref() != Some("True")
            || !rev.exhausted()
            || valid.contains("Ctrl")
            || valid.contains("Own")
            || valid.contains("Self")
            || valid.contains("Other")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((tap_api, mut tap)) = self.svars.get(&tap_var).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        tap.drop_prose();
        let check_var = tap.take("ConditionCheckSVar")?;
        let cleanup_var = tap.take("SubAbility");
        if tap_api != "Tap"
            || tap.take("ETB").as_deref() != Some("True")
            || tap.take("Defined").as_deref() != Some("Self")
            || tap.take("ConditionSVarCompare").as_deref() != Some("EQ0")
            || !tap.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        if let Some(clean) = cleanup_var {
            let Some((c_api, mut cp)) = self.svars.get(&clean).and_then(|s| Params::parse(s))
            else {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            };
            cp.drop_prose();
            if c_api != "Cleanup"
                || cp.take("ClearRemembered").as_deref() != Some("True")
                || !cp.exhausted()
            {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            }
        }
        let ctrl_var = self
            .svars
            .get(check_var.trim())?
            .trim()
            .strip_prefix("Count$RememberedSize/Plus.")?;
        let ctrl_def = self.svars.get(ctrl_var.trim())?.trim();
        let expected_a = format!("Count$Valid {valid}.inZoneBattlefield+YouCtrl");
        let expected_b = format!("Count$Valid {valid}.YouCtrl+inZoneBattlefield");
        if ctrl_def != expected_a && ctrl_def != expected_b {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let filter = self.body.filter_static("REVEAL", &expr);
        self.body.enter_modifiers.push(format!(
            "EnterModifier::TappedUnlessRevealOrControl(&{filter})"
        ));
        Some(())
    }

    fn moved_lose_life_equal_to_life(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        body.drop_prose();
        let life_var = body.take("LifeAmount")?;
        if api != "LoseLife"
            || !matches!(body.take("Defined").as_deref(), None | Some("You"))
            || !body.exhausted()
            || self.svars.get(life_var.trim()).map(|s| s.trim()) != Some("Count$YourLifeTotal")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::LoseLifeEqualToLife".to_string());
        Some(())
    }

    fn moved_external_enter_orientation(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let valid = p.take("ValidCard")?;
        if p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted()
            || valid.contains("Self")
            || valid.contains("Other")
            || valid.contains("EnchantedPlayer")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        body.drop_prose();
        if body.take("ETB").as_deref() != Some("True")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || !body.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let rule = match api.as_str() {
            "Tap" => "EnterTapped",
            // Keep `EnterUntapped` scoped to your own permanents (preserving
            // `gond_entry`'s `Gate.OppCtrl` refusal boundary).
            "Untap"
                if valid
                    .split(',')
                    .all(|alt| alt.split(['.', '+']).any(|a| a.trim() == "YouCtrl")) =>
            {
                "EnterUntapped"
            }
            _ => return self.deny("replacement `Moved` this rule cannot read".to_string()),
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("ENTER", &expr);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::{rule}(&{filter}))"
        ));
        Some(())
    }

    fn replace_create_token(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        if let Some(effect_only) = p.take("EffectOnly")
            && effect_only != "True"
        {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some(valid_token) = p.take("ValidToken") else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some((api, mut body)) = self.svars.get(&replace_with).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        body.drop_prose();
        if api != "ReplaceToken" || body.take("Type").as_deref() != Some("Amount") {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let factor: u32 = match body.take("Amount").as_deref() {
            None | Some("Twice" | "2") => 2,
            Some("Thrice" | "3") => 3,
            Some(other) => match other.parse::<u32>() {
                Ok(n) if n >= 2 => n,
                _ => return self.deny("replacement `R: Event$ CreateToken`".to_string()),
            },
        };
        if !body.exhausted() {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        if valid_token == "Card.YouCtrl" && factor == 2 {
            self.body.abilities.push(
                "AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation { \
                 controller_filter: &Filter::ControlledByYou })"
                    .to_string(),
            );
            return Some(());
        }
        if let Some(type_word) = valid_token.strip_suffix(".YouCtrl")
            && let Some(ty) = card_type_const(type_word)
        {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::MultiplyTypedTokens {{ \
                 controller_filter: &Filter::ControlledByYou, types: {ty}, factor: {factor} }})"
            ));
            return Some(());
        }
        self.deny("replacement `R: Event$ CreateToken`".to_string())
    }

    fn replace_add_counter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") || p.has("ValidCounterType") {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some((api, mut body)) = self.svars.get(&replace_with).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        body.drop_prose();
        if api != "ReplaceCounter" || body.has("ValidCounterType") {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        let Some(amt_var) = body.take("Amount") else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some(amt_def) = self.svars.get(&amt_var).map(|s| s.trim().to_string()) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };

        // Case 1: "If you would put one or more counters on a permanent or
        // player, put twice that many ... instead." (Innkeeper's Talent).
        if p.peek("ValidSource") == Some("You")
            && p.peek("ValidObject") == Some("Permanent.inZoneBattlefield,Player")
        {
            return self.replace_add_counter_by_you(p, body, &amt_def);
        }

        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some(stripped) = strip_battlefield_zone(&valid_card) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let effect_only = match p.take("EffectOnly").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return self.deny("replacement `R: Event$ AddCounter`".to_string()),
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }

        // Case 2: effect-only counter doubler on matching battlefield permanents
        // (Doubling Season).
        if effect_only
            && body.exhausted()
            && matches!(
                amt_def.as_str(),
                "ReplaceCount$CounterNum/Twice" | "ReplaceCount$CounterNum/Times.2"
            )
        {
            let expr = self.filter_expr(&stripped)?;
            let filter = self.body.filter_static("COUNTERS", &expr);
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement {{ \
                 object_filter: &{filter} }})"
            ));
            return Some(());
        }

        // Case 3: unconditional additive counter placement on matching
        // battlefield permanents (`Modifier::AdditionalCounterPlacement`).
        if !effect_only
            && matches!(body.take("ChooseCounter").as_deref(), None | Some("True"))
            && body.exhausted()
            && let Some(n_str) = amt_def.strip_prefix("ReplaceCount$CounterNum/Plus.")
            && let Ok(n) = n_str.trim().parse::<u16>()
            && n > 0
        {
            let expr = self.filter_expr(&stripped)?;
            self.body.abilities.push(Self::static_expr(
                &expr,
                &format!("Modifier::AdditionalCounterPlacement {{ n: {n} }}"),
            ));
            return Some(());
        }

        self.deny("replacement `R: Event$ AddCounter`".to_string())
    }

    fn replace_add_counter_by_you(
        &mut self,
        mut p: Params,
        mut body: Params,
        amt_def: &str,
    ) -> Option<()> {
        p.take("ValidSource");
        p.take("ValidObject");
        if let Some(src) = body.take("ValidSource")
            && src != "You"
        {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        if !matches!(body.take("ChooseCounter").as_deref(), None | Some("True")) {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        if p.exhausted()
            && body.exhausted()
            && matches!(
                amt_def,
                "ReplaceCount$CounterNum/Twice" | "ReplaceCount$CounterNum/Times.2"
            )
        {
            self.body.abilities.push(Self::static_expr(
                "Filter::Any",
                "Modifier::DoubleCountersPlacedByYou",
            ));
            return Some(());
        }
        self.deny("replacement `R: Event$ AddCounter`".to_string())
    }
}

/// Strips `+inZoneBattlefield` or `.inZoneBattlefield` from every comma-
/// separated alternative in a `ValidCard$` filter, returning `None` if any
/// alternative does not require the battlefield.
fn strip_battlefield_zone(valid: &str) -> Option<String> {
    let mut out = Vec::new();
    for alt in valid.split(',') {
        let alt = alt.trim();
        let cleaned = alt
            .strip_suffix("+inZoneBattlefield")
            .or_else(|| alt.strip_suffix(".inZoneBattlefield"))?;
        if cleaned.is_empty() {
            return None;
        }
        out.push(cleaned);
    }
    (!out.is_empty()).then(|| out.join(","))
}

fn apply_etb_amount_suffix(base: String, suffix: &str) -> Option<String> {
    match suffix.trim() {
        "Twice" => {
            if base == "Amount::X" {
                Some("Amount::DoubleX".to_string())
            } else {
                Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"))
            }
        }
        "HalfDown" => Some(format!("Amount::Half {{ base: &{base}, round_up: false }}")),
        "HalfUp" => Some(format!("Amount::Half {{ base: &{base}, round_up: true }}")),
        other => {
            if let Some(n_str) = other.strip_prefix("Plus.")
                && let Ok(offset) = n_str.trim().parse::<u32>()
                && offset > 0
            {
                return Some(format!(
                    "Amount::Plus {{ base: &{base}, offset: {offset} }}"
                ));
            }
            if let Some(n_str) = other.strip_prefix("Minus.")
                && let Ok(subtract) = n_str.trim().parse::<u32>()
                && subtract > 0
            {
                return Some(format!(
                    "Amount::SaturatingSub {{ base: &{base}, subtract: {subtract} }}"
                ));
            }
            if let Some(n_str) = other.strip_prefix("Times.")
                && let Ok(factor) = n_str.trim().parse::<u32>()
                && factor > 0
            {
                return Some(if factor == 1 {
                    base
                } else if factor == 2 && base == "Amount::X" {
                    "Amount::DoubleX".to_string()
                } else {
                    format!("Amount::Times {{ base: &{base}, factor: {factor} }}")
                });
            }
            None
        }
    }
}

fn parse_station_threshold(raw: &str) -> Option<u8> {
    let rest = raw.trim().strip_prefix("Card.counters_GE")?;
    let n: u8 = rest.strip_suffix("_CHARGE")?.parse().ok()?;
    (n >= 1).then_some(n)
}

fn parse_level_band(raw: &str) -> Option<String> {
    let rest = raw.trim().strip_prefix("Card.Self+counters_GE")?;
    let (lo_str, tail) = rest.split_once("_LEVEL")?;
    let lo: u8 = lo_str.parse().ok()?;
    if lo == 0 {
        return None;
    }
    if tail.is_empty() {
        return Some(format!(
            "Condition::CountersOnSelf(CounterKind::Level, {lo})"
        ));
    }
    if let Some(hi_str) = tail
        .strip_prefix("+counters_LE")
        .and_then(|s| s.strip_suffix("_LEVEL"))
    {
        let hi: u8 = hi_str.parse().ok()?;
        return (hi >= lo)
            .then(|| format!("Condition::CountersOnSelfBetween(CounterKind::Level, {lo}, {hi})"));
    }
    if let Some(hi_str) = tail
        .strip_prefix("+counters_LT")
        .and_then(|s| s.strip_suffix("_LEVEL"))
    {
        let hi: u8 = hi_str.parse().ok()?;
        let max = hi.checked_sub(1)?;
        return (max >= lo)
            .then(|| format!("Condition::CountersOnSelfBetween(CounterKind::Level, {lo}, {max})"));
    }
    None
}

fn join_card_types(raw: &str) -> Option<String> {
    let mut types = Vec::new();
    for word in raw
        .split([',', '&'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let ty = card_type_const(word)?;
        if !types.contains(&ty) {
            types.push(ty);
        }
    }
    if types.is_empty() {
        return None;
    }
    Some(
        types
            .iter()
            .skip(1)
            .fold(types[0].to_string(), |mut acc, t| {
                acc.push_str(".union(");
                acc.push_str(t);
                acc.push(')');
                acc
            }),
    )
}

fn attach_condition_to_ability(expr: &str, cond: &str, is_station: bool) -> Option<String> {
    let is_supported_macro = expr.starts_with("static_ability!(")
        || expr.starts_with("activated!(")
        || expr.starts_with("mana_ability!(")
        || expr.starts_with("triggered!(")
        || expr.starts_with("modal_triggered!(");
    if !is_supported_macro {
        return None;
    }
    let normalized = if let Some(rest) = expr.strip_prefix("mana_ability!(&[") {
        format!("mana_ability!(Cost::TAP, &[{rest}")
    } else {
        expr.to_string()
    };
    let open = normalized.strip_suffix(')')?;
    if let Some((head, existing_tail)) = open.rsplit_once(", condition = Some(") {
        if head.contains("condition =") {
            return None;
        }
        let existing = existing_tail.strip_suffix(')')?;
        if existing == cond {
            return Some(normalized);
        }
        // `progress/triggers.rs` checks top-level `Condition::Station(_)` for
        // CR 722.2a; do not bury `Station` inside `Condition::All` on a trigger.
        if is_station
            && (normalized.starts_with("triggered!(")
                || normalized.starts_with("modal_triggered!("))
        {
            return None;
        }
        return Some(format!(
            "{head}, condition = Some(Condition::All(&[{existing}, {cond}])))"
        ));
    }
    if open.contains("condition =") {
        return None;
    }
    Some(format!("{open}, condition = Some({cond}))"))
}

fn parse_at_least_u16(cmp: &str) -> Option<u16> {
    if let Some(n) = cmp.strip_prefix("GE") {
        let v: u16 = n.parse().ok()?;
        return (v >= 1).then_some(v);
    }
    if let Some(n) = cmp.strip_prefix("GT") {
        let v: u16 = n.parse().ok()?;
        return v.checked_add(1);
    }
    None
}

fn parse_at_most_u16(cmp: &str) -> Option<u16> {
    if let Some(n) = cmp.strip_prefix("LE") {
        return n.parse().ok();
    }
    if let Some(n) = cmp.strip_prefix("LT") {
        let v: u16 = n.parse().ok()?;
        return v.checked_sub(1);
    }
    if cmp == "EQ0" {
        return Some(0);
    }
    None
}

fn strip_matching_level_present(spec: &str, expected_present: &str) -> String {
    let has_match = spec.split(" | ").any(|part| {
        part.split_once('$')
            .is_some_and(|(k, v)| k.trim() == "IsPresent" && v.trim() == expected_present)
    });
    if !has_match {
        return spec.to_string();
    }
    let bad_cmp = spec.split(" | ").any(|part| {
        part.split_once('$').is_some_and(|(k, v)| {
            k.trim() == "PresentCompare" && !matches!(v.trim(), "EQ1" | "GE1")
        })
    });
    if bad_cmp {
        return spec.to_string();
    }
    spec.split(" | ")
        .filter(|part| {
            !part
                .split_once('$')
                .is_some_and(|(k, _)| matches!(k.trim(), "IsPresent" | "PresentCompare"))
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
