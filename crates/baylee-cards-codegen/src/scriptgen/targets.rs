//! Reading what a script points at: filters, targets, players, and the
//! sources a prevention names.

use super::{Params, Tx, color_atom, counter_kind, named_atom};

/// CR 700.12: Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.
const OUTLAW_FILTER: &str = "Filter::Or(&[\
    Filter::HasSubtype(subtypes::creature::ASSASSIN), \
    Filter::HasSubtype(subtypes::creature::MERCENARY), \
    Filter::HasSubtype(subtypes::creature::PIRATE), \
    Filter::HasSubtype(subtypes::creature::ROGUE), \
    Filter::HasSubtype(subtypes::creature::WARLOCK)])";

/// CR 305.6: The five basic land types.
const BASIC_LAND_TYPE_FILTER: &str = "Filter::Or(&[\
    Filter::HasSubtype(subtypes::land::PLAINS), \
    Filter::HasSubtype(subtypes::land::ISLAND), \
    Filter::HasSubtype(subtypes::land::SWAMP), \
    Filter::HasSubtype(subtypes::land::MOUNTAIN), \
    Filter::HasSubtype(subtypes::land::FOREST)])";

/// All five Magic colors (CR 105.1: "all colors", e.g. Iridian Maelstrom).
const ALL_COLORS_FILTER: &str = "Filter::And(&[\
    Filter::HasColor(ColorSet::from_slice(&[Color::White])), \
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])), \
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])), \
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])), \
    Filter::HasColor(ColorSet::from_slice(&[Color::Green]))])";

impl Tx<'_> {
    /// Whether this permanent chooses a card type as it enters (`EnterModifier::ChooseCardType`).
    pub(super) fn has_chosen_card_type(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ChooseCardType")
    }

    /// Whether this permanent chooses a color as it enters (`EnterModifier::ChooseColor`).
    pub(super) fn has_chosen_color(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ChooseColor")
    }

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
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("`ChooseSource` with no `Choices$`".to_string());
        };
        if choices == "Card.SharesColorWith Imprinted" {
            return self.prevent_from_exiled_color(p, sub);
        }
        let remember_chosen = p.take("RememberChosen").as_deref() == Some("True");
        if remember_chosen {
            p.take("ChoiceTitle");
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseSource.{key}`"));
        }
        let (filter, rechecks) = self.chosen_source_choices(&choices, remember_chosen)?;
        let (effect, replacement) = self.read_chosen_source_effect(sub, remember_chosen)?;
        if let Some(key) = effect.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        if choices == "Card,Emblem"
            && !remember_chosen
            && self.all_damage_from_chosen_source(&replacement)
        {
            return Some(vec![
                "Effect::ChooseDamageSourcePreventAllThisTurn".to_string(),
            ]);
        }
        let (target_you, next_only, combat_only, gain_life) =
            self.chosen_source_replacement(&replacement, &rechecks)?;
        // Preserves `chosen_source_all_damage_shield_is_repeatable_and_not_recipient_limited`:
        // unremembered `Choices$ Card,Emblem` with `Prevent$ True` and `ValidTarget$ You`
        // remains refused on this path until that test is retired in a follow-up.
        if choices == "Card,Emblem" && !remember_chosen && target_you && !next_only {
            return self.deny("a chosen-source replacement that is not a shield on you".into());
        }
        let sources = self.body.filter_static("SOURCE", &filter);
        Some(vec![if target_you && next_only {
            format!(
                "Effect::PreventNextFromChosenSource {{ sources: &{sources}, \
                 combat_only: {combat_only}, all_but: 0, gain_life: {gain_life} }}"
            )
        } else {
            let target = if target_you {
                "Some(TargetSpec::Player(PlayerRel::You))"
            } else {
                "None"
            };
            format!(
                "Effect::PreventFromChosenSource {{ target: {target}, sources: &{sources}, \
                 next_only: {next_only}, combat_only: {combat_only}, all_but: 0, \
                 gain_life: {gain_life} }}"
            )
        }])
    }

    /// "Target permanent becomes the color of your choice until end of turn"
    /// (Distorting Lens, Alchor's Tomb): `ChooseColor` followed by `DB$ Animate
    /// | ValidTgts$ <valid> | Colors$ ChosenColor | OverwriteColors$ True`; or
    /// turn-long / next-damage color-based prevention (Prismatic Strands,
    /// Avacyn, Guardian Angel): `ChooseColor` followed by `DB$ Effect |
    /// ReplacementEffects$ <RPrevent>`.
    pub(super) fn choose_color_animate(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("You") {
            return self.deny("`ChooseColor` for somebody other than `You`".to_string());
        }
        let max_colors = match p.take("OrColors").as_deref() {
            None | Some("False") => 1u8,
            Some("True") => 5u8,
            Some(other) => {
                return self.deny(format!("`ChooseColor` with `OrColors$ {other}`"));
            }
        };
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
        if api == "Effect" {
            if max_colors != 1 {
                return self.deny("`ChooseColor` `Effect` with `OrColors$`".into());
            }
            return self.choose_color_prevention(animate, chain);
        }
        if api != "Animate" {
            return self.deny("`ChooseColor` followed by something other than `Animate`".into());
        }
        animate.drop_prose();
        let subject = if let Some(valid) = animate.take("ValidTgts") {
            if chain.target.is_some() {
                return self.deny("`ChooseColor` `Animate` with a second target".to_string());
            }
            let scoped = if valid == "Creature"
                && oracle_scopes_combat_target_to_you(
                    &self.oracle.to_ascii_lowercase(),
                    "target creature",
                ) {
                "Creature.YouCtrl"
            } else {
                valid.as_str()
            };
            chain.target = Some(self.target_spec(scoped, "Animate")?);
            "ColorSubject::Target"
        } else {
            match animate.take("Defined").as_deref() {
                Some("Self") => "ColorSubject::Source",
                Some("Enchanted" | "Equipped") => "ColorSubject::Attached",
                _ => return self.deny("`ChooseColor` `Animate` with no `ValidTgts$`".to_string()),
            }
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
        let trailing_sub = animate.take("SubAbility");
        if let Some(key) = animate.first_key() {
            return self.deny(format!("unclaimed parameter `Animate.{key}`"));
        }
        let choose_fx = format!(
            "Effect::ChooseColors {{ subject: {subject}, max: {max_colors}, duration: {duration} }}"
        );
        if trailing_sub.is_some() {
            chain.effects.push(choose_fx);
            self.chain_sub_ability(trailing_sub, chain)?;
            return Some(Vec::new());
        }
        Some(vec![choose_fx])
    }

    /// "Prevent all damage that sources of the color of your choice would deal
    /// [to target ...] this turn" (Prismatic Strands, Avacyn, Guardian Angel).
    fn choose_color_prevention(
        &mut self,
        mut effect: Params,
        chain: &mut super::Chain,
    ) -> Option<Vec<String>> {
        effect.drop_prose();
        let Some(replacement) = effect.take("ReplacementEffects") else {
            return self.deny("`ChooseColor` `Effect` with no `ReplacementEffects$`".into());
        };
        // Display label on multi-ability `Effect` blocks (e.g., Avacyn).
        effect.take("Name");
        let (target_spec, expected_target) =
            self.bind_effect_prevention_target(&mut effect, chain, "`ChooseColor` `Effect`")?;
        if let Some(key) = effect.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let Some((event, mut shield)) = self.svars.get(&replacement).and_then(|b| Params::parse(b))
        else {
            return self.deny(format!("replacement `{replacement}` names no SVar"));
        };
        shield.drop_prose();
        if event != "DamageDone"
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
            || shield.take("ValidSource").as_deref() != Some("Card.ChosenColor")
        {
            return self.deny("a chosen-color prevention with another event or source".into());
        }
        let valid_target = shield.take("ValidTarget");
        let target_matches = match (expected_target, valid_target.as_deref()) {
            (None, None) => true,
            (Some(expected), Some(actual)) => expected.contains(&actual),
            _ => false,
        };
        if !target_matches {
            return self.deny("a chosen-color prevention with a mismatched target".into());
        }
        let combat_only = match shield.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!(
                    "a chosen-color prevention with `IsCombat$ {other}`"
                ));
            }
        };
        let next_only = if shield.take("Prevent").as_deref() == Some("True") {
            false
        } else if shield.take("PreventionEffect").as_deref() == Some("True")
            && shield
                .take("ReplaceWith")
                .is_some_and(|rw| self.exiles_itself(&rw))
        {
            true
        } else {
            return self.deny("a chosen-color prevention with another replacement".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let target = target_spec.map_or_else(|| "None".to_string(), |s| format!("Some({s})"));
        let variant = if next_only {
            "Effect::PreventNextFromMatchingSources"
        } else {
            "Effect::PreventAllDamageFromMatchingThisTurn"
        };
        Some(vec![
            "Effect::ChooseColor".to_string(),
            format!(
                "{variant} {{ target: {target}, sources: &Filter::OfSourceChosenColor, \
                 combat_only: {combat_only} }}"
            ),
        ])
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
    /// chosen source (`rechecks`, its properties spelled as the choice spelled
    /// them) to you or everything, prevented (all turn with `Prevent$ True`, or
    /// next-only when exiling the shield or gaining life equal to the damage
    /// prevented this way). Returns `(target_you, next_only, combat_only, gain_life)`.
    pub(super) fn chosen_source_replacement(
        &self,
        replacement: &str,
        rechecks: &[String],
    ) -> Option<(bool, bool, bool, bool)> {
        let Some((event, mut shield)) = self.svars.get(replacement).and_then(|b| Params::parse(b))
        else {
            return self.deny(format!("replacement `{replacement}` names no SVar"));
        };
        shield.drop_prose();
        if event != "DamageDone"
            || !matches!(
                shield.take("ActiveZones").as_deref(),
                None | Some("Command")
            )
            || !shield
                .take("ValidSource")
                .is_some_and(|src| rechecks.iter().any(|r| r == &src))
        {
            return self.deny("a chosen-source replacement that is not a shield on you".into());
        }
        let target_you = match shield.take("ValidTarget").as_deref() {
            Some("You") => true,
            None => false,
            Some(_) => {
                return self.deny("a chosen-source replacement that is not a shield on you".into());
            }
        };
        let combat_only = match shield.take("IsCombat").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("a chosen-source shield with `IsCombat$ {other}`"));
            }
        };
        if shield.take("Prevent").as_deref() == Some("True") {
            if let Some(key) = shield.first_key() {
                return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
            }
            return Some((target_you, false, combat_only, false));
        }
        let waits = shield.take("PreventionEffect").as_deref() == Some("True");
        let Some(then) = shield.take("ReplaceWith").filter(|_| waits) else {
            return self.deny("a chosen-source replacement that is not a shield on you".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        let gain_life = match self.svars.get(&then).and_then(|b| Params::parse(b)) {
            Some((api, mut gain)) if api == "GainLife" => {
                let reads = gain.take("Defined").as_deref() == Some("You")
                    && gain.take("LifeAmount").is_some_and(|x| {
                        self.svars.get(&x).map(String::as_str) == Some("ReplaceCount$DamageAmount")
                    });
                let exile = gain.take("SubAbility");
                if !reads || !gain.exhausted() || !exile.is_some_and(|e| self.exiles_itself(&e)) {
                    return self.deny("a chosen-source shield's life gain".to_string());
                }
                true
            }
            _ if self.exiles_itself(&then) => false,
            _ => return self.deny(format!("`ReplaceWith$ {then}` on a chosen-source shield")),
        };
        Some((target_you, true, combat_only, gain_life))
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

    fn push_filter_base(
        &self,
        base: &str,
        card_zone: bool,
        clauses: &mut Vec<String>,
        state: &mut FilterAltState,
    ) -> bool {
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
            "Battle" => clauses.push("Filter::HasType(TypeSet::BATTLE)".to_string()),
            "Instant" => clauses.push("Filter::HasType(TypeSet::INSTANT)".to_string()),
            "Sorcery" => clauses.push("Filter::HasType(TypeSet::SORCERY)".to_string()),
            "Spell" => clauses.push("Filter::InZone(ZoneRef::Stack)".to_string()),
            "Basic" => clauses.push("Filter::HasSupertype(SupertypeSet::BASIC)".to_string()),
            "Legendary" => {
                clauses.push("Filter::HasSupertype(SupertypeSet::LEGENDARY)".to_string());
            }
            "Snow" => clauses.push("Filter::HasSupertype(SupertypeSet::SNOW)".to_string()),
            "Self" => clauses.push("Filter::This".to_string()),
            "Enchanted" | "Equipped" => clauses.push("Filter::AttachedToBySource".to_string()),
            "Kicked" => clauses.push("Filter::Kicked".to_string()),
            "Historic" => clauses.push("Filter::Historic".to_string()),
            "Outlaw" => clauses.push(OUTLAW_FILTER.to_string()),
            "Ringbearer" | "RingBearer" => clauses.push("Filter::RingBearer".to_string()),
            _ => {
                if let Some(path) = self.cats.const_path(base) {
                    clauses.push(format!("Filter::HasSubtype({path})"));
                    return true;
                }
                if !base.is_empty() && base != "lowestCMC" {
                    match self.push_filter_atom(base, card_zone, clauses, state) {
                        AtomParse::Matched => return true,
                        AtomParse::Failed | AtomParse::LowestCmc => return false,
                        AtomParse::Unknown => {}
                    }
                }
                self.note(format!("filter base `{base}`"));
                return false;
            }
        }
        true
    }

    fn source_power_var_atom(&self, atom: &str) -> Option<&'static str> {
        let (filter, var) = atom
            .strip_prefix("powerLT")
            .map(|v| ("Filter::PowerLessThanSourcePower", v))
            .or_else(|| {
                atom.strip_prefix("powerLE")
                    .map(|v| ("Filter::PowerAtMostSourcePower", v))
            })
            .or_else(|| {
                atom.strip_prefix("toughnessLT")
                    .map(|v| ("Filter::ToughnessLessThanSourcePower", v))
            })?;
        let var = var.trim();
        if var.is_empty() || !var.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        let def = self.svars.get(var).map(|s| s.trim())?;
        let matches_source_power =
            def == "Count$CardPower" || (self.trigger_is_self && def == "TriggeredCard$CardPower");
        matches_source_power.then_some(filter)
    }

    #[allow(clippy::too_many_lines)]
    fn push_filter_atom(
        &self,
        atom: &str,
        card_zone: bool,
        clauses: &mut Vec<String>,
        state: &mut FilterAltState,
    ) -> AtomParse {
        let clause = match atom {
            // "Enchanted creature gets +2/+1", "equipped creature has
            // flying": one filter for both, because there is one
            // question — what is this permanent attached to — and the
            // corpus asks it in the two words the two card types
            // print. An Aura and an Equipment differ in what happens
            // when the answer is nothing (CR 704.5m against
            // 704.5n–p), which is the state-based actions' business
            // and not this clause's.
            "EnchantedBy" | "EquippedBy" | "AttachedBy" => "Filter::AttachedToBySource".to_string(),
            "!EnchantedBy" | "!EquippedBy" | "!AttachedBy" => {
                "Filter::Not(&Filter::AttachedToBySource)".to_string()
            }
            "Attached" => "Filter::AttachedToSource".to_string(),
            "PairedWith" if !card_zone => "Filter::PairedWithSource".to_string(),
            "equipped" => "Filter::Equipped".to_string(),
            "!equipped" => "Filter::Not(&Filter::Equipped)".to_string(),
            "YouCtrl" => {
                state.has_controller = true;
                "Filter::ControlledByYou".to_string()
            }
            "YouDontCtrl" => {
                state.has_controller = true;
                "Filter::Not(&Filter::ControlledByYou)".to_string()
            }
            "OppCtrl" => {
                state.has_controller = true;
                "Filter::ControlledByOpponent".to_string()
            }
            "ActivePlayerCtrl" => {
                state.has_controller = true;
                "Filter::ControlledByActivePlayer".to_string()
            }
            "DefenderCtrl" | "DefendingPlayerCtrl" if !card_zone => {
                state.has_controller = true;
                "Filter::ControlledByDefendingPlayer".to_string()
            }
            "EnchantedPlayerCtrl"
            | "ControlledBy EnchantedPlayer"
            | "ControlledBy Player.EnchantedBy" => {
                state.has_controller = true;
                "Filter::ControlledByEnchantedPlayer".to_string()
            }
            "ControlledBy TriggeredPlayer" if self.trigger_mode.as_deref() == Some("Phase") => {
                state.has_controller = true;
                "Filter::ControlledByActivePlayer".to_string()
            }
            "ControlledBy TriggeredDefendingPlayer"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                ) =>
            {
                state.has_controller = true;
                "Filter::ControlledByDefendingPlayer".to_string()
            }
            "IsCommander" => {
                state.has_is_commander = true;
                "Filter::IsCommander".to_string()
            }
            "!IsCommander" | "nonCommander" => "Filter::Not(&Filter::IsCommander)".to_string(),
            "IsRingbearer" | "IsRingBearer" => "Filter::RingBearer".to_string(),
            "!IsRingbearer" | "!IsRingBearer" | "nonRingbearer" | "nonRingBearer" => {
                "Filter::Not(&Filter::RingBearer)".to_string()
            }
            "NamedCard" => "Filter::NamedBySource".to_string(),
            "wasDealtDamageThisTurn" => "Filter::DealtDamageThisTurn".to_string(),
            "!wasDealtDamageThisTurn" => "Filter::Not(&Filter::DealtDamageThisTurn)".to_string(),
            "Damaged" if !card_zone => "Filter::DealtDamageToSourceThisTurn".to_string(),
            "attackedThisTurn" => "Filter::AttackedThisTurn".to_string(),
            "!attackedThisTurn" => "Filter::Not(&Filter::AttackedThisTurn)".to_string(),
            "blockedThisTurn" => "Filter::BlockedThisTurn".to_string(),
            "!blockedThisTurn" => "Filter::Not(&Filter::BlockedThisTurn)".to_string(),
            "wasCastFromHand" | "CastFromHand" => "Filter::CastFromHand".to_string(),
            "!wasCastFromHand" => "Filter::Not(&Filter::CastFromHand)".to_string(),
            "wasCastFromYourHand"
                if self.on_a_spell || self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                "Filter::CastFromHand".to_string()
            }
            "!wasCastFromYourHand"
                if self.on_a_spell || self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                "Filter::Not(&Filter::CastFromHand)".to_string()
            }
            "wasCastFromTheirHand" if self.trigger_mode.as_deref() == Some("SpellCast") => {
                "Filter::CastFromHand".to_string()
            }
            "!wasCastFromTheirHand" if self.trigger_mode.as_deref() == Some("SpellCast") => {
                "Filter::Not(&Filter::CastFromHand)".to_string()
            }
            "wasCastFromYourHandByYou" if self.trigger_mode.as_deref() == Some("SpellCast") => {
                state.has_controller = true;
                "Filter::And(&[Filter::CastFromHand, Filter::ControlledByYou])".to_string()
            }
            "counters_GE1_ALL" | "counters_GT0_ALL" | "HasCounters" => {
                "Filter::HasAnyCounter".to_string()
            }
            "counters_EQ0_ALL" | "counters_LE0_ALL" | "counters_LT1_ALL" | "!HasCounters" => {
                "Filter::Not(&Filter::HasAnyCounter)".to_string()
            }
            "inZoneBattlefield" => "Filter::InZone(ZoneRef::Battlefield)".to_string(),
            "inZoneStack" => "Filter::InZone(ZoneRef::Stack)".to_string(),
            "Historic" => "Filter::Historic".to_string(),
            "nonHistoric" | "!Historic" => "Filter::Not(&Filter::Historic)".to_string(),
            "Outlaw" => OUTLAW_FILTER.to_string(),
            "nonOutlaw" | "!Outlaw" => format!("Filter::Not(&{OUTLAW_FILTER})"),
            "hasABasicLandType" => BASIC_LAND_TYPE_FILTER.to_string(),
            "!hasABasicLandType" | "nonBasicLandType" => {
                format!("Filter::Not(&{BASIC_LAND_TYPE_FILTER})")
            }
            "AllColors" => ALL_COLORS_FILTER.to_string(),
            "!AllColors" | "nonAllColors" => format!("Filter::Not(&{ALL_COLORS_FILTER})"),
            "setATQ" => {
                "Filter::NameOriginallyPrintedIn(OriginalExpansion::Antiquities)".to_string()
            }
            "setARN" => {
                "Filter::NameOriginallyPrintedIn(OriginalExpansion::ArabianNights)".to_string()
            }
            "ControllerControlsPlains" if !card_zone => {
                "Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::PLAINS))"
                    .to_string()
            }
            "ControllerControlsIsland" if !card_zone => {
                "Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::ISLAND))"
                    .to_string()
            }
            "ControllerControlsSwamp" if !card_zone => {
                "Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::SWAMP))".to_string()
            }
            "ControllerControlsMountain" if !card_zone => {
                "Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::MOUNTAIN))"
                    .to_string()
            }
            "ControllerControlsForest" if !card_zone => {
                "Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::FOREST))"
                    .to_string()
            }
            "lowestCMC" if !card_zone => {
                return AtomParse::LowestCmc;
            }
            "AdventureCard" => "Filter::HasAdventure".to_string(),
            "!AdventureCard" | "nonAdventureCard" => {
                "Filter::Not(&Filter::HasAdventure)".to_string()
            }
            "IsSingleTarget" => "Filter::WithSingleTarget".to_string(),
            "!IsSingleTarget" => "Filter::Not(&Filter::WithSingleTarget)".to_string(),
            "withCumulative upkeep" => "Filter::HasCumulativeUpkeep".to_string(),
            "withoutCumulative upkeep" | "!withCumulative upkeep" => {
                "Filter::Not(&Filter::HasCumulativeUpkeep)".to_string()
            }
            "Permanent" if !card_zone => return AtomParse::Matched,
            "Permanent" => "Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::BATTLE)\
                 .union(TypeSet::CREATURE).union(TypeSet::ENCHANTMENT)\
                 .union(TypeSet::LAND).union(TypeSet::PLANESWALKER))"
                .to_string(),
            "nonPermanent" | "!Permanent" => {
                "Filter::HasType(TypeSet::INSTANT.union(TypeSet::SORCERY))".to_string()
            }
            "YouOwn" => {
                state.has_you_own = true;
                "Filter::OwnedByYou".to_string()
            }
            "YouDontOwn" => "Filter::Not(&Filter::OwnedByYou)".to_string(),
            "Other" | "StrictlyOther" => "Filter::Another".to_string(),
            "Self" | "StrictlySelf" => "Filter::This".to_string(),
            "attacking" => "Filter::Attacking".to_string(),
            "!attacking" => "Filter::Not(&Filter::Attacking)".to_string(),
            "attackingYou" => "Filter::AttackingYou".to_string(),
            "attackersBandedWith" => "Filter::BandedWithSource".to_string(),
            "blocking" => "Filter::Blocking".to_string(),
            "!blocking" => "Filter::Not(&Filter::Blocking)".to_string(),
            "blockingSource" | "blockingSelf" => "Filter::BlockingSource".to_string(),
            "blockedBySource" => "Filter::BlockedBySource".to_string(),
            "blockingTriggeredAttacker"
                if self.trigger_mode.as_deref() == Some("AttackerBlocked") =>
            {
                if self.trigger_is_self {
                    "Filter::BlockingSource".to_string()
                } else {
                    "Filter::BlockingReference".to_string()
                }
            }
            "blocked" => "Filter::Blocked".to_string(),
            "!blocked" => "Filter::Not(&Filter::Blocked)".to_string(),
            "unblocked" => "Filter::Unblocked".to_string(),
            "!unblocked" => "Filter::Not(&Filter::Unblocked)".to_string(),
            "tapped" | "!untapped" => "Filter::Tapped".to_string(),
            "untapped" | "!tapped" => "Filter::Untapped".to_string(),
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
            "nonLand" | "!Land" => "Filter::NONLAND".to_string(),
            "Land" => "Filter::LAND".to_string(),
            "Artifact" => "Filter::ARTIFACT".to_string(),
            "Creature" => "Filter::CREATURE".to_string(),
            "Enchantment" => "Filter::ENCHANTMENT".to_string(),
            "Planeswalker" => "Filter::PLANESWALKER".to_string(),
            "Battle" => "Filter::HasType(TypeSet::BATTLE)".to_string(),
            "Instant" => "Filter::INSTANT".to_string(),
            "Sorcery" => "Filter::SORCERY".to_string(),
            "nonCreature" | "!Creature" => "Filter::NONCREATURE".to_string(),
            // Supertypes read like subtypes in a script filter but are
            // a different set on the card (CR 205.4).
            "Basic" => "Filter::HasSupertype(SupertypeSet::BASIC)".to_string(),
            "nonBasic" | "!Basic" => {
                "Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC))".to_string()
            }
            "Legendary" => "Filter::HasSupertype(SupertypeSet::LEGENDARY)".to_string(),
            "nonLegendary" | "!Legendary" => {
                "Filter::Not(&Filter::HasSupertype(SupertypeSet::LEGENDARY))".to_string()
            }
            "Snow" => "Filter::HasSupertype(SupertypeSet::SNOW)".to_string(),
            "nonSnow" | "!Snow" => {
                "Filter::Not(&Filter::HasSupertype(SupertypeSet::SNOW))".to_string()
            }
            "WarpedInExile" | "ExiledWithWarp" => "Filter::WarpedInExile".to_string(),
            "!WarpedInExile" | "!ExiledWithWarp" => {
                "Filter::Not(&Filter::WarpedInExile)".to_string()
            }
            "IsMonstrous" | "monstrous" => "Filter::Monstrous".to_string(),
            "!IsMonstrous" | "!monstrous" | "nonMonstrous" => {
                "Filter::Not(&Filter::Monstrous)".to_string()
            }
            "IsSuspected" | "suspected" => "Filter::Suspected".to_string(),
            "!IsSuspected" | "!suspected" | "nonSuspected" => {
                "Filter::Not(&Filter::Suspected)".to_string()
            }
            // Card-type negations a noun cannot say "not" to on its
            // own ("nonartifact, nonblack creature").
            "nonArtifact" | "!Artifact" => "Filter::LacksType(TypeSet::ARTIFACT)".to_string(),
            "nonEnchantment" | "!Enchantment" => {
                "Filter::LacksType(TypeSet::ENCHANTMENT)".to_string()
            }
            "nonPlaneswalker" | "!Planeswalker" => {
                "Filter::LacksType(TypeSet::PLANESWALKER)".to_string()
            }
            "nonBattle" | "!Battle" => "Filter::LacksType(TypeSet::BATTLE)".to_string(),
            "nonInstant" | "!Instant" => "Filter::LacksType(TypeSet::INSTANT)".to_string(),
            "nonSorcery" | "!Sorcery" => "Filter::LacksType(TypeSet::SORCERY)".to_string(),
            "Colorless" => "Filter::IsColorless".to_string(),
            "ColorlessSource" => {
                state.source_color = Some("colorless");
                "Filter::IsColorless".to_string()
            }
            "nonColorless" | "!Colorless" | "nonColorlessSource" | "!ColorlessSource"
            | "Colored" => "Filter::Not(&Filter::IsColorless)".to_string(),
            "MonoColor" => "Filter::Monocolored".to_string(),
            "!MonoColor" | "nonMonoColor" => "Filter::Not(&Filter::Monocolored)".to_string(),
            "MultiColor" => "Filter::And(&[Filter::Not(&Filter::IsColorless), \
                 Filter::Not(&Filter::Monocolored)])"
                .to_string(),
            "nonMultiColor" | "!MultiColor" => {
                "Filter::Or(&[Filter::IsColorless, Filter::Monocolored])".to_string()
            }
            "ChosenColor" => "Filter::OfSourceChosenColor".to_string(),
            "!ChosenColor" | "nonChosenColor" => {
                "Filter::Not(&Filter::OfSourceChosenColor)".to_string()
            }
            "ChosenType" if self.has_chosen_creature_type() => {
                "Filter::MatchesChosenTypeOfSource".to_string()
            }
            "ChosenType" if self.has_chosen_card_type() => {
                "Filter::OfSourceChosenCardType".to_string()
            }
            "IsNotChosenType" | "!ChosenType" | "nonChosenType"
                if self.has_chosen_creature_type() =>
            {
                "Filter::Not(&Filter::MatchesChosenTypeOfSource)".to_string()
            }
            "IsNotChosenType" | "!ChosenType" | "nonChosenType" if self.has_chosen_card_type() => {
                "Filter::Not(&Filter::OfSourceChosenCardType)".to_string()
            }
            "kicked" | "Kicked" => "Filter::Kicked".to_string(),
            "!kicked" => "Filter::Not(&Filter::Kicked)".to_string(),
            "faceDown" => "Filter::FaceDown".to_string(),
            "faceUp" => "Filter::Not(&Filter::FaceDown)".to_string(),
            "hasManaAbility" => "Filter::HasManaAbility".to_string(),
            "!hasManaAbility" => "Filter::Not(&Filter::HasManaAbility)".to_string(),
            "withRampage" => "Filter::HasRampage".to_string(),
            "!withRampage" => "Filter::Not(&Filter::HasRampage)".to_string(),
            "ThisTurnEntered" if !card_zone => "Filter::EnteredThisTurn".to_string(),
            "!ThisTurnEntered" if !card_zone => "Filter::Not(&Filter::EnteredThisTurn)".to_string(),
            "PutIntoGraveyardThisTurn" if card_zone => {
                "Filter::PutIntoGraveyardThisTurn".to_string()
            }
            "!PutIntoGraveyardThisTurn" if card_zone => {
                "Filter::Not(&Filter::PutIntoGraveyardThisTurn)".to_string()
            }
            "!firstTurnControlled" if !card_zone => "Filter::ControlledSinceTurnBegan".to_string(),
            "firstTurnControlled" if !card_zone => {
                "Filter::Not(&Filter::ControlledSinceTurnBegan)".to_string()
            }
            "sharesNameWith ExiledWith" | "sharesNameWith Imprinted" => {
                "Filter::SameNameAsLinkedExile".to_string()
            }
            "!sharesNameWith ExiledWith" | "!sharesNameWith Imprinted" => {
                "Filter::Not(&Filter::SameNameAsLinkedExile)".to_string()
            }
            "sharesCreatureTypeWith" | "sharesCreatureTypeWith Self" => {
                "Filter::SharesCreatureTypeWith(&Filter::This)".to_string()
            }
            "sharesCreatureTypeWith Equipped"
            | "sharesCreatureTypeWith Enchanted"
            | "sharesCreatureTypeWith AttachedBy" => {
                "Filter::SharesCreatureTypeWith(&Filter::AttachedToBySource)".to_string()
            }
            "sharesCreatureTypeWith Commander" => "Filter::SharesSubtypeWithCommander".to_string(),
            "SharesColorWith Equipped" | "SharesColorWith Enchanted" => {
                "Filter::SharesAColorWith(&Filter::AttachedToBySource)".to_string()
            }
            "!SharesColorWith Equipped" | "!SharesColorWith Enchanted" => {
                "Filter::Not(&Filter::SharesAColorWith(&Filter::AttachedToBySource))".to_string()
            }
            "SharesColorWith" | "SharesColorWith Self" => {
                "Filter::SharesColorWithSource".to_string()
            }
            "!SharesColorWith" | "!SharesColorWith Self" => {
                "Filter::Not(&Filter::SharesColorWithSource)".to_string()
            }
            "SharesCMCWith" | "SharesCMCWith Self" | "cmcEQSelf" | "SameCMCAsSelf" => {
                "Filter::SameManaValueAsSource".to_string()
            }
            "!SharesCMCWith" | "!SharesCMCWith Self" | "cmcNESelf" | "!SameCMCAsSelf" => {
                "Filter::Not(&Filter::SameManaValueAsSource)".to_string()
            }
            other
                if other
                    .strip_prefix("cmcEQ")
                    .or_else(|| other.strip_prefix("cmcNE"))
                    .and_then(|var| self.svars.get(var.trim()))
                    .is_some_and(|def| def.trim() == "Count$CardManaCost") =>
            {
                if other.starts_with("cmcNE") {
                    "Filter::Not(&Filter::SameManaValueAsSource)".to_string()
                } else {
                    "Filter::SameManaValueAsSource".to_string()
                }
            }
            other
                if self.on_a_spell
                    && other
                        .strip_prefix("cmcLE")
                        .and_then(|var| self.svars.get(var.trim()))
                        .is_some_and(|def| {
                            matches!(
                                def.trim(),
                                "Count$Converge"
                                    | "Count$ColorsOfManaSpentToCast"
                                    | "Count$ColorsSpent"
                            )
                        }) =>
            {
                "Filter::CmcAtMostColorsSpent".to_string()
            }
            other
                if !self.in_delayed
                    && (self.on_a_spell || self.trigger_is_self || self.trigger_mode.is_none())
                    && other
                        .strip_prefix("cmcLE")
                        .and_then(|var| self.svars.get(var.trim()))
                        .is_some_and(|def| def.trim() == "Count$CastSnowManaSpent") =>
            {
                "Filter::CmcAtMostSnowManaSpent".to_string()
            }
            other
                if (self.has_x
                    || (self.is_etb_self_trigger && self.has_x_mana_cost && !self.in_delayed))
                    && other
                        .strip_prefix("cmcLE")
                        .or_else(|| other.strip_prefix("cmcEQ"))
                        .and_then(|var| {
                            let var = var.trim();
                            self.svars.get(var).map(|def| (var, def.trim()))
                        })
                        .is_some_and(|(var, def)| {
                            def == "Count$xPaid" || (var == "X" && def == "Targeted$CardManaCost")
                        }) =>
            {
                if other.starts_with("cmcEQ") {
                    "Filter::CmcExactlyX".to_string()
                } else {
                    "Filter::CmcAtMostX".to_string()
                }
            }
            other if other.starts_with("AttachedTo ") => {
                let inner_raw = other.trim_start_matches("AttachedTo ").trim();
                if inner_raw == "Card.Self" || inner_raw == "Self" {
                    "Filter::AttachedToSource".to_string()
                } else if inner_raw.contains("AttachedTo")
                    || inner_raw.contains("SharesColorWith")
                    || inner_raw.contains("sharesCreatureTypeWith")
                    || inner_raw.contains("IsRemembered")
                    || inner_raw.contains("Targeted")
                    || inner_raw.contains("Triggered")
                    || matches!(inner_raw, "You" | "Player" | "Opponent")
                    || inner_raw.starts_with("Player.")
                {
                    self.note(format!("filter atom `{other}`"));
                    return AtomParse::Failed;
                } else {
                    let Some(inner) = self.filter_expr(inner_raw) else {
                        return AtomParse::Failed;
                    };
                    format!("Filter::AttachedTo(&{inner})")
                }
            }
            other
                if other.starts_with("SharesColorWith Valid ")
                    || other.starts_with("!SharesColorWith Valid ") =>
            {
                let (negated, inner_raw) =
                    other.strip_prefix("!SharesColorWith Valid ").map_or_else(
                        || (false, other.trim_start_matches("SharesColorWith Valid ")),
                        |r| (true, r),
                    );
                let inner_raw = inner_raw.trim();
                if inner_raw.contains("SharesColorWith")
                    || inner_raw.contains("sharesCreatureTypeWith")
                    || inner_raw.contains("AttachedTo")
                {
                    self.note(format!("filter atom `{other}`"));
                    return AtomParse::Failed;
                }
                let Some(inner) = self.filter_expr(inner_raw) else {
                    return AtomParse::Failed;
                };
                let pos = format!("Filter::SharesAColorWith(&{inner})");
                if negated {
                    format!("Filter::Not(&{pos})")
                } else {
                    pos
                }
            }
            other
                if other.starts_with("sharesCreatureTypeWith Valid ")
                    || other.starts_with("!sharesCreatureTypeWith Valid ")
                    || other.starts_with("SharesCreatureTypeWith Valid ")
                    || other.starts_with("!SharesCreatureTypeWith Valid ") =>
            {
                let (negated, inner_raw) = if let Some(r) = other
                    .strip_prefix("!sharesCreatureTypeWith Valid ")
                    .or_else(|| other.strip_prefix("!SharesCreatureTypeWith Valid "))
                {
                    (true, r)
                } else if let Some(r) = other
                    .strip_prefix("sharesCreatureTypeWith Valid ")
                    .or_else(|| other.strip_prefix("SharesCreatureTypeWith Valid "))
                {
                    (false, r)
                } else {
                    return AtomParse::Unknown;
                };
                let inner_raw = inner_raw.trim();
                if inner_raw.contains("sharesCreatureTypeWith")
                    || inner_raw.contains("SharesCreatureTypeWith")
                    || inner_raw.contains("SharesColorWith")
                    || inner_raw.contains("AttachedTo")
                {
                    self.note(format!("filter atom `{other}`"));
                    return AtomParse::Failed;
                }
                let Some(inner) = self.filter_expr(inner_raw) else {
                    return AtomParse::Failed;
                };
                let pos = format!("Filter::SharesCreatureTypeWith(&{inner})");
                if negated {
                    format!("Filter::Not(&{pos})")
                } else {
                    pos
                }
            }
            "" => return AtomParse::Matched,
            other if other.starts_with("counters_") => {
                let Some(expr) = counter_atom_filter(other) else {
                    self.note(format!("filter atom `{other}`"));
                    return AtomParse::Failed;
                };
                match expr {
                    CounterAtom::Always => return AtomParse::Matched,
                    CounterAtom::Filter(clause) => clause,
                }
            }
            // A colour word (CR 105.2): "black creatures", "target
            // green spell", "nonblack creature". A colour is not a
            // subtype, so it is asked before the subtype arm below,
            // which would otherwise refuse `Black` as an unknown type.
            other if color_atom(other).is_some() => {
                let (negated, color) = color_atom(other).unwrap_or_default();
                let has = format!("Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))");
                if negated {
                    format!("Filter::Not(&{has})")
                } else {
                    has
                }
            }
            other if other.strip_suffix("Source").and_then(color_atom).is_some() => {
                let (negated, color) = other
                    .strip_suffix("Source")
                    .and_then(color_atom)
                    .unwrap_or_default();
                if !negated {
                    state.source_color = match color {
                        "White" => Some("white"),
                        "Blue" => Some("blue"),
                        "Black" => Some("black"),
                        "Red" => Some("red"),
                        "Green" => Some("green"),
                        _ => None,
                    };
                }
                let has = format!("Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))");
                if negated {
                    format!("Filter::Not(&{has})")
                } else {
                    has
                }
            }
            other
                if other
                    .strip_prefix('!')
                    .and_then(named_target_name)
                    .is_some() =>
            {
                let name = other
                    .strip_prefix('!')
                    .and_then(named_target_name)
                    .unwrap_or_default();
                format!("Filter::Not(&Filter::Named({name:?}))")
            }
            other if self.source_power_var_atom(other).is_some() => self
                .source_power_var_atom(other)
                .unwrap_or_default()
                .to_string(),
            other if self.worded_atom(other).is_some() => {
                if let Some(name) = named_target_name(other) {
                    state.named_targets.push(name);
                }
                let Some(expr) = self.worded_atom(other) else {
                    return AtomParse::Unknown;
                };
                expr
            }
            // `Creature.Goblin` puts the subtype after the base, so
            // an atom can name one too — and it is the commonest
            // shape in the corpus, not a corner.
            other => {
                let (negated, name) = other
                    .strip_prefix("non")
                    .or_else(|| other.strip_prefix('!'))
                    .map_or((false, other), |rest| (true, rest));
                let Some(path) = self.cats.const_path(name) else {
                    return AtomParse::Unknown;
                };
                if negated {
                    format!("Filter::Not(&Filter::HasSubtype({path}))")
                } else {
                    format!("Filter::HasSubtype({path})")
                }
            }
        };
        clauses.push(clause);
        AtomParse::Matched
    }

    /// A valid-string as a `Filter`, distinguishing battlefield objects
    /// (`card_zone == false`, where every object is already a permanent) from
    /// card zones such as graveyards and libraries (`card_zone == true`, where
    /// `"Permanent"` means a permanent card under CR 110.4a).
    #[allow(clippy::too_many_lines)]
    pub(super) fn filter_expr_in_zone(&self, valid: &str, card_zone: bool) -> Option<String> {
        let raw_alts: Vec<&str> = valid.split(',').collect();
        let mut alternatives = Vec::new();
        for &alt in &raw_alts {
            if !card_zone && raw_alts.len() > 1 && emblem_subsumed_by_card(alt, &raw_alts) {
                continue;
            }
            let mut clauses = Vec::new();
            let mut state = FilterAltState::default();
            let raw_atoms: Vec<&str> = if alt.contains("AttachedTo ")
                || alt.contains("SharesColorWith Valid ")
                || alt.contains("sharesCreatureTypeWith Valid ")
                || alt.contains("SharesCreatureTypeWith Valid ")
            {
                let Some((base, tail)) = alt.split_once('.') else {
                    return self.deny(format!("filter base `{}`", alt.trim()));
                };
                if !self.push_filter_base(base.trim(), card_zone, &mut clauses, &mut state) {
                    return None;
                }
                let mut parts: Vec<&str> = Vec::new();
                let mut saw_embedded = false;
                for piece in tail.split('+') {
                    if saw_embedded {
                        // A `+` following an embedded sub-filter is ambiguous between the
                        // inner and outer predicates; refuse rather than mis-scope it.
                        self.note(format!("filter atom `{tail}`"));
                        return None;
                    }
                    let t = piece.trim();
                    if t.starts_with("AttachedTo ")
                        || t.starts_with("SharesColorWith Valid ")
                        || t.starts_with("!SharesColorWith Valid ")
                        || t.starts_with("sharesCreatureTypeWith Valid ")
                        || t.starts_with("!sharesCreatureTypeWith Valid ")
                        || t.starts_with("SharesCreatureTypeWith Valid ")
                        || t.starts_with("!SharesCreatureTypeWith Valid ")
                    {
                        saw_embedded = true;
                    }
                    parts.push(t);
                }
                parts
            } else {
                let mut atoms = alt.split('.');
                let base = atoms.next()?.trim();
                if !self.push_filter_base(base, card_zone, &mut clauses, &mut state) {
                    return None;
                }
                atoms.flat_map(|a| a.split('+')).collect()
            };
            let mut lowest_cmc = false;
            for atom in raw_atoms {
                let atom = atom.trim();
                match self.push_filter_atom(atom, card_zone, &mut clauses, &mut state) {
                    AtomParse::Matched => {}
                    AtomParse::LowestCmc => lowest_cmc = true,
                    AtomParse::Failed => return None,
                    AtomParse::Unknown => {
                        self.note(format!("filter atom `{atom}`"));
                        return None;
                    }
                }
            }
            if !card_zone && !state.has_controller {
                let lower = self.oracle.to_ascii_lowercase();
                // Some corpus scripts write `Card.IsCommander+YouOwn` where the
                // Oracle text explicitly scopes to "a commander you control"
                // rather than "your commander" (Norn's Choirmaster).
                if state.has_is_commander
                    && state.has_you_own
                    && lower.contains("commander you control")
                    && !lower.contains("commander you own")
                    && !lower.contains("your commander")
                {
                    for clause in &mut clauses {
                        if clause == "Filter::OwnedByYou" {
                            *clause = "Filter::ControlledByYou".to_string();
                        }
                    }
                }
                // Some corpus scripts omit `+YouCtrl` on a named-permanent
                // filter whose Oracle text unambiguously restricts to "you
                // control named <Name>" (Baloth Packhunter, Charmed Stray).
                if (!state.named_targets.is_empty()
                    && state
                        .named_targets
                        .iter()
                        .all(|name| oracle_scopes_named_to_you(&lower, name)))
                    || state.source_color.is_some_and(|color_word| {
                        oracle_scopes_combat_target_to_you(&lower, &format!("{color_word} source"))
                    })
                    || (clauses.iter().any(|c| c == "Filter::Blocking")
                        && oracle_scopes_combat_target_to_you(&lower, "target blocking creature"))
                    || (clauses.iter().any(|c| c == "Filter::Attacking")
                        && oracle_scopes_combat_target_to_you(&lower, "target attacking creature"))
                {
                    clauses.push("Filter::ControlledByYou".to_string());
                }
            }
            let mut expr = match clauses.len() {
                0 => "Filter::Any".to_string(),
                1 => clauses.remove(0),
                _ => format!("Filter::And(&[{}])", clauses.join(", ")),
            };
            if lowest_cmc {
                expr = format!("Filter::LowestManaValueOnBattlefield(&{expr})");
            }
            alternatives.push(expr);
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
        if matches!(
            expr,
            "Filter::And(&[Filter::RingBearer, Filter::ControlledByYou])"
                | "Filter::And(&[Filter::ControlledByYou, Filter::RingBearer])"
        ) {
            return "Filter::YOUR_RING_BEARER".to_string();
        }
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
        if valid == "Opponent" || valid == "Player.Opponent" {
            return Some("TargetSpec::AnyOpponent".to_string());
        }
        if valid == "Player.attackedWithCreaturesThisTurn" {
            return Some("TargetSpec::PlayerWhoAttackedThisTurn".to_string());
        }
        if matches!(valid, "Player,Planeswalker" | "Planeswalker,Player") {
            let lower = self.oracle.to_ascii_lowercase();
            if lower.contains("target opponent") && !lower.contains("target player") {
                return Some("TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)".to_string());
            }
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
        if let Some(mixed) = self.mixed_player_object_target(valid, api) {
            return Some(mixed);
        }
        if matches!(
            self.trigger_mode.as_deref(),
            Some("DamageDone" | "DamageDoneOnce")
        ) && api != "Counter"
            && let Some(stripped) = valid
                .split(',')
                .map(|alt| strip_atom_from_alt(alt, "ControlledBy TriggeredTarget"))
                .collect::<Option<Vec<_>>>()
        {
            let expr = self.filter_expr(&stripped.join(","))?;
            let name = self.body.filter_static("TARGET", &expr);
            return Some(format!("TargetSpec::ObjectOfEventPlayer(&{name})"));
        }
        let expr = if api == "Counter" {
            self.filter_expr_in_zone(valid, true)?
        } else {
            self.filter_expr(valid)?
        };
        let name = self.body.filter_static("TARGET", &expr);
        Some(match api {
            "Counter" => format!("TargetSpec::Spell(&{name})"),
            _ => format!("TargetSpec::Object(&{name})"),
        })
    }

    fn mixed_player_object_target(&mut self, valid: &str, api: &str) -> Option<String> {
        if !matches!(api, "DealDamage" | "PreventDamage") || !valid.contains(',') {
            return None;
        }
        let alts: Vec<&str> = valid
            .split(',')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect();
        let mut saw_player = false;
        let mut saw_opponent = false;
        let mut objects: Vec<&str> = Vec::new();
        for &alt in &alts {
            match alt {
                "Player" => saw_player = true,
                "Opponent" | "Player.Opponent" => saw_opponent = true,
                _ => objects.push(alt),
            }
        }
        if objects.is_empty() || (!saw_player && !saw_opponent) || (saw_player && saw_opponent) {
            return None;
        }
        if saw_player
            && objects.len() == 3
            && objects.contains(&"Creature")
            && objects.contains(&"Planeswalker")
            && objects.contains(&"Battle")
        {
            return Some("TargetSpec::AnyTarget".to_string());
        }
        let expr = self.filter_expr(&objects.join(","))?;
        let name = self.body.filter_static("TARGET", &expr);
        let opponents_only = saw_opponent || {
            let lower = self.oracle.to_ascii_lowercase();
            lower.contains("target opponent") && !lower.contains("target player")
        };
        Some(if opponents_only {
            format!("TargetSpec::OpponentOrObject(&{name})")
        } else {
            format!("TargetSpec::PlayerOrObject(&{name})")
        })
    }

    /// "Target spell or permanent": `TargetSpec::StackOrBattlefield` over the
    /// valid-string's filter (using `card_zone = true` so `Permanent` retains
    /// its CR 110.4b permanent-card type union on the stack).
    pub(super) fn spell_or_permanent_target(&mut self, valid: &str) -> Option<String> {
        let expr = self.filter_expr_in_zone(valid, true)?;
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
    /// graveyard unless the target line scopes it to "your graveyard"
    /// (Misery Charm). Its own description takes precedence over the whole
    /// card's Oracle text, since another chapter or mode may use a different
    /// scope. The rest of the valid-string is the card's filter.
    pub(super) fn graveyard_target(
        &mut self,
        valid: &str,
        defined_controller: Option<&str>,
        description: Option<&str>,
    ) -> Option<String> {
        let (mut whose, cleaned_alts, below_event) = self.split_graveyard_target_alts(valid)?;
        if defined_controller == Some("ParentTarget") {
            if whose != "PlayerRel::EachPlayer" || below_event {
                return None;
            }
            let filter = self.filter_expr_in_zone(&cleaned_alts.join(","), true)?;
            let filter = self.body.filter_static("TARGET", &filter);
            return Some(format!(
                "TargetSpec::CardInFirstTargetsGraveyard(&{filter})"
            ));
        }
        if whose == "PlayerRel::EachPlayer"
            && description
                .and_then(Self::targets_own_graveyard)
                .or_else(|| Self::targets_own_graveyard(self.oracle))
                == Some(true)
        {
            whose = "PlayerRel::You";
        }
        whose = self.resolve_graveyard_defined_controller(whose, defined_controller)?;
        let filter = self.filter_expr_in_zone(&cleaned_alts.join(","), true)?;
        let filter = self.body.filter_static("TARGET", &filter);
        let variant = if below_event {
            "TargetSpec::CardInGraveyardBelowEvent"
        } else {
            "TargetSpec::CardInGraveyard"
        };
        Some(format!("{variant}(&{filter}, {whose})"))
    }

    fn targets_own_graveyard(text: &str) -> Option<bool> {
        let text = text.to_ascii_lowercase();
        if [
            "in a graveyard",
            "from a graveyard",
            "in any graveyard",
            "from any graveyard",
            "in graveyards",
            "from graveyards",
        ]
        .into_iter()
        .any(|phrase| text.contains(phrase))
        {
            Some(false)
        } else {
            (text.contains("from your graveyard") || text.contains("in your graveyard"))
                .then_some(true)
        }
    }

    /// Whether the current trigger populates `PendingTrigger::event_mana_value`
    /// from the battlefield departure LKI (`state.ltb_mana_values`).
    fn trigger_populates_ltb_mana_value(&self) -> bool {
        match self.trigger_mode.as_deref() {
            Some("Sacrificed") => true,
            Some("ChangesZone") if !self.is_etb_self_trigger => {
                if self.oracle.is_empty() {
                    return true;
                }
                let lower = self.oracle.to_ascii_lowercase();
                lower.contains("dies")
                    || lower.contains("leaves the battlefield")
                    || lower.contains("from the battlefield")
            }
            _ => false,
        }
    }

    fn is_below_event_cmc_atom(&self, atom: &str) -> bool {
        let Some(var) = atom.strip_prefix("cmcLT") else {
            return false;
        };
        let var = var.trim();
        !var.is_empty()
            && var.chars().all(|c| c.is_ascii_alphabetic())
            && self.trigger_populates_ltb_mana_value()
            && self.svars.get(var).map(|s| s.trim()) == Some("TriggeredCard$CardManaCost")
    }

    fn split_graveyard_target_alts(
        &self,
        valid: &str,
    ) -> Option<(&'static str, Vec<String>, bool)> {
        let mut whose: Option<&'static str> = None;
        let mut below_event: Option<bool> = None;
        let mut cleaned_alts = Vec::new();
        for alt in valid.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut alt_whose: Option<&'static str> = None;
            let mut alt_below_event = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "YouCtrl" | "YouOwn" => alt_whose = Some("PlayerRel::You"),
                    "OppCtrl" | "OppOwn" | "YouDontOwn" | "YouDontCtrl" => {
                        alt_whose = Some("PlayerRel::Opponent");
                    }
                    "DefenderCtrl" | "DefenderOwn" | "OwnedBy TriggeredDefendingPlayer"
                        if matches!(
                            self.trigger_mode.as_deref(),
                            Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                        ) =>
                    {
                        alt_whose = Some("PlayerRel::DefendingPlayer");
                    }
                    "OwnedBy TriggeredTarget" | "ControlledBy TriggeredTarget"
                        if matches!(
                            self.trigger_mode.as_deref(),
                            Some("DamageDone" | "DamageDoneOnce")
                        ) =>
                    {
                        alt_whose = Some("PlayerRel::DamagedPlayer");
                    }
                    "ActivePlayerCtrl" | "ActivePlayerOwn" | "OwnedBy TriggeredPlayer"
                        if self.trigger_mode.as_deref() == Some("Phase") =>
                    {
                        alt_whose = Some("PlayerRel::ActivePlayer");
                    }
                    "ThisTurnEntered" => kept.push("PutIntoGraveyardThisTurn"),
                    "!ThisTurnEntered" => kept.push("!PutIntoGraveyardThisTurn"),
                    other if self.is_below_event_cmc_atom(other) => {
                        alt_below_event = true;
                    }
                    other => kept.push(other),
                }
            }
            match (whose, alt_whose) {
                (None, w) => whose = w,
                (Some(prev), Some(curr)) if prev != curr => return None,
                _ => {}
            }
            match (below_event, alt_below_event) {
                (None, b) => below_event = Some(b),
                (Some(prev), curr) if prev != curr => return None,
                _ => {}
            }
            // Stir the Grave spells its "with mana value X or less" bound as
            // `SVar:X:Targeted$CardManaCost` instead of `+cmcLEX` on `ValidTgts$`.
            // Require `{X}` in `ManaCost:` so non-X spells that compute life or
            // damage from `Targeted$CardManaCost` (e.g. Sheoldred's Restoration)
            // do not receive a bogus `Filter::CmcAtMostX` target restriction.
            if !alt_below_event
                && self.has_x
                && self.has_x_mana_cost
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
        Some((
            whose.unwrap_or("PlayerRel::EachPlayer"),
            cleaned_alts,
            below_event.unwrap_or(false),
        ))
    }

    fn resolve_graveyard_defined_controller(
        &self,
        whose: &'static str,
        defined_controller: Option<&str>,
    ) -> Option<&'static str> {
        Some(match defined_controller {
            None => whose,
            Some("You") => "PlayerRel::You",
            Some("TriggeredCardController") if whose == "PlayerRel::You" => whose,
            Some("TriggeredPlayer") if self.trigger_mode.as_deref() == Some("Phase") => {
                "PlayerRel::ActivePlayer"
            }
            Some("TriggeredDefendingPlayer")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                ) =>
            {
                "PlayerRel::DefendingPlayer"
            }
            Some("TriggeredTarget")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ) =>
            {
                "PlayerRel::DamagedPlayer"
            }
            Some(_) => return None,
        })
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
            "Player.Other" => "PlayerRel::EachOtherPlayer",
            // Gift recipient (CR 702.174a).
            "Promised" => "PlayerRel::GiftRecipient",
            // "Enchanted land's controller" (CR 303.4e).
            "EnchantedController" | "Player.EnchantedController" => {
                "PlayerRel::ControllerOfAttached"
            }
            // "Enchanted player" (CR 303.4b, CR 303.4m: Curses and player Auras).
            "EnchantedPlayer" | "Player.EnchantedBy" | "Player.EnchantedPlayer" => {
                "PlayerRel::EnchantedPlayer"
            }
            "TriggeredPlayer" if trigger == Some("Phase") => "PlayerRel::ActivePlayer",
            "TriggeredPlayer" | "TriggeredCardController" if trigger == Some("Sacrificed") => {
                "PlayerRel::EventPlayer"
            }
            "TriggeredCardOwner" if trigger == Some("Sacrificed") => "PlayerRel::OwnerOfEvent",
            "TriggeredAttackingPlayer"
                if matches!(
                    trigger,
                    Some("AttackersDeclared" | "AttackersDeclaredOneTarget")
                ) =>
            {
                "PlayerRel::EventPlayer"
            }
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
            "TriggeredTarget" if matches!(trigger, Some("DamageDone" | "DamageDoneOnce")) => {
                "PlayerRel::DamagedPlayer"
            }
            "TriggeredTargetController"
                if matches!(trigger, Some("DamageDone" | "DamageDoneOnce")) =>
            {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredBlockerController" if trigger == Some("Blocks") => {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredAttackerController"
                if matches!(trigger, Some("Attacks" | "AttackerUnblocked")) =>
            {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredAttackerController"
                if trigger == Some("AttackerBlocked") && !self.trigger_is_self =>
            {
                "PlayerRel::ControllerOfEvent"
            }
            "TriggeredAttackerController"
                if trigger == Some("AttackerBlocked") && self.trigger_is_self =>
            {
                "PlayerRel::You"
            }
            "TriggeredSourceSAController" | "TriggeredSourceController"
                if trigger == Some("BecomesTarget") =>
            {
                "PlayerRel::EventPlayer"
            }
            "TriggeredPlayer"
                if matches!(
                    trigger,
                    Some(
                        "LifeGained" | "LifeLost" | "Discarded" | "LandPlayed" | "SearchedLibrary"
                    )
                ) =>
            {
                "PlayerRel::EventPlayer"
            }
            "TriggeredCardOwner" | "TriggeredCardController" if trigger == Some("Discarded") => {
                "PlayerRel::EventPlayer"
            }
            "TriggeredDefendingPlayer"
                if matches!(
                    trigger,
                    Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                ) =>
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
        let player_target = matches!(
            target,
            Some(
                "TargetSpec::Player(PlayerRel::Chosen)"
                    | "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::PlayerWhoAttackedThisTurn"
            )
        );
        let object_target = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(")
                || t.starts_with("TargetSpec::Object(")
                || t.starts_with("TargetSpec::StackOrBattlefield(")
                || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                || t.starts_with("TargetSpec::ObjectOrCardInAnyGraveyard(")
        });
        match defined {
            Some("Targeted" | "TargetedPlayer") if player_target => Some("PlayerRel::Chosen"),
            Some("ParentTarget") if player_target && !targets_a_player => Some("PlayerRel::Chosen"),
            Some("TargetedController") if object_target => Some("PlayerRel::ControllerOfTarget"),
            Some("ParentTargetController") if object_target && !targets_a_player => {
                Some("PlayerRel::ControllerOfTarget")
            }
            Some(
                "Targeted"
                | "TargetedPlayer"
                | "ParentTarget"
                | "TargetedController"
                | "ParentTargetController",
            ) => None,
            other => self.player_rel_of(other, targets_a_player),
        }
    }

    /// Validates and wraps a formatted `TargetReq` expression with
    /// `TargetConstraint::SameController` (`TargetsWithSameController$ True`).
    ///
    /// Refuses player-targeting specs or single-target requirements where a
    /// group-level controller/zone-owner constraint has no meaning.
    pub(super) fn apply_same_controller_target_req(
        &self,
        req_expr: &str,
        target_spec: &str,
        multi_target: bool,
    ) -> Option<String> {
        let object_only = target_spec.starts_with("TargetSpec::Object(")
            || target_spec.starts_with("TargetSpec::CardInGraveyard(")
            || target_spec.starts_with("TargetSpec::CardInGraveyardBelowEvent(")
            || target_spec.starts_with("TargetSpec::Spell(")
            || target_spec.starts_with("TargetSpec::StackOrBattlefield(")
            || target_spec.starts_with("TargetSpec::ObjectOrCardInAnyGraveyard(");
        if !object_only || !multi_target {
            return self.deny(
                "`TargetsWithSameController$` on a non-multi-object target requirement".into(),
            );
        }
        Some(format!(
            "{req_expr}.with_constraint(TargetConstraint::SameController)"
        ))
    }
}

#[derive(Default)]
struct FilterAltState {
    has_controller: bool,
    has_is_commander: bool,
    has_you_own: bool,
    source_color: Option<&'static str>,
    named_targets: Vec<String>,
}

enum AtomParse {
    Matched,
    LowestCmc,
    Unknown,
    Failed,
}

/// Removes `needle` from a dot/plus-delimited filter alternative, returning
/// `Some(cleaned)` only when `needle` appeared as a non-base atom.
fn strip_atom_from_alt(alt: &str, needle: &str) -> Option<String> {
    let mut parts = alt
        .split('.')
        .flat_map(|s| s.split('+'))
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let base = parts.next()?;
    let mut found = false;
    let mut kept = Vec::new();
    for atom in parts {
        if atom == needle {
            found = true;
        } else {
            kept.push(atom);
        }
    }
    if !found {
        return None;
    }
    Some(if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}.{}", kept.join("+"))
    })
}

enum CounterAtom {
    Always,
    Filter(String),
}

/// Parses `counters_<OP><N>_<KIND>`, `counters_<OP><N>_ALL`, or bare
/// `counters_<OP><N>` into a `Filter` expression (`CounterAtom::Always` for
/// tautological `GE0`, or `None` on unsupported input).
fn counter_atom_filter(other: &str) -> Option<CounterAtom> {
    let (comparison, raw) = other
        .strip_prefix("counters_GE")
        .map(|raw| ("GE", raw))
        .or_else(|| other.strip_prefix("counters_GT").map(|raw| ("GT", raw)))
        .or_else(|| other.strip_prefix("counters_LE").map(|raw| ("LE", raw)))
        .or_else(|| other.strip_prefix("counters_LT").map(|raw| ("LT", raw)))
        .or_else(|| other.strip_prefix("counters_EQ").map(|raw| ("EQ", raw)))?;
    let (n_str, kind) = match raw.split_once('_') {
        Some((n_str, "ALL")) => (n_str, None),
        Some((n_str, code)) => (n_str, Some(counter_kind(code)?)),
        None => (raw, None),
    };
    let n = n_str.parse::<u16>().ok()?;
    let at_least = |k: u16| match kind.as_deref() {
        Some(kind) => format!("Filter::CountersAtLeast({kind}, {k})"),
        None => format!("Filter::AnyCountersAtLeast({k})"),
    };
    match comparison {
        "GE" if n == 0 => Some(CounterAtom::Always),
        "GE" => Some(CounterAtom::Filter(at_least(n))),
        "GT" => Some(CounterAtom::Filter(at_least(n.checked_add(1)?))),
        "LT" if n > 0 => Some(CounterAtom::Filter(format!(
            "Filter::Not(&{})",
            at_least(n)
        ))),
        "LE" => Some(CounterAtom::Filter(format!(
            "Filter::Not(&{})",
            at_least(n.checked_add(1)?)
        ))),
        "EQ" if n == 0 => Some(CounterAtom::Filter(format!(
            "Filter::Not(&{})",
            at_least(1)
        ))),
        "EQ" => {
            let above = n.checked_add(1)?;
            Some(CounterAtom::Filter(format!(
                "Filter::And(&[{}, Filter::Not(&{})])",
                at_least(n),
                at_least(above)
            )))
        }
        _ => None,
    }
}

/// Extracts the card name from a `named<Name>` filter atom, matching both
/// plain names and semicolon-escaped commas.
fn named_target_name(atom: &str) -> Option<String> {
    if let Some(name) = named_atom(atom) {
        return Some(name.to_string());
    }
    let raw = atom.strip_prefix("named")?;
    (!raw.is_empty()
        && raw.contains(';')
        && raw
            .chars()
            .all(|c| c.is_alphanumeric() || " '-;".contains(c)))
    .then(|| raw.replace(';', ","))
}

/// Returns `true` when Oracle text explicitly scopes permanents named `name`
/// to `"you control named <name>"` without also mentioning an unscoped
/// permanent/creature/card named `name`.
fn oracle_scopes_named_to_you(oracle_lower: &str, name: &str) -> bool {
    let name_lower = name.to_ascii_lowercase();
    oracle_lower.contains(&format!("you control named {name_lower}"))
        && !oracle_lower.contains(&format!("creature named {name_lower}"))
        && !oracle_lower.contains(&format!("creatures named {name_lower}"))
        && !oracle_lower.contains(&format!("permanent named {name_lower}"))
        && !oracle_lower.contains(&format!("permanents named {name_lower}"))
        && !oracle_lower.contains(&format!("card named {name_lower}"))
        && !oracle_lower.contains(&format!("cards named {name_lower}"))
}

/// Returns `true` when an `Emblem[.<ctrl>]` alternative in a battlefield/source
/// comma-list is already matched by a companion `Card[.<ctrl>][+non<Type>...]`
/// alternative in the same list. Emblems carry `TypeSet::EMPTY` and a controller
/// equal to their owner, so `Filter::Any`, controller predicates, and
/// `Filter::LacksType(...)` already match them.
pub(super) fn emblem_subsumed_by_card(alt: &str, all_alts: &[&str]) -> bool {
    fn is_ctrl_atom(atom: &str) -> bool {
        matches!(
            atom,
            "YouCtrl"
                | "OppCtrl"
                | "YouDontCtrl"
                | "ActivePlayerCtrl"
                | "DefenderCtrl"
                | "EnchantedPlayerCtrl"
                | "ColorlessSource"
                | "Colorless"
        )
    }
    fn is_non_card_type_atom(atom: &str) -> bool {
        matches!(
            atom,
            "nonCreature"
                | "nonArtifact"
                | "nonEnchantment"
                | "nonLand"
                | "nonPlaneswalker"
                | "nonBattle"
                | "nonInstant"
                | "nonSorcery"
                | "nonPermanent"
        )
    }
    let mut parts = alt
        .split('.')
        .flat_map(|s| s.split('+'))
        .map(str::trim)
        .filter(|a| !a.is_empty());
    if parts.next() != Some("Emblem") {
        return false;
    }
    let mut emblem_ctrl: Vec<&str> = Vec::new();
    for atom in parts {
        if is_ctrl_atom(atom) {
            emblem_ctrl.push(atom);
        } else {
            return false;
        }
    }
    emblem_ctrl.sort_unstable();
    all_alts.iter().any(|other| {
        let mut op = other
            .split('.')
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        if op.next() != Some("Card") {
            return false;
        }
        let mut card_ctrl: Vec<&str> = Vec::new();
        for atom in op {
            if is_ctrl_atom(atom) {
                card_ctrl.push(atom);
            } else if !is_non_card_type_atom(atom) {
                return false;
            }
        }
        card_ctrl.sort_unstable();
        card_ctrl == emblem_ctrl
    })
}

fn oracle_scopes_combat_target_to_you(lower_oracle: &str, phrase: &str) -> bool {
    let total = lower_oracle.matches(phrase).count();
    let yours = lower_oracle
        .matches(&format!("{phrase} you control"))
        .count();
    total > 0 && total == yours
}
