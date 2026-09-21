//! Stump Stomp // Burnwillow Clearing — {1}{R/G} — Sorcery // Land
//! Oracle: Target creature you control deals damage equal to its power to target creature or planeswalker you don't control.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Set: MH3 #259 — Modern Horizons 3 | Scryfall ID: 49974246-0a3b-4ec9-b5ea-2a89df9bb0b5 | Oracle ID: eb7b1284-0b2c-4b6a-a389-b2b932838083
//! Face: Stump Stomp — {1}{R/G} — Sorcery
//! Face: Burnwillow Clearing —  — Land
// PARTIAL — Burnwillow Clearing enters tapped and adds {R} or {G}; Stump
// Stomp's clause has no effect variant.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Red,
    ManaColor::Green,
])])];

card!(
    index = index::STUMP_STOMP,
    oracle_id = "eb7b1284-0b2c-4b6a-a389-b2b932838083",
    scryfall_id = "49974246-0a3b-4ec9-b5ea-2a89df9bb0b5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(
            name = "Stump Stomp",
            mana_cost = mana!("{1}{R/G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Burnwillow Clearing",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "Stump Stomp: two different targets, one dealing damage equal to its power to the other — no Effect variant carries a second TargetSpec",
    ),
    // NOT SUPPORTED: Target creature you control deals damage equal to its
    // power to target creature or planeswalker you don't control.
);
