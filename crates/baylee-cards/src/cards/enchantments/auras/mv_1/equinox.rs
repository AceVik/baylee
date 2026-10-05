//! Equinox — {W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Counter target spell if it would destroy a land you control."
//! Set: LEG #13 — Legends | Scryfall ID: 840c6586-a7a9-4ae8-96be-a995a0693eb6 | Oracle ID: 0fa2cb01-476e-4e82-94e6-9639e53a7743
// PARTIAL — enchant land is written; the granted counter ability is off the
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EQUINOX,
    oracle_id = "0fa2cb01-476e-4e82-94e6-9639e53a7743",
    scryfall_id = "840c6586-a7a9-4ae8-96be-a995a0693eb6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Modifier::GrantActivated carries no target requirement, so the \
         granted \"{T}: Counter target spell\" cannot be aimed, and no \
         predicate can ask whether a spell would destroy a land you control"
    ),
    faces = &[face!(
        name = "Equinox",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Enchanted land has \"{T}: Counter target spell if it
    // would destroy a land you control.\"" — `Modifier::GrantActivated
    // { cost, effects, mana_ability }` has no targets field, so
    // `Effect::CounterTargetSpell` would find nothing to counter; and no
    // `Condition` or `Filter` can ask whether a spell "would destroy a land
    // you control".
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::LAND)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
