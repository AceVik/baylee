//! Secret Tunnel — (no cost) — Land — Cave
//! Oracle: This land can't be blocked.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn.
//! Set: TLA #278 — Avatar: The Last Airbender | Scryfall ID: 2d39a0e1-6484-409c-ab05-5b276925a949 | Oracle ID: 632e2979-d88a-482e-9bb8-57b683c5310f
// IMPLEMENTED — intrinsic unblockability, colorless mana, and a shared-type
// creature pair made unblockable until the end of the turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PAIR: Filter = Filter::And(&[
    Filter::YOUR_CREATURE,
    Filter::SharesCreatureTypeWithOtherTarget,
]);

card!(
    index = index::SECRET_TUNNEL,
    oracle_id = "632e2979-d88a-482e-9bb8-57b683c5310f",
    scryfall_id = "2d39a0e1-6484-409c-ab05-5b276925a949",
    faces = &[face!(
        name = "Secret Tunnel",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // This land can't be blocked.
        static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[
                Effect::PumpTargetSlot {
                    slot: TargetSlot::First,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::UNBLOCKABLE,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::PumpTargetSlot {
                    slot: TargetSlot::Second,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::UNBLOCKABLE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            target = Some(TargetSpec::Object(&PAIR)),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&PAIR))),
        ),
    ],
);
