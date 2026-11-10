//! Coils of the Medusa — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/-1.
//! Oracle: Sacrifice this Aura: Destroy all non-Wall creatures blocking enchanted creature.
//! Set: WTH #65 — Weatherlight | Scryfall ID: 502bfb38-4a37-4053-af20-d5606ffc67c8 | Oracle ID: 02a8a4e6-2081-4903-81ad-96e5a949dff8
// PARTIAL — enchants a creature and gives it +1/-1; the sacrifice ability is not implemented, because nothing can name "the creature the sacrificed Aura was attached to" to find the creatures blocking it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COILS_OF_THE_MEDUSA,
    oracle_id = "02a8a4e6-2081-4903-81ad-96e5a949dff8",
    scryfall_id = "502bfb38-4a37-4053-af20-d5606ffc67c8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "enchants a creature and gives it +1/-1; \"Sacrifice this Aura: Destroy all non-Wall creatures blocking enchanted creature\" is not implemented, because no filter or target can name the creature the sacrificed Aura was attached to (it would destroy every non-Wall blocker, not only those blocking enchanted creature)",
    ),
    faces = &[face!(
        name = "Coils of the Medusa",
        mana_cost = mana!("{1}{B}"),
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
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(1, -1)
        ),
        // NOT SUPPORTED: "Sacrifice this Aura: Destroy all non-Wall creatures
        // blocking enchanted creature." Only "all non-Wall creatures that
        // block" can be said, which also kills the blockers of the
        // controller's other attackers; a filter or target for the creature
        // the sacrificed Aura was attached to (CR 608.2h) is missing.
    ],
);
