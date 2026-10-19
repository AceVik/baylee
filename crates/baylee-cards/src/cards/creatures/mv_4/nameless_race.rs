//! Nameless Race — {3}{B} — Creature
//! Oracle: Trample
//! Oracle: As this creature enters, pay any amount of life. The amount you pay can't be more than the total number of white nontoken permanents your opponents control plus the total number of white cards in their graveyards.
//! Oracle: Nameless Race's power and toughness are each equal to the life paid as it entered.
//! Set: DRK #50 — The Dark | Scryfall ID: 348a467a-4661-4fdb-af1d-9171a1a930d9 | Oracle ID: 35a717ea-bd32-4714-b402-de7f15398e52

use baylee_cards_dsl::prelude::*;

static OPPONENT_WHITE_NONTOKEN_PERMANENT: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::Not(&Filter::IsToken),
]);

static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

card!(
    index = index::NAMELESS_RACE,
    oracle_id = "35a717ea-bd32-4714-b402-de7f15398e52",
    scryfall_id = "348a467a-4661-4fdb-af1d-9171a1a930d9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nameless Race",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::PayLifeUpTo(Amount::Sum {
            left: &Amount::CountOf {
                zone: ZoneSel::Battlefield,
                filter: &OPPONENT_WHITE_NONTOKEN_PERMANENT,
            },
            right: &Amount::CountOf {
                zone: ZoneSel::GraveyardOpponents,
                filter: &WHITE_CARD,
            },
        })],
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::EntrySacrifices,
            toughness_plus: 0,
        }
    )],
);
