//! Chaos Harlequin — {2}{R}{R} — Creature — Human
//! Oracle: {R}: Exile the top card of your library. If that card is a land card, this creature gets -4/-0 until end of turn. Otherwise, this creature gets +2/+0 until end of turn.
//! Set: ALL #69 — Alliances | Scryfall ID: ec7d7c80-4e3c-454e-b2ed-6f0436df19c9 | Oracle ID: c8e96a51-24c9-4955-8bf0-6ef448fa8011
// NOT SUPPORTED: exiling the top card of your library and giving -4/-0 instead if that card is a land card is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAOS_HARLEQUIN,
    oracle_id = "c8e96a51-24c9-4955-8bf0-6ef448fa8011",
    scryfall_id = "ec7d7c80-4e3c-454e-b2ed-6f0436df19c9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "'{R}: Exile the top card of your library. If that card is a land card, this creature gets -4/-0 until end of turn. Otherwise, this creature gets +2/+0 until end of turn' is not supported: no effect exiles the top card and then branches on whether it was a land card, and a bare +2/+0 for {R} would never give the -4/-0, so the ability is left off and the card is a 2/4 creature",
    ),
    faces = &[face!(
        name = "Chaos Harlequin",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "{R}: Exile the top card of your library. If that card is a land card, this creature gets -4/-0 until end of turn. Otherwise, this creature gets +2/+0 until end of turn." `ThisWay` counts nothing exiled from a library and no effect branches on the type of the card it just exiled; a flat "{R}: +2/+0" is always better than the printed ability and exiles nothing.
);
