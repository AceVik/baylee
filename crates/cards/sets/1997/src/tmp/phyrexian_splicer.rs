//! Phyrexian Splicer — {2} — Artifact
//! Oracle: {2}, {T}, Choose flying, first strike, trample, or shadow: Until end of turn, target creature with the chosen ability loses it and another target creature gains it.
//! Set: TMP #303 — Tempest | Scryfall ID: b1e9061a-b9e6-49ec-bc7e-c18557da9fd5 | Oracle ID: 4433a5fd-2daa-440a-b9cc-4ad028358cbc
// PARTIAL: models the choice of flying, first strike, trample, or shadow as four separate '{2}, {T}' activated abilities that each remove that keyword from target creature with it and grant it to another target creature until end of turn; choosing the keyword as a single activated ability's cost is not in CostPart.

use baylee_cards_dsl::prelude::*;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);
static CREATURE_WITH_FIRST_STRIKE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FIRST_STRIKE),
]);
static CREATURE_WITH_TRAMPLE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::TRAMPLE)]);
static CREATURE_WITH_SHADOW: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);
static ANOTHER_TARGET_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Reference)]);

card!(
    index = index::PHYREXIAN_SPLICER,
    oracle_id = "4433a5fd-2daa-440a-b9cc-4ad028358cbc",
    scryfall_id = "b1e9061a-b9e6-49ec-bc7e-c18557da9fd5",
    coverage = Coverage::Partial(
        "models the choice of flying, first strike, trample, or shadow as four separate '{2}, {T}' activated abilities that each remove that keyword from target creature with it and grant it to another target creature until end of turn; choosing the keyword as a single activated ability's cost is not in CostPart",
    ),
    faces = &[face!(
        name = "Phyrexian Splicer",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::Reference,
                    Modifier::RemoveKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
                Effect::PumpTargetSlot {
                    slot: TargetSlot::Second,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FLYING,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_FLYING))),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_TARGET_CREATURE))),
        ),
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::Reference,
                    Modifier::RemoveKeyword(KeywordSet::FIRST_STRIKE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::PumpTargetSlot {
                    slot: TargetSlot::Second,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FIRST_STRIKE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &CREATURE_WITH_FIRST_STRIKE,
            ))),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_TARGET_CREATURE))),
        ),
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::Reference,
                    Modifier::RemoveKeyword(KeywordSet::TRAMPLE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::PumpTargetSlot {
                    slot: TargetSlot::Second,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::TRAMPLE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_TRAMPLE))),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_TARGET_CREATURE))),
        ),
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::Reference,
                    Modifier::RemoveKeyword(KeywordSet::SHADOW),
                    Duration::UntilEndOfTurn,
                ),
                Effect::PumpTargetSlot {
                    slot: TargetSlot::Second,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::SHADOW,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_SHADOW))),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_TARGET_CREATURE))),
        ),
    ],
);
