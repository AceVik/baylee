//! Resolution-time creature-type choice chains (`ChooseType | Type$ Creature`)
//! and `DigUntil` library reveal-until-match effects.

use super::{Chain, Params, Tx};
use baylee_core::types::SubtypeKind;

impl Tx<'_> {
    /// `ChooseType | Type$ Creature` in an effect chain: emits
    /// `Effect::ChooseCreatureType { exclude }` and reads its `SubAbility$`
    /// under `in_chosen_creature_type = true`.
    ///
    /// When the immediate sub-ability is `Animate` or `AnimateAll` setting
    /// `Types$ ChosenType`, this reader translates it into
    /// `Effect::BecomeChosenCreatureType` (preserving any `ValidTgts$`,
    /// `Defined$`, `RemoveCreatureTypes$`, `Duration$`, and trailing
    /// `SubAbility$`). Any other sub-ability (`DestroyAll`, `PumpAll`,
    /// `UntapAll`, `Dig`, `DigUntil`, `Discard`, `ChangeZoneAll`,
    /// `GainControl`) flows through the normal chain reader with
    /// `ChosenType` filter atoms enabled.
    pub(super) fn choose_type_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(other) => {
                return self.deny(format!(
                    "`ChooseType` for somebody other than `You` (`{other}`)"
                ));
            }
        }
        if p.peek("Type") == Some("Basic Land") {
            return self.wave18c_choose_basic_land_type_chain(p, sub, chain);
        }
        match p.take("Type").as_deref() {
            Some("Creature") => {}
            other => {
                return self.deny(format!("`ChooseType` of `{}`", other.unwrap_or("nameless")));
            }
        }
        let exclude = match p.take("InvalidTypes") {
            None => "None".to_string(),
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.contains([',', ';', '!', '+']) {
                    return self.deny(format!("`ChooseType` excluding `{trimmed}`"));
                }
                let Some(path) = self.cats.const_path_of(SubtypeKind::Creature, trimmed) else {
                    return self.deny(format!(
                        "`ChooseType` excluding unknown creature type `{trimmed}`"
                    ));
                };
                format!("Some({path})")
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseType.{key}`"));
        }
        let Some(sub_name) = sub else {
            return self.deny("`ChooseType` with no `SubAbility$`".to_string());
        };
        let Some(sub_body) = self.svars.get(sub_name).cloned() else {
            return self.deny(format!("`SubAbility$ {sub_name}` names no SVar"));
        };
        chain.effects.push(format!(
            "Effect::ChooseCreatureType {{ exclude: {exclude} }}"
        ));
        let prev = std::mem::replace(&mut self.in_chosen_creature_type, true);
        let result = self.choose_type_sub(&sub_body, chain);
        self.in_chosen_creature_type = prev;
        result
    }

    fn choose_type_sub(&mut self, sub_body: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(sub_body) else {
            return self.deny("a `ChooseType` `SubAbility$` with no `$` in it".to_string());
        };
        if api == "Animate" && p.peek("Types") == Some("ChosenType") {
            p.drop_prose();
            return self.chosen_type_animate(p, chain);
        }
        if api == "AnimateAll" && p.peek("Types") == Some("ChosenType") {
            p.drop_prose();
            return self.chosen_type_animate_all(p, chain);
        }
        self.chain(sub_body, chain)
    }

    fn chosen_type_animate(&mut self, mut p: Params, chain: &mut Chain) -> Option<()> {
        p.take("Types");
        let next_sub = p.take("SubAbility");
        let valid_tgts = p.take("ValidTgts");
        let defined = p.take("Defined");
        let filter = if let Some(valid) = valid_tgts {
            if !matches!(defined.as_deref(), None | Some("Targeted")) {
                return self
                    .deny("`ChooseType` `Animate` with both `ValidTgts$` and `Defined$`".into());
            }
            if chain.target.is_some() {
                return self.deny("`ChooseType` `Animate` with a second target".into());
            }
            chain.target = Some(self.target_spec(&valid, "Animate")?);
            "Filter::This"
        } else {
            match defined.as_deref() {
                None | Some("Self") => "Filter::This",
                Some("Enchanted" | "Equipped") => "Filter::AttachedToBySource",
                Some(other) => {
                    return self.deny(format!("`ChooseType` `Animate` on `Defined$ {other}`"));
                }
            }
        };
        let additive = match p.take("RemoveCreatureTypes").as_deref() {
            Some("True") => false,
            None => true,
            Some(other) => {
                return self.deny(format!(
                    "`ChooseType` `Animate` with `RemoveCreatureTypes$ {other}`"
                ));
            }
        };
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(other) => {
                return self.deny(format!("`ChooseType` `Animate` lasting `{other}`"));
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Animate.{key}`"));
        }
        chain.effects.push(format!(
            "Effect::BecomeChosenCreatureType {{ filter: &{filter}, additive: {additive}, duration: {duration} }}"
        ));
        self.chain_sub_ability(next_sub, chain)
    }

    fn chosen_type_animate_all(&mut self, mut p: Params, chain: &mut Chain) -> Option<()> {
        p.take("Types");
        let next_sub = p.take("SubAbility");
        let Some(valid) = p.take("ValidCards") else {
            return self.deny("`ChooseType` `AnimateAll` with no `ValidCards$`".into());
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("BECOME_ALL", &expr);
        let additive = match p.take("RemoveCreatureTypes").as_deref() {
            Some("True") => false,
            None => true,
            Some(other) => {
                return self.deny(format!(
                    "`ChooseType` `AnimateAll` with `RemoveCreatureTypes$ {other}`"
                ));
            }
        };
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(other) => {
                return self.deny(format!("`ChooseType` `AnimateAll` lasting `{other}`"));
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `AnimateAll.{key}`"));
        }
        chain.effects.push(format!(
            "Effect::BecomeChosenCreatureType {{ filter: &{filter}, additive: {additive}, duration: {duration} }}"
        ));
        self.chain_sub_ability(next_sub, chain)
    }

    /// `DigUntil`: "Reveal cards from the top of your library until you reveal
    /// a matching card. Put that card in `FoundDestination$` and the rest in
    /// `RevealedDestination$` (optionally shuffling)."
    pub(super) fn dig_until_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if target.is_some() || targets_a_player {
            return None;
        }
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        p.claim_label("ValidDescription", "Valid");
        let valid = p.take("Valid")?;
        let matched = match p.take("FoundDestination").as_deref() {
            Some("Battlefield") => "SearchDest::Battlefield",
            Some("Hand") => "SearchDest::Hand",
            _ => return None,
        };
        let otherwise = match p.take("RevealedDestination").as_deref() {
            Some("Library") => "SearchDest::TopOfLibrary",
            Some("Graveyard") => "SearchDest::Graveyard",
            Some("Exile") => "SearchDest::Exile",
            _ => return None,
        };
        let shuffle = match p.take("Shuffle").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("REVEAL_UNTIL", &expr);
        let mut out = vec![format!(
            "Effect::RevealUntilMatching {{ player: PlayerRel::You, filter: &{name}, matched: {matched}, otherwise: {otherwise} }}"
        )];
        if shuffle {
            out.push("Effect::ShuffleLibrary { who: PlayerRel::You }".to_string());
        }
        Some(out)
    }
}
