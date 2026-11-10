//! Sword of the Ages — {6} — Artifact
//! Oracle: This artifact enters tapped.
//! Oracle: {T}, Sacrifice this artifact and any number of creatures you control: This artifact deals X damage to any target, where X is the total power of the creatures sacrificed this way, then exile this artifact and those creature cards.
//! Set: ME3 #202 — Masters Edition III | Scryfall ID: aad10ff3-6d8d-4de4-9c30-1ae1aec85e5e | Oracle ID: 201f2434-96b3-408c-a5ce-74d0675920ed
// IMPLEMENTED — enters tapped; {T}, sacrifice it and X creatures: X damage
// where X is their total power, then exile it and those cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SWORD_OF_THE_AGES,
    oracle_id = "201f2434-96b3-408c-a5ce-74d0675920ed",
    scryfall_id = "aad10ff3-6d8d-4de4-9c30-1ae1aec85e5e",
    faces = &[face!(
        name = "Sword of the Ages",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf, SacrificeX(&Filter::YOUR_CREATURE)),
        &[
            Effect::DealDamage {
                amount: Amount::SacrificedTotalPower,
                target: TargetSpec::AnyTarget
            },
            Effect::ExileSourceAndSacrificed,
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
