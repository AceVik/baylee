//! Aisling Leprechaun — {G} — Creature — Faerie
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, that creature becomes green. (This effect lasts indefinitely.)
//! Set: LEG #173 — Legends | Scryfall ID: 640a161d-ad7b-4e5b-8f2d-d3753cb9daa3 | Oracle ID: 5456f00c-0bef-4c14-902f-f5c14475f284
// IMPLEMENTED — the block-pair trigger sets the other creature's color to
// green with `Duration::Indefinitely`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AISLING_LEPRECHAUN,
    oracle_id = "5456f00c-0bef-4c14-902f-f5c14475f284",
    scryfall_id = "640a161d-ad7b-4e5b-8f2d-d3753cb9daa3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Aisling Leprechaun",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetColor(ColorSet::from_slice(&[Color::Green])),
            Duration::Indefinitely,
        )],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);
