//! Reading triggered abilities, delayed ones, and what happens as a
//! permanent enters.

use super::{
    BlockHalf, BlockRole, Chain, Params, Tx, activated::OptionalTriggerPrice, card_type_const,
    color_word, counter_kind,
};

enum LegacyPhase {
    Handled(String),
    Fallthrough,
    Refused,
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
            None if p.has("ValidActivatingPlayer") || p.has("TargetsValid") => "Card".to_string(),
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
            let Some(target) = target else {
                return self.deny("a `DamageDoneOnce` trigger with no `ValidTarget$`".to_string());
            };
            if let Some(source) = source {
                if combat && target == "Player" {
                    let filter = filter_of(self, &source)?;
                    return Some(format!(
                        "Trigger::OneOrMoreDealCombatDamageToPlayer({filter})"
                    ));
                }
                return self.deny("a `DamageDoneOnce` trigger naming its source".to_string());
            }
            if !combat
                && matches!(
                    target.as_str(),
                    "You" | "Player" | "Opponent" | "Player.Opponent"
                )
            {
                let who = match target.as_str() {
                    "You" => "PlayerRel::You",
                    "Player" => "PlayerRel::EachPlayer",
                    "Opponent" | "Player.Opponent" => "PlayerRel::EachOpponent",
                    _ => unreachable!(),
                };
                return Some(format!("Trigger::PlayerDealtDamage({who})"));
            }
            if target
                .split(['.', ','])
                .any(|w| matches!(w, "You" | "Player" | "Opponent"))
            {
                return self.deny(format!("damage dealt to `{target}` at once"));
            }
            let filter = filter_of(self, &target)?;
            let kind = if combat {
                "DealtCombatDamage"
            } else {
                "DealtDamage"
            };
            return Some(format!("Trigger::{kind}({filter})"));
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
            other => self.deny(format!("trigger mode `{other}`")),
        }
    }

    fn phase_step_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(phase) = p.take("Phase") else {
            return self.deny("a `Phase` trigger with no `Phase$`".to_string());
        };
        let step = match phase.as_str() {
            "Upkeep" => "StepKind::Upkeep",
            "Draw" => "StepKind::Draw",
            "Main1" => "StepKind::FirstMain",
            "BeginCombat" => "StepKind::CombatBegin",
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
            Some("You.descended") => Some("PlayerRel::You"),
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
        if p.has("ValidAttackersAmount") {
            return None;
        }
        if let Some(att) = p.take("ValidAttackers")
            && att != "Creature"
        {
            return self.deny(format!(
                "`AttackersDeclaredOneTarget` with `ValidAttackers$ {att}`"
            ));
        }
        let Some(target_raw) = p.take("AttackedTarget") else {
            return self.deny("`AttackersDeclaredOneTarget` with no `AttackedTarget$`".to_string());
        };
        let Some(who) = Self::attacked_player_rel(&target_raw) else {
            return self.deny(format!(
                "`AttackersDeclaredOneTarget` target `{target_raw}`"
            ));
        };
        if let Some(attacker) = p.take("AttackingPlayer") {
            let ok = attacker == "Player"
                || (who == "PlayerRel::You"
                    && matches!(attacker.as_str(), "Opponent" | "Player.Opponent"));
            if !ok {
                return self.deny(format!(
                    "`AttackersDeclaredOneTarget` with `AttackingPlayer$ {attacker}`"
                ));
            }
        }
        Some(format!("Trigger::PlayerAttacked({who})"))
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
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        Some(format!(
            "Trigger::CountersPlaced {{ filter: {filter}, kind: {kind}, by: {by}, first_this_turn: false }}"
        ))
    }

    fn becomes_target_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid_target) = p.take("ValidTarget") else {
            return self.deny("a `BecomesTarget` trigger not on `Card.Self`".to_string());
        };
        match p.take("ValidSource").as_deref() {
            None => match valid_target.as_str() {
                "Card.Self" => Some("Trigger::BecomesTarget".to_string()),
                "Card.EnchantedBy" => Some("Trigger::EnchantedBecomesTarget".to_string()),
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
        let dest = p.take("Destination");
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
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("an `AttackerBlocked` trigger with no `ValidCard$`".to_string());
        };
        if valid != "Card.Self" {
            if p.peek("ValidBlocker").is_some() {
                return self.deny("an `AttackerBlocked` trigger not on `Card.Self`".to_string());
            }
            let expr = self.filter_expr(&valid)?;
            let filter = self.body.filter_static("TRIGGER", &expr);
            return Some(format!("Trigger::ControlledBecomesBlocked(&{filter})"));
        }
        match p.take("ValidBlocker") {
            None => Some("Trigger::BecomesBlocked".to_string()),
            Some(blocker) => {
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
        let phase_descended = mode == "Phase" && p.peek("ValidPlayer") == Some("You.descended");
        p.drop_prose();
        self.trigger_mode = Some(mode.clone());
        if self.is_link_cleanup_trigger(&mode, &p) {
            return Some(());
        }
        if self.is_imprison_clause(&mode, &p) {
            return self.imprison_clause(&mode, p);
        }
        if mode != "AttackerBlockedByCreature" {
            match p.take("Secondary").as_deref() {
                None | Some("True") => {}
                Some(other) => return self.deny(format!("`Secondary$ {other}`")),
            }
        }
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
        let (trigger, mut condition) = if mode == "Always" {
            (self.state_trigger(&mut p)?, String::new())
        } else {
            let trigger = self.trigger_expr_in_zone(&mut p, &mode, !zone.is_empty())?;
            let wave8_cond = Self::wave8_trigger_present_condition(&mut p);
            let mut cond = self.condition(&mut p)?;
            if let Some(clause) = wave8_cond {
                cond = if cond.is_empty() {
                    format!(", condition = Some({clause})")
                } else if let Some(inner) = cond
                    .strip_prefix(", condition = Some(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    format!(", condition = Some(Condition::All(&[{inner}, {clause}]))")
                } else {
                    return self.deny("`IsPresent$` beside another clause".to_string());
                };
            }
            (trigger, cond)
        };
        if let Some(clause) = self_etb_extra_cond {
            condition = if condition.is_empty() {
                format!(", condition = Some({clause})")
            } else if let Some(inner) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!(", condition = Some(Condition::All(&[{inner}, {clause}]))")
            } else {
                return self.deny("`Card.Self+bargained/prototyped` beside another clause".into());
            };
        }
        if phase_descended {
            condition = if condition.is_empty() {
                ", condition = Some(Condition::DescendedThisTurn)".to_string()
            } else if let Some(inner) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!(
                    ", condition = Some(Condition::All(&[{inner}, Condition::DescendedThisTurn]))"
                )
            } else {
                return self.deny("`You.descended` beside another clause".to_string());
            };
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
                condition = if condition.is_empty() {
                    format!(", condition = Some({clause})")
                } else if let Some(inner) = condition
                    .strip_prefix(", condition = Some(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    format!(", condition = Some(Condition::All(&[{inner}, {clause}]))")
                } else {
                    return self.deny("`CheckSVar$` beside another clause".to_string());
                };
            }
        }
        if let Some(cond_raw) = p.peek("Condition")
            && let Some(clause) =
                Self::named_condition(cond_raw).or_else(|| Self::wave7_named_condition(cond_raw))
        {
            p.take("Condition");
            condition = if condition.is_empty() {
                format!(", condition = Some({clause})")
            } else if let Some(inner) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!(", condition = Some(Condition::All(&[{inner}, {clause}]))")
            } else {
                return self.deny("`Condition$` beside another clause".to_string());
            };
        }
        if p.peek("PlayerTurn") == Some("True") {
            p.take("PlayerTurn");
            condition = if condition.is_empty() {
                ", condition = Some(Condition::YourTurn)".to_string()
            } else if let Some(inner) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!(", condition = Some(Condition::All(&[{inner}, Condition::YourTurn]))")
            } else {
                return self.deny("`PlayerTurn$ True` beside another clause".to_string());
            };
        }
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
            || trigger == "Trigger::PHASES_IN"
            || trigger == "Trigger::PHASES_OUT"
            || trigger == "Trigger::EXERTED_THIS"
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
        if !may
            && price.is_none()
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
        self.cost_sacrifices_self = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p)) if p == "Sacrifice(&Filter::This)"
        );
        self.cost_sacrifices_other = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p))
                if p.starts_with("Sacrifice(") && p != "Sacrifice(&Filter::This)"
        );
        let chain_ok = self.chain(&body, &mut chain);
        self.trigger_is_self = false;
        self.is_etb_self_trigger = false;
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        chain_ok?;
        if chain.effects.is_empty() {
            return self.deny("a trigger that reads as no effect at all".to_string());
        }
        let chain_asks = chain.target_up_to.is_some()
            || chain
                .effects
                .iter()
                .any(|e| e.contains("ChooseCard {") && e.contains("min: 0,"))
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
            None if may && !asks_already => format!("Effect::MayDo {{ effects: &[{effects}] }}"),
            None => effects,
        };
        let tail = format!("&[{effects}]{zone}{once_per_turn}{targets}{second}{condition})");
        let ability = format!("triggered!({trigger}, {tail}");
        if let Some((role, other, secondary)) = self.block_line.take() {
            return self.pair_block_half(role, other, secondary, ability);
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
        ability: String,
    ) -> Option<()> {
        match self.block_half.take() {
            None => {
                self.body.abilities.push(ability.clone());
                self.block_half = Some(BlockHalf {
                    role,
                    other,
                    secondary,
                    ability,
                });
                Some(())
            }
            Some(first)
                if first.role != role
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
        if half.role == BlockRole::Blocked && !half.secondary {
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
            let block_pair = self.block_line.as_ref().is_some_and(|(role, ..)| {
                let expected = match role {
                    BlockRole::Blocks => "TriggeredAttacker",
                    BlockRole::Blocked => "TriggeredBlocker",
                };
                remembered.strip_suffix("LKICopy").unwrap_or(remembered) == expected
            });
            if self_combat || damage_done_target || block_pair {
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
                let mut tmp = Params {
                    entries: Vec::new(),
                };
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
        Some(match clauses.as_slice() {
            [] => "None".to_string(),
            [only] => format!("Some(&{only})"),
            many => format!("Some(&Condition::All(&[{}]))", many.join(", ")),
        })
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

    fn wave8_trigger_present_condition(p: &mut Params) -> Option<String> {
        if p.has("IsPresent2") || p.has("PresentPlayer") {
            return None;
        }
        let valid = p.peek("IsPresent")?;
        if valid == "Card.StrictlySelf"
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
            | "GiveGift"
            | "GaveGift" => true,
            "ChangesZone" | "TapsForMana" | "Taps" | "Untaps" | "Exerted" | "Enlisted"
            | "Sacrificed" => non_self,
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

    fn wave15_amount_cmp(lhs: &str, cmp: &str) -> Option<String> {
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

    fn wave15_entered_subtype_const(&self, rest: &str) -> Option<String> {
        let sub_word = rest
            .strip_suffix(".YouCtrl")
            .or_else(|| {
                rest.strip_prefix("Creature.")
                    .and_then(|s| s.strip_suffix("+YouCtrl"))
            })
            .or_else(|| {
                rest.strip_prefix("Permanent.")
                    .and_then(|s| s.strip_suffix("+YouCtrl"))
            })?
            .trim();
        self.single_subtype_const(sub_word).or_else(|| {
            if sub_word.is_empty() || sub_word.contains(['.', '+', ',', ' ', '!']) {
                return None;
            }
            let expr = self.filter_expr(&format!("Card.{sub_word}"))?;
            expr.strip_prefix("Filter::HasSubtype(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::to_string)
        })
    }

    /// Wave-15c trigger `CheckSVar$` / `SVarCompare$` clauses backed by
    /// existing `Condition` / `Amount` variants.
    fn wave15_trigger_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.svars.get(var.trim())?.trim();
        let cmp = cmp.trim();
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
            && let Some(sub_const) = self.wave15_entered_subtype_const(rest)
        {
            return bool_flag(&format!(
                "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{sub_const}])"
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
            return Self::wave15_amount_cmp("Amount::OpponentsYouAttackedThisCombat", cmp);
        }
        if raw == "Count$LifeYouGainedThisTurn" {
            return Self::wave15_amount_cmp("Amount::LifeGainedThisTurn(PlayerRel::You)", cmp);
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
        if p.has("Boon") || p.has("EffectOwner") || p.has("Abilities") || p.has("ImprintCards") {
            return None;
        }
        let is_emblem_name = p
            .peek("Name")
            .is_some_and(|n| n.trim_start().starts_with("Emblem"));
        if p.peek("Duration") == Some("Permanent") && is_emblem_name {
            return self.emblem_effect_api(p, target);
        }
        if p.peek("Triggers").is_none() || is_emblem_name {
            return None;
        }
        self.delayed_triggers_effect_api(p, target, chain)
    }

    fn collect_emblem_abilities(
        &mut self,
        svar_list: &str,
        is_trigger: bool,
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
            let saved_mode = is_trigger.then(|| self.trigger_mode.take()).flatten();
            let ok = if is_trigger {
                self.triggered(&body)
            } else {
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
            };
            self.on_a_spell = saved_on_spell;
            if is_trigger {
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
            if emitted.iter().any(|a| {
                a.contains("Filter::This")
                    || a.contains("TargetSpec::ThisObject")
                    || a.contains("Effect::SacrificeSelf")
                    || a.contains("Effect::ExileSource")
                    || a.contains("Effect::ModifySource")
                    || a.contains("Amount::Source")
                    || a.contains("Amount::CountersOnSource")
            }) {
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
        let only_cond_left = p
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"));
        if (statics.is_none() && triggers.is_none()) || !only_cond_left {
            return None;
        }
        let mut emblem_abilities = Vec::new();
        let saved_enter_len = self.body.enter_modifiers.len();
        if let Some(statics) = statics {
            self.collect_emblem_abilities(&statics, false, saved_enter_len, &mut emblem_abilities)?;
        }
        if let Some(triggers) = triggers {
            self.collect_emblem_abilities(&triggers, true, saved_enter_len, &mut emblem_abilities)?;
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
        if p.has("Planeswalker") {
            return None;
        }
        let dur_raw = p.take("Duration");
        let (static_dur, repeat_dur, oneshot_dur) = match dur_raw.as_deref() {
            None | Some("UntilEndOfTurn") => (
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
            if !matches!(
                mode.as_str(),
                "ChangesZone"
                    | "ChangesZoneAll"
                    | "SpellCast"
                    | "SpellCastOrCopy"
                    | "Attacks"
                    | "AttackersDeclared"
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
                    | "Sacrificed"
                    | "CounterAddedOnce"
                    | "AttackerBlocked"
                    | "AttackerUnblocked"
                    | "PhaseIn"
                    | "PhaseOut"
                    | "FlippedCoin"
                    | "RolledDie"
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
            if !uses_watch || mentions_triggered || exec_raw.contains("RememberedPlayer") {
                return None;
            }
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
        } else {
            (exec_raw.to_string(), "TriggerRemember::None")
        };
        let (effects, targets) =
            self.parse_delayed_body_str(&body_to_parse, Some(mode.to_string()), optional)?;
        Some((effects, targets, remember))
    }
}
