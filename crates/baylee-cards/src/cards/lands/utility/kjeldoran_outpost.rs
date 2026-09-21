//! Kjeldoran Outpost — (no cost) — Land
//! Oracle: If this land would enter, sacrifice a Plains instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}: Create a 1/1 white Soldier creature token.
//! Set: VMA #301 — Vintage Masters | Scryfall ID: db90c357-26e3-4ab2-a280-0d8d74b95048 | Oracle ID: 8b370db5-dfb9-4ea0-9017-bae3e767b041
// PARTIAL — {T}: Add {W} and {1}{W}, {T}: create a 1/1 white Soldier are
// built; the enter replacement is not expressible, see NOT SUPPORTED below.

use crate::tokens::SOLDIER_1_1_WHITE;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::KJELDORAN_OUTPOST,
    oracle_id = "8b370db5-dfb9-4ea0-9017-bae3e767b041",
    scryfall_id = "db90c357-26e3-4ab2-a280-0d8d74b95048",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Kjeldoran Outpost", types = TypeSet::LAND,)],
    // NOT SUPPORTED: If this land would enter, sacrifice a Plains instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
    coverage = Coverage::Partial(
        "the enter replacement — sacrifice a Plains instead, or this land goes \
         to its owner's graveyard — has no EnterModifier variant",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf),
            &[Effect::CreateToken {
                token: &SOLDIER_1_1_WHITE,
            }]
        ),
    ],
);
