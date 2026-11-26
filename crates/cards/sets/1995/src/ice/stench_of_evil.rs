//! Stench of Evil — {2}{B}{B} — Sorcery
//! Oracle: Destroy all Plains. For each land destroyed this way, Stench of Evil deals 1 damage to that land's controller unless they pay {2}.
//! Set: ICE #165 — Ice Age | Scryfall ID: 4c7065a2-f819-4cbe-b453-a55e904f0461 | Oracle ID: 14062abe-14df-4dbe-ad7b-530e5f6c4988
// IMPLEMENTED — destroys all Plains and taxes or pings each destroyed land's controller for each land destroyed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PLAINS: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::PLAINS)]);
static TAX_OR_DAMAGE: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::EventPlayer,
    mana: Amount::Fixed(2),
    effect: &Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::Player(PlayerRel::EventPlayer),
    },
}];

card!(
    index = index::STENCH_OF_EVIL,
    oracle_id = "14062abe-14df-4dbe-ad7b-530e5f6c4988",
    scryfall_id = "4c7065a2-f819-4cbe-b453-a55e904f0461",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Stench of Evil",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyAllThenForEachDestroyed {
        filter: &PLAINS,
        no_regen: false,
        effects: TAX_OR_DAMAGE,
    }])],
);
