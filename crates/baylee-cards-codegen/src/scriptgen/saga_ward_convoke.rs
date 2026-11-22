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

enum ZoneContinuousOutcome {
    Handled(Option<()>),
    Pass(Params),
}

enum EtbTap {
    Plain,
    Sub(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenReplaceAmt {
    Omitted,
    One,
    MatchCount,
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
            && let Some(body) = self.svars.get(svar.trim())
        {
            let trimmed = body.trim_start();
            if trimmed.starts_with("DB$ NameCard") {
                return ExtraOutcome::Handled(self.etb_name_card(svar.trim()));
            }
            if trimmed.starts_with("DB$ ChoosePlayer") {
                return ExtraOutcome::Handled(self.etb_choose_opponent(svar.trim()));
            }
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
    /// mana costs across all card types (post-suspend haste for creatures and
    /// instant/flash suspend timing are supported in the engine).
    fn suspend(&mut self, rest: &str) -> Option<()> {
        // Kept read so `scriptgen/mod.rs::Tx::has_flash` stays warning-free
        // under Wave 6 disjoint file ownership.
        let _ = self.has_flash;
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
        let effect = self.ward_effect_expr(rest, false)?;
        self.body
            .abilities
            .push(format!("triggered!(Trigger::Ward, &[{effect}])"));
        Some(())
    }

    /// Lowers the cost of `Ward:<rest>` (from `K:Ward:` or static
    /// `AddKeyword$ Ward:`) to its `Effect::PlayerMay*Or` expression.
    /// Generic integer ward (`1..=10`) is accepted only when `allow_generic`
    /// is true (granted static abilities; printed `K:Ward:<n>` uses
    /// `AbilityDef::Ward { mana }`).
    pub(super) fn ward_effect_expr(&mut self, rest: &str, allow_generic: bool) -> Option<String> {
        let rest = rest.trim();
        if rest.contains(':') {
            return self.deny(format!("ward `{rest}`"));
        }
        if allow_generic
            && let Ok(n) = rest.parse::<u32>()
            && (1..=10).contains(&n)
        {
            return Some(format!(
                "Effect::PlayerMayPayOr {{ player: PlayerRel::ControllerOfTarget, \
                 mana: Amount::Fixed({n}), effect: &Effect::CounterTargetSpellOrAbility }}"
            ));
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
        if let Some(body) = rest
            .strip_prefix("Blight<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return self.ward_blight(body, rest);
        }
        if !rest.contains('<') {
            let mana = self.parse_fixed_mana_cost(rest, "Ward")?;
            return Some(format!(
                "Effect::PlayerMayPayManaOr {{ player: PlayerRel::ControllerOfTarget, \
                 cost: mana!(\"{mana}\"), effect: &Effect::CounterTargetSpellOrAbility }}"
            ));
        }
        self.deny(format!("ward `{rest}`"))
    }

    fn ward_pay_life(&mut self, body: &str, full: &str) -> Option<String> {
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
        Some(format!(
            "Effect::PlayerMayPayLifeOr {{ player: PlayerRel::ControllerOfTarget, life: {life}, \
             effect: &Effect::CounterTargetSpellOrAbility }}"
        ))
    }

    fn ward_poison(&mut self, body: &str, full: &str) -> Option<String> {
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
        Some(format!(
            "Effect::PlayerMayGetPoisonOr {{ player: PlayerRel::ControllerOfTarget, \
             poison: Amount::Fixed({n}), effect: &Effect::CounterTargetSpellOrAbility }}"
        ))
    }

    fn ward_discard(&mut self, body: &str, full: &str) -> Option<String> {
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
        Some(format!(
            "Effect::PlayerMayPayCostOr {{ player: PlayerRel::ControllerOfTarget, \
             cost: &CostPart::Discard(&{filter}), effect: &Effect::CounterTargetSpellOrAbility }}"
        ))
    }

    fn ward_blight(&self, body: &str, full: &str) -> Option<String> {
        let mut parts = body.split('/');
        let raw = parts.next()?.trim();
        let _desc = parts.next();
        if parts.next().is_some() {
            return self.deny(format!("ward `{full}`"));
        }
        let Ok(n) = raw.parse::<u16>() else {
            return self.deny(format!("ward `{full}`"));
        };
        if n == 0 {
            return self.deny(format!("ward `{full}`"));
        }
        Some(format!(
            "Effect::PlayerMayPayCostOr {{ player: PlayerRel::ControllerOfTarget, \
             cost: &CostPart::PutCounterOnOther {{ filter: &Filter::YOUR_CREATURE, \
             kind: CounterKind::MinusOneMinusOne, n: {n} }}, \
             effect: &Effect::CounterTargetSpellOrAbility }}"
        ))
    }

    fn ward_sacrifice(&mut self, body: &str, full: &str) -> Option<String> {
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
        Some(if count == 1 {
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
        })
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
    /// `CantGainLife`, `CantSacrifice`, `CombatDamageToughness`,
    /// `AssignCombatDamageAsUnblocked`, `CantSearchLibrary`) not modeled by
    /// `statics.rs`.
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
            "CantGainLife" => ExtraOutcome::Handled(self.cant_gain_life(p)),
            "CantSacrifice" => ExtraOutcome::Handled(self.cant_sacrifice(p)),
            "CombatDamageToughness" => ExtraOutcome::Handled(self.combat_damage_toughness(p)),
            "AssignCombatDamageAsUnblocked" => {
                ExtraOutcome::Handled(self.assign_combat_damage_as_unblocked(p))
            }
            "CantSearchLibrary" => ExtraOutcome::Handled(self.cant_search_library(p)),
            "CantAttach" if !p.has("ValidCardToTarget") => {
                ExtraOutcome::Handled(self.cant_attach_static(p))
            }
            "TapPowerValue" => ExtraOutcome::Handled(self.tap_power_value_static(p)),
            "Activations" => ExtraOutcome::Handled(self.activations_boast_static(p)),
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
            "CantTarget" => self.cant_target_extra(p),
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
        let condition = self.condition_any_wave21(&mut p)?;
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
        let who = match p.take("Caster").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            _ => return self.deny("static ability `S: Mode$ CastWithFlash`".to_string()),
        };
        if !matches!(p.take("ValidSA").as_deref(), None | Some("Spell")) {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        let zone_arg = match p.take("EffectZone").as_deref() {
            None | Some("Battlefield") => None,
            Some("Command") => Some(", zone = ZoneRef::Command"),
            Some("Battlefield,Command" | "Command,Battlefield") => {
                Some(", zone = ZoneRef::BattlefieldOrCommand")
            }
            Some(_) => return self.deny("static ability `S: Mode$ CastWithFlash`".to_string()),
        };
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        };
        let mut clauses: Vec<String> = Vec::new();
        if let Some(turn) = p.take("PlayerTurn") {
            match turn.trim() {
                "You" => clauses.push("Condition::YourTurn".to_string()),
                "Opponent" => clauses.push("Condition::OpponentsTurn".to_string()),
                _ => return self.deny("static ability `S: Mode$ CastWithFlash`".to_string()),
            }
        }
        if let Some(phase) = p.take("Phases") {
            let step = match phase.trim() {
                "Upkeep" => "StepKind::Upkeep",
                "Draw" => "StepKind::Draw",
                "Main1" => "StepKind::FirstMain",
                "Main2" => "StepKind::PostcombatMain",
                "BeginCombat" => "StepKind::CombatBegin",
                "Declare Attackers" => "StepKind::DeclareAttackers",
                "Declare Blockers" => "StepKind::DeclareBlockers",
                "Combat Damage" => "StepKind::CombatDamage",
                "EndCombat" | "End of Combat" => "StepKind::CombatEnd",
                "End of Turn" => "StepKind::End",
                _ => return self.deny("static ability `S: Mode$ CastWithFlash`".to_string()),
            };
            clauses.push(format!("Condition::DuringStep({step})"));
        }
        if let Some(base) = self
            .static_conditions(&mut p, "static ability `S: Mode$ CastWithFlash`")?
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(base.to_string());
        }
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        }
        let modifier = if who == "PlayerRel::You"
            && let Some(legacy) = flash_modifier_for_valid(&valid_card)
        {
            legacy
        } else {
            let Some(filter_ref) = self.grant_flash_filter(&valid_card) else {
                return self.deny("static ability `S: Mode$ CastWithFlash`".to_string());
            };
            format!("Modifier::GrantFlash {{ who: {who}, filter: &{filter_ref} }}")
        };
        let condition = match clauses.as_slice() {
            [] => String::new(),
            [one] => format!(", condition = Some({one})"),
            many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
        };
        let mut expr = Self::static_expr("Filter::Any", &modifier);
        if (zone_arg.is_some() || !condition.is_empty())
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{}{condition})", zone_arg.unwrap_or_default());
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn grant_flash_filter(&mut self, valid_card: &str) -> Option<String> {
        let trimmed = valid_card.trim();
        if trimmed == "Card" {
            return Some("Filter::Any".to_string());
        }
        if trimmed.split(',').any(|alt| {
            alt.split(['.', '+']).any(|atom| {
                let a = atom.trim();
                a.contains("Ctrl")
                    || a.contains("Own")
                    || a.contains("Remembered")
                    || a.contains("Targeted")
                    || a.contains("Triggered")
                    || matches!(
                        a,
                        "Self"
                            | "Other"
                            | "StrictlyOther"
                            | "Equipped"
                            | "Enchanted"
                            | "Attached"
                            | "EquippedBy"
                            | "EnchantedBy"
                            | "AttachedBy"
                    )
            })
        }) {
            return None;
        }
        let expr = self.filter_expr_in_zone(trimmed, true)?;
        Some(if expr == "Filter::Any" {
            "Filter::Any".to_string()
        } else {
            self.body.filter_static("FLASH", &expr)
        })
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
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
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

    fn cant_gain_life(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantGainLife")?;
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CantGainLife`".to_string());
        }
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                "PlayerRel::EnchantedPlayer"
            }
            Some(_) => return self.deny("static ability `S: Mode$ CantGainLife`".to_string()),
        };
        let cond =
            self.static_conditions(&mut p, "unclaimed parameter `CantGainLife.CheckSVar`")?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantGainLife`".to_string());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &format!("Modifier::PlayersCantGainLife({who})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn cant_sacrifice(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantSacrifice")?;
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CantSacrifice`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ CantSacrifice`".to_string());
        };
        let opponents_only = match (
            p.take("ValidCause").as_deref(),
            p.take("ForCost").as_deref(),
        ) {
            (None, None) => false,
            (Some("SpellAbility.OppCtrl"), Some("False"))
                if valid_card
                    .split(',')
                    .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl")) =>
            {
                true
            }
            _ => return self.deny("static ability `S: Mode$ CantSacrifice`".to_string()),
        };
        let cond =
            self.static_conditions(&mut p, "unclaimed parameter `CantSacrifice.CheckSVar`")?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantSacrifice`".to_string());
        }
        let filter = self.filter_expr(&valid_card)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::CantBeSacrificed {{ opponents_only: {opponents_only} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn combat_damage_toughness(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CombatDamageToughness")?;
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self.deny("static ability `S: Mode$ CombatDamageToughness`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ CombatDamageToughness`".to_string());
        };
        let cond = self.static_conditions(
            &mut p,
            "unclaimed parameter `CombatDamageToughness.CheckSVar`",
        )?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CombatDamageToughness`".to_string());
        }
        let Ok((if_greater, effective_valid)) = strip_power_lt_toughness(&valid_card) else {
            return self.deny("static ability `S: Mode$ CombatDamageToughness`".to_string());
        };
        let filter = self.filter_expr(&effective_valid)?;
        let modifier = if if_greater {
            "Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater"
        } else {
            "Modifier::AssignCombatDamageUsingToughness"
        };
        self.body
            .abilities
            .push(Self::format_static_expr(&filter, modifier, zone, &cond));
        Some(())
    }

    fn assign_combat_damage_as_unblocked(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "AssignCombatDamageAsUnblocked")?;
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return self
                .deny("static ability `S: Mode$ AssignCombatDamageAsUnblocked`".to_string());
        }
        if p.take("Optional").as_deref() != Some("True") {
            return self
                .deny("static ability `S: Mode$ AssignCombatDamageAsUnblocked`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self
                .deny("static ability `S: Mode$ AssignCombatDamageAsUnblocked`".to_string());
        };
        let cond = self.static_conditions(
            &mut p,
            "unclaimed parameter `AssignCombatDamageAsUnblocked.CheckSVar`",
        )?;
        if !p.exhausted() {
            return self
                .deny("static ability `S: Mode$ AssignCombatDamageAsUnblocked`".to_string());
        }
        let filter = self.filter_expr(&valid_card)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::MayAssignCombatDamageAsThoughUnblocked",
            zone,
            &cond,
        ));
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

    fn activations_boast_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || p.take("ValidSA").as_deref() != Some("Activated.Boast")
            || p.take("ValidPlayer").as_deref() != Some("You")
            || p.take("MinLimit").as_deref() != Some("2")
            || !matches!(p.take("PlayerTurn").as_deref(), None | Some("You"))
        {
            return self.deny("static ability `S: Mode$ Activations`".to_string());
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ Activations`".to_string());
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ Activations`".to_string());
        }
        let filter = self.filter_expr(&valid_card)?;
        self.push_static_with_cond(&filter, "Modifier::BoastTwice", &cond);
        Some(())
    }

    fn try_extra_zone_continuous(&mut self, p: Params) -> ZoneContinuousOutcome {
        if p.peek("AffectedZone") == Some("Library") {
            if matches!(
                p.peek("EffectZone"),
                Some("Command" | "Battlefield,Command" | "Command,Battlefield")
            ) {
                return ZoneContinuousOutcome::Handled(self.continuous_library_top_command(p));
            }
            return ZoneContinuousOutcome::Handled(self.continuous_library_top_dispatch(p));
        }
        if p.peek("AffectedZone") == Some("Graveyard")
            && matches!(p.peek("EffectZone"), None | Some("Battlefield"))
            && p.peek("MayPlay") == Some("True")
        {
            return ZoneContinuousOutcome::Handled(self.continuous_graveyard_play_dispatch(p));
        }
        if p.peek("AffectedZone") == Some("Graveyard")
            && p.peek("EffectZone") == Some("Graveyard")
            && p.peek("Affected") == Some("Card.Self")
            && p.peek("MayPlay") == Some("True")
        {
            return ZoneContinuousOutcome::Handled(self.continuous_graveyard_self_play_wave14a(p));
        }
        if p.peek("AffectedZone") == Some("Graveyard")
            && matches!(
                p.peek("EffectZone"),
                None | Some(
                    "Battlefield" | "Command" | "Battlefield,Command" | "Command,Battlefield"
                )
            )
            && p.peek("AddKeyword").is_some_and(|k| {
                k.trim() == "Flashback"
                    || k.trim().starts_with("Escape:CardManaCost ExileFromGrave<")
            })
        {
            return ZoneContinuousOutcome::Handled(self.continuous_graveyard_keyword(p));
        }
        if p.peek("AffectedZone") == Some("Hand")
            && p.peek("MayLookAt") == Some("Player")
            && matches!(p.peek("Affected"), Some("Card.YouOwn" | "Card.YouCtrl"))
        {
            return ZoneContinuousOutcome::Handled(self.continuous_reveal_your_hand(p));
        }
        if p.peek("AffectedZone") == Some("Exile")
            && p.peek("MayPlay") == Some("True")
            && matches!(
                p.peek("Affected"),
                Some(
                    "Card.ExiledWithSource+ThisTurnEntered"
                        | "Card.ThisTurnEntered+ExiledWithSource"
                )
            )
        {
            return ZoneContinuousOutcome::Handled(self.continuous_draw_exile_play(p));
        }
        ZoneContinuousOutcome::Pass(p)
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
        let p = match self.try_extra_zone_continuous(p) {
            ZoneContinuousOutcome::Handled(res) => return ExtraOutcome::Handled(res),
            ZoneContinuousOutcome::Pass(p) => p,
        };
        if self.has_owner_graveyard_cda(&p) {
            return ExtraOutcome::Handled(self.characteristic_pt_graveyard_owner(p));
        }
        if p.has("AdditionalVote") || p.has("AdditionalOptionalVote") {
            return ExtraOutcome::Handled(self.continuous_additional_votes(p));
        }
        if matches!(p.peek("EffectZone"), None | Some("Battlefield"))
            && matches!(p.peek("AffectedZone"), None | Some("Battlefield"))
            && !p.has("CharacteristicDefining")
            && p.peek("SetPower") != Some("AffectedX")
            && p.peek("RemoveCardTypes") == Some("True")
            && !self.is_wave5_single_subtype_become(&p)
        {
            return ExtraOutcome::Handled(self.continuous_become_types(p));
        }
        if let Some(cond) = p.peek("Condition").and_then(Self::wave21_named_condition)
            && !p.has("ConditionDescription")
        {
            return ExtraOutcome::Handled(self.continuous_with_stripped_params(
                spec,
                &["Condition"],
                cond,
            ));
        }
        if p.peek("Affected")
            .is_some_and(is_player_continuous_affected)
            && (p.has("SetMaxHandSize")
                || p.has("NoHandSize")
                || p.has("AdjustLandPlays")
                || p.has("ControlOpponentsSearchingLibrary")
                || p.peek("AddKeyword").is_some_and(|k| {
                    k == "Hexproof"
                        || k == "CantSearchLibrary"
                        || k == "LimitSearchLibrary"
                        || k.starts_with("Protection:")
                        || k.starts_with("Protection from ")
                }))
        {
            return ExtraOutcome::Handled(self.continuous_player_rule(p));
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
        if let Some(var) = p.peek("CheckSVar") {
            let cmp = p.peek("SVarCompare").unwrap_or("GE1");
            if let Some(first) = self
                .svar_condition_clause(var, cmp)
                .or_else(|| self.promised_gift_svar_condition(var, cmp))
            {
                let cond = if let Some(var2) = p.peek("CheckSecondSVar") {
                    let cmp2 = p.peek("SecondSVarCompare").unwrap_or("GE1");
                    let Some(second) = self
                        .svar_condition_clause(var2, cmp2)
                        .or_else(|| self.promised_gift_svar_condition(var2, cmp2))
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

    fn continuous_library_top_dispatch(&mut self, p: Params) -> Option<()> {
        let legacy_look_only = p.has("MayLookAt") && !p.has("MayPlay");
        let legacy_land_only = !p.has("MayLookAt")
            && p.peek("MayPlay") == Some("True")
            && !p.has("MayPlayLimit")
            && !p.has("MayPlayWithFlash")
            && !p.has("RaiseCost")
            && !p.has("MayPlayAltManaCost")
            && p.peek("Affected").is_some_and(|a| {
                matches!(
                    a.trim(),
                    "Land.TopLibrary+YouCtrl"
                        | "Land.YouCtrl+TopLibrary"
                        | "Land.TopLibrary+YouOwn"
                        | "Land.YouOwn+TopLibrary"
                )
            });
        let legacy_citadel = !p.has("MayLookAt")
            && p.peek("MayPlay") == Some("True")
            && p.peek("MayPlayAltManaCost") == Some("PayLife<ConvertedManaCost>")
            && !p.has("MayPlayLimit")
            && !p.has("MayPlayWithFlash")
            && !p.has("RaiseCost")
            && p.peek("Affected").is_some_and(|a| {
                matches!(
                    a.trim(),
                    "Card.TopLibrary+YouCtrl"
                        | "Card.YouCtrl+TopLibrary"
                        | "Card.TopLibrary+YouOwn"
                        | "Card.YouOwn+TopLibrary"
                )
            });
        if legacy_look_only || legacy_land_only || legacy_citadel {
            return self.continuous_library_top(p);
        }
        self.continuous_library_top_wave14a(p)
    }

    fn continuous_library_top_wave14a(&mut self, mut p: Params) -> Option<()> {
        let deny = |tx: &Self| tx.deny("static ability reaching `AffectedZone$ Library`".into());
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Library")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.take("MayPlay").as_deref() != Some("True")
        {
            return deny(self);
        }
        let Some(affected) = p.take("Affected") else {
            return deny(self);
        };
        let may_look = p.take("MayLookAt");
        let with_flash = match p.take("MayPlayWithFlash").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return deny(self),
        };
        let once_per_turn = match p.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(_) => return deny(self),
        };
        let alt_cost = p.take("MayPlayAltManaCost");
        let raise_cost = p.take("RaiseCost");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }
        let price = match (alt_cost.as_deref(), raise_cost.as_deref()) {
            (None, None) => "AltCastPrice::Normal".to_string(),
            (Some("PayLife<ConvertedManaCost>"), None) => {
                "AltCastPrice::PayLifeEqualToManaValue".to_string()
            }
            (None, Some(raw)) => {
                let cost_lit = self.parse_normal_plus_cost(raw)?;
                format!("AltCastPrice::NormalPlus({cost_lit})")
            }
            _ => return deny(self),
        };
        let Some(shape) = strip_top_library_yours(&affected) else {
            return deny(self);
        };
        let is_unfiltered_card = shape.cleaned == "Card";
        if let Some(look_who) = may_look {
            // Only emit unconditional look/reveal when `Affected$` covers any top card.
            if !is_unfiltered_card {
                return deny(self);
            }
            let look_mod = match look_who.trim() {
                "You" => "Modifier::MayLookAtTopOfLibrary { who: PlayerRel::You }",
                "Player" => "Modifier::RevealLibraryTop",
                _ => return deny(self),
            };
            self.push_static_with_cond("Filter::Any", look_mod, &cond);
        }
        let filter = if shape.cleaned == "Card" {
            "Filter::Any".to_string()
        } else {
            let Some(expr) = self.filter_expr_in_zone(&shape.cleaned, true) else {
                return deny(self);
            };
            expr
        };
        let allow_lands = self.zone_play_allows_lands(&shape, alt_cost.is_some());
        let modifier = format!(
            "Modifier::MayCastWithAltCost {{ who: PlayerRel::You, \
             zones: AltCastZones::TopOfLibrary, grant_zone_permission: true, \
             allow_lands: {allow_lands}, with_flash: {with_flash}, \
             max_mana_value: None, once_per_turn: {once_per_turn}, price: {price} }}"
        );
        self.push_static_with_cond(&filter, &modifier, &cond);
        Some(())
    }

    fn continuous_graveyard_play_dispatch(&mut self, p: Params) -> Option<()> {
        let has_wave14a_param =
            p.has("RaiseCost") || p.has("MayPlayWithFlash") || p.has("ValidAfterStack");
        if !has_wave14a_param {
            let is_legacy_affected = p.peek("Affected").is_some_and(|a| {
                matches!(
                    a.trim(),
                    "Land.YouOwn"
                        | "Land.YouCtrl"
                        | "Card.YouOwn"
                        | "Card.YouCtrl"
                        | "Permanent.nonLand+YouOwn"
                        | "Permanent.YouOwn+nonLand"
                        | "Permanent.nonLand+YouCtrl"
                        | "Permanent.YouCtrl+nonLand"
                )
            });
            let is_legacy_once =
                p.peek("MayPlayLimit") == Some("1") && p.peek("ReplaceGraveyard") == Some("Exile");
            if (is_legacy_affected && !p.has("MayPlayLimit")) || is_legacy_once {
                return self.continuous_graveyard_play(p);
            }
        }
        self.continuous_graveyard_play_wave14a(p)
    }

    fn continuous_graveyard_play_wave14a(&mut self, mut p: Params) -> Option<()> {
        let deny = |tx: &Self| tx.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        let desc_has_unmodeled_rider = p.peek("Description").is_some_and(|d| {
            let lower = d.to_ascii_lowercase();
            lower.contains("finality counter") || lower.contains("exile it instead")
        });
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.take("MayPlay").as_deref() != Some("True")
            || desc_has_unmodeled_rider
        {
            return deny(self);
        }
        let Some(affected) = p.take("Affected") else {
            return deny(self);
        };
        let with_flash = match p.take("MayPlayWithFlash").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return deny(self),
        };
        let once_per_turn = match p.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(_) => return deny(self),
        };
        let raise_cost = p.take("RaiseCost");
        let valid_after_stack = p.take("ValidAfterStack");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }
        if let Some(vas) = valid_after_stack.as_deref()
            && !valid_after_stack_is_redundant(vas, &affected)
        {
            return deny(self);
        }
        let Some(shape) = strip_graveyard_yours(&affected) else {
            return deny(self);
        };
        // Filtered graveyard land permission (`Modifier::PlayLandsFromGraveyard` with filter).
        // Land-only grants with `with_flash`, `once_per_turn`, or `raise_cost` stay refused
        // because land drops do not go through spell-cast pricing/timing.
        if !shape.has_nonland_all && self.all_alts_are_land_only(&shape.cleaned) {
            if with_flash || once_per_turn || raise_cost.is_some() {
                return deny(self);
            }
            let Some(filter) = self.filter_expr_in_zone(&shape.cleaned, true) else {
                return deny(self);
            };
            self.push_static_with_cond(&filter, "Modifier::PlayLandsFromGraveyard", &cond);
            return Some(());
        }
        // `Instant,Sorcery` once-per-turn graveyard grants without `ReplaceGraveyard$ Exile`
        // (Kess without its exile replacement) stay refused; real Instant/Sorcery GY cards
        // either carry `ReplaceGraveyard$ Exile` (legacy Kess) or `RaiseCost$` (Festival of Embers).
        if once_per_turn
            && raise_cost.is_none()
            && matches!(
                shape.cleaned.as_str(),
                "Instant,Sorcery" | "Sorcery,Instant"
            )
        {
            return deny(self);
        }
        let price = match raise_cost.as_deref() {
            None => "AltCastPrice::Normal".to_string(),
            Some(raw) => {
                let cost_lit = self.parse_normal_plus_cost(raw)?;
                format!("AltCastPrice::NormalPlus({cost_lit})")
            }
        };
        let Some(base_filter) = self.filter_expr_in_zone(&shape.cleaned, true) else {
            return deny(self);
        };
        // `AltCastZones::Zones(&[ZoneRef::Graveyard])` matches graveyard zone without
        // checking zone owner; always scope to `Filter::OwnedByYou`.
        let owned_filter = if base_filter == "Filter::Any" {
            "Filter::OwnedByYou".to_string()
        } else {
            format!("Filter::And(&[{base_filter}, Filter::OwnedByYou])")
        };
        let allow_lands = self.zone_play_allows_lands(&shape, false);
        let modifier = format!(
            "Modifier::MayCastWithAltCost {{ who: PlayerRel::You, \
             zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), grant_zone_permission: true, \
             allow_lands: {allow_lands}, with_flash: {with_flash}, \
             max_mana_value: None, once_per_turn: {once_per_turn}, price: {price} }}"
        );
        self.push_static_with_cond(&owned_filter, &modifier, &cond);
        Some(())
    }

    fn continuous_graveyard_self_play_wave14a(&mut self, mut p: Params) -> Option<()> {
        let is_land = self.type_line.split_whitespace().any(|w| w == "Land");
        let has_extra = p.has("RaiseCost") || p.has("MayPlayWithFlash") || p.has("MayPlayLimit");
        if !is_land && !has_extra {
            return self.continuous_graveyard_self_play(p);
        }
        let deny = |tx: &Self| tx.deny("static ability reaching `AffectedZone$ Graveyard`".into());
        let desc_mentions_finality = p
            .peek("Description")
            .is_some_and(|d| d.to_ascii_lowercase().contains("finality counter"));
        p.drop_prose();
        if p.take("AffectedZone").as_deref() != Some("Graveyard")
            || p.take("EffectZone").as_deref() != Some("Graveyard")
            || p.take("Affected").as_deref() != Some("Card.Self")
            || p.take("MayPlay").as_deref() != Some("True")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || desc_mentions_finality
        {
            return deny(self);
        }
        let with_flash = match p.take("MayPlayWithFlash").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return deny(self),
        };
        let once_per_turn = match p.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(_) => return deny(self),
        };
        let raise_cost = p.take("RaiseCost");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }
        if is_land {
            if with_flash || once_per_turn || raise_cost.is_some() {
                return deny(self);
            }
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::PlayLandsFromGraveyard, \
                 zone = ZoneRef::Graveyard{cond})"
            ));
            return Some(());
        }
        let price = match raise_cost.as_deref() {
            None => "AltCastPrice::Normal".to_string(),
            Some(raw) => {
                let cost_lit = self.parse_normal_plus_cost(raw)?;
                format!("AltCastPrice::NormalPlus({cost_lit})")
            }
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::MayCastWithAltCost {{ \
             who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), \
             grant_zone_permission: true, allow_lands: false, with_flash: {with_flash}, \
             max_mana_value: None, once_per_turn: {once_per_turn}, price: {price} }}, \
             zone = ZoneRef::Graveyard{cond})"
        ));
        Some(())
    }

    fn parse_normal_plus_cost(&mut self, raw: &str) -> Option<String> {
        let saved = self.unclaimed.borrow().clone();
        let res = self.parse_kicker_cost_expr(raw);
        let Some((expr, uses_x)) = res else {
            *self.unclaimed.borrow_mut() = saved;
            return None;
        };
        if uses_x {
            *self.unclaimed.borrow_mut() = saved;
            return None;
        }
        Some(expr)
    }

    fn all_alts_are_land_only(&self, cleaned: &str) -> bool {
        let mut saw = false;
        for alt in cleaned.split(',') {
            let base = alt.trim().split('.').next().unwrap_or("").trim();
            if base.is_empty() {
                return false;
            }
            let is_land = base == "Land"
                || self
                    .cats
                    .const_path_of(baylee_core::types::SubtypeKind::Land, base)
                    .is_some();
            if !is_land {
                return false;
            }
            saw = true;
        }
        saw
    }

    fn zone_play_allows_lands(&self, shape: &ZonePlayShape, force_allow: bool) -> bool {
        if force_allow || shape.any_land_base {
            return true;
        }
        if shape.has_nonland_all || shape.all_nonland_bases {
            return false;
        }
        let lower = self.oracle.to_ascii_lowercase();
        !lower.contains("cast ") || lower.contains("play ")
    }

    fn continuous_library_top_command(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone_arg = match (
            p.take("AffectedZone").as_deref(),
            p.take("EffectZone").as_deref(),
            p.take("Secondary").as_deref(),
        ) {
            (Some("Library"), Some("Command"), None | Some("True")) => ", zone = ZoneRef::Command",
            (
                Some("Library"),
                Some("Battlefield,Command" | "Command,Battlefield"),
                None | Some("True"),
            ) => ", zone = ZoneRef::BattlefieldOrCommand",
            _ => return self.deny("static ability reaching `AffectedZone$ Library`".into()),
        };
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Library`".into());
        };
        let may_look = p.take("MayLookAt");
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Library`".into());
        }
        let is_yours_card = matches!(
            affected.trim(),
            "Card.TopLibrary+YouCtrl"
                | "Card.YouCtrl+TopLibrary"
                | "Card.TopLibrary+YouOwn"
                | "Card.YouOwn+TopLibrary"
        );
        let modifier = match (may_look.as_deref(), affected.trim()) {
            (Some("You"), _) if is_yours_card => "Modifier::LookAtLibraryTop",
            (Some("Player"), _) if is_yours_card => "Modifier::RevealLibraryTop",
            (Some("Player"), "Card.TopLibrary") => "Modifier::RevealAllLibraryTops",
            _ => return self.deny("static ability reaching `AffectedZone$ Library`".into()),
        };
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, {modifier}{zone_arg}{cond})"
        ));
        Some(())
    }

    fn condition_any_wave21(&mut self, p: &mut Params) -> Option<String> {
        let extra = p.peek("Condition").and_then(Self::wave21_named_condition);
        if extra.is_some() {
            p.take("Condition");
        }
        let base = self.condition_any(p)?;
        let base_clause = base
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'));
        match (extra, base_clause) {
            (None, _) => Some(base),
            (Some(e), None) => Some(format!(", condition = Some({e})")),
            (Some(e), Some(b)) => Some(format!(", condition = Some(Condition::All(&[{e}, {b}]))")),
        }
    }

    fn continuous_additional_votes(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || p.has("AffectedZone")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("`Continuous.AdditionalVote` outside battlefield".to_string());
        }
        if p.take("Affected").as_deref() != Some("You") {
            return self.deny("`Continuous.AdditionalVote` not affecting `You`".to_string());
        }
        let mut votes = Vec::new();
        for (key, optional) in [("AdditionalVote", false), ("AdditionalOptionalVote", true)] {
            if let Some(raw) = p.take(key) {
                let Ok(count) = raw.trim().parse::<u8>() else {
                    return self.deny(format!("`Continuous.{key}` `{raw}`"));
                };
                if count == 0 {
                    return self.deny(format!("`Continuous.{key}$ 0`"));
                }
                votes.push((count, optional));
            }
        }
        let cond = self.condition_any_wave21(&mut p)?;
        if !p.exhausted() || votes.is_empty() {
            return self.deny("unsupported `Continuous.AdditionalVote`".to_string());
        }
        for (count, optional) in votes {
            self.push_static_with_cond(
                "Filter::This",
                &format!("Modifier::AdditionalVotes {{ count: {count}, optional: {optional} }}"),
                &cond,
            );
        }
        Some(())
    }

    fn is_wave5_single_subtype_become(&self, p: &Params) -> bool {
        if p.has("RemoveSubTypes")
            || p.has("RemoveEnchantmentTypes")
            || p.has("RemoveLandTypes")
            || p.has("KeepSupertypes")
        {
            return false;
        }
        let Some(raw) = p.peek("AddType") else {
            return false;
        };
        let tokens: Vec<&str> = raw
            .split('&')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if tokens.len() != 2 {
            return false;
        }
        if tokens.contains(&"Creature") && p.peek("RemoveCreatureTypes") == Some("True") {
            return tokens.iter().any(|&w| {
                w != "Creature"
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Creature, w)
                        .is_some()
            });
        }
        if tokens.contains(&"Artifact") && p.peek("RemoveArtifactTypes") == Some("True") {
            return tokens.iter().any(|&w| {
                w != "Artifact"
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Artifact, w)
                        .is_some()
            });
        }
        false
    }

    fn continuous_become_types(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let affected = self.continuous_affected(&mut p)?;
        let filter = self.filter_expr(&affected)?;
        let cond = self.condition_any_wave21(&mut p)?;
        if p.take("RemoveCardTypes").as_deref() != Some("True") {
            return self.deny("`Continuous.RemoveCardTypes` not `True`".to_string());
        }
        let add_types = p.take("AddType").unwrap_or_default();
        let mut out: Vec<String> = self
            .become_types_modifiers(&mut p, &add_types, &['&', ','])?
            .into_iter()
            .map(|m| Self::static_expr(&filter, &m))
            .collect();
        if !p.has("AddKeyword")
            && !p.has("AddAbility")
            && !p.has("AddTrigger")
            && !p.has("AddStaticAbility")
            && let Some(val) = p.take("RemoveAllAbilities")
        {
            if val != "True" {
                return self.deny("`Continuous.RemoveAllAbilities` not `True`".to_string());
            }
            out.push(Self::static_expr(&filter, "Modifier::LoseAllAbilities"));
        }
        if p.peek("SetColor") == Some("Colorless") {
            p.take("SetColor");
            out.push(Self::static_expr(
                &filter,
                "Modifier::SetColor(ColorSet::EMPTY)",
            ));
        }
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        let mut self_granted = false;
        self.grant_modifiers(&mut p, &filter, false, &cond, &mut out, &mut self_granted)?;
        let _ = self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() || out.is_empty() {
            return self.deny("unsupported `Continuous.RemoveCardTypes`".to_string());
        }
        if !cond.is_empty() {
            for ability in &mut out {
                let open = ability.strip_suffix(')')?;
                *ability = format!("{open}{cond})");
            }
        }
        self.body.abilities.extend(out);
        Some(())
    }

    fn continuous_with_extra_condition(&mut self, spec: &str, cond: &str) -> Option<()> {
        self.continuous_with_stripped_params(
            spec,
            &[
                "CheckSVar",
                "SVarCompare",
                "CheckSecondSVar",
                "SecondSVarCompare",
            ],
            cond,
        )
    }

    fn continuous_with_stripped_params(
        &mut self,
        spec: &str,
        strip_keys: &[&str],
        cond: &str,
    ) -> Option<()> {
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| {
                let key = part.split_once('$').map_or("", |(k, _)| k.trim());
                !strip_keys.contains(&key)
            })
            .collect();
        let clean_spec = stripped.join(" | ");
        let start = self.body.abilities.len();
        let res = match self.try_extra_static(&clean_spec) {
            ExtraOutcome::Handled(res) => res,
            ExtraOutcome::Pass => self.static_ability(&clean_spec),
        };
        if res.is_none() || self.body.abilities.len() == start {
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

    /// Resolves a `CheckSVar$` value whether it names an `SVar` or is written
    /// inline as `Count$...` / `PlayerCount...`.
    pub(super) fn resolve_svar_or_inline(&self, var: &str) -> Option<String> {
        let trimmed = var.trim();
        if trimmed.starts_with("Count$") || trimmed.starts_with("PlayerCount") {
            return Some(trimmed.to_string());
        }
        self.svars.get(trimmed).map(|s| s.trim().to_string())
    }

    /// Additional `CheckSVar$` + `SVarCompare$` conditions mapped 1:1 to existing
    /// `baylee-cards-dsl::Condition` variants not yet handled by `activated::svar_condition`.
    pub(super) fn extra_svar_condition_clause(&mut self, var: &str, cmp: &str) -> Option<String> {
        self.extra_svar_condition(var, cmp)
            .or_else(|| self.extra_svar_condition_wave5(var, cmp))
            .or_else(|| self.extra_svar_amount_condition(var, cmp))
    }

    /// General `CheckSVar$` fallback over any static-safe `Amount` via
    /// `Condition::AmountAtLeast` / `Condition::AmountIsEven`.
    fn extra_svar_amount_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.resolve_svar_or_inline(var)?;
        let cmp = cmp.trim();
        if let Some(lhs) = raw.strip_suffix("/Mod.2") {
            let amt = self.static_count_expr(lhs.trim())?;
            return match cmp {
                "EQ0" => Some(format!("Condition::AmountIsEven({amt})")),
                "EQ1" | "GE1" | "GT0" | "NE0" => {
                    Some(format!("Condition::Not(&Condition::AmountIsEven({amt}))"))
                }
                _ => None,
            };
        }
        let amt = self.static_count_expr(&raw)?;
        if let Some(min) = parse_at_least_u16(cmp) {
            return (min > 0).then(|| format!("Condition::AmountAtLeast({amt}, {min})"));
        }
        if let Some(max) = parse_at_most_u16(cmp) {
            let bound = u32::from(max).checked_add(1)?;
            return Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({amt}, {bound}))"
            ));
        }
        if let Some(eq_str) = cmp.strip_prefix("EQ")
            && let Ok(n) = eq_str.parse::<u32>()
        {
            return Some(if n == 0 {
                format!("Condition::Not(&Condition::AmountAtLeast({amt}, 1))")
            } else {
                let next = n.checked_add(1)?;
                format!(
                    "Condition::All(&[Condition::AmountAtLeast({amt}, {n}), \
                     Condition::Not(&Condition::AmountAtLeast({amt}, {next}))])"
                )
            });
        }
        if let Some(ne_str) = cmp.strip_prefix("NE")
            && let Ok(n) = ne_str.parse::<u32>()
        {
            return Some(format!(
                "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::NotEqual, \
                 right: &Amount::Fixed({n}) }}"
            ));
        }
        if let Some((op, rhs_var)) = cmp.split_at_checked(2)
            && !rhs_var.is_empty()
            && !rhs_var.chars().all(|c| c.is_ascii_digit())
            && let Some(rhs_def) = self.svars.get(rhs_var.trim()).cloned()
        {
            let rhs = self.static_count_expr(&rhs_def)?;
            let gt = format!(
                "Condition::AmountAtLeast(Amount::Difference {{ left: &{amt}, right: &{rhs} }}, 1)"
            );
            let lt = format!(
                "Condition::AmountAtLeast(Amount::Difference {{ left: &{rhs}, right: &{amt} }}, 1)"
            );
            return match op {
                "GT" => Some(gt),
                "GE" => Some(format!("Condition::Not(&{lt})")),
                "LT" => Some(lt),
                "LE" => Some(format!("Condition::Not(&{gt})")),
                "EQ" => Some(format!(
                    "Condition::All(&[Condition::Not(&{lt}), Condition::Not(&{gt})])"
                )),
                "NE" => Some(format!(
                    "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::NotEqual, \
                     right: &{rhs} }}"
                )),
                _ => None,
            };
        }
        None
    }

    fn extra_svar_condition_wave5(&self, var: &str, cmp: &str) -> Option<String> {
        let raw_owned = self.resolve_svar_or_inline(var)?;
        let raw = raw_owned.as_str();
        let cmp = cmp.trim();
        if raw == "Count$ThisTurnCast_Card.YouCtrl" {
            return match cmp {
                "EQ0" | "LE0" | "LT1" => Some("Condition::YouCastNoSpellThisTurn".to_string()),
                "GE1" | "GT0" => {
                    Some("Condition::Not(&Condition::YouCastNoSpellThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if let Some(cond) = Self::extra_svar_condition_wave26g(raw, cmp) {
            return Some(cond);
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
        if raw == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!("Condition::CreaturesDiedThisTurnAtLeast({n})"));
            }
            if let Some(max) = parse_at_most_u16(cmp) {
                let bound = u32::from(max).checked_add(1)?;
                return Some(format!(
                    "Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast({bound}))"
                ));
            }
        }
        if raw == "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!(
                    "Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, {n})"
                ));
            }
            if let Some(max) = parse_at_most_u16(cmp) {
                let bound = u32::from(max).checked_add(1)?;
                return Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, {bound}))"
                ));
            }
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
        Self::extra_svar_condition_wave7(raw, cmp)
    }

    fn extra_svar_condition_wave26g(raw: &str, cmp: &str) -> Option<String> {
        let (cond, inverted) = match raw {
            "Count$Morbid.1.0" => ("Condition::CreaturesDiedThisTurnAtLeast(1)", false),
            "Count$Morbid.0.1" => ("Condition::CreaturesDiedThisTurnAtLeast(1)", true),
            "Count$wasCastFromYourHandByYou.1.0" => (
                "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceMatches(&Filter::CastFromHand)])",
                false,
            ),
            "Count$Raid.1.0" | "Count$AttackedThisTurn.1.0" => {
                ("Condition::AttackedWithAtLeast(1)", false)
            }
            "Count$Revolt.1.0" => ("Condition::Revolt", false),
            "Count$Threshold.1.0" => ("Condition::GraveyardCountAtLeast(7)", false),
            "Count$Delirium.1.0" => ("Condition::GraveyardTypesAtLeast(4)", false),
            "PlayerCountOpponents$HasPropertyIsCorrupted" => {
                ("Condition::OpponentPoisonAtLeast(3)", false)
            }
            "PlayerCountOpponents$HasPropertywasDealtDamageThisTurn"
            | "PlayerCountRegisteredOpponents$HasPropertywasDealtDamageThisTurn" => {
                ("Condition::OpponentDealtDamageThisTurn", false)
            }
            _ => return None,
        };
        let holds = match cmp {
            "GE1" | "GT0" | "EQ1" => true,
            "EQ0" | "LT1" | "LE0" => false,
            _ => return None,
        };
        if holds ^ inverted {
            Some(cond.to_string())
        } else {
            Some(format!("Condition::Not(&{cond})"))
        }
    }

    fn extra_svar_condition_wave7(raw: &str, cmp: &str) -> Option<String> {
        if matches!(
            raw,
            "Count$TotalPower Creature.YouCtrl" | "Count$Valid Creature.YouCtrl$CardPower"
        ) && let Some(n) = parse_at_least_u16(cmp)
        {
            return Some(format!("Condition::ControlledCreaturePowerAtLeast({n})"));
        }
        if raw == "Count$AttackersDeclared" {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!("Condition::AttackedWithAtLeast({n})"));
            }
            if matches!(cmp, "EQ0" | "LT1") {
                return Some("Condition::Not(&Condition::AttackedWithAtLeast(1))".to_string());
            }
        }
        if raw == "Count$Kicked.1.0" {
            return match cmp {
                "GE1" | "GT0" | "EQ1" => {
                    Some("Condition::AmountAtLeast(Amount::TimesKicked, 1)".to_string())
                }
                "EQ0" | "LT1" => Some(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::TimesKicked, 1))".to_string(),
                ),
                _ => None,
            };
        }
        if raw == "Count$CardCounters.ALL" {
            match cmp {
                "GE1" | "GT0" => {
                    return Some("Condition::SourceMatches(&Filter::HasAnyCounter)".to_string());
                }
                "EQ0" | "LT1" | "LE0" => {
                    return Some(
                        "Condition::Not(&Condition::SourceMatches(&Filter::HasAnyCounter))"
                            .to_string(),
                    );
                }
                _ => {}
            }
            if let Some(n) = parse_at_least_u16(cmp).filter(|&n| n > 0) {
                return Some(format!(
                    "Condition::SourceMatches(&Filter::AnyCountersAtLeast({n}))"
                ));
            }
            if let Some(max) = parse_at_most_u16(cmp)
                && let Some(above) = max.checked_add(1)
            {
                return Some(format!(
                    "Condition::Not(&Condition::SourceMatches(&Filter::AnyCountersAtLeast({above})))"
                ));
            }
            if let Some(eq_str) = cmp.strip_prefix("EQ")
                && let Ok(n) = eq_str.parse::<u16>()
                && n > 0
                && let Some(above) = n.checked_add(1)
            {
                return Some(format!(
                    "Condition::All(&[Condition::SourceMatches(&Filter::AnyCountersAtLeast({n})), \
                     Condition::Not(&Condition::SourceMatches(&Filter::AnyCountersAtLeast({above})))])"
                ));
            }
            return None;
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

    fn extra_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.resolve_svar_or_inline(var)?;
        let cmp = cmp.trim();
        if raw == "PlayerCountOpponents$HighestCounters.Poison" {
            let n = parse_at_least_u16(cmp)?;
            return Some(format!("Condition::OpponentPoisonAtLeast({n})"));
        }
        if raw == "Count$TotalOppPoisonCounters" {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::OpponentPoisonAtLeast(1)".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::OpponentPoisonAtLeast(1))".to_string())
                }
                _ => None,
            };
        }
        if raw == "PlayerCountPropertyYou$HasPropertyActive" {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::YourTurn".to_string()),
                "EQ0" | "NE1" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::YourTurn)".to_string())
                }
                _ => None,
            };
        }
        if raw == "PlayerCountPropertyYou$HasPropertyMaxSpeed" {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::MaxSpeed".to_string()),
                "EQ0" | "NE1" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::MaxSpeed)".to_string())
                }
                _ => None,
            };
        }
        if matches!(
            raw.as_str(),
            "Count$ValidHand Card.All" | "Count$ValidHand Card"
        ) && cmp == "EQ0"
        {
            return Some("Condition::EveryHandEmpty".to_string());
        }
        if raw == "Count$YourLifeTotal"
            && let Some(rhs) = cmp.strip_prefix("LT")
            && matches!(
                self.svars.get(rhs.trim()).map(|s| s.trim()),
                Some("Count$StartingLife" | "Count$YourStartingLife")
            )
        {
            return Some("Condition::LifeBelowStarting".to_string());
        }
        if matches!(
            raw.as_str(),
            "Count$TypeInYourYard" | "Count$ValidGraveyard Card.YouOwn$CardTypes"
        ) {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            return Some(format!("Condition::GraveyardTypesAtLeast({n})"));
        }
        if raw == "Count$ValidGraveyard Card.YouOwn$CardTypesPermanent" {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            return Some(format!(
                "Condition::GraveyardTypesAmongAtLeast {{ \
                 types: TypeSet::ARTIFACT.union(TypeSet::BATTLE).union(TypeSet::CREATURE)\
                 .union(TypeSet::ENCHANTMENT).union(TypeSet::LAND).union(TypeSet::PLANESWALKER), \
                 minimum: {n} }}"
            ));
        }
        if let Some(valid) = raw
            .strip_prefix("Count$Valid ")
            .and_then(|s| s.strip_suffix("$DifferentCardNames"))
            && valid
                .split(',')
                .all(|alt| alt.split(['.', '+']).any(|a| a.trim() == "YouCtrl"))
        {
            let n = u8::try_from(parse_at_least_u16(cmp)?).ok()?;
            let expr = self.filter_expr(valid)?;
            let name = self.body.filter_static("CHECK", &expr);
            return Some(format!("Condition::ControlDistinctNames(&{name}, {n})"));
        }
        if let Some(cond) = Self::extra_svar_condition_tail(&raw, cmp) {
            return Some(cond);
        }
        self.extra_svar_condition_wave8(var, cmp)
    }

    fn highest_cards_in_gy_condition(cmp: &str) -> Option<String> {
        if let Some(n) = parse_at_least_u16(cmp)
            && n >= 1
        {
            return Some(format!("Condition::AnyGraveyardCountAtLeast({n})"));
        }
        if let Some(max) = parse_at_most_u16(cmp)
            && let Some(min) = max.checked_add(1)
        {
            return Some(format!(
                "Condition::Not(&Condition::AnyGraveyardCountAtLeast({min}))"
            ));
        }
        None
    }

    fn extra_svar_condition_tail(raw: &str, cmp: &str) -> Option<String> {
        if raw == "PlayerCountPlayers$HighestCardsInGraveyard" {
            return Self::highest_cards_in_gy_condition(cmp);
        }
        if matches!(
            raw,
            "Count$ValidGraveyard Card.OppOwn" | "PlayerCountOpponents$HighestCardsInGraveyard"
        ) {
            if let Some(n) = parse_at_least_u16(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::OpponentGraveyardCountAtLeast({n})"));
            }
            if let Some(max) = parse_at_most_u16(cmp)
                && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::OpponentGraveyardCountAtLeast({min}))"
                ));
            }
            return None;
        }
        if raw == "PlayerCount$LowestCardsInHand" {
            let n = u8::try_from(parse_at_most_u16(cmp)?).ok()?;
            return Some(format!("Condition::APlayerHandSizeAtMost({n})"));
        }
        if matches!(
            raw,
            "Count$LandsPlayed" | "PlayerCountPropertyYou$LandsPlayed"
        ) {
            if let Some(n) = parse_at_least_u16(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::LandsPlayedThisTurnAtLeast({n})"));
            }
            if let Some(max) = parse_at_most_u16(cmp)
                && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::LandsPlayedThisTurnAtLeast({min}))"
                ));
            }
            return None;
        }
        if matches!(
            raw,
            "Count$OpponentsLifeLostThisTurn" | "Count$LifeOppsLostThisTurn"
        ) {
            if let Some(n) = parse_at_least_u16(cmp) {
                return Some(format!(
                    "Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, {n})"
                ));
            }
            if let Some(max) = parse_at_most_u16(cmp) {
                if max == 0 {
                    return Some(
                        "Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string(),
                    );
                }
                let bound = u32::from(max).checked_add(1)?;
                return Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, {bound}))"
                ));
            }
            return None;
        }
        if matches!(
            raw,
            "Count$ThisTurnEntered_Graveyard_from_Battlefield_Permanent.YouOwn+!token"
                | "Count$ThisTurnEntered_Graveyard_from_Any_Permanent.YouOwn+!token"
                | "Count$ThisTurnEntered_Graveyard_Permanent.YouOwn+!token"
        ) {
            return match cmp {
                "GE1" | "GT0" => Some("Condition::DescendedThisTurn".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::DescendedThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if raw == "Count$CardCounters.ALL/Mod.2" {
            return match cmp {
                "EQ0" => Some("Condition::EvenCountersOnSelf".to_string()),
                "EQ1" | "GE1" | "GT0" | "NE0" => {
                    Some("Condition::Not(&Condition::EvenCountersOnSelf)".to_string())
                }
                _ => None,
            };
        }
        if raw == "Count$ValidExile Card.ExiledWithSource" {
            return Self::linked_exile_count_condition(cmp);
        }
        Self::wave21_svar_condition(raw, cmp)
    }

    fn wave21_svar_condition(raw: &str, cmp: &str) -> Option<String> {
        let (cond, allow_eq1, invert) = match raw {
            "Count$Monarch" | "Count$Monarch.1.0" => {
                ("Condition::IsMonarch(PlayerRel::You)", true, false)
            }
            "Count$Monarch.0.1" => ("Condition::IsMonarch(PlayerRel::You)", true, true),
            "Count$Initiative"
            | "Count$Initiative.1.0"
            | "Count$HasInitiative"
            | "Count$HasInitiative.1.0" => {
                ("Condition::HasInitiative(PlayerRel::You)", true, false)
            }
            "Count$Initiative.0.1" | "Count$HasInitiative.0.1" => {
                ("Condition::HasInitiative(PlayerRel::You)", true, true)
            }
            "DungeonsCompleted$Amount" => {
                ("Condition::CompletedADungeon(PlayerRel::You)", false, false)
            }
            _ => return None,
        };
        let holds = match cmp {
            "GE1" | "GT0" => true,
            "EQ1" if allow_eq1 => true,
            "EQ0" | "LT1" | "LE0" => false,
            _ => return None,
        };
        Some(if holds ^ invert {
            cond.to_string()
        } else {
            format!("Condition::Not(&{cond})")
        })
    }

    fn linked_exile_count_condition(cmp: &str) -> Option<String> {
        match super::count_bound(cmp)? {
            super::Bound::AtLeast(n) if n >= 1 => Some(format!(
                "Condition::AmountAtLeast(Amount::CountOf {{ \
                 filter: &Filter::Any, zone: ZoneSel::LinkedExile }}, {n})"
            )),
            super::Bound::AtMost(n) => {
                let threshold = u32::from(n).saturating_add(1);
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast(Amount::CountOf {{ \
                     filter: &Filter::Any, zone: ZoneSel::LinkedExile }}, {threshold}))"
                ))
            }
            super::Bound::AtLeast(_) => None,
        }
    }

    /// Additional `R:` replacement handlers (`BeginPhase`, `BeginTurn`, `Draw`,
    /// `CreateToken`, `AddCounter`, `GainLife`, `LifeReduced`, and extended `Moved`).
    pub(super) fn try_extra_replacement(&mut self, spec: &str) -> ExtraOutcome {
        let Some((event, p)) = Params::parse(spec) else {
            return ExtraOutcome::Pass;
        };
        match event.as_str() {
            "BeginPhase" => self.replace_begin_phase_draw(p),
            "BeginTurn" => ExtraOutcome::Handled(self.replace_begin_turn(p)),
            "Draw" => match self.replace_draw_wave11a(p) {
                ZoneContinuousOutcome::Handled(res) => ExtraOutcome::Handled(res),
                ZoneContinuousOutcome::Pass(p) => ExtraOutcome::Handled(self.replace_draw_extra(p)),
            },
            "CreateToken" => ExtraOutcome::Handled(self.replace_create_token(p)),
            "AddCounter" => ExtraOutcome::Handled(self.replace_add_counter(p)),
            "GainLife" => ExtraOutcome::Handled(self.replace_gain_life(p)),
            "LifeReduced" => ExtraOutcome::Handled(self.replace_life_reduced(p)),
            "RollDice" => ExtraOutcome::Handled(self.replace_roll_dice(p)),
            "Explore" => ExtraOutcome::Handled(self.replace_explore(p)),
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
                Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                    "PlayerRel::EnchantedPlayer"
                }
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
        // Discard-to-library replacement (`ReplacementRule::MayDiscardToLibraryTop` /
        // `ReplacementRule::MayRevealOpponentDiscardToLibraryTop`).
        if p.peek("Discard") == Some("True") {
            return ExtraOutcome::Handled(self.moved_discard_to_library(p));
        }
        if valid_card == Some("Land.OppCtrl+ControllerControlsAtLeastAsManyLand")
            && dest == Some("Battlefield")
        {
            return ExtraOutcome::Handled(self.moved_land_equilibrium(p));
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
        // Self-ETB replacements gated on kicker (`Card.Self+kicked` / `+kicked 1|2`).
        if dest == Some("Battlefield")
            && valid_card
                .is_some_and(|v| v == "Card.Self+kicked" || v.starts_with("Card.Self+kicked "))
        {
            return ExtraOutcome::Handled(self.moved_self_kicked_etb(p));
        }
        // Self-ETB replacements whose `ReplaceWith$` is NOT a plain `DB$ Tap`.
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
                if matches!(api.as_str(), "Discard" | "Sacrifice") {
                    return ExtraOutcome::Handled(self.moved_pay_before_etb(p));
                }
                if api == "SacrificeAll" {
                    return ExtraOutcome::Handled(self.moved_sacrifice_others(p));
                }
                if api == "GenericChoice" && p.peek("Layer") == Some("Copy") {
                    return ExtraOutcome::Handled(self.moved_choose_shape(p));
                }
                if api == "Tap" {
                    if let ExtraOutcome::Handled(out) =
                        self.moved_tapped_unless_condition(p.clone())
                    {
                        return ExtraOutcome::Handled(out);
                    }
                    return self.moved_self_tapped_with_counters(p);
                }
                if api == "PutCounter" {
                    return self.moved_self_tapped_with_counters(p);
                }
                if api == "AlterAttribute" {
                    return ExtraOutcome::Handled(self.moved_self_prepared(p));
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

    fn parse_etb_tap_params(mut tap: Params) -> Option<EtbTap> {
        tap.drop_prose();
        if !matches!(
            tap.take("Defined").as_deref(),
            None | Some("Self" | "ReplacedCard")
        ) || !matches!(tap.take("ETB").as_deref(), None | Some("True"))
        {
            return None;
        }
        let sub = tap.take("SubAbility");
        if !tap.exhausted() {
            return None;
        }
        Some(sub.map_or(EtbTap::Plain, EtbTap::Sub))
    }

    fn parse_etb_put_counter(
        &mut self,
        mut put: Params,
    ) -> Option<(String, String, Option<String>)> {
        put.drop_prose();
        if !matches!(
            put.take("Defined").as_deref(),
            None | Some("Self" | "ReplacedCard")
        ) || !matches!(put.take("ETB").as_deref(), None | Some("True"))
        {
            return None;
        }
        let kind_raw = put.take("CounterType")?;
        let kind = super::counter_kind(kind_raw.trim())?;
        let num_raw = put.take("CounterNum").unwrap_or_else(|| "1".to_string());
        let sub = put.take("SubAbility");
        if !put.exhausted() {
            return None;
        }
        let amount = self.parse_moved_counter_amount(num_raw.trim())?;
        Some((kind, amount, sub))
    }

    fn parse_moved_counter_amount(&mut self, raw: &str) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return (n > 0).then(|| format!("Amount::Fixed({n})"));
        }
        let def = if raw.starts_with("Count$") {
            raw.to_string()
        } else {
            self.svars.get(raw)?.trim().to_string()
        };
        self.static_count_expr(&def)
    }

    /// Unconditional self-ETB replacement entering tapped *and* with counters
    /// (`DB$ Tap | SubAbility$ DB$ PutCounter` or unconditional reverse), lowering
    /// to `EnterModifier::Tapped` + `EnterModifier::WithCounters`.
    fn moved_self_tapped_with_counters(&mut self, p: Params) -> ExtraOutcome {
        if p.has("Origin")
            || p.has("ActiveZones")
            || p.has("Secondary")
            || p.has("Condition")
            || p.has("IsPresent")
            || p.has("CheckSVar")
            || p.has("ValidCause")
            || !matches!(p.peek("ReplacementResult"), None | Some("Updated"))
        {
            return ExtraOutcome::Pass;
        }
        let Some(replace_name) = p.peek("ReplaceWith") else {
            return ExtraOutcome::Pass;
        };
        let Some((api1, p1)) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return ExtraOutcome::Pass;
        };
        let (kind, amount) = match api1.as_str() {
            "Tap" => {
                let Some(EtbTap::Sub(sub_name)) = Self::parse_etb_tap_params(p1) else {
                    return ExtraOutcome::Pass;
                };
                let Some((api2, p2)) = self
                    .svars
                    .get(sub_name.trim())
                    .and_then(|b| Params::parse(b))
                else {
                    return ExtraOutcome::Pass;
                };
                if api2 != "PutCounter" {
                    return ExtraOutcome::Pass;
                }
                let Some((kind, amount, None)) = self.parse_etb_put_counter(p2) else {
                    return ExtraOutcome::Pass;
                };
                (kind, amount)
            }
            "PutCounter" => {
                let Some((kind, amount, Some(sub_name))) = self.parse_etb_put_counter(p1) else {
                    return ExtraOutcome::Pass;
                };
                let Some((api2, p2)) = self
                    .svars
                    .get(sub_name.trim())
                    .and_then(|b| Params::parse(b))
                else {
                    return ExtraOutcome::Pass;
                };
                if api2 != "Tap" {
                    return ExtraOutcome::Pass;
                }
                if !matches!(Self::parse_etb_tap_params(p2), Some(EtbTap::Plain)) {
                    return ExtraOutcome::Pass;
                }
                (kind, amount)
            }
            _ => return ExtraOutcome::Pass,
        };
        let mut check = p;
        check.drop_prose();
        check.take("ValidCard");
        check.take("Destination");
        check.take("ReplacementResult");
        check.take("ReplaceWith");
        if !check.exhausted() {
            return ExtraOutcome::Pass;
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::Tapped".to_string());
        self.body.enter_modifiers.push(format!(
            "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
        ));
        ExtraOutcome::Handled(Some(()))
    }

    /// `R: Event$ Moved | ValidCard$ Card.Self+kicked | Destination$ Battlefield`
    /// placing counters and optionally granting permanent keyword(s) or
    /// `CanAttackDefender` via `EnterModifier::WithCountersIf` + conditional static.
    fn moved_self_kicked_etb(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Destination").as_deref() != Some("Battlefield")
            || !matches!(
                p.take("ReplacementResult").as_deref(),
                None | Some("Updated")
            )
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let valid_card = p.take("ValidCard")?;
        let idx = if valid_card == "Card.Self+kicked" && self.has_single_kicker {
            0
        } else if let Some(slot) = valid_card.strip_prefix("Card.Self+kicked ") {
            let Some(i) = self.dual_kicker_index(slot) else {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            };
            i
        } else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        let Some(replace_name) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((api1, p1)) = self
            .svars
            .get(replace_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if api1 != "PutCounter" {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let Some((kind, amount, sub)) = self.parse_etb_put_counter(p1) else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        let granted_modifier = match sub {
            Some(sub_name) => Some(self.kicked_etb_animate_modifier(&sub_name)?),
            None => None,
        };
        self.body.enter_modifiers.push(format!(
            "EnterModifier::WithCountersIf {{ kind: {kind}, count: {amount}, \
             condition: Condition::KickedWith({idx}) }}"
        ));
        if let Some(modifier) = granted_modifier {
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, {modifier}, condition = Some(Condition::KickedWith({idx})))"
            ));
        }
        Some(())
    }

    fn kicked_etb_animate_modifier(&mut self, sub_name: &str) -> Option<String> {
        let Some((api2, mut anim)) = self
            .svars
            .get(sub_name.trim())
            .and_then(|b| Params::parse(b))
        else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        anim.drop_prose();
        if api2 != "Animate"
            || !matches!(anim.take("Defined").as_deref(), None | Some("Self"))
            || anim.take("Duration").as_deref() != Some("Permanent")
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let modifier = if let Some(kw_raw) = anim.take("Keywords") {
            let mut kws = Vec::new();
            for part in kw_raw.split('&') {
                let Some(kw) = super::constants::keyword_const(part.trim()) else {
                    return self.deny("replacement `Moved` this rule cannot read".to_string());
                };
                kws.push(kw);
            }
            if kws.is_empty() {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            }
            let joined = kws[1..].iter().fold(kws[0].to_string(), |mut acc, k| {
                acc.push_str(".union(");
                acc.push_str(k);
                acc.push(')');
                acc
            });
            format!("Modifier::AddKeyword({joined})")
        } else if let Some(static_name) = anim.take("staticAbilities") {
            let Some(static_body) = self.svars.get(static_name.trim()) else {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            };
            let Some((mode, mut sp)) = Params::parse(static_body) else {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            };
            sp.drop_prose();
            if mode != "CanAttackDefender"
                || !matches!(sp.take("ValidCard").as_deref(), None | Some("Card.Self"))
                || !sp.exhausted()
            {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            }
            "Modifier::AttacksDespiteDefender".to_string()
        } else {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        };
        if !anim.exhausted() {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        Some(modifier)
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
        let fallback = p.clone();
        p.drop_prose();
        let active_zones = p.take("ActiveZones");
        if p.take("Destination").as_deref() != Some("Graveyard")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        let origin = p.take("Origin");
        let Some(valid) = p.take("ValidCard").or_else(|| p.take("ValidLKI")) else {
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
        if body.exhausted()
            && matches!(origin.as_deref(), None | Some("All"))
            && matches!(body_origin.as_deref(), None | Some("All"))
        {
            if valid == "Card.Self"
                && counter_raw.is_none()
                && matches!(active_zones.as_deref(), None | Some("All" | "Battlefield"))
            {
                self.body.abilities.push(
                    "AbilityDef::Replacement(ReplacementRule::ExileSelfInsteadOfGraveyard)"
                        .to_string(),
                );
                return Some(());
            }
            if active_zones.as_deref() != Some("Battlefield") {
                return self.deny("replacement `Moved` this rule cannot read".to_string());
            }
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
        if self.lki_exile_instead_of_graveyard(fallback).is_some() {
            return Some(());
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
        if !p.exhausted() || valid.contains("Self") || valid.contains("Other") {
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

    fn moved_self_prepared(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || !matches!(p.take("Origin").as_deref(), None | Some("Any"))
            || !matches!(p.take("ActiveZones").as_deref(), None | Some("All"))
            || !matches!(
                p.take("ReplacementResult").as_deref(),
                None | Some("Updated")
            )
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
        if api != "AlterAttribute"
            || !matches!(body.take("Defined").as_deref(), None | Some("Self"))
            || body.take("Attributes").as_deref() != Some("Prepared")
            || !body.exhausted()
        {
            return self.deny("replacement `Moved` this rule cannot read".to_string());
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::Prepared".to_string());
        self.body.notes.push("prepared".to_string());
        Some(())
    }

    fn continuous_draw_exile_play(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("AffectedZone");
        p.take("Affected");
        p.take("MayPlay");
        if !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield")) {
            return self.deny("static ability `S: Mode$ Continuous`".to_string());
        }
        let may_play_player = p.take("MayPlayPlayer");
        let allow_each = matches!(may_play_player.as_deref(), Some("CardOwner" | "Player"));
        let allow_you = matches!(may_play_player.as_deref(), None | Some("You"));
        if !(allow_each || allow_you) || !p.exhausted() {
            return self.deny("static ability `S: Mode$ Continuous`".to_string());
        }
        let candidates: &[&str] = if allow_each {
            &["PlayerRel::EachPlayer", "PlayerRel::EachOpponent"]
        } else {
            &["PlayerRel::You"]
        };
        for ability in &mut self.body.abilities {
            for who in candidates {
                let needle = format!(
                    "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop {{ \
                     who: {who}, face_down: false, may_play_this_turn: false }})"
                );
                if ability == &needle {
                    *ability = format!(
                        "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop {{ \
                         who: {who}, face_down: false, may_play_this_turn: true }})"
                    );
                    return Some(());
                }
            }
        }
        self.deny("static ability `S: Mode$ Continuous`".to_string())
    }

    fn replace_draw_wave11a(&mut self, orig: Params) -> ZoneContinuousOutcome {
        let mut p = orig.clone();
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") {
            return ZoneContinuousOutcome::Pass(orig);
        }
        let Some(with) = p.take("ReplaceWith") else {
            if p.peek("Prevent") == Some("True") || p.peek("ReplacementResult") == Some("Prevent") {
                return ZoneContinuousOutcome::Handled(self.replace_draw_skip(p));
            }
            return ZoneContinuousOutcome::Pass(orig);
        };
        let Some((api, body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return ZoneContinuousOutcome::Pass(orig);
        };
        match api.as_str() {
            "Draw" => ZoneContinuousOutcome::Handled(self.replace_draw_with_draws(p, body)),
            "WinsGame" => ZoneContinuousOutcome::Handled(self.replace_draw_win(p, body)),
            "Dig" => ZoneContinuousOutcome::Handled(self.replace_draw_dig(p, body)),
            "ChangeZone" => ZoneContinuousOutcome::Handled(self.replace_draw_change_zone(p, body)),
            _ => ZoneContinuousOutcome::Pass(orig),
        }
    }

    fn take_optional_condition(&mut self, p: &mut Params) -> Option<String> {
        let raw = self.condition_any(p)?;
        if raw.is_empty() {
            Some("None".to_string())
        } else {
            raw.strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
                .map(|cond| format!("Some({cond})"))
        }
    }

    fn take_empty_library_flag(&self, p: &mut Params) -> bool {
        if p.peek("Library") == Some("Empty") {
            p.take("Library");
            return true;
        }
        if p.peek("IsPresent") == Some("Card.YouOwn")
            && p.peek("PresentZone") == Some("Library")
            && p.peek("PresentCompare") == Some("EQ0")
        {
            p.take("IsPresent");
            p.take("PresentZone");
            p.take("PresentCompare");
            return true;
        }
        if let Some(var) = p.peek("CheckSVar")
            && p.peek("SVarCompare") == Some("EQ0")
            && !p.has("CheckSecondSVar")
            && let Some(def) = self.svars.get(var.trim()).map(|s| s.trim())
            && matches!(
                def,
                "Count$CardsInYourLibrary" | "Count$ValidLibrary Card.YouOwn"
            )
        {
            p.take("CheckSVar");
            p.take("SVarCompare");
            return true;
        }
        false
    }

    fn replace_draw_skip(&mut self, mut p: Params) -> Option<()> {
        let Some(who) = p.take("ValidPlayer").as_deref().and_then(parse_player_rel) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        let optional = match p.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return self.deny("replacement `R: Event$ Draw`".to_string()),
        };
        let decider_ok = match p.take("OptionalDecider").as_deref() {
            None => true,
            Some("You") => optional && who == "PlayerRel::You",
            Some("ReplacedPlayer") => optional,
            Some(_) => false,
        };
        let prevent_ok = matches!(p.take("Prevent").as_deref(), None | Some("True"));
        let result_ok = matches!(
            p.take("ReplacementResult").as_deref(),
            None | Some("Prevent")
        );
        let layer_ok = matches!(p.take("Layer").as_deref(), None | Some("CantHappen"));
        let empty_library_only = who == "PlayerRel::You" && self.take_empty_library_flag(&mut p);
        if !decider_ok || !prevent_ok || !result_ok || !layer_ok || !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::SkipDrawInstead {{ \
             who: {who}, optional: {optional}, empty_library_only: {empty_library_only} }})"
        ));
        Some(())
    }

    fn replace_draw_with_draws(&mut self, mut p: Params, mut body: Params) -> Option<()> {
        let Some(who) = p.take("ValidPlayer").as_deref().and_then(parse_player_rel) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        let skip_first = match p.take("NotFirstCardInDrawStep").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return self.deny("replacement `R: Event$ Draw`".to_string()),
        };
        body.drop_prose();
        let defined = body.take("Defined");
        if who == "PlayerRel::EachOpponent" && defined.as_deref() == Some("You") {
            let count_ok = matches!(body.take("NumCards").as_deref(), None | Some("1"));
            if !skip_first || !count_ok || !body.exhausted() || !p.exhausted() {
                return self.deny("replacement `R: Event$ Draw`".to_string());
            }
            self.body.abilities.push(
                "AbilityDef::Replacement(ReplacementRule::OpponentsExtraDrawsYouDraw)".to_string(),
            );
            return Some(());
        }
        let defined_ok = if who == "PlayerRel::You" {
            matches!(defined.as_deref(), None | Some("You" | "ReplacedPlayer"))
        } else {
            defined.as_deref() == Some("ReplacedPlayer")
        };
        let Ok(count) = body
            .take("NumCards")
            .as_deref()
            .unwrap_or("1")
            .parse::<u8>()
        else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        let Some(cond) = self.take_optional_condition(&mut p) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !defined_ok || !body.exhausted() || !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawWithDraws {{ \
             who: {who}, skip_first_in_draw_step: {skip_first}, condition: {cond}, count: {count} }})"
        ));
        Some(())
    }

    fn replace_draw_win(&mut self, mut p: Params, mut body: Params) -> Option<()> {
        if p.take("ValidPlayer").as_deref() != Some("You") || !self.take_empty_library_flag(&mut p)
        {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        body.drop_prose();
        if !p.exhausted()
            || !matches!(body.take("Defined").as_deref(), None | Some("You"))
            || !body.exhausted()
        {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body
            .abilities
            .push("AbilityDef::Replacement(ReplacementRule::WinOnEmptyLibraryDraw)".to_string());
        Some(())
    }

    fn replace_draw_dig(&mut self, mut p: Params, mut body: Params) -> Option<()> {
        body.drop_prose();
        if body.peek("DestinationZone") == Some("Exile") {
            return self.replace_draw_exile_dig(p, body);
        }
        if p.take("ValidPlayer").as_deref() != Some("You") || !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        let Some(count) = body
            .take("DigNum")
            .and_then(|s| s.parse::<u8>().ok())
            .filter(|&n| n >= 1)
        else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !matches!(body.take("Defined").as_deref(), None | Some("You"))
            || !matches!(body.take("DestinationZone").as_deref(), None | Some("Hand"))
            || !matches!(body.take("ChangeNum").as_deref(), None | Some("1"))
            || !matches!(body.take("NoReveal").as_deref(), None | Some("True"))
            || body.has("Reveal")
        {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        let rest_to_graveyard = match (
            body.take("DestinationZone2").as_deref(),
            body.take("LibraryPosition2").as_deref(),
        ) {
            (Some("Graveyard"), None) => true,
            (None | Some("Library"), None | Some("-1")) => {
                if !matches!(body.take("LibraryPosition").as_deref(), None | Some("-1")) {
                    return self.deny("replacement `R: Event$ Draw`".to_string());
                }
                false
            }
            _ => return self.deny("replacement `R: Event$ Draw`".to_string()),
        };
        if !body.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawLookTop {{ \
             count: {count}, rest_to_graveyard: {rest_to_graveyard} }})"
        ));
        Some(())
    }

    fn replace_draw_exile_dig(&mut self, mut p: Params, mut body: Params) -> Option<()> {
        let Some(who) = p.take("ValidPlayer").as_deref().and_then(parse_player_rel) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        let defined_ok = if who == "PlayerRel::You" {
            matches!(
                body.take("Defined").as_deref(),
                None | Some("You" | "ReplacedPlayer")
            )
        } else {
            body.take("Defined").as_deref() == Some("ReplacedPlayer")
        };
        let num_ok = body.take("DigNum").as_deref() == Some("1")
            && matches!(body.take("ChangeNum").as_deref(), Some("1" | "All"))
            && body.take("DestinationZone").as_deref() == Some("Exile")
            && matches!(body.take("ExileWithSource").as_deref(), None | Some("True"));
        let Some(face_down) = parse_face_down_flag(&mut body) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !defined_ok || !num_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop {{ \
             who: {who}, face_down: {face_down}, may_play_this_turn: false }})"
        ));
        Some(())
    }

    fn replace_draw_change_zone(&mut self, mut p: Params, mut body: Params) -> Option<()> {
        let Some(who) = p.take("ValidPlayer").as_deref().and_then(parse_player_rel) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !p.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        body.drop_prose();
        let def = body.take("Defined").or_else(|| body.take("DefinedPlayer"));
        let defined_ok = if who == "PlayerRel::You" {
            matches!(def.as_deref(), None | Some("You" | "ReplacedPlayer"))
        } else {
            def.as_deref() == Some("ReplacedPlayer")
        };
        let zone_ok = body.take("Origin").as_deref() == Some("Library")
            && body.take("Destination").as_deref() == Some("Exile")
            && matches!(body.take("LibraryPosition").as_deref(), None | Some("0"))
            && matches!(body.take("ChangeNum").as_deref(), None | Some("1"))
            && matches!(body.take("ChangeType").as_deref(), None | Some("Card"))
            && matches!(body.take("Mandatory").as_deref(), None | Some("True"));
        let Some(face_down) = parse_face_down_flag(&mut body) else {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        };
        if !defined_ok || !zone_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ Draw`".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop {{ \
             who: {who}, face_down: {face_down}, may_play_this_turn: false }})"
        ));
        Some(())
    }

    fn replace_create_token(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || !matches!(
                p.take("ReplacementResult").as_deref(),
                None | Some("Updated")
            )
        {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let effect_only = p.take("EffectOnly");
        if !matches!(effect_only.as_deref(), None | Some("True")) {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let valid_token = p.take("ValidToken");
        let valid_player = p.take("ValidPlayer");
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some((api, mut body)) = self.svars.get(&replace_with).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        body.drop_prose();
        if api != "ReplaceToken" {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        match body.take("Type").as_deref() {
            Some("Amount") if valid_player.is_none() && p.exhausted() => {
                self.replace_create_token_amount(valid_token.as_deref(), body)
            }
            Some("AddToken") if effect_only.is_none() => self.replace_create_token_add(
                p,
                valid_token.as_deref(),
                valid_player.as_deref(),
                body,
            ),
            Some(ty @ ("Token" | "ReplaceToken" | "Replace" | "Substitute" | "MultipleTokens"))
                if effect_only.is_none() =>
            {
                self.replace_create_token_substitute(
                    p,
                    valid_token.as_deref(),
                    valid_player.as_deref(),
                    ty,
                    body,
                )
            }
            _ => self.deny("replacement `R: Event$ CreateToken`".to_string()),
        }
    }

    fn replace_create_token_amount(
        &mut self,
        valid_token: Option<&str>,
        mut body: Params,
    ) -> Option<()> {
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
        let (base_token, controller_filter) = match valid_token {
            None | Some("Card") => ("Card", "&Filter::Any"),
            Some("Card.YouCtrl") => ("Card", "&Filter::ControlledByYou"),
            Some("Card.OppCtrl") => ("Card", "&Filter::ControlledByOpponent"),
            Some(raw) => {
                if let Some(t) = raw.strip_suffix(".YouCtrl") {
                    (t, "&Filter::ControlledByYou")
                } else if let Some(t) = raw.strip_suffix(".OppCtrl") {
                    (t, "&Filter::ControlledByOpponent")
                } else {
                    (raw, "&Filter::Any")
                }
            }
        };
        if base_token == "Card" && factor == 2 {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {{ \
                 controller_filter: {controller_filter} }})"
            ));
            return Some(());
        }
        if let Some(ty) = card_type_const(base_token) {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::MultiplyTypedTokens {{ \
                 controller_filter: {controller_filter}, types: {ty}, factor: {factor} }})"
            ));
            return Some(());
        }
        self.deny("replacement `R: Event$ CreateToken`".to_string())
    }

    fn resolve_token_body(&self, stem: &str) -> Option<crate::tokengen::TokenBody> {
        let trimmed = stem.trim();
        if trimmed.is_empty() || trimmed == "Chosen" {
            return None;
        }
        self.tokens?.body(trimmed, self.cats)
    }

    fn take_token_replace_amount(&self, body: &mut Params) -> Option<TokenReplaceAmt> {
        let a = body.take("Amount");
        let t = body.take("TokenAmount");
        let raw = match (a, t) {
            (None, None) => return Some(TokenReplaceAmt::Omitted),
            (Some(v), None) | (None, Some(v)) => v,
            (Some(v1), Some(v2)) if v1 == v2 => v1,
            (Some(_), Some(_)) => return None,
        };
        let def = self
            .svars
            .get(raw.trim())
            .map_or(raw.trim(), String::as_str)
            .trim();
        match def {
            "1" => Some(TokenReplaceAmt::One),
            "ReplaceCount$TokenNum" => Some(TokenReplaceAmt::MatchCount),
            _ => None,
        }
    }

    fn token_owner_matches_scope(owner: Option<&str>, controller_filter: &str) -> bool {
        match owner {
            None | Some("ReplacedPlayer") => true,
            Some("You") => controller_filter == "&Filter::ControlledByYou",
            _ => false,
        }
    }

    fn replace_create_token_add(
        &mut self,
        mut p: Params,
        valid_token: Option<&str>,
        valid_player: Option<&str>,
        mut body: Params,
    ) -> Option<()> {
        let Some(cond) = self.take_optional_condition(&mut p) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some((controller_filter, stripped)) = parse_token_rule_scope(valid_token, valid_player)
        else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some(stem) = body.take("TokenScript") else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some(amt) = self.take_token_replace_amount(&mut body) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let match_count = amt == TokenReplaceAmt::MatchCount;
        let owner_ok =
            Self::token_owner_matches_scope(body.take("TokenOwner").as_deref(), controller_filter);
        if stem.contains(',') || !p.exhausted() || !owner_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some(tok_body) = self.resolve_token_body(&stem) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let tok_expr = if stripped == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr(&stripped)?
        };
        let token_filter = self.body.filter_static("TOKEN", &tok_expr);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::AdditionalTokenCreation {{ \
             controller_filter: {controller_filter}, token_filter: &{token_filter}, \
             bonus_token: &generated_tokens::{}, match_count: {match_count}, condition: {cond} }})",
            tok_body.constant
        ));
        Some(())
    }

    fn replace_create_token_substitute(
        &mut self,
        mut p: Params,
        valid_token: Option<&str>,
        valid_player: Option<&str>,
        ty: &str,
        mut body: Params,
    ) -> Option<()> {
        let Some(stem_raw) = body.take("TokenScript") else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        if stem_raw.contains(',') {
            return self.replace_create_token_manufactor(
                &p,
                valid_token,
                valid_player,
                &stem_raw,
                body,
            );
        }
        if ty == "MultipleTokens" {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some(cond) = self.take_optional_condition(&mut p) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some((controller_filter, stripped)) = parse_token_rule_scope(valid_token, valid_player)
        else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let Some(amt) = self.take_token_replace_amount(&mut body) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let owner_ok =
            Self::token_owner_matches_scope(body.take("TokenOwner").as_deref(), controller_filter);
        if amt == TokenReplaceAmt::One || !p.exhausted() || !owner_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some(tok_body) = self.resolve_token_body(&stem_raw) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let tok_expr = if stripped == "Card" {
            "Filter::Any".to_string()
        } else {
            self.filter_expr(&stripped)?
        };
        let token_filter = self.body.filter_static("TOKEN", &tok_expr);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::SubstituteTokenCreation {{ \
             controller_filter: {controller_filter}, token_filter: &{token_filter}, \
             replacement: &generated_tokens::{}, condition: {cond} }})",
            tok_body.constant
        ));
        Some(())
    }

    fn replace_create_token_manufactor(
        &mut self,
        p: &Params,
        valid_token: Option<&str>,
        valid_player: Option<&str>,
        stem_raw: &str,
        mut body: Params,
    ) -> Option<()> {
        let Some((controller_filter, stripped)) = parse_token_rule_scope(valid_token, valid_player)
        else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let mut types: Vec<&str> = stripped.split(',').map(str::trim).collect();
        types.sort_unstable();
        if types != ["Clue", "Food", "Treasure"] {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let Some(amt) = self.take_token_replace_amount(&mut body) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        let owner_ok =
            Self::token_owner_matches_scope(body.take("TokenOwner").as_deref(), controller_filter);
        if amt == TokenReplaceAmt::MatchCount || !p.exhausted() || !owner_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        }
        let mut clue = None;
        let mut food = None;
        let mut treasure = None;
        for stem in stem_raw.split(',') {
            let Some(tok) = self.resolve_token_body(stem) else {
                return self.deny("replacement `R: Event$ CreateToken`".to_string());
            };
            match manufactor_slot(&tok) {
                Some("Clue") if clue.is_none() => clue = Some(tok.constant),
                Some("Food") if food.is_none() => food = Some(tok.constant),
                Some("Treasure") if treasure.is_none() => treasure = Some(tok.constant),
                _ => return self.deny("replacement `R: Event$ CreateToken`".to_string()),
            }
        }
        let (Some(clue), Some(food), Some(treasure)) = (clue, food, treasure) else {
            return self.deny("replacement `R: Event$ CreateToken`".to_string());
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ManufactorTokenCreation {{ \
             controller_filter: {controller_filter}, clue: &generated_tokens::{clue}, \
             food: &generated_tokens::{food}, treasure: &generated_tokens::{treasure} }})"
        ));
        Some(())
    }

    fn resolve_counter_amt_def(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if let Some(def) = self.svars.get(trimmed) {
            return Some(def.trim().to_string());
        }
        trimmed
            .starts_with("ReplaceCount$")
            .then(|| trimmed.to_string())
    }

    fn replace_add_counter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield") {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        if !p.has("ReplaceWith") && p.take("Prevent").as_deref() == Some("True") {
            if !matches!(p.take("Layer").as_deref(), None | Some("CantHappen")) {
                return self.deny("replacement `R: Event$ AddCounter`".to_string());
            }
            return self.emit_add_counter_wave11a(p, None, None, "CounterReplaceOp::Sub(u16::MAX)");
        }
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some((api, mut body)) = self.svars.get(&replace_with).and_then(|s| Params::parse(s))
        else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        body.drop_prose();
        if api != "ReplaceCounter" {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        let Some(amt_var) = body.take("Amount") else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some(amt_def) = self.resolve_counter_amt_def(&amt_var) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };

        // Case 1: "If you would put one or more counters on a permanent or
        // player, put twice that many ... instead." (Innkeeper's Talent).
        if !p.has("ValidCounterType")
            && !body.has("ValidCounterType")
            && p.peek("ValidSource") == Some("You")
            && p.peek("ValidObject") == Some("Permanent.inZoneBattlefield,Player")
        {
            return self.replace_add_counter_by_you(p, body, &amt_def);
        }

        if let ExtraOutcome::Handled(res) = self.try_legacy_add_counter(&mut p, &mut body, &amt_def)
        {
            return res;
        }
        let body_kind = body.take("ValidCounterType");
        let body_src = body.take("ValidSource");
        let choose_ok = matches!(body.take("ChooseCounter").as_deref(), None | Some("True"));
        let res_ok = matches!(
            body.take("ReplacementResult").as_deref(),
            None | Some("Updated")
        );
        if !choose_ok || !res_ok || !body.exhausted() {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        let Some(op) = parse_counter_replace_op(&amt_def) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        self.emit_add_counter_wave11a(p, body_kind, body_src, &op)
    }

    fn try_legacy_add_counter(
        &mut self,
        p: &mut Params,
        body: &mut Params,
        amt_def: &str,
    ) -> ExtraOutcome {
        if p.has("ValidCounterType")
            || body.has("ValidCounterType")
            || p.has("ValidSource")
            || body.has("ValidSource")
            || p.has("ValidPlayer")
            || p.has("ValidObject")
            || p.has("ValidCause")
        {
            return ExtraOutcome::Pass;
        }
        let Some(valid_card) = p.peek("ValidCard") else {
            return ExtraOutcome::Pass;
        };
        let Some(stripped) = strip_battlefield_zone(valid_card) else {
            return ExtraOutcome::Pass;
        };
        let effect_only = match p.peek("EffectOnly") {
            None => false,
            Some("True") => true,
            Some(_) => return ExtraOutcome::Pass,
        };
        if effect_only
            && matches!(
                amt_def,
                "ReplaceCount$CounterNum/Twice" | "ReplaceCount$CounterNum/Times.2"
            )
            && !p.has("Condition")
            && body.exhausted()
        {
            p.take("ValidCard");
            p.take("EffectOnly");
            if !p.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("replacement `R: Event$ AddCounter`".to_string()),
                );
            }
            let Some(expr) = self.filter_expr(&stripped) else {
                return ExtraOutcome::Handled(None);
            };
            let filter = self.body.filter_static("COUNTERS", &expr);
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement {{ \
                 object_filter: &{filter} }})"
            ));
            return ExtraOutcome::Handled(Some(()));
        }
        if !effect_only
            && matches!(body.peek("ChooseCounter"), None | Some("True"))
            && let Some(n_str) = amt_def.strip_prefix("ReplaceCount$CounterNum/Plus.")
            && let Ok(n) = n_str.trim().parse::<u16>()
            && n > 0
        {
            p.take("ValidCard");
            body.take("ChooseCounter");
            if !p.exhausted() || !body.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("replacement `R: Event$ AddCounter`".to_string()),
                );
            }
            let Some(expr) = self.filter_expr(&stripped) else {
                return ExtraOutcome::Handled(None);
            };
            self.body.abilities.push(Self::static_expr(
                &expr,
                &format!("Modifier::AdditionalCounterPlacement {{ n: {n} }}"),
            ));
            return ExtraOutcome::Handled(Some(()));
        }
        ExtraOutcome::Pass
    }

    fn emit_add_counter_wave11a(
        &mut self,
        mut p: Params,
        body_kind: Option<String>,
        body_src: Option<String>,
        op: &str,
    ) -> Option<()> {
        let Some(kind_expr) = merge_counter_kind(p.take("ValidCounterType"), body_kind) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let Some(placer_expr) = merge_placer(p.take("ValidSource"), body_src) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        let effect_only = match p.take("EffectOnly").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return self.deny("replacement `R: Event$ AddCounter`".to_string()),
        };
        if !matches!(
            p.take("ReplacementResult").as_deref(),
            None | Some("Updated")
        ) {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        let Some((perm_filter, player_rel)) = parse_counter_targets(
            p.take("ValidCard"),
            p.take("ValidObject"),
            p.take("ValidPlayer"),
        ) else {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        };
        if (perm_filter.is_none() && player_rel.is_none())
            || (player_rel.is_some() && effect_only)
            || !p.exhausted()
        {
            return self.deny("replacement `R: Event$ AddCounter`".to_string());
        }
        if let Some(raw_perm) = perm_filter {
            let expr = self.filter_expr(&raw_perm)?;
            let filter = self.body.filter_static("COUNTERS", &expr);
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement {{ \
                 object_filter: &{filter}, kind: {kind_expr}, placer: {placer_expr}, \
                 effect_only: {effect_only}, op: {op} }})"
            ));
        }
        if let Some(recipient) = player_rel {
            self.body.abilities.push(format!(
                "AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement {{ \
                 recipient: {recipient}, kind: {kind_expr}, placer: {placer_expr}, op: {op} }})"
            ));
        }
        Some(())
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

fn parse_player_rel(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "You" => Some("PlayerRel::You"),
        "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
        "Player" => Some("PlayerRel::EachPlayer"),
        _ => None,
    }
}

fn parse_face_down_flag(body: &mut Params) -> Option<bool> {
    match (
        body.take("ExileFaceDown").as_deref(),
        body.take("FaceDown").as_deref(),
    ) {
        (Some("True"), None | Some("True")) | (None, Some("True")) => Some(true),
        (None | Some("False"), None | Some("False")) => Some(false),
        _ => None,
    }
}

fn strip_token_ctrl_alt(alt: &str) -> Option<(Option<&'static str>, String)> {
    match alt {
        "YouCtrl" => return Some((Some("&Filter::ControlledByYou"), "Card".to_string())),
        "OppCtrl" => return Some((Some("&Filter::ControlledByOpponent"), "Card".to_string())),
        _ => {}
    }
    let Some((head, quals)) = alt.split_once('.') else {
        return Some((None, alt.to_string()));
    };
    let mut ctrl = None;
    let mut rest = Vec::new();
    for q in quals.split('+') {
        let q = q.trim();
        let next = match q {
            "YouCtrl" => Some("&Filter::ControlledByYou"),
            "OppCtrl" => Some("&Filter::ControlledByOpponent"),
            "" => continue,
            _ => {
                rest.push(q);
                continue;
            }
        };
        if let (Some(prev), Some(n)) = (ctrl, next)
            && prev != n
        {
            return None;
        }
        ctrl = next;
    }
    let tok = if rest.is_empty() {
        head.trim().to_string()
    } else {
        format!("{}.{}", head.trim(), rest.join("+"))
    };
    Some((ctrl, tok))
}

fn parse_token_rule_scope(
    valid_token: Option<&str>,
    valid_player: Option<&str>,
) -> Option<(&'static str, String)> {
    let mut ctrl: Option<&'static str> = None;
    let mut out = Vec::new();
    for (idx, alt) in valid_token.unwrap_or("Card").split(',').enumerate() {
        let (alt_ctrl, tok) = strip_token_ctrl_alt(alt.trim())?;
        if idx == 0 {
            ctrl = alt_ctrl;
        } else if alt_ctrl != ctrl {
            return None;
        }
        out.push(tok);
    }
    let next_ctrl = match valid_player {
        None => ctrl,
        Some("You") => match ctrl {
            None | Some("&Filter::ControlledByYou") => Some("&Filter::ControlledByYou"),
            Some(_) => return None,
        },
        Some("Opponent" | "Player.Opponent") => match ctrl {
            None | Some("&Filter::ControlledByOpponent") => Some("&Filter::ControlledByOpponent"),
            Some(_) => return None,
        },
        Some("Player") if ctrl.is_none() => None,
        _ => return None,
    };
    Some((next_ctrl.unwrap_or("&Filter::Any"), out.join(",")))
}

fn manufactor_slot(tok: &crate::tokengen::TokenBody) -> Option<&'static str> {
    if tok.constant == "CLUE" || tok.literal.contains("artifact::CLUE") {
        Some("Clue")
    } else if tok.constant == "FOOD" || tok.literal.contains("artifact::FOOD") {
        Some("Food")
    } else if tok.constant == "TREASURE" || tok.literal.contains("artifact::TREASURE") {
        Some("Treasure")
    } else {
        None
    }
}

fn parse_counter_replace_op(amt_def: &str) -> Option<String> {
    match amt_def.trim() {
        "ReplaceCount$CounterNum/Twice" | "ReplaceCount$CounterNum/Times.2" => {
            Some("CounterReplaceOp::Multiply(2)".to_string())
        }
        "ReplaceCount$CounterNum/Thrice" | "ReplaceCount$CounterNum/Times.3" => {
            Some("CounterReplaceOp::Multiply(3)".to_string())
        }
        "ReplaceCount$CounterNum/HalfDown" => Some("CounterReplaceOp::HalveFloor".to_string()),
        s => {
            if let Some(n_str) = s.strip_prefix("ReplaceCount$CounterNum/Times.") {
                let n = n_str.trim().parse::<u16>().ok().filter(|&n| n >= 2)?;
                return Some(format!("CounterReplaceOp::Multiply({n})"));
            }
            if let Some(n_str) = s.strip_prefix("ReplaceCount$CounterNum/Plus.") {
                let n = n_str.trim().parse::<u16>().ok().filter(|&n| n >= 1)?;
                return Some(format!("CounterReplaceOp::Add({n})"));
            }
            if let Some(n_str) = s.strip_prefix("ReplaceCount$CounterNum/Minus.") {
                let n = n_str.trim().parse::<u16>().ok().filter(|&n| n >= 1)?;
                return Some(format!("CounterReplaceOp::Sub({n})"));
            }
            None
        }
    }
}

fn merge_counter_kind(p_kind: Option<String>, b_kind: Option<String>) -> Option<String> {
    let raw = match (p_kind, b_kind) {
        (None, None) => return Some("None".to_string()),
        (Some(a), None) | (None, Some(a)) => a,
        (Some(a), Some(b)) if a == b => a,
        (Some(_), Some(_)) => return None,
    };
    Some(format!("Some({})", counter_kind(raw.trim())?))
}

fn merge_placer(p_src: Option<String>, b_src: Option<String>) -> Option<String> {
    let raw = match (p_src, b_src) {
        (None, None) => return Some("None".to_string()),
        (Some(a), None) | (None, Some(a)) => a,
        (Some(a), Some(b)) if a == b => a,
        (Some(_), Some(_)) => return None,
    };
    Some(format!("Some({})", parse_player_rel(&raw)?))
}

fn strip_counter_perm_alt(alt: &str) -> Option<String> {
    let alt = alt.trim();
    if alt == "Card.Self" {
        return Some("Card.Self".to_string());
    }
    let (head, quals) = alt.split_once('.')?;
    let mut found = false;
    let mut rest = Vec::new();
    for q in quals.split('+') {
        let q = q.trim();
        if q == "inZoneBattlefield" {
            found = true;
        } else if !q.is_empty() {
            rest.push(q);
        }
    }
    if !found || head.trim().is_empty() {
        return None;
    }
    Some(if rest.is_empty() {
        head.trim().to_string()
    } else {
        format!("{}.{}", head.trim(), rest.join("+"))
    })
}

fn parse_counter_targets(
    valid_card: Option<String>,
    valid_object: Option<String>,
    valid_player: Option<String>,
) -> Option<(Option<String>, Option<&'static str>)> {
    let mut perm_alts = Vec::new();
    let mut player_rel: Option<&'static str> = None;
    let mut set_player = |rel: &'static str| -> Option<()> {
        if let Some(prev) = player_rel
            && prev != rel
        {
            return None;
        }
        player_rel = Some(rel);
        Some(())
    };
    if let Some(vp) = valid_player {
        set_player(parse_player_rel(&vp)?)?;
    }
    if let Some(vc) = valid_card {
        for alt in vc.split(',') {
            perm_alts.push(strip_counter_perm_alt(alt)?);
        }
    }
    if let Some(vo) = valid_object {
        for alt in vo.split(',') {
            let t = alt.trim();
            if let Some(rel) = parse_player_rel(t) {
                set_player(rel)?;
            } else {
                perm_alts.push(strip_counter_perm_alt(t)?);
            }
        }
    }
    let perm = (!perm_alts.is_empty()).then(|| perm_alts.join(","));
    Some((perm, player_rel))
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
        let existing_inner = existing
            .strip_prefix("Condition::All(&[")
            .and_then(|s| s.strip_suffix("])"))
            .unwrap_or(existing);
        let cond_inner = cond
            .strip_prefix("Condition::All(&[")
            .and_then(|s| s.strip_suffix("])"))
            .unwrap_or(cond);
        return Some(format!(
            "{head}, condition = Some(Condition::All(&[{existing_inner}, {cond_inner}])))"
        ));
    }
    if open.contains("condition =") {
        return None;
    }
    Some(format!("{open}, condition = Some({cond}))"))
}

pub(super) fn parse_at_least_u16(cmp: &str) -> Option<u16> {
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

pub(super) fn parse_at_most_u16(cmp: &str) -> Option<u16> {
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

fn is_player_continuous_affected(raw: &str) -> bool {
    let trimmed = raw.trim();
    matches!(
        trimmed,
        "You"
            | "Player"
            | "Opponent"
            | "Player.Opponent"
            | "Player.Chosen"
            | "EnchantedPlayer"
            | "Player.EnchantedBy"
            | "Player.EnchantedPlayer"
    ) || (trimmed.contains(',') && trimmed.split(',').any(|p| p.trim() == "You"))
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

fn strip_power_lt_toughness(valid: &str) -> Result<(bool, String), ()> {
    let mut saw_true = false;
    let mut saw_false = false;
    let mut out = Vec::new();
    for alt in valid.split(',') {
        let alt = alt.trim();
        if alt.is_empty() {
            return Err(());
        }
        if let Some((base, tail)) = alt.split_once('.') {
            let base = base.trim();
            if base.is_empty() || matches!(base, "powerLTtoughness" | "!powerLTtoughness") {
                return Err(());
            }
            let mut has_atom = false;
            let mut kept = Vec::new();
            for atom in tail.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if atom == "!powerLTtoughness" {
                    return Err(());
                }
                if atom == "powerLTtoughness" {
                    has_atom = true;
                } else {
                    kept.push(atom);
                }
            }
            if has_atom {
                saw_true = true;
            } else {
                saw_false = true;
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        } else {
            if matches!(alt, "powerLTtoughness" | "!powerLTtoughness") {
                return Err(());
            }
            saw_false = true;
            out.push(alt.to_string());
        }
    }
    if saw_true && saw_false {
        return Err(());
    }
    Ok((saw_true, out.join(",")))
}

fn flash_modifier_for_valid(valid_card: &str) -> Option<String> {
    match valid_card.trim() {
        "Card" => Some("Modifier::SpellsHaveFlash(None)".to_string()),
        "Sorcery" => Some("Modifier::SorceriesHaveFlash".to_string()),
        other => {
            let mut types = Vec::new();
            for alt in other.split(',') {
                let ty = card_type_const(alt.trim())?;
                if !types.contains(&ty) {
                    types.push(ty);
                }
            }
            let (first, rest) = types.split_first()?;
            let joined = rest.iter().fold((*first).to_string(), |mut acc, t| {
                acc.push_str(".union(");
                acc.push_str(t);
                acc.push(')');
                acc
            });
            Some(format!("Modifier::SpellsHaveFlash(Some({joined}))"))
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
struct ZonePlayShape {
    cleaned: String,
    has_nonland_all: bool,
    all_nonland_bases: bool,
    any_land_base: bool,
}

fn strip_top_library_yours(affected: &str) -> Option<ZonePlayShape> {
    strip_zone_play_yours(affected, true)
}

fn strip_graveyard_yours(affected: &str) -> Option<ZonePlayShape> {
    strip_zone_play_yours(affected, false)
}

fn strip_zone_play_yours(affected: &str, require_top_library: bool) -> Option<ZonePlayShape> {
    let mut cleaned = Vec::new();
    let mut has_nonland_all = true;
    let mut all_nonland_bases = true;
    let mut any_land_base = false;
    for alt in affected.split(',') {
        let mut parts = alt
            .split('.')
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let base = parts.next()?;
        if matches!(base, "Self" | "Other" | "StrictlyOther")
            || base.contains("Ctrl")
            || base.contains("Own")
        {
            return None;
        }
        let mut saw_top = false;
        let mut saw_you = false;
        let mut alt_nonland = false;
        let mut kept = Vec::new();
        for atom in parts {
            match atom {
                "TopLibrary" if require_top_library => saw_top = true,
                "YouCtrl" | "YouOwn" => saw_you = true,
                "nonLand" => {
                    alt_nonland = true;
                    kept.push(atom);
                }
                other
                    if other.contains("Ctrl")
                        || other.contains("Own")
                        || other.contains("Remembered")
                        || matches!(other, "Self" | "Other" | "StrictlyOther" | "TopLibrary") =>
                {
                    return None;
                }
                other => kept.push(other),
            }
        }
        if !saw_you || (require_top_library && !saw_top) {
            return None;
        }
        if !alt_nonland {
            has_nonland_all = false;
        }
        if base == "Land" {
            any_land_base = true;
        }
        if !matches!(
            base,
            "Instant" | "Sorcery" | "Planeswalker" | "Battle" | "Aura" | "Equipment"
        ) {
            all_nonland_bases = false;
        }
        if kept.is_empty() {
            cleaned.push(base.to_string());
        } else {
            cleaned.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!cleaned.is_empty()).then(|| ZonePlayShape {
        cleaned: cleaned.join(","),
        has_nonland_all,
        all_nonland_bases,
        any_land_base,
    })
}

fn valid_after_stack_is_redundant(valid_after_stack: &str, affected: &str) -> bool {
    let Some(atom) = valid_after_stack.trim().strip_prefix("Spell.") else {
        return false;
    };
    if atom.is_empty() || atom.contains(',') || atom.contains('+') {
        return false;
    }
    affected
        .split(',')
        .all(|alt| alt.split(['.', '+']).any(|a| a.trim() == atom))
}
