//! Hanweir Battlements — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {R}, {T}: Target creature gains haste until end of turn.
//! Oracle: {3}{R}{R}, {T}: If you both own and control this land and a creature named Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township.
//! Set: INR #279 — Innistrad Remastered | Scryfall ID: 8c0acb91-edfc-43a5-af77-6614327fce43 | Oracle ID: 0e735ba6-7fd1-4d12-b20c-21525dc1e2b5
// PARTIAL — mana and haste work; the meld partner is not in the playable pool.

use crate::tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::HANWEIR_BATTLEMENTS,
    oracle_id = "0e735ba6-7fd1-4d12-b20c-21525dc1e2b5",
    scryfall_id = "8c0acb91-edfc-43a5-af77-6614327fce43",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Hanweir Battlements", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Hanweir Garrison is absent from the playable pool; full meld integration remains unverified"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{R}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
        activated!(
            cost!("{3}{R}{R}", TapSelf),
            &[Effect::MeldWithNamed {
                source_name: "Hanweir Battlements",
                partner_name: "Hanweir Garrison",
                melded: &tokens::HANWEIR_THE_WRITHING_TOWNSHIP,
            }]
        ),
    ],
);
