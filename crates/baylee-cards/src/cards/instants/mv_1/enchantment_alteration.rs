//! Enchantment Alteration — {U} — Instant
//! Oracle: Attach target Aura attached to a creature or land to another permanent of that type.
//! Set: USG #72 — Urza's Saga | Scryfall ID: 254aa8d0-f0f5-4fb2-a6ba-07453d71e229 | Oracle ID: c80935bf-c17f-4940-ad9f-3f4e3e8f71bc
// PARTIAL — nothing is built: moving another Aura cannot be stated.
// NOT SUPPORTED: "Attach target Aura attached to a creature or land to another
// permanent of that type." — the only attachment effects are
// `Effect::AttachSelf`, which attaches the source, and
// `Effect::DestroyEventThenMayReattach`, which moves the source Aura after its
// host is destroyed; neither takes the Aura and the new host as two targets.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENCHANTMENT_ALTERATION,
    oracle_id = "c80935bf-c17f-4940-ad9f-3f4e3e8f71bc",
    scryfall_id = "254aa8d0-f0f5-4fb2-a6ba-07453d71e229",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Enchantment Alteration",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "no effect attaches a targeted Aura to a second target: AttachSelf \
         attaches the source and DestroyEventThenMayReattach is the source \
         Aura's own reattach after a destroyed host"
    ),
);
