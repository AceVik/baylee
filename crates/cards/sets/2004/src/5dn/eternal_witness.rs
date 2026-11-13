//! Eternal Witness — {1}{G}{G} — Creature — Human Shaman
//! Oracle: When this creature enters, you may return target card from your graveyard to your hand.
//! Set: CMM #286 — Commander Masters | Scryfall ID: 39704000-65d3-4d39-849e-a3b617376bbc | Oracle ID: 30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c
// IMPLEMENTED — ETB regrowth: any one card in your own graveyard comes back
// to your hand. The printed "you may" is the target minimum of none, the way
// Sun Titan and Hagra Diabolist say it — declining is choosing no target.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ETERNAL_WITNESS,
    oracle_id = "30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c",
    scryfall_id = "39704000-65d3-4d39-849e-a3b617376bbc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Eternal Witness",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You),
        }],
        targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::You,
        )))
    )],
);

// Engine-level test belongs in baylee-engine (card_tests): this is the first
// card to pair `GraveyardToHand` with a target minimum of none, so the path
// worth playing once is the declined one — the trigger has to resolve and
// move nothing rather than reach for a target that was never chosen.
