//! Vaporous Djinn — {2}{U}{U} — Creature — Djinn
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, this creature phases out unless you pay {U}{U}. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: MIR #101 — Mirage | Scryfall ID: e7ea65e2-68d8-429f-9be7-e6e5e12a2a4d | Oracle ID: 66fc64db-7dc4-43fa-8087-281a72286c35
// IMPLEMENTED — flying and upkeep trigger phasing out unless you pay {U}{U}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VAPOROUS_DJINN,
    oracle_id = "66fc64db-7dc4-43fa-8087-281a72286c35",
    scryfall_id = "e7ea65e2-68d8-429f-9be7-e6e5e12a2a4d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vaporous Djinn",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(3),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{U}{U}"),
            effect: &Effect::PhaseOut { target: None },
        }],
    )],
);
