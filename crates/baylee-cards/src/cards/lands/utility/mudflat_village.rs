//! Mudflat Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {B}. Spend this mana only to cast a creature spell.
//! Oracle: {1}{B}, {T}, Sacrifice this land: Return target Bat, Lizard, Rat, or Squirrel card from your graveyard to your hand.
//! Set: BLB #257 — Bloomburrow | Scryfall ID: 53ec4ad3-9cf0-4f1b-a9db-d63feee594ab | Oracle ID: aeeab1df-0b8b-4bc4-a5f9-aac413449bec
// IMPLEMENTED — colorless mana, black mana restricted to creature spells,
// and the graveyard-to-hand return for the four printed tribes.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "a Bat, Lizard, Rat, or Squirrel card" — the four tribes the third
/// ability names, in the order the card prints them.
static BAT_LIZARD_RAT_OR_SQUIRREL: Filter = Filter::Or(&[
    Filter::HasSubtype(creature::BAT),
    Filter::HasSubtype(creature::LIZARD),
    Filter::HasSubtype(creature::RAT),
    Filter::HasSubtype(creature::SQUIRREL),
]);

card!(
    index = index::MUDFLAT_VILLAGE,
    oracle_id = "aeeab1df-0b8b-4bc4-a5f9-aac413449bec",
    scryfall_id = "53ec4ad3-9cf0-4f1b-a9db-d63feee594ab",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Mudflat Village", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::Black, 1).restricted(&Filter::CREATURE, SpendRider::None)
        ]),
        activated!(
            cost!("{1}{B}", TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&BAT_LIZARD_RAT_OR_SQUIRREL, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &BAT_LIZARD_RAT_OR_SQUIRREL,
                PlayerRel::You,
            ))
        ),
    ],
);
