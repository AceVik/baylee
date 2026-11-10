//! Physical rules metadata resolved independently of presentation print handles.

use baylee_core::card_metadata::{ReferencePrintRules, RulesMetadata};
use baylee_core::ids::CardIndex;
use baylee_core::preset::GamePreset;

/// Verified facts for the actual printing pinned by this registry.
#[must_use]
pub fn reference(card: CardIndex) -> Option<ReferencePrintRules> {
    let rows = crate::generated_sides::REFERENCE_RULES;
    rows.binary_search_by_key(&card, |(index, _)| *index)
        .ok()
        .map(|at| rows[at].1)
}

/// Fills entries whose artwork id is the registry's verified reference printing.
/// Alternate printings stay unresolved for an authoritative catalog to hydrate.
/// # Errors
/// Rejects malformed metadata or a missing artwork-table reference.
pub fn hydrate_reference(preset: &mut GamePreset) -> Result<(), String> {
    let prints = &preset.prints;
    let rules: &mut RulesMetadata = &mut preset.rules;
    for seat in &mut preset.seats {
        for entry in seat
            .deck
            .iter_mut()
            .chain(&mut seat.sideboard)
            .chain(&mut seat.commanders)
            .chain(seat.starting_hand.iter_mut().flatten())
            .chain(&mut seat.starting_battlefield)
        {
            if entry.rule_print.is_some() {
                continue;
            }
            let print = prints
                .get(usize::from(entry.print.get()))
                .ok_or("printing reference is out of range")?;
            let Some(def) = crate::by_index(entry.card) else {
                continue;
            };
            if print.scryfall_id.to_string() != def.scryfall_id {
                continue;
            }
            let facts = reference(entry.card).ok_or("missing verified reference-print facts")?;
            entry.rule_print = Some(rules.insert(entry.card, facts.watermarks, facts.is_un)?);
        }
    }
    Ok(())
}

/// Checks card-specific deck choices before a physical copy enters a game.
/// # Errors
/// Cryptic Spires needs two circles; other cards may not carry them.
pub fn validate_choices(
    card: CardIndex,
    choices: baylee_core::card_metadata::DeckChoices,
) -> Result<(), String> {
    if !choices.is_valid() {
        return Err("invalid circled colors".into());
    }
    if card == baylee_core::generated::index::CRYPTIC_SPIRES {
        if choices.circled_colors.is_none() {
            return Err("Cryptic Spires needs two circled colors (circles=WU)".into());
        }
    } else if choices.circled_colors.is_some() {
        return Err("only Cryptic Spires may carry circled colors".into());
    }
    Ok(())
}

/// The color identity of this physical copy, including deck-construction choices.
#[must_use]
pub fn identity(
    card: CardIndex,
    choices: baylee_core::card_metadata::DeckChoices,
) -> Option<baylee_core::color::ColorSet> {
    crate::by_index(card).map(|def| def.color_identity.union(choices.colors()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::card_metadata::DeckChoices;
    use baylee_core::color::{Color, ColorPair};
    use baylee_core::generated::index;

    #[test]
    fn verified_reference_facts_distinguish_watermarks_and_un_printings() {
        assert_eq!(
            reference(index::SECRET_BASE).unwrap().watermarks,
            &[Some("orderofthewidget")]
        );
        assert!(reference(index::SECRET_BASE).unwrap().is_un);
        assert!(reference(index::UNDERDOME).unwrap().is_un);
        let forest = reference(index::FOREST).unwrap();
        assert_eq!(forest.watermarks, &[None]);
        assert!(!forest.is_un);
    }

    #[test]
    fn spires_choice_validation_accepts_ten_pairs_and_requires_them_only_on_spires() {
        assert!(validate_choices(index::CRYPTIC_SPIRES, DeckChoices::default()).is_err());
        for (at, first) in Color::ALL.iter().enumerate() {
            for second in &Color::ALL[at + 1..] {
                let choices = DeckChoices {
                    circled_colors: Some(ColorPair::new(*first, *second)),
                };
                assert!(validate_choices(index::CRYPTIC_SPIRES, choices).is_ok());
                assert!(validate_choices(index::FOREST, choices).is_err());
                assert_eq!(
                    identity(index::CRYPTIC_SPIRES, choices).unwrap(),
                    choices.colors()
                );
            }
        }
    }

    #[test]
    fn explicit_alternate_printing_does_not_inherit_reference_facts() {
        let mut preset = crate::decks::probe_preset(1, index::SECRET_BASE).unwrap();
        let entry = &mut preset.seats[0].deck[0];
        entry.rule_print = None;
        let slot = usize::from(entry.print.get());
        preset.prints[slot].scryfall_id = uuid::Uuid::nil();
        hydrate_reference(&mut preset).unwrap();
        assert!(preset.seats[0].deck[0].rule_print.is_none());
    }
}
