//! Spitting Slug — {1}{G}{G} — Creature — Slug
//! Oracle: Whenever this creature blocks or becomes blocked, you may pay {1}{G}. If you do, this creature gains first strike until end of turn. Otherwise, each creature blocking or blocked by this creature gains first strike until end of turn.
//! Set: TSB #85 — Time Spiral Timeshifted | Scryfall ID: d98e413b-35bb-424f-9ae9-3319cf433785 | Oracle ID: 77af5e31-ac4d-4112-9138-e12abb5ce144
// IMPLEMENTED — whenever this creature blocks or becomes blocked, you may pay {1}{G}; if you do, it gains first strike until end of turn, otherwise each creature blocking or blocked by it gains first strike until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKING_OR_BLOCKED_BY_SOURCE: Filter =
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);

static SLUG_EFFECTS: &[Effect] = &[Effect::PlayerMayPayManaEachThenElse {
    player: PlayerRel::You,
    each: mana!("{1}{G}"),
    times: Amount::Fixed(1),
    paid: &[Effect::ModifySource {
        modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
        duration: Duration::UntilEndOfTurn,
    }],
    unpaid: &[Effect::CreateContinuousEffect {
        layer: Layer::Ability,
        filter: &BLOCKING_OR_BLOCKED_BY_SOURCE,
        modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
        duration: Duration::UntilEndOfTurn,
    }],
}];

card!(
    index = index::SPITTING_SLUG,
    oracle_id = "77af5e31-ac4d-4112-9138-e12abb5ce144",
    scryfall_id = "d98e413b-35bb-424f-9ae9-3319cf433785",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spitting Slug",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLUG],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(Trigger::Blocks(&Filter::This), SLUG_EFFECTS),
        triggered!(Trigger::BecomesBlocked, SLUG_EFFECTS),
    ],
);
