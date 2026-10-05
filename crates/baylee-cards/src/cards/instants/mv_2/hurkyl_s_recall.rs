//! Hurkyl's Recall — {1}{U} — Instant
//! Oracle: Return all artifacts target player owns to their hand.
//! Set: MM2 #48 — Modern Masters 2015 | Scryfall ID: 73edeaaa-6a87-4cf1-b013-bab9a7bb94d9 | Oracle ID: ed1e5d24-c8a8-48fe-a88f-1003ad432477
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HURKYL_S_RECALL,
    oracle_id = "ed1e5d24-c8a8-48fe-a88f-1003ad432477",
    scryfall_id = "73edeaaa-6a87-4cf1-b013-bab9a7bb94d9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hurkyl's Recall",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "Effect::ReturnAllToHand takes only a filter and an opponents_only \
         flag, and no Filter names an ownership relation to a chosen or \
         targeted player (Filter::OwnedByYou answers for the spell's \
         controller only), so the effect cannot sweep the artifacts the \
         printed target player owns"
    ),
    // NOT SUPPORTED: "Return all artifacts target player owns to their
    // hand." — `Effect::ReturnAllToHand { filter, opponents_only }` sweeps
    // the battlefield by filter alone and has no player parameter. No
    // `Filter` names another player's ownership: `Filter::OwnedByYou` is the
    // spell's controller, `opponents_only` reads control rather than
    // ownership, and none of the `PlayerRel`s can be asked inside a filter.
    // Targeting a player with `TargetSpec::AnyPlayer` and running the sweep
    // anyway would return every artifact on the battlefield, so the whole
    // spell comes off the card.
    abilities = &[],
);
