//! Codegen Wave 27d: Cost reductions (`ActivatedAbilitiesCostLess` &
//! `FirstSpellEachTurnCostAdjust`), `MustAttackPlayer`, `Filter::Saddled`, and
//! owner exile play permission (`ExilePermission::caster` /
//! `PlayerRel::OwnerOfTarget`), mapped onto Engine Wave 15d primitives.

use super::effects::EffectSubject;
use super::statics::simple_const_filter;
use super::{Params, Tx, card_type_const, printed_mana};

/// `IsSaddled` / `isSaddled` / `saddled` (and negations) -> `Filter::Saddled`
/// (CR 702.171b). Deliberately refuses `SaddledThisTurn` (different predicate:
/// creatures that tapped to saddle this Mount this turn).
pub(super) fn saddled_filter_atom(atom: &str) -> Option<String> {
    match atom {
        "IsSaddled" | "isSaddled" | "saddled" => Some("Filter::Saddled".to_string()),
        "!IsSaddled" | "!isSaddled" | "!saddled" | "nonSaddled" => {
            Some("Filter::Not(&Filter::Saddled)".to_string())
        }
        _ => None,
    }
}

/// Maps a Forge `MustAttack$` player reference to a static/source-evaluable
/// [`PlayerRel`](baylee_cards_dsl::PlayerRel) supported by
/// `eval::players_for_source` in `baylee-engine`.
///
/// Refuses planeswalker targets (`EffectSource`), arbitrary remembered/chosen
/// players outside EOT riders, and life-comparison expressions (`Opponent.lifeEQX`).
fn static_must_attack_player_rel(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "You" => Some("PlayerRel::You"),
        "EnchantedController" => Some("PlayerRel::ControllerOfAttached"),
        "EnchantedPlayer" | "Player.EnchantedBy" => Some("PlayerRel::EnchantedPlayer"),
        "CardOwner" => Some("PlayerRel::OwnerOfSource"),
        "Player.Other" => Some("PlayerRel::EachOtherPlayer"),
        "Opponent" | "Player.Opponent" => Some("PlayerRel::EachOpponent"),
        "Player" => Some("PlayerRel::EachPlayer"),
        _ => None,
    }
}

/// Maps `Activator$` on `OnlyFirstSpell$ True` cost adjustments to `PlayerRel`.
fn first_spell_activator_rel(raw: Option<&str>) -> Option<&'static str> {
    match raw.map(str::trim) {
        None | Some("Player") => Some("PlayerRel::EachPlayer"),
        Some("You") => Some("PlayerRel::You"),
        Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
        Some("EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer") => {
            Some("PlayerRel::EnchantedPlayer")
        }
        Some(_) => None,
    }
}

/// Rewrites Forge `ValidSpell$` filters like `Spell.Instant,Spell.Sorcery` or
/// `Spell.Kicked` into card-level filters (`Card.Instant,Card.Sorcery`,
/// `Card.Kicked`) so `Modifier::FirstSpellEachTurnCostAdjust` does not inject
/// `Filter::InZone(ZoneRef::Stack)` when pricing prospective spells from hand.
fn normalize_first_spell_valid(
    valid_card: Option<&str>,
    valid_spell: Option<&str>,
) -> Option<String> {
    match (valid_card, valid_spell) {
        (vc, None | Some("Spell")) => {
            let card = vc.unwrap_or("Card").trim();
            if card == "Card.Self" {
                return None;
            }
            Some(card.to_string())
        }
        (vc, Some("Spell.isCastFaceDown")) => {
            let card = vc.unwrap_or("Card").trim();
            if card == "Card.Self" {
                return None;
            }
            Some(Tx::append_reduce_atom(card, "faceDown"))
        }
        (None | Some("Card"), Some(raw_spell)) => {
            let mut out = Vec::new();
            for alt in raw_spell.split(',').map(str::trim) {
                let rest = alt.strip_prefix("Spell.")?;
                if rest.is_empty()
                    || rest.contains(' ')
                    || rest.contains("Ctrl")
                    || rest.contains("Own")
                    || rest.contains("Targeting")
                    || rest.contains("Buyback")
                    || rest.contains("Flashback")
                    || rest.contains("Activated")
                {
                    return None;
                }
                out.push(format!("Card.{rest}"));
            }
            (!out.is_empty()).then(|| out.join(","))
        }
        _ => None,
    }
}

struct OwnerExilePermissionSpec {
    window: &'static str,
    cast_only: bool,
    spell_types: Option<String>,
    free: bool,
    mana: &'static str,
    caster: &'static str,
}

impl OwnerExilePermissionSpec {
    fn to_expr(&self) -> String {
        let mut expr = if self.cast_only || self.spell_types.is_some() {
            format!("ExilePermission::cast({})", self.window)
        } else {
            format!("ExilePermission::play({})", self.window)
        };
        if let Some(types) = &self.spell_types {
            expr = format!("{expr}.with_spell_types({types})");
        }
        if self.free {
            expr.push_str(".without_paying_mana_cost()");
        }
        match self.mana {
            "ExileMana::AnyColor" => expr.push_str(".spend_mana_as_any_color()"),
            "ExileMana::AnyType" => expr.push_str(".spend_mana_as_any_type()"),
            _ => {}
        }
        if self.caster != "PlayerRel::You" {
            expr = format!("{expr}.for_player({})", self.caster);
        }
        expr
    }
}

fn parse_remembered_exile_affected(
    affected: &str,
    allow_you_own: bool,
) -> Option<(bool, Option<String>)> {
    let alts: Vec<&str> = affected.split(',').map(str::trim).collect();
    if alts.is_empty() {
        return None;
    }
    let mut type_bits: Vec<&'static str> = Vec::new();
    let mut saw_nonland = false;
    let mut saw_untyped = false;
    for alt in &alts {
        let mut parts = alt
            .split(['.', '+'])
            .map(str::trim)
            .filter(|p| !p.is_empty());
        let base = parts.next()?;
        let mut saw_marker = false;
        let mut alt_type: Option<&'static str> = None;
        if base != "Card" {
            let tc = card_type_const(base)?;
            if tc == "TypeSet::LAND" {
                return None;
            }
            alt_type = Some(tc);
        }
        for atom in parts {
            if atom == "IsRemembered" {
                saw_marker = true;
            } else if matches!(atom, "nonLand" | "!Land") {
                saw_nonland = true;
            } else if atom == "YouOwn" && allow_you_own {
                // Own library exile is always owned by that player.
            } else if let Some(tc) = card_type_const(atom)
                && tc != "TypeSet::LAND"
                && alt_type.is_none()
            {
                alt_type = Some(tc);
            } else {
                return None;
            }
        }
        if !saw_marker {
            return None;
        }
        if let Some(tc) = alt_type {
            type_bits.push(tc);
        } else {
            saw_untyped = true;
        }
    }
    if saw_untyped {
        if alts.len() != 1 || !type_bits.is_empty() {
            return None;
        }
        return Some((saw_nonland, None));
    }
    if type_bits.is_empty() {
        return None;
    }
    let mut expr = type_bits[0].to_string();
    for bit in &type_bits[1..] {
        expr = format!("{expr}.union({bit})");
    }
    Some((saw_nonland, Some(expr)))
}

impl Tx<'_> {
    /// `S: Mode$ ReduceCost` / `S: Mode$ RaiseCost` with `OnlyFirstSpell$ True`
    /// -> `Modifier::FirstSpellEachTurnCostAdjust` (*Artificer Class*,
    /// *Acolyte of Bahamut*, *Kadena*, *Melek, Reforged Researcher*,
    /// *Shadow in the Warp*, *Vine Gecko*, *Conduit of Ruin*, *Serah Farron*,
    /// *Radagast of Rhosgobel*, *Heartwood Storyteller Avatar*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27d_first_spell_cost_adjust(
        &mut self,
        mut p: Params,
        increase: bool,
    ) -> Option<()> {
        let mode_label = if increase {
            "static ability `S: Mode$ RaiseCost`"
        } else {
            "static ability `S: Mode$ ReduceCost`"
        };
        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return self.deny(mode_label.into()),
        }
        if p.take("OnlyFirstSpell").as_deref() != Some("True") {
            return self.deny(mode_label.into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny(mode_label.into());
        };
        if p.has("ValidTarget")
            || p.has("Relative")
            || p.has("MinMana")
            || p.has("RememberObjects")
            || p.has("Affected")
        {
            return self.deny(mode_label.into());
        }
        match p.take("Type").as_deref() {
            Some("Spell") => {}
            None if p.peek("ValidSpell").is_some_and(|v| v.starts_with("Spell")) => {}
            _ => return self.deny(mode_label.into()),
        }
        let Some(who) = first_spell_activator_rel(p.take("Activator").as_deref()) else {
            return self.deny(mode_label.into());
        };
        let valid_card_raw = p.take("ValidCard");
        let valid_spell_raw = p.take("ValidSpell");
        let Some(valid_card) =
            normalize_first_spell_valid(valid_card_raw.as_deref(), valid_spell_raw.as_deref())
        else {
            return self.deny(mode_label.into());
        };
        let (cost_expr, colored_only) = if increase {
            if p.has("Color") || p.has("IgnoreGeneric") {
                return self.deny(mode_label.into());
            }
            match (p.take("Amount"), p.take("Cost")) {
                (Some(amt_raw), None) => {
                    let Some(n) = amt_raw.trim().parse::<u32>().ok().filter(|&n| n > 0) else {
                        return self.deny(mode_label.into());
                    };
                    (format!("mana!(\"{{{n}}}\")"), false)
                }
                (None, Some(cost_raw)) => {
                    let Some(mana) = printed_mana(&cost_raw) else {
                        return self.deny(mode_label.into());
                    };
                    (format!("mana!(\"{mana}\")"), false)
                }
                _ => return self.deny(mode_label.into()),
            }
        } else if let Some(color_raw) = p.take("Color") {
            if p.take("Amount").as_deref() != Some("1") {
                return self.deny(mode_label.into());
            }
            let colored_only = match p.take("IgnoreGeneric").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return self.deny(mode_label.into()),
            };
            let Some(mana) = printed_mana(&color_raw) else {
                return self.deny(mode_label.into());
            };
            (format!("mana!(\"{mana}\")"), colored_only)
        } else {
            if p.has("IgnoreGeneric") {
                return self.deny(mode_label.into());
            }
            let Some(amt_raw) = p.take("Amount") else {
                return self.deny(mode_label.into());
            };
            let Some(n) = amt_raw.trim().parse::<u32>().ok().filter(|&n| n > 0) else {
                return self.deny(mode_label.into());
            };
            (format!("mana!(\"{{{n}}}\")"), false)
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            let prefix = if increase { "RaiseCost" } else { "ReduceCost" };
            return self.deny(format!("unclaimed parameter `{prefix}.{key}`"));
        }
        let expr = self.filter_expr_in_zone(&valid_card, true)?;
        let inner_filter = if simple_const_filter(&expr) {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("FIRST_SPELL_COST", &expr))
        };
        let modifier = format!(
            "Modifier::FirstSpellEachTurnCostAdjust {{ who: {who}, filter: {inner_filter}, \
             cost: {cost_expr}, increase: {increase}, colored_only: {colored_only} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    /// `S: Mode$ ReduceCost` for activated abilities with dynamic `Amount$`
    /// (*Agatha of the Vile Cauldron*) or `ValidSpell$ Activated.Exhaust`
    /// (*Boom Scholar*) -> `Modifier::ActivatedAbilitiesCostLess`.
    pub(super) fn wave27d_activated_abilities_cost_less(
        &mut self,
        mut p: Params,
        exhaust_only: bool,
    ) -> Option<()> {
        if exhaust_only {
            if p.take("ValidSpell").as_deref() != Some("Activated.Exhaust") {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
            if !matches!(p.take("Type").as_deref(), None | Some("Ability")) {
                return self.deny("static ability `S: Mode$ ReduceCost`".into());
            }
        }
        if p.has("ValidTarget")
            || p.has("Color")
            || p.has("IgnoreGeneric")
            || p.has("Relative")
            || p.has("OnlyFirstSpell")
            || p.has("RememberObjects")
        {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(zone) = Self::static_effect_zone(p.take("EffectZone").as_deref(), false) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let affected_zone = p.take("AffectedZone");
        let mut graveyard_zone = false;
        let explicit_battlefield = match affected_zone.as_deref() {
            Some("Graveyard") => {
                graveyard_zone = true;
                false
            }
            Some("Battlefield") => true,
            None => false,
            Some(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
        };
        let Some(mut raw_card) = p.take("ValidCard") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        if affected_zone.is_none()
            && let Some(stripped) = Self::strip_in_zone_graveyard(&raw_card)
        {
            raw_card = stripped;
            graveyard_zone = true;
        }
        if raw_card == "Card" && !explicit_battlefield && !graveyard_zone {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        }
        let Some(valid_card) =
            Self::apply_reduce_activator(&raw_card, p.take("Activator").as_deref())
        else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let min_mana = match p.take("MinMana") {
            None => 0u8,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) => n,
                Err(_) => return self.deny("static ability `S: Mode$ ReduceCost`".into()),
            },
        };
        let Some(amt_raw) = p.take("Amount") else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let Some(amount_expr) = self.wave27d_ability_reduction_amount(&amt_raw) else {
            return self.deny("static ability `S: Mode$ ReduceCost`".into());
        };
        let cond = self.reduce_cost_condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ReduceCost.{key}`"));
        }
        let filter_expr = if graveyard_zone {
            let base = self.filter_expr_in_zone(&valid_card, true)?;
            if base == "Filter::Any" {
                "Filter::InZone(ZoneRef::Graveyard)".to_string()
            } else {
                format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::Graveyard)])")
            }
        } else {
            self.filter_expr(&valid_card)?
        };
        let inner_filter = if simple_const_filter(&filter_expr) {
            format!("&{filter_expr}")
        } else {
            format!("&{}", self.body.filter_static("COST_ABILITY", &filter_expr))
        };
        let modifier = format!(
            "Modifier::ActivatedAbilitiesCostLess {{ filter: {inner_filter}, \
             amount: {amount_expr}, min_mana: {min_mana}, exhaust_only: {exhaust_only} }}"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            &modifier,
            zone,
            &cond,
        ));
        Some(())
    }

    fn wave27d_ability_reduction_amount(&mut self, amt_raw: &str) -> Option<String> {
        let trimmed = amt_raw.trim();
        if let Ok(n) = trimmed.parse::<u32>() {
            return (n > 0).then(|| format!("Amount::Fixed({n})"));
        }
        let def = if trimmed.starts_with("Count$") || trimmed.starts_with("SVar$") {
            trimmed.to_string()
        } else {
            self.svars.get(trimmed)?.clone()
        };
        let def_trimmed = def.trim();
        if def_trimmed == "Count$CardPower" && !self.on_a_spell {
            return Some("Amount::SourcePower".to_string());
        }
        self.wave14_static_amount(def_trimmed)
            .or_else(|| self.static_count_expr_ext(def_trimmed))
    }

    /// `S: Mode$ MustAttack | ValidCreature$ ... | MustAttack$ <player>` ->
    /// `Modifier::MustAttackPlayer(PlayerRel)` (*Public Enemy*, *The Dead Shall Serve*).
    pub(super) fn wave27d_must_attack_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let zone = self.take_static_effect_zone(&mut p, "MustAttack")?;
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`MustAttack.Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCreature") else {
            return self.deny("`MustAttack` naming no creature".to_string());
        };
        let Some(must_raw) = p.take("MustAttack") else {
            return self.deny("unclaimed parameter `MustAttack.MustAttack`".to_string());
        };
        let Some(who) = static_must_attack_player_rel(&must_raw) else {
            return self.deny("unclaimed parameter `MustAttack.MustAttack`".to_string());
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
            &format!("Modifier::MustAttackPlayer({who})"),
            zone,
            &condition,
        ));
        Some(())
    }

    /// `DB$ Effect` / `AB$ Effect` / `SP$ Effect` `StaticAbilities$` entry with
    /// `Mode$ MustAttack` (both plain `AttacksEachCombat` and `MustAttack$`
    /// player requirements: *Alluring Siren*, *Nahiri, the Unforgiving*,
    /// *Kardur, Doomscourge*, *Maximum Carnage*).
    pub(super) fn wave27d_effect_must_attack(
        &mut self,
        st: &mut Params,
        subject: EffectSubject,
        target: Option<&str>,
        duration: &str,
    ) -> Option<Vec<String>> {
        match st.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return None,
        }
        let valid = st.take("ValidCreature")?;
        let must_attack = st.take("MustAttack");
        st.exhausted().then_some(())?;
        let Some(raw_player) = must_attack else {
            return self.emit_effect_modifiers(
                subject,
                &valid,
                &["Modifier::AttacksEachCombat".to_string()],
                duration,
            );
        };
        if let Some(who) = static_must_attack_player_rel(&raw_player) {
            return self.emit_effect_modifiers(
                subject,
                &valid,
                &[format!("Modifier::MustAttackPlayer({who})")],
                duration,
            );
        }
        // `resolve/wave14d.rs` binds resolution-scoped `MustAttackPlayer(Chosen)`
        // via `Rider::MustAttackPlayerThisTurn` exclusively for `Duration::UntilEndOfTurn`.
        let has_chosen_player = self.has_chosen_source_player()
            || matches!(
                target,
                Some(
                    "TargetSpec::AnyPlayer"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::Player(PlayerRel::Chosen)"
                )
            );
        if raw_player.trim() == "ChosenPlayer"
            && matches!(
                duration,
                "Duration::UntilEndOfTurn" | "Duration::UntilEndOfCombat"
            )
            && has_chosen_player
        {
            let tgt = match (subject, valid.as_str()) {
                (EffectSubject::Targets, "Card.IsRemembered" | "Creature.IsRemembered") => target
                    .filter(|t| t.starts_with("TargetSpec::Object("))
                    .unwrap_or("TargetSpec::Object(&Filter::Any)"),
                (EffectSubject::Source, "Card.IsRemembered" | "Creature.IsRemembered")
                | (
                    EffectSubject::Unbound,
                    "Card.Self" | "Creature.Self" | "Card.EffectSource" | "Creature.EffectSource",
                ) if !self.on_a_spell => "TargetSpec::ThisObject",
                (EffectSubject::Event, "Card.IsRemembered" | "Creature.IsRemembered") => {
                    "TargetSpec::EventObject"
                }
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::AddTempModifier {{ target: {tgt}, \
                 modifier: Modifier::MustAttackPlayer(PlayerRel::Chosen), duration: {duration} }}"
            )]);
        }
        None
    }

    /// Returns `true` when `sub` points to a `DB$ Effect` with a non-`You`
    /// `EffectOwner$` (`TargetedOwner` or `RememberedOwner`) granting `MayPlay$ True`
    /// on exiled card(s) (*Aerial Extortionist*, *Release to the Wind*, *Prowl*).
    pub(super) fn sub_is_owner_exile_play_effect(&self, sub: Option<&str>) -> bool {
        let Some(name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, p)) = Params::parse(body) else {
            return false;
        };
        if api != "Effect"
            || !matches!(
                p.peek("EffectOwner"),
                Some("TargetedOwner" | "RememberedOwner")
            )
            || p.has("Triggers")
            || p.has("ReplacementEffects")
            || p.has("Abilities")
        {
            return false;
        }
        let Some(static_name) = p.peek("StaticAbilities") else {
            return false;
        };
        if static_name.contains(',') {
            return false;
        }
        self.svars
            .get(static_name.trim())
            .and_then(|s| Params::parse(s))
            .is_some_and(|(mode, sp)| {
                mode == "Continuous"
                    && sp.peek("MayPlay") == Some("True")
                    && sp.peek("AffectedZone") == Some("Exile")
            })
    }

    /// `ChangeZone | Origin$ Battlefield|Graveyard | Destination$ Exile`
    /// followed by `DB$ Effect | EffectOwner$ TargetedOwner|RememberedOwner |
    /// StaticAbilities$ <MayPlay>` -> `Effect::ExileWithPermission` with
    /// `ExilePermission::for_player(PlayerRel::OwnerOfTarget)`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27d_owner_exile_with_permission(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        if !matches!(origin.as_str(), "Battlefield" | "Graveyard")
            || p.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let remembered_changed = match p.take("RememberChanged").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!(
                    "owner exile-with-permission `RememberChanged$ {other}`"
                ));
            }
        };
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return self.deny(format!(
                "exile-with-permission `ForgetOtherRemembered$ {forget}`"
            ));
        }
        p.take("IsCurse");
        let until = match p.take("Duration").as_deref() {
            None => "None",
            Some("UntilHostLeavesPlay") if origin == "Battlefield" && !self.on_a_spell => {
                "Some(ExileUntil::SourceLeavesBattlefield)"
            }
            Some(other) => {
                return self.deny(format!("exile-with-permission `Duration$ {other}`"));
            }
        };
        let defined = p.take("Defined");
        let (target_spec, is_targeted) = match (origin.as_str(), defined.as_deref()) {
            ("Battlefield", None) if targets_here => (
                target
                    .filter(|t| {
                        t.starts_with("TargetSpec::Object(")
                            || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                    })?
                    .to_string(),
                true,
            ),
            ("Battlefield", Some("Targeted" | "ParentTarget")) if !targets_here => (
                target
                    .filter(|t| {
                        t.starts_with("TargetSpec::Object(")
                            || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                    })?
                    .to_string(),
                true,
            ),
            ("Battlefield", Some("Self"))
                if remembered_changed && !targets_here && !self.on_a_spell && target.is_none() =>
            {
                ("TargetSpec::ThisObject".to_string(), false)
            }
            ("Battlefield", Some(def))
                if remembered_changed
                    && !targets_here
                    && target.is_none()
                    && self.is_event_object_defined(def) =>
            {
                ("TargetSpec::EventObject".to_string(), false)
            }
            ("Graveyard", None) if targets_here => (
                target
                    .filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?
                    .to_string(),
                true,
            ),
            ("Graveyard", Some("Targeted" | "ParentTarget")) if !targets_here => (
                target
                    .filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?
                    .to_string(),
                true,
            ),
            ("Graveyard", Some(def))
                if remembered_changed
                    && !targets_here
                    && target.is_none()
                    && self.is_event_object_defined(def) =>
            {
                ("TargetSpec::EventObject".to_string(), false)
            }
            _ => {
                return self.deny("exile-with-permission target outside supported zones".into());
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChangeZone.{key}`"));
        }
        let Some(eff_name) = sub.take() else {
            return self.deny("exile-with-permission without `SubAbility$`".into());
        };
        let (spec, next_sub) = self.wave27d_consume_owner_exile_effect(
            &eff_name,
            remembered_changed,
            is_targeted,
            false,
            "exile-with-permission",
        )?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        let permission = spec.to_expr();
        Some(vec![format!(
            "Effect::ExileWithPermission {{ target: {target_spec}, until: {until}, \
             permission: {permission} }}"
        )])
    }

    /// Follow-up `DB$ Effect` for top-of-library exile (`Dig | DestinationZone$
    /// Exile | RememberChanged$ True`) when `EffectOwner$` names a non-`You`
    /// player (`RememberedOwner`, `TriggeredSourceController` on damage triggers).
    pub(super) fn wave27d_exile_top_non_you_owner(
        &mut self,
        who: &str,
        amount_expr: &str,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let eff_name = sub.as_deref()?.trim();
        let eff_body = self.svars.get(eff_name)?.clone();
        let (eff_api, ep) = Params::parse(&eff_body)?;
        if eff_api != "Effect" || matches!(ep.peek("EffectOwner"), None | Some("You")) {
            return None;
        }
        let eff_name = sub.take()?;
        let (spec, next_sub) = self.wave27d_consume_owner_exile_effect(
            &eff_name,
            true,
            false,
            true,
            "top-of-library exile",
        )?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        let permission = spec.to_expr();
        Some(vec![format!(
            "Effect::ExileTopWithPermission {{ who: {who}, count: {amount_expr}, \
             permission: {permission} }}"
        )])
    }

    fn wave27d_consume_owner_exile_effect(
        &mut self,
        eff_name: &str,
        remembered_changed: bool,
        is_targeted: bool,
        from_library: bool,
        context: &str,
    ) -> Option<(OwnerExilePermissionSpec, Option<String>)> {
        let Some(eff_body) = self.svars.get(eff_name.trim()).cloned() else {
            return self.deny(format!("`SubAbility$ {eff_name}` names no SVar"));
        };
        let Some((eff_api, mut ep)) = Params::parse(&eff_body) else {
            return self.deny(format!("{context} follow-up with no `$`"));
        };
        ep.drop_prose();
        if eff_api != "Effect" {
            return self.deny(format!(
                "a {context} follow-up other than an exiled-card play permission"
            ));
        }
        let rem_ok = match ep.take("RememberObjects").as_deref() {
            Some("Remembered" | "RememberedCard") => remembered_changed,
            Some("Targeted" | "ParentTarget") => is_targeted,
            _ => false,
        };
        if !rem_ok {
            return self.deny(format!(
                "a {context} follow-up with unsupported `RememberObjects$`"
            ));
        }
        let caster = match ep.take("EffectOwner").as_deref() {
            Some("TargetedOwner") if is_targeted => "PlayerRel::OwnerOfTarget",
            Some("RememberedOwner") if remembered_changed || is_targeted => {
                "PlayerRel::OwnerOfTarget"
            }
            Some("RememberedController") if from_library => "PlayerRel::OwnerOfTarget",
            Some("TriggeredSourceController")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ) =>
            {
                "PlayerRel::ControllerOfEvent"
            }
            other => {
                return self.deny(format!(
                    "{context} `EffectOwner$ {}`",
                    other.unwrap_or("none")
                ));
            }
        };
        if !matches!(
            ep.take("ExileOnMoved")
                .or_else(|| ep.take("ForgetOnMoved"))
                .as_deref(),
            None | Some("Exile")
        ) {
            return self.deny(format!(
                "a {context} follow-up other than an exiled-card play permission"
            ));
        }
        let raw_duration = ep.take("Duration");
        let Some(static_name) = ep.take("StaticAbilities") else {
            return self.deny(format!("{context} `Effect` without `StaticAbilities$`"));
        };
        if static_name.contains(',') {
            return self.deny(format!(
                "{context} `Effect` with multiple `StaticAbilities$`"
            ));
        }
        ep.take("Name");
        if let Some(non_stack) = ep.take("NonStackingEffect")
            && non_stack != "True"
        {
            return self.deny(format!(
                "{context} `Effect` with `NonStackingEffect$ {non_stack}`"
            ));
        }
        if let Some(stackable) = ep.take("Stackable")
            && stackable != "False"
        {
            return self.deny(format!("{context} `Effect` with `Stackable$ {stackable}`"));
        }
        if let Some(forget_cast) = ep.take("ForgetOnCast")
            && forget_cast != "False"
        {
            return self.deny(format!(
                "{context} `Effect` with `ForgetOnCast$ {forget_cast}`"
            ));
        }
        let next_sub = ep.take("SubAbility");
        if let Some(key) = ep.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let spec = self.wave27d_parse_owner_exile_static(
            &static_name,
            from_library,
            raw_duration.as_deref(),
            caster,
        )?;
        Some((spec, next_sub))
    }

    #[allow(clippy::too_many_lines)]
    fn wave27d_parse_owner_exile_static(
        &mut self,
        static_name: &str,
        allow_you_own: bool,
        raw_duration: Option<&str>,
        caster: &'static str,
    ) -> Option<OwnerExilePermissionSpec> {
        let Some(static_body) = self.svars.get(static_name.trim()).cloned() else {
            return self.deny(format!("`StaticAbilities$ {static_name}` names no SVar"));
        };
        let Some((mode, mut sp)) = Params::parse(&static_body) else {
            return self.deny("exile play static ability without `Mode$`".into());
        };
        let desc_lower = format!(
            "{} {}",
            sp.peek("Description")
                .map_or_default(str::to_ascii_lowercase),
            sp.take("Secondary")
                .map_or_default(|s| s.to_ascii_lowercase()),
        );
        sp.drop_prose();
        if !matches!(sp.take("EffectZone").as_deref(), None | Some("Command")) {
            return self.deny("exile play static ability outside `Command` zone".into());
        }
        if mode != "Continuous"
            || sp.take("MayPlay").as_deref() != Some("True")
            || sp.take("AffectedZone").as_deref() != Some("Exile")
        {
            return self
                .deny("exile play static ability other than playing the exiled card".into());
        }
        let window = match raw_duration {
            None | Some("EndOfTurn" | "UntilEndOfTurn") => "PlayWindow::ThisTurn",
            Some("UntilYourNextTurn") => "PlayWindow::UntilYourNextTurn",
            Some("UntilYourNextUpkeep") => "PlayWindow::UntilYourNextUpkeep",
            Some("UntilTheEndOfYourNextTurn") => "PlayWindow::ThroughYourNextTurn",
            Some("AsLongAsInPlay" | "UntilHostLeavesPlay") if !self.on_a_spell => {
                "PlayWindow::WhileSourceOnBattlefield"
            }
            Some("Permanent") => {
                if desc_lower.contains("you control") {
                    return self
                        .deny("exile permission tied to continuing to control the source".into());
                }
                if desc_lower.contains("remains on the battlefield")
                    || desc_lower.contains("is on the battlefield")
                {
                    if self.on_a_spell {
                        return self.deny(
                            "exile permission tied to source on battlefield on a spell".into(),
                        );
                    }
                    "PlayWindow::WhileSourceOnBattlefield"
                } else {
                    "PlayWindow::WhileExiled"
                }
            }
            Some(other) => {
                return self.deny(format!("exile permission `Duration$ {other}`"));
            }
        };
        let affected = sp.take("Affected")?;
        let Some((mut cast_only, spell_types)) =
            parse_remembered_exile_affected(&affected, allow_you_own)
        else {
            return self.deny(format!(
                "exile play static ability with unsupported `Affected$ {affected}`"
            ));
        };
        if !cast_only
            && spell_types.is_none()
            && desc_lower.contains("may cast")
            && !desc_lower.contains("may play")
        {
            cast_only = true;
        }
        let free = match sp.take("MayPlayWithoutManaCost").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayWithoutManaCost$ {other}`"));
            }
        };
        let ignore_type = match sp.take("MayPlayIgnoreType").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayIgnoreType$ {other}`"));
            }
        };
        let ignore_color = match sp.take("MayPlayIgnoreColor").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayIgnoreColor$ {other}`"));
            }
        };
        let mana = match (ignore_type, ignore_color) {
            (false, false) => "ExileMana::Exact",
            (false, true) => "ExileMana::AnyColor",
            (true, false) => "ExileMana::AnyType",
            (true, true) => {
                if desc_lower.contains("any color") && !desc_lower.contains("any type") {
                    "ExileMana::AnyColor"
                } else if desc_lower.contains("any type") && !desc_lower.contains("any color") {
                    "ExileMana::AnyType"
                } else {
                    return self.deny(
                        "exile play with both `MayPlayIgnoreType$` and `MayPlayIgnoreColor$`"
                            .into(),
                    );
                }
            }
        };
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `StaticAbility.{key}`"));
        }
        Some(OwnerExilePermissionSpec {
            window,
            cast_only,
            spell_types,
            free,
            mana,
            caster,
        })
    }
}
