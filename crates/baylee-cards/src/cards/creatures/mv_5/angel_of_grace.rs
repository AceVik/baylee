//! Angel of Grace — {3}{W}{W} — Creature — Angel
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, until end of turn, damage that would reduce your life total to less than 1 reduces it to 1 instead.
//! Oracle: {4}{W}{W}, Exile this card from your graveyard: Your life total becomes 10.
//! Set: RNA #1 — Ravnica Allegiance | Scryfall ID: 80164e61-3e94-4e10-9bd1-518b8dc7fc4c | Oracle ID: fcfcac6b-6d26-4ffe-ae9e-b0ee7fa49787
// IMPLEMENTED — flash/flying, turn-limited life floor, and graveyard life reset.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANGEL_OF_GRACE,
    oracle_id = "fcfcac6b-6d26-4ffe-ae9e-b0ee7fa49787",
    scryfall_id = "80164e61-3e94-4e10-9bd1-518b8dc7fc4c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING.union(KeywordSet::FLASH),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of Grace",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::DamageLifeLossFloor {
                    players: PlayerRel::You,
                    minimum: 1,
                },
                duration: Duration::UntilEndOfTurn,
            }]
        ),
        activated!(
            cost!("{4}{W}{W}", ExileSelf),
            &[Effect::SetLifeTotal {
                who: PlayerRel::You,
                amount: Amount::Fixed(10),
            }],
            zone = ActivationZone::Graveyard,
        ),
    ],
);
