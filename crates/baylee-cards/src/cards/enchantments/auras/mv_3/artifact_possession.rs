//! Artifact Possession — {2}{B} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: Whenever enchanted artifact becomes tapped or a player activates an ability of enchanted artifact without {T} in its activation cost, this Aura deals 2 damage to that artifact's controller.
//! Set: ATQ #15 — Antiquities | Scryfall ID: 587d6ac8-fad8-49e0-862e-636e06628ff9 | Oracle ID: 5c641df8-97d7-484b-8d0e-790279fd6177
// PARTIAL — the "becomes tapped" half is written; the "activates an ability
// without {T}" half is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARTIFACT_POSSESSION,
    oracle_id = "5c641df8-97d7-484b-8d0e-790279fd6177",
    scryfall_id = "587d6ac8-fad8-49e0-862e-636e06628ff9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no trigger hears an activated ability being activated: only \
         `Trigger::TappedForMana` observes an activation, and only of a mana \
         ability that tapped its source, so the {T}-less-activation half is off"
    ),
    faces = &[face!(
        name = "Artifact Possession",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "…or a player activates an ability of enchanted artifact
    // without {T} in its activation cost" — no `Trigger` variant hears an
    // activated ability; `Trigger::TappedForMana` is mana abilities only and
    // requires the activation to have tapped the source.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ARTIFACT)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::ControllerOfEvent)
            }]
        ),
    ],
);
