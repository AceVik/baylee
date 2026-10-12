//! Sunken Palace — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {1}{U}, {T}, Exile seven cards from your graveyard: Add {U}. When you spend this mana to cast a spell or activate an ability, copy that spell or ability. You may choose new targets for the copy. (Mana abilities can't be copied.)
//! Set: M3C #81 — Modern Horizons 3 Commander | Scryfall ID: e44ee47c-95de-4090-97b6-188585d86b0c | Oracle ID: c098c507-5154-423a-a70b-f6dfd4959cf6

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static COPY_MANA: &[Effect] = &[Effect::AddMana {
    source: ManaSource::Fixed(ManaColor::Blue),
    amount: Amount::Fixed(1),
    distribution: ManaDistribution::OneColor,
    restriction: Some(ManaRestriction {
        uses: ManaUse::SpellsOrAbilities,
        filter: &Filter::Any,
        ability_filter: None,
        rider: SpendRider::Copy,
        restricts: false,
        until_end_of_turn: false,
    }),
}];

card!(
    index = index::SUNKEN_PALACE,
    oracle_id = "c098c507-5154-423a-a70b-f6dfd4959cf6",
    scryfall_id = "e44ee47c-95de-4090-97b6-188585d86b0c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sunken Palace",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped]
    )],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            Cost {
                mana: mana!("{1}{U}"),
                parts: &[
                    CostPart::TapSelf,
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                    CostPart::ExileFromGraveyard(&Filter::Any),
                ]
            },
            COPY_MANA,
            mana_ability = true
        ),
    ],
);
