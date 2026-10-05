//! Enchanted Being — {1}{W}{W} — Creature — Human
//! Oracle: Prevent all combat damage that would be dealt to this creature by enchanted creatures.
//! Set: LEG #12 — Legends | Scryfall ID: 94c2880d-b37a-43ea-9fee-cd5a8ed75a7e | Oracle ID: c98b725e-ca16-4576-bf53-653d4028d861
// PARTIAL — the prevention is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENCHANTED_BEING,
    oracle_id = "c98b725e-ca16-4576-bf53-653d4028d861",
    scryfall_id = "94c2880d-b37a-43ea-9fee-cd5a8ed75a7e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Enchanted Being",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "the prevention is off the card: no static Modifier prevents damage \
         from a filtered source (`PreventDamageToIt` is combat-only, unfiltered \
         and reaches the affected object; `ProtectionFrom` also stops targeting \
         and blocking), and no `Filter` names an enchanted creature"
    ),
    // NOT SUPPORTED: "Prevent all combat damage that would be dealt to this
    // creature by enchanted creatures." — `Modifier::PreventDamageToIt`
    // prevents all combat damage to this creature whatever its source, so it
    // would also stop an unenchanted creature's damage; `Modifier::ProtectionFrom`
    // could name enchanted creatures and does prevent their damage, but it also
    // stops them targeting and blocking this creature, and no `Filter` matches
    // a creature that has an Aura attached (`Filter::IsAttached` is about the
    // creature itself being attached to something). No other static `Modifier`
    // reaches damage.
    abilities = &[],
);
