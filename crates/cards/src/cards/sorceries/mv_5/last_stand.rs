//! Last Stand — {W}{U}{B}{R}{G} — Sorcery
//! Oracle: Target opponent loses 2 life for each Swamp you control. Last Stand deals damage to target creature equal to the number of Mountains you control. Create a 1/1 green Saproling creature token for each Forest you control. You gain 2 life for each Plains you control. Draw a card for each Island you control, then discard that many cards.
//! Set: PCA #100 — Planechase Anthology | Scryfall ID: d7f3c92b-5a31-450e-8cff-9289d266050a | Oracle ID: 4d2a465e-9ebd-4002-b6cd-e0eab08bad54
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAST_STAND,
    oracle_id = "4d2a465e-9ebd-4002-b6cd-e0eab08bad54",
    scryfall_id = "d7f3c92b-5a31-450e-8cff-9289d266050a",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "Last Stand",
        mana_cost = mana!("{W}{U}{B}{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
