//! Memorial to Folly — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand.
//! Set: TDC #377 — Tarkir: Dragonstorm Commander | Scryfall ID: 2c5c070e-91d3-4613-8a46-0323ee425027 | Oracle ID: 2bc38f14-0314-4351-8138-e2b8bf041404
// IMPLEMENTED — enters tapped, taps for {B}, and the {2}{B} sacrifice
// ability returns a target creature card from your graveyard to your hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORIAL_TO_FOLLY,
    oracle_id = "2bc38f14-0314-4351-8138-e2b8bf041404",
    scryfall_id = "2c5c070e-91d3-4613-8a46-0323ee425027",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Memorial to Folly",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{2}{B}", TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You
            ))
        ),
    ],
);
