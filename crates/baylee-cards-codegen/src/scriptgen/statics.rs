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
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => {
                return self.deny(format!("a `doesn't untap` with `Secondary$ {other}`"));
            }
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
        let condition = self.static_conditions(p, "a `doesn't untap` with `CheckSVar`")?;
        if !p.exhausted() {
            self.note(format!(
                "a `doesn't untap` with `{}`",
                p.first_key().unwrap_or_default()
            ));
            return None;
        }
        self.body.abilities.push(format!(
            "static_ability!({filter}, Modifier::DoesNotUntap{condition})"
        ));
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
            "AlternativeCost" if p.peek("EffectZone") == Some("Graveyard") => {
                return self.wave28c_graveyard_self_alt_cost(p);
            }
            "AlternativeCost" => return self.alternative_cost(p),
            "RaiseCost" => return self.mode_raise_cost(p),
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
            "MustBlock" => return self.must_block(p),
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
            "CantPutCounter" => return self.cant_put_counter(p),
            "OptionalAttackCost" => return self.optional_attack_cost(p),
            "IgnoreLandWalk" => return self.ignore_landwalk(p),
            "ForetellAnyTurn" => return self.wave17c_foretell_any_turn(p),
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
        let exile_self_play = p.peek("EffectZone") == Some("Exile")
            && p.peek("AffectedZone") == Some("Exile")
            && p.peek("Affected") == Some("Card.Self")
            && p.peek("MayPlay") == Some("True");
        if exile_self_play {
            return self.exile_self_may_play(p);
        }
        let graveyard_self_grant = p.peek("EffectZone") == Some("Graveyard")
            && p.peek("AffectedZone") == Some("Graveyard")
            && p.peek("Affected") == Some("Card.Self")
            && (p.has("AddAbility") || p.has("AddStaticAbility"));
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
            && !p.has("MayPlayAltManaCost")
            && !p.has("MayPlayWithoutManaCost")
        {
            return self.graveyard_may_play(p, effect_zone);
        }
        if effect_zone != Some("ZoneRef::Graveyard")
            && p.peek("AffectedZone") == Some("Graveyard")
            && !p.has("MayPlay")
            && (p.peek("AddKeyword").map(str::trim) == Some("Retrace")
                || (!p.has("AddKeyword")
                    && (p.has("AddType") || p.has("AddColor") || p.has("SetColor"))))
        {
            p.take("AffectedZone");
            return self.graveyard_layer45_static(p, effect_zone);
        }
        if let Some(zone) = p.peek("AffectedZone")
            && matches!(zone, "Stack" | "Battlefield,Stack" | "Stack,Battlefield")
        {
            let zone = p.take("AffectedZone").unwrap_or_default();
            return self.stack_continuous_static(p, effect_zone, &zone);
        }
        if effect_zone != Some("ZoneRef::Graveyard")
            && p.peek("AffectedZone") == Some("All")
            && !p.has("MayPlay")
            && !p.has("AddKeyword")
            && !p.has("RemoveKeyword")
            && !p.has("CantHaveKeyword")
            && !p.has("AddPower")
            && !p.has("AddToughness")
            && !p.has("SetPower")
            && !p.has("SetToughness")
            && (p.has("AddType")
                || p.has("AddAllCreatureTypes")
                || p.has("AddColor")
                || p.has("SetColor"))
        {
            p.take("AffectedZone");
            return self.all_zones_layer45_static(p, effect_zone);
        }
        if effect_zone != Some("ZoneRef::Graveyard")
            && p.peek("AffectedZone")
                .is_some_and(is_all_non_battlefield_zones)
            && p.peek("AddKeyword").map(str::trim) == Some("Flash")
            && !p.has("MayPlay")
        {
            let raw_zone = p.take("AffectedZone").unwrap_or_default();
            return self.offboard_flash_static(p, effect_zone, &raw_zone);
        }
        if effect_zone != Some("ZoneRef::Graveyard")
            && p.peek("MayPlay") == Some("True")
            && (p.has("MayPlayAltManaCost")
                || p.has("MayPlayWithoutManaCost")
                || p.peek("AffectedZone") == Some("Exile"))
        {
            return self.may_cast_with_alt_cost(p, effect_zone);
        }
        if effect_zone.is_none() && p.peek("AffectedZone") == Some("Library") {
            return self.continuous_library_top_dispatch(p);
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
        // Non-P/T characteristic-defining abilities on `Card.Self` (or omitted
        // `Affected$`): unconditional Layer 4/5 color/type CDAs (*Ghostfire*,
        // *Transguild Courier*, *Mistform Ultimus*, Party creatures),
        // conditional self-Flash CDAs (*Colossal Rattlewurm*, *Crashing Tide*,
        // *Take for a Ride*), and conditional kicked stack Split Second
        // (*Molten Disaster*).
        if effect_zone.is_none()
            && p.peek("CharacteristicDefining").map(str::trim) == Some("True")
            && !p.has("SetPower")
            && !p.has("SetToughness")
            && matches!(p.peek("Affected"), None | Some("Card.Self"))
            && !p.has("AffectedDefined")
        {
            return self.non_pt_cda_static(p);
        }
        // "This creature's power and toughness are each equal to the number
        // of Swamps you control" (Nightmare): a characteristic-defining
        // ability (CR 604.3), about the card itself and so with no
        // `Affected$`.
        if (p.peek("Affected").is_none()
            || (p.peek("Affected") == Some("Card.Self")
                && (p.has("SetPower") || p.has("SetToughness"))
                && p.entries.iter().all(|(k, _)| {
                    matches!(
                        k.as_str(),
                        "Affected" | "CharacteristicDefining" | "SetPower" | "SetToughness"
                    )
                })))
            && p.take("CharacteristicDefining").as_deref() == Some("True")
        {
            p.take("Affected");
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
                (f.to_string(), Some(c.to_string()))
            } else if let Some((f, c)) = self.wave18b_chosen_mode_affected(&affected) {
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
        let condition = Self::merge_static_condition(implicit_cond.as_deref(), &base_cond);
        let mut out = Vec::new();
        let mut preconditioned_out = Vec::new();
        let mut self_granted = false;
        let zone_arg = effect_zone.map_or_else(String::new, |z| format!(", zone = {z}"));
        if !self.conditional_pt_modifiers(
            &mut p,
            &filter,
            &zone_arg,
            &condition,
            &mut preconditioned_out,
        ) {
            self.pt_modifiers(&mut p, &filter, &mut out)?;
        }
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        if effect_zone == Some("ZoneRef::Graveyard")
            && (p.has("AddAbility")
                || p.has("AddTrigger")
                || p.has("AddStaticAbility")
                || p.has("GainsAbilitiesOf"))
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
        self.gain_activated_abilities_modifier(&mut p, &affected, &filter, effect_zone, &mut out)?;
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
            for ability in &mut out {
                if let Some(open) = ability.strip_suffix(')') {
                    *ability = format!("{open}{zone_arg}{condition})");
                }
            }
        }
        out.extend(preconditioned_out);
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

    pub(super) fn take_static_effect_zone(
        &self,
        p: &mut Params,
        mode: &str,
    ) -> Option<StaticEffectZone> {
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
            "Card.Self+IsMonstrous"
            | "Creature.Self+IsMonstrous"
            | "Card.Self+monstrous"
            | "Creature.Self+monstrous" => Some(("Filter::This", "Condition::IsMonstrous")),
            "Card.Self+!IsMonstrous"
            | "Creature.Self+!IsMonstrous"
            | "Card.Self+!monstrous"
            | "Creature.Self+!monstrous" => {
                Some(("Filter::This", "Condition::Not(&Condition::IsMonstrous)"))
            }
            "Card.Self+IsSuspected"
            | "Creature.Self+IsSuspected"
            | "Card.Self+suspected"
            | "Creature.Self+suspected" => Some(("Filter::This", "Condition::IsSuspected")),
            "Card.Self+!IsSuspected"
            | "Creature.Self+!IsSuspected"
            | "Card.Self+!suspected"
            | "Creature.Self+!suspected" => {
                Some(("Filter::This", "Condition::Not(&Condition::IsSuspected)"))
            }
            _ => None,
        }
    }

    pub(super) fn take_self_state_present(&self, p: &mut Params) -> Result<Option<String>, ()> {
        let Some(present) = p.peek("IsPresent") else {
            return Ok(None);
        };
        let (cond, negated_atom) = match present.trim() {
            "Card.Self+IsRenowned" | "Creature.Self+IsRenowned" => ("Condition::IsRenowned", false),
            "Card.Self+Paired" | "Creature.Self+Paired" => ("Condition::IsPaired", false),
            "Card.Self+PromisedGift" | "Creature.Self+PromisedGift" | "Card.PromisedGift" => {
                ("Condition::GiftWasPromised", false)
            }
            "Card.Self+CastSa Spell.Warp"
            | "Creature.Self+CastSa Spell.Warp"
            | "Card.Self+Warped"
            | "Creature.Self+Warped" => ("Condition::WarpCostPaid", false),
            "Card.Self+IsMonstrous"
            | "Creature.Self+IsMonstrous"
            | "Card.Self+monstrous"
            | "Creature.Self+monstrous" => ("Condition::IsMonstrous", false),
            "Card.Self+!IsMonstrous"
            | "Creature.Self+!IsMonstrous"
            | "Card.Self+!monstrous"
            | "Creature.Self+!monstrous" => ("Condition::IsMonstrous", true),
            "Card.Self+IsSuspected"
            | "Creature.Self+IsSuspected"
            | "Card.Self+suspected"
            | "Creature.Self+suspected" => ("Condition::IsSuspected", false),
            "Card.Self+!IsSuspected"
            | "Creature.Self+!IsSuspected"
            | "Card.Self+!suspected"
            | "Creature.Self+!suspected" => ("Condition::IsSuspected", true),
            _ => return Ok(None),
        };
        p.take("IsPresent");
        let want_present = match p.take("PresentCompare").as_deref() {
            None | Some("GE1" | "EQ1" | "GT0") => true,
            Some("EQ0" | "LT1") => false,
            Some(other) => {
                self.note(format!("`PresentCompare$ {other}`"));
                return Err(());
            }
        };
        if want_present == negated_atom {
            Ok(Some(format!("Condition::Not(&{cond})")))
        } else {
            Ok(Some(cond.to_string()))
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

    pub(super) fn wave21_named_condition(cond: &str) -> Option<&'static str> {
        Some(match cond.trim() {
            "Ferocious" => {
                "Condition::ControlCount(&Filter::YOUR_CREATURE_WITH_POWER_4_OR_GREATER, 1)"
            }
            "Monarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" | "HasInitiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" | "CompletedADungeon" => {
                "Condition::CompletedADungeon(PlayerRel::You)"
            }
            _ => return None,
        })
    }

    fn static_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        if let Some(cond) = self
            .svar_condition_clause(var, cmp)
            .or_else(|| self.promised_gift_svar_condition(var, cmp))
            .or_else(|| self.reduce_cost_svar_condition(var, cmp))
        {
            return Some(cond);
        }
        let raw = if var.trim().starts_with("Count$") || var.trim().starts_with("PlayerCount") {
            var.trim().to_string()
        } else {
            self.svars.get(var.trim())?.trim().to_string()
        };
        if raw == "ExiledWith$Amount" && !self.on_a_spell {
            return Self::wave15_amount_cmp(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::LinkedExile }",
                cmp.trim(),
            );
        }
        if matches!(
            raw.as_str(),
            "PlayerCountPropertyYou$LifeLostThisTurn"
                | "Count$LifeYouLostThisTurn"
                | "PlayerCountPropertyYou$SacrificedThisTurn Permanent"
        ) {
            return self.wave16a_trigger_svar_condition(&raw, cmp);
        }
        None
    }

    pub(super) fn static_conditions(
        &mut self,
        p: &mut Params,
        check_svar_denial: &str,
    ) -> Option<String> {
        let mut clauses: Vec<String> = Vec::new();
        Self::take_ability_word_flags(p, &mut clauses)?;
        if let Some(cond_raw) = p.peek("Condition")
            && let Some(named) = Self::wave21_named_condition(cond_raw)
        {
            p.take("Condition");
            clauses.push(named.to_string());
        }
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
        if let Some(phases) = p.peek("Phases")
            && let Some(phase_cond) = static_phase_condition(phases)
        {
            p.take("Phases");
            clauses.push(phase_cond.to_string());
        }
        self.take_extended_present_clauses(p, &mut clauses)?;
        if let Some(base) = self
            .condition(p)?
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(base.to_string());
        }
        if let Some(var) = p.take("CheckSVar") {
            let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
            let Some(first) = self.static_svar_condition(&var, &cmp) else {
                return self.deny(check_svar_denial.to_string());
            };
            clauses.push(first);
            if let Some(var2) = p.take("CheckSecondSVar") {
                let cmp2 = p
                    .take("SecondSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let Some(second) = self.static_svar_condition(&var2, &cmp2) else {
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
    pub(super) fn continuous_affected(&self, p: &mut Params) -> Option<String> {
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
            _ => &[],
        };
        if !modifiers.is_empty() {
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
            if p.first_key().is_some() {
                return deny_zone(self);
            }
            for modifier in modifiers {
                self.body.abilities.push(format!(
                    "static_ability!(Filter::Any, {modifier}{zone_arg}{condition})"
                ));
            }
            return Some(());
        }
        // Filtered graveyard land-play permissions (Wave 14a: `Modifier::PlayLandsFromGraveyard`
        // evaluates the static ability's `filter` against land cards in your graveyard).
        if !p.has("MayPlayLimit")
            && !p.has("MayPlayWithFlash")
            && !p.has("RaiseCost")
            && !p.has("MayPlayPlayer")
            && let Some(land_filter) = self.filtered_graveyard_land_filter(&affected)
        {
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
            if p.first_key().is_some() {
                return deny_zone(self);
            }
            self.body.abilities.push(format!(
                "static_ability!({land_filter}, Modifier::PlayLandsFromGraveyard{zone_arg}{condition})"
            ));
            return Some(());
        }
        // General filtered graveyard `MayPlay$ True` (including `MayPlayLimit$ 1`,
        // `MayPlayWithFlash$ True`, `RaiseCost$`, `cmcLE...`) -> `Modifier::MayCastWithAltCost`
        // with `AltCastPrice::Normal` or `AltCastPrice::NormalPlus`.
        let mut alt_p = p;
        alt_p.entries.push(("Affected".to_string(), affected));
        alt_p
            .entries
            .push(("AffectedZone".to_string(), "Graveyard".to_string()));
        alt_p
            .entries
            .push(("MayPlay".to_string(), "True".to_string()));
        self.may_cast_with_alt_cost(alt_p, effect_zone)
    }

    fn filtered_graveyard_land_filter(&self, affected: &str) -> Option<String> {
        let mut stripped_alts = Vec::new();
        for raw_alt in affected.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return None;
            }
            let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
            if base.trim() != "Land" {
                return None;
            }
            let mut saw_you = false;
            let mut kept = Vec::new();
            for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                match atom {
                    "YouOwn" | "YouCtrl" => saw_you = true,
                    "Self" | "Other" | "StrictlyOther" | "EnchantedBy" | "EquippedBy"
                    | "AttachedBy" | "Attached" | "IsRemembered" | "ExiledWithSource"
                    | "ExiledWith" | "IsImprinted" | "TopLibrary" => return None,
                    a if a.contains("Ctrl") || a.contains("Own") || a.starts_with("cmcLE") => {
                        return None;
                    }
                    other => kept.push(other),
                }
            }
            if !saw_you || kept.is_empty() {
                return None;
            }
            stripped_alts.push(format!("Land.{}", kept.join("+")));
        }
        if stripped_alts.is_empty() {
            return None;
        }
        self.filter_expr_in_zone(&stripped_alts.join(","), true)
    }

    /// `S: Mode$ Continuous | EffectZone$ Exile | AffectedZone$ Exile | Affected$ Card.Self | MayPlay$ True`
    /// (*Eternal Scourge*, *Misthollow Griffin*, *Squee, the Immortal* — normal cost or `RaiseCost$`).
    fn exile_self_may_play(&mut self, mut p: Params) -> Option<()> {
        p.take("EffectZone");
        p.take("AffectedZone");
        p.take("Affected");
        p.take("MayPlay");
        if self.type_line.split_whitespace().any(|w| w == "Land") {
            return self.deny("static ability from `EffectZone$ Exile`".into());
        }
        if p.has("MayPlayAltManaCost") || p.has("MayPlayWithoutManaCost") {
            return self.deny("static ability from `EffectZone$ Exile`".into());
        }
        let price = if let Some(raw_raise) = p.take("RaiseCost") {
            let cost = self.normal_plus_cost_expr(&raw_raise)?;
            format!("AltCastPrice::NormalPlus({cost})")
        } else {
            "AltCastPrice::Normal".to_string()
        };
        let once_per_turn = match p.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(other) => return self.deny(format!("`Continuous.MayPlayLimit$ {other}`")),
        };
        let with_flash = match p.take("MayPlayWithFlash").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Continuous.MayPlayWithFlash$ {other}`")),
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let modifier = format!(
            "Modifier::MayCastWithAltCost {{ who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Exile]), grant_zone_permission: true, allow_lands: false, with_flash: {with_flash}, max_mana_value: None, once_per_turn: {once_per_turn}, price: {price} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::This",
            &modifier,
            StaticEffectZone::Explicit("ZoneRef::Exile"),
            &condition,
        ));
        Some(())
    }

    /// Self-granted graveyard `AddStaticAbility$` wrapping `MayPlay$ True`
    /// with optional outer condition (`EffectZone$ Graveyard | AffectedZone$
    /// Graveyard | Affected$ Card.Self | AddStaticAbility$ ...`).
    fn graveyard_self_granted_static(&mut self, mut p: Params) -> Option<()> {
        p.take("AffectedZone");
        if p.take("Affected").as_deref() != Some("Card.Self") {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        let Some(var) = p.take("AddStaticAbility") else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        let Some(svar_raw) = self.svars.get(var.trim()).cloned() else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        };
        let Some((mode, mut inner)) = Params::parse(&svar_raw) else {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        };
        inner.drop_prose();
        if !svar_raw.trim_start().starts_with("Mode$")
            || mode != "Continuous"
            || inner.take("Affected").as_deref() != Some("Card.Self")
            || inner.take("AffectedZone").as_deref() != Some("Graveyard")
            || inner.take("EffectZone").as_deref() != Some("Graveyard")
            || inner.take("MayPlay").as_deref() != Some("True")
            || !matches!(inner.take("Secondary").as_deref(), None | Some("True"))
            || !inner.exhausted()
        {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CastSpellsFromGraveyard, \
             zone = ZoneRef::Graveyard{condition})"
        ));
        Some(())
    }

    /// Self-granted graveyard activated abilities (`EffectZone$ Graveyard |
    /// AffectedZone$ Graveyard | Affected$ Card.Self | AddAbility$ ...`, e.g.
    /// Max speed graveyard cycling/draw activations).
    fn graveyard_self_granted_ability(&mut self, mut p: Params) -> Option<()> {
        if p.has("AddStaticAbility") && !p.has("AddAbility") {
            return self.graveyard_self_granted_static(p);
        }
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
    #[allow(clippy::too_many_lines)]
    fn stack_continuous_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
        affected_zone: &str,
    ) -> Option<()> {
        if effect_zone == Some("ZoneRef::Stack")
            && affected_zone == "Stack"
            && p.peek("Affected") == Some("Card.Self")
            && p.peek("AddKeyword").map(str::trim) == Some("Cascade")
            && !p.has("AffectedDefined")
        {
            p.take("Affected");
            p.take("AddKeyword");
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
            let Some(inner_cond) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                return self.deny("static ability reaching `AffectedZone$ Stack`".to_string());
            };
            self.strip_ai_hint_add_svar(&mut p);
            if !p.exhausted() {
                return self.deny("static ability reaching `AffectedZone$ Stack`".to_string());
            }
            self.body.abilities.push(format!(
                "triggered!(Trigger::SpellCast(&Filter::This), &[Effect::Cascade], \
                 condition = Some(Condition::TriggerOnly(&{inner_cond})))"
            ));
            return Some(());
        }
        let affected = self.continuous_affected(&mut p)?;
        let base_filter = self.filter_expr_in_zone(&affected, true)?;
        let zone_clause = if affected_zone == "Stack" {
            "Filter::InZone(ZoneRef::Stack)"
        } else {
            "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::Stack)])"
        };
        let mut filter_clauses: Vec<String> = Vec::new();
        if base_filter != "Filter::Any" {
            filter_clauses.push(base_filter);
        }
        if p.peek("SVarCompare") == Some("EQ0")
            && !p.has("CheckSecondSVar")
            && let Some(var_name) = p.peek("CheckSVar")
            && let Some(def) = self.svars.get(var_name.trim()).cloned()
            && let Some(cast_valid) = def.trim().strip_prefix("Count$ThisTurnCast_")
            && !cast_valid.contains(['$', '/'])
        {
            let cast_expr = self.filter_expr_in_zone(cast_valid, true)?;
            let cast_name = self.body.filter_static("FIRST_SPELL", &cast_expr);
            p.take("CheckSVar");
            p.take("SVarCompare");
            filter_clauses.push(format!("Filter::FirstMatchingSpellThisTurn(&{cast_name})"));
        }
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        let mut bits = Vec::new();
        let mut extra_modifiers = Vec::new();
        let mut cascade_count: u8 = 0;
        if let Some(raw_kw) = p.take("AddKeyword") {
            for word in raw_kw.split(" & ").map(str::trim).filter(|s| !s.is_empty()) {
                if let Some(bit) = stack_live_keyword_const(word) {
                    bits.push(bit);
                    continue;
                }
                match word {
                    "Convoke" => bits.push("KeywordSet::CONVOKE"),
                    "Improvise" => bits.push("KeywordSet::IMPROVISE"),
                    "Delve" => bits.push("KeywordSet::DELVE"),
                    "Conspire" => bits.push("KeywordSet::CONSPIRE"),
                    "Demonstrate" => bits.push("KeywordSet::DEMONSTRATE"),
                    "Cascade" => cascade_count = cascade_count.saturating_add(1),
                    "Replicate:CardManaCost" => {
                        extra_modifiers.push("Modifier::GrantReplicateCardManaCost".to_string());
                    }
                    "Blitz:CardManaCost" => {
                        extra_modifiers.push("Modifier::GrantBlitzCardManaCost".to_string());
                    }
                    _ if word.starts_with("Casualty:") => {
                        let Some(n) = word
                            .strip_prefix("Casualty:")
                            .and_then(|s| s.trim().parse::<u8>().ok())
                            .filter(|&n| n > 0)
                        else {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        };
                        extra_modifiers.push(format!("Modifier::GrantCasualty({n})"));
                    }
                    _ if word.starts_with("Ripple:") => {
                        let Some(n) = word
                            .strip_prefix("Ripple:")
                            .and_then(|s| s.trim().parse::<u8>().ok())
                            .filter(|&n| n > 0)
                        else {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        };
                        extra_modifiers.push(format!("Modifier::GrantRipple({n})"));
                    }
                    _ if word.starts_with("Affinity:") => {
                        let kind = word.strip_prefix("Affinity:").unwrap_or("").trim();
                        if kind.is_empty() || kind.contains(['$', '/']) {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        }
                        let aff_expr = self.filter_expr(kind)?;
                        let aff_ref = if simple_const_filter(&aff_expr) {
                            format!("&{aff_expr}")
                        } else {
                            format!("&{}", self.body.filter_static("AFFINITY", &aff_expr))
                        };
                        extra_modifiers.push(format!("Modifier::GrantAffinity({aff_ref})"));
                    }
                    _ if word.starts_with("Prowl:") => {
                        let raw = word.strip_prefix("Prowl:").unwrap_or("").trim();
                        let Some(mana) = super::printed_mana(raw) else {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        };
                        if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        }
                        extra_modifiers.push(format!("Modifier::GrantProwl(mana!(\"{mana}\"))"));
                    }
                    _ if word.starts_with("Freerunning:") => {
                        let raw = word.strip_prefix("Freerunning:").unwrap_or("").trim();
                        let Some(mana) = super::printed_mana(raw) else {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        };
                        if baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        }
                        extra_modifiers
                            .push(format!("Modifier::GrantFreerunning(mana!(\"{mana}\"))"));
                    }
                    _ if word.starts_with("Blitz:CardManaCost:") => {
                        let extra_raw = word
                            .strip_prefix("Blitz:CardManaCost:")
                            .unwrap_or("")
                            .trim();
                        let Some(extra_valid) = extra_raw.strip_prefix("Spell.") else {
                            return self.deny(format!(
                                "static ability reaching `AffectedZone$ {affected_zone}`"
                            ));
                        };
                        let extra_expr =
                            self.filter_expr_in_zone(&format!("Card.{extra_valid}"), true)?;
                        filter_clauses.push(extra_expr);
                        extra_modifiers.push("Modifier::GrantBlitzCardManaCost".to_string());
                    }
                    _ => {
                        return self.deny(format!(
                            "static ability reaching `AffectedZone$ {affected_zone}`"
                        ));
                    }
                }
            }
        }
        filter_clauses.push(zone_clause.to_string());
        let filter = if filter_clauses.len() == 1 {
            filter_clauses.remove(0)
        } else {
            format!("Filter::And(&[{}])", filter_clauses.join(", "))
        };
        let mut out = Vec::new();
        if let Some((head, tail)) = bits.split_first() {
            let set = tail
                .iter()
                .fold((*head).to_string(), |acc, b| format!("{acc}.union({b})"));
            out.push(Self::static_expr(
                &filter,
                &format!("Modifier::AddKeyword({set})"),
            ));
        }
        if cascade_count > 0 {
            out.push(Self::static_expr(
                &filter,
                &format!("Modifier::GrantCascade({cascade_count})"),
            ));
        }
        for modifier in extra_modifiers {
            out.push(Self::static_expr(&filter, &modifier));
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

    /// Layer 4/5 type- or color-changing static abilities reaching cards in
    /// graveyards (`AffectedZone$ Graveyard`, e.g. *Ashes of the Fallen*):
    /// conjoins `Filter::InZone(ZoneRef::Graveyard)` so the engine's cross-zone
    /// projection pass updates graveyard characteristics.
    fn graveyard_layer45_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        let affected = self.continuous_affected(&mut p)?;
        let base_filter = self.filter_expr_in_zone(&affected, true)?;
        let filter = if base_filter == "Filter::Any" {
            "Filter::InZone(ZoneRef::Graveyard)".to_string()
        } else {
            format!("Filter::And(&[{base_filter}, Filter::InZone(ZoneRef::Graveyard)])")
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        let mut out = Vec::new();
        if p.peek("AddKeyword").map(str::trim) == Some("Retrace") {
            p.take("AddKeyword");
            out.push(Self::static_expr(
                &filter,
                "Modifier::AddKeyword(KeywordSet::RETRACE)",
            ));
        }
        if !p.has("RemoveLandTypes") {
            self.type_modifiers(&mut p, &filter, &mut out)?;
        }
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() || out.is_empty() {
            return self.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
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

    /// Lowers conditional two-branch P/T buffs (`Count$<Flag>.<a>.<b>` and
    /// `Count$Compare <var> <cmp>.<a>.<b>`) on `AddPower$`/`AddToughness$` into
    /// an unconditional base `ModifyPT(b_p, b_t)` plus a conditional delta
    /// `ModifyPT(a_p - b_p, a_t - b_t)`. Returns `false` when the line does not
    /// use conditional P/T `SVar`s so `pt_modifiers` handles it unchanged.
    fn conditional_pt_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        zone_arg: &str,
        outer_cond: &str,
        out: &mut Vec<String>,
    ) -> bool {
        if !p.has("AddPower") && !p.has("AddToughness") {
            return false;
        }
        let Some((p_cond, p_true, p_false)) = self.parse_conditional_pt_half(p.peek("AddPower"))
        else {
            return false;
        };
        let Some((t_cond, t_true, t_false)) =
            self.parse_conditional_pt_half(p.peek("AddToughness"))
        else {
            return false;
        };
        let cond = match (p_cond, t_cond) {
            (Some(c), None) | (None, Some(c)) => c,
            (Some(c1), Some(c2)) if c1 == c2 => c1,
            _ => return false,
        };
        let Some(delta_p) = p_true.checked_sub(p_false) else {
            return false;
        };
        let Some(delta_t) = t_true.checked_sub(t_false) else {
            return false;
        };
        p.take("AddPower");
        p.take("AddToughness");
        if p_false != 0 || t_false != 0 {
            out.push(format!(
                "static_ability!({filter}, Modifier::ModifyPT({p_false}, {t_false}){zone_arg}{outer_cond})"
            ));
        }
        if delta_p != 0 || delta_t != 0 {
            let merged = Self::merge_static_condition(Some(&cond), outer_cond);
            out.push(format!(
                "static_ability!({filter}, Modifier::ModifyPT({delta_p}, {delta_t}){zone_arg}{merged})"
            ));
        }
        true
    }

    fn parse_conditional_pt_half(
        &mut self,
        raw: Option<&str>,
    ) -> Option<(Option<String>, i16, i16)> {
        let Some(raw) = raw else {
            return Some((None, 0, 0));
        };
        let trimmed = raw.trim();
        if let Ok(n) = trimmed.parse::<i16>() {
            return Some((None, n, n));
        }
        let neg = trimmed.starts_with('-');
        let var = trimmed.trim_start_matches(['+', '-']);
        let body = self.svars.get(var)?.trim().to_string();
        let (cond, raw_true, raw_false) = if let Some(rest) = body.strip_prefix("Count$Compare ") {
            let (check_part, rest) = rest.split_once('.')?;
            let (a_str, b_str) = rest.split_once('.')?;
            let (lhs_var, cmp) = check_part.trim().split_once(' ')?;
            let cond = self.svar_condition_clause(lhs_var.trim(), cmp.trim())?;
            (
                cond,
                a_str.trim().parse::<i16>().ok()?,
                b_str.trim().parse::<i16>().ok()?,
            )
        } else {
            let rest = body.strip_prefix("Count$")?;
            let (flag, tail) = rest.split_once('.')?;
            let (a_str, b_str) = tail.split_once('.')?;
            let cond = match flag {
                "Threshold" => "Condition::GraveyardCountAtLeast(7)",
                "Delirium" => "Condition::GraveyardTypesAtLeast(4)",
                "Metalcraft" => "Condition::ControlCount(&Filter::ARTIFACT, 3)",
                "Hellbent" => "Condition::HandSizeAtMost(0)",
                "FatefulHour" => "Condition::LifeAtMost(5)",
                "Blessing" => "Condition::CitysBlessing",
                "Revolt" => "Condition::Revolt",
                "Raid" => "Condition::AttackedWithAtLeast(1)",
                "Monstrous" => "Condition::IsMonstrous",
                "Suspected" => "Condition::IsSuspected",
                _ => return None,
            };
            (
                cond.to_string(),
                a_str.trim().parse::<i16>().ok()?,
                b_str.trim().parse::<i16>().ok()?,
            )
        };
        let (t_val, f_val) = if neg {
            (raw_true.checked_neg()?, raw_false.checked_neg()?)
        } else {
            (raw_true, raw_false)
        };
        Some((Some(cond), t_val, f_val))
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
                let inline_plus = toughness
                    .trim()
                    .strip_prefix(&format!("{}/Plus.", power.trim()))
                    .and_then(|rest| rest.trim().parse::<i8>().ok());
                let svar_plus = self
                    .svars
                    .get(toughness.trim())
                    .and_then(|t_def| {
                        t_def
                            .trim()
                            .strip_prefix(&format!("SVar${}/Plus.", power.trim()))
                    })
                    .and_then(|rest| rest.trim().parse::<i8>().ok());
                if let Some(plus) = inline_plus.or(svar_plus) {
                    let count = self.pt_count_for_svar(&power)?;
                    self.body.abilities.push(format!(
                        "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                         count: {count}, toughness_plus: {plus} }})"
                    ));
                    return Some(());
                }
                let p_count = self.pt_count_or_fixed(&power)?;
                let t_count = self.pt_count_or_fixed(&toughness)?;
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

    fn pt_count_or_fixed(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if let Ok(n) = trimmed.parse::<u16>() {
            return Some(format!("PtCount::Fixed({n})"));
        }
        self.pt_count_for_svar(trimmed)
    }

    fn pt_count_for_svar(&mut self, var: &str) -> Option<String> {
        let trimmed = var.trim();
        if trimmed.starts_with("Count$") {
            return self.pt_count_expr(trimmed, 0);
        }
        let Some(raw) = self.svars.get(trimmed).cloned() else {
            return self.deny(format!("`{var}` names no SVar"));
        };
        self.pt_count_expr(raw.trim(), 0)
    }

    fn pt_count_expr(&mut self, raw: &str, depth: u8) -> Option<String> {
        if depth > 4 {
            return self.deny(format!("cyclic count `{raw}`"));
        }
        let raw = raw.trim();
        if let Some((lhs, rhs)) = raw.rsplit_once("/Plus.")
            && let Ok(offset) = rhs.trim().parse::<u16>()
        {
            let base = self.pt_count_atom(lhs.trim(), depth + 1)?;
            return Some(format!(
                "PtCount::Plus {{ base: &{base}, offset: {offset} }}"
            ));
        }
        if let Some(lhs) = raw
            .strip_suffix("/Twice")
            .or_else(|| raw.strip_suffix("/Times.2"))
        {
            let base = self.pt_count_atom(lhs.trim(), depth + 1)?;
            return Some(format!("PtCount::Twice(&{base})"));
        }
        self.pt_count_atom(raw, depth + 1)
    }

    fn pt_count_atom(&mut self, raw: &str, depth: u8) -> Option<String> {
        let trimmed = raw.trim();
        if let Some(inner_var) = trimmed.strip_prefix("SVar$") {
            let inner_name = inner_var.trim();
            let Some(inner) = self.svars.get(inner_name).cloned() else {
                return self.deny(format!("`{inner_name}` names no SVar"));
            };
            return self.pt_count_expr(inner.trim(), depth + 1);
        }
        self.pt_count_base(trimmed)
    }

    fn pt_count_base(&mut self, count: &str) -> Option<String> {
        if let Some(counter) = count.strip_prefix("Count$CardCounters.") {
            let Some(kind) = super::counter_kind(counter) else {
                return self.deny(format!("counter `{counter}`"));
            };
            return Some(format!("PtCount::CountersOnThis({kind})"));
        }
        match count {
            "Count$Domain" => return Some("PtCount::Domain(PlayerRel::You)".to_string()),
            "Count$Party" => return Some("PtCount::PartySize(PlayerRel::You)".to_string()),
            "Count$YourLifeTotal" => return Some("PtCount::LifeTotal(PlayerRel::You)".to_string()),
            "Count$CardsInYourHand"
            | "Count$ValidHand Card.YouOwn"
            | "Count$ValidHand Card.YouCtrl" => {
                return Some("PtCount::CardsInHand(PlayerRel::You)".to_string());
            }
            "Count$NumInAllHands" | "Count$ValidHand Card" => {
                return Some("PtCount::CardsInHand(PlayerRel::EachPlayer)".to_string());
            }
            "Count$ValidHand Card.OppOwn" | "Count$ValidHand Card.OppCtrl" => {
                return Some("PtCount::CardsInHand(PlayerRel::EachOpponent)".to_string());
            }
            _ => {}
        }
        if let Some(valid) = count.strip_prefix("Count$ValidGraveyard ") {
            return self.pt_count_graveyard(count, valid);
        }
        let Some(valid) = count.strip_prefix("Count$Valid ") else {
            return self.deny(format!("count `{count}`"));
        };
        if let Some(inner) = valid.strip_suffix("$DifferentCardNames")
            && !inner.contains(['/', '$'])
        {
            let filter = self.filter_expr(inner)?;
            let name = self.body.filter_static("COUNTED", &filter);
            return Some(format!("PtCount::DistinctNamesAmong(&{name})"));
        }
        if let Some(inner) = valid.strip_suffix("$GreatestCardManaCost")
            && !inner.contains(['/', '$'])
        {
            let filter = self.filter_expr(inner)?;
            let name = self.body.filter_static("COUNTED", &filter);
            return Some(format!("PtCount::GreatestManaValueAmong(&{name})"));
        }
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
        let yours = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
        let opps = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"));
        let other_ctrl_or_own =
            valid
                .split(',')
                .flat_map(|alt| alt.split(['.', '+']))
                .any(|atom| {
                    let atom = atom.trim();
                    (atom.contains("Ctrl") || atom.contains("Own"))
                        && !matches!(atom, "YouCtrl" | "OppCtrl")
                });
        if other_ctrl_or_own
            || (yours && opps)
            || (!yours && !opps && (valid.contains("Ctrl") || valid.contains("Own")))
        {
            return self.deny(format!("count `{count}`"));
        }
        let filter = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNTED", &filter);
        let kind = if yours { "YouControl" } else { "OnBattlefield" };
        Some(format!("PtCount::{kind}(&{name})"))
    }

    fn pt_count_graveyard(&mut self, count: &str, valid: &str) -> Option<String> {
        if valid == "Card$CardTypes" {
            return Some("PtCount::CardTypesInAllGraveyards".to_string());
        }
        if valid == "Card.OppOwn$CardTypes" {
            return Some("PtCount::CardTypesInOpponentsGraveyards".to_string());
        }
        if valid.contains('/') || valid.contains('$') {
            return self.deny(format!("count `{count}`"));
        }
        let Some((who, cleaned)) = strip_graveyard_owner(valid) else {
            return self.deny(format!("count `{count}`"));
        };
        let filter = self.filter_expr_in_zone(&cleaned, true)?;
        let name = self.body.filter_static("COUNTED", &filter);
        Some(match who {
            Some(rel) => format!("PtCount::InGraveyards {{ who: {rel}, filter: &{name} }}"),
            None => format!("PtCount::InAllGraveyards(&{name})"),
        })
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
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`{modifier}.Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny(format!("`{modifier}` naming no creature"));
        };
        let (filter, implicit_cond) =
            if let Some((f, c)) = Self::soulbond_or_renown_affected(&valid) {
                (f.to_string(), Some(c))
            } else {
                (self.filter_expr(&valid)?, None)
            };
        let base_cond = self.static_conditions(
            &mut p,
            &format!("unclaimed parameter `{modifier}.CheckSVar`"),
        )?;
        let condition = Self::merge_static_condition(implicit_cond, &base_cond);
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `{modifier}.{key}`"));
        }
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
        let defender_ctrl_cond = if zone.is_battlefield() {
            self.defender_ctrl_present_condition(&mut p).ok()?
        } else {
            None
        };
        let base_cond =
            self.static_conditions(&mut p, "unclaimed parameter `CantBlockBy.CheckSVar`")?;
        let condition = Self::merge_static_condition(defender_ctrl_cond.as_deref(), &base_cond);
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
        if p.has("MustAttack") {
            return self.wave27d_must_attack_static(p);
        }
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "MustAttack")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`MustAttack.Secondary$ {other}`")),
        }
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

    /// `S: Mode$ MustBlock | ValidCreature$ ...` -> `Modifier::MustBlock`
    /// (*Watchdog*, *Grand Melee*, *Invasion Plans*, *Iron Golem*,
    /// *Khârn the Betrayer*, *Razorgrass Screen*, *Relentless Raptor*, *Spirespine*).
    pub(super) fn must_block(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "MustBlock")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`MustBlock.Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCreature") else {
            return self.deny("`MustBlock` naming no creature".to_string());
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `MustBlock.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `MustBlock.{key}`"));
            return None;
        }
        let filter = self.filter_expr(&valid)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::MustBlock",
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
    #[allow(clippy::too_many_lines)]
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
        if matches!(
            p.peek("Target"),
            Some("You,Planeswalker.YouCtrl" | "Planeswalker.YouCtrl,You")
        ) && !p.has("UnlessDefender")
        {
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
                "Modifier::CantAttackYouOrYourPlaneswalkers",
                zone,
                &condition,
            ));
            return Some(());
        }
        if p.peek("Target") == Some("Card.Self+AttachedTo Creature") && !p.has("UnlessDefender") {
            p.take("Target");
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `CantAttack.CheckSVar`")?;
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantAttack.{key}`"));
                return None;
            }
            let attackers = self.filter_expr(&valid)?;
            let attackers_ref = if simple_const_filter(&attackers) {
                format!("&{attackers}")
            } else {
                format!("&{}", self.body.filter_static("ATTACKER", &attackers))
            };
            self.body.abilities.push(Self::format_static_expr(
                "Filter::And(&[Filter::This, Filter::AttachedTo(&Filter::CREATURE)])",
                &format!("Modifier::CantBeAttackedBy({attackers_ref})"),
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

    /// Reads `IsPresent$ <Filter with DefenderCtrl>` on battlefield `CantBlockBy`
    /// statics ("as long as defending player controls ..."). Refuses when
    /// qualified by counts/zones/second clauses so `static_conditions` handles them.
    fn defender_ctrl_present_condition(&mut self, p: &mut Params) -> Result<Option<String>, ()> {
        let Some(valid) = p.peek("IsPresent").map(str::to_string) else {
            return Ok(None);
        };
        if !valid.contains("DefenderCtrl")
            || p.has("IsPresent2")
            || p.has("PresentCompare")
            || p.has("PresentZone")
            || p.has("PresentPlayer")
            || p.has("PresentDefined")
        {
            return Ok(None);
        }
        let mut stripped_alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let Some((base, rest)) = alt.split_once('.') else {
                return Ok(None);
            };
            let mut saw_defender = false;
            let mut kept = Vec::new();
            for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if atom == "DefenderCtrl" {
                    saw_defender = true;
                } else {
                    kept.push(atom);
                }
            }
            if !saw_defender {
                return Ok(None);
            }
            if kept.is_empty() {
                stripped_alts.push(base.trim().to_string());
            } else {
                stripped_alts.push(format!("{}.{}", base.trim(), kept.join("+")));
            }
        }
        p.take("IsPresent");
        let expr = self.filter_expr(&stripped_alts.join(",")).ok_or(())?;
        let name = self.body.filter_static("TARGET", &expr);
        Ok(Some(format!(
            "Condition::Not(&Condition::SomeDefendingPlayerControlsNone(&{name}))"
        )))
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
            let modifier = if filter.contains("Filter::ARTIFACT")
                || filter.contains("Filter::YOUR_ARTIFACT")
                || filter.contains("Filter::AttachedToBySource")
            {
                "Modifier::AnimateNoncreatureArtifact"
            } else {
                "Modifier::CreatureWithManaValuePT"
            };
            out.push(Self::static_expr(filter, modifier));
            return Some(());
        }
        let add_p = p.take("AddPower");
        let add_t = p.take("AddToughness");
        if add_p.is_some() || add_t.is_some() {
            let mut emitted_pt = false;
            if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && let Some(neg_var) = var_p.trim().strip_prefix('-')
                && neg_var.parse::<i16>().is_err()
                && let Some(count) = self.svars.get(neg_var).cloned()
                && count.trim() == "PlayerCountOpponents$CardsInHand"
            {
                out.push(Self::static_expr(
                    filter,
                    "Modifier::ReducePTByCount { count: PtCount::CardsInHand(PlayerRel::EachOpponent), minimum_toughness: None }",
                ));
                emitted_pt = true;
            }
            if !emitted_pt
                && let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
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
                    && !valid.contains('$')
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
                } else if matches!(
                    trimmed,
                    "Count$ValidHand Card.OwnedBy EnchantedController"
                        | "Count$ValidHand Card.OwnedBy AttachedController"
                ) {
                    out.push(Self::static_expr(
                        filter,
                        "Modifier::ModifyPTByCount(PtCount::CardsInHand(PlayerRel::ControllerOfAttached))",
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
                    let modifier =
                        self.computed_set_pt_modifier(filter, set_p.as_deref(), set_t.as_deref())?;
                    out.push(Self::static_expr(filter, &modifier));
                }
            }
        }
        Some(())
    }

    pub(super) fn computed_set_pt_modifier(
        &mut self,
        filter: &str,
        set_p: Option<&str>,
        set_t: Option<&str>,
    ) -> Option<String> {
        let count_ok = |count: &str| {
            filter == "Filter::This"
                || (!count.contains("PtCount::CountersOnThis")
                    && !count.contains("PtCount::ExiledWithThis")
                    && !count.contains("PtCount::EntrySacrifices")
                    && !count.contains("PtCount::ChosenNumber"))
        };
        match (set_p, set_t) {
            (Some(p_raw), Some(t_raw)) if p_raw.trim() == t_raw.trim() => {
                let count = self.pt_count_for_svar(p_raw)?;
                if !count_ok(&count) {
                    return self.deny("source-bound `PtCount` on non-self filter".to_string());
                }
                Some(format!("Modifier::SetPTToCount({count})"))
            }
            (Some(p_raw), Some(t_raw)) => {
                let power = self.pt_count_or_fixed(p_raw)?;
                let toughness = self.pt_count_or_fixed(t_raw)?;
                if !count_ok(&power) || !count_ok(&toughness) {
                    return self.deny("source-bound `PtCount` on non-self filter".to_string());
                }
                Some(format!(
                    "Modifier::SetPTSeparateToCount {{ power: {power}, toughness: {toughness} }}"
                ))
            }
            (Some(p_raw), None) => {
                let power = self.pt_count_for_svar(p_raw)?;
                if !count_ok(&power) {
                    return self.deny("source-bound `PtCount` on non-self filter".to_string());
                }
                Some(format!("Modifier::SetPowerToCount({power})"))
            }
            (None, Some(t_raw)) => {
                let toughness = self.pt_count_for_svar(t_raw)?;
                if !count_ok(&toughness) {
                    return self.deny("source-bound `PtCount` on non-self filter".to_string());
                }
                Some(format!("Modifier::SetToughnessToCount({toughness})"))
            }
            (None, None) => None,
        }
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
        let inner = self
            .static_count_expr_ext(&body)
            .or_else(|| self.wave14_non_power_amount(&body))
            .or_else(|| Self::wave29d_static_pt_amount(&body))?;
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
        let allows_equip = p
            .peek("AddType")
            .is_some_and(|raw| raw.split_whitespace().any(|w| w == "Equipment"))
            || filter.contains("subtypes::artifact::EQUIPMENT");
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
                if modifier == "AddKeyword"
                    && self.try_granted_keyword_ability(
                        word,
                        filter,
                        allows_equip,
                        &mut bits,
                        out,
                    )?
                {
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
        if let Some(raw) = p.take("CantHaveKeyword") {
            let mut bits = Vec::new();
            for word in raw.split(" & ").map(str::trim).filter(|w| !w.is_empty()) {
                let Some(bit) = keyword_const(word) else {
                    self.note(format!("static ability forbidding keyword `{word}`"));
                    return None;
                };
                bits.push(bit.to_string());
            }
            let Some(set) = bits.split_first().map(|(head, tail)| {
                tail.iter()
                    .fold(head.clone(), |acc, b| format!("{acc}.union({b})"))
            }) else {
                self.note("static ability forbidding empty keyword list".to_string());
                return None;
            };
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::CantHaveKeywords({set})"),
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
        if let Some(hidden) = p.peek("AddHiddenKeyword").map(str::trim) {
            let modifier = match hidden {
                LURED => Some("Modifier::MustBeBlockedByAllAble"),
                "CARDNAME can only attack alone." => Some("Modifier::AttacksAloneOnly"),
                "CARDNAME can't block." => Some("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
                "CARDNAME can't attack." => Some("Modifier::AddKeyword(KeywordSet::CANT_ATTACK)"),
                "CARDNAME can't attack or block." => Some(
                    "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))",
                ),
                _ => None,
            };
            if let Some(modifier) = modifier {
                p.take("AddHiddenKeyword");
                out.push(Self::static_expr(filter, modifier));
            }
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
        match p.take("AddAllCreatureTypes").as_deref() {
            None => {}
            Some("True") => {
                out.push(Self::static_expr(filter, "Modifier::AllCreatureTypes"));
            }
            Some(_) => return None,
        }
        match p.take("RemoveLandTypes").as_deref() {
            None => {}
            Some("True") => {
                let raw = p.take("AddType")?;
                let modifier = if raw.trim() == "ChosenType" {
                    self.has_chosen_basic_land_type()
                        .then(|| "Modifier::SetLandTypeToChosen".to_string())?
                } else if raw.trim() == "ChosenType2" {
                    self.body
                        .enter_modifiers
                        .iter()
                        .any(|m| m == "EnterModifier::ChooseTwoBasicLandTypes")
                        .then(|| "Modifier::SetLandTypeToSecondChosen".to_string())?
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
        if filter == "Filter::This"
            && allow_self_shortcut
            && condition.contains("Condition::ChosenMode(")
            && self.wave18b_try_self_conditional_grants(
                p,
                triggers.as_deref(),
                statics.as_deref(),
                condition,
                self_granted,
            )?
        {
            return Some(());
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
        let prev_on_a_spell = std::mem::replace(&mut self.on_a_spell, false);
        let parsed = self.parse_activated_spec(&line, false, false);
        self.on_a_spell = prev_on_a_spell;
        let parsed = parsed?;
        if parsed.non_battlefield_zone {
            return self.deny("a granted ability activated outside the battlefield".to_string());
        }
        if !parsed.is_loyalty && parsed.extras.is_empty() {
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
        let prev_block_half = self.block_half.take();
        let prev_block_one = std::mem::replace(&mut self.block_line_one_or_more, false);
        let prev_on_a_spell = std::mem::replace(&mut self.on_a_spell, false);
        let prev_in_granted_trigger = std::mem::replace(&mut self.in_granted_trigger, true);
        let ok = self.triggered(&line);
        self.in_granted_trigger = prev_in_granted_trigger;
        self.on_a_spell = prev_on_a_spell;
        self.trigger_mode = prev_trigger_mode;
        let block_ok =
            ok.is_some() && self.block_line.is_none() && self.block_halves_paired().is_some();
        if !block_ok || self.body.abilities.len() != prev_abilities_len + 1 {
            self.body.abilities.truncate(prev_abilities_len);
            self.body.statics.truncate(prev_statics_len);
            self.block_line = prev_block_line;
            self.block_half = prev_block_half;
            self.block_line_one_or_more = prev_block_one;
            if ok.is_some() && self.block_line.is_some() {
                return self.deny("a granted trigger that is not a single ability".to_string());
            }
            return None;
        }
        self.block_line = prev_block_line;
        self.block_half = prev_block_half;
        self.block_line_one_or_more = prev_block_one;
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
            let ok = match self.try_extra_static(&line) {
                super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
                super::saga_ward_convoke::ExtraOutcome::Pass => self.static_ability(&line),
            };
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

    /// Lowers a single `Animate` / `AnimateAll` `staticAbilities$` `SVar` reference
    /// into one or more `Modifier::GrantStatic` expressions. Excludes
    /// `Mode$ CantRegenerate` so `regeneration.rs` keeps its narrow contract.
    pub(super) fn grant_static_modifiers_for_animate(&mut self, name: &str) -> Option<Vec<String>> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.contains([',', '&']) {
            return None;
        }
        let line = self.svars.get(trimmed)?.clone();
        let trimmed_line = line.trim_start();
        if !trimmed_line.starts_with("Mode$")
            || trimmed_line
                .strip_prefix("Mode$")
                .is_some_and(|rest| rest.trim_start().starts_with("CantRegenerate"))
        {
            return None;
        }
        let prev_abilities_len = self.body.abilities.len();
        let prev_statics_len = self.body.statics.len();
        let prev_block_line = self.block_line.take();
        let ok = match self.try_extra_static(&line) {
            super::saga_ward_convoke::ExtraOutcome::Handled(res) => res,
            super::saga_ward_convoke::ExtraOutcome::Pass => self.static_ability(&line),
        };
        let block_seen = self.block_line.is_some();
        self.block_line = prev_block_line;
        if ok.is_none() || block_seen || self.body.abilities.len() <= prev_abilities_len {
            self.body.abilities.truncate(prev_abilities_len);
            self.body.statics.truncate(prev_statics_len);
            return None;
        }
        let emitted = self.body.abilities.split_off(prev_abilities_len);
        let mut modifiers = Vec::with_capacity(emitted.len());
        for expr in emitted {
            let Some((inner_filter, inner_modifier)) = split_plain_static_ability(&expr) else {
                self.body.statics.truncate(prev_statics_len);
                return None;
            };
            if inner_modifier.contains("Modifier::GrantStatic")
                || inner_filter.contains("Filter::InZone(")
            {
                self.body.statics.truncate(prev_statics_len);
                return None;
            }
            let inner_filter_ref = if simple_const_filter(inner_filter) {
                format!("&{inner_filter}")
            } else {
                format!(
                    "&{}",
                    self.body.filter_static("GRANTED_STATIC", inner_filter)
                )
            };
            modifiers.push(format!(
                "Modifier::GrantStatic {{ filter: {inner_filter_ref}, modifier: &{inner_modifier} }}"
            ));
        }
        Some(modifiers)
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
            let trimmed = raw.trim();
            if key == "SetColor" && trimmed == "Colorless" {
                out.push(Self::static_expr(
                    filter,
                    "Modifier::SetColor(ColorSet::EMPTY)",
                ));
                continue;
            }
            if trimmed == "All" {
                out.push(Self::static_expr(
                    filter,
                    &format!(
                        "Modifier::{modifier}(ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black, Color::Red, Color::Green]))"
                    ),
                ));
                continue;
            }
            let mut colors = Vec::new();
            for word in trimmed.split_whitespace().filter(|&w| w != "&") {
                colors.push(match word {
                    "White" => "Color::White",
                    "Blue" => "Color::Blue",
                    "Black" => "Color::Black",
                    "Red" => "Color::Red",
                    "Green" => "Color::Green",
                    // `Colorless` on `AddColor` is meaningless and `ChosenColor`
                    // is a choice this rule cannot make.
                    other => {
                        self.note(format!("static ability setting colour `{other}`"));
                        return None;
                    }
                });
            }
            if colors.is_empty() {
                self.note(format!("static ability setting colour `{raw}`"));
                return None;
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

    /// `S: Mode$ RaiseCost`: delegates plain existing battlefield/stack shapes to
    /// `final_portals::mana_life_tax`, and extends spell/ability/mana-life taxes
    /// with `Secondary$ True`, `EffectZone$ Command`/`Battlefield,Command`,
    /// omitted `ValidCard$` on `Type$ Spell` (e.g. *God-Pharaoh's Statue*),
    /// `PlayerTurn$`, ability-word flags, `Activator$` on battlefield ability
    /// taxes, and general `PayLife<N>` mana-ability taxes.
    fn mode_raise_cost(&mut self, mut p: Params) -> Option<()> {
        if p.peek("OnlyFirstSpell") == Some("True") {
            return self.wave27d_first_spell_cost_adjust(p, true);
        }
        let hum_of_the_radix = p.peek("Relative") == Some("True")
            && p.peek("Amount").is_some_and(|a| {
                self.svars
                    .get(a.trim())
                    .is_some_and(|d| d.trim().starts_with("Count$Valid "))
            });
        let dynamic_spell_tax = p.peek("Type") == Some("Spell")
            && !p.has("Cost")
            && p.peek("Amount")
                .is_some_and(|a| a.trim().parse::<u8>().is_err());
        if !p.has("Secondary")
            && !p.has("PlayerTurn")
            && !p.has("Affected")
            && !p.has("ForEachShard")
            && (hum_of_the_radix || (p.peek("Relative") != Some("True") && !dynamic_spell_tax))
            && !matches!(
                p.peek("Activator"),
                Some("Player.!Active" | "Player.Active")
            )
            && (matches!(p.peek("EffectZone"), None | Some("Battlefield" | "Stack"))
                || (p.peek("EffectZone") == Some("All")
                    && p.peek("Type") == Some("Ability")
                    && p.peek("ValidSpell") == Some("Activated.ManaAbility")
                    && p.peek("ValidCard") == Some("Card.Self")
                    && p.peek("Cost") == Some("PayLife<1>")))
            && p.has("Type")
            && matches!(p.peek("ValidSpell"), None | Some("Activated.ManaAbility"))
            && (p.peek("Type") != Some("Spell") || p.has("ValidCard") || p.has("ValidTarget"))
            && p.peek("ValidTarget").is_none_or(|vt| {
                Self::raise_cost_target_filter(vt, p.peek("EffectZone") == Some("Stack")).is_some()
            })
            && (p.peek("Type") != Some("Ability")
                || matches!(p.peek("Activator"), None | Some("Player")))
            && (p.peek("ValidSpell") != Some("Activated.ManaAbility")
                || p.peek("Cost") == Some("PayLife<1>"))
        {
            let saved_p = p.clone();
            let saved_statics = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            if self.mana_life_tax(p).is_some() {
                return Some(());
            }
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            if let Some(res) = self.wave17c_try_raise_cost_extra(saved_p) {
                return res.ok();
            }
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let saved_w17c_p = p.clone();
        let saved_w17c_statics = self.body.statics.len();
        let saved_w17c_unclaimed = self.unclaimed.borrow().clone();
        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        }
        if p.has("ForEachShard") {
            return self.raise_cost_shard_sacrifice(p);
        }
        if p.peek("Type") == Some("Ability")
            && p.peek("ValidSpell") == Some("Activated.ManaAbility")
        {
            return self.raise_cost_mana_life(p);
        }
        let legacy_res = if ((p.peek("Type") == Some("Ability")
            && matches!(p.peek("ValidSpell"), None | Some("Activated")))
            || (p.peek("Type").is_none() && p.peek("ValidSpell") == Some("Activated")))
            && !p.has("Cost")
        {
            self.raise_cost_ability(p)
        } else if p.peek("Type") == Some("Spell")
            || (p.peek("Type").is_none() && p.peek("ValidSpell") == Some("Spell"))
        {
            self.raise_cost_spell(p)
        } else {
            None
        };
        if legacy_res.is_some() {
            return Some(());
        }
        self.body.statics.truncate(saved_w17c_statics);
        *self.unclaimed.borrow_mut() = saved_w17c_unclaimed;
        if let Some(res) = self.wave17c_try_raise_cost_extra(saved_w17c_p) {
            return res.ok();
        }
        self.deny("static ability `S: Mode$ RaiseCost`".into())
    }

    fn raise_cost_shard_sacrifice(&mut self, mut p: Params) -> Option<()> {
        let Some(ty) = p.take("Type") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let modifier_name = match ty.as_str() {
            "Spell" => "SpellsCostSacrificePerManaSymbol",
            "Ability" => "AbilitiesCostSacrificePerManaSymbol",
            _ => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
        };
        let zone = match p.take("EffectZone").as_deref() {
            None | Some("Battlefield" | "All") => StaticEffectZone::Battlefield,
            other => match Self::static_effect_zone(other, false) {
                Some(z) => z,
                None => return self.deny("static ability `S: Mode$ RaiseCost`".into()),
            },
        };
        let Some(shard_raw) = p.take("ForEachShard") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(color) = super::color_const(shard_raw.trim()) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(inner) = cost_raw
            .trim()
            .strip_prefix("Sac<")
            .and_then(|s| s.strip_suffix('>'))
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let mut parts = inner.split('/');
        let (Some(count), Some(sac_valid)) = (parts.next(), parts.next()) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let _desc = parts.next();
        if count.trim() != "1" || parts.next().is_some() || sac_valid.trim().is_empty() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        let Some(valid_card) =
            Self::apply_reduce_activator(&raw_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let sac_expr = self.filter_expr(sac_valid.trim())?;
        let sac_static = self.body.filter_static("SAC_COST", &sac_expr);
        let filter = self.filter_expr(&valid_card)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::{modifier_name} {{ color: {color}, sacrifice: &{sac_static} }}"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn raise_cost_mana_life(&mut self, mut p: Params) -> Option<()> {
        p.take("Type");
        p.take("ValidSpell");
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        if p.take("ValidCard").as_deref() != Some("Card.Self") {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let Some(cost) = p.take("Cost") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(amount) = cost
            .strip_prefix("PayLife<")
            .and_then(|s| s.strip_suffix('>'))
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        self.body.abilities.push(Self::format_static_expr(
            "Filter::This",
            &format!("Modifier::ManaAbilitiesCostLife({amount})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn raise_cost_ability(&mut self, mut p: Params) -> Option<()> {
        p.take("Type");
        let valid_spell = p.take("ValidSpell");
        let affected_zone = p.take("AffectedZone");
        let zone_ok = matches!(
            (valid_spell.as_deref(), affected_zone.as_deref()),
            (Some("Activated"), None | Some("Battlefield")) | (None, Some("Battlefield"))
        );
        if !zone_ok {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(valid_card) =
            Self::apply_reduce_activator(&raw_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let Some(amount) = p
            .take("Amount")
            .and_then(|a| a.trim().parse::<u32>().ok())
            .filter(|&n| n > 0)
        else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let filter = self.filter_expr(&valid_card)?;
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            &format!("Modifier::AbilitiesCostMore({amount})"),
            zone,
            &cond,
        ));
        Some(())
    }

    fn raise_cost_spell(&mut self, mut p: Params) -> Option<()> {
        if p.take("Type").is_none() {
            p.take("ValidSpell");
        }
        let effect_zone = p.take("EffectZone");
        let valid_target = p.take("ValidTarget");
        let stack_self_target = effect_zone.as_deref() == Some("Stack")
            && valid_target.as_deref() == Some("Spell.Self")
            && !p.has("ValidCard")
            && !p.has("Affected");
        let self_stack_tax = matches!(effect_zone.as_deref(), Some("All" | "Stack"))
            && valid_target.is_none()
            && !stack_self_target
            && matches!(p.peek("Activator"), None | Some("You"))
            && (p.peek("ValidCard") == Some("Card.Self")
                || (!p.has("ValidCard") && p.peek("Affected") == Some("Card.Self")));
        let zone = if stack_self_target || self_stack_tax {
            StaticEffectZone::Explicit("ZoneRef::Stack")
        } else if let Some(z) = Self::static_effect_zone(effect_zone.as_deref(), false) {
            z
        } else {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        };
        if p.peek("Relative") == Some("True")
            || (!p.has("Cost")
                && p.peek("Amount")
                    .is_some_and(|a| a.trim().parse::<u8>().is_err()))
        {
            if valid_target.is_none() && !stack_self_target {
                let saved_p = p.clone();
                let saved_statics = self.body.statics.len();
                let saved_unclaimed = self.unclaimed.borrow().clone();
                if self
                    .adjust_spell_cost_by_cast(p, true, self_stack_tax, zone)
                    .is_some()
                {
                    return Some(());
                }
                self.body.statics.truncate(saved_statics);
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                if self
                    .adjust_spell_cost_by_amount(saved_p, true, self_stack_tax, zone, true)
                    .is_some()
                {
                    return Some(());
                }
            }
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let target_filter = match valid_target.as_deref() {
            None => None,
            Some(raw) => Some(self.resolve_raise_cost_target_filter(raw, stack_self_target)?),
        };
        let (raw_card, act_filter) = if self_stack_tax {
            p.take("Activator");
            let raw = p
                .take("ValidCard")
                .or_else(|| p.take("Affected"))
                .unwrap_or_else(|| "Card.Self".to_string());
            (raw, None)
        } else {
            let raw = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw == "Card.Self" {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            }
            let Some(resolved) = Self::apply_spell_activator(&raw, p.take("Activator").as_deref())
            else {
                return self.deny("static ability `S: Mode$ RaiseCost`".into());
            };
            resolved
        };
        if self_stack_tax
            && !p.has("Amount")
            && p.peek("Cost")
                .is_some_and(|cost| super::printed_mana(cost).is_none())
        {
            return self.raise_cost_self_mandatory_non_mana(p);
        }
        let modifier = Self::raise_cost_amount_modifier(p.take("Amount"), p.take("Cost"))
            .or_else(|| self.deny("static ability `S: Mode$ RaiseCost`".into()))?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let base_filter = if self_stack_tax {
            "Filter::This".to_string()
        } else {
            self.filter_expr(&raw_card)?
        };
        let filter = Self::conjoin_filters(
            base_filter,
            target_filter
                .into_iter()
                .chain(act_filter.map(str::to_string)),
        );
        self.body
            .abilities
            .push(Self::format_static_expr(&filter, &modifier, zone, &cond));
        Some(())
    }

    fn resolve_raise_cost_target_filter(
        &mut self,
        raw: &str,
        stack_self_target: bool,
    ) -> Option<String> {
        Self::raise_cost_target_filter(raw, stack_self_target)
            .map(str::to_string)
            .or_else(|| {
                (!stack_self_target)
                    .then(|| self.wave28c_raise_cost_target_filter(raw))
                    .flatten()
            })
            .or_else(|| self.deny("static ability `S: Mode$ RaiseCost`".into()))
    }

    fn raise_cost_target_filter(
        valid_target: &str,
        stack_self_target: bool,
    ) -> Option<&'static str> {
        match valid_target {
            "Spell.Self" if stack_self_target => Some("Filter::SpellTargetingAffected"),
            "Card.Self" | "Creature.Self" | "Permanent.Self" if !stack_self_target => {
                Some("Filter::SpellTargetingAffected")
            }
            "Permanent.YouCtrl"
            | "Permanent.YouCtrl+inZoneBattlefield"
            | "Card.YouCtrl+inZoneBattlefield"
                if !stack_self_target =>
            {
                Some("Filter::TargetsPermanentYouControl")
            }
            _ => None,
        }
    }

    fn raise_cost_amount_modifier(amount: Option<String>, cost: Option<String>) -> Option<String> {
        match (amount, cost) {
            (Some(amount), None) => {
                let amount = amount.trim().parse::<u8>().ok().filter(|&n| n > 0)?;
                Some(format!("Modifier::SpellsCostMore({amount})"))
            }
            (None, Some(cost)) => {
                let mana = super::printed_mana(&cost)?;
                if let Some(amount) = super::generic_mana(&mana) {
                    Some(format!("Modifier::SpellsCostMore({amount})"))
                } else {
                    Some(format!("Modifier::SpellsCostMoreMana(mana!(\"{mana}\"))"))
                }
            }
            _ => None,
        }
    }

    fn raise_cost_self_mandatory_non_mana(&mut self, mut p: Params) -> Option<()> {
        let cost = p.take("Cost")?;
        if !p.exhausted() || !self.body.mandatory_additional_costs.is_empty() {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let has_mana_token = super::cost_parts(&cost).iter().any(|token| {
            token.chars().all(|c| c.is_ascii_digit())
                || matches!(
                    token.as_str(),
                    "W" | "U" | "B" | "R" | "G" | "C" | "S" | "X"
                )
                || super::hybrid_pair(token).is_some()
                || super::mana::special_mana_symbol(token).is_some()
        });
        if has_mana_token {
            return self.deny("static ability `S: Mode$ RaiseCost`".into());
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(parts) = self.parse_spell_cost_parts(&cost) {
            self.body.mandatory_additional_costs.extend(parts);
            return Some(());
        }
        (*self.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
        self.body.statics.truncate(saved_statics_len);
        if let Some(part) = self.parse_discard_x_spell_cost(&cost) {
            self.body.mandatory_additional_costs.push(part);
            return Some(());
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics_len);
        self.deny("static ability `S: Mode$ RaiseCost`".into())
    }

    fn is_plain_battlefield_reduce_cost(p: &Params) -> bool {
        !p.has("Condition")
            && !p.has("IsPresent")
            && !p.has("CheckSVar")
            && !p.has("PlayerTurn")
            && !p.has("Secondary")
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
    }

    /// `S: Mode$ ReduceCost`: battlefield and self-stack (`ZoneRef::Stack`)
    /// spell cost reductions (fixed generic, dynamic `Amount`, fixed mana
    /// `Color$` / `IgnoreGeneric$`, face-down, shared creature type), plus
    /// `CyclingCostsLess` (`ValidSpell$ Activated.Cycling`).
    #[allow(clippy::too_many_lines)]
    fn mode_reduce_cost(&mut self, mut p: Params) -> Option<()> {
        if p.peek("OnlyFirstSpell") == Some("True") {
            return self.wave27d_first_spell_cost_adjust(p, false);
        }
        // Keep `final_portals::reduce_cost` handling plain battlefield fixed
        // generic spell reductions unchanged.
        if Self::is_plain_battlefield_reduce_cost(&p) {
            return self.reduce_cost(p);
        }

        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        }
        if p.peek("ValidSpell") == Some("Static.Foretelling") || p.peek("Type") == Some("Foretell")
        {
            return self.wave17c_reduce_cost_foretell(p);
        }
        if p.peek("ValidSpell") == Some("Static.Unlock") {
            return self.wave17c_reduce_cost_unlock(p);
        }
        if p.peek("ValidSpell") == Some("Spell.Bargain") {
            return self.wave17c_reduce_cost_bargain(p);
        }
        if p.peek("ValidSpell") == Some("Activated.Cycling") {
            return self.reduce_cost_cycling(p);
        }
        if p.peek("ValidSpell") == Some("Activated.Equip") {
            return self.reduce_cost_equip(p);
        }
        if p.peek("ValidSpell") == Some("Activated.Exhaust") {
            return self.wave27d_activated_abilities_cost_less(p, true);
        }
        if p.peek("ValidSpell") == Some("Spell.Buyback") {
            return self.reduce_cost_buyback(p);
        }

        let spell_type = p.take("Type");
        let valid_spell = p.take("ValidSpell");
        if matches!(
            (spell_type.as_deref(), valid_spell.as_deref()),
            (Some("Ability"), None | Some("Activated")) | (None, Some("Activated"))
        ) {
            return self.reduce_cost_ability(p);
        }
        let mut blitz_only = false;
        let face_down = match (spell_type.as_deref(), valid_spell.as_deref()) {
            (Some("Spell"), None | Some("Spell")) | (None, Some("Spell")) => false,
            (Some("Spell") | None, Some("Spell.Blitz")) => {
                blitz_only = true;
                false
            }
            (Some("Spell") | None, Some("Spell.isCastFaceDown")) => true,
            _ => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let (is_self_zone, zone) = match p.take("EffectZone").as_deref() {
            Some("All")
                if !self.is_spell_card_type()
                    && p.peek("Relative") == Some("True")
                    && p.peek("ValidCard").is_some_and(|vc| vc != "Card.Self")
                    && !self.oracle.to_ascii_lowercase().contains("graveyard")
                    && !self.oracle.to_ascii_lowercase().contains("command zone") =>
            {
                (false, StaticEffectZone::Battlefield)
            }
            Some("All" | "Stack") => (true, StaticEffectZone::Explicit("ZoneRef::Stack")),
            raw => match Self::static_effect_zone(raw, false) {
                Some(z) => (false, z),
                None => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            },
        };
        let target_filter = if p.has("ValidTarget") {
            if is_legacy_reduce_cost_target(&p, is_self_zone, face_down, zone) {
                Some(self.reduce_cost_target_filter(&mut p, is_self_zone, face_down, zone)?)
            } else {
                return self.reduce_cost_if_targets(p, is_self_zone, face_down, zone);
            }
        } else {
            None
        };
        if p.peek("Relative") == Some("True") {
            if face_down || target_filter.is_some() {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            return self
                .adjust_spell_cost_by_cast(p, false, is_self_zone, zone)
                .or_else(|| self.deny("static ability `S: Mode$ ReduceCost`".into()));
        }
        if !face_down && target_filter.is_none() && p.has("Color") && p.peek("Amount") != Some("1")
        {
            return self.wave17c_reduce_cost_colored_by_amount(
                p,
                spell_type.as_deref(),
                valid_spell.as_deref(),
                is_self_zone,
                zone,
            );
        }
        let adjust_fallback =
            (!face_down && target_filter.is_none() && !p.has("Color")).then(|| {
                (
                    p.clone(),
                    self.body.statics.len(),
                    self.unclaimed.borrow().clone(),
                )
            });
        let (valid_card, act_filter) = self.resolve_reduce_cost_card(
            &mut p,
            spell_type.as_deref(),
            valid_spell.as_deref(),
            is_self_zone,
            face_down,
        )?;
        let mut cond = self.reduce_cost_condition(&mut p)?;
        let modifier = if is_self_zone
            && cond.is_empty()
            && !face_down
            && target_filter.is_none()
            && !p.has("Color")
            && p.peek("Amount").is_some_and(|raw| {
                let raw = raw.trim();
                raw == "Count$StartingPlayer.0.1"
                    || self
                        .svars
                        .get(raw)
                        .is_some_and(|d| d.trim() == "Count$StartingPlayer.0.1")
            }) {
            p.take("Amount");
            cond = ", condition = Some(Condition::NotStartingPlayer)".to_string();
            "Modifier::SpellsCostLess(1)".to_string()
        } else {
            let Some(modifier) = self.reduce_cost_modifier(&mut p) else {
                if let Some((saved_p, saved_statics, saved_unclaimed)) = adjust_fallback {
                    return self.try_reduce_cost_dynamic_fallback(
                        saved_p,
                        saved_statics,
                        saved_unclaimed,
                        is_self_zone,
                        zone,
                    );
                }
                return None;
            };
            modifier
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let base_filter = self.filter_expr(&valid_card)?;
        let filter = Self::conjoin_filters(
            base_filter,
            target_filter
                .into_iter()
                .map(str::to_string)
                .chain(act_filter.map(str::to_string))
                .chain(blitz_only.then(|| "Filter::CastForBlitz".to_string())),
        );
        self.body
            .abilities
            .push(Self::format_static_expr(&filter, &modifier, zone, &cond));
        Some(())
    }

    fn try_reduce_cost_dynamic_fallback(
        &mut self,
        saved_p: Params,
        saved_statics: usize,
        saved_unclaimed: Option<String>,
        is_self_zone: bool,
        zone: StaticEffectZone,
    ) -> Option<()> {
        let after_statics = self.body.statics.len();
        let after_unclaimed = self.unclaimed.borrow().clone();
        self.body.statics.truncate(saved_statics);
        (*self.unclaimed.borrow_mut()).clone_from(&saved_unclaimed);
        if self
            .adjust_spell_cost_by_cast(saved_p.clone(), false, is_self_zone, zone)
            .is_some()
        {
            return Some(());
        }
        self.body.statics.truncate(saved_statics);
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        if self
            .adjust_spell_cost_by_amount(saved_p, false, is_self_zone, zone, false)
            .is_some()
        {
            return Some(());
        }
        self.body.statics.truncate(after_statics);
        *self.unclaimed.borrow_mut() = after_unclaimed;
        None
    }

    fn resolve_reduce_cost_card(
        &self,
        p: &mut Params,
        spell_type: Option<&str>,
        valid_spell: Option<&str>,
        is_self_zone: bool,
        face_down: bool,
    ) -> Option<(String, Option<&'static str>)> {
        let mut valid_card = match p.take("ValidCard") {
            Some(vc) => vc,
            None if matches!(valid_spell, None | Some("Spell"))
                && matches!(spell_type, None | Some("Spell"))
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
            return Some((valid_card, None));
        }
        if valid_card == "Card.Self" {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        if face_down {
            valid_card = Self::append_reduce_atom(&valid_card, "faceDown");
        }
        let Some((with_act, act_filter)) =
            Self::apply_spell_activator(&valid_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        Some((with_act, act_filter))
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
        let amt_expr = self.static_count_expr_ext(&amt_def)?;
        Some(format!("Modifier::SpellsCostLessByAmount({amt_expr})"))
    }

    fn adjust_spell_cost_by_cast(
        &mut self,
        mut p: Params,
        increase: bool,
        is_self_zone: bool,
        zone: StaticEffectZone,
    ) -> Option<()> {
        let relative = match p.take("Relative").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let (outer_filter, who, inner_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You")) {
                return None;
            }
            let raw_card = p.take("ValidCard").or_else(|| p.take("Affected"))?;
            if raw_card != "Card.Self" {
                return None;
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let raw_act = p.take("Activator");
            let mut who = match raw_act.as_deref() {
                None | Some("Player") => "PlayerRel::EachPlayer",
                Some("You") => "PlayerRel::You",
                Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                    "PlayerRel::EnchantedPlayer"
                }
                Some(_) => return None,
            };
            let mut raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return None;
            }
            if relative
                && raw_act.is_none()
                && let Some(stripped) = Self::wave17c_strip_all_you_ctrl(&raw_card)
            {
                who = "PlayerRel::You";
                raw_card = stripped;
            }
            // In `Relative$ True` Forge statics, `YouCtrl`/`OppCtrl`/`YouOwn`/`OppOwn` inside
            // `ValidCard$` bind to the casting player, whereas `Modifier::AdjustSpellCostByCast`'s
            // `filter` evaluates relative to the static's controller; refuse when those differ.
            if relative
                && who != "PlayerRel::You"
                && (raw_card.contains("Ctrl") || raw_card.contains("Own"))
            {
                return None;
            }
            let expr = self.filter_expr_in_zone(&raw_card, true)?;
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let amt_raw = p.take("Amount")?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let (metric, factor) =
            self.parse_spell_cast_metric(&amt_raw, relative, is_self_zone, who)?;
        let delta_per_unit = if increase { factor } else { -factor };
        let modifier = format!(
            "Modifier::AdjustSpellCostByCast {{ who: {who}, filter: {inner_filter}, delta_per_unit: {delta_per_unit}, metric: {metric} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn parse_spell_cast_metric(
        &mut self,
        amt_raw: &str,
        relative: bool,
        is_self_zone: bool,
        who: &str,
    ) -> Option<(String, i8)> {
        let mut expr = amt_raw.trim().to_string();
        let mut factor: i32 = 1;
        for _ in 0..8 {
            if let Some(base) = expr.strip_suffix("/Twice") {
                factor = factor.checked_mul(2)?;
                expr = base.trim().to_string();
                continue;
            }
            if let Some((base, mult_str)) = expr.rsplit_once("/Times.") {
                let mult: i32 = mult_str.trim().parse().ok()?;
                if mult <= 0 {
                    return None;
                }
                factor = factor.checked_mul(mult)?;
                expr = base.trim().to_string();
                continue;
            }
            if expr.contains('/') {
                return None;
            }
            if let Some(var) = expr.strip_prefix("SVar$") {
                let def = self.svars.get(var.trim())?.trim().to_string();
                expr = def;
                continue;
            }
            if !expr.contains('$') {
                let def = self.svars.get(expr.as_str())?.trim().to_string();
                expr = def;
                continue;
            }
            break;
        }
        if expr.contains('/') {
            return None;
        }
        let factor = i8::try_from(factor).ok().filter(|&n| n > 0)?;
        let per_turn_ok = relative || who == "PlayerRel::You";
        let metric = match expr.as_str() {
            "TargetedObjectsDistinct$Amount" if relative => {
                "SpellCastMetric::DistinctTargets".to_string()
            }
            "Count$xPaid" if relative && is_self_zone => "SpellCastMetric::XPaid".to_string(),
            _ if relative
                && is_self_zone
                && self.body.optional_cost.is_some()
                && expr
                    .strip_prefix("Count$OptionalGenericCostPaid.")
                    .and_then(|s| s.strip_suffix(".0"))
                    .and_then(|n| n.parse::<i32>().ok())
                    .is_some_and(|n| n > 0) =>
            {
                let n: i32 = expr
                    .strip_prefix("Count$OptionalGenericCostPaid.")?
                    .strip_suffix(".0")?
                    .parse()
                    .ok()?;
                let scaled = i8::try_from(i32::from(factor).checked_mul(n)?).ok()?;
                return Some(("SpellCastMetric::OptionalCostPaid".to_string(), scaled));
            }
            "PlayerCountPropertyYou$CardsDiscardedThisTurn" if per_turn_ok => {
                "SpellCastMetric::CardsDiscardedThisTurn".to_string()
            }
            "Count$LifeYouGainedThisTurn" if per_turn_ok => {
                "SpellCastMetric::LifeGainedThisTurn".to_string()
            }
            "Count$LifeOppsLostThisTurn" if per_turn_ok => {
                "SpellCastMetric::LifeLostByOpponentsThisTurn".to_string()
            }
            "Count$ThisTurnCast_Card.YouCtrl" if per_turn_ok => {
                "SpellCastMetric::SpellsCastThisTurn".to_string()
            }
            "Count$YouDrewThisTurn" if per_turn_ok => {
                "SpellCastMetric::CardsDrawnThisTurn".to_string()
            }
            other => {
                if let Some(m) = self.wave17c_distinct_targets_matching_metric(other, relative) {
                    return Some((m, factor));
                }
                let sac_raw = other
                    .strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ")
                    .filter(|_| per_turn_ok)?
                    .trim();
                if sac_raw.is_empty() || sac_raw.contains('$') || sac_raw.contains('/') {
                    return None;
                }
                let sac_expr = self.filter_expr(sac_raw)?;
                let sac_ref = if simple_const_filter(&sac_expr) {
                    format!("&{sac_expr}")
                } else {
                    format!("&{}", self.body.filter_static("SAC_METRIC", &sac_expr))
                };
                format!("SpellCastMetric::PermanentsSacrificedThisTurn({sac_ref})")
            }
        };
        Some((metric, factor))
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
        if p.has("ValidTarget") {
            return self.wave17c_reduce_cost_equip_if_targets(p);
        }
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
        if p.has("ValidTarget") {
            return self.reduce_cost_ability_if_targets(p);
        }
        if p.peek("Amount")
            .is_some_and(|a| a.trim().parse::<u32>().is_err())
        {
            return self.wave27d_activated_abilities_cost_less(p, false);
        }
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

    fn reduce_cost_if_targets(
        &mut self,
        mut p: Params,
        is_self_zone: bool,
        face_down: bool,
        zone: StaticEffectZone,
    ) -> Option<()> {
        if face_down || p.peek("Relative") == Some("True") {
            return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
        }
        let raw_target = p.take("ValidTarget")?;
        let (outer_filter, who, spell_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You"))
                || p.take("ValidCard").as_deref() != Some("Card.Self")
            {
                return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let who = match p.take("Activator").as_deref() {
                None | Some("Player") => "PlayerRel::EachPlayer",
                Some("You") => "PlayerRel::You",
                Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                    "PlayerRel::EnchantedPlayer"
                }
                Some(_) => return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into()),
            };
            let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let expr = self.filter_expr_in_zone(&raw_card, true)?;
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let target_filter = self.parse_target_cost_filter(&raw_target, is_self_zone)?;
        let reduction = self.parse_fixed_reduction_mana(&mut p)?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let modifier = format!(
            "Modifier::ReduceCostIfTargets {{ who: {who}, spell_filter: {spell_filter}, target_filter: {target_filter}, reduction: {reduction}, abilities: false }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn reduce_cost_ability_if_targets(&mut self, mut p: Params) -> Option<()> {
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if !matches!(
            p.take("AffectedZone").as_deref(),
            None | Some("Battlefield")
        ) || !matches!(p.take("MinMana").as_deref(), None | Some("0"))
            || p.peek("Relative") == Some("True")
        {
            return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
        }
        let who = match p.take("Activator").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some(_) => return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into()),
        };
        let Some(raw_card) = p.take("ValidCard") else {
            return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
        };
        let raw_target = p.take("ValidTarget")?;
        let spell_expr = self.filter_expr(&raw_card)?;
        let spell_filter = if simple_const_filter(&spell_expr) {
            format!("&{spell_expr}")
        } else {
            format!("&{}", self.body.filter_static("COST_SPELL", &spell_expr))
        };
        let target_filter = self.parse_target_cost_filter(&raw_target, false)?;
        let reduction = self.parse_fixed_reduction_mana(&mut p)?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let modifier = format!(
            "Modifier::ReduceCostIfTargets {{ who: {who}, spell_filter: {spell_filter}, target_filter: {target_filter}, reduction: {reduction}, abilities: true }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    pub(super) fn parse_target_cost_filter(
        &mut self,
        raw_target: &str,
        forbid_self: bool,
    ) -> Option<String> {
        let mut normalized = Vec::new();
        for raw_alt in raw_target.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
            }
            let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
            let base = base.trim();
            if matches!(
                base,
                "Player"
                    | "Opponent"
                    | "You"
                    | "TargetedPlayer"
                    | "Any"
                    | "Object"
                    | "SpellAbility"
            ) {
                return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
            }
            let mut has_in_zone = false;
            for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if forbid_self && atom == "Self" {
                    return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
                }
                if atom.starts_with("inZone") {
                    has_in_zone = true;
                }
            }
            if forbid_self && base == "Self" {
                return self.deny("unclaimed parameter `ReduceCost.ValidTarget`".into());
            }
            if base == "Permanent" && !has_in_zone {
                if qual.is_empty() {
                    normalized.push("Permanent.inZoneBattlefield".to_string());
                } else {
                    normalized.push(format!("{alt}+inZoneBattlefield"));
                }
            } else {
                normalized.push(alt.to_string());
            }
        }
        let joined = normalized.join(",");
        let expr = self.filter_expr(&joined)?;
        Some(if simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("TARGET_COST", &expr))
        })
    }

    fn parse_fixed_reduction_mana(&self, p: &mut Params) -> Option<String> {
        if let Some(color_raw) = p.take("Color") {
            if p.take("Amount").as_deref() != Some("1")
                || !matches!(p.take("IgnoreGeneric").as_deref(), None | Some("False"))
            {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            let Some(mana) = super::printed_mana(&color_raw) else {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            };
            return Some(format!("mana!(\"{mana}\")"));
        }
        let Some(amt_raw) = p.take("Amount") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Ok(n) = amt_raw.trim().parse::<u32>() else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if n == 0 {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        Some(format!("mana!(\"{{{n}}}\")"))
    }

    /// `S: Mode$ ReduceCost` / `RaiseCost` with dynamic `Amount$` ->
    /// `Modifier::AdjustSpellCostByAmount { who, filter, increase, amount }`.
    fn adjust_spell_cost_by_amount(
        &mut self,
        mut p: Params,
        increase: bool,
        is_self_zone: bool,
        zone: StaticEffectZone,
        allow_base_static_count: bool,
    ) -> Option<()> {
        if p.peek("Relative") == Some("True") {
            return None;
        }
        p.take("Relative");
        let (outer_filter, who, inner_filter) = if is_self_zone {
            if !matches!(p.take("Activator").as_deref(), None | Some("You")) {
                return None;
            }
            let raw_card = p.take("ValidCard").or_else(|| p.take("Affected"))?;
            if raw_card != "Card.Self" {
                return None;
            }
            (
                "Filter::This",
                "PlayerRel::You",
                "&Filter::This".to_string(),
            )
        } else {
            let who = match p.take("Activator").as_deref() {
                None | Some("Player") => "PlayerRel::EachPlayer",
                Some("You") => "PlayerRel::You",
                Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
                Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
                    "PlayerRel::EnchantedPlayer"
                }
                Some(_) => return None,
            };
            let raw_card = p.take("ValidCard").unwrap_or_else(|| "Card".to_string());
            if raw_card == "Card.Self" {
                return None;
            }
            let expr = self.filter_expr_in_zone(&raw_card, true)?;
            let inner = if simple_const_filter(&expr) {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COST_SPELL", &expr))
            };
            ("Filter::Any", who, inner)
        };
        let amt_raw = p.take("Amount")?;
        let amt_def = if amt_raw.starts_with("Count$") || amt_raw.starts_with("SVar$") {
            amt_raw
        } else {
            self.svars.get(amt_raw.trim())?.clone()
        };
        let amt_expr = self
            .wave14_static_amount(&amt_def)
            .or_else(|| allow_base_static_count.then(|| self.static_count_expr_ext(&amt_def))?)
            .or_else(|| self.adjust_spell_cost_extra_amount(&amt_def, is_self_zone))?;
        let cond = self.reduce_cost_condition(&mut p)?;
        if !p.exhausted() {
            return None;
        }
        let modifier = format!(
            "Modifier::AdjustSpellCostByAmount {{ who: {who}, filter: {inner_filter}, increase: {increase}, amount: &{amt_expr} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            outer_filter,
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn adjust_spell_cost_extra_amount(&mut self, def: &str, is_self_zone: bool) -> Option<String> {
        let trimmed = def.trim();
        if trimmed == "PlayerCount$SacrificedThisTurn Permanent" {
            return Some(
                "Amount::PermanentsSacrificedThisTurn { who: PlayerRel::EachPlayer, filter: &Filter::Any }"
                    .to_string(),
            );
        }
        if trimmed == "Count$AttackersDeclared" {
            return Some(
                "Amount::CreaturesAttackedThisTurn { filter: &Filter::Any, controller: PlayerRel::EachPlayer }"
                    .to_string(),
            );
        }
        if !is_self_zone {
            return None;
        }
        if trimmed == "Count$ThisTurnCast_Card.OppCtrl" {
            return Some("Amount::SpellsCastThisTurn(PlayerRel::EachOpponent)".to_string());
        }
        if let Some(valid) = trimmed.strip_prefix("PlayerCountOpponents$HighestValid ") {
            return self.wave17b_highest_controlled_amount("PlayerRel::EachOpponent", valid);
        }
        None
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
        let mut graveyard_zone = false;
        let mut implicit_graveyard_filter = false;
        let explicit_battlefield = if allow_affected_zone {
            let affected_zone = p.take("AffectedZone");
            if forbid_unscoped_card && affected_zone.as_deref() == Some("Graveyard") {
                graveyard_zone = true;
                false
            } else {
                implicit_graveyard_filter = forbid_unscoped_card && affected_zone.is_none();
                zones_ok &= matches!(affected_zone.as_deref(), None | Some("Battlefield"));
                affected_zone.as_deref() == Some("Battlefield")
            }
        } else {
            false
        };
        if !zones_ok || !matches!(p.take("Type").as_deref(), None | Some("Ability")) {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(mut raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if implicit_graveyard_filter
            && let Some(stripped) = Self::strip_in_zone_graveyard(&raw_card)
        {
            raw_card = stripped;
            graveyard_zone = true;
        }
        if forbid_unscoped_card && raw_card == "Card" && !explicit_battlefield && !graveyard_zone {
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
        let filter = if graveyard_zone {
            let base = self.filter_expr_in_zone(&valid_card, true)?;
            if base == "Filter::Any" {
                "Filter::InZone(ZoneRef::Graveyard)".to_string()
            } else {
                format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::Graveyard)])")
            }
        } else {
            self.filter_expr(&valid_card)?
        };
        Some((filter, amount, minimum, zone, cond))
    }

    pub(super) fn append_reduce_atom(valid: &str, atom: &str) -> String {
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

    pub(super) fn strip_in_zone_graveyard(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for raw_alt in valid.split(',') {
            let alt = raw_alt.trim();
            let (base, atoms_str) = alt.split_once('.')?;
            let mut found_gy = false;
            let mut kept = Vec::new();
            for atom in atoms_str.split('+').filter(|a| !a.is_empty()) {
                if atom == "inZoneGraveyard" {
                    found_gy = true;
                } else if atom.starts_with("inZone") {
                    return None;
                } else {
                    kept.push(atom);
                }
            }
            if !found_gy {
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

    fn apply_spell_activator(
        valid: &str,
        activator: Option<&str>,
    ) -> Option<(String, Option<&'static str>)> {
        match activator {
            Some("Player.!Active") => Some((
                valid.to_string(),
                Some("Filter::Not(&Filter::ControlledByActivePlayer)"),
            )),
            Some("Player.Active") => {
                Some((valid.to_string(), Some("Filter::ControlledByActivePlayer")))
            }
            other => Self::apply_reduce_activator(valid, other).map(|v| (v, None)),
        }
    }

    fn conjoin_filters<S: Into<String>>(
        base: String,
        extras: impl IntoIterator<Item = S>,
    ) -> String {
        let mut parts = Vec::new();
        if base != "Filter::Any" {
            parts.push(base);
        }
        for extra in extras {
            parts.push(extra.into());
        }
        match parts.len() {
            0 => "Filter::Any".to_string(),
            1 => parts.remove(0),
            _ => format!("Filter::And(&[{}])", parts.join(", ")),
        }
    }

    pub(super) fn apply_reduce_activator(valid: &str, activator: Option<&str>) -> Option<String> {
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

    pub(super) fn reduce_cost_condition(&mut self, p: &mut Params) -> Option<String> {
        let mut clauses = Vec::new();
        Self::take_ability_word_flags(p, &mut clauses)?;
        if let Some(pt) = p.take("PlayerTurn") {
            if !matches!(pt.as_str(), "True" | "You") {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            clauses.push("Condition::YourTurn".to_string());
        }
        if let Some(cond_raw) = p.take("Condition") {
            let Some(cond) = Self::named_condition(&cond_raw)
                .or_else(|| Self::wave21_named_condition(&cond_raw))
            else {
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
        if let Some(state_cond) = self.take_self_state_present(p).ok()? {
            clauses.push(state_cond);
        }
        if let Some(phases) = p.peek("Phases")
            && let Some(phase_cond) = static_phase_condition(phases)
        {
            p.take("Phases");
            clauses.push(phase_cond.to_string());
        }
        self.take_extended_present_clauses(p, &mut clauses)?;
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
            let Some(clause) = self
                .reduce_cost_svar_condition(&var, &cmp)
                .or_else(|| self.cost_adjust_svar_condition(&var, &cmp))
            else {
                return self.deny("unclaimed parameter `ReduceCost.CheckSVar`".to_string());
            };
            clauses.push(clause);
            if let Some(var2) = p.take("CheckSecondSVar") {
                let cmp2 = p
                    .take("SecondSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let Some(clause2) = self
                    .reduce_cost_svar_condition(&var2, &cmp2)
                    .or_else(|| self.cost_adjust_svar_condition(&var2, &cmp2))
                else {
                    return self.deny("unclaimed parameter `ReduceCost.CheckSVar`".to_string());
                };
                clauses.push(clause2);
            }
        }
        Some(match clauses.as_slice() {
            [] => String::new(),
            [one] => format!(", condition = Some({one})"),
            many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
        })
    }

    /// `CheckSVar$` + `SVarCompare$` clauses specific to `ReduceCost`/`RaiseCost`
    /// (`Count$Void.*`, `Count$Left(Battlefield|Graveyard)ThisTurn`, and non-subtype
    /// `PlayerCountPropertyYou$SacrificedThisTurn <Valid>`). Kept outside
    /// `reduce_cost_svar_condition` so `Mode$ Continuous` preserves its existing
    /// `BadSacCardType` refusal boundary.
    fn cost_adjust_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let trimmed = var.trim();
        let def = if trimmed.starts_with("Count$") || trimmed.starts_with("PlayerCount") {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.trim().to_string()
        };
        let def = def.trim();
        if let Some(cond) = Self::wave16a_void_condition(def, cmp) {
            return Some(cond);
        }
        let bound = super::count_bound(cmp)?;
        if let Some(valid) = def.strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ") {
            let valid = valid.trim();
            if self.cats.const_path(valid).is_none()
                && !has_relative_turn_atom(valid)
                && let Some(amt) = self.wave16a_sacrificed_this_turn_amount(valid)
                && amt.contains("who: PlayerRel::You")
            {
                return format_amount_bound(&amt, bound);
            }
            return None;
        }
        let (valid, battlefield) =
            if let Some(valid) = def.strip_prefix("Count$LeftBattlefieldThisTurn ") {
                (valid.trim(), true)
            } else {
                let valid = def.strip_prefix("Count$LeftGraveyardThisTurn ")?;
                (valid.trim(), false)
            };
        if super::wave16a::has_unmodelled_turn_atom(valid, false)
            || has_relative_turn_atom(valid)
            || (!battlefield
                && valid
                    .split(',')
                    .any(|alt| alt.split(['.', '+']).any(|a| a.trim().ends_with("Ctrl"))))
        {
            return None;
        }
        let amt = self.wave29b_left_zone_this_turn_amount(valid, battlefield)?;
        format_amount_bound(&amt, bound)
    }

    fn reduce_cost_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        if let Some(c) = self
            .svar_condition_clause(var, cmp)
            .or_else(|| self.promised_gift_svar_condition(var, cmp))
        {
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
        if def == "Count$StartingPlayer.0.1" {
            return match cmp.trim() {
                "GE1" | "EQ1" => Some("Condition::NotStartingPlayer".to_string()),
                "EQ0" | "LT1" => Some("Condition::Not(&Condition::NotStartingPlayer)".to_string()),
                _ => None,
            };
        }
        if let Some(sub) = def.strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ") {
            let bound = super::count_bound(cmp)?;
            if let Some(path) = self.cats.const_path(sub.trim()) {
                return match bound {
                    super::Bound::AtLeast(n) if n >= 1 => Some(format!(
                        "Condition::SacrificedSubtypeThisTurn {{ subtype: {path}, count: {n} }}"
                    )),
                    super::Bound::AtMost(n) => {
                        let threshold = u32::from(n).saturating_add(1);
                        Some(format!(
                            "Condition::Not(&Condition::SacrificedSubtypeThisTurn {{ subtype: {path}, count: {threshold} }})"
                        ))
                    }
                    super::Bound::AtLeast(_) => None,
                };
            }
            return None;
        }
        if let Some(valid) = def.strip_prefix("Count$Valid ")
            && !valid.is_empty()
            && !valid.contains('/')
            && !valid.contains('$')
        {
            if Self::is_relative_you_ctrl_present(valid) {
                return self.relative_you_ctrl_present_clause(valid, cmp);
            }
            if Self::is_opp_ctrl_present(valid) {
                return self.opp_ctrl_present_clause(valid, cmp);
            }
        }
        if let Some((op, rhs_var)) = parse_svar_amount_compare(cmp)
            && let Some(rhs_def) = self.svars.get(rhs_var).cloned()
            && let Some(left) = self
                .static_count_expr(&def)
                .or_else(|| self.wave14_condition_amount(&def))
            && let Some(right) = self
                .static_count_expr(&rhs_def)
                .or_else(|| self.wave14_condition_amount(&rhs_def))
        {
            return Some(format!(
                "Condition::AmountCompare {{ left: &{left}, op: AmountCmp::{op}, right: &{right} }}"
            ));
        }
        if let Some(clause) = self.wave14_svar_condition_clause(&def, cmp) {
            return Some(clause);
        }
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
            return format_amount_bound(&count, bound);
        }
        None
    }

    fn wave14_svar_condition_clause(&mut self, def: &str, cmp: &str) -> Option<String> {
        let amt = self.wave14_condition_amount(def)?;
        if let Some(bound) = super::count_bound(cmp) {
            return format_amount_bound(&amt, bound);
        }
        if def != "Count$ThisTurnCast_Card.YouCtrl" {
            let trimmed_cmp = cmp.trim();
            if let Some(n) = trimmed_cmp
                .strip_prefix("EQ")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&n| n >= 1)
            {
                if def == "Count$Party" && n == 4 {
                    return Some(format!("Condition::AmountAtLeast({amt}, 4)"));
                }
                return Some(format!(
                    "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::Equal, right: &Amount::Fixed({n}) }}"
                ));
            }
            if let Some(n) = trimmed_cmp
                .strip_prefix("NE")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&n| n >= 1)
            {
                return Some(format!(
                    "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::NotEqual, right: &Amount::Fixed({n}) }}"
                ));
            }
        } else if let Some(clause) = Self::wave28c_this_turn_cast_compare(def, cmp) {
            return Some(clause);
        }
        None
    }

    pub(super) fn reduce_cost_zone_present(
        &mut self,
        p: &mut Params,
        valid: &str,
    ) -> Option<String> {
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
            || chain.target_range.is_some()
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

    /// `S: Mode$ CantPutCounter` (`Blightbeetle`, `Melira, Sylvok Outcast`,
    /// `Solemnity`, `Melira's Keepers`, `Tatterkite`): permanent or player
    /// counter prohibition lowered to `ReplacementRule::ModifyCounterPlacement`
    /// or `ModifyPlayerCounterPlacement` with `CounterReplaceOp::Sub(u16::MAX)`.
    fn cant_put_counter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(
                p.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
        {
            return self.deny("static ability `S: Mode$ CantPutCounter`".to_string());
        }
        let kind_opt = match p.take("CounterType") {
            None => "None".to_string(),
            Some(raw) => {
                let Some(kind) = super::counter_kind(raw.trim()) else {
                    return self.deny("static ability `S: Mode$ CantPutCounter`".to_string());
                };
                format!("Some({kind})")
            }
        };
        let valid_card = p.take("ValidCard");
        let valid_player = p.take("ValidPlayer");
        if !p.exhausted() {
            return self.deny("static ability `S: Mode$ CantPutCounter`".to_string());
        }
        match (valid_card, valid_player) {
            (Some(raw_card), None) => {
                let Some(cleaned) = strip_in_zone_battlefield(&raw_card) else {
                    return self.deny("static ability `S: Mode$ CantPutCounter`".to_string());
                };
                let filter_ref = if cleaned == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&cleaned)?;
                    format!("&{}", self.body.filter_static("COUNTER_TARGET", &expr))
                };
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement {{ \
                     object_filter: {filter_ref}, kind: {kind_opt}, placer: None, \
                     effect_only: false, op: CounterReplaceOp::Sub(u16::MAX) }})"
                ));
                Some(())
            }
            (None, Some(raw_player)) => {
                let who = match raw_player.trim() {
                    "You" => "PlayerRel::You",
                    "Opponent" | "Player.Opponent" => "PlayerRel::EachOpponent",
                    "Player" => "PlayerRel::EachPlayer",
                    "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                        "PlayerRel::EnchantedPlayer"
                    }
                    _ => return self.deny("static ability `S: Mode$ CantPutCounter`".to_string()),
                };
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement {{ \
                     recipient: {who}, kind: {kind_opt}, placer: None, \
                     op: CounterReplaceOp::Sub(u16::MAX) }})"
                ));
                Some(())
            }
            _ => self.deny("static ability `S: Mode$ CantPutCounter`".to_string()),
        }
    }

    fn is_relative_you_ctrl_present(valid: &str) -> bool {
        let mut saw_other = false;
        for alt in valid.split(',') {
            let mut has_you_ctrl = false;
            for atom in alt
                .split(['.', '+'])
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                match atom {
                    "YouCtrl" => has_you_ctrl = true,
                    "Other" | "StrictlyOther" => saw_other = true,
                    "Self" | "EnchantedBy" | "EquippedBy" | "AttachedBy" | "Attached" => {
                        return false;
                    }
                    other if other.contains("Ctrl") || other.contains("Own") => return false,
                    _ => {}
                }
            }
            if !has_you_ctrl {
                return false;
            }
        }
        saw_other
    }

    fn relative_you_ctrl_present_clause(&mut self, valid: &str, compare: &str) -> Option<String> {
        let compare = compare.trim();
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("CHECK", &expr);
        let count = format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}");
        if compare == "EQ1" {
            let at_least_two = format!("Condition::AmountAtLeast({count}, 2)");
            let two_name = self.body.condition_static("CHECK", &at_least_two);
            return Some(format!(
                "Condition::All(&[Condition::AmountAtLeast({count}, 1), Condition::Not(&{two_name})])"
            ));
        }
        match count_bound(compare)? {
            Bound::AtLeast(n) if n >= 1 => Some(format!("Condition::AmountAtLeast({count}, {n})")),
            Bound::AtMost(n) => {
                let threshold = u32::from(n).checked_add(1)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({count}, {threshold}))"
                ))
            }
            Bound::AtLeast(_) => None,
        }
    }

    fn is_opp_ctrl_present(valid: &str) -> bool {
        let mut saw_alt = false;
        for raw_alt in valid.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return false;
            }
            saw_alt = true;
            let mut has_opp_ctrl = false;
            for atom in alt
                .split(['.', '+'])
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if matches!(
                    atom,
                    "Self"
                        | "Other"
                        | "StrictlyOther"
                        | "EnchantedBy"
                        | "EquippedBy"
                        | "AttachedBy"
                        | "Attached"
                ) {
                    return false;
                }
                if atom == "OppCtrl" {
                    has_opp_ctrl = true;
                } else if atom.contains("Ctrl") || atom.contains("Own") {
                    return false;
                }
            }
            if !has_opp_ctrl {
                return false;
            }
        }
        saw_alt
    }

    fn opp_ctrl_present_clause(&mut self, valid: &str, compare: &str) -> Option<String> {
        let bound = count_bound(compare)?;
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("CHECK", &expr);
        match bound {
            Bound::AtLeast(n) if n >= 1 => {
                Some(format!("Condition::BattlefieldCount(&{name}, {n})"))
            }
            Bound::AtMost(n) => Some(format!("Condition::BattlefieldCountAtMost(&{name}, {n})")),
            Bound::AtLeast(_) => None,
        }
    }

    /// Consumes source-relative `Other+YouCtrl` and opponent-controlled
    /// battlefield `IsPresent$` clauses, `PresentZone$ Hand` clauses, and
    /// paired `IsPresent2$` clauses before falling back to `present_clause`.
    fn take_extended_present_clauses(
        &mut self,
        p: &mut Params,
        clauses: &mut Vec<String>,
    ) -> Option<()> {
        if p.has("NoResolvingCheck") || !p.has("IsPresent") {
            return Some(());
        }
        let second = Self::extract_second_present(p);
        if let Some(clause) = self
            .try_take_extended_present(p, "PresentCompare$", false)
            .ok()?
        {
            clauses.push(clause);
        }
        if let Some(mut p2) = second {
            if let Some(state_cond) = self.take_self_state_present(&mut p2).ok()? {
                clauses.push(state_cond);
            } else if let Some(clause) = self
                .try_take_extended_present(&mut p2, "PresentCompare2$", true)
                .ok()?
            {
                clauses.push(clause);
            } else if let Some(valid2) = p2.take("IsPresent") {
                clauses.push(self.present_clause(&mut p2, &valid2)?);
            }
            if !p2.exhausted() {
                return self.deny("a second `IsPresent2$` clause".to_string());
            }
        }
        Some(())
    }

    fn extract_second_present(p: &mut Params) -> Option<Params> {
        if !p.has("IsPresent2") {
            return None;
        }
        let mut entries = Vec::new();
        for (src_key, dst_key) in [
            ("IsPresent2", "IsPresent"),
            ("PresentCompare2", "PresentCompare"),
            ("PresentZone2", "PresentZone"),
            ("PresentPlayer2", "PresentPlayer"),
            ("PresentDefined2", "PresentDefined"),
        ] {
            if let Some(val) = p.take(src_key) {
                entries.push((dst_key.to_string(), val));
            }
        }
        Some(Params {
            entries,
            opponents_only_lure: false,
        })
    }

    fn try_take_extended_present(
        &mut self,
        p: &mut Params,
        cmp_label: &str,
        allow_graveyard: bool,
    ) -> Result<Option<String>, ()> {
        let Some(valid) = p.peek("IsPresent").map(str::to_string) else {
            return Ok(None);
        };
        if matches!(p.peek("PresentZone"), None | Some("Battlefield"))
            && p.peek("PresentPlayer").is_none()
            && p.peek("PresentDefined").is_none()
        {
            if Self::is_relative_you_ctrl_present(&valid) {
                p.take("IsPresent");
                p.take("PresentZone");
                let compare = p
                    .take("PresentCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let Some(clause) = self.relative_you_ctrl_present_clause(&valid, &compare) else {
                    self.note(format!("`{cmp_label} {compare}`"));
                    return Err(());
                };
                return Ok(Some(clause));
            }
            if Self::is_opp_ctrl_present(&valid) {
                p.take("IsPresent");
                p.take("PresentZone");
                let compare = p
                    .take("PresentCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let Some(clause) = self.opp_ctrl_present_clause(&valid, &compare) else {
                    self.note(format!("`{cmp_label} {compare}`"));
                    return Err(());
                };
                return Ok(Some(clause));
            }
        }
        let zone_matches = p.peek("PresentZone") == Some("Hand")
            || (allow_graveyard && p.peek("PresentZone") == Some("Graveyard"));
        if zone_matches
            && matches!(p.peek("PresentPlayer"), None | Some("You"))
            && !matches!(valid.as_str(), "Card.StrictlySelf" | "Card.Self")
        {
            p.take("IsPresent");
            return self.reduce_cost_zone_present(p, &valid).map(Some).ok_or(());
        }
        Ok(None)
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
    pub(super) fn strip_ai_hint_add_svar(&self, p: &mut Params) -> bool {
        let Some(raw) = p.peek("AddSVar") else {
            return false;
        };
        let mut any = false;
        for name in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            any = true;
            let Some(def) = self.svars.get(name) else {
                return false;
            };
            if !(is_pure_ai_hint_svar(def)
                || (is_pure_ai_hint_name(name) && is_scalar_ai_hint_value(def)))
            {
                return false;
            }
        }
        if any {
            p.take("AddSVar");
        }
        any
    }

    /// `S: Mode$ Continuous | GainsAbilitiesOf$ ...` ->
    /// `Modifier::GainActivatedAbilitiesOf { filter, zone }` (`CR 613.1f`).
    fn gain_activated_abilities_modifier(
        &mut self,
        p: &mut Params,
        affected: &str,
        filter: &str,
        effect_zone: Option<&'static str>,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if !p.has("GainsAbilitiesOf") {
            return Some(());
        }
        match p.take("GainsValidAbilities").as_deref() {
            None | Some("Activated") => {}
            Some(other) => {
                return self.deny(format!("`Continuous.GainsValidAbilities$ {other}`"));
            }
        }
        let raw_donor = p.take("GainsAbilitiesOf")?;
        let raw_zone = p.take("GainsAbilitiesOfZones");
        let (donor_expr, donor_zone) = match raw_zone.as_deref() {
            None | Some("Battlefield") => {
                if p.has("TopCardOfLibraryIs") {
                    return self.deny("unclaimed parameter `Continuous.TopCardOfLibraryIs`".into());
                }
                if donor_has_unsupported_bf_atoms(&raw_donor, affected == "Card.Self") {
                    return self.deny(format!("`Continuous.GainsAbilitiesOf$ {raw_donor}`"));
                }
                let expr = self.filter_expr(&raw_donor)?;
                (expr, "AbilityDonorZone::Battlefield".to_string())
            }
            Some("Graveyard") => {
                if p.has("TopCardOfLibraryIs") {
                    return self.deny("unclaimed parameter `Continuous.TopCardOfLibraryIs`".into());
                }
                let Some((owner, stripped)) = strip_strict_zone_owner(&raw_donor) else {
                    return self.deny(format!("`Continuous.GainsAbilitiesOf$ {raw_donor}`"));
                };
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                let who = owner.unwrap_or("PlayerRel::EachPlayer");
                (expr, format!("AbilityDonorZone::Graveyard({who})"))
            }
            Some("Library") => {
                let Some(stripped) = strip_top_of_your_library(&raw_donor) else {
                    return self.deny(format!("`Continuous.GainsAbilitiesOf$ {raw_donor}`"));
                };
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                if let Some(top_is) = p.take("TopCardOfLibraryIs") {
                    let top_expr = self.filter_expr_in_zone(&top_is, true)?;
                    if top_expr != expr {
                        return self.deny(format!("`Continuous.TopCardOfLibraryIs$ {top_is}`"));
                    }
                }
                (expr, "AbilityDonorZone::TopOfYourLibrary".to_string())
            }
            Some("Exile") => {
                if p.has("TopCardOfLibraryIs") {
                    return self.deny("unclaimed parameter `Continuous.TopCardOfLibraryIs`".into());
                }
                self.gain_activated_exile_donor(&raw_donor, effect_zone)?
            }
            Some(other) => {
                return self.deny(format!("`Continuous.GainsAbilitiesOfZones$ {other}`"));
            }
        };
        let donor_ref = if simple_const_filter(&donor_expr) {
            format!("&{donor_expr}")
        } else {
            format!("&{}", self.body.filter_static("DONOR", &donor_expr))
        };
        out.push(Self::static_expr(
            filter,
            &format!(
                "Modifier::GainActivatedAbilitiesOf {{ filter: {donor_ref}, zone: {donor_zone} }}"
            ),
        ));
        Some(())
    }

    fn gain_activated_exile_donor(
        &mut self,
        raw_donor: &str,
        effect_zone: Option<&'static str>,
    ) -> Option<(String, String)> {
        let mentions_link = raw_donor.split(',').any(|alt| {
            alt.split(['.', '+']).any(|a| {
                matches!(
                    a.trim(),
                    "ExiledWithSource" | "ExiledWith" | "IsImprinted" | "ChosenCard"
                )
            })
        });
        if mentions_link {
            if effect_zone.is_some() {
                return self.deny("linked-exile `GainsAbilitiesOf$` outside battlefield".into());
            }
            let Some(stripped) = strip_imprinted_linked_exile(raw_donor) else {
                return self.deny(
                    "linked-exile `GainsAbilitiesOf$` requires an `ExileTargetsWithSource` producer"
                        .into(),
                );
            };
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            return Some((expr, "AbilityDonorZone::LinkedExile".to_string()));
        }
        let Some((owner, stripped)) = strip_strict_zone_owner(raw_donor) else {
            return self.deny(format!("`Continuous.GainsAbilitiesOf$ {raw_donor}`"));
        };
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let who = owner.unwrap_or("PlayerRel::EachPlayer");
        Some((expr, format!("AbilityDonorZone::Exile({who})")))
    }

    /// `S: Mode$ Continuous | MayPlay$ True | (MayPlayAltManaCost$ | MayPlayWithoutManaCost$)` ->
    /// `Modifier::MayCastWithAltCost` (`CR 118.9`, `CR 601.3e`).
    fn may_cast_with_alt_cost(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        p.take("MayPlay");
        let price = self.take_alt_cast_price(&mut p)?;
        let who = match p.take("MayPlayPlayer").as_deref() {
            None | Some("You") => "PlayerRel::You",
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent") => "PlayerRel::EachOpponent",
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlayPlayer$ {other}`"));
            }
        };
        let once_per_turn = match p.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlayLimit$ {other}`"));
            }
        };
        let with_flash = match p.take("MayPlayWithFlash").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlayWithFlash$ {other}`"));
            }
        };
        let dont_grant_zone = match p.take("MayPlayDontGrantZonePermissions").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!(
                    "`Continuous.MayPlayDontGrantZonePermissions$ {other}`"
                ));
            }
        };
        let Some(affected) = p.take("Affected") else {
            return self.deny("`MayPlay` static with no `Affected$`".into());
        };
        let Some(raw_zones) = p.take("AffectedZone") else {
            return self.deny("`MayPlay` static with no `AffectedZone$`".into());
        };
        let has_linked_exile_producer = self.body.abilities.iter().any(|a| {
            a.contains("ReplacementRule::ExileInsteadOfGraveyard")
                || a.contains("ExileTargetsWithSource")
                || a.contains("ExileMatchingWithSource")
                || a.contains("ExileTopLinked")
                || a.contains("RevealUntilFound::ExileLinked")
                || a.contains("link_to_source: true")
        });
        let Some(zone_plan) = plan_alt_cast_zones(
            &raw_zones,
            &affected,
            dont_grant_zone,
            who,
            effect_zone,
            has_linked_exile_producer,
        ) else {
            return self.deny(format!(
                "static ability reaching `AffectedZone$ {raw_zones}`"
            ));
        };
        let (card_filter, max_mv_expr, allow_lands) =
            self.parse_alt_cast_affected(&affected, who, dont_grant_zone, &zone_plan)?;
        if allow_lands
            && (price.starts_with("AltCastPrice::NormalPlus(")
                || (once_per_turn && price != "AltCastPrice::Normal"))
        {
            return self.deny("`MayPlayLimit$` combined with land-play permission".into());
        }
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let static_zone = match effect_zone {
            None => StaticEffectZone::Battlefield,
            Some(z) => StaticEffectZone::Explicit(z),
        };
        let modifier = format!(
            "Modifier::MayCastWithAltCost {{ who: {who}, zones: {}, grant_zone_permission: {}, allow_lands: {allow_lands}, with_flash: {with_flash}, max_mana_value: {max_mv_expr}, once_per_turn: {once_per_turn}, price: {price} }}",
            zone_plan.zones_expr, zone_plan.grant_zone_permission
        );
        if zone_plan.zones_expr == "AltCastZones::LinkedExile" {
            for ability in &mut self.body.abilities {
                if ability.contains("ReplacementRule::ExileInsteadOfGraveyard") {
                    *ability = ability.replace("link_to_source: false", "link_to_source: true");
                }
            }
        }
        self.body.abilities.push(Self::format_static_expr(
            &card_filter,
            &modifier,
            static_zone,
            &condition,
        ));
        Some(())
    }

    fn take_alt_cast_price(&mut self, p: &mut Params) -> Option<String> {
        let raw_alt = p.take("MayPlayAltManaCost");
        let raw_free = p.take("MayPlayWithoutManaCost");
        let raw_raise = p.take("RaiseCost");
        if raw_raise.is_some() && (raw_alt.is_some() || raw_free.is_some()) {
            return self.deny("`RaiseCost$` combined with an alternative mana cost".into());
        }
        match (
            raw_alt.as_deref(),
            raw_free.as_deref(),
            raw_raise.as_deref(),
        ) {
            (None, Some("True"), None) => Some("AltCastPrice::Mana(ManaCost::ZERO)".to_string()),
            (Some("PayLife<ConvertedManaCost>"), None, None) => {
                Some("AltCastPrice::PayLifeEqualToManaValue".to_string())
            }
            (Some(cost), None, None) => parse_fixed_alt_mana_price(cost)
                .or_else(|| self.deny(format!("`Continuous.MayPlayAltManaCost$ {cost}`"))),
            (Some(_), Some(_), _) => {
                self.deny("both `MayPlayAltManaCost$` and `MayPlayWithoutManaCost$`".into())
            }
            (None, Some(other), _) => {
                self.deny(format!("`Continuous.MayPlayWithoutManaCost$ {other}`"))
            }
            (None, None, Some(raw_cost)) => {
                let cost = self.normal_plus_cost_expr(raw_cost)?;
                Some(format!("AltCastPrice::NormalPlus({cost})"))
            }
            (None, None, None) => Some("AltCastPrice::Normal".to_string()),
            (_, _, Some(_)) => {
                self.deny("`RaiseCost$` combined with an alternative mana cost".into())
            }
        }
    }

    fn normal_plus_cost_expr(&mut self, raw: &str) -> Option<String> {
        let Some((mana, parts)) = self.cost_pieces(raw) else {
            return self.deny(format!("`Continuous.RaiseCost$ {raw}`"));
        };
        let paid = |part: &String| {
            part.starts_with("PayLife(")
                || part.starts_with("DealDamageToYou(")
                || part.starts_with("PayEnergy(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("ExilePermanent(")
                || part.starts_with("ExileFromHand(")
                || part.starts_with("ExileFromGraveyard(")
                || part.starts_with("ExileTopOfLibrary(")
                || part.starts_with("Discard(")
                || part == "DiscardRandom"
                || part == "DiscardHand"
                || part.starts_with("TapOther(")
                || part.starts_with("TapN {")
                || part.starts_with("RemoveAnyCounter {")
                || part == "Forage"
                || part.starts_with("Behold(")
                || part.starts_with("BeholdAndExile(")
                || part.starts_with("CollectEvidence(")
        };
        if (mana.is_empty() && parts.is_empty()) || !parts.iter().all(paid) {
            return self.deny(format!("`Continuous.RaiseCost$ {raw}`"));
        }
        Some(crate::body::cost_literal(&mana, &parts))
    }

    fn parse_alt_cast_affected(
        &mut self,
        affected: &str,
        who: &str,
        dont_grant_zone: bool,
        plan: &AltCastZonePlan,
    ) -> Option<(String, String, bool)> {
        let mut cleaned_alts = Vec::new();
        let mut shared_cmc: Option<Option<String>> = None;
        let mut all_allow_lands = plan.grant_zone_permission;
        for raw_alt in affected.split(',') {
            let Some((cleaned, alt_cmc, saw_nonland)) =
                clean_alt_cast_alt(raw_alt, who, dont_grant_zone, plan)
            else {
                return self.deny(format!("`MayPlay` `Affected$ {affected}`"));
            };
            if saw_nonland {
                all_allow_lands = false;
            }
            match &shared_cmc {
                None => shared_cmc = Some(alt_cmc),
                Some(prev) if *prev == alt_cmc => {}
                Some(_) => {
                    return self.deny(format!("`MayPlay` `Affected$ {affected}`"));
                }
            }
            cleaned_alts.push(cleaned);
        }
        let max_mv_expr = match shared_cmc.flatten() {
            None => "None".to_string(),
            Some(rhs) => {
                let amt = if let Ok(n) = rhs.parse::<u32>() {
                    format!("Amount::Fixed({n})")
                } else {
                    let Some(def) = self.svars.get(rhs.as_str()).cloned() else {
                        return self.deny(format!("`MayPlay` `cmcLE{rhs}` naming no SVar"));
                    };
                    let Some(expr) = self.static_count_expr(&def) else {
                        return self.deny(format!("`MayPlay` `cmcLE{rhs}` count `{def}`"));
                    };
                    expr
                };
                format!("Some({amt})")
            }
        };
        let cleaned = cleaned_alts.join(",");
        let card_filter = self.filter_expr_in_zone(&cleaned, true)?;
        Some((card_filter, max_mv_expr, all_allow_lands))
    }

    /// Cross-zone Layer 4/5 static abilities (`AffectedZone$ All`, e.g.
    /// *Maskwood Nexus*, *Arcane Adaptation*, *Leyline of Transformation*,
    /// *Biotransference*, *Dune Chanter*, *Encroaching Mycosynth*, *Mycosynth
    /// Lattice*, *Roshan, Hidden Magister*, *Rukarumel, Biologist*).
    /// Strictly refuses P/T changes, keywords, `RemoveCreatureTypes`, and
    /// `ChosenColor`.
    fn all_zones_layer45_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        if p.has("AffectedDefined") || p.has("RemoveLandTypes") || p.has("RemoveCreatureTypes") {
            return self.deny("static ability reaching `AffectedZone$ All`".to_string());
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ All`".to_string());
        };
        let filter = if let Some(stripped) = strip_all_zones_you_ctrl(&affected) {
            let base = self.filter_expr_in_zone(&stripped, true)?;
            let scope = "Filter::Or(&[Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Battlefield)]), Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Stack)]), Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::NotBattlefield), Filter::Not(&Filter::InZone(ZoneRef::Stack))])])";
            if base == "Filter::Any" {
                scope.to_string()
            } else {
                format!("Filter::And(&[{base}, {scope}])")
            }
        } else if is_unscoped_all_zones_affected(&affected) {
            let base = self.filter_expr_in_zone(&affected, true)?;
            let scope = "Filter::Or(&[Filter::InZone(ZoneRef::Battlefield), Filter::InZone(ZoneRef::NotBattlefield)])";
            if base == "Filter::Any" {
                scope.to_string()
            } else {
                format!("Filter::And(&[{base}, {scope}])")
            }
        } else {
            return self.deny("static ability reaching `AffectedZone$ All`".to_string());
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        let mut out = Vec::new();
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() || out.is_empty() {
            return self.deny("static ability reaching `AffectedZone$ All`".to_string());
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

    /// Off-battlefield Flash grant (`AffectedZone$ Hand,Graveyard,Exile,Library,Command,Stack`,
    /// e.g. *Teferi, Mage of Zhalfir*), lowered to `Modifier::AddKeyword(KeywordSet::FLASH)`
    /// with `Filter::InZone(ZoneRef::NotBattlefield)`.
    fn offboard_flash_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
        raw_zone: &str,
    ) -> Option<()> {
        if p.has("AffectedDefined")
            || p.take("AddKeyword").as_deref().map(str::trim) != Some("Flash")
        {
            return self.deny(format!(
                "static ability reaching `AffectedZone$ {raw_zone}`"
            ));
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny(format!(
                "static ability reaching `AffectedZone$ {raw_zone}`"
            ));
        };
        let base = self.filter_expr_in_zone(&affected, true)?;
        let filter = if base == "Filter::Any" {
            "Filter::InZone(ZoneRef::NotBattlefield)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::NotBattlefield)])")
        };
        let condition =
            self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() {
            return self.deny(format!(
                "static ability reaching `AffectedZone$ {raw_zone}`"
            ));
        }
        let zone_arg = effect_zone.map_or_else(String::new, |z| format!(", zone = {z}"));
        self.body.abilities.push(format!(
            "static_ability!({filter}, Modifier::AddKeyword(KeywordSet::FLASH){zone_arg}{condition})"
        ));
        Some(())
    }

    /// Non-P/T characteristic-defining abilities on `Card.Self` (or omitted
    /// `Affected$`): unconditional Layer 4/5 color/type CDAs, conditional
    /// self-Flash CDAs, and kicked stack Split Second.
    fn non_pt_cda_static(&mut self, mut p: Params) -> Option<()> {
        p.take("CharacteristicDefining");
        p.take("Affected");
        if p.peek("AddKeyword").map(str::trim) == Some("Split second") {
            if p.peek("IsPresent").map(str::trim) != Some("Card.Self+kicked")
                || p.peek("PresentZone").map(str::trim) != Some("Stack")
                || self.body.additional_costs.is_empty()
            {
                return self.deny("a characteristic-defining ability with no P/T".to_string());
            }
            p.take("AddKeyword");
            p.take("IsPresent");
            p.take("PresentZone");
            if !p.exhausted() {
                return self.deny("a characteristic-defining ability with no P/T".to_string());
            }
            self.body.abilities.push(
                "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::SPLIT_SECOND), \
                 zone = ZoneRef::Stack, condition = Some(Condition::KickedWith(0)))"
                    .to_string(),
            );
            return Some(());
        }
        if p.peek("AddKeyword").map(str::trim) == Some("Flash") {
            p.take("AddKeyword");
            let condition =
                self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
            if !p.exhausted() {
                return self.deny("a characteristic-defining ability with no P/T".to_string());
            }
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::FLASH), \
                 zone = ZoneRef::NotBattlefield{condition})"
            ));
            return Some(());
        }
        if p.has("AddKeyword")
            || p.has("RemoveKeyword")
            || p.has("CantHaveKeyword")
            || p.has("RemoveLandTypes")
            || p.peek("AddType").map(str::trim) == Some("ChosenType")
        {
            return self
                .deny("unclaimed parameter `Continuous.CharacteristicDefining`".to_string());
        }
        let mut out = Vec::new();
        self.type_modifiers(&mut p, "Filter::This", &mut out)?;
        self.color_modifiers(&mut p, "Filter::This", &mut out)?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() || out.is_empty() {
            return self
                .deny("unclaimed parameter `Continuous.CharacteristicDefining`".to_string());
        }
        let is_perm = self.is_permanent_card_pub();
        for ability in out {
            let Some(open) = ability.strip_suffix(')') else {
                return self.deny("unclaimed parameter `Continuous.CharacteristicDefining`".into());
            };
            if is_perm {
                self.body.abilities.push(ability.clone());
            }
            self.body
                .abilities
                .push(format!("{open}, zone = ZoneRef::NotBattlefield)"));
        }
        Some(())
    }

    /// Lowers a single `AddKeyword$` entry that represents a granted static,
    /// prevention, triggered, or activated keyword ability. Returns `Ok(true)`
    /// (`Some(true)`) if handled, `Some(false)` to fall through to
    /// `keyword_const`, and `None` if refused.
    fn try_granted_keyword_ability(
        &mut self,
        word: &str,
        filter: &str,
        allows_equip: bool,
        bits: &mut Vec<String>,
        out: &mut Vec<String>,
    ) -> Option<bool> {
        let trimmed = word.trim();
        if trimmed == "Protection from each color" {
            out.push(Self::static_expr(
                filter,
                "Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[\
                 Color::White, Color::Blue, Color::Black, Color::Red, Color::Green])))",
            ));
            return Some(true);
        }
        if trimmed == "Protection from everything" {
            out.push(Self::static_expr(
                filter,
                "Modifier::ProtectionFrom(&Filter::Any)",
            ));
            return Some(true);
        }
        if let Some(from) = self.protection_filter(word) {
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::ProtectionFrom(&{from})"),
            ));
            return Some(true);
        }
        if let Some(land) = self.landwalk_filter(word) {
            let name = self.body.filter_static("LANDWALK", &land);
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::LandwalkMatching(&{name})"),
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Cumulative upkeep:") {
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
            return Some(true);
        }
        if self.try_granted_triggered_keyword(trimmed, word, filter, bits, out)? {
            return Some(true);
        }
        if let Some(mods) = granted_damage_prevention_modifiers(trimmed) {
            for m in mods {
                out.push(Self::static_expr(filter, m));
            }
            return Some(true);
        }
        self.try_granted_activated_keyword(trimmed, word, filter, allows_equip, bits, out)
    }

    fn try_granted_triggered_keyword(
        &mut self,
        trimmed: &str,
        word: &str,
        filter: &str,
        bits: &mut Vec<String>,
        out: &mut Vec<String>,
    ) -> Option<bool> {
        let mut mods = Vec::new();
        let Some(handled) = self.push_granted_triggered_keyword_modifiers(trimmed, bits, &mut mods)
        else {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        };
        if handled {
            for m in mods {
                out.push(Self::static_expr(filter, &m));
            }
            return Some(true);
        }
        Some(false)
    }

    /// Emits keyword bits and `Modifier` strings for triggered keyword grants
    /// shared between static `AddKeyword$` and `Pump` / `PumpAll` `KW$`.
    pub(super) fn push_granted_triggered_keyword_modifiers(
        &mut self,
        trimmed: &str,
        bits: &mut Vec<String>,
        mods: &mut Vec<String>,
    ) -> Option<bool> {
        if let Some(rest) = trimmed.strip_prefix("Ward:") {
            let effect = self.ward_effect_expr(rest, true)?;
            mods.push(format!(
                "Modifier::GrantTriggered {{ trigger: Trigger::Ward, \
                 effects: &[{effect}], target: None }}"
            ));
            return Some(true);
        }
        if trimmed == "Exalted" {
            mods.push(
                "Modifier::GrantTriggered { \
                 trigger: Trigger::AttacksAlone(&Filter::YOUR_CREATURE), \
                 effects: &[Effect::PumpEventObject { power: Amount::Fixed(1), \
                 toughness: Amount::Fixed(1), keywords: KeywordSet::EMPTY, \
                 duration: Duration::UntilEndOfTurn }], target: None }"
                    .to_string(),
            );
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Annihilator:") {
            let n = rest.trim().parse::<u16>().ok().filter(|&n| n >= 1)?;
            mods.push(format!(
                "Modifier::GrantTriggered {{ trigger: Trigger::Attacks(&Filter::This), \
                 effects: &[Effect::SacrificeAmountForPlayers {{ \
                 who: PlayerRel::DefendingPlayer, filter: &Filter::Any, \
                 amount: Amount::Fixed({n}) }}], target: None }}"
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Afflict:") {
            let n = rest.trim().parse::<u32>().ok().filter(|&n| n >= 1)?;
            bits.push("KeywordSet::AFFLICT".to_string());
            mods.push(format!(
                "Modifier::GrantTriggered {{ trigger: Trigger::BecomesBlocked, \
                 effects: &[Effect::LoseLife {{ amount: Amount::Fixed({n}), \
                 target: PlayerRel::DefendingPlayer }}], target: None }}"
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Afterlife:") {
            let n = rest.trim().parse::<u32>().ok().filter(|&n| n >= 1)?;
            bits.push("KeywordSet::AFTERLIFE".to_string());
            mods.push(format!(
                "Modifier::GrantTriggered {{ trigger: Trigger::Dies(&Filter::This), \
                 effects: &[Effect::CreateTokenN {{ \
                 token: &generated_tokens::SPIRIT_1_1_WHITE_BLACK_FLYING, \
                 amount: Amount::Fixed({n}) }}], target: None }}"
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Poisonous:") {
            let n = rest.trim().parse::<u32>().ok().filter(|&n| n >= 1)?;
            mods.push(format!(
                "Modifier::GrantTriggered {{ \
                 trigger: Trigger::DealsCombatDamageToPlayer(&Filter::This), \
                 effects: &[Effect::Poison {{ who: PlayerRel::DamagedPlayer, \
                 amount: Amount::Fixed({n}) }}], target: None }}"
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Renown:") {
            let n = rest.trim().parse::<u8>().ok().filter(|&n| n >= 1)?;
            bits.push("KeywordSet::RENOWN".to_string());
            let trig_expr = format!(
                "triggered!(Trigger::DealsCombatDamageToPlayer(&Filter::This), \
                 &[Effect::BecomeRenowned({n})], \
                 condition = Some(Condition::Not(&Condition::IsRenowned)))"
            );
            let def_name = self.body.ability_static("GRANTED_TRIGGER_", &trig_expr);
            mods.push(format!("Modifier::GrantTriggeredDefinition(&{def_name})"));
            return Some(true);
        }
        Some(false)
    }

    fn try_granted_activated_keyword(
        &mut self,
        trimmed: &str,
        word: &str,
        filter: &str,
        allows_equip: bool,
        bits: &mut Vec<String>,
        out: &mut Vec<String>,
    ) -> Option<bool> {
        if let Some(rest) = trimmed.strip_prefix("Crew:") {
            let Ok(n) = rest.trim().parse::<u8>() else {
                self.note(format!("static ability granting keyword `{word}`"));
                return None;
            };
            let def_name = self
                .body
                .ability_static("GRANTED_ACTIVATED_", &format!("crew!({n})"));
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::GrantActivatedDefinition(&{def_name})"),
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Saddle:") {
            let Ok(n) = rest.trim().parse::<u8>() else {
                self.note(format!("static ability granting keyword `{word}`"));
                return None;
            };
            let def_name = self
                .body
                .ability_static("GRANTED_ACTIVATED_", &format!("saddle!({n})"));
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::GrantActivatedDefinition(&{def_name})"),
            ));
            return Some(true);
        }
        if let Some(rest) = trimmed.strip_prefix("Outlast:") {
            return self.granted_outlast_ability(rest, word, filter, bits, out);
        }
        if let Some(rest) = trimmed.strip_prefix("Equip:")
            && allows_equip
        {
            return self.granted_equip_ability(rest, word, filter, out);
        }
        Some(false)
    }

    fn granted_outlast_ability(
        &mut self,
        rest: &str,
        word: &str,
        filter: &str,
        bits: &mut Vec<String>,
        out: &mut Vec<String>,
    ) -> Option<bool> {
        let trimmed = rest.trim();
        if trimmed.is_empty() || trimmed.contains([':', '$', '<', '>']) {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        }
        let Some((mana, parts)) = self.cost_pieces(trimmed) else {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        };
        if !parts.is_empty() || mana.is_empty() {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        }
        let cost = crate::body::cost_literal(&mana, &["TapSelf".to_string()]);
        bits.push("KeywordSet::OUTLAST".to_string());
        let ab_expr = format!(
            "activated!({cost}, &[Effect::AddCounterSource {{ kind: CounterKind::P1P1, \
             amount: Amount::Fixed(1) }}], timing = ActivationTiming::SorcerySpeed)"
        );
        let def_name = self.body.ability_static("GRANTED_ACTIVATED_", &ab_expr);
        out.push(Self::static_expr(
            filter,
            &format!("Modifier::GrantActivatedDefinition(&{def_name})"),
        ));
        Some(true)
    }

    fn granted_equip_ability(
        &mut self,
        rest: &str,
        word: &str,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<bool> {
        let fields: Vec<&str> = rest.split(':').collect();
        let cost_raw = fields.first().map_or_default(|s| s.trim());
        if cost_raw.is_empty() || cost_raw.contains(['$', '/']) {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        }
        let Some(cost) = self.cost_expr(cost_raw) else {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        };
        if cost == "Cost::TAP" || cost.contains("TapSelf") {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        }
        let ab_expr = if fields.len() == 1 {
            format!("equip!({cost})")
        } else if fields.len() == 3 {
            let valid = fields[1].trim();
            if valid.is_empty()
                || valid.contains(['$', '/'])
                || fields[2].contains('$')
                || !is_granted_equip_target_valid(valid)
            {
                self.note(format!("static ability granting keyword `{word}`"));
                return None;
            }
            let target_expr = self.filter_expr(valid)?;
            if target_expr != "Filter::YOUR_CREATURE"
                && !target_expr.contains("Filter::ControlledByYou")
            {
                self.note(format!("static ability granting keyword `{word}`"));
                return None;
            }
            let target_name = self.body.filter_static("EQUIP", &target_expr);
            format!(
                "activated!({cost}, &[Effect::AttachSelf {{ target: TargetSpec::Object(&{target_name}) }}], \
                 timing = ActivationTiming::SorcerySpeed, \
                 target = Some(TargetSpec::Object(&{target_name})))"
            )
        } else {
            self.note(format!("static ability granting keyword `{word}`"));
            return None;
        };
        let def_name = self.body.ability_static("GRANTED_ACTIVATED_", &ab_expr);
        out.push(Self::static_expr(
            filter,
            &format!("Modifier::GrantActivatedDefinition(&{def_name})"),
        ));
        Some(true)
    }

    /// Extended static-safe `Amount` expressions local to `statics.rs`:
    /// delegates to `static_count_expr`, plus `Count$ValidGraveyard,Exile`
    /// (*Huskburster Swarm*, *Sailors' Bane*) and `Count$ValidHand Card.ChosenCtrl`
    /// (*Nyxathid*, guarded by `EnterModifier::ChooseOpponent`).
    pub(super) fn static_count_expr_ext(&mut self, def: &str) -> Option<String> {
        if let Some(expr) = self.static_count_expr(def) {
            return Some(expr);
        }
        let trimmed = def.trim();
        if trimmed == "Count$ValidHand Card.ChosenCtrl" && self.has_chosen_opponent() {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::HandChosen }".to_string(),
            );
        }
        if trimmed == "Count$ValidGraveyard Card.ChosenCtrl" && self.has_chosen_source_player() {
            return Some(
                "Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::ChosenBySource) }"
                    .to_string(),
            );
        }
        if let Some(valid) = trimmed
            .strip_prefix("Count$ValidGraveyard,Exile ")
            .or_else(|| trimmed.strip_prefix("Count$ValidExile,Graveyard "))
            && !valid.contains(['$', '/'])
        {
            match strip_strict_zone_owner(valid)? {
                (Some("PlayerRel::You"), stripped) => {
                    let gy_expr = self.filter_expr_in_zone(&stripped, true)?;
                    let ex_expr = self.filter_expr_in_zone(valid, true)?;
                    let gy_name = self.body.filter_static("COUNT", &gy_expr);
                    let ex_name = self.body.filter_static("COUNT", &ex_expr);
                    return Some(format!(
                        "Amount::Sum {{ left: &Amount::CountOf {{ filter: &{gy_name}, \
                         zone: ZoneSel::GraveyardOf(PlayerRel::You) }}, \
                         right: &Amount::CountOf {{ filter: &{ex_name}, zone: ZoneSel::ExileAll }} }}"
                    ));
                }
                (None, stripped) => {
                    let expr = self.filter_expr_in_zone(&stripped, true)?;
                    let name = self.body.filter_static("COUNT", &expr);
                    return Some(format!(
                        "Amount::Sum {{ left: &Amount::CountOf {{ filter: &{name}, \
                         zone: ZoneSel::GraveyardAll }}, \
                         right: &Amount::CountOf {{ filter: &{name}, zone: ZoneSel::ExileAll }} }}"
                    ));
                }
                _ => return None,
            }
        }
        self.wave28c_static_count_expr(trimmed)
    }

    /// Wave 14a/b `Amount` readers usable in static cost adjustments
    /// (`Modifier::AdjustSpellCostByAmount`) and `CheckSVar` conditions:
    /// `Devotion`, `DevotionDual`, `PartySize`, `TotalPowerAmong`,
    /// `GreatestPowerAmong`, `GreatestManaValueAmong` (with optional arithmetic
    /// suffixes `/Times.N`, `/Twice`, `/Plus.N`, `/Minus.N`, `/HalfDown`, `/HalfUp`).
    pub(super) fn wave14_static_amount(&mut self, def: &str) -> Option<String> {
        self.wave14_amount_depth(def, 0, true, false)
    }

    /// Wave 14a/b `Amount` readers safe inside Layer 7c P/T modifiers
    /// (excludes `TotalPowerAmong`/`GreatestPowerAmong` to avoid P/T circularity,
    /// includes `CardsDrawnThisTurn`).
    fn wave14_non_power_amount(&mut self, def: &str) -> Option<String> {
        self.wave14_amount_depth(def, 0, false, true)
    }

    /// Wave 14a/b `Amount` readers for `CheckSVar` conditions (includes
    /// `TotalPowerAmong`, `GreatestPowerAmong`, `GreatestManaValueAmong`,
    /// `Devotion`, `PartySize`, `CardsDrawnThisTurn`, `SpellsCastThisTurn`).
    pub(super) fn wave14_condition_amount(&mut self, def: &str) -> Option<String> {
        self.wave14_amount_depth(def, 0, true, true)
    }

    fn wave14_amount_depth(
        &mut self,
        def: &str,
        depth: u8,
        allow_power_scan: bool,
        allow_turn_counts: bool,
    ) -> Option<String> {
        let trimmed = def.trim();
        if depth < 4
            && let Some(var) = trimmed.strip_prefix("SVar$")
            && !var.contains('/')
            && let Some(inner) = self.svars.get(var.trim()).cloned()
        {
            return self.wave14_amount_depth(
                &inner,
                depth + 1,
                allow_power_scan,
                allow_turn_counts,
            );
        }
        if depth < 4
            && let Some((lhs, rhs)) = trimmed.rsplit_once('/')
        {
            return self.wave14_amount_arith(
                lhs,
                rhs,
                depth + 1,
                allow_power_scan,
                allow_turn_counts,
            );
        }
        if let Some(color_name) = trimmed.strip_prefix("Count$Devotion.") {
            let color = parse_wave14_color(color_name.trim())?;
            return Some(format!(
                "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::{color}]) }}"
            ));
        }
        if let Some(dual) = trimmed.strip_prefix("Count$DevotionDual.") {
            let (c1, c2) = dual.split_once('.')?;
            let color1 = parse_wave14_color(c1.trim())?;
            let color2 = parse_wave14_color(c2.trim())?;
            if color1 == color2 || c2.contains('.') {
                return None;
            }
            return Some(format!(
                "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::{color1}, Color::{color2}]) }}"
            ));
        }
        if trimmed == "Count$Party" {
            return Some("Amount::PartySize(PlayerRel::You)".to_string());
        }
        if allow_turn_counts {
            match trimmed {
                "Count$YouDrewThisTurn" => {
                    return Some("Amount::CardsDrawnThisTurn(PlayerRel::You)".to_string());
                }
                "Count$ThisTurnCast_Card.YouCtrl" => {
                    return Some("Amount::SpellsCastThisTurn(PlayerRel::You)".to_string());
                }
                "Count$ThisTurnCast_Card" if allow_power_scan => {
                    return Some("Amount::SpellsCastThisTurn(PlayerRel::EachPlayer)".to_string());
                }
                "Count$ThisTurnCast_Card.OppCtrl" if allow_power_scan => {
                    return Some("Amount::SpellsCastThisTurn(PlayerRel::EachOpponent)".to_string());
                }
                _ => {}
            }
        }
        if let Some(rest) = trimmed.strip_prefix("Count$Valid ") {
            return self.wave14_valid_stat_amount(rest, allow_power_scan);
        }
        self.wave27a_static_amount(trimmed, allow_power_scan, allow_turn_counts)
            .or_else(|| self.wave17b_static_amount(trimmed, allow_power_scan, allow_turn_counts))
    }

    fn wave14_amount_arith(
        &mut self,
        lhs: &str,
        rhs: &str,
        next_depth: u8,
        allow_power_scan: bool,
        allow_turn_counts: bool,
    ) -> Option<String> {
        let base =
            self.wave14_amount_depth(lhs, next_depth, allow_power_scan, allow_turn_counts)?;
        let rhs = rhs.trim();
        if rhs == "Twice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"));
        }
        if rhs == "Thrice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}"));
        }
        if rhs == "HalfDown" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if rhs == "HalfUp" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
        if let Some(n) = rhs
            .strip_prefix("Times.")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|&v| v >= 1)
        {
            return Some(format!("Amount::Times {{ base: &{base}, factor: {n} }}"));
        }
        if let Some(n) = rhs
            .strip_prefix("Plus.")
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            return Some(format!("Amount::Plus {{ base: &{base}, offset: {n} }}"));
        }
        if let Some(n) = rhs
            .strip_prefix("Minus.")
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            return Some(format!(
                "Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}"
            ));
        }
        None
    }

    fn wave14_valid_stat_amount(&mut self, rest: &str, allow_power_scan: bool) -> Option<String> {
        let (valid, variant) = if let Some(valid) = rest.strip_suffix("$CardPower")
            && allow_power_scan
        {
            (valid, "Amount::TotalPowerAmong")
        } else if let Some(valid) = rest.strip_suffix("$GreatestCardPower")
            && allow_power_scan
        {
            (valid, "Amount::GreatestPowerAmong")
        } else if let Some(valid) = rest.strip_suffix("$DifferentCardPower")
            && allow_power_scan
        {
            (valid, "Amount::DistinctPowersAmong")
        } else {
            let valid = rest.strip_suffix("$GreatestCardManaCost")?;
            (valid, "Amount::GreatestManaValueAmong")
        };
        if valid.is_empty() || valid.contains(['$', '/']) || valid.contains("inZone") {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        let filter_ref = if simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("COUNT", &expr))
        };
        Some(format!("{variant}({filter_ref})"))
    }

    fn parse_discard_x_spell_cost(&mut self, cost: &str) -> Option<String> {
        let tokens = super::cost_parts(cost);
        if tokens.len() != 1
            || !self.has_x_mana_cost
            || self.svars.get("X").map(String::as_str) != Some("Count$xPaid")
            || self.oracle.to_ascii_lowercase().contains("x is ")
        {
            return None;
        }
        let inner = tokens[0]
            .strip_prefix("Discard<")
            .and_then(|s| s.strip_suffix('>'))?;
        let mut fields = inner.splitn(3, '/');
        let (Some("X"), Some(spec)) = (fields.next(), fields.next()) else {
            return None;
        };
        let spec = spec.trim();
        if spec.is_empty() || matches!(spec, "Random" | "CARDNAME" | "NICKNAME") {
            return None;
        }
        let expr = self.filter_expr_in_zone(spec, true)?;
        let filter_name = self.body.filter_static("COST", &expr);
        Some(format!("CostPart::DiscardX(&{filter_name})"))
    }
}

fn granted_damage_prevention_modifiers(trimmed: &str) -> Option<&'static [&'static str]> {
    Some(match trimmed {
        "Prevent all combat damage that would be dealt to CARDNAME." => {
            &["Modifier::PreventDamageToIt"]
        }
        "Prevent all combat damage that would be dealt by CARDNAME." => {
            &["Modifier::PreventDamageFromIt"]
        }
        "Prevent all combat damage that would be dealt to and dealt by CARDNAME." => &[
            "Modifier::PreventDamageToIt",
            "Modifier::PreventDamageFromIt",
        ],
        "Prevent all damage that would be dealt by CARDNAME." => {
            &["Modifier::PreventAllDamageFromIt"]
        }
        "Prevent all damage that would be dealt to CARDNAME." => {
            &["Modifier::PreventDamageFrom(&Filter::Any)"]
        }
        _ => return None,
    })
}

fn static_phase_condition(phases: &str) -> Option<&'static str> {
    Some(match phases.trim() {
        "BeginCombat->EndCombat" => "Condition::DuringCombat",
        "BeginCombat->Declare Attackers" => {
            "Condition::Any(&[Condition::DuringStep(StepKind::CombatBegin), Condition::DuringStep(StepKind::DeclareAttackers)])"
        }
        "Main2->Cleanup" => "Condition::AfterCombat",
        "Upkeep->Main1,Main2->Cleanup" => "Condition::Not(&Condition::DuringCombat)",
        "Upkeep" => "Condition::DuringStep(StepKind::Upkeep)",
        "Draw" => "Condition::DuringStep(StepKind::Draw)",
        "Main1" => "Condition::DuringStep(StepKind::FirstMain)",
        "Main2" => "Condition::DuringStep(StepKind::PostcombatMain)",
        "BeginCombat" => "Condition::DuringStep(StepKind::CombatBegin)",
        "Declare Attackers" => "Condition::DuringStep(StepKind::DeclareAttackers)",
        "Declare Blockers" => "Condition::DuringStep(StepKind::DeclareBlockers)",
        "EndCombat" => "Condition::DuringStep(StepKind::CombatEnd)",
        "End of Turn" => "Condition::DuringStep(StepKind::End)",
        _ => return None,
    })
}

fn is_all_non_battlefield_zones(raw: &str) -> bool {
    let mut zones: Vec<&str> = Vec::new();
    for z in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !zones.contains(&z) {
            zones.push(z);
        }
    }
    zones.len() == 6
        && ["Hand", "Graveyard", "Exile", "Library", "Command", "Stack"]
            .iter()
            .all(|z| zones.contains(z))
}

fn strip_all_zones_you_ctrl(valid: &str) -> Option<String> {
    let mut alts = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Ctrl") || base.contains("Own") {
            return None;
        }
        let mut saw_you_ctrl = false;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            match atom {
                "YouCtrl" => saw_you_ctrl = true,
                "Self" | "EnchantedBy" | "EquippedBy" | "AttachedBy" | "Attached"
                | "IsRemembered" | "ChosenCard" | "ExiledWithSource" | "ExiledWith"
                | "IsImprinted" | "TopLibrary" => return None,
                a if a.contains("Ctrl") || a.contains("Own") || a.contains("inZone") => {
                    return None;
                }
                other => kept.push(other),
            }
        }
        if !saw_you_ctrl {
            return None;
        }
        if kept.is_empty() {
            alts.push(base.trim().to_string());
        } else {
            alts.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!alts.is_empty()).then(|| alts.join(","))
}

fn is_unscoped_all_zones_affected(valid: &str) -> bool {
    let mut saw_alt = false;
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return false;
        }
        saw_alt = true;
        for atom in alt
            .split(['.', '+'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if matches!(
                atom,
                "Self"
                    | "Other"
                    | "StrictlyOther"
                    | "EnchantedBy"
                    | "EquippedBy"
                    | "AttachedBy"
                    | "Attached"
                    | "IsRemembered"
                    | "ChosenCard"
                    | "ExiledWithSource"
                    | "ExiledWith"
                    | "IsImprinted"
                    | "TopLibrary"
            ) || atom.contains("Ctrl")
                || atom.contains("Own")
                || atom.contains("inZone")
            {
                return false;
            }
        }
    }
    saw_alt
}

fn is_granted_equip_target_valid(valid: &str) -> bool {
    let mut saw_alt = false;
    for alt in valid.split(',').map(str::trim) {
        if alt.is_empty() {
            return false;
        }
        saw_alt = true;
        let Some((base, atoms_raw)) = alt.split_once('.') else {
            return false;
        };
        if base.trim() != "Creature" {
            return false;
        }
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
                || matches!(atom, "Self" | "StrictlySelf")
            {
                return false;
            }
        }
        if !saw_you {
            return false;
        }
    }
    saw_alt
}

fn parse_svar_amount_compare(cmp: &str) -> Option<(&'static str, &str)> {
    let trimmed = cmp.trim();
    if trimmed.len() < 3 {
        return None;
    }
    let (prefix, rhs) = trimmed.split_at(2);
    let rhs = rhs.trim();
    if rhs.is_empty()
        || rhs.chars().next().is_some_and(|c| c.is_ascii_digit())
        || rhs.contains([' ', '$', '/'])
    {
        return None;
    }
    let op = match prefix {
        "GE" => "AtLeast",
        "GT" => "GreaterThan",
        "LE" => "AtMost",
        "LT" => "LessThan",
        "EQ" => "Equal",
        "NE" => "NotEqual",
        _ => return None,
    };
    Some((op, rhs))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AltCastZoneKind {
    Generic,
    HandOnly,
    GraveyardGrant,
    ExileCounter,
    LinkedExile { requires_imprinted: bool },
}

struct AltCastZonePlan {
    zones_expr: String,
    grant_zone_permission: bool,
    kind: AltCastZoneKind,
}

fn is_forbidden_alt_cast_atom(atom: &str) -> bool {
    matches!(
        atom,
        "Self"
            | "Other"
            | "StrictlyOther"
            | "EnchantedBy"
            | "EquippedBy"
            | "AttachedBy"
            | "Attached"
            | "IsRemembered"
            | "TopLibrary"
            | "ThisTurnEntered"
            | "ExiledWithEffectSource"
            | "ExiledWith"
            | "ExiledByYou"
            | "foretold"
    ) || atom.starts_with("ControlledBy")
}

fn clean_alt_cast_alt(
    raw_alt: &str,
    who: &str,
    dont_grant_zone: bool,
    plan: &AltCastZonePlan,
) -> Option<(String, Option<String>, bool)> {
    let alt = raw_alt.trim();
    if alt.is_empty() {
        return None;
    }
    let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
    let base = base.trim();
    if matches!(base, "Player" | "You" | "Opponent")
        || base.contains("Own")
        || base.contains("Ctrl")
    {
        return None;
    }
    let is_hand_only = plan.kind == AltCastZoneKind::HandOnly;
    let is_gy_grant = plan.kind == AltCastZoneKind::GraveyardGrant;
    let is_exile_counter = plan.kind == AltCastZoneKind::ExileCounter;
    let linked_requires_imprinted = match plan.kind {
        AltCastZoneKind::LinkedExile { requires_imprinted } => Some(requires_imprinted),
        AltCastZoneKind::Generic
        | AltCastZoneKind::HandOnly
        | AltCastZoneKind::GraveyardGrant
        | AltCastZoneKind::ExileCounter => None,
    };
    let is_linked_exile = linked_requires_imprinted.is_some();
    let mut saw_you_owner_or_ctrl = false;
    let mut saw_nonland = base != "Card";
    let mut saw_exiled_with = false;
    let mut saw_imprinted = false;
    let mut saw_counter = false;
    let mut alt_cmc: Option<String> = None;
    let mut kept = Vec::new();
    for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
        if is_forbidden_alt_cast_atom(atom) {
            return None;
        }
        match atom {
            "ExiledWithSource" if is_linked_exile => saw_exiled_with = true,
            "IsImprinted" if is_linked_exile => saw_imprinted = true,
            "nonLand" => saw_nonland = true,
            "surveilledThisTurn" if who == "PlayerRel::You" && is_gy_grant => {
                saw_you_owner_or_ctrl = true;
                kept.push("surveilledThisTurn");
                kept.push("YouOwn");
            }
            "ExiledWithSource" | "IsImprinted" | "surveilledThisTurn" | "!surveilledThisTurn" => {
                return None;
            }
            "YouCtrl" if who == "PlayerRel::You" => {
                saw_you_owner_or_ctrl = true;
                if is_gy_grant {
                    kept.push("YouOwn");
                }
            }
            "YouOwn" if who == "PlayerRel::You" => {
                saw_you_owner_or_ctrl = true;
                if !is_hand_only {
                    kept.push(atom);
                }
            }
            "OppOwn" | "YouDontOwn" if who == "PlayerRel::You" => {
                if (is_hand_only && !dont_grant_zone) || is_gy_grant || is_exile_counter {
                    return None;
                }
                kept.push(atom);
            }
            a if a.contains("Ctrl") || a.contains("Own") => return None,
            a if a.starts_with("cmcLE") => {
                let rhs = a.strip_prefix("cmcLE").unwrap_or_default();
                if rhs.is_empty() || alt_cmc.is_some() {
                    return None;
                }
                alt_cmc = Some(rhs.to_string());
            }
            other => {
                if other.starts_with("counters_GE") {
                    saw_counter = true;
                }
                kept.push(other);
            }
        }
    }
    if let Some(requires_imprinted) = linked_requires_imprinted
        && (!saw_exiled_with || (requires_imprinted && !saw_imprinted))
    {
        return None;
    }
    if (is_hand_only && !dont_grant_zone && !saw_you_owner_or_ctrl)
        || (is_gy_grant && !saw_you_owner_or_ctrl)
        || (is_exile_counter && (!saw_you_owner_or_ctrl || !saw_counter))
    {
        return None;
    }
    let cleaned = if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}.{}", kept.join("+"))
    };
    Some((cleaned, alt_cmc, saw_nonland))
}

fn plan_exile_alt_cast_zone(
    affected: &str,
    effect_zone: Option<&'static str>,
    who: &str,
    has_linked_exile_producer: bool,
) -> Option<AltCastZonePlan> {
    if effect_zone.is_some() || who != "PlayerRel::You" {
        return None;
    }
    let all_linked = affected.split(',').all(|alt| {
        let mut has_exiled = false;
        let mut has_imprinted = false;
        for a in alt.split(['.', '+']).map(str::trim) {
            if a == "ExiledWithSource" {
                has_exiled = true;
            } else if a == "IsImprinted" {
                has_imprinted = true;
            }
        }
        has_exiled && (has_imprinted || has_linked_exile_producer)
    });
    if all_linked {
        return Some(AltCastZonePlan {
            zones_expr: "AltCastZones::LinkedExile".to_string(),
            grant_zone_permission: true,
            kind: AltCastZoneKind::LinkedExile {
                requires_imprinted: !has_linked_exile_producer,
            },
        });
    }
    let all_owned_counter = affected.split(',').all(|alt| {
        let mut has_you_own = false;
        let mut has_counter = false;
        for a in alt.split(['.', '+']).map(str::trim) {
            if a == "YouOwn" {
                has_you_own = true;
            } else if a.starts_with("counters_GE") {
                has_counter = true;
            } else if matches!(
                a,
                "ExiledWithSource" | "ExiledWith" | "ExiledByYou" | "IsImprinted"
            ) {
                return false;
            }
        }
        has_you_own && has_counter
    });
    all_owned_counter.then(|| AltCastZonePlan {
        zones_expr: "AltCastZones::Zones(&[ZoneRef::Exile])".to_string(),
        grant_zone_permission: true,
        kind: AltCastZoneKind::ExileCounter,
    })
}

fn plan_alt_cast_zones(
    raw_zones: &str,
    affected: &str,
    dont_grant_zone: bool,
    who: &str,
    effect_zone: Option<&'static str>,
    has_linked_exile_producer: bool,
) -> Option<AltCastZonePlan> {
    let mut zones: Vec<&str> = Vec::new();
    for z in raw_zones
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !zones.contains(&z) {
            zones.push(z);
        }
    }
    if zones.is_empty() {
        return None;
    }
    let is_all_five = zones.len() == 5
        && ["Hand", "Graveyard", "Library", "Exile", "Command"]
            .iter()
            .all(|z| zones.contains(z));
    if is_all_five {
        return dont_grant_zone.then(|| AltCastZonePlan {
            zones_expr: "AltCastZones::AnyPermitted".to_string(),
            grant_zone_permission: false,
            kind: AltCastZoneKind::Generic,
        });
    }
    if zones == ["Hand"] {
        if !dont_grant_zone && who != "PlayerRel::You" {
            return None;
        }
        return Some(AltCastZonePlan {
            zones_expr: "AltCastZones::Zones(&[ZoneRef::Hand])".to_string(),
            grant_zone_permission: false,
            kind: AltCastZoneKind::HandOnly,
        });
    }
    if zones == ["Graveyard"] && !dont_grant_zone && who == "PlayerRel::You" {
        return Some(AltCastZonePlan {
            zones_expr: "AltCastZones::Zones(&[ZoneRef::Graveyard])".to_string(),
            grant_zone_permission: true,
            kind: AltCastZoneKind::GraveyardGrant,
        });
    }
    if zones == ["Exile"] && !dont_grant_zone {
        return plan_exile_alt_cast_zone(affected, effect_zone, who, has_linked_exile_producer);
    }
    if !dont_grant_zone {
        return None;
    }
    let mut refs = Vec::with_capacity(zones.len());
    for z in zones {
        let zref = match z {
            "Hand" => "ZoneRef::Hand",
            "Graveyard" => "ZoneRef::Graveyard",
            "Exile" => "ZoneRef::Exile",
            "Command" => "ZoneRef::Command",
            _ => return None,
        };
        refs.push(zref);
    }
    Some(AltCastZonePlan {
        zones_expr: format!("AltCastZones::Zones(&[{}])", refs.join(", ")),
        grant_zone_permission: false,
        kind: AltCastZoneKind::Generic,
    })
}

fn is_legacy_reduce_cost_target(
    p: &Params,
    is_self_zone: bool,
    face_down: bool,
    zone: StaticEffectZone,
) -> bool {
    !is_self_zone
        && !face_down
        && !p.has("Color")
        && matches!(zone, StaticEffectZone::Battlefield)
        && matches!(
            p.peek("ValidTarget"),
            Some(
                "Card.Self"
                    | "Creature.Self"
                    | "Permanent.Self"
                    | "Permanent.YouCtrl"
                    | "Permanent.YouCtrl+inZoneBattlefield"
                    | "Card.YouCtrl+inZoneBattlefield"
            )
        )
}

pub(super) fn parse_wave14_color(raw: &str) -> Option<&'static str> {
    Some(match raw {
        "White" => "White",
        "Blue" => "Blue",
        "Black" => "Black",
        "Red" => "Red",
        "Green" => "Green",
        _ => return None,
    })
}

fn has_relative_turn_atom(valid: &str) -> bool {
    valid.split(',').any(|alt| {
        alt.split(['.', '+']).any(|a| {
            matches!(
                a.trim(),
                "Self"
                    | "StrictlySelf"
                    | "Other"
                    | "StrictlyOther"
                    | "EnchantedBy"
                    | "EquippedBy"
                    | "AttachedBy"
                    | "Attached"
            )
        })
    })
}

fn format_amount_bound(amt: &str, bound: super::Bound) -> Option<String> {
    match bound {
        super::Bound::AtLeast(n) if n >= 1 => Some(format!("Condition::AmountAtLeast({amt}, {n})")),
        super::Bound::AtMost(n) => {
            let threshold = u32::from(n).saturating_add(1);
            Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({amt}, {threshold}))"
            ))
        }
        super::Bound::AtLeast(_) => None,
    }
}

pub(super) fn parse_fixed_alt_mana_price(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed == "0" {
        return Some("AltCastPrice::Mana(ManaCost::ZERO)".to_string());
    }
    let mut out = String::new();
    for part in super::cost_parts(trimmed) {
        if part.is_empty() || part.contains(['<', '>']) {
            return None;
        }
        if part.chars().all(|c| c.is_ascii_digit())
            || matches!(part.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "S")
        {
            out.push('{');
            out.push_str(&part);
            out.push('}');
        } else if let Some(hyb) = super::hybrid_pair(&part) {
            out.push_str(&hyb);
        } else {
            let sym = super::mana::special_mana_symbol(&part)?;
            out.push_str(&sym);
        }
    }
    if out.is_empty() {
        return None;
    }
    if out == "{0}" {
        Some("AltCastPrice::Mana(ManaCost::ZERO)".to_string())
    } else {
        Some(format!("AltCastPrice::Mana(mana!(\"{out}\"))"))
    }
}

pub(super) fn simple_const_filter(expr: &str) -> bool {
    matches!(
        expr,
        "Filter::Any"
            | "Filter::CREATURE"
            | "Filter::LAND"
            | "Filter::ARTIFACT"
            | "Filter::ENCHANTMENT"
            | "Filter::PLANESWALKER"
            | "Filter::BATTLE"
            | "Filter::INSTANT"
            | "Filter::SORCERY"
            | "Filter::PERMANENT"
            | "Filter::NONLAND"
            | "Filter::NONCREATURE"
            | "Filter::YOUR_CREATURE"
            | "Filter::YOUR_LAND"
            | "Filter::OPPONENT_CREATURE"
            | "Filter::This"
    )
}

fn donor_has_unsupported_bf_atoms(valid: &str, affected_is_self: bool) -> bool {
    valid.split(',').any(|alt| {
        alt.split(['.', '+']).map(str::trim).any(|atom| match atom {
            "Other" | "StrictlyOther" => !affected_is_self,
            "Self" | "IsRemembered" | "ChosenCard" | "ExiledWithSource" | "ExiledWith"
            | "IsImprinted" | "TopLibrary" => true,
            a if a.contains("sharesNameWith") => true,
            _ => false,
        })
    })
}

fn strip_strict_zone_owner(valid: &str) -> Option<(Option<&'static str>, String)> {
    let mut owner: Option<&'static str> = None;
    let mut saw_plain = false;
    let mut alts = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Own") || base.contains("Ctrl") {
            return None;
        }
        let mut alt_owner: Option<&'static str> = None;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            match atom {
                "YouOwn" => {
                    if alt_owner.is_some_and(|prev| prev != "PlayerRel::You") {
                        return None;
                    }
                    alt_owner = Some("PlayerRel::You");
                }
                "OppOwn" | "YouDontOwn" => {
                    if alt_owner.is_some_and(|prev| prev != "PlayerRel::EachOpponent") {
                        return None;
                    }
                    alt_owner = Some("PlayerRel::EachOpponent");
                }
                "Self" | "Other" | "StrictlyOther" | "IsRemembered" | "ChosenCard"
                | "ExiledWithSource" | "ExiledWith" | "IsImprinted" | "TopLibrary" => return None,
                a if a.contains("Own") || a.contains("Ctrl") => return None,
                other => kept.push(other),
            }
        }
        if let Some(rel) = alt_owner {
            if saw_plain || owner.is_some_and(|prev| prev != rel) {
                return None;
            }
            owner = Some(rel);
        } else {
            if owner.is_some() {
                return None;
            }
            saw_plain = true;
        }
        if kept.is_empty() {
            alts.push(base.trim().to_string());
        } else {
            alts.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!alts.is_empty()).then(|| (owner, alts.join(",")))
}

fn strip_top_of_your_library(valid: &str) -> Option<String> {
    let mut alts = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Own") || base.contains("Ctrl") {
            return None;
        }
        let mut saw_top = false;
        let mut saw_you = false;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            match atom {
                "TopLibrary" => saw_top = true,
                "YouCtrl" | "YouOwn" => saw_you = true,
                "Self" | "Other" | "StrictlyOther" | "IsRemembered" | "ChosenCard"
                | "ExiledWithSource" | "ExiledWith" | "IsImprinted" => return None,
                a if a.contains("Own") || a.contains("Ctrl") => return None,
                other => kept.push(other),
            }
        }
        if !saw_top || !saw_you {
            return None;
        }
        if kept.is_empty() {
            alts.push(base.trim().to_string());
        } else {
            alts.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!alts.is_empty()).then(|| alts.join(","))
}

fn strip_imprinted_linked_exile(valid: &str) -> Option<String> {
    let mut alts = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Own") || base.contains("Ctrl") {
            return None;
        }
        let mut saw_exiled = false;
        let mut saw_imprinted = false;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            match atom {
                "ExiledWithSource" => saw_exiled = true,
                "IsImprinted" => saw_imprinted = true,
                "Self" | "Other" | "StrictlyOther" | "IsRemembered" | "ChosenCard"
                | "ExiledWith" | "TopLibrary" => return None,
                a if a.contains("Own") || a.contains("Ctrl") => return None,
                other => kept.push(other),
            }
        }
        if !saw_exiled || !saw_imprinted {
            return None;
        }
        if kept.is_empty() {
            alts.push(base.trim().to_string());
        } else {
            alts.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!alts.is_empty()).then(|| alts.join(","))
}

fn strip_in_zone_battlefield(valid: &str) -> Option<String> {
    let mut out = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let Some((base, qual)) = alt.split_once('.') else {
            out.push(alt.to_string());
            continue;
        };
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            if atom == "inZoneBattlefield" {
                continue;
            }
            if atom.starts_with("inZone") {
                return None;
            }
            kept.push(atom);
        }
        if kept.is_empty() {
            out.push(base.trim().to_string());
        } else {
            out.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    (!out.is_empty()).then(|| out.join(","))
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

fn strip_graveyard_owner(valid: &str) -> Option<(Option<&'static str>, String)> {
    let mut owner: Option<&'static str> = None;
    let mut saw_plain = false;
    let mut alts = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let (base, qual) = alt.split_once('.').unwrap_or((alt, ""));
        if base.contains("Own") || base.contains("Ctrl") {
            return None;
        }
        let mut alt_owner: Option<&'static str> = None;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            let rel = match atom {
                "YouOwn" | "YouCtrl" => Some("PlayerRel::You"),
                "OppOwn" | "OppCtrl" => Some("PlayerRel::EachOpponent"),
                _ if atom.contains("Own") || atom.contains("Ctrl") => return None,
                _ => None,
            };
            if let Some(rel) = rel {
                if alt_owner.is_some_and(|prev| prev != rel) {
                    return None;
                }
                alt_owner = Some(rel);
            } else {
                kept.push(atom);
            }
        }
        if let Some(rel) = alt_owner {
            if saw_plain || owner.is_some_and(|prev| prev != rel) {
                return None;
            }
            owner = Some(rel);
        } else {
            if owner.is_some() {
                return None;
            }
            saw_plain = true;
        }
        if kept.is_empty() {
            alts.push(base.to_string());
        } else {
            alts.push(format!("{base}.{}", kept.join("+")));
        }
    }
    Some((owner, alts.join(",")))
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

fn is_pure_ai_hint_name(key: &str) -> bool {
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

fn is_scalar_ai_hint_value(def: &str) -> bool {
    let trimmed = def.trim();
    !trimmed.is_empty()
        && !trimmed.contains(['$', ':', '|', ' '])
        && (matches!(
            trimmed,
            "TRUE" | "True" | "true" | "FALSE" | "False" | "false" | "Once" | "Multiple" | "Always"
        ) || trimmed.parse::<i32>().is_ok())
}

fn is_pure_ai_hint_svar(def: &str) -> bool {
    let Some(rest) = def.trim().strip_prefix("SVar:") else {
        return false;
    };
    let Some((key, _)) = rest.split_once(':') else {
        return false;
    };
    is_pure_ai_hint_name(key)
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

        for unsupported_kw in ["Flash", "Assist", "Replicate:3", "Emerge:CardManaCost"] {
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

    #[test]
    fn wave17b_monstrous_suspected_retrace_and_svar_conditions() {
        let state_card = read(
            "Name:Wave17b Beast\nManaCost:2 G\nTypes:Creature Beast\nPT:3/3\n\
             S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.IsMonstrous | AddKeyword$ Trample | CanBlockAmount$ 2\n\
             S:Mode$ CanAttackDefender | ValidCard$ Card.Self+IsMonstrous | Secondary$ True\n\
             S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.IsSuspected | AddKeyword$ Flying\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Vigilance | CheckSVar$ M\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Haste | CheckSVar$ S\n\
             A:SP$ Destroy | ValidTgts$ Creature.IsMonstrous,Creature.!IsSuspected\n\
             SVar:M:Count$Monstrous.1.0\n\
             SVar:S:Count$ValidSelf Card.IsSuspected",
        );
        assert!(state_card.abilities[0].contains("Modifier::AddKeyword(KeywordSet::TRAMPLE)"));
        assert!(state_card.abilities[0].contains("condition = Some(Condition::IsMonstrous)"));
        assert!(state_card.abilities[1].contains("Modifier::CanBlockAdditional(2)"));
        assert!(state_card.abilities[2].contains("Modifier::AttacksDespiteDefender"));
        assert!(state_card.abilities[2].contains("condition = Some(Condition::IsMonstrous)"));
        assert!(state_card.abilities[3].contains("condition = Some(Condition::IsSuspected)"));
        assert!(state_card.abilities[4].contains("condition = Some(Condition::IsMonstrous)"));
        assert!(state_card.abilities[5].contains("condition = Some(Condition::IsSuspected)"));
        assert!(
            state_card.statics.contains("Filter::Monstrous")
                && state_card
                    .statics
                    .contains("Filter::Not(&Filter::Suspected)")
        );

        let retrace_grant = read(
            "Name:Grave Historian\nManaCost:2 G\nTypes:Creature Wizard\nPT:2/3\n\
             S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Goblin.YouOwn,Wizard.YouOwn | AddKeyword$ Retrace",
        );
        assert!(retrace_grant.abilities[0].contains("Modifier::AddKeyword(KeywordSet::RETRACE)"));
        assert!(retrace_grant.abilities[0].contains("Filter::InZone(ZoneRef::Graveyard)"));

        let svars = read(
            "Name:SVar Probe\nManaCost:2 W\nTypes:Creature Wizard\nPT:2/2\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ C | SVarCompare$ EQ0\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Haste | CheckSVar$ L\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Vigilance | CheckSVar$ P | SVarCompare$ GE4\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Lifelink | CheckSVar$ CR | SVarCompare$ EQAR\n\
             S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Menace | CheckSVar$ E\n\
             SVar:C:Count$ThisTurnCast_Card.YouCtrl\n\
             SVar:L:PlayerCountRegisteredOpponents$HighestLifeLostThisTurn\n\
             SVar:P:Count$CardPower\n\
             SVar:CR:Count$Valid Creature.YouCtrl\n\
             SVar:AR:Count$Valid Artifact.YouCtrl\n\
             SVar:E:Count$ThisTurnEntered_Battlefield_Goblin.YouCtrl",
        );
        assert!(svars.abilities[0].contains("Condition::YouCastNoSpellThisTurn"));
        assert!(svars.abilities[1].contains("Condition::OpponentLostLifeThisTurn"));
        assert!(
            svars.abilities[2].contains("Condition::AmountAtLeast(Amount::SourcePower, 4)")
                || svars.abilities[2]
                    .contains("Condition::SourceMatches(&Filter::PowerAtLeast(4))"),
            "unexpected power condition: {}",
            svars.abilities[2]
        );
        assert!(svars.abilities[3].contains("Condition::AmountCompare"));
        assert!(svars.abilities[3].contains("AmountCmp::Equal"));
        assert!(svars.abilities[4].contains("Condition::EnteredSubtypeUnderYourControlThisTurn"));

        assert!(
            refused(
                "Name:Multi Opp Loss\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
                 S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ L | SVarCompare$ GE2\n\
                 SVar:L:PlayerCountRegisteredOpponents$HighestLifeLostThisTurn"
            )
            .is_some()
        );
        assert!(
            refused(
                "Name:Entered Hand\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
                 S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ S\n\
                 SVar:S:Count$ThisTurnEntered_Hand_from_Battlefield_Permanent.YouOwn"
            )
            .is_some()
        );
    }

    #[test]
    fn wave17b_cost_sweeps_conditional_pt_graveyard_and_prevention() {
        let costs = read(
            "Name:Tax And Discount\nManaCost:3\nTypes:Artifact\n\
             S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ 2\n\
             S:Mode$ RaiseCost | ValidCard$ Card.nonCreature | Activator$ Opponent | Type$ Spell | Amount$ 1 | Secondary$ True | PlayerTurn$ True\n\
             S:Mode$ RaiseCost | ValidCard$ Artifact | Activator$ Opponent | Type$ Ability | AffectedZone$ Battlefield | Amount$ 2\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1 | Secondary$ True\n\
             S:Mode$ ReduceCost | ValidCard$ Card.YouOwn | Type$ Ability | Amount$ 2 | MinMana$ 1 | AffectedZone$ Graveyard | IsPresent$ Card.IsCommander+YouOwn+YouCtrl",
        );
        assert!(costs.abilities[0].contains("Modifier::SpellsCostMore(2)"));
        assert!(costs.abilities[1].contains("condition = Some(Condition::YourTurn)"));
        assert!(costs.abilities[2].contains("Modifier::AbilitiesCostMore(2)"));
        assert!(costs.abilities[3].contains("Modifier::SpellsCostLess(1)"));
        assert!(
            costs.abilities[4].contains("Modifier::AbilitiesCostLess { amount: 2, minimum: 1 }")
        );
        assert!(costs.abilities[4].contains("Filter::InZone(ZoneRef::Graveyard)"));

        let cond_pt = read(
            "Name:Threshold Blade\nManaCost:2 B\nTypes:Enchantment Aura\n\
             K:Enchant:Creature\n\
             S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ X | AddToughness$ -X\n\
             SVar:X:Count$Threshold.4.2",
        );
        assert!(
            cond_pt
                .abilities
                .iter()
                .any(|a| a.contains("Modifier::ModifyPT(2, -2)") && !a.contains("condition ="))
        );
        assert!(cond_pt.abilities.iter().any(|a| {
            a.contains("Modifier::ModifyPT(2, -2)")
                && a.contains("Condition::GraveyardCountAtLeast(7)")
        }));

        let gy_type = read(
            "Name:Grave Clan\nManaCost:2\nTypes:Artifact\n\
             K:ETBReplacement:Other:ChooseCT\n\
             SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouOwn | AffectedZone$ Graveyard | AddType$ ChosenType",
        );
        assert!(gy_type.abilities[0].contains("Modifier::AddChosenSubtype"));
        assert!(gy_type.abilities[0].contains("Filter::InZone(ZoneRef::Graveyard)"));

        let dmg_flags = read(
            "Name:Revolt Flame\nManaCost:2 R\nTypes:Enchantment\n\
             R:Event$ DamageDone | Revolt$ True | ActiveZones$ Battlefield | ValidSource$ Card.YouCtrl | ValidTarget$ Permanent.OppCtrl,Opponent | IsCombat$ False | ReplaceWith$ Plus2\n\
             R:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Creature.YouCtrl | ReplaceWith$ Twice\n\
             SVar:Plus2:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Plus.2\n\
             SVar:Twice:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Twice",
        );
        assert!(dmg_flags.abilities[0].contains("Condition::Revolt"));
        assert!(dmg_flags.abilities[1].contains("zone = ZoneRef::Command"));
    }
}
