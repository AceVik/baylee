//! Reading multi-layer animation effects.

use super::{Params, Tx, card_type_const, color_const, keyword_const};

impl Tx<'_> {
    /// `Animate`: the manland sentence — "until end of turn, this land
    /// becomes a 4/4 white and blue Elemental creature with flying and
    /// vigilance. It's still a land."
    ///
    /// One printed sentence, four continuous effects, because CR 613.1
    /// applies type, colour, ability and power/toughness in that order and
    /// each is its own layer. "It's still a land" is why the types are
    /// *added* rather than set — an animated Colonnade that stopped being a
    /// land would stop making mana.
    ///
    /// Two objects are read, each the one `Filter::This` binds to: the
    /// source (`Defined$ Self`) on a chain that targets nothing, and the
    /// target on a line that names no `Defined$` — the Laces' "target spell
    /// or permanent becomes red". `Filter::This` binds to the first target
    /// when there is one and to the source when there is not, so a chain
    /// with both would animate the wrong permanent, and is refused.
    ///
    /// `Duration$ Permanent` is "for the rest of the game"
    /// (`Duration::Indefinitely`), `UntilEndOfCombat` is until end of combat;
    /// without one it is until end of turn.
    ///
    /// `RemoveCreatureTypes$ True` is CR 205.1b's "becomes a [creature type]
    /// artifact creature": the first creature type named replaces the ones it
    /// had (`Modifier::ReplaceCreatureTypes`) and every other type is kept.
    ///
    /// `RemoveLandTypes$ True` is "target land becomes a Forest" (Gaea's
    /// Liege): CR 305.7's setting of a land's subtype, `Modifier::SetLandType`,
    /// for the one basic land type named; a second land type, or none, is
    /// refused. `Duration$ UntilHostLeavesPlay` is "until [this] leaves the
    /// battlefield", `Duration::WhileSourceOnBattlefield`.
    pub(super) fn animate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        match (p.take("Defined").as_deref(), target) {
            (Some("Self"), None) | (None, Some(_)) => {}
            _ => {
                self.note("`Animate` of something other than the source".to_string());
                return None;
            }
        }
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilHostLeavesPlay") => "Duration::WhileSourceOnBattlefield",
            Some(other) => {
                self.note(format!("`Animate` lasting `{other}`"));
                return None;
            }
        };
        let replace_creature_types = match p.take("RemoveCreatureTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveCreatureTypes$ {other}`")),
        };
        let set_land_type = match p.take("RemoveLandTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveLandTypes$ {other}`")),
        };
        let mut out = Vec::new();
        let types = p.take("Types").unwrap_or_default();
        self.animate_types(
            &types,
            (replace_creature_types, set_land_type),
            duration,
            &mut out,
        )?;
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 105.3).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            let colors: Option<Vec<&str>> = raw.split(',').map(|c| color_const(c.trim())).collect();
            let Some(colors) = colors else {
                self.note(format!("`Animate` into colours `{raw}`"));
                return None;
            };
            let which = if overwrite { "SetColor" } else { "AddColor" };
            out.push(Self::animate_expr(
                &format!(
                    "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
                duration,
            ));
        }
        // Layer 6: keywords it gains.
        if let Some(raw) = p.take("Keywords") {
            let each: Option<Vec<&str>> = raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let Some(each) = each else {
                self.note(format!("`Animate` granting `{raw}`"));
                return None;
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            out.push(Self::animate_expr(
                &format!("Modifier::AddKeyword({joined})"),
                duration,
            ));
        }
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr(
                    &format!("Modifier::SetPT({power}, {toughness})"),
                    duration,
                ));
            }
            (None, None) => {}
            _ => {
                self.note("`Animate` setting only one of power and toughness".to_string());
                return None;
            }
        }
        if out.is_empty() {
            self.note("`Animate` that changes nothing".to_string());
            return None;
        }
        Some(out)
    }

    /// Layer 4 of an [`Self::animate_effect`]: the types it becomes. A word
    /// is either a card type or a subtype, and the corpus writes both in one
    /// list. `remove` is (`RemoveCreatureTypes$ True`, `RemoveLandTypes$
    /// True`), each of which needs the one word it replaces with.
    pub(super) fn animate_types(
        &mut self,
        types: &str,
        remove: (bool, bool),
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let (replace_creature_types, set_land_type) = remove;
        let mut set = false;
        let mut replaced = false;
        for word in types.split(',').filter(|w| !w.trim().is_empty()) {
            let word = word.trim();
            let modifier = if let Some(types) = card_type_const(word) {
                format!("Modifier::AddType({types})")
            } else if set_land_type && let Some(path) = self.basic_land_type(word) {
                if set {
                    return self.deny("`RemoveLandTypes$` naming two land types".to_string());
                }
                set = true;
                format!("Modifier::SetLandType({path})")
            } else {
                if set_land_type
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Land, word)
                        .is_some()
                {
                    return self.deny(format!("`RemoveLandTypes$` into `{word}`"));
                }
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("`Animate` into `{word}`"));
                    return None;
                };
                let creature_type = self
                    .cats
                    .const_path_of(baylee_core::types::SubtypeKind::Creature, word)
                    .is_some();
                if replace_creature_types && creature_type && !replaced {
                    replaced = true;
                    format!("Modifier::ReplaceCreatureTypes({path})")
                } else {
                    format!("Modifier::AddSubtype({path})")
                }
            };
            out.push(Self::animate_expr(&modifier, duration));
        }
        if replace_creature_types && !replaced {
            return self.deny("`RemoveCreatureTypes$` naming no creature type".to_string());
        }
        if set_land_type && !set {
            return self.deny("`RemoveLandTypes$` naming no basic land type".to_string());
        }
        Some(())
    }

    /// One layer of an [`Self::animate_effect`], as the `Effect` expression.
    ///
    /// No layer is passed in because none is written out: `Effect::continuous`
    /// derives it from the modifier the way CR 613.1 does, so the emitter
    /// cannot name a layer that disagrees with what it is applying.
    pub(super) fn animate_expr(modifier: &str, duration: &str) -> String {
        format!("Effect::continuous(&Filter::This, {modifier}, {duration})")
    }
}
