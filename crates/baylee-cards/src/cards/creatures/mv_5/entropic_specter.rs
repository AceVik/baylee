//! Entropic Specter — {3}{B}{B} — Creature — Specter Spirit
//! Oracle: Flying
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Entropic Specter's power and toughness are each equal to the number of cards in the chosen player's hand.
//! Oracle: Whenever this creature deals damage to a player, that player discards a card.
//! Set: EXO #61 — Exodus | Scryfall ID: bdb04d81-b0ab-4bc7-935d-c31005887240 | Oracle ID: a9c44443-85c7-4d0a-82ab-36137151e4e5

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENTROPIC_SPECTER,
    oracle_id = "a9c44443-85c7-4d0a-82ab-36137151e4e5",
    scryfall_id = "bdb04d81-b0ab-4bc7-935d-c31005887240",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Characteristic-defining power and toughness equal to the number of cards in the chosen player's hand is not supported; enters as a 1/1 flyer that chooses an opponent on entry and makes a player discard a card whenever it deals damage to them."
    ),
    faces = &[face!(
        name = "Entropic Specter",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER, subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::DiscardCount {
            who: PlayerRel::DamagedPlayer,
            count: Amount::Fixed(1),
        }],
    )],
);
