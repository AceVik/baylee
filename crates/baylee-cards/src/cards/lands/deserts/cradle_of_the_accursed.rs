//! Cradle of the Accursed — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice this land: Create a 2/2 black Zombie creature token. Activate only as a sorcery.
//! Set: AKH #241 — Amonkhet | Scryfall ID: 41713e82-c3d3-4c2f-b075-f684cbd68ce8 | Oracle ID: 36d06c91-5080-4f97-8e4c-ca8ac390e808
// PARTIAL — {T}: Add {C} is built. The token half of the second ability is
// off the card: it needs a 2/2 black Zombie `TokenDef` with an id in
// `crate::tokens::ALL`, the pool's registry has none, and a card file may
// not define one (`no_card_file_defines_its_own_token`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRADLE_OF_THE_ACCURSED,
    oracle_id = "36d06c91-5080-4f97-8e4c-ca8ac390e808",
    scryfall_id = "41713e82-c3d3-4c2f-b075-f684cbd68ce8",
    faces = &[face!(
        name = "Cradle of the Accursed",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "\"{3}, {T}, Sacrifice this land: Create a 2/2 black Zombie creature \
         token\" is not built: Effect::CreateToken needs a &'static TokenDef \
         whose index in crate::tokens::ALL is its art key, no 2/2 black Zombie \
         is in that registry, and a card file may not define a TokenDef itself"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{3}, {T}, Sacrifice this land: Create a 2/2 black
        // Zombie creature token. Activate only as a sorcery." — the cost
        // (cost!("{3}", TapSelf, SacrificeSelf)) and the sorcery timing are
        // both sayable; the effect is not, because the token it makes does
        // not exist in the shared registry and a literal here would leave it
        // with no id (`token_id` answers u16::MAX) and therefore no art.
    ],
);
