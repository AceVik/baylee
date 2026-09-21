//! Valakut Awakening // Valakut Stoneforge — {2}{R} — Instant // Land
//! Oracle: Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #174 — Zendikar Rising | Scryfall ID: 228e551e-023a-4c9a-8f32-58dae6ffdf7f | Oracle ID: ff0ab867-b710-4b1a-baed-95fc3cf68f79
//! Face: Valakut Awakening — {2}{R} — Instant
//! Face: Valakut Stoneforge —  — Land
// PARTIAL — the back face is a whole land ({T}: Add {R}, and it enters
// tapped); the front face's spell cannot be said with this vocabulary.

use baylee_cards_dsl::prelude::*;

/// The back face's mana ability sits on the face rather than on the card,
/// because the front face is a spell and prints no mana.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::VALAKUT_AWAKENING,
    oracle_id = "ff0ab867-b710-4b1a-baed-95fc3cf68f79",
    scryfall_id = "228e551e-023a-4c9a-8f32-58dae6ffdf7f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Valakut Awakening: \"Put any number of cards from your hand on the \
         bottom of your library, then draw that many cards plus one\" — no \
         Effect moves an open-ended number of chosen cards from the hand to \
         the bottom (BottomCardFromHand is one card, PutFromHandOnTop is a \
         fixed count and the other end of the library), and no Amount reads \
         \"that many\"."
    ),
    faces = &[
        face!(
            name = "Valakut Awakening",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
            // NOT SUPPORTED: "Put any number of cards from your hand on the
            // bottom of your library, then draw that many cards plus one."
            // — there is no Effect for a variable-count hand-to-bottom move
            // and no Amount for "that many", so the front face states no
            // ability at all.
        ),
        face!(
            name = "Valakut Stoneforge",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
);
