//! Compost — {1}{G} — Enchantment
//! Oracle: Whenever a black card is put into an opponent's graveyard from anywhere, you may draw a card.
//! Set: 7ED #235 — Seventh Edition | Scryfall ID: 91fc8eca-7549-45b5-b3db-89a58d7d2a4a | Oracle ID: 4681bcc2-8a46-44cd-8b04-6ab140904dea
// PARTIAL: whenever a black permanent owned by an opponent dies, you may draw a card; triggering when a black card is put into an opponent's graveyard from a zone other than the battlefield is not in Trigger.

use baylee_cards_dsl::prelude::*;

static BLACK_OPPONENT_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::COMPOST,
    oracle_id = "4681bcc2-8a46-44cd-8b04-6ab140904dea",
    scryfall_id = "91fc8eca-7549-45b5-b3db-89a58d7d2a4a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "whenever a black permanent owned by an opponent dies, you may draw a card; triggering when a black card is put into an opponent's graveyard from a zone other than the battlefield is not in Trigger",
    ),
    faces = &[face!(
        name = "Compost",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&BLACK_OPPONENT_PERMANENT),
        &[Effect::MayDo {
            effects: &[Effect::draw(1)],
        }],
    )],
);
