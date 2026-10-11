//! Junktown — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{R}, {T}, Sacrifice this land: Create three Junk tokens. (They're artifacts with "{T}, Sacrifice this token: Exile the top card of your library. You may play that card this turn. Activate only as a sorcery.")
//! Set: PIP #150 — Fallout | Scryfall ID: 5e24c687-f070-469d-a09d-acf4ac6ed374 | Oracle ID: c470a802-931f-4b63-92fd-ef9cf2e796dd

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUNKTOWN,
    oracle_id = "c470a802-931f-4b63-92fd-ef9cf2e796dd",
    scryfall_id = "5e24c687-f070-469d-a09d-acf4ac6ed374",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Junktown", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}{R}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenN {
                token: &crate::tokens::JUNK,
                amount: Amount::Fixed(3),
            }]
        ),
    ],
);
