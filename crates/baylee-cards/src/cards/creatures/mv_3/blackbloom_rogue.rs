//! Blackbloom Rogue // Blackbloom Bog — {2}{B} — Creature — Human Rogue // Land
//! Oracle: Menace (This creature can't be blocked except by two or more creatures.)
//! Oracle: This creature gets +3/+0 as long as an opponent has eight or more cards in their graveyard.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #91 — Zendikar Rising | Scryfall ID: 32779721-b021-4bd4-95d1-4a19b78d9faa | Oracle ID: 34320ebf-da97-44a4-bbeb-a9da06548289
//! Face: Blackbloom Rogue — {2}{B} — Creature — Human Rogue
//! Face: Blackbloom Bog —  — Land
// PARTIAL — a 2/3 Rogue with menace on the front; the back is an MDFC land
// reached by the face choice on a land play (CR 712.12), which comes down
// tapped and taps for {B}.
// NOT SUPPORTED: "This creature gets +3/+0 as long as an opponent has eight
// or more cards in their graveyard." A `StaticAbility` is a layer, a filter
// and a modifier, with nowhere to put a condition, and a `Filter` asks about
// an object — its types, its controller, its zone — never about how many
// cards a graveyard holds, so there is no way to say the *while* half of the
// sentence. `Condition::OpponentGraveyardCountAtLeast(8)` states
// exactly this condition and gates an activated ability alone.
// The modifier is left off the card rather than written unconditionally: a
// bare `Modifier::ModifyPT(3, 0)` would be a permanent 5/3, stronger than the
// printing, where leaving it out only ever holds the Rogue at its printed
// 2/3 in a game the condition would have turned on.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's whole printed text apart from the tapland clause, which
/// is an `EnterModifier` rather than an ability.
static BOG_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::BLACKBLOOM_ROGUE,
    oracle_id = "34320ebf-da97-44a4-bbeb-a9da06548289",
    scryfall_id = "32779721-b021-4bd4-95d1-4a19b78d9faa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Blackbloom Rogue",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
            power = Some(2),
            toughness = Some(3),
        ),
        face!(
            name = "Blackbloom Bog",
            types = TypeSet::LAND,
            abilities = BOG_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    keywords = KeywordSet::MENACE,
    coverage = Coverage::Partial("the graveyard-threshold +3/+0 is never applied"),
);

// Behaviour belongs in `baylee-engine`'s `mdfc_tests`: the front face is the
// only one that is not a land, so a land play resolves straight to face 1,
// which enters tapped and taps for {B} once it untaps; cast as a creature it
// is a 2/3 that one blocker may not block on its own (CR 702.111b).
