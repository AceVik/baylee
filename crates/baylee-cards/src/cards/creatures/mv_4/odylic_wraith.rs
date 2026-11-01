//! Odylic Wraith — {3}{B} — Creature — Wraith
//! Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.)
//! Oracle: Whenever this creature deals damage to a player, that player discards a card.
//! Set: WTH #77 — Weatherlight | Scryfall ID: 3a3b7cd1-051c-43a8-b5f0-72a9d704efbc | Oracle ID: 613fa223-856e-49c4-8952-224895e00b5e
// IMPLEMENTED — Swampwalk and forces a player it deals damage to to discard a card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ODYLIC_WRAITH,
    oracle_id = "613fa223-856e-49c4-8952-224895e00b5e",
    scryfall_id = "3a3b7cd1-051c-43a8-b5f0-72a9d704efbc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SWAMPWALK,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Odylic Wraith",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WRAITH],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::SWAMPWALK,
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::DiscardForPlayers {
            who: PlayerRel::DamagedPlayer,
            count: 1,
        }],
    ),],
);
