//! Wall of Caltrops — {1}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature blocks a creature, if at least one other Wall creature is blocking that creature and no non-Wall creatures are blocking that creature, this creature gains banding until end of turn. (If any creatures with banding you control are blocking a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by.)
//! Set: LEG #42 — Legends | Scryfall ID: 664ad588-3002-4f63-93bd-38663171018f | Oracle ID: 20fa17d4-9c3e-470b-9993-a4a2799e33d1
// PARTIAL — defender only; the conditional banding trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_CALTROPS,
    oracle_id = "20fa17d4-9c3e-470b-9993-a4a2799e33d1",
    scryfall_id = "664ad588-3002-4f63-93bd-38663171018f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "the only block trigger is the union of blocking and becoming \
         blocked (no one-sided \"blocks a creature\"), and its intervening \
         condition reads the other creatures blocking the same attacker, \
         which no Condition can say"
    ),
    faces = &[face!(
        name = "Wall of Caltrops",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "Whenever this creature blocks a creature, if at least one
// other Wall creature is blocking that creature and no non-Wall creatures
// are blocking that creature, this creature gains banding until end of
// turn." — the effect half is expressible (`Effect::continuous` with
// `Modifier::AddKeyword(KeywordSet::BANDING)` for
// `Duration::UntilEndOfTurn`), but the trigger head is not:
// `Trigger::BlocksOrBecomesBlockedBy` fires on both sides of a block
// (CR 509.3b, 509.3d), so it would also fire once an effect lets this Wall
// attack and something blocks it, and there is no blocks-only trigger. Its
// intervening condition is missing too: no `Condition` reads the set of
// creatures blocking one attacker (`AttackedOrBlockedThisCombat` is about
// the source itself). Writing the trigger without the condition would grant
// banding whenever this creature blocks alone, which is not the card.
