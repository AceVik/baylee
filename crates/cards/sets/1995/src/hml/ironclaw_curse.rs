//! Ironclaw Curse — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -0/-1.
//! Oracle: Enchanted creature can't block creatures with power equal to or greater than the enchanted creature's toughness.
//! Set: 5ED #244 — Fifth Edition | Scryfall ID: 6ed35d03-80fd-421d-a8be-6708a5570a85 | Oracle ID: 51708bd1-23db-4597-84a7-f36abd93c537
// PARTIAL — blocking restriction comparing attacker power to enchanted creature's toughness is not in Filter; enchanted creature gets -0/-1.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IRONCLAW_CURSE,
    oracle_id = "51708bd1-23db-4597-84a7-f36abd93c537",
    scryfall_id = "6ed35d03-80fd-421d-a8be-6708a5570a85",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "blocking restriction comparing attacker power to enchanted creature's toughness is not in Filter; enchanted creature gets -0/-1"
    ),
    faces = &[face!(
        name = "Ironclaw Curse",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(0, -1)),
    ],
);
