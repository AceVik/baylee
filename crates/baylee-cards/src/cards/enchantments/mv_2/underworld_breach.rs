//! Underworld Breach — {1}{R} — Enchantment
//! Oracle: Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)
//! Oracle: At the beginning of the end step, sacrifice this enchantment.
//! Set: THB #161 — Theros Beyond Death | Scryfall ID: 0e51d796-7279-4c06-87f0-37adbdaa41df | Oracle ID: 27e0948b-9916-473b-8d8c-a51bdfbc7457
// PARTIAL — the end-step sacrifice is built; the escape grant is not
// expressible in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERWORLD_BREACH,
    oracle_id = "27e0948b-9916-473b-8d8c-a51bdfbc7457",
    scryfall_id = "0e51d796-7279-4c06-87f0-37adbdaa41df",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Each nonland card in your graveyard has escape — the DSL has \
         Modifier::GrantsFlashback and no escape grant, and escape is its own \
         keyword with its own cost (the card's mana cost plus exiling three \
         other cards from your graveyard), not a spelling of flashback"
    ),
    faces = &[face!(
        name = "Underworld Breach",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "Each nonland card in your graveyard has escape. The
    // escape cost is equal to the card's mana cost plus exile three other
    // cards from your graveyard." — no Modifier grants a graveyard card a
    // permission to be cast; Modifier::GrantsFlashback is the nearest and is
    // a different printed keyword with a different cost.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::SacrificeSelf]
    )],
);
