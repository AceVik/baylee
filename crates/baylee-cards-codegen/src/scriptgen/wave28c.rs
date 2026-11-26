//! Wave 28c static-ability, cost-modifier, flash, trigger-multiplier, zone-play,
//! and mana-restriction lowerings over existing `baylee-cards-dsl` primitives.
//!
//! Every rule here lowers only shapes whose exact Comprehensive Rules semantics
//! are already faithfully implemented by `baylee-engine`; anything narrower or
//! wider (e.g. `Creature.YouCtrl` attack Panharmonicon, `ValidSA$ Activated.*`
//! flash grants, `ValidTarget$` without `YouCtrl`, `CantAttack.Target$ You,Planeswalker`,
//! `UntapOtherPlayer`, `RaiseCost` on non-mana abilities) stays honestly refused.

use baylee_core::types::SubtypeKind;

use super::{
    Chain, Params, Tx,
    effects::EffectSubject,
    statics::{parse_fixed_alt_mana_price, simple_const_filter},
};

impl Tx<'_> {
    /// Whether `S: Mode$ CastWithFlash` targets a Wave 28c shape:
    /// - Conditional self-Flash (`ValidCard$ Card.Self | EffectZone$ All | Caster$ You`), or
    /// - Battlefield/command grant writing the spell filter on `ValidSA$ Spell.<qual>`
    ///   with `ValidCard$ Card`.
    pub(super) fn is_wave28c_cast_with_flash(p: &Params) -> bool {
        let self_all = p.peek("ValidCard") == Some("Card.Self")
            && p.peek("EffectZone") == Some("All")
            && matches!(p.peek("ValidSA"), None | Some("Spell"));
        let valid_sa_spell_filter = p.peek("ValidCard") == Some("Card")
            && p.peek("ValidSA").is_some_and(|sa| sa.starts_with("Spell."));
        self_all || valid_sa_spell_filter
    }

    /// Lowers Wave 28c `S: Mode$ CastWithFlash` shapes onto
    /// `Modifier::GrantFlash`:
    ///
    /// 1. Conditional self-Flash (`EffectZone$ All | ValidCard$ Card.Self |
    ///    ValidSA$ Spell | Caster$ You`), emitted with `Filter::This`,
    ///    `zone = ZoneRef::NotBattlefield`, and the required condition — keeping
    ///    the spell's printed keywords unchanged (CR 601.3b).
    /// 2. Battlefield/command Flash grants whose spell qualifier lives on
    ///    `ValidSA$ Spell.<qual>` with `ValidCard$ Card` (*Blur of Heroism*:
    ///    `ValidSA$ Spell.Hero`), rejecting targeting/X-cost/zone/controller
    ///    qualifiers that `Modifier::GrantFlash` cannot represent.
    pub(super) fn wave28c_cast_with_flash(&mut self, mut p: Params) -> Option<()> {
        let deny = |tx: &Self| tx.deny("static ability `S: Mode$ CastWithFlash`".to_string());
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return deny(self);
        }
        let caster_raw = p.take("Caster");
        let valid_sa = p.take("ValidSA");
        let effect_zone = p.take("EffectZone");
        let Some(valid_card) = p.take("ValidCard") else {
            return deny(self);
        };

        // Shape 1: conditional self-Flash from off-battlefield zones.
        if valid_card == "Card.Self" && effect_zone.as_deref() == Some("All") {
            if caster_raw.as_deref() != Some("You")
                || !matches!(valid_sa.as_deref(), None | Some("Spell"))
            {
                return deny(self);
            }
            let clauses = self.wave28c_flash_conditions(&mut p)?;
            if clauses.is_empty() || !p.exhausted() {
                return deny(self);
            }
            let cond = format_condition_clauses(&clauses);
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::GrantFlash {{ \
                 who: PlayerRel::You, filter: &Filter::This }}, \
                 zone = ZoneRef::NotBattlefield{cond})"
            ));
            return Some(());
        }

        // Shape 2: `ValidCard$ Card | ValidSA$ Spell.<simple_qual>`.
        if valid_card != "Card" {
            return deny(self);
        }
        let who = match caster_raw.as_deref() {
            Some("You") => "PlayerRel::You",
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            _ => return deny(self),
        };
        let zone_arg = match effect_zone.as_deref() {
            None | Some("Battlefield") => None,
            Some("Command") => Some(", zone = ZoneRef::Command"),
            Some("Battlefield,Command" | "Command,Battlefield") => {
                Some(", zone = ZoneRef::BattlefieldOrCommand")
            }
            Some(_) => return deny(self),
        };
        let Some(qual) = valid_sa.as_deref().and_then(|sa| sa.strip_prefix("Spell.")) else {
            return deny(self);
        };
        if !is_simple_flash_spell_qualifier(qual) {
            return deny(self);
        }
        let clauses = self.wave28c_flash_conditions(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }
        let rewritten = format!("Card.{qual}");
        let Some(expr) = self.filter_expr_in_zone(&rewritten, true) else {
            return deny(self);
        };
        let filter_ref = if expr == "Filter::Any" {
            "Filter::Any".to_string()
        } else {
            self.body.filter_static("FLASH", &expr)
        };
        let modifier = format!("Modifier::GrantFlash {{ who: {who}, filter: &{filter_ref} }}");
        let cond = format_condition_clauses(&clauses);
        let mut ability = Self::static_expr("Filter::Any", &modifier);
        if (zone_arg.is_some() || !cond.is_empty())
            && let Some(open) = ability.strip_suffix(')')
        {
            ability = format!("{open}{}{cond})", zone_arg.unwrap_or_default());
        }
        self.body.abilities.push(ability);
        Some(())
    }

    fn wave28c_flash_conditions(&mut self, p: &mut Params) -> Option<Vec<String>> {
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
        if matches!(p.peek("PresentZone"), Some("Hand" | "Graveyard"))
            && matches!(p.peek("PresentPlayer"), None | Some("You"))
            && let Some(valid) = p.peek("IsPresent")
            && !matches!(valid, "Card.StrictlySelf" | "Card.Self")
        {
            let valid = p.take("IsPresent")?;
            clauses.push(self.reduce_cost_zone_present(p, &valid)?);
        }
        if let Some(base) = self
            .static_conditions(p, "static ability `S: Mode$ CastWithFlash`")?
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        {
            clauses.push(base.to_string());
        }
        Some(clauses)
    }

    /// `CheckSVar$` comparison on `Count$ThisTurnCast_Card.YouCtrl` for exact
    /// counts `EQ<n>` / `NE<n>` with `n >= 1` (e.g. `EQ1` for "the second spell
    /// you cast each turn" on *Alisaie Leveilleur*, *Highspire Bell-Ringer*,
    /// *Raging Battle Mouse*, *Uthros Psionicist*). `EQ0`/`LT1`/`GE1` continue
    /// to emit canonical `Condition::YouCastNoSpellThisTurn`.
    pub(super) fn wave28c_this_turn_cast_compare(def: &str, cmp: &str) -> Option<String> {
        if def != "Count$ThisTurnCast_Card.YouCtrl" {
            return None;
        }
        let trimmed = cmp.trim();
        if let Some(n) = trimmed
            .strip_prefix("EQ")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&n| n >= 1)
        {
            return Some(format!(
                "Condition::AmountCompare {{ left: &Amount::SpellsCastThisTurn(PlayerRel::You), \
                 op: AmountCmp::Equal, right: &Amount::Fixed({n}) }}"
            ));
        }
        if let Some(n) = trimmed
            .strip_prefix("NE")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&n| n >= 1)
        {
            return Some(format!(
                "Condition::AmountCompare {{ left: &Amount::SpellsCastThisTurn(PlayerRel::You), \
                 op: AmountCmp::NotEqual, right: &Amount::Fixed({n}) }}"
            ));
        }
        None
    }

    /// Generalized `S: Mode$ RaiseCost` `ValidTarget$` filter for battlefield
    /// permanents you control (*Esior, Wardwing Familiar*, *Kasmina, Enigmatic
    /// Mentor*, spell half of *Kopala, Warden of Waves*), lowered onto
    /// `Filter::TargetsYouControl(&Filter)`.
    ///
    /// Every comma-alternative must carry `YouCtrl` and target a battlefield
    /// permanent (`inZoneBattlefield` or a permanent-only base type/subtype);
    /// targets without `YouCtrl` stay honestly refused.
    pub(super) fn wave28c_raise_cost_target_filter(
        &mut self,
        valid_target: &str,
    ) -> Option<String> {
        let mut stripped_alts = Vec::new();
        for raw_alt in valid_target.split(',') {
            let alt = raw_alt.trim();
            if alt.is_empty() {
                return None;
            }
            let mut parts = alt
                .split('.')
                .flat_map(|segment| segment.split('+'))
                .map(str::trim)
                .filter(|atom| !atom.is_empty());
            let base = parts.next()?;
            if matches!(
                base,
                "Player" | "You" | "Opponent" | "Spell" | "Instant" | "Sorcery" | "Self" | "Other"
            ) || base.contains("Ctrl")
                || base.contains("Own")
            {
                return None;
            }
            let mut saw_you_ctrl = false;
            let mut saw_bf = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "YouCtrl" => saw_you_ctrl = true,
                    "inZoneBattlefield" => saw_bf = true,
                    other
                        if other.contains("Ctrl")
                            || other.contains("Own")
                            || other.contains("inZone")
                            || other.contains("Remembered")
                            || other.contains("Targeted")
                            || other.contains("Triggered")
                            || matches!(
                                other,
                                "Self"
                                    | "StrictlySelf"
                                    | "Other"
                                    | "StrictlyOther"
                                    | "Enchanted"
                                    | "EnchantedBy"
                                    | "Equipped"
                                    | "EquippedBy"
                                    | "Attached"
                                    | "AttachedBy"
                            ) =>
                    {
                        return None;
                    }
                    other => kept.push(other),
                }
            }
            if !saw_you_ctrl {
                return None;
            }
            if !saw_bf && !self.is_permanent_only_base(base) {
                return None;
            }
            if kept.is_empty() {
                stripped_alts.push(base.to_string());
            } else {
                stripped_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        if stripped_alts.is_empty() {
            return None;
        }
        let expr = self.filter_expr(&stripped_alts.join(","))?;
        if expr == "Filter::Any" {
            return Some("Filter::TargetsPermanentYouControl".to_string());
        }
        let inner_ref = if simple_const_filter(&expr) || expr == "Filter::IsCommander" {
            format!("&{expr}")
        } else {
            format!("&{}", self.body.filter_static("TARGET_YOU_CTRL", &expr))
        };
        Some(format!("Filter::TargetsYouControl({inner_ref})"))
    }

    fn is_permanent_only_base(&self, base: &str) -> bool {
        matches!(
            base,
            "Permanent"
                | "Creature"
                | "Planeswalker"
                | "Artifact"
                | "Enchantment"
                | "Land"
                | "Battle"
        ) || [
            SubtypeKind::Creature,
            SubtypeKind::Artifact,
            SubtypeKind::Enchantment,
            SubtypeKind::Land,
            SubtypeKind::Planeswalker,
            SubtypeKind::Battle,
        ]
        .into_iter()
        .any(|kind| self.cats.const_path_of(kind, base).is_some())
    }

    /// `Mana.RestrictValid$ CantCastSpellFromHand` (*Karolina Dean, Runaway*,
    /// *Vhal, Candlekeep Researcher*): mana may be spent on any non-spell
    /// payment or on any spell NOT cast from your hand.
    ///
    /// Both pre-cast offers (`Zone::Hand`, before `CastHistory` is populated)
    /// and stack payments (`Zone::Stack` with `Filter::CastFromHand`) are
    /// excluded for cards owned by you, without falsely rejecting Exile,
    /// Graveyard, Command, or Library spells (whose default `CastHistory` is
    /// still `from_hand`).
    pub(super) fn wave28c_cant_cast_spell_from_hand_restriction()
    -> (String, Option<&'static str>, Option<String>) {
        (
            "Filter::Not(&Filter::And(&[Filter::OwnedByYou, \
             Filter::Or(&[Filter::InZone(ZoneRef::Hand), \
             Filter::And(&[Filter::InZone(ZoneRef::Stack), Filter::CastFromHand])])]))"
                .to_string(),
            Some("ManaUse::NonSpellPaymentsOrMatchingSpells"),
            None,
        )
    }

    /// Mixed unconditional land-play + filtered spell-cast `MayPlay$ True` with
    /// `ValidAfterStack$ Spell.<qual>` on `AffectedZone$ Library` (*Glarb,
    /// Calamity's Augur*) or `AffectedZone$ Graveyard` (*Zask, Skittering
    /// Swarmlord*).
    ///
    /// Emits `PlayLandsFromLibraryTop` / `PlayLandsFromGraveyard` alongside a
    /// nonland `MayCastWithAltCost { allow_lands: false, price: Normal }`.
    /// Refuses `MayPlayLimit$`, `MayPlayWithFlash$`, `RaiseCost$`, alt-costs,
    /// and exile/finality riders because the two permissions do not share a
    /// once-per-turn cap or replacement rider.
    pub(super) fn wave28c_mixed_land_and_spell_play(
        &mut self,
        mut p: Params,
        is_library: bool,
    ) -> Option<()> {
        let zone_name = if is_library { "Library" } else { "Graveyard" };
        let deny = |tx: &Self| {
            tx.deny(format!(
                "static ability reaching `AffectedZone$ {zone_name}`"
            ))
        };
        let desc_has_unmodeled_rider = p.peek("Description").is_some_and(|d| {
            let lower = d.to_ascii_lowercase();
            lower.contains("finality counter") || lower.contains("exile it instead")
        });
        p.drop_prose();
        if desc_has_unmodeled_rider
            || p.take("AffectedZone").as_deref() != Some(zone_name)
            || !matches!(p.take("EffectZone").as_deref(), None | Some("Battlefield"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
            || p.take("MayPlay").as_deref() != Some("True")
            || p.has("MayPlayLimit")
            || p.has("MayPlayWithFlash")
            || p.has("RaiseCost")
            || p.has("MayPlayAltManaCost")
            || p.has("MayPlayWithoutManaCost")
            || p.has("MayLookAt")
            || p.has("ReplaceGraveyard")
        {
            return deny(self);
        }
        let Some(affected) = p.take("Affected") else {
            return deny(self);
        };
        let Some(valid_after_stack) = p.take("ValidAfterStack") else {
            return deny(self);
        };
        let cond = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }

        let Some(vas_qual) = valid_after_stack.trim().strip_prefix("Spell.") else {
            return deny(self);
        };
        if !is_simple_flash_spell_qualifier(vas_qual) {
            return deny(self);
        }
        let vas_atoms: Vec<&str> = vas_qual
            .split('+')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect();
        if vas_atoms.is_empty() {
            return deny(self);
        }

        let Some(spell_alts) = parse_mixed_land_and_spell_alts(&affected, &vas_atoms, is_library)
        else {
            return deny(self);
        };
        let Some(base_spell_filter) = self.filter_expr_in_zone(&spell_alts.join(","), true) else {
            return deny(self);
        };
        let (land_mod, zones_expr, spell_filter) = if is_library {
            (
                "Modifier::PlayLandsFromLibraryTop",
                "AltCastZones::TopOfLibrary",
                base_spell_filter,
            )
        } else {
            let owned = if base_spell_filter == "Filter::Any" {
                "Filter::OwnedByYou".to_string()
            } else {
                format!("Filter::And(&[{base_spell_filter}, Filter::OwnedByYou])")
            };
            (
                "Modifier::PlayLandsFromGraveyard",
                "AltCastZones::Zones(&[ZoneRef::Graveyard])",
                owned,
            )
        };
        self.push_static_with_cond("Filter::Any", land_mod, &cond);
        let cast_mod = format!(
            "Modifier::MayCastWithAltCost {{ who: PlayerRel::You, zones: {zones_expr}, \
             grant_zone_permission: true, allow_lands: false, with_flash: false, \
             max_mana_value: None, once_per_turn: false, price: AltCastPrice::Normal }}"
        );
        self.push_static_with_cond(&spell_filter, &cast_mod, &cond);
        Some(())
    }

    /// Additional static-safe `Amount` expressions hooked from
    /// `statics::static_count_expr_ext`:
    /// - `Count$ValidGraveyard <valid>$GreatestCardPower` (*Carrion Grub*) ->
    ///   `Amount::GreatestPowerAmong(&COUNT_N)` with `InZone(Graveyard)`
    /// - `Count$ValidGraveyard <valid>$GreatestCardManaCost` ->
    ///   `Amount::GreatestManaValueAmong(&COUNT_N)` with `InZone(Graveyard)`
    /// - `Count$ValidExile <valid>` (unscoped or `YouOwn`, excluding
    ///   linked-exile / remembered / opponent-ownership approximations) ->
    ///   `Amount::CountOf { filter: ..., zone: ZoneSel::ExileAll }`
    pub(super) fn wave28c_static_count_expr(&mut self, trimmed: &str) -> Option<String> {
        if let Some(expr) = self.wave28c_graveyard_greatest_expr(trimmed) {
            return Some(expr);
        }
        if let Some(valid) = trimmed.strip_prefix("Count$ValidExile ") {
            if valid.is_empty()
                || valid.contains(['$', '/'])
                || has_forbidden_zone_count_atom(valid)
                || valid.contains("OppCtrl")
                || valid.contains("YouCtrl")
            {
                return None;
            }
            let expr = if valid.contains("OppOwn") {
                if valid.contains(',') || valid.contains("YouOwn") || valid.contains("YouDontOwn") {
                    return None;
                }
                let (base, rest) = valid.split_once('.')?;
                if base.contains("OppOwn") {
                    return None;
                }
                let mut saw_opp = false;
                let mut kept = Vec::new();
                for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                    if atom == "OppOwn" {
                        saw_opp = true;
                    } else {
                        kept.push(atom);
                    }
                }
                if !saw_opp {
                    return None;
                }
                let cleaned = if kept.is_empty() {
                    base.to_string()
                } else {
                    format!("{base}.{}", kept.join("+"))
                };
                let inner = self.filter_expr_in_zone(&cleaned, true)?;
                if inner == "Filter::Any" {
                    "Filter::OwnedBy(PlayerRel::EachOpponent)".to_string()
                } else {
                    format!("Filter::And(&[{inner}, Filter::OwnedBy(PlayerRel::EachOpponent)])")
                }
            } else {
                self.filter_expr_in_zone(valid, true)?
            };
            let filter_ref = if simple_const_filter(&expr) || expr == "Filter::OwnedByYou" {
                format!("&{expr}")
            } else {
                format!("&{}", self.body.filter_static("COUNT", &expr))
            };
            return Some(format!(
                "Amount::CountOf {{ filter: {filter_ref}, zone: ZoneSel::ExileAll }}"
            ));
        }
        None
    }

    fn wave28c_graveyard_greatest_expr(&mut self, trimmed: &str) -> Option<String> {
        let rest = trimmed.strip_prefix("Count$ValidGraveyard ")?;
        let (valid, variant) = if let Some(v) = rest.strip_suffix("$GreatestCardPower") {
            (v, "Amount::GreatestPowerAmong")
        } else {
            let v = rest.strip_suffix("$GreatestCardManaCost")?;
            (v, "Amount::GreatestManaValueAmong")
        };
        if valid.is_empty() || valid.contains(['$', '/']) || has_forbidden_zone_count_atom(valid) {
            return None;
        }
        // Cards in graveyards have owners, not controllers (CR 404.1 / 108.4a);
        // Forge sometimes writes `YouCtrl` for "in your graveyard" (*Hapatra*,
        // *Wick's Patrol*). Normalize `YouCtrl` -> `YouOwn` and refuse opponent
        // ownership/controller approximations.
        let normalized = normalize_graveyard_owner_for_greatest(valid)?;
        let base = self.filter_expr_in_zone(&normalized, true)?;
        let full = if base == "Filter::Any" {
            "Filter::InZone(ZoneRef::Graveyard)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::Graveyard)])")
        };
        let name = self.body.filter_static("COUNT", &full);
        Some(format!("{variant}(&{name})"))
    }

    /// Wave 28c-p2 `S: Mode$ AlternativeCost | ValidSA$ Spell.Self |
    /// EffectZone$ Graveyard | Cost$ <pure_mana>` (*Raffine's Guidance*) ->
    /// `Modifier::MayCastWithAltCost` from `ZoneRef::Graveyard`.
    ///
    /// Refuses non-mana costs (*Scourge of Nel Toth*, *Squee, Dubious Monarch*),
    /// `Count$AltCost` rider cards (*Worldheart Phoenix*), non-self spells, and
    /// opponent players honestly.
    pub(super) fn wave28c_graveyard_self_alt_cost(&mut self, mut p: Params) -> Option<()> {
        let deny = |tx: &Self| tx.deny("alternative cost from `EffectZone$ Graveyard`".to_string());
        if self.has_evoke || self.svars.values().any(|v| v.contains("Count$AltCost")) {
            return deny(self);
        }
        p.drop_prose();
        match p.take("ValidSA").as_deref() {
            Some("Spell.Self") => {
                if let Some(card) = p.take("ValidCard")
                    && card != "Card.Self"
                {
                    return deny(self);
                }
            }
            Some("Spell") if p.take("ValidCard").as_deref() == Some("Card.Self") => {}
            _ => return deny(self),
        }
        if !matches!(p.take("ValidPlayer").as_deref(), None | Some("You"))
            || p.take("EffectZone").as_deref() != Some("Graveyard")
        {
            return deny(self);
        }
        let Some(raw_cost) = p.take("Cost") else {
            return deny(self);
        };
        if raw_cost.contains('X') {
            return deny(self);
        }
        let Some(price) = parse_fixed_alt_mana_price(&raw_cost) else {
            return deny(self);
        };
        let cond =
            self.static_conditions(&mut p, "alternative cost from `EffectZone$ Graveyard`")?;
        if !p.exhausted() {
            return deny(self);
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::MayCastWithAltCost {{ \
             who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), \
             grant_zone_permission: true, allow_lands: false, with_flash: false, \
             max_mana_value: None, once_per_turn: false, price: {price} }}, \
             zone = ZoneRef::Graveyard{cond})"
        ));
        Some(())
    }

    /// Wave 28c-p2 `Mana.CombatMana$ True` (*Avatar Roku, Firebender*) ->
    /// `Effect::AddCombatMana { color, amount }`.
    ///
    /// Accepts fixed single-color mana symbols with a positive constant amount
    /// and no spend restriction, spend rider, or `PersistentMana$ True`.
    pub(super) fn wave28c_combat_mana_effects(
        &self,
        produced: &str,
        fixed: Option<u32>,
        has_modifier: bool,
        persistent: bool,
    ) -> Option<Vec<String>> {
        let deny = || self.deny("unclaimed parameter `Mana.CombatMana`".to_string());
        let Some(mult) = fixed.filter(|&n| n >= 1) else {
            return deny();
        };
        if has_modifier || persistent {
            return deny();
        }
        let mut counts: Vec<(&'static str, u32)> = Vec::new();
        for tok in produced.split_whitespace() {
            let color = match tok {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                _ => return deny(),
            };
            if let Some((_, existing)) = counts.iter_mut().find(|(c, _)| *c == color) {
                *existing = existing.checked_add(mult)?;
            } else {
                counts.push((color, mult));
            }
        }
        if counts.is_empty() {
            return deny();
        }
        let mut out = Vec::with_capacity(counts.len());
        for (color, total) in counts {
            let Ok(amount) = u16::try_from(total) else {
                return deny();
            };
            out.push(format!(
                "Effect::AddCombatMana {{ color: {color}, amount: {amount} }}"
            ));
        }
        Some(out)
    }

    /// Wave 28c-p2 general `Amount` readers hooked into `mana::count_expr_depth`:
    /// - `<lhs>/LimitMin.<k|var>` (*Triumphant Chomp*, *Prophet of the Scarab*)
    ///   -> `Amount::IfCondition` (resolution-only)
    /// - `Count$Devotion.<Color>` & `Count$DevotionDual.<C1>.<C2>` ->
    ///   `Amount::Devotion`
    /// - `Count$YouDrewThisTurn` / `Count$CardsDrawnThisTurn` ->
    ///   `Amount::CardsDrawnThisTurn(PlayerRel::You)`
    /// - `Count$ValidGraveyard <valid>$GreatestCardPower` /
    ///   `$GreatestCardManaCost` -> `Amount::GreatestPowerAmong` /
    ///   `Amount::GreatestManaValueAmong`
    /// - `Count$Valid <valid>$GreatestCardPower` / `$CardPower` (resolution-only,
    ///   preserving Layer 7c static P/T circularity guards) ->
    ///   `Amount::GreatestPowerAmong` / `Amount::TotalPowerAmong`
    pub(super) fn wave28c_count_expr(
        &mut self,
        def: &str,
        depth: u8,
        static_only: bool,
    ) -> Option<String> {
        if static_only {
            return None;
        }
        let trimmed = def.trim();
        if depth < 3
            && let Some((lhs, rhs)) = trimmed.rsplit_once("/LimitMin.")
        {
            return self.wave28c_limit_min_amount(lhs.trim(), rhs.trim(), depth);
        }
        if let Some(devotion) = parse_wave28c_devotion(trimmed) {
            return Some(devotion);
        }
        if matches!(
            trimmed,
            "Count$YouDrewThisTurn" | "Count$CardsDrawnThisTurn"
        ) {
            return Some("Amount::CardsDrawnThisTurn(PlayerRel::You)".to_string());
        }
        if let Some(expr) = self.wave28c_graveyard_greatest_expr(trimmed) {
            return Some(expr);
        }
        if let Some(rest) = trimmed.strip_prefix("Count$Valid ") {
            return self.wave28c_battlefield_power_amount(rest);
        }
        None
    }

    fn wave28c_limit_min_amount(&mut self, lhs: &str, rhs: &str, depth: u8) -> Option<String> {
        let base = if let Some(var) = lhs.strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.clone();
            self.count_expr_depth(&inner, depth + 1, false)?
        } else {
            self.count_expr_depth(lhs, depth + 1, false)?
        };
        if base.starts_with("Amount::Fixed(") || base.contains("Neg") {
            return None;
        }
        let min_expr = if let Ok(k) = rhs.parse::<u32>() {
            if k == 0 {
                return None;
            }
            format!("Amount::Fixed({k})")
        } else {
            let inner = self.svars.get(rhs)?.clone();
            let right = self.count_expr_depth(&inner, depth + 1, false)?;
            if right.contains("Neg") {
                return None;
            }
            right
        };
        Some(format!(
            "Amount::IfCondition {{ condition: &Condition::AmountCompare {{ left: &{base}, \
             op: AmountCmp::AtLeast, right: &{min_expr} }}, then: &{base}, otherwise: &{min_expr} }}"
        ))
    }

    fn wave28c_battlefield_power_amount(&mut self, rest: &str) -> Option<String> {
        let (valid, variant) = if let Some(v) = rest.strip_suffix("$GreatestCardPower") {
            (v, "Amount::GreatestPowerAmong")
        } else {
            let v = rest.strip_suffix("$CardPower")?;
            (v, "Amount::TotalPowerAmong")
        };
        if valid.is_empty()
            || valid.contains(['$', '/'])
            || valid.contains("inZone")
            || valid.contains("ChosenCard")
            || valid.contains("Remembered")
            || valid.contains("Targeted")
            || valid.contains("Triggered")
        {
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
}

fn normalize_graveyard_owner_for_greatest(valid: &str) -> Option<String> {
    let mut out = Vec::new();
    for raw_alt in valid.split(',') {
        let alt = raw_alt.trim();
        if alt.is_empty() {
            return None;
        }
        let mut parts = alt
            .split('.')
            .flat_map(|seg| seg.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let base = parts.next()?;
        if base.contains("Ctrl") || base.contains("Own") {
            return None;
        }
        let mut kept = Vec::new();
        for atom in parts {
            match atom {
                "YouCtrl" | "YouOwn" => {
                    if !kept.contains(&"YouOwn") {
                        kept.push("YouOwn");
                    }
                }
                other if other.contains("Ctrl") || other.contains("Own") => return None,
                other => kept.push(other),
            }
        }
        if kept.is_empty() {
            out.push(base.to_string());
        } else {
            out.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!out.is_empty()).then(|| out.join(","))
}

fn parse_wave28c_devotion(def: &str) -> Option<String> {
    if let Some(color_name) = def.strip_prefix("Count$Devotion.") {
        let color = parse_devotion_color(color_name.trim())?;
        return Some(format!(
            "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::{color}]) }}"
        ));
    }
    if let Some(dual) = def.strip_prefix("Count$DevotionDual.") {
        let (c1, c2) = dual.split_once('.')?;
        if c2.contains('.') {
            return None;
        }
        let color1 = parse_devotion_color(c1.trim())?;
        let color2 = parse_devotion_color(c2.trim())?;
        if color1 == color2 {
            return None;
        }
        return Some(format!(
            "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::{color1}, Color::{color2}]) }}"
        ));
    }
    None
}

fn parse_devotion_color(raw: &str) -> Option<&'static str> {
    Some(match raw {
        "White" => "White",
        "Blue" => "Blue",
        "Black" => "Black",
        "Red" => "Red",
        "Green" => "Green",
        _ => return None,
    })
}

fn parse_mixed_land_and_spell_alts(
    affected: &str,
    vas_atoms: &[&str],
    is_library: bool,
) -> Option<Vec<String>> {
    let mut saw_plain_land = false;
    let mut spell_alts: Vec<String> = Vec::new();
    for raw_alt in affected.split(',') {
        let mut parts = raw_alt
            .split('.')
            .flat_map(|segment| segment.split('+'))
            .map(str::trim)
            .filter(|atom| !atom.is_empty());
        let base = parts.next()?;
        if matches!(base, "Self" | "Other" | "StrictlyOther")
            || base.contains("Ctrl")
            || base.contains("Own")
        {
            return None;
        }
        let mut saw_you = false;
        let mut saw_top = false;
        let mut kept: Vec<&str> = Vec::new();
        for atom in parts {
            match atom {
                "YouCtrl" | "YouOwn" => saw_you = true,
                "TopLibrary" if is_library => saw_top = true,
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
        if !saw_you || (is_library && !saw_top) {
            return None;
        }
        if base == "Land" {
            if !kept.is_empty() {
                return None;
            }
            saw_plain_land = true;
            continue;
        }
        for &extra in vas_atoms {
            if extra != base && !kept.contains(&extra) {
                kept.push(extra);
            }
        }
        if kept.is_empty() {
            spell_alts.push(base.to_string());
        } else {
            spell_alts.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (saw_plain_land && !spell_alts.is_empty()).then_some(spell_alts)
}

fn format_condition_clauses(clauses: &[String]) -> String {
    match clauses {
        [] => String::new(),
        [one] => format!(", condition = Some({one})"),
        many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
    }
}

fn is_simple_flash_spell_qualifier(qual: &str) -> bool {
    if qual.is_empty() || qual.contains([' ', ',', '$', '/']) {
        return false;
    }
    qual.split('+').map(str::trim).all(|atom| {
        !atom.is_empty()
            && !atom.starts_with("IsTargeting")
            && !atom.starts_with("XCost")
            && !atom.starts_with("CastSa")
            && !atom.contains("Ctrl")
            && !atom.contains("Own")
            && !atom.contains("Remembered")
            && !atom.contains("Targeted")
            && !atom.contains("Triggered")
            && !atom.contains("inZone")
            && !atom.contains("MayPlay")
            && !atom.contains("wasCast")
            && !atom.contains("WasCast")
            && !matches!(
                atom,
                "Self"
                    | "StrictlySelf"
                    | "Other"
                    | "StrictlyOther"
                    | "Equipped"
                    | "Enchanted"
                    | "Attached"
                    | "EquippedBy"
                    | "EnchantedBy"
                    | "AttachedBy"
                    | "Flashback"
                    | "Buyback"
                    | "Mutate"
                    | "Blitz"
                    | "Warp"
                    | "Sneak"
                    | "Kicked"
                    | "kicked"
                    | "Bargain"
                    | "Activated"
            )
    })
}

fn has_forbidden_zone_count_atom(valid: &str) -> bool {
    valid.split(',').any(|alt| {
        alt.split(['.', '+']).map(str::trim).any(|atom| {
            atom.contains("inZone")
                || atom.contains("Remembered")
                || atom.contains("Targeted")
                || atom.contains("Triggered")
                || matches!(
                    atom,
                    "Self"
                        | "StrictlySelf"
                        | "Other"
                        | "StrictlyOther"
                        | "ExiledWithSource"
                        | "ExiledWith"
                        | "IsImprinted"
                        | "ChosenCard"
                        | "Equipped"
                        | "EquippedBy"
                        | "Enchanted"
                        | "EnchantedBy"
                        | "Attached"
                        | "AttachedBy"
                        | "TopLibrary"
                )
        })
    })
}

// =========================================================================
// Wave 28c-p3: CR-exact `Effect` API lowerings hooked from `regeneration.rs`
// =========================================================================

impl Tx<'_> {
    /// Fallback `Effect` API lowering for Wave 28c-p3 patterns whose exact CR
    /// semantics already exist in `baylee-cards-dsl` + `baylee-engine`.
    ///
    /// Runs only when `regeneration.rs::static_effect_inner` returns `None`,
    /// with full snapshot/rollback around every sub-matcher so unhandled shapes
    /// still fall through cleanly to `wave3_effect_api` and existing handlers.
    pub(super) fn wave28c_static_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        #[allow(clippy::type_complexity)]
        let handlers: &[fn(&mut Self, &mut Params, Option<&str>) -> Option<Vec<String>>] = &[
            Self::wave28c_effect_cant_block_source,
            Self::wave28c_effect_cant_target,
            Self::wave28c_effect_stack_prevent_damage,
            Self::wave28c_effect_assign_unblocked,
            Self::wave28c_effect_misstep_dont_untap,
            Self::wave28c_effect_multi_lock_or_player_combat,
            Self::wave28c_effect_attached_or_locked_set,
            Self::wave28c_effect_source_lifetime_lock,
            Self::wave28c_effect_yawgmoths_will,
            Self::wave28c_effect_temp_zone_play_or_cost,
            Self::wave28c_effect_targeted_damage_or_life_lock,
            Self::wave28c_effect_delayed_triggers,
        ];
        let snap = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        for handler in handlers {
            if let Some(effects) = handler(self, p, target) {
                return Some(effects);
            }
            p.clone_from(&snap);
            self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
            self.body.statics.truncate(saved_statics_len);
        }
        None
    }

    /// Targeted `Mode$ CantBlockBy` (`ValidAttacker$ Creature.EffectSource`,
    /// `ValidBlocker$ Creature.IsRemembered`) including `Duration$ UntilHostLeavesPlayOrEOT`
    /// (*Auriok Siege Sled*, *Burning-Tree Bloodscale*, *Fearsome Temper*,
    /// *Kozilek's Pathfinder*, *Screeching Griffin*, *Shrewd Hatchling*, *Spin Engine*).
    fn wave28c_effect_cant_block_source(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            || p.has("ReplacementEffects")
            || p.has("Triggers")
        {
            return None;
        }
        if !matches!(
            p.take("Duration").as_deref(),
            None | Some("UntilEndOfTurn" | "UntilHostLeavesPlayOrEOT")
        ) {
            return None;
        }
        if !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget" | "ThisTargetedCard")
        ) {
            return None;
        }
        if !matches!(
            p.take("ForgetOnMoved")
                .or_else(|| p.take("ExileOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        p.take("IsCurse");
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantBlockBy"
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
            || !matches!(
                st.take("ValidAttacker").as_deref(),
                Some("Creature.EffectSource" | "Card.EffectSource")
            )
            || !matches!(
                st.take("ValidBlocker").as_deref(),
                Some("Creature.IsRemembered" | "Card.IsRemembered")
            )
            || !st.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::ModifyTargets { modifier: Modifier::CantBlock(&Filter::This), \
             duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// Temporary `Mode$ CantTarget` -> `Modifier::CantBeTargetedBy`
    /// (*Vines of Vastwood*, *Veilstone Amulet*, *Display of Dominance*).
    fn wave28c_effect_cant_target(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.has("ReplacementEffects") || p.has("Triggers") {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        if !matches!(
            p.take("ExileOnMoved")
                .or_else(|| p.take("ForgetOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        let subject = self.effect_remember_subject(p, target)?;
        if matches!(
            (p.peek("Defined"), subject),
            (Some("Targeted"), EffectSubject::Targets)
        ) {
            p.take("Defined");
        }
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantTarget"
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
        {
            return None;
        }
        let valid_target = st.take("ValidTarget")?;
        let activator = st.take("Activator");
        let valid_source = st.take("ValidSource");
        let valid_sa = st.take("ValidSA");
        if !st.exhausted() {
            return None;
        }
        let src_ref = match (
            activator.as_deref(),
            valid_source.as_deref(),
            valid_sa.as_deref(),
        ) {
            (Some("Opponent" | "Player.Opponent"), None, None) => {
                "&Filter::ControlledByOpponent".to_string()
            }
            (Some("You"), None, None) => "&Filter::ControlledByYou".to_string(),
            (Some("Player"), None, None) => "&Filter::Any".to_string(),
            (None, Some(vs), sa @ (None | Some("Spell")))
                if !vs.contains("Remembered")
                    && !vs.contains("EffectSource")
                    && !vs.contains("ChosenCard")
                    && !vs.contains("Self") =>
            {
                let inner = self.filter_expr_in_zone(vs, true)?;
                let full = if sa == Some("Spell") {
                    if inner == "Filter::Any" {
                        "Filter::InZone(ZoneRef::Stack)".to_string()
                    } else {
                        format!("Filter::And(&[Filter::InZone(ZoneRef::Stack), {inner}])")
                    }
                } else {
                    inner
                };
                format!("&{}", self.body.filter_static("EFFECT", &full))
            }
            _ => return None,
        };
        let modifier = format!("Modifier::CantBeTargetedBy({src_ref})");
        self.emit_effect_modifiers(subject, &valid_target, &[modifier], duration)
    }

    /// Stack-spell `PreventAllDamageFromIt` (*Dromoka's Command*, *Hidden Retreat*,
    /// *Shieldmage Elder*).
    fn wave28c_effect_stack_prevent_damage(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !target.is_some_and(|t| t.starts_with("TargetSpec::Spell("))
            || p.has("ReplacementEffects")
            || p.has("Triggers")
        {
            return None;
        }
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
            || p.take("RememberObjects").as_deref() != Some("TargetedSource")
            || !matches!(
                p.take("ExileOnMoved")
                    .or_else(|| p.take("ForgetOnMoved"))
                    .as_deref(),
                None | Some("Stack")
            )
            || !matches!(p.take("Stackable").as_deref(), None | Some("False"))
        {
            return None;
        }
        if p.peek("TargetType") == Some("Spell") {
            p.take("TargetType");
        }
        p.take("IsCurse");
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "Continuous"
            || !matches!(
                st.take("AffectedZone").as_deref(),
                Some("Stack" | "Battlefield,Stack" | "Stack,Battlefield")
            )
            || st.take("Affected").as_deref() != Some("Card.IsRemembered")
            || st.take("AddKeyword").as_deref()
                != Some("Prevent all damage that would be dealt by CARDNAME.")
            || !st.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::ModifyTargets { modifier: Modifier::PreventAllDamageFromIt, \
             duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// Temporary `Mode$ AssignCombatDamageAsUnblocked` — strictly requires
    /// `Optional$ True` because `Modifier::MayAssignCombatDamageAsThoughUnblocked`
    /// is optional (lowers *Predatory Focus*; refuses mandatory *Outmaneuver*).
    fn wave28c_effect_assign_unblocked(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.has("ReplacementEffects") || p.has("Triggers") {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let subject = self.effect_remember_subject(p, target)?;
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "AssignCombatDamageAsUnblocked"
            || st.take("Optional").as_deref() != Some("True")
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
        {
            return None;
        }
        let valid = st.take("ValidCard")?;
        if !st.exhausted() {
            return None;
        }
        self.emit_effect_modifiers(
            subject,
            &valid,
            &["Modifier::MayAssignCombatDamageAsThoughUnblocked".to_string()],
            duration,
        )
    }

    /// "Creatures target player controls don't untap during that player's next
    /// untap step" (*Misstep* -> `Effect::TargetPlayersCreaturesDontUntapNextUntap`).
    /// Strictly rejects lands (*Mana Vapors*, *Exhaustion*).
    fn wave28c_effect_misstep_dont_untap(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !matches!(
            target,
            Some(
                "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::Player(PlayerRel::Chosen)"
            )
        ) || p.has("ReplacementEffects")
        {
            return None;
        }
        if p.take("Duration").as_deref() != Some("Permanent")
            || p.take("RememberObjects").as_deref() != Some("Targeted")
        {
            return None;
        }
        p.take("IsCurse");
        let statics = p.take("StaticAbilities")?;
        let triggers = p.take("Triggers")?;
        if statics.contains(',') || triggers.contains(',') || !p.exhausted() {
            return None;
        }
        let sbody = self.svars.get(statics.trim())?.clone();
        let (smode, mut st) = Params::parse(&sbody)?;
        st.drop_prose();
        if smode != "Continuous"
            || !matches!(
                st.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || st.take("Affected").as_deref() != Some("Creature.RememberedPlayerCtrl")
            || st.take("AddHiddenKeyword").as_deref()
                != Some("This card doesn't untap during your next untap step.")
            || !st.exhausted()
        {
            return None;
        }
        let tbody = self.svars.get(triggers.trim())?.clone();
        let (tmode, mut tp) = Params::parse(&tbody)?;
        tp.drop_prose();
        if tmode != "Phase"
            || tp.take("Phase").as_deref() != Some("Untap")
            || tp.take("ValidPlayer").as_deref() != Some("Player.IsRemembered")
            || !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command"))
            || tp.take("Static").as_deref() != Some("True")
        {
            return None;
        }
        let exec = tp.take("Execute")?;
        if !tp.exhausted() || !self.exiles_itself(exec.trim()) {
            return None;
        }
        Some(vec![
            "Effect::TargetPlayersCreaturesDontUntapNextUntap".to_string(),
        ])
    }

    /// 6a/6b. Multi-mode `CantAttack,CantBlock[,CantBeActivated]` on targeted
    /// permanents (*Linvala, Shield of Sea Gate*, *Mythos of Vadrok*, *Kaito* `+1`)
    /// and player-targeted `CantBlock` / `CantAttack` (+ `CantBeCast` bundle on
    /// *Ember Gale*, *Headliner Scarlett*, *Oriss, Samite Guardian*).
    #[allow(clippy::too_many_lines)]
    fn wave28c_effect_multi_lock_or_player_combat(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.has("ReplacementEffects") || p.has("Triggers") {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        if !matches!(
            p.take("ForgetOnMoved")
                .or_else(|| p.take("ExileOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        p.take("IsCurse");
        let subject = self.effect_remember_subject(p, target)?;
        let statics = p.take("StaticAbilities")?;
        if !p.exhausted() {
            return None;
        }
        let mut out = Vec::new();
        for name in statics.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let body = self.svars.get(name)?.clone();
            let (mode, mut st) = Params::parse(&body)?;
            st.drop_prose();
            if !matches!(st.take("EffectZone").as_deref(), None | Some("Command")) {
                return None;
            }
            if mode.contains(',') && subject == EffectSubject::Targets {
                let valid = st.take("ValidCard")?;
                let valid_sa = st.take("ValidSA");
                if !matches!(
                    valid.as_str(),
                    "Card.IsRemembered" | "Creature.IsRemembered"
                ) || !st.exhausted()
                {
                    return None;
                }
                for part in mode.split(',').map(str::trim) {
                    let modifier = match part {
                        "CantAttack" => "Modifier::CantAttack",
                        "CantBlock" => "Modifier::CantBlock(&Filter::Any)",
                        "CantBeActivated" => match valid_sa.as_deref() {
                            None => "Modifier::CantActivate",
                            Some("Activated.!ManaAbility") => "Modifier::CantActivateNonMana",
                            Some(_) => return None,
                        },
                        _ => return None,
                    };
                    out.push(format!(
                        "Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"
                    ));
                }
                continue;
            }
            if let EffectSubject::Player(who) = subject {
                match mode.as_str() {
                    // CR 611.2c: Layer 6 `AddKeyword(CANT_BLOCK)` locks creatures at
                    // resolution (*Ember Gale*, *Headliner Scarlett*).
                    "CantBlock"
                        if st.take("ValidCard").as_deref()
                            == Some("Creature.RememberedPlayerCtrl")
                            && st.exhausted() =>
                    {
                        let name = self.body.filter_static(
                            "EFFECT",
                            &format!(
                                "Filter::And(&[Filter::CREATURE, Filter::ControlledBy({who})])"
                            ),
                        );
                        out.push(format!(
                            "Effect::AddGlobalTempModifier {{ filter: &{name}, \
                             modifier: Modifier::AddKeyword(KeywordSet::CANT_BLOCK), \
                             duration: {duration} }}"
                        ));
                    }
                    // Rules-modifying `Modifier::CantAttack` stays dynamic via
                    // `DslControlledBy` (*Oriss, Samite Guardian*).
                    "CantAttack"
                        if st.take("ValidCard").as_deref()
                            == Some("Creature.RememberedPlayerCtrl")
                            && st.exhausted() =>
                    {
                        out.push(format!(
                            "Effect::ModifyFilter {{ filter: &Filter::CREATURE, \
                             controlled_by: Some({who}), \
                             modifier: Modifier::CantAttack, duration: {duration} }}"
                        ));
                    }
                    "CantBeCast" if st.take("Caster").as_deref() == Some("Player.IsRemembered") => {
                        let valid = st.take("ValidCard").unwrap_or_else(|| "Card".to_string());
                        if !st.exhausted()
                            || valid.contains("Remembered")
                            || valid.contains("EffectSource")
                        {
                            return None;
                        }
                        let filter_ref = if valid == "Card" {
                            "&Filter::Any".to_string()
                        } else {
                            let expr = self.filter_expr(&valid)?;
                            format!("&{}", self.body.filter_static("EFFECT", &expr))
                        };
                        out.push(format!(
                            "Effect::continuous(&Filter::Any, \
                             Modifier::PlayersCantCast {{ who: {who}, filter: {filter_ref} }}, \
                             {duration})"
                        ));
                    }
                    _ => return None,
                }
                continue;
            }
            return None;
        }
        (!out.is_empty()).then_some(out)
    }

    /// 6c/6d/6f. Attached `MustAttack` (*Nettling Curse*), locked-set
    /// `RememberObjects$ Valid <filter>` (`MustAttack` on *Suicidal Charge*,
    /// `CARDNAME can't block.` on *Taunt from the Rampart*), and `Targeted & Self`
    /// `MustAttack` (*Gogo, Mysterious Mime*).
    fn wave28c_effect_attached_or_locked_set(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.has("ReplacementEffects") || p.has("Triggers") {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        if !matches!(
            p.take("ExileOnMoved")
                .or_else(|| p.take("ForgetOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        p.take("IsCurse");
        let rem = p.take("RememberObjects")?;
        let cond_self = p.peek("ConditionPresent") == Some("Card.Self");
        if cond_self {
            p.take("ConditionPresent");
        }
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        let modifier = match mode.as_str() {
            "MustAttack"
                if matches!(
                    st.take("ValidCreature").as_deref(),
                    Some("Creature.IsRemembered" | "Card.IsRemembered")
                ) && st.exhausted() =>
            {
                "Modifier::AttacksEachCombat"
            }
            "Continuous"
                if matches!(
                    st.take("AffectedZone").as_deref(),
                    None | Some("Battlefield")
                ) && matches!(
                    st.take("Affected").as_deref(),
                    Some("Card.IsRemembered" | "Creature.IsRemembered")
                ) && st.take("AddHiddenKeyword").as_deref() == Some("CARDNAME can't block.")
                    && st.exhausted() =>
            {
                "Modifier::CantBlock(&Filter::Any)"
            }
            _ => return None,
        };
        if matches!(rem.as_str(), "Enchanted" | "Equipped")
            && !self.on_a_spell
            && target.is_none()
            && !cond_self
        {
            return Some(vec![format!(
                "Effect::ModifyAttached {{ modifier: {modifier}, duration: {duration} }}"
            )]);
        }
        if matches!(rem.as_str(), "Targeted & Self" | "Self & Targeted")
            && !self.on_a_spell
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
        {
            let both = vec![
                format!("Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"),
                format!("Effect::ModifySource {{ modifier: {modifier}, duration: {duration} }}"),
            ];
            return Some(if cond_self {
                let check = self.body.filter_static(
                    "CHECK",
                    "Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::This])",
                );
                vec![format!(
                    "Effect::IfCondition {{ condition: Condition::SourceMatches(&{check}), \
                     then: &[{}], otherwise: &[] }}",
                    both.join(", ")
                )]
            } else {
                both
            });
        }
        if let Some(valid) = rem.strip_prefix("Valid ").map(str::trim)
            && target.is_none()
            && !cond_self
            && !valid.is_empty()
            && !valid.contains("Remembered")
            && !valid.contains("EffectSource")
            && !valid.contains("Targeted")
        {
            let expr = self.filter_expr(valid)?;
            let filter_static = self.body.filter_static("EFFECT", &expr);
            return Some(vec![format!(
                "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &{filter_static}, \
                 only_found: false, then: &[Effect::ModifyEvent {{ modifier: {modifier}, \
                 duration: {duration} }}] }}"
            )]);
        }
        None
    }

    /// 6e. Source-lifetime targeted locks:
    /// - `DontUntap` with `Duration$ UntilHostLeavesPlay` (*Origin of Iron Man*;
    ///   deliberately refuses `AsLongAsInPlay` so *Tidebinder Mage* stays refused!)
    /// - `RemoveAllAbilities$ True` with `Duration$ UntilHostLeavesPlay | AsLongAsInPlay`
    ///   (*The Wondrous Wasp*, guarding against Oracle "as long as you control")
    /// - `Goad$ True` with `Duration$ UntilHostLeavesPlay` (*Hot Pursuit*).
    fn wave28c_effect_source_lifetime_lock(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            || p.has("Triggers")
        {
            return None;
        }
        let dur_raw = p.take("Duration")?;
        if !matches!(
            p.take("ForgetOnMoved")
                .or_else(|| p.take("ExileOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) || !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget" | "ThisTargetedCard")
        ) {
            return None;
        }
        if p.peek("ConditionDefined") == Some("Targeted")
            && p.peek("ConditionPresent") == Some("Card")
        {
            p.take("ConditionDefined");
            p.take("ConditionPresent");
        }
        p.take("IsCurse");
        if let Some(rep_name) = p.take("ReplacementEffects") {
            if dur_raw != "UntilHostLeavesPlay" || p.has("StaticAbilities") || !p.exhausted() {
                return None;
            }
            let body = self.svars.get(rep_name.trim())?.clone();
            let (event, mut r) = Params::parse(&body)?;
            r.drop_prose();
            if event != "Untap"
                || r.take("Layer").as_deref() != Some("CantHappen")
                || r.take("ValidStepTurnToController").as_deref() != Some("You")
                || !matches!(
                    r.take("ValidCard").as_deref(),
                    Some("Card.IsRemembered" | "Creature.IsRemembered")
                )
                || !r.exhausted()
            {
                return None;
            }
            return Some(vec![
                "Effect::ModifyTargets { modifier: Modifier::DoesNotUntap, \
                 duration: Duration::WhileSourceOnBattlefield }"
                    .to_string(),
            ]);
        }
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "Continuous"
            || !matches!(
                st.take("AffectedZone").as_deref(),
                None | Some("Battlefield")
            )
            || !matches!(
                st.take("Affected").as_deref(),
                Some("Card.IsRemembered" | "Creature.IsRemembered")
            )
        {
            return None;
        }
        if st.take("RemoveAllAbilities").as_deref() == Some("True") && st.exhausted() {
            if !matches!(dur_raw.as_str(), "UntilHostLeavesPlay" | "AsLongAsInPlay")
                || self
                    .oracle
                    .to_ascii_lowercase()
                    .contains("as long as you control")
            {
                return None;
            }
            return Some(vec![
                "Effect::ModifyTargets { modifier: Modifier::LoseAllAbilities, \
                 duration: Duration::WhileSourceOnBattlefield }"
                    .to_string(),
            ]);
        }
        if st.take("Goad").as_deref() == Some("True") && st.exhausted() {
            let duration = match dur_raw.as_str() {
                "UntilHostLeavesPlay" => "Duration::WhileSourceOnBattlefield",
                "AsLongAsControl" => "Duration::WhileYouControlSource",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::ModifyTargets {{ modifier: Modifier::Goaded, duration: {duration} }}"
            )]);
        }
        None
    }

    /// Yawgmoth's Will bundle (*Gaea's Will*, *Magus of the Will*).
    fn wave28c_effect_yawgmoths_will(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("Triggers")
            || p.has("RememberObjects")
            || !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
        {
            return None;
        }
        p.take("AINoRecursiveCheck");
        let statics = p.take("StaticAbilities")?;
        let reps = p.take("ReplacementEffects")?;
        if statics.contains(',') || reps.contains(',') || !p.exhausted() {
            return None;
        }
        let sbody = self.svars.get(statics.trim())?.clone();
        let (smode, mut st) = Params::parse(&sbody)?;
        st.drop_prose();
        if smode != "Continuous"
            || st.take("Affected").as_deref() != Some("Card.YouCtrl")
            || st.take("AffectedZone").as_deref() != Some("Graveyard")
            || st.take("MayPlay").as_deref() != Some("True")
            || !st.exhausted()
        {
            return None;
        }
        let rbody = self.svars.get(reps.trim())?.clone();
        let (revent, mut r) = Params::parse(&rbody)?;
        r.drop_prose();
        if revent != "Moved"
            || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
            || r.take("Destination").as_deref() != Some("Graveyard")
            || r.take("ValidCard").as_deref() != Some("Card.!token+YouOwn")
        {
            return None;
        }
        let rw = r.take("ReplaceWith")?;
        if !r.exhausted() {
            return None;
        }
        let ebody = self.svars.get(rw.trim())?.clone();
        let (eapi, mut ep) = Params::parse(&ebody)?;
        ep.drop_prose();
        if eapi != "ChangeZone"
            || !matches!(ep.take("Hidden").as_deref(), None | Some("True"))
            || ep.take("Origin").as_deref() != Some("All")
            || ep.take("Destination").as_deref() != Some("Exile")
            || ep.take("Defined").as_deref() != Some("ReplacedCard")
            || !ep.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::continuous(&Filter::Any, Modifier::CastSpellsFromGraveyard, \
             Duration::UntilEndOfTurn)"
                .to_string(),
            "Effect::continuous(&Filter::Any, Modifier::ExileInsteadOfYourGraveyard, \
             Duration::UntilEndOfTurn)"
                .to_string(),
            "Effect::continuous(&Filter::Any, Modifier::PlayLandsFromGraveyard, \
             Duration::UntilEndOfTurn)"
                .to_string(),
        ])
    }

    /// 8 & 9. Temporary graveyard/hand/library-top play permissions (*Liliana,
    /// Untouched by Death*, *Chainer, Nightmare Adept*, *Chandra, Flame's
    /// Catalyst*, *The Belligerent*) and temporary face-down spell cost
    /// reduction (*Goblin Maskmaker*).
    #[allow(clippy::too_many_lines)]
    fn wave28c_effect_temp_zone_play_or_cost(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("RememberObjects")
            || p.has("ReplacementEffects")
            || p.has("Triggers")
            || p.has("ForgetOnCast")
        {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        if !matches!(p.take("Stackable").as_deref(), None | Some("False")) {
            return None;
        }
        let statics = p.take("StaticAbilities")?;
        if statics.contains(',') || !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode == "ReduceCost" {
            if st.take("Type").as_deref() != Some("Spell")
                || st.take("Activator").as_deref() != Some("You")
                || !matches!(st.take("EffectZone").as_deref(), None | Some("Battlefield"))
            {
                return None;
            }
            let valid = st.take("ValidCard")?;
            let amount: u8 = st.take("Amount")?.trim().parse().ok().filter(|&n| n > 0)?;
            if !st.exhausted() {
                return None;
            }
            let expr = match valid.as_str() {
                "Card.faceDown" => "Filter::And(&[Filter::FaceDown, Filter::ControlledByYou])",
                "Creature.faceDown" => {
                    "Filter::And(&[Filter::CREATURE, Filter::FaceDown, Filter::ControlledByYou])"
                }
                _ => return None,
            };
            let filter_static = self.body.filter_static("EFFECT", expr);
            return Some(vec![format!(
                "Effect::continuous(&{filter_static}, Modifier::SpellsCostLess({amount}), \
                 {duration})"
            )]);
        }
        if mode != "Continuous" || st.take("MayPlay").as_deref() != Some("True") {
            return None;
        }
        let zone = st.take("AffectedZone")?;
        let affected = st.take("Affected")?;
        let may_look = st.take("MayLookAt");
        let free = match st.take("MayPlayWithoutManaCost").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let once_per_turn = match st.take("MayPlayLimit").as_deref() {
            None => false,
            Some("1") => true,
            Some(_) => return None,
        };
        if !st.exhausted() {
            return None;
        }
        let price = if free {
            "AltCastPrice::Mana(ManaCost::ZERO)"
        } else {
            "AltCastPrice::Normal"
        };
        match zone.as_str() {
            "Library"
                if matches!(
                    affected.as_str(),
                    "Card.TopLibrary+YouCtrl" | "Card.TopLibrary+YouOwn"
                ) && may_look.as_deref() == Some("You")
                    && !free
                    && !once_per_turn =>
            {
                Some(vec![
                    format!(
                        "Effect::continuous(&Filter::Any, \
                         Modifier::MayLookAtTopOfLibrary {{ who: PlayerRel::You }}, {duration})"
                    ),
                    format!(
                        "Effect::continuous(&Filter::Any, Modifier::PlayLandsFromLibraryTop, \
                         {duration})"
                    ),
                    format!(
                        "Effect::continuous(&Filter::Any, Modifier::MayCastWithAltCost {{ \
                         who: PlayerRel::You, zones: AltCastZones::TopOfLibrary, \
                         grant_zone_permission: true, allow_lands: false, with_flash: false, \
                         max_mana_value: None, once_per_turn: false, price: {price} }}, \
                         {duration})"
                    ),
                ])
            }
            "Hand"
                if may_look.is_none()
                    && free
                    && !once_per_turn
                    && matches!(
                        affected.as_str(),
                        "Card.nonLand+YouOwn" | "Card.nonLand+YouCtrl"
                    ) =>
            {
                Some(vec![format!(
                    "Effect::continuous(&Filter::OwnedByYou, Modifier::MayCastWithAltCost {{ \
                     who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Hand]), \
                     grant_zone_permission: false, allow_lands: false, with_flash: false, \
                     max_mana_value: None, once_per_turn: false, price: {price} }}, {duration})"
                )])
            }
            "Graveyard" if may_look.is_none() => {
                let mut alts = Vec::new();
                for alt in affected.split(',').map(str::trim) {
                    let (base, rest) = alt.split_once('.')?;
                    if matches!(base, "Card" | "Land" | "Permanent") && !rest.contains("nonLand") {
                        return None;
                    }
                    let mut saw_you = false;
                    let mut kept = Vec::new();
                    for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                        match atom {
                            "YouCtrl" | "YouOwn" => saw_you = true,
                            "nonLand" => {}
                            other
                                if other.contains("Ctrl")
                                    || other.contains("Own")
                                    || other.contains("Remembered")
                                    || other.contains("EffectSource")
                                    || matches!(other, "Self" | "TopLibrary") =>
                            {
                                return None;
                            }
                            other => kept.push(other),
                        }
                    }
                    if !saw_you {
                        return None;
                    }
                    // `AltCastZones::Zones(&[ZoneRef::Graveyard])` checks zone kind only,
                    // so retain `YouOwn` (`Filter::OwnedByYou`) to keep opponents' graveyards closed.
                    kept.push("YouOwn");
                    alts.push(format!("{base}.{}", kept.join("+")));
                }
                let joined = alts.join(",");
                let filter_ref = if joined == "Card.YouOwn" {
                    "&Filter::OwnedByYou".to_string()
                } else {
                    let expr = self.filter_expr_in_zone(&joined, true)?;
                    format!("&{}", self.body.filter_static("EFFECT", &expr))
                };
                Some(vec![format!(
                    "Effect::continuous({filter_ref}, Modifier::MayCastWithAltCost {{ \
                     who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), \
                     grant_zone_permission: true, allow_lands: false, with_flash: false, \
                     max_mana_value: None, once_per_turn: {once_per_turn}, price: {price} }}, \
                     {duration})"
                )])
            }
            _ => None,
        }
    }

    /// Targeted damage prevention / redirection / life-gain lock:
    /// - *Dazzling Reflection* -> `Effect::PreventNextDamageFromTarget { gain_life: false }`
    /// - *Turn the Tables* -> `Effect::RedirectCombatDamageToYouThisTurn`
    /// - *Vassal's Duty* -> `Effect::RedirectNextDamage`
    /// - *Flames of the Blood Hand* -> `Modifier::PlayersCantGainLife(PlayerRel::TargetPlayerOrController)`
    /// - *Encircling Fissure* -> `Effect::ModifyFilter { ..., modifier: Modifier::PreventDamageFromIt }`
    #[allow(clippy::too_many_lines)]
    fn wave28c_effect_targeted_damage_or_life_lock(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target_spec = target?;
        if p.has("StaticAbilities")
            || !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
        {
            return None;
        }
        p.take("IsCurse");
        if !matches!(
            p.take("ExileOnMoved")
                .or_else(|| p.take("ForgetOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        // 10a. Dazzling Reflection (`ImprintCards$ Targeted` + self-exile trigger).
        if p.peek("ImprintCards") == Some("Targeted") {
            p.take("ImprintCards");
            if !target_spec.starts_with("TargetSpec::Object(") {
                return None;
            }
            let rep_name = p.take("ReplacementEffects")?;
            let trig_name = p.take("Triggers")?;
            if rep_name.contains(',') || trig_name.contains(',') || !p.exhausted() {
                return None;
            }
            let rbody = self.svars.get(rep_name.trim())?.clone();
            let (revent, mut r) = Params::parse(&rbody)?;
            r.drop_prose();
            if revent != "DamageDone"
                || r.take("ValidSource").as_deref() != Some("Card.IsImprinted")
                || r.take("PreventionEffect").as_deref() != Some("True")
            {
                return None;
            }
            let rw = r.take("ReplaceWith")?;
            if !r.exhausted() || !self.exiles_itself(rw.trim()) {
                return None;
            }
            let tbody = self.svars.get(trig_name.trim())?.clone();
            let (tmode, mut tp) = Params::parse(&tbody)?;
            tp.drop_prose();
            if tmode != "ChangesZone"
                || tp.take("Origin").as_deref() != Some("Battlefield")
                || tp.take("Destination").as_deref() != Some("Any")
                || !matches!(
                    tp.take("ValidCard").as_deref(),
                    Some("Creature.IsImprinted" | "Card.IsImprinted")
                )
                || tp.take("Static").as_deref() != Some("True")
            {
                return None;
            }
            let exec = tp.take("Execute")?;
            if !tp.exhausted() || !self.exiles_itself(exec.trim()) {
                return None;
            }
            return Some(vec![
                "Effect::PreventNextDamageFromTarget { gain_life: false }".to_string(),
            ]);
        }
        if p.has("Triggers") {
            return None;
        }
        let rem = p.take("RememberObjects")?;
        let rep_name = p.take("ReplacementEffects")?;
        if rep_name.contains(',') || !p.exhausted() {
            return None;
        }
        let rbody = self.svars.get(rep_name.trim())?.clone();
        let (revent, mut r) = Params::parse(&rbody)?;
        r.drop_prose();
        // 10d. Flames of the Blood Hand (`RememberObjects$ TargetedOrController`).
        if rem == "TargetedOrController"
            && revent == "GainLife"
            && r.take("ValidPlayer").as_deref() == Some("Player.IsRemembered")
            && r.take("Prevent").as_deref() == Some("True")
            && r.exhausted()
        {
            return Some(vec![
                "Effect::continuous(&Filter::Any, \
                 Modifier::PlayersCantGainLife(PlayerRel::TargetPlayerOrController), \
                 Duration::UntilEndOfTurn)"
                    .to_string(),
            ]);
        }
        if rem != "Targeted" || revent != "DamageDone" {
            return None;
        }
        // 10e. Encircling Fissure (`ValidTgts$ Opponent | ValidSource$ Creature.ControlledBy Player.IsRemembered`).
        if matches!(
            target_spec,
            "TargetSpec::AnyOpponent"
                | "TargetSpec::AnyPlayer"
                | "TargetSpec::Player(PlayerRel::Chosen)"
        ) && r.peek("Prevent") == Some("True")
        {
            r.take("Prevent");
            if !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
                || r.take("ValidSource").as_deref()
                    != Some("Creature.ControlledBy Player.IsRemembered")
            {
                return None;
            }
            let modifier = match r.take("IsCombat").as_deref() {
                None => "Modifier::PreventAllDamageFromIt",
                Some("True") => "Modifier::PreventDamageFromIt",
                Some(_) => return None,
            };
            if !r.exhausted() {
                return None;
            }
            return Some(vec![format!(
                "Effect::ModifyFilter {{ filter: &Filter::CREATURE, \
                 controlled_by: Some(PlayerRel::Chosen), modifier: {modifier}, \
                 duration: Duration::UntilEndOfTurn }}"
            )]);
        }
        if !target_spec.starts_with("TargetSpec::Object(") {
            return None;
        }
        // 10b. Turn the Tables (`RedirectCombatDamageToYouThisTurn`).
        if r.peek("IsCombat") == Some("True") && r.peek("ValidTarget") == Some("You") {
            r.take("IsCombat");
            r.take("ValidTarget");
            if r.take("DamageTarget").as_deref() != Some("Remembered")
                || !matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
            {
                return None;
            }
            let valid_source = r.take("ValidSource");
            let rw = r.take("ReplaceWith")?;
            if !r.exhausted() {
                return None;
            }
            let rw_body = self.svars.get(rw.trim())?.clone();
            let (rw_api, mut rp) = Params::parse(&rw_body)?;
            rp.drop_prose();
            if rw_api != "ReplaceEffect"
                || rp.take("VarName").as_deref() != Some("Affected")
                || rp.take("VarValue").as_deref() != Some("Remembered")
                || rp.take("VarType").as_deref() != Some("Card")
                || !rp.exhausted()
            {
                return None;
            }
            let src_ref = match valid_source.as_deref() {
                None => "&Filter::Any".to_string(),
                Some(vs) => {
                    let expr = self.prevention_source_filter(vs)?;
                    format!("&{}", self.body.filter_static("SOURCE", &expr))
                }
            };
            return Some(vec![format!(
                "Effect::RedirectCombatDamageToYouThisTurn {{ by: {src_ref}, \
                 to: DamageRedirectTo::Target({target_spec}) }}"
            )]);
        }
        // 10c. Vassal's Duty (`ReplaceSplitDamage` redirecting next N damage to you).
        if matches!(
            r.take("ValidTarget").as_deref(),
            Some("Creature.IsRemembered" | "Card.IsRemembered")
        ) && r.take("DamageTarget").as_deref() == Some("You")
            && matches!(r.take("ActiveZones").as_deref(), None | Some("Command"))
        {
            let rw = r.take("ReplaceWith")?;
            if !r.exhausted() {
                return None;
            }
            let rw_body = self.svars.get(rw.trim())?.clone();
            let (rw_api, mut rp) = Params::parse(&rw_body)?;
            rp.drop_prose();
            if rw_api != "ReplaceSplitDamage" || rp.take("DamageTarget").as_deref() != Some("You") {
                return None;
            }
            let n: u32 = match rp.take("Amount") {
                None => 1,
                Some(raw) => raw.trim().parse::<u32>().ok().filter(|&v| v > 0)?,
            };
            if !rp.exhausted() {
                return None;
            }
            return Some(vec![format!(
                "Effect::RedirectNextDamage {{ target: {target_spec}, \
                 amount: Amount::Fixed({n}), to: PlayerRel::You }}"
            )]);
        }
        None
    }

    /// Turn-long / until-your-next-turn `Effect | Triggers$` gaps:
    /// - `Mode$ Taps` destroying `TriggeredCardLKICopy` (*Don't Move*)
    /// - `Mode$ Blocks` with omitted `ValidCard$` (*Mage Hunters' Onslaught*)
    /// - `Mode$ DamageDoneOnce | DamageDealtOnce` on targeted creature -> gain
    ///   that much life (*Spiritualize*, *Paladin of Prahv*)
    /// - `TriggerZones$ Battlefield` on `Effect | Triggers$` (*Gnawing Crescendo*,
    ///   *Infested Thrinax*, *Harald Unites the Elves*, *Roads Go Ever, Ever On*,
    ///   *Mistway Spy*).
    #[allow(clippy::too_many_lines)]
    fn wave28c_effect_delayed_triggers(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.has("StaticAbilities")
            || p.has("ReplacementEffects")
            || p.has("Abilities")
            || p.has("Boon")
            || p.has("EffectOwner")
            || p.has("ImprintCards")
            || p.peek("Name")
                .is_some_and(|n| n.trim_start().starts_with("Emblem"))
        {
            return None;
        }
        let delayed_dur = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "DelayedDuration::ThisTurn",
            Some("UntilYourNextTurn") => "DelayedDuration::UntilYourNextTurn",
            Some("UntilEndOfCombat") => "DelayedDuration::ThisCombat",
            Some(_) => return None,
        };
        if !matches!(
            p.take("ExileOnMoved")
                .or_else(|| p.take("ForgetOnMoved"))
                .as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        p.take("IsCurse");
        let rem = p.take("RememberObjects");
        let trig_name = p.take("Triggers")?;
        if trig_name.contains(',') || !p.exhausted() {
            return None;
        }
        let tbody = self.svars.get(trig_name.trim())?.clone();
        let (mode, mut tp) = Params::parse(&tbody)?;
        tp.drop_prose();
        if !matches!(
            tp.take("TriggerZones").as_deref(),
            None | Some("Command" | "Battlefield")
        ) {
            return None;
        }
        if tp.has("OneOff") || tp.has("TriggerOnce") || tp.has("Static") {
            return None;
        }
        let optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        // Case A: Spiritualize / Paladin of Prahv (`DamageDoneOnce | DamageDealtOnce`
        // on targeted creature -> gain life equal to `TriggerCount$DamageAmount`).
        if matches!(mode.as_str(), "DamageDoneOnce" | "DamageDealtOnce")
            && rem.as_deref() == Some("Targeted")
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && matches!(
                tp.take("ValidSource").as_deref(),
                Some("Creature.IsRemembered" | "Card.IsRemembered")
            )
            && !optional
        {
            let exec = tp.take("Execute")?;
            if !tp.exhausted() {
                return None;
            }
            let ebody = self.svars.get(exec.trim())?.clone();
            let (eapi, mut ep) = Params::parse(&ebody)?;
            ep.drop_prose();
            if eapi != "GainLife" || ep.take("Defined").as_deref() != Some("You") {
                return None;
            }
            let amt_var = ep.take("LifeAmount")?;
            if !ep.exhausted()
                || self.svars.get(amt_var.trim()).map(String::as_str)
                    != Some("TriggerCount$DamageAmount")
            {
                return None;
            }
            return Some(vec![format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &Trigger::DealsDamage(&Filter::This), \
                 watch: TriggerRemember::FirstTarget, duration: {delayed_dur} }}, \
                 condition: None, targets: None, remember: TriggerRemember::None, \
                 effects: &[Effect::GainLife {{ amount: Amount::EventAmount }}] }}"
            )]);
        }
        if rem.is_some() {
            return None;
        }
        // Case B: Don't Move (`Mode$ Taps | ValidCard$ Creature` -> destroy event object).
        if mode == "Taps" && !optional {
            let valid = tp.take("ValidCard")?;
            let exec = tp.take("Execute")?;
            if !tp.exhausted() || valid.contains("Remembered") || valid.contains("EffectSource") {
                return None;
            }
            let ebody = self.svars.get(exec.trim())?.clone();
            let (eapi, mut ep) = Params::parse(&ebody)?;
            ep.drop_prose();
            if eapi != "Destroy"
                || !matches!(
                    ep.take("Defined").as_deref(),
                    Some("TriggeredCardLKICopy" | "TriggeredCard")
                )
                || !ep.exhausted()
            {
                return None;
            }
            let expr = self.filter_expr(&valid)?;
            let filter_static = self.body.filter_static("TRIGGER", &expr);
            return Some(vec![format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &Trigger::BecomesTapped(&{filter_static}), \
                 watch: TriggerRemember::None, duration: {delayed_dur} }}, \
                 condition: None, targets: None, remember: TriggerRemember::None, \
                 effects: &[Effect::destroy(TargetSpec::EventObject)] }}"
            )]);
        }
        // Case C: General unbound `Triggers$` (`TriggerZones$ Battlefield` or bare `Mode$ Blocks`).
        if !matches!(
            mode.as_str(),
            "ChangesZone" | "Attacks" | "AttackersDeclared" | "Blocks" | "DamageDone"
        ) {
            return None;
        }
        if mode == "Blocks" && !tp.has("ValidCard") {
            tp.entries
                .push(("ValidCard".to_string(), "Creature".to_string()));
        }
        if tp.entries.iter().any(|(_, v)| {
            v.contains("IsRemembered")
                || v.contains("EffectSource")
                || v.contains("RememberedPlayer")
                || v.contains("ChosenCard")
        }) {
            return None;
        }
        let trigger_expr = self.trigger_expr(&mut tp, &mode)?;
        let exec = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let ebody = self.svars.get(exec.trim())?.clone();
        if ebody.contains("Remembered") || ebody.contains("EffectSource") {
            return None;
        }
        let mut inner = Chain::default();
        let saved_delayed = std::mem::replace(&mut self.in_delayed, true);
        let saved_mode = self.trigger_mode.replace(mode);
        let saved_self = std::mem::replace(&mut self.trigger_is_self, false);
        let saved_block = self.block_line.take();
        let read = self.chain(&ebody, &mut inner);
        self.in_delayed = saved_delayed;
        self.trigger_mode = saved_mode;
        self.trigger_is_self = saved_self;
        self.block_line = saved_block;
        read?;
        if inner.second_target.is_some() || inner.effects.is_empty() {
            return None;
        }
        let joined = inner.effects.join(", ");
        let effects = if optional {
            format!("Effect::MayDo {{ effects: &[{joined}] }}")
        } else {
            joined
        };
        let targets_expr = Self::target_req_expr(&inner)
            .map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        Some(vec![format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
             trigger: &{trigger_expr}, watch: TriggerRemember::None, duration: {delayed_dur} }}, \
             condition: None, targets: {targets_expr}, remember: TriggerRemember::None, \
             effects: &[{effects}] }}"
        )])
    }
}
