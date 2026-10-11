//! Nykthos, Shrine to Nyx — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Choose a color. Add an amount of mana of that color equal to your devotion to that color. (Your devotion to a color is the number of mana symbols of that color in the mana costs of permanents you control.)
//! Set: THS #223 — Theros | Scryfall ID: 834b27a0-dfd7-4f96-8cde-cacac4b24acc | Oracle ID: 84dc18f0-8225-4b40-a165-b10321e41769
use baylee_cards_dsl::prelude::*;

card!(
    index = index::NYKTHOS_SHRINE_TO_NYX,
    oracle_id = "84dc18f0-8225-4b40-a165-b10321e41769",
    scryfall_id = "834b27a0-dfd7-4f96-8cde-cacac4b24acc",
    faces = &[face!(
        name = "Nykthos, Shrine to Nyx",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{2}", TapSelf),
            &[Effect::AddMana {
                source: ManaSource::Choice(&[
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green
                ]),
                amount: Amount::DevotionToManaColor,
                combination: false,
                restriction: None,
            }]
        ),
    ],
);
