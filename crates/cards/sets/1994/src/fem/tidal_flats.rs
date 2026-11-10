//! Tidal Flats — {U} — Enchantment
//! Oracle: {U}{U}: For each attacking creature without flying, its controller may pay {1}. If that player doesn't, creatures you control blocking that creature gain first strike until end of turn.
//! Set: FEM #27a — Fallen Empires | Scryfall ID: 2e820f3f-434e-4d09-91b9-0ebd6966b393 | Oracle ID: bf39d0f7-f689-45fd-b37b-12d1618fd6e7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const ATTACKING_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);
const YOUR_BLOCKERS_OF_IT: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::BlockingReference]);

card!(
    index = index::TIDAL_FLATS,
    oracle_id = "bf39d0f7-f689-45fd-b37b-12d1618fd6e7",
    scryfall_id = "2e820f3f-434e-4d09-91b9-0ebd6966b393",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tidal Flats",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{U}{U}"),
        &[Effect::ForEachMatching {
            filter: &ATTACKING_WITHOUT_FLYING,
            effects: &[Effect::PlayerMayPayOr {
                player: PlayerRel::EventPlayer,
                mana: Amount::Fixed(1),
                effect: &Effect::PumpFilter {
                    filter: &YOUR_BLOCKERS_OF_IT,
                    controlled_by: Some(PlayerRel::You),
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FIRST_STRIKE,
                    duration: Duration::UntilEndOfTurn,
                },
            }],
        }],
    ),],
);
