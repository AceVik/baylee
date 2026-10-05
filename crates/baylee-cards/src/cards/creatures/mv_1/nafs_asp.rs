//! Nafs Asp — {G} — Creature — Snake
//! Oracle: Whenever this creature deals damage to a player, that player loses 1 life at the beginning of their next draw step unless they pay {1} before that draw step.
//! Set: 4ED #264 — Fourth Edition | Scryfall ID: 0db4a4ef-20a8-415f-8d7d-a6740b482f73 | Oracle ID: da7bc9d6-a0cc-407a-bed9-d677c2dd74f2
// PARTIAL — the damage trigger and its delayed payment are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NAFS_ASP,
    oracle_id = "da7bc9d6-a0cc-407a-bed9-d677c2dd74f2",
    scryfall_id = "0db4a4ef-20a8-415f-8d7d-a6740b482f73",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nafs Asp",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no trigger names this creature as the source of damage dealt to a \
         player, and no delayed ability waits for that player's next draw \
         step with a {1} payment window before it"
    ),
    // NOT SUPPORTED: "Whenever this creature deals damage to a player, that
    // player loses 1 life at the beginning of their next draw step unless
    // they pay {1} before that draw step." — Trigger::PlayerDealtDamage
    // hears a player dealt damage but cannot require this creature as the
    // source, and Trigger::DealsCombatDamageToPlayer is combat only; no
    // effect installs a delayed trigger at a future draw step
    // (PayCostOrLoseLater is your next upkeep and loses the game), and
    // PlayerMayPayOr asks its price now rather than before that step.
);
