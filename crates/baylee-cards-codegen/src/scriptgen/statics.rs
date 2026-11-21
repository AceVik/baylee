//! Reading static abilities and the modifiers they apply.

use super::{
    Bound, Chain, Params, Tx, animation::supertype_const, card_type_const, count_bound,
    counter_kind, keyword_const,
};

pub(super) enum GrantedActivated {
    Modifier(String),
    SelfTarget {
        cost: String,
        effects: String,
        target: String,
    },
}

#[derive(Clone, Copy)]
pub(super) enum StaticEffectZone {
    Battlefield,
    Explicit(&'static str),
}

impl StaticEffectZone {
    const fn as_arg(self) -> Option<&'static str> {
        match self {
            Self::Battlefield => None,
            Self::Explicit(zone) => Some(zone),
        }
    }

    const fn is_battlefield(self) -> bool {
        matches!(self, Self::Battlefield)
    }
}

enum CantBeCastHandling {
    Handled(Option<()>),
    Pass,
}

impl Tx<'_> {
    pub(super) fn is_ai_only_static(&self, spec: &str) -> bool {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return false;
        };
        p.drop_prose();
        mode == "Continuous"
            && p.take("Affected").is_some()
            && p.take("AddSVar")
                .and_then(|s| self.svars.get(&s))
                .is_some_and(|v| {
                    matches!(
                        v.trim(),
                        "SVar:AITapDown:True" | "SVar:DestroyWhenDamaged:True"
                    )
                })
            && p.exhausted()
    }

    /// `R:Event$ BeginPhase | Phase$ Untap | Skip$ True` — "players skip
    /// their untap steps" (Stasis) — as `Modifier::SkipUntapStep` for every
    /// player.
    ///
    /// A static ability and not a replacement here, for the reason
    /// [`Tx::does_not_untap`] gives: the skip is read by the untap step
    /// itself (CR 614.10), with nothing to put in the step's place. Only the
    /// untap step, only from the battlefield, and only with no player named:
    /// the corpus writes this line three times, and the third is a plane's,
    /// from the command zone, which refuses.
    pub(super) fn skip_untap_steps(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        match p.take("ActiveZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => {
                return self.deny(format!("a skipped step from `ActiveZones$ {zone}`"));
            }
        }
        match (p.take("Phase").as_deref(), p.take("Skip").as_deref()) {
            (Some("Untap"), Some("True")) => {}
            (phase, skip) => {
                return self.deny(format!(
                    "`BeginPhase` of `{}` with `Skip$ {}`",
                    phase.unwrap_or("any step"),
                    skip.unwrap_or("nothing")
                ));
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `BeginPhase.{key}`"));
        }
        self.body.abilities.push(
            "static_ability!(Filter::Any, Modifier::SkipUntapStep { who: PlayerRel::EachPlayer })"
                .to_string(),
        );
        Some(())
    }

    /// `R:Event$ Untap | … | Layer$ CantHappen` as
    /// `Modifier::DoesNotUntap`.
    ///
    /// **A static ability and not a replacement**, although the reference
    /// writes it on an `R:` line. CR 502.3 makes untapping a turn-based
    /// action whose *determination* an effect changes, and CR 613.11 calls
    /// that a continuous effect modifying a game rule — there is no event
    /// being replaced with another. `Layer$ CantHappen` is the reference's
    /// own word for the same thing, and requiring it is what keeps the two
    /// scripts that really do replace the untap (with a counter removal)
    /// out of this rule.
    ///
    /// 157 scripts print the line and 136 are exactly this shape. Every key
    /// is claimed or the card is refused, which is what leaves the rest
    /// out: `IsPresent$` is a static that is only sometimes on,
    /// `CheckSVar$` a computed condition, `ReplaceWith$` a real
    /// replacement, and `Secondary$` a reference-side marker that is not on
    /// the prose list and will not be put there to move a number.
    pub(super) fn does_not_untap(&mut self, p: &mut Params) -> Option<()> {
        let enchanted_permanent = p
            .peek("Description")
            .is_some_and(|d| d.starts_with("Enchanted permanent"));
        p.drop_prose();
        // The battlefield is where a permanent untaps, and it is CR 113.6's
        // default — written out on 96 of the scripts and left off the other
        // 40. `Command` is Kaito's, which is a different card entirely.
        if let Some(zone) = p.take("ActiveZones")
            && zone != "Battlefield"
        {
            self.note(format!("a `doesn't untap` from `ActiveZones$ {zone}`"));
            return None;
        }
        // Whose untap step. Every printing says "your", and the variant is
        // documented as the effect controller's — so anything else here is
        // a rule this side cannot say rather than one it may assume.
        match p.take("ValidStepTurnToController").as_deref() {
            Some("You") => {}
            other => {
                return self.deny(format!(
                    "a `doesn't untap` during `{}`",
                    other.unwrap_or("any untap step")
                ));
            }
        }
        match p.take("Layer").as_deref() {
            Some("CantHappen") => {}
            other => {
                return self.deny(format!(
                    "an untap replacement on layer `{}`",
                    other.unwrap_or("none")
                ));
            }
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `doesn't untap` with no `ValidCard$`".to_string());
        };
        // Inlined rather than hoisted into a `static`, the way the `S:` path
        // one function down does it: `static_ability!` is a `const fn` over
        // a `Filter` *value*, and a `static` item cannot be read in a const
        // context.
        let filter = if valid == "Card.Self" {
            "Filter::This".to_string()
        } else if enchanted_permanent && valid == "Creature.EnchantedBy" {
            // Tractor Beam writes `ValidCard$ Creature.EnchantedBy` beside
            // `Description$ Enchanted permanent doesn't untap…`; an uncrewed
            // Spacecraft is not a creature, so follow the printed noun.
            "Filter::AttachedToBySource".to_string()
        } else {
            self.filter_expr(&valid)?
        };
        if p.peek("AddSVar")
            .and_then(|s| self.svars.get(s))
            .is_some_and(|v| v.trim() == "SVar:DestroyWhenDamaged:True")
        {
            p.take("AddSVar");
        }
        if !p.exhausted() {
            self.note(format!(
                "a `doesn't untap` with `{}`",
                p.first_key().unwrap_or_default()
            ));
            return None;
        }
        self.body
            .abilities
            .push(Self::static_expr(&filter, "Modifier::DoesNotUntap"));
        Some(())
    }

    /// An `S: Mode$ Continuous` line as one or more `AbilityDef::Static`.
    ///
    /// One printed sentence can be several continuous effects: "get +1/+1
    /// and have flying" changes power/toughness in layer 7c and abilities
    /// in layer 6, and CR 613.1 applies those in order. The corpus writes both
    /// on one line, so this emits one `StaticAbility` per layer touched
    /// rather than trying to fold them into one.
    #[allow(clippy::too_many_lines)]
    pub(super) fn static_ability(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("an `S:` line with no `$` in it".to_string());
        };
        if mode == "Continuous"
            && p.peek("AddKeyword")
                .is_some_and(|keyword| keyword.starts_with("Protection:"))
            && p.peek("CheckSVar").is_some()
        {
            return self.protection_from_imprinted(p);
        }
        match mode.as_str() {
            "AlternativeCost" => return self.alternative_cost(p),
            "RaiseCost" => return self.mana_life_tax(p),
            "ReduceCost" => return self.mode_reduce_cost(p),
            "FlipCoinDoubler" => {
                p.drop_prose();
                if p.take("ValidPlayer").as_deref() != Some("You") || !p.exhausted() {
                    return self.deny("a coin-flip replacement for another player".to_string());
                }
                self.body.abilities.push(Self::static_expr(
                    "Filter::Any",
                    "Modifier::FlipTwoCoinsIgnoreOne",
                ));
                return Some(());
            }
            "CantBeCast" => {
                p.drop_prose();
                match self.try_cant_be_cast_zone(&mut p) {
                    CantBeCastHandling::Handled(res) => return res,
                    CantBeCastHandling::Pass => return self.one_spell_each_turn(p),
                }
            }
            "CantBlockBy" => return self.cant_block_by(p),
            "MustAttack" => return self.must_attack(p),
            "CantAttack,CantBlock,CantBeActivated" | "CantAttack,CantBlock" => {
                return self.cant_attack_block_activate(&mode, p);
            }
            "CantBeActivated" if p.peek("ValidSA") == Some("Activated") => {
                return self.cant_be_activated(p);
            }
            "CantAttack" if p.peek("ValidCard") == Some("Card.Self+attackedLastTurn") => {
                return self.cant_attack_after_last_own_turn(p);
            }
            "CantAttackUnless" | "CantBlockUnless" => {
                return self.combat_mana_tax_per_counter(&mode, p);
            }
            "CantAttack" => return self.cant_attack_unless(p),
            "CantBlock" => return self.cant_block(p),
            "CanAttackDefender" => return self.attack_as_though("AttacksDespiteDefender", p),
            "CanAttackIfHaste" => return self.attack_as_though("AttacksAsThoughHaste", p),
            "CantPlayLand" => return self.cant_play_land(p),
            "CantPhaseOut" => return self.cant_phase_out(p),
            "OptionalAttackCost" => return self.optional_attack_cost(p),
            "IgnoreLandWalk" => return self.ignore_landwalk(p),
            _ => {}
        }
        if p.peek("EffectZone") == Some("Graveyard") && p.peek("Affected") == Some("Land.YouCtrl") {
            return self.riftstone_graveyard_grant(&mode, p);
        }
        if mode == "CantTarget"
            && (p.peek("SourceCanOnlyTarget").is_some() || p.peek("ValidSource").is_some())
        {
            return self.wall_only_target_prohibition(p);
        }
        if super::guardian52::candidate(&mode, &p) {
            return self.guardian_permission(&mode, p);
        }
        if mode != "Continuous" {
            self.note(format!("static ability `S: Mode$ {mode}`"));
            return None;
        }
        let had_description = p.has("Description");
        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Continuous.Secondary$ {other}`")),
        }
        // `EffectZone$ Battlefield` is the default written out; any other
        // zone means the source works from somewhere else, which is a
        // different rule than the one below.
        let graveyard_self_grant = p.peek("EffectZone") == Some("Graveyard")
            && p.peek("AffectedZone") == Some("Graveyard")
            && p.peek("Affected") == Some("Card.Self")
            && p.has("AddAbility");
        let graveyard_ok = (p.peek("Affected") == Some("Creature.YouCtrl") && p.has("AddKeyword"))
            || graveyard_self_grant;
        let stack_self_zone = p.peek("EffectZone") == Some("Stack")
            && p.peek("AffectedZone") == Some("Stack")
            && p.peek("Affected") == Some("Card.Self");
        let effect_zone = if stack_self_zone {
            p.take("EffectZone");
            Some("ZoneRef::Stack")
        } else {
            match p.take("EffectZone").as_deref() {
                raw @ Some(zone) => {
                    let Some(resolved) = Self::static_effect_zone(raw, graveyard_ok) else {
                        self.note(format!("static ability from `EffectZone$ {zone}`"));
                        return None;
                    };
                    resolved.as_arg()
                }
                None => None,
            }
        };
        if p.peek("AffectedZone") == Some("Hand")
            && p.peek("MayLookAt") == Some("Player")
            && p.peek("Affected").is_none()
        {
            return self.reveal_all_hands(p);
        }
        if graveyard_self_grant {
            return self.graveyard_self_granted_ability(p);
        }
        if effect_zone != Some("ZoneRef::Graveyard")
            && p.peek("AffectedZone") == Some("Graveyard")
            && p.peek("MayPlay") == Some("True")
        {
            return self.graveyard_may_play(p, effect_zone);
        }
        if let Some(zone) = p.peek("AffectedZone")
            && matches!(zone, "Stack" | "Battlefield,Stack" | "Stack,Battlefield")
        {
            let zone = p.take("AffectedZone").unwrap_or_default();
            return self.stack_continuous_static(p, effect_zone, &zone);
        }
        // Likewise `AffectedZone`: reaching past the battlefield is said by
        // a `Filter::InZone` in the filter, and a filter that has no zone
        // predicate in it cannot say *which* other zone. Refuse rather than
        // guess.
        if let Some(zone) = p.take("AffectedZone")
            && zone != "Battlefield"
        {
            self.note(format!("static ability reaching `AffectedZone$ {zone}`"));
            return None;
        }
        // "This creature's power and toughness are each equal to the number
        // of Swamps you control" (Nightmare): a characteristic-defining
        // ability (CR 604.3), about the card itself and so with no
        // `Affected$`.
        if p.peek("Affected").is_none()
            && p.take("CharacteristicDefining").as_deref() == Some("True")
        {
            return self.characteristic_pt(p);
        }
        let affected = self.continuous_affected(&mut p)?;
        if p.peek("AddKeyword")
            .is_some_and(|k| k.starts_with("UntapAdjust:"))
        {
            return self.untap_limit(&affected, p);
        }
        let (filter, implicit_cond) =
            if let Some((f, c)) = Self::soulbond_or_renown_affected(&affected) {
                (f.to_string(), Some(c))
            } else {
                (self.filter_expr(&affected)?, None)
            };
        // "Gets +1/+1 as long as you control a Swamp" (Sedge Troll): the
        // clause the `A:` line reads as a restriction is, on a static, the
        // condition under which the ability exists at all, and the engine
        // registers and removes it as the condition changes.
        let base_cond =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        let condition = Self::merge_static_condition(implicit_cond, &base_cond);
        let mut out = Vec::new();
        let mut self_granted = false;
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        if effect_zone == Some("ZoneRef::Graveyard")
            && (p.has("AddAbility") || p.has("AddTrigger") || p.has("AddStaticAbility"))
        {
            self.note("static ability from `EffectZone$ Graveyard`".to_string());
            return None;
        }
        self.grant_modifiers(
            &mut p,
            &filter,
            effect_zone.is_none(),
            &condition,
            &mut out,
            &mut self_granted,
        )?;
        self.block_modifiers(&mut p, &filter, &mut out)?;
        // "You control enchanted creature" (Control Magic): layer 2
        // (CR 613.1b), and the static's controller is who gains control —
        // `You` is the only player the modifier can name.
        match p.take("GainControl").as_deref() {
            None => {}
            Some("You") => out.push(Self::static_expr(&filter, "Modifier::GainControl")),
            Some(other) => {
                self.note(format!("control of a static given to `{other}`"));
                return None;
            }
        }
        match p.take("Goad").as_deref() {
            None => {}
            Some("True") => out.push(Self::static_expr(&filter, "Modifier::Goaded")),
            Some(other) => {
                self.note(format!("continuous static with `Goad$ {other}`"));
                return None;
            }
        }
        let dropped_ai_svar = self.strip_ai_hint_add_svar(&mut p);
        if effect_zone.is_some() || !condition.is_empty() {
            let zone_arg = effect_zone.map_or_else(String::new, |z| format!(", zone = {z}"));
            for ability in &mut out {
                if let Some(open) = ability.strip_suffix(')') {
                    *ability = format!("{open}{zone_arg}{condition})");
                }
            }
        }
        if out.is_empty() && !self_granted && dropped_ai_svar && !had_description && p.exhausted() {
            return Some(());
        }
        // The honest-stub rule: one key nothing claimed and the card stays
        // a stub, however much of the line was understood.
        if !p.exhausted() || (out.is_empty() && !self_granted) {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `Continuous.{key}`"));
            } else {
                self.note("a continuous static that changes nothing".to_string());
            }
            return None;
        }
        self.body.abilities.extend(out);
        Some(())
    }

    pub(super) fn static_effect_zone(
        zone: Option<&str>,
        allow_graveyard: bool,
    ) -> Option<StaticEffectZone> {
        match zone {
            None | Some("Battlefield") => Some(StaticEffectZone::Battlefield),
            Some("Command") => Some(StaticEffectZone::Explicit("ZoneRef::Command")),
            Some("Battlefield,Command" | "Command,Battlefield") => {
                Some(StaticEffectZone::Explicit("ZoneRef::BattlefieldOrCommand"))
            }
            Some("Graveyard") if allow_graveyard => {
                Some(StaticEffectZone::Explicit("ZoneRef::Graveyard"))
            }
            Some(_) => None,
        }
    }

    fn take_static_effect_zone(&self, p: &mut Params, mode: &str) -> Option<StaticEffectZone> {
        match p.take("EffectZone").as_deref() {
            raw @ Some(zone) => match Self::static_effect_zone(raw, false) {
                Some(resolved) => Some(resolved),
                None => self.deny(format!("static ability `{mode}` from `EffectZone$ {zone}`")),
            },
            None => Some(StaticEffectZone::Battlefield),
        }
    }

    pub(super) fn format_static_expr(
        filter: &str,
        modifier: &str,
        zone: StaticEffectZone,
        cond: &str,
    ) -> String {
        let zone_arg = zone
            .as_arg()
            .map_or_else(String::new, |z| format!(", zone = {z}"));
        format!("static_ability!({filter}, {modifier}{zone_arg}{cond})")
    }

    fn try_cant_be_cast_zone(&mut self, p: &mut Params) -> CantBeCastHandling {
        if p.has("NumLimitEachTurn")
            || !p
                .entries
                .iter()
                .all(|(k, _)| matches!(k.as_str(), "ValidCard" | "Caster" | "EffectZone"))
        {
            return CantBeCastHandling::Pass;
        }
        let Some(valid) = p.peek("ValidCard") else {
            return CantBeCastHandling::Pass;
        };
        if valid == "Card.Self" {
            return CantBeCastHandling::Pass;
        }
        let Some(caster) = p.peek("Caster") else {
            return CantBeCastHandling::Pass;
        };
        let has_cmd_zone = matches!(
            p.peek("EffectZone"),
            Some("Command" | "Battlefield,Command" | "Command,Battlefield")
        );
        let allow = caster == "You" || (caster == "Opponent" && has_cmd_zone);
        if !allow {
            return CantBeCastHandling::Pass;
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return CantBeCastHandling::Pass;
        };
        let Some(valid) = p.take("ValidCard") else {
            return CantBeCastHandling::Pass;
        };
        let Some(caster) = p.take("Caster") else {
            return CantBeCastHandling::Pass;
        };
        let filter_ref = if caster == "You" && valid == "Creature" && zone.is_battlefield() {
            "&Filter::CREATURE".to_string()
        } else {
            let Some(expr) = self.filter_expr(&valid) else {
                return CantBeCastHandling::Handled(None);
            };
            format!("&{}", self.body.filter_static("CANT_CAST", &expr))
        };
        let modifier = if caster == "You" {
            format!("Modifier::PlayersCantCast {{ who: PlayerRel::You, filter: {filter_ref} }}")
        } else {
            format!("Modifier::OpponentsCantCast({filter_ref})")
        };
        self.body
            .abilities
            .push(Self::format_static_expr("Filter::Any", &modifier, zone, ""));
        CantBeCastHandling::Handled(Some(()))
    }

    fn soulbond_or_renown_affected(affected: &str) -> Option<(&'static str, &'static str)> {
        match affected.trim() {
            "Creature.PairedWith,Creature.Self+Paired"
            | "Creature.Self+Paired,Creature.PairedWith"
            | "Card.PairedWith,Card.Self+Paired"
            | "Card.Self+Paired,Card.PairedWith" => {
                Some(("Filter::SOULBOND_PAIR", "Condition::IsPaired"))
            }
            "Creature.Self+Paired" | "Card.Self+Paired" => {
                Some(("Filter::This", "Condition::IsPaired"))
            }
            "Card.Self+IsRenowned" | "Creature.Self+IsRenowned" => {
                Some(("Filter::This", "Condition::IsRenowned"))
            }
            "Card.Self+!IsRenowned" | "Creature.Self+!IsRenowned" => {
                Some(("Filter::This", "Condition::Not(&Condition::IsRenowned)"))
            }
            "Card.Self+PromisedGift" | "Creature.Self+PromisedGift" => {
                Some(("Filter::This", "Condition::GiftWasPromised"))
            }
            "Card.Self+!PromisedGift" | "Creature.Self+!PromisedGift" => Some((
                "Filter::This",
                "Condition::Not(&Condition::GiftWasPromised)",
            )),
            "Card.Self+CastSa Spell.Warp"
            | "Creature.Self+CastSa Spell.Warp"
            | "Card.Self+Warped"
            | "Creature.Self+Warped" => Some(("Filter::This", "Condition::WarpCostPaid")),
            "Card.Self+!CastSa Spell.Warp"
            | "Creature.Self+!CastSa Spell.Warp"
            | "Card.Self+!Warped"
            | "Creature.Self+!Warped" => {
                Some(("Filter::This", "Condition::Not(&Condition::WarpCostPaid)"))
            }
            _ => None,
        }
    }

    pub(super) fn take_self_state_present(&self, p: &mut Params) -> Result<Option<String>, ()> {
        let Some(present) = p.peek("IsPresent") else {
            return Ok(None);
        };
        let cond = match present.trim() {
            "Card.Self+IsRenowned" | "Creature.Self+IsRenowned" => "Condition::IsRenowned",
            "Card.Self+Paired" | "Creature.Self+Paired" => "Condition::IsPaired",
            "Card.Self+PromisedGift" | "Creature.Self+PromisedGift" | "Card.PromisedGift" => {
                "Condition::GiftWasPromised"
            }
            "Card.Self+CastSa Spell.Warp"
            | "Creature.Self+CastSa Spell.Warp"
            | "Card.Self+Warped"
            | "Creature.Self+Warped" => "Condition::WarpCostPaid",
            _ => return Ok(None),
        };
        p.take("IsPresent");
        match p.take("PresentCompare").as_deref() {
            None | Some("GE1" | "EQ1" | "GT0") => Ok(Some(cond.to_string())),
            Some("EQ0" | "LT1") => Ok(Some(format!("Condition::Not(&{cond})"))),
            Some(other) => {
                self.note(format!("`PresentCompare$ {other}`"));
                Err(())
            }
        }
    }

    pub(super) fn promised_gift_svar_condition(&self, var: &str, cmp: &str) -> Option<String> {
        let def = self
            .svars
            .get(var.trim())
            .map_or(var, String::as_str)
            .trim();
        let positive = match def {
            "Count$PromisedGift.1.0" => true,
            "Count$PromisedGift.0.1" => false,
            _ => return None,
        };
        let holds_when_one = match cmp.trim() {
            "GE1" | "EQ1" | "GT0" => true,
            "EQ0" | "LT1" => false,
            _ => return None,
        };
        Some(if positive == holds_when_one {
            "Condition::GiftWasPromised".to_string()
        } else {
            "Condition::Not(&Condition::GiftWasPromised)".to_string()
        })
    }

    fn merge_static_condition(extra: Option<&str>, base: &str) -> String {
        let Some(extra) = extra else {
            return base.to_string();
        };
        if let Some(existing) = base
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            format!(", condition = Some(Condition::All(&[{extra}, {existing}]))")
        } else {
            format!(", condition = Some({extra})")
        }
    }

    fn static_conditions(&mut self, p: &mut Params, check_svar_denial: &str) -> Option<String> {
        let mut clauses: Vec<String> = Vec::new();
        if let Some(cond_raw) = p.peek("Condition")
            && p.has("IsPresent")
            && let Some(named) = Self::named_condition(cond_raw)
        {
            p.take("Condition");
            clauses.push(named.to_string());
        }
        if let Some(state_cond) = self.take_self_state_present(p).ok()? {
            clauses.push(state_cond);
        }
        if let Some(base) = self
            .condition(p)?
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(base.to_string());
        }
        if let Some(var) = p.take("CheckSVar") {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            let Some(first) = self
                .svar_condition_clause(&var, &cmp)
                .or_else(|| self.promised_gift_svar_condition(&var, &cmp))
            else {
                return self.deny(check_svar_denial.to_string());
            };
            clauses.push(first);
            if let Some(var2) = p.take("CheckSecondSVar") {
                let cmp2 = p
                    .take("SecondSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let Some(second) = self
                    .svar_condition_clause(&var2, &cmp2)
                    .or_else(|| self.promised_gift_svar_condition(&var2, &cmp2))
                else {
                    return self.deny(check_svar_denial.to_string());
                };
                clauses.push(second);
            }
        }
        Some(match clauses.as_slice() {
            [] => String::new(),
            [one] => format!(", condition = Some({one})"),
            many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
        })
    }

    fn reveal_all_hands(&mut self, mut p: Params) -> Option<()> {
        p.take("AffectedZone");
        p.take("MayLookAt");
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::RevealAllHands{condition})"
        ));
        Some(())
    }

    fn cant_attack_block_activate(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, mode)?;
        let Some(valid) = p.take("ValidCard") else {
            return self.deny(format!("a `{mode}` with no `ValidCard$`"));
        };
        let filter = self.filter_expr(&valid)?;
        let condition =
            self.static_conditions(&mut p, &format!("unclaimed parameter `{mode}.CheckSVar`"))?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{mode}.{key}`"));
            }
            return None;
        }
        let mut modifiers =
            vec!["Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))"];
        if mode == "CantAttack,CantBlock,CantBeActivated" {
            modifiers.push("Modifier::CantActivate");
        }
        for modifier in modifiers {
            self.body.abilities.push(Self::format_static_expr(
                &filter, modifier, zone, &condition,
            ));
        }
        Some(())
    }

    fn cant_be_activated(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantBeActivated")?;
        p.take("ValidSA");
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CantBeActivated` with no `ValidCard$`".to_string());
        };
        let filter = self.filter_expr(&valid)?;
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `CantBeActivated.CheckSVar`")?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantBeActivated.{key}`"));
            }
            return None;
        }
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::CantActivate",
            zone,
            &condition,
        ));
        Some(())
    }

    fn cant_block(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantBlock")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`CantBlock.Secondary$ {other}`")),
        }
        if let Some(other) = p.take("NonStackingEffect")
            && other != "True"
        {
            return self.deny(format!("`NonStackingEffect$ {other}`"));
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CantBlock` static with no `ValidCard$`".to_string());
        };
        if valid == "Card.Self"
            && zone.is_battlefield()
            && (p.has("CheckSVar") || p.has("SVarCompare"))
        {
            let condition = p.take("Condition");
            let checked = p.take("CheckSVar");
            let comparison = p.take("SVarCompare");
            let yours = self.svars.get("X");
            let active = self.svars.get("Y");
            if condition.as_deref() != Some("NotPlayerTurn")
                || checked.as_deref() != Some("Y")
                || comparison.as_deref() != Some("GEX")
                || yours.is_none_or(|v| v.trim() != "Count$Valid Creature.YouCtrl")
                || active.is_none_or(|v| v.trim() != "Count$Valid Creature.ActivePlayerCtrl")
            {
                return self.deny(
                    "a `CantBlock` creature-count comparison other than active fewer than yours"
                        .to_string(),
                );
            }
            if let Some(key) = p.first_key() {
                return self.deny(format!("unclaimed parameter `CantBlock.{key}`"));
            }
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                "Modifier::CantBlockUnlessYouControlMoreCreaturesThanAttacker",
            ));
            return Some(());
        }
        let filter = self.filter_expr(&valid)?;
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `CantBlock.CheckSVar`")?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantBlock.{key}`"));
            }
            return None;
        }
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)",
            zone,
            &condition,
        ));
        Some(())
    }

    /// Normalize the selected objects before reading their validity predicate.
    /// Applying the relation to every alternative preserves comma-union semantics.
    /// Unknown definitions must never silently widen to every permanent.
    fn continuous_affected(&self, p: &mut Params) -> Option<String> {
        let affected = p.take("Affected");
        let Some(defined) = p.take("AffectedDefined") else {
            return affected
                .or_else(|| self.deny("a continuous static with no `Affected$`".to_string()));
        };
        let relation = match defined.as_str() {
            "Self" => "Self",
            "Enchanted" => "EnchantedBy",
            "Equipped" => "EquippedBy",
            "AttachedBy Self" => "AttachedBy",
            other => return self.deny(format!("continuous `AffectedDefined$ {other}`")),
        };
        let valid = affected.as_deref().unwrap_or("Card");
        Some(
            valid
                .split(',')
                .map(|alternative| {
                    let (base, tail) = alternative
                        .trim()
                        .split_once('.')
                        .unwrap_or((alternative.trim(), ""));
                    if tail.is_empty() {
                        format!("{base}.{relation}")
                    } else {
                        format!("{base}.{relation}+{tail}")
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// "Players can't untap more than one creature during their untap
    /// steps" (Smoke), and Winter Orb's "…one land…" while it is untapped:
    /// `Affected$ <player> | AddKeyword$ UntapAdjust:<valid>:<n>` as
    /// `Modifier::UntapAtMost`. The keyword is all the line may grant, the
    /// player one this reader can name, and the only other clause read is
    /// the `IsPresent$` condition every continuous static may carry.
    pub(super) fn untap_limit(&mut self, affected: &str, mut p: Params) -> Option<()> {
        let who = match affected {
            "Player" => "PlayerRel::EachPlayer",
            "You" => "PlayerRel::You",
            "Opponent" | "Player.Opponent" => "PlayerRel::EachOpponent",
            "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                "PlayerRel::EnchantedPlayer"
            }
            other => return self.deny(format!("an untap limit on `Affected$ {other}`")),
        };
        let keyword = p.take("AddKeyword")?;
        let mut parts = keyword.split(':');
        let (Some("UntapAdjust"), Some(valid), Some(count), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return self.deny(format!("keyword `{keyword}` beside an untap limit"));
        };
        let Ok(count) = count.trim().parse::<u8>() else {
            return self.deny(format!("an untap limit of `{count}`"));
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("UNTAPPING", &expr);
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::UntapAtMost {{ who: {who}, of: &{name}, \
             count: {count} }}{condition})"
        ));
        Some(())
    }

    /// Graveyard cast/play permissions (`AffectedZone$ Graveyard | MayPlay$ True`):
    /// - `Land.YouOwn` (+ optional static condition) -> `Modifier::PlayLandsFromGraveyard`
    /// - `Permanent.nonLand+YouOwn` -> `Modifier::CastPermanentSpellsFromGraveyard`
    /// - `Permanent.YouOwn` -> both `PlayLandsFromGraveyard` and `CastPermanentSpellsFromGraveyard`
    /// - `Card.YouOwn` -> both `PlayLandsFromGraveyard` and `CastSpellsFromGraveyard`
    /// - `Instant.YouCtrl,Sorcery.YouCtrl` (or `YouOwn`) with `Condition$ PlayerTurn`,
    ///   `ReplaceGraveyard$ Exile`, and `MayPlayLimit$ 1` ->
    ///   `Modifier::GraveyardInstantSorceryOnce`
    fn graveyard_may_play(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        p.take("AffectedZone");
        p.take("MayPlay");
        let deny_zone = |this: &mut Self| -> Option<()> {
            this.note("static ability reaching `AffectedZone$ Graveyard`".to_string());
            None
        };
        let Some(affected) = p.take("Affected") else {
            return deny_zone(self);
        };
        let zone_arg = effect_zone.map_or_else(String::new, |z| format!(", zone = {z}"));
        if matches!(
            affected.as_str(),
            "Instant.YouCtrl,Sorcery.YouCtrl"
                | "Sorcery.YouCtrl,Instant.YouCtrl"
                | "Instant.YouOwn,Sorcery.YouOwn"
                | "Sorcery.YouOwn,Instant.YouOwn"
        ) {
            if p.take("Condition").as_deref() != Some("PlayerTurn")
                || p.take("ReplaceGraveyard").as_deref() != Some("Exile")
                || p.take("MayPlayLimit").as_deref() != Some("1")
                || p.first_key().is_some()
            {
                return deny_zone(self);
            }
            self.body.abilities.push(format!(
                "static_ability!(Filter::Any, Modifier::GraveyardInstantSorceryOnce{zone_arg})"
            ));
            return Some(());
        }
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        if p.first_key().is_some() {
            return deny_zone(self);
        }
        let modifiers: &[&str] = match affected.trim() {
            "Land.YouOwn" | "Land.YouCtrl" => &["Modifier::PlayLandsFromGraveyard"],
            "Card.YouOwn" | "Card.YouCtrl" => &[
                "Modifier::PlayLandsFromGraveyard",
                "Modifier::CastSpellsFromGraveyard",
            ],
            "Permanent.nonLand+YouOwn"
            | "Permanent.YouOwn+nonLand"
            | "Permanent.nonLand+YouCtrl"
            | "Permanent.YouCtrl+nonLand" => &["Modifier::CastPermanentSpellsFromGraveyard"],
            "Permanent.YouOwn" | "Permanent.YouCtrl" => &[
                "Modifier::PlayLandsFromGraveyard",
                "Modifier::CastPermanentSpellsFromGraveyard",
            ],
            _ => return deny_zone(self),
        };
        for modifier in modifiers {
            self.body.abilities.push(format!(
                "static_ability!(Filter::Any, {modifier}{zone_arg}{condition})"
            ));
        }
        Some(())
    }

    /// Self-granted graveyard activated abilities (`EffectZone$ Graveyard |
    /// AffectedZone$ Graveyard | Affected$ Card.Self | AddAbility$ ...`, e.g.
    /// Max speed graveyard cycling/draw activations).
    fn graveyard_self_granted_ability(&mut self, mut p: Params) -> Option<()> {
        p.take("AffectedZone");
        if p.take("Affected").as_deref() != Some("Card.Self") {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        let Some(raw) = p.take("AddAbility") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        let names: Vec<&str> = raw
            .split(" & ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if names.is_empty() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        for name in names {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("`AddAbility$ {name}` naming no `SVar`"));
            };
            if !line.trim_start().starts_with("AB$") {
                return self.deny("a granted ability that is not an activated one".to_string());
            }
            let cleaned = strip_secondary_true(&line);
            let parsed = self.parse_activated_spec(&cleaned, false, false)?;
            if !parsed.non_battlefield_zone
                || parsed.alternate_ability_expr.is_some()
                || !parsed.ability_expr.contains("ActivationZone::Graveyard")
            {
                return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
            }
            let expr = splice_activated_condition(&parsed.ability_expr, &condition)?;
            self.body.abilities.push(expr);
        }
        Some(())
    }

    /// Continuous statics targeting objects on the stack (`AffectedZone$ Stack`)
    /// or across both battlefield and stack (`AffectedZone$ Battlefield,Stack`).
    /// Strictly restricted to stack-live keywords (`Lifelink`, `Deathtouch`,
    /// `Wither`, `Infect`, `Split second`, `Rebound`, `Storm`, uncounterable)
    /// and Layer 4/5 type/color modifiers that `baylee-engine` projects on
    /// `Zone::Stack`. Cast-time keywords (`Convoke`, `Delve`, `Cascade`,
    /// `Affinity`, `Flash`) stay honestly refused.
    fn stack_continuous_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
        affected_zone: &str,
    ) -> Option<()> {
        let affected = self.continuous_affected(&mut p)?;
        let base_filter = self.filter_expr_in_zone(&affected, true)?;
        let zone_clause = if affected_zone == "Stack" {
            "Filter::InZone(ZoneRef::Stack)"
        } else {
            "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::Stack)])"
        };
        let filter = if base_filter == "Filter::Any" {
            zone_clause.to_string()
        } else {
            format!("Filter::And(&[{base_filter}, {zone_clause}])")
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        let mut out = Vec::new();
        if let Some(raw_kw) = p.take("AddKeyword") {
            let mut bits = Vec::new();
            for word in raw_kw.split(" & ").map(str::trim).filter(|s| !s.is_empty()) {
                let Some(bit) = stack_live_keyword_const(word) else {
                    return self.deny(format!(
                        "static ability reaching `AffectedZone$ {affected_zone}`"
                    ));
                };
                bits.push(bit);
            }
            if let Some((head, tail)) = bits.split_first() {
                let set = tail
                    .iter()
                    .fold((*head).to_string(), |acc, b| format!("{acc}.union({b})"));
                out.push(Self::static_expr(
                    &filter,
                    &format!("Modifier::AddKeyword({set})"),
                ));
            }
        }
        if !p.has("RemoveLandTypes") {
            self.type_modifiers(&mut p, &filter, &mut out)?;
        }
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() || out.is_empty() {
            return self.deny(format!(
                "static ability reaching `AffectedZone$ {affected_zone}`"
            ));
        }
        let zone_arg = effect_zone.map_or_else(String::new, |z| format!(", zone = {z}"));
        for ability in &mut out {
            if (!zone_arg.is_empty() || !condition.is_empty())
                && let Some(open) = ability.strip_suffix(')')
            {
                *ability = format!("{open}{zone_arg}{condition})");
            }
        }
        self.body.abilities.extend(out);
        Some(())
    }

    /// A characteristic-defining power and toughness (CR 604.3): both equal
    /// to one count of permanents, the ones you control
    /// (`PtCount::YouControl`) or all of them (`PtCount::OnBattlefield`).
    /// Layer 7a on the battlefield, and the card's own number everywhere
    /// else (CR 604.3), both of which `Modifier::CharacteristicPT` is. A
    /// count of somebody else's permanents, a toughness apart from the power
    /// or a clause beside it (Gaea's Liege's "as long as it isn't
    /// attacking") is another sentence, and refused.
    pub(super) fn characteristic_pt(&mut self, mut p: Params) -> Option<()> {
        let power_var = p.take("SetPower");
        let tough_var = p.take("SetToughness");
        if power_var.is_none() && tough_var.is_none() {
            return self.deny("a characteristic-defining ability with no P/T".to_string());
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        match (power_var, tough_var) {
            (Some(power), Some(toughness)) if power == toughness => {
                let count = self.pt_count_for_svar(&power)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                     count: {count}, toughness_plus: 0 }})"
                ));
                Some(())
            }
            (Some(power), Some(toughness)) => {
                let t_def = self.svars.get(&toughness).cloned();
                if let Some(t_def) = t_def.as_deref()
                    && let Some(rest) = t_def.trim().strip_prefix(&format!("SVar${power}/Plus."))
                    && let Ok(plus) = rest.trim().parse::<i8>()
                {
                    let count = self.pt_count_for_svar(&power)?;
                    self.body.abilities.push(format!(
                        "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                         count: {count}, toughness_plus: {plus} }})"
                    ));
                    return Some(());
                }
                let p_count = self.pt_count_for_svar(&power)?;
                let t_count = self.pt_count_for_svar(&toughness)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: {p_count}, toughness: {t_count} }})"
                ));
                Some(())
            }
            (Some(power), None) => {
                let (_, fixed_t) = self.pt.and_then(|pt| pt.split_once('/'))?;
                let Ok(t_num) = fixed_t.trim().parse::<u16>() else {
                    return self.deny("a characteristic-defining ability with no P/T".to_string());
                };
                let p_count = self.pt_count_for_svar(&power)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: {p_count}, toughness: PtCount::Fixed({t_num}) }})"
                ));
                Some(())
            }
            (None, Some(toughness)) => {
                let (fixed_p, _) = self.pt.and_then(|pt| pt.split_once('/'))?;
                let Ok(p_num) = fixed_p.trim().parse::<u16>() else {
                    return self.deny("a characteristic-defining ability with no P/T".to_string());
                };
                let t_count = self.pt_count_for_svar(&toughness)?;
                self.body.abilities.push(format!(
                    "static_ability!(Filter::This, Modifier::CharacteristicPTSeparate {{ \
                     power: PtCount::Fixed({p_num}), toughness: {t_count} }})"
                ));
                Some(())
            }
            (None, None) => unreachable!(),
        }
    }

    fn pt_count_for_svar(&mut self, var: &str) -> Option<String> {
        let Some(raw) = self.svars.get(var.trim()).cloned() else {
            return self.deny(format!("`{var}` names no SVar"));
        };
        let raw = raw.trim();
        if let Some((lhs, rhs)) = raw.rsplit_once("/Plus.")
            && let Ok(offset) = rhs.trim().parse::<u16>()
        {
            let base = if let Some(inner_var) = lhs.trim().strip_prefix("SVar$") {
                let inner = self.svars.get(inner_var.trim())?.clone();
                self.pt_count_base(inner.trim())?
            } else {
                self.pt_count_base(lhs.trim())?
            };
            return Some(format!(
                "PtCount::Plus {{ base: &{base}, offset: {offset} }}"
            ));
        }
        self.pt_count_base(raw)
    }

    fn pt_count_base(&mut self, count: &str) -> Option<String> {
        if let Some(counter) = count.strip_prefix("Count$CardCounters.") {
            let Some(kind) = super::counter_kind(counter) else {
                return self.deny(format!("counter `{counter}`"));
            };
            return Some(format!("PtCount::CountersOnThis({kind})"));
        }
        if count == "Count$ValidGraveyard Card$CardTypes" {
            return Some("PtCount::CardTypesInAllGraveyards".to_string());
        }
        if count == "Count$ValidGraveyard Card.OppOwn$CardTypes" {
            return Some("PtCount::CardTypesInOpponentsGraveyards".to_string());
        }
        if let Some(valid) = count.strip_prefix("Count$ValidGraveyard ") {
            if valid.contains("Ctrl")
                || valid.contains("Own")
                || valid.contains('/')
                || valid.contains('$')
            {
                return self.deny(format!("count `{count}`"));
            }
            let filter = self.filter_expr_in_zone(valid, true)?;
            let name = self.body.filter_static("COUNTED", &filter);
            return Some(format!("PtCount::InAllGraveyards(&{name})"));
        }
        let Some(valid) = count.strip_prefix("Count$Valid ") else {
            return self.deny(format!("count `{count}`"));
        };
        if let Some(color_valid) = valid.strip_suffix("$Colors")
            && !color_valid.contains('/')
            && !color_valid.contains('$')
        {
            let yours = color_valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
            if !yours && (color_valid.contains("Ctrl") || color_valid.contains("Own")) {
                return self.deny(format!("count `{count}`"));
            }
            let filter = self.filter_expr(color_valid)?;
            let name = self.body.filter_static("COUNTED", &filter);
            return Some(format!("PtCount::DistinctColorsAmong(&{name})"));
        }
        if valid.contains('/') || valid.contains('$') {
            return self.deny(format!("count `{count}`"));
        }
        let yours = valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
        if !yours && (valid.contains("Ctrl") || valid.contains("Own")) {
            return self.deny(format!("count `{count}`"));
        }
        let filter = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNTED", &filter);
        let kind = if yours { "YouControl" } else { "OnBattlefield" };
        Some(format!("PtCount::{kind}(&{name})"))
    }

    /// "Enchanted Wall can attack as though it didn't have defender"
    /// (Animate Wall) and "enchanted creature can attack as though it had
    /// haste" (Instill Energy): a permission on the creatures `ValidCard$`
    /// names, as `Modifier::AttacksDespiteDefender` or
    /// `Modifier::AttacksAsThoughHaste`. Only `ValidCard$` and the
    /// `IsPresent$` condition are read; a line that names what may be
    /// attacked (`ValidTarget$`) is another sentence and refuses.
    pub(super) fn attack_as_though(&mut self, modifier: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        let Some(valid) = p.take("ValidCard") else {
            return self.deny(format!("`{modifier}` naming no creature"));
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `{modifier}.{key}`"));
        }
        let filter = self.filter_expr(&valid)?;
        self.body.abilities.push(format!(
            "static_ability!({filter}, Modifier::{modifier}{condition})"
        ));
        Some(())
    }

    /// Nine per-type static lines can describe one protection ability whose
    /// set is the card types among the source's exiled cards. Read all nine,
    /// but emit the shared rule only once.
    fn protection_from_imprinted(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("AffectedDefined").as_deref() != Some("Self") {
            return None;
        }
        let keyword = p.take("AddKeyword")?;
        let check = p.take("CheckSVar")?;
        if !matches!(p.take("SVarCompare").as_deref(), None | Some("GE1")) || !p.exhausted() {
            return None;
        }
        let valid = self
            .svars
            .get(check.trim())?
            .strip_prefix("Imprinted$Valid ")?;
        if keyword != format!("Protection:{valid}")
            || !matches!(
                valid,
                "Artifact"
                    | "Battle"
                    | "Creature"
                    | "Enchantment"
                    | "Instant"
                    | "Kindred"
                    | "Land"
                    | "Planeswalker"
                    | "Sorcery"
            )
        {
            return None;
        }
        if valid == "Creature" {
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                "Modifier::ProtectionFromExiledWithSourceTypes",
            ));
        }
        Some(())
    }

    /// "Can't attack or block unless you pay {1} for each counter of a kind
    /// on this creature": one restriction per combat declaration.
    fn combat_mana_tax_per_counter(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self") {
            return None;
        }
        let cost = p.take("Cost")?;
        let code = self
            .svars
            .get(cost.trim())
            .and_then(|definition| definition.strip_prefix("Count$CardCounters."))?;
        let kind = counter_kind(code)?;
        if !p.exhausted() {
            return None;
        }
        let modifier = match mode {
            "CantAttackUnless" => format!("Modifier::CantAttackUnlessPayManaPerCounter({kind})"),
            "CantBlockUnless" => format!("Modifier::CantBlockUnlessPayManaPerCounter({kind})"),
            _ => return None,
        };
        self.body
            .abilities
            .push(Self::static_expr("Filter::This", &modifier));
        Some(())
    }

    pub(super) fn cant_play_land(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::PlayersCantPlay(&Filter::LAND){condition})"
        ));
        Some(())
    }

    /// "Can't be blocked by Walls" (Juggernaut), "can't be blocked except
    /// by Walls" (Invisibility), "can't block creatures with power 2 or
    /// greater" (Ironclaw Orcs): one restriction on a pairing (CR 509.1b),
    /// which `Modifier::CantBeBlockedBy` states from the attacker's side.
    ///
    /// `combat::can_block` reads the blocker's filter against the static's
    /// own source, so `Self` on that side is the card that states it, and
    /// a blocker-side restriction is the same modifier on every attacker
    /// the other filter names.
    pub(super) fn cant_block_by(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantBlockBy")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`CantBlockBy.Secondary$ {other}`")),
        }
        let Some(attacker) = p.take("ValidAttacker") else {
            return self.deny("`CantBlockBy` naming no attacker or no blocker".to_string());
        };
        let blocker = p.take("ValidBlocker");
        // Peeked before it is taken: a clause of another value stays for
        // `condition` to read or refuse, and is never dropped as if absent.
        if attacker == "Card.Self"
            && blocker.is_none()
            && zone.is_battlefield()
            && p.peek("IsPresent") == Some("Artifact.DefenderCtrl")
        {
            p.take("IsPresent");
            self.body.abilities.push(
                "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNBLOCKABLE), condition = Some(Condition::Not(&Condition::SomeDefendingPlayerControlsNone(&Filter::ARTIFACT))))"
                    .to_string(),
            );
            return Some(());
        }
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `CantBlockBy.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `CantBlockBy.{key}`"));
            return None;
        }
        let (attackers, modifier) = if let Some(blocker) = blocker {
            let attackers = self.filter_expr(&attacker)?;
            let blockers = self.filter_expr(&blocker)?;
            let name = self.body.filter_static("BLOCKER", &blockers);
            (attackers, format!("Modifier::CantBeBlockedBy(&{name})"))
        } else {
            let attackers = if matches!(attacker.as_str(), "Creature.Self" | "Card.Self") {
                "Filter::This".to_string()
            } else {
                self.filter_expr(&attacker)?
            };
            (
                attackers,
                "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)".to_string(),
            )
        };
        self.body.abilities.push(Self::format_static_expr(
            &attackers, &modifier, zone, &condition,
        ));
        Some(())
    }

    /// "Attacks each combat if able" (CR 508.1d): `S:Mode$ MustAttack`, a
    /// static naming the creatures it binds. `MustAttack$` names *what*
    /// they attack if able, a requirement about the pair the engine does
    /// not know, and stays unclaimed.
    pub(super) fn must_attack(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "MustAttack")?;
        let Some(valid) = p.take("ValidCreature") else {
            return self.deny("`MustAttack` naming no creature".to_string());
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `MustAttack.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `MustAttack.{key}`"));
            return None;
        }
        let filter = self.filter_expr(&valid)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::AttacksEachCombat",
            zone,
            &condition,
        ));
        Some(())
    }

    /// One source's attack during its current controller's last actual turn.
    /// The exact historical predicate is not an ordinary battlefield filter.
    fn cant_attack_after_last_own_turn(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self+attackedLastTurn") {
            return None;
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `CantAttack.{key}`"));
        }
        self.body.abilities.push(
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_ATTACK), condition = Some(Condition::SourceAttackedDuringYourLastTurn))".to_string(),
        );
        Some(())
    }

    /// "Can't attack unless defending player controls an Island" (CR
    /// 508.1c): `S:Mode$ CantAttack | UnlessDefender$ controls<filter>`.
    /// Goblin Goon's comparative creature count has its own exact spelling.
    /// A plain "can't attack" and the other `UnlessDefender$` questions
    /// (poisoned, the monarch) stay unclaimed, and so does a negated one
    /// (`!controls…`): "unless defending player controls no untapped lands"
    /// is a different sentence.
    pub(super) fn cant_attack_unless(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantAttack")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`CantAttack.Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("`CantAttack` naming no creature".to_string());
        };
        if p.peek("UnlessDefender") == Some("hasFewerCreaturesInPlayThanYou")
            && zone.is_battlefield()
        {
            p.take("UnlessDefender");
            if valid != "Card.Self" {
                return self.deny(format!("a creature-count attack restriction on `{valid}`"));
            }
            if let Some(key) = p.first_key() {
                return self.deny(format!("unclaimed parameter `CantAttack.{key}`"));
            }
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                "Modifier::CantAttackUnlessYouControlMoreCreaturesThanDefender",
            ));
            return Some(());
        }
        if p.peek("Target") == Some("You") && !p.has("UnlessDefender") {
            p.take("Target");
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `CantAttack.CheckSVar`")?;
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantAttack.{key}`"));
                return None;
            }
            let attackers = self.filter_expr(&valid)?;
            self.body.abilities.push(Self::format_static_expr(
                &attackers,
                "Modifier::CantAttackYou",
                zone,
                &condition,
            ));
            return Some(());
        }
        if !p.has("UnlessDefender") {
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `CantAttack.CheckSVar`")?;
            if valid == "Card.Self" && condition.is_empty() {
                return self.deny("`CantAttack` with no `UnlessDefender$`".to_string());
            }
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantAttack.{key}`"));
                return None;
            }
            let attackers = self.filter_expr(&valid)?;
            self.body.abilities.push(Self::format_static_expr(
                &attackers,
                "Modifier::AddKeyword(KeywordSet::CANT_ATTACK)",
                zone,
                &condition,
            ));
            return Some(());
        }
        let Some(unless) = p.take("UnlessDefender") else {
            return self.deny("`CantAttack` with no `UnlessDefender$`".to_string());
        };
        let (modifier, wanted) = if let Some(wanted) = unless.strip_prefix("controls") {
            ("CantAttackUnlessDefenderControls", wanted)
        } else if let Some(wanted) = unless.strip_prefix("!controls")
            && wanted.starts_with("Creature")
        {
            ("CantAttackIfDefenderControls", wanted)
        } else {
            return self.deny(format!("`UnlessDefender$ {unless}`"));
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `CantAttack.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `CantAttack.{key}`"));
            return None;
        }
        let attackers = self.filter_expr(&valid)?;
        let wanted = self.filter_expr(wanted)?;
        let name = self.body.filter_static("DEFENDER_CONTROLS", &wanted);
        self.body.abilities.push(Self::format_static_expr(
            &attackers,
            &format!("Modifier::{modifier}(&{name})"),
            zone,
            &condition,
        ));
        Some(())
    }

    /// `AddPower`/`AddToughness` (layer 7c) and `SetPower`/`SetToughness`
    /// (layer 7b) as static abilities.
    #[allow(clippy::too_many_lines)]
    pub(super) fn pt_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if p.peek("SetPower") == Some("AffectedX")
            && p.peek("SetToughness") == Some("AffectedX")
            && p.peek("AddType") == Some("Creature")
            && self.svars.get("AffectedX").map(String::as_str) == Some("Count$CardManaCost")
        {
            p.take("SetPower");
            p.take("SetToughness");
            p.take("AddType");
            out.push(Self::static_expr(
                filter,
                "Modifier::AnimateNoncreatureArtifact",
            ));
            return Some(());
        }
        let add_p = p.take("AddPower");
        let add_t = p.take("AddToughness");
        if add_p.is_some() || add_t.is_some() {
            let mut emitted_pt = false;
            if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && var_p.trim().parse::<i16>().is_err()
                && let Some(count) = self.svars.get(var_p.trim()).cloned()
            {
                let trimmed = count.trim();
                if let Some(counter) = trimmed.strip_prefix("Count$CardCounters.") {
                    let Some(kind) = super::counter_kind(counter) else {
                        self.note(format!("counter `{counter}`"));
                        return None;
                    };
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ModifyPTByCount(PtCount::CountersOnThis({kind}))"),
                    ));
                    emitted_pt = true;
                } else if let Some(valid) = trimmed.strip_prefix("Count$ValidGraveyard ")
                    && !valid.contains("Ctrl")
                    && !valid.contains("Own")
                    && !valid.contains('/')
                    && let Some(counted_filter) = self.filter_expr(valid)
                {
                    let name = self.body.filter_static("COUNTED", &counted_filter);
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ModifyPTByCount(PtCount::InAllGraveyards(&{name}))"),
                    ));
                    emitted_pt = true;
                } else if trimmed == "Count$ValidHand Card.YouOwn" {
                    out.push(Self::static_expr(
                        filter,
                        "Modifier::ModifyPTByAmount { power: Amount::HandSize(PlayerRel::You), \
                         toughness: Amount::HandSize(PlayerRel::You) }",
                    ));
                    emitted_pt = true;
                } else if let Some(color_valid) = trimmed
                    .strip_prefix("Count$Valid ")
                    .and_then(|v| v.strip_suffix("$Colors"))
                    && !color_valid.contains('/')
                    && !color_valid.contains('$')
                    && (color_valid.split(['.', '+']).any(|a| a == "YouCtrl")
                        || (!color_valid.contains("Ctrl") && !color_valid.contains("Own")))
                    && let Some(counted_filter) = self.filter_expr(color_valid)
                {
                    let name = self.body.filter_static("COUNTED", &counted_filter);
                    out.push(Self::static_expr(
                        filter,
                        &format!(
                            "Modifier::ModifyPTByCount(PtCount::DistinctColorsAmong(&{name}))"
                        ),
                    ));
                    emitted_pt = true;
                } else if let Some(raw_valid) = trimmed.strip_prefix("Count$Valid ")
                    && let Some((valid, times)) = parse_pt_multiplier(raw_valid)
                    && !valid.contains('/')
                    && !valid.contains('$')
                {
                    let yours = valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
                    let opp = valid.split(['.', '+']).any(|atom| atom == "OppCtrl");
                    if (yours || opp || (!valid.contains("Ctrl") && !valid.contains("Own")))
                        && let Some(counted_filter) = self.filter_expr(valid)
                    {
                        let name = self.body.filter_static("COUNTED", &counted_filter);
                        let kind = if yours { "YouControl" } else { "OnBattlefield" };
                        for _ in 0..times {
                            out.push(Self::static_expr(
                                filter,
                                &format!("Modifier::ModifyPTByCount(PtCount::{kind}(&{name}))"),
                            ));
                        }
                        emitted_pt = true;
                    }
                }
            }
            if !emitted_pt
                && let Some((var, power_half)) = match (add_p.as_deref(), add_t.as_deref()) {
                    (Some(vp), None | Some("0" | "+0")) if vp.trim().parse::<i16>().is_err() => {
                        Some((vp.trim(), true))
                    }
                    (None | Some("0" | "+0"), Some(vt)) if vt.trim().parse::<i16>().is_err() => {
                        Some((vt.trim(), false))
                    }
                    _ => None,
                }
                && let Some(count) = self.svars.get(var).cloned()
                && let Some(raw_valid) = count.trim().strip_prefix("Count$Valid ")
                && let Some((valid, times)) = parse_pt_multiplier(raw_valid)
                && !valid.contains('/')
                && !valid.contains('$')
            {
                let alternatives = valid.split(',').collect::<Vec<_>>();
                let scoped = alternatives
                    .iter()
                    .map(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"))
                    .collect::<Vec<_>>();
                let yours = scoped.iter().all(|scoped| *scoped);
                if scoped.iter().any(|scoped| *scoped) != yours {
                    self.note("static ability with mixed controller scopes".to_string());
                    return None;
                }
                let attached = valid.split(['.', '+']).any(|atom| atom == "Attached");
                let allow_other = !valid
                    .split(',')
                    .flat_map(|alt| alt.split(['.', '+']))
                    .any(|atom| matches!(atom.trim(), "Other" | "StrictlyOther"))
                    || filter == "Filter::This";
                if (yours || attached)
                    && allow_other
                    && (!attached || (!valid.contains("Ctrl") && !valid.contains("Own")))
                    && let Some(counted_filter) = {
                        let valid_without_you = alternatives
                            .iter()
                            .map(|alt| {
                                alt.split(['.', '+'])
                                    .filter(|atom| atom.trim() != "YouCtrl")
                                    .collect::<Vec<_>>()
                                    .join(".")
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        self.filter_expr(&valid_without_you)
                    }
                {
                    let counted_filter = if yours {
                        format!("Filter::And(&[{counted_filter}, Filter::ControlledByYou])")
                    } else {
                        counted_filter
                    };
                    let name = self.body.filter_static("COUNTED", &counted_filter);
                    let (p, t) = if power_half { (times, 0) } else { (0, times) };
                    out.push(Self::static_expr(
                        filter,
                        &format!(
                            "Modifier::ModifyPTPerCount {{ filter: &{name}, p: {p}, t: {t} }}"
                        ),
                    ));
                    emitted_pt = true;
                }
            }
            if !emitted_pt {
                if let (Some(power), Some(tough)) = (
                    add_p
                        .as_deref()
                        .map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
                    add_t
                        .as_deref()
                        .map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
                ) {
                    // An anthem that names only one half still moves the other by
                    // zero, which is what the printed "+1/+0" says.
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ModifyPT({power}, {tough})"),
                    ));
                } else if let (Some(power), Some(toughness)) = (
                    self.static_pt_amount(add_p.as_deref()),
                    self.static_pt_amount(add_t.as_deref()),
                ) {
                    out.push(Self::static_expr(
                        filter,
                        &format!(
                            "Modifier::ModifyPTByAmount {{ power: {power}, toughness: {toughness} }}"
                        ),
                    ));
                } else {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
            }
        }
        let set_p = p.take("SetPower");
        let set_t = p.take("SetToughness");
        if set_p.is_some() || set_t.is_some() {
            match (
                set_p.as_deref().map(|v| v.trim().parse::<i16>().ok()),
                set_t.as_deref().map(|v| v.trim().parse::<i16>().ok()),
            ) {
                (Some(Some(power)), Some(Some(tough))) => out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::SetPT({power}, {tough})"),
                )),
                (Some(Some(power)), None) => {
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::SetPower({power})"),
                    ));
                }
                (None, Some(Some(tough))) => out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::SetToughness({tough})"),
                )),
                _ => {
                    self.note("static ability setting one half of P/T".to_string());
                    return None;
                }
            }
        }
        Some(())
    }

    fn static_pt_amount(&mut self, raw: Option<&str>) -> Option<String> {
        let Some(raw) = raw else {
            return Some("Amount::Fixed(0)".to_string());
        };
        let trimmed = raw.trim();
        if let Ok(n) = trimmed.parse::<i16>() {
            return Some(if n >= 0 {
                format!("Amount::Fixed({n})")
            } else {
                format!("Amount::NegXFixed({})", n.unsigned_abs())
            });
        }
        let neg = trimmed.starts_with('-');
        let var = trimmed.trim_start_matches(['+', '-']);
        let body = self.svars.get(var)?.clone();
        let inner = self.static_count_expr(&body)?;
        Some(if neg {
            format!("Amount::Negated(&{inner})")
        } else {
            inner
        })
    }

    /// `AddKeyword`/`RemoveKeyword` (layer 6) as static abilities.
    pub(super) fn keyword_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        for (key, modifier) in [
            ("AddKeyword", "AddKeyword"),
            ("RemoveKeyword", "RemoveKeyword"),
        ] {
            let Some(raw) = p.take(key) else { continue };
            // A keyword the engine reads as a bit, or nothing: a keyword
            // that carries data ("Enchant creature", "Equip {2}") is an
            // ability, and granting it as a bit would grant a keyword no
            // rule reads.
            let mut bits = Vec::new();
            for word in raw.split(" & ") {
                // Protection is granted as the static ability it is (CR
                // 702.16a), never as a bit; removing it is a different
                // sentence and is refused.
                if modifier == "AddKeyword"
                    && let Some(from) = self.protection_filter(word)
                {
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ProtectionFrom(&{from})"),
                    ));
                    continue;
                }
                if modifier == "AddKeyword"
                    && let Some(rest) = word.trim().strip_prefix("Cumulative upkeep:")
                {
                    let payment = self.cumulative_upkeep_payment(rest)?;
                    out.push(Self::static_expr(
                        filter,
                        &format!(
                            "Modifier::GrantTriggered {{ \
                             trigger: Trigger::StepBegin {{ step: StepKind::Upkeep, whose: PlayerRel::You }}, \
                             effects: &[Effect::AddCounter {{ kind: counters::AGE, amount: Amount::Fixed(1) }}, {payment}], \
                             target: None }}"
                        ),
                    ));
                    continue;
                }
                if modifier == "AddKeyword"
                    && let Some(rest) = word.trim().strip_prefix("Ward:")
                {
                    let Some(effect) = self.ward_effect_expr(rest, true) else {
                        self.note(format!("static ability granting keyword `{word}`"));
                        return None;
                    };
                    out.push(Self::static_expr(
                        filter,
                        &format!(
                            "Modifier::GrantTriggered {{ trigger: Trigger::Ward, \
                             effects: &[{effect}], target: None }}"
                        ),
                    ));
                    continue;
                }
                let Some(bit) = keyword_const(word) else {
                    self.note(format!("static ability granting keyword `{word}`"));
                    return None;
                };
                bits.push(bit.to_string());
            }
            let Some(set) = bits.split_first().map(|(head, tail)| {
                tail.iter()
                    .fold(head.clone(), |acc, b| format!("{acc}.union({b})"))
            }) else {
                continue;
            };
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::{modifier}({set})"),
            ));
        }
        Some(())
    }

    /// "Can block an additional creature each combat" (Two-Headed Giant of
    /// Foriys, `CanBlockAmount$ 1`) and "all creatures able to block
    /// enchanted creature do so" (Lure, a hidden keyword): rules of the
    /// declaration of blockers (CR 509.1a, 509.1c), as
    /// `Modifier::CanBlockAdditional` and `Modifier::MustBeBlockedByAllAble`.
    /// An amount that is not a number, and every other hidden keyword, stay
    /// unclaimed.
    pub(super) fn block_modifiers(
        &self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        const LURED: &str = "All creatures able to block CARDNAME do so.";
        if let Some(amount) = p.peek("CanBlockAmount") {
            let Ok(more) = amount.trim().parse::<u8>() else {
                return self.deny(format!("`CanBlockAmount$ {amount}`"));
            };
            p.take("CanBlockAmount");
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::CanBlockAdditional({more})"),
            ));
        }
        if let Some(any) = p.take("CanBlockAny") {
            if any.trim() != "True" {
                return self.deny(format!("`CanBlockAny$ {any}`"));
            }
            out.push(Self::static_expr(filter, "Modifier::CanBlockAnyNumber"));
        }
        if p.peek("AddHiddenKeyword").map(str::trim) == Some(LURED) {
            p.take("AddHiddenKeyword");
            out.push(Self::static_expr(
                filter,
                "Modifier::MustBeBlockedByAllAble",
            ));
        }
        Some(())
    }

    /// `AddType`/`RemoveType` (layer 4) as static abilities.
    ///
    /// The corpus writes card types and subtypes in one list and the engine
    /// keeps them apart — card types and supertypes have independent
    /// bitmasks, a subtype is an interned id — so a mixed list becomes
    /// separate modifiers on the same layer.
    ///
    /// `RemoveLandTypes$ True` beside an `AddType$` of one basic land type
    /// is "enchanted land is a Swamp" (Evil Presence, Conversion): CR 305.7's
    /// setting of a land's subtype, `Modifier::SetLandType`, which also takes
    /// the abilities the land's rules text gives it. `AddType$ ChosenType` is
    /// "enchanted land is the chosen type" (Phantasmal Terrain),
    /// `Modifier::SetLandTypeToChosen`, read only on a card that asks for a
    /// basic land type as it enters. Any other `AddType$` beside it — two
    /// types, a nonbasic one, a type chosen some other way — is refused.
    pub(super) fn type_modifiers(
        &self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        match p.take("RemoveLandTypes").as_deref() {
            None => {}
            Some("True") => {
                let raw = p.take("AddType")?;
                let modifier = if raw.trim() == "ChosenType" {
                    self.body
                        .enter_modifiers
                        .iter()
                        .any(|m| m == "EnterModifier::ChooseBasicLandType")
                        .then(|| "Modifier::SetLandTypeToChosen".to_string())?
                } else {
                    let path = self.basic_land_type(raw.trim())?;
                    format!("Modifier::SetLandType({path})")
                };
                out.push(Self::static_expr(filter, &modifier));
            }
            Some(_) => return None,
        }
        for (key, modifier) in [("AddType", "AddType"), ("RemoveType", "RemoveType")] {
            let Some(raw) = p.take(key) else { continue };
            if raw == "ChosenType" && modifier == "AddType" {
                if filter != "Filter::This" {
                    return None;
                }
                let chosen_mod = if self
                    .body
                    .enter_modifiers
                    .iter()
                    .any(|m| m == "EnterModifier::ChooseBasicLandType")
                {
                    "Modifier::AddChosenBasicLandType"
                } else if self
                    .body
                    .enter_modifiers
                    .iter()
                    .any(|m| m == "EnterModifier::ChooseSubtype")
                {
                    "Modifier::AddChosenSubtype"
                } else {
                    return None;
                };
                out.push(Self::static_expr(filter, chosen_mod));
                continue;
            }
            let mut types = Vec::new();
            let mut subtypes = Vec::new();
            for word in raw.split_whitespace().filter(|&w| w != "&") {
                if let Some(t) = card_type_const(word) {
                    types.push(t);
                } else if let Some(supertype) = supertype_const(word) {
                    let modifier = if modifier == "AddType" {
                        "AddSupertype"
                    } else {
                        "RemoveSupertype"
                    };
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::{modifier}({supertype})"),
                    ));
                } else if baylee_core::types::SupertypeSet::from_word(word).is_some() {
                    return self.deny(format!("`{key}$` changing unsupported supertype `{word}`"));
                } else if modifier == "AddType" {
                    subtypes.push(self.cats.const_path(word)?);
                } else {
                    // `Modifier::RemoveType` takes a `TypeSet`, and there is
                    // no "remove one subtype" — refuse rather than drop it.
                    return None;
                }
            }
            if let Some((head, tail)) = types.split_first() {
                let set = tail
                    .iter()
                    .fold((*head).to_string(), |acc, t| format!("{acc}.union({t})"));
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::{modifier}({set})"),
                ));
            }
            for path in subtypes {
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::AddSubtype({path})"),
                ));
            }
        }
        Some(())
    }

    /// The constant of one basic land type (CR 205.3i names the five), or
    /// `None` for any other word.
    pub(super) fn basic_land_type(&self, word: &str) -> Option<String> {
        if !matches!(word, "Plains" | "Island" | "Swamp" | "Mountain" | "Forest") {
            return None;
        }
        self.cats
            .const_path_of(baylee_core::types::SubtypeKind::Land, word)
    }

    /// `AddColor`/`SetColor` (layer 5) as static abilities.
    /// `S: Mode$ Continuous | AddAbility$` and `AddTrigger$`:
    ///
    /// - Plain activated abilities (no target, second target, timing,
    ///   inner condition, or turn limit) emit `Modifier::GrantActivated`.
    /// - Complete activated abilities carrying targets, `second_targets`,
    ///   `SorcerySpeed$`, inner conditions, or `ActivationLimit$` hoist a
    ///   `static GRANTED_ACTIVATED_<N>: AbilityDef` via `body.ability_static`
    ///   and emit `Modifier::GrantActivatedDefinition(&GRANTED_ACTIVATED_<N>)`
    ///   (preserving the direct `activated!(...)` self-grant shortcut when
    ///   `filter == "Filter::This"` with a single plain target).
    /// - Granted triggered abilities (`AddTrigger$`) hoist a `static
    ///   GRANTED_TRIGGER_<N>: AbilityDef` via `Tx::triggered` (strictly in
    ///   `TriggerZone::Battlefield`) and emit
    ///   `Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_<N>)`.
    pub(super) fn grant_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        allow_self_shortcut: bool,
        condition: &str,
        out: &mut Vec<String>,
        self_granted: &mut bool,
    ) -> Option<()> {
        let abilities = p.take("AddAbility");
        let triggers = p.take("AddTrigger");
        let statics = p.take("AddStaticAbility");
        if abilities.is_none() && triggers.is_none() && statics.is_none() {
            return Some(());
        }
        if let Some(svars_raw) = p.take("AddSVar") {
            for name in svars_raw
                .split('&')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if !self.svars.contains_key(name) {
                    return self.deny(format!("`AddSVar$ {name}` naming no `SVar`"));
                }
            }
        }
        if let Some(raw) = abilities {
            self.grant_activated_list(
                &raw,
                filter,
                allow_self_shortcut,
                condition,
                out,
                self_granted,
            )?;
        }
        if let Some(raw) = triggers {
            self.grant_triggered_list(&raw, filter, out)?;
        }
        if let Some(raw) = statics {
            self.grant_static_list(&raw, filter, out)?;
        }
        Some(())
    }

    fn grant_activated_list(
        &mut self,
        raw: &str,
        filter: &str,
        allow_self_shortcut: bool,
        condition: &str,
        out: &mut Vec<String>,
        self_granted: &mut bool,
    ) -> Option<()> {
        for name in raw.split(" & ").map(str::trim).filter(|s| !s.is_empty()) {
            match self.build_grant_activated_modifier(
                name,
                "AddAbility$",
                allow_self_shortcut && filter == "Filter::This",
            )? {
                GrantedActivated::Modifier(modifier) => {
                    out.push(Self::static_expr(filter, &modifier));
                }
                GrantedActivated::SelfTarget {
                    cost,
                    effects,
                    target,
                } => {
                    self.body.abilities.push(format!(
                        "activated!({cost}, {effects}, target = Some({target}){condition})"
                    ));
                    *self_granted = true;
                }
            }
        }
        Some(())
    }

    pub(super) fn build_grant_activated_modifier(
        &mut self,
        name: &str,
        source_key: &str,
        for_static_this: bool,
    ) -> Option<GrantedActivated> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`{source_key} {name}` naming no `SVar`"));
        };
        if !line.trim_start().starts_with("AB$") {
            return self.deny("a granted ability that is not an activated one".to_string());
        }
        let parsed = self.parse_activated_spec(&line, false, false)?;
        if parsed.non_battlefield_zone {
            return self.deny("a granted ability activated outside the battlefield".to_string());
        }
        if parsed.extras.is_empty() {
            return Some(GrantedActivated::Modifier(format!(
                "Modifier::GrantActivated {{ cost: {}, effects: {}, mana_ability: {} }}",
                parsed.cost, parsed.effects, parsed.mana_ability
            )));
        }
        if for_static_this && let Some(target) = parsed.simple_target {
            return Some(GrantedActivated::SelfTarget {
                cost: parsed.cost,
                effects: parsed.effects,
                target,
            });
        }
        let def_name = self
            .body
            .ability_static("GRANTED_ACTIVATED_", &parsed.ability_expr);
        Some(GrantedActivated::Modifier(format!(
            "Modifier::GrantActivatedDefinition(&{def_name})"
        )))
    }

    fn grant_triggered_list(
        &mut self,
        raw: &str,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        for name in raw.split(" & ").map(str::trim).filter(|s| !s.is_empty()) {
            let modifier = self.build_grant_triggered_modifier(name, "AddTrigger$")?;
            out.push(Self::static_expr(filter, &modifier));
        }
        Some(())
    }

    pub(super) fn build_grant_triggered_modifier(
        &mut self,
        name: &str,
        source_key: &str,
    ) -> Option<String> {
        let Some(line) = self.svars.get(name).cloned() else {
            return self.deny(format!("`{source_key} {name}` naming no `SVar`"));
        };
        if !line.trim_start().starts_with("Mode$") {
            return self.deny("a granted trigger without `Mode$`".to_string());
        }
        let prev_abilities_len = self.body.abilities.len();
        let prev_statics_len = self.body.statics.len();
        let prev_trigger_mode = self.trigger_mode.take();
        let prev_block_line = self.block_line.take();
        let ok = self.triggered(&line);
        self.trigger_mode = prev_trigger_mode;
        if ok.is_none()
            || self.block_line.is_some()
            || self.body.abilities.len() != prev_abilities_len + 1
        {
            self.body.abilities.truncate(prev_abilities_len);
            self.body.statics.truncate(prev_statics_len);
            self.block_line = prev_block_line;
            if ok.is_some() {
                return self.deny("a granted trigger that is not a single ability".to_string());
            }
            return None;
        }
        self.block_line = prev_block_line;
        let ability_expr = self.body.abilities.pop()?;
        if (!ability_expr.starts_with("triggered!(")
            && !ability_expr.starts_with("modal_triggered!("))
            || ability_expr.contains("zone = TriggerZone::")
        {
            self.body.statics.truncate(prev_statics_len);
            return self.deny("a granted trigger outside the battlefield".to_string());
        }
        let def_name = self.body.ability_static("GRANTED_TRIGGER_", &ability_expr);
        Some(format!("Modifier::GrantTriggeredDefinition(&{def_name})"))
    }

    fn grant_static_list(&mut self, raw: &str, filter: &str, out: &mut Vec<String>) -> Option<()> {
        for name in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("`AddStaticAbility$ {name}` naming no `SVar`"));
            };
            if !line.trim_start().starts_with("Mode$") {
                return self.deny("a granted static without `Mode$`".to_string());
            }
            let prev_abilities_len = self.body.abilities.len();
            let prev_statics_len = self.body.statics.len();
            let prev_block_line = self.block_line.take();
            let ok = self.static_ability(&line);
            let block_seen = self.block_line.is_some();
            self.block_line = prev_block_line;
            if ok.is_none() || block_seen || self.body.abilities.len() <= prev_abilities_len {
                self.body.abilities.truncate(prev_abilities_len);
                self.body.statics.truncate(prev_statics_len);
                if ok.is_some() {
                    return self.deny("a granted static that is not a static ability".to_string());
                }
                return None;
            }
            let emitted = self.body.abilities.split_off(prev_abilities_len);
            for expr in emitted {
                let Some((inner_filter, inner_modifier)) = split_plain_static_ability(&expr) else {
                    self.body.statics.truncate(prev_statics_len);
                    return self.deny(
                        "a granted static with a zone, condition, or replacement rule".to_string(),
                    );
                };
                if inner_modifier.contains("Modifier::GrantStatic") {
                    self.body.statics.truncate(prev_statics_len);
                    return self.deny("a nested granted static ability".to_string());
                }
                let inner_filter_ref = self.body.filter_static("GRANTED_STATIC", inner_filter);
                let modifier = format!(
                    "Modifier::GrantStatic {{ filter: &{inner_filter_ref}, modifier: &{inner_modifier} }}"
                );
                out.push(Self::static_expr(filter, &modifier));
            }
        }
        Some(())
    }

    fn cant_phase_out(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "CantPhaseOut")?;
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CantPhaseOut` with no `ValidCard$`".to_string());
        };
        let filter = self.filter_expr(&valid)?;
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `CantPhaseOut.CheckSVar`")?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantPhaseOut.{key}`"));
            }
            return None;
        }
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::CantPhaseOut",
            zone,
            &condition,
        ));
        Some(())
    }

    /// `R: Event$ RollDice | ActiveZones$ Battlefield | ValidPlayer$ You`
    /// chaining `Plus.1` on `Number` and `Ignore` (`Modifier::RollAdditionalDieIgnoreLowest`, CR 706.6).
    pub(super) fn roll_dice_replacement(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("ActiveZones").as_deref(), None | Some("Battlefield"))
            || p.take("ValidPlayer").as_deref() != Some("You")
        {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        }
        let Some(replace_with) = p.take("ReplaceWith") else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        if !p.exhausted() {
            return self.deny(format!(
                "unclaimed parameter `RollDice.{}`",
                p.first_key().unwrap_or_default()
            ));
        }
        let Some(first_body) = self.svars.get(replace_with.trim()).cloned() else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        let Some((api1, mut p1)) = Params::parse(&first_body) else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        p1.drop_prose();
        if api1 != "ReplaceEffect"
            || p1.take("VarName").as_deref() != Some("Number")
            || p1.take("VarValue").as_deref() != Some("ReplaceCount$Number/Plus.1")
        {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        }
        let Some(sub) = p1.take("SubAbility") else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        if !p1.exhausted() {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        }
        let Some(second_body) = self.svars.get(sub.trim()).cloned() else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        let Some((api2, mut p2)) = Params::parse(&second_body) else {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        };
        p2.drop_prose();
        if api2 != "ReplaceEffect"
            || p2.take("VarName").as_deref() != Some("Ignore")
            || p2.take("VarValue").as_deref() != Some("ReplaceCount$Ignore/Plus.1")
            || !p2.exhausted()
        {
            return self.deny("replacement `R: Event$ RollDice`".to_string());
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::Any",
            "Modifier::RollAdditionalDieIgnoreLowest",
        ));
        Some(())
    }

    pub(super) fn color_modifiers(
        &self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        for (key, modifier) in [("AddColor", "AddColor"), ("SetColor", "SetColor")] {
            let Some(raw) = p.take(key) else { continue };
            let mut colors = Vec::new();
            for word in raw.split_whitespace().filter(|&w| w != "&") {
                colors.push(match word {
                    "White" => "Color::White",
                    "Blue" => "Color::Blue",
                    "Black" => "Color::Black",
                    "Red" => "Color::Red",
                    "Green" => "Color::Green",
                    // `Colorless` is the empty set rather than a colour, and
                    // `ChosenColor` is a choice this rule cannot make.
                    other => {
                        self.note(format!("static ability setting colour `{other}`"));
                        return None;
                    }
                });
            }
            out.push(Self::static_expr(
                filter,
                &format!(
                    "Modifier::{modifier}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
            ));
        }
        Some(())
    }

    /// `S: Mode$ ReduceCost`: battlefield and self-stack (`ZoneRef::Stack`)
    /// spell cost reductions (fixed generic, dynamic `Amount`, fixed mana
    /// `Color$` / `IgnoreGeneric$`, face-down, shared creature type), plus
    /// `CyclingCostsLess` (`ValidSpell$ Activated.Cycling`).
    fn mode_reduce_cost(&mut self, mut p: Params) -> Option<()> {
        // Keep `final_portals::reduce_cost` handling plain battlefield fixed
        // generic spell reductions unchanged.
        if !p.has("Condition")
            && !p.has("IsPresent")
            && !p.has("CheckSVar")
            && !p.has("PlayerTurn")
            && !p.has("Color")
            && !p.has("IgnoreGeneric")
            && !p.has("ValidSpell")
            && !p.has("ValidTarget")
            && matches!(p.peek("EffectZone"), None | Some("Battlefield"))
            && p.peek("Type") == Some("Spell")
            && matches!(p.peek("Activator"), None | Some("You" | "Opponent"))
            && p.peek("Amount").is_some_and(|a| a.parse::<u8>().is_ok())
            && p.peek("ValidCard").is_some_and(|vc| {
                !vc.contains("sharesCreatureTypeWith") && !vc.contains("faceDown")
            })
        {
            return self.reduce_cost(p);
        }

        p.drop_prose();
        if p.peek("ValidSpell") == Some("Activated.Cycling") {
            return self.reduce_cost_cycling(p);
        }
        if p.peek("ValidSpell") == Some("Activated.Equip") {
            return self.reduce_cost_equip(p);
        }
        if p.peek("ValidSpell") == Some("Spell.Buyback") {
            return self.reduce_cost_buyback(p);
        }

        let spell_type = p.take("Type");
        let valid_spell = p.take("ValidSpell");
        if spell_type.as_deref() == Some("Ability") && valid_spell.is_none() {
            return self.reduce_cost_ability(p);
        }
        let face_down = match (spell_type.as_deref(), valid_spell.as_deref()) {
            (Some("Spell"), None) => false,
            (Some("Spell") | None, Some("Spell.isCastFaceDown")) => true,
            _ => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let (is_self_zone, zone) = match p.take("EffectZone").as_deref() {
            Some("All" | "Stack") => (true, StaticEffectZone::Explicit("ZoneRef::Stack")),
            raw => match Self::static_effect_zone(raw, false) {
                Some(z) => (false, z),
                None => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            },
        };
        let target_filter = if p.has("ValidTarget") {
            Some(self.reduce_cost_target_filter(&mut p, is_self_zone, face_down, zone)?)
        } else {
            None
        };
        let mut valid_card = match p.take("ValidCard") {
            Some(vc) => vc,
            None if spell_type.as_deref() == Some("Spell")
                && valid_spell.is_none()
                && !is_self_zone =>
            {
                "Card".to_string()
            }
            None => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        if is_self_zone {
            if valid_card != "Card.Self" {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            if !matches!(p.take("Activator").as_deref(), None | Some("You")) {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            if face_down {
                valid_card = "Card.Self+faceDown".to_string();
            }
        } else {
            if valid_card == "Card.Self" {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            if face_down {
                valid_card = Self::append_reduce_atom(&valid_card, "faceDown");
            }
            let Some(with_act) =
                Self::apply_reduce_activator(&valid_card, p.take("Activator").as_deref())
            else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            valid_card = with_act;
        }
        let cond = self.reduce_cost_condition(&mut p)?;
        let modifier = self.reduce_cost_modifier(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let base_filter = self.filter_expr(&valid_card)?;
        let filter = match target_filter {
            None => base_filter,
            Some(tf) if base_filter == "Filter::Any" => tf.to_string(),
            Some(tf) => format!("Filter::And(&[{base_filter}, {tf}])"),
        };
        self.body
            .abilities
            .push(Self::format_static_expr(&filter, &modifier, zone, &cond));
        Some(())
    }

    fn reduce_cost_target_filter(
        &mut self,
        p: &mut Params,
        is_self_zone: bool,
        face_down: bool,
        zone: StaticEffectZone,
    ) -> Option<&'static str> {
        let raw_target = p.peek("ValidTarget")?;
        if is_self_zone
            || face_down
            || p.has("Color")
            || !matches!(zone, StaticEffectZone::Battlefield)
        {
            return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
        }
        let tf = match raw_target {
            "Card.Self" | "Creature.Self" | "Permanent.Self" => "Filter::SpellTargetingAffected",
            "Permanent.YouCtrl"
            | "Permanent.YouCtrl+inZoneBattlefield"
            | "Card.YouCtrl+inZoneBattlefield" => "Filter::TargetsPermanentYouControl",
            _ => return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into()),
        };
        p.take("ValidTarget");
        Some(tf)
    }

    fn reduce_cost_modifier(&mut self, p: &mut Params) -> Option<String> {
        if let Some(color_raw) = p.take("Color") {
            if p.take("Amount").as_deref() != Some("1") {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let colored_only = match p.take("IgnoreGeneric").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            };
            let Some(mana) = super::printed_mana(&color_raw) else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            return Some(format!(
                "Modifier::SpellsCostLessMana {{ cost: mana!(\"{mana}\"), colored_only: {colored_only} }}"
            ));
        }
        let Some(amt_raw) = p.take("Amount") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if let Ok(n) = amt_raw.parse::<u32>() {
            if n == 0 {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            return Some(format!("Modifier::SpellsCostLess({n})"));
        }
        let amt_def = if amt_raw.starts_with("Count$") {
            amt_raw
        } else {
            let Some(def) = self.svars.get(amt_raw.trim()).cloned() else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            def
        };
        let amt_expr = self.static_count_expr(&amt_def)?;
        Some(format!("Modifier::SpellsCostLessByAmount({amt_expr})"))
    }

    fn reduce_cost_cycling(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        let (filter, amount, _, zone, cond) =
            self.parse_ability_reduction(p, false, false, false)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::CyclingCostsLess({amount})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_equip(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        let (filter, amount, _, zone, cond) =
            self.parse_ability_reduction(p, true, false, false)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::EquipCostsLess({amount})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_buyback(&mut self, mut p: Params) -> Option<()> {
        p.take("ValidSpell");
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if !matches!(p.take("Type").as_deref(), None | Some("Spell")) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        let Some(valid_card) =
            Self::apply_reduce_activator(&raw_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.parse::<u32>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let filter = self.filter_expr(&valid_card)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::BuybackCostsLess({amount})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_ability(&mut self, p: Params) -> Option<()> {
        let (filter, amount, minimum, zone, cond) =
            self.parse_ability_reduction(p, true, true, true)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::AbilitiesCostLess {{ amount: {amount}, minimum: {minimum} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn parse_ability_reduction(
        &mut self,
        mut p: Params,
        allow_affected_zone: bool,
        allow_nonzero_min_mana: bool,
        forbid_unscoped_card: bool,
    ) -> Option<(String, u32, u32, StaticEffectZone, String)> {
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let mut zones_ok = true;
        let explicit_battlefield = if allow_affected_zone {
            let affected_zone = p.take("AffectedZone");
            zones_ok &= matches!(affected_zone.as_deref(), None | Some("Battlefield"));
            affected_zone.as_deref() == Some("Battlefield")
        } else {
            false
        };
        if !zones_ok || !matches!(p.take("Type").as_deref(), None | Some("Ability")) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if forbid_unscoped_card && raw_card == "Card" && !explicit_battlefield {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(valid_card) =
            Self::apply_reduce_activator(&raw_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.parse::<u32>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let minimum = match (
            allow_nonzero_min_mana,
            allow_affected_zone,
            p.take("MinMana"),
        ) {
            (_, _, None) => 0,
            (true, _, Some(min)) => match min.parse::<u32>() {
                Ok(n) => n,
                Err(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            },
            (false, true, Some(min)) if min == "0" => 0,
            (false, _, Some(_)) => {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let filter = self.filter_expr(&valid_card)?;
        Some((filter, amount, minimum, zone, cond))
    }

    fn append_reduce_atom(valid: &str, atom: &str) -> String {
        valid
            .split(',')
            .map(|alt| {
                let alt = alt.trim();
                if alt.split(['.', '+']).any(|part| part == atom) {
                    alt.to_string()
                } else if alt.contains('.') {
                    format!("{alt}+{atom}")
                } else {
                    format!("{alt}.{atom}")
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn apply_reduce_activator(valid: &str, activator: Option<&str>) -> Option<String> {
        match activator {
            None | Some("Player") => Some(valid.to_string()),
            Some("You") => Some(Self::append_reduce_atom(valid, "YouCtrl")),
            Some("Opponent" | "Player.Opponent") => {
                Some(Self::append_reduce_atom(valid, "OppCtrl"))
            }
            Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                Some(Self::append_reduce_atom(valid, "EnchantedPlayerCtrl"))
            }
            Some(_) => None,
        }
    }

    fn reduce_cost_condition(&mut self, p: &mut Params) -> Option<String> {
        let mut clauses = Vec::new();
        if let Some(pt) = p.take("PlayerTurn") {
            if !matches!(pt.as_str(), "True" | "You") {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            clauses.push("Condition::YourTurn".to_string());
        }
        if let Some(cond_raw) = p.take("Condition") {
            let Some(cond) = Self::named_condition(&cond_raw) else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            clauses.push(cond.to_string());
        }
        if p.peek("LifeTotal") == Some("You") {
            p.take("LifeTotal");
            let life_amount = p.take("LifeAmount").unwrap_or_else(|| "GE1".to_string());
            clauses.push(match count_bound(&life_amount) {
                Some(Bound::AtLeast(n)) => {
                    format!("Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n})")
                }
                Some(Bound::AtMost(n)) => format!("Condition::LifeAtMost({n})"),
                _ => return self.deny(format!("`LifeAmount$ {life_amount}`")),
            });
        }
        if let Some(valid) = p.take("IsPresent") {
            let clause = if matches!(p.peek("PresentZone"), Some("Hand" | "Graveyard"))
                && matches!(p.peek("PresentPlayer"), None | Some("You"))
                && !matches!(valid.as_str(), "Card.StrictlySelf" | "Card.Self")
            {
                self.reduce_cost_zone_present(p, &valid)?
            } else {
                self.present_clause(p, &valid)?
            };
            clauses.push(clause);
        }
        if let Some(var) = p.take("CheckSVar") {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            let Some(clause) = self.reduce_cost_svar_condition(&var, &cmp) else {
                return self.deny("unclaimed parameter `ReduceCost.CheckSVar`".to_string());
            };
            clauses.push(clause);
        }
        Some(match clauses.as_slice() {
            [] => String::new(),
            [one] => format!(", condition = Some({one})"),
            many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
        })
    }

    fn reduce_cost_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        if let Some(c) = self.svar_condition_clause(var, cmp) {
            return Some(c);
        }
        let trimmed = var.trim();
        if let Some(c) = self.inline_valid_count_condition(trimmed, cmp) {
            return Some(c);
        }
        let def = if trimmed.starts_with("Count$") || trimmed.starts_with("PlayerCount") {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.trim().to_string()
        };
        let bound = super::count_bound(cmp)?;
        if def == "PlayerCountOpponents$HighestCardsInGraveyard" {
            return match bound {
                super::Bound::AtLeast(n) if n >= 1 => u8::try_from(n)
                    .ok()
                    .map(|n| format!("Condition::OpponentGraveyardCountAtLeast({n})")),
                super::Bound::AtMost(n) => {
                    n.checked_add(1)
                        .and_then(|t| u8::try_from(t).ok())
                        .map(|t| {
                            format!(
                                "Condition::Not(&Condition::OpponentGraveyardCountAtLeast({t}))"
                            )
                        })
                }
                super::Bound::AtLeast(_) => None,
            };
        }
        if def
            .strip_prefix("Count$ValidGraveyard ")
            .is_some_and(|rest| !rest.contains('$') && !rest.contains('/'))
            && let Some(count) = self.static_count_expr(&def)
        {
            return match bound {
                super::Bound::AtLeast(n) if n >= 1 => {
                    Some(format!("Condition::AmountAtLeast({count}, {n})"))
                }
                super::Bound::AtMost(n) => {
                    let threshold = u32::from(n).saturating_add(1);
                    Some(format!(
                        "Condition::Not(&Condition::AmountAtLeast({count}, {threshold}))"
                    ))
                }
                super::Bound::AtLeast(_) => None,
            };
        }
        None
    }

    fn reduce_cost_zone_present(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        if p.take("NoResolvingCheck").is_some() {
            return self.deny("`NoResolvingCheck$`, a clause checked once".to_string());
        }
        if p.take("IsPresent2").is_some() {
            return self.deny("a second `IsPresent2$` clause".to_string());
        }
        if let Some(defined) = p.take("PresentDefined") {
            return self.deny(format!("`PresentDefined$ {defined}`"));
        }
        let zone = p.take("PresentZone")?;
        let who = p.take("PresentPlayer");
        let explicit_you = valid.split(',').all(|alt| {
            alt.split(['.', '+'])
                .any(|atom| matches!(atom.trim(), "YouCtrl" | "YouOwn"))
        });
        let bad_atom = valid.split(',').any(|alt| {
            alt.split(['.', '+']).any(|a| {
                let a = a.trim();
                matches!(
                    a,
                    "Self"
                        | "Other"
                        | "StrictlyOther"
                        | "EnchantedBy"
                        | "EquippedBy"
                        | "AttachedBy"
                        | "Attached"
                ) || ((a.contains("Ctrl") || a.contains("Own"))
                    && !matches!(a, "YouCtrl" | "YouOwn"))
            })
        });
        if bad_atom || (who.is_none() && !explicit_you) {
            return self.deny(format!("`IsPresent$ {valid}` in {zone}"));
        }
        let stripped = valid
            .split(',')
            .map(|alt| {
                let mut parts = alt.split('.');
                let base = parts.next().unwrap_or_default().trim();
                let rest: Vec<&str> = parts
                    .flat_map(|s| s.split('+'))
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && !matches!(*a, "YouCtrl" | "YouOwn"))
                    .collect();
                if rest.is_empty() {
                    base.to_string()
                } else {
                    format!("{base}.{}", rest.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let compare = match p.take("PresentCompare") {
            Some(c) => c,
            None => self.default_zone_present_compare(&zone, &stripped)?,
        };
        let Some(bound) = count_bound(&compare) else {
            return self.deny(format!("`PresentCompare$ {compare}`"));
        };
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let name = self.body.filter_static("CHECK", &expr);
        let zone_sel = if zone == "Hand" {
            "ZoneSel::HandYou"
        } else {
            "ZoneSel::GraveyardOf(PlayerRel::You)"
        };
        let count = format!("Amount::CountOf {{ filter: &{name}, zone: {zone_sel} }}");
        Some(match bound {
            Bound::AtLeast(n) if n >= 1 => format!("Condition::AmountAtLeast({count}, {n})"),
            Bound::AtMost(n) => {
                let threshold = u32::from(n).saturating_add(1);
                format!("Condition::Not(&Condition::AmountAtLeast({count}, {threshold}))")
            }
            Bound::AtLeast(_) => return self.deny(format!("`PresentCompare$ {compare}`")),
        })
    }

    fn default_zone_present_compare(&self, zone: &str, stripped: &str) -> Option<String> {
        if zone == "Graveyard" && stripped == "Permanent" {
            let lower = self.oracle.to_ascii_lowercase();
            let has_d4 = lower.contains("descend 4");
            let has_d8 = lower.contains("descend 8");
            if has_d8 && !has_d4 {
                return Some("GE8".to_string());
            }
            if has_d4 && !has_d8 {
                return Some("GE4".to_string());
            }
            if has_d4
                || has_d8
                || lower.contains("descend ")
                || lower.contains("or more permanent cards in your graveyard")
            {
                return self.deny(
                    "omitted `PresentCompare$` on multi-card Descend graveyard condition"
                        .to_string(),
                );
            }
        }
        Some("GE1".to_string())
    }

    pub(super) fn inline_valid_count_condition(&mut self, def: &str, cmp: &str) -> Option<String> {
        let valid = def.strip_prefix("Count$Valid ")?;
        if valid.is_empty() || valid.contains('/') || valid.contains('$') {
            return None;
        }
        let bound = count_bound(cmp)?;
        let relative = valid.split(',').any(|alt| {
            alt.split(['.', '+']).any(|a| {
                matches!(
                    a.trim(),
                    "Self"
                        | "Other"
                        | "StrictlyOther"
                        | "EnchantedBy"
                        | "EquippedBy"
                        | "AttachedBy"
                        | "Attached"
                )
            })
        });
        if relative {
            return None;
        }
        let yours = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
        let opps = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"))
            && matches!(bound, Bound::AtMost(0));
        let nobodys = !valid
            .split(',')
            .flat_map(|alt| alt.split(['.', '+']))
            .any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
        if !yours && !opps && !nobodys {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("CHECK", &expr);
        let (variant, n) = match (yours, bound) {
            (true, Bound::AtLeast(n)) => ("ControlCount", n),
            (true, Bound::AtMost(n)) => ("ControlCountAtMost", n),
            (false, Bound::AtLeast(n)) => ("BattlefieldCount", n),
            (false, Bound::AtMost(n)) => ("BattlefieldCountAtMost", n),
        };
        Some(format!("Condition::{variant}(&{name}, {n})"))
    }

    /// One `AbilityDef::Static` expression.
    ///
    /// `static_ability!` takes no layer for the same reason [`Self::animate_expr`]
    /// passes none: CR 613.1 makes the layer a function of the modifier, and
    /// `Modifier::layer` is that function.
    pub(super) fn static_expr(filter: &str, modifier: &str) -> String {
        format!("static_ability!({filter}, {modifier})")
    }

    /// `S: Mode$ OptionalAttackCost | ValidCard$ Card.Self | Cost$ Exert<1/CARDNAME>`
    /// (with optional reflexive `Trigger$ <DB$>`, CR 701.43a–c, CR 603.12).
    fn optional_attack_cost(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(
                p.take("ValidCard").as_deref(),
                Some("Card.Self" | "Creature.Self")
            )
        {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        }
        let Some(cost) = p.take("Cost") else {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        };
        if !is_exert_self_cost(&cost) {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        }
        let trigger_svar = p.take("Trigger");
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        }
        let Some(svar_name) = trigger_svar else {
            self.body.abilities.push(
                "triggered!(Trigger::Attacks(&Filter::This), \
                 &[Effect::MayDo { effects: &[Effect::ExertSource] }])"
                    .to_string(),
            );
            return Some(());
        };
        let Some(body) = self.svars.get(svar_name.trim()).cloned() else {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        };
        if !body.trim_start().starts_with("DB$") {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        }
        let prev_mode = self.trigger_mode.take();
        let prev_self = std::mem::replace(&mut self.trigger_is_self, false);
        let prev_spell = std::mem::replace(&mut self.on_a_spell, false);
        let mut chain = Chain::default();
        let ok = self.chain(&body, &mut chain);
        self.trigger_mode = prev_mode;
        self.trigger_is_self = prev_self;
        self.on_a_spell = prev_spell;
        ok?;
        if chain.effects.is_empty()
            || chain.second_target.is_some()
            || chain.target_is_x
            || chain.target_up_to.is_some()
            || matches!(chain.target_exact, Some(n) if n != 1)
        {
            return self.deny("static ability `S: Mode$ OptionalAttackCost`".to_string());
        }
        let target = chain
            .target
            .as_deref()
            .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
        let effects = chain.effects.join(", ");
        self.body.abilities.push(format!(
            "triggered!(Trigger::Attacks(&Filter::This), \
             &[Effect::MayDo {{ effects: &[Effect::ExertSource] }}, \
             Effect::Reflexive {{ when: ReflexiveEvent::ExertedThis, \
             effects: &[{effects}], target: {target} }}])"
        ));
        Some(())
    }

    /// `S: Mode$ IgnoreLandWalk` without `ValidBlocker$` (`Modifier::BlockableWithoutAnyLandwalk`
    /// or `Modifier::BlockableWithoutLandwalk`).
    fn ignore_landwalk(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True")) {
            return self.deny("static ability `S: Mode$ IgnoreLandWalk`".to_string());
        }
        let zone = self.take_static_effect_zone(&mut p, "IgnoreLandWalk")?;
        let modifier = match p.take("ValidKeyword").as_deref() {
            None => "Modifier::BlockableWithoutAnyLandwalk".to_string(),
            Some("Landwalk:Plains") => {
                "Modifier::BlockableWithoutLandwalk(KeywordSet::PLAINSWALK)".to_string()
            }
            Some("Landwalk:Island") => {
                "Modifier::BlockableWithoutLandwalk(KeywordSet::ISLANDWALK)".to_string()
            }
            Some("Landwalk:Swamp") => {
                "Modifier::BlockableWithoutLandwalk(KeywordSet::SWAMPWALK)".to_string()
            }
            Some("Landwalk:Mountain") => {
                "Modifier::BlockableWithoutLandwalk(KeywordSet::MOUNTAINWALK)".to_string()
            }
            Some("Landwalk:Forest") => {
                "Modifier::BlockableWithoutLandwalk(KeywordSet::FORESTWALK)".to_string()
            }
            Some(_) => return self.deny("static ability `S: Mode$ IgnoreLandWalk`".to_string()),
        };
        let cond =
            self.static_conditions(&mut p, "unclaimed parameter `IgnoreLandWalk.CheckSVar`")?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ IgnoreLandWalk`".to_string());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::CREATURE",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    /// Claims `AddSVar$` on a `Continuous` static when every referenced `SVar` is
    /// strictly a non-rules Forge AI hint (`SVar:<hint>:<value>`).
    fn strip_ai_hint_add_svar(&self, p: &mut Params) -> bool {
        let Some(raw) = p.peek("AddSVar") else {
            return false;
        };
        let mut any = false;
        for name in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            any = true;
            let Some(def) = self.svars.get(name) else {
                return false;
            };
            if !is_pure_ai_hint_svar(def) {
                return false;
            }
        }
        if any {
            p.take("AddSVar");
        }
        any
    }
}

fn stack_live_keyword_const(word: &str) -> Option<&'static str> {
    match word {
        "Lifelink" => Some("KeywordSet::LIFELINK"),
        "Deathtouch" => Some("KeywordSet::DEATHTOUCH"),
        "Wither" => Some("KeywordSet::WITHER"),
        "Infect" => Some("KeywordSet::INFECT"),
        "Split second" => Some("KeywordSet::SPLIT_SECOND"),
        "Rebound" => Some("KeywordSet::REBOUND"),
        "Storm" => Some("KeywordSet::STORM"),
        "CARDNAME can't be countered." | "This spell can't be countered." => {
            Some("KeywordSet::UNCOUNTERABLE")
        }
        _ => None,
    }
}

fn strip_secondary_true(spec: &str) -> String {
    spec.split(" | ")
        .filter(|part| part.trim() != "Secondary$ True")
        .collect::<Vec<_>>()
        .join(" | ")
}

fn splice_activated_condition(ability_expr: &str, condition: &str) -> Option<String> {
    if condition.is_empty() {
        return Some(ability_expr.to_string());
    }
    if ability_expr.contains("condition = ") {
        return None;
    }
    let open = ability_expr.strip_suffix(')')?;
    Some(format!("{open}{condition})"))
}

fn is_exert_self_cost(cost: &str) -> bool {
    let Some(inner) = cost
        .trim()
        .strip_prefix("Exert<")
        .and_then(|s| s.strip_suffix('>'))
    else {
        return false;
    };
    let mut parts = inner.split('/');
    let Some(count) = parts.next() else {
        return false;
    };
    let Some(subject) = parts.next() else {
        return false;
    };
    let _desc = parts.next();
    parts.next().is_none()
        && count.trim() == "1"
        && matches!(subject.trim(), "CARDNAME" | "NICKNAME")
}

fn is_pure_ai_hint_svar(def: &str) -> bool {
    let Some(rest) = def.trim().strip_prefix("SVar:") else {
        return false;
    };
    let Some((key, _)) = rest.split_once(':') else {
        return false;
    };
    matches!(
        key.trim(),
        "HasAttackEffect"
            | "HasCombatEffect"
            | "MustBeBlocked"
            | "EquipMe"
            | "DontEquip"
            | "EnchantMe"
            | "DestroyWhenDamaged"
            | "AITapDown"
            | "SacMe"
            | "MustAttack"
            | "Targeting"
            | "EndOfTurnLeavePlay"
            | "DiscardMe"
            | "NonStackingEffect"
    )
}

fn parse_pt_multiplier(raw: &str) -> Option<(&str, i16)> {
    if let Some(v) = raw.strip_suffix("/Twice") {
        return Some((v, 2));
    }
    if let Some((v, n)) = raw.split_once("/Times.") {
        return n
            .trim()
            .parse::<i16>()
            .ok()
            .filter(|&n| (1..=4).contains(&n))
            .map(|n| (v, n));
    }
    Some((raw, 1))
}

/// Splits a plain battlefield unconditional `static_ability!(<filter>, <modifier>)`
/// expression into `(<filter>, <modifier>)`. Refuses any expression carrying
/// extra named arguments (`zone = ...`, `condition = ...`) or non-static items.
fn split_plain_static_ability(expr: &str) -> Option<(&str, &str)> {
    let inner = expr.strip_prefix("static_ability!(")?.strip_suffix(')')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escape = false;
    let mut split_at: Option<usize> = None;
    for (idx, b) in inner.bytes().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.checked_sub(1)?,
            b',' if depth == 0 => {
                if split_at.is_some() {
                    return None;
                }
                split_at = Some(idx);
            }
            _ => {}
        }
    }
    if depth != 0 || in_str {
        return None;
    }
    let comma = split_at?;
    let filter = inner[..comma].trim();
    let modifier = inner[comma + 1..].trim();
    if filter.is_empty()
        || modifier.is_empty()
        || modifier.starts_with("zone")
        || modifier.starts_with("condition")
    {
        return None;
    }
    Some((filter, modifier))
}

#[cfg(test)]
mod wave16b_tests {
    use super::super::{parse, refusal_reason, transcode};
    use crate::body::CardBody;
    use crate::catalog::SubtypeCatalogs;

    fn cats() -> SubtypeCatalogs {
        let mut c = SubtypeCatalogs {
            creature: vec!["Goblin".into(), "Wizard".into()],
            land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
            ..SubtypeCatalogs::default()
        };
        c.normalize();
        c
    }

    fn read(text: &str) -> CardBody {
        let parsed = parse(text);
        let c = cats();
        transcode(&parsed, &c, None).unwrap_or_else(|| {
            panic!(
                "expected clean transcode, refused: {:?}",
                refusal_reason(&parsed, &c, None)
            )
        })
    }

    fn refused(text: &str) -> Option<String> {
        let parsed = parse(text);
        let c = cats();
        assert!(transcode(&parsed, &c, None).is_none(), "expected refusal");
        refusal_reason(&parsed, &c, None)
    }

    #[test]
    fn wave16b_warp_snow_bloodthirst_devour_x_and_hand_conditions() {
        let snow_spell = read(
            "Name:Glacial Tally\nManaCost:2 U\nTypes:Snow Sorcery\n\
             A:SP$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBReturn\n\
             SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | \
             ValidTgts$ Creature.YouOwn+cmcLEX | TgtZone$ Graveyard\n\
             SVar:X:Count$CastTotalManaSpent Snow",
        );
        assert!(snow_spell.abilities[0].contains("Amount::SnowManaSpentToCast"));
        assert!(
            snow_spell
                .statics
                .contains("Filter::CmcAtMostSnowManaSpent")
        );

        let snow_etb = read(
            "Name:Frost Watcher\nManaCost:3 U\nTypes:Snow Creature Wizard\nPT:3/3\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1 | ConditionCheckSVar$ S\n\
             SVar:S:Count$CastTotalManaSpent Snow",
        );
        assert!(snow_etb.abilities[0].contains("Condition::SnowManaWasSpent"));

        let snow_static = read(
            "Name:Rime Guard\nManaCost:2 W\nTypes:Snow Creature Wizard\nPT:2/2\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ S | SVarCompare$ GE2\n\
             S:Mode$ Continuous | Affected$ Creature.!Snow | AddPower$ -1 | AddToughness$ -1\n\
             R:Event$ DamageDone | ValidSource$ Card.SnowSource | ValidTarget$ You | Prevent$ True\n\
             A:AB$ Mana | Cost$ S T | Produced$ U | RestrictValid$ Spell.Snow\n\
             SVar:S:Count$CastTotalManaSpent Snow",
        );
        assert!(
            snow_static.abilities[0]
                .contains("Condition::AmountAtLeast(Amount::SnowManaSpentToCast, 2)")
        );
        assert!(
            snow_static.abilities[1]
                .contains("Filter::Not(&Filter::HasSupertype(SupertypeSet::SNOW))")
        );
        assert!(
            snow_static
                .statics
                .contains("Filter::HasSupertype(SupertypeSet::SNOW)")
                || snow_static.abilities[2].contains("Filter::HasSupertype(SupertypeSet::SNOW)")
        );
        assert!(snow_static.abilities[3].contains("\"{S}\""));
        assert!(snow_static.abilities[3].contains(".restricted("));

        let warp_conds = read(
            "Name:Phase Beacon\nManaCost:2 U\nTypes:Artifact\n\
             S:Mode$ Continuous | Affected$ Card.Self+CastSa Spell.Warp | AddKeyword$ Haste\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ W1\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Vigilance | CheckSVar$ W2\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Lifelink | CheckSVar$ W3 | SVarCompare$ GE2\n\
             A:AB$ Draw | Cost$ 1 | NumCards$ W3\n\
             SVar:W1:Count$ThisTurnWarped\n\
             SVar:W2:Count$ThisTurnWarped_You\n\
             SVar:W3:Count$ThisTurnWarped_Opponent",
        );
        assert!(warp_conds.abilities[0].contains("Condition::WarpCostPaid"));
        assert!(warp_conds.abilities[1].contains("Condition::SpellWarpedThisTurn(None)"));
        assert!(
            warp_conds.abilities[2]
                .contains("Condition::SpellWarpedThisTurn(Some(PlayerRel::You))")
        );
        assert!(warp_conds.abilities[3].contains(
            "Condition::AmountAtLeast(Amount::SpellsWarpedThisTurn(PlayerRel::Opponent), 2)"
        ));
        assert!(
            warp_conds.abilities[4].contains("Amount::SpellsWarpedThisTurn(PlayerRel::Opponent)")
        );

        let warp_filter = read(
            "Name:Warp Watcher\nManaCost:1 U\nTypes:Enchantment\n\
             S:Mode$ Continuous | Affected$ Card.WarpedInExile | AddKeyword$ Lifelink",
        );
        assert!(warp_filter.abilities[0].contains("Filter::WarpedInExile"));
        assert!(
            refused(
                "Name:Keyword Warp Refusal\nManaCost:1 B\nTypes:Sorcery\n\
                 A:SP$ ChangeZone | Origin$ Exile | ValidTgts$ Card.withWarp | Destination$ Library"
            )
            .is_some()
        );
    }

    #[test]
    fn wave16b_bloodthirst_devour_x_and_hand_conditions() {
        let opp_dmg = read(
            "Name:Ember Raider\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
             A:AB$ ChangeZone | Cost$ R | Origin$ Graveyard | Destination$ Hand | ActivationZone$ Graveyard | CheckSVar$ D\n\
             SVar:D:PlayerCountRegisteredOpponents$HasPropertywasDealtDamageThisTurn",
        );
        assert!(opp_dmg.abilities[0].contains("Condition::OpponentDealtDamageThisTurn"));
        assert!(
            refused(
                "Name:Multi Opp Dmg\nManaCost:2 R\nTypes:Enchantment\n\
                 S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Haste | CheckSVar$ D | SVarCompare$ GE2\n\
                 SVar:D:PlayerCountOpponents$HasPropertywasDealtDamageThisTurn"
            )
            .is_some()
        );

        let devour = read(
            "Name:Feast Maw\nManaCost:4 G\nTypes:Creature Goblin\nPT:3/3\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigGain\n\
             SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
             SVar:X:Count$Devoured Card/Twice",
        );
        assert!(
            devour.abilities[0]
                .contains("Amount::Times { base: &Amount::EntrySacrifices, factor: 2 }")
        );
        assert!(
            refused(
                "Name:Filtered Feast\nManaCost:4 R\nTypes:Creature Goblin\nPT:3/3\n\
                 T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigGain\n\
                 SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
                 SVar:X:Count$Devoured Card.Goblin/Twice"
            )
            .is_some()
        );

        let act_x = read(
            "Name:Time Vessel\nManaCost:3\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ X T | NumCards$ 1 | CheckSVar$ X | SVarCompare$ GE3\n\
             SVar:X:Count$xPaid",
        );
        assert!(act_x.abilities[0].contains("Condition::XAtLeast(3)"));

        let any_hand = read(
            "Name:Silent Archive\nManaCost:2 B\nTypes:Enchantment\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ H | SVarCompare$ LE1\n\
             SVar:H:PlayerCountPlayers$LowestCardsInHand",
        );
        assert!(any_hand.abilities[0].contains("Condition::APlayerHandSizeAtMost(1)"));
        assert!(
            refused(
                "Name:Opp Hand Only\nManaCost:2 B\nTypes:Enchantment\n\
                 S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ H | SVarCompare$ EQ0\n\
                 SVar:H:PlayerCountOpponents$LowestCardsInHand"
            )
            .is_some()
        );
    }

    #[test]
    fn wave16b_descend_reduce_cost_stack_and_graveyard_sweeps() {
        let descend8 = read(
            "Name:Deep Relic\nManaCost:5\nTypes:Artifact\n\
             S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 3 | EffectZone$ All | \
             IsPresent$ Permanent.YouOwn | PresentZone$ Graveyard\n\
             Oracle:Descend 8 — This spell costs {3} less to cast if there are eight or more permanent cards in your graveyard.",
        );
        assert!(descend8.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You) }, 8)"));

        let descend4 = read(
            "Name:Shallow Relic\nManaCost:4\nTypes:Artifact\n\
             S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | \
             IsPresent$ Permanent.YouOwn | PresentZone$ Graveyard\n\
             Oracle:Descend 4 — This spell costs {2} less to cast if there are four or more permanent cards in your graveyard.",
        );
        assert!(descend4.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You) }, 4)"));

        assert!(
            refused(
                "Name:Ambiguous Descend\nManaCost:4\nTypes:Artifact\n\
                 S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | \
                 IsPresent$ Permanent.YouOwn | PresentZone$ Graveyard\n\
                 Oracle:Descend 4 — This spell costs {2} less if there are six or more permanent cards in your graveyard. Descend 8 — Draw a card."
            )
            .is_some()
        );

        let stack_statics = read(
            "Name:Spell Aura\nManaCost:2 U R\nTypes:Enchantment\n\
             S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | AddKeyword$ Lifelink | AffectedZone$ Stack\n\
             S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | AddKeyword$ Deathtouch & Split second | AffectedZone$ Stack\n\
             S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | AddKeyword$ Rebound | AffectedZone$ Stack\n\
             S:Mode$ Continuous | Affected$ Permanent.nonLand+YouCtrl | AffectedZone$ Battlefield,Stack | AddType$ Enchantment",
        );
        assert!(stack_statics.abilities[0].contains("Filter::InZone(ZoneRef::Stack)"));
        assert!(stack_statics.abilities[0].contains("Modifier::AddKeyword(KeywordSet::LIFELINK)"));
        assert!(
            stack_statics.abilities[1]
                .contains("KeywordSet::DEATHTOUCH.union(KeywordSet::SPLIT_SECOND)")
        );
        assert!(stack_statics.abilities[2].contains("Modifier::AddKeyword(KeywordSet::REBOUND)"));
        assert!(stack_statics.abilities[3].contains(
            "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::Stack)])"
        ));
        assert!(stack_statics.abilities[3].contains("Modifier::AddType(TypeSet::ENCHANTMENT)"));

        for unsupported_kw in ["Convoke", "Delve", "Cascade", "Affinity:Artifact", "Flash"] {
            assert!(
                refused(&format!(
                    "Name:Unsupported Stack Grant\nManaCost:3 U\nTypes:Enchantment\n\
                     S:Mode$ Continuous | Affected$ Instant.YouCtrl | AddKeyword$ {unsupported_kw} | AffectedZone$ Stack"
                ))
                .is_some(),
                "must refuse stack grant of {unsupported_kw}"
            );
        }

        let gy_statics = read(
            "Name:Crypt Ward\nManaCost:2 G\nTypes:Enchantment\n\
             S:Mode$ CantTarget | AffectedZone$ Graveyard\n\
             S:Mode$ CantTarget | ValidTarget$ Land | AffectedZone$ Battlefield,Graveyard | Activator$ Opponent\n\
             S:Mode$ Continuous | EffectZone$ Graveyard | AffectedZone$ Graveyard | Affected$ Card.Self | Condition$ MaxSpeed | AddAbility$ ABDraw\n\
             S:Mode$ Continuous | EffectZone$ Command | AffectedZone$ Graveyard | Affected$ Permanent.nonLand+YouOwn | MayPlay$ True\n\
             SVar:ABDraw:AB$ Draw | Cost$ 3 ExileFromGrave<1/CARDNAME> | ActivationZone$ Graveyard | Secondary$ True",
        );
        assert_eq!(
            gy_statics.abilities[0],
            "static_ability!(Filter::InZone(ZoneRef::Graveyard), Modifier::CantBeTargetedBy(&Filter::Any))"
        );
        assert!(gy_statics.abilities[1].contains(
            "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::Graveyard)])"
        ));
        assert!(gy_statics.abilities[2].contains("ActivationZone::Graveyard"));
        assert!(gy_statics.abilities[2].contains("condition = Some(Condition::MaxSpeed)"));
        assert!(
            gy_statics.abilities[3]
                .contains("Modifier::CastPermanentSpellsFromGraveyard, zone = ZoneRef::Command")
        );

        let gy_tgt = read(
            "Name:Grave Shift\nManaCost:1 B\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature.YouDontOwn | TgtZone$ Graveyard",
        );
        assert!(
            gy_tgt.abilities[0]
                .contains("TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::Opponent)")
        );

        assert!(
            refused(
                "Name:Mixed Grave Owner\nManaCost:1 B\nTypes:Sorcery\n\
                 A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature.YouOwn,Artifact.OppOwn | TgtZone$ Graveyard"
            )
            .is_some()
        );
    }
}
