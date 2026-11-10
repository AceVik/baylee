//! Kithkin Armor — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by creatures with power 3 or greater.
//! Oracle: Sacrifice this Aura: The next time a source of your choice would deal damage to enchanted creature this turn, prevent that damage.
//! Set: WTH #19 — Weatherlight | Scryfall ID: 395e7882-0429-46aa-8e38-be707067c588 | Oracle ID: 35e2730e-b3aa-4080-aae8-7b3ad1e59848
// PARTIAL — enchant creature and "can't be blocked by creatures with power 3 or greater". NOT SUPPORTED: "Sacrifice this Aura: The next time a source of your choice would deal damage to enchanted creature this turn, prevent that damage" — no effect prevents only the next damage from one chosen source to the enchanted creature, and `PreventDamageFrom(Any)` would prevent all damage from every source this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static POWER_3_OR_MORE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtLeast(3)]);

card!(
    index = index::KITHKIN_ARMOR,
    oracle_id = "35e2730e-b3aa-4080-aae8-7b3ad1e59848",
    scryfall_id = "395e7882-0429-46aa-8e38-be707067c588",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "enchants a creature, which cannot be blocked by creatures with power 3 or greater; the sacrifice ability (prevent the next damage a chosen source would deal to the enchanted creature this turn) is left off because no effect prevents only the next damage from one chosen source to a creature",
    ),
    faces = &[face!(
        name = "Kithkin Armor",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::CantBeBlockedBy(&POWER_3_OR_MORE)
        ),
    ],
);
