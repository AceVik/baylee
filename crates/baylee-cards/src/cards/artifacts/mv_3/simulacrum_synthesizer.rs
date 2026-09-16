//! Simulacrum Synthesizer — {2}{U} — Artifact
//! Oracle: When this artifact enters, scry 2.
//! Oracle: Whenever another artifact you control with mana value 3 or greater enters, create a 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control."
//! Set: BIG #6 — The Big Score | Scryfall ID: aaa05ad1-5cda-4edd-b6bf-562ae3e5011a | Oracle ID: eb7a1f21-a66d-415b-8520-710b44890bb6
// IMPLEMENTED — scry 2 on entry, then a Construct that grows with the board
// for every *other* artifact you control of mana value 3 or greater that
// enters.

use baylee_cards_dsl::prelude::*;

use crate::tokens::CONSTRUCT_0_0 as CONSTRUCT;

/// "another artifact you control with mana value 3 or greater".
///
/// `Another` is the whole of the word the card prints: the Synthesizer is
/// itself a {2}{U} artifact, so without it the card would trigger on its own
/// arrival and hand out a Construct beside the scry. A token has no mana cost
/// and so mana value 0 (CR 202.3a), which is why the Constructs this makes
/// never feed each other.
static ANOTHER_BIG_ARTIFACT_YOU_CONTROL: Filter = Filter::And(&[
    Filter::YOUR_ARTIFACT,
    Filter::Another,
    Filter::CmcAtLeast(3),
]);

card!(
    index = index::SIMULACRUM_SYNTHESIZER,
    oracle_id = "eb7a1f21-a66d-415b-8520-710b44890bb6",
    scryfall_id = "aaa05ad1-5cda-4edd-b6bf-562ae3e5011a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Simulacrum Synthesizer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::scry(2)]),
        // The token is the printed 0/0; its "+1/+1 for each artifact you
        // control" is the `Modifier::ModifyPTPerCount` this registers on the
        // token it just created, and that modifier counts only the
        // permanents the effect's controller controls — the same Construct
        // Urza's Saga makes.
        triggered!(
            Trigger::EntersBattlefield(&ANOTHER_BIG_ARTIFACT_YOU_CONTROL),
            &[Effect::CreateTokenPtPerCount {
                token: &CONSTRUCT,
                filter: &Filter::ARTIFACT,
                p: 1,
                t: 1,
            }]
        ),
    ],
);

// Behaviour belongs in `baylee-engine`'s trigger tests: play a {3} artifact
// beside this one and the Construct that enters is a 3/3 — the Synthesizer,
// the artifact just played, and the token itself, which is an artifact
// creature and counts itself the way Nettlecyst does.
