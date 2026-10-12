//! Validated printing facts and per-copy deck choices, separate from artwork.

use crate::color::{ColorPair, ColorSet};
use crate::ids::{CardIndex, RulePrintRef, WatermarkId};
use serde::{Deserialize, Serialize};

/// Choices made for a physical copy while constructing its deck (CR 207.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckChoices {
    /// Cryptic Spires' two circled colors, fixed before the game begins.
    #[serde(default)]
    pub circled_colors: Option<ColorPair>,
}

impl DeckChoices {
    /// The circled colors, or an empty set when no circles were recorded.
    #[must_use]
    pub fn colors(self) -> ColorSet {
        self.circled_colors
            .map_or(ColorSet::EMPTY, ColorSet::of_pair)
    }

    /// Rejects duplicate or noncanonical pairs arriving through deserialization.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.circled_colors.is_none_or(|pair| {
            pair.first() != pair.second() && pair == ColorPair::new(pair.first(), pair.second())
        })
    }
}

/// Explicit rules facts of one printing, resolved outside the engine.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RulePrint {
    /// Rules identity against which the printing was validated.
    pub card: CardIndex,
    /// The watermark on each rules face; `None` is a verified absence.
    pub watermarks: Vec<Option<WatermarkId>>,
    /// The physical printing is silver-bordered or has an acorn security stamp.
    pub is_un: bool,
}

/// Reference-print facts generated from the pinned printing's cached metadata.
/// Strings are interned once at setup, never interpreted in a rules hot path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReferencePrintRules {
    /// Watermark tags by rules face, including an explicit absent tag.
    pub watermarks: &'static [Option<&'static str>],
    /// Silver border or acorn stamp on that actual printing.
    pub is_un: bool,
}

/// Rules-only metadata. Its references are deliberately unrelated to `PrintRef`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesMetadata {
    /// Exact source watermark tags; ids are stable within this preset/replay.
    pub watermarks: Vec<String>,
    /// Validated, deduplicated physical-print facts used by the entries.
    pub prints: Vec<RulePrint>,
}

impl RulesMetadata {
    /// Interns genuine provider metadata at the boundary, with bounded ids.
    ///
    /// # Errors
    /// Rejects empty/oversized tags, excessive face counts or exhausted ids.
    pub fn insert(
        &mut self,
        card: CardIndex,
        watermarks: &[Option<&str>],
        is_un: bool,
    ) -> Result<RulePrintRef, String> {
        if watermarks.is_empty() || watermarks.len() > usize::from(u8::MAX) {
            return Err("a rules printing needs 1–255 faces".to_string());
        }
        if watermarks.iter().flatten().any(|tag| !valid_watermark(tag)) {
            return Err("invalid watermark tag".to_string());
        }
        let mut faces = Vec::with_capacity(watermarks.len());
        for tag in watermarks {
            let id = match tag {
                None => None,
                Some(tag) => {
                    let index = if let Some(index) =
                        self.watermarks.iter().position(|known| known == tag)
                    {
                        index
                    } else {
                        let index = self.watermarks.len();
                        u16::try_from(index).map_err(|_| "too many watermarks")?;
                        self.watermarks.push((*tag).to_string());
                        index
                    };
                    Some(WatermarkId::new(
                        u16::try_from(index).map_err(|_| "too many watermarks")?,
                    ))
                }
            };
            faces.push(id);
        }
        let print = RulePrint {
            card,
            watermarks: faces,
            is_un,
        };
        if let Some(index) = self.prints.iter().position(|known| *known == print) {
            return u16::try_from(index)
                .map(RulePrintRef::new)
                .map_err(|_| "too many rules printings".to_string());
        }
        let index = u16::try_from(self.prints.len()).map_err(|_| "too many rules printings")?;
        self.prints.push(print);
        Ok(RulePrintRef::new(index))
    }

    /// Validates all references before a preset can allocate engine objects.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let bound = usize::from(u16::MAX) + 1;
        self.watermarks.len() <= bound
            && self.prints.len() <= bound
            && self.watermarks.iter().all(|tag| valid_watermark(tag))
            && self
                .watermarks
                .iter()
                .enumerate()
                .all(|(i, tag)| !self.watermarks[..i].contains(tag))
            && self.prints.iter().all(|print| {
                !print.watermarks.is_empty()
                    && u8::try_from(print.watermarks.len()).is_ok()
                    && print
                        .watermarks
                        .iter()
                        .flatten()
                        .all(|id| usize::from(id.get()) < self.watermarks.len())
            })
    }
}

fn valid_watermark(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 80
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn rules_metadata_deduplicates_only_equal_actual_facts() {
        let mut rules = RulesMetadata::default();
        let card = CardIndex::new(7);
        let first = rules.insert(card, &[Some("azorius")], false).unwrap();
        assert_eq!(
            first,
            rules.insert(card, &[Some("azorius")], false).unwrap()
        );
        assert_ne!(first, rules.insert(card, &[None], false).unwrap());
        assert_ne!(first, rules.insert(card, &[Some("azorius")], true).unwrap());
        assert!(rules.is_valid());
        rules.prints[0].watermarks[0] = Some(WatermarkId::new(8));
        assert!(!rules.is_valid());
    }

    #[test]
    fn circles_require_two_distinct_canonically_ordered_colors() {
        for first in Color::ALL {
            for second in Color::ALL {
                if first != second {
                    let choices = DeckChoices {
                        circled_colors: Some(ColorPair::new(first, second)),
                    };
                    assert!(choices.is_valid());
                    assert_eq!(choices.colors().len(), 2);
                }
            }
        }
        let invalid: DeckChoices =
            serde_json::from_str(r#"{"circled_colors":["Blue","Blue"]}"#).unwrap();
        assert!(!invalid.is_valid());
    }
}
