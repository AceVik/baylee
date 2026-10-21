//! Giant Albatross — {1}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: When this creature dies, you may pay {1}{U}. If you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life. A creature destroyed this way can't be regenerated.
//! Set: HML #27a — Homelands | Scryfall ID: bce05870-74d3-43f1-92d0-dc1744c0138d | Oracle ID: 8b59cacb-46ff-4400-a7f6-b258c81a9b14
// PARTIAL — per-creature controller 2-life payment to prevent destruction is not in Effect; when Giant Albatross dies, you may pay {1}{U} to destroy without regeneration each creature that dealt damage to it this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEALT_DAMAGE_TO_IT: Filter =
    Filter::And(&[Filter::CREATURE, Filter::DealtDamageToSourceThisTurn]);

card!(
    index = index::GIANT_ALBATROSS,
    oracle_id = "8b59cacb-46ff-4400-a7f6-b258c81a9b14",
    scryfall_id = "bce05870-74d3-43f1-92d0-dc1744c0138d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "per-creature controller 2-life payment to prevent destruction is not in Effect; when Giant Albatross dies, you may pay {1}{U} to destroy without regeneration each creature that dealt damage to it this turn"
    ),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Giant Albatross",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{1}{U}"),
            effects: &[Effect::DestroyAll {
                filter: &DEALT_DAMAGE_TO_IT,
                no_regen: true,
            }],
        }]
    )],
);
