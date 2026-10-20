//! Dreams of the Dead — {3}{U} — Enchantment
//! Oracle: {1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains "Cumulative upkeep {2}." If the creature would leave the battlefield, exile it instead of putting it anywhere else. (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)
//! Set: ME2 #46 — Masters Edition II | Scryfall ID: acff5d75-3d24-426d-9254-f17269aff9d3 | Oracle ID: 3b6cf935-3b99-4371-a797-6f40b89adeeb
// PARTIAL: granted cumulative upkeep {2} and leave-battlefield exile replacement on the returned creature not supported.

use baylee_cards_dsl::prelude::*;

const WHITE_OR_BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White, Color::Black])),
]);
const TARGET_CREATURE_IN_YOUR_GRAVEYARD: TargetSpec =
    TargetSpec::CardInGraveyard(&WHITE_OR_BLACK_CREATURE, PlayerRel::You);

// NOT SUPPORTED: That creature gains "Cumulative upkeep {2}." If the creature would leave the battlefield, exile it instead of putting it anywhere else.
card!(
    index = index::DREAMS_OF_THE_DEAD,
    oracle_id = "3b6cf935-3b99-4371-a797-6f40b89adeeb",
    scryfall_id = "acff5d75-3d24-426d-9254-f17269aff9d3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "granted cumulative upkeep {2} and leave-battlefield exile replacement on the returned creature not supported"
    ),
    faces = &[face!(
        name = "Dreams of the Dead",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}{U}"),
        &[Effect::GraveyardToBattlefield {
            target: TARGET_CREATURE_IN_YOUR_GRAVEYARD,
            owner_control: false,
            counters: None,
        }],
        target = Some(TARGET_CREATURE_IN_YOUR_GRAVEYARD),
    )],
);
