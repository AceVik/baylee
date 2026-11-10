//! Great Hall of the Biblioplex — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell.
//! Oracle: {5}: If this land isn't a creature, it becomes a 2/4 Wizard creature with "Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn." It's still a land.
//! Set: SOS #257 — Secrets of Strixhaven | Scryfall ID: 42d92674-2664-411c-b9c5-b04da7c845f4 | Oracle ID: a8c70dab-1e27-4a9c-bd2d-910d5720d02d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "…it becomes a 2/4 Wizard creature with 'Whenever you cast an instant or
/// sorcery spell, this creature gets +1/+0 until end of turn.' It's still a
/// land." The sentence prints no duration, so the land stays a Wizard; and
/// `AddType` adds, so the land type is never taken away.
static BECOME_A_WIZARD: &[Effect] = &[
    Effect::continuous(
        &Filter::This,
        Modifier::AddType(TypeSet::CREATURE),
        Duration::Indefinitely,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddSubtype(creature::WIZARD),
        Duration::Indefinitely,
    ),
    Effect::continuous(&Filter::This, Modifier::SetPT(2, 4), Duration::Indefinitely),
    Effect::continuous(
        &Filter::This,
        Modifier::GrantTriggered {
            trigger: Trigger::SpellCast(&f!(your INSTANT_OR_SORCERY)),
            effects: &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 0),
                Duration::UntilEndOfTurn,
            )],
            target: None,
        },
        Duration::Indefinitely,
    ),
];

card!(
    index = index::GREAT_HALL_OF_THE_BIBLIOPLEX,
    oracle_id = "a8c70dab-1e27-4a9c-bd2d-910d5720d02d",
    scryfall_id = "42d92674-2664-411c-b9c5-b04da7c845f4",
    faces = &[face!(
        name = "Great Hall of the Biblioplex",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_of_any_color()
                .restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::None)]
        ),
        // "If this land isn't a creature" is asked as the ability resolves:
        // a land that is already a Wizard is left as it is.
        activated!(
            cost!("{5}"),
            &[Effect::IfCondition {
                condition: Condition::SourceMatches(&Filter::CREATURE),
                then: &[],
                otherwise: BECOME_A_WIZARD,
            }]
        ),
    ],
);
