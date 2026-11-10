//! Hall of Heliod's Generosity — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{W}, {T}: Put target enchantment card from your graveyard on top of your library.
//! Set: DSC #283 — Duskmourn: House of Horror Commander | Scryfall ID: f08dfd99-59fa-4a6c-bd69-c647f639de4d | Oracle ID: 2fc070dc-f2f7-4648-8069-31d74790a39c
// IMPLEMENTED — {T} for {C}, and {1}{W}, {T} to put a target enchantment card from your graveyard on top of your library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_HELIOD_S_GENEROSITY,
    oracle_id = "2fc070dc-f2f7-4648-8069-31d74790a39c",
    scryfall_id = "f08dfd99-59fa-4a6c-bd69-c647f639de4d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Hall of Heliod's Generosity",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf),
            &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::ENCHANTMENT, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::ENCHANTMENT,
                PlayerRel::You
            )),
        ),
    ],
);
