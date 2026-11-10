//! Dauthi Mindripper — {3}{B} — Creature — Dauthi Minion
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, defending player discards three cards.
//! Set: TMP #125 — Tempest | Scryfall ID: 6fb990ee-f027-4c74-a67e-98ada6aa21e4 | Oracle ID: ecae5eba-8407-4c71-aeae-ab10fb1e5226
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAUTHI_MINDRIPPER,
    oracle_id = "ecae5eba-8407-4c71-aeae-ab10fb1e5226",
    scryfall_id = "6fb990ee-f027-4c74-a67e-98ada6aa21e4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dauthi Mindripper",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::DiscardForPlayers {
                    who: PlayerRel::DefendingPlayer,
                    count: 3,
                }],
            }],
        }],
    )],
);
