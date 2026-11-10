//! Mortuary Mire — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, you may put target creature card from your graveyard on top of your library.
//! Oracle: {T}: Add {B}.
//! Set: CLB #900 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 058f30e5-64a9-4d6b-b7a6-0fd95d460cae | Oracle ID: 1b3fb20a-e090-4286-9c03-6b71c27c45be
// IMPLEMENTED — enters tapped; the enter trigger is "you may", which the
// target requirement already offers (min 0), and it puts the card on top of
// its owner's library; {T}: Add {B}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MORTUARY_MIRE,
    oracle_id = "1b3fb20a-e090-4286-9c03-6b71c27c45be",
    scryfall_id = "058f30e5-64a9-4d6b-b7a6-0fd95d460cae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mortuary Mire",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            )))
        ),
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    ],
);
