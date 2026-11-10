//! Profane Procession // Tomb of the Dusk Rose — {1}{W}{B} — Legendary Enchantment // Legendary Land
//! Oracle: {3}{W}{B}: Exile target creature. Then if there are three or more cards exiled with Profane Procession, transform it.
//! Oracle: (Transforms from Profane Procession.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {2}{W}{B}, {T}: Put a creature card exiled with this permanent onto the battlefield under your control.
//! Set: RIX #166 — Rivals of Ixalan | Scryfall ID: 1d94ff37-f04e-48ee-8253-d62ab07f0632 | Oracle ID: a656ad7f-133f-4d93-919a-43bcf1f815f3
//! Face: Profane Procession — {1}{W}{B} — Legendary Enchantment
//! Face: Tomb of the Dusk Rose —  — Legendary Land
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana_of_any_color()]),
    activated!(
        cost!("{2}{W}{B}", TapSelf),
        &[Effect::ReturnChosenLinkedToBattlefield {
            filter: &Filter::CREATURE,
            other_to_hand: false,
        }]
    ),
];

card!(
    index = index::PROFANE_PROCESSION,
    oracle_id = "a656ad7f-133f-4d93-919a-43bcf1f815f3",
    scryfall_id = "1d94ff37-f04e-48ee-8253-d62ab07f0632",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[
        face!(
            name = "Profane Procession",
            mana_cost = mana!("{1}{W}{B}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Tomb of the Dusk Rose",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{3}{W}{B}"),
        &[
            Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE)),
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::CountOf {
                        filter: &Filter::Any,
                        zone: ZoneSel::LinkedExile,
                    },
                    3,
                ),
                then: &[Effect::TransformSource],
                otherwise: &[],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
