//! Edgewall Inn — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {3}, {T}, Sacrifice this land: Return target card that has an Adventure from your graveyard to your hand.
//! Set: WOE #255 — Wilds of Eldraine | Scryfall ID: ec435e54-628a-43bd-8804-cbc37e375bce | Oracle ID: 4dfd33e8-7e30-493b-8564-f7df5f0257aa

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EDGEWALL_INN,
    oracle_id = "4dfd33e8-7e30-493b-8564-f7df5f0257aa",
    scryfall_id = "ec435e54-628a-43bd-8804-cbc37e375bce",
    faces = &[face!(
        name = "Edgewall Inn",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_chosen()]),
        activated!(
            cost!("{3}", TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::HasAdventure, PlayerRel::You)
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::HasAdventure,
                PlayerRel::You
            ))
        )
    ],
);
