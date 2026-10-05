//! Power Artifact — {U}{U} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: Enchanted artifact's activated abilities cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.
//! Set: ME4 #57 — Masters Edition IV | Scryfall ID: c7b0c6c4-ec99-4af1-9527-41c428af7260 | Oracle ID: 9a8c8d52-1701-4ae0-9d2d-cce4e476673b
// PARTIAL — enchant artifact is written; the cost reduction is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POWER_ARTIFACT,
    oracle_id = "9a8c8d52-1701-4ae0-9d2d-cce4e476673b",
    scryfall_id = "c7b0c6c4-ec99-4af1-9527-41c428af7260",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no modifier reduces another object's activated-ability mana cost \
         (`Modifier::AbilitiesCostMore` only raises it) and none imposes the \
         one-mana floor, so the reduction sentence is off the card"
    ),
    faces = &[face!(
        name = "Power Artifact",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Enchanted artifact's activated abilities cost {2} less to
    // activate. This effect can't reduce the mana in that cost to less than one
    // mana." — `Modifier` has `AbilitiesCostMore` and no reduction, and
    // `AbilityDef::Activated.cost_reduction` is the ability's own, not a static
    // reaching another object.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::ARTIFACT)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
    )],
);
