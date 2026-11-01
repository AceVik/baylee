//! Reading what a script points at: filters, targets, players, and the
//! sources a prevention names.

use super::{Params, Tx, color_atom, counter_kind};

impl Tx<'_> {
    /// "The next time a red source of your choice would deal damage to you
    /// this turn, prevent that damage" (the Circles of Protection; CR 609.7a,
    /// 615.8), and Reverse Damage's "you gain life equal to the damage
    /// prevented this way" after it (CR 615.5).
    ///
    /// The reference says it in three lines: `ChooseSource` picks, an
    /// `Effect` it puts in the command zone carries a replacement waiting for
    /// the chosen source, and the replacement exiles that effect once it has
    /// prevented. Every line must be this sentence key for key or the card
    /// is refused — a pact's delayed trigger, a chosen colour, a replacement
    /// that does not recheck what was chosen are other sentences.
    pub(super) fn prevent_from_chosen_source(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let Some(choices) = p.take("Choices") else {
            return self.deny("`ChooseSource` with no `Choices$`".to_string());
        };
        if choices == "Card.SharesColorWith Imprinted" {
            return self.prevent_from_exiled_color(p, sub);
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseSource.{key}`"));
        }
        // What may be chosen, and the same words as the replacement must
        // recheck them when the damage comes (CR 609.7b). "A source of your
        // choice" is any card or emblem; the engine does not offer an emblem
        // (`prevention::source_options`), and no emblem in the pool deals
        // damage.
        let (filter, recheck) = if choices == "Card,Emblem" {
            (
                "Filter::Any".to_string(),
                "Card.ChosenCardStrict,Emblem.ChosenCard".to_string(),
            )
        } else if choices.contains(',') {
            return self.deny(format!("chosen source `{choices}`"));
        } else {
            let atoms = choices.strip_prefix("Card.").unwrap_or(&choices);
            // "A red source" is a red object: the reference's `RedSource`.
            let read = match atoms.strip_suffix("Source") {
                Some(color) if color_atom(color).is_some() => format!("Card.{color}"),
                Some(_) => return self.deny(format!("chosen source `{choices}`")),
                None => choices.clone(),
            };
            let filter = self.filter_expr(&read)?;
            (filter, format!("Card.ChosenCardStrict+{atoms}"))
        };
        let Some(effect_name) = sub else {
            return self.deny("`ChooseSource` with nothing chosen for".to_string());
        };
        let Some(body) = self.svars.get(effect_name).cloned() else {
            return self.deny(format!("`SubAbility$ {effect_name}` names no SVar"));
        };
        let Some((api, mut effect)) = Params::parse(&body) else {
            return self.deny(format!("`SubAbility$ {effect_name}` is no ability"));
        };
        if api != "Effect" {
            return self.deny("`ChooseSource` followed by something other than its shield".into());
        }
        effect.drop_prose();
        let condition_holds = effect.take("ConditionDefined").as_deref() == Some("ChosenCard")
            && matches!(
                effect.take("ConditionPresent").as_deref(),
                Some("Card" | "Card,Emblem")
            )
            && matches!(
                effect.take("ConditionCompare").as_deref(),
                None | Some("GE1")
            );
        if !condition_holds {
            return self.deny("a chosen-source shield on another condition".to_string());
        }
        let cleanup = effect.take("SubAbility");
        let Some(replacement) = effect.take("ReplacementEffects") else {
            return self.deny("a chosen-source `Effect` with no replacement".to_string());
        };
        if let Some(key) = effect.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        if let Some(name) = cleanup {
            let clears = self
                .svars
                .get(&name)
                .and_then(|body| Params::parse(body))
                .is_some_and(|(api, mut c)| {
                    api == "Cleanup"
                        && c.take("ClearChosenCard").as_deref() == Some("True")
                        && c.exhausted()
                });
            if !clears {
                return self.deny(format!("`{name}` after a chosen-source shield"));
            }
        }
        if choices == "Card,Emblem" && self.all_damage_from_chosen_source(&replacement) {
            return Some(vec![
                "Effect::ChooseDamageSourcePreventAllThisTurn".to_string(),
            ]);
        }
        let gain_life = self.chosen_source_replacement(&replacement, recheck)?;
        let sources = self.body.filter_static("SOURCE", &filter);
        Some(vec![format!(
            "Effect::PreventNextFromChosenSource {{ sources: &{sources}, combat_only: false, \
             all_but: 0, gain_life: {gain_life} }}"
        )])
    }

    /// "Target permanent becomes the color of your choice until end of turn"
    /// (Distorting Lens, Alchor's Tomb): `ChooseColor` followed by `DB$ Animate
    /// | ValidTgts$ <valid> | Colors$ ChosenColor | OverwriteColors$ True`.
    pub(super) fn choose_color_animate(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("You") {
            return self.deny("`ChooseColor` for somebody other than `You`".to_string());
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseColor.{key}`"));
        }
        let Some(animate_name) = sub else {
            return self.deny("`ChooseColor` with nothing chosen for".to_string());
        };
        let Some(body) = self.svars.get(animate_name).cloned() else {
            return self.deny(format!("`SubAbility$ {animate_name}` names no SVar"));
        };
        let Some((api, mut animate)) = Params::parse(&body) else {
            return self.deny(format!("`SubAbility$ {animate_name}` is no ability"));
        };
        if api != "Animate" {
            return self.deny("`ChooseColor` followed by something other than `Animate`".into());
        }
        animate.drop_prose();
        let subject = if let Some(valid) = animate.take("ValidTgts") {
            if chain.target.is_some() {
                return self.deny("`ChooseColor` `Animate` with a second target".to_string());
            }
            chain.target = Some(self.target_spec(&valid, "Animate")?);
            "ColorSubject::Target"
        } else if animate.take("Defined").as_deref() == Some("Self") {
            "ColorSubject::Source"
        } else {
            return self.deny("`ChooseColor` `Animate` with no `ValidTgts$`".to_string());
        };
        if animate.take("Colors").as_deref() != Some("ChosenColor")
            || animate.take("OverwriteColors").as_deref() != Some("True")
        {
            return self.deny("`ChooseColor` `Animate` not overwriting with `ChosenColor`".into());
        }
        let duration = match animate.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(other) => return self.deny(format!("`ChooseColor` `Animate` lasting `{other}`")),
        };
        if let Some(key) = animate.first_key() {
            return self.deny(format!("unclaimed parameter `Animate.{key}`"));
        }
        Some(vec![format!(
            "Effect::ChooseColors {{ subject: {subject}, max: 1, duration: {duration} }}"
        )])
    }

    /// Choose a source sharing a color with the source's exiled card and
    /// prevent all of its damage this turn. The imprint link is structural;
    /// the cleanup lines only mirror what the runtime riders already do.
    fn prevent_from_exiled_color(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.take("RememberChosen").as_deref() != Some("True") {
            return None;
        }
        p.take("AILogic");
        p.take("ChoiceTitle");
        if !p.exhausted() {
            return None;
        }
        let (api, mut effect) = Params::parse(self.svars.get(sub?.trim())?)?;
        effect.drop_prose();
        if api != "Effect" || effect.take("RememberObjects").as_deref() != Some("Remembered") {
            return None;
        }
        let replacement = effect.take("ReplacementEffects")?;
        let cleanup = effect.take("SubAbility");
        if !effect.exhausted() {
            return None;
        }
        if let Some(name) = cleanup {
            let (cleanup_api, mut c) = Params::parse(self.svars.get(name.trim())?)?;
            if cleanup_api != "Cleanup"
                || c.take("ClearRemembered").as_deref() != Some("True")
                || !c.exhausted()
            {
                return None;
            }
        }
        let (event, mut shield) = Params::parse(self.svars.get(replacement.trim())?)?;
        shield.drop_prose();
        if event != "DamageDone"
            || shield.take("ValidSource").as_deref() != Some("Card.IsRemembered")
            || shield.take("Prevent").as_deref() != Some("True")
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
            || !shield.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::ChooseDamageSourcePreventAllThisTurnMatching(&Filter::SharesColorWithExiledWithSource)"
                .to_string(),
        ])
    }

    /// Exact repeatable prevention, all recipients and no consumption instruction.
    fn all_damage_from_chosen_source(&self, name: &str) -> bool {
        self.svars
            .get(name)
            .and_then(|body| Params::parse(body))
            .is_some_and(|(event, mut shield)| {
                shield.drop_prose();
                event == "DamageDone"
                    && shield.take("ValidSource").as_deref()
                        == Some("Card.ChosenCardStrict,Emblem.ChosenCard")
                    && shield.take("Prevent").as_deref() == Some("True")
                    && shield.exhausted()
            })
    }

    /// The replacement a chosen-source shield waits with: damage from the
    /// chosen source (`recheck`, its properties spelled as the choice spelled
    /// them) to you, prevented, and then either nothing more or "you gain
    /// life equal to the damage prevented this way". The answer is whether
    /// it gains the life.
    pub(super) fn chosen_source_replacement(
        &self,
        replacement: &str,
        recheck: String,
    ) -> Option<bool> {
        let Some((event, mut shield)) = self.svars.get(replacement).and_then(|b| Params::parse(b))
        else {
            return self.deny(format!("replacement `{replacement}` names no SVar"));
        };
        shield.drop_prose();
        let waits = event == "DamageDone"
            && shield.take("ValidSource") == Some(recheck)
            && shield.take("ValidTarget").as_deref() == Some("You")
            && shield.take("PreventionEffect").as_deref() == Some("True");
        let Some(then) = shield.take("ReplaceWith").filter(|_| waits) else {
            return self.deny("a chosen-source replacement that is not a shield on you".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        match self.svars.get(&then).and_then(|b| Params::parse(b)) {
            Some((api, mut gain)) if api == "GainLife" => {
                let reads = gain.take("Defined").as_deref() == Some("You")
                    && gain.take("LifeAmount").is_some_and(|x| {
                        self.svars.get(&x).map(String::as_str) == Some("ReplaceCount$DamageAmount")
                    });
                let exile = gain.take("SubAbility");
                if !reads || !gain.exhausted() || !exile.is_some_and(|e| self.exiles_itself(&e)) {
                    return self.deny("a chosen-source shield's life gain".to_string());
                }
                Some(true)
            }
            _ if self.exiles_itself(&then) => Some(false),
            _ => self.deny(format!("`ReplaceWith$ {then}` on a chosen-source shield")),
        }
    }

    /// Whether `name` is the line a used-up command-zone effect exiles
    /// itself with, and nothing more.
    pub(super) fn exiles_itself(&self, name: &str) -> bool {
        self.svars
            .get(name)
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, mut z)| {
                api == "ChangeZone"
                    && z.take("Defined").as_deref() == Some("Self")
                    && z.take("Origin").as_deref() == Some("Command")
                    && z.take("Destination").as_deref() == Some("Exile")
                    && z.exhausted()
            })
    }

    /// A valid-string (`Creature.YouCtrl+nonToken`) as a `Filter`.
    pub(super) fn filter_expr(&self, valid: &str) -> Option<String> {
        self.filter_expr_in_zone(valid, false)
    }

    fn push_filter_base(&self, base: &str, card_zone: bool, clauses: &mut Vec<String>) -> bool {
        match base {
            "Card" | "card" => {}
            "Permanent" if !card_zone => {}
            "Permanent" => clauses.push(
                "Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::BATTLE)\
                 .union(TypeSet::CREATURE).union(TypeSet::ENCHANTMENT)\
                 .union(TypeSet::LAND).union(TypeSet::PLANESWALKER))"
                    .to_string(),
            ),
            "Creature" => clauses.push("Filter::CREATURE".to_string()),
            "Artifact" => clauses.push("Filter::ARTIFACT".to_string()),
            "Enchantment" => clauses.push("Filter::ENCHANTMENT".to_string()),
            "Land" => clauses.push("Filter::LAND".to_string()),
            "Planeswalker" => clauses.push("Filter::PLANESWALKER".to_string()),
            "Instant" => clauses.push("Filter::HasType(TypeSet::INSTANT)".to_string()),
            "Sorcery" => clauses.push("Filter::HasType(TypeSet::SORCERY)".to_string()),
            "Basic" => clauses.push("Filter::HasSupertype(SupertypeSet::BASIC)".to_string()),
            "Legendary" => {
                clauses.push("Filter::HasSupertype(SupertypeSet::LEGENDARY)".to_string());
            }
            "Snow" => clauses.push("Filter::HasSupertype(SupertypeSet::SNOW)".to_string()),
            "Enchanted" => clauses.push("Filter::AttachedToBySource".to_string()),
            _ => {
                let Some(path) = self.cats.const_path(base) else {
                    self.note(format!("filter base `{base}`"));
                    return false;
                };
                clauses.push(format!("Filter::HasSubtype({path})"));
            }
        }
        true
    }

    /// A valid-string as a `Filter`, distinguishing battlefield objects
    /// (`card_zone == false`, where every object is already a permanent) from
    /// card zones such as graveyards and libraries (`card_zone == true`, where
    /// `"Permanent"` means a permanent card under CR 110.4a).
    #[allow(clippy::too_many_lines)]
    pub(super) fn filter_expr_in_zone(&self, valid: &str, card_zone: bool) -> Option<String> {
        let mut alternatives = Vec::new();
        for alt in valid.split(',') {
            let mut clauses = Vec::new();
            let mut atoms = alt.split('.');
            let base = atoms.next()?.trim();
            if !self.push_filter_base(base, card_zone, &mut clauses) {
                return None;
            }
            for atom in atoms.flat_map(|a| a.split('+')) {
                clauses.push(match atom.trim() {
                    // "Enchanted creature gets +2/+1", "equipped creature has
                    // flying": one filter for both, because there is one
                    // question — what is this permanent attached to — and the
                    // corpus asks it in the two words the two card types
                    // print. An Aura and an Equipment differ in what happens
                    // when the answer is nothing (CR 704.5m against
                    // 704.5n–p), which is the state-based actions' business
                    // and not this clause's.
                    "EnchantedBy" | "EquippedBy" | "AttachedBy" => {
                        "Filter::AttachedToBySource".to_string()
                    }
                    "Attached" => "Filter::AttachedToSource".to_string(),
                    "equipped" => "Filter::Equipped".to_string(),
                    "YouCtrl" => "Filter::ControlledByYou".to_string(),
                    "YouDontCtrl" => "Filter::Not(&Filter::ControlledByYou)".to_string(),
                    "OppCtrl" => "Filter::ControlledByOpponent".to_string(),
                    "ActivePlayerCtrl" => "Filter::ControlledByActivePlayer".to_string(),
                    "YouOwn" => "Filter::OwnedByYou".to_string(),
                    "Other" | "StrictlyOther" => "Filter::Another".to_string(),
                    "Self" => "Filter::This".to_string(),
                    "attacking" => "Filter::Attacking".to_string(),
                    "attackingYou" => "Filter::AttackingYou".to_string(),
                    "blocking" => "Filter::Blocking".to_string(),
                    "blockingSource" => "Filter::BlockingSource".to_string(),
                    "blockedBySource" => "Filter::BlockedBySource".to_string(),
                    "blockingTriggeredAttacker"
                        if self.trigger_mode.as_deref() == Some("AttackerBlocked") =>
                    {
                        "Filter::BlockingSource".to_string()
                    }
                    "blocked" => "Filter::Blocked".to_string(),
                    "unblocked" => "Filter::Unblocked".to_string(),
                    "tapped" => "Filter::Tapped".to_string(),
                    "untapped" => "Filter::Untapped".to_string(),
                    "enchanted" => "Filter::Enchanted".to_string(),
                    "!enchanted" | "unEnchanted" => "Filter::Not(&Filter::Enchanted)".to_string(),
                    "token" => "Filter::IsToken".to_string(),
                    "nonToken" | "!token" => "Filter::Not(&Filter::IsToken)".to_string(),
                    // `LacksType` and not `Not(&…)`: the two are the same
                    // predicate (`eval.rs` negates one line to get the
                    // other) and the DSL already carries a name for each,
                    // so writing the negation out would give "not a
                    // creature" a second spelling that only `filter_hash`
                    // can tell from the first.
                    "nonLand" => "Filter::NONLAND".to_string(),
                    "Artifact" => "Filter::ARTIFACT".to_string(),
                    "Creature" => "Filter::CREATURE".to_string(),
                    "nonCreature" => "Filter::NONCREATURE".to_string(),
                    // Supertypes read like subtypes in a script filter but are
                    // a different set on the card (CR 205.4).
                    "Basic" => "Filter::HasSupertype(SupertypeSet::BASIC)".to_string(),
                    "nonBasic" => {
                        "Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC))".to_string()
                    }
                    "Legendary" => "Filter::HasSupertype(SupertypeSet::LEGENDARY)".to_string(),
                    "nonLegendary" => {
                        "Filter::Not(&Filter::HasSupertype(SupertypeSet::LEGENDARY))".to_string()
                    }
                    "Snow" => "Filter::HasSupertype(SupertypeSet::SNOW)".to_string(),
                    "nonSnow" => {
                        "Filter::Not(&Filter::HasSupertype(SupertypeSet::SNOW))".to_string()
                    }
                    // The two card types a noun cannot say "not" to on its
                    // own ("nonartifact, nonblack creature").
                    "nonArtifact" => "Filter::LacksType(TypeSet::ARTIFACT)".to_string(),
                    "nonEnchantment" => "Filter::LacksType(TypeSet::ENCHANTMENT)".to_string(),
                    "Colorless" => "Filter::IsColorless".to_string(),
                    "MonoColor" => "Filter::Monocolored".to_string(),
                    "MultiColor" => "Filter::And(&[Filter::Not(&Filter::IsColorless), \
                         Filter::Not(&Filter::Monocolored)])"
                        .to_string(),
                    "nonMultiColor" => {
                        "Filter::Or(&[Filter::IsColorless, Filter::Monocolored])".to_string()
                    }
                    "ChosenColor" => "Filter::OfSourceChosenColor".to_string(),
                    "" => continue,
                    other if other.starts_with("counters_") => {
                        let Some((comparison, raw)) = other
                            .strip_prefix("counters_GE")
                            .map(|raw| ("GE", raw))
                            .or_else(|| other.strip_prefix("counters_LT").map(|raw| ("LT", raw)))
                        else {
                            return self.deny(format!("filter atom `{other}`"));
                        };
                        let Some((n, code)) = raw.split_once('_') else {
                            return self.deny(format!("filter atom `{other}`"));
                        };
                        let n = n.parse::<u16>().ok()?;
                        let kind = counter_kind(code)?;
                        let at_least = format!("Filter::CountersAtLeast({kind}, {n})");
                        if comparison == "GE" {
                            at_least
                        } else {
                            format!("Filter::Not(&{at_least})")
                        }
                    }
                    // A colour word (CR 105.2): "black creatures", "target
                    // green spell", "nonblack creature". A colour is not a
                    // subtype, so it is asked before the subtype arm below,
                    // which would otherwise refuse `Black` as an unknown type.
                    other if color_atom(other).is_some() => {
                        let (negated, color) = color_atom(other).unwrap_or_default();
                        let has =
                            format!("Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))");
                        if negated {
                            format!("Filter::Not(&{has})")
                        } else {
                            has
                        }
                    }
                    other if self.worded_atom(other).is_some() => self.worded_atom(other)?,
                    // `Creature.Goblin` puts the subtype after the base, so
                    // an atom can name one too — and it is the commonest
                    // shape in the corpus, not a corner.
                    other => {
                        let (negated, name) = other
                            .strip_prefix("non")
                            .map_or((false, other), |rest| (true, rest));
                        let Some(path) = self.cats.const_path(name) else {
                            self.note(format!("filter atom `{other}`"));
                            return None;
                        };
                        if negated {
                            format!("Filter::Not(&Filter::HasSubtype({path}))")
                        } else {
                            format!("Filter::HasSubtype({path})")
                        }
                    }
                });
            }
            alternatives.push(match clauses.len() {
                0 => "Filter::Any".to_string(),
                1 => clauses.remove(0),
                _ => format!("Filter::And(&[{}])", clauses.join(", ")),
            });
        }
        let expr = match alternatives.len() {
            0 => return None,
            1 => alternatives.remove(0),
            _ => format!("Filter::Or(&[{}])", alternatives.join(", ")),
        };
        Some(Self::named_constant(&expr))
    }

    /// The name for a filter the DSL already has one for, or the expression
    /// unchanged.
    pub(super) fn named_constant(expr: &str) -> String {
        Self::NAMED
            .iter()
            .find(|(written, _)| *written == expr)
            .map_or_else(|| expr.to_string(), |(_, name)| (*name).to_string())
    }

    /// A `ValidTgts$` value as a `TargetSpec` expression.
    /// The zone a target lives in comes from the *effect*, not from the
    /// valid-string: `TargetSpec::Object` enumerates the battlefield and
    /// nothing else, so a counterspell built out of one offers permanents
    /// as targets and counters nothing.
    pub(super) fn target_spec(&mut self, valid: &str, api: &str) -> Option<String> {
        // "Target player" and "target opponent" are both a *choice*, and
        // they are different choices: `Player(PlayerRel::Opponent)` would be
        // every opponent and no choice at all. When the corpus script writes
        // `ValidTgts$ Player` on a card whose Oracle text says "target
        // opponent" (Dread Rider), the Oracle restriction wins.
        if valid == "Player" {
            let lower = self.oracle.to_ascii_lowercase();
            if lower.contains("target opponent") && !lower.contains("target player") {
                return Some("TargetSpec::AnyOpponent".to_string());
            }
            return Some("TargetSpec::AnyPlayer".to_string());
        }
        if valid == "Opponent" {
            return Some("TargetSpec::AnyOpponent".to_string());
        }
        if matches!(valid, "Player,Planeswalker" | "Planeswalker,Player") {
            return Some("TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)".to_string());
        }
        if matches!(valid, "Opponent,Planeswalker" | "Planeswalker,Opponent") {
            return Some("TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)".to_string());
        }
        // "Any target" (CR 115.4) spans objects and players, which is why it
        // is a spec and not a filter — there is nothing on a player for a
        // `Filter` to match.
        if valid == "Any" {
            return Some("TargetSpec::AnyTarget".to_string());
        }
        if self.trigger_mode.as_deref() == Some("DamageDone")
            && api != "Counter"
            && let Some(stripped) = valid
                .split(',')
                .map(|alt| {
                    alt.trim()
                        .strip_suffix(".ControlledBy TriggeredTarget")
                        .or_else(|| alt.trim().strip_suffix("+ControlledBy TriggeredTarget"))
                })
                .collect::<Option<Vec<_>>>()
        {
            let expr = self.filter_expr(&stripped.join(","))?;
            let name = self.body.filter_static("TARGET", &expr);
            return Some(format!("TargetSpec::ObjectOfEventPlayer(&{name})"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("TARGET", &expr);
        Some(match api {
            "Counter" => format!("TargetSpec::Spell(&{name})"),
            _ => format!("TargetSpec::Object(&{name})"),
        })
    }

    /// "Target spell or permanent": `TargetSpec::StackOrBattlefield` over the
    /// valid-string's filter.
    pub(super) fn spell_or_permanent_target(&mut self, valid: &str) -> Option<String> {
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("TARGET", &expr);
        Some(format!("TargetSpec::StackOrBattlefield(&{name})"))
    }

    /// "Target activated ability [from a <valid> source]":
    /// `TargetSpec::AbilityOnStack` over `Filter::ActivatedAbilityFrom`.
    pub(super) fn activated_ability_target(&mut self, valid: &str) -> Option<String> {
        let source = self.filter_expr(valid)?;
        let expr = format!("Filter::ActivatedAbilityFrom(&{source})");
        let name = self.body.filter_static("TARGET", &expr);
        Some(format!("TargetSpec::AbilityOnStack(&{name})"))
    }

    /// "Target creature card in your graveyard": a `CardInGraveyard` spec,
    /// whose player is whose graveyard. In a graveyard the reference's
    /// `YouCtrl` means "yours" (a card there is controlled by nobody, and
    /// its owner is whose graveyard it is in); no such qualifier is any
    /// graveyard unless the Oracle text explicitly scopes it to "your
    /// graveyard" (Misery Charm). The rest of the valid-string is the card's
    /// filter.
    pub(super) fn graveyard_target(
        &mut self,
        valid: &str,
        defined_controller: Option<&str>,
    ) -> Option<String> {
        let mut whose = "PlayerRel::EachPlayer";
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "YouCtrl" | "YouOwn" => whose = "PlayerRel::You",
                    "OppCtrl" | "OppOwn" => whose = "PlayerRel::Opponent",
                    other => kept.push(other),
                }
            }
            // Stir the Grave spells its "with mana value X or less" bound as
            // `SVar:X:Targeted$CardManaCost` instead of `+cmcLEX` on `ValidTgts$`.
            if self.has_x
                && self.svars.get("X").map(String::as_str) == Some("Targeted$CardManaCost")
                && !kept.iter().any(|a| a.starts_with("cmc"))
            {
                kept.push("cmcLEX");
            }
            if kept.is_empty() {
                cleaned_alts.push(base.to_string());
            } else {
                cleaned_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        if whose == "PlayerRel::EachPlayer" {
            let lower = self.oracle.to_ascii_lowercase();
            if (lower.contains("from your graveyard") || lower.contains("in your graveyard"))
                && !lower.contains("from a graveyard")
                && !lower.contains("in a graveyard")
            {
                whose = "PlayerRel::You";
            }
        }
        match defined_controller {
            None => {}
            Some("You") => whose = "PlayerRel::You",
            Some("TriggeredCardController") if whose == "PlayerRel::You" => {}
            Some("TriggeredTarget") if self.trigger_mode.as_deref() == Some("DamageDone") => {
                whose = "PlayerRel::DamagedPlayer";
            }
            Some(_) => return None,
        }
        let filter = self.filter_expr_in_zone(&cleaned_alts.join(","), true)?;
        let filter = self.body.filter_static("TARGET", &filter);
        Some(format!("TargetSpec::CardInGraveyard(&{filter}, {whose})"))
    }

    /// `Defined$ You` and friends as a `PlayerRel`.
    ///
    /// Two of them mean "that player" of the trigger being read, and what
    /// that is depends on the trigger: the player whose step began for a
    /// `Phase` trigger, the controller of the card that moved for a
    /// `ChangesZone` one. Anywhere else the same words name something this
    /// reader cannot see, and refuse.
    pub(super) fn player_rel(&self, defined: Option<&str>) -> Option<&'static str> {
        let trigger = self.trigger_mode.as_deref();
        Some(match defined.unwrap_or("You") {
            "You" => "PlayerRel::You",
            "Opponent" | "Player.Opponent" => "PlayerRel::Opponent",
            "Player" => "PlayerRel::EachPlayer",
            // "Enchanted land's controller" (CR 303.4e).
            "EnchantedController" | "Player.EnchantedController" => {
                "PlayerRel::ControllerOfAttached"
            }
            "TriggeredPlayer" if trigger == Some("Phase") => "PlayerRel::ActivePlayer",
            // Last known (CR 603.10a): a land put into a graveyard is
            // controlled by nobody by the time the ability resolves.
            "TriggeredCardController" if trigger == Some("ChangesZone") => {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredCardOwner" if trigger == Some("ChangesZone") => "PlayerRel::OwnerOfEvent",
            // The permanent tapped for mana is the event's object, and its
            // controller is the only player who can have tapped it for mana
            // (CR 602.2): "its controller" and "that player" are one seat.
            "TriggeredCardController" | "TriggeredActivator" if trigger == Some("TapsForMana") => {
                "PlayerRel::ControllerOfEvent"
            }
            // The permanent that became tapped is the event's object.
            "TriggeredCardController" if matches!(trigger, Some("Taps" | "Untaps")) => {
                "PlayerRel::ControllerOfEvent"
            }
            // "Whenever an opponent casts a spell, … that player": the one
            // who cast it, read off the cast (CR 112.2: the player who put
            // it on the stack). Not "each opponent", which is what the
            // hand-written cards had said (Rhystic Study).
            "TriggeredActivator" if trigger == Some("SpellCast") => "PlayerRel::EventPlayer",
            // "That spell's controller": the spell is the event's object,
            // asked for its controller as the ability resolves, last known
            // if it has left the stack (CR 608.2h).
            "TriggeredCardController" if trigger == Some("SpellCast") => {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredCardController" if trigger == Some("Drawn") => "PlayerRel::EventPlayer",
            // The player a damage trigger's damage was dealt to: the
            // `DamageDone` rule reads only triggers whose target is a
            // player, so `TriggeredTarget` is one.
            "TriggeredTarget" if trigger == Some("DamageDone") => "PlayerRel::DamagedPlayer",
            "TriggeredTargetController" if trigger == Some("DamageDone") => {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredDefendingPlayer"
                if matches!(trigger, Some("Attacks" | "AttackerBlocked")) =>
            {
                "PlayerRel::DefendingPlayer"
            }
            _ => return None,
        })
    }

    /// The same as [`Self::player_rel`], for an effect on a line that may
    /// *target a player itself*.
    ///
    /// The corpus leaves `Defined$` off when the effect means its own line's
    /// target. Reading the absent key as `You` there is how Piranha Marsh —
    /// "target player loses 1 life" — generated as a land that drains its
    /// own controller.
    ///
    /// Its **own line's**, and never the chain's: a sub-ability inherits no
    /// target from the line in front of it, and says `Defined$ Targeted`
    /// when it means one. Last Caress is the card that proved it — "target
    /// player loses 1 life and you gain 1 life. Draw a card." is a targeting
    /// `LoseLife` followed by a bare `GainLife` and a bare `Draw`, and read
    /// against the chain it generated as a sorcery whose target gained the
    /// life and drew the card while its caster got neither.
    pub(super) fn player_rel_of(
        &self,
        defined: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        if defined.is_none() && targets_a_player {
            return Some("PlayerRel::Chosen");
        }
        self.player_rel(defined)
    }

    /// [`Self::player_rel_of`], and the two `Defined$` words that name the
    /// chain's target: `Targeted` is the player it targeted (`Chosen`),
    /// `TargetedController` the controller of the object or spell it
    /// targeted (`ControllerOfTarget`, last known, CR 608.2h). Each is read
    /// only against a chain whose target is that kind of thing; a word that
    /// names a target the chain does not have is refused.
    pub(super) fn player_of_line(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        let player_target = target == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let object_target = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        match defined {
            Some("Targeted" | "TargetedPlayer") if player_target => Some("PlayerRel::Chosen"),
            Some("TargetedController") if object_target => Some("PlayerRel::ControllerOfTarget"),
            Some("Targeted" | "TargetedPlayer" | "TargetedController") => None,
            other => self.player_rel_of(other, targets_a_player),
        }
    }
}
