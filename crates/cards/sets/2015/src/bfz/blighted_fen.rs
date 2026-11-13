//! Blighted Fen — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{B}, {T}, Sacrifice this land: Target opponent sacrifices a creature of their choice.
//! Set: BFZ #230 — Battle for Zendikar | Scryfall ID: 29d02950-cd50-4662-97af-3106598dc3c4 | Oracle ID: b8f3da11-7c8f-4846-98a6-204bfd8d572b
// IMPLEMENTED — {T} for {C}, and the {4}{B}/{T}/sacrifice activation against a
// chosen opponent (SacrificeFilter, who = Chosen).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLIGHTED_FEN,
    oracle_id = "b8f3da11-7c8f-4846-98a6-204bfd8d572b",
    scryfall_id = "29d02950-cd50-4662-97af-3106598dc3c4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Blighted Fen", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}{B}", TapSelf, SacrificeSelf),
            &[Effect::SacrificeFilter {
                who: PlayerRel::Chosen,
                filter: &Filter::CREATURE,
            }],
            target = Some(TargetSpec::AnyOpponent)
        ),
    ],
);
