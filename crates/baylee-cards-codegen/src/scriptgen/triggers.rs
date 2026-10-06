//! Reading triggered abilities, delayed ones, and what happens as a
//! permanent enters.

use super::{BlockHalf, BlockRole, Chain, Params, Price, Tx, card_type_const, color_word};

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
    pub(super) fn spell_cast_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        // **An absent `TriggerZones$` is not the battlefield here.** 114 of
        // the corpus's 1444 `SpellCast` lines write no zone at all, and 98
        // of them are `ValidCard$ Card.Self` — "when you cast this spell",
        // which fires while the card is on the *stack*. This engine collects
        // triggers off the battlefield, so reading one of those as an
        // ordinary trigger is a card whose ability can never fire under a
        // `Coverage::Implemented` that says otherwise. The zone is therefore
        // demanded rather than defaulted, which is the opposite of what the
        // other modes may do.
        if !zoned {
            return self.deny("a `SpellCast` trigger with no `TriggerZones$`".to_string());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `SpellCast` trigger with no `ValidCard$`".to_string());
        };
        // The same sentence, refused a second way: a cast trigger
        // is one whatever zone it claims.
        if valid.split(['.', '+']).any(|atom| atom == "Self") {
            return self.deny("a `SpellCast` trigger on the card itself".to_string());
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
        let valid = if whose.is_empty() {
            valid
        } else {
            valid
                .split(',')
                .map(|alt| format!("{alt}{whose}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::SpellCast(&{filter})"))
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
            if source.is_some() || combat {
                return self.deny("a `DamageDoneOnce` trigger naming its source".to_string());
            }
            if target
                .split(['.', ','])
                .any(|w| matches!(w, "You" | "Player" | "Opponent"))
            {
                return self.deny(format!("damage dealt to `{target}` at once"));
            }
            let filter = filter_of(self, &target)?;
            return Some(format!("Trigger::DealtDamage({filter})"));
        }
        let Some(source) = source else {
            return self.deny("a `DamageDone` trigger with no `ValidSource$`".to_string());
        };
        let filter = filter_of(self, &source)?;
        match (target.as_deref(), combat) {
            (Some("Player"), true) => Some(format!("Trigger::DealsCombatDamageToPlayer({filter})")),
            (Some("Opponent" | "Player.Opponent"), true) => {
                Some(format!("Trigger::DealsCombatDamageToOpponent({filter})"))
            }
            (Some("Opponent" | "Player.Opponent"), false) => {
                Some(format!("Trigger::DealsDamageToOpponent({filter})"))
            }
            (other, _) => self.deny(format!(
                "damage dealt to `{}`{}",
                other.unwrap_or("anything"),
                if combat { " in combat" } else { "" }
            )),
        }
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
        // Where the ability triggers **from**. This used to be taken and
        // thrown away, which is the one thing this reader is not allowed to
        // do: `AbilityDef::Triggered` carries no zone, the engine collects
        // triggers off the battlefield (plus CR 603.10's look-back and the
        // command zone's emblems), so a `TriggerZones$ Graveyard` read as an
        // ordinary trigger is a card whose ability can never fire and a
        // `Coverage::Implemented` that says otherwise.
        //
        // Five corpus scripts were being read this way, and all five are
        // Vanguard avatars whose trigger fires from the command zone:
        // Fallen Angel, Gerrard, Rofellos, Royal Assassin and Rumbling Slum.
        // None of them is a card this pool compiles — `Vanguard` is not a
        // card type here — so nothing in the tree changes, which is what
        // makes this the cheapest possible moment to shut the door.
        let zones = p.take("TriggerZones");
        match zones.as_deref() {
            None | Some("Battlefield") => {}
            Some(zones) => return self.deny(format!("`TriggerZones$ {zones}`")),
        }
        match mode {
            "AttackerBlockedByCreature" => self.block_trigger(p),
            "ChangesZone" => {
                let origin = p.take("Origin");
                let dest = p.take("Destination");
                let valid = p.take("ValidCard");
                let (Some(origin), Some(dest), Some(valid)) = (origin, dest, valid) else {
                    return self
                        .deny("a `ChangesZone` trigger missing one of its three keys".to_string());
                };
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                match (origin.as_str(), dest.as_str()) {
                    // `Trigger::ETB` is the same bytes as the variant with
                    // `&Filter::This` in it, and is what the DSL carries the
                    // constant for: ninety-nine of the pool's hundred and ten
                    // enter-triggers point at the source, so the long form is
                    // the rare one and deserves to look rare.
                    ("Any", "Battlefield") if filter == "&Filter::This" => {
                        Some("Trigger::ETB".to_string())
                    }
                    ("Any", "Battlefield") => Some(format!("Trigger::EntersBattlefield({filter})")),
                    ("Battlefield", "Graveyard") => Some(format!("Trigger::Dies({filter})")),
                    _ => self.deny(format!("trigger on a move from {origin} to {dest}")),
                }
            }
            "Phase" => {
                let Some(phase) = p.take("Phase") else {
                    return self.deny("a `Phase` trigger with no `Phase$`".to_string());
                };
                let step = match phase.as_str() {
                    "Upkeep" => "StepKind::Upkeep",
                    "Draw" => "StepKind::Draw",
                    "BeginCombat" => "StepKind::CombatBegin",
                    "End of Turn" => "StepKind::End",
                    other => return self.deny(format!("trigger at step `{other}`")),
                };
                // No player named is every player's step: "at the beginning
                // of each upkeep" (Verdant Force), "at the beginning of the
                // end step" (Pestilence). The reference writes `ValidPlayer$
                // You` for "your", and reading its absence as "your" too made
                // Verdant Force a card that made a Saproling on one upkeep in
                // two.
                let valid = p.take("ValidPlayer");
                let whose = match valid.as_deref() {
                    None => Some("PlayerRel::EachPlayer"),
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
            "Attacks" => {
                let Some(valid) = p.take("ValidCard") else {
                    return self.deny("an `Attacks` trigger with no `ValidCard$`".to_string());
                };
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                Some(format!("Trigger::Attacks({filter})"))
            }
            "SpellCast" => self.spell_cast_trigger(p, zones.is_some()),
            "TapsForMana" => self.taps_for_mana_trigger(p),
            "DamageDone" => self.damage_trigger(p, false),
            "DamageDoneOnce" => self.damage_trigger(p, true),
            // "Whenever [a permanent] becomes tapped": the source (City of
            // Brass), or any permanent the filter matches (Lifetap).
            "Taps" => {
                let Some(valid) = p.take("ValidCard") else {
                    return self.deny("a `Taps` trigger with no `ValidCard$`".to_string());
                };
                if valid == "Card.Self" {
                    return Some("Trigger::BecomesTapped(&Filter::This)".to_string());
                }
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                Some(format!("Trigger::BecomesTapped(&{filter})"))
            }
            other => self.deny(format!("trigger mode `{other}`")),
        }
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
        let mut fields = rest.split(':');
        let kind = fields.next()?.trim();
        if kind == "Copy" {
            let svar = fields.next().map(str::trim).unwrap_or_default().to_string();
            let optional = fields.next().map(str::trim) == Some("Optional");
            return self.copy_on_enter(&svar, optional);
        }
        if kind != "Other" {
            return self.deny(format!("`ETBReplacement:{kind}`"));
        }
        let Some(svar) = fields.next().map(str::trim) else {
            return self.deny("an `ETBReplacement` naming no ability".to_string());
        };
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

    /// `DB$ ChooseType | Type$ Basic Land` behind an `ETBReplacement:Other`:
    /// "as this enters, choose a basic land type" (Phantasmal Terrain),
    /// `EnterModifier::ChooseBasicLandType`. The choice is its controller's,
    /// so a `Defined$` other than `You` is refused, and so is every other
    /// `Type$`: a creature type is a different question with other readers.
    pub(super) fn choose_type_on_enter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(other) => return self.deny(format!("a type chosen by `{other}`")),
        }
        match p.take("Type").as_deref() {
            Some("Basic Land") => {}
            other => {
                return self.deny(format!(
                    "as-enters choice of a `{}` type",
                    other.unwrap_or("nameless")
                ));
            }
        }
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `ChooseType.{key}`"));
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::ChooseBasicLandType".to_string());
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

    pub(super) fn triggered(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("a `T:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        self.trigger_mode = Some(mode.clone());
        let (trigger, condition) = if mode == "Always" {
            (self.state_trigger(&mut p)?, String::new())
        } else {
            let trigger = self.trigger_expr(&mut p, &mode)?;
            (trigger, self.condition(&mut p)?)
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
        let may = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => return self.deny(format!("a `may` decided by `{other}`")),
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
        let (body, price) = match Self::optional_price(&body, may) {
            Some((stripped, price)) => (stripped, Some(price)),
            None => (body, None),
        };
        let mut chain = Chain::default();
        self.chain(&body, &mut chain)?;
        if chain.effects.is_empty() {
            return self.deny("a trigger that reads as no effect at all".to_string());
        }
        let targets = chain
            .target
            .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
            .unwrap_or_default();
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
        let effects = chain.effects.join(", ");
        let effects = match price {
            Some(Price::Generic(mana)) => format!(
                "Effect::PlayerMayPayThen {{ player: PlayerRel::You, \
                 mana: {mana}, effects: &[{effects}] }}"
            ),
            // Farmstead's "you may pay {W}{W}. If you do, you gain 1 life".
            Some(Price::Printed(cost)) => format!(
                "Effect::PlayerMayPayManaThen {{ player: PlayerRel::You, \
                 cost: mana!(\"{cost}\"), effects: &[{effects}] }}"
            ),
            Some(Price::Part(_)) => unreachable!("`optional_price` reads mana only"),
            None if may => format!("Effect::MayDo {{ effects: &[{effects}] }}"),
            None => effects,
        };
        let ability = format!("triggered!({trigger}, &[{effects}]{targets}{condition})");
        if let Some((role, other, secondary)) = self.block_line.take() {
            return self.pair_block_half(role, other, secondary, ability);
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

    /// Refuses a script that ended with one half of a "blocks or becomes
    /// blocked by" trigger read and its mirror never met.
    pub(super) fn block_halves_paired(&self) -> Option<()> {
        match &self.block_half {
            None => Some(()),
            Some(half) => self.deny(format!(
                "a `{}` trigger without its mirror",
                match half.role {
                    BlockRole::Blocks => "blocks",
                    BlockRole::Blocked => "becomes blocked by",
                }
            )),
        }
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
    ) -> Option<String> {
        let mode = p.take("Mode").unwrap_or_default();
        let phase = p.take("Phase").unwrap_or_default();
        if mode != "Phase" || phase != "EndCombat" {
            return self.deny(format!("a delayed trigger at `{mode} {phase}`"));
        }
        match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => {}
            Some(other) => {
                return self.deny(format!("a delayed trigger in `{other}`'s turn"));
            }
        }
        let remembered = p.take("RememberObjects").unwrap_or_default();
        let other_side = match self.block_line.as_ref().map(|(role, ..)| *role) {
            Some(BlockRole::Blocks) => "TriggeredAttacker",
            Some(BlockRole::Blocked) => "TriggeredBlocker",
            None => {
                return self.deny(format!(
                    "a delayed trigger remembering `{remembered}` outside a block trigger"
                ));
            }
        };
        if remembered.strip_suffix("LKICopy").unwrap_or(&remembered) != other_side {
            return self.deny(format!("a delayed trigger remembering `{remembered}`"));
        }
        if target.is_some() {
            return self.deny("a delayed trigger on a line that targets".to_string());
        }
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
        Some(format!(
            "Effect::AtEndOfCombat {{ about: TargetSpec::EventObject, effects: &[{}] }}",
            inner.effects.join(", ")
        ))
    }
}
