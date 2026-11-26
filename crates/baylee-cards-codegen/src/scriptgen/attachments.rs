//! Generalized attachment and unattachment effects (`Unattach`, `AttachObject`,
//! `MoveAllAttachments`, `AttachLtbAttachments`, `AttachChosenThen`,
//! `RelocateAttachment`).

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Lower `DB$/SP$ Unattach` into `Effect::Unattach` or
    /// `Effect::UnattachAllEquipmentFromTarget`.
    ///
    /// Multi-target `ValidTgts$ Creature` (`TargetMin$/Max$ > 1`) stays refused
    /// because `Effect::UnattachAllEquipmentFromTarget` only acts on `targets.first()`.
    pub(super) fn unattach_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        let def = p.take("Defined")?;
        if def == "Valid Equipment.AttachedTo Card.targetedBy"
            && targets_here
            && single_target
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
        {
            return Some(vec!["Effect::UnattachAllEquipmentFromTarget".to_string()]);
        }
        if self.in_delayed
            && matches!(
                def.as_str(),
                "DelayTriggerRemembered" | "DelayTriggerRememberedLKI"
            )
            && target.is_none()
        {
            return Some(vec![
                "Effect::Unattach { target: TargetSpec::EventObject }".to_string(),
            ]);
        }
        if !self.on_a_spell && !targets_here && target.is_none() && def == "Enchanted" {
            return Some(vec![
                "Effect::Unattach { target: TargetSpec::Object(&Filter::AttachedToBySource) }"
                    .to_string(),
            ]);
        }
        None
    }

    /// Lower `DB$ Attach | Choices$ <F> | RememberAttached$ True | SubAbility$ ...`
    /// into `Effect::AttachChosenThen` with conditional `then_if_attached` steps.
    pub(super) fn try_attach_chosen_then_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if p.peek("RememberAttached") != Some("True")
            || !p.has("Choices")
            || p.has("Object")
            || p.has("Chooser")
            || p.has("PlayerChoices")
            || p.has("Move")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("RememberAttached");
        probe.take("ChoiceTitle");
        probe.take("IsCurse");
        let optional = Self::take_attach_optional(&mut probe)?;
        let choices_raw = probe.take("Choices")?;
        let def_raw = match probe.take("Defined") {
            Some(d) => d,
            None if !self.on_a_spell && target.is_none() => "Self".to_string(),
            None => return None,
        };
        let to = self.resolve_attach_host_spec(&def_raw, target, targets_here)?;
        if !probe.exhausted() {
            return None;
        }
        let att_name = self.static_attachment_filter("ATTACH_CHOSEN", &choices_raw)?;
        let (then_if_attached, trailing) =
            self.collect_attach_chosen_then_followup(sub.clone(), chain)?;
        if then_if_attached.is_empty() {
            return None;
        }
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::AttachChosenThen {{ filter: &{att_name}, to: {to}, max: Amount::Fixed(1), optional: {optional}, then_if_attached: &[{}] }}",
            then_if_attached.join(", ")
        )])
    }

    /// Lower extended `DB$ Attach` patterns (`MoveAllAttachments`,
    /// `AttachLtbAttachments`, `AttachObject`, and `RelocateAttachment`).
    pub(super) fn attach_extended_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        flags: super::chains::LineFlags,
        optional: bool,
    ) -> Option<Vec<String>> {
        let super::chains::LineFlags {
            targets_here,
            targets_a_player,
            single_target,
            ..
        } = flags;
        if let Some(eff) =
            self.move_all_attachments_effect(p, target, targets_here, single_target, optional)
        {
            return Some(eff);
        }
        if let Some(eff) =
            self.attach_ltb_attachments_effect(p, target, targets_here, single_target, optional)
        {
            return Some(eff);
        }
        if let Some(eff) =
            self.relocate_attachment_effect(p, target, targets_here, targets_a_player, optional)
        {
            return Some(eff);
        }
        self.attach_object_effect(p, target, targets_here, optional)
    }

    fn take_attach_optional(p: &mut Params) -> Option<bool> {
        match p.take("Optional").as_deref() {
            None | Some("False") => Some(false),
            Some("True") => Some(true),
            Some(_) => None,
        }
    }

    fn static_attachment_filter(&mut self, prefix: &str, raw: &str) -> Option<String> {
        if raw.contains("Remembered")
            || raw.contains("Imprinted")
            || raw.contains("ChosenCard")
            || raw.contains("Targeted")
        {
            return None;
        }
        let expr = self.filter_expr(raw)?;
        Some(self.body.filter_static(prefix, &expr))
    }

    fn resolve_attach_host_spec(
        &self,
        def: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<String> {
        match def {
            "Self" if !self.on_a_spell => Some("TargetSpec::ThisObject".to_string()),
            "Targeted" | "ParentTarget" => target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::PlayerOrObject(")
                })
                .map(str::to_string),
            _ if !targets_here && self.is_event_object_defined(def) => {
                Some("TargetSpec::EventObject".to_string())
            }
            _ => None,
        }
    }

    fn collect_attach_chosen_then_followup(
        &mut self,
        first_sub: Option<String>,
        chain: &Chain,
    ) -> Option<(Vec<String>, Option<String>)> {
        let mut cursor = first_sub;
        let mut out = Vec::new();
        let mut tail = None;
        for _ in 0..4 {
            let Some(name) = cursor.take() else {
                break;
            };
            let body = self.svars.get(name.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                tail = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            if sp.peek("ConditionDefined") != Some("Remembered")
                || sp.peek("ConditionPresent") != Some("Card")
                || !matches!(
                    sp.peek("ConditionCompare"),
                    None | Some("GE1" | "EQ1" | "GT0")
                )
                || sp.has("ConditionZone")
                || sp.has("ValidTgts")
            {
                tail = Some(name);
                break;
            }
            sp.take("ConditionDefined");
            sp.take("ConditionPresent");
            sp.take("ConditionCompare");
            sp.take("ConditionDescription");
            let next = sp.take("SubAbility");
            if s_api == "DelayedTrigger" {
                if sp.peek("RememberObjects") != Some("Remembered") {
                    return None;
                }
                let mut delayed_chain = Chain::default();
                delayed_chain
                    .effects
                    .push("Effect::AttachChosenThen".to_string());
                let trig = self.delayed_trigger(&mut sp, None, false, &delayed_chain)?;
                if !sp.exhausted() {
                    return None;
                }
                out.push(trig);
            } else {
                let _ = chain;
                let effs = self.effect_of(&s_api, &mut sp, None, false, false)?;
                if !sp.exhausted() || effs.is_empty() {
                    return None;
                }
                out.extend(effs);
            }
            cursor = next;
        }
        if cursor.is_some() {
            return None;
        }
        Some((out, tail))
    }

    /// `DB$ Attach | Object$ Valid <A>.AttachedTo Targeted |
    /// Choices$ <H>.sharesControllerWith Targeted | Move$ True` (*Glamer Spinners*).
    fn move_all_attachments_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        optional: bool,
    ) -> Option<Vec<String>> {
        if !targets_here
            || !single_target
            || p.peek("Move") != Some("True")
            || p.has("Defined")
            || p.has("Chooser")
            || p.has("PlayerChoices")
            || p.has("RememberAttached")
        {
            return None;
        }
        let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
        let obj_raw = p.peek("Object")?.strip_prefix("Valid ")?;
        let att_raw = obj_raw.strip_suffix(".AttachedTo Targeted")?.trim();
        let host_raw = p
            .peek("Choices")?
            .strip_suffix(".sharesControllerWith Targeted")?
            .trim();
        let att_name = self.static_attachment_filter("MOVE_ATT", att_raw)?;
        if host_raw.contains("Remembered")
            || host_raw.contains("Imprinted")
            || host_raw.contains("ChosenCard")
            || host_raw.contains("Targeted")
        {
            return None;
        }
        let host_expr = self.filter_expr(host_raw)?;
        let rec_expr = if host_expr == "Filter::Any" {
            "Filter::ControlledBy(PlayerRel::ControllerOfTarget)".to_string()
        } else {
            format!(
                "Filter::And(&[{host_expr}, Filter::ControlledBy(PlayerRel::ControllerOfTarget)])"
            )
        };
        let rec_name = self.body.filter_static("MOVE_HOST", &rec_expr);
        p.take("Object");
        p.take("Choices");
        p.take("Move");
        p.take("ChoiceTitle");
        p.take("IsCurse");
        Some(vec![format!(
            "Effect::MoveAllAttachments {{ attachments: &{att_name}, from: {tgt}, recipient_filter: &{rec_name}, must_change_recipient: true, optional: {optional} }}"
        )])
    }

    /// `DB$ Attach | Object$ AttachedTo <Subject>[.<Filter>] | Defined$ <Host>`
    /// (*Rhuk, Hexgold Nabber*).
    fn attach_ltb_attachments_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        optional: bool,
    ) -> Option<Vec<String>> {
        if p.has("Choices")
            || p.has("Move")
            || p.has("Chooser")
            || p.has("PlayerChoices")
            || p.has("RememberAttached")
        {
            return None;
        }
        let raw_obj = p.peek("Object")?.strip_prefix("AttachedTo ")?.trim();
        let (subj, qual) = match raw_obj.split_once('.') {
            Some((s, q)) => (s.trim(), q.trim()),
            None => (raw_obj, "Card"),
        };
        let of_spec = if !targets_here && self.is_event_object_defined(subj) {
            "TargetSpec::EventObject".to_string()
        } else if !targets_here && single_target && matches!(subj, "Targeted" | "ParentTarget") {
            target
                .filter(|t| t.starts_with("TargetSpec::Object("))?
                .to_string()
        } else {
            return None;
        };
        let def_raw = match p.peek("Defined") {
            Some(d) => d.to_string(),
            None if !self.on_a_spell && target.is_none() => "Self".to_string(),
            None => return None,
        };
        let to_spec = if def_raw == "Self" && !self.on_a_spell {
            "TargetSpec::ThisObject".to_string()
        } else if matches!(def_raw.as_str(), "Targeted" | "ParentTarget") {
            target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::PlayerOrObject(")
                })?
                .to_string()
        } else if !targets_here && self.is_event_object_defined(&def_raw) {
            "TargetSpec::EventObject".to_string()
        } else {
            return None;
        };
        let att_name = self.static_attachment_filter("ATTACH_LTB", qual)?;
        p.take("Object");
        p.take("Defined");
        p.take("ChoiceTitle");
        p.take("IsCurse");
        Some(vec![format!(
            "Effect::AttachLtbAttachments {{ of: {of_spec}, filter: &{att_name}, to: {to_spec}, optional: {optional} }}"
        )])
    }

    /// `DB$ Attach | Object$ <EventObj> | ValidTgts$ <Host>` (*Sigarda's Aid*,
    /// *Hammer of Nazahn*) or `Object$ Targeted | Defined$ <EventObj> | ValidTgts$ <Eq>`
    /// (*Kemba, Kha Enduring*).
    fn attach_object_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        optional: bool,
    ) -> Option<Vec<String>> {
        if p.has("Choices")
            || p.has("Move")
            || p.has("Chooser")
            || p.has("PlayerChoices")
            || p.has("RememberAttached")
        {
            return None;
        }
        let raw_obj = p.peek("Object")?;
        let raw_def = p.peek("Defined");
        let (object_spec, to_spec) =
            if targets_here && self.is_event_object_defined(raw_obj) && raw_def.is_none() {
                let tgt = target.filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::PlayerOrObject(")
                })?;
                ("TargetSpec::EventObject".to_string(), tgt.to_string())
            } else if matches!(raw_obj, "Targeted" | "ParentTarget")
                && ((raw_obj == "Targeted" && targets_here)
                    || (raw_obj == "ParentTarget" && !targets_here))
                && let Some(def) = raw_def
                && self.is_event_object_defined(def)
            {
                let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
                (tgt.to_string(), "TargetSpec::EventObject".to_string())
            } else {
                return None;
            };
        p.take("Object");
        p.take("Defined");
        p.take("ChoiceTitle");
        p.take("IsCurse");
        Some(vec![format!(
            "Effect::AttachObject {{ object: {object_spec}, to: {to_spec}, optional: {optional} }}"
        )])
    }

    /// `DB$ Attach | Object$ Self|OriginalHost|Targeted | Chooser$ ... | Choices$ ...`
    /// (*Steam Vines*, *Kudzu*, *Nettlevine Blight*).
    fn relocate_attachment_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        optional: bool,
    ) -> Option<Vec<String>> {
        if !p.has("Choices")
            || p.has("Defined")
            || p.has("PlayerChoices")
            || p.has("RememberAttached")
            || (!p.has("Chooser") && p.peek("Object") != Some("OriginalHost"))
        {
            return None;
        }
        let attachment = match p.peek("Object") {
            Some("Self") if !self.on_a_spell => {
                "AttachmentSource::Spec(TargetSpec::ThisObject)".to_string()
            }
            Some("OriginalHost") if self.in_granted_trigger => {
                "AttachmentSource::GrantedSource".to_string()
            }
            Some("Targeted")
                if targets_here && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                format!("AttachmentSource::Spec({})", target?)
            }
            Some("ParentTarget")
                if !targets_here
                    && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                format!("AttachmentSource::Spec({})", target?)
            }
            _ => return None,
        };
        let chooser = match p.peek("Chooser") {
            None => "PlayerRel::You",
            Some(raw) => self.wave11_player_of_line(Some(raw), target, targets_a_player)?,
        };
        let must_change_recipient = match p.peek("Move") {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let choices = p.peek("Choices")?;
        if choices.contains("Remembered")
            || choices.contains("Imprinted")
            || choices.contains("ChosenCard")
            || choices.contains("Targeted")
        {
            return None;
        }
        let rec_expr = self.filter_expr(choices)?;
        let rec_ref = if rec_expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("ATTACH_TO", &rec_expr))
        };
        p.take("Object");
        p.take("Chooser");
        p.take("Choices");
        p.take("Move");
        p.take("ChoiceTitle");
        p.take("IsCurse");
        Some(vec![format!(
            "Effect::RelocateAttachment {{ attachment: {attachment}, chooser: {chooser}, recipient_filter: {rec_ref}, must_change_recipient: {must_change_recipient}, optional: {optional}, then_if_moved: &[] }}"
        )])
    }
}
