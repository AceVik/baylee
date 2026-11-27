//! Clone, copy-choice, and top-of-library reveal-clone lowerings
//! (`Clone` with `Choices$` / `ChoiceZone$ Exile` -> `ChooseCardEx` + `BecomeCopy`,
//! and `PeekAndReveal | NoPeek$ True` -> `RevealTop` + `BecomeCopy`).

use super::Params;
use super::Tx;

impl Tx<'_> {
    /// Lower `Clone` with `Choices$` (`Cytoshape`, `Deepfathom Echo`,
    /// `Lazav, Wearer of Faces`, `Reflection Net`) into `Effect::ChooseCardEx`
    /// followed by `Effect::BecomeCopy { original: TargetSpec::EventObject, .. }`.
    pub(super) fn clone_choices_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        second_target_here: bool,
    ) -> Option<Vec<String>> {
        p.claim_label("ChoiceTitle", "Choices");
        let choices_raw = p.take("Choices")?;
        if p.has("ExcludeChosen") {
            return None;
        }
        let choice_zone_raw = p.take("ChoiceZone");
        let (choose_zone, filter_expr) = match choice_zone_raw.as_deref() {
            None | Some("Battlefield") => (
                "ChooseZone::Battlefield",
                self.filter_expr(choices_raw.trim())?,
            ),
            Some("Exile") => {
                if self.card_has_unlinked_exile_producer() {
                    return None;
                }
                let stripped = Self::strip_exiled_with_source_filter(choices_raw.trim())?;
                (
                    "ChooseZone::LinkedExile",
                    self.filter_expr_in_zone(&stripped, true)?,
                )
            }
            Some(_) => return None,
        };
        let filter_name = self.body.filter_static("CLONE_CHOICE", &filter_expr);

        let until_eot = match p.take("Duration").as_deref() {
            None | Some("Permanent") => false,
            Some("UntilEndOfTurn") => true,
            Some(_) => return None,
        };
        let opt_a = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let opt_b = match p.take("ChoiceOptional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let min: u8 = u8::from(!(opt_a || opt_b));

        let mods = self.copy_mods(p, true)?;
        if let Some(extra_svars) = p.peek("AddSVars")
            && mods.iter().any(|m| m == "CopyMod::KeepResolvingAbility")
            && self
                .svars
                .get(extra_svars.trim())
                .and_then(|body| Params::parse(body))
                .is_some_and(|(api, _)| api == "Clone")
        {
            p.take("AddSVars");
        }

        if p.take("Defined").is_some() {
            return None;
        }
        let obj_target = target.filter(|t| {
            t.starts_with("TargetSpec::Object(") || t.starts_with("TargetSpec::CardInGraveyard(")
        });
        let clone_target = p.take("CloneTarget");
        if second_target_here {
            return None;
        }
        let subject = if clone_target.as_deref() == Some("Targeted") && obj_target.is_some() {
            "TargetSlot::First"
        } else if matches!(clone_target.as_deref(), None | Some("Self")) {
            if !self.on_a_spell && obj_target.is_none() {
                "TargetSlot::This"
            } else if self.on_a_spell && obj_target.is_some() {
                "TargetSlot::First"
            } else {
                return None;
            }
        } else {
            return None;
        };

        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: {choose_zone}, \
             filter: &{filter_name}, min: {min}, max: Amount::Fixed(1), reveal_chosen: false, \
             movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
             then: &[Effect::BecomeCopy {{ subject: {subject}, subject_filter: None, \
             original: TargetSpec::EventObject, original_is_second_target: false, \
             until_eot: {until_eot}, mods: &[{}] }}], for_unchosen: &[] }}",
            mods.join(", ")
        )])
    }

    /// Lower `PeekAndReveal | NoPeek$ True` (without `RememberRevealed$`) whose
    /// `SubAbility$` is `DB$ Clone | ConditionDefined$ TopOfLibrary |
    /// ConditionPresent$ <filter> | Defined$ TopOfLibrary | CloneTarget$ Self`
    /// (*Vesuvan Drifter*) into `Effect::RevealTop { ..., if_matches: &[BecomeCopy] }`.
    pub(super) fn peek_no_peek_clone(
        &mut self,
        library: &str,
        sub_name: &str,
    ) -> Option<(String, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Clone"
            || self.on_a_spell
            || sp.take("ConditionDefined").as_deref() != Some("TopOfLibrary")
            || sp.take("Defined").as_deref() != Some("TopOfLibrary")
            || !matches!(sp.take("CloneTarget").as_deref(), None | Some("Self"))
            || !matches!(sp.take("CloneZone").as_deref(), None | Some("Battlefield"))
        {
            return None;
        }
        let cond_present = sp.take("ConditionPresent")?;
        if !matches!(
            sp.take("ConditionCompare").as_deref(),
            None | Some("GE1" | "EQ1" | "GT0")
        ) {
            return None;
        }
        let until_eot = match sp.take("Duration").as_deref() {
            None | Some("Permanent") => false,
            Some("UntilEndOfTurn") => true,
            Some(_) => return None,
        };
        let optional = match sp.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mods = self.copy_mods(&mut sp, true)?;
        let trailing = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let cond_expr = self.filter_expr_in_zone(cond_present.trim(), true)?;
        let cond_filter = self.body.filter_static("REVEAL_TOP", &cond_expr);
        let become_copy = format!(
            "Effect::BecomeCopy {{ subject: TargetSlot::This, subject_filter: None, \
             original: TargetSpec::EventObject, original_is_second_target: false, \
             until_eot: {until_eot}, mods: &[{}] }}",
            mods.join(", ")
        );
        let then_eff = if optional {
            format!("Effect::MayDo {{ effects: &[{become_copy}] }}")
        } else {
            become_copy
        };
        Some((
            format!(
                "Effect::RevealTop {{ player: {library}, count: Amount::Fixed(1), \
                 filter: &{cond_filter}, if_matches: &[{then_eff}], otherwise: &[] }}"
            ),
            trailing,
        ))
    }

    /// Lower `Counter | TargetType$ Spell | RememberCountered$ True | Destination$ Exile`
    /// followed by `Play | Valid$ Card.[nonLand+]IsRemembered | ValidZone$ Exile |
    /// WithoutManaCost$ True | Optional$ True` (*Transcendent Dragon*) into
    /// `Effect::CaptureAffected { first: &[CounterTargetSpellToExile], then: &[ForEachRemembered(Play)] }`.
    pub(super) fn counter_to_exile_play_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("Destination") != Some("Exile")
            || p.peek("RememberCountered") != Some("True")
            || !target.is_some_and(|t| t.starts_with("TargetSpec::Spell("))
        {
            return None;
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if s_api != "Play" {
            return None;
        }
        let valid_raw = sp.take("Valid")?;
        let nonland = match valid_raw.as_str() {
            "Card.nonLand+IsRemembered" | "Card.IsRemembered+nonLand" => true,
            "Card.IsRemembered" => false,
            _ => return None,
        };
        if sp.take("ValidZone").as_deref() != Some("Exile")
            || sp.take("ValidSA").as_deref() != Some("Spell")
            || !matches!(sp.take("Controller").as_deref(), None | Some("You"))
            || sp.take("Optional").as_deref() != Some("True")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
            || !matches!(sp.take("Amount").as_deref(), None | Some("1"))
        {
            return None;
        }
        let next_sub = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }

        let mut probe = p.clone();
        probe.take("Destination");
        probe.take("RememberCountered");
        probe.take("TargetType");
        if !probe.exhausted() {
            return None;
        }
        let trailing = self.wave29c_consume_remember_cleanup(next_sub)?;
        *p = probe;
        *sub = trailing;
        let filter_ref = if nonland {
            "&Filter::NONLAND"
        } else {
            "&Filter::Any"
        };
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::CounterTargetSpellToExile], \
             then: &[Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[ \
             Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: {filter_ref}, controller: PlayerRel::You, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: false, \
             exile_instead_of_graveyard: false, optional: true }}] }}] }}"
        )])
    }
}
