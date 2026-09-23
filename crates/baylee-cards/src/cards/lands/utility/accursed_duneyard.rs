//! Accursed Duneyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Regenerate target Shade, Skeleton, Specter, Spirit, Vampire, Wraith, or Zombie. (The next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: DRC #20 — Aetherdrift Commander | Scryfall ID: bd9e6ba8-1c5e-4416-8bff-90db3b3b1f41 | Oracle ID: 48edc348-93f6-4dce-9cc4-7244d76b6f4a
// IMPLEMENTED — {T}: Add {C}, and the {2}, {T} regeneration shield for one
// of seven creature types.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// The seven the card prints, in the card's own order.
static UNDEAD: Filter = Filter::Or(&[
    Filter::HasSubtype(creature::SHADE),
    Filter::HasSubtype(creature::SKELETON),
    Filter::HasSubtype(creature::SPECTER),
    Filter::HasSubtype(creature::SPIRIT),
    Filter::HasSubtype(creature::VAMPIRE),
    Filter::HasSubtype(creature::WRAITH),
    Filter::HasSubtype(creature::ZOMBIE),
]);

card!(
    index = index::ACCURSED_DUNEYARD,
    oracle_id = "48edc348-93f6-4dce-9cc4-7244d76b6f4a",
    scryfall_id = "bd9e6ba8-1c5e-4416-8bff-90db3b3b1f41",
    faces = &[face!(name = "Accursed Duneyard", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::regenerate(TargetSpec::Object(&UNDEAD))],
            target = Some(TargetSpec::Object(&UNDEAD))
        ),
    ],
);
