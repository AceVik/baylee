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
        if let Some(rest) = line.strip_prefix("Suspend:") {
            return ExtraOutcome::Handled(self.suspend(rest));
        }
        if let Some(rest) = line.strip_prefix("Partner with:") {
            return ExtraOutcome::Handled(self.partner_with(rest));
        }
        if let Some(rest) = line.strip_prefix("etbCounter:") {
            return ExtraOutcome::Handled(self.etb_counter_extra(rest));
        }
        match line.trim() {
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
    /// `EnterModifier::WithCounters` (`eval::amount` on entry).
    fn etb_counter_extra(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let (Some(kind_raw), Some(amount_raw)) = (fields.next(), fields.next()) else {
            return self.deny("keyword `etbCounter`".to_string());
        };
        if let Some(cond) = fields.next()
            && !cond.trim().eq_ignore_ascii_case("no condition")
        {
            return self.deny("keyword `etbCounter`".to_string());
        }
        let _desc = fields.next();
        if fields.next().is_some() {
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

    fn etb_counter_amount(&mut self, raw: &str) -> Option<String> {
        let def = self.svars.get(raw)?.trim().to_string();
        match def.as_str() {
            "Count$CastTotalManaSpent" => return Some("Amount::ManaSpentToCast".to_string()),
            "Count$YourLifeTotal" => return Some("Amount::LifeTotal(PlayerRel::You)".to_string()),
            "Count$ValidHand Card.YouOwn" | "Count$ValidHand Card.YouCtrl" => {
                return Some("Amount::HandSize(PlayerRel::You)".to_string());
            }
            "Count$xPaid/Twice" if self.has_x_mana_cost => {
                return Some("Amount::DoubleX".to_string());
            }
            _ => {}
        }
        if let Some(var) = def
            .strip_prefix("SVar$")
            .and_then(|s| s.strip_suffix("/Twice"))
            && self.has_x_mana_cost
            && self.svars.get(var.trim()).map(|s| s.trim()) == Some("Count$xPaid")
        {
            return Some("Amount::DoubleX".to_string());
        }
        if let Some(spec) = def.strip_prefix("Count$ValidGraveyard ") {
            return self.etb_graveyard_count(spec);
        }
        let spec = def
            .strip_prefix("Count$Valid ")
            .or_else(|| def.strip_prefix("Count$LastStateBattlefield "))?;
        if spec.contains('$') {
            return None;
        }
        let (valid, suffix) = match spec.split_once('/') {
            None => (spec, None),
            Some((v, s)) => (v, Some(s)),
        };
        let expr = self.filter_expr(valid.trim())?;
        let name = self.body.filter_static("COUNT", &expr);
        let base = format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}");
        match suffix {
            None => Some(base),
            Some(s) => {
                if let Some(n_str) = s.strip_prefix("Plus.")
                    && let Ok(offset) = n_str.trim().parse::<u32>()
                    && offset > 0
                {
                    return Some(format!(
                        "Amount::Plus {{ base: &{base}, offset: {offset} }}"
                    ));
                }
                if let Some(n_str) = s.strip_prefix("Times.")
                    && let Ok(factor) = n_str.trim().parse::<u32>()
                    && factor > 0
                {
                    return Some(if factor == 1 {
                        base
                    } else {
                        format!("Amount::Times {{ base: &{base}, factor: {factor} }}")
                    });
                }
                None
            }
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
    /// `CastWithFlash`, `Panharmonicon`, `DisableTriggers`) not modeled by
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
        let condition = self.condition(&mut p)?;
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
        let condition = self.condition(&mut p)?;
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

    /// Additional `R:` replacement handlers (`CreateToken`, `AddCounter`).
    pub(super) fn try_extra_replacement(&mut self, spec: &str) -> ExtraOutcome {
        let Some((event, p)) = Params::parse(spec) else {
            return ExtraOutcome::Pass;
        };
        match event.as_str() {
            "CreateToken" => ExtraOutcome::Handled(self.replace_create_token(p)),
            "AddCounter" => ExtraOutcome::Handled(self.replace_add_counter(p)),
            _ => ExtraOutcome::Pass,
        }
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
