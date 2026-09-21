//! Soldevi Excavations — (no cost) — Land
//! Oracle: If this land would enter, sacrifice an untapped Island instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {C}{U}.
//! Oracle: {1}, {T}: Scry 1.
//! Set: ME2 #236 — Masters Edition II | Scryfall ID: 4678132d-85bc-49cc-b3be-62cf709de42b | Oracle ID: 5baa7abe-5bdf-40ce-9a83-a93b7cae71a3
// PARTIAL — {T}: Add {C}{U} (two AddMana effects, one mana ability) and
// {1}, {T}: Scry 1 are built. The entry replacement is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLDEVI_EXCAVATIONS,
    oracle_id = "5baa7abe-5bdf-40ce-9a83-a93b7cae71a3",
    scryfall_id = "4678132d-85bc-49cc-b3be-62cf709de42b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the entry replacement — sacrifice an untapped Island or this goes to its owner's graveyard — has no DSL shape: no EnterModifier sacrifices a permanent and no ReplacementRule covers an entry"
    ),
    // NOT SUPPORTED: "If this land would enter, sacrifice an untapped Island
    // instead. If you do, put this land onto the battlefield. If you don't,
    // put it into its owner's graveyard." — the land therefore enters
    // unconditionally.
    faces = &[face!(name = "Soldevi Excavations", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Colorless, 1),
            Effect::mana(ManaColor::Blue, 1),
        ]),
        activated!(cost!("{1}", TapSelf), &[Effect::scry(1)]),
    ],
);
