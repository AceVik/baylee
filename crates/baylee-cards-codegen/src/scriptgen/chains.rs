//! Reading an effect and the chain of sub-abilities it hands on to, with
//! the "unless" a player may pay.

use super::{
    Chain, Params, Price, Tx, Unless, amount, asks_for_an_object, counter_kind, generic_mana,
    is_supported_api, plain_number,
};

impl Tx<'_> {
    /// One effect and everything its `SubAbility$` chain adds.
    pub(super) fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an ability spec with no `$` in it".to_string());
        };
        p.drop_prose();
        chain.could_add_mana |= api == "Mana";
        chain.moves_library |= matches!(api.as_str(), "Draw" | "Mill" | "Surveil")
            || (matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                && (p
                    .peek("Origin")
                    .is_some_and(|zones| zones.split(',').any(|zone| zone == "Library"))
                    != (p.peek("Destination") == Some("Library"))));
        let valid = p.take("ValidTgts");
        let targets_here = valid.is_some();
        if let Some(valid) = valid {
            // A card in a graveyard is a different kind of target from a
            // permanent (CR 115.1 names both, and they are chosen from
            // different zones), so the zone the line moves *from* decides
            // the spec before the valid-string is read.
            let spec = if api == "ChangeZone" && p.peek("Origin") == Some("Graveyard") {
                self.graveyard_target(&valid)
            } else {
                // "Target spell or permanent" (the Laces): the stack is a
                // zone a target may be chosen in only where the line says
                // so. The battlefield alone is what every target already
                // is.
                match p.take("TgtZone").as_deref() {
                    None | Some("Battlefield") => self.target_spec(&valid, &api),
                    Some("Stack,Battlefield" | "Battlefield,Stack") => {
                        self.spell_or_permanent_target(&valid)
                    }
                    Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
                }
            };
            let Some(spec) = spec else {
                return self.deny(format!("target `{valid}`"));
            };
            if chain.target.get_or_insert(spec.clone()) != &spec {
                return self.deny("two different targets in one chain".to_string());
            }
        }
        let sub = p.take("SubAbility");
        // The chosen-source shield is three lines in the reference and one
        // sentence on the card: its `SubAbility$` is part of the sentence,
        // not the next one, so this rule reads the rest of the chain itself.
        if api == "ChooseSource" {
            let effects = self.prevent_from_chosen_source(p, sub.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
        }
        // The requirement and the effect name the target differently when it
        // is a player: the wizard resolves `AnyPlayer`/`AnyOpponent` into the
        // spell's chosen player, and the effect then reads it back as
        // `PlayerRel::Chosen`. Handing the *requirement* to the effect
        // instead is how a burn spell ends up dealing damage to nothing at
        // all: `DealDamage` looks for an object target and finds none.
        let target: Option<String> = match chain.target.as_deref() {
            Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") => {
                Some("TargetSpec::Player(PlayerRel::Chosen)".to_string())
            }
            Some(other) => Some(other.to_string()),
            None => None,
        };
        // Whether an absent `Defined$` on this line means a chosen player:
        // only where this line declared the player target itself.
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        // "Sacrifice it unless you pay {U}", "counter target spell unless
        // its controller pays {2}": the price is the line's and not its
        // effect's, so it is read here for every API and wraps whatever the
        // line turns out to say.
        let unless = match p.take("UnlessCost") {
            Some(cost) => Some(self.unless(&cost, &mut p, target.as_deref())?),
            None => None,
        };
        // "If that creature would die this turn, exile it instead": a rider
        // on whatever the line does to its target, read for every API.
        let dying = match p.take("ReplaceDyingDefined") {
            Some(defined) => Some(self.exile_if_dies(&defined, target.as_deref())?),
            None => None,
        };
        let at_end = match p.take("AtEOT") {
            Some(what) => Some(self.at_next_end_step(&api, &what, target.as_deref())?),
            None => None,
        };

        let Some(mut effects) = self.effect_of(&api, &mut p, target.as_deref(), targets_a_player)
        else {
            // An API with no rule at all is a different report than a rule
            // that met a value it cannot say — the first is a missing
            // effect, the second is a missing case in one that exists.
            if is_supported_api(&api) {
                self.note(format!("unreadable value in `{api}`"));
            } else {
                self.note(format!("effect `{api}`"));
            }
            return None;
        };
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        effects.extend(dying);
        effects.extend(at_end);
        let effects = match unless {
            Some(unless) => vec![self.unless_wrap(unless, &effects)?],
            None => effects,
        };
        chain.effects.extend(effects);
        match sub {
            Some(name) => {
                let Some(body) = self.svars.get(&name).cloned() else {
                    return self.deny(format!("`SubAbility$ {name}` names no SVar"));
                };
                self.chain(&body, chain)
            }
            None => Some(()),
        }
    }

    /// `ReplaceDyingDefined$` — "if that creature would die this turn,
    /// exile it instead" (Magma Spray), a replacement on the line's own
    /// target for the rest of the turn.
    ///
    /// `Targeted` is the target whatever it is (Scorching Dragonfire's
    /// "that creature or planeswalker"); `ThisTargetedCard.Creature` is
    /// Disintegrate's "if it's a creature", asked of an any-target as the
    /// spell resolves. `Remembered` is "a creature dealt damage this way",
    /// which asks whether damage was dealt, and is refused, as is a
    /// condition on the rider (`ReplaceDyingCondition$`, left unclaimed) and
    /// a line whose target is a player.
    pub(super) fn exile_if_dies(&mut self, defined: &str, target: Option<&str>) -> Option<String> {
        let Some(target) = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        }) else {
            return self.deny(format!(
                "`ReplaceDyingDefined$ {defined}` with no object target"
            ));
        };
        let exile = format!("Effect::ExileIfDiesThisTurn {{ target: {target} }}");
        match defined {
            "Targeted" => Some(exile),
            "ThisTargetedCard.Creature" => Some(format!(
                "Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{exile}] }}"
            )),
            other => self.deny(format!("`ReplaceDyingDefined$ {other}`")),
        }
    }

    /// `AtEOT$` on a line that pumps its target — "destroy that creature at
    /// the beginning of the next end step" (Stone Giant): a delayed trigger
    /// about the target, `Effect::AtNextEndStep`.
    ///
    /// Only `Destroy` on a targeted `Pump`. A token's or a copy's "sacrifice
    /// it" is about the object the line made, not a target, and "exile it",
    /// "return it to your hand" and the upkeep spellings are other
    /// sentences; each is refused by name.
    pub(super) fn at_next_end_step(
        &mut self,
        api: &str,
        what: &str,
        target: Option<&str>,
    ) -> Option<String> {
        let aimed = target.is_some_and(|t| {
            !matches!(
                t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        });
        if api != "Pump" || !aimed || what != "Destroy" {
            return self.deny(format!("`AtEOT$ {what}` on `{api}`"));
        }
        Some(
            "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
                .to_string(),
        )
    }

    /// The "unless" of a line: `UnlessCost$` with the keys that qualify it.
    ///
    /// **The payer.** `UnlessPayer$ You` is the source's controller. Absent,
    /// the reference asks the controller of the line's target (its default
    /// is `TargetedController`), which is Mana Leak's "unless its controller
    /// pays" — so an absent payer is read only on a line that targets a
    /// spell or a permanent, and refused on one that targets nothing, where
    /// that default names nobody. Every other payer is refused by name:
    /// `Player` is every player at once, a price no one question can put.
    ///
    /// **The subs.** Without `UnlessResolveSubs$` the rest of the chain runs
    /// either way, which is what wrapping this line alone gives. With it the
    /// rest runs on one answer only (Power Sink's "if that player doesn't,
    /// they tap all lands…"), a shape this does not read yet.
    pub(super) fn unless(
        &mut self,
        cost: &str,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Unless> {
        if let Some(subs) = p.take("UnlessResolveSubs") {
            return self.deny(format!("`UnlessResolveSubs$ {subs}`"));
        }
        let switched = match p.take("UnlessSwitched").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`UnlessSwitched$ {other}`")),
        };
        let payer = p.take("UnlessPayer");
        let targets_a_controlled_object = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        let payer = match payer.as_deref() {
            Some("You") => "PlayerRel::You",
            // Paralyze's "that player may pay {4}": the enchanted
            // creature's controller (CR 303.4e names the Aura's host).
            Some("EnchantedController") => "PlayerRel::ControllerOfAttached",
            None | Some("TargetedController") if targets_a_controlled_object => {
                "PlayerRel::ControllerOfTarget"
            }
            other => {
                return self.deny(format!(
                    "an unless-cost paid by `{}`",
                    other.unwrap_or("TargetedController")
                ));
            }
        };
        let price = self.unless_price(cost.trim())?;
        Some(Unless {
            payer,
            price,
            switched,
        })
    }

    /// An `UnlessCost$` value as a [`Price`].
    ///
    /// Read by [`Tx::cost_pieces`], the reader an activation cost goes
    /// through, and then held to what the "unless" effects can carry. Plain
    /// generic mana stays an [`Amount`] (`PlayerMayPayOr`); anything with a
    /// colour in it is printed exactly (`PlayerMayPayManaOr`), where it used
    /// to be refused — a `{U}` charged as `{1}` is a card anyone could keep
    /// with a Mountain. One part the player pays by naming an object is
    /// `PlayerMayPayCostOr`. A part that needs no answer (`PayLife<2>`) is
    /// refused even though `CostPart` can hold it: that effect asks by
    /// putting up the list of what may pay, and an empty list is how a
    /// player declines — so a price nobody names an object for would
    /// decline itself every time.
    pub(super) fn unless_price(&mut self, raw: &str) -> Option<Price> {
        // "Unless its controller pays {X}" (Power Sink): the X announced
        // for the source, and only where the card says that is what X is.
        if raw == "X" {
            let Some(x) = amount(raw, self.svars, self.has_x) else {
                return self.deny("an unless-cost of `X`".to_string());
            };
            return Some(Price::Generic(x));
        }
        let (mana, parts) = self.cost_pieces(raw)?;
        match (mana.as_str(), parts.as_slice()) {
            (m, []) if !m.is_empty() => Some(match generic_mana(m) {
                Some(n) => Price::Generic(format!("Amount::Fixed({n})")),
                None => Price::Printed(m.to_string()),
            }),
            ("", [one]) if asks_for_an_object(one) => Some(Price::Part(one.clone())),
            _ => self.deny(format!("an unless-cost of `{raw}`")),
        }
    }

    /// The line's effects behind its price. A tax runs them on a refusal and
    /// takes them as one effect (a `Sequence` when there are several); a
    /// switched price runs them on a payment and takes the list.
    pub(super) fn unless_wrap(&mut self, unless: Unless, effects: &[String]) -> Option<String> {
        let Unless {
            payer,
            price,
            switched,
        } = unless;
        let one = match effects {
            [] => return self.deny("an unless-cost on a line with no effect".to_string()),
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        let list = effects.join(", ");
        Some(match (price, switched) {
            (Price::Generic(mana), false) => format!(
                "Effect::PlayerMayPayOr {{ player: {payer}, mana: {mana}, effect: &{one} }}"
            ),
            (Price::Generic(mana), true) => format!(
                "Effect::PlayerMayPayThen {{ player: {payer}, mana: {mana}, effects: &[{list}] }}"
            ),
            (Price::Printed(cost), false) => format!(
                "Effect::PlayerMayPayManaOr {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effect: &{one} }}"
            ),
            (Price::Printed(cost), true) => format!(
                "Effect::PlayerMayPayManaThen {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effects: &[{list}] }}"
            ),
            (Price::Part(part), false) => format!(
                "Effect::PlayerMayPayCostOr {{ player: {payer}, cost: &CostPart::{part}, \
                 effect: &{one} }}"
            ),
            // "You may <sacrifice a creature>. If you do, …" is a price no
            // effect here takes on the paying answer.
            (Price::Part(part), true) => {
                return self.deny(format!("a switched unless-cost of `{part}`"));
            }
        })
    }

    /// One effect API as the `Effect` expressions it stands for.
    ///
    /// Every parameter a rule reads is *taken* from `p`; the caller then
    /// refuses the card if anything is left, which is what stops an ignored
    /// `NoRegen$ True` from generating a card that does the wrong thing.
    // One arm per reference API, and the reasons a reading is what it is
    // live beside the arm that makes it.
    #[allow(clippy::too_many_lines)]
    pub(super) fn effect_of(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
            "DealDamage" => {
                let n = self.amount_or_count(&p.take("NumDmg")?)?;
                // "Deals 4 damage to any target and 2 damage to you"
                // (Psionic Blast): the reference gathers both into one
                // simultaneous event and deals it at `DamageResolve`. The
                // engine journals one `DamageDealt` per recipient either
                // way, as it does for `DealDamageEach`, and nothing checks
                // state-based actions between two effects of one
                // resolution, so the two in sequence are the same event.
                if p.take("DamageMap").is_some_and(|v| v != "True") {
                    return None;
                }
                let to = match p.take("Defined").as_deref() {
                    None => aimed.to_string(),
                    // "Deals 1 damage to that player" and every other
                    // player the line names without targeting one.
                    Some(who) => format!("TargetSpec::Player({})", self.player_rel(Some(who))?),
                };
                vec![format!(
                    "Effect::DealDamage {{ amount: {n}, target: {to} }}"
                )]
            }
            // A number, or the X the player announced (Stream of Life's
            // "target player gains X life"): [`amount`] asks both of its
            // questions of an `X`, so a count spelled with the same letter
            // is still refused.
            "GainLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::gain_life({fixed})")],
                    ("PlayerRel::You", None) => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    (who, _) => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            // The same two readings as `GainLife`: Braingeyser's "target
            // player draws X cards".
            "Draw" => {
                let n = amount(
                    p.take("NumCards").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::draw({fixed})")],
                    ("PlayerRel::You", None) => {
                        vec![format!("Effect::DrawCards {{ amount: {n} }}")]
                    }
                    (who, _) => vec![format!(
                        "Effect::DrawCardsFor {{ amount: {n}, who: {who} }}"
                    )],
                }
            }
            // "Target player discards a card": the discarding player
            // chooses, which is what discarding means unless the effect
            // says otherwise (CR 701.9b).
            // "Each player discards their hand" (Wheel of Fortune): every
            // card, nobody choosing.
            "Discard" if p.peek("Mode") == Some("Hand") => {
                p.take("Mode");
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::DiscardHand {{ who: {who} }}")]
            }
            "Discard" if p.peek("Mode") == Some("TgtChoose") => {
                p.take("Mode");
                let n = p
                    .take("NumCards")
                    .as_deref()
                    .unwrap_or("1")
                    .parse::<u8>()
                    .ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardForPlayers {{ who: {who}, count: {n} }}"
                )]
            }
            "Discard" => {
                // Refuse other modes instead of turning a chosen discard
                // into a random one. Unconsumed qualifiers also refuse it.
                if p.take("Mode").as_deref() != Some("Random") {
                    return None;
                }
                let n = amount(
                    p.take("NumCards").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardRandom {{ who: {who}, count: {n} }}"
                )]
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars, self.has_x)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "PutCounter" => {
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = amount(
                    p.take("CounterNum").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                // `AddCounter` puts them on the first target, or on the
                // source when the ability has none. That is the API's own
                // default and so is the right reading of a line with no
                // `Defined$` at all — but `Defined$ Self` says the *source*,
                // and on a line that also targets those are two different
                // permanents.
                //
                // Consumptive Goo is the card that proved it: "{2}{B}{B}:
                // Target creature gets -1/-1 until end of turn. Put a +1/+1
                // counter on this creature." Read as `AddCounter` the counter
                // landed on the *target*, where it cancelled the -1/-1 it was
                // paired with exactly — so the ability resolved, charged four
                // mana and changed nothing at all that a test could see.
                // `AddCounterFilter` over `Filter::This` is the spelling that
                // names the source whatever the ability targets.
                match p.take("Defined").as_deref() {
                    Some("Self") if target.is_some() => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::This, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    None | Some("Self") => {}
                    Some(_) => return None,
                }
                vec![format!(
                    "Effect::AddCounter {{ kind: {kind}, amount: {n} }}"
                )]
            }
            "Scry" => {
                let n = plain_number(p.take("ScryNum").as_deref().unwrap_or("1"), self.svars)?;
                vec![format!("Effect::scry({n})")]
            }
            "Surveil" => {
                // `Amount$` and never `Defined$`: not one of the reference
                // corpus's 228 surveil lines names a player, because a
                // surveil is the controller's own library by construction
                // (CR 701.25a). 224 of them write `Amount$` and 219 of those
                // write a plain number; the rest announce an `X`, and the
                // constructor only fits the first kind.
                let raw = p.take("Amount")?;
                if let Some(n) = plain_number(&raw, self.svars) {
                    vec![format!("Effect::surveil({n})")]
                } else {
                    let n = amount(&raw, self.svars, self.has_x)?;
                    vec![format!("Effect::Surveil {{ amount: {n} }}")]
                }
            }
            "Mana" => self.mana_effect(p)?,
            "DelayedTrigger" => vec![self.delayed_trigger(p, target)?],
            "Destroy" => {
                // `NoRegen$ True` is **read** and no longer merely consumed.
                // It was vacuous while this engine had no regeneration at
                // all — one door, and every card printing "it can't be
                // regenerated" was right for free — and the day a shield
                // existed the two doors stopped being the same function
                // (CR 701.19c). Any other value is a third thing the DSL
                // cannot say, so it refuses.
                let no_regen = match p.take("NoRegen").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let verb = if no_regen {
                    "destroy_no_regen"
                } else {
                    "destroy"
                };
                // "Destroy that creature" in a delayed trigger's body: the
                // object it remembers ([`Tx::delayed_trigger`]). Anywhere
                // else the key is left for the unclaimed check to refuse.
                let remembered = self.in_delayed
                    && target.is_none()
                    && matches!(
                        p.peek("Defined"),
                        Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                    );
                if remembered {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::EventObject)")]
                } else {
                    vec![format!("Effect::{verb}({aimed})")]
                }
            }
            // "Destroy all lands", "destroy all creatures. They can't be
            // regenerated": every permanent the valid-string names, none of
            // them a target (so an ability that also targets is refused
            // rather than read as a sweep of its target). `NoRegen$` is the
            // same two doors as the single destroy above (CR 701.19c).
            "DestroyAll" => {
                if target.is_some() {
                    return None;
                }
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let verb = match p.take("NoRegen").as_deref() {
                    None => "destroy_all",
                    Some("True") => "destroy_all_no_regen",
                    Some(_) => return None,
                };
                vec![format!("Effect::{verb}(&{filter})")]
            }
            // "X damage to each creature without flying and each player"
            // (Earthquake): `DealDamageEach` for the permanents and a
            // `DealDamage` for the players, which is the one spelling
            // `DealDamageEach` names for the second half. Nothing is
            // targeted, so an ability that also targets is refused.
            // Where the reference deals the damage `DamageMap$` gathered:
            // the `DealDamage` lines before it already did.
            "DamageResolve" => Vec::new(),
            // "Prevent the next N damage that would be dealt to any target
            // this turn" (Samite Healer; CR 615.7): a shield on what the
            // line targets, or on the player `Defined$` names. With
            // neither, nothing is shielded, and that is not a card.
            "PreventDamage" => {
                let n = amount(&p.take("Amount")?, self.svars, self.has_x)?;
                let to = match (p.take("Defined").as_deref(), target) {
                    (Some(who), None) => {
                        format!("TargetSpec::Player({})", self.player_rel(Some(who))?)
                    }
                    (None, Some(aimed)) => aimed.to_string(),
                    _ => return None,
                };
                vec![format!(
                    "Effect::PreventNextDamage {{ target: {to}, amount: {n} }}"
                )]
            }
            // "Prevent all combat damage that would be dealt this turn."
            // The bare line only: the reference narrows it with keys this
            // rule does not claim, and those refuse.
            "Fog" => vec!["Effect::PreventAllCombatDamageThisTurn".to_string()],
            "DamageAll" => {
                if target.is_some() {
                    return None;
                }
                let n = amount(&p.take("NumDmg")?, self.svars, self.has_x)?;
                // The printed words for the two keys below ("each creature
                // and each player"), which say nothing those keys do not.
                p.take("ValidDescription");
                let mut out = Vec::new();
                if let Some(valid) = p.take("ValidCards") {
                    let filter = self.filter_expr(&valid)?;
                    let filter = self.body.filter_static("EACH", &filter);
                    out.push(format!(
                        "Effect::DealDamageEach {{ amount: {n}, filter: &{filter} }}"
                    ));
                }
                if let Some(players) = p.take("ValidPlayers") {
                    let who = self.player_rel(Some(&players))?;
                    out.push(format!(
                        "Effect::DealDamage {{ amount: {n}, target: TargetSpec::Player({who}) }}"
                    ));
                }
                if out.is_empty() {
                    return None;
                }
                out
            }
            "Regenerate" => {
                // Bare `AB$ Regenerate` is "regenerate CARDNAME" — 181 of
                // the reference's 275 lines writing this API name no target
                // at all — so the absent target is the source and not a
                // missing one, the same reading `Untap` makes two arms
                // below. `Defined$` names something else entirely (the
                // enchanted creature, a remembered object; 33 lines) and is
                // refused rather than guessed at, which leaves the 62 that
                // carry a `ValidTgts$` as the targeted half.
                match p.take("Defined").as_deref() {
                    None => {}
                    // "Regenerate enchanted creature" (Regeneration): the
                    // Aura's host, which the filter binds to the source.
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(vec![
                            "Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"
                                .to_string(),
                        ]);
                    }
                    Some(_) => return None,
                }
                match target {
                    Some(t) => vec![format!("Effect::regenerate({t})")],
                    None => vec!["Effect::regenerate(TargetSpec::ThisObject)".to_string()],
                }
            }
            "Token" => self.token_effect(p, targets_a_player)?,
            "Investigate" => self.investigate_effect(p, targets_a_player)?,
            "Animate" => self.animate_effect(p, target)?,
            "Pump" => self.pump_effect(p, aimed)?,
            "Effect" => self.static_effect(p, target)?,
            "ChangeZone" => self.change_zone(p, target)?,
            "ChangeZoneAll" => self.shuffle_into_library(p, target, targets_a_player)?,
            // "Look at the top three cards of target player's library and
            // put them back in any order. You may have that player shuffle"
            // (Natural Selection). The ability's controller looks and
            // decides; a count the player announced is refused.
            "RearrangeTopOfLibrary" => {
                let count: u8 = p.take("NumCards")?.parse().ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                let mut effects = vec![if who == "PlayerRel::You" {
                    format!("Effect::ReorderTopLibrary {{ count: {count} }}")
                } else {
                    format!("Effect::ReorderTopLibraryOf {{ who: {who}, count: {count} }}")
                }];
                match p.take("MayShuffle").as_deref() {
                    None => {}
                    Some("True") => effects.push(format!(
                        "Effect::MayDo {{ effects: &[Effect::ShuffleLibrary {{ who: {who} }}] }}"
                    )),
                    Some(_) => return None,
                }
                effects
            }
            "Sacrifice" => self.sacrifice_effect(p)?,
            // "Tap enchanted creature" (Paralyze): the host, and not a
            // target.
            // "Tap all lands target player controls" (Mana Short): the
            // permanents of the players `Defined$` names, or of the line's
            // own player target. Without either it is every matching
            // permanent, `TapAll`, and the line must target nothing, for
            // a target it never uses is not a card.
            "TapAll" => {
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let defined = p.take("Defined");
                if defined.is_none() && !targets_a_player {
                    if target.is_some() {
                        return None;
                    }
                    return Some(vec![format!("Effect::TapAll {{ filter: &{filter} }}")]);
                }
                let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::TapAllOf {{ who: {who}, filter: &{filter} }}"
                )]
            }
            // "That player loses all unspent mana" (CR 106.4).
            "DrainMana" => {
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseUnspentMana {{ who: {who} }}")]
            }
            // "You may tap or untap target artifact, creature, or land"
            // (Twiddle): one of the two choices always does nothing, so
            // the choice is a yes or a no (`Effect::ToggleTapTarget`).
            "TapOrUntap" if target.is_some() && p.take("Defined").is_none() => {
                vec!["Effect::MayDo { effects: &[Effect::ToggleTapTarget] }".to_string()]
            }
            "Tap" => match p.take("Defined").as_deref() {
                None => vec!["Effect::TapTarget".to_string()],
                Some("Enchanted" | "Equipped") if target.is_none() => {
                    vec!["Effect::TapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                Some(_) => return None,
            },
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(_)) => vec!["Effect::UntapTarget".to_string()],
                (None, None) => vec!["Effect::UntapSelf".to_string()],
                // "Untap enchanted creature" (Instill Energy).
                (Some("Enchanted" | "Equipped"), None) => {
                    vec!["Effect::UntapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                _ => return None,
            },
            // "Take an extra turn after this one" (Time Walk; CR 500.7).
            // One turn, and yours: another count or another player is a
            // different sentence.
            "AddTurn" => {
                if p.take("NumTurns").as_deref() != Some("1")
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                vec!["Effect::TakeExtraTurn".to_string()]
            }
            "Counter" => {
                if p.take("TargetType").as_deref() != Some("Spell") {
                    return None;
                }
                vec!["Effect::CounterTargetSpell".to_string()]
            }
            _ => return None,
        })
    }
}
