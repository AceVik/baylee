//! Zhalfirin Crusader — {1}{W}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{W}: The next 1 damage that would be dealt to this creature this turn is dealt to any target instead.
//! Set: VMA #54 — Vintage Masters | Scryfall ID: f566f52d-0eba-4b36-936f-002dab3354b3 | Oracle ID: 9e60c102-412e-4956-a7bb-a2cd737d6692
// PARTIAL: has flanking, and {1}{W} redirects the next 1 damage that would be dealt to this creature this turn to a target player; "any target" is not said, because the redirection can name a player and not a creature or planeswalker.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZHALFIRIN_CRUSADER,
    oracle_id = "9e60c102-412e-4956-a7bb-a2cd737d6692",
    scryfall_id = "f566f52d-0eba-4b36-936f-002dab3354b3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Partial(
        "has flanking, and {1}{W} redirects the next 1 damage that would be dealt to this creature this turn to a target player; \"any target\" is not said, because the redirection can name a player and not a creature or planeswalker",
    ),
    faces = &[face!(
        name = "Zhalfirin Crusader",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    // Flanking is the keyword above: the engine raises the -1/-1 trigger itself (CR 702.25a).
    abilities = &[
        // NOT SUPPORTED: the printed target is "any target"; the redirection names a player only.
        activated!(
            cost!("{1}{W}"),
            &[Effect::RedirectNextDamage {
                target: TargetSpec::ThisObject,
                amount: Amount::Fixed(1),
                to: PlayerRel::Chosen,
            }],
            target = Some(TargetSpec::AnyPlayer),
        ),
    ],
);
