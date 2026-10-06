//! Reading activated abilities, spells, charms and their conditions.

use super::{
    Bound, Chain, Params, Price, Tx, cost_parts, count_bound, hybrid_pair, on_the_battlefield,
    printed_mana,
};

impl Tx<'_> {
    /// When an ability may be activated, beyond its `IsPresent$` clause: a
    /// restriction on activating (CR 602.5) that the `condition` field says,
    /// and so only where no other clause already fills it. Hands back the
    /// keys it claimed.
    ///
    /// - `PlayerTurn$ True`: "activate only during your turn" (Disrupting
    ///   Scepter).
    /// - `ActivationPhases$ BeginCombat->EndCombat`: "activate only during
    ///   combat" (Jade Statue). The other spellings — an upkeep, a single
    ///   combat step, "before blockers are declared" — are other sentences.
    pub(super) fn activation_restriction(
        &mut self,
        probe: &mut Params,
        is_activated: bool,
        condition: &mut String,
    ) -> Option<Vec<&'static str>> {
        let mut claimed = Vec::new();
        if let Some(your_turn) = probe.take("PlayerTurn") {
            if your_turn != "True" || !is_activated || !condition.is_empty() {
                return self.deny(format!("`PlayerTurn$ {your_turn}` beside another clause"));
            }
            *condition = ", condition = Some(Condition::YourTurn)".to_string();
            claimed.push("PlayerTurn$");
        }
        if let Some(phases) = probe.take("ActivationPhases") {
            if phases != "BeginCombat->EndCombat" || !is_activated || !condition.is_empty() {
                return self.deny(format!("`ActivationPhases$ {phases}`"));
            }
            *condition = ", condition = Some(Condition::DuringCombat)".to_string();
            claimed.push("ActivationPhases$");
        }
        Some(claimed)
    }

    pub(super) fn activated_or_spell(&mut self, spec: &str) -> Option<()> {
        let is_activated = spec.starts_with("AB$");
        if !is_activated && Params::parse(spec).is_some_and(|(api, _)| api == "Charm") {
            return self.charm(spec);
        }
        let Some((_, mut probe)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        probe.drop_prose();
        let cost = probe.take("Cost");
        // "Activate only once each turn" belongs to the ability for the same
        // reason the cost does: it restricts activating it, not what happens
        // when it resolves.
        let limit = probe.take("ActivationLimit");
        // And so does the clause, which on an `A:` line is a restriction on
        // activating rather than CR 603.4's intervening `if`: the reference
        // spells a resolution-time condition `ConditionPresent$`, a
        // different key on a different line (1521 of them, all on `SVar:`),
        // and this reader claims neither it nor its family.
        let mut condition = self.condition(&mut probe)?;
        // The `IsPresent$` family is claimed only where that clause was read,
        // not where a restriction below filled `condition` instead.
        let present_read = !condition.is_empty();
        let restrictions = self.activation_restriction(&mut probe, is_activated, &mut condition)?;
        let mut chain = Chain::default();
        // The cost belongs to the ability, not to the effect chain, so it is
        // removed from the spec before the chain reads it. The clause's keys
        // go with it, but **only** once the clause was read: a line carrying
        // `PresentZone$` and no `IsPresent$` at all keeps it, and refuses one
        // level down as the unclaimed parameter it is.
        let claimed: &[&str] = if present_read {
            &[
                "IsPresent$",
                "PresentZone$",
                "PresentCompare$",
                "PresentDefined$",
            ]
        } else {
            &[]
        };
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$") && !part.starts_with("ActivationLimit$"))
            .filter(|part| !restrictions.iter().any(|key| part.starts_with(key)))
            .filter(|part| !claimed.iter().any(|key| part.starts_with(key)))
            .collect();
        self.chain(&stripped.join(" | "), &mut chain)?;
        if chain.effects.is_empty() {
            return self.deny("an ability that reads as no effect at all".to_string());
        }
        let effects = format!("&[{}]", chain.effects.join(", "));
        if is_activated {
            let Some(cost) = cost else {
                return self.deny("an activated ability with no `Cost$`".to_string());
            };
            let cost = self.cost_expr(&cost)?;
            // CR 605.1a includes the library-movement exclusion. These
            // properties describe this chain, not separately created delayed
            // or granted abilities and not any external replacement.
            let mana_ability =
                chain.could_add_mana && !chain.moves_library && chain.target.is_none();
            let target = chain
                .target
                .map(|t| format!(", target = Some({t})"))
                .unwrap_or_default();
            let limit = match limit {
                None => String::new(),
                Some(n) => {
                    let Ok(n) = n.parse::<u8>() else {
                        // `GE4` and `X` are the corpus's other two spellings
                        // (5 scripts between them) and neither is a count
                        // this side can read: one is a *threshold* on the
                        // counters already spent, the other a number the
                        // board works out.
                        return self.deny(format!("activation limit `{n}`"));
                    };
                    format!(", limit = ActivationLimit::PerTurn({n})")
                }
            };
            // `mana_ability!(effects)` *is* `mana_ability!({T}, effects)`
            // — the macro supplies the tap, because tapping is what almost
            // every mana ability costs. Writing the cost out again says
            // nothing and reads as though this one were the exception.
            //
            // The short form takes the cost as its *only* positional
            // argument, so it is right exactly while there is nothing else
            // to say: `mana_ability!(&[…], limit = …)` would bind the
            // effects where the cost goes and `limit = …` — an assignment
            // expression, and so a legal one — where the effects go.
            let extras = format!("{target}{limit}{condition}");
            let line = match (mana_ability, cost.as_str()) {
                (true, "Cost::TAP") if extras.is_empty() => format!("mana_ability!({effects})"),
                (true, _) => format!("mana_ability!({cost}, {effects}{extras})"),
                (false, _) => format!("activated!({cost}, {effects}{extras})"),
            };
            self.body.abilities.push(line);
        } else {
            if limit.is_some() {
                // A spell is cast, not activated, so there is nothing for the
                // key to restrict and dropping it quietly would be the
                // unclaimed-parameter fault one level down.
                return self.deny("`ActivationLimit$` on a spell line".to_string());
            }
            if !condition.is_empty() {
                // The same, for the same reason: `spell!` has no
                // precondition, and a restriction on casting is a rule this
                // DSL does not have (CR 601.2 has no place for one).
                return self.deny("`IsPresent$` on a spell line".to_string());
            }
            // And the third of them, which this branch dropped in silence
            // until a batch of four hundred cards walked into it. A spell's
            // `Cost$` is its mana cost *plus* whatever else the card charges
            // (CR 601.2b), and only the mana half is on the face — so
            // Kaervek's Spite came out as three mana for "target player
            // loses 5 life", with "sacrifice all permanents you control and
            // discard your hand" nowhere in the card, and Crop Rotation as a
            // one-mana tutor that sacrifices no land. 215 of the 236
            // `A:SP$` lines naming a `Cost$` charge something beside mana.
            //
            // `FaceDef::mandatory_additional_costs` is where they belong and
            // the engine collects them at cast; emitting them is the next
            // step and not this one, because it would walk a dozen cards
            // into a path exactly one card in the pool has ever played
            // (Toxic Deluge's `PayLifeX`). Until then the honest answer is
            // the stub. See #52.
            //
            // The token is read here rather than through `cost_pieces`,
            // which prices an *activation* and denies what it cannot price:
            // routing a spell's cost through it would refuse `Cost$ X G` for
            // its bare `X` — mana the face already carries — and take a card
            // off the list for the one thing that is not wrong with it.
            //
            // So the question asked of each token is the narrow one: is this
            // the card's own mana cost? Everything else refuses, which is an
            // allow-list on purpose. The alternative — refusing the `<…>`
            // spelling every one of those 215 additional costs happens to
            // use — goes silent on a restriction that needs no brackets, and
            // one of those is already here: `XMin1` is "X can't be 0" and
            // not mana at all (Ertai's Meddling and four others, none of
            // them in this pool yet, all five reachable by a batch).
            if let Some(raw) = &cost {
                for token in cost_parts(raw) {
                    let is_mana = token.chars().all(|c| c.is_ascii_digit())
                        || matches!(token.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "X")
                        || hybrid_pair(&token).is_some();
                    if !is_mana {
                        let head = token.split('<').next().unwrap_or(&token);
                        return self.deny(format!(
                            "`{head}` on a spell line, beside the mana the face carries"
                        ));
                    }
                }
            }
            let targets = chain
                .target
                .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
                .unwrap_or_default();
            self.body
                .abilities
                .push(format!("spell!({effects}{targets})"));
        }
        Some(())
    }

    /// "Choose one —" (CR 700.2): `SP$ Charm | Choices$ A,B` is a modal
    /// spell whose modes are the named `SVar`s, each read as the chain it
    /// is and each targeting for itself, since a target a mode names is
    /// chosen only when that mode is (CR 700.2c).
    ///
    /// Only a spell, and only "choose one" or "choose two": a modal
    /// activated ability is a different `AbilityDef`, and the other counts
    /// the reference spells (`MinCharmNum$`, a count the board works out)
    /// are rules this reader has not met yet.
    pub(super) fn charm(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        let choose = match p.take("CharmNum").as_deref() {
            None | Some("1") => "ModeCount::ONE",
            Some("2") => "ModeCount::TWO",
            Some(n) => return self.deny(format!("a charm choosing `{n}`")),
        };
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `Charm.{key}`"));
            return None;
        }
        let mut modes = Vec::new();
        for name in choices.split(',').map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("charm choice `{name}` with no `SVar`"));
            };
            let mut chain = Chain::default();
            self.chain(&line, &mut chain)?;
            if chain.effects.is_empty() {
                return self.deny("a charm mode that reads as no effect at all".to_string());
            }
            let targets = chain
                .target
                .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
                .unwrap_or_default();
            modes.push(format!("mode!(&[{}]{targets})", chain.effects.join(", ")));
        }
        if modes.len() < 2 {
            return self.deny("a charm with fewer than two modes".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::ModalSpell {{ choose: {choose}, modes: &[{}] }}",
            modes.join(", ")
        ));
        Some(())
    }

    /// The reference's `IsPresent$` family as an intervening-`if` clause
    /// (CR 603.4).
    ///
    /// What comes back is the macro argument and not the condition: three
    /// answers fit in one `Option<String>` that way — `None` is a line
    /// refused with a reason, an empty string one that prints no clause at
    /// all, and anything else the `, condition = Some(…)` to splice in.
    /// `triggered!`, `activated!` and `mana_ability!` all spell the field
    /// the same, so the next caller needs no second shape.
    ///
    /// The family is seven keys and they are read here once, rather than at
    /// each `Mode$` that might carry them: 605 `T:` lines in the corpus
    /// write `IsPresent$`, spread across every trigger mode there is.
    ///
    /// **Two sentences come out of it**, and which one depends on what the
    /// clause is *about*. `PresentDefined$ Self`, or a valid-string whose
    /// every alternative pins the object with `Self`, is a clause about
    /// this card — `Condition::SourceMatches`. Anything else is a count,
    /// and a count is only readable here when the filter says whose: every
    /// alternative has to carry `YouCtrl`, or the sentence is "there exists
    /// a creature" and `Condition::ControlCount` would answer a narrower
    /// question than the card asks.
    ///
    /// **The zone is written into the filter**, and that is not a
    /// redundancy. `IsPresent$` asks whether an object is present *in a
    /// zone* — `PresentZone$`, defaulting to the battlefield — so the
    /// clause about this card is "this permanent is on the battlefield and
    /// matches", and a filter that left the zone out would be a different
    /// sentence at CR 603.4's **second** check: the source is a battlefield
    /// permanent when the trigger is collected, and need not still be one
    /// when the ability resolves. This engine keeps an object's id across a
    /// zone change, so `SourceMatches` on its own answers for a card that
    /// has died. What is cleared on the way out (CR 400.7) is the half a
    /// permanent has — status, damage, counters — so `Card.tapped` would
    /// have been right by accident, and `Card.Self+YouCtrl` wrong, since a
    /// card in a graveyard keeps the controller it had.
    ///
    /// `NoResolvingCheck$ True` is the one that has to be named rather than
    /// ignored: 61 scripts carry it, and it is the reference opting *out*
    /// of CR 603.4's second check — a clause asked once instead of twice,
    /// which this DSL cannot say at all.
    pub(super) fn condition(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("IsPresent") else {
            return Some(String::new());
        };
        let clause = self.present_clause(p, &valid)?;
        Some(format!(", condition = Some({clause})"))
    }

    /// The rest of the `IsPresent$` family, `valid` being that key's value,
    /// as a `Condition` expression, or `None` when it was refused. What
    /// [`Self::condition`] splices in as an intervening "if", and what a
    /// state trigger (`Mode$ Always`, CR 603.8) triggers on.
    pub(super) fn present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        if p.take("NoResolvingCheck").is_some() {
            return self.deny("`NoResolvingCheck$`, a clause checked once".to_string());
        }
        if p.take("IsPresent2").is_some() {
            return self.deny("a second `IsPresent2$` clause".to_string());
        }
        if let Some(who) = p.take("PresentPlayer") {
            return self.deny(format!("`PresentPlayer$ {who}`"));
        }
        match p.take("PresentZone").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => return self.deny(format!("`PresentZone$ {zone}`")),
        }
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let pins_self = |alt: &str| alt.split(['.', '+']).any(|atom| atom.trim() == "Self");
        let about_source = match p.take("PresentDefined").as_deref() {
            Some("Self") => true,
            Some(other) => return self.deny(format!("`PresentDefined$ {other}`")),
            None => valid.split(',').all(pins_self),
        };
        // Both readings need a number before the filter is worth building,
        // so that a refusal names the clause rather than an atom inside it.
        let count = if about_source {
            if compare != "GE1" {
                return self.deny(format!("a clause about this card compared `{compare}`"));
            }
            None
        } else {
            let Some(bound) = count_bound(&compare) else {
                return self.deny(format!("`PresentCompare$ {compare}`"));
            };
            // Whose permanents: yours (`YouCtrl` in every alternative), or
            // everybody's when no alternative names a controller or an
            // owner at all ("if no creatures are on the battlefield").
            let atoms = || valid.split(',').flat_map(|alt| alt.split(['.', '+']));
            let yours = valid
                .split(',')
                .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
            let nobodys = !atoms().any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
            if !yours && !nobodys {
                return self.deny(format!("`IsPresent$ {valid}`, a count of somebody else's"));
            }
            // And a count says nothing about the card that states it.
            // `eval::condition_holds` walks a battlefield handing each
            // candidate its *own* id as the object a filter's `This` and
            // `Another` compare against, so "another creature you control"
            // would count nothing at all — 28 corpus lines write one, and
            // a trigger that can never fire is exactly the wrong card the
            // honest-stub rule exists to refuse.
            // The attachment atoms are relative too: `AttachedToBySource`
            // asks what *this* card is attached to.
            if valid.split(',').any(|alt| {
                alt.split(['.', '+']).any(|a| {
                    matches!(
                        a.trim(),
                        "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
                    )
                })
            }) {
                return self.deny(format!(
                    "`IsPresent$ {valid}`, a count relative to this card"
                ));
            }
            Some((yours, bound))
        };
        let expr = self.filter_expr(valid)?;
        let clause = match count {
            None => {
                let zoned = on_the_battlefield(&expr);
                let name = self.body.filter_static("CHECK", &zoned);
                format!("Condition::SourceMatches(&{name})")
            }
            // `ControlCount` counts one player's battlefield and nothing
            // else, and `BattlefieldCount` all of it, so the zone and the
            // player are both already in the sentence each is.
            Some((yours, bound)) => {
                let name = self.body.filter_static("CHECK", &expr);
                let (variant, n) = match (yours, bound) {
                    (true, Bound::AtLeast(n)) => ("ControlCount", n),
                    (true, Bound::AtMost(n)) => ("ControlCountAtMost", n),
                    (false, Bound::AtLeast(n)) => ("BattlefieldCount", n),
                    (false, Bound::AtMost(n)) => ("BattlefieldCountAtMost", n),
                };
                format!("Condition::{variant}(&{name}, {n})")
            }
        };
        Some(clause)
    }

    /// The executed line of a "you may pay {N}. If you do, …" trigger with
    /// its price taken off, and the price: `AB$ GainLife | Cost$ 1 | …`
    /// under `OptionalDecider$ You`. `None` for anything else — no "may",
    /// no `Cost$`, or a cost that is not mana alone — which leaves the line
    /// as it was for the chain to read or refuse.
    pub(super) fn optional_price(body: &str, may: bool) -> Option<(String, Price)> {
        if !may || !body.trim_start().starts_with("AB$") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        let price = match cost.parse::<u32>() {
            Ok(0) => return None,
            Ok(n) => Price::Generic(format!("Amount::Fixed({n})")),
            Err(_) => Price::Printed(printed_mana(cost)?),
        };
        let rest: Vec<&str> = parts
            .into_iter()
            .filter(|part| !part.starts_with("Cost$"))
            .collect();
        Some((rest.join(" | "), price))
    }
}
