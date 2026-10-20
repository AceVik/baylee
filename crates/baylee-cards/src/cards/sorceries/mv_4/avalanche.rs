//! Avalanche — {X}{2}{R}{R} — Sorcery
//! Oracle: Destroy X target snow lands.
//! Set: ICE #171 — Ice Age | Scryfall ID: d3a925e5-0d0a-42ec-b1c6-9793b8e11625 | Oracle ID: a349e526-882c-454f-ba79-ad17340f8153

use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::AVALANCHE,
    oracle_id = "a349e526-882c-454f-ba79-ad17340f8153",
    scryfall_id = "d3a925e5-0d0a-42ec-b1c6-9793b8e11625",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Avalanche",
        mana_cost = mana!("{X}{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&SNOW_LAND))],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&SNOW_LAND)))
    )],
);
