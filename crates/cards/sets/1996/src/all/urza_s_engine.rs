//! Urza's Engine — {5} — Artifact Creature — Juggernaut
//! Oracle: Trample
//! Oracle: {3}: This creature gains banding until end of turn. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: {3}: Attacking creatures banded with this creature gain trample until end of turn.
//! Set: ALL #135 — Alliances | Scryfall ID: 273b54c3-325b-4f2e-857b-fc1d59b6b3c5 | Oracle ID: bf6ea878-044b-40a8-a21b-0191a948a69a
// IMPLEMENTED — Trample; {3}: gains banding until end of turn; {3}: attacking creatures banded with this creature gain trample until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_BANDED_WITH_SOURCE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::BandedWithSource,
]);

card!(
    index = index::URZA_S_ENGINE,
    oracle_id = "bf6ea878-044b-40a8-a21b-0191a948a69a",
    scryfall_id = "273b54c3-325b-4f2e-857b-fc1d59b6b3c5",
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urza's Engine",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(1),
        toughness = Some(5),
    ),],
    abilities = &[
        activated!(
            cost!("{3}"),
            &[Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::BANDING),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        activated!(
            cost!("{3}"),
            &[Effect::continuous(
                &ATTACKING_BANDED_WITH_SOURCE,
                Modifier::AddKeyword(KeywordSet::TRAMPLE),
                Duration::UntilEndOfTurn,
            )],
        ),
    ],
);
