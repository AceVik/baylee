//! The Everflowing Well // The Myriad Pools — {2}{U} — Legendary Artifact // Legendary Artifact Land
//! Oracle: When The Everflowing Well enters, mill two cards, then draw two cards.
//! Oracle: Descend 8 — At the beginning of your upkeep, if there are eight or more permanent cards in your graveyard, transform The Everflowing Well.
//! Oracle: (Transforms from The Everflowing Well.)
//! Oracle: {T}: Add {U}.
//! Oracle: Whenever you cast a permanent spell using mana produced by The Myriad Pools, up to one other target permanent you control becomes a copy of that spell until end of turn.
//! Set: LCI #56 — The Lost Caverns of Ixalan | Scryfall ID: bf573fb7-fa6c-4df7-8e5e-1e071585361e | Oracle ID: 1f57a9f1-6b95-4395-bdf0-c5289b786ab1
//! Face: The Everflowing Well — {2}{U} — Legendary Artifact
//! Face: The Myriad Pools —  — Legendary Artifact Land

use baylee_cards_dsl::prelude::*;

static PERMANENT: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::BATTLE)
        .union(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::LAND)
        .union(TypeSet::PLANESWALKER),
);
static MYRIAD_POOLS: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
    triggered!(
        Trigger::SpellCastUsingSourceMana(&Filter::And(&[PERMANENT, Filter::ControlledByYou])),
        &[Effect::TargetsCopyEventSpellUntilEndOfTurn],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::And(&[
                PERMANENT,
                Filter::ControlledByYou,
                Filter::Another,
            ])),
            1
        )),
    ),
];

card!(
    index = index::THE_EVERFLOWING_WELL,
    oracle_id = "1f57a9f1-6b95-4395-bdf0-c5289b786ab1",
    scryfall_id = "bf573fb7-fa6c-4df7-8e5e-1e071585361e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "The Everflowing Well",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY
        ),
        face!(
            name = "The Myriad Pools",
            castable_from_hand = false,
            types = TypeSet::ARTIFACT.union(TypeSet::LAND),
            supertypes = SupertypeSet::LEGENDARY,
            abilities = MYRIAD_POOLS
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::Mill {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You
                },
                Effect::draw(2),
            ]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::TransformSource],
            condition = Some(Condition::AmountAtLeast(
                Amount::CountOf {
                    filter: &Filter::And(&[PERMANENT, Filter::Not(&Filter::IsToken)]),
                    zone: ZoneSel::GraveyardYou,
                },
                8
            )),
        ),
    ],
);
