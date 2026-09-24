//! Murderous Rider // Swift End — {1}{B}{B} — Creature — Zombie Knight // Instant — Adventure
//! Oracle: Lifelink
//! Oracle: When this creature dies, put it on the bottom of its owner's library.
//! Oracle: Destroy target creature or planeswalker. You lose 2 life. (Then exile this card. You may cast the creature later from exile.)
//! Set: MOC #258 — March of the Machine Commander | Scryfall ID: 80fffad3-2486-4350-8dff-54a215ebfc28 | Oracle ID: 1080c5b5-6651-4c6a-93e6-099fbe389e26
//! Face: Murderous Rider — {1}{B}{B} — Creature — Zombie Knight
//! Face: Swift End — {1}{B}{B} — Instant — Adventure
// PARTIAL — Lifelink, and Swift End's "destroy target creature or planeswalker. You lose
// 2 life.", cast as an Adventure (CR 715): it resolves into exile, and the Rider may be cast
// from there. The dies trigger is not written (see the NOT SUPPORTED line).

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
    coverage = Coverage::Partial(
        "the dies trigger is unwritten (#240): PutTargetOnBottomOfLibrary moves its target from whatever zone it is in and the triggering card is not re-checked, so a Rider that left the graveyard in response would still be put on the bottom",
    ),
    // NOT SUPPORTED: "When this creature dies, put it on the bottom of its owner's library." —
    // `PutTargetOnBottomOfLibrary` aimed at the card that died says it, but the resolver does not
    // check that the card is still the one that died (CR 400.7); that is #240.
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
