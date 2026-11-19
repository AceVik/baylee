//! Reading `Dig`: the top cards of a library looked at or revealed, some
//! of them moved, and the rest put where the line says.

use super::{Params, Tx};

impl Tx<'_> {
    /// One `Dig` line as the effects it stands for.
    pub(super) fn dig_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.take("Reveal").as_deref() != Some("True")
            || p.take("DestinationZone2").as_deref() != Some("Graveyard")
            || !matches!(p.take("DestinationZone").as_deref(), None | Some("Hand"))
        {
            return None;
        }
        let count = p.take("DigNum")?.parse::<u8>().ok()?;
        let valid = p.take("ChangeValid")?;
        let filter = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("DIG", &filter);
        Some(
            match (
                count,
                p.take("ChangeNum").as_deref(),
                p.take("Optional").as_deref(),
            ) {
                (1, Some("All"), None) => vec![format!(
                    "Effect::RevealTopAndSort {{ filter: &{name}, matched: SearchDest::Hand, otherwise: SearchDest::Graveyard }}"
                )],
                (count, None, Some("True")) => vec![format!(
                    "Effect::RevealTopFind {{ found: Find::HAND, count: {count}, filter: &{name}, otherwise: SearchDest::Graveyard }}"
                )],
                _ => return None,
            },
        )
    }
}
