//! Warping Wurm — {2}{G}{U} — Creature — Wurm
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: At the beginning of your upkeep, this creature phases out unless you pay {2}{G}{U}.
//! Oracle: Whenever this creature phases in, put a +1/+1 counter on it.
//! Set: MIR #287 — Mirage | Scryfall ID: a2c41d0f-f1db-4797-b245-7de12ffa3a0d | Oracle ID: c54c1517-cf7c-4afb-92f8-70c9d1591b45
// PARTIAL — phasing and upkeep trigger phasing out unless you pay {2}{G}{U}; no phase-in trigger ("Whenever this creature phases in, put a +1/+1 counter on it") exists yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARPING_WURM,
    oracle_id = "c54c1517-cf7c-4afb-92f8-70c9d1591b45",
    scryfall_id = "a2c41d0f-f1db-4797-b245-7de12ffa3a0d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    keywords = KeywordSet::PHASING,
    coverage = Coverage::Partial(
        "phasing and upkeep trigger phasing out unless you pay {2}{G}{U}; no phase-in trigger ('Whenever this creature phases in, put a +1/+1 counter on it') exists yet",
    ),
    faces = &[face!(
        name = "Warping Wurm",
        mana_cost = mana!("{2}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(1),
        toughness = Some(1),
        keywords = KeywordSet::PHASING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{2}{G}{U}"),
            effect: &Effect::PhaseOut { target: None },
        }],
    )],
);
