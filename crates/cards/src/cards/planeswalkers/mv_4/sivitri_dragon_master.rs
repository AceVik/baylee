//! Sivitri, Dragon Master — {2}{U}{B} — Legendary Planeswalker — Sivitri
//! Oracle: +1: Until your next turn, creatures can't attack you or planeswalkers you control unless their controller pays 2 life for each of those creatures.
//! Oracle: −3: Search your library for a Dragon card, reveal it, put it into your hand, then shuffle.
//! Oracle: −7: Destroy all non-Dragon creatures.
//! Oracle: Sivitri, Dragon Master can be your commander.
//! Set: DMC #43 — Dominaria United Commander | Scryfall ID: b7505068-484e-4844-b3dd-89c28f18275d | Oracle ID: 5cf86603-cc47-4644-890a-0997c694817f
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SIVITRI_DRAGON_MASTER,
    oracle_id = "5cf86603-cc47-4644-890a-0997c694817f",
    scryfall_id = "b7505068-484e-4844-b3dd-89c28f18275d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::ExplicitlyAllowed,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sivitri, Dragon Master",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::SIVITRI],
        loyalty = Some(4),
    ),],
    abilities = &[
        loyalty!(
            1,
            &[Effect::continuous(
                &Filter::CREATURE,
                Modifier::AttackTaxLife { life: 2 },
                Duration::UntilYourNextTurn,
            )]
        ),
        loyalty!(
            -3,
            &[Effect::SearchLibrary {
                filter: &Filter::HasSubtype(subtypes::creature::DRAGON),
                finds: &[Find::HAND],
                optional: true,
            }]
        ),
        loyalty!(
            -7,
            &[Effect::destroy_all(&Filter::And(&[
                Filter::CREATURE,
                Filter::Not(&Filter::HasSubtype(subtypes::creature::DRAGON)),
            ]))]
        ),
    ],
);
