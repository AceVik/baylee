//! Arguel's Blood Fast // Temple of Aclazotz — {1}{B} — Legendary Enchantment // Legendary Land
//! Oracle: {1}{B}, Pay 2 life: Draw a card.
//! Oracle: At the beginning of your upkeep, if you have 5 or less life, you may transform Arguel's Blood Fast.
//! Oracle: (Transforms from Arguel's Blood Fast.)
//! Oracle: {T}: Add {B}.
//! Oracle: {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
//! Set: XLN #90 — Ixalan | Scryfall ID: c4ac7570-e74e-4081-ac53-cf41e695b7eb | Oracle ID: be2a4bc4-8af6-48c5-9421-32d26272e71a
//! Face: Arguel's Blood Fast — {1}{B} — Legendary Enchantment
//! Face: Temple of Aclazotz —  — Legendary Land
// IMPLEMENTED — front: {1}{B}, pay 2 life: draw a card. Back: {T}: Add {B}, and the
// transforming back is not castable from the hand (CR 712.2 — same file, two layouts, and
// this one is not a modal back). Two printed clauses have no DSL spelling: see NOT
// SUPPORTED below, and `coverage` is therefore Partial.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARGUEL_S_BLOOD_FAST,
    oracle_id = "be2a4bc4-8af6-48c5-9421-32d26272e71a",
    scryfall_id = "c4ac7570-e74e-4081-ac53-cf41e695b7eb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Arguel's Blood Fast",
            mana_cost = mana!("{1}{B}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Temple of Aclazotz",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "the upkeep transform — \"if you have 5 or less life\" is an intervening `if` no \
         Condition variant can read, and no Effect transforms a permanent in place — and \
         Temple's \"You gain life equal to the sacrificed creature's toughness\", whose \
         amount no Amount reads",
    ),
    abilities = &[activated!(cost!("{1}{B}", PayLife(2)), &[Effect::draw(1)])],
);

// NOT SUPPORTED: "At the beginning of your upkeep, if you have 5 or less life, you may
// transform Arguel's Blood Fast." — the trigger itself is a StepBegin{Upkeep, You}, but
// the intervening `if` is a life total and `Condition` has no variant for one, so the
// ability would fire on every upkeep and must come off the card rather than fire wrongly.
// (Even with the condition, nothing transforms a permanent in place.)
// NOT SUPPORTED: "{T}, Sacrifice a creature: You gain life equal to the sacrificed
// creature's toughness." — `cost!(TapSelf, Sacrifice(&Filter::CREATURE))` is a cost the
// wizard can pay, but the effect's amount is the toughness of the permanent that paid it
// and no `Amount` can reach a sacrificed object: `TargetPower` reads the first target and
// this ability names none.
