//! Crypt of Agadeem — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {2}, {T}: Add {B} for each black creature card in your graveyard.
//! Set: TDC #354 — Tarkir: Dragonstorm Commander | Scryfall ID: 18adafe6-de67-4101-8b39-b53ab695ec4a | Oracle ID: 4fe8af73-c84a-44bd-9739-ee5c8b027874
// IMPLEMENTED — enters tapped; {T} adds {B}; {2}, {T} adds {B} per black
// creature card in your graveyard (a mana ability with a counted amount).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYPT_OF_AGADEEM,
    oracle_id = "4fe8af73-c84a-44bd-9739-ee5c8b027874",
    scryfall_id = "18adafe6-de67-4101-8b39-b53ab695ec4a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crypt of Agadeem",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        mana_ability!(
            cost!("{2}", TapSelf),
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::CREATURE,
                        Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
                    ]),
                    zone: ZoneSel::GraveyardYou,
                },
            )]
        ),
    ],
);
