//! Petrified Hamlet — (no cost) — Land
//! Oracle: When this land enters, choose a land card name.
//! Oracle: Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.
//! Oracle: Lands with the chosen name have "{T}: Add {C}."
//! Oracle: {T}: Add {C}.
//! Set: SOS #259 — Secrets of Strixhaven | Scryfall ID: 355dd460-b0e9-41f2-a058-b7f7e39ac387 | Oracle ID: 78a2972c-14f4-41f3-99f9-167948bdd73a
use baylee_cards_dsl::prelude::*;

card!(
    index = index::PETRIFIED_HAMLET,
    oracle_id = "78a2972c-14f4-41f3-99f9-167948bdd73a",
    scryfall_id = "355dd460-b0e9-41f2-a058-b7f7e39ac387",
    faces = &[face!(name = "Petrified Hamlet", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        triggered!(
            Trigger::ETB,
            &[Effect::ChooseCardName {
                player: PlayerRel::You,
                types: TypeSet::LAND,
                excluded_types: TypeSet::EMPTY,
                remember: true
            }]
        ),
        static_ability!(Filter::Any, Modifier::ChosenNameCantActivate),
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::NamedBySource]),
            Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: &[Effect::mana(ManaColor::Colorless, 1)],
                mana_ability: true,
            }
        ),
    ],
);
