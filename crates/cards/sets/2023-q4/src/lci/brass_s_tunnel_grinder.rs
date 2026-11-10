//! Brass's Tunnel-Grinder // Tecutlan, the Searing Rift — {2}{R} — Legendary Artifact // Legendary Land — Cave
//! Oracle: When Brass's Tunnel-Grinder enters, discard any number of cards, then draw that many cards plus one.
//! Oracle: At the beginning of your end step, if you descended this turn, put a bore counter on Brass's Tunnel-Grinder. Then if there are three or more bore counters on it, remove those counters and transform it. (You descended if a permanent card was put into your graveyard from anywhere.)
//! Oracle: (Transforms from Brass's Tunnel-Grinder.)
//! Oracle: {T}: Add {R}.
//! Oracle: Whenever you cast a permanent spell using mana produced by Tecutlan, discover X, where X is that spell's mana value.
//! Set: LCI #135 — The Lost Caverns of Ixalan | Scryfall ID: d61d8895-7f2e-4c77-951f-4f1a49e96f57 | Oracle ID: af1553eb-4f9f-4335-9078-56649bd8d8fc
//! Face: Brass's Tunnel-Grinder — {2}{R} — Legendary Artifact
//! Face: Tecutlan, the Searing Rift —  — Legendary Land — Cave

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;
static PERMANENT: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::BATTLE)
        .union(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::LAND)
        .union(TypeSet::PLANESWALKER),
);
static BACK_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
    triggered!(
        Trigger::SpellCastUsingSourceMana(&Filter::And(&[PERMANENT, Filter::ControlledByYou])),
        &[Effect::DiscoverAmount {
            mana_value: Amount::EventManaValue
        }]
    ),
];

card!(
    index = index::BRASS_S_TUNNEL_GRINDER,
    oracle_id = "af1553eb-4f9f-4335-9078-56649bd8d8fc",
    scryfall_id = "d61d8895-7f2e-4c77-951f-4f1a49e96f57",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Brass's Tunnel-Grinder",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Tecutlan, the Searing Rift",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_MANA,
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::DiscardAnyThenDraw, Effect::draw(1)]),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You
            },
            &[
                Effect::AddCounterSource {
                    kind: CounterKind::Bore,
                    amount: Amount::Fixed(1)
                },
                Effect::IfCondition {
                    condition: Condition::CountersOnSelf(CounterKind::Bore, 3),
                    then: &[
                        Effect::RemoveAllCountersSelf {
                            kind: CounterKind::Bore
                        },
                        Effect::TransformSource,
                    ],
                    otherwise: &[]
                },
            ],
            condition = Some(Condition::DescendedThisTurn)
        ),
    ],
);
