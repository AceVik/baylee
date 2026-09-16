//! Brazen Borrower // Petty Theft — {1}{U}{U} — Creature — Faerie Rogue // Instant — Adventure
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: This creature can block only creatures with flying.
//! Oracle: Return target nonland permanent an opponent controls to its owner's hand.
//! Set: SOC #190 — Secrets of Strixhaven Commander | Scryfall ID: 25d309d6-9e56-441e-bd29-5c903d5221bf | Oracle ID: c7b044c3-3cfa-407e-bf20-2875e8e04b7b
//! Face: Brazen Borrower — {1}{U}{U} — Creature — Faerie Rogue
//! Face: Petty Theft — {1}{U} — Instant — Adventure
// PARTIAL — a 3/1 flash flier whose adventure, Petty Theft, bounces a
// permanent an opponent controls and exiles the card on an adventure
// (CR 715), from where the Borrower itself may be cast later.
// NOT SUPPORTED: "This creature can block only creatures with flying."
// `combat::can_block` reads the attacker's flying/menace/unblockable, the
// blocker's flying/reach and `eval::protected_from`, and nothing else; no
// `Modifier` names the attackers a given blocker may be paired with, so
// `Filter::HasKeyword(KeywordSet::FLYING)` — the half of the sentence that
// *is* sayable — has nothing to be consumed by. The card plays as though
// the line were not printed, which here makes it a little better than
// printed rather than inert: a 3/1 that may also block on the ground.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Nonland permanent an opponent controls."
///
/// `TargetSpec::Object` draws its options from the battlefield, so
/// "permanent" costs no clause; the two printed restrictions are the two here.
static THEFT_TARGET: Filter = Filter::And(&[Filter::NONLAND, Filter::ControlledByOpponent]);

static PETTY_THEFT: &[AbilityDef] = &[spell!(
    &[Effect::bounce(TargetSpec::Object(&THEFT_TARGET))],
    targets = Some(TargetReq::one(TargetSpec::Object(&THEFT_TARGET)))
)];

card!(
    index = index::BRAZEN_BORROWER,
    oracle_id = "c7b044c3-3cfa-407e-bf20-2875e8e04b7b",
    scryfall_id = "25d309d6-9e56-441e-bd29-5c903d5221bf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Brazen Borrower",
            mana_cost = mana!("{1}{U}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::FAERIE, subtypes::creature::ROGUE],
            power = Some(3),
            toughness = Some(1),
        ),
        face!(
            name = "Petty Theft",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
            abilities = PETTY_THEFT,
            adventure = true,
        ),
    ],
    keywords = KeywordSet::FLASH.union(KeywordSet::FLYING),
    coverage = Coverage::Partial("the blocking restriction to fliers is not enforced"),
);

// Engine-level coverage belongs in `card_tests`: cast Petty Theft at flash
// speed, bounce an opponent's permanent, and cast Brazen Borrower off the
// adventure from exile afterwards — the path `cast_face_tests` already walks
// for Twining Twins, over a card whose adventure is a targeted bounce.
