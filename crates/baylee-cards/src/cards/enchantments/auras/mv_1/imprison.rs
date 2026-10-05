//! Imprison — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever a player activates an ability of enchanted creature with {T} in its activation cost that isn't a mana ability, you may pay {1}. If you do, counter that ability. If you don't, destroy this Aura.
//! Oracle: Whenever enchanted creature attacks or blocks, you may pay {1}. If you do, tap the creature, remove it from combat, and creatures it was blocking that had become blocked by only that creature this combat become unblocked. If you don't, destroy this Aura.
//! Set: LEG #107 — Legends | Scryfall ID: 12671381-beb7-41b8-9484-97f8aca5c981 | Oracle ID: 632de66b-2314-4299-847c-16a84bf9121f
// PARTIAL — enchant creature is written; both triggered abilities are off the
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IMPRISON,
    oracle_id = "632de66b-2314-4299-847c-16a84bf9121f",
    scryfall_id = "12671381-beb7-41b8-9484-97f8aca5c981",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no Trigger hears a filtered creature's ability being activated, and \
         no Effect removes a creature from combat or unblocks, so both \
         triggered abilities are off the card"
    ),
    faces = &[face!(
        name = "Imprison",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Whenever a player activates an ability of enchanted
    // creature with {T} in its activation cost that isn't a mana ability, you
    // may pay {1}. If you do, counter that ability. If you don't, destroy this
    // Aura." — no `Trigger` listens for a permanent's activated ability
    // (`Trigger::TappedForMana` hears mana abilities alone), so the
    // pay-or-destroy and the counter have no event to ride on.
    // NOT SUPPORTED: "Whenever enchanted creature attacks or blocks, you may
    // pay {1}. If you do, tap the creature, remove it from combat, and
    // creatures it was blocking that had become blocked by only that creature
    // this combat become unblocked. If you don't, destroy this Aura." — the
    // attack half is `Trigger::Attacks(&Filter::AttachedToBySource)` but the
    // block half has no trigger at all (`Trigger::BlocksOrBecomesBlockedBy`
    // is about the ability's own source, and the Aura never blocks), so the
    // "or blocks" trigger cannot be written; and no Effect removes a creature
    // from combat or unblocks what it was blocking.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
