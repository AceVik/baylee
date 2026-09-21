//! Detection Tower — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Until end of turn, your opponents and creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.
//! Set: M19 #249 — Core Set 2019 | Scryfall ID: 02f99756-d334-4dba-a375-ba3d91ecae62 | Oracle ID: 93695c16-c441-492d-af12-b57df9739846
// PARTIAL — {T}: Add {C}, and the second ability strips hexproof from your
// opponents' creatures; the opponents themselves are NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DETECTION_TOWER,
    oracle_id = "93695c16-c441-492d-af12-b57df9739846",
    scryfall_id = "02f99756-d334-4dba-a375-ba3d91ecae62",
    faces = &[face!(name = "Detection Tower", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability also reaches your opponents themselves, and no Filter \
         matches a player — nor is there a Modifier that removes player hexproof"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "your opponents and creatures your opponents control with
        // hexproof" — a player is not an object, so no Filter can match one and
        // nothing takes back `Modifier::PlayerHexproof`. The creature half is
        // written, and it strips hexproof outright rather than only for your spells.
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::continuous(
                &Filter::OPPONENT_CREATURE,
                Modifier::RemoveKeyword(KeywordSet::HEXPROOF),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);
