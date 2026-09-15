//! Harabaz Druid — {1}{G} — Creature — Human Druid Ally
//! Oracle: {T}: Add X mana of any one color, where X is the number of Allies you control.
//! Set: WWK #105 — Worldwake | Scryfall ID: 78a538cf-2291-49aa-8429-17d97d454479 | Oracle ID: ead985ec-f29f-4a3b-b8b1-061142cc5bd1
// IMPLEMENTED — dynamic Ally mana (choose a color, X = Allies).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card!(
    index = 66,
    oracle_id = "ead985ec-f29f-4a3b-b8b1-061142cc5bd1",
    scryfall_id = "78a538cf-2291-49aa-8429-17d97d454479",
    faces = &[face!(
        name = "Harabaz Druid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::HUMAN, creature::DRUID, creature::ALLY],
        power = Some(0),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice_dynamic(
        ALL_MANA_COLORS,
        Amount::CountOf {
            filter: &ALLIES_YOU,
            zone: ZoneSel::Battlefield,
        },
    )])],
);

// X = Allies is `Amount::CountOf`, evaluated at resolution against your own
// battlefield. The colour is **one** pick for the whole of X, which is what
// `mana_choice_dynamic` says and `mana_combination` — which this was written
// with — does not: that one is "in any combination", a pick per mana, so a
// player with three Allies was asked three times and could make {W}{U}{B}.
