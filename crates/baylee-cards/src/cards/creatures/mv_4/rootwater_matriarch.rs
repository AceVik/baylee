//! Rootwater Matriarch — {2}{U}{U} — Creature — Merfolk
//! Oracle: {T}: Gain control of target creature for as long as that creature is enchanted.
//! Set: 10E #103 — Tenth Edition | Scryfall ID: 09179261-aa02-4144-9028-3b8d6420a860 | Oracle ID: 1d260526-db57-486d-b0bc-d523139a0148
// PARTIAL: taps to gain control of target enchanted creature indefinitely; targeting an unenchanted creature and ending the control-change effect when that creature stops being enchanted is not in Duration.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Enchanted]);

card!(
    index = index::ROOTWATER_MATRIARCH,
    oracle_id = "1d260526-db57-486d-b0bc-d523139a0148",
    scryfall_id = "09179261-aa02-4144-9028-3b8d6420a860",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "taps to gain control of target enchanted creature indefinitely; targeting an unenchanted creature and ending the control-change effect when that creature stops being enchanted is not in Duration",
    ),
    faces = &[face!(
        name = "Rootwater Matriarch",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::GainControlAll {
            filter: &Filter::Reference,
            to: PlayerRel::You,
        }],
        target = Some(TargetSpec::Object(&ENCHANTED_CREATURE)),
    )],
);
