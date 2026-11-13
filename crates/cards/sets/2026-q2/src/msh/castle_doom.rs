//! Castle Doom — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an artifact spell.
//! Oracle: {3}, {T}, Sacrifice an artifact: Create a 3/3 colorless Robot Villain artifact creature token named Doombot. Activate only as a sorcery.
//! Set: MSH #263 — Marvel Super Heroes | Scryfall ID: 6b39d7a6-ca2d-4376-a18e-efd0138e83bc | Oracle ID: 9bd013df-ad75-4099-940b-1765c58faf26
// IMPLEMENTED — both mana abilities, the {C} one plain and the any-colour
// one restricted to artifact spells, and the Doombot activation.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::CASTLE_DOOM,
    oracle_id = "9bd013df-ad75-4099-940b-1765c58faf26",
    scryfall_id = "6b39d7a6-ca2d-4376-a18e-efd0138e83bc",
    faces = &[face!(name = "Castle Doom", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&Filter::ARTIFACT, SpendRider::None)
        ]),
        // `Filter::ARTIFACT` and not `YOUR_ARTIFACT`: the card prints
        // "Sacrifice an artifact" and says nothing about control, because
        // CR 701.21a already does — a player may only sacrifice a permanent
        // they control, and `cost_wizard::options` reads that rule for
        // every sacrifice rather than leaving it to each card's filter.
        activated!(
            cost!("{3}", TapSelf, Sacrifice(&Filter::ARTIFACT)),
            &[Effect::CreateToken {
                token: &generated_tokens::DOOMBOT_ARTIFACT_3_3
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
