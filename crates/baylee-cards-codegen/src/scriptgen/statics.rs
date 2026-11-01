//! Reading static abilities and the modifiers they apply.

use super::{
    Chain, Params, Tx, animation::supertype_const, card_type_const, counter_kind, keyword_const,
};

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
            "RaiseCost" => return self.mana_life_tax(p),
            "ReduceCost" => return self.reduce_cost(p),
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
                if p.peek("ValidCard") == Some("Creature")
                    && p.peek("Caster") == Some("You")
                    && p.entries.len() == 2
                {
                    p.take("ValidCard");
                    p.take("Caster");
                    self.body.abilities.push(Self::static_expr(
                        "Filter::Any",
                        "Modifier::PlayersCantCast { who: PlayerRel::You, filter: &Filter::CREATURE }",
                    ));
                    return Some(());
                }
                return self.one_spell_each_turn(p);
            }
            "CantBlockBy" => return self.cant_block_by(p),
            "MustAttack" => return self.must_attack(p),
            "CantAttack,CantBlock,CantBeActivated" => {
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
        p.drop_prose();
        // `EffectZone$ Battlefield` is the default written out; any other
        // zone means the source works from somewhere else, which is a
        // different rule than the one below.
        let mut graveyard_zone = false;
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            if zone == "Graveyard"
                && p.peek("Affected") == Some("Creature.YouCtrl")
                && p.has("AddKeyword")
            {
                graveyard_zone = true;
            } else {
                self.note(format!("static ability from `EffectZone$ {zone}`"));
                return None;
            }
        }
        if p.peek("AffectedZone") == Some("Hand")
            && p.peek("MayLookAt") == Some("Player")
            && p.peek("Affected").is_none()
        {
            return self.reveal_all_hands(p);
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
        let filter = self.filter_expr(&affected)?;
        // "Gets +1/+1 as long as you control a Swamp" (Sedge Troll): the
        // clause the `A:` line reads as a restriction is, on a static, the
        // condition under which the ability exists at all, and the engine
        // registers and removes it as the condition changes.
        let mut condition = self.condition(&mut p)?;
        if condition.is_empty() && p.peek("CheckSVar").is_some() {
            condition = self.static_svar_condition(&mut p)?;
        }
        let mut out = Vec::new();
        let mut self_granted = false;
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.grant_modifiers(&mut p, &filter, &condition, &mut out, &mut self_granted)?;
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
        if graveyard_zone || !condition.is_empty() {
            let zone_arg = if graveyard_zone {
                ", zone = ZoneRef::Graveyard"
            } else {
                ""
            };
            for ability in &mut out {
                if let Some(open) = ability.strip_suffix(')') {
                    *ability = format!("{open}{zone_arg}{condition})");
                }
            }
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

    fn static_svar_condition(&mut self, p: &mut Params) -> Option<String> {
        let var = p.take("CheckSVar")?;
        let cmp = p.take("SVarCompare").unwrap_or_else(|| "GE1".to_string());
        if cmp != "EQ0" {
            return self.deny("unclaimed parameter `Continuous.CheckSVar`".to_string());
        }
        let Some(raw) = self
            .svars
            .get(var.trim())
            .and_then(|s| s.trim().strip_prefix("Count$Valid "))
        else {
            return self.deny("unclaimed parameter `Continuous.CheckSVar`".to_string());
        };
        let valid = match raw.split_once("/Plus.") {
            None => raw.to_string(),
            Some((first, second_var)) => {
                let Some(second) = self
                    .svars
                    .get(second_var.trim())
                    .and_then(|s| s.trim().strip_prefix("Count$Valid "))
                else {
                    return self.deny("unclaimed parameter `Continuous.CheckSVar`".to_string());
                };
                format!("{first},{second}")
            }
        };
        if valid.contains('/') {
            return self.deny("unclaimed parameter `Continuous.CheckSVar`".to_string());
        }
        let relative = valid.split(',').any(|alt| {
            alt.split(['.', '+']).any(|a| {
                matches!(
                    a.trim(),
                    "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
                )
            })
        });
        let yours = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
        let opps = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"));
        let nobodys = !valid
            .split(',')
            .flat_map(|alt| alt.split(['.', '+']))
            .any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
        if relative || (!yours && !opps && !nobodys) {
            return self.deny("unclaimed parameter `Continuous.CheckSVar`".to_string());
        }
        let expr = self.filter_expr(&valid)?;
        let name = self.body.filter_static("CHECK", &expr);
        let variant = if yours {
            "ControlCountAtMost"
        } else {
            "BattlefieldCountAtMost"
        };
        Some(format!(
            ", condition = Some(Condition::{variant}(&{name}, 0))"
        ))
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
        let Some(valid) = p.take("ValidCard") else {
            return self
                .deny("a `CantAttack,CantBlock,CantBeActivated` with no `ValidCard$`".to_string());
        };
        let filter = self.filter_expr(&valid)?;
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{mode}.{key}`"));
            }
            return None;
        }
        for modifier in [
            "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))",
            "Modifier::CantActivate",
        ] {
            let mut expr = Self::static_expr(&filter, modifier);
            if !condition.is_empty()
                && let Some(open) = expr.strip_suffix(')')
            {
                expr = format!("{open}{condition})");
            }
            self.body.abilities.push(expr);
        }
        Some(())
    }

    fn cant_be_activated(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        p.take("ValidSA");
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `CantBeActivated` with no `ValidCard$`".to_string());
        };
        let filter = self.filter_expr(&valid)?;
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantBeActivated.{key}`"));
            }
            return None;
        }
        let mut expr = Self::static_expr(&filter, "Modifier::CantActivate");
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
        Some(())
    }

    fn cant_block(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
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
        let filter = self.filter_expr(&valid)?;
        let condition = self.condition(&mut p)?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantBlock.{key}`"));
            }
            return None;
        }
        let mut expr = Self::static_expr(&filter, "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)");
        if !condition.is_empty()
            && let Some(open) = expr.strip_suffix(')')
        {
            expr = format!("{open}{condition})");
        }
        self.body.abilities.push(expr);
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

    /// A characteristic-defining power and toughness (CR 604.3): both equal
    /// to one count of permanents, the ones you control
    /// (`PtCount::YouControl`) or all of them (`PtCount::OnBattlefield`).
    /// Layer 7a on the battlefield, and the card's own number everywhere
    /// else (CR 604.3), both of which `Modifier::CharacteristicPT` is. A
    /// count of somebody else's permanents, a toughness apart from the power
    /// or a clause beside it (Gaea's Liege's "as long as it isn't
    /// attacking") is another sentence, and refused.
    pub(super) fn characteristic_pt(&mut self, mut p: Params) -> Option<()> {
        let (Some(power), Some(toughness)) = (p.take("SetPower"), p.take("SetToughness")) else {
            return self.deny("a characteristic-defining ability with no P/T".to_string());
        };
        if power != toughness {
            return self.deny(format!(
                "a defined toughness `{toughness}` apart from power"
            ));
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let Some(count) = self.svars.get(&power).cloned() else {
            return self.deny(format!("`{power}` names no SVar"));
        };
        if let Some(counter) = count.trim().strip_prefix("Count$CardCounters.") {
            let Some(kind) = super::counter_kind(counter) else {
                return self.deny(format!("counter `{counter}`"));
            };
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                 count: PtCount::CountersOnThis({kind}), toughness_plus: 0 }})"
            ));
            return Some(());
        }
        if let Some(valid) = count.strip_prefix("Count$ValidGraveyard ") {
            if valid.contains("Ctrl") || valid.contains("Own") || valid.contains('/') {
                return self.deny(format!("count `{count}`"));
            }
            let filter = self.filter_expr(valid)?;
            let name = self.body.filter_static("COUNTED", &filter);
            self.body.abilities.push(format!(
                "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
                 count: PtCount::InAllGraveyards(&{name}), toughness_plus: 0 }})"
            ));
            return Some(());
        }
        let Some(valid) = count.strip_prefix("Count$Valid ") else {
            return self.deny(format!("count `{count}`"));
        };
        let yours = valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
        if !yours && (valid.contains("Ctrl") || valid.contains("Own")) {
            return self.deny(format!("count `{count}`"));
        }
        let filter = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNTED", &filter);
        let count = if yours { "YouControl" } else { "OnBattlefield" };
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
             count: PtCount::{count}(&{name}), toughness_plus: 0 }})"
        ));
        Some(())
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
            && p.peek("IsPresent") == Some("Artifact.DefenderCtrl")
        {
            p.take("IsPresent");
            self.body.abilities.push(
                "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNBLOCKABLE), condition = Some(Condition::Not(&Condition::SomeDefendingPlayerControlsNone(&Filter::ARTIFACT))))"
                    .to_string(),
            );
            return Some(());
        }
        let condition = self.condition(&mut p)?;
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
        self.body.abilities.push(format!(
            "static_ability!({attackers}, {modifier}{condition})"
        ));
        Some(())
    }

    /// "Attacks each combat if able" (CR 508.1d): `S:Mode$ MustAttack`, a
    /// static naming the creatures it binds. `MustAttack$` names *what*
    /// they attack if able, a requirement about the pair the engine does
    /// not know, and stays unclaimed.
    pub(super) fn must_attack(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let Some(valid) = p.take("ValidCreature") else {
            return self.deny("`MustAttack` naming no creature".to_string());
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `MustAttack.{key}`"));
            return None;
        }
        let filter = self.filter_expr(&valid)?;
        self.body.abilities.push(format!(
            "static_ability!({filter}, Modifier::AttacksEachCombat{condition})"
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
    /// A plain "can't attack" and every other `UnlessDefender$` question
    /// (fewer creatures, poisoned, the monarch) stay unclaimed, and so does
    /// a negated one (`!controls…`): "unless defending player controls no
    /// untapped lands" is a different sentence.
    pub(super) fn cant_attack_unless(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`CantAttack.Secondary$ {other}`")),
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("`CantAttack` naming no creature".to_string());
        };
        if p.peek("Target") == Some("You") && !p.has("UnlessDefender") {
            p.take("Target");
            let condition = self.condition(&mut p)?;
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantAttack.{key}`"));
                return None;
            }
            let attackers = self.filter_expr(&valid)?;
            self.body.abilities.push(format!(
                "static_ability!({attackers}, Modifier::CantAttackYou{condition})"
            ));
            return Some(());
        }
        if !p.has("UnlessDefender") && valid != "Card.Self" {
            let condition = self.condition(&mut p)?;
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `CantAttack.{key}`"));
                return None;
            }
            let attackers = self.filter_expr(&valid)?;
            self.body.abilities.push(format!(
                "static_ability!({attackers}, Modifier::AddKeyword(KeywordSet::CANT_ATTACK){condition})"
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
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `CantAttack.{key}`"));
            return None;
        }
        let attackers = self.filter_expr(&valid)?;
        let wanted = self.filter_expr(wanted)?;
        let name = self.body.filter_static("DEFENDER_CONTROLS", &wanted);
        self.body.abilities.push(format!(
            "static_ability!({attackers}, Modifier::{modifier}(&{name}){condition})"
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
            if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && var_p.trim().parse::<i16>().is_err()
                && let Some(count) = self.svars.get(var_p.trim()).cloned()
                && let Some(counter) = count.trim().strip_prefix("Count$CardCounters.")
            {
                let Some(kind) = super::counter_kind(counter) else {
                    self.note(format!("counter `{counter}`"));
                    return None;
                };
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::ModifyPTByCount(PtCount::CountersOnThis({kind}))"),
                ));
            } else if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && var_p.trim().parse::<i16>().is_err()
                && let Some(count) = self.svars.get(var_p.trim()).cloned()
                && let Some(valid) = count.strip_prefix("Count$ValidGraveyard ")
            {
                if valid.contains("Ctrl") || valid.contains("Own") || valid.contains('/') {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
                let Some(counted_filter) = self.filter_expr(valid) else {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                };
                let name = self.body.filter_static("COUNTED", &counted_filter);
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::ModifyPTByCount(PtCount::InAllGraveyards(&{name}))"),
                ));
            } else if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && var_p.trim().parse::<i16>().is_err()
                && let Some(count) = self.svars.get(var_p.trim()).cloned()
                && count.trim() == "Count$ValidHand Card.YouOwn"
            {
                out.push(Self::static_expr(
                    filter,
                    "Modifier::ModifyPTByAmount { power: Amount::HandSize(PlayerRel::You), \
                     toughness: Amount::HandSize(PlayerRel::You) }",
                ));
            } else if let (Some(var_p), Some(var_t)) = (add_p.as_deref(), add_t.as_deref())
                && var_p == var_t
                && var_p.trim().parse::<i16>().is_err()
                && let Some(count) = self.svars.get(var_p.trim()).cloned()
                && let Some(raw_valid) = count.strip_prefix("Count$Valid ")
            {
                let (valid, times) = match raw_valid.split_once("/Times.") {
                    None => (raw_valid, 1_u8),
                    Some((v, n)) => {
                        let Ok(n) = n.trim().parse::<u8>() else {
                            self.note("static ability with a computed P/T".to_string());
                            return None;
                        };
                        if !(1..=4).contains(&n) {
                            self.note("static ability with a computed P/T".to_string());
                            return None;
                        }
                        (v, n)
                    }
                };
                if valid.contains('/') {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
                let yours = valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
                let opp = valid.split(['.', '+']).any(|atom| atom == "OppCtrl");
                if !yours && !opp && (valid.contains("Ctrl") || valid.contains("Own")) {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
                let Some(counted_filter) = self.filter_expr(valid) else {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                };
                let name = self.body.filter_static("COUNTED", &counted_filter);
                let kind = if yours { "YouControl" } else { "OnBattlefield" };
                for _ in 0..times {
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ModifyPTByCount(PtCount::{kind}(&{name}))"),
                    ));
                }
            } else if let Some((var, power_half)) = match (add_p.as_deref(), add_t.as_deref()) {
                (Some(vp), None | Some("0" | "+0")) if vp.trim().parse::<i16>().is_err() => {
                    Some((vp.trim(), true))
                }
                (None | Some("0" | "+0"), Some(vt)) if vt.trim().parse::<i16>().is_err() => {
                    Some((vt.trim(), false))
                }
                _ => None,
            } && let Some(count) = self.svars.get(var).cloned()
                && let Some(raw_valid) = count.strip_prefix("Count$Valid ")
            {
                let (valid, times) = match raw_valid.split_once("/Times.") {
                    None => (raw_valid, 1_i16),
                    Some((v, n)) => {
                        let Ok(n) = n.trim().parse::<i16>() else {
                            self.note("static ability with a computed P/T".to_string());
                            return None;
                        };
                        if !(1..=4).contains(&n) {
                            self.note("static ability with a computed P/T".to_string());
                            return None;
                        }
                        (v, n)
                    }
                };
                if valid.contains('/') {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
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
                if (!yours && !attached)
                    || (attached && (valid.contains("Ctrl") || valid.contains("Own")))
                {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                }
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
                let Some(counted_filter) = self.filter_expr(&valid_without_you) else {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                };
                let counted_filter = if yours {
                    format!("Filter::And(&[{counted_filter}, Filter::ControlledByYou])")
                } else {
                    counted_filter
                };
                let name = self.body.filter_static("COUNTED", &counted_filter);
                let (p, t) = if power_half { (times, 0) } else { (0, times) };
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::ModifyPTPerCount {{ filter: &{name}, p: {p}, t: {t} }}"),
                ));
            } else {
                // An anthem that names only one half still moves the other by
                // zero, which is what the printed "+1/+0" says.
                let (Some(power), Some(tough)) = (
                    add_p.map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
                    add_t.map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
                ) else {
                    self.note("static ability with a computed P/T".to_string());
                    return None;
                };
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::ModifyPT({power}, {tough})"),
                ));
            }
        }
        let set_p = p.take("SetPower");
        let set_t = p.take("SetToughness");
        if set_p.is_some() || set_t.is_some() {
            // Setting one half and leaving the other alone is a real card
            // ("base power 4"), and `SetPT` cannot say it — refuse rather
            // than invent a value for the half that was not named.
            let (Some(power), Some(tough)) = (
                set_p.and_then(|v| v.trim().parse::<i16>().ok()),
                set_t.and_then(|v| v.trim().parse::<i16>().ok()),
            ) else {
                self.note("static ability setting one half of P/T".to_string());
                return None;
            };
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::SetPT({power}, {tough})"),
            ));
        }
        Some(())
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
                if filter != "Filter::This"
                    || !self
                        .body
                        .enter_modifiers
                        .iter()
                        .any(|m| m == "EnterModifier::ChooseBasicLandType")
                {
                    return None;
                }
                out.push(Self::static_expr(
                    filter,
                    "Modifier::AddChosenBasicLandType",
                ));
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
    /// `AddAbility$ <SVar>[ & <SVar>]`: "Other Zombies have '{B}:
    /// Regenerate this permanent.'" (Zombie Master). Each named `SVar` is an
    /// `AB$` line, read as an activated ability of the object that gains it:
    /// `Modifier::GrantActivated` activates from that object, so "this
    /// permanent" in it is the one that has it.
    ///
    /// Without a target, because the modifier carries none — a granted
    /// ability that targets would lose its target here and act on nothing
    /// — and without anything else an activated line may say beside its
    /// cost and effect: the chain refuses a key it does not claim.
    pub(super) fn grant_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        condition: &str,
        out: &mut Vec<String>,
        self_granted: &mut bool,
    ) -> Option<()> {
        let Some(raw) = p.take("AddAbility") else {
            return Some(());
        };
        for name in raw.split(" & ").map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("`AddAbility$ {name}` naming no `SVar`"));
            };
            if !line.trim_start().starts_with("AB$") {
                return self.deny("a granted ability that is not an activated one".to_string());
            }
            let Some((_, mut probe)) = Params::parse(&line) else {
                return self.deny("a granted ability with no `$` in it".to_string());
            };
            let Some(cost) = probe.take("Cost") else {
                return self.deny("a granted ability with no `Cost$`".to_string());
            };
            let cost = self.cost_expr(&cost)?;
            let stripped: Vec<&str> = line
                .split(" | ")
                .filter(|part| !part.starts_with("Cost$"))
                .collect();
            let mut chain = Chain::default();
            self.chain(&stripped.join(" | "), &mut chain)?;
            if let Some(target) = &chain.target {
                if filter == "Filter::This"
                    && chain.second_target.is_none()
                    && !chain.target_is_x
                    && chain.target_up_to.is_none()
                    && !chain.effects.is_empty()
                    && !chain.could_add_mana
                {
                    self.body.abilities.push(format!(
                        "activated!({cost}, &[{}], target = Some({target}){condition})",
                        chain.effects.join(", ")
                    ));
                    *self_granted = true;
                    continue;
                }
                return self.deny("a granted ability with a target".to_string());
            }
            if chain.effects.is_empty() {
                return self.deny("a granted ability that reads as no effect at all".to_string());
            }
            let mana = chain.could_add_mana && !chain.moves_library;
            out.push(Self::static_expr(
                filter,
                &format!(
                    "Modifier::GrantActivated {{ cost: {cost}, effects: &[{}], mana_ability: {mana} }}",
                    chain.effects.join(", ")
                ),
            ));
        }
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

    /// One `AbilityDef::Static` expression.
    ///
    /// `static_ability!` takes no layer for the same reason [`Self::animate_expr`]
    /// passes none: CR 613.1 makes the layer a function of the modifier, and
    /// `Modifier::layer` is that function.
    pub(super) fn static_expr(filter: &str, modifier: &str) -> String {
        format!("static_ability!({filter}, {modifier})")
    }
}
