//! Angel of the Ruins — {5}{W}{W} — Artifact Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature enters, exile up to two target artifacts and/or enchantments.
//! Oracle: Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)
//! Set: EOC #63 — Edge of Eternities Commander | Scryfall ID: 4ad8d899-5901-4fc8-b4f0-adcc0d97fc6e | Oracle ID: 14878a5f-7983-4133-a1a7-9d539531e092
// IMPLEMENTED — two-target exile and plainscycling from hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);
static PLAINS: Filter = Filter::HasSubtype(subtypes::land::PLAINS);

card!(
    index = index::ANGEL_OF_THE_RUINS,
    oracle_id = "14878a5f-7983-4133-a1a7-9d539531e092",
    scryfall_id = "4ad8d899-5901-4fc8-b4f0-adcc0d97fc6e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of the Ruins",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(5),
        toughness = Some(7),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::exile(TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT))],
            targets = Some(TargetReq::up_to(
                TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT),
                2,
            )),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::SearchLibrary {
                filter: &PLAINS,
                finds: &[Find::HAND],
                optional: false,
            }],
            zone = ActivationZone::Hand,
        ),
    ],
);
