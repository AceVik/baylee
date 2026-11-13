//! Gaea's Touch — {G}{G} — Enchantment
//! Oracle: {0}: You may put a basic Forest card from your hand onto the battlefield. Activate only as a sorcery and only once each turn.
//! Oracle: Sacrifice this enchantment: Add {G}{G}.
//! Set: ME3 #120 — Masters Edition III | Scryfall ID: 2f23950c-f583-4559-8c3b-bd28bedecf80 | Oracle ID: ddd053ea-5b91-4def-a003-2e297a170d99

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BASIC_FOREST: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(subtypes::land::FOREST),
]);

card!(
    index = index::GAEA_S_TOUCH,
    oracle_id = "ddd053ea-5b91-4def-a003-2e297a170d99",
    scryfall_id = "2f23950c-f583-4559-8c3b-bd28bedecf80",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gaea's Touch",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        activated!(
            Cost::FREE,
            &[Effect::PutFromHandOntoBattlefield {
                tapped: false,
                filter: &BASIC_FOREST,
                mana_value: None,
                optional: true,
            }],
            timing = ActivationTiming::SorcerySpeed,
            limit = ActivationLimit::PerTurn(1),
        ),
        mana_ability!(cost!(SacrificeSelf), &[Effect::mana(ManaColor::Green, 2)]),
    ],
);
