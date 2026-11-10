//! Essence Vortex — {1}{U}{B} — Instant
//! Oracle: Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated.
//! Set: ICE #287 — Ice Age | Scryfall ID: fe07e496-5070-4116-a91a-a3bbe19c12af | Oracle ID: ea38a211-a7b0-49fe-b5dc-7eca7610cccb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static DESTROY_NO_REGEN: Effect = Effect::Destroy {
    target: TargetSpec::Object(&Filter::CREATURE),
    no_regen: true,
};

card!(
    index = index::ESSENCE_VORTEX,
    oracle_id = "ea38a211-a7b0-49fe-b5dc-7eca7610cccb",
    scryfall_id = "fe07e496-5070-4116-a91a-a3bbe19c12af",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Essence Vortex",
        mana_cost = mana!("{1}{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::ControllerOfTarget,
            life: Amount::TargetToughness,
            effect: &DESTROY_NO_REGEN,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
