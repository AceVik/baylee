//! Zhalfirin Crusader — {1}{W}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{W}: The next 1 damage that would be dealt to this creature this turn is dealt to any target instead.
//! Set: VMA #54 — Vintage Masters | Scryfall ID: f566f52d-0eba-4b36-936f-002dab3354b3 | Oracle ID: 9e60c102-412e-4956-a7bb-a2cd737d6692
// PARTIAL: has flanking; the {1}{W} activated ability redirecting the next 1 damage that would be dealt to this creature this turn to any target is not yet modeled in RedirectNextDamage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WITHOUT_FLANKING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLANKING)),
]);

card!(
    index = index::ZHALFIRIN_CRUSADER,
    oracle_id = "9e60c102-412e-4956-a7bb-a2cd737d6692",
    scryfall_id = "f566f52d-0eba-4b36-936f-002dab3354b3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Partial(
        "has flanking; the {1}{W} activated ability redirecting the next 1 damage that would be dealt to this creature this turn to any target is not yet modeled in RedirectNextDamage",
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
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&WITHOUT_FLANKING),
        &[Effect::ModifyEvent {
            modifier: Modifier::ModifyPT(-1, -1),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
