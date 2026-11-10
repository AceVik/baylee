//! Tolaria West — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: Transmute {1}{U}{U} ({1}{U}{U}, Discard this card: Search your library for a card with mana value 0, reveal it, put it into your hand, then shuffle. Transmute only as a sorcery.)
//! Set: TSR #286 — Time Spiral Remastered | Scryfall ID: b005eef6-75f3-454f-a42b-d851bc84ac4e | Oracle ID: 46eefc72-2d9e-4389-8ae1-26d9ee472b5c
// IMPLEMENTED — the tapped entry, {U} from the mana ability, and transmute
// as a hand-zone, sorcery-speed activation that discards this card and
// searches for a mana-value-0 card (SearchLibrary; the reveal and the
// shuffle are derived from the filter and the destination).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOLARIA_WEST,
    oracle_id = "46eefc72-2d9e-4389-8ae1-26d9ee472b5c",
    scryfall_id = "b005eef6-75f3-454f-a42b-d851bc84ac4e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tolaria West",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            cost!("{1}{U}{U}", DiscardSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::CmcAtMost(0),
                finds: &[Find::HAND],
                optional: false,
            }],
            zone = ActivationZone::Hand,
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
