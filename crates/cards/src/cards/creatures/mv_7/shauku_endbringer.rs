//! Shauku, Endbringer — {5}{B}{B} — Legendary Creature — Vampire
//! Oracle: Flying
//! Oracle: Shauku can't attack if there's another creature on the battlefield.
//! Oracle: At the beginning of your upkeep, you lose 3 life.
//! Oracle: {T}: Exile target creature and put a +1/+1 counter on Shauku.
//! Set: MIR #142 — Mirage | Scryfall ID: 06d94b21-7568-4e5c-a8ec-ff5bb48a4f36 | Oracle ID: 49478593-29ff-4e5a-b1e8-6462c2d6176d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE: Filter = Filter::And(&[Filter::Another, Filter::CREATURE]);

card!(
    index = index::SHAUKU_ENDBRINGER,
    oracle_id = "49478593-29ff-4e5a-b1e8-6462c2d6176d",
    scryfall_id = "06d94b21-7568-4e5c-a8ec-ff5bb48a4f36",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shauku, Endbringer",
        mana_cost = mana!("{5}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
            condition = Some(Condition::BattlefieldCount(&OTHER_CREATURE, 1)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }],
        ),
        activated!(
            Cost::TAP,
            &[
                Effect::Exile {
                    target: TargetSpec::Object(&Filter::CREATURE),
                },
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
