//! Wall of Dust — {2}{R} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature blocks a creature, that creature can't attack during its controller's next turn.
//! Set: 4ED #229 — Fourth Edition | Scryfall ID: b25b81fb-1d0f-4c0c-8d54-fbf9c1d54578 | Oracle ID: 14895574-9c87-4e80-9bc9-cc3dd22a42b8
// PARTIAL — defender is on the card; the block trigger is off, see the
// NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_DUST,
    oracle_id = "14895574-9c87-4e80-9bc9-cc3dd22a42b8",
    scryfall_id = "b25b81fb-1d0f-4c0c-8d54-fbf9c1d54578",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "no Duration keeps a creature from attacking through its controller's \
         next turn, so the blocked-creature trigger has no window to live in"
    ),
    faces = &[face!(
        name = "Wall of Dust",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// NOT SUPPORTED: "Whenever this creature blocks a creature, that creature
// can't attack during its controller's next turn." — the trigger half is
// `Trigger::BlocksOrBecomesBlockedBy` (the Wall can never attack, so the
// union fires only when it blocks), and "can't attack" is
// `Modifier::AddKeyword(KeywordSet::CANT_ATTACK)`, but no `Duration` lasts
// until the blocked creature's controller's next turn: `UntilYourNextTurn`
// ends at the Wall's controller's next turn, one turn too early, and
// `UntilYourNextUntapStep` is earlier still. The trigger comes off the card
// rather than shipping an effect that expires before the turn it names.
