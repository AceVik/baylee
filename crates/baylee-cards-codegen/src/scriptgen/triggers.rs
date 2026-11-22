//! Reading triggered abilities, delayed ones, and what happens as a
//! permanent enters.

use super::{
    BlockHalf, BlockRole, Bound, Chain, Params, Tx, activated::OptionalTriggerPrice,
    card_type_const, color_word, count_bound, counter_kind, keyword_const, pump_amount,
};
use baylee_core::types::SubtypeKind;

enum LegacyPhase {
    Handled(String),
    Fallthrough,
    Refused,
}

enum RelativePresent {
    Matched(String),
    NotApplicable,
    Denied,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EmblemKind {
    Static,
    Trigger,
    Activated,
}

impl Tx<'_> {
    /// `T:Mode$ SpellCast` — "whenever a player casts a spell".
    ///
    /// One printed sentence and two script keys, because the reference asks
    /// *what* was cast and *who* cast it separately. `Trigger::SpellCast`
    /// carries one filter, which is the right shape — a spell on the stack
    /// is controlled by the player who cast it — so the two keys are joined
    /// by appending the controller atom to every alternative and letting
    /// [`Tx::filter_expr`] compose it: `Instant,Sorcery` with `You` is
    /// `Instant.YouCtrl,Sorcery.YouCtrl`. The atom is spelled the way a
    /// script would have spelled it rather than built here, so the two
    /// spellings cannot drift.
    #[allow(clippy::too_many_lines)]
    pub(super) fn spell_cast_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        // **An absent `TriggerZones$` is not the battlefield here.** Most
        // zoneless `SpellCast` lines write `ValidCard$ Card.Self` — "when you
        // cast this spell", which fires on the stack via
        // `cast_this_spell_triggers` for `Trigger::SpellCast(&Filter::This)`
        // (CR 113.6k). Any other zoneless `SpellCast` stays refused.
        if !zoned {
            if p.peek("ValidCard") == Some("Card.Self")
                && matches!(p.peek("ValidActivatingPlayer"), None | Some("You"))
                && !p.has("ActivatorThisTurnCast")
                && !p.has("TargetsValid")
                && !p.has("ValidSA")
                && !p.has("Static")
            {
                p.take("ValidCard");
                p.take("ValidActivatingPlayer");
                return Some("Trigger::SpellCast(&Filter::This)".to_string());
            }
            return self.deny("a `SpellCast` trigger with no `TriggerZones$`".to_string());
        }
        let valid = match p.take("ValidCard") {
            Some(valid) => valid,
            None if !p.has("ValidSA") => "Card".to_string(),
            None => return self.deny("a `SpellCast` trigger with no `ValidCard$`".to_string()),
        };
        // The same sentence, refused a second way: a cast trigger
        // is one whatever zone it claims.
        if valid.split(['.', '+']).any(|atom| atom == "Self") {
            return self.deny("a `SpellCast` trigger on the card itself".to_string());
        }
        // Guard against a script narrowing a player-wide kicker trigger
        // ("Whenever a player kicks a spell") to `YouCtrl`.
        if valid.split(['.', '+', ',']).any(|atom| atom == "YouCtrl")
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("whenever a player kicks")
        {
            return self.deny(
                "a `SpellCast` trigger restricting a player-wide kicker trigger to `YouCtrl`"
                    .to_string(),
            );
        }
        // An absent `ValidActivatingPlayer$` is every player, which
        // is `Player`'s own meaning — 220 lines write nothing and
        // 39 write the word.
        let whose = match p.take("ValidActivatingPlayer").as_deref() {
            None | Some("Player") => "",
            Some("You") => ".YouCtrl",
            Some("Opponent" | "Player.Opponent") => ".OppCtrl",
            Some(other) => {
                return self.deny(format!("a spell cast by `{other}`"));
            }
        };
        if p.peek("ValidSA") == Some("Spell.Warp")
            && !p.has("TargetsValid")
            && !p.has("ActivatorThisTurnCast")
            && !p.has("Static")
        {
            p.take("ValidSA");
            let who = match whose {
                "" => "PlayerRel::EachPlayer",
                ".YouCtrl" => "PlayerRel::You",
                ".OppCtrl" => "PlayerRel::EachOpponent",
                _ => unreachable!("player relation was checked above"),
            };
            let filter = if valid == "Card" {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(&valid, true)?;
                format!("&{}", self.body.filter_static("TRIGGER", &expr))
            };
            return Some(format!(
                "Trigger::SpellWarped {{ who: {who}, filter: {filter} }}"
            ));
        }
        if let Some(targets_valid) = p.take("TargetsValid") {
            if p.has("ActivatorThisTurnCast") {
                return self.deny(
                    "a `SpellCast` trigger with both `TargetsValid$` and `ActivatorThisTurnCast$`"
                        .to_string(),
                );
            }
            return self.spell_cast_targeting_trigger(&valid, whose, &targets_valid, p);
        }
        if let Some(count) = p.take("ActivatorThisTurnCast") {
            if count == "GT1" && valid == "Instant" {
                let who = match whose {
                    "" => "PlayerRel::EachPlayer",
                    ".YouCtrl" => "PlayerRel::You",
                    ".OppCtrl" => "PlayerRel::EachOpponent",
                    _ => unreachable!("player relation was checked above"),
                };
                return Some(format!("Trigger::InstantCastAfterFirst {{ who: {who} }}"));
            }
            if count == "EQ1"
                && matches!(
                    valid.as_str(),
                    "Card.NonCreature" | "Card.NonCreature+YouCtrl" | "Card.NonCreature+OppCtrl"
                )
            {
                let who = match (valid.as_str(), whose) {
                    ("Card.NonCreature", "") => "PlayerRel::EachPlayer",
                    ("Card.NonCreature", ".YouCtrl")
                    | ("Card.NonCreature+YouCtrl", "" | ".YouCtrl") => "PlayerRel::You",
                    ("Card.NonCreature", ".OppCtrl")
                    | ("Card.NonCreature+OppCtrl", "" | ".OppCtrl") => "PlayerRel::EachOpponent",
                    _ => return self.deny(format!("cast history `{valid}` compared `{count}`")),
                };
                return Some(format!("Trigger::FirstNoncreatureSpellCast({who})"));
            }
            if let Some(n) = count
                .strip_prefix("EQ")
                .and_then(|s| s.parse::<u8>().ok())
                .filter(|&n| n >= 1)
            {
                let filter = match (valid.as_str(), whose) {
                    ("Card", "") => "&Filter::Any",
                    ("Card", ".YouCtrl") | ("Card.YouCtrl", "" | ".YouCtrl") => {
                        "&Filter::ControlledByYou"
                    }
                    ("Card", ".OppCtrl") | ("Card.OppCtrl", "" | ".OppCtrl") => {
                        "&Filter::ControlledByOpponent"
                    }
                    _ => return self.deny(format!("cast history `{valid}` compared `{count}`")),
                };
                return Some(format!(
                    "Trigger::NthSpellCast {{ n: {n}, filter: {filter} }}"
                ));
            }
            return self.deny(format!("cast history `{valid}` compared `{count}`"));
        }
        let valid = if whose.is_empty() {
            valid
        } else {
            valid
                .split(',')
                .map(|alt| format!("{alt}{whose}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        // CR 110.4b: spells on the stack are not permanents, so `"Permanent"`
        // must keep its permanent-card type union (`card_zone = true`) rather
        // than collapsing away as on the battlefield.
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::SpellCast(&{filter})"))
    }

    /// `T:Mode$ SpellCastOrCopy` — magecraft ("whenever a player casts or
    /// copies a matching spell", CR 707.10).
    fn spell_cast_or_copy_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        if !zoned {
            return self.deny("a `SpellCastOrCopy` trigger with no `TriggerZones$`".to_string());
        }
        let valid = match p.take("ValidCard") {
            Some(valid) => valid,
            None if p.has("ValidActivatingPlayer") => "Card".to_string(),
            None => {
                return self.deny("a `SpellCastOrCopy` trigger with no `ValidCard$`".to_string());
            }
        };
        if valid.split(['.', '+', ',']).any(|atom| atom == "Self") {
            return self.deny("a `SpellCastOrCopy` trigger on the card itself".to_string());
        }
        let whose = match p.take("ValidActivatingPlayer").as_deref() {
            None | Some("Player") => "",
            Some("You") => ".YouCtrl",
            Some("Opponent" | "Player.Opponent") => ".OppCtrl",
            Some(other) => {
                return self.deny(format!("a spell cast or copied by `{other}`"));
            }
        };
        let valid = if whose.is_empty() {
            valid
        } else {
            valid
                .split(',')
                .map(|alt| format!("{alt}{whose}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::SpellCastOrCopied(&{filter})"))
    }

    #[allow(clippy::too_many_lines)]
    fn spell_cast_targeting_trigger(
        &mut self,
        valid: &str,
        whose: &str,
        targets_valid: &str,
        p: &mut Params,
    ) -> Option<String> {
        let only = match p.take("IsSingleTarget").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`SpellCast` with `IsSingleTarget$ {other}`")),
        };
        let who = match whose {
            "" => "PlayerRel::EachPlayer",
            ".YouCtrl" => "PlayerRel::You",
            ".OppCtrl" => "PlayerRel::EachOpponent",
            _ => unreachable!("player relation was checked above"),
        };
        let mut targets_you = false;
        let mut obj_alts = Vec::new();
        for alt in targets_valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            if alt == "You" {
                targets_you = true;
                continue;
            }
            if alt
                .split(['.', '+'])
                .any(|atom| matches!(atom, "You" | "Player" | "Opponent"))
            {
                return self.deny(format!("a `SpellCast` trigger targeting `{alt}`"));
            }
            let (noun, rest) = match alt.split_once('.') {
                Some((n, r)) => (n.trim(), Some(r)),
                None => (alt, None),
            };
            if noun.starts_with("inZone") {
                return self.deny(format!("a `SpellCast` trigger targeting `{alt}`"));
            }
            let cleaned = if let Some(rest) = rest {
                let mut kept = Vec::new();
                for atom in rest.split('+').map(str::trim) {
                    if atom == "inZoneBattlefield" {
                        continue;
                    }
                    if atom.starts_with("inZone") {
                        return self.deny(format!("a `SpellCast` trigger targeting `{alt}`"));
                    }
                    if !atom.is_empty() {
                        kept.push(atom);
                    }
                }
                if kept.is_empty() {
                    noun.to_string()
                } else {
                    format!("{noun}.{}", kept.join("+"))
                }
            } else {
                noun.to_string()
            };
            obj_alts.push(cleaned);
        }
        if obj_alts.is_empty() && !targets_you {
            return None;
        }
        let target_opt = if obj_alts.is_empty() {
            "None".to_string()
        } else if obj_alts.len() == 1 && obj_alts[0] == "Card.Self" {
            "Some(&Filter::This)".to_string()
        } else {
            let joined = obj_alts.join(",");
            let expr = self.filter_expr(&joined)?;
            if expr == "Filter::This" {
                "Some(&Filter::This)".to_string()
            } else {
                let filter = self.body.filter_static("TRIGGER", &expr);
                format!("Some(&{filter})")
            }
        };
        let spell_ref = match (valid, whose) {
            ("Card", _) | ("Card.YouCtrl", ".YouCtrl") | ("Card.OppCtrl", ".OppCtrl") => {
                "&Filter::Any".to_string()
            }
            _ => {
                let expr = self.filter_expr_in_zone(valid, true)?;
                format!("&{}", self.body.filter_static("TRIGGER", &expr))
            }
        };
        if who == "PlayerRel::You"
            && spell_ref == "&Filter::Any"
            && target_opt == "Some(&Filter::This)"
            && !targets_you
            && !only
        {
            return Some("Trigger::HEROIC".to_string());
        }
        Some(format!(
            "Trigger::SpellCastTargeting {{ who: {who}, spell: {spell_ref}, target: {target_opt}, \
             targets_you: {targets_you}, only: {only} }}"
        ))
    }

    fn enlisted_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("an `Enlisted` trigger with no `ValidCard$`".to_string());
        };
        if valid == "Card.Self" {
            return Some("Trigger::ENLISTED_THIS".to_string());
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::Enlisted(&{filter})"))
    }

    fn exploited_trigger(&mut self, p: &mut Params) -> Option<String> {
        let exploiter = match p.take("ValidSource").as_deref() {
            None => "&Filter::Any".to_string(),
            Some("Card.Self") => "&Filter::This".to_string(),
            Some(valid) => {
                let expr = self.filter_expr(valid)?;
                format!("&{}", self.body.filter_static("TRIGGER", &expr))
            }
        };
        let exploited = match p.take("ValidCard").as_deref() {
            None | Some("Creature") => "&Filter::Any".to_string(),
            Some(valid) => {
                let expr = self.filter_expr(valid)?;
                format!("&{}", self.body.filter_static("TRIGGER", &expr))
            }
        };
        if exploiter == "&Filter::This" && exploited == "&Filter::Any" {
            return Some("Trigger::EXPLOITED_THIS".to_string());
        }
        Some(format!(
            "Trigger::Exploited {{ exploiter: {exploiter}, exploited: {exploited} }}"
        ))
    }

    fn scry_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `Scry` trigger for `{other}`")),
        };
        Some(format!("Trigger::Scried {{ who: {who} }}"))
    }

    fn becomes_monstrous_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("ValidCard").as_deref() {
            None | Some("Card.Self") => Some("Trigger::BecomesMonstrous".to_string()),
            Some(other) => self.deny(format!("a `BecomeMonstrous` trigger on `{other}`")),
        }
    }

    fn plotted_trigger(&mut self, p: &mut Params) -> Option<String> {
        let valid_player = p.take("ValidPlayer");
        let who = match valid_player.as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `Plotted` trigger for `{other}`")),
        };
        let valid = match p.take("ValidCard") {
            Some(valid) => valid,
            None if valid_player.is_some() => "Card".to_string(),
            None => return self.deny("a `Plotted` trigger with no `ValidCard$`".to_string()),
        };
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else if valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&valid, true)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!(
            "Trigger::Plotted {{ who: {who}, filter: {filter} }}"
        ))
    }

    fn mutates_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("ValidCard").as_deref() {
            Some("Card.Self") => Some("Trigger::Mutates".to_string()),
            Some(other) => self.deny(format!("`Mutates` trigger on `{other}`")),
            None => self.deny("a `Mutates` trigger with no `ValidCard$`".to_string()),
        }
    }

    fn ability_cast_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        if !zoned {
            return self.deny("an `AbilityCast` trigger with no `TriggerZones$`".to_string());
        }
        let who = match p.take("ValidActivatingPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("an `AbilityCast` trigger by `{other}`")),
        };
        let (tap_cost, mana_ability) = match p.take("ValidSA").as_deref() {
            None | Some("SpellAbility" | "Activated") => ("None", "None"),
            Some("SpellAbility.!ManaAbility" | "Activated.!ManaAbility") => ("None", "Some(false)"),
            Some("SpellAbility.ManaAbility" | "Activated.ManaAbility") => ("None", "Some(true)"),
            Some(
                "SpellAbility.!ManaAbility+hasTapCost"
                | "Activated.!ManaAbility+hasTapCost"
                | "SpellAbility.hasTapCost+!ManaAbility"
                | "Activated.hasTapCost+!ManaAbility",
            ) => ("Some(true)", "Some(false)"),
            Some(
                "SpellAbility.!ManaAbility+!hasTapCost"
                | "Activated.!ManaAbility+!hasTapCost"
                | "SpellAbility.!hasTapCost+!ManaAbility"
                | "Activated.!hasTapCost+!ManaAbility",
            ) => ("Some(false)", "Some(false)"),
            Some("SpellAbility.!hasTapCost" | "Activated.!hasTapCost") => ("Some(false)", "None"),
            Some("SpellAbility.hasTapCost" | "Activated.hasTapCost") => ("Some(true)", "None"),
            Some(other) => return self.deny(format!("`AbilityCast` with `ValidSA$ {other}`")),
        };
        let filter = match p.take("ValidCard").as_deref() {
            None | Some("Card" | "Permanent") => "&Filter::Any".to_string(),
            Some("Card.Self") => "&Filter::This".to_string(),
            Some(valid) => {
                let (cleaned, require_bf) = self.strip_ability_cast_battlefield_zone(valid)?;
                let mut expr = self.filter_expr(&cleaned)?;
                if require_bf {
                    expr = super::on_the_battlefield(&expr);
                }
                format!("&{}", self.body.filter_static("TRIGGER", &expr))
            }
        };
        Some(format!(
            "Trigger::AbilityActivated {{ filter: {filter}, who: {who}, tap_cost: {tap_cost}, \
             mana_ability: {mana_ability} }}"
        ))
    }

    fn strip_ability_cast_battlefield_zone(&self, valid: &str) -> Option<(String, bool)> {
        let mut saw_bf = false;
        let mut all_bf = true;
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            if base.starts_with("inZone") || base.starts_with("inRealZone") {
                return self.deny(format!("`AbilityCast` source `{valid}`"));
            }
            let mut alt_bf = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if matches!(atom, "inZoneBattlefield" | "inRealZoneBattlefield") {
                    alt_bf = true;
                    saw_bf = true;
                } else if atom.starts_with("inZone") || atom.starts_with("inRealZone") {
                    return self.deny(format!("`AbilityCast` source `{valid}`"));
                } else {
                    kept.push(atom);
                }
            }
            if !alt_bf {
                all_bf = false;
            }
            if kept.is_empty() {
                cleaned_alts.push(base.to_string());
            } else {
                cleaned_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        if saw_bf && !all_bf {
            return self.deny(format!("mixed zones in `AbilityCast` source `{valid}`"));
        }
        Some((cleaned_alts.join(","), saw_bf))
    }

    fn commit_crime_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `CommitCrime` trigger for `{other}`")),
        };
        let once_per_turn = if p.peek("ActivationLimit") == Some("1") {
            p.take("ActivationLimit");
            true
        } else {
            false
        };
        if who == "PlayerRel::You" {
            Some(
                if once_per_turn {
                    "Trigger::COMMITTED_CRIME_ONCE_PER_TURN"
                } else {
                    "Trigger::COMMITTED_CRIME"
                }
                .to_string(),
            )
        } else {
            Some(format!(
                "Trigger::CommittedCrime {{ who: {who}, once_per_turn: {once_per_turn} }}"
            ))
        }
    }

    fn mana_expend_trigger(&mut self, p: &mut Params) -> Option<String> {
        let player_a = p.take("Player");
        let player_b = p.take("ValidPlayer");
        if player_a.is_some() && player_b.is_some() {
            return self.deny("`ManaExpend` with both `Player$` and `ValidPlayer$`".to_string());
        }
        let who = match player_a.or(player_b).as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `ManaExpend` trigger for `{other}`")),
        };
        let Some(amount_raw) = p.take("Amount") else {
            return self.deny("a `ManaExpend` trigger with no `Amount$`".to_string());
        };
        let Some(threshold) = amount_raw.trim().parse::<u32>().ok().filter(|&n| n >= 1) else {
            return self.deny(format!("`ManaExpend` threshold `{amount_raw}`"));
        };
        Some(format!(
            "Trigger::Expended {{ who: {who}, threshold: {threshold} }}"
        ))
    }

    fn ring_tempts_you_trigger(&mut self, p: &mut Params) -> Option<String> {
        if let Some(valid) = p.peek("ValidCard") {
            return self.deny(format!(
                "a `RingTemptsYou` trigger with `ValidCard$ {valid}`"
            ));
        }
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `RingTemptsYou` trigger for `{other}`")),
        };
        Some(format!("Trigger::RingTemptsYou {{ who: {who} }}"))
    }

    fn clashed_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `Clashed` trigger for `{other}`")),
        };
        let won_only = match p.take("Won").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Clashed` with `Won$ {other}`")),
        };
        Some(format!(
            "Trigger::Clashed {{ who: {who}, won_only: {won_only} }}"
        ))
    }

    fn firebends_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("a `Firebend` trigger for `{other}`")),
        };
        Some(format!("Trigger::Firebends({who})"))
    }

    /// `T:Mode$ DamageDone` — "whenever this creature deals [combat] damage
    /// to [a player | an opponent]" — and `DamageDoneOnce`, "whenever this
    /// creature is dealt damage".
    ///
    /// `DamageDone` needs a source and a player target: `Player` in combat
    /// is `DealsCombatDamageToPlayer`, `Opponent` is
    /// `DealsCombatDamageToOpponent` in combat and `DealsDamageToOpponent`
    /// out of it (Hypnotic Specter). Damage to a player out of combat, to a
    /// creature, or to anything else is refused by name.
    ///
    /// `DamageDoneOnce` is the reference's "once however many sources",
    /// which is what the rules make of simultaneous damage (CR 510.2,
    /// 603.2c) and what `Trigger::DealtDamage` does; it is read for a
    /// permanent dealt damage by anything, and refused for a player or with
    /// a source or combat named.
    pub(super) fn damage_trigger(&mut self, p: &mut Params, once: bool) -> Option<String> {
        let target = p.take("ValidTarget");
        let source = p.take("ValidSource");
        let combat = match p.take("CombatDamage").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`CombatDamage$ {other}`")),
        };
        let filter_of = |this: &mut Self, valid: &str| -> Option<String> {
            if valid == "Card.Self" {
                return Some("&Filter::This".to_string());
            }
            let expr = this.filter_expr(valid)?;
            Some(format!("&{}", this.body.filter_static("TRIGGER", &expr)))
        };
        if once {
            return self.damage_done_once_trigger(source, target, combat);
        }
        let Some(source) = source else {
            return self.deny("a `DamageDone` trigger with no `ValidSource$`".to_string());
        };
        let filter = filter_of(self, &source)?;
        match (target.as_deref(), combat) {
            (Some("Player"), true) => Some(format!("Trigger::DealsCombatDamageToPlayer({filter})")),
            (Some("Player,Planeswalker" | "Planeswalker,Player"), true) => Some(format!(
                "Trigger::DealsCombatDamageToPlayerOrPlaneswalker({filter})"
            )),
            (Some("You"), true) => Some(format!("Trigger::DealsCombatDamageToYou({filter})")),
            (Some("Creature"), true) => {
                Some(format!("Trigger::DealsCombatDamageToCreature({filter})"))
            }
            (Some("Opponent" | "Player.Opponent"), true) => {
                Some(format!("Trigger::DealsCombatDamageToOpponent({filter})"))
            }
            (Some("Opponent" | "Player.Opponent"), false) => {
                Some(format!("Trigger::DealsDamageToOpponent({filter})"))
            }
            (Some("You"), false) => Some(format!(
                "Trigger::DealsDamageToPlayer {{ source: {filter}, player: PlayerRel::You }}"
            )),
            (Some("Player"), false) => Some(format!(
                "Trigger::DealsDamageToPlayer {{ source: {filter}, player: PlayerRel::EachPlayer }}"
            )),
            (Some(tgt), false)
                if !tgt
                    .split([',', '.', '+'])
                    .any(|w| matches!(w.trim(), "Player" | "You" | "Opponent" | "Self")) =>
            {
                let recipient = if tgt == "Creature" {
                    "&Filter::CREATURE".to_string()
                } else {
                    filter_of(self, tgt)?
                };
                Some(format!(
                    "Trigger::DealsDamageToObject {{ source: {filter}, recipient: {recipient} }}"
                ))
            }
            (other, _) => self.deny(format!(
                "damage dealt to `{}`{}",
                other.unwrap_or("anything"),
                if combat { " in combat" } else { "" }
            )),
        }
    }

    fn damage_done_once_trigger(
        &mut self,
        source: Option<String>,
        target: Option<String>,
        combat: bool,
    ) -> Option<String> {
        let Some(target) = target else {
            return self.deny("a `DamageDoneOnce` trigger with no `ValidTarget$`".to_string());
        };
        let player_rel = match target.as_str() {
            "You" => Some("PlayerRel::You"),
            "Player" => Some("PlayerRel::EachPlayer"),
            "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
            _ => None,
        };
        if let Some(source) = source {
            if combat && target == "Player" {
                let filter = if source == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&source)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                return Some(format!(
                    "Trigger::OneOrMoreDealCombatDamageToPlayer({filter})"
                ));
            }
            if !combat
                && source == "Card"
                && let Some(who) = player_rel
            {
                return Some(format!("Trigger::PlayerDealtDamage({who})"));
            }
            return self.deny("a `DamageDoneOnce` trigger naming its source".to_string());
        }
        if !combat && let Some(who) = player_rel {
            return Some(format!("Trigger::PlayerDealtDamage({who})"));
        }
        if target
            .split(['.', ','])
            .any(|w| matches!(w, "You" | "Player" | "Opponent"))
        {
            return self.deny(format!("damage dealt to `{target}` at once"));
        }
        let filter = if target == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&target)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        let kind = if combat {
            "DealtCombatDamage"
        } else {
            "DealtDamage"
        };
        Some(format!("Trigger::{kind}({filter})"))
    }

    /// `T:Mode$ DamageDealtOnce` — "whenever a creature you control deals
    /// combat damage" (Noble Purpose) or "whenever [this] deals damage"
    /// (El-Hajjâj): once per damage event or combat-damage step per source.
    pub(super) fn damage_dealt_once_trigger(&mut self, p: &mut Params) -> Option<String> {
        let combat = match p.take("CombatDamage").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`CombatDamage$ {other}`")),
        };
        match p.take("ValidTarget").as_deref() {
            None | Some("Player,Permanent" | "Permanent,Player") => {}
            Some(target) => {
                return self.deny(format!("`DamageDealtOnce` naming `ValidTarget$ {target}`"));
            }
        }
        let Some(source) = p.take("ValidSource") else {
            return self.deny("a `DamageDealtOnce` trigger with no `ValidSource$`".to_string());
        };
        let filter = if source == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&source)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        let kind = if combat {
            "DealsCombatDamage"
        } else {
            "DealsDamage"
        };
        Some(format!("Trigger::{kind}({filter})"))
    }

    /// `T:Mode$ TapsForMana` — "whenever a Mountain is tapped for mana"
    /// (Gauntlet of Might), "whenever a player taps a land for mana"
    /// (Manabarbs): CR 106.12a. No `Activator$` is anybody, as the two
    /// sentences say. `Static$ True` is the reference's mark on the ones that
    /// resolve at once, which the engine derives from the ability's shape
    /// instead (CR 605.1b) and so needs no word for.
    pub(super) fn taps_for_mana_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `TapsForMana` trigger with no `ValidCard$`".to_string());
        };
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        let by = match p.take("Activator").as_deref() {
            None => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent") => "PlayerRel::EachOpponent",
            Some(other) => {
                return self.deny(format!("a permanent tapped for mana by `{other}`"));
            }
        };
        match p.take("Static").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Static$ {other}`")),
        }
        Some(format!(
            "Trigger::TappedForMana {{ by: {by}, filter: {filter} }}"
        ))
    }

    /// `T:Mode$ …` as a `Trigger` expression.
    pub(super) fn trigger_expr(&mut self, p: &mut Params, mode: &str) -> Option<String> {
        self.trigger_expr_in_zone(p, mode, false)
    }

    fn trigger_expr_in_zone(
        &mut self,
        p: &mut Params,
        mode: &str,
        zone_taken: bool,
    ) -> Option<String> {
        let zones = p.take("TriggerZones");
        if !zone_taken {
            match zones.as_deref() {
                None | Some("Battlefield") => {}
                Some(zones) => return self.deny(format!("`TriggerZones$ {zones}`")),
            }
        }
        let zoned = zone_taken || zones.is_some();
        match mode {
            "AttackerBlockedByCreature" => self.block_trigger(p),
            "AttackerBlocked" => self.attacker_blocked_trigger(p),
            "ChangesZone" => self.changes_zone_trigger(p),
            "Phase" => self.phase_step_trigger(p),
            "Attacks" => self.attacks_trigger(p),
            "AttackersDeclared" => self.attackers_declared_trigger(p),
            "AttackerUnblocked" => self.attacker_unblocked_trigger(p),
            "Blocks" => self.blocks_trigger(p),
            "BecomesTarget" => self.becomes_target_trigger(p),
            "SpellCast" => self.spell_cast_trigger(p, zoned),
            "TapsForMana" => self.taps_for_mana_trigger(p),
            "DamageDone" => self.damage_trigger(p, false),
            "DamageDoneOnce" => self.damage_trigger(p, true),
            "DamageDealtOnce" => self.damage_dealt_once_trigger(p),
            // "Whenever [a permanent] becomes tapped": the source (City of
            // Brass), or any permanent the filter matches (Lifetap).
            "Taps" | "Untaps" => {
                let untap = mode == "Untaps";
                let Some(valid) = p.take("ValidCard") else {
                    return self.deny("a tap-state trigger with no `ValidCard$`".to_string());
                };
                if valid == "Card.Self" {
                    return Some(if untap {
                        "Trigger::BecomesUntapped(&Filter::This)".to_string()
                    } else {
                        "Trigger::BecomesTapped(&Filter::This)".to_string()
                    });
                }
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                let variant = if untap {
                    "BecomesUntapped"
                } else {
                    "BecomesTapped"
                };
                Some(format!("Trigger::{variant}(&{filter})"))
            }
            "LandPlayed" => self.land_played_trigger(p),
            "Drawn" => self.drawn_trigger(p, zoned),
            "TurnFaceUp" => self.turn_face_up_trigger(p),
            "Transformed" => self.transformed_trigger(p),
            "Cycled" => self.cycled_trigger(p, zoned),
            "Discarded" => self.discarded_trigger(p, zoned),
            "LifeGained" => self.life_change_trigger(p, false),
            "LifeLost" => self.life_change_trigger(p, true),
            "SearchedLibrary" => self.searched_library_trigger(p),
            "CounterAddedOnce" => self.counter_added_once_trigger(p),
            "PhaseIn" => self.phase_trigger(p, false),
            "PhaseOut" => self.phase_trigger(p, true),
            "FlippedCoin" => self.flipped_coin_trigger(p),
            "RolledDie" => self.rolled_die_trigger(p),
            "Exerted" => self.exerted_trigger(p),
            "Enlisted" => self.enlisted_trigger(p),
            "Exploited" => self.exploited_trigger(p),
            "Scry" => self.scry_trigger(p),
            "BecomeMonstrous" | "BecomesMonstrous" => self.becomes_monstrous_trigger(p),
            "BecomesPlotted" | "Plotted" => self.plotted_trigger(p),
            "SpellCastOrCopy" => self.spell_cast_or_copy_trigger(p, zoned),
            // "Whenever you give a gift" (CR 702.174c). `EventPlayer` is the
            // recipient, so `TriggeredPlayer` stays unread here.
            "GiveGift" | "GaveGift" => {
                let who = match p.take("ValidPlayer").as_deref() {
                    None | Some("Player") => "PlayerRel::EachPlayer",
                    Some("You") => "PlayerRel::You",
                    Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                    Some(other) => return self.deny(format!("a gift given by `{other}`")),
                };
                Some(format!("Trigger::GaveGift({who})"))
            }
            "ChangesZoneAll" => self.changes_zone_all_trigger(p),
            "Sacrificed" => self.sacrificed_trigger(p),
            "Attacked" => self.player_attacked_trigger(p),
            "AttackersDeclaredOneTarget" => self.attackers_declared_one_target_trigger(p),
            "Mutates" => self.mutates_trigger(p),
            "AbilityCast" => self.ability_cast_trigger(p, zoned),
            "CommitCrime" => self.commit_crime_trigger(p),
            "ManaExpend" => self.mana_expend_trigger(p),
            "RingTemptsYou" => self.ring_tempts_you_trigger(p),
            "Clashed" => self.clashed_trigger(p),
            "Firebend" | "Firebends" => self.firebends_trigger(p),
            "UnlockDoor" => self.unlock_door_trigger(p, zone_taken),
            other => self.deny(format!("trigger mode `{other}`")),
        }
    }

    /// `T:Mode$ UnlockDoor | ThisDoor$ True` — CR 709.5h "When you unlock this
    /// door, …" on a Duskmourn Room split card (`Trigger::UnlockThisDoor(0|1)`).
    fn unlock_door_trigger(&mut self, p: &mut Params, zone_taken: bool) -> Option<String> {
        if zone_taken {
            return self.deny("an `UnlockDoor` trigger outside the battlefield".to_string());
        }
        let Some(door) = self.room_door else {
            return self.deny("an `UnlockDoor` trigger outside a Room split card".to_string());
        };
        match p.take("ValidPlayer").as_deref() {
            None | Some("You") => {}
            Some(other) => return self.deny(format!("`UnlockDoor` player `{other}`")),
        }
        match p.take("ValidCard").as_deref() {
            Some("Card.Self") => {}
            Some(other) => return self.deny(format!("`UnlockDoor` on `{other}`")),
            None => return self.deny("an `UnlockDoor` trigger with no `ValidCard$`".to_string()),
        }
        match p.take("ThisDoor").as_deref() {
            Some("True") => {}
            Some(other) => return self.deny(format!("`UnlockDoor` with `ThisDoor$ {other}`")),
            None => {
                return self.deny("an `UnlockDoor` trigger without `ThisDoor$ True`".to_string());
            }
        }
        Some(format!("Trigger::UnlockThisDoor({door})"))
    }

    fn phase_step_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(phase) = p.take("Phase") else {
            return self.deny("a `Phase` trigger with no `Phase$`".to_string());
        };
        let step = match phase.as_str() {
            "Upkeep" => "StepKind::Upkeep",
            "Draw" => "StepKind::Draw",
            "Main1" => "StepKind::FirstMain",
            "Main2" => {
                if let Some(count) = p.peek("PhaseCount") {
                    return self.deny(format!(
                        "trigger at step `Main2` with `PhaseCount$ {count}`"
                    ));
                }
                "StepKind::SecondMain"
            }
            "Main" => match p.take("PhaseCount").as_deref() {
                Some("1") => "StepKind::FirstMain",
                Some("2") => "StepKind::SecondMain",
                Some(other) => {
                    return self.deny(format!("trigger at step `Main` with `PhaseCount$ {other}`"));
                }
                None => return self.deny("trigger at step `Main`".to_string()),
            },
            "BeginCombat" => "StepKind::CombatBegin",
            "EndCombat" | "End of Combat" => "StepKind::CombatEnd",
            "End of Turn" => "StepKind::End",
            other => return self.deny(format!("trigger at step `{other}`")),
        };
        // No player named is every player's step: "at the beginning of each
        // upkeep" (Verdant Force), "at the beginning of the end step"
        // (Pestilence). The reference writes `ValidPlayer$ You` for "your", and
        // reading its absence as "your" too made Verdant Force a card that made
        // a Saproling on one upkeep in two.
        let valid = p.take("ValidPlayer");
        let whose = match valid.as_deref() {
            None => Some("PlayerRel::EachPlayer"),
            Some("You" | "You.descended") if step == "StepKind::End" => {
                let lower = self.oracle.to_ascii_lowercase();
                if valid.as_deref() == Some("You")
                    && lower.contains("at the beginning of the end step")
                    && !lower.contains("your end step")
                {
                    Some("PlayerRel::EachPlayer")
                } else {
                    Some("PlayerRel::You")
                }
            }
            Some(
                "You.descended" | "You.isMonarch" | "You.hasInitiative" | "You.completedDungeon",
            ) => Some("PlayerRel::You"),
            Some("Player.isMonarch" | "Player.hasInitiative" | "Player.completedDungeon") => {
                Some("PlayerRel::EachPlayer")
            }
            Some(who) => self.player_rel(Some(who)),
        };
        let Some(whose) = whose else {
            return self.deny(format!(
                "trigger for player `{}`",
                valid.unwrap_or_default()
            ));
        };
        Some(format!(
            "Trigger::StepBegin {{ step: {step}, whose: {whose} }}"
        ))
    }

    fn trigger_zone_ref(raw: &str) -> Option<&'static str> {
        match raw {
            "Battlefield" => Some("ZoneRef::Battlefield"),
            "Graveyard" => Some("ZoneRef::Graveyard"),
            "Exile" => Some("ZoneRef::Exile"),
            "Hand" => Some("ZoneRef::Hand"),
            "Library" => Some("ZoneRef::Library"),
            "Stack" => Some("ZoneRef::Stack"),
            "Command" => Some("ZoneRef::Command"),
            _ => None,
        }
    }

    fn changes_zone_all_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCards") else {
            return self.deny("a `ChangesZoneAll` trigger with no `ValidCards$`".to_string());
        };
        let origin_raw = p.take("Origin");
        let origin = match origin_raw.as_deref() {
            None | Some("Any") => None,
            Some(z) => Some(
                Self::trigger_zone_ref(z)
                    .ok_or_else(|| format!("`ChangesZoneAll` origin `{z}`"))
                    .ok()?,
            ),
        };
        let dest_raw = p.take("Destination");
        let destination = match dest_raw.as_deref() {
            None | Some("Any") => None,
            Some(z) => Some(
                Self::trigger_zone_ref(z)
                    .ok_or_else(|| format!("`ChangesZoneAll` destination `{z}`"))
                    .ok()?,
            ),
        };
        if origin.is_none() && destination.is_none() {
            return self.deny("`ChangesZoneAll` from `Any` to `Any`".to_string());
        }
        let mut all_other = true;
        let mut stripped_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut saw_other = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == "Self" || atom == "StrictlySelf" {
                    return self.deny(format!("`ChangesZoneAll` on `{valid}`"));
                }
                if matches!(atom, "Other" | "StrictlyOther") {
                    saw_other = true;
                } else {
                    kept.push(atom);
                }
            }
            if !saw_other {
                all_other = false;
            }
            if kept.is_empty() {
                stripped_alts.push(base.to_string());
            } else {
                stripped_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        let (exclude_self, filter_src) = if all_other && !stripped_alts.is_empty() {
            (true, stripped_alts.join(","))
        } else {
            (false, valid)
        };
        let battlefield_side =
            origin == Some("ZoneRef::Battlefield") || destination == Some("ZoneRef::Battlefield");
        let expr = if battlefield_side {
            self.filter_expr(&filter_src)?
        } else {
            self.filter_expr_in_zone(&filter_src, true)?
        };
        let filter = self.body.filter_static("TRIGGER", &expr);
        let origin_expr = origin.map_or_else(|| "None".to_string(), |z| format!("Some({z})"));
        let dest_expr = destination.map_or_else(|| "None".to_string(), |z| format!("Some({z})"));
        Some(format!(
            "Trigger::ChangesZoneAll {{ origin: {origin_expr}, destination: {dest_expr}, \
             filter: &{filter}, exclude_self: {exclude_self} }}"
        ))
    }

    fn sacrificed_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `Sacrificed` trigger with no `ValidCard$`".to_string());
        };
        // Forge scripts sometimes retain copy-pasted `Origin$ Any | Destination$ Battlefield`
        // metadata on `Mode$ Sacrificed` triggers; sacrifices only ever happen from the
        // battlefield (CR 701.21a), so accept only this exact inert pair.
        if p.peek("Origin") == Some("Any") && p.peek("Destination") == Some("Battlefield") {
            p.take("Origin");
            p.take("Destination");
        }
        let explicit_who = match p.take("ValidPlayer").as_deref() {
            None => None,
            Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some(other) => return self.deny(format!("`Sacrificed` by `{other}`")),
        };
        if valid == "Card.Self" {
            let who = explicit_who.unwrap_or("PlayerRel::EachPlayer");
            return Some(format!(
                "Trigger::Sacrificed {{ who: {who}, filter: &Filter::This }}"
            ));
        }
        let who = if let Some(w) = explicit_who {
            w
        } else {
            let alts: Vec<&str> = valid.split(',').map(str::trim).collect();
            let all_you = !alts.is_empty()
                && alts
                    .iter()
                    .all(|alt| alt.split(['.', '+']).skip(1).any(|a| a.trim() == "YouCtrl"));
            let all_opp = !alts.is_empty()
                && alts
                    .iter()
                    .all(|alt| alt.split(['.', '+']).skip(1).any(|a| a.trim() == "OppCtrl"));
            if all_you {
                "PlayerRel::You"
            } else if all_opp {
                "PlayerRel::EachOpponent"
            } else {
                "PlayerRel::EachPlayer"
            }
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!(
            "Trigger::Sacrificed {{ who: {who}, filter: &{filter} }}"
        ))
    }

    fn attacked_player_rel(raw: &str) -> Option<&'static str> {
        match raw.trim() {
            "You" | "Player.You" => Some("PlayerRel::You"),
            "Player.EnchantedBy" | "EnchantedPlayer" => Some("PlayerRel::EnchantedPlayer"),
            "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
            "Player" => Some("PlayerRel::EachPlayer"),
            _ => None,
        }
    }

    fn player_attacked_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        if let Some(att) = p.take("ValidAttackers")
            && att != "Creature"
        {
            return self.deny(format!("`Attacked` with `ValidAttackers$ {att}`"));
        }
        let target_raw = p.take("ValidPlayer").or_else(|| p.take("AttackedTarget"))?;
        let Some(who) = Self::attacked_player_rel(&target_raw) else {
            return self.deny(format!("`Attacked` target `{target_raw}`"));
        };
        if let Some(attacker) = p.take("AttackingPlayer") {
            let ok = attacker == "Player"
                || (who == "PlayerRel::You"
                    && matches!(attacker.as_str(), "Opponent" | "Player.Opponent"));
            if !ok {
                return self.deny(format!("`Attacked` by `{attacker}`"));
            }
        }
        Some(format!("Trigger::PlayerAttacked({who})"))
    }

    fn attackers_declared_one_target_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let (exec_uses_attacking_player, exec_uses_attacked_or_attackers) = p
            .peek("Execute")
            .map_or((false, false), |name| self.wave25c_exec_attack_refs(name));

        // Preserve existing `Trigger::PlayerAttacked(who)` lowering for plain
        // unfiltered player-attacked triggers (where `EventPlayer` is the
        // attacking player) whenever the body does not reference the attacked
        // defender or the matched attackers group.
        if !p.has("ValidAttackersAmount")
            && !exec_uses_attacked_or_attackers
            && matches!(p.peek("ValidAttackers"), None | Some("Creature"))
            && let Some(target_raw) = p.peek("AttackedTarget")
            && let Some(who) = Self::attacked_player_rel(target_raw)
        {
            let attacker_ok = match p.peek("AttackingPlayer") {
                None | Some("Player") => true,
                Some("Opponent" | "Player.Opponent") => who == "PlayerRel::You",
                Some(_) => false,
            };
            if attacker_ok {
                p.take("ValidAttackers");
                p.take("AttackedTarget");
                p.take("AttackingPlayer");
                return Some(format!("Trigger::PlayerAttacked({who})"));
            }
        }

        // `Trigger::AttackersDeclaredAtTarget` binds `EventPlayer` to the
        // attacked defender (not the attacking player), so refuse bodies that
        // read `TriggeredAttackingPlayer` / `TriggeredPlayer`.
        if exec_uses_attacking_player {
            return self.deny(
                "`AttackersDeclaredOneTarget` body referencing attacking player".to_string(),
            );
        }

        let Some(target_raw) = p.take("AttackedTarget") else {
            return self.deny("`AttackersDeclaredOneTarget` with no `AttackedTarget$`".to_string());
        };
        let valid_attackers = p.take("ValidAttackers");
        let attacking_player = p.take("AttackingPlayer");
        let exact_count: Option<u8> = match p.take("ValidAttackersAmount").as_deref() {
            None => None,
            Some("EQ1") => {
                // `exact_count: Some(1)` checks total attackers declared at
                // that defender (CR 508.3 "attacks a player alone"), which
                // matches Forge's `ValidAttackersAmount$ EQ1` strictly when
                // `ValidAttackers$` does not filter below all of a player's
                // creatures.
                if !matches!(
                    valid_attackers.as_deref(),
                    None | Some("Creature" | "Creature.YouCtrl" | "Creature.OppCtrl")
                ) {
                    return self.deny(format!(
                        "`AttackersDeclaredOneTarget` `ValidAttackersAmount$ EQ1` with filtered \
                         `ValidAttackers$ {}`",
                        valid_attackers.as_deref().unwrap_or_default()
                    ));
                }
                Some(1)
            }
            Some(other) => {
                return self.deny(format!(
                    "`AttackersDeclaredOneTarget` with `ValidAttackersAmount$ {other}`"
                ));
            }
        };

        let (attackers_ref, all_you_ctrl) =
            self.wave25c_attackers_filter(valid_attackers.as_deref(), attacking_player.as_deref())?;
        let Some(attacked) = Self::wave25c_attacked_target_kind(&target_raw, all_you_ctrl) else {
            return self.deny(format!(
                "`AttackersDeclaredOneTarget` target `{target_raw}`"
            ));
        };
        let exact_expr = match exact_count {
            Some(n) => format!("Some({n})"),
            None => "None".to_string(),
        };
        Some(format!(
            "Trigger::AttackersDeclaredAtTarget {{ attackers: {attackers_ref}, \
             attacked: {attacked}, exact_count: {exact_expr} }}"
        ))
    }

    fn wave25c_attackers_filter(
        &mut self,
        valid_attackers: Option<&str>,
        attacking_player: Option<&str>,
    ) -> Option<(String, bool)> {
        let ctrl_atom = match attacking_player {
            None | Some("Player") => None,
            Some("You") => Some("YouCtrl"),
            Some("Opponent" | "Player.Opponent") => Some("OppCtrl"),
            Some(other) => {
                return self.deny(format!(
                    "`AttackersDeclaredOneTarget` with `AttackingPlayer$ {other}`"
                ));
            }
        };
        let raw_attackers = valid_attackers.unwrap_or("Creature");
        let mut combined_alts = Vec::new();
        let mut all_you_ctrl = true;
        for alt in raw_attackers.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let has_you = alt.split(['.', '+']).skip(1).any(|a| a.trim() == "YouCtrl");
            let has_opp = alt.split(['.', '+']).skip(1).any(|a| a.trim() == "OppCtrl");
            let mut s = alt.to_string();
            match ctrl_atom {
                Some("YouCtrl") => {
                    if has_opp {
                        return self.deny(format!(
                            "`AttackersDeclaredOneTarget` conflicting attacker control `{alt}`"
                        ));
                    }
                    if !has_you {
                        let sep = if s.contains('.') { '+' } else { '.' };
                        s.push(sep);
                        s.push_str("YouCtrl");
                    }
                }
                Some("OppCtrl") => {
                    if has_you {
                        return self.deny(format!(
                            "`AttackersDeclaredOneTarget` conflicting attacker control `{alt}`"
                        ));
                    }
                    if !has_opp {
                        let sep = if s.contains('.') { '+' } else { '.' };
                        s.push(sep);
                        s.push_str("OppCtrl");
                    }
                    all_you_ctrl = false;
                }
                _ => {
                    if !has_you {
                        all_you_ctrl = false;
                    }
                }
            }
            combined_alts.push(s);
        }
        let combined = combined_alts.join(",");
        let attackers_ref = if combined == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&combined)?;
            match expr.as_str() {
                "Filter::Any" => "&Filter::Any".to_string(),
                "Filter::This" => "&Filter::This".to_string(),
                "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
                "Filter::YOUR_CREATURE" => "&Filter::YOUR_CREATURE".to_string(),
                "Filter::OPPONENT_CREATURE" => "&Filter::OPPONENT_CREATURE".to_string(),
                _ => format!("&{}", self.body.filter_static("TRIGGER", &expr)),
            }
        };
        Some((attackers_ref, all_you_ctrl))
    }

    fn phase_trigger(&mut self, p: &mut Params, out: bool) -> Option<String> {
        let Some(raw) = p.take("ValidCard") else {
            return self.deny(format!(
                "a `{}` trigger with no `ValidCard$`",
                if out { "PhaseOut" } else { "PhaseIn" }
            ));
        };
        if raw == "Card.Self" || (out && raw == "Card.phasedOutSelf") {
            return Some(if out {
                "Trigger::PHASES_OUT".to_string()
            } else {
                "Trigger::PHASES_IN".to_string()
            });
        }
        let normalized = if out && raw.contains("phasedOutSelf") {
            raw.replace("phasedOutSelf", "Self")
        } else {
            raw
        };
        let expr = self.filter_expr(&normalized)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        let variant = if out { "PhasesOut" } else { "PhasesIn" };
        Some(format!("Trigger::{variant}(&{filter})"))
    }

    fn flipped_coin_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("`FlippedCoin` player `{other}`")),
        };
        let outcome = match p.take("ValidResult").as_deref() {
            None => "None",
            Some("Win") => "Some(true)",
            Some("Lose") => "Some(false)",
            Some(other) => return self.deny(format!("`FlippedCoin` result `{other}`")),
        };
        Some(format!(
            "Trigger::CoinFlipped {{ who: {who}, outcome: {outcome} }}"
        ))
    }

    fn rolled_die_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("`RolledDie` player `{other}`")),
        };
        let min_result = match p.take("ValidResult").as_deref() {
            None => "None".to_string(),
            Some(raw) => {
                let Some(n) = raw.strip_prefix("GE").and_then(|s| s.parse::<u8>().ok()) else {
                    return self.deny(format!("`RolledDie` result `{raw}`"));
                };
                format!("Some({n})")
            }
        };
        Some(format!(
            "Trigger::DieRolled {{ who: {who}, min_result: {min_result} }}"
        ))
    }

    fn exerted_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(raw) = p.take("ValidCard") else {
            return self.deny("an `Exerted` trigger with no `ValidCard$`".to_string());
        };
        if raw == "Card.Self" {
            return Some("Trigger::EXERTED_THIS".to_string());
        }
        let expr = self.filter_expr(&raw)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::Exerted(&{filter})"))
    }

    fn counter_added_once_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CounterAddedOnce` trigger with no `ValidCard$`".to_string());
        };
        let Some(kind_code) = p.take("CounterType") else {
            return self.deny("a `CounterAddedOnce` trigger with no `CounterType$`".to_string());
        };
        let Some(kind) = counter_kind(&kind_code) else {
            return self.deny(format!("a `CounterAddedOnce` trigger for `{kind_code}`"));
        };
        let by = match p.take("ValidSource").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("`CounterAddedOnce` by `{other}`")),
        };
        let first_this_turn = match p.take("FirstTime").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`FirstTime$ {other}`")),
        };
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let mut cleaned = Vec::new();
            for alt in valid.split(',').map(str::trim) {
                let (base, atoms) = alt.split_once('.').unwrap_or((alt, ""));
                let kept: Vec<&str> = atoms
                    .split('+')
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && *a != "inZoneBattlefield")
                    .collect();
                if kept.iter().any(|a| a.starts_with("inZone")) {
                    return self.deny(format!("`CounterAddedOnce` on `{valid}`"));
                }
                if kept.is_empty() {
                    cleaned.push(base.to_string());
                } else {
                    cleaned.push(format!("{base}.{}", kept.join("+")));
                }
            }
            let expr = self.filter_expr(&cleaned.join(","))?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!(
            "Trigger::CountersPlaced {{ filter: {filter}, kind: {kind}, by: {by}, first_this_turn: {first_this_turn} }}"
        ))
    }

    fn becomes_target_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid_target) = p.take("ValidTarget") else {
            return self.deny("a `BecomesTarget` trigger not on `Card.Self`".to_string());
        };
        match p.take("ValidSource").as_deref() {
            None => match valid_target.as_str() {
                "Card.Self" => Some("Trigger::BecomesTarget".to_string()),
                "Card.EnchantedBy" | "Card.AttachedBy" => {
                    Some("Trigger::EnchantedBecomesTarget".to_string())
                }
                _ => self.deny("a `BecomesTarget` trigger not on `Card.Self`".to_string()),
            },
            Some("SpellAbility.OppCtrl") => {
                if valid_target == "Card.Self" {
                    return Some(
                        "Trigger::TargetedByOpponent { filter: &Filter::This, you: false }"
                            .to_string(),
                    );
                }
                let mut you = false;
                let mut perms = Vec::new();
                for alt in valid_target.split(',').map(str::trim) {
                    if alt == "You" {
                        you = true;
                        continue;
                    }
                    let (base, atoms) = alt.split_once('.').unwrap_or((alt, ""));
                    let kept: Vec<&str> = atoms
                        .split('+')
                        .map(str::trim)
                        .filter(|a| !a.is_empty() && *a != "inZoneBattlefield")
                        .collect();
                    if matches!(base, "Player" | "You" | "Opponent")
                        || kept.iter().any(|a| {
                            a.starts_with("inZone") || matches!(*a, "Player" | "You" | "Opponent")
                        })
                    {
                        return self.deny(format!("`BecomesTarget` on `{valid_target}`"));
                    }
                    if kept.is_empty() {
                        perms.push(base.to_string());
                    } else {
                        perms.push(format!("{base}.{}", kept.join("+")));
                    }
                }
                if perms.is_empty() {
                    return self.deny(format!("`BecomesTarget` on `{valid_target}`"));
                }
                let filter = if perms.len() == 1 && perms[0] == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&perms.join(","))?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                Some(format!(
                    "Trigger::TargetedByOpponent {{ filter: {filter}, you: {you} }}"
                ))
            }
            Some(other) => self.deny(format!("`BecomesTarget` with `ValidSource$ {other}`")),
        }
    }

    fn changes_zone_trigger(&mut self, p: &mut Params) -> Option<String> {
        let origin = p
            .take("Origin")
            .or_else(|| (p.peek("Destination") == Some("Battlefield")).then(|| "Any".to_string()));
        let dest = p
            .take("Destination")
            .or_else(|| (origin.as_deref() == Some("Battlefield")).then(|| "Any".to_string()));
        let valid = p.take("ValidCard");
        let (Some(origin), Some(dest), Some(valid)) = (origin, dest, valid) else {
            return self.deny("a `ChangesZone` trigger missing one of its three keys".to_string());
        };
        if origin == "Battlefield" && dest == "Graveyard" && valid == "Creature.DamagedBy" {
            return Some("Trigger::DiesAfterDamageByThis(&Filter::CREATURE)".to_string());
        }
        if origin == "Battlefield" && dest == "Graveyard" && valid.contains("DamagedBy") {
            let mut cleaned = Vec::new();
            let mut all_damaged = true;
            for alt in valid.split(',').map(str::trim) {
                let (base, atoms) = alt.split_once('.').unwrap_or((alt, ""));
                let mut saw = false;
                let mut kept = Vec::new();
                for atom in atoms.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                    if atom == "DamagedBy" {
                        saw = true;
                    } else {
                        kept.push(atom);
                    }
                }
                if !saw {
                    all_damaged = false;
                    break;
                }
                if kept.is_empty() {
                    cleaned.push(base.to_string());
                } else {
                    cleaned.push(format!("{base}.{}", kept.join("+")));
                }
            }
            if all_damaged && !cleaned.is_empty() {
                let expr = self.filter_expr(&cleaned.join(","))?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                return Some(format!("Trigger::DiesAfterDamageByThis(&{filter})"));
            }
        }
        if origin == "Any"
            && dest == "Battlefield"
            && let Some(slot) = valid.strip_prefix("Card.Self+kicked ")
            && let Some(idx) = self.dual_kicker_index(slot)
        {
            self.trigger_self_etb_kicked = true;
            self.trigger_self_etb_kicked_with = Some(idx);
            return Some("Trigger::ETB".to_string());
        }
        if valid == "Card.Self"
            && origin == "Battlefield"
            && matches!(dest.as_str(), "Graveyard" | "Exile" | "Any")
            && p.peek("TriggerController") == Some("TriggeredCardController")
        {
            // CR 603.10a / CR 603.3a: a permanent's own leaves-the-battlefield
            // trigger is already controlled by its last-known battlefield
            // controller.
            p.take("TriggerController");
        }
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        if origin == "Any" && dest == "Battlefield" && valid == "Card.Self+kicked" {
            self.trigger_self_etb_kicked = true;
        }
        match (origin.as_str(), dest.as_str()) {
            // `Trigger::ETB` is the same bytes as the variant with
            // `&Filter::This` in it, and is what the DSL carries the
            // constant for: ninety-nine of the pool's hundred and ten
            // enter-triggers point at the source, so the long form is
            // the rare one and deserves to look rare.
            ("Any", "Battlefield") if filter == "&Filter::This" => Some("Trigger::ETB".to_string()),
            ("Any", "Battlefield") => Some(format!("Trigger::EntersBattlefield({filter})")),
            ("Battlefield", "Graveyard") => Some(format!("Trigger::Dies({filter})")),
            ("Battlefield", "Exile") => Some(format!("Trigger::ExiledFromBattlefield({filter})")),
            ("Graveyard", "Battlefield") if filter == "&Filter::This" => {
                Some("Trigger::EnteredThisFrom(ZoneRef::Graveyard)".to_string())
            }
            ("Battlefield", "Any") => Some(format!("Trigger::LeavesBattlefield({filter})")),
            _ => self.deny(format!("trigger on a move from {origin} to {dest}")),
        }
    }

    fn drawn_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        if !zoned {
            return self.deny("a `Drawn` trigger with no `TriggerZones$`".to_string());
        }
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let valid_player = p.take("ValidPlayer");
        let valid_card = p.take("ValidCard");
        let number = p.take("Number");
        let except_first_in_draw_step = match p.take("FirstCardInDrawStep").as_deref() {
            None => false,
            Some("False") => true,
            Some(other) => {
                return self.deny(format!("`Drawn` with `FirstCardInDrawStep$ {other}`"));
            }
        };
        if valid_card.is_none() && valid_player.is_none() {
            return self.deny("a `Drawn` trigger with no `ValidCard$`".to_string());
        }
        let from_player = match valid_player.as_deref() {
            None => None,
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some(other) => return self.deny(format!("`Drawn` by `{other}`")),
        };
        let from_card = match valid_card.as_deref() {
            None | Some("Card") => None,
            Some("Card.YouCtrl" | "Card.YouOwn") => Some("PlayerRel::You"),
            Some("Card.OppCtrl") => Some(if from_player.is_none() && number.is_none() {
                "PlayerRel::Opponent"
            } else {
                "PlayerRel::EachOpponent"
            }),
            Some("Card.OppOwn") => Some("PlayerRel::EachOpponent"),
            Some(other) => return self.deny(format!("`Drawn` on `{other}`")),
        };
        let who = match (from_player, from_card) {
            (Some(a), Some(b)) if a != b => {
                return self.deny(
                    "conflicting draw players in `ValidPlayer$` and `ValidCard$`".to_string(),
                );
            }
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) => "PlayerRel::EachPlayer",
        };
        if except_first_in_draw_step {
            if number.is_some() {
                return self.deny(
                    "a `Drawn` trigger with both `Number$` and `FirstCardInDrawStep$`".to_string(),
                );
            }
            return Some(format!("Trigger::DrawsExceptFirst({who})"));
        }
        if let Some(raw) = number {
            let Some(n) = raw.trim().parse::<u8>().ok().filter(|&n| n >= 1) else {
                return self.deny(format!("`Drawn` with `Number$ {raw}`"));
            };
            return Some(format!(
                "Trigger::NthCardDrawnInTurn {{ who: {who}, n: {n} }}"
            ));
        }
        Some(format!("Trigger::Draws({who})"))
    }

    fn land_played_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `LandPlayed` trigger with no `ValidCard$`".to_string());
        };
        match valid.as_str() {
            "Land" => Some("Trigger::PlaysLand(PlayerRel::EachPlayer)".to_string()),
            "Land.YouCtrl" => Some("Trigger::PlaysLand(PlayerRel::You)".to_string()),
            "Land.OppCtrl" => Some("Trigger::PlaysLand(PlayerRel::EachOpponent)".to_string()),
            "Land.Other+YouCtrl" => Some("Trigger::PlaysAnotherLand".to_string()),
            other => self.deny(format!("`LandPlayed` on `{other}`")),
        }
    }

    fn attacker_blocked_trigger(&mut self, p: &mut Params) -> Option<String> {
        let secondary = match p.take("Secondary").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        };
        let valid = p.take("ValidCard");
        if valid.as_deref() != Some("Card.Self") {
            if let Some(blocker) = p.take("ValidBlocker") {
                if blocker != "Card.Self" {
                    return self
                        .deny("an `AttackerBlocked` trigger not on `Card.Self`".to_string());
                }
                let attacker = valid.as_deref().unwrap_or("Creature");
                if attacker.split([',', '.', '+']).any(|atom| {
                    matches!(
                        atom.trim(),
                        "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
                    )
                }) {
                    return self.deny(format!(
                        "an `AttackerBlocked` trigger blocking `{attacker}`, relative to another card"
                    ));
                }
                let filter = if attacker == "Creature" {
                    "&Filter::CREATURE".to_string()
                } else {
                    let expr = self.filter_expr(attacker)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                return Some(format!("Trigger::BlocksCreature({filter})"));
            }
            let Some(valid) = valid else {
                return self.deny("an `AttackerBlocked` trigger with no `ValidCard$`".to_string());
            };
            let expr = self.filter_expr(&valid)?;
            let filter = self.body.filter_static("TRIGGER", &expr);
            return Some(format!("Trigger::ControlledBecomesBlocked(&{filter})"));
        }
        match p.take("ValidBlocker") {
            None => Some("Trigger::BecomesBlocked".to_string()),
            Some(blocker) => {
                if secondary
                    && self.block_half.as_ref().is_some_and(|half| {
                        half.one_or_more
                            && half.role == BlockRole::Blocks
                            && !half.secondary
                            && half.other == blocker
                    })
                {
                    let expr = self.filter_expr(&blocker)?;
                    let filter = self.body.filter_static("TRIGGER", &expr);
                    self.block_line = Some((BlockRole::Blocked, blocker, true));
                    self.block_line_one_or_more = true;
                    return Some(format!(
                        "Trigger::BlocksOrBecomesBlockedByOneOrMore(&{filter})"
                    ));
                }
                let expr = self.filter_expr(&blocker)?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                Some(format!("Trigger::BecomesBlockedBy(&{filter})"))
            }
        }
    }

    fn attacks_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("an `Attacks` trigger with no `ValidCard$`".to_string());
        };
        let alone = match p.take("Alone").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Alone$ {other}`")),
        };
        let valid = match p.take("Attacked").as_deref() {
            None => valid,
            Some("You") if valid != "Card.Self" && !alone => valid
                .split(',')
                .map(|alt| format!("{alt}.attackingYou"))
                .collect::<Vec<_>>()
                .join(","),
            Some(other) => return self.deny(format!("an attack against `{other}`")),
        };
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        if alone {
            Some(format!("Trigger::AttacksAlone({filter})"))
        } else {
            Some(format!("Trigger::Attacks({filter})"))
        }
    }

    fn attackers_declared_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let attacking_player = p.take("AttackingPlayer");
        let valid_attackers = p.take("ValidAttackers");
        let attacked_target = p.take("AttackedTarget");
        if let Some(amount_raw) = p.take("ValidAttackersAmount") {
            if attacked_target.is_some() || attacking_player.as_deref() != Some("You") {
                return self.deny(format!(
                    "`AttackersDeclared` with `ValidAttackersAmount$ {amount_raw}`"
                ));
            }
            let Some(n) = amount_raw
                .strip_prefix("GE")
                .and_then(|s| s.parse::<u8>().ok())
                .filter(|&n| n >= 2)
            else {
                return self.deny(format!("`ValidAttackersAmount$ {amount_raw}`"));
            };
            return match valid_attackers.as_deref() {
                None | Some("Creature" | "Creature.YouCtrl") => {
                    Some(format!("Trigger::AttackWithAtLeast({n})"))
                }
                Some(valid) => {
                    let expr = self.filter_expr(valid)?;
                    let filter = self.body.filter_static("TRIGGER", &expr);
                    Some(format!(
                        "Trigger::AttackWithAtLeastMatching {{ n: {n}, filter: &{filter} }}"
                    ))
                }
            };
        }
        if let Some(target_raw) = attacked_target.as_deref()
            && target_raw != "You"
            && matches!(valid_attackers.as_deref(), None | Some("Creature"))
            && matches!(attacking_player.as_deref(), None | Some("Player"))
            && let Some(who) = Self::attacked_player_rel(target_raw)
        {
            return Some(format!("Trigger::PlayerAttacked({who})"));
        }
        let ctrl_suffix = match attacking_player.as_deref() {
            None | Some("Player") => "",
            Some("You") => ".YouCtrl",
            Some("Opponent" | "Player.Opponent") => ".OppCtrl",
            Some(other) => return self.deny(format!("`AttackingPlayer$ {other}`")),
        };
        let target_suffix = match attacked_target.as_deref() {
            None => "",
            Some("You") => ".attackingYou",
            Some(other) => return self.deny(format!("`AttackedTarget$ {other}`")),
        };
        let base = valid_attackers.unwrap_or_else(|| "Creature".to_string());
        if base == "Card.Self" && ctrl_suffix.is_empty() && target_suffix.is_empty() {
            return Some("Trigger::OneOrMoreAttack(&Filter::This)".to_string());
        }
        let combined = base
            .split(',')
            .map(|alt| {
                let mut s = alt.trim().to_string();
                if !ctrl_suffix.is_empty() && !s.contains("YouCtrl") && !s.contains("OppCtrl") {
                    let sep = if s.contains('.') { '+' } else { '.' };
                    s.push(sep);
                    s.push_str(&ctrl_suffix[1..]);
                }
                if !target_suffix.is_empty() {
                    let sep = if s.contains('.') { '+' } else { '.' };
                    s.push(sep);
                    s.push_str(&target_suffix[1..]);
                }
                s
            })
            .collect::<Vec<_>>()
            .join(",");
        let expr = self.filter_expr(&combined)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::OneOrMoreAttack(&{filter})"))
    }

    fn attacker_unblocked_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("an `AttackerUnblocked` trigger with no `ValidCard$`".to_string());
        };
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!("Trigger::AttacksAndIsntBlocked({filter})"))
    }

    fn transformed_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `Transformed` trigger with no `ValidCard$`".to_string());
        };
        if valid != "Card.Self" {
            return self.deny(format!("`Transformed` on `{valid}`"));
        }
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        Some("Trigger::TransformsIntoThis".to_string())
    }

    fn discarded_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `Discarded` trigger with no `ValidCard$`".to_string());
        };
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        if valid == "Card.Self" {
            if !zoned && p.take("ValidCause").as_deref() == Some("SpellAbility.OppCtrl") {
                return Some("Trigger::DiscardedThisByOpponent".to_string());
            }
            return self.deny("a self-`Discarded` trigger without opponent cause".to_string());
        }
        if !zoned {
            return self.deny("a `Discarded` trigger with no `TriggerZones$`".to_string());
        }
        if valid.split(['.', '+', ',']).any(|atom| atom == "Self") {
            return self.deny(format!("a `Discarded` trigger on `{valid}`"));
        }
        let explicit_by = match p.take("ValidPlayer").as_deref() {
            None => None,
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some(other) => return self.deny(format!("a `Discarded` trigger by `{other}`")),
        };
        let (derived_by, card_valid) = self.split_cycled_valid(&valid)?;
        let who = match (explicit_by, derived_by) {
            (Some(a), Some(b)) if a != b => {
                return self.deny(format!(
                    "conflicting discard players in `{valid}` and `ValidPlayer$`"
                ));
            }
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) => "PlayerRel::EachPlayer",
        };
        let filter = if card_valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&card_valid, true)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!(
            "Trigger::Discarded {{ who: {who}, filter: {filter} }}"
        ))
    }

    fn life_change_trigger(&mut self, p: &mut Params, lost: bool) -> Option<String> {
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Opponent.Active") => "PlayerRel::ActivePlayer",
            Some(other) => return self.deny(format!("life change for `{other}`")),
        };
        let variant = if lost { "LosesLife" } else { "GainsLife" };
        Some(format!("Trigger::{variant}({who})"))
    }

    fn searched_library_trigger(&mut self, p: &mut Params) -> Option<String> {
        let who = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(other) => return self.deny(format!("`SearchedLibrary` by `{other}`")),
        };
        Some(format!("Trigger::SearchesLibrary({who})"))
    }

    fn blocks_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `Blocks` trigger with no `ValidCard$`".to_string());
        };
        let secondary = match p.take("Secondary").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        };
        if let Some(blocked) = p.peek("ValidBlocked").map(str::to_string) {
            if valid == "Card.Self" && blocked == "Creature" && !secondary {
                p.take("ValidBlocked");
                return Some("Trigger::Blocks(&Filter::This)".to_string());
            }
            if valid == "Creature" && blocked == "Card.Self" && secondary {
                p.take("ValidBlocked");
                self.block_line = Some((BlockRole::Blocked, "Creature".to_string(), true));
                self.block_line_one_or_more = true;
                return Some(
                    "Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE)".to_string(),
                );
            }
            if valid == "Card.Self"
                && !blocked.split([',', '.', '+']).any(|atom| {
                    matches!(
                        atom.trim(),
                        "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
                    )
                })
            {
                p.take("ValidBlocked");
                let expr = self.filter_expr(&blocked)?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                self.block_line = Some((BlockRole::Blocks, blocked, secondary));
                self.block_line_one_or_more = true;
                return Some(format!(
                    "Trigger::BlocksOrBecomesBlockedByOneOrMore(&{filter})"
                ));
            }
        }
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!("Trigger::Blocks({filter})"))
    }

    /// `T:Mode$ TurnFaceUp` — "when this permanent is turned face up"
    /// (`Trigger::TurnedFaceUp`, CR 708.8). Only `ValidCard$ Card.Self` maps to
    /// [`baylee_cards_dsl::Trigger::TurnedFaceUp`]; triggers watching other
    /// permanents have no DSL variant and stay refused.
    fn turn_face_up_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `TurnFaceUp` trigger with no `ValidCard$`".to_string());
        };
        if valid != "Card.Self" {
            return self.deny(format!("`TurnFaceUp` on `{valid}`"));
        }
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        Some("Trigger::TurnedFaceUp".to_string())
    }

    /// `T:Mode$ Cycled` — "when you cycle this card" (`Trigger::CycledThis`,
    /// CR 702.29c) or "whenever a player cycles a card" from the battlefield
    /// (`Trigger::Cycled { by, filter }`).
    fn cycled_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `Cycled` trigger with no `ValidCard$`".to_string());
        };
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        let explicit_player = p.take("ValidPlayer");
        if valid == "Card.Self" {
            if zoned {
                return self
                    .deny("a self-`Cycled` trigger with `TriggerZones$ Battlefield`".to_string());
            }
            return match explicit_player.as_deref() {
                None | Some("You") => Some("Trigger::CycledThis".to_string()),
                Some(other) => self.deny(format!("a self-`Cycled` trigger by `{other}`")),
            };
        }
        if !zoned {
            return self.deny("a `Cycled` trigger with no `TriggerZones$`".to_string());
        }
        if valid.split(['.', '+', ',']).any(|atom| atom == "Self") {
            return self.deny(format!("a `Cycled` trigger on `{valid}`"));
        }
        let explicit_by = match explicit_player.as_deref() {
            None => None,
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some(other) => return self.deny(format!("a `Cycled` trigger by `{other}`")),
        };
        let (derived_by, card_valid) = self.split_cycled_valid(&valid)?;
        let by = match (explicit_by, derived_by) {
            (Some(a), Some(b)) if a != b => {
                return self.deny(format!(
                    "conflicting cycling players in `{valid}` and `ValidPlayer$`"
                ));
            }
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) => "PlayerRel::EachPlayer",
        };
        let filter = if card_valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&card_valid, true)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!("Trigger::Cycled {{ by: {by}, filter: {filter} }}"))
    }

    fn split_cycled_valid(&self, valid: &str) -> Option<(Option<&'static str>, String)> {
        if valid.contains(',') {
            let has_player_atom = valid
                .split(['.', '+', ','])
                .any(|a| matches!(a.trim(), "YouCtrl" | "YouOwn" | "OppCtrl" | "OppOwn"));
            if has_player_atom {
                return self.deny(format!(
                    "a multi-clause `Cycled` filter carrying player atoms `{valid}`"
                ));
            }
            return Some((None, valid.to_string()));
        }
        let mut derived: Option<&'static str> = None;
        let mut rest = Vec::new();
        for dot_part in valid.split('.') {
            let mut kept = Vec::new();
            for atom in dot_part.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                let rel = match atom {
                    "YouCtrl" | "YouOwn" => Some("PlayerRel::You"),
                    "OppCtrl" | "OppOwn" => Some("PlayerRel::EachOpponent"),
                    _ => None,
                };
                if let Some(rel) = rel {
                    if let Some(prev) = derived
                        && prev != rel
                    {
                        return self.deny(format!("conflicting player atoms in `{valid}`"));
                    }
                    derived = Some(rel);
                } else {
                    kept.push(atom);
                }
            }
            if !kept.is_empty() {
                rest.push(kept.join("+"));
            }
        }
        let cleaned = if rest.is_empty() {
            "Card".to_string()
        } else {
            rest.join(".")
        };
        Some((derived, cleaned))
    }

    /// One `K:` line, as whichever of the four things it is.
    ///
    /// A keyword line is the corpus's catch-all and this is the one place
    /// that says so: a bit on the face, a static ability CR 613.11 makes of
    /// it, an as-it-enters modifier, or a whole activated ability the rules
    /// define for the word (CR 702.6a). Reading it here rather than in
    /// [`transcode`]'s loop is what lets a rule need the card's `SVar`s, its
    /// subtype catalogs and a cost parser — and what leaves exactly one
    /// answer to "was this line read", which two loops used to guess at
    /// separately and disagree about.
    /// `K:ETBReplacement:Other:<svar>` — "as this enters, …".
    ///
    /// The keyword is a *pointer*: what actually happens is the `SVar` it
    /// names, and the transcoder reads exactly one of them so far. `Other`
    /// is the ordinary as-it-enters replacement; `Copy` is a clone choosing
    /// what to come down as, which is a different mechanism and is refused
    /// by not being this.
    ///
    /// Every card in the reference writes the choice with `Defined$ You`,
    /// and this insists on it rather than ignoring it: "as this enters,
    /// choose a color" is a choice its *controller* makes, and a card that
    /// handed it to somebody else would be a different sentence.
    pub(super) fn etb_replacement(&mut self, rest: &str) -> Option<()> {
        let fields: Vec<&str> = rest.split(':').map(str::trim).collect();
        let kind = *fields.first()?;
        if kind == "Copy" {
            let svar = fields.get(1).copied().unwrap_or_default().to_string();
            let optional = fields.get(2).copied() == Some("Optional");
            return self.copy_on_enter(&svar, optional);
        }
        if kind != "Other" {
            return self.deny(format!("`ETBReplacement:{kind}`"));
        }
        let Some(&svar) = fields.get(1) else {
            return self.deny("an `ETBReplacement` naming no ability".to_string());
        };
        if fields.len() == 5 && fields[2] == "Mandatory" && fields[3].is_empty() {
            let cond = fields[4];
            if let Some(slot) = cond.strip_prefix("Card.Self+kicked ")
                && let Some(idx) = self.dual_kicker_index(slot)
            {
                return self.volver_etb_replacement(svar, Some(idx));
            }
            if cond == "Card.Self+kicked" && self.has_single_kicker {
                return self.volver_etb_replacement(svar, None);
            }
            return self.deny(format!("`ETBReplacement:Other` condition `{cond}`"));
        }
        if fields.len() > 2 {
            return self.deny(format!("`ETBReplacement:Other` extra fields `{rest}`"));
        }
        let Some(body) = self.svars.get(svar) else {
            return self.deny(format!("an `ETBReplacement` naming the missing `{svar}`"));
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return self.deny("an `ETBReplacement` ability with no `$` in it".to_string());
        };
        if api == "ChooseType" {
            return self.choose_type_on_enter(p);
        }
        if api != "ChooseColor" {
            return self.deny(format!("as-enters effect `{api}`"));
        }
        p.drop_prose();
        match p.take("Defined").as_deref() {
            Some("You") => {}
            other => {
                return self.deny(format!(
                    "a colour chosen by `{}`",
                    other.unwrap_or("nobody")
                ));
            }
        }
        let modifier = match p.take("Exclude") {
            None => "EnterModifier::ChooseColor".to_string(),
            Some(name) => {
                let Some(color) = color_word(&name) else {
                    return self.deny(format!("a colour choice excluding `{name}`"));
                };
                format!("EnterModifier::ChooseColorExcept({color})")
            }
        };
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `ChooseColor.{key}`"));
        }
        self.body.enter_modifiers.push(modifier);
        Some(())
    }

    /// `DB$ ChooseType | Type$ Basic Land` or `Type$ Creature` behind an
    /// `ETBReplacement:Other`: "as this enters, choose a basic land type"
    /// (`EnterModifier::ChooseBasicLandType`) or "as this enters, choose a
    /// creature type" (`EnterModifier::ChooseSubtype`). The choice is its
    /// controller's, so a `Defined$` other than `You` is refused.
    pub(super) fn choose_type_on_enter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(other) => return self.deny(format!("a type chosen by `{other}`")),
        }
        let modifier = match p.take("Type").as_deref() {
            Some("Basic Land") => "EnterModifier::ChooseBasicLandType",
            Some("Creature") => "EnterModifier::ChooseSubtype",
            other => {
                return self.deny(format!(
                    "as-enters choice of a `{}` type",
                    other.unwrap_or("nameless")
                ));
            }
        };
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `ChooseType.{key}`"));
        }
        self.body.enter_modifiers.push(modifier.to_string());
        Some(())
    }

    /// `K:ETBReplacement:Copy:<svar>:Optional` — "you may have this enter
    /// as a copy of any creature on the battlefield" (Clone), as
    /// `AbilityDef::CopyOnEnter`.
    ///
    /// The choice is made before the permanent enters (CR 614.12a), so
    /// the reference's `Other` on the choices names nothing the choice
    /// could include, and is dropped. `AddTypes$` is the one exception the
    /// copy may carry here ("except it's an enchantment", Copy Artifact).
    /// A clone that *must* copy, one that sets its colour or grants itself
    /// a trigger (Vesuvan Doppelganger), is another sentence and refused:
    /// `CopyOnEnter` asks with an empty answer allowed, which is the "may".
    pub(super) fn copy_on_enter(&mut self, svar: &str, optional: bool) -> Option<()> {
        if !optional {
            return self.deny("a clone that must copy".to_string());
        }
        let Some(body) = self.svars.get(svar).cloned() else {
            return self.deny(format!("an `ETBReplacement` naming the missing `{svar}`"));
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return self.deny("an `ETBReplacement` ability with no `$` in it".to_string());
        };
        if api != "Clone" {
            return self.deny(format!("as-enters copy `{api}`"));
        }
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("a clone with no `Choices$`".to_string());
        };
        let (base, atoms) = choices.split_once('.').unwrap_or((&choices, ""));
        let kept: Vec<&str> = atoms
            .split('+')
            .filter(|atom| !atom.is_empty() && *atom != "Other")
            .collect();
        let valid = if kept.is_empty() {
            base.to_string()
        } else {
            format!("{base}.{}", kept.join("+"))
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("CHOICE", &expr);
        let mut mods = Vec::new();
        if let Some(types) = p.take("AddTypes") {
            for word in types.split(',').map(str::trim) {
                let Some(types) = card_type_const(word) else {
                    return self.deny(format!("a clone that adds `{word}`"));
                };
                mods.push(format!("CopyMod::AddType({types})"));
            }
        }
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `Clone.{key}`"));
        }
        self.body.abilities.push(format!(
            "AbilityDef::CopyOnEnter {{ target: TargetSpec::Object(&{filter}), mods: &[{}] }}",
            mods.join(", ")
        ));
        Some(())
    }

    /// `T:Mode$ Always`: a state trigger (CR 603.8), which triggers
    /// whenever the game state matches its condition rather than on an
    /// event. The `IsPresent$` clause is that condition, so it is the
    /// trigger's own (`Trigger::State`) and not an intervening "if": the
    /// ability is not asked again as it resolves (CR 603.4 is the other
    /// sentence), and "When you control no Islands, sacrifice this" still
    /// sacrifices after an Island arrives in response.
    ///
    /// The engine collects them off the battlefield only, so any other
    /// `TriggerZones$` is refused; so is every key the clause reader does
    /// not claim (`ResolvingCheck$`, a life total, a computed `SVar`), by
    /// the caller's exhaustion check.
    pub(super) fn state_trigger(&mut self, p: &mut Params) -> Option<String> {
        match p.take("TriggerZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(zones) => return self.deny(format!("`TriggerZones$ {zones}`")),
        }
        let Some(valid) = p.take("IsPresent") else {
            return self.deny("an `Always` trigger with no `IsPresent$`".to_string());
        };
        let clause = self.present_clause(p, &valid)?;
        let name = self.body.condition_static("STATE", &clause);
        Some(format!("Trigger::State(&{name})"))
    }

    /// Static cleanup triggers for imprint links have no runtime ability:
    /// versioned riders already end on the relevant zone changes (CR 400.7,
    /// 406.6). Accept only the exact cleanup forms, never an arbitrary no-op.
    fn is_link_cleanup_trigger(&self, mode: &str, p: &Params) -> bool {
        if mode != "ChangesZone"
            || p.peek("Static") != Some("True")
            || !p.entries.iter().all(|(key, _)| {
                matches!(
                    key.as_str(),
                    "Origin" | "Destination" | "ValidCard" | "Static" | "Execute"
                )
            })
        {
            return false;
        }
        let Some((api, mut child)) = p
            .peek("Execute")
            .and_then(|name| self.svars.get(name))
            .and_then(|body| Params::parse(body))
        else {
            return false;
        };
        match api.as_str() {
            "Cleanup" => {
                child.take("ClearImprinted").as_deref() == Some("True") && child.exhausted()
            }
            "Pump" | "Bookkeeping" => child.take("ForgetImprinted").is_some() && child.exhausted(),
            _ => false,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn triggered(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("a `T:` line with no `$` in it".to_string());
        };
        if self.is_transform48_trigger(&p) {
            let saved_abilities_len = self.body.abilities.len();
            let saved_statics_len = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            if self.transform48_trigger(&mode, p.clone()).is_some() {
                return Some(());
            }
            self.body.abilities.truncate(saved_abilities_len);
            self.body.statics.truncate(saved_statics_len);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            if p.peek("CheckSVar")
                .and_then(|name| self.svars.get(name))
                .is_some_and(|s| s.contains("SacrificedThisTurn Clue"))
                && p.peek("Execute")
                    .and_then(|name| self.svars.get(name))
                    .is_some_and(|body| {
                        body.contains("SetState") && body.contains("Mode$ Transform")
                    })
            {
                return None;
            }
        }
        if p.peek("Execute")
            .and_then(|name| self.svars.get(name))
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, child)| {
                api == "Pump" && child.peek("KW") == Some("CARDNAME can't attack.")
            })
        {
            return self.johan_combat_trigger(&mode, p);
        }
        if p.peek("ValidCard").is_some_and(|valid| {
            valid.contains("wasCastByYou")
                || (mode == "ChangesZone" && valid.contains("wasCastFromYourHandByYou"))
        }) {
            return self.cast_self_protection_trigger(&mode, p);
        }
        let sac_prose_one_or_more = mode == "Sacrificed"
            && p.peek("TriggerDescription")
                .is_some_and(|d| d.to_ascii_lowercase().contains("one or more"));
        // The printed "you may have …" is the choice only where it opens the
        // effect, right after the trigger's own clause: the whole chain is
        // then the optional part (CR 603.5). After a mandatory clause ("draw a
        // card, then you may have …") it would make the mandatory half
        // optional too, so it is not read there; `validate` holds such a card
        // to its printed may.
        let prose_effect = p.peek("TriggerDescription").and_then(|d| {
            d.to_ascii_lowercase()
                .split_once(", ")
                .map(|(_, effect)| effect.to_string())
        });
        let oracle_has_you_may =
            self.oracle.is_empty() || self.oracle.to_ascii_lowercase().contains("you may");
        let prose_you_may = oracle_has_you_may
            && prose_effect
                .as_deref()
                .is_some_and(|effect| effect.starts_with("you may "));
        let prose_may = oracle_has_you_may
            && prose_effect
                .as_deref()
                .is_some_and(|effect| effect.starts_with("you may have "));
        let valid_player_cond = match (mode.as_str(), p.peek("ValidPlayer")) {
            ("Phase", Some("You.descended")) => Some("Condition::DescendedThisTurn"),
            ("Phase", Some("You.isMonarch")) => Some("Condition::IsMonarch(PlayerRel::You)"),
            ("Phase", Some("You.hasInitiative")) => {
                Some("Condition::HasInitiative(PlayerRel::You)")
            }
            ("Phase", Some("You.completedDungeon")) => {
                Some("Condition::CompletedADungeon(PlayerRel::You)")
            }
            ("Phase", Some("Player.isMonarch")) => {
                Some("Condition::IsMonarch(PlayerRel::ActivePlayer)")
            }
            ("Phase", Some("Player.hasInitiative")) => {
                Some("Condition::HasInitiative(PlayerRel::ActivePlayer)")
            }
            ("Phase", Some("Player.completedDungeon")) => {
                Some("Condition::CompletedADungeon(PlayerRel::ActivePlayer)")
            }
            ("LifeLost" | "LifeGained", Some("Opponent.Active")) => {
                Some("Condition::OpponentsTurn")
            }
            _ => None,
        };
        let werewolf_cond = if mode == "Phase" && p.peek("Phase") == Some("Upkeep") {
            let transform = p.take("WerewolfTransformCondition");
            let untransform = p.take("WerewolfUntransformCondition");
            match (transform.as_deref(), untransform.as_deref()) {
                (None, None) => None,
                (Some("True"), None) => Some("Condition::NoSpellsCastLastTurn"),
                (None, Some("True")) => Some("Condition::APlayerCastLastTurnAtLeast(2)"),
                _ => {
                    return self
                        .deny("unsupported werewolf upkeep transform condition".to_string());
                }
            }
        } else {
            None
        };
        p.drop_prose();
        self.trigger_mode = Some(mode.clone());
        if self.is_link_cleanup_trigger(&mode, &p) {
            return Some(());
        }
        if self.is_imprison_clause(&mode, &p) {
            return self.imprison_clause(&mode, p);
        }
        if !matches!(
            mode.as_str(),
            "AttackerBlockedByCreature" | "Blocks" | "AttackerBlocked"
        ) {
            match p.take("Secondary").as_deref() {
                None | Some("True") => {}
                Some(other) => return self.deny(format!("`Secondary$ {other}`")),
            }
        }
        let no_resolving_check = if mode == "Always" {
            false
        } else {
            match p.take("NoResolvingCheck").as_deref() {
                None => false,
                Some("True") => true,
                Some(other) => return self.deny(format!("`NoResolvingCheck$ {other}`")),
            }
        };
        let zone = Self::take_trigger_zone(&mode, &mut p);
        self.trigger_self_etb_kicked = false;
        self.trigger_self_etb_kicked_with = None;
        let self_etb_extra_cond = if mode == "ChangesZone"
            && p.peek("Origin") == Some("Any")
            && p.peek("Destination") == Some("Battlefield")
        {
            match p.peek("ValidCard") {
                Some("Card.Self+bargained") => Some("Condition::Bargained"),
                Some("Card.Self+prototyped") => Some("Condition::Prototyped"),
                Some("Card.Self+PromisedGift") => Some("Condition::GiftWasPromised"),
                Some("Card.Self+CastSa Spell.Warp" | "Card.Self+Warped") => {
                    Some("Condition::WarpCostPaid")
                }
                _ => None,
            }
        } else {
            None
        };
        if self_etb_extra_cond.is_some() {
            for (k, v) in &mut p.entries {
                if k == "ValidCard" {
                    *v = "Card.Self".to_string();
                    break;
                }
            }
        }
        // `(is_trigger_clause, condition_expr)` preserves exact existing clause
        // order when `NoResolvingCheck$` is absent, while cleanly partitioning
        // trigger-only clauses from genuine intervening-if clauses when
        // `NoResolvingCheck$ True` is present.
        let mut tagged_clauses: Vec<(bool, String)> = Vec::new();
        let mut always_trigger_only_clauses: Vec<String> = Vec::new();
        let trigger = if mode == "Always" {
            self.state_trigger(&mut p)?
        } else {
            let trigger = self.trigger_expr_in_zone(&mut p, &mode, !zone.is_empty())?;
            if mode != "Phase"
                && let Some(phase_raw) = p.peek("Phase")
                && let Some(phase_cond) = Self::trigger_phase_condition(phase_raw)
            {
                p.take("Phase");
                always_trigger_only_clauses.push(phase_cond.to_string());
            }
            if mode == "AttackerBlocked"
                && trigger.starts_with("Trigger::BlocksCreature(")
                && let Some(clause) = self.event_blocked_by_others_clause(&mut p)
            {
                tagged_clauses.push((true, clause));
            }
            let mut present_clauses = Vec::new();
            if p.has("IsPresent") || p.has("IsPresent2") {
                self.read_trigger_present_clauses(&mut p, &mut present_clauses)?;
            }
            let cond = self.condition(&mut p)?;
            if !cond.is_empty() {
                let Some(inner) = cond
                    .strip_prefix(", condition = Some(")
                    .and_then(|s| s.strip_suffix(')'))
                else {
                    return self.deny("`IsPresent$` beside another clause".to_string());
                };
                tagged_clauses.push((true, inner.to_string()));
            }
            for clause in present_clauses {
                tagged_clauses.push((true, clause));
            }
            trigger
        };
        if let Some(clause) = self_etb_extra_cond {
            tagged_clauses.push((false, clause.to_string()));
        }
        if let Some(clause) = werewolf_cond {
            tagged_clauses.push((false, clause.to_string()));
        }
        if let Some(clause) = valid_player_cond {
            tagged_clauses.push((false, clause.to_string()));
        }
        if p.peek("CheckSVar").is_some() {
            let var = p.peek("CheckSVar").map(str::to_string).unwrap();
            let cmp = p
                .peek("SVarCompare")
                .map_or_else(|| "GE1".to_string(), str::to_string);
            if let Some(clause) = self
                .svar_condition(&var, &cmp)
                .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                .or_else(|| self.extra_svar_condition_wave8(&var, &cmp))
                .or_else(|| self.wave7_svar_condition(&var, &cmp))
                .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, false))
                .or_else(|| self.wave15_trigger_svar_condition(&var, &cmp))
                .or_else(|| self.svar_condition_clause(&var, &cmp))
                .or_else(|| {
                    // The SVar's own `Count$…` read as an amount, the reader
                    // static and activated conditions already share.
                    let body = self.svars.get(var.trim())?.trim().to_string();
                    body.starts_with("Count$")
                        .then(|| self.svar_condition_clause(&body, &cmp))
                        .flatten()
                })
            {
                p.take("CheckSVar");
                p.take("SVarCompare");
                tagged_clauses.push((true, clause));
            }
        }
        if let Some(cond_raw) = p.peek("Condition")
            && let Some(clause) = Self::named_condition(cond_raw)
                .or_else(|| Self::wave7_named_condition(cond_raw))
                .or_else(|| Self::wave21c_named_condition(cond_raw))
        {
            p.take("Condition");
            tagged_clauses.push((true, clause.to_string()));
        }
        if let Some(clause) = Self::take_check_defined_player(&mut p) {
            tagged_clauses.push((true, clause.to_string()));
        }
        if let Some(bool_clauses) = Self::take_trigger_bool_conditions(&mut p) {
            for clause in bool_clauses {
                tagged_clauses.push((true, clause.to_string()));
            }
        }
        if no_resolving_check && !tagged_clauses.iter().any(|(is_trig, _)| *is_trig) {
            return self.deny("`NoResolvingCheck$ True` with no trigger condition".to_string());
        }
        let join_all = |mut items: Vec<String>| -> Option<String> {
            match items.len() {
                0 => None,
                1 => items.pop(),
                _ => Some(format!("Condition::All(&[{}])", items.join(", "))),
            }
        };
        let mut final_clauses = Vec::new();
        if no_resolving_check {
            let mut once_clauses = always_trigger_only_clauses;
            let mut intervening = Vec::new();
            for (is_trig, clause) in tagged_clauses {
                if is_trig {
                    once_clauses.push(clause);
                } else {
                    intervening.push(clause);
                }
            }
            if let Some(inner) = join_all(once_clauses) {
                final_clauses.push(format!("Condition::TriggerOnly(&{inner})"));
            }
            final_clauses.extend(intervening);
        } else {
            if let Some(inner) = join_all(always_trigger_only_clauses) {
                final_clauses.push(format!("Condition::TriggerOnly(&{inner})"));
            }
            final_clauses.extend(tagged_clauses.into_iter().map(|(_, c)| c));
        }
        let condition = join_all(final_clauses)
            .map_or_else(String::new, |expr| format!(", condition = Some({expr})"));
        let once_per_turn = match p.take("ActivationLimit").as_deref() {
            None => "",
            Some("1") => ", once_per_turn = true",
            Some(other) => return self.deny(format!("trigger `ActivationLimit$ {other}`")),
        };
        if sac_prose_one_or_more && once_per_turn.is_empty() {
            return self.deny(
                "a `Sacrificed` trigger saying \"one or more\" without `ActivationLimit$ 1`"
                    .to_string(),
            );
        }
        let trigger_self_etb_kicked = std::mem::take(&mut self.trigger_self_etb_kicked);
        let condition = match (
            self.trigger_self_etb_kicked_with.take(),
            condition.is_empty(),
        ) {
            (Some(idx), true) => format!(", condition = Some(Condition::KickedWith({idx}))"),
            (Some(_), false) => {
                return self.deny(
                    "a dual-kicker ETB trigger with a second intervening condition".to_string(),
                );
            }
            (None, _) => condition,
        };
        let Some(execute) = p.take("Execute") else {
            return self.deny(format!("a `{mode}` trigger with no `Execute$`"));
        };
        // "When …, you may …" (CR 603.5): the ability triggers and goes on
        // the stack whatever its controller intends, and the choice is made
        // as it resolves. That is `Effect::MayDo` exactly, and it is where
        // the pool's hand-written "may" triggers already put it.
        //
        // `You` and nothing else. 1506 of the corpus's 1584
        // `OptionalDecider$` values are `You`, which `Effect::MayDo` asks by
        // construction — the resolver puts the question to the resolving
        // ability's controller. Every other value names somebody else
        // (`TriggeredCardController`, `EnchantedController`, `Opponent`) and
        // no effect here can ask them, so they are refused by name rather
        // than quietly asked of the wrong player.
        let trigger_is_self = trigger == "Trigger::ETB"
            || trigger == "Trigger::TurnedFaceUp"
            || trigger == "Trigger::TransformsIntoThis"
            || trigger == "Trigger::CycledThis"
            || trigger == "Trigger::DiscardedThisByOpponent"
            || trigger == "Trigger::BecomesBlocked"
            || trigger == "Trigger::Mutates"
            || trigger == "Trigger::PHASES_IN"
            || trigger == "Trigger::PHASES_OUT"
            || trigger == "Trigger::EXERTED_THIS"
            || trigger.starts_with("Trigger::UnlockThisDoor(")
            || trigger.contains("&Filter::This")
            || trigger_self_etb_kicked;
        let optional_flag = match p.take("Optional").as_deref() {
            None => None,
            Some("Yes" | "True") => Some(true),
            Some("No" | "False") => Some(false),
            Some(other) => return self.deny(format!("trigger `Optional$ {other}`")),
        };
        let decider = p.take("OptionalDecider");
        // Only a "may" read from the description alone (or `Optional$ Yes`)
        // can meet an executed line that asks by itself; an `OptionalDecider$`
        // keeps its old wrap.
        let may_from_prose = decider.is_none() && optional_flag.is_none() && prose_may;
        let mut may = match (decider.as_deref(), optional_flag) {
            (None, Some(false)) => false,
            (None, None) => prose_may,
            (None, Some(true)) | (Some("You"), None | Some(true)) => true,
            (Some("You"), Some(false)) => {
                return self
                    .deny("conflicting `OptionalDecider$ You` and `Optional$ No`".to_string());
            }
            (Some("TriggeredCardController"), None | Some(true))
                if trigger_is_self && self.oracle.to_ascii_lowercase().contains("you may") =>
            {
                true
            }
            (Some(other), _) => return self.deny(format!("a `may` decided by `{other}`")),
        };
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                return self.deny(format!("unclaimed parameter `{mode}.{key}`"));
            }
            return None;
        }
        let Some(body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        // "You may pay {1}. If you do, you gain 1 life" (Crystal Rod): the
        // reference writes the payment as a `Cost$` on the executed line and
        // the "may" as `OptionalDecider$ You`. The two are one decision —
        // the player who will not pay has declined — so the price replaces
        // the `MayDo` rather than sitting inside it, which would ask twice.
        // Generic mana only: a coloured price or a non-mana cost is another
        // payment the effect cannot take, and stays unclaimed.
        let (body, price) = match self.optional_trigger_price(&body) {
            Some((stripped, price)) => (stripped, Some(price)),
            None => (body, None),
        };
        let (body, mandatory_sac_self) = if !may && price.is_none() {
            match Self::strip_mandatory_sac_self_trigger(&body) {
                Some(stripped) => (stripped, true),
                None => (body, false),
            }
        } else {
            (body, false)
        };
        if !may
            && price.is_none()
            && !mandatory_sac_self
            && self.block_line.is_none()
            && let Some((api, mut cp)) = Params::parse(&body)
            && api == "Charm"
        {
            cp.drop_prose();
            let Some(choices) = cp.take("Choices") else {
                return self.deny("a modal trigger with no `Choices$`".to_string());
            };
            match cp.take("CharmNum").as_deref() {
                None | Some("1") => {}
                Some(n) => return self.deny(format!("a modal trigger choosing `{n}`")),
            }
            if let Some(key) = cp.first_key() {
                return self.deny(format!("unclaimed parameter `Charm.{key}`"));
            }
            self.trigger_is_self = trigger_is_self;
            self.is_etb_self_trigger = trigger == "Trigger::ETB";
            let parsed = self.read_charm_modes(&choices);
            self.trigger_is_self = false;
            self.is_etb_self_trigger = false;
            let parsed = parsed?;
            if parsed.len() < 2 {
                return self.deny("a charm with fewer than two modes".to_string());
            }
            let modes: Vec<String> = parsed.into_iter().map(|(m, _, _, _)| m).collect();
            self.body.abilities.push(format!(
                "modal_triggered!({trigger}, &[{}]{zone}{once_per_turn}{condition})",
                modes.join(", ")
            ));
            return Some(());
        }
        let mut chain = Chain::default();
        self.trigger_is_self = trigger_is_self;
        self.is_etb_self_trigger = trigger == "Trigger::ETB";
        self.trigger_blocks_creature = trigger.starts_with("Trigger::BlocksCreature(");
        self.cost_sacrifices_self = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p)) if p == "Sacrifice(&Filter::This)"
        );
        self.cost_sacrifices_other = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p))
                if p.starts_with("Sacrifice(") && p != "Sacrifice(&Filter::This)"
        );
        let is_attackers_at_target = trigger.starts_with("Trigger::AttackersDeclaredAtTarget");
        let saved_trigger_mode = if is_attackers_at_target {
            self.trigger_mode
                .replace("AttackersDeclaredAtTarget".to_string())
        } else {
            None
        };
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let mut chain_ok = self.chain(&body, &mut chain);
        if chain_ok.is_none() && is_attackers_at_target {
            let mut at_chain = Chain::default();
            let after_first_statics = self.body.statics.len();
            let after_first_unclaimed = self.unclaimed.borrow().clone();
            self.body.statics.truncate(saved_statics);
            (*self.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
            let exact_one = trigger.contains("exact_count: Some(1)");
            if self
                .wave25c_attackers_at_target_chain(&body, exact_one, &mut at_chain)
                .is_some()
            {
                chain = at_chain;
                chain_ok = Some(());
            } else {
                self.body.statics.truncate(after_first_statics);
                *self.unclaimed.borrow_mut() = after_first_unclaimed;
            }
        }
        if chain_ok.is_none() {
            let mut dig_chain = Chain::default();
            let after_first_statics = self.body.statics.len();
            let after_first_unclaimed = self.unclaimed.borrow().clone();
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            if self
                .wave24e_dig_trigger_chain(&body, &mut dig_chain)
                .is_some()
            {
                chain = dig_chain;
                chain_ok = Some(());
            } else {
                self.body.statics.truncate(after_first_statics);
                *self.unclaimed.borrow_mut() = after_first_unclaimed;
            }
        }
        if is_attackers_at_target {
            self.trigger_mode = saved_trigger_mode;
        }
        self.trigger_is_self = false;
        self.is_etb_self_trigger = false;
        self.trigger_blocks_creature = false;
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        chain_ok?;
        if chain.effects.is_empty() {
            return self.deny("a trigger that reads as no effect at all".to_string());
        }
        let chain_asks = chain.target_up_to.is_some()
            || chain.effects.iter().any(|e| {
                (e.contains("ChooseCard {") && e.contains("min: 0,"))
                    || (e.contains("PeekAndReveal {") && e.contains("min_pick: 0,"))
            })
            || chain.effects.iter().any(|e| {
                [
                    "MayDo",
                    "PlayerMayPay",
                    "optional: true",
                    "Scry",
                    "OptionalBasicLandSearchFor",
                    "SearchLibraryOrGraveyard",
                    "SearchLibraryBeforeShuffle",
                    "LookAtTop",
                    "RevealTopFind",
                    "RevealTopOnePerType",
                    "RevealTopPlayerMayPutOntoBattlefield",
                    "ExileTopToPlay",
                    "ExileTopMayCast",
                    "ExileTopWithPermission",
                    "GrantLinkedExilePermission",
                    "DiscardUpToThenDraw",
                    "MillMayTakeOne",
                    "ChooseExiledToPlay",
                    "MayExileFromYourGraveyardThen",
                    "MayPayEnergyOr",
                    "MayChooseNotToUntap",
                    "CopyTarget",
                    "CopySpell",
                    "CopyEventSpellOrAbility",
                    "CopyThisSpell",
                    "BottomCardFromHand",
                    "PutFromHandOnTop",
                    "ReorderTopLibrary",
                    "MayPlayLinkedExile",
                    "ChooseAndCreateTokenCopy",
                ]
                .iter()
                .any(|needle| e.contains(needle))
            });
        if prose_you_may && !may && price.is_none() && !chain_asks {
            if mode == "Cycled" {
                return self.deny(
                    "a `Cycled` trigger saying \"you may\" without `OptionalDecider$` or an optional effect"
                        .to_string(),
                );
            }
            let prose_optional_search = optional_flag.is_none()
                && chain.effects.len() == 1
                && chain.effects[0].starts_with("Effect::SearchLibrary {")
                && chain.effects[0].ends_with("optional: false }")
                && prose_effect.as_deref().is_some_and(|e| {
                    e.starts_with("you may search your library")
                        && !e.contains(". ")
                        && !e.contains("if you do")
                        && !e.contains("if you don't")
                        && !e.contains("when you do")
                        && !e.contains("otherwise")
                        && !e.contains("pay ")
                        && !e.contains("unless ")
                        && !e.contains("instead")
                });
            let simple_prose_you_may = optional_flag.is_none()
                && chain.effects.len() == 1
                && prose_effect.as_deref().is_some_and(|e| {
                    !e.contains(". ")
                        && !e.contains("if you do")
                        && !e.contains("if you don't")
                        && !e.contains("when you do")
                        && !e.contains("otherwise")
                        && !e.contains(", then ")
                        && !e.contains("pay ")
                        && !e.contains("unless ")
                        && !e.contains("instead")
                });
            if prose_optional_search {
                let updated = chain.effects[0]
                    .trim_end_matches("optional: false }")
                    .to_string()
                    + "optional: true }";
                chain.effects[0] = updated;
            } else if simple_prose_you_may {
                may = true;
            } else {
                return self.deny(format!(
                    "a `{mode}` trigger saying \"you may\" without `OptionalDecider$` or an optional effect"
                ));
            }
        }
        let targets =
            Self::target_req_expr(&chain).map_or_default(|req| format!(", targets = Some({req})"));
        let second = Self::second_target_req_expr(&chain)
            .map_or_default(|req| format!(", second_targets = Some({req})"));
        // The targets stay **outside** the `may`, and the two rules say why:
        // CR 603.3d chose them when the ability went on the stack, CR 603.5
        // puts the choice at resolution. A declined "may" is therefore an
        // ability that targeted and then did nothing, which is what hoisting
        // `chain.target` into the macro's own field already gives.
        //
        // And the wrap is around the **whole** list rather than each effect,
        // because the printed word covers a whole clause: Ondu Cleric's "you
        // may gain life equal to the number of Allies you control" is one
        // decision, not one per operation it expands into.
        // A printed "you may have …" in the description (or `Optional$ Yes`)
        // and an optional inner effect are one decision: a chain that already
        // asks (`Effect::MayDo` / `PlayerMayPay*`) is not asked twice. Likewise
        // a bare `TapOrUntap` already wraps its toggle in `MayDo`.
        let asks_already = ((may_from_prose || optional_flag == Some(true))
            && chain.effects.iter().all(|e| {
                e.starts_with("Effect::MayDo {") || e.starts_with("Effect::PlayerMayPay")
            }))
            || (chain.effects.len() == 1
                && chain.effects[0] == "Effect::MayDo { effects: &[Effect::ToggleTapTarget] }");
        let one_effect = match chain.effects.as_slice() {
            [only] => only.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        let effects = chain.effects.join(", ");
        let effects = match price {
            Some(OptionalTriggerPrice::Generic(mana)) => format!(
                "Effect::PlayerMayPayThen {{ player: PlayerRel::You, \
                 mana: {mana}, effects: &[{effects}] }}"
            ),
            // Farmstead's "you may pay {W}{W}. If you do, you gain 1 life".
            Some(OptionalTriggerPrice::Printed(cost)) => format!(
                "Effect::PlayerMayPayManaThen {{ player: PlayerRel::You, \
                 cost: mana!(\"{cost}\"), effects: &[{effects}] }}"
            ),
            Some(OptionalTriggerPrice::Life(life)) => format!(
                "Effect::PlayerMayPayLifeThen {{ player: PlayerRel::You, \
                 life: {life}, effects: &[{effects}] }}"
            ),
            Some(OptionalTriggerPrice::CostPart(part)) => format!(
                "Effect::PlayerMayPayCostThen {{ player: PlayerRel::You, \
                 cost: &CostPart::{part}, effect: &{one_effect} }}"
            ),
            Some(OptionalTriggerPrice::SacrificeN { filter, count }) => format!(
                "Effect::PlayerMaySacrificeNThen {{ player: PlayerRel::You, \
                 filter: {filter}, count: {count}, effect: &{one_effect} }}"
            ),
            None if mandatory_sac_self => {
                format!("Effect::SacrificeSelfThen {{ then: &[{effects}] }}")
            }
            None if may && !asks_already => format!("Effect::MayDo {{ effects: &[{effects}] }}"),
            None => effects,
        };
        let tail = format!("&[{effects}]{zone}{once_per_turn}{targets}{second}{condition})");
        let ability = format!("triggered!({trigger}, {tail}");
        let one_or_more = std::mem::take(&mut self.block_line_one_or_more);
        if let Some((role, other, secondary)) = self.block_line.take() {
            if one_or_more && secondary && self.block_half.is_none() {
                if role == BlockRole::Blocks
                    && let Some(filter_ref) = trigger
                        .strip_prefix("Trigger::BlocksOrBecomesBlockedByOneOrMore(")
                        .and_then(|s| s.strip_suffix(')'))
                    && let Some(prev) = self.body.abilities.last_mut()
                    && *prev
                        == format!("triggered!(Trigger::BecomesBlockedBy({filter_ref}), {tail}")
                {
                    *prev = ability;
                    return Some(());
                }
                if role == BlockRole::Blocked
                    && other == "Creature"
                    && let Some(prev) = self.body.abilities.last_mut()
                    && *prev == format!("triggered!(Trigger::Blocks(&Filter::This), {tail}")
                {
                    *prev = ability;
                    return Some(());
                }
                return self.deny(
                    "a secondary one-or-more block trigger without its matching first half"
                        .to_string(),
                );
            }
            return self.pair_block_half(role, other, secondary, one_or_more, ability);
        }
        if let Some(filter_ref) = trigger
            .strip_prefix("Trigger::LeavesBattlefield(")
            .and_then(|s| s.strip_suffix(')'))
            && let Some(prev) = self.body.abilities.last_mut()
        {
            let expected_prev = if filter_ref == "&Filter::This" {
                format!("triggered!(Trigger::PHASES_OUT, {tail}")
            } else {
                format!("triggered!(Trigger::PhasesOut({filter_ref}), {tail}")
            };
            if *prev == expected_prev {
                *prev = format!(
                    "triggered!(Trigger::PhasesOutOrLeavesBattlefield({filter_ref}), {tail}"
                );
                return Some(());
            }
        }
        self.body.abilities.push(ability);
        Some(())
    }

    /// Strips `Cost$ Mandatory Sac<1/CARDNAME>` from a trigger-executed `AB$`
    /// line (*Aspiring Champion* shape: "sacrifice it. If you do, ...") so the
    /// compiled body wraps in `Effect::SacrificeSelfThen`. Refuses lines that
    /// read `Sacrificed$` since `SacrificeSelfThen` does not populate cost LKI.
    fn strip_mandatory_sac_self_trigger(body: &str) -> Option<String> {
        if !body.trim_start().starts_with("AB$") || body.contains("Sacrificed") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        let rest_cost = cost.strip_prefix("Mandatory ")?.trim();
        let is_sac_self = rest_cost == "Sac<1/CARDNAME>"
            || (rest_cost.starts_with("Sac<1/CARDNAME/")
                && rest_cost.find('>') == Some(rest_cost.len() - 1));
        if !is_sac_self {
            return None;
        }
        Some(
            parts
                .into_iter()
                .filter(|part| !part.starts_with("Cost$"))
                .collect::<Vec<_>>()
                .join(" | "),
        )
    }

    /// `T:Mode$ AttackerBlockedByCreature` — "whenever this creature blocks
    /// or becomes blocked by a non-Wall creature" (Cockatrice), as
    /// `Trigger::BlocksOrBecomesBlockedBy`.
    ///
    /// The reference writes the one printed ability as **two** lines, one
    /// for each side of the block: `ValidCard$ <other> | ValidBlocker$
    /// Card.Self` for "blocks" and `ValidCard$ Card.Self | ValidBlocker$
    /// <other>` for "becomes blocked by", the second marked `Secondary$
    /// True`. Each line is read here; [`Tx::pair_block_half`] keeps the
    /// first, drops its mirror, and a script that ends with a half unpaired
    /// is refused, because either half alone is another sentence ("whenever
    /// this creature blocks a creature", CR 509.3b, or "becomes blocked by
    /// a creature", 509.3d) that the trigger here would over-read.
    ///
    /// A source on neither side is an Aura's or an Equipment's "enchanted
    /// creature blocks", and an other side relative to another permanent
    /// (`AttachedBy`, `EnchantedBy`) is a sentence about it; both refuse.
    pub(super) fn block_trigger(&mut self, p: &mut Params) -> Option<String> {
        let card = p.take("ValidCard");
        let blocker = p.take("ValidBlocker");
        let secondary = match p.take("Secondary").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        };
        let (role, other) = match (card, blocker) {
            (Some(card), Some(blocker)) if blocker == "Card.Self" && card != "Card.Self" => {
                (BlockRole::Blocks, card)
            }
            (Some(card), Some(blocker)) if card == "Card.Self" && blocker != "Card.Self" => {
                (BlockRole::Blocked, blocker)
            }
            (card, blocker) => {
                return self.deny(format!(
                    "a block between `{}` and `{}`",
                    card.unwrap_or_default(),
                    blocker.unwrap_or_default()
                ));
            }
        };
        if other.split([',', '.', '+']).any(|atom| {
            matches!(
                atom.trim(),
                "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
            )
        }) {
            return self.deny(format!("a block with `{other}`, relative to another card"));
        }
        let expr = self.filter_expr(&other)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        self.block_line = Some((role, other, secondary));
        self.block_line_one_or_more = false;
        Some(format!("Trigger::BlocksOrBecomesBlockedBy(&{filter})"))
    }

    /// The pairing half of [`Tx::block_trigger`]: the first half read is
    /// written and kept; the second must be its mirror — the other side of
    /// the block, the same other creature, `Secondary$` on exactly one of
    /// the two, and the same ability once read — and writes nothing, since
    /// the card prints one ability.
    pub(super) fn pair_block_half(
        &mut self,
        role: BlockRole,
        other: String,
        secondary: bool,
        one_or_more: bool,
        ability: String,
    ) -> Option<()> {
        match self.block_half.take() {
            None => {
                self.body.abilities.push(ability.clone());
                self.block_half = Some(BlockHalf {
                    role,
                    other,
                    secondary,
                    one_or_more,
                    ability,
                });
                Some(())
            }
            Some(first)
                if first.one_or_more == one_or_more
                    && first.role != role
                    && first.other == other
                    && first.secondary != secondary
                    && first.ability == ability =>
            {
                Some(())
            }
            Some(_) => {
                self.deny("two block triggers that are not the halves of one sentence".to_string())
            }
        }
    }

    /// A lone "becomes blocked by" half has its own exact trigger. A lone
    /// "blocks" half is not the paired trigger and stays refused.
    pub(super) fn block_halves_paired(&mut self) -> Option<()> {
        let Some(half) = self.block_half.take() else {
            return Some(());
        };
        if !half.one_or_more && half.role == BlockRole::Blocked && !half.secondary {
            let ability = self
                .body
                .abilities
                .iter_mut()
                .find(|ability| **ability == half.ability)?;
            *ability = ability.replacen(
                "Trigger::BlocksOrBecomesBlockedBy",
                "Trigger::BecomesBlockedBy",
                1,
            );
            return Some(());
        }
        self.deny(format!(
            "a `{}` trigger without its mirror",
            match half.role {
                BlockRole::Blocks => "blocks",
                BlockRole::Blocked => "becomes blocked by",
            }
        ))
    }

    /// `DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat` — "destroy that
    /// creature at end of combat" (Cockatrice): a delayed trigger that
    /// triggers as the end of combat step begins (CR 511.2),
    /// `Effect::AtEndOfCombat`.
    ///
    /// Only under a block trigger ([`Tx::block_trigger`]) and only
    /// remembering the **other** creature of the block — the attacker on the
    /// "blocks" half, the blocker on the "becomes blocked by" one — which is
    /// the trigger's event object. Its `Execute$` is read as a chain of its
    /// own in which `Defined$ DelayTriggerRememberedLKI` is that object, and
    /// which may target nothing. Every other phase, player, remembered
    /// object or delayed mode is refused by name.
    pub(super) fn delayed_trigger(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        chain: &Chain,
    ) -> Option<String> {
        let mode = p.take("Mode").unwrap_or_default();
        if mode == "Phase" {
            let phase = p.take("Phase").unwrap_or_default();
            return match self.legacy_delayed_phase(p, &phase, target, targets_here) {
                LegacyPhase::Handled(expr) => Some(expr),
                LegacyPhase::Refused => None,
                LegacyPhase::Fallthrough => self.delayed_phase_trigger(p, &phase, target, chain),
            };
        }
        if mode == "ChangesController" {
            return self.delayed_observation_trigger(p, target, chain);
        }
        self.delayed_event_trigger(p, &mode, target, chain)
    }

    fn legacy_delayed_phase(
        &mut self,
        p: &mut Params,
        phase: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> LegacyPhase {
        let has_extra = p.has("IsPresent")
            || p.has("CheckSVar")
            || p.has("ConditionDefined")
            || p.has("ConditionPresent")
            || p.has("OptionalDecider")
            || p.has("ThisTurn");
        if has_extra {
            return LegacyPhase::Fallthrough;
        }
        if phase == "Upkeep"
            && p.peek("NextTurn") == Some("True")
            && matches!(p.peek("ValidPlayer"), None | Some("Player"))
            && !p.has("RememberObjects")
            && !targets_here
        {
            p.take("NextTurn");
            p.take("ValidPlayer");
            return match self.delayed_execute_effects(p) {
                Some(effects) => LegacyPhase::Handled(format!(
                    "Effect::AtNextTurnsUpkeep {{ effects: &[{effects}] }}"
                )),
                None => LegacyPhase::Refused,
            };
        }
        if phase == "EndCombat"
            && !p.has("NextTurn")
            && matches!(p.peek("ValidPlayer"), None | Some("Player"))
            && target.is_none()
        {
            let remembered = p.peek("RememberObjects").unwrap_or_default();
            let self_combat = remembered.is_empty()
                && self.block_line.is_none()
                && matches!(self.trigger_mode.as_deref(), Some("Attacks" | "Blocks"));
            let damage_done_target = self.block_line.is_none()
                && self.trigger_mode.as_deref() == Some("DamageDone")
                && matches!(remembered, "TriggeredTargetLKICopy" | "TriggeredTarget");
            let blocks_creature_attacker = self.block_line.is_none()
                && self.trigger_blocks_creature
                && matches!(remembered, "TriggeredAttackerLKICopy" | "TriggeredAttacker");
            let block_pair = self.block_line.as_ref().is_some_and(|(role, ..)| {
                let expected = match role {
                    BlockRole::Blocks => "TriggeredAttacker",
                    BlockRole::Blocked => "TriggeredBlocker",
                };
                remembered.strip_suffix("LKICopy").unwrap_or(remembered) == expected
            });
            if self_combat || damage_done_target || blocks_creature_attacker || block_pair {
                p.take("ValidPlayer");
                p.take("RememberObjects");
                return match self.delayed_execute_effects(p) {
                    Some(effects) => LegacyPhase::Handled(format!(
                        "Effect::AtEndOfCombat {{ about: TargetSpec::EventObject, effects: &[{effects}] }}"
                    )),
                    None => LegacyPhase::Refused,
                };
            }
        }
        LegacyPhase::Fallthrough
    }

    fn delayed_phase_trigger(
        &mut self,
        p: &mut Params,
        phase: &str,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        if p.has("NextTurn") {
            return self.deny(format!(
                "a delayed trigger at `Phase {phase}` with `NextTurn$`"
            ));
        }
        let step = match phase {
            "Upkeep" => "DelayedStep::Upkeep",
            "Draw" => "DelayedStep::Draw",
            "Main1,Main2" | "Main2,Main1" => "DelayedStep::MainPhase",
            "BeginCombat" => "DelayedStep::CombatBegin",
            "EndCombat" => "DelayedStep::CombatEnd",
            "End of Turn" | "End Of Turn" => "DelayedStep::EndStep",
            "Cleanup" => "DelayedStep::Cleanup",
            _ => return self.deny(format!("a delayed trigger at `Phase {phase}`")),
        };
        let whose = match p.take("ValidPlayer").as_deref() {
            None | Some("Player") if phase == "Upkeep" => {
                return self.deny(
                    "a delayed trigger at `Phase Upkeep` without `NextTurn$ True`".to_string(),
                );
            }
            None | Some("Player") => "None",
            Some("You") => "Some(PlayerRel::You)",
            Some("Opponent" | "Player.Opponent") => "Some(PlayerRel::EachOpponent)",
            Some(other) => return self.deny(format!("a delayed trigger in `{other}`'s turn")),
        };
        let this_turn_only = match p.take("ThisTurn").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("a delayed trigger with `ThisTurn$ {other}`")),
        };
        let optional = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => {
                return self.deny(format!("a delayed trigger `OptionalDecider$ {other}`"));
            }
        };
        let execute = p.peek("Execute")?.to_string();
        let body_uses_rem = self
            .svars
            .get(&execute)
            .is_some_and(|b| b.contains("DelayTriggerRemembered"));
        let raw_rem = p.take("RememberObjects");
        let Some(remember) =
            self.delayed_remember(raw_rem.as_deref(), target, chain, body_uses_rem)
        else {
            return self.deny(format!(
                "a delayed trigger remembering `{}`",
                raw_rem.unwrap_or_default()
            ));
        };
        if p.peek("ConditionDefined") == Some("Targeted")
            && p.peek("ConditionPresent") == Some("Card")
            && !p.has("ConditionCompare")
            && matches!(
                remember,
                "TriggerRemember::FirstTarget" | "TriggerRemember::EachTarget"
            )
        {
            p.take("ConditionDefined");
            p.take("ConditionPresent");
        } else if p.peek("ConditionDefined") == Some("Remembered")
            && p.peek("ConditionPresent") == Some("Card")
            && matches!(p.peek("ConditionCompare"), None | Some("GE1"))
            && matches!(
                remember,
                "TriggerRemember::EachCreatedToken" | "TriggerRemember::EachExiledThisWay"
            )
        {
            p.take("ConditionDefined");
            p.take("ConditionPresent");
            p.take("ConditionCompare");
        }
        let cond = self.delayed_optional_condition(p)?;
        let (effects, targets) = self.parse_delayed_body(p, None, optional)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DelayedTrigger.{key}`"));
        }
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ step: {step}, \
             whose: {whose}, this_turn_only: {this_turn_only}, recurring: false }}, \
             condition: {cond}, targets: {targets_expr}, remember: {remember}, \
             effects: &[{effects}] }}"
        ))
    }

    fn delayed_event_trigger(
        &mut self,
        p: &mut Params,
        mode: &str,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        if mode.is_empty()
            || !matches!(
                mode,
                "ChangesZone"
                    | "SpellCast"
                    | "SpellCastOrCopy"
                    | "Attacks"
                    | "Blocks"
                    | "DamageDone"
                    | "DamageDoneOnce"
                    | "DamageDealtOnce"
                    | "Taps"
                    | "Untaps"
                    | "LifeGained"
                    | "LifeLost"
                    | "Drawn"
                    | "LandPlayed"
                    | "Discarded"
                    | "Cycled"
                    | "CounterAddedOnce"
                    | "AttackerBlocked"
                    | "AttackerUnblocked"
                    | "PhaseIn"
                    | "PhaseOut"
                    | "FlippedCoin"
                    | "RolledDie"
            )
        {
            return self.deny(format!("a delayed trigger at `{mode} `"));
        }
        if p.has("NextTurn") || p.has("Static") || p.has("OnlyOnce") || p.has("TriggerOnce") {
            return self.deny(format!(
                "a delayed trigger at `{mode} ` with unsupported lifetime"
            ));
        }
        let duration = match p.take("ThisTurn").as_deref() {
            Some("True") => "DelayedDuration::OneShotThisTurn",
            None | Some("False") => "DelayedDuration::OneShot",
            Some(other) => return self.deny(format!("a delayed trigger with `ThisTurn$ {other}`")),
        };
        let optional = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => {
                return self.deny(format!("a delayed trigger `OptionalDecider$ {other}`"));
            }
        };
        let raw_rem = p.take("RememberObjects");
        let (watch, remember, watched_remembered) =
            self.delayed_event_watch(p, raw_rem.as_deref(), target, chain)?;
        let trigger_expr = self.trigger_expr_in_zone(p, mode, true)?;
        let cond = self.delayed_optional_condition(p)?;
        let (effects, targets) = self.parse_delayed_body(p, Some(mode.to_string()), optional)?;
        let remember = if watched_remembered
            && mode != "ChangesZone"
            && (effects.contains("TargetSpec::EventObject")
                || effects.contains("Effect::SacrificeEvent")
                || effects.contains("Effect::TapEventObject")
                || effects.contains("Amount::Event"))
        {
            watch
        } else {
            remember
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DelayedTrigger.{key}`"));
        }
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
             trigger: &{trigger_expr}, watch: {watch}, duration: {duration} }}, \
             condition: {cond}, targets: {targets_expr}, remember: {remember}, \
             effects: &[{effects}] }}"
        ))
    }

    fn delayed_event_watch(
        &mut self,
        p: &mut Params,
        raw_rem: Option<&str>,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<(&'static str, &'static str, bool)> {
        let mut watched_remembered = false;
        let mut watched_self = false;
        for (k, v) in &mut p.entries {
            if matches!(k.as_str(), "ValidCard" | "ValidSource" | "ValidTarget") {
                if v == "Card.IsTriggerRemembered" {
                    *v = "Card.Self".to_string();
                    watched_remembered = true;
                } else if v == "Card.StrictlySelf" {
                    *v = "Card.Self".to_string();
                    watched_self = true;
                } else if v == "Card.Self" {
                    watched_self = true;
                }
            }
        }
        let Some(rem) = self.delayed_remember(raw_rem, target, chain, true) else {
            return self.deny(format!(
                "a delayed trigger remembering `{}`",
                raw_rem.unwrap_or_default()
            ));
        };
        if watched_remembered {
            if rem == "TriggerRemember::None" {
                return self
                    .deny("a watched delayed trigger with no `RememberObjects$`".to_string());
            }
            Some((rem, "TriggerRemember::None", true))
        } else if watched_self {
            if self.on_a_spell {
                return self.deny("a self-watching delayed trigger on a spell".to_string());
            }
            Some(("TriggerRemember::Source", rem, false))
        } else {
            Some(("TriggerRemember::None", rem, false))
        }
    }

    fn delayed_observation_trigger(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        let this_turn_only = match p.take("ThisTurn").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("a delayed trigger with `ThisTurn$ {other}`")),
        };
        let optional = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => {
                return self.deny(format!("a delayed trigger `OptionalDecider$ {other}`"));
            }
        };
        let valid = p.take("ValidCard")?;
        let raw_rem = p.take("RememberObjects");
        let (watch, remember) = match valid.as_str() {
            "Card.StrictlySelf" | "Card.Self" if !self.on_a_spell => {
                let rem = self.delayed_remember(raw_rem.as_deref(), target, chain, true)?;
                ("TriggerRemember::Source", rem)
            }
            "Card.IsTriggerRemembered" => {
                let w = self.delayed_remember(raw_rem.as_deref(), target, chain, true)?;
                if w == "TriggerRemember::None" {
                    return None;
                }
                (w, "TriggerRemember::None")
            }
            _ => {
                return self.deny(format!(
                    "a `ChangesController` delayed trigger on `{valid}`"
                ));
            }
        };
        let cond = self.delayed_optional_condition(p)?;
        let (effects, targets) = self.parse_delayed_body(p, None, optional)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DelayedTrigger.{key}`"));
        }
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnObservation {{ \
             watch: {watch}, observation: DelayedObservation::YouLoseControl, \
             this_turn_only: {this_turn_only} }}, condition: {cond}, \
             targets: {targets_expr}, remember: {remember}, effects: &[{effects}] }}"
        ))
    }

    pub(super) fn immediate_trigger(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
        starts_whenever: bool,
    ) -> Option<String> {
        if p.has("Announce")
            || p.has("AnnounceMax")
            || p.has("RememberSVarAmount")
            || p.has("TriggerAmount")
        {
            return self
                .deny("an `ImmediateTrigger` with announced or remembered amount".to_string());
        }
        let optional = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => {
                return self.deny(format!("an `ImmediateTrigger` `OptionalDecider$ {other}`"));
            }
        };
        let (when, once, extra_cond) = if p.peek("ConditionDefined") == Some("Remembered") {
            let when = self.immediate_remembered_when(p, chain)?;
            (when, !starts_whenever, None)
        } else if p.has("ConditionDefined") {
            return self.deny("an `ImmediateTrigger` with `ConditionDefined$`".to_string());
        } else {
            let cond_present = p.take("ConditionPresent");
            let cond_clause = if let Some(valid) = cond_present {
                if p.peek("IsPresent") == Some(valid.as_str()) {
                    p.take("IsPresent");
                }
                let mut tmp = Params::default();
                if let Some(cmp) = p.take("ConditionCompare") {
                    tmp.entries.push(("PresentCompare".to_string(), cmp));
                }
                Some(self.present_clause(&mut tmp, &valid)?)
            } else {
                None
            };
            ("ReflexiveWhen::Always".to_string(), true, cond_clause)
        };
        let base_cond = self.delayed_optional_condition(p)?;
        let cond = match (extra_cond, base_cond.as_str()) {
            (None, other) => other.to_string(),
            (Some(c), "None") => format!("Some(&{c})"),
            (Some(c), rest) => {
                let inner = rest.strip_prefix("Some(&")?.strip_suffix(')')?;
                format!("Some(&Condition::All(&[{c}, {inner}]))")
            }
        };
        let raw_rem = p.take("RememberObjects");
        let (effects, targets) = self.parse_delayed_body(p, None, optional)?;
        let reads_event = effects.contains("TargetSpec::EventObject")
            || effects.contains("Effect::SacrificeEvent")
            || effects.contains("Effect::TapEventObject")
            || effects.contains("Amount::Event");
        let remember = match raw_rem.as_deref() {
            None if (self.cost_sacrifices_other || self.cost_sacrifices_self) && reads_event => {
                "TriggerRemember::SacrificedObject"
            }
            Some("Remembered" | "RememberedLKI" | "RememberedCard")
                if when != "ReflexiveWhen::Always" =>
            {
                "TriggerRemember::None"
            }
            other => self.delayed_remember(other, target, chain, reads_event)?,
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ImmediateTrigger.{key}`"));
        }
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::ReflexiveTrigger {{ when: {when}, once: {once}, condition: {cond}, \
             targets: {targets_expr}, remember: {remember}, effects: &[{effects}] }}"
        ))
    }

    fn immediate_remembered_when(&mut self, p: &mut Params, chain: &Chain) -> Option<String> {
        p.take("ConditionDefined");
        if !matches!(p.take("ConditionCompare").as_deref(), None | Some("GE1")) {
            return self
                .deny("an `ImmediateTrigger` with unsupported `ConditionCompare$`".to_string());
        }
        let cond_valid = p.take("ConditionPresent")?;
        let last = chain.effects.last()?;
        let zone_filter = |this: &mut Self, in_zone: bool| -> Option<String> {
            if cond_valid == "Card" {
                return Some("&Filter::Any".to_string());
            }
            let expr = if in_zone {
                this.filter_expr_in_zone(&cond_valid, true)?
            } else {
                this.filter_expr(&cond_valid)?
            };
            Some(format!("&{}", this.body.filter_static("REFLEX", &expr)))
        };
        if last.starts_with("Effect::Mill {") {
            let f = zone_filter(self, true)?;
            return Some(format!("ReflexiveWhen::ThisWay(ThisWay::Milled({f}))"));
        }
        if last.starts_with("Effect::Discard") || last.starts_with("Effect::RevealHandDiscard") {
            if cond_valid == "Card" {
                return Some("ReflexiveWhen::ThisWay(ThisWay::Discarded)".to_string());
            }
            let f = zone_filter(self, true)?;
            return Some(format!(
                "ReflexiveWhen::ThisWay(ThisWay::DiscardedMatching({f}))"
            ));
        }
        if last.starts_with("Effect::RevealUntil") || last.starts_with("Effect::RevealHand {") {
            let f = zone_filter(self, true)?;
            return Some(format!("ReflexiveWhen::ThisWay(ThisWay::Revealed({f}))"));
        }
        if last.starts_with("Effect::SacrificeFilter")
            || last.starts_with("Effect::SacrificeAll")
            || last == "Effect::SacrificeSelf"
        {
            let f = zone_filter(self, false)?;
            return Some(format!(
                "ReflexiveWhen::Event(ReflexiveEvent::Sacrificed({f}))"
            ));
        }
        if last.starts_with("Effect::exile(TargetSpec::CardInGraveyard(") {
            let f = zone_filter(self, true)?;
            return Some(format!(
                "ReflexiveWhen::ThisWay(ThisWay::ExiledFromGraveyard({f}))"
            ));
        }
        if last.starts_with("Effect::exile(") || last.starts_with("Effect::ExileAll {") {
            let f = zone_filter(self, false)?;
            return Some(format!(
                "ReflexiveWhen::ThisWay(ThisWay::ExiledFromBattlefield({f}))"
            ));
        }
        if last.starts_with("Effect::ExileTop {") {
            let f = zone_filter(self, true)?;
            return Some(format!(
                "ReflexiveWhen::ThisWay(ThisWay::ExiledFromLibrary({f}))"
            ));
        }
        if last.starts_with("Effect::bounce(")
            || last.starts_with("Effect::ReturnAllToHand")
            || last.starts_with("Effect::ReturnChosenToHand")
        {
            let f = zone_filter(self, false)?;
            return Some(format!(
                "ReflexiveWhen::ThisWay(ThisWay::ReturnedToHand({f}))"
            ));
        }
        if last.starts_with("Effect::destroy") || last.starts_with("Effect::DestroyAll") {
            let f = zone_filter(self, false)?;
            return Some(format!("ReflexiveWhen::ThisWay(ThisWay::Destroyed({f}))"));
        }
        if last.starts_with("Effect::Tap") {
            let f = zone_filter(self, false)?;
            return Some(format!("ReflexiveWhen::Tapped({f})"));
        }
        None
    }

    fn delayed_remember(
        &self,
        raw: Option<&str>,
        target: Option<&str>,
        chain: &Chain,
        body_uses_remembered: bool,
    ) -> Option<&'static str> {
        let multi_target = chain.target_exact.is_some_and(|n| n > 1)
            || chain.target_up_to.is_some_and(|n| n > 1)
            || chain.target_is_x;
        let obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        match raw {
            None => Some("TriggerRemember::None"),
            Some("Self") if !self.on_a_spell => Some("TriggerRemember::Source"),
            Some("Targeted" | "ParentTarget" | "TargetedCard") if obj_target => {
                Some(if multi_target {
                    "TriggerRemember::EachTarget"
                } else {
                    "TriggerRemember::FirstTarget"
                })
            }
            Some("Targeted") if target.is_none() && !body_uses_remembered => {
                Some("TriggerRemember::None")
            }
            Some("Remembered" | "RememberedLKI" | "RememberedCard") => {
                if chain
                    .effects
                    .iter()
                    .any(|e| e.contains("Effect::CreateToken"))
                {
                    Some("TriggerRemember::EachCreatedToken")
                } else if chain.effects.iter().any(|e| {
                    e.contains("Effect::exile(")
                        || e.contains("Effect::ExileAll")
                        || e.contains("Effect::ExileTop")
                }) {
                    Some("TriggerRemember::EachExiledThisWay")
                } else {
                    None
                }
            }
            Some(def) if self.is_event_object_defined(def) => Some("TriggerRemember::EventObject"),
            Some(_) => None,
        }
    }

    fn delayed_optional_condition(&mut self, p: &mut Params) -> Option<String> {
        let mut clauses = Vec::new();
        let cond_str = self.condition(p)?;
        if let Some(inner) = cond_str
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(inner.to_string());
        }
        if let Some(var) = p.peek("CheckSVar").map(str::to_string) {
            let cmp = p
                .peek("SVarCompare")
                .map_or_else(|| "GE1".to_string(), str::to_string);
            let clause = self
                .svar_condition(&var, &cmp)
                .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                .or_else(|| self.extra_svar_condition_wave8(&var, &cmp))
                .or_else(|| self.wave7_svar_condition(&var, &cmp))
                .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, false))
                .or_else(|| self.wave15_trigger_svar_condition(&var, &cmp))?;
            p.take("CheckSVar");
            p.take("SVarCompare");
            clauses.push(clause);
        }
        if let Some(cond_raw) = p.peek("Condition")
            && let Some(clause) = Self::named_condition(cond_raw)
                .or_else(|| Self::wave7_named_condition(cond_raw))
                .or_else(|| Self::wave21c_named_condition(cond_raw))
        {
            p.take("Condition");
            clauses.push(clause.to_string());
        }
        if let Some(clause) = Self::take_check_defined_player(p) {
            clauses.push(clause.to_string());
        }
        if let Some(bool_clauses) = Self::take_trigger_bool_conditions(p) {
            for clause in bool_clauses {
                clauses.push(clause.to_string());
            }
        }
        Some(match clauses.as_slice() {
            [] => "None".to_string(),
            [only] => format!("Some(&{only})"),
            many => format!("Some(&Condition::All(&[{}]))", many.join(", ")),
        })
    }

    pub(super) fn wave21c_named_condition(cond: &str) -> Option<&'static str> {
        Some(match cond.trim() {
            "Monarch" | "IsMonarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" | "HasInitiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" | "CompletedADungeon" => {
                "Condition::CompletedADungeon(PlayerRel::You)"
            }
            _ => return None,
        })
    }

    /// Consumes supported `CheckDefinedPlayer$` trigger/ETB clauses backed by
    /// `Condition::{IsMonarch, HasInitiative, CompletedADungeon, CommittedCrimeThisTurn}`.
    pub(super) fn take_check_defined_player(p: &mut Params) -> Option<&'static str> {
        let clause = match p.peek("CheckDefinedPlayer")? {
            "You.isMonarch" => "Condition::IsMonarch(PlayerRel::You)",
            "You.!isMonarch" => "Condition::Not(&Condition::IsMonarch(PlayerRel::You))",
            "Opponent.isMonarch" | "Player.Opponent+isMonarch" => {
                "Condition::IsMonarch(PlayerRel::EachOpponent)"
            }
            "Player.isMonarch" => "Condition::IsMonarch(PlayerRel::EachPlayer)",
            "You.hasInitiative" => "Condition::HasInitiative(PlayerRel::You)",
            "You.!hasInitiative" => "Condition::Not(&Condition::HasInitiative(PlayerRel::You))",
            "Opponent.hasInitiative" | "Player.Opponent+hasInitiative" => {
                "Condition::HasInitiative(PlayerRel::EachOpponent)"
            }
            "Player.hasInitiative" => "Condition::HasInitiative(PlayerRel::EachPlayer)",
            "You.completedDungeon" => "Condition::CompletedADungeon(PlayerRel::You)",
            "You.!completedDungeon" => {
                "Condition::Not(&Condition::CompletedADungeon(PlayerRel::You))"
            }
            "Opponent.completedDungeon" | "Player.Opponent+completedDungeon" => {
                "Condition::CompletedADungeon(PlayerRel::EachOpponent)"
            }
            "Player.completedDungeon" => "Condition::CompletedADungeon(PlayerRel::EachPlayer)",
            "You.committedCrimeThisTurn" => "Condition::CommittedCrimeThisTurn(PlayerRel::You)",
            "You.!committedCrimeThisTurn" => {
                "Condition::Not(&Condition::CommittedCrimeThisTurn(PlayerRel::You))"
            }
            "Opponent.committedCrimeThisTurn" | "Player.Opponent+committedCrimeThisTurn" => {
                "Condition::CommittedCrimeThisTurn(PlayerRel::EachOpponent)"
            }
            "Player.committedCrimeThisTurn" => {
                "Condition::CommittedCrimeThisTurn(PlayerRel::EachPlayer)"
            }
            _ => return None,
        };
        p.take("CheckDefinedPlayer");
        Some(clause)
    }

    /// Consumes recognized boolean trigger/ETB condition flags (`PlayerTurn$`,
    /// `OpponentTurn$`, `Revolt$`, `Delirium$`, `Threshold$`, `Metalcraft$`,
    /// `Hellbent$`, `FatefulHour$`, `APlayerHasMoreLifeThanEachOther$`).
    /// Leaves unrecognized values unconsumed so the caller's exhaustion check
    /// refuses them cleanly.
    pub(super) fn take_trigger_bool_conditions(p: &mut Params) -> Option<Vec<&'static str>> {
        const FLAGS: &[(&str, &str)] = &[
            ("PlayerTurn", "Condition::YourTurn"),
            ("NotPlayerTurn", "Condition::Not(&Condition::YourTurn)"),
            ("OpponentTurn", "Condition::OpponentsTurn"),
            ("Blessing", "Condition::CitysBlessing"),
            ("Revolt", "Condition::Revolt"),
            ("Delirium", "Condition::GraveyardTypesAtLeast(4)"),
            ("Threshold", "Condition::GraveyardCountAtLeast(7)"),
            (
                "Metalcraft",
                "Condition::ControlCount(&Filter::ARTIFACT, 3)",
            ),
            ("Hellbent", "Condition::HandSizeAtMost(0)"),
            ("FatefulHour", "Condition::LifeAtMost(5)"),
            (
                "APlayerHasMoreLifeThanEachOther",
                "Condition::APlayerHasMostLife",
            ),
        ];
        if p.peek("PlayerTurn") == Some("True")
            && (p.peek("OpponentTurn") == Some("True") || p.peek("NotPlayerTurn") == Some("True"))
        {
            return None;
        }
        let mut out = Vec::new();
        for &(key, cond) in FLAGS {
            if p.peek(key) == Some("True") {
                p.take(key);
                out.push(cond);
            }
        }
        (!out.is_empty()).then_some(out)
    }

    fn parse_delayed_body(
        &mut self,
        p: &mut Params,
        inner_trigger_mode: Option<String>,
        optional: bool,
    ) -> Option<(String, Option<String>)> {
        let Some(execute) = p.take("Execute") else {
            return self.deny("a delayed trigger with no `Execute$`".to_string());
        };
        let Some(body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        self.parse_delayed_body_str(&body, inner_trigger_mode, optional)
    }

    fn parse_delayed_body_str(
        &mut self,
        body: &str,
        inner_trigger_mode: Option<String>,
        optional: bool,
    ) -> Option<(String, Option<String>)> {
        let mut inner = Chain::default();
        let saved_delayed = std::mem::replace(&mut self.in_delayed, true);
        let saved_mode = std::mem::replace(&mut self.trigger_mode, inner_trigger_mode);
        let saved_self = std::mem::replace(&mut self.trigger_is_self, false);
        let saved_block = self.block_line.take();
        let read = self.chain(body, &mut inner);
        self.in_delayed = saved_delayed;
        self.trigger_mode = saved_mode;
        self.trigger_is_self = saved_self;
        self.block_line = saved_block;
        read?;
        if inner.second_target.is_some() {
            return self.deny("a delayed trigger with a second target".to_string());
        }
        if inner.effects.is_empty() {
            return self.deny("a delayed trigger that reads as no effect".to_string());
        }
        let joined = inner.effects.join(", ");
        let effects = if optional {
            format!("Effect::MayDo {{ effects: &[{joined}] }}")
        } else {
            joined
        };
        let targets = Self::target_req_expr(&inner);
        Some((effects, targets))
    }

    fn delayed_execute_effects(&mut self, p: &mut Params) -> Option<String> {
        let Some(execute) = p.take("Execute") else {
            return self.deny("a delayed trigger with no `Execute$`".to_string());
        };
        let Some(body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        let mut inner = Chain::default();
        let outer = std::mem::replace(&mut self.in_delayed, true);
        let read = self.chain(&body, &mut inner);
        self.in_delayed = outer;
        read?;
        if inner.target.is_some() {
            return self.deny("a delayed trigger that targets".to_string());
        }
        if inner.effects.is_empty() {
            return self.deny("a delayed trigger that reads as no effect".to_string());
        }
        Some(inner.effects.join(", "))
    }

    pub(super) fn is_your_commander_present(valid: &str) -> bool {
        let Some((noun, rest)) = valid.split_once('.') else {
            return false;
        };
        if !matches!(noun.trim(), "Card" | "Permanent") || rest.contains(',') {
            return false;
        }
        let atoms: Vec<&str> = rest.split('+').map(str::trim).collect();
        atoms.len() == 3
            && atoms.contains(&"IsCommander")
            && atoms.contains(&"YouOwn")
            && atoms.contains(&"YouCtrl")
    }

    /// Trigger-time phase/step gate (`Phase$` on a non-`Phase` trigger, e.g.
    /// "during combat", "during the declare attackers step"), always wrapped in
    /// `Condition::TriggerOnly`.
    fn trigger_phase_condition(phase: &str) -> Option<&'static str> {
        match phase {
            "BeginCombat->EndCombat" => Some("Condition::DuringCombat"),
            "Upkeep" => Some("Condition::DuringStep(StepKind::Upkeep)"),
            "Draw" => Some("Condition::DuringStep(StepKind::Draw)"),
            "Main1" => Some("Condition::DuringStep(StepKind::FirstMain)"),
            "Main2" => Some("Condition::DuringStep(StepKind::SecondMain)"),
            "BeginCombat" => Some("Condition::DuringStep(StepKind::CombatBegin)"),
            "Declare Attackers" => Some("Condition::DuringStep(StepKind::DeclareAttackers)"),
            "Declare Blockers" => Some("Condition::DuringStep(StepKind::DeclareBlockers)"),
            "EndCombat" | "End of Combat" => Some("Condition::DuringStep(StepKind::CombatEnd)"),
            "End of Turn" => Some("Condition::DuringStep(StepKind::End)"),
            _ => None,
        }
    }

    /// Wall of Caltrops: `IsPresent$ <some>+Other+sharesBlockingAssignmentWith`
    /// paired with `IsPresent2$ <none>+sharesBlockingAssignmentWith | PresentCompare2$ EQ0`
    /// on a `Trigger::BlocksCreature` trigger -> `Condition::EventBlockedByOthers`.
    fn event_blocked_by_others_clause(&mut self, p: &mut Params) -> Option<String> {
        let valid1 = p.peek("IsPresent")?.to_string();
        let valid2 = p.peek("IsPresent2")?.to_string();
        if !valid1.contains("sharesBlockingAssignmentWith")
            || !valid2.contains("sharesBlockingAssignmentWith")
        {
            return None;
        }
        if p.has("PresentZone")
            || p.has("PresentZone2")
            || p.has("PresentPlayer")
            || p.has("PresentPlayer2")
            || p.has("PresentDefined")
            || p.has("PresentDefined2")
            || p.has("ResolvingCheck")
        {
            return None;
        }
        if !matches!(p.peek("PresentCompare"), None | Some("GE1"))
            || p.peek("PresentCompare2") != Some("EQ0")
        {
            return None;
        }
        let strip_atoms = |raw: &str, require_other: bool| -> Option<String> {
            let mut alts = Vec::new();
            for alt in raw.split(',').map(str::trim) {
                let (base, rest) = alt.split_once('.')?;
                let mut saw_shares = false;
                let mut saw_other = false;
                let mut kept = Vec::new();
                for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                    match atom {
                        "sharesBlockingAssignmentWith" => saw_shares = true,
                        "Other" | "StrictlyOther" if require_other => saw_other = true,
                        other => kept.push(other),
                    }
                }
                if !saw_shares || (require_other && !saw_other) {
                    return None;
                }
                if kept.is_empty() {
                    alts.push(base.to_string());
                } else {
                    alts.push(format!("{base}.{}", kept.join("+")));
                }
            }
            (!alts.is_empty()).then(|| alts.join(","))
        };
        let some_raw = strip_atoms(&valid1, true)?;
        let none_raw = strip_atoms(&valid2, false)?;
        let some_expr = self.filter_expr(&some_raw)?;
        let none_expr = self.filter_expr(&none_raw)?;
        let some_name = self.body.filter_static("CHECK", &some_expr);
        let none_name = self.body.filter_static("CHECK", &none_expr);
        p.take("IsPresent");
        p.take("PresentCompare");
        p.take("IsPresent2");
        p.take("PresentCompare2");
        Some(format!(
            "Condition::EventBlockedByOthers {{ some: &{some_name}, none: &{none_name} }}"
        ))
    }

    /// Reads `IsPresent$` and optional `IsPresent2$` clauses on a non-`Always`
    /// trigger into `out`.
    fn read_trigger_present_clauses(
        &mut self,
        p: &mut Params,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if p.has("IsPresent2") {
            if !p.has("IsPresent") {
                return self.deny("`IsPresent2$` without `IsPresent$`".to_string());
            }
            if p.has("ResolvingCheck") {
                return self
                    .deny("a second `IsPresent2$` clause with `ResolvingCheck$`".to_string());
            }
            let mut p2 = Params::default();
            for (src, dst) in [
                ("IsPresent2", "IsPresent"),
                ("PresentCompare2", "PresentCompare"),
                ("PresentZone2", "PresentZone"),
                ("PresentPlayer2", "PresentPlayer"),
                ("PresentDefined2", "PresentDefined"),
            ] {
                if let Some(val) = p.take(src) {
                    p2.entries.push((dst.to_string(), val));
                }
            }
            let first = self.read_one_trigger_present_clause(p)?;
            out.push(first);
            let second = self.read_one_trigger_present_clause(&mut p2)?;
            if !p2.exhausted() {
                if let Some(key) = p2.first_key() {
                    return self.deny(format!("unclaimed parameter `IsPresent2.{key}`"));
                }
                return None;
            }
            out.push(second);
            return Some(());
        }
        if p.has("IsPresent") {
            let first = self.read_one_trigger_present_clause(p)?;
            out.push(first);
        }
        Some(())
    }

    fn read_one_trigger_present_clause(&mut self, p: &mut Params) -> Option<String> {
        if let Some(clause) = Self::wave8_trigger_present_condition(p) {
            return Some(clause);
        }
        if matches!(p.peek("PresentZone"), None | Some("Battlefield"))
            && !p.has("PresentPlayer")
            && !p.has("PresentDefined")
            && matches!(p.peek("PresentCompare"), None | Some("GE1" | "EQ1"))
            && matches!(
                p.peek("IsPresent"),
                Some(
                    "Card.Self+attackedThisCombat,Card.Self+blockedThisCombat"
                        | "Card.Self+blockedThisCombat,Card.Self+attackedThisCombat"
                )
            )
        {
            p.take("IsPresent");
            p.take("PresentZone");
            p.take("PresentCompare");
            return Some("Condition::AttackedOrBlockedThisCombat".to_string());
        }
        match self.relative_battlefield_present_clause(p) {
            RelativePresent::Matched(clause) => return Some(clause),
            RelativePresent::Denied => return None,
            RelativePresent::NotApplicable => {}
        }
        let cond = self.condition(p)?;
        let Some(inner) = cond
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return self.deny("an `IsPresent$` clause that produced no condition".to_string());
        };
        Some(inner.to_string())
    }

    fn attached_matches_present_clause(
        &mut self,
        p: &mut Params,
        valid: &str,
        compare: &str,
    ) -> RelativePresent {
        if !matches!(compare, "GE1" | "EQ1") {
            return RelativePresent::NotApplicable;
        }
        let mut stripped_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let Some((base, rest)) = alt.split_once('.') else {
                return RelativePresent::NotApplicable;
            };
            let mut saw_attached = false;
            let mut kept = Vec::new();
            for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                match atom {
                    "EnchantedBy" | "EquippedBy" | "AttachedBy" => saw_attached = true,
                    "Self" | "Other" | "StrictlyOther" | "blockedBySource" | "Attached"
                    | "!EnchantedBy" | "!EquippedBy" | "!AttachedBy" => {
                        return RelativePresent::NotApplicable;
                    }
                    other => kept.push(other),
                }
            }
            if !saw_attached {
                return RelativePresent::NotApplicable;
            }
            if kept.is_empty() {
                stripped_alts.push(base.to_string());
            } else {
                stripped_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        let Some(expr) = self.filter_expr(&stripped_alts.join(",")) else {
            return RelativePresent::Denied;
        };
        let name = self.body.filter_static("CHECK", &expr);
        p.take("IsPresent");
        p.take("PresentZone");
        p.take("PresentCompare");
        RelativePresent::Matched(format!("Condition::AttachedMatches(&{name})"))
    }

    /// Handles battlefield `IsPresent$` clauses that reference the source:
    /// - Positive `EnchantedBy` / `EquippedBy` / `AttachedBy` in every
    ///   alternative (`GE1` / `EQ1`) -> `Condition::AttachedMatches(&filter)`,
    ///   which preserves LKI of the attached permanent if the Aura/Equipment
    ///   leaves before resolution.
    /// - `Other`, `StrictlyOther`, `blockedBySource`, `!EquippedBy`,
    ///   `!EnchantedBy`, `!AttachedBy` -> `Condition::BattlefieldCount` /
    ///   `BattlefieldCountAtMost` (which preserve `context.source`, unlike
    ///   `Condition::ControlCount`).
    /// - *Barren Glory*'s `PresentPlayer$ You | IsPresent$ Card.Other |
    ///   PresentCompare$ EQ0`.
    fn relative_battlefield_present_clause(&mut self, p: &mut Params) -> RelativePresent {
        if !matches!(p.peek("PresentZone"), None | Some("Battlefield")) || p.has("PresentDefined") {
            return RelativePresent::NotApplicable;
        }
        let Some(valid) = p.peek("IsPresent").map(str::to_string) else {
            return RelativePresent::NotApplicable;
        };
        if p.peek("PresentPlayer") == Some("You") {
            if matches!(valid.as_str(), "Card.Other" | "Permanent.Other")
                && p.peek("PresentCompare") == Some("EQ0")
            {
                let Some(expr) = self.filter_expr("Permanent.Other+YouCtrl") else {
                    return RelativePresent::Denied;
                };
                let name = self.body.filter_static("CHECK", &expr);
                p.take("IsPresent");
                p.take("PresentZone");
                p.take("PresentPlayer");
                p.take("PresentCompare");
                return RelativePresent::Matched(format!(
                    "Condition::BattlefieldCountAtMost(&{name}, 0)"
                ));
            }
            return RelativePresent::NotApplicable;
        }
        if p.has("PresentPlayer") {
            return RelativePresent::NotApplicable;
        }
        let compare = p
            .peek("PresentCompare")
            .map_or_else(|| "GE1".to_string(), str::to_string);
        if valid.split(',').any(|alt| {
            alt.split(['.', '+'])
                .any(|atom| matches!(atom.trim(), "EnchantedBy" | "EquippedBy" | "AttachedBy"))
        }) {
            return self.attached_matches_present_clause(p, &valid, &compare);
        }
        let has_relative = valid.split(',').any(|alt| {
            alt.split(['.', '+']).any(|atom| {
                matches!(
                    atom.trim(),
                    "Other"
                        | "StrictlyOther"
                        | "blockedBySource"
                        | "!EnchantedBy"
                        | "!EquippedBy"
                        | "!AttachedBy"
                )
            })
        });
        if !has_relative {
            return RelativePresent::NotApplicable;
        }
        if valid.split(',').any(|alt| {
            alt.split(['.', '+'])
                .any(|atom| matches!(atom.trim(), "Self" | "Attached"))
        }) {
            return RelativePresent::NotApplicable;
        }
        let exact_one = compare == "EQ1";
        let bound = if exact_one {
            None
        } else {
            let Some(bound) = count_bound(&compare) else {
                return RelativePresent::NotApplicable;
            };
            Some(bound)
        };
        let atoms = || valid.split(',').flat_map(|alt| alt.split(['.', '+']));
        let yours = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
        let opponents = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"))
            && matches!(bound, Some(Bound::AtMost(0)));
        let nobodys = !atoms().any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
        if !yours && !nobodys && !opponents {
            return RelativePresent::NotApplicable;
        }
        let Some(expr) = self.filter_expr(&valid) else {
            return RelativePresent::Denied;
        };
        let name = self.body.filter_static("CHECK", &expr);
        p.take("IsPresent");
        p.take("PresentZone");
        p.take("PresentCompare");
        RelativePresent::Matched(match bound {
            Some(Bound::AtLeast(n)) => format!("Condition::BattlefieldCount(&{name}, {n})"),
            Some(Bound::AtMost(n)) => format!("Condition::BattlefieldCountAtMost(&{name}, {n})"),
            None => format!(
                "Condition::All(&[Condition::BattlefieldCount(&{name}, 1), Condition::BattlefieldCountAtMost(&{name}, 1)])"
            ),
        })
    }

    fn wave8_trigger_present_condition(p: &mut Params) -> Option<String> {
        if p.has("IsPresent2") || p.has("PresentPlayer") {
            return None;
        }
        let valid = p.peek("IsPresent")?;
        if (valid == "Card.StrictlySelf"
            || (valid == "Card.Self" && p.peek("PresentZone") != Some("Battlefield")))
            && matches!(p.peek("PresentCompare"), None | Some("GE1" | "EQ1"))
        {
            let zone_ref = match p.peek("PresentZone")? {
                "Command" => "ZoneRef::Command",
                "Battlefield" => "ZoneRef::Battlefield",
                "Graveyard" => "ZoneRef::Graveyard",
                "Exile" => "ZoneRef::Exile",
                "Battlefield,Command" | "Command,Battlefield" => "ZoneRef::BattlefieldOrCommand",
                _ => return None,
            };
            p.take("IsPresent");
            p.take("PresentZone");
            p.take("PresentCompare");
            return Some(format!("Condition::SourceInZone({zone_ref})"));
        }
        if Self::is_your_commander_present(valid)
            && matches!(p.peek("PresentZone"), None | Some("Battlefield"))
        {
            let cond = match p.peek("PresentCompare") {
                None | Some("GE1") => "Condition::ControlYourCommander",
                Some("EQ0") => "Condition::Not(&Condition::ControlYourCommander)",
                _ => return None,
            };
            p.take("IsPresent");
            p.take("PresentZone");
            p.take("PresentCompare");
            return Some(cond.to_string());
        }
        if matches!(p.peek("PresentZone"), None | Some("Battlefield"))
            && let Some((base, positive_present)) = match valid {
                "Card.Self+IsMonstrous" | "Card.Self+monstrous" => {
                    Some(("Condition::IsMonstrous", true))
                }
                "Card.Self+!IsMonstrous" | "Card.Self+!monstrous" => {
                    Some(("Condition::IsMonstrous", false))
                }
                "Card.Self+IsSuspected" | "Card.Self+suspected" => {
                    Some(("Condition::IsSuspected", true))
                }
                "Card.Self+!IsSuspected" | "Card.Self+!suspected" => {
                    Some(("Condition::IsSuspected", false))
                }
                "Card.YouCtrl+IsRingbearer"
                | "Creature.YouCtrl+IsRingbearer"
                | "Card.IsRingbearer+YouCtrl"
                | "Creature.IsRingbearer+YouCtrl" => Some(("Condition::HasRingBearer", true)),
                "Card.Self+IsRingbearer" | "Creature.Self+IsRingbearer" => {
                    Some(("Condition::SourceMatches(&Filter::RingBearer)", true))
                }
                _ => None,
            }
        {
            let positive = match p.peek("PresentCompare") {
                None | Some("GE1" | "EQ1" | "GT0") => positive_present,
                Some("EQ0" | "LT1") => !positive_present,
                _ => return None,
            };
            p.take("IsPresent");
            p.take("PresentZone");
            p.take("PresentCompare");
            return Some(if positive {
                base.to_string()
            } else {
                format!("Condition::Not(&{base})")
            });
        }
        None
    }

    fn take_trigger_zone(mode: &str, p: &mut Params) -> &'static str {
        let zone = match p.peek("TriggerZones") {
            Some("Graveyard") => ", zone = TriggerZone::Graveyard",
            Some("Exile") => ", zone = TriggerZone::Exile",
            Some("Command") => ", zone = TriggerZone::Command",
            Some("Battlefield,Command" | "Command,Battlefield") => {
                ", zone = TriggerZone::BattlefieldOrCommand"
            }
            _ => return "",
        };
        let non_self = p
            .peek("ValidCard")
            .is_some_and(|v| !v.split([',', '.', '+']).any(|a| a.trim() == "Self"));
        let non_self_cards = p
            .peek("ValidCards")
            .is_some_and(|v| !v.split([',', '.', '+']).any(|a| a.trim() == "Self"));
        let allowed = match mode {
            "SpellCast" => !p
                .peek("TargetsValid")
                .is_some_and(|v| v.split([',', '.', '+']).any(|a| a.trim() == "Self")),
            "Phase"
            | "DamageDone"
            | "DamageDoneOnce"
            | "DamageDealtOnce"
            | "Drawn"
            | "LandPlayed"
            | "Attacks"
            | "AttackersDeclared"
            | "Attacked"
            | "AttackersDeclaredOneTarget"
            | "AttackerBlocked"
            | "AttackerUnblocked"
            | "Blocks"
            | "BecomesTarget"
            | "LifeGained"
            | "LifeLost"
            | "Discarded"
            | "Cycled"
            | "CounterAddedOnce"
            | "FlippedCoin"
            | "RolledDie"
            | "SearchedLibrary"
            | "BecomesPlotted"
            | "Plotted"
            | "SpellCastOrCopy"
            | "Scry"
            | "GiveGift"
            | "GaveGift"
            | "CommitCrime"
            | "ManaExpend"
            | "RingTemptsYou"
            | "Clashed"
            | "Firebend"
            | "Firebends" => true,
            "AbilityCast" => !p
                .peek("ValidCard")
                .is_some_and(|v| v.split([',', '.', '+']).any(|a| a.trim() == "Self")),
            "ChangesZone" | "TapsForMana" | "Taps" | "Untaps" | "Exerted" | "Enlisted"
            | "Sacrificed" => non_self,
            "Exploited" => p
                .peek("ValidSource")
                .is_some_and(|v| !v.split([',', '.', '+']).any(|a| a.trim() == "Self")),
            "ChangesZoneAll" => non_self_cards,
            _ => false,
        };
        if allowed {
            p.take("TriggerZones");
            zone
        } else {
            ""
        }
    }

    pub(super) fn wave15_amount_cmp(lhs: &str, cmp: &str) -> Option<String> {
        let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
        let n: u32 = rhs_raw.parse().ok()?;
        let op = match op_code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        Some(format!(
            "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &Amount::Fixed({n}) }}"
        ))
    }

    fn wave15_entered_subtype_consts(&self, rest: &str) -> Option<Vec<String>> {
        let mut out = Vec::new();
        for alt in rest.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let sub_word = alt
                .strip_suffix(".YouCtrl")
                .or_else(|| {
                    alt.strip_prefix("Creature.")
                        .and_then(|s| s.strip_suffix("+YouCtrl"))
                })
                .or_else(|| {
                    alt.strip_prefix("Permanent.")
                        .and_then(|s| s.strip_suffix("+YouCtrl"))
                })?
                .trim();
            let konst = self.single_subtype_const(sub_word).or_else(|| {
                if sub_word.is_empty() || sub_word.contains(['.', '+', ',', ' ', '!']) {
                    return None;
                }
                let expr = self.filter_expr(&format!("Card.{sub_word}"))?;
                expr.strip_prefix("Filter::HasSubtype(")
                    .and_then(|s| s.strip_suffix(')'))
                    .map(str::to_string)
            })?;
            if !out.contains(&konst) {
                out.push(konst);
            }
        }
        (!out.is_empty()).then_some(out)
    }

    /// Wave-15c trigger `CheckSVar$` / `SVarCompare$` clauses backed by
    /// existing `Condition` / `Amount` variants.
    pub(super) fn wave15_trigger_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let trimmed = var.trim();
        let raw = if let Some(body) = self.svars.get(trimmed) {
            body.trim()
        } else if trimmed.starts_with("Count$")
            || trimmed.starts_with("PlayerCount")
            || trimmed.starts_with("DungeonsCompleted$")
        {
            trimmed
        } else {
            return None;
        };
        let resolved_cmp;
        let cmp = {
            let trimmed_cmp = cmp.trim();
            if let Some((op, rhs)) = trimmed_cmp.split_at_checked(2)
                && matches!(op, "GE" | "GT" | "LE" | "LT" | "EQ" | "NE")
                && rhs.parse::<u32>().is_err()
                && let Some(num) = self
                    .svars
                    .get(rhs)
                    .and_then(|s| s.trim().strip_prefix("Number$"))
                    .and_then(|n| n.trim().parse::<u32>().ok())
            {
                resolved_cmp = format!("{op}{num}");
                resolved_cmp.as_str()
            } else {
                trimmed_cmp
            }
        };
        let bool_flag = |cond: &str| match cmp {
            "GE1" | "GT0" => Some(cond.to_string()),
            "EQ0" | "LT1" => Some(format!("Condition::Not(&{cond})")),
            _ => None,
        };
        if raw == "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" {
            return Self::wave15_amount_cmp(
                "Amount::CreaturesEnteredUnderYourControlThisTurn",
                cmp,
            );
        }
        if let Some(rest) = raw.strip_prefix("Count$ThisTurnEntered_Battlefield_")
            && let Some(sub_consts) = self.wave15_entered_subtype_consts(rest)
        {
            return bool_flag(&format!(
                "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{}])",
                sub_consts.join(", ")
            ));
        }
        if matches!(
            raw,
            "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl"
                | "Count$ThisTurnCast_Sorcery.YouCtrl,Instant.YouCtrl"
        ) {
            return bool_flag("Condition::CastInstantOrSorceryThisTurn");
        }
        if raw == "Count$ThisTurnCast_Creature.YouCtrl" {
            return bool_flag("Condition::CastCreatureThisTurn");
        }
        if raw == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" {
            if let Some(n) = cmp
                .strip_prefix("GE")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&n| n >= 1)
                .or_else(|| {
                    cmp.strip_prefix("GT")
                        .and_then(|s| s.parse::<u32>().ok())
                        .and_then(|n| n.checked_add(1))
                })
            {
                return Some(format!("Condition::CreaturesDiedThisTurnAtLeast({n})"));
            }
            return matches!(cmp, "EQ0" | "LT1").then(|| {
                "Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string()
            });
        }
        if raw == "Count$CardPower" && !self.on_a_spell {
            return Self::wave15_amount_cmp("Amount::SourcePower", cmp);
        }
        if raw == "PlayerCountPropertyYou$OpponentsAttackedThisCombat" {
            return Self::wave15_amount_cmp(self.opponents_attacked_amount(), cmp);
        }
        if raw == "Count$LifeYouGainedThisTurn" {
            return Self::wave15_amount_cmp("Amount::LifeGainedThisTurn(PlayerRel::You)", cmp);
        }
        Self::wave16_svar_condition(raw, cmp).or_else(|| self.wave19c_svar_condition(raw, cmp))
    }

    /// Wave-16c shared `CheckSVar$` / `ConditionCheckSVar$` clauses for Wave 7b/8a
    /// (`WarpCostPaid`, `SpellWarpedThisTurn`, `SnowManaWasSpent`,
    /// `OpponentDealtDamageThisTurn`) and `Phase.CheckSVar` life/hand/graveyard counts.
    pub(super) fn wave16_svar_condition(raw: &str, cmp: &str) -> Option<String> {
        let bool_flag = |cond: &str| match cmp {
            "GE1" | "GT0" => Some(cond.to_string()),
            "EQ0" | "LT1" => Some(format!("Condition::Not(&{cond})")),
            _ => None,
        };
        match raw {
            "Count$Warped.1.0" | "Count$Warped" | "Count$Warp.1.0" | "Count$Warp" => match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::WarpCostPaid".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::WarpCostPaid)".to_string()),
                _ => None,
            },
            "Count$Warped.0.1" | "Count$Warp.0.1" => match cmp {
                "EQ0" | "LT1" => Some("Condition::WarpCostPaid".to_string()),
                "GE1" | "EQ1" | "GT0" => {
                    Some("Condition::Not(&Condition::WarpCostPaid)".to_string())
                }
                _ => None,
            },
            "Count$CastTotalManaSpent Snow" => bool_flag("Condition::SnowManaWasSpent")
                .or_else(|| Self::wave15_amount_cmp("Amount::SnowManaSpentToCast", cmp)),
            "Count$ThisTurnCast_Card.CastSa Spell.Warp" | "Count$ThisTurnCast_Spell.Warp" => {
                bool_flag("Condition::SpellWarpedThisTurn(None)").or_else(|| {
                    Self::wave15_amount_cmp(
                        "Amount::SpellsWarpedThisTurn(PlayerRel::EachPlayer)",
                        cmp,
                    )
                })
            }
            "Count$ThisTurnCast_Card.YouCtrl+CastSa Spell.Warp"
            | "Count$ThisTurnCast_Card.CastSa Spell.Warp+YouCtrl" => {
                bool_flag("Condition::SpellWarpedThisTurn(Some(PlayerRel::You))").or_else(|| {
                    Self::wave15_amount_cmp("Amount::SpellsWarpedThisTurn(PlayerRel::You)", cmp)
                })
            }
            "Count$ThisTurnCast_Card.OppCtrl+CastSa Spell.Warp"
            | "Count$ThisTurnCast_Card.CastSa Spell.Warp+OppCtrl" => bool_flag(
                "Condition::SpellWarpedThisTurn(Some(PlayerRel::EachOpponent))",
            )
            .or_else(|| {
                Self::wave15_amount_cmp(
                    "Amount::SpellsWarpedThisTurn(PlayerRel::EachOpponent)",
                    cmp,
                )
            }),
            "Count$DamageOpponentsTookThisTurn" => {
                bool_flag("Condition::OpponentDealtDamageThisTurn").or_else(|| {
                    Self::wave15_amount_cmp("Amount::DamageDealtToOpponentsThisTurn", cmp)
                })
            }
            "Count$OpponentsDamagedThisTurn"
            | "PlayerCountOpponents$HasPropertyDamagedThisTurn" => {
                bool_flag("Condition::OpponentDealtDamageThisTurn")
            }
            "Count$Monstrous"
            | "Count$Monstrous.1.0"
            | "Count$ValidSelf Card.IsMonstrous"
            | "Count$Valid Card.Self+IsMonstrous"
            | "Count$ValidSelf Card.monstrous"
            | "Count$Valid Card.Self+monstrous" => bool_flag("Condition::IsMonstrous"),
            "Count$Suspected"
            | "Count$Suspected.1.0"
            | "Count$ValidSelf Card.IsSuspected"
            | "Count$Valid Card.Self+IsSuspected"
            | "Count$ValidSelf Card.suspected"
            | "Count$Valid Card.Self+suspected" => bool_flag("Condition::IsSuspected"),
            "Count$YourLifeTotal" => {
                Self::wave15_amount_cmp("Amount::LifeTotal(PlayerRel::You)", cmp)
            }
            "Count$ValidHand Card.YouOwn" => {
                Self::wave15_amount_cmp("Amount::HandSize(PlayerRel::You)", cmp)
            }
            "Count$ValidGraveyard Card.YouOwn" => Self::wave16_graveyard_count_cmp(cmp),
            _ => Self::wave18c_svar_condition(raw, cmp),
        }
    }

    fn wave18c_svar_condition(raw: &str, cmp: &str) -> Option<String> {
        let bool_flag = |cond: &str| match cmp {
            "GE1" | "GT0" => Some(cond.to_string()),
            "EQ0" | "LT1" => Some(format!("Condition::Not(&{cond})")),
            _ => None,
        };
        match raw {
            "Count$CommittedCrimeThisTurn.1.0" | "Count$CommittedCrimeThisTurn" => {
                bool_flag("Condition::CommittedCrimeThisTurn(PlayerRel::You)")
            }
            "Count$CommittedCrimeThisTurn.0.1" => match cmp {
                "EQ0" | "LT1" => Some("Condition::CommittedCrimeThisTurn(PlayerRel::You)".into()),
                "GE1" | "EQ1" | "GT0" => Some(
                    "Condition::Not(&Condition::CommittedCrimeThisTurn(PlayerRel::You))".into(),
                ),
                _ => None,
            },
            "Count$Morbid.1.0" | "Count$Morbid" => {
                bool_flag("Condition::CreaturesDiedThisTurnAtLeast(1)")
            }
            "Count$Morbid.0.1" => match cmp {
                "EQ0" | "LT1" => Some("Condition::CreaturesDiedThisTurnAtLeast(1)".into()),
                "GE1" | "EQ1" | "GT0" => {
                    Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".into())
                }
                _ => None,
            },
            "Count$ThisTurnCast_Card.YouCtrl" => match cmp {
                "EQ0" | "LT1" => Some("Condition::YouCastNoSpellThisTurn".to_string()),
                "GE1" | "GT0" => {
                    Some("Condition::Not(&Condition::YouCastNoSpellThisTurn)".to_string())
                }
                _ => None,
            },
            "Count$ManaExpendedThisTurn" | "Count$ManaSpentOnSpellsThisTurn" => {
                if let Some(n) = cmp
                    .strip_prefix("GE")
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n >= 1)
                    .or_else(|| {
                        cmp.strip_prefix("GT")
                            .and_then(|s| s.parse::<u32>().ok())
                            .and_then(|n| n.checked_add(1))
                    })
                {
                    Some(format!("Condition::ExpendedThisTurn({n})"))
                } else {
                    Self::wave15_amount_cmp(
                        "Amount::ManaExpendedOnSpellsThisTurn(PlayerRel::You)",
                        cmp,
                    )
                }
            }
            "Count$Valid Creature.YouCtrl+IsRingbearer"
            | "Count$Valid Card.YouCtrl+IsRingbearer"
            | "Count$Valid Creature.IsRingbearer+YouCtrl"
            | "Count$Valid Card.IsRingbearer+YouCtrl" => bool_flag("Condition::HasRingBearer"),
            "Count$Valid Card.Self+IsRingbearer" | "Count$ValidSelf Card.IsRingbearer" => {
                bool_flag("Condition::SourceMatches(&Filter::RingBearer)")
            }
            _ => Self::wave20c_named_svar_flag(raw, cmp),
        }
    }

    fn wave20c_named_svar_flag(raw: &str, cmp: &str) -> Option<String> {
        // Counts that may exceed 1 (dungeons completed by you, or multiple
        // players/opponents having completed a dungeon in multiplayer): only
        // zero vs non-zero comparisons are exact against boolean
        // `Condition::CompletedADungeon`.
        let multi_count_cond = match raw {
            "DungeonsCompleted$Amount" => Some("Condition::CompletedADungeon(PlayerRel::You)"),
            "PlayerCountPlayers$HasPropertycompletedDungeon" => {
                Some("Condition::CompletedADungeon(PlayerRel::EachPlayer)")
            }
            "PlayerCountOpponents$HasPropertycompletedDungeon" => {
                Some("Condition::CompletedADungeon(PlayerRel::EachOpponent)")
            }
            _ => None,
        };
        if let Some(cond) = multi_count_cond {
            return match cmp {
                "GE1" | "GT0" | "NE0" => Some(cond.to_string()),
                "EQ0" | "LT1" | "LE0" => Some(format!("Condition::Not(&{cond})")),
                _ => None,
            };
        }
        let (base, inverted) = match raw {
            "Count$Revolt" | "Count$Revolt.1.0" => ("Condition::Revolt", false),
            "Count$Revolt.0.1" => ("Condition::Revolt", true),
            "Count$Delirium" | "Count$Delirium.1.0" => {
                ("Condition::GraveyardTypesAtLeast(4)", false)
            }
            "Count$Delirium.0.1" => ("Condition::GraveyardTypesAtLeast(4)", true),
            "Count$Threshold" | "Count$Threshold.1.0" => {
                ("Condition::GraveyardCountAtLeast(7)", false)
            }
            "Count$Threshold.0.1" => ("Condition::GraveyardCountAtLeast(7)", true),
            "Count$Metalcraft" | "Count$Metalcraft.1.0" => {
                ("Condition::ControlCount(&Filter::ARTIFACT, 3)", false)
            }
            "Count$Metalcraft.0.1" => ("Condition::ControlCount(&Filter::ARTIFACT, 3)", true),
            "Count$Hellbent" | "Count$Hellbent.1.0" => ("Condition::HandSizeAtMost(0)", false),
            "Count$Hellbent.0.1" => ("Condition::HandSizeAtMost(0)", true),
            "Count$FatefulHour" | "Count$FatefulHour.1.0" => ("Condition::LifeAtMost(5)", false),
            "Count$FatefulHour.0.1" => ("Condition::LifeAtMost(5)", true),
            // CR 724.1 / CR 725.1: at most one player is the monarch or has the
            // initiative at a time, so these counts are strictly 0 or 1.
            "Count$Monarch"
            | "Count$Monarch.1.0"
            | "PlayerCountPropertyYou$HasPropertyisMonarch" => {
                ("Condition::IsMonarch(PlayerRel::You)", false)
            }
            "Count$Monarch.0.1" => ("Condition::IsMonarch(PlayerRel::You)", true),
            "PlayerCountPlayers$HasPropertyisMonarch" => {
                ("Condition::IsMonarch(PlayerRel::EachPlayer)", false)
            }
            "PlayerCountOpponents$HasPropertyisMonarch" => {
                ("Condition::IsMonarch(PlayerRel::EachOpponent)", false)
            }
            "Count$Initiative"
            | "Count$Initiative.1.0"
            | "PlayerCountPropertyYou$HasPropertyhasInitiative" => {
                ("Condition::HasInitiative(PlayerRel::You)", false)
            }
            "Count$Initiative.0.1" => ("Condition::HasInitiative(PlayerRel::You)", true),
            "PlayerCountPlayers$HasPropertyhasInitiative" => {
                ("Condition::HasInitiative(PlayerRel::EachPlayer)", false)
            }
            "PlayerCountOpponents$HasPropertyhasInitiative" => {
                ("Condition::HasInitiative(PlayerRel::EachOpponent)", false)
            }
            "Count$CompletedDungeon"
            | "Count$CompletedDungeon.1.0"
            | "PlayerCountPropertyYou$HasPropertycompletedDungeon" => {
                ("Condition::CompletedADungeon(PlayerRel::You)", false)
            }
            "Count$CompletedDungeon.0.1" => ("Condition::CompletedADungeon(PlayerRel::You)", true),
            _ => return None,
        };
        let want_true = match cmp {
            "GE1" | "GT0" | "EQ1" | "NE0" => !inverted,
            "EQ0" | "LT1" | "LE0" => inverted,
            _ => return None,
        };
        Some(if want_true {
            base.to_string()
        } else {
            format!("Condition::Not(&{base})")
        })
    }

    fn wave16_graveyard_count_cmp(cmp: &str) -> Option<String> {
        let (op, n_str) = cmp.split_at_checked(2)?;
        let n: u8 = n_str.parse().ok()?;
        match op {
            "GE" if n >= 1 => Some(format!("Condition::GraveyardCountAtLeast({n})")),
            "GT" => Some(format!(
                "Condition::GraveyardCountAtLeast({})",
                n.checked_add(1)?
            )),
            "LE" => Some(format!(
                "Condition::Not(&Condition::GraveyardCountAtLeast({}))",
                n.checked_add(1)?
            )),
            "LT" if n >= 1 => Some(format!(
                "Condition::Not(&Condition::GraveyardCountAtLeast({n}))"
            )),
            "EQ" if n == 0 => {
                Some("Condition::Not(&Condition::GraveyardCountAtLeast(1))".to_string())
            }
            _ => None,
        }
    }

    fn wave19c_amount_op(code: &str) -> Option<&'static str> {
        Some(match code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        })
    }

    fn wave19c_at_least_u32(cmp: &str) -> Option<u32> {
        if let Some(n) = cmp.strip_prefix("GE").and_then(|s| s.parse::<u32>().ok()) {
            return (n >= 1).then_some(n);
        }
        cmp.strip_prefix("GT")
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|n| n.checked_add(1))
    }

    fn wave19c_at_most_u32(cmp: &str) -> Option<u32> {
        if cmp == "EQ0" {
            return Some(0);
        }
        if let Some(n) = cmp.strip_prefix("LE").and_then(|s| s.parse::<u32>().ok()) {
            return Some(n);
        }
        cmp.strip_prefix("LT")
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|n| n.checked_sub(1))
    }

    fn wave22c_svar_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        if raw == "Count$MaxOppDamageThisTurn" {
            if let Some(n) = Self::wave19c_at_least_u32(cmp) {
                return Some(format!("Condition::OpponentWasDealtAtLeast({n})"));
            }
            if let Some(max) = Self::wave19c_at_most_u32(cmp)
                && let Some(min) = max.checked_add(1)
            {
                return Some(format!(
                    "Condition::Not(&Condition::OpponentWasDealtAtLeast({min}))"
                ));
            }
            if let Some(n) = cmp
                .strip_prefix("EQ")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&v| v >= 1)
                && let Some(next) = n.checked_add(1)
            {
                return Some(format!(
                    "Condition::All(&[Condition::OpponentWasDealtAtLeast({n}), \
                     Condition::Not(&Condition::OpponentWasDealtAtLeast({next}))])"
                ));
            }
            return None;
        }
        if raw == "PlayerCountPropertyYou$DomainPlayer" {
            return Self::wave15_amount_cmp("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)", cmp);
        }
        if raw == "PlayerCountPropertyYou$OpponentsAttackedThisCombat" {
            return Self::wave15_amount_cmp("Amount::OpponentsYouAttackedThisCombat", cmp);
        }
        if let Some(valid) = raw.strip_prefix("PlayerCountOpponents$LowestValid ") {
            let neutral_valid = Self::wave19c_rewrite_you_ctrl(valid.trim(), None)?;
            if let Some(max) = Self::wave19c_at_most_u32(cmp) {
                let expr = self.filter_expr(&neutral_valid)?;
                let name = self.body.filter_static("CHECK", &expr);
                return Some(format!(
                    "Condition::OpponentControlCountAtMost(&{name}, {max})"
                ));
            }
            if let Some(min) = Self::wave19c_at_least_u32(cmp)
                && let Some(max) = min.checked_sub(1)
            {
                let expr = self.filter_expr(&neutral_valid)?;
                let name = self.body.filter_static("CHECK", &expr);
                return Some(format!(
                    "Condition::Not(&Condition::OpponentControlCountAtMost(&{name}, {max}))"
                ));
            }
            return None;
        }
        if raw == "PlayerCountPlayers$LowestCardsInHand" {
            if let Some(max) = Self::wave19c_at_most_u32(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::APlayerHandSizeAtMost({max})"));
            }
            if let Some(max) = Self::wave19c_at_least_u32(cmp)
                .and_then(|v| v.checked_sub(1))
                .and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::APlayerHandSizeAtMost({max}))"
                ));
            }
            return None;
        }
        if raw == "PlayerCountPlayers$HighestCardsInHand" {
            return match cmp {
                "EQ0" | "LE0" | "LT1" => Some("Condition::EveryHandEmpty".to_string()),
                "GE1" | "GT0" | "NE0" => {
                    Some("Condition::Not(&Condition::EveryHandEmpty)".to_string())
                }
                _ => None,
            };
        }
        None
    }

    /// Wave-19c `Phase.CheckSVar` and trigger/ETB `CheckSVar$` lowerings backed
    /// by existing `Condition` / `Amount` variants.
    #[allow(clippy::too_many_lines)]
    fn wave19c_svar_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        if raw == "ExiledWith$Amount" && !self.on_a_spell {
            return Self::wave15_amount_cmp(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::LinkedExile }",
                cmp,
            );
        }
        if raw == "PlayerCountPropertyYou$DamageThisTurn" {
            return Self::wave15_amount_cmp("Amount::DamageDealtToYouThisTurn", cmp);
        }
        if let Some(cond) = self.wave22c_svar_condition(raw, cmp) {
            return Some(cond);
        }
        if matches!(
            raw,
            "PlayerCountOpponents$HighestLifeLostThisTurn"
                | "PlayerCountRegisteredOpponents$HighestLifeLostThisTurn"
        ) {
            return match cmp {
                "GE1" | "GT0" | "NE0" => Some("Condition::OpponentLostLifeThisTurn".to_string()),
                "EQ0" | "LE0" | "LT1" => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if let Some(valid) = raw.strip_prefix("Count$Valid ")
            && let Some((op, rhs_var)) = cmp.split_at_checked(2)
            && matches!(op, "GT" | "LE")
            && let Some(rhs_def) = self.svars.get(rhs_var.trim()).map(|s| s.trim())
            && rhs_def.strip_prefix("PlayerCountOther$HighestValid ") == Some(valid.trim())
            && let Some(neutral) = Self::wave19c_rewrite_you_ctrl(valid.trim(), None)
        {
            let expr = self.filter_expr(&neutral)?;
            let filter_ref = match expr.as_str() {
                "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
                "Filter::LAND" => "&Filter::LAND".to_string(),
                "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
                "Filter::PERMANENT" => "&Filter::PERMANENT".to_string(),
                _ => format!("&{}", self.body.filter_static("CHECK", &expr)),
            };
            return Some(if op == "GT" {
                format!("Condition::ControlMoreThanEachOtherPlayer({filter_ref})")
            } else {
                format!("Condition::Not(&Condition::ControlMoreThanEachOtherPlayer({filter_ref}))")
            });
        }
        if let Some(sub_raw) = raw.strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ") {
            return self.wave19c_sacrificed_subtype_condition(sub_raw.trim(), cmp);
        }
        if let Some(valid) = raw.strip_prefix("PlayerCountOpponents$HighestValid ") {
            return self.wave19c_opp_highest_valid(valid.trim(), cmp);
        }
        if let Some(valid) = raw
            .strip_prefix("Count$Valid ")
            .and_then(|s| s.strip_suffix("$DifferentCardNames"))
            && valid
                .split(',')
                .all(|alt| alt.split(['.', '+']).any(|a| a.trim() == "YouCtrl"))
            && let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(min) = max.checked_add(1).and_then(|n| u8::try_from(n).ok())
        {
            let expr = self.filter_expr(valid)?;
            let name = self.body.filter_static("CHECK", &expr);
            return Some(format!(
                "Condition::Not(&Condition::ControlDistinctNames(&{name}, {min}))"
            ));
        }
        if matches!(
            raw,
            "Count$TypeInYourYard" | "Count$ValidGraveyard Card.YouOwn$CardTypes"
        ) && let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(min) = max.checked_add(1).and_then(|n| u8::try_from(n).ok())
        {
            return Some(format!(
                "Condition::Not(&Condition::GraveyardTypesAtLeast({min}))"
            ));
        }
        if raw == "Count$ValidGraveyard Card.YouOwn$CardTypesPermanent"
            && let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(min) = max.checked_add(1).and_then(|n| u8::try_from(n).ok())
        {
            return Some(format!(
                "Condition::Not(&Condition::GraveyardTypesAmongAtLeast {{ \
                 types: TypeSet::ARTIFACT.union(TypeSet::BATTLE).union(TypeSet::CREATURE)\
                 .union(TypeSet::ENCHANTMENT).union(TypeSet::LAND).union(TypeSet::PLANESWALKER), \
                 minimum: {min} }})"
            ));
        }
        if raw == "Count$ThisTurnLandPlayed" {
            if let Some(n) = Self::wave19c_at_least_u32(cmp).and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::LandsPlayedThisTurnAtLeast({n})"));
            }
            if let Some(max) = Self::wave19c_at_most_u32(cmp)
                && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
            {
                return Some(format!(
                    "Condition::Not(&Condition::LandsPlayedThisTurnAtLeast({min}))"
                ));
            }
        }
        if raw == "PlayerCountOpponents$HighestCounters.Poison"
            && let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(min) = max.checked_add(1).and_then(|v| u16::try_from(v).ok())
        {
            return Some(format!(
                "Condition::Not(&Condition::OpponentPoisonAtLeast({min}))"
            ));
        }
        if raw == "Count$YourLifeTotal"
            && let Some((op_code, rhs_var)) = cmp.split_at_checked(2)
            && let Some(op) = Self::wave19c_amount_op(op_code)
            && let Some((rhs_amt, bare_starting)) = self.wave19c_starting_life_rhs(rhs_var, false)
        {
            if op_code == "LT" && bare_starting {
                return Some("Condition::LifeBelowStarting".to_string());
            }
            return Some(format!(
                "Condition::AmountCompare {{ left: &Amount::LifeTotal(PlayerRel::You), \
                 op: {op}, right: &{rhs_amt} }}"
            ));
        }
        if self.trigger_mode.as_deref() == Some("Phase") {
            return self.wave19c_phase_triggered_player_condition(raw, cmp);
        }
        None
    }

    fn wave19c_sacrificed_subtype_condition(&self, sub: &str, cmp: &str) -> Option<String> {
        if sub.is_empty() || sub.contains(['.', '+', ',', ' ', '!']) {
            return None;
        }
        let path = self.cats.const_path(sub)?;
        if let Some(count) = Self::wave19c_at_least_u32(cmp) {
            return Some(format!(
                "Condition::SacrificedSubtypeThisTurn {{ subtype: {path}, count: {count} }}"
            ));
        }
        if let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(count) = max.checked_add(1)
        {
            return Some(format!(
                "Condition::Not(&Condition::SacrificedSubtypeThisTurn {{ \
                 subtype: {path}, count: {count} }})"
            ));
        }
        None
    }

    fn wave19c_starting_life_rhs(
        &self,
        rhs_var: &str,
        active_player: bool,
    ) -> Option<(String, bool)> {
        let def = self.svars.get(rhs_var.trim())?.trim();
        let (base, tail) = if active_player {
            (
                "Amount::StartingLifeTotal(PlayerRel::ActivePlayer)",
                def.strip_prefix("TriggeredPlayer$StartingLife")?,
            )
        } else {
            (
                "Amount::StartingLifeTotal(PlayerRel::You)",
                def.strip_prefix("Count$StartingLife")
                    .or_else(|| def.strip_prefix("Count$YourStartingLife"))?,
            )
        };
        let tail = tail.trim();
        if tail.is_empty() {
            return Some((base.to_string(), true));
        }
        let expr = if tail == "/HalfDown" {
            format!("Amount::Half {{ base: &{base}, round_up: false }}")
        } else if tail == "/HalfUp" {
            format!("Amount::Half {{ base: &{base}, round_up: true }}")
        } else if tail == "/Twice" {
            format!("Amount::Times {{ base: &{base}, factor: 2 }}")
        } else if let Some(n) = tail
            .strip_prefix("/Plus.")
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            format!("Amount::Plus {{ base: &{base}, offset: {n} }}")
        } else {
            let n = tail
                .strip_prefix("/Minus.")
                .and_then(|s| s.trim().parse::<u32>().ok())?;
            format!("Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}")
        };
        Some((expr, false))
    }

    fn wave19c_rewrite_you_ctrl(valid: &str, replacement: Option<&str>) -> Option<String> {
        if valid.contains(['$', '/']) {
            return None;
        }
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.')?;
            let mut saw_you = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == "YouCtrl" {
                    saw_you = true;
                    if let Some(rep) = replacement {
                        kept.push(rep);
                    }
                } else if atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom == "Self"
                    || atom == "StrictlySelf"
                    || atom == "Other"
                    || atom == "StrictlyOther"
                {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if !saw_you {
                return None;
            }
            if kept.is_empty() {
                out.push(base.trim().to_string());
            } else {
                out.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    fn wave19c_opp_highest_valid(&mut self, valid: &str, cmp: &str) -> Option<String> {
        let neutral_valid = Self::wave19c_rewrite_you_ctrl(valid, None)?;
        if let Some((op_code, rhs_var)) = cmp.split_at_checked(2)
            && matches!(op_code, "GT" | "GE")
            && !rhs_var.is_empty()
            && !rhs_var.chars().all(|c| c.is_ascii_digit())
            && let Some(rhs_def) = self.svars.get(rhs_var.trim()).map(|s| s.trim().to_string())
        {
            let rest = rhs_def.strip_prefix("Count$Valid ")?;
            let (rhs_valid, plus_k) = match rest.split_once("/Plus.") {
                Some((v, k_raw)) => (v.trim(), Some(k_raw.trim().parse::<u32>().ok()?)),
                None => (rest.trim(), None),
            };
            if rhs_valid != valid {
                return None;
            }
            let expr = self.filter_expr(&neutral_valid)?;
            let name = self.body.filter_static("CHECK", &expr);
            return match (op_code, plus_k) {
                ("GT", None) => Some(format!("Condition::OpponentControlsMore(&{name})")),
                ("GE", Some(k)) if k >= 1 => {
                    Some(format!("Condition::OpponentControlsMoreBy(&{name}, {k})"))
                }
                ("GT", Some(k)) => {
                    let min = k.checked_add(1)?;
                    Some(format!("Condition::OpponentControlsMoreBy(&{name}, {min})"))
                }
                _ => None,
            };
        }
        let opp_valid = Self::wave19c_rewrite_you_ctrl(valid, Some("OppCtrl"))?;
        if let Some(n) = Self::wave19c_at_least_u32(cmp).and_then(|v| u8::try_from(v).ok()) {
            let expr = self.filter_expr(&opp_valid)?;
            let name = self.body.filter_static("CHECK", &expr);
            return Some(format!("Condition::OpponentControlCount(&{name}, {n})"));
        }
        if let Some(max) = Self::wave19c_at_most_u32(cmp)
            && let Some(min) = max.checked_add(1).and_then(|v| u8::try_from(v).ok())
        {
            let expr = self.filter_expr(&opp_valid)?;
            let name = self.body.filter_static("CHECK", &expr);
            return Some(format!(
                "Condition::Not(&Condition::OpponentControlCount(&{name}, {min}))"
            ));
        }
        None
    }

    fn wave19c_strip_controlled_by_triggered(valid: &str) -> Option<String> {
        if valid.contains(['$', '/']) {
            return None;
        }
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.')?;
            let mut saw = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == "ControlledBy TriggeredPlayer" {
                    saw = true;
                } else if atom.contains("Ctrl")
                    || atom.contains("Own")
                    || atom.contains("ControlledBy")
                    || atom == "Self"
                    || atom == "StrictlySelf"
                    || atom == "Other"
                    || atom == "StrictlyOther"
                {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if !saw {
                return None;
            }
            if kept.is_empty() {
                out.push(base.trim().to_string());
            } else {
                out.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    fn wave19c_phase_triggered_player_condition(&mut self, raw: &str, cmp: &str) -> Option<String> {
        if let Some(valid) = raw.strip_prefix("Count$Valid ")
            && let Some(base_valid) = Self::wave19c_strip_controlled_by_triggered(valid.trim())
        {
            let expr = self.filter_expr(&base_valid)?;
            let filter = self.body.filter_static("CHECK", &expr);
            let lhs = format!(
                "Amount::ControlledCount {{ who: PlayerRel::ActivePlayer, filter: &{filter} }}"
            );
            if let Some(res) = Self::wave15_amount_cmp(&lhs, cmp) {
                return Some(res);
            }
            if let Some((op_code, rhs_var)) = cmp.split_at_checked(2)
                && let Some(op) = Self::wave19c_amount_op(op_code)
                && let Some(rhs_def) = self.svars.get(rhs_var.trim())
                && let Some(rhs_valid) = rhs_def.trim().strip_prefix("Count$Valid ")
                && Self::wave19c_rewrite_you_ctrl(rhs_valid.trim(), None).as_deref()
                    == Some(base_valid.as_str())
            {
                let rhs =
                    format!("Amount::ControlledCount {{ who: PlayerRel::You, filter: &{filter} }}");
                return Some(format!(
                    "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &{rhs} }}"
                ));
            }
            return None;
        }
        let lhs = match raw {
            "TriggeredPlayer$CardsInHand" => "Amount::HandSize(PlayerRel::ActivePlayer)",
            "TriggeredPlayer$LifeTotal" => "Amount::LifeTotal(PlayerRel::ActivePlayer)",
            "TriggeredPlayer$CardsInLibrary" => "Amount::LibrarySize(PlayerRel::ActivePlayer)",
            "TriggeredPlayer$CardsInGraveyard" => {
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::ActivePlayer) }"
            }
            "TriggeredPlayer$LifeGainedThisTurn" => {
                "Amount::LifeGainedThisTurn(PlayerRel::ActivePlayer)"
            }
            "TriggeredPlayer$DomainPlayer" => {
                "Amount::BasicLandTypesAmong(&Filter::And(&[Filter::LAND, Filter::ControlledByActivePlayer]))"
            }
            _ => return None,
        };
        if let Some(res) = Self::wave15_amount_cmp(lhs, cmp) {
            return Some(res);
        }
        if raw == "TriggeredPlayer$LifeTotal"
            && let Some((op_code, rhs_var)) = cmp.split_at_checked(2)
            && let Some(op) = Self::wave19c_amount_op(op_code)
            && let Some((rhs_amt, _)) = self.wave19c_starting_life_rhs(rhs_var, true)
        {
            return Some(format!(
                "Condition::AmountCompare {{ left: &{lhs}, op: {op}, right: &{rhs_amt} }}"
            ));
        }
        None
    }

    /// Wave-15c lowering for `SP$/AB$/DB$ Effect` carrying `Triggers$` (or a
    /// permanent planeswalker emblem): lowers planeswalker emblems to
    /// `Effect::CreateEmblem` and turn/combat/watched `Triggers$` effects to
    /// `Effect::RegisterDelayedTrigger` (optionally paired with static or
    /// replacement lines on the same `Effect`).
    pub(super) fn effect_triggers_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if p.has("Boon") || p.has("EffectOwner") || p.has("ImprintCards") {
            return None;
        }
        let is_emblem_name = p
            .peek("Name")
            .is_some_and(|n| n.trim_start().starts_with("Emblem"));
        if p.peek("Duration") == Some("Permanent") && is_emblem_name {
            return self.emblem_effect_api(p, target);
        }
        if p.has("Abilities") || p.peek("Triggers").is_none() || is_emblem_name {
            return None;
        }
        self.delayed_triggers_effect_api(p, target, chain)
    }

    fn emblem_ability_disallowed(emitted: &str, kind: EmblemKind) -> bool {
        if emitted.contains("TargetSpec::ThisObject")
            || emitted.contains("Effect::SacrificeSelf")
            || emitted.contains("Effect::ExileSource")
            || emitted.contains("Effect::ModifySource")
            || emitted.contains("Amount::Source")
            || emitted.contains("Amount::CountersOnSource")
        {
            return true;
        }
        if kind == EmblemKind::Activated {
            if !emitted.contains("ActivationZone::Command")
                || emitted.contains("Cost::TAP")
                || emitted.contains("TapSelf")
                || emitted.contains("UntapSelf")
                || emitted.contains("SacrificeSelf")
                || emitted.contains("ExileSelf")
                || emitted.contains("ReturnSelf")
                || emitted.contains("RemoveCounterSelf")
                || emitted.contains("AddCounterSelf")
                || emitted.contains("CostPart::RemoveCounter")
                || emitted.contains("Filter::This")
            {
                return true;
            }
            return false;
        }
        if emitted.contains("Filter::This") {
            // `Effect::continuous(&Filter::This, ...)` / `SetPTFilter` / `ModifyFilter`
            // on a targeted emblem trigger binds `Filter::This` to the targeted
            // permanent via `this_to_affect`, never to the emblem itself.
            if kind == EmblemKind::Trigger
                && emitted.contains("targets = Some(TargetReq::one(TargetSpec::Object(")
            {
                let scrubbed = emitted
                    .replace("Effect::continuous(&Filter::This, ", "")
                    .replace("Effect::SetPTFilter { filter: &Filter::This, ", "")
                    .replace("Effect::ModifyFilter { filter: &Filter::This, ", "");
                return scrubbed.contains("Filter::This");
            }
            return true;
        }
        false
    }

    fn read_emblem_ability(&mut self, body: &str, kind: EmblemKind) -> Option<()> {
        match kind {
            EmblemKind::Trigger => {
                if body.contains("TriggerZones$") {
                    let normalized = body
                        .split('|')
                        .map(str::trim)
                        .map(|part| {
                            if part == "TriggerZones$ Battlefield" {
                                "TriggerZones$ Command"
                            } else {
                                part
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" | ");
                    self.triggered(&normalized)
                } else {
                    let zoned_body = format!("{body} | TriggerZones$ Command");
                    self.triggered(&zoned_body)
                }
            }
            EmblemKind::Activated => {
                if body.contains("Planeswalker$") || body.contains("LOYALTY") {
                    None
                } else if let Some(zone_part) = body
                    .split('|')
                    .map(str::trim)
                    .find(|part| part.starts_with("ActivationZone$"))
                {
                    if zone_part == "ActivationZone$ Command" {
                        self.activated_or_spell(body)
                    } else {
                        None
                    }
                } else {
                    let zoned_body = format!("{body} | ActivationZone$ Command");
                    self.activated_or_spell(&zoned_body)
                }
            }
            EmblemKind::Static => {
                let normalized = body
                    .split('|')
                    .map(str::trim)
                    .filter(|part| {
                        !part.is_empty()
                            && *part != "EffectZone$ Command"
                            && *part != "EffectZone$ Battlefield"
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                match self.try_extra_static(&normalized) {
                    super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                    super::saga_ward_convoke::ExtraOutcome::Pass => {
                        self.static_ability(&normalized)
                    }
                }
            }
        }
    }

    fn collect_emblem_abilities(
        &mut self,
        svar_list: &str,
        kind: EmblemKind,
        saved_enter_len: usize,
        out: &mut Vec<String>,
    ) -> Option<()> {
        for svar_name in svar_list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let body = self.svars.get(svar_name)?.clone();
            if body.contains("Card.Self")
                || body.contains("EffectSource")
                || body.contains("IsRemembered")
                || body.contains("Enchanted")
                || body.contains("Equipped")
                || body.contains("RememberedPlayer")
                || body.contains("ChosenCard")
                || body.contains("Imprinted")
            {
                return None;
            }
            let prev_len = self.body.abilities.len();
            let saved_on_spell = std::mem::replace(&mut self.on_a_spell, false);
            let saved_mode = (kind == EmblemKind::Trigger)
                .then(|| self.trigger_mode.take())
                .flatten();
            let ok = self.read_emblem_ability(&body, kind);
            self.on_a_spell = saved_on_spell;
            if kind == EmblemKind::Trigger {
                self.trigger_mode = saved_mode;
            }
            ok?;
            if self.body.enter_modifiers.len() != saved_enter_len
                || self.body.abilities.len() <= prev_len
            {
                self.body.abilities.truncate(prev_len);
                self.body.enter_modifiers.truncate(saved_enter_len);
                return None;
            }
            let emitted: Vec<String> = self.body.abilities.drain(prev_len..).collect();
            if emitted
                .iter()
                .any(|a| Self::emblem_ability_disallowed(a, kind))
            {
                return None;
            }
            out.extend(emitted);
        }
        Some(())
    }

    fn emblem_effect_api(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("RememberObjects")
            || p.has("ReplacementEffects")
            || p.has("Planeswalker")
            || p.take("Duration").as_deref() != Some("Permanent")
        {
            return None;
        }
        let name = p.take("Name")?;
        if !name.trim_start().starts_with("Emblem") {
            return None;
        }
        p.take("Image");
        p.take("Ultimate");
        if let Some(svars_list) = p.take("sVars")
            && !svars_list
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .all(|s| self.svars.contains_key(s))
        {
            return None;
        }
        if let Some(s) = p.take("Stackable")
            && !matches!(s.as_str(), "True" | "False")
        {
            return None;
        }
        if let Some(ns) = p.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        let statics = p.take("StaticAbilities");
        let triggers = p.take("Triggers");
        let activated = p.take("Abilities");
        let only_cond_left = p
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"));
        if (statics.is_none() && triggers.is_none() && activated.is_none()) || !only_cond_left {
            return None;
        }
        let mut emblem_abilities = Vec::new();
        let saved_enter_len = self.body.enter_modifiers.len();
        if let Some(statics) = statics {
            self.collect_emblem_abilities(
                &statics,
                EmblemKind::Static,
                saved_enter_len,
                &mut emblem_abilities,
            )?;
        }
        if let Some(triggers) = triggers {
            self.collect_emblem_abilities(
                &triggers,
                EmblemKind::Trigger,
                saved_enter_len,
                &mut emblem_abilities,
            )?;
        }
        if let Some(activated) = activated {
            self.collect_emblem_abilities(
                &activated,
                EmblemKind::Activated,
                saved_enter_len,
                &mut emblem_abilities,
            )?;
        }
        if emblem_abilities.is_empty() {
            return None;
        }
        Some(vec![format!(
            "Effect::CreateEmblem {{ abilities: &[{}] }}",
            emblem_abilities.join(", ")
        )])
    }

    #[allow(clippy::too_many_lines)]
    fn delayed_triggers_effect_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if p.has("Planeswalker") || p.has("Abilities") {
            return None;
        }
        let dur_raw = p.take("Duration");
        let (static_dur, repeat_dur, oneshot_dur) = match dur_raw.as_deref() {
            None | Some("UntilEndOfTurn" | "EndOfTurn" | "End of Turn") => (
                "Duration::UntilEndOfTurn",
                "DelayedDuration::ThisTurn",
                Some("DelayedDuration::OneShotThisTurn"),
            ),
            Some("UntilEndOfCombat") => (
                "Duration::UntilEndOfCombat",
                "DelayedDuration::ThisCombat",
                None,
            ),
            Some("UntilYourNextTurn") => (
                "Duration::UntilYourNextTurn",
                "DelayedDuration::UntilYourNextTurn",
                None,
            ),
            Some("Permanent") => (
                "Duration::Indefinitely",
                "DelayedDuration::Indefinitely",
                Some("DelayedDuration::OneShot"),
            ),
            Some(_) => return None,
        };
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield" | "Graveyard")) {
            return None;
        }
        p.take("IsCurse");
        p.take("Name");
        if let Some(ns) = p.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        let raw_rem = p.peek("RememberObjects").map(str::to_string);
        let subject = self.effect_remember_subject(p, target)?;
        let base_watch = match raw_rem.as_deref() {
            None => "TriggerRemember::None",
            Some(rem) => self.delayed_remember(Some(rem), target, chain, true)?,
        };
        if matches!(ends.as_deref(), Some("Battlefield" | "Graveyard"))
            && base_watch == "TriggerRemember::None"
        {
            return None;
        }
        let statics = p.take("StaticAbilities");
        let replacements = p.take("ReplacementEffects");
        let triggers = p.take("Triggers")?;
        let only_cond_left = p
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"));
        if !only_cond_left {
            return None;
        }

        let mut out = Vec::new();
        if let Some(statics) = statics {
            for svar_name in statics.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let body = self.svars.get(svar_name)?.clone();
                let (mode, mut st) = Params::parse(&body)?;
                st.drop_prose();
                out.extend(self.wave3_static_effect_line(&mode, &mut st, subject, static_dur)?);
            }
        }
        if let Some(replacements) = replacements {
            for svar_name in replacements
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let body = self.svars.get(svar_name)?.clone();
                let (event, mut r) = Params::parse(&body)?;
                r.drop_prose();
                out.extend(
                    self.wave11_effect_replacement_line(
                        &event, &mut r, subject, false, static_dur,
                    )?,
                );
            }
        }

        let mut trigger_count = 0usize;
        for svar_name in triggers.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let body = self.svars.get(svar_name)?.clone();
            let (mode, mut tp) = Params::parse(&body)?;
            tp.drop_prose();
            if mode == "Phase" {
                let expr = self.effect_phase_delayed_trigger(
                    &mut tp,
                    dur_raw.as_deref(),
                    ends.as_deref(),
                    base_watch,
                )?;
                out.push(expr);
                trigger_count += 1;
                continue;
            }
            if mode == "Attacks"
                && matches!(
                    tp.peek("Attacked"),
                    Some("You,Planeswalker.YouCtrl" | "Planeswalker.YouCtrl,You")
                )
            {
                let expr = self.effect_tamiyo_attack_trigger(
                    &mut tp,
                    dur_raw.as_deref(),
                    ends.as_deref(),
                    base_watch,
                )?;
                out.push(expr);
                trigger_count += 1;
                continue;
            }
            if mode == "ChangesController" {
                let expr = self.effect_lose_control_trigger(
                    &mut tp,
                    dur_raw.as_deref(),
                    ends.as_deref(),
                    base_watch,
                )?;
                out.push(expr);
                trigger_count += 1;
                continue;
            }
            if !matches!(
                mode.as_str(),
                "ChangesZone"
                    | "ChangesZoneAll"
                    | "SpellCast"
                    | "SpellCastOrCopy"
                    | "Attacks"
                    | "AttackersDeclared"
                    | "Attacked"
                    | "AttackersDeclaredOneTarget"
                    | "Blocks"
                    | "BecomesTarget"
                    | "DamageDone"
                    | "DamageDoneOnce"
                    | "DamageDealtOnce"
                    | "Taps"
                    | "Untaps"
                    | "TapsForMana"
                    | "LifeGained"
                    | "LifeLost"
                    | "Drawn"
                    | "LandPlayed"
                    | "Discarded"
                    | "Cycled"
                    | "Sacrificed"
                    | "CounterAddedOnce"
                    | "AttackerBlocked"
                    | "AttackerUnblocked"
                    | "PhaseIn"
                    | "PhaseOut"
                    | "FlippedCoin"
                    | "RolledDie"
                    | "SearchedLibrary"
                    | "Scry"
                    | "Exerted"
                    | "Enlisted"
                    | "Exploited"
                    | "BecomesPlotted"
                    | "Plotted"
                    | "GiveGift"
                    | "GaveGift"
                    | "AbilityCast"
                    | "CommitCrime"
                    | "ManaExpend"
                    | "RingTemptsYou"
                    | "Clashed"
                    | "Firebend"
                    | "Firebends"
            ) {
                return None;
            }
            if !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command")) {
                return None;
            }
            if let Some(sec) = tp.take("Secondary")
                && sec != "True"
            {
                return None;
            }
            if ends.as_deref() == Some("Graveyard")
                && !(mode == "ChangesZone"
                    && tp.peek("Origin") == Some("Battlefield")
                    && tp.peek("Destination") == Some("Graveyard"))
            {
                return None;
            }
            let one_off = match (
                tp.take("OneOff").as_deref(),
                tp.take("TriggerOnce").as_deref(),
            ) {
                (None | Some("False"), None | Some("False")) => false,
                (Some("True"), None | Some("False")) | (None | Some("False"), Some("True")) => true,
                _ => return None,
            };
            let delayed_dur = if one_off { oneshot_dur? } else { repeat_dur };
            let optional = match tp.take("OptionalDecider").as_deref() {
                None => false,
                Some("You") => true,
                Some(_) => return None,
            };

            // Rewrite `IsRemembered` / `EffectSource` on the watched filter key
            // to `Self` so `Filter::This` binds to the watched incarnation.
            let mut trigger_watch = "TriggerRemember::None";
            let mut uses_watch = false;
            for (k, v) in &mut tp.entries {
                if !matches!(k.as_str(), "ValidCard" | "ValidSource" | "ValidTarget") {
                    continue;
                }
                if v.contains("IsRemembered") {
                    if base_watch == "TriggerRemember::None" {
                        return None;
                    }
                    *v = Self::rewrite_remembered_atom(v, "IsRemembered")?;
                    trigger_watch = base_watch;
                    uses_watch = true;
                } else if v.contains("EffectSource") && !self.on_a_spell {
                    *v = Self::rewrite_remembered_atom(v, "EffectSource")?;
                    trigger_watch = "TriggerRemember::Source";
                    uses_watch = true;
                }
            }
            if dur_raw.as_deref() == Some("Permanent")
                && !one_off
                && trigger_watch == "TriggerRemember::None"
            {
                return None;
            }
            if tp.entries.iter().any(|(_, v)| {
                v.contains("IsRemembered")
                    || v.contains("EffectSource")
                    || v.contains("RememberedPlayer")
                    || v.contains("ChosenCard")
            }) {
                return None;
            }

            let trigger_expr = self.trigger_expr_in_zone(&mut tp, &mode, true)?;
            let cond = self.delayed_optional_condition(&mut tp)?;
            let execute = tp.take("Execute")?;
            let exec_raw = self.svars.get(execute.trim())?.clone();
            if !tp.exhausted() {
                return None;
            }

            let (effects, targets, remember) = self.parse_effect_trigger_execute(
                &exec_raw,
                &mode,
                optional,
                uses_watch,
                trigger_watch,
                base_watch,
            )?;
            let targets_expr =
                targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
            out.push(format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &{trigger_expr}, watch: {trigger_watch}, duration: {delayed_dur} }}, \
                 condition: {cond}, targets: {targets_expr}, remember: {remember}, \
                 effects: &[{effects}] }}"
            ));
            trigger_count += 1;
        }
        (trigger_count > 0).then_some(out)
    }

    fn effect_phase_delayed_trigger(
        &mut self,
        tp: &mut Params,
        dur_raw: Option<&str>,
        ends: Option<&str>,
        base_watch: &'static str,
    ) -> Option<String> {
        if ends.is_some() || tp.has("Static") || tp.has("NextTurn") {
            return None;
        }
        if !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command")) {
            return None;
        }
        if let Some(sec) = tp.take("Secondary")
            && sec != "True"
        {
            return None;
        }
        let one_off = match (
            tp.take("OneOff").as_deref(),
            tp.take("TriggerOnce").as_deref(),
        ) {
            (None | Some("False"), None | Some("False")) => false,
            (Some("True"), None | Some("False")) | (None | Some("False"), Some("True")) => true,
            _ => return None,
        };
        let optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let step = match tp.take("Phase")?.as_str() {
            "Upkeep" => "DelayedStep::Upkeep",
            "Draw" => "DelayedStep::Draw",
            "Main1,Main2" | "Main2,Main1" => "DelayedStep::MainPhase",
            "BeginCombat" => "DelayedStep::CombatBegin",
            "EndCombat" => "DelayedStep::CombatEnd",
            "End of Turn" | "End Of Turn" => "DelayedStep::EndStep",
            "Cleanup" => "DelayedStep::Cleanup",
            _ => return None,
        };
        let whose = match tp.take("ValidPlayer").as_deref() {
            None | Some("Player") => "None",
            Some("You") => "Some(PlayerRel::You)",
            Some("Opponent" | "Player.Opponent") => "Some(PlayerRel::EachOpponent)",
            Some(_) => return None,
        };
        let (this_turn_only, recurring) = match (dur_raw, one_off) {
            (Some("Permanent"), true) => (false, false),
            (None | Some("UntilEndOfTurn"), true) => (true, false),
            (None | Some("UntilEndOfTurn"), false) => match step {
                "DelayedStep::CombatBegin" if base_watch == "TriggerRemember::None" => (true, true),
                "DelayedStep::EndStep" | "DelayedStep::Cleanup" => (true, false),
                _ => return None,
            },
            _ => return None,
        };
        if tp.entries.iter().any(|(_, v)| {
            v.contains("IsRemembered")
                || v.contains("EffectSource")
                || v.contains("RememberedPlayer")
                || v.contains("ChosenCard")
        }) {
            return None;
        }
        let saved_mode = self.trigger_mode.replace("Phase".to_string());
        let cond = self.delayed_optional_condition(tp);
        self.trigger_mode = saved_mode;
        let cond = cond?;
        let execute = tp.take("Execute")?;
        let exec_raw = self.svars.get(execute.trim())?.clone();
        if !tp.exhausted()
            || exec_raw.contains("RememberedPlayer")
            || exec_raw.contains("Triggered")
            || exec_raw.contains("TriggerCount$")
        {
            return None;
        }
        let (body_to_parse, remember) = if exec_raw.contains("Defined$ Remembered") {
            if base_watch == "TriggerRemember::None" {
                return None;
            }
            let normalized = exec_raw
                .replace(
                    "Defined$ RememberedLKI",
                    "Defined$ DelayTriggerRememberedLKI",
                )
                .replace("Defined$ Remembered", "Defined$ DelayTriggerRemembered");
            (normalized, base_watch)
        } else {
            if base_watch != "TriggerRemember::None" {
                return None;
            }
            (exec_raw, "TriggerRemember::None")
        };
        let (effects, targets) = self.parse_delayed_body_str(&body_to_parse, None, optional)?;
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ step: {step}, \
             whose: {whose}, this_turn_only: {this_turn_only}, recurring: {recurring} }}, \
             condition: {cond}, targets: {targets_expr}, remember: {remember}, \
             effects: &[{effects}] }}"
        ))
    }

    fn effect_tamiyo_attack_trigger(
        &mut self,
        tp: &mut Params,
        dur_raw: Option<&str>,
        ends: Option<&str>,
        base_watch: &'static str,
    ) -> Option<String> {
        if dur_raw != Some("UntilYourNextTurn")
            || ends.is_some()
            || base_watch != "TriggerRemember::None"
            || tp.has("Static")
        {
            return None;
        }
        if !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command")) {
            return None;
        }
        if let Some(sec) = tp.take("Secondary")
            && sec != "True"
        {
            return None;
        }
        if !matches!(
            tp.take("ValidCard").as_deref(),
            Some("Creature" | "Creature.OppCtrl")
        ) || !matches!(
            tp.take("Attacked").as_deref(),
            Some("You,Planeswalker.YouCtrl" | "Planeswalker.YouCtrl,You")
        ) {
            return None;
        }
        let optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let execute = tp.take("Execute")?;
        let exec_raw = self.svars.get(execute.trim())?.clone();
        if !tp.exhausted() {
            return None;
        }
        let (effects, targets) =
            self.parse_delayed_body_str(&exec_raw, Some("Attacks".to_string()), optional)?;
        if targets.is_some() {
            return None;
        }
        Some(format!(
            "Effect::WheneverCreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {{ \
             effects: &[{effects}] }}"
        ))
    }

    fn effect_lose_control_trigger(
        &mut self,
        tp: &mut Params,
        dur_raw: Option<&str>,
        ends: Option<&str>,
        base_watch: &'static str,
    ) -> Option<String> {
        if dur_raw != Some("Permanent")
            || ends.is_some()
            || base_watch == "TriggerRemember::None"
            || tp.has("Static")
        {
            return None;
        }
        if !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command")) {
            return None;
        }
        if let Some(sec) = tp.take("Secondary")
            && sec != "True"
        {
            return None;
        }
        if tp.take("ValidCard").as_deref() != Some("Card.IsRemembered")
            || tp.take("ValidOriginalController").as_deref() != Some("You")
        {
            return None;
        }
        let optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let cond = self.delayed_optional_condition(tp)?;
        let execute = tp.take("Execute")?;
        let exec_raw = self.svars.get(execute.trim())?.clone();
        if !tp.exhausted()
            || exec_raw.contains("RememberedPlayer")
            || exec_raw.contains("Triggered")
            || exec_raw.contains("TriggerCount$")
        {
            return None;
        }
        let cleaned = self.strip_command_exile_subability(&exec_raw);
        let normalized = cleaned
            .replace(
                "Defined$ RememberedLKI",
                "Defined$ DelayTriggerRememberedLKI",
            )
            .replace("Defined$ Remembered", "Defined$ DelayTriggerRemembered");
        let (effects, targets) = self.parse_delayed_body_str(&normalized, None, optional)?;
        let targets_expr = targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnObservation {{ \
             watch: {base_watch}, observation: DelayedObservation::YouLoseControl, \
             this_turn_only: false }}, condition: {cond}, targets: {targets_expr}, \
             remember: TriggerRemember::None, effects: &[{effects}] }}"
        ))
    }

    /// Strips a trailing `SubAbility$` pointing solely to an `Effect` self-exile
    /// (`DB$ ChangeZone | Origin$ Command | Destination$ Exile`) from a one-shot
    /// observation trigger body.
    fn strip_command_exile_subability(&self, exec_raw: &str) -> String {
        let Some((_, mut p)) = Params::parse(exec_raw) else {
            return exec_raw.to_string();
        };
        let Some(sub_name) = p.take("SubAbility") else {
            return exec_raw.to_string();
        };
        let Some(sub_body) = self.svars.get(sub_name.trim()) else {
            return exec_raw.to_string();
        };
        let Some((sub_api, mut sp)) = Params::parse(sub_body) else {
            return exec_raw.to_string();
        };
        sp.drop_prose();
        let is_self_exile = sub_api == "ChangeZone"
            && sp.take("Origin").as_deref() == Some("Command")
            && sp.take("Destination").as_deref() == Some("Exile")
            && matches!(sp.take("Defined").as_deref(), None | Some("Self"))
            && sp.exhausted();
        if !is_self_exile {
            return exec_raw.to_string();
        }
        exec_raw
            .split('|')
            .map(str::trim)
            .filter(|part| {
                part.strip_prefix("SubAbility$")
                    .is_none_or(|v| v.trim() != sub_name.trim())
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn rewrite_remembered_atom(valid: &str, needle: &str) -> Option<String> {
        let mut alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let (base, atoms_raw) = alt.split_once('.')?;
            let mut saw = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == needle {
                    saw = true;
                    kept.push("Self");
                } else if atom.contains(needle) {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if !saw {
                return None;
            }
            alts.push(format!("{base}.{}", kept.join("+")));
        }
        (!alts.is_empty()).then(|| alts.join(","))
    }

    fn parse_effect_trigger_execute(
        &mut self,
        exec_raw: &str,
        mode: &str,
        optional: bool,
        uses_watch: bool,
        trigger_watch: &'static str,
        base_watch: &'static str,
    ) -> Option<(String, Option<String>, &'static str)> {
        if !self.on_a_spell
            && let Some((api, mut ep)) = Params::parse(exec_raw)
            && matches!(api.as_str(), "Sacrifice" | "SacrificeAll")
            && ep.take("Defined").as_deref() == Some("EffectSource")
        {
            ep.drop_prose();
            if ep.exhausted() {
                let eff = if optional {
                    "Effect::MayDo { effects: &[Effect::SacrificeSelf] }".to_string()
                } else {
                    "Effect::SacrificeSelf".to_string()
                };
                return Some((eff, None, "TriggerRemember::None"));
            }
        }
        let mentions_triggered =
            exec_raw.contains("Triggered") || exec_raw.contains("TriggerCount$");
        let mentions_remembered = exec_raw.contains("Defined$ Remembered");
        let (body_to_parse, remember) = if mentions_remembered {
            if mentions_triggered || exec_raw.contains("RememberedPlayer") {
                return None;
            }
            if uses_watch {
                let normalized = exec_raw
                    .replace(
                        "Defined$ RememberedLKI",
                        "Defined$ DelayTriggerRememberedLKI",
                    )
                    .replace("Defined$ Remembered", "Defined$ DelayTriggerRemembered");
                let rem = if mode == "ChangesZone" {
                    "TriggerRemember::None"
                } else {
                    trigger_watch
                };
                (normalized, rem)
            } else if !exec_raw.contains("ValidTgts$") && base_watch != "TriggerRemember::None" {
                let normalized = exec_raw
                    .replace(
                        "Defined$ RememberedLKI",
                        "Defined$ DelayTriggerRememberedLKI",
                    )
                    .replace("Defined$ Remembered", "Defined$ DelayTriggerRemembered");
                (normalized, base_watch)
            } else {
                return None;
            }
        } else {
            (exec_raw.to_string(), "TriggerRemember::None")
        };
        let (effects, targets) =
            self.parse_delayed_body_str(&body_to_parse, Some(mode.to_string()), optional)?;
        Some((effects, targets, remember))
    }

    /// Scans a trigger's `Execute$` `SVar` chain (following `SubAbility$` and
    /// `Choices$`, bounded depth) for references to `TriggeredAttackingPlayer`
    /// / `TriggeredPlayer` vs `TriggeredAttackedTarget` / `TriggeredAttackers`.
    fn wave25c_exec_attack_refs(&self, start: &str) -> (bool, bool) {
        let mut uses_attacking_player = false;
        let mut uses_attacked_or_attackers = false;
        let mut queue = vec![start.trim().to_string()];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(name) = queue.pop() {
            if visited.len() >= 16 || !visited.insert(name.clone()) {
                continue;
            }
            let Some(body) = self.svars.get(&name) else {
                continue;
            };
            if body.contains("TriggeredAttackingPlayer") || body.contains("TriggeredPlayer") {
                uses_attacking_player = true;
            }
            if body.contains("TriggeredAttackedTarget") || body.contains("TriggeredAttackers") {
                uses_attacked_or_attackers = true;
            }
            if let Some((_, mut ep)) = Params::parse(body) {
                if let Some(sub) = ep.take("SubAbility") {
                    queue.push(sub.trim().to_string());
                }
                if let Some(choices) = ep.take("Choices") {
                    for c in choices.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        queue.push(c.to_string());
                    }
                }
            }
        }
        (uses_attacking_player, uses_attacked_or_attackers)
    }

    fn wave25c_attacked_target_kind(raw: &str, all_you_ctrl: bool) -> Option<&'static str> {
        let mut parts: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        parts.sort_unstable();
        parts.dedup();
        Some(match parts.as_slice() {
            ["Any"] | ["Battle", "Planeswalker", "Player"] => "AttackedTargetKind::Any",
            ["Player"] => "AttackedTargetKind::Player",
            ["Opponent" | "Player.Opponent"] => "AttackedTargetKind::Opponent",
            ["You" | "Player.You"] => "AttackedTargetKind::You",
            ["Planeswalker"] => "AttackedTargetKind::Planeswalker",
            ["Battle"] => "AttackedTargetKind::Battle",
            ["Planeswalker.YouCtrl", "Player.You" | "You"] => {
                "AttackedTargetKind::YouOrYourPlaneswalker"
            }
            ["Opponent", "Planeswalker.OppCtrl"] | ["Planeswalker.OppCtrl", "Player.Opponent"] => {
                "AttackedTargetKind::OpponentOrTheirPlaneswalker"
            }
            // CR 508.1b: a player may only declare attackers against opponents,
            // planeswalkers opponents control, or battles — so when every
            // matching attacker is `YouCtrl`, `Player,Planeswalker` is exact
            // for `OpponentOrTheirPlaneswalker` (excluding battles).
            ["Planeswalker", "Player"] if all_you_ctrl => {
                "AttackedTargetKind::OpponentOrTheirPlaneswalker"
            }
            ["EnchantedPlayer" | "Player.EnchantedBy"] => "AttackedTargetKind::EnchantedPlayer",
            _ => return None,
        })
    }

    fn wave25c_pump_kw_bits(kw: &str) -> Option<String> {
        let mut bits = Vec::new();
        for part in kw.split('&').map(str::trim) {
            let bit = match part {
                "HIDDEN CARDNAME can't attack or block." => {
                    "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)"
                }
                "HIDDEN CARDNAME can't block." | "CARDNAME can't block." => {
                    "KeywordSet::CANT_BLOCK"
                }
                "HIDDEN CARDNAME can't attack." => "KeywordSet::CANT_ATTACK",
                "HIDDEN CARDNAME can't be blocked." => "KeywordSet::UNBLOCKABLE",
                other => keyword_const(other)?,
            };
            bits.push(bit);
        }
        if bits.is_empty() {
            return None;
        }
        let mut out = bits[0].to_string();
        for b in &bits[1..] {
            out = format!("{out}.union({b})");
        }
        Some(out)
    }

    /// Lowers `Execute$` chains specific to `Trigger::AttackersDeclaredAtTarget`
    /// where `EventPlayer` is the attacked defender (`TriggeredAttackedTarget`),
    /// `EventObject` is the first matching attacker, and `SubjectContext::found`
    /// holds all matching attackers toward that defender (`TriggeredAttackers`).
    #[allow(clippy::too_many_lines)]
    fn wave25c_attackers_at_target_chain(
        &mut self,
        body: &str,
        exact_count_is_one: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        let (api, mut p) = Params::parse(body)?;
        p.drop_prose();

        // Pattern A: `DB$ Pump | Defined$ TriggeredAttackers` (*Yuriko, Blade of the Mighty*,
        // *Frontier Warmonger*, *Frenzied Gorespawn*, *Garruk, Curse Breaker*).
        if api == "Pump"
            && p.peek("Defined") == Some("TriggeredAttackers")
            && !p.has("ValidTgts")
            && !p.has("TargetsWithDefinedController")
        {
            p.take("Defined");
            p.take("IsCurse");
            if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
                return None;
            }
            let num_att = p.take("NumAtt");
            let num_def = p.take("NumDef");
            let kw_raw = p.take("KW");
            if num_att.is_none() && num_def.is_none() && kw_raw.is_none() {
                return None;
            }
            let sub = p.take("SubAbility");
            if !p.exhausted() {
                return None;
            }
            let power = match num_att.as_deref() {
                None => "Amount::Fixed(0)".to_string(),
                Some(raw) => pump_amount(raw, self.svars, false)?,
            };
            let toughness = match num_def.as_deref() {
                None => "Amount::Fixed(0)".to_string(),
                Some(raw) => pump_amount(raw, self.svars, false)?,
            };
            let keywords = match kw_raw.as_deref() {
                None => "KeywordSet::EMPTY".to_string(),
                Some(kw) => Self::wave25c_pump_kw_bits(kw)?,
            };
            let single = format!(
                "Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
                 keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
            );
            if exact_count_is_one {
                chain.effects.push(single);
            } else {
                chain.effects.push(format!(
                    "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, \
                     filter: &Filter::Any, only_found: true, then: &[{single}] }}"
                ));
            }
            if let Some(sub_name) = sub {
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                self.chain(&sub_body, chain)?;
            }
            return Some(());
        }

        // Pattern B: `DB$ PutCounter | Defined$ TriggeredAttackers` when
        // `exact_count: Some(1)` (sole attacker is `EventObject`).
        if api == "PutCounter"
            && exact_count_is_one
            && p.peek("Defined") == Some("TriggeredAttackers")
            && !p.has("ValidTgts")
        {
            p.take("Defined");
            let kind = counter_kind(p.take("CounterType")?.trim())?;
            let amount = match p.take("CounterNum").as_deref() {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self.amount_num(raw)?,
            };
            let sub = p.take("SubAbility");
            if !p.exhausted() {
                return None;
            }
            chain.effects.push(format!(
                "Effect::AddCounterFilter {{ filter: &Filter::Reference, kind: {kind}, \
                 amount: {amount} }}"
            ));
            if let Some(sub_name) = sub {
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                self.chain(&sub_body, chain)?;
            }
            return Some(());
        }

        // Pattern C: `DB$ LoseLife | Defined$ TriggeredAttackedTarget` (*Archnemesis*).
        if api == "LoseLife" && p.peek("Defined") == Some("TriggeredAttackedTarget") {
            p.take("Defined");
            let amount = self.amount_num(&p.take("LifeAmount")?)?;
            let sub = p.take("SubAbility");
            if !p.exhausted() {
                return None;
            }
            chain.effects.push(format!(
                "Effect::LoseLife {{ target: PlayerRel::EventPlayer, amount: {amount} }}"
            ));
            if let Some(sub_name) = sub {
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                self.chain(&sub_body, chain)?;
            }
            return Some(());
        }

        // Pattern D: `TargetsWithDefinedController$ TriggeredAttackedTarget`
        // -> `TargetSpec::ObjectOfEventPlayer(&<filter>)` (*Whirlwind, Killer Cyclone*,
        // *Seifer, Balamb Rival*, *Firkraag, Cunning Instigator*, *Gornog, the Red Reaper*).
        if p.peek("TargetsWithDefinedController") == Some("TriggeredAttackedTarget")
            && p.has("ValidTgts")
            && !p.has("Defined")
            && !p.has("TargetMin")
            && !p.has("TargetMax")
            && !p.has("Optional")
        {
            p.take("TargetsWithDefinedController");
            let valid_tgts = p.take("ValidTgts")?;
            if valid_tgts.contains("Ctrl")
                || valid_tgts.contains("Own")
                || valid_tgts.contains("ControlledBy")
                || valid_tgts.contains("OwnedBy")
                || valid_tgts.contains("Self")
            {
                return None;
            }
            let tgt_expr = self.filter_expr(&valid_tgts)?;
            let tgt_ref = match tgt_expr.as_str() {
                "Filter::Any" => "&Filter::Any".to_string(),
                "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
                "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
                "Filter::LAND" => "&Filter::LAND".to_string(),
                "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
                "Filter::PERMANENT" => "&Filter::PERMANENT".to_string(),
                _ => format!("&{}", self.body.filter_static("TARGET", &tgt_expr)),
            };
            let target_spec = format!("TargetSpec::ObjectOfEventPlayer({tgt_ref})");
            let sub = p.take("SubAbility");
            let effect = match api.as_str() {
                "Pump" => {
                    p.take("IsCurse");
                    if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
                        return None;
                    }
                    let num_att = p.take("NumAtt");
                    let num_def = p.take("NumDef");
                    let kw_raw = p.take("KW");
                    if num_att.is_none() && num_def.is_none() && kw_raw.is_none() {
                        return None;
                    }
                    if !p.exhausted() {
                        return None;
                    }
                    let power = match num_att.as_deref() {
                        None => "Amount::Fixed(0)".to_string(),
                        Some(raw) => pump_amount(raw, self.svars, false)?,
                    };
                    let toughness = match num_def.as_deref() {
                        None => "Amount::Fixed(0)".to_string(),
                        Some(raw) => pump_amount(raw, self.svars, false)?,
                    };
                    let keywords = match kw_raw.as_deref() {
                        None => "KeywordSet::EMPTY".to_string(),
                        Some(kw) => Self::wave25c_pump_kw_bits(kw)?,
                    };
                    format!(
                        "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                         keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
                    )
                }
                "Goad" => {
                    p.take("IsCurse");
                    if !p.exhausted() {
                        return None;
                    }
                    format!("Effect::Goad {{ target: {target_spec} }}")
                }
                "Tap" => {
                    p.take("IsCurse");
                    if !p.exhausted() {
                        return None;
                    }
                    "Effect::TapTarget".to_string()
                }
                "Animate" => {
                    p.take("IsCurse");
                    if p.take("RemoveCreatureTypes").as_deref() != Some("True") {
                        return None;
                    }
                    let duration = match p.take("Duration").as_deref() {
                        None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
                        Some("Permanent") => "Duration::Indefinitely",
                        Some(_) => return None,
                    };
                    let raw_type = p.take("Types")?;
                    if raw_type.contains([',', '&', ' ', '.']) || !p.exhausted() {
                        return None;
                    }
                    let path = self
                        .cats
                        .const_path_of(SubtypeKind::Creature, raw_type.trim())?;
                    format!(
                        "Effect::ModifyTargets {{ modifier: Modifier::ReplaceCreatureTypes({path}), \
                         duration: {duration} }}"
                    )
                }
                _ => return None,
            };
            chain.target = Some(target_spec);
            chain.effects.push(effect);
            if let Some(sub_name) = sub {
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                self.chain(&sub_body, chain)?;
            }
            return Some(());
        }

        None
    }

    /// Wave 25c LKI graveyard-exile replacement lowering (`CR 603.10a`, `CR 614.6`):
    /// lowers `R:Event$ Moved | Destination$ Graveyard` into
    /// `ReplacementRule::ExileInsteadOfGraveyard { filter, origin, owner, controller_lki,
    /// damaged_by, link_to_source, counter }` (*Etching of Kumano*, *Incendiary Oracle*,
    /// *Valgavoth, Terror Eater*, *Gisa, Glorious Resurrector*, *Rest in Peace*,
    /// *Anafenza, the Foremost*, *Dryad Militant*, *Sanctifier en-Vec*, etc.).
    #[allow(clippy::too_many_lines)]
    pub(super) fn lki_exile_instead_of_graveyard(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Graveyard")
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let r_origin = p.take("Origin");
        let (valid, is_lki) = match (p.take("ValidCard"), p.take("ValidLKI")) {
            (Some(v), None) => (v, false),
            (None, Some(v)) => (v, true),
            _ => return None,
        };
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut body) = self.svars.get(&with).and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if api != "ChangeZone"
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || body.take("Destination").as_deref() != Some("Exile")
            || !matches!(body.take("Hidden").as_deref(), None | Some("True"))
        {
            return None;
        }
        let body_origin = body.take("Origin");
        let counter_raw = body.take("WithCountersType");
        if let Some(cnt_amt) = body.take("WithCountersAmount")
            && (counter_raw.is_none() || cnt_amt.trim() != "1")
        {
            return None;
        }
        if !body.exhausted() {
            return None;
        }
        let origin = match (r_origin.as_deref(), body_origin.as_deref()) {
            (Some("Battlefield"), None | Some("All" | "Battlefield")) => {
                Some("ZoneRef::Battlefield")
            }
            (None | Some("All"), None | Some("All")) => None,
            // Forge writes `ValidLKI$ Creature.DamagedBy*` with `Origin$ Battlefield`
            // on the `ReplaceWith$` `ChangeZone` rather than on the `R:` header
            // (*Incendiary Oracle*, *Etching of Kumano*, *Frostwielder*, *Kumano*).
            (None | Some("All"), Some("Battlefield")) if is_lki && valid.contains("DamagedBy") => {
                Some("ZoneRef::Battlefield")
            }
            _ => return None,
        };
        let counter_expr = match counter_raw {
            None => "None".to_string(),
            Some(code) => format!("Some({})", counter_kind(code.trim())?),
        };

        // Parse each comma-separated alternative, extracting owner, controller_lki,
        // and damaged_by uniformly across all alternatives.
        let mut common_owner: Option<Option<&'static str>> = None;
        let mut common_ctrl: Option<Option<&'static str>> = None;
        let mut common_dmg: Option<&'static str> = None;
        let mut cleaned_alts = Vec::new();
        for raw_alt in valid.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let base = base.trim();
            let mut alt_owner: Option<&'static str> = None;
            let mut alt_ctrl: Option<&'static str> = None;
            let mut alt_dmg: &'static str = "DamagedByScope::Any";
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                match atom {
                    "DamagedBy" => {
                        if origin != Some("ZoneRef::Battlefield")
                            || !is_lki
                            || alt_dmg != "DamagedByScope::Any"
                        {
                            return None;
                        }
                        alt_dmg = "DamagedByScope::SourceThisTurn";
                    }
                    "DamagedByCard.YouCtrl;Emblem.YouCtrl" | "DamagedByCard.YouCtrl" => {
                        if origin != Some("ZoneRef::Battlefield")
                            || !is_lki
                            || alt_dmg != "DamagedByScope::Any"
                        {
                            return None;
                        }
                        alt_dmg = "DamagedByScope::SourceControllerThisTurn";
                    }
                    "OppOwn" => {
                        if alt_owner.is_some() {
                            return None;
                        }
                        alt_owner = Some("PlayerRel::EachOpponent");
                    }
                    "YouOwn" => {
                        if alt_owner.is_some() {
                            return None;
                        }
                        alt_owner = Some("PlayerRel::You");
                    }
                    "OppCtrl" => {
                        if alt_ctrl.is_some() {
                            return None;
                        }
                        alt_ctrl = Some("PlayerRel::EachOpponent");
                    }
                    "YouCtrl" => {
                        if alt_ctrl.is_some() {
                            return None;
                        }
                        alt_ctrl = Some("PlayerRel::You");
                    }
                    "YouDontCtrl" => {
                        if alt_ctrl.is_some() {
                            return None;
                        }
                        alt_ctrl = Some("PlayerRel::EachOtherPlayer");
                    }
                    other => {
                        if other.contains("DamagedBy")
                            || other.contains("Ctrl")
                            || other.contains("Own")
                            || other.contains("ControlledBy")
                            || other.contains("OwnedBy")
                            || other.contains("CastSa")
                            || other.contains("MayPlaySource")
                            || other.contains("IsRemembered")
                            || other.contains(';')
                        {
                            return None;
                        }
                        kept.push(other);
                    }
                }
            }
            match common_owner {
                None => common_owner = Some(alt_owner),
                Some(prev) if prev != alt_owner => return None,
                Some(_) => {}
            }
            match common_ctrl {
                None => common_ctrl = Some(alt_ctrl),
                Some(prev) if prev != alt_ctrl => return None,
                Some(_) => {}
            }
            match common_dmg {
                None => common_dmg = Some(alt_dmg),
                Some(prev) if prev != alt_dmg => return None,
                Some(_) => {}
            }
            if kept.is_empty() {
                cleaned_alts.push(base.to_string());
            } else {
                cleaned_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        let owner = common_owner?;
        let controller_lki = common_ctrl?;
        let damaged_by = common_dmg?;
        let cleaned = cleaned_alts.join(",");

        let filter_expr = if cleaned == "Card.Self" {
            if owner.is_some() || controller_lki.is_some() || damaged_by != "DamagedByScope::Any" {
                return None;
            }
            "Filter::This".to_string()
        } else if cleaned == "Card"
            || (cleaned == "Permanent" && origin == Some("ZoneRef::Battlefield"))
        {
            "Filter::Any".to_string()
        } else {
            self.filter_expr_in_zone(&cleaned, origin.is_none())?
        };
        let filter_ref = match filter_expr.as_str() {
            "Filter::Any" => "&Filter::Any".to_string(),
            "Filter::This" => "&Filter::This".to_string(),
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::NONTOKEN_CREATURE" => "&Filter::NONTOKEN_CREATURE".to_string(),
            "Filter::ANOTHER_CREATURE" => "&Filter::ANOTHER_CREATURE".to_string(),
            "Filter::INSTANT_OR_SORCERY" => "&Filter::INSTANT_OR_SORCERY".to_string(),
            "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
            "Filter::LAND" => "&Filter::LAND".to_string(),
            "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
            "Filter::PLANESWALKER" => "&Filter::PLANESWALKER".to_string(),
            "Filter::PERMANENT" => "&Filter::PERMANENT".to_string(),
            _ => format!("&{}", self.body.filter_static("REPLACE", &filter_expr)),
        };

        let link_to_source = cleaned != "Card.Self"
            && (self.svars.values().any(|v| v.contains("ExiledWith"))
                || self
                    .body
                    .abilities
                    .iter()
                    .any(|a| a.contains("LinkedExile"))
                || self.oracle.to_ascii_lowercase().contains("exiled with"));
        let origin_expr = origin.map_or_else(|| "None".to_string(), |z| format!("Some({z})"));
        let owner_expr = owner.map_or_else(|| "None".to_string(), |o| format!("Some({o})"));
        let ctrl_expr = controller_lki.map_or_else(|| "None".to_string(), |c| format!("Some({c})"));
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard {{ \
             filter: {filter_ref}, origin: {origin_expr}, owner: {owner_expr}, \
             controller_lki: {ctrl_expr}, damaged_by: {damaged_by}, \
             link_to_source: {link_to_source}, counter: {counter_expr} }})"
        ));
        Some(())
    }
}
