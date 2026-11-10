//! Oboro, Palace in the Clouds — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}.
//! Oracle: {1}: Return Oboro to its owner's hand.
//! Set: SOK #164 — Saviors of Kamigawa | Scryfall ID: ffc2d68e-6543-43ec-b67a-afff1325a32f | Oracle ID: 645fb11b-d684-4bec-8532-8fa97e8f7b28
// IMPLEMENTED — {T} for {U}, and {1} to return the source itself to hand:
// the return is the effect on the right of the colon, and it names the
// source (`TargetSpec::ThisObject`) rather than a target, so nothing is
// chosen and hexproof has nothing to answer.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OBORO_PALACE_IN_THE_CLOUDS,
    oracle_id = "645fb11b-d684-4bec-8532-8fa97e8f7b28",
    scryfall_id = "ffc2d68e-6543-43ec-b67a-afff1325a32f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Oboro, Palace in the Clouds",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(cost!("{1}"), &[Effect::bounce(TargetSpec::ThisObject)]),
    ],
);
