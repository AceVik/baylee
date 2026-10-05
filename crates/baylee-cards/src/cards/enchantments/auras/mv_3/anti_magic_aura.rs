//! Anti-Magic Aura — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be the target of spells and can't be enchanted by other Auras.
//! Set: 5ED #72 — Fifth Edition | Scryfall ID: 54642f0e-2d5f-49eb-8181-054c84038072 | Oracle ID: 6f78c1e2-e38f-431b-8864-8aad982e9912
// PARTIAL — enchant creature and "can't be enchanted by other Auras" are
// written; "can't be the target of spells" is off the card (see NOT SUPPORTED
// below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANTI_MAGIC_AURA,
    oracle_id = "6f78c1e2-e38f-431b-8864-8aad982e9912",
    scryfall_id = "54642f0e-2d5f-49eb-8181-054c84038072",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "\"can't be the target of spells\" is dropped: `Modifier::CantBeTargetedBy` \
         is asked of the casting spell's card, which is still in its owner's hand \
         as targets are announced, so no filter names a spell"
    ),
    faces = &[face!(
        name = "Anti-Magic Aura",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Enchanted creature can't be the target of spells." —
    // `Modifier::CantBeTargetedBy` asks its filter of the spell's card or the
    // ability's source. At target announcement that card is still in its
    // owner's hand (it reaches the stack only after costs are paid), so
    // `Filter::InZone(ZoneRef::Stack)` does not match it, and every type or
    // zone filter that does match a spell also matches an ability's source,
    // which the card does not stop.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeEnchantedExceptSource
        ),
    ],
);
