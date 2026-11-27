//! Lowering Forge `SetState` effects (`Mode$ Flip`, `Transform`,
//! `TurnFaceDown`, `TurnFaceUp`) and `ETBReplacement:Other` face-down entry
//! effects (*Ixidron*).

use super::{Params, Tx, card_type_const};

impl Tx<'_> {
    /// Lowers a `SetState` step inside a chain or standalone ability.
    ///
    /// Keeps existing `Effect::TransformSource` and simple `Effect::TurnFaceUp`
    /// output byte-identical for single-self transforms and basic turn-face-up
    /// scripts while extending full `Effect::SetState` coverage for Kamigawa
    /// Flip, non-self Transform, `TurnFaceDown` (default & custom `FaceDownSpec`),
    /// and reveal-conditional / linked-exile `TurnFaceUp`.
    ///
    /// Strictly refuses Alchemy `Mode$ Unspecialize` and any unknown mode or
    /// unclaimed parameters.
    #[allow(clippy::too_many_lines)]
    pub(super) fn lower_set_state(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mode = p.take("Mode")?;
        let eff = match mode.as_str() {
            "Flip" => {
                let subject =
                    self.set_state_subject(p.take("Defined").as_deref(), target, targets_here)?;
                format!(
                    "Effect::SetState {{ subject: {subject}, transition: StateTransition::Flip, if_cant: &[] }}"
                )
            }
            "Transform" => {
                let defined = p.take("Defined");
                if !self.on_a_spell
                    && !targets_here
                    && matches!(defined.as_deref(), None | Some("Self"))
                    && target.is_none()
                {
                    "Effect::TransformSource".to_string()
                } else {
                    let subject =
                        self.set_state_subject(defined.as_deref(), target, targets_here)?;
                    format!(
                        "Effect::SetState {{ subject: {subject}, transition: StateTransition::Transform, if_cant: &[] }}"
                    )
                }
            }
            "TurnFaceDown" => {
                let subject =
                    self.set_state_subject(p.take("Defined").as_deref(), target, targets_here)?;
                let spec = self.parse_face_down_spec(p)?;
                format!(
                    "Effect::SetState {{ subject: {subject}, transition: StateTransition::TurnFaceDown({spec}), if_cant: &[] }}"
                )
            }
            "TurnFaceUp" => {
                let defined = p.take("Defined");
                let reveal_first = match p.take("RevealFirst").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let valid_new_face = p.take("ValidNewFace");
                if reveal_first || valid_new_face.is_some() {
                    let raw_filter = valid_new_face?;
                    let subject =
                        self.set_state_subject(defined.as_deref(), target, targets_here)?;
                    let expr = self.filter_expr(&raw_filter)?;
                    let filter_name = self.body.filter_static("FACE_UP", &expr);
                    return Some(vec![format!(
                        "Effect::SetState {{ subject: {subject}, transition: StateTransition::RevealAndTurnFaceUpIf {{ filter: &{filter_name}, optional: {optional} }}, if_cant: &[] }}"
                    )]);
                }
                if matches!(defined.as_deref(), Some("ExiledWithSource" | "Imprinted")) {
                    "Effect::SetState { subject: StateSubject::LinkedExile, transition: StateTransition::TurnFaceUp, if_cant: &[] }".to_string()
                } else if matches!(defined.as_deref(), Some("Remembered")) {
                    "Effect::SetState { subject: StateSubject::Found, transition: StateTransition::TurnFaceUp, if_cant: &[] }".to_string()
                } else if let Some(raw) = defined.as_deref().and_then(|d| d.strip_prefix("Valid "))
                {
                    let expr = self.filter_expr(raw.trim())?;
                    let filter_name = self.body.filter_static("STATE", &expr);
                    format!(
                        "Effect::SetState {{ subject: StateSubject::BattlefieldFilter(&{filter_name}), transition: StateTransition::TurnFaceUp, if_cant: &[] }}"
                    )
                } else {
                    let to = match (defined.as_deref(), target) {
                        (None | Some("Targeted" | "ParentTarget"), Some(aimed))
                            if aimed.starts_with("TargetSpec::Object(") =>
                        {
                            aimed.to_string()
                        }
                        (None | Some("Self"), None) if !self.on_a_spell => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        (Some("Self"), _) if !self.on_a_spell && !targets_here => {
                            "TargetSpec::ThisObject".to_string()
                        }
                        (Some(def), None) if self.is_event_object_defined(def) => {
                            "TargetSpec::EventObject".to_string()
                        }
                        _ => return None,
                    };
                    format!("Effect::TurnFaceUp {{ target: {to} }}")
                }
            }
            // Alchemy `Unspecialize` and any unknown mode remain refused.
            _ => return None,
        };
        if optional {
            Some(vec![format!("Effect::MayDo {{ effects: &[{eff}] }}")])
        } else {
            Some(vec![eff])
        }
    }

    /// Lowers `K:ETBReplacement:Other:<svar>` when `<svar>` is
    /// `DB$ SetState | Defined$ Valid <filter> | Mode$ TurnFaceDown` (*Ixidron*).
    pub(super) fn set_state_turn_face_down_on_enter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Mode").as_deref() != Some("TurnFaceDown") {
            return self.deny("as-enters `SetState` mode other than `TurnFaceDown`".to_string());
        }
        let Some(defined) = p.take("Defined") else {
            return self.deny("as-enters `SetState` without `Defined$`".to_string());
        };
        let Some(raw) = defined.strip_prefix("Valid ") else {
            return self.deny(format!("as-enters `SetState` defined `{defined}`"));
        };
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `SetState.{key}`"));
        }
        let expr = self.filter_expr(raw.trim())?;
        let name = self.body.filter_static("FACE_DOWN", &expr);
        self.body
            .enter_modifiers
            .push(format!("EnterModifier::TurnMatchingFaceDown(&{name})"));
        Some(())
    }

    fn set_state_subject(
        &mut self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<String> {
        match (defined, target) {
            (None | Some("Targeted" | "ParentTarget"), Some(aimed))
                if aimed.starts_with("TargetSpec::Object(") =>
            {
                Some("StateSubject::Targets".to_string())
            }
            (None | Some("Self"), None) if !self.on_a_spell => {
                Some("StateSubject::Source".to_string())
            }
            (Some("Self"), _) if !self.on_a_spell && !targets_here => {
                Some("StateSubject::Source".to_string())
            }
            (Some("Remembered"), _) => Some("StateSubject::Remembered".to_string()),
            (Some(def), _) if self.is_event_object_defined(def) => {
                Some("StateSubject::EventObject".to_string())
            }
            (Some(def), _) if def.starts_with("Valid ") => {
                let raw = def.strip_prefix("Valid ")?.trim();
                let expr = self.filter_expr(raw)?;
                let name = self.body.filter_static("STATE", &expr);
                Some(format!("StateSubject::BattlefieldFilter(&{name})"))
            }
            _ => None,
        }
    }

    fn parse_face_down_spec(&self, p: &mut Params) -> Option<String> {
        let raw_types = p.take("FaceDownSetType");
        let raw_power = p.take("FaceDownPower");
        let raw_toughness = p.take("FaceDownToughness");
        if raw_types.is_none() && raw_power.is_none() && raw_toughness.is_none() {
            return Some("FaceDownSpec::DEFAULT".to_string());
        }
        let mut type_bits: Vec<&'static str> = Vec::new();
        let mut subtypes: Vec<String> = Vec::new();
        if let Some(raw) = raw_types {
            for part in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                if let Some(t) = card_type_const(part) {
                    type_bits.push(t);
                } else {
                    let sub = self.cats.const_path(part)?;
                    subtypes.push(sub);
                }
            }
        } else {
            type_bits.push("TypeSet::CREATURE");
        }
        if type_bits.is_empty() {
            return None;
        }
        let mut types_expr = type_bits[0].to_string();
        for bit in &type_bits[1..] {
            types_expr = format!("{types_expr}.union({bit})");
        }
        let power = match raw_power {
            Some(v) => format!("Some({})", v.trim().parse::<i16>().ok()?),
            None => "Some(2)".to_string(),
        };
        let toughness = match raw_toughness {
            Some(v) => format!("Some({})", v.trim().parse::<i16>().ok()?),
            None => "Some(2)".to_string(),
        };
        Some(format!(
            "FaceDownSpec {{ types: {types_expr}, subtypes: &[{}], power: {power}, toughness: {toughness} }}",
            subtypes.join(", ")
        ))
    }
}
