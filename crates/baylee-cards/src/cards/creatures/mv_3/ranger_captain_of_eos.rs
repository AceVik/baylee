//! Ranger-Captain of Eos — {1}{W}{W} — Creature — Human Soldier Ranger
//! Oracle: When this creature enters, you may search your library for a creature card with mana value 1 or less, reveal it, put it into your hand, then shuffle.
//! Oracle: Sacrifice this creature: Your opponents can't cast noncreature spells this turn.
//! Set: MH1 #21 — Modern Horizons | Scryfall ID: af3928b4-813a-4120-8799-de34235d60ac | Oracle ID: cada3481-cc2b-4412-b9b5-0436af53aad2
// PARTIAL — the ETB search is built (optional, one creature card with mana
// value ≤ 1 to hand; reveal and shuffle are derived). The sacrifice ability
// is not expressible — see the // NOT SUPPORTED: line.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RANGER_CAPTAIN_OF_EOS,
    oracle_id = "cada3481-cc2b-4412-b9b5-0436af53aad2",
    scryfall_id = "af3928b4-813a-4120-8799-de34235d60ac",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Sacrifice this creature: Your opponents can't cast noncreature spells this turn."
    ),
    faces = &[face!(
        name = "Ranger-Captain of Eos",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::RANGER
        ],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::SearchLibrary {
                filter: &Filter::And(&[Filter::CREATURE, Filter::CmcAtMost(1)]),
                finds: &[Find::HAND],
                optional: true,
            }]
        ),
        // NOT SUPPORTED: "Sacrifice this creature: Your opponents can't cast
        // noncreature spells this turn." — no Modifier says "the effect's
        // opponents can't cast this kind of spell": Modifier::OpponentsCastAsSorcery
        // restricts *when* they may cast, not which cards. The ability is off
        // the card rather than offered as an activation that does nothing.
    ],
);
