//! Tomb Fortress — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {2}{B}{B}{B}, {T}, Exile this land: Mill four cards, then return a creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: 40K #168 — Warhammer 40,000 Commander | Scryfall ID: 5ffd1a43-5613-4058-9e51-fd3cc1585651 | Oracle ID: 986f510c-e2ec-423e-a443-51a169939558

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOMB_FORTRESS,
    oracle_id = "986f510c-e2ec-423e-a443-51a169939558",
    scryfall_id = "5ffd1a43-5613-4058-9e51-fd3cc1585651",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tomb Fortress",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{2}{B}{B}{B}", TapSelf, ExileSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::You
                },
                Effect::ChooseFromYourGraveyard {
                    filter: &Filter::CREATURE,
                    destination: SearchDest::Battlefield,
                    tapped: false,
                    optional: false
                }
            ],
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
