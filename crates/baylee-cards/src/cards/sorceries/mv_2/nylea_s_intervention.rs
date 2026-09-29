//! Nylea's Intervention — {X}{G}{G} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Search your library for up to X land cards, reveal them, put them into your hand, then shuffle.
//! Oracle: • Nylea's Intervention deals twice X damage to each creature with flying.
//! Set: THB #188 — Theros Beyond Death | Scryfall ID: daa2f963-9d16-4224-b24e-b6a79f2b9d75 | Oracle ID: acf388b2-c4e3-4f1b-a16c-88f991d5c17b
// IMPLEMENTED — both modes: up to X lands to hand, twice X to each flier.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NYLEA_S_INTERVENTION,
    oracle_id = "acf388b2-c4e3-4f1b-a16c-88f991d5c17b",
    scryfall_id = "daa2f963-9d16-4224-b24e-b6a79f2b9d75",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nylea's Intervention",
        mana_cost = mana!("{X}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        modes: &[
            // "Search your library for up to X land cards, reveal them, put
            // them into your hand, then shuffle." The reveal is the search's
            // own rule: narrower than "a card", ending in a hand.
            mode!(&[Effect::SearchLibraryUpTo {
                filter: &Filter::LAND,
                count: Amount::X,
                find: &Find::HAND,
            }]),
            // "Nylea's Intervention deals twice X damage to each creature
            // with flying."
            mode!(&[Effect::DealDamageEach {
                amount: Amount::DoubleX,
                filter: &Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]),
            }]),
        ],
    }],
);
