use super::{Content, Error};
use baylee_core::{ids::CardIndex, preset::Finish};
use baylee_tower::cards::{CardRarity, Presentation};

/// Current publication content. Retained `expeditions()` stays byte-identical;
/// this palette is shared across all fifty decks rather than copied per deck.
/// # Errors
/// Missing reviewed printing or invalid generated formation.
pub fn runic() -> Result<Content, Error> {
    let mut content = super::expeditions()?;
    for card in &mut content.catalog.cards {
        card.run.as_mut().ok_or(Error::Metadata)?.presentation = Some(of(card.card)?);
    }
    super::validate_catalog(&content.catalog)?;
    Ok(content)
}

pub(super) fn of(index: CardIndex) -> Result<Presentation, Error> {
    let print = baylee_cards::decks::reference_print(index);
    if print.scryfall_id.is_nil() {
        return Err(Error::Metadata);
    }
    let rarity = match baylee_cards::sides::rarity(index)
        .ok_or(Error::Metadata)?
        .word()
    {
        "mythic" => CardRarity::Mythic,
        "rare" | "special" | "bonus" => CardRarity::Rare,
        "uncommon" => CardRarity::Uncommon,
        _ => CardRarity::Common,
    };
    // Cosmetic treatments are already supported by the shared builder, including
    // cards not sold physically in foil. Every commander and one third of the
    // remaining identities shine; the frozen catalog owns that choice.
    let commander =
        baylee_cards::decks::leader_of(index).is_some_and(|leader| leader.eligible_alone);
    Ok(Presentation {
        printing: print.scryfall_id,
        finish: if commander || index.get().is_multiple_of(3) {
            Finish::Foil
        } else {
            Finish::Normal
        },
        rarity,
    })
}
