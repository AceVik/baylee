//! Reading static abilities and the modifiers they apply.

use super::{Chain, Params, Tx, card_type_const, keyword_const};

impl Tx<'_> {
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
        } else {
            self.filter_expr(&valid)?
        };
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
    pub(super) fn static_ability(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("an `S:` line with no `$` in it".to_string());
        };
        if mode == "CantBlockBy" {
            return self.cant_block_by(p);
        }
        if mode == "MustAttack" {
            return self.must_attack(p);
        }
        if mode == "CantAttack" {
            return self.cant_attack_unless(p);
        }
        if let Some(modifier) = match mode.as_str() {
            "CanAttackDefender" => Some("AttacksDespiteDefender"),
            "CanAttackIfHaste" => Some("AttacksAsThoughHaste"),
            _ => None,
        } {
            return self.attack_as_though(modifier, p);
        }
        if mode != "Continuous" {
            self.note(format!("static ability `S: Mode$ {mode}`"));
            return None;
        }
        p.drop_prose();
        // `EffectZone$ Battlefield` is the default written out; any other
        // zone means the source works from somewhere else, which is a
        // different rule than the one below.
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            self.note(format!("static ability from `EffectZone$ {zone}`"));
            return None;
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
        let Some(affected) = p.take("Affected") else {
            return self.deny("a continuous static with no `Affected$`".to_string());
        };
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
        let condition = self.condition(&mut p)?;
        let mut out = Vec::new();
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.grant_modifiers(&mut p, &filter, &mut out)?;
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
        if !condition.is_empty() {
            for ability in &mut out {
                if let Some(open) = ability.strip_suffix(')') {
                    *ability = format!("{open}{condition})");
                }
            }
        }
        // The honest-stub rule: one key nothing claimed and the card stays
        // a stub, however much of the line was understood.
        if !p.exhausted() || out.is_empty() {
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
        let (Some(attacker), Some(blocker)) = (p.take("ValidAttacker"), p.take("ValidBlocker"))
        else {
            return self.deny("`CantBlockBy` naming no attacker or no blocker".to_string());
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `CantBlockBy.{key}`"));
            return None;
        }
        let attackers = self.filter_expr(&attacker)?;
        let blockers = self.filter_expr(&blocker)?;
        let name = self.body.filter_static("BLOCKER", &blockers);
        self.body.abilities.push(format!(
            "static_ability!({attackers}, Modifier::CantBeBlockedBy(&{name}){condition})"
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

    /// "Can't attack unless defending player controls an Island" (CR
    /// 508.1c): `S:Mode$ CantAttack | UnlessDefender$ controls<filter>`.
    /// A plain "can't attack" and every other `UnlessDefender$` question
    /// (fewer creatures, poisoned, the monarch) stay unclaimed, and so does
    /// a negated one (`!controls…`): "unless defending player controls no
    /// untapped lands" is a different sentence.
    pub(super) fn cant_attack_unless(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("`CantAttack` naming no creature".to_string());
        };
        let Some(unless) = p.take("UnlessDefender") else {
            return self.deny("`CantAttack` with no `UnlessDefender$`".to_string());
        };
        let Some(wanted) = unless.strip_prefix("controls") else {
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
            "static_ability!({attackers}, Modifier::CantAttackUnlessDefenderControls(&{name}){condition})"
        ));
        Some(())
    }

    /// `AddPower`/`AddToughness` (layer 7c) and `SetPower`/`SetToughness`
    /// (layer 7b) as static abilities.
    pub(super) fn pt_modifiers(
        &self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let add_p = p.take("AddPower");
        let add_t = p.take("AddToughness");
        if add_p.is_some() || add_t.is_some() {
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
        &self,
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
    /// keeps them apart — a `TypeSet` is a bitmask the rules read, a
    /// subtype is an interned id — so `AddType$ Artifact Goblin` becomes
    /// two modifiers on the same layer.
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
            let mut types = Vec::new();
            let mut subtypes = Vec::new();
            for word in raw.split_whitespace() {
                if let Some(t) = card_type_const(word) {
                    types.push(t);
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
        out: &mut Vec<String>,
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
            if chain.target.is_some() {
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
            for word in raw.split_whitespace() {
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
