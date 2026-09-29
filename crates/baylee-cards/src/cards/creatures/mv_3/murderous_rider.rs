//! Murderous Rider // Swift End — {1}{B}{B} — Creature — Zombie Knight // Instant — Adventure
//! Oracle: Lifelink
//! Oracle: When this creature dies, put it on the bottom of its owner's library.
//! Oracle: Destroy target creature or planeswalker. You lose 2 life. (Then exile this card. You may cast the creature later from exile.)
//! Set: MOC #258 — March of the Machine Commander | Scryfall ID: 80fffad3-2486-4350-8dff-54a215ebfc28 | Oracle ID: 1080c5b5-6651-4c6a-93e6-099fbe389e26
//! Face: Murderous Rider — {1}{B}{B} — Creature — Zombie Knight
//! Face: Swift End — {1}{B}{B} — Instant — Adventure
// IMPLEMENTED — Lifelink; the dies trigger puts the card that died on the
// bottom of its owner's library, and only while it is still in the graveyard
// (CR 400.7); and Swift End's "destroy target creature or planeswalker. You
// lose 2 life.", cast as an Adventure (CR 715): it resolves into exile, and
// the Rider may be cast from there.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Swift End's own effect: destroy a creature or planeswalker, and you lose 2 life.
static SWIFT_END: &[AbilityDef] = &[spell!(
    &[
        Effect::destroy(TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER)),
        Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::You,
        },
    ],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &Filter::CREATURE_OR_PLANESWALKER
    )))
)];

card!(
    index = index::MURDEROUS_RIDER,
    oracle_id = "1080c5b5-6651-4c6a-93e6-099fbe389e26",
    scryfall_id = "80fffad3-2486-4350-8dff-54a215ebfc28",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::LIFELINK,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PutOnBottomOfLibraryFromGraveyard {
            target: TargetSpec::EventObject,
        }]
    )],
    faces = &[
        face!(
            name = "Murderous Rider",
            mana_cost = mana!("{1}{B}{B}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::KNIGHT],
            power = Some(2),
            toughness = Some(3),
        ),
        face!(
            name = "Swift End",
            mana_cost = mana!("{1}{B}{B}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
            abilities = SWIFT_END,
            adventure = true,
        ),
    ],
);
