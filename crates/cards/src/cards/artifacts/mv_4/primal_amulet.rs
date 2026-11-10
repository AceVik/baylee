//! Primal Amulet // Primal Wellspring — {4} — Artifact // Land
//! Oracle: Instant and sorcery spells you cast cost {1} less to cast.
//! Oracle: Whenever you cast an instant or sorcery spell, put a charge counter on this artifact. Then if there are four or more charge counters on it, you may remove those counters and transform it.
//! Oracle: (Transforms from Primal Amulet.)
//! Oracle: {T}: Add one mana of any color. When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose new targets for the copy.
//! Set: XLN #243 — Ixalan | Scryfall ID: d4d379b5-7f56-4a7d-a4ac-131fc3d579c6 | Oracle ID: 8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49
//! Face: Primal Amulet — {4} — Artifact
//! Face: Primal Wellspring —  — Land

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::AddMana {
    source: ManaSource::Choice(&[
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]),
    amount: Amount::Fixed(1),
    distribution: ManaDistribution::OneColor,
    restriction: Some(ManaRestriction {
        uses: ManaUse::Spells,
        filter: &Filter::INSTANT_OR_SORCERY,
        ability_filter: None,
        rider: SpendRider::Copy,
        restricts: false,
        until_end_of_turn: false,
    }),
}])];

card!(
    index = index::PRIMAL_AMULET,
    oracle_id = "8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49",
    scryfall_id = "d4d379b5-7f56-4a7d-a4ac-131fc3d579c6",
    faces = &[
        face!(
            name = "Primal Amulet",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT
        ),
        face!(
            name = "Primal Wellspring",
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_MANA
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(f!(your INSTANT_OR_SORCERY), Modifier::SpellsCostLess(1)),
        triggered!(
            Trigger::SpellCast(&f!(your INSTANT_OR_SORCERY)),
            &[
                Effect::AddCounter {
                    kind: CounterKind::Charge,
                    amount: Amount::Fixed(1)
                },
                Effect::IfCondition {
                    condition: Condition::CountersOnSelf(CounterKind::Charge, 4),
                    then: &[Effect::MayDo {
                        effects: &[
                            Effect::RemoveAllCountersSelf {
                                kind: CounterKind::Charge
                            },
                            Effect::TransformSource,
                        ]
                    }],
                    otherwise: &[],
                },
            ]
        ),
    ],
);
