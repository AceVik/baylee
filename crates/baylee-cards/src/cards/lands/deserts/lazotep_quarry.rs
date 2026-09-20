//! Lazotep Quarry — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a creature: Add one mana of any color.
//! Oracle: {X}{2}, {T}, Sacrifice a Desert: Exile target creature card with mana value X from your graveyard. Create a token that's a copy of it, except it's a 4/4 black Zombie. Activate only as a sorcery.
//! Set: PLST #M3C-131 — The List | Scryfall ID: 22bf056b-24bb-4f32-93af-088817f42ce8 | Oracle ID: 0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169
// PARTIAL — both mana abilities are built; the {X}{2} reanimation ability is
// not expressible and is left off (see the coverage reason).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAZOTEP_QUARRY,
    oracle_id = "0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169",
    scryfall_id = "22bf056b-24bb-4f32-93af-088817f42ce8",
    faces = &[face!(
        name = "Lazotep Quarry",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "the {X}{2} ability: its target is a creature card whose mana value is the announced X, \
         and no Filter compares a mana value against X (CmcAtMost/CmcAtLeast take a constant); \
         Effect::CreateTokenCopyOf copies a permanent and not a card exiled from a graveyard; \
         and CopyMod carries neither SetPT nor SetColor, so \"except it's a 4/4 black Zombie\" \
         cannot be said. The whole ability is left off rather than half-built."
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::mana_of_any_color()]
        ),
        // NOT SUPPORTED: "{X}{2}, {T}, Sacrifice a Desert: Exile target creature card with
        // mana value X from your graveyard. Create a token that's a copy of it, except it's a
        // 4/4 black Zombie. Activate only as a sorcery."
    ],
);
