//! Earthbind — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains "Enchanted creature loses flying."
//! Set: SUM #147 — Summer Magic / Edgar | Scryfall ID: 784cc686-e5a2-4835-8ff2-820461868ffa | Oracle ID: e8e35b49-8cfb-4fb5-89aa-8050f15b11bf
// PARTIAL — the enter trigger's condition and the ability the Aura gains are not
// read; the Aura attaches and does nothing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EARTHBIND,
    oracle_id = "e8e35b49-8cfb-4fb5-89aa-8050f15b11bf",
    scryfall_id = "784cc686-e5a2-4835-8ff2-820461868ffa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "the enter trigger's condition and the ability the Aura gains are not read; the Aura attaches and does nothing"
    ),
    faces = &[face!(
        name = "Earthbind",
        mana_cost = mana!("{R}"),
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
        // NOT SUPPORTED: When this Aura enters, if enchanted creature has flying, this
        // Aura deals 2 damage to that creature and this Aura gains "Enchanted creature
        // loses flying."
    ],
);
