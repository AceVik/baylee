//! An-Havva Inn — {1}{G}{G} — Sorcery
//! Oracle: You gain X plus 1 life, where X is the number of green creatures on the battlefield.
//! Set: HML #82 — Homelands | Scryfall ID: eff4531f-d19d-44af-861a-33087197d21c | Oracle ID: 3cd72848-7b99-4d46-96f5-53f297c9d2a0
// IMPLEMENTED — gains 1 plus the number of green creatures on the battlefield life.

use baylee_cards_dsl::prelude::*;

static GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);
static GREEN_CREATURE_COUNT: Amount = Amount::CountOf {
    filter: &GREEN_CREATURE,
    zone: ZoneSel::Battlefield,
};

card!(
    index = index::AN_HAVVA_INN,
    oracle_id = "3cd72848-7b99-4d46-96f5-53f297c9d2a0",
    scryfall_id = "eff4531f-d19d-44af-861a-33087197d21c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "An-Havva Inn",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::Plus {
            base: &GREEN_CREATURE_COUNT,
            offset: 1,
        },
    }])],
);
