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
    Params, Tx,
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

    /// Attack-trigger `S: Mode$ Panharmonicon` (*Isshin, Two Heavens as One*) ->
    /// `TriggerEventKind::Attack`.
    ///
    /// Requires all three Forge attack trigger modes (`Attacks`,
    /// `AttackersDeclared`, `AttackersDeclaredOneTarget`) AND an unscoped
    /// `ValidCause$ Creature`. Narrower causes like `Creature.YouCtrl`
    /// (*Wulfgar*) stay refused because `ReplacementRule::TriggerMultiplier`
    /// does not filter the attacking creature's controller.
    pub(super) fn wave28c_panharmonicon_attack(
        p: &mut Params,
        valid_mode: &str,
    ) -> Option<&'static str> {
        let mut saw_attacks = false;
        let mut saw_declared = false;
        let mut saw_one_target = false;
        let mut count = 0usize;
        for part in valid_mode.split(',').map(str::trim) {
            count += 1;
            match part {
                "Attacks" => saw_attacks = true,
                "AttackersDeclared" => saw_declared = true,
                "AttackersDeclaredOneTarget" => saw_one_target = true,
                _ => return None,
            }
        }
        if !(count == 3 && saw_attacks && saw_declared && saw_one_target) {
            return None;
        }
        if p.has("Origin") || p.has("Destination") {
            return None;
        }
        if p.take("ValidCause").as_deref() != Some("Creature") {
            return None;
        }
        Some("TriggerEventKind::Attack")
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
                || valid.contains("OppOwn")
                || valid.contains("OppCtrl")
                || valid.contains("YouCtrl")
            {
                return None;
            }
            let expr = self.filter_expr_in_zone(valid, true)?;
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
