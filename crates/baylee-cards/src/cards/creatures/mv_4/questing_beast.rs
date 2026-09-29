//! Questing Beast — {2}{G}{G} — Legendary Creature — Beast
//! Oracle: Vigilance, deathtouch, haste
//! Oracle: Questing Beast can't be blocked by creatures with power 2 or less.
//! Oracle: Combat damage that would be dealt by creatures you control can't be prevented.
//! Oracle: Whenever Questing Beast deals combat damage to an opponent, it deals that much damage to target planeswalker that player controls.
//! Set: ELD #171 — Throne of Eldraine | Scryfall ID: e41cf82d-3213-47ce-a015-6e51a8b07e4f | Oracle ID: b685757b-521e-4353-a233-97052359723d
// PARTIAL — vigilance, deathtouch, haste; can't be blocked by power 2 or
// less; your creatures' combat damage can't be prevented.
// NOT SUPPORTED: "Whenever Questing Beast deals combat damage to an opponent,
// it deals that much damage to target planeswalker that player controls" —
// the DSL has no amount for "that much" damage an event dealt, and no target
// filter relative to the player the event named.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUESTING_BEAST,
    oracle_id = "b685757b-521e-4353-a233-97052359723d",
    scryfall_id = "e41cf82d-3213-47ce-a015-6e51a8b07e4f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::VIGILANCE
        .union(KeywordSet::DEATHTOUCH)
        .union(KeywordSet::HASTE),
    faces = &[face!(
        name = "Questing Beast",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "Whenever Questing Beast deals combat damage to an opponent, it deals that much \
         damage to target planeswalker that player controls."
    ),
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantBeBlockedBy(&Filter::PowerAtMost(2))
        ),
        static_ability!(Filter::YOUR_CREATURE, Modifier::CombatDamageCantBePrevented),
    ],
);
